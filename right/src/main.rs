//! Firmware for Tangentbord1, right half.

// NOTE: the order of attributes matters here..
#![no_main]
#![cfg(target_os = "none")] // only try to compile this for embedded
#![no_std]
#![feature(type_alias_impl_trait)]

extern crate alloc;
extern crate cortex_m_rt;

use alloc::vec::Vec;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_time::Timer;
use log::error;
use tk_pie::{
    board::Board,
    event::Half,
    interrupts::Irqs,
    keyboard::KeyboardConfig,
    layer::Layer,
    logger::LogMultiplexer,
    rgb::Rgb,
    util::stall,
    ws2812::Ws2812,
    {allocator, rtt, uart, usb},
};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let half = Half::Right;

    let rtt_logger = rtt::init_rtt_logger();

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
            board.d12.degrade(),
            board.d11.degrade(),
            board.d10.degrade(),
            board.d9.degrade(),
            board.d7.degrade(),
            // row 2
            board.a2.degrade(),
            board.a1.degrade(),
            board.a0.degrade(),
            board.sda.degrade(),
            board.scl.degrade(),
            // row 3
            board.mosi.degrade(),
            board.sck.degrade(),
            board.d25.degrade(),
            board.d24.degrade(),
            board.a3.degrade(),
            // thumbpad
            board.d3.degrade(),
            board.d2.degrade(),
            board.miso.degrade(),
        ],
        led_map: [0, 1, 2, 3, 4, 9, 8, 7, 6, 5, 10, 11, 12, 13, 14, 15, 16, 17],
        led_driver: neopixels_d5,
        layers,
    };

    let Some([events1, events2]) = keyboard.create().await else {
        error!("failed to create keyboard");
        return;
    };

    uart::start(board.tx, board.rx, board.UART0, half, events2).await;

    // TODO: delaying the logger until here is not ideal
    let usb_logger = usb::setup_logger_and_keyboard(board.USB, events1).await;

    let logger = LogMultiplexer {
        outputs: [rtt_logger, usb_logger],
    };
    logger.init();

    log::error!("log_level: error");
    log::warn!("log_level: warn");
    log::info!("log_level: info");
    log::debug!("log_level: debug");
    log::trace!("log_level: trace");

    neopixel.write(&[Rgb::new(0x00, 0x00, 0xFF)]).await;
    Timer::after_secs(5).await;

    for b in (0u8..0xff).rev() {
        neopixel.write(&[Rgb::new(0, 0, b)]).await;
        Timer::after_millis(10).await;
    }

    stall().await
}
