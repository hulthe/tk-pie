pub mod shaders;

use crate::ws2812::Ws2812;
use embassy_rp::pio;
use embassy_sync::mutex::Mutex;

use crate::{rgb::Rgb, util::CS};

pub struct Lights<P: pio::Instance + 'static, const N: usize> {
    state: Mutex<CS, State<P, N>>,
}

struct State<P: pio::Instance + 'static, const N: usize> {
    colors: [Rgb; N],
    driver: Ws2812<P>,
}

impl<P: pio::Instance, const N: usize> Lights<P, N> {
    pub const fn new(driver: Ws2812<P>) -> Self {
        Lights {
            state: Mutex::new(State {
                colors: [Rgb::new(0, 0, 0); N],
                driver,
            }),
        }
    }

    pub fn colors_mut(&mut self) -> &[Rgb; N] {
        &self.state.get_mut().colors
    }

    /// Run a function to update the colors, and then immediately refresh the LEDs.
    pub async fn update(&self, f: impl FnOnce(&mut [Rgb; N])) {
        let State { colors, driver } = &mut *self.state.lock().await;
        f(colors);
        driver.write(colors).await
    }

    /// Update the LEDs with the currently set colors.
    pub async fn refresh(&self) {
        self.update(|_| ()).await;
    }
}
