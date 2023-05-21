use crc_any::CRCu16;
use embassy_executor::Spawner;
use embassy_rp::peripherals::{PIN_0, PIN_1, UART0};
use embassy_rp::uart::{self, BufferedUart, DataBits, Parity, StopBits};
use embedded_io::asynch::{Read, Write};
use futures::{select_biased, FutureExt};
use heapless::Vec;
use serde::{Deserialize, Serialize};
use static_cell::StaticCell;

use crate::keyboard::{self, Half, KbEvents};
use crate::Irqs;

#[derive(Clone, Debug, Serialize, Deserialize)]
enum Message {
    KeyboardEvent(keyboard::Event),
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

    const HEADER_LEN: usize = 4;

    let rx_task = async {
        let mut buf: heapless::Vec<u8, 1024> = Vec::new();
        loop {
            if let &[len, random, crc1, crc2, ref rest @ ..] = buf.as_slice() {
                let crc = u16::from_le_bytes([crc1, crc2]);
                let mut calculated_crc = CRCu16::crc16();
                calculated_crc.digest(&[len, random]);
                let calculated_crc = calculated_crc.get_crc();

                if calculated_crc != crc {
                    log::error!("invalid uart package crc: {:x?}", &buf[..HEADER_LEN]);
                    buf.remove(0); // pop the first byte and hope we find a good packet header
                    continue;
                }

                log::debug!("got uart header {:x?}", &buf[..HEADER_LEN]);

                let len = usize::from(len);
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
        let mut counter = 1u8;
        loop {
            // forward messages to the other keyboard half
            let event = events_rx.recv().await;
            if event.source != this_half {
                continue; // do not forward messages from the other half back to it
            }

            let message = Message::KeyboardEvent(event);
            let (header, body) = buf.split_at_mut(HEADER_LEN);
            let serialized = match postcard::to_slice(&message, body) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("failed to serialize uart message: {e}");
                    continue;
                }
            };

            // add a "random" value to feed the crc
            let random = counter;
            counter = counter.wrapping_add(1);

            let len = serialized.len() as u8;

            let mut crc = CRCu16::crc16();
            crc.digest(&[len, random]);
            let [crc1, crc2] = crc.get_crc().to_le_bytes();

            header.copy_from_slice(&[len, random, crc1, crc2]);

            let package = &buf[..HEADER_LEN + usize::from(len)];
            tx.write_all(package).await.ok();
        }
    };

    select_biased! {
        _ = tx_task.fuse() => log::error!("uart tx_task exited"),
        _ = rx_task.fuse() => log::error!("eart rx_task exited"),
    }
}
