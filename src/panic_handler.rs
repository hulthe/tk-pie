use core::panic::PanicInfo;

use embassy_executor::Executor;
use embassy_rp::gpio::{AnyPin, Level, Output, Pin};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

#[panic_handler]
fn panic_blink(_info: &PanicInfo) -> ! {
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();

    // SAFETY: we panicked, so no other code will be running.
    let p = unsafe { embassy_rp::Peripherals::steal() };

    EXECUTOR.init(Executor::new()).run(|spawner| {
        spawner.spawn(blink(p.PIN_11.degrade())).ok();
    });
}

#[embassy_executor::task]
async fn blink(led: AnyPin) {
    let mut led = Output::new(led, Level::High);

    loop {
        Timer::after(Duration::from_secs(1)).await;
        led.toggle();
    }
}
