use egui::Event;
use serde::{Deserialize, Serialize};
use tangentbord1::layer::Layer;

use crate::{key_mapper::map_key, mat::Mat, RonEdit};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ButtonIdentifier {
    pub layer_x: u16,
    pub layer_y: u16,
    pub index: usize,
}

#[derive(Deserialize, Serialize, Default)]
pub struct EditMode {
    pub selected_button: Option<ButtonIdentifier>,
}

// Used to allow for modifications of this value without borrowing the parent object.
#[derive(Deserialize, Serialize, Default)]
pub struct EditModeWrapper {
    pub edit_mode: Option<EditMode>,
}

impl EditModeWrapper {
    pub fn toggle_edit_mode(&mut self) {
        self.edit_mode = if self.edit_mode.is_some() {
            None
        } else {
            Some(EditMode::default())
        };
    }

    pub fn handle_input(&mut self, layers: &mut Mat<RonEdit<Layer>>, raw_input: &egui::RawInput) {
        let Some(edit) = self.edit_mode.as_mut() else {
            return;
        };

        let Some(selected_button) = edit.selected_button.as_mut() else {
            return;
        };

        let Some(key) = raw_input
            .events
            .iter()
            .filter_map(|event| match event {
                Event::Key {
                    key,
                    physical_key,
                    pressed,
                    repeat: _,
                    modifiers: _,
                } => match (physical_key, pressed) {
                    (Some(physical_key), true) => Some(physical_key),
                    (None, _) => {
                        log::warn!(
                            "Physical key was NONE? Not sure what this means, key was: {key:?}"
                        );
                        None
                    }
                    _ => None,
                },
                _ => None,
            })
            .next()
        else {
            return;
        };

        let Some(layer) = layers.get_mut(
            selected_button.layer_x as usize,
            selected_button.layer_y as usize,
        ) else {
            log::warn!("Selected layer did not exist?");
            return;
        };

        if let Some(key) = map_key(key) {
            if selected_button.index >= layer.t.buttons.len() {
                while selected_button.index >= layer.t.buttons.len() {
                    layer.t.buttons.push(tangentbord1::button::Button::None);
                }
            }
            // Index is now guaranteed to exist.
            layer.t.buttons[selected_button.index] = key;
            match ron::ser::to_string_pretty(&layer.t, ron::ser::PrettyConfig::default()) {
                Ok(new) => layer.ron = new,
                Err(e) => log::error!("Failed to serialize new ron, error: {e:?}"),
            }
        }

        edit.selected_button = None;
    }
}
