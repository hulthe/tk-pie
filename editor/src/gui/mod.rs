use serde::{Deserialize, Serialize};

pub mod central_panel;
pub mod side_panel;

#[derive(Deserialize, Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct GuiSettings {
    u1: f32,
    margin: f32,
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            u1: 75.0,
            margin: 0.05,
        }
    }
}
