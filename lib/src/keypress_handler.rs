use core::future::pending;

use embassy_sync::pubsub::{publisher::Pub, subscriber::Sub, PubSubBehavior, WaitResult};
use embassy_time::{Duration, Instant, Timer};
use futures::FutureExt;
use heapless::Deque;
use log::{debug, error};
use tgnt::button::Button;

use crate::event::{button, switch, Half};

// TODO
const MOD_TAP_TIME: Duration = Duration::from_millis(150);
const SWITCH_COUNT: usize = 18;

/// This function perpetually converts between [switch::Event]s and [button::Event]s.
///
/// Call it from a dedicated task.
pub async fn keypress_handler(
    input: &mut Sub<'_, impl PubSubBehavior<switch::Event>, switch::Event>,
    output: &mut Pub<'_, impl PubSubBehavior<button::Event>, button::Event>,
) -> ! {
    type SwitchIndex = usize;
    #[derive(Debug)]
    struct PressEvent {
        source_button: SwitchIndex,
        source_half: Half,
        button: Button,
        time: Instant,
    }

    async fn slow_press(
        output: &mut Pub<'_, impl PubSubBehavior<button::Event>, button::Event>,
        button: &Button,
    ) {
        let event = match button {
            &Button::Mod(m) | &Button::ModTap(_, m) => button::Event::PressMod(m),
            &Button::Key(k) => button::Event::PressKey(k),
            _ => return,
        };
        output.publish_immediate(event);
    }

    async fn slow_release(
        output: &mut Pub<'_, impl PubSubBehavior<button::Event>, button::Event>,
        button: &Button,
    ) {
        let event = match button {
            &Button::Mod(m) | &Button::ModTap(_, m) => button::Event::ReleaseMod(m),
            &Button::Key(k) => button::Event::ReleaseKey(k),
            _ => return,
        };
        output.publish_immediate(event);
    }

    // queue of button presses that are waiting for ModTap
    let mut queue = Deque::<PressEvent, { 2 * SWITCH_COUNT }>::new();
    let queue = &mut queue;

    loop {
        // create a future that waits for the next ModTap to time out
        let modtap_timeout = async {
            loop {
                if let Some(event) = queue.front() {
                    let Button::ModTap(..) = event.button else {
                        error!("first element in queue wasn't a modtap, wtf?");
                        let _ = queue.pop_front();
                        continue;
                    };

                    let timeout = event.time + MOD_TAP_TIME;
                    Timer::at(timeout).await;
                    return queue.pop_front().unwrap();
                } else {
                    // if the queue is empty, never return.
                    return pending().await;
                }
            }
        };

        let event = futures::select_biased! {
            event = input.next_message().fuse() => event,
            event = modtap_timeout.fuse() => {
                // first element in queue timed out, and will be treated as a Mod
                let &Button::ModTap(..) = &event.button else {
                    error!("first element in queue wasn't a modtap, wtf?");
                    continue;
                };
                debug!("modtap resolved as mod: {:?}", event.button);
                slow_press(output, &event.button).await;

                loop {
                    let Some(event) = queue.pop_front() else { break };

                    // resolve events until we encounter another ModTap,
                    // then put it back in the queue and stop
                    if let Button::ModTap(..) = &event.button {
                        queue.push_front(event).expect("we just popped, the queue can't be full");
                        break;
                    }
                    slow_press(output, &event.button).await;
                }

                continue;
            },
        };

        let WaitResult::Message(event) = event else {
            error!("lagged");
            continue;
        };

        let time = Instant::now();
        debug!("event: {:?}", event.kind);
        match event.kind {
            switch::EventKind::Press { button } => {
                let insert = |queue: &mut Deque<_, 36>, button| {
                    if let Some(_queued) = queue
                        .iter()
                        .find(|queued: &&PressEvent| queued.source_button == event.source_button)
                        .filter(|queued| queued.source_half == event.source)
                    {
                        error!("tried to add PressEvent to queue twice");
                    } else if queue
                        .push_back(PressEvent {
                            source_half: event.source,
                            source_button: event.source_button,
                            button,
                            time,
                        })
                        .is_err()
                    {
                        error!("button queue full. this shouldn't happen.");
                    }
                };

                match button {
                    Button::ModTap(..) => {
                        debug!("adding modtap to queue");
                        // add event to queue
                        insert(queue, button);
                    }
                    Button::Mod(..) | Button::Key(..) => {
                        if queue.is_empty() {
                            debug!("sending key now");
                            // otherwise, send immediately
                            slow_press(output, &button).await;
                        } else {
                            debug!("adding key to queue");
                            // if events in queue, also add to queue
                            insert(queue, button);
                        }
                    }
                    Button::Compose(..) => {
                        if queue.is_empty() {
                            // otherwise, send immediately
                            // TODO
                        } else {
                            // if events in queue, also add to queue
                            insert(queue, button);
                        }
                    }
                    _ => {}
                }
            }
            switch::EventKind::Release { button, .. } => {
                let position_in_queue = queue
                    .iter()
                    .enumerate()
                    .filter(|(_, queued)| queued.source_half == event.source)
                    .find(|(_, queued)| queued.source_button == event.source_button)
                    .map(|(i, _)| i);

                match button {
                    Button::ModTap(k, _) => {
                        // check if modtap in queue
                        if let Some(position_in_queue) = position_in_queue {
                            // If the modtap was still in the queue, it hasn't been resolved as a mod
                            // yet. Therefore, it is a Tap. Resolve all ModTaps before this one as Mods
                            debug!("modtap was still in queue: {k:?}");
                            for _ in 0..position_in_queue {
                                let prev_event = queue.pop_front().unwrap();
                                debug!("pressing earlier event {:?}", prev_event);
                                slow_press(output, &prev_event.button).await;
                            }
                            let _ = queue.pop_front();
                            debug!("pressing modtap as key");
                            slow_press(output, &Button::Key(k)).await;
                            slow_release(output, &Button::Key(k)).await;
                        } else {
                            // If the ModTap wasn't in the queue, it has already been resolved as a Mod.
                            debug!("modtap wasn't in queue, releasing");
                            slow_release(output, &button).await;
                        };
                    }
                    Button::Mod(..) | Button::Key(..) => {
                        // if this press event was in queue, resolve all ModTaps before in queue as Mods
                        // otherwise, just resolve this
                        if let Some(position_in_queue) = position_in_queue {
                            debug!(
                                "key was in queue, pressing all events up to and including this"
                            );
                            for _ in 0..=position_in_queue {
                                let prev_event = queue.pop_front().unwrap();
                                debug!("pressing {prev_event:?}");
                                slow_press(output, &prev_event.button).await;
                            }
                        }
                        debug!("releasing key {button:?}");
                        slow_release(output, &button).await;
                    }
                    Button::Compose(..) => {
                        // if this press event was in queue, resolve all ModTaps before in queue as Mods
                        // otherwise, just resolve this
                        // TODO
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(all(target_arch = "x86_64", test))]
mod tests {
    use core::time;

    use super::*;

    use alloc::vec;
    use alloc::vec::Vec;
    use embassy_futures::{
        join::join,
        select::{select, Either},
    };
    use embassy_sync::{blocking_mutex::raw::NoopRawMutex, pubsub::PubSubChannel};
    use embassy_time::with_timeout;
    use tgnt::{button::Modifier, keys::Key};

    struct Test {
        // button index, pressed, delay
        input: Vec<(usize, bool, Duration)>,
        expected: Vec<button::Event>,
    }

    macro_rules! timing_test {
        ($name:ident, $test:expr) => {
            #[test]
            fn $name() {
                let test: Test = $test;
                run_test(test);
            }
        };
    }

    const SHORT: Duration = Duration::from_millis(1);
    const LONG: Duration = Duration::from_millis(160);

    timing_test! {
        modtap_mod,
        Test {
            input: vec![
                (0, true, SHORT),
                (1, true, SHORT),
                (1, false, SHORT),
                (0, false, SHORT),
            ],
            expected: vec![
                button::Event::PressMod(Modifier::LShift),
                button::Event::PressKey(Key::B),
                button::Event::ReleaseKey(Key::B),
                button::Event::ReleaseMod(Modifier::LShift),
            ],
        }
    }
    timing_test! {
        modtap_tap,
        Test {
            input: vec![
                (0, true, SHORT),
                (1, true, SHORT),
                (0, false, SHORT),
                (1, false, SHORT),
            ],
            expected: vec![
                button::Event::PressKey(Key::A),
                button::Event::ReleaseKey(Key::A),
                button::Event::PressKey(Key::B),
                button::Event::ReleaseKey(Key::B),
            ],
        }
    }

    timing_test! { modtap_tap_2x, Test {
        input: vec![
            (0, true, SHORT),
            (2, true, SHORT),
            (2, false, SHORT),
            (0, false, SHORT),
            (0, true, SHORT),
            (2, true, SHORT),
            (2, false, SHORT),
            (0, false, SHORT),
        ],
        expected: vec![
            button::Event::PressMod(Modifier::LShift),
            button::Event::PressKey(Key::C),
            button::Event::ReleaseKey(Key::C),
            button::Event::ReleaseMod(Modifier::LShift),
            button::Event::PressMod(Modifier::LShift),
            button::Event::PressKey(Key::C),
            button::Event::ReleaseKey(Key::C),
            button::Event::ReleaseMod(Modifier::LShift),
        ],
    }}

    timing_test! { modtap_double_shift, Test {
        input: vec![
            (0, true, LONG),
            (3, true, SHORT),
            (0, false, LONG),
            (3, false, SHORT),
        ],
        expected: vec![
            button::Event::PressMod(Modifier::LShift),
            button::Event::ReleaseMod(Modifier::LShift),
            button::Event::PressMod(Modifier::RShift),
            button::Event::ReleaseMod(Modifier::RShift),
        ],
    }}

    fn run_test(test: Test) {
        let _ = simple_logger::SimpleLogger::new().init();

        let switch_events = PubSubChannel::<NoopRawMutex, switch::Event, 10, 1, 1>::new();
        let button_events = PubSubChannel::<NoopRawMutex, button::Event, 10, 1, 1>::new();
        let mut button_sub = button_events.subscriber().unwrap();

        let buttons = [
            Button::ModTap(Key::A, Modifier::LShift),
            Button::ModTap(Key::B, Modifier::LCtrl),
            Button::Key(Key::C),
            Button::ModTap(Key::D, Modifier::RShift),
        ];

        // future that iterates over the test input and sends switch events.
        let clicker = async {
            for &(i, pressed, delay) in &test.input {
                let kind = if pressed {
                    switch::EventKind::Press {
                        button: buttons[i].clone(),
                    }
                } else {
                    switch::EventKind::Release {
                        button: buttons[i].clone(),
                        after: time::Duration::ZERO, // ignore
                    }
                };

                let event = switch::Event {
                    source: Half::Left,
                    source_button: i,
                    kind,
                };

                switch_events.publish_immediate(event);

                Timer::after(delay).await;
            }
        };
        let output_listener = async {
            let mut got = Vec::new();
            for _expected in &test.expected {
                let timeout = LONG + Duration::from_millis(50);
                let event = match with_timeout(timeout, button_sub.next_message()).await {
                    Ok(WaitResult::Message(event)) => event,
                    Ok(WaitResult::Lagged(_)) => return Err(("lagged", got)),
                    Err(_) => return Err(("timeout", got)),
                };

                got.push(event.clone());
            }

            for (event, expected) in got.iter().zip(test.expected.iter()) {
                if event != expected {
                    return Err(("unexpected event", got));
                }
            }

            Ok(())
        };

        embassy_futures::block_on(async {
            let r = select(
                join(clicker, output_listener),
                keypress_handler(
                    &mut *switch_events.subscriber().unwrap(),
                    &mut *button_events.publisher().unwrap(),
                ),
            )
            .await;

            match r {
                Either::First(((), Ok(()))) => {}
                Either::First(((), Err((msg, got)))) => panic!(
                    "timing test failed due to {msg}.\nexpected={:#?} got={:#?}",
                    test.expected, got
                ),
                Either::Second(never) => never,
            }
        });

        panic!();
    }
}
