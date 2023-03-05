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

extern crate cortex_m_rt;
extern crate panic_halt;

mod board;
mod keyboard;
mod usb;

use board::Board;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_time::{Duration, Timer};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
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

    let mut led = Output::new(board.d13, Level::High);

    let mut builder = usb::builder(p.USB);

    usb::logger::setup(&mut builder).await;

    log::error!("log_level: error");
    log::warn!("log_level: warn");
    log::info!("log_level: info");
    log::debug!("log_level: debug");
    log::trace!("log_level: trace");

    usb::keyboard::setup(&mut builder).await;

    let usb = builder.build();

    spawner.must_spawn(usb::run(usb));

    Timer::after(Duration::from_millis(1000)).await;

    crate::keyboard::test_type("Hello there!\n").await;

    loop {
        Timer::after(Duration::from_millis(500)).await;
        led.toggle();
    }
}
