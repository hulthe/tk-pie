#![allow(dead_code)]

/// KeyboardReport describes a report and its companion descriptor that can be
/// used to send keyboard button presses to a host and receive the status of the
/// keyboard LEDs.
///
/// Unlike usbd_hids KeyboardReport, this one supports N-key rollover.
#[derive(Clone, Copy, PartialEq, Eq, Zeroable, Pod)]
#[cfg(feature = "n-key-rollover")]
#[repr(C, packed)]
pub struct KeyboardReport {
    pub modifier: u8,

    /// Bitmap representing all keycodes from 0 to 215
    pub keycodes: [u8; 27],
}

use bytemuck::{cast_ref, Pod, Zeroable};
use core::mem::size_of;
use tgnt::{button::Modifier, keys::Key};

#[cfg(not(feature = "n-key-rollover"))]
pub use ::usbd_hid::descriptor::KeyboardReport;

#[cfg(feature = "n-key-rollover")]
pub const EMPTY_KEYBOARD_REPORT: KeyboardReport = KeyboardReport {
    modifier: 0,
    keycodes: [0; 27],
};

#[cfg(not(feature = "n-key-rollover"))]
pub const EMPTY_KEYBOARD_REPORT: KeyboardReport = KeyboardReport {
    modifier: 0,
    leds: 0,
    reserved: 0,
    keycodes: [0; 6],
};

/// Get the byte index, and the mask for that byte, in the keycode bitmap.
fn key_to_byte_mask(key: Key) -> (usize, u8) {
    let keycode = u8::from(key);
    let byte = keycode >> 3;
    let bit = keycode & 0b111;
    let mask = 1 << bit;

    (usize::from(byte), mask)
}

#[cfg(feature = "n-key-rollover")]
impl KeyboardReport {
    #[inline(always)]
    pub fn set_key(&mut self, key: Key, pressed: bool) {
        let (byte, mask) = key_to_byte_mask(key);

        if let Some(k) = self.keycodes.get_mut(byte as usize) {
            if pressed {
                *k |= mask;
            } else {
                *k &= !mask;
            }
        } else {
            log::warn!("Tried to set out-of-range keycode: 0x{:x}", u8::from(key));
        }
    }

    #[inline(always)]
    pub fn press_key(&mut self, key: Key) {
        self.set_key(key, true)
    }

    #[inline(always)]
    pub fn release_key(&mut self, key: Key) {
        self.set_key(key, false)
    }

    #[inline(always)]
    pub fn key_pressed(&mut self, key: Key) -> bool {
        let (byte, mask) = key_to_byte_mask(key);
        if let Some(k) = self.keycodes.get_mut(byte as usize) {
            (*k & mask) != 0
        } else {
            log::warn!("Tried to get out-of-range keycode: 0x{:x}", u8::from(key));
            false
        }
    }

    #[inline(always)]
    pub fn set_modifier(&mut self, modifier: Modifier, pressed: bool) {
        if pressed {
            self.modifier |= u8::from(modifier);
        } else {
            self.modifier &= !u8::from(modifier);
        }
    }

    #[inline(always)]
    pub fn press_modifier(&mut self, modifier: Modifier) {
        self.set_modifier(modifier, true)
    }

    #[inline(always)]
    pub fn release_modifier(&mut self, modifier: Modifier) {
        self.set_modifier(modifier, false)
    }

    #[inline(always)]
    pub fn modifier_pressed(&mut self, modifier: Modifier) -> bool {
        (self.modifier & u8::from(modifier)) != 0
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8; size_of::<KeyboardReport>()] {
        cast_ref(self)
    }
}

// bitmasks for the `modifier` field

#[cfg(feature = "n-key-rollover")]
impl usbd_hid::descriptor::SerializedDescriptor for KeyboardReport {
    fn desc() -> &'static [u8] {
        // Manually define the descriptor since I can't figure out how to get
        // gen_hid_descriptor to generate the correct one.
        &[
            0x05, 0x01, // usage page 1 (generic desktop)
            0x09, 0x06, // usage (keyboard)
            0xa1, 0x01, // collection (application)
            //
            0x05, 0x07, // usage page 7 (keyboard/keypad)
            0x19, 0xe0, // local usage minimum
            0x29, 0xe7, // local usage maximum
            0x15, 0x00, // local minimum
            0x25, 0x01, // local maximum
            0x75, 0x01, // report size (1 bit)
            0x95, 0x08, // report count (8)
            0x81, 0x02, // input (variable)
            //
            0x19, 0x00, // local usage minimum
            //0x29, 0x67, // local usage maximum (0x67)
            //0x95, 0x68, // report count (0x68)
            0x29, 215, // local usage maximum (215)
            0x95, 216, // report count (216)
            0x81, 0x02, // input (variable)
            //
            0x05, 0x08, // usage page 8 (led page)
            0x19, 0x01, // local usage minimum
            0x29, 0x05, // local usage maximum
            0x15, 0x00, // logical min
            0x25, 0x01, // logical max
            0x75, 0x01, // report size
            0x95, 0x05, // report count
            0x91, 0x02, // output (variable)
            //
            0x75, 0x03, // report size
            0x95, 0x01, // report count
            0x91, 0x01, // output (constant)
            0xc0, // end collection
        ]
    }
}
