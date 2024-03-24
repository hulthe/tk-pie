use core::{fmt::Arguments, sync::atomic::Ordering};

use embassy_time::Instant;
use log::{Level, Metadata, Record};
use portable_atomic::AtomicBool;
use static_cell::StaticCell;

pub const LOGGER_OUTPUTS: usize = 2;

/// A logger which timestamps logs and forwards them to mutiple other loggers.
pub struct LogMultiplexer {
    pub outputs: [&'static dyn LogOutput; LOGGER_OUTPUTS],
}

pub struct TimestampedRecord<'a> {
    pub record: Record<'a>,

    /// Timestamp
    pub timestamp: Instant,
}

impl LogMultiplexer {
    /// Set this as the global logger.
    ///
    /// Calling this function more than once does nothing.
    pub fn init(self) {
        // guard against calling this multiple times
        static INITIALIZED: AtomicBool = AtomicBool::new(false);
        if INITIALIZED.fetch_or(true, Ordering::SeqCst) {
            return;
        }

        static LOGGER: StaticCell<LogMultiplexer> = StaticCell::new();
        let logger = LOGGER.init(self);
        unsafe {
            log::set_logger_racy(logger).unwrap();
            log::set_max_level_racy(log::LevelFilter::Debug);
        }
    }
}

impl log::Log for LogMultiplexer {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let timestamp = Instant::now();

            let record = TimestampedRecord {
                timestamp,
                record: record.clone(),
            };

            for output in &self.outputs {
                output.log(&record);
            }
        }
    }

    fn flush(&self) {}
}

pub trait LogOutput: Send + Sync {
    fn log(&self, record: &TimestampedRecord);
}

impl TimestampedRecord<'_> {
    pub fn args(&self) -> &Arguments<'_> {
        self.record.args()
    }

    pub fn level(&self) -> Level {
        self.metadata().level()
    }

    pub fn metadata(&self) -> &Metadata<'_> {
        self.record.metadata()
    }
}
