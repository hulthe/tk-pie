use core::cmp::min;

use embassy_time::{Duration, Timer};
use futures::{select_biased, FutureExt};

use crate::{util::wheel, ws2812::Rgb};

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

#[embassy_executor::task]
pub(super) async fn task(mut events: KbEvents, state: &'static State) {
    loop {
        select_biased! {
            event = events.recv().fuse() => handle_event(event, state).await,
            _ = Timer::after(UNTIL_IDLE).fuse() => {
                select_biased! {
                    event = events.recv().fuse() => {
                        state.lights.update(|lights| {
                            lights.iter_mut().for_each(|rgb| *rgb = Rgb::new(0, 0, 0));
                        }).await;
                        handle_event(event, state).await;
                    }
                    _ = idle_animation(state).fuse() => {}
                }
            }
        }
    }
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
                    let (r, g, b) = wheel(
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

async fn handle_event(event: Event, state: &'static State) {
    let set_button_led = |color: Rgb| {
        let led_num = state.led_map.get(event.source_button).copied();
        move |leds: &mut [Rgb; SWITCH_COUNT]| {
            if event.source != state.half {
                return;
            }

            if let Some(led) = led_num.and_then(|i| leds.get_mut(i)) {
                *led = color;
            }
        }
    };

    match event.kind {
        EventKind::PressKey(_) => {
            state
                .lights
                .update(set_button_led(Rgb::new(0, 150, 0)))
                .await;
        }
        EventKind::PressModifier(_) => {
            state
                .lights
                .update(set_button_led(Rgb::new(0, 0, 150)))
                .await;
        }
        EventKind::ReleaseKey(_) | EventKind::ReleaseModifier(_) => {
            state.lights.update(set_button_led(Rgb::new(0, 0, 0))).await;
        }
        EventKind::SetLayer(layer) => {
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

            state
                .lights
                .update(|leds| {
                    for &button in buttons_to_light_up {
                        let Some(&led_id) = state.led_map.get(button) else { continue; };
                        let Some(rgb) = leds.get_mut(led_id) else { continue; };
                        *rgb = Rgb::new(100, 0, 100);
                    }
                })
                .await;

            Timer::after(Duration::from_millis(200)).await;

            state
                .lights
                .update(|leds| {
                    for &button in buttons_to_light_up {
                        let Some(&led_id) = state.led_map.get(button) else { continue; };
                        let Some(rgb) = leds.get_mut(led_id) else { continue; };
                        *rgb = Rgb::new(0, 0, 0);
                    }
                })
                .await;
        }
    }
}
