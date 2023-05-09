use core::sync::atomic::{AtomicU16, Ordering};

use alloc::{boxed::Box, vec::Vec};
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{AnyPin, Input, Pin, Pull},
    pio::PioInstanceBase,
};
use embassy_sync::pubsub::{ImmediatePublisher, PubSubChannel, Subscriber};
use embassy_time::{Duration, Timer};
use futures::{select_biased, FutureExt};
use log::{debug, error, info, warn};
use static_cell::StaticCell;
use tgnt::{
    button::{Button, Modifier},
    keys::Key,
    layer::Layer,
};

use crate::{
    lights::Lights,
    util::CS,
    ws2812::{Rgb, Ws2812},
};

pub struct KeyboardConfig {
    /// Which board is this.
    pub half: Half,
    /// Array of input pins of each switch
    pub pins: [AnyPin; SWITCH_COUNT],
    /// Array of LED indices of each switch
    pub led_map: [usize; SWITCH_COUNT],
    pub led_driver: Ws2812<PioInstanceBase<1>>,
    pub layers: Vec<Layer>,
}

struct State {
    /// Which board is this.
    half: Half,
    current_layer: AtomicU16,
    layers: &'static [Layer],
    /// Array of LED indices of each switch
    led_map: [usize; SWITCH_COUNT],
    lights: Lights<PioInstanceBase<1>, SWITCH_COUNT>,
}

/// A keyboard half.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Half {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub struct Event {
    pub source: Half,
    pub kind: EventKind,
}

#[derive(Clone, Debug)]
pub enum EventKind {
    PressKey(Key),
    ReleaseKey(Key),
    PressModifier(Modifier),
    ReleaseModifier(Modifier),
    SetLayer(u16),
}

pub const KB_SUBSCRIBERS: usize = 2;
pub const ACTUAL_KB_SUBSCRIBERS: usize = KB_SUBSCRIBERS + 1;
const KB_EVENT_CAP: usize = 128;
static KB_EVENTS: PubSubChannel<CS, Event, KB_EVENT_CAP, ACTUAL_KB_SUBSCRIBERS, 0> =
    PubSubChannel::new();
pub struct KbEvents {
    subscriber: Subscriber<'static, CS, Event, KB_EVENT_CAP, ACTUAL_KB_SUBSCRIBERS, 0>,
    publisher: ImmediatePublisher<'static, CS, Event, KB_EVENT_CAP, ACTUAL_KB_SUBSCRIBERS, 0>,
}

pub struct KbEventsTx<'a> {
    publisher:
        &'a mut ImmediatePublisher<'static, CS, Event, KB_EVENT_CAP, ACTUAL_KB_SUBSCRIBERS, 0>,
}

pub struct KbEventsRx<'a> {
    subscriber: &'a mut Subscriber<'static, CS, Event, KB_EVENT_CAP, ACTUAL_KB_SUBSCRIBERS, 0>,
}

impl KeyboardConfig {
    pub async fn create(self) -> Option<[KbEvents; KB_SUBSCRIBERS]> {
        let spawner = Spawner::for_current_executor().await;

        if self.layers.is_empty() {
            error!("no layers defined");
            return None;
        }

        info!(
            "setting up keyboard layout with {} layer(s)",
            self.layers.len()
        );

        static STATE: StaticCell<State> = StaticCell::new();
        let state = STATE.init_with(|| State {
            half: self.half,
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

        spawner.must_spawn(layer_switch_task(
            KbEvents {
                publisher: KB_EVENTS.immediate_publisher(),
                subscriber: KB_EVENTS.subscriber().unwrap(),
            },
            state,
        ));

        Some([(); KB_SUBSCRIBERS].map(|_| KbEvents {
            publisher: KB_EVENTS.immediate_publisher(),
            subscriber: KB_EVENTS.subscriber().unwrap(),
        }))
    }
}

impl KbEvents {
    pub async fn send(&mut self, event: Event) {
        self.publisher.publish_immediate(event);
    }

    pub async fn recv(&mut self) -> Event {
        self.subscriber.next_message_pure().await
    }

    pub fn split(&mut self) -> (KbEventsRx, KbEventsTx) {
        let tx = KbEventsTx {
            publisher: &mut self.publisher,
        };
        let rx = KbEventsRx {
            subscriber: &mut self.subscriber,
        };
        (rx, tx)
    }
}

impl KbEventsRx<'_> {
    pub async fn recv(&mut self) -> Event {
        self.subscriber.next_message_pure().await
    }
}

impl KbEventsTx<'_> {
    pub fn send(&mut self, event: Event) {
        self.publisher.publish_immediate(event);
    }
}

const MOD_TAP_TIME: Duration = Duration::from_millis(150);
const SWITCH_COUNT: usize = 18;

/// Task for monitoring a single switch pin, and handling button presses.
#[embassy_executor::task(pool_size = 18)]
async fn switch_task(switch_num: usize, pin: AnyPin, state: &'static State) -> ! {
    let _pin_nr = pin.pin();
    let mut pin = Input::new(pin, Pull::Up);
    let events = KB_EVENTS.immediate_publisher();
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

        let ev = |kind| Event {
            source: state.half,
            kind,
        };

        use EventKind::*;
        match button {
            &Button::Key(key) => {
                events.publish_immediate(ev(PressKey(key)));
                state.lights.update(set_led(Rgb::new(0, 150, 0))).await;
                wait_for_release.await;
                events.publish_immediate(ev(ReleaseKey(key)));
                state.lights.update(set_led(Rgb::new(0, 0, 0))).await;
                continue;
            }
            &Button::Mod(modifier) => {
                events.publish_immediate(ev(PressModifier(modifier)));
                state.lights.update(set_led(Rgb::new(100, 100, 0))).await;
                wait_for_release.await;
                events.publish_immediate(ev(ReleaseModifier(modifier)));
                state.lights.update(set_led(Rgb::new(0, 0, 0))).await;
                continue;
            }
            &Button::ModTap(key, modifier) => {
                state.lights.update(set_led(Rgb::new(100, 100, 0))).await;
                select_biased! {
                    _ = Timer::after(MOD_TAP_TIME).fuse() => {
                        events.publish_immediate(ev(PressModifier(modifier)));
                        state.lights.update(set_led(Rgb::new(0, 0, 150))).await;
                        pin.wait_for_high().await;
                        events.publish_immediate(ev(ReleaseModifier(modifier)));
                        state.lights.update(set_led(Rgb::new(0, 0, 0))).await;
                        debug!("switch {switch_num} button {button:?} released");
                        continue;
                    }
                    _ = wait_for_release.fuse() => {
                        events.publish_immediate(ev(PressKey(key)));
                        state.lights.update(set_led(Rgb::new(0, 150, 0))).await;
                        Timer::after(Duration::from_millis(10)).await;
                        events.publish_immediate(ev(ReleaseKey(key)));
                        state.lights.update(set_led(Rgb::new(0, 0, 0))).await;
                        continue;
                    }
                }
            }
            Button::NextLayer => {
                let next_layer = (current_layer + 1) % layer_count;
                state.lights.update(set_led(Rgb::new(100, 0, 100))).await;
                events.publish_immediate(ev(SetLayer(next_layer)));
                debug!("switched to layer {next_layer}");
            }
            Button::PrevLayer => {
                let prev_layer = current_layer.checked_sub(1).unwrap_or(layer_count - 1);
                state.lights.update(set_led(Rgb::new(100, 0, 100))).await;
                events.publish_immediate(ev(SetLayer(prev_layer)));
                debug!("switched to layer {prev_layer}");
            }
            Button::None => {}
        }

        wait_for_release.await;
        state.lights.update(set_led(Rgb::new(0, 0, 0))).await;
    }
}

#[embassy_executor::task]
async fn layer_switch_task(mut events: KbEvents, state: &'static State) {
    loop {
        let event = events.recv().await;
        if let EventKind::SetLayer(new_layer) = event.kind {
            state.current_layer.store(new_layer, Ordering::Relaxed);
        }
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
