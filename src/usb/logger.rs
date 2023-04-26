use crate::util::CS;

use super::MAX_PACKET_SIZE;
use core::fmt::Write as WriteFmt;
use embassy_executor::Spawner;
use embassy_rp::{peripherals::USB, usb::Driver};
use embassy_sync::pipe::Pipe;
use embassy_time::Instant;
use embassy_usb::{
    class::cdc_acm::{self, CdcAcmClass},
    Builder,
};
use log::{Metadata, Record};
use static_cell::StaticCell;

pub const BUFFER_SIZE: usize = 16 * 1024;
static BUFFER: Pipe<CS, BUFFER_SIZE> = Pipe::new();

struct UsbLogger;

pub async fn setup(usb_builder: &mut Builder<'static, Driver<'static, USB>>) {
    unsafe {
        static LOGGER: UsbLogger = UsbLogger;
        log::set_logger_racy(&LOGGER).unwrap();
    }
    log::set_max_level(log::LevelFilter::Debug);

    let spawner = Spawner::for_current_executor().await;

    static STATE: StaticCell<cdc_acm::State<'static>> = StaticCell::new();
    let state = STATE.init(cdc_acm::State::new());

    let class = CdcAcmClass::new(usb_builder, state, MAX_PACKET_SIZE as u16);

    spawner.must_spawn(log_task(class));
}

#[embassy_executor::task]
async fn log_task(mut class: CdcAcmClass<'static, Driver<'static, USB>>) {
    let mut buf = [0u8; MAX_PACKET_SIZE as usize];

    class.wait_connection().await;
    loop {
        let n = BUFFER.read(&mut buf).await;

        // not much we can do if this fails, just ignore the error
        let _ = class.write_packet(&buf[..n]).await;
    }
}

impl log::Log for UsbLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut w = Writer;
            let now = Instant::now();
            let s = now.as_secs();
            let ms = now.as_millis() % 1000;
            let level = record.metadata().level();
            let _ = writeln!(w, "[{s}.{ms:04}] ({level}) {}", record.args());
        }
    }

    fn flush(&self) {}
}

struct Writer;

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        let _ = BUFFER.try_write(s.as_bytes());
        Ok(())
    }
}
