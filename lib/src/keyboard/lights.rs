use core::cmp::min;

use atomic_polyfill::Ordering;
use embassy_futures::yield_now;
use embassy_time::{Duration, Instant, Timer};
use futures::{select_biased, FutureExt};
use tgnt::button::Button;

use crate::{rgb::Rgb, util::wheel};

use super::{Event, EventKind, KbEvents, State, SWITCH_COUNT};

/// Duration until the keyboard starts the idle animation
const UNTIL_IDLE: Duration = Duration::from_secs(30);

///// Duration from idle until the keyboard goes to sleep
//const UNTIL_SLEEP: Duration = Duration::from_secs(10);

const IDLE_ANIMATION_SPEED: u64 = 3;
const IDLE_ANIMATION_KEY_OFFSET: u64 = 10;
const IDLE_BRIGHTNESS_RAMPUP: Duration = Duration::from_secs(120);
const MAX_IDLE_BRIGHTESS: f32 = 1.0;
const MIN_IDLE_BRIGHTESS: f32 = 0.05;

#[derive(Clone, Copy)]
enum LightState {
    Solid(Rgb),
    SolidThenFade {
        color: Rgb,
        solid_until: Instant,
        fade_by: f32,
    },
    FadeBy(f32),
    None,
}

#[embassy_executor::task]
pub(super) async fn task(mut events: KbEvents, state: &'static State) {
    let mut lights: [LightState; SWITCH_COUNT] = [LightState::None; SWITCH_COUNT];
    let mut next_frame = Instant::now();
    let mut idle_at = Instant::now() + UNTIL_IDLE;
    loop {
        select_biased! {
            event = events.recv().fuse() => {
                handle_event(event, state, &mut lights).await;
                idle_at = Instant::now() + UNTIL_IDLE;
            }
            _ = Timer::at(next_frame).fuse() => {
                tick(state, &mut lights).await;
                next_frame = Instant::now() + Duration::from_millis(16);
            }
            _ = Timer::at(idle_at).fuse() => {
                select_biased! {
                    event = events.recv().fuse() => {
                        state.lights.update(|lights| {
                            lights.iter_mut().for_each(|rgb| *rgb = Rgb::new(0, 0, 0));
                        }).await;
                        handle_event(event, state, &mut lights).await;
                        idle_at = Instant::now() + UNTIL_IDLE;
                    }
                    _ = idle_animation(state).fuse() => {}
                }
            }
        }
    }
}

async fn tick(state: &'static State, lights: &mut [LightState; SWITCH_COUNT]) {
    let now = Instant::now();

    state
        .lights
        .update(|rgbs| {
            for (button, light) in lights.iter_mut().enumerate() {
                let Some(&led_id) = state.led_map.get(button) else { continue; };
                let Some(rgb) = rgbs.get_mut(led_id) else { continue; };

                match &*light {
                    LightState::None => {}
                    LightState::FadeBy(fade) => {
                        let [r, g, b] = rgb
                            .components()
                            .map(|c| ((c as f32) * fade.clamp(0.0, 1.0)) as u8);
                        *rgb = Rgb::new(r, g, b);
                        if *rgb == Rgb::new(0, 0, 0) {
                            *light = LightState::None;
                        }
                    }
                    &LightState::Solid(color) => *rgb = color,
                    &LightState::SolidThenFade {
                        color,
                        solid_until,
                        fade_by,
                    } => {
                        *rgb = color;
                        if now >= solid_until {
                            *light = LightState::FadeBy(fade_by);
                        }
                    }
                }
            }
        })
        .await;
}

async fn idle_animation(state: &'static State) {
    for tick in 0.. {
        const FRAMETIME: Duration = Duration::from_millis(16);

        state
            .lights
            .update(|lights| {
                const N_MAX: u64 = IDLE_BRIGHTNESS_RAMPUP.as_millis() / FRAMETIME.as_millis();

                let brightness = if tick >= N_MAX {
                    MAX_IDLE_BRIGHTESS
                } else {
                    ((tick as f32) / N_MAX as f32).clamp(MIN_IDLE_BRIGHTESS, MAX_IDLE_BRIGHTESS)
                };

                for (n, &i) in state.led_map.iter().enumerate() {
                    let Some(light) = lights.get_mut(i) else { continue };
                    let [r, g, b] = wheel(
                        (n as u64 * IDLE_ANIMATION_KEY_OFFSET + tick * IDLE_ANIMATION_SPEED) as u8,
                    )
                    .components();
                    let rgb = Rgb::new(
                        ((r as f32) * brightness) as u8,
                        ((g as f32) * brightness) as u8,
                        ((b as f32) * brightness) as u8,
                    );
                    *light = rgb;
                }
            })
            .await;

        Timer::after(FRAMETIME).await;
    }
}

async fn handle_event(
    event: Event,
    state: &'static State,
    lights: &mut [LightState; SWITCH_COUNT],
) {
    let rgb = match event.kind {
        EventKind::Press { button } => match button {
            Button::Key(..) => LightState::Solid(Rgb::new(0, 150, 0)),
            Button::Mod(..) => LightState::Solid(Rgb::new(0, 0, 150)),
            Button::ModTap(..) => LightState::Solid(Rgb::new(0, 0, 150)),
            Button::Compose(..) => LightState::Solid(Rgb::new(0, 100, 100)),
            Button::NextLayer | Button::PrevLayer => {
                yield_now().await; // dirty hack to make sure layer_switch_task gets to run first
                let layer = state.current_layer.load(Ordering::Relaxed);
                let layer = min(layer, state.layers.len().saturating_sub(1) as u16);
                let buttons_to_light_up = if state.layers.len() <= 3 {
                    match layer {
                        0 => [0, 1, 2, 3, 4].as_ref(),
                        1 => &[5, 6, 7, 8, 9],
                        2 => &[10, 11, 12, 13, 14],
                        _ => &[],
                    }
                } else {
                    match layer {
                        0 => [0, 5, 10].as_ref(),
                        1 => &[1, 6, 11],
                        2 => &[2, 7, 12],
                        3 => &[3, 8, 13],
                        4 => &[4, 9, 14],
                        _ => &[],
                    }
                };

                let solid_until = Instant::now() + Duration::from_millis(200);
                for &button in buttons_to_light_up {
                    let Some(light) = lights.get_mut(button) else { continue; };
                    *light = LightState::SolidThenFade {
                        color: Rgb::new(120, 0, 120),
                        solid_until,
                        fade_by: 0.85,
                    }
                }
                LightState::Solid(Rgb::new(100, 0, 100))
            }
            _ => LightState::Solid(Rgb::new(150, 0, 0)),
        },
        EventKind::Release { .. } => LightState::FadeBy(0.85),
    };

    if event.source != state.half {
        return;
    }

    let Some(light) = lights.get_mut(event.source_button) else { return; };
    *light = rgb;
}
