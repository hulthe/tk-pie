use core::future::pending;

use embassy_time::{Duration, Instant, Timer};
use futures::{select_biased, FutureExt};

use crate::{
    button::Button,
    lights::shaders::{PowerOffAnim, PowerOnAnim, Shader, Shaders},
    rgb::Rgb,
    usb::{UsbEvent, USB_EVENTS},
};

use super::{Event, EventKind, KbEvents, State, SWITCH_COUNT};

/// Duration until the keyboard starts the idle animation
const UNTIL_IDLE: Duration = Duration::from_secs(30);

/// DUration between each animation frame.
const FRAMETIME: Duration = Duration::from_millis(16);

const IDLE_ANIM: Shaders = Shaders::OrthoRainbow;

#[derive(Default)]
enum LightsState {
    Active {
        keys: [KeyLedState; SWITCH_COUNT],
        next_frame: Instant,
        idle_at: Instant,
    },
    PoweringOff(PowerOffAnim),
    PoweringOn(PowerOnAnim),
    #[default]
    PoweredOff,
    Idle(Shaders),
}

impl LightsState {
    pub fn active_default() -> Self {
        let now = Instant::now();
        LightsState::Active {
            keys: Default::default(),
            next_frame: now,
            idle_at: now + UNTIL_IDLE,
        }
    }
}

#[derive(Clone, Copy, Default)]
enum KeyLedState {
    #[default]
    None,
    Solid(Rgb),
    FadeBy(f32),
}

#[embassy_executor::task]
pub(super) async fn task(mut events: KbEvents, state: &'static State) {
    let mut lights = LightsState::default();
    let mut usb_events = USB_EVENTS
        .dyn_subscriber()
        .expect("USB_EVENTS: out of subscribers");

    loop {
        match &mut lights {
            LightsState::Active {
                keys,
                next_frame,
                idle_at,
            } => {
                select_biased! {
                    event = events.recv().fuse() => {
                        *idle_at = Instant::now() + UNTIL_IDLE;
                        handle_event(event, state, keys).await;
                    }
                    ev = usb_events.next_message_pure().fuse() => {
                        *idle_at = Instant::now() + UNTIL_IDLE;
                        handle_usb_event(ev, &mut lights).await;
                    }
                    _ = Timer::at(*idle_at).fuse() => lights = LightsState::Idle(IDLE_ANIM),
                    _ = Timer::at(*next_frame).fuse() => {
                        keypress_tick(state, keys).await;
                        *next_frame = Instant::now() + FRAMETIME;
                    }
                }
            }
            LightsState::PoweringOff(anim) => {
                select_biased! {
                    ev = usb_events.next_message_pure().fuse() => handle_usb_event(ev, &mut lights).await,
                    _ = play_shader(state, anim).fuse() => lights = LightsState::PoweredOff,
                }
            }
            LightsState::PoweringOn(anim) => {
                select_biased! {
                    ev = usb_events.next_message_pure().fuse() => handle_usb_event(ev, &mut lights).await,
                    _ = play_shader(state, anim).fuse() => lights = LightsState::active_default(),
                }
            }
            LightsState::PoweredOff => {
                let ev = usb_events.next_message_pure().await;
                handle_usb_event(ev, &mut lights).await;
            }
            LightsState::Idle(anim) => {
                select_biased! {
                    ev = usb_events.next_message_pure().fuse() => handle_usb_event(ev, &mut lights).await,
                    event = events.recv().fuse() => {
                        let now = Instant::now();
                        let mut keys = Default::default();
                        handle_event(event, state, &mut keys).await;
                        lights = LightsState::Active { keys, next_frame: now, idle_at: now + UNTIL_IDLE};
                    }
                    _ = play_shader(state, anim).fuse() => {}
                }
            }
        };
    }
}

async fn play_shader(state: &'static State, shader: &impl Shader) {
    const SWITCH_COORDS: [(u16, u16); SWITCH_COUNT] = [
        (0, 1),
        (1, 1),
        (2, 1),
        (3, 1),
        (4, 1),
        (4, 2),
        (3, 2),
        (2, 2),
        (1, 2),
        (0, 2),
        (0, 3),
        (1, 3),
        (2, 3),
        (3, 3),
        (4, 3),
        (2, 4),
        (3, 4),
        (4, 4),
    ];

    const BRIGHTNESS: f32 = 1.00;

    let switch_coords = SWITCH_COORDS.map(|(x, y)| (f32::from(x) / 4.0, f32::from(y) / 4.0));

    let animate_shader = async {
        loop {
            let now = Instant::now();

            state
                .lights
                .update(|rgbs| {
                    (switch_coords.into_iter().zip(rgbs.iter_mut()))
                        .for_each(|(uv, rgb)| *rgb = shader.sample(now, uv) * BRIGHTNESS)
                })
                .await;

            Timer::after(FRAMETIME).await;
        }
    };

    let end_animation = async {
        match shader.end_time() {
            Some(end_time) => Timer::at(end_time).await,
            None => pending().await,
        };
    };

    select_biased! {
        _ = animate_shader.fuse() => {}
        _ = end_animation.fuse() => {}
    }
}

async fn keypress_tick(state: &'static State, lights: &mut [KeyLedState; SWITCH_COUNT]) {
    state
        .lights
        .update(|rgbs| {
            for (button, light) in lights.iter_mut().enumerate() {
                let Some(&led_id) = state.led_map.get(button) else {
                    continue;
                };
                let Some(rgb) = rgbs.get_mut(led_id) else {
                    continue;
                };

                match &*light {
                    KeyLedState::None => *rgb = Rgb::new(0, 0, 0),
                    KeyLedState::FadeBy(fade) => {
                        let [r, g, b] = rgb
                            .components()
                            .map(|c| ((c as f32) * fade.clamp(0.0, 1.0)) as u8);
                        *rgb = Rgb::new(r, g, b);
                        if *rgb == Rgb::new(0, 0, 0) {
                            *light = KeyLedState::None;
                        }
                    }
                    &KeyLedState::Solid(color) => *rgb = color,
                }
            }
        })
        .await;
}

async fn handle_event(
    event: Event,
    state: &'static State,
    lights: &mut [KeyLedState; SWITCH_COUNT],
) {
    let rgb = match event.kind {
        EventKind::Press { button } => match button {
            Button::Key(..) => KeyLedState::Solid(Rgb::new(0, 150, 0)),
            Button::Mod(..) => KeyLedState::Solid(Rgb::new(0, 0, 150)),
            Button::ModTap(..) => KeyLedState::Solid(Rgb::new(0, 0, 150)),
            Button::Compose2(..) | Button::Compose3(..) => {
                KeyLedState::Solid(Rgb::new(0, 100, 100))
            }
            Button::Layer(..) => KeyLedState::Solid(Rgb::new(120, 0, 120)),
            _ => KeyLedState::Solid(Rgb::new(150, 0, 0)),
        },
        EventKind::Release { .. } => KeyLedState::FadeBy(0.85),
    };

    if event.source != state.half {
        return;
    }

    let Some(light) = lights.get_mut(event.source_button) else {
        return;
    };
    *light = rgb;
}

async fn handle_usb_event(event: UsbEvent, state: &mut LightsState) {
    let usb_enabled = match event {
        UsbEvent::Suspended(false) | UsbEvent::Configured(true) => true,
        UsbEvent::Configured(false) | UsbEvent::Suspended(true) | UsbEvent::Reset => false,
        _ => return,
    };

    let start = Instant::now();
    *state = match (&state, usb_enabled) {
        (LightsState::PoweringOn(..), true) => return,
        (LightsState::PoweringOff(..), false) => return,
        (LightsState::PoweredOff, false) => return,
        (_, true) => LightsState::PoweringOn(PowerOnAnim { start }),
        (_, false) => LightsState::PoweringOff(PowerOffAnim { start }),
    };
}
