use core::{cell::RefCell, fmt::Write};

use critical_section::Mutex;
use rtt_target::{rtt_init, UpChannel};

use crate::logger::{LogOutput, TimestampedRecord};

pub type RttWriteFn = fn(&str);

static CHANNEL: Mutex<RefCell<Option<UpChannel>>> = Mutex::new(RefCell::new(None));

/// Write directly to the rtt output. Must call [init_rtt_logger] first.
pub fn rtt_write(s: &str) {
    critical_section::with(|cs| {
        let mut slot = CHANNEL.borrow_ref_mut(cs);
        if let Some(channel) = slot.as_mut() {
            channel.write(s.as_bytes());
        };
    });
}

pub fn init_rtt_logger() -> &'static RttLogger {
    let channels = rtt_init! {
        up: {
            0: {
                size: 1024
                mode: NoBlockSkip
                name: "Terminal"
            }
        }
    };

    critical_section::with(|cs| {
        let mut slot = CHANNEL.borrow_ref_mut(cs);

        if slot.is_none() {
            *slot = Some(channels.up.0);
        }

        static RTT_LOGGER: RttLogger = RttLogger;
        &RTT_LOGGER
    })
}

pub struct RttLogger;

impl LogOutput for RttLogger {
    fn log(&self, record: &TimestampedRecord) {
        let s = record.timestamp.as_secs();
        let ms = record.timestamp.as_millis() % 1000;
        let level = record.level();
        let mut w = &mut Writer;
        let _ = writeln!(&mut w, "[{s}.{ms:04}] ({level}) {}", record.args());
    }
}

struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        rtt_write(s);
        Ok(())
    }
}
