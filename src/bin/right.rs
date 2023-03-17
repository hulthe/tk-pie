//! Firmware for Tangentbord1, right half.

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;
extern crate cortex_m_rt;

use alloc::vec::Vec;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_time::{Duration, Timer};
use tangentbord1::board::Board;
use tangentbord1::keyboard::KeyboardConfig;
use tangentbord1::util::{stall, wheel};
use tangentbord1::ws2812::{Rgb, Ws2812};
use tangentbord1::{allocator, usb};
use tgnt::layer::Layer;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    allocator::init();

    let p = embassy_rp::init(Default::default());
    let board = Board::from(p);

    let _led = Output::new(board.d13, Level::High);
    let _neopixel_power = Output::new(board.neopixel_power, Level::High);

    let mut neopixel = Ws2812::new(board.PIO0, board.DMA_CH0, board.neopixel.degrade());
    let mut neopixels_d5 = Ws2812::new(board.PIO1, board.DMA_CH1, board.d5.degrade());

    neopixel.write(&[Rgb::new(0xFF, 0x00, 0x00)]).await;
    usb::setup_logger_and_keyboard(board.USB).await;
    neopixel.write(&[Rgb::new(0x00, 0x00, 0xFF)]).await;

    //Timer::after(Duration::from_millis(3000)).await;

    let layers = include_bytes!("layers-right.pc");
    let Ok(layers): Result<Vec<Layer>, _> = postcard::from_bytes(layers) else {
        log::error!("Failed to deserialize layer config");
        stall().await
    };

    let keyboard = KeyboardConfig {
        layers,
        pins: [
            // TODO: reconfigure these for right PCB
            // row 1
            board.d24.degrade(),
            board.a3.degrade(),
            board.a2.degrade(),
            board.a1.degrade(),
            board.a0.degrade(),
            // row 2
            board.d25.degrade(),
            board.sck.degrade(),
            board.mosi.degrade(),
            board.miso.degrade(),
            board.d2.degrade(),
            // row 3
            board.d12.degrade(),
            board.d11.degrade(),
            board.d10.degrade(),
            board.d9.degrade(),
            board.d3.degrade(),
            // thumbpad
            board.d7.degrade(),
            board.scl.degrade(),
            board.sda.degrade(),
        ],
    };

    keyboard.create().await;

    for w in 0usize.. {
        neopixel.write(&[wheel(w as u8)]).await;
        neopixels_d5
            .write(&[
                wheel((w + 50) as u8),
                wheel((w + 100) as u8),
                wheel((w + 150) as u8),
                wheel((w + 200) as u8),
            ])
            .await;
        Timer::after(Duration::from_millis(10)).await;
    }
}
