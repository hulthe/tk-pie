use core::mem::size_of;

use bytemuck::{cast, AnyBitPattern, NoUninit};
use crc_any::CRCu16;
use embassy_executor::Spawner;
use embassy_rp::peripherals::{PIN_0, PIN_1, UART0};
use embassy_rp::uart::{self, BufferedUart, DataBits, Parity, StopBits};
use embassy_sync::channel::Channel;
use embassy_sync::pubsub::PubSubBehavior;
use embedded_io_async::{Read, Write};
use futures::{select_biased, FutureExt};
use heapless::Vec;
use serde::{Deserialize, Serialize};
use static_cell::StaticCell;

use crate::event::{switch, Half};
use crate::interrupts::Irqs;
use crate::keyboard::KbEvents;
use crate::usb::{UsbEvent, USB_EVENTS};
use crate::util::CS;

/// Channel for [UsbEvent]s to be sent on the uart line.
pub static UART_USB_EVENTS_OUT: Channel<CS, UsbEvent, 8> = Channel::new();

#[derive(Clone, Debug, Serialize, Deserialize)]
enum Message {
    KeyboardEvent(switch::Event),
    UsbEvent(UsbEvent),
}

pub async fn start(tx: PIN_0, rx: PIN_1, uart: UART0, board: Half, events: KbEvents) {
    static TX_BUF: StaticCell<[u8; 256]> = StaticCell::new();
    static RX_BUF: StaticCell<[u8; 256]> = StaticCell::new();

    let mut config = uart::Config::default();
    config.baudrate = 115200;
    config.data_bits = DataBits::DataBits8;
    config.stop_bits = StopBits::STOP1;
    config.parity = Parity::ParityNone;

    let uart = embassy_rp::uart::BufferedUart::new(
        uart,
        Irqs,
        tx,
        rx,
        TX_BUF.init_with(|| [0u8; 256]),
        RX_BUF.init_with(|| [0u8; 256]),
        config,
    );

    Spawner::for_current_executor()
        .await
        .must_spawn(uart_task(uart, board, events))
}

#[embassy_executor::task]
async fn uart_task(uart: BufferedUart<'static, UART0>, this_half: Half, mut events: KbEvents) {
    let (mut rx, mut tx) = uart.split();
    let (mut events_rx, mut events_tx) = events.split();

    /// The header of a UART packet.
    #[repr(C)]
    #[derive(Clone, Copy, Debug, NoUninit, AnyBitPattern)]
    struct Header {
        /// The length of the payload.
        len: u8,

        /// An arbitrary value to feed the crc, should be different for each message.
        random: u8,

        /// A little-endian crc16.
        crc: [u8; 2],
    }

    const HEADER_LEN: usize = size_of::<Header>();

    let rx_task = async {
        let mut buf: heapless::Vec<u8, 1024> = Vec::new();
        loop {
            if buf.len() >= HEADER_LEN {
                let (&header, rest) = buf.split_array_ref::<HEADER_LEN>();
                let header: Header = cast(header);
                let crc = u16::from_le_bytes(header.crc);
                let mut calculated_crc = CRCu16::crc16();
                calculated_crc.digest(&[header.len, header.random]);
                let calculated_crc = calculated_crc.get_crc();

                if calculated_crc != crc {
                    log::error!("invalid uart header crc: {header:x?}");
                    buf.remove(0); // drop the first byte and hope we find a good packet header
                    continue;
                }

                log::trace!(
                    "reading from uart, header={header:?}, bytes_received={}",
                    rest.len()
                );

                let len = usize::from(header.len);
                if rest.len() >= len {
                    let r = postcard::from_bytes(&rest[..len]);

                    // drop packet from buffer
                    buf.rotate_left(len + HEADER_LEN);
                    buf.truncate(buf.len() - len - HEADER_LEN);

                    let message: Message = match r {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("failed to deserialize message: {e}");
                            continue;
                        }
                    };

                    match &message {
                        Message::KeyboardEvent(event) => events_tx.send(event.clone()),
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
    };

    let tx_task = async {
        let mut buf = [0u8; 256 + HEADER_LEN];
        let mut counter = 0u8;

        // forward messages to the other keyboard half
        loop {
            let message = select_biased! {
                event = events_rx.recv().fuse() => {
                    if event.source != this_half {
                        continue; // do not forward messages from the other half back to it
                    }
                    Message::KeyboardEvent(event)
                }
                event = UART_USB_EVENTS_OUT.receive().fuse() => Message::UsbEvent(event),
            };

            let (buf_header, body) = buf.split_array_mut();
            let serialized = match postcard::to_slice(&message, body) {
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

            let mut crc = CRCu16::crc16();
            crc.digest(&[len, random]);
            let header = Header {
                len: serialized.len() as u8,
                random,
                crc: crc.get_crc().to_le_bytes(),
            };
            let header: [u8; HEADER_LEN] = cast(header);
            *buf_header = header;

            let package = &buf[..HEADER_LEN + usize::from(len)];
            tx.write_all(package).await.ok();
        }
    };

    select_biased! {
        _ = tx_task.fuse() => log::error!("uart tx_task exited"),
        _ = rx_task.fuse() => log::error!("eart rx_task exited"),
    }
}
