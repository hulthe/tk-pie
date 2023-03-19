pub mod report;

use embassy_executor::Spawner;
use embassy_rp::{peripherals::USB, usb::Driver};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use embassy_usb::{
    class::hid::{self, HidReaderWriter, ReadError, ReportId, RequestHandler},
    control::OutResponse,
    Builder,
};
use embassy_usb_driver::EndpointError;
use static_cell::StaticCell;
use usbd_hid::descriptor::{MouseReport, SerializedDescriptor};

use crate::{
    usb::keyboard::report::{KeyboardReport, EMPTY_KEYBOARD_REPORT},
    util::CS,
};

use super::MAX_PACKET_SIZE;

struct Handler;

static CONTEXT: StaticCell<Context> = StaticCell::new();

pub static KB_REPORT: Mutex<CS, KeyboardReport> = Mutex::new(EMPTY_KEYBOARD_REPORT);

struct Context {
    handler: Handler,
    state: hid::State<'static>,
}

pub async fn setup(builder: &mut Builder<'static, Driver<'static, USB>>) {
    log::info!("setting up usb hid");

    let context = CONTEXT.init(Context {
        handler: Handler,
        state: hid::State::new(),
    });

    let config = hid::Config {
        //report_descriptor: MouseReport::desc(),
        report_descriptor: KeyboardReport::desc(),
        request_handler: Some(&context.handler),
        poll_ms: 2,
        max_packet_size: MAX_PACKET_SIZE as u16,
    };

    let stream = HidStream::new(builder, &mut context.state, config);

    let spawner = Spawner::for_current_executor().await;

    spawner.must_spawn(task(stream, &context.handler));

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
async fn task(stream: HidStream, handler: &'static Handler) {
    if let Err(e) = keyboard_test(stream, handler).await {
        log::error!("keyboard error: {e:?}");
    }
    //if let Err(e) = mouse_wiggler(stream).await {
    //    log::error!("mouse wiggler: {e:?}");
    //}
}

async fn keyboard_test(mut stream: HidStream, _handler: &'static Handler) -> Result<(), Error> {
    stream.ready().await;
    loop {
        Timer::after(Duration::from_millis(2)).await;

        let report = KB_REPORT.lock().await.clone();
        if report.keycodes != EMPTY_KEYBOARD_REPORT.keycodes {
            log::trace!("keys: {:x?}", report.keycodes);
        }

        #[cfg(feature = "n-key-rollover")]
        stream.write(report.as_bytes()).await?;

        #[cfg(not(feature = "n-key-rollover"))]
        stream.write_serialize(&report).await?;
    }
}

#[allow(dead_code)]
async fn mouse_wiggler(mut stream: HidStream) -> Result<(), Error> {
    stream.ready().await;

    let (_r, mut w) = stream.split();

    let write_fut = async move {
        let mut x = 1;
        loop {
            for _ in 0..100 {
                Timer::after(Duration::from_millis(10)).await;
                log::info!("sending mouse report");
                //w.ready().await;
                w.write_serialize(&MouseReport {
                    x,
                    y: 0,
                    buttons: 0,
                    wheel: 0,
                    pan: 0,
                })
                .await?;
            }
            x = -x;
        }
    };

    //let read_fut = async move {
    //    let mut buf = [0u8; MAX_PACKET_SIZE as usize];
    //    loop {
    //        Timer::after(Duration::from_millis(30)).await;
    //        let n = r.read(&mut buf).await?;
    //        log::info!("got packet: {:?}", &buf[..n]);
    //    }
    //};

    //let r: Result<((), ()), Error> = try_join(write_fut, read_fut).await;
    let r: Result<(), Error> = write_fut.await;
    r?;

    Ok(())
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
