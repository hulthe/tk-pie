use core::{fmt::Write, sync::atomic::Ordering};

use atomic_polyfill::AtomicBool;
use embassy_time::Instant;
use log::{Metadata, Record};
use static_cell::StaticCell;

pub const LOGGER_OUTPUTS: usize = 1;

pub struct Logger {
    pub outputs: [fn(&[u8]); LOGGER_OUTPUTS],
}

impl Logger {
    /// Set this as the global logger.
    ///
    /// Calling this function more than once does nothing.
    pub fn init(self) {
        // guard against calling this multiple times
        static INITIALIZED: AtomicBool = AtomicBool::new(false);
        if INITIALIZED.fetch_or(true, Ordering::SeqCst) {
            return;
        }

        static LOGGER: StaticCell<Logger> = StaticCell::new();
        let logger = LOGGER.init(self);
        unsafe {
            log::set_logger_racy(logger).unwrap();
            log::set_max_level_racy(log::LevelFilter::Debug);
        }
    }
}

impl log::Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let now = Instant::now();
            let s = now.as_secs();
            let ms = now.as_millis() % 1000;
            let level = record.metadata().level();
            let mut w = &mut Writer(self);
            let _ = writeln!(&mut w, "[{s}.{ms:04}] ({level}) {}", record.args());
        }
    }

    fn flush(&self) {}
}

struct Writer<'a>(&'a Logger);

impl Write for Writer<'_> {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        for output in &self.0.outputs {
            output(s.as_bytes());
        }
        Ok(())
    }
}
