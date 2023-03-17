#![no_std]
#![feature(type_alias_impl_trait)]

extern crate alloc;

pub mod allocator;
pub mod board;
pub mod keyboard;
pub mod neopixel;
pub mod panic_handler;
pub mod usb;
pub mod util;
pub mod ws2812;
