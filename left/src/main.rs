//! Firmware for Tangentbord1, left half.

// NOTE: the order of attributes matters here..
#![no_main]
#![cfg(target_os = "none")] // only try to compile this for embedded
#![no_std]
#![feature(type_alias_impl_trait)]

extern crate alloc;
extern crate cortex_m_rt;

use alloc::vec::Vec;
use cortex_m_rt::entry;
use embassy_rp::gpio::Pin;
use tk_pie::{
    board::MappedBoard, event::Half, interrupts::Irqs, keyboard::KeyboardConfig, layer::Layer,
    ws2812::Ws2812,
};

#[entry]
fn main() -> ! {
    tk_pie::run(|board| {
        let layers = include_bytes!("layers.pc");
        let Ok(layers): Result<Vec<Vec<Layer>>, _> = postcard::from_bytes(layers) else {
            log::error!("Failed to deserialize layer config");
            loop {}
        };

        let keyboard = KeyboardConfig {
            half: Half::Left,
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
            led_driver: Ws2812::new(board.PIO1, Irqs, board.DMA_CH1, board.d5),
            layers,
        };

        let neopixel = Ws2812::new(board.PIO0, Irqs, board.DMA_CH0, board.neopixel);

        MappedBoard {
            USB: board.USB,
            UART0: board.UART0,
            UART1: board.UART1,

            keyboard,

            rx: board.rx,
            tx: board.tx,

            neopixel,
            neopixel_power: board.neopixel_power,
        }
    })
}
