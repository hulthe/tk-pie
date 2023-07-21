mod lights;

use core::sync::atomic::Ordering;

use alloc::{boxed::Box, vec::Vec};
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{AnyPin, Input, Pin, Pull},
    peripherals::PIO1,
};
use embassy_sync::pubsub::{ImmediatePublisher, PubSubChannel, Subscriber};
use embassy_time::{Duration, Instant, Timer};
use log::{debug, error, info, warn};
use static_cell::StaticCell;
use tgnt::{
    button::{Button, LayerDir, LayerShift},
    layer::Layer,
};

use crate::{
    atomics::AtomicCoord,
    event::{
        switch::{Event, EventKind},
        Half,
    },
    lights::Lights,
    util::CS,
    ws2812::Ws2812,
};

pub struct KeyboardConfig {
    /// Which board is this.
    pub half: Half,
    /// Array of input pins of each switch
    pub pins: [AnyPin; SWITCH_COUNT],
    /// Array of LED indices of each switch
    pub led_map: [usize; SWITCH_COUNT],
    pub led_driver: Ws2812<PIO1>,
    /// Matrix of layers. Stored as rows of columns.
    pub layers: Vec<Vec<Layer>>,
}

struct State {
    /// Which board is this.
    half: Half,
    current_layer: AtomicCoord,
    layer_cols: usize,
    layers: &'static [Vec<Layer>],
    /// Array of LED indices of each switch
    led_map: [usize; SWITCH_COUNT],
    lights: Lights<PIO1, SWITCH_COUNT>,
}

pub const KB_SUBSCRIBERS: usize = 2;
const ACTUAL_KB_SUBSCRIBERS: usize = KB_SUBSCRIBERS + 2;
const KB_EVENT_CAP: usize = 128;
static KB_EVENTS: PubSubChannel<CS, Event, KB_EVENT_CAP, ACTUAL_KB_SUBSCRIBERS, 0> =
    PubSubChannel::new();
pub struct KbEvents {
    pub subscriber: Subscriber<'static, CS, Event, KB_EVENT_CAP, ACTUAL_KB_SUBSCRIBERS, 0>,
    pub publisher: ImmediatePublisher<'static, CS, Event, KB_EVENT_CAP, ACTUAL_KB_SUBSCRIBERS, 0>,
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
            current_layer: AtomicCoord::new(),
            layer_cols: self.layers.iter().map(|row| row.len()).max().unwrap_or(0),
            layers: Box::leak(self.layers.into_boxed_slice()),
            lights: Lights::new(self.led_driver),
            led_map: self.led_map,
        });

        for (y, row) in state.layers.iter().enumerate() {
            for (x, layer) in row.iter().enumerate() {
                if layer.buttons.len() != SWITCH_COUNT {
                    warn!(
                        "layer ({x}, {y}) defines {} buttons, but there are {SWITCH_COUNT} switches",
                        layer.buttons.len(),
                    )
                }
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

        spawner.must_spawn(lights::task(
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

pub const MOD_TAP_TIME: Duration = Duration::from_millis(200);
pub const SWITCH_COUNT: usize = 18;
pub const DEBOUNCE_THRESHOLD: Duration = Duration::from_millis(1);

/// Task for monitoring a single switch pin, and handling button presses.
#[embassy_executor::task(pool_size = 18)]
async fn switch_task(switch_num: usize, pin: AnyPin, state: &'static State) -> ! {
    let _pin_nr = pin.pin();
    let mut pin = Input::new(pin, Pull::Up);
    let events = KB_EVENTS.immediate_publisher();
    loop {
        // pins are pull-up, so when the switch is pressed they are brought low.
        pin.wait_for_low().await;
        let pressed_at = Instant::now();

        // TODO: do we need debouncing?

        // get current layer
        let (x, y) = state.current_layer.load(Ordering::Relaxed);

        let Some(Layer { buttons }) = state.layers.get(usize::from(y))
            .and_then(|row| row.get(usize::from(x))) else {
            // currently layer is null, do nothing
            pin.wait_for_high().await;
            continue;
        };

        // and current button
        let Some(button) = buttons.get(switch_num) else {
            warn!("no button defined for switch {switch_num}");
            pin.wait_for_high().await;
            continue;
        };

        debug!("switch {switch_num} button {button:?} pressed");

        let ev = |kind| Event {
            source: state.half,
            source_button: switch_num,
            kind,
        };

        events.publish_immediate(ev(EventKind::Press {
            button: button.clone(),
        }));

        Timer::after(DEBOUNCE_THRESHOLD).await;
        pin.wait_for_high().await;
        let released_after = pressed_at.elapsed();

        debug!("switch {switch_num} button {button:?} released");

        events.publish_immediate(ev(EventKind::Release {
            button: button.clone(),
            after: released_after.into(),
        }));

        Timer::after(DEBOUNCE_THRESHOLD).await;
    }
}

#[embassy_executor::task]
async fn layer_switch_task(mut events: KbEvents, state: &'static State) {
    let col_count = state.layer_cols as u16;
    let row_count = state.layers.len() as u16;
    let Some(last_row) = row_count.checked_sub(1) else {
        error!("no layers specified");
        return;
    };

    let Some(last_col) = col_count.checked_sub(1) else {
        error!("no layers specified");
        return;
    };

    loop {
        use LayerDir::*;
        use LayerShift::*;

        let event = events.recv().await;
        let (x, y) = state.current_layer.load(Ordering::Relaxed);
        let (nx, ny) = match event.kind {
            EventKind::Press { button } => match button {
                Button::Layer(_, Up, n) => (x, y.checked_sub(n).unwrap_or(last_row)),
                Button::Layer(_, Down, n) => (x, y.wrapping_add(n) % row_count),
                Button::Layer(_, Left, n) => (x.checked_sub(n).unwrap_or(last_col), y),
                Button::Layer(_, Right, n) => (x.wrapping_add(n) % col_count, y),
                _ => continue,
            },
            EventKind::Release { button, .. } => match button {
                Button::Layer(Peek, Up, n) => (x, y.wrapping_add(n) % row_count),
                Button::Layer(Peek, Down, n) => (x, y.checked_sub(n).unwrap_or(last_row)),
                Button::Layer(Peek, Left, n) => (x.wrapping_add(n) % col_count, y),
                Button::Layer(Peek, Right, n) => (x.checked_sub(n).unwrap_or(last_col), y),
                _ => continue,
            },
        };

        state.current_layer.store(nx, ny, Ordering::Relaxed);
        debug!("switched to layer ({nx}, {ny})");
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
