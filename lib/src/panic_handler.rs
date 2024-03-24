use core::{fmt::Write, panic::PanicInfo};

use cortex_m::asm;
use embassy_rp::gpio::{Level, Output};

use crate::rtt::rtt_write;

#[panic_handler]
fn panic_blink(info: &PanicInfo) -> ! {
    cortex_m::interrupt::disable();

    let _ = write!(&mut Writer, "{info}");

    // SAFETY: we panicked, so no other code will be running.
    let p = unsafe { embassy_rp::Peripherals::steal() };
    let _led = Output::new(p.PIN_11, Level::High);

    asm::udf()
}

/// Write to RTT
struct Writer;

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        rtt_write(s);
        Ok(())
    }
}
