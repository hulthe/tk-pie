use embassy_time::{Duration, Timer};
use futures::{select_biased, FutureExt};

use crate::ws2812::Rgb;

use super::{EventKind, KbEvents, State, SWITCH_COUNT};

#[embassy_executor::task]
pub(super) async fn task(mut events: KbEvents, state: &'static State) {
    loop {
        select_biased! {
            event = events.recv().fuse() => {
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
                        state.lights.update(set_button_led(Rgb::new(0, 150, 0))).await;
                    }
                    EventKind::PressModifier(_) => {
                        state.lights.update(set_button_led(Rgb::new(0, 0, 150))).await;
                    }
                    EventKind::ReleaseKey(_) | EventKind::ReleaseModifier(_) => {
                        state.lights.update(set_button_led(Rgb::new(0, 0, 0))).await;
                    }
                    EventKind::SetLayer(layer) => {
                        let buttons_to_light_up = match layer {
                            0 => [0, 1, 2, 3, 4].as_ref(),
                            1 => &[5, 6, 7, 8, 9],
                            2 => &[10, 11, 12, 13, 14],
                            _ => &[],
                        };

                        state.lights.update(|leds| {
                            for &button in buttons_to_light_up {
                                let Some(&led_id) = state.led_map.get(button) else { continue; };
                                let Some(rgb) = leds.get_mut(led_id) else { continue; };
                                *rgb = Rgb::new(100, 0, 100);
                            }
                        }).await;

                        Timer::after(Duration::from_millis(200)).await;

                        state.lights.update(|leds| {
                            for &button in buttons_to_light_up {
                                let Some(&led_id) = state.led_map.get(button) else { continue; };
                                let Some(rgb) = leds.get_mut(led_id) else { continue; };
                                *rgb = Rgb::new(0, 0, 0);
                            }
                        }).await;
                    }
                }
            }
        }
    }
}
