use adafruit_itsy_bitsy_rp2040::hal::gpio::{Output, Pin, PinId, PushPull};
use cortex_m::delay::Delay;
use embedded_hal::digital::v2::OutputPin;

pub struct Led<I: PinId> {
    pub pin: Pin<I, Output<PushPull>>,
}

pub struct Blink {
    ms: u32,
}

pub const SHORT: Blink = Blink { ms: 200 };
pub const LONG: Blink = Blink { ms: 600 };

impl<I: PinId> Led<I> {
    pub fn dance(&mut self, delay: &mut Delay, moves: &[Blink]) {
        for m in moves {
            self.pin.set_high().unwrap();
            delay.delay_ms(m.ms);
            self.pin.set_low().unwrap();
            delay.delay_ms(100);
        }
    }
}
