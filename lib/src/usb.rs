use embassy_sync::pubsub::PubSubChannel;
use serde::{Deserialize, Serialize};

use crate::util::CS;

#[cfg(target_arch = "arm")]
pub mod driver;
#[cfg(target_arch = "arm")]
pub mod keyboard;
#[cfg(target_arch = "arm")]
pub mod serial;

pub const MAX_PACKET_SIZE: u8 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsbEvent {
    Enabled(bool),
    Suspended(bool),
    Configured(bool),
    Addressed(u8),
    Reset,
}

pub type UsbEventChannel = PubSubChannel<CS, UsbEvent, 8, 24, 0>;
pub static USB_EVENTS: UsbEventChannel = UsbEventChannel::new();
