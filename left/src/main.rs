//! Firmware for Tangentbord1, left half.

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![cfg(target_arch = "arm")]

extern crate alloc;
extern crate cortex_m_rt;

use alloc::vec::Vec;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_time::{Duration, Timer};
use log::error;
use tangentbord1::{
    board::Board,
    event::Half,
    interrupts::Irqs,
    keyboard::KeyboardConfig,
    logger::Logger,
    rgb::Rgb,
    util::{stall, wheel},
    ws2812::Ws2812,
    {allocator, rtt, uart, usb},
};
use tgnt::layer::Layer;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let half = Half::Left;

    let rtt_write = rtt::init_rtt_logger();
    let logger = Logger {
        outputs: [rtt_write],
    };
    logger.init();

    log::error!("log_level: error");
    log::warn!("log_level: warn");
    log::info!("log_level: info");
    log::debug!("log_level: debug");
    log::trace!("log_level: trace");

    allocator::init();

    let p = embassy_rp::init(Default::default());
    let board = Board::from(p);

    let _neopixel_power = Output::new(board.neopixel_power, Level::High);

    let mut neopixel = Ws2812::new(board.PIO0, Irqs, board.DMA_CH0, board.neopixel);
    let neopixels_d5 = Ws2812::new(board.PIO1, Irqs, board.DMA_CH1, board.d5);

    neopixel.write(&[Rgb::new(0xFF, 0x00, 0x00)]).await;

    let layers = include_bytes!("layers.pc");
    let Ok(layers): Result<Vec<Vec<Layer>>, _> = postcard::from_bytes(layers) else {
        log::error!("Failed to deserialize layer config");
        stall().await
    };

    let keyboard = KeyboardConfig {
        half,
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

    let Some([events1, events2]) = keyboard.create().await else {
        error!("failed to create keyboard");
        return;
    };

    uart::start(board.tx, board.rx, board.UART0, half, events2).await;

    neopixel.write(&[Rgb::new(0x00, 0x99, 0x99)]).await;

    usb::setup_logger_and_keyboard(board.USB, events1).await;
    neopixel.write(&[Rgb::new(0x00, 0x00, 0xFF)]).await;

    for w in 0usize.. {
        neopixel.write(&[wheel(w as u8) * 0.15]).await;
        Timer::after(Duration::from_millis(10)).await;
    }
}
