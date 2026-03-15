pub mod shaders;

use core::future::Future;
use embassy_sync::mutex::Mutex;

use crate::{rgb::Rgb, util::CS};

pub struct Lights<D: LightDriver, const N: usize> {
    state: Mutex<CS, State<D, N>>,
}

struct State<D, const N: usize> {
    colors: [Rgb; N],
    driver: D,
}

pub trait LightDriver {
    fn write(&mut self, colors: &[Rgb]) -> impl Future<Output = ()>;
}

impl<D: LightDriver, const N: usize> Lights<D, N> {
    pub const fn new(driver: D) -> Self {
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
