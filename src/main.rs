//! # GPIO 'Blinky' Example
//!
//! Blinks the LED on a Adafruit itsy-bitsy RP2040 board
//!
//! It may need to be adapted to your particular board layout and/or pin assignment.
//!
//! See the `Cargo.toml` file for Copyright and license details.

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;
extern crate cortex_m_rt;

mod allocator;
mod board;
mod keyboard;
mod neopixel;
mod panic_handler;
mod usb;
mod util;
mod ws2812;

use alloc::vec::Vec;
use board::Board;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_time::{Duration, Timer};
use tgnt::layer::Layer;
use ws2812::Rgb;

use crate::keyboard::KeyboardConfig;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    allocator::init();

    let p = embassy_rp::init(Default::default());

    let board = Board {
        a0: p.PIN_26,
        a1: p.PIN_27,
        a2: p.PIN_28,
        a3: p.PIN_29,
        d24: p.PIN_24,
        d25: p.PIN_25,
        sck: p.PIN_18,
        mosi: p.PIN_19,
        miso: p.PIN_20,
        d2: p.PIN_12,
        d3: p.PIN_5,
        d4: p.PIN_4,
        rx: p.PIN_1,
        tx: p.PIN_0,
        sda: p.PIN_2,
        scl: p.PIN_3,
        d5: p.PIN_14,
        d7: p.PIN_6,
        d9: p.PIN_7,
        d10: p.PIN_8,
        d11: p.PIN_9,
        d12: p.PIN_10,
        d13: p.PIN_11,
        neopixel: p.PIN_17,
        neopixel_power: p.PIN_16,
    };

    //let mut led = Output::new(board.d13, Level::Low);
    let _neopixel_power = Output::new(board.neopixel_power, Level::High);

    let mut neopixel = ws2812::Ws2812::new(p.PIO0, p.DMA_CH0, board.neopixel.degrade());
    let mut neopixels_d5 = ws2812::Ws2812::new(p.PIO1, p.DMA_CH1, board.d5.degrade());

    neopixel.write(&[Rgb::new(0xb7, 0x31, 0x2c)]).await;

    let mut builder = usb::builder(p.USB);

    usb::logger::setup(&mut builder).await;

    neopixel.write(&[Rgb::new(0xf0, 0xd0, 0x20)]).await;

    log::error!("log_level: error");
    log::warn!("log_level: warn");
    log::info!("log_level: info");
    log::debug!("log_level: debug");
    log::trace!("log_level: trace");

    usb::keyboard::setup(&mut builder).await;

    let usb = builder.build();

    spawner.must_spawn(usb::run(usb));

    Timer::after(Duration::from_millis(3000)).await;

    let layers = include_bytes!("layers.pc");
    let Ok(layers): Result<Vec<Layer>, _> = postcard::from_bytes(layers) else {
        log::error!("Failed to deserialize layer config");
        loop_forever().await
    };

    let keyboard = KeyboardConfig {
        layers,
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

async fn loop_forever() -> ! {
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

/// Input a value 0 to 255 to get a color value
// The colours are a transition r - g - b - back to r.
fn wheel(mut wheel_pos: u8) -> Rgb {
    wheel_pos = 255 - wheel_pos;
    if wheel_pos < 85 {
        return Rgb::new(255 - wheel_pos * 3, 0, wheel_pos * 3);
    }
    if wheel_pos < 170 {
        wheel_pos -= 85;
        return Rgb::new(0, wheel_pos * 3, 255 - wheel_pos * 3);
    }
    wheel_pos -= 170;
    Rgb::new(wheel_pos * 3, 255 - wheel_pos * 3, 0)
}
