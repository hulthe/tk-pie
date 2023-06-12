use core::cell::RefCell;

use critical_section::Mutex;
use rtt_target::{rtt_init, UpChannel};

pub type RttWriteFn = fn(&[u8]);

static CHANNEL: Mutex<RefCell<Option<UpChannel>>> = Mutex::new(RefCell::new(None));

/// Write directly to the rtt output. Must call [init_rtt_logger] first.
pub fn rtt_write(bytes: &[u8]) {
    critical_section::with(|cs| {
        let mut slot = CHANNEL.borrow_ref_mut(cs);
        if let Some(channel) = slot.as_mut() {
            channel.write(bytes);
        };
    });
}

pub fn init_rtt_logger() -> RttWriteFn {
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

        if slot.is_some() {
            return rtt_write;
        }

        *slot = Some(channels.up.0);

        rtt_write
    })
}
