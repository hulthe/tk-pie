use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::{Duration, Timer};

use crate::ws2812::Rgb;

pub type CS = CriticalSectionRawMutex;

pub async fn stall() -> ! {
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

/// Input a value 0 to 255 to get a color value
// The colours are a transition r - g - b - back to r.
pub fn wheel(mut wheel_pos: u8) -> Rgb {
    wheel_pos = 255 - wheel_pos;
    if wheel_pos < 85 {
        return Rgb::new(255 - wheel_pos * 3, 0, wheel_pos * 3);
    }
    if wheel_pos < 170 {
        wheel_pos -= 85;
        return Rgb::new(0, wheel_pos * 3, 255 - wheel_pos * 3);
    }
    wheel_pos -= 170;
    Rgb::new(wheel_pos * 3, 255 - wheel_pos * 3, 0)
}
