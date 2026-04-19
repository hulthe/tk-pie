//! UART implementation of communication between keyboard halves

use bytemuck::cast;
use embassy_rp::peripherals::{PIN_0, PIN_1, UART0};
use embassy_rp::uart::{self, BufferedUartRx, BufferedUartTx, DataBits, Parity, StopBits};
use embassy_rp::Peri;
use embassy_sync::pubsub::PubSubBehavior;
use embedded_io_async::{Read, Write};
use futures::{select_biased, FutureExt};
use heapless::Vec;
use static_cell::StaticCell;

use super::{
    compare_seqnums, Header, Message, HEADER_LEN, MAX_BODY_LEN, RESEND_COUNT, RX_BUF_CAP,
    USB_EVENTS_OUT,
};

use crate::{
    event::Half,
    interrupts::Irqs,
    keyboard::{KbEvents, KbEventsRx, KbEventsTx},
    usb::USB_EVENTS,
    Spawners,
};

type Device = UART0;

/// Spawn a task to forward keyboard and usb events between keyboard halves.
pub async fn start(
    tx: Peri<'static, PIN_0>,
    rx: Peri<'static, PIN_1>,
    uart: Peri<'static, Device>,
    board: Half,
    spawners: Spawners,
    events: KbEvents,
) {
    static TX_BUF: StaticCell<[u8; 256]> = StaticCell::new();
    static RX_BUF: StaticCell<[u8; 256]> = StaticCell::new();

    let mut config = uart::Config::default();
    config.baudrate = 115200;
    config.data_bits = DataBits::DataBits8;
    config.stop_bits = StopBits::STOP2;
    config.parity = Parity::ParityOdd;

    let uart = embassy_rp::uart::BufferedUart::new(
        uart,
        tx,
        rx,
        Irqs,
        TX_BUF.init_with(|| [0u8; 256]),
        RX_BUF.init_with(|| [0u8; 256]),
        config,
    );

    let (tx, rx) = uart.split();
    let (events_rx, events_tx) = events.split();
    spawners.med.must_spawn(send_messages(events_rx, tx, board));
    spawners.high.must_spawn(receive_messages(events_tx, rx));
}

#[embassy_executor::task]
async fn send_messages(mut events: KbEventsRx, mut tx: BufferedUartTx, this_half: Half) -> ! {
    let mut buf = [0u8; HEADER_LEN + MAX_BODY_LEN];
    let mut counter = 0u8;
    let mut seqnum = 0u16;

    // forward messages to the other keyboard half
    loop {
        let message = select_biased! {
            event = events.recv().fuse() => {
                if event.source != this_half {
                    continue; // do not forward messages from the other half back to it
                }
                Message::KeyboardEvent(event)
            }
            event = USB_EVENTS_OUT.receive().fuse() => Message::UsbEvent(event),
        };

        let (buf_header, buf_body) = buf
            .split_first_chunk_mut()
            .expect("buf length is >= HEADER_LEN");
        let serialized = match postcard::to_slice(&message, buf_body) {
            Ok(s) => s,
            Err(e) => {
                log::error!("failed to serialize uart message: {e}");
                continue;
            }
        };

        // add a "random" value to feed the crc
        counter = counter.wrapping_add(1);
        let random = counter;
        let len = serialized.len() as u8;
        let header = Header::new(random, seqnum, serialized);

        seqnum = seqnum.wrapping_add(1);

        let header: [u8; HEADER_LEN] = cast(header);
        *buf_header = header;

        let package = &buf[..HEADER_LEN + usize::from(len)];

        for _ in 0..RESEND_COUNT {
            tx.write_all(package).await.ok();
        }
    }
}

#[embassy_executor::task]
async fn receive_messages(mut events: KbEventsTx, mut rx: BufferedUartRx) -> ! {
    let mut buf: heapless::Vec<u8, RX_BUF_CAP> = Vec::new();
    let mut newest_seen_seqnum = 0u16;

    loop {
        if buf.len() >= HEADER_LEN {
            let (&header, rest) = buf.split_first_chunk::<HEADER_LEN>().unwrap();
            let header: Header = cast(header);
            if !header.valid_header_crc() {
                log::error!("invalid uart header crc: {header:x?}");
                buf.remove(0); // drop the first byte and hope we find a good packet header
                continue;
            }

            log::trace!(
                "reading from uart, header={header:?}, bytes_received={}",
                rest.len()
            );

            let len = usize::from(header.len);

            let drop_packet = |buf: &mut heapless::Vec<u8, RX_BUF_CAP>| {
                // drop the packet bytes and shuffle the remaining bytes forward
                buf.rotate_left(len + HEADER_LEN);
                buf.truncate(buf.len() - len - HEADER_LEN);
            };

            if rest.len() >= len {
                // check if this is a resend of an old packet, if so: ignore it.
                if compare_seqnums(header.seqnum.value(), newest_seen_seqnum)
                    .should_discard_message()
                {
                    drop_packet(&mut buf);
                    continue;
                }

                let body = &rest[..len];
                if !header.valid_body_crc(body) {
                    log::error!("invalid uart body crc: {header:x?}");

                    drop_packet(&mut buf);
                    continue;
                }

                newest_seen_seqnum = header.seqnum.value();

                let r = postcard::from_bytes(body);

                drop_packet(&mut buf);

                let message: Message = match r {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!("failed to deserialize uart message: {e}");
                        continue;
                    }
                };

                match &message {
                    Message::KeyboardEvent(event) => events.send(event.clone()),
                    &Message::UsbEvent(event) => USB_EVENTS.publish_immediate(event),
                }

                log::info!("got msg: {:?}", message);
            }
        }

        let mut chunk = [0u8; 128];
        let n = match rx.read(&mut chunk).await {
            Ok(n) => n,
            Err(e) => {
                log::error!("uart error: {:?}", e);
                continue;
            }
        };
        if buf.extend_from_slice(&chunk[..n]).is_err() {
            log::error!("uart buffer full");
            buf.clear();
        }
    }
}
