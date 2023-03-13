use core::sync::atomic::{AtomicBool, Ordering};

use embassy_rp::gpio::{AnyPin, Input, Pin, Pull};
use embassy_time::{Duration, Timer};
use log::info;

pub const ROWS: usize = 3;
pub const COLS: usize = 5;
const NEW_SWITCH: Switch = Switch::new();
const NEW_ROW: [Switch; COLS] = [NEW_SWITCH; COLS];
pub static MATRIX: [[Switch; COLS]; ROWS] = [NEW_ROW; ROWS];

pub static TEST_KEYMAP: [[Button; COLS]; ROWS] = [
    [
        Button::KEY_A,
        Button::KEY_B,
        Button::KEY_C,
        Button::KEY_D,
        Button::KEY_E,
    ],
    [
        Button::KEY_T,
        Button::KEY_R,
        Button::KEY_H,
        Button::KEY_I,
        Button::KEY_SPACE,
    ],
    [
        Button::KEY_O,
        Button::KEY_L,
        Button::KEY_LSHIFT,
        Button::KEY_LCTRL,
        Button::KEY_LALT,
    ],
];

pub struct Switch {
    state: AtomicBool,
}

#[allow(dead_code)]
impl Switch {
    pub const fn new() -> Self {
        Switch {
            state: AtomicBool::new(false),
        }
    }

    pub fn press(&self) {
        self.state.store(true, Ordering::Relaxed);
    }

    pub fn release(&self) {
        self.state.store(false, Ordering::Relaxed);
    }

    pub fn is_pressed(&self) -> bool {
        self.state.load(Ordering::Relaxed)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Button {
    Key { keycode: u8 },
}

#[embassy_executor::task(pool_size = 20)]
pub async fn monitor_switch(pin: AnyPin) -> ! {
    let pin_nr = pin.pin();
    let mut pin = Input::new(pin, Pull::Up);
    loop {
        pin.wait_for_low().await;
        info!("pin {pin_nr} low");
        pin.wait_for_high().await;
        info!("pin {pin_nr} high");
    }
}

#[allow(dead_code)]
impl Button {
    pub const fn key(keycode: u8) -> Self {
        Button::Key { keycode }
    }

    // https://usb.org/sites/default/files/hut1_3_0.pdf
    pub const KEY_A: Button = Button::key(0x04);
    pub const KEY_B: Button = Button::key(0x05);
    pub const KEY_C: Button = Button::key(0x06);
    pub const KEY_D: Button = Button::key(0x07);
    pub const KEY_E: Button = Button::key(0x08);
    pub const KEY_F: Button = Button::key(0x09);
    pub const KEY_G: Button = Button::key(0x0A);
    pub const KEY_H: Button = Button::key(0x0B);
    pub const KEY_I: Button = Button::key(0x0C);
    pub const KEY_J: Button = Button::key(0x0D);
    pub const KEY_K: Button = Button::key(0x0E);
    pub const KEY_L: Button = Button::key(0x0F);
    pub const KEY_M: Button = Button::key(0x10);
    pub const KEY_N: Button = Button::key(0x11);
    pub const KEY_O: Button = Button::key(0x12);
    pub const KEY_P: Button = Button::key(0x13);
    pub const KEY_Q: Button = Button::key(0x14);
    pub const KEY_R: Button = Button::key(0x15);
    pub const KEY_S: Button = Button::key(0x16);
    pub const KEY_T: Button = Button::key(0x17);
    pub const KEY_U: Button = Button::key(0x18);
    pub const KEY_V: Button = Button::key(0x19);
    pub const KEY_W: Button = Button::key(0x1A);
    pub const KEY_X: Button = Button::key(0x1B);
    pub const KEY_Y: Button = Button::key(0x1C);
    pub const KEY_Z: Button = Button::key(0x1D);

    pub const KEY_1: Button = Button::key(0x1E);
    pub const KEY_2: Button = Button::key(0x1F);
    pub const KEY_3: Button = Button::key(0x20);
    pub const KEY_4: Button = Button::key(0x21);
    pub const KEY_5: Button = Button::key(0x22);
    pub const KEY_6: Button = Button::key(0x23);
    pub const KEY_7: Button = Button::key(0x24);
    pub const KEY_8: Button = Button::key(0x25);
    pub const KEY_9: Button = Button::key(0x26);
    pub const KEY_0: Button = Button::key(0x27);

    pub const KEY_SPACE: Button = Button::key(0x2C);
    pub const KEY_RETURN: Button = Button::key(0x28);

    pub const KEY_LCTRL: Button = Button::key(0xE0);
    pub const KEY_RCTRL: Button = Button::key(0xE4);

    pub const KEY_LSHIFT: Button = Button::key(0xE1);
    pub const KEY_RSHIFT: Button = Button::key(0xE5);

    pub const KEY_LALT: Button = Button::key(0xE2);
    pub const KEY_RALT: Button = Button::key(0xE6);
}

/// Random functions for testing
#[allow(dead_code)]
pub mod test {
    use super::*;

    pub fn letter_to_key(c: char) -> Option<Button> {
        if !c.is_ascii() {
            return None;
        }

        let c = c.to_ascii_uppercase();

        let key = match c {
            'A' => Button::KEY_A,
            'B' => Button::KEY_B,
            'C' => Button::KEY_C,
            'D' => Button::KEY_D,
            'E' => Button::KEY_E,
            'F' => Button::KEY_F,
            'G' => Button::KEY_G,
            'H' => Button::KEY_H,
            'I' => Button::KEY_I,
            'J' => Button::KEY_J,
            'K' => Button::KEY_K,
            'L' => Button::KEY_L,
            'M' => Button::KEY_M,
            'N' => Button::KEY_N,
            'O' => Button::KEY_O,
            'P' => Button::KEY_P,
            'Q' => Button::KEY_Q,
            'R' => Button::KEY_R,
            'S' => Button::KEY_S,
            'T' => Button::KEY_T,
            'U' => Button::KEY_U,
            'V' => Button::KEY_V,
            'W' => Button::KEY_W,
            'X' => Button::KEY_X,
            'Y' => Button::KEY_Y,
            'Z' => Button::KEY_Z,
            ' ' => Button::KEY_SPACE,
            '\n' => Button::KEY_RETURN,
            _ => {
                log::info!("char {c:?} -> None");
                return None;
            }
        };

        log::info!("char {c:?} -> {key:?}");

        Some(key)
    }

    pub async fn type_string(s: &str) {
        log::info!("typing {s:?}");

        for c in s.chars() {
            let Some(key) = letter_to_key(c) else {
            continue;
        };

            for col in 0..COLS {
                for row in 0..ROWS {
                    if TEST_KEYMAP[row][col] == key {
                        MATRIX[row][col].press();
                        Timer::after(Duration::from_millis(20)).await;
                        MATRIX[row][col].release();
                        Timer::after(Duration::from_millis(5)).await;
                    }
                }
            }
        }
    }

    /// Press all keys at once
    pub async fn rollover<const N: usize>(letters: [char; N]) {
        log::info!("pressing all letters in {letters:?}");

        let keys = letters.map(letter_to_key);

        for key in &keys {
            for col in 0..COLS {
                for row in 0..ROWS {
                    if Some(&TEST_KEYMAP[row][col]) == key.as_ref() {
                        MATRIX[row][col].press();
                    }
                }
            }
        }

        Timer::after(Duration::from_millis(200)).await;

        for key in &keys {
            for col in 0..COLS {
                for row in 0..ROWS {
                    if Some(&TEST_KEYMAP[row][col]) == key.as_ref() {
                        MATRIX[row][col].release();
                    }
                }
            }
        }
    }
}
