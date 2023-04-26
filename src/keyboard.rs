use core::sync::atomic::{AtomicU16, Ordering};

use alloc::{boxed::Box, vec::Vec};
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{AnyPin, Input, Pin, Pull},
    pio::PioInstanceBase,
};
use embassy_time::{Duration, Timer};
use futures::{select_biased, FutureExt};
use log::{debug, error, info, warn};
use static_cell::StaticCell;
use tgnt::{button::Button, layer::Layer};

use crate::{
    lights::Lights,
    usb::keyboard::KB_REPORT,
    ws2812::{Rgb, Ws2812},
};

pub struct KeyboardConfig {
    /// Array of input pins of each switch
    pub pins: [AnyPin; SWITCH_COUNT],
    /// Array of LED indices of each switch
    pub led_map: [usize; SWITCH_COUNT],
    pub led_driver: Ws2812<PioInstanceBase<1>>,
    pub layers: Vec<Layer>,
}

struct State {
    current_layer: AtomicU16,
    layers: &'static [Layer],
    /// Array of LED indices of each switch
    led_map: [usize; SWITCH_COUNT],
    lights: Lights<PioInstanceBase<1>, SWITCH_COUNT>,
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

        static STATE: StaticCell<State> = StaticCell::new();
        let state = STATE.init_with(|| State {
            current_layer: AtomicU16::new(0),
            layers: Box::leak(self.layers.into_boxed_slice()),
            lights: Lights::new(self.led_driver),
            led_map: self.led_map,
        });

        for (i, layer) in state.layers.iter().enumerate() {
            if layer.buttons.len() != SWITCH_COUNT {
                warn!(
                    "layer {i} defines {} buttons, but there are {SWITCH_COUNT} switches",
                    layer.buttons.len(),
                )
            }
        }

        for (i, pin) in self.pins.into_iter().enumerate() {
            if spawner.spawn(switch_task(i, pin, state)).is_err() {
                error!("failed to spawn switch task, pool_size mismatch?");
                break;
            }
        }
    }
}

const MOD_TAP_TIME: Duration = Duration::from_millis(150);
const SWITCH_COUNT: usize = 18;

/// Task for monitoring a single switch pin, and handling button presses.
#[embassy_executor::task(pool_size = 18)]
async fn switch_task(switch_num: usize, pin: AnyPin, state: &'static State) -> ! {
    let _pin_nr = pin.pin();
    let mut pin = Input::new(pin, Pull::Up);
    loop {
        // pins are pull-up, so when the switch is pressed they are brought low.
        pin.wait_for_low().await;

        // TODO: do we need debouncing?

        // get current layer
        let mut current_layer = state.current_layer.load(Ordering::Relaxed);
        let layer_count = state.layers.len() as u16;
        if current_layer >= layer_count {
            error!("current layer was out of bounds for some reason ({current_layer})");
            current_layer = 0;
        }
        let Some(Layer { buttons }) = state.layers.get(usize::from(current_layer)) else {
            error!("current layer was out of bounds for some reason ({current_layer})");
            state.current_layer.store(0, Ordering::Relaxed);
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

        let set_led = |color: Rgb| {
            let led_num = state.led_map.get(switch_num).copied();
            move |leds: &mut [Rgb; SWITCH_COUNT]| {
                if let Some(led) = led_num.and_then(|i| leds.get_mut(i)) {
                    *led = color;
                }
            }
        };

        match button {
            &Button::Key(key) => {
                KB_REPORT.lock().await.press_key(key);
                state.lights.update(set_led(Rgb::new(0, 150, 0))).await;
                wait_for_release.await;
                KB_REPORT.lock().await.release_key(key);
                state.lights.update(set_led(Rgb::new(0, 0, 0))).await;
                continue;
            }
            &Button::Mod(modifier) => {
                KB_REPORT.lock().await.press_modifier(modifier);
                state.lights.update(set_led(Rgb::new(100, 100, 0))).await;
                wait_for_release.await;
                KB_REPORT.lock().await.release_modifier(modifier);
                state.lights.update(set_led(Rgb::new(0, 0, 0))).await;
                continue;
            }
            &Button::ModTap(key, modifier) => {
                state.lights.update(set_led(Rgb::new(100, 100, 0))).await;
                select_biased! {
                    _ = Timer::after(MOD_TAP_TIME).fuse() => {
                        KB_REPORT.lock().await.press_modifier(modifier);
                        state.lights.update(set_led(Rgb::new(0, 0, 150))).await;
                        pin.wait_for_high().await;
                        KB_REPORT.lock().await.release_modifier(modifier);
                        state.lights.update(set_led(Rgb::new(0, 0, 0))).await;
                        debug!("switch {switch_num} button {button:?} released");
                        continue;
                    }
                    _ = wait_for_release.fuse() => {
                        KB_REPORT.lock().await.press_key(key);
                        state.lights.update(set_led(Rgb::new(0, 150, 0))).await;
                        Timer::after(Duration::from_millis(10)).await;
                        KB_REPORT.lock().await.release_key(key);
                        state.lights.update(set_led(Rgb::new(0, 0, 0))).await;
                        continue;
                    }
                }
            }
            Button::NextLayer => {
                let next_layer = (current_layer + 1) % layer_count;
                state.current_layer.store(next_layer, Ordering::Relaxed);
                debug!("switched to layer {next_layer}");
            }
            Button::PrevLayer => {
                let prev_layer = current_layer.checked_sub(1).unwrap_or(layer_count - 1);
                state.current_layer.store(prev_layer, Ordering::Relaxed);
                debug!("switched to layer {prev_layer}");
            }
            Button::None => {}
        }

        wait_for_release.await;
        state.lights.update(set_led(Rgb::new(0, 0, 0))).await;
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
