#![no_std]
#![feature(type_alias_impl_trait)]

use embassy_rp::{
    bind_interrupts,
    peripherals::{UART0, USB},
};

extern crate alloc;

pub mod allocator;
pub mod board;
pub mod keyboard;
pub mod lights;
pub mod logger;
pub mod neopixel;
pub mod panic_handler;
pub mod rtt;
pub mod uart;
pub mod usb;
pub mod util;
pub mod ws2812;

bind_interrupts! {
    pub struct Irqs {
        UART0_IRQ => embassy_rp::uart::BufferedInterruptHandler<UART0>;
        USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
    }
}
