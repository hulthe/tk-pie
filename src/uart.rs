use core::mem::{size_of, transmute};

use embassy_executor::Spawner;
use embassy_rp::peripherals::{PIN_0, PIN_1, UART0};
use embassy_rp::uart::{self, BufferedUart, DataBits, Parity, StopBits};
use embassy_time::{with_timeout, Duration, TimeoutError};
use embedded_io::asynch::{Read, Write};
use futures::{select_biased, FutureExt};
use log::{error, info};
use static_cell::StaticCell;

use crate::keyboard::{self, Half, KbEvents};
use crate::Irqs;

#[derive(Clone, Debug)]
enum Message {
    KeyboardEvent(keyboard::Event),
}

pub async fn start(tx: PIN_0, rx: PIN_1, uart: UART0, board: Half, events: KbEvents) {
    static TX_BUF: StaticCell<[u8; 1024]> = StaticCell::new();
    static RX_BUF: StaticCell<[u8; 1024]> = StaticCell::new();

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
        TX_BUF.init_with(|| [0u8; 1024]),
        RX_BUF.init_with(|| [0u8; 1024]),
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

    let rx_task = async {
        loop {
            let mut buf = [0u8; size_of::<Message>()];
            // TODO: this timout thing seems sketchy
            match with_timeout(Duration::from_millis(5), rx.read_exact(&mut buf)).await {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    error!("uart error: {:?}", e);
                    continue;
                }
                Err(TimeoutError) => continue,
            };

            let message: Message = unsafe { transmute(buf) }; // crimes :)
            match &message {
                Message::KeyboardEvent(event) => events_tx.send(event.clone()),
            }
            info!("got msg: {:?}", message);
        }
    };

    let tx_task = async {
        loop {
            // forward messages to the other keyboard half
            let event = events_rx.recv().await;
            if event.source != this_half {
                continue; // do not forward messages from the other half back to it
            }
            let message = Message::KeyboardEvent(event);
            let message: [u8; size_of::<Message>()] = unsafe { transmute(message) }; // crimes :)
            tx.write_all(&message).await.ok();
        }
    };

    select_biased! {
        _ = tx_task.fuse() => error!("uart tx_task exited"),
        _ = rx_task.fuse() => error!("eart rx_task exited"),
    }
}
