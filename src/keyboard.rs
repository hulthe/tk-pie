use core::sync::atomic::{AtomicU16, Ordering};

use alloc::{boxed::Box, vec::Vec};
use embassy_executor::Spawner;
use embassy_rp::gpio::{AnyPin, Input, Pin, Pull};
use embassy_time::{Duration, Timer};
use futures::{select_biased, FutureExt};
use log::{debug, error, info, warn};
use tgnt::{button::Button, layer::Layer};

use crate::usb::keyboard::KB_REPORT;

static CURRENT_LAYER: AtomicU16 = AtomicU16::new(0);

pub struct KeyboardConfig {
    pub pins: [AnyPin; SWITCH_COUNT],
    pub layers: Vec<Layer>,
}

impl KeyboardConfig {
    pub async fn create(self) {
        let spawner = Spawner::for_current_executor().await;

        if self.layers.is_empty() {
            error!("no layers defined");
            return;
        }

        info!(
            "setting up keyboard layout with {} layer(s)",
            self.layers.len()
        );

        let layers = Box::leak(self.layers.into_boxed_slice());
        for (i, layer) in layers.iter().enumerate() {
            if layer.buttons.len() != SWITCH_COUNT {
                warn!(
                    "layer {i} defines {} buttons, but there are {SWITCH_COUNT} switches",
                    layer.buttons.len(),
                )
            }
        }

        for (i, pin) in self.pins.into_iter().enumerate() {
            if spawner.spawn(switch_task(i, pin, layers)).is_err() {
                error!("failed to spawn switch task, pool_size mismatch?");
                break;
            }
        }
    }
}

const MOD_TAP_TIME: Duration = Duration::from_millis(100);
const SWITCH_COUNT: usize = 18;

/// Task for monitoring a single switch pin, and handling button presses.
#[embassy_executor::task(pool_size = 18)]
async fn switch_task(switch_num: usize, pin: AnyPin, layers: &'static [Layer]) -> ! {
    let _pin_nr = pin.pin();
    let mut pin = Input::new(pin, Pull::Up);
    loop {
        // pins are pull-up, so when the switch is pressed they are brought low.
        pin.wait_for_low().await;

        // TODO: do we need debouncing?

        // get current layer
        let mut current_layer = CURRENT_LAYER.load(Ordering::Relaxed);
        let layer_count = layers.len() as u16;
        if current_layer >= layer_count {
            error!("current layer was out of bounds for some reason ({current_layer})");
            current_layer = 0;
        }
        let Some(Layer { buttons }) = layers.get(usize::from(current_layer)) else {
            error!("current layer was out of bounds for some reason ({current_layer})");
            CURRENT_LAYER.store(0, Ordering::Relaxed);
            continue;
        };

        // and current button
        let Some(button) = buttons.get(switch_num) else {
            warn!("no button defined for switch {switch_num}");
            continue;
        };

        debug!("switch {switch_num} button {button:?} pressed");

        let wait_for_release = async {
            pin.wait_for_high().await;
            debug!("switch {switch_num} button {button:?} released");
        };

        match button {
            &Button::Key(key) => {
                KB_REPORT.lock().await.press_key(key);
                wait_for_release.await;
                KB_REPORT.lock().await.release_key(key);
                continue;
            }
            Button::Mod(modifier) => {
                // TODO
                //KB_REPORT.lock().await.press_mod(modifier);
                //pin.wait_for_high().await;
                //KB_REPORT.lock().await.release_mod(modifier);
                //continue;
            }
            Button::ModTap { keycode, modifier } => {
                select_biased! {
                    _ = Timer::after(MOD_TAP_TIME).fuse() => {
                        // TODO: Modifier
                        pin.wait_for_high().await;
                        continue;
                    }
                    _ = wait_for_release.fuse() => {
                        KB_REPORT.lock().await.press_key(*keycode);
                        Timer::after(Duration::from_millis(10)).await;
                        KB_REPORT.lock().await.release_key(*keycode);
                        continue;
                    }
                }
            }
            Button::NextLayer => {
                let next_layer = (current_layer + 1) % layer_count;
                CURRENT_LAYER.store(next_layer, Ordering::Relaxed);
            }
            Button::PrevLayer => {
                let prev_layer = current_layer.checked_sub(1).unwrap_or(layer_count - 1);
                CURRENT_LAYER.store(prev_layer, Ordering::Relaxed);
            }
            Button::None => {}
        }

        wait_for_release.await;
    }
}

/// Random functions for testing
#[allow(dead_code)]
pub mod test {
    use tgnt::{button::Button, keys::Key};

    pub fn letter_to_key(c: char) -> Button {
        if !c.is_ascii() {
            return Button::None;
        }

        let c = c.to_ascii_uppercase();

        let key = match c {
            'A' => Key::A,
            'B' => Key::B,
            'C' => Key::C,
            'D' => Key::D,
            'E' => Key::E,
            'F' => Key::F,
            'G' => Key::G,
            'H' => Key::H,
            'I' => Key::I,
            'J' => Key::J,
            'K' => Key::K,
            'L' => Key::L,
            'M' => Key::M,
            'N' => Key::N,
            'O' => Key::O,
            'P' => Key::P,
            'Q' => Key::Q,
            'R' => Key::R,
            'S' => Key::S,
            'T' => Key::T,
            'U' => Key::U,
            'V' => Key::V,
            'W' => Key::W,
            'X' => Key::X,
            'Y' => Key::Y,
            'Z' => Key::Z,
            ' ' => Key::Space,
            '\n' => Key::Return,
            _ => {
                log::info!("char {c:?} -> None");
                return Button::None;
            }
        };

        log::info!("char {c:?} -> {key:?}");

        Button::Key(key)
    }
}
