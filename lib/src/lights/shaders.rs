use core::f32::consts::PI;

use embassy_time::{Duration, Instant};
use glam::{vec2, vec3, Vec3};
use libm::cosf;

use crate::rgb::Rgb;

/// A fragment shader.
pub trait Shader {
    /// Sample a normalized coordinate (0 to 1) using the shader function and return a color.
    fn sample(&self, time: Instant, uv: (f32, f32)) -> Rgb;

    fn end_time(&self) -> Option<Instant> {
        None
    }
}

pub enum Shaders {
    OrthoRainbow,
    LsdHyperspace,
}

impl Shader for Shaders {
    fn sample(&self, time: Instant, uv: (f32, f32)) -> Rgb {
        match self {
            Shaders::OrthoRainbow => OrthoRainbow.sample(time, uv),
            Shaders::LsdHyperspace => LsdHyperspace.sample(time, uv),
        }
    }
}

pub struct OrthoRainbow;
impl Shader for OrthoRainbow {
    fn sample(&self, time: Instant, (x, y): (f32, f32)) -> Rgb {
        let time = time.as_millis() as f32 / 1000.0;
        let r = 0.5 + 0.5 * cosf(time + x + 0.0);
        let g = 0.5 + 0.5 * cosf(time + y + 2.0);
        let b = 0.5 + 0.5 * cosf(time + x + 4.0);

        Rgb::from_f32s(r, g, b)
    }
}

pub struct LsdHyperspace;
impl Shader for LsdHyperspace {
    fn sample(&self, time: Instant, uv: (f32, f32)) -> Rgb {
        let time = time.as_millis() as f32 / 1000.0 * 3.0;
        let uv = vec2(uv.0, uv.1);

        let center = vec2(0.5, 0.5);

        let dist = (uv - center).length();

        let fac = dist * PI + vec3(0.0, 1.0, 4.0) - time;
        let col = cos3(fac) * cos3(fac * 0.5);

        Rgb::from_f32s(col.x, col.y, col.z)
    }
}

pub struct PowerOffAnim {
    /// Animation starting time.
    pub start: Instant,
}

impl PowerOffAnim {
    const FADE_FACTOR: f32 = 0.5;

    /// Animation duration, in seconds.
    const DURATION_SEC: u16 = 5;
}

impl Shader for PowerOffAnim {
    fn sample(&self, time: Instant, (_, y): (f32, f32)) -> Rgb {
        let time = time.as_millis().saturating_sub(self.start.as_millis());
        let time = time as f32 / 1000.0;

        let duration: f32 = Self::DURATION_SEC.into();
        let r = Self::FADE_FACTOR * (duration - time - 1.0 + y);
        Rgb::from_f32s(r, 0.0, 0.0)
    }

    fn end_time(&self) -> Option<Instant> {
        Some(self.start + Duration::from_secs(Self::DURATION_SEC.into()))
    }
}

pub struct PowerOnAnim {
    /// Animation starting time.
    pub start: Instant,
}

impl PowerOnAnim {
    const FADE_FACTOR: f32 = 1.0;

    /// Animation duration, in seconds.
    const DURATION_SEC: u16 = 1;
}

fn cos3(v: Vec3) -> Vec3 {
    vec3(cosf(v.x), cosf(v.y), cosf(v.z))
}

impl Shader for PowerOnAnim {
    fn sample(&self, time: Instant, (_, y): (f32, f32)) -> Rgb {
        let time = time.as_millis().saturating_sub(self.start.as_millis());
        let time = time as f32 / 1000.0;

        let duration: f32 = Self::DURATION_SEC.into();
        let g = Self::FADE_FACTOR * (duration - time - y);
        Rgb::from_f32s(0.0, g, 0.0)
    }
    fn end_time(&self) -> Option<Instant> {
        Some(self.start + Duration::from_secs(Self::DURATION_SEC.into()))
    }
}
