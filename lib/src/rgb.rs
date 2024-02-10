use bytemuck::{cast_slice, Pod, Zeroable};
use core::{
    fmt::{self, Debug},
    ops::{Div, Mul},
};

/// An Rgb value that can be safely transmuted to u32 for use with Ws2812.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct Rgb(u32);

impl Rgb {
    #[inline(always)]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self(u32::from_be_bytes([g, r, b, 0]))
    }

    pub fn from_f64s(r: f64, g: f64, b: f64) -> Self {
        let r = r.clamp(0.0, 1.0) * 255.0;
        let g = g.clamp(0.0, 1.0) * 255.0;
        let b = b.clamp(0.0, 1.0) * 255.0;
        Self::new(r as u8, g as u8, b as u8)
    }

    pub fn from_f32s(r: f32, g: f32, b: f32) -> Self {
        let r = r.clamp(0.0, 1.0) * 255.0;
        let g = g.clamp(0.0, 1.0) * 255.0;
        let b = b.clamp(0.0, 1.0) * 255.0;
        Self::new(r as u8, g as u8, b as u8)
    }

    /// Get the red, green, and blue components of this Rgb.
    #[inline(always)]
    pub const fn components(&self) -> [u8; 3] {
        let [g, r, b, _] = self.0.to_be_bytes();
        [r, g, b]
    }

    #[inline(always)]
    pub fn slice_as_u32s(rgbs: &[Rgb]) -> &[u32] {
        cast_slice(rgbs)
    }
}

impl Debug for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [r, g, b] = self.components();
        f.debug_tuple("Rgb").field(&r).field(&g).field(&b).finish()
    }
}

impl Div<u8> for Rgb {
    type Output = Rgb;

    fn div(self, d: u8) -> Self::Output {
        let [r, g, b] = self.components();
        Rgb::new(r / d, g / d, b / d)
    }
}

impl Mul<f32> for Rgb {
    type Output = Rgb;

    fn mul(self, factor: f32) -> Self::Output {
        let [r, g, b] = self.components();
        Rgb::new(
            ((r as f32) * factor) as u8,
            ((g as f32) * factor) as u8,
            ((b as f32) * factor) as u8,
        )
    }
}

#[cfg(all(target_arch = "x86_64", test))]
mod tests {
    use super::*;
    use bytemuck::cast;

    #[test]
    fn test_rgb_as_u32() {
        let rgb = Rgb::new(0x11, 0x22, 0xCC);
        let rgb_u32: u32 = cast(rgb);
        assert_eq!(rgb_u32, 0x2211CC00);
    }
}
