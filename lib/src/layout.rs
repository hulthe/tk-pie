use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Layout {
    pub buttons: Vec<Rect>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,

    #[serde(default = "default_width")]
    pub w: f32,
    #[serde(default = "default_height")]
    pub h: f32,
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: default_width(),
            h: default_height(),
        }
    }
}

fn default_width() -> f32 {
    1.0
}
fn default_height() -> f32 {
    1.0
}
