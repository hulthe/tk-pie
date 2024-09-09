#![cfg_attr(not(feature = "std"), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(split_array)]

extern crate alloc;
extern crate cortex_m_rt;

#[cfg(target_arch = "arm")]
pub mod allocator;
#[cfg(target_arch = "arm")]
pub mod board;
#[cfg(target_arch = "arm")]
pub mod interrupts;
#[cfg(target_arch = "arm")]
pub mod keyboard;
#[cfg(target_arch = "arm")]
pub mod panic_handler;
#[cfg(target_arch = "arm")]
pub mod ws2812;

pub mod atomics;
pub mod button;
pub mod event;
pub mod hemicom;
pub mod keypress_handler;
pub mod keys;
pub mod layer;
pub mod layout;
pub mod lights;
pub mod logger;
pub mod rgb;
pub mod rtt;
pub mod serial_proto;
pub mod usb;
pub mod util;

#[cfg(target_arch = "arm")]
mod entry;
#[cfg(target_arch = "arm")]
pub use entry::*;
