use std::mem;

use serde::{Deserialize, Serialize};
use tk_pie::{button_layout::ButtonLayout, layer::Layer};

use crate::{mat::Mat, ron_utils::RonEdit};

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Layout {
    pub name: String,
    pub button_layout: RonEdit<ButtonLayout>,
    pub layers: Mat<RonEdit<Layer>>,
}

impl Default for Layout {
    fn default() -> Self {
        let layer_str = include_str!("default_layer.ron");
        let layer: Layer = ron::from_str(layer_str).expect("Failed to deserialize default layer");

        let layout_str = include_str!("default_layout.ron");
        let button_layout: ButtonLayout =
            ron::from_str(layout_str).expect("Failed to deserialize default layout");

        let mut layers = Mat::default();
        layers.push_row(RonEdit::new(layer));

        Self {
            name: "New layout".into(),
            // Avoid screwing up formatting.
            button_layout: RonEdit::from_parts(layout_str.into(), button_layout),
            layers,
        }
    }
}

impl Layout {
    pub fn duplicate(&self) -> Self {
        Self {
            name: format!("Duplicate of {}", self.name),
            button_layout: self.button_layout.clone(),
            layers: self.layers.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Layouts {
    pub active: Layout,
    pub alternatives: Vec<Layout>,
}

impl Layouts {
    pub fn switch_active(&mut self, new: Layout) -> Layout {
        mem::replace(&mut self.active, new)
    }
}
