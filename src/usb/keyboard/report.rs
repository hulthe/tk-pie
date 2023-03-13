#![allow(dead_code)]

/// KeyboardReport describes a report and its companion descriptor that can be
/// used to send keyboard button presses to a host and receive the status of the
/// keyboard LEDs.
///
/// Unlike usbd_hids KeyboardReport, this one supports N-key rollover.
#[derive(PartialEq, Eq)]
#[cfg(feature = "n-key-rollover")]
pub struct KeyboardReport {
    pub modifier: u8,

    /// Bitmap representing all keycodes from 0 to 104
    pub keycodes: [u8; 13],
}

#[cfg(not(feature = "n-key-rollover"))]
pub use usbd_hid::descriptor::KeyboardReport;

#[cfg(feature = "n-key-rollover")]
pub const EMPTY_KEYBOARD_REPORT: KeyboardReport = KeyboardReport {
    modifier: 0,
    keycodes: [0; 13],
};

#[cfg(not(feature = "n-key-rollover"))]
pub const EMPTY_KEYBOARD_REPORT: KeyboardReport = KeyboardReport {
    modifier: 0,
    leds: 0,
    reserved: 0,
    keycodes: [0; 6],
};

#[cfg(feature = "n-key-rollover")]
impl KeyboardReport {
    pub fn set_key(&mut self, keycode: u8) {
        log::info!("setting keycode: {keycode}");
        let byte = keycode >> 3;
        let bit = keycode & 0b111;
        let mask = 1 << bit;

        if let Some(k) = self.keycodes.get_mut(byte as usize) {
            *k |= mask;
        } else {
            log::warn!("Tried to set out-of-range keycode: {keycode:x}");
        }
    }

    pub fn serialized(&self) -> [u8; 14] {
        let [a, b, c, d, e, f, g, h, i, j, k, l, m] = self.keycodes;
        [self.modifier, a, b, c, d, e, f, g, h, i, j, k, l, m]
    }
}

// bitmasks for the `modifier` field
pub const MOD_LCTRL: u8 = 0x01;
pub const MOD_LSHIFT: u8 = 0x02;
pub const MOD_LALT: u8 = 0x04;
pub const MOD_LSUPER: u8 = 0x08;
pub const MOD_RCTRL: u8 = 0x10;
pub const MOD_RSHIFT: u8 = 0x20;
pub const MOD_RALT: u8 = 0x40;
pub const MOD_RSUPER: u8 = 0x80;

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
            0x29, 0x67, // local usage maximum
            0x95, 0x68, // report count
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
