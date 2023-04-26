//! Firmware for Tangentbord1, left half.

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
    let neopixels_d5 = Ws2812::new(board.PIO1, board.DMA_CH1, board.d5.degrade());

    neopixel.write(&[Rgb::new(0xFF, 0x00, 0x00)]).await;
    usb::setup_logger_and_keyboard(board.USB).await;
    neopixel.write(&[Rgb::new(0x00, 0x00, 0xFF)]).await;

    //Timer::after(Duration::from_millis(3000)).await;

    let layers = include_bytes!("layers-left.pc");
    let Ok(layers): Result<Vec<Layer>, _> = postcard::from_bytes(layers) else {
        log::error!("Failed to deserialize layer config");
        stall().await
    };

    let keyboard = KeyboardConfig {
        pins: [
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
        // the index of the LEDs is different than the switch index.
        // each number is the index of the LED for the switch of that index.
        led_map: [4, 3, 2, 1, 0, 5, 6, 7, 8, 9, 14, 13, 12, 11, 10, 15, 16, 17],
        led_driver: neopixels_d5,
        layers,
    };

    keyboard.create().await;

    for w in 0usize.. {
        neopixel.write(&[wheel(w as u8)]).await;
        Timer::after(Duration::from_millis(10)).await;
    }
}
