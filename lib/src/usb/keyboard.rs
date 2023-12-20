pub mod report;

use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_rp::{peripherals::USB, usb::Driver};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    mutex::Mutex,
    pubsub::{PubSubChannel, WaitResult},
};
use embassy_time::{Duration, Timer};
use embassy_usb::{
    class::hid::{self, HidReaderWriter, ReadError, ReportId, RequestHandler},
    control::OutResponse,
    Builder,
};
use embassy_usb_driver::EndpointError;
use log::error;
use static_cell::StaticCell;
use usbd_hid::descriptor::SerializedDescriptor;

use crate::{
    event::button,
    keyboard::KbEvents,
    keypress_handler::keypress_handler,
    usb::keyboard::report::{KeyboardReport, EMPTY_KEYBOARD_REPORT},
    util::CS,
};

use super::MAX_PACKET_SIZE;

struct Handler;

struct Reports {
    /// The report to be sent to the host machine.
    actual: KeyboardReport,

    /// Key presses which hasn't been sent yet.
    unsent: KeyboardReport,
}

struct Context {
    handler: Handler,
    state: hid::State<'static>,
}

/// Set up a USB HID keyboard. This function panics if called more than once.
pub async fn setup(builder: &mut Builder<'static, Driver<'static, USB>>, events: KbEvents) {
    log::info!("setting up usb hid");

    let context = {
        static CONTEXT: StaticCell<Context> = StaticCell::new();
        // this panics if the functon is called twice
        CONTEXT.init(Context {
            handler: Handler,
            state: hid::State::new(),
        })
    };

    let reports = {
        static KB_REPORT: Mutex<CS, Reports> = Mutex::new(Reports {
            actual: EMPTY_KEYBOARD_REPORT,
            unsent: EMPTY_KEYBOARD_REPORT,
        });
        &KB_REPORT
    };

    let config = hid::Config {
        //report_descriptor: MouseReport::desc(),
        report_descriptor: KeyboardReport::desc(),
        request_handler: Some(&context.handler),
        poll_ms: 2,
        max_packet_size: MAX_PACKET_SIZE as u16,
    };

    let stream = HidStream::new(builder, &mut context.state, config);

    let spawner = Spawner::for_current_executor().await;

    spawner.must_spawn(report_task(stream, &context.handler, reports));
    spawner.must_spawn(event_listener_task(events, reports));

    log::info!("done setting up usb keyboard");
}

impl RequestHandler for Handler {
    fn get_report(&self, id: ReportId, buf: &mut [u8]) -> Option<usize> {
        log::info!("get_report({id:?}, {buf:?})");
        let _ = (id, buf);
        None
    }

    fn set_report(&self, id: ReportId, data: &[u8]) -> embassy_usb::control::OutResponse {
        log::info!("set_report({id:?}, {data:?})");
        let _ = (id, data);
        OutResponse::Rejected
    }

    fn get_idle_ms(&self, id: Option<ReportId>) -> Option<u32> {
        log::info!("get_idle_ms({id:?})");
        let _ = id;
        None
    }

    fn set_idle_ms(&self, id: Option<ReportId>, duration_ms: u32) {
        log::info!("set_idle_ms({id:?}, {duration_ms})");
        let _ = (id, duration_ms);
    }
}
type HidStream = HidReaderWriter<'static, Driver<'static, USB>, 256, 256>;

#[embassy_executor::task]
async fn event_listener_task(mut events: KbEvents, reports: &'static Mutex<CS, Reports>) {
    let button_events = PubSubChannel::<NoopRawMutex, button::Event, 10, 1, 1>::new();
    let mut button_pub = button_events.publisher().unwrap();
    let mut button_sub = button_events.subscriber().unwrap();

    select(
        async {
            loop {
                let WaitResult::Message(event) = button_sub.next_message().await else {
                    error!("lagged");
                    continue;
                };

                loop {
                    let mut r = reports.lock().await;
                    match event {
                        button::Event::PressKey(k) => {
                            r.actual.press_key(k);
                            r.unsent.press_key(k);
                        }
                        button::Event::PressMod(m) => {
                            r.actual.press_modifier(m);
                            r.unsent.press_modifier(m);
                        }

                        // we got a key release, but if the key press hasn't been sent yet, we
                        // wait for a bit until it has.
                        button::Event::ReleaseKey(k) if r.unsent.key_pressed(k) => {
                            drop(r);
                            Timer::after(Duration::from_millis(1)).await;
                            continue;
                        }
                        button::Event::ReleaseMod(m) if r.unsent.modifier_pressed(m) => {
                            drop(r);
                            Timer::after(Duration::from_millis(1)).await;
                            continue;
                        }

                        button::Event::ReleaseKey(k) => r.actual.release_key(k),
                        button::Event::ReleaseMod(m) => r.actual.release_modifier(m),
                    }
                    break;
                }
            }
        },
        keypress_handler(&mut *events.subscriber, &mut *button_pub),
    )
    .await;
}

#[embassy_executor::task]
async fn report_task(
    stream: HidStream,
    handler: &'static Handler,
    reports: &'static Mutex<CS, Reports>,
) {
    if let Err(e) = keyboard_report(stream, handler, reports).await {
        log::error!("keyboard error: {e:?}");
    }
}

async fn keyboard_report(
    mut stream: HidStream,
    _handler: &'static Handler,
    reports: &'static Mutex<CS, Reports>,
) -> Result<(), Error> {
    stream.ready().await;
    loop {
        Timer::after(Duration::from_millis(2)).await;

        let report = {
            let mut reports = reports.lock().await;
            reports.unsent = EMPTY_KEYBOARD_REPORT;
            reports.actual
        };

        if report.keycodes != EMPTY_KEYBOARD_REPORT.keycodes {
            log::trace!("keys: {:x?}", report.keycodes);
        }

        #[cfg(feature = "n-key-rollover")]
        stream.write(report.as_bytes()).await?;

        #[cfg(not(feature = "n-key-rollover"))]
        stream.write_serialize(&report).await?;
    }
}

#[derive(Debug)]
enum Error {
    Read(ReadError),
    Endpoint(EndpointError),
}

impl From<ReadError> for Error {
    fn from(value: ReadError) -> Self {
        Error::Read(value)
    }
}
impl From<EndpointError> for Error {
    fn from(value: EndpointError) -> Self {
        Error::Endpoint(value)
    }
}
