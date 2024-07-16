use egui::{Button, Color32, Frame, Rect, ScrollArea, Stroke, Vec2};
use tk_pie::{layer::Layer, layout::Layout};

use crate::{
    edit_mode::{ButtonIdentifier, EditModeWrapper},
    mat::Mat,
    ron_utils::RonEdit,
    serial::SerialState,
};

use super::GuiSettings;

pub fn central_panel(
    ctx: &egui::Context,
    gui_settings: &GuiSettings,
    layout: &mut RonEdit<Layout>,
    edit_mode: &mut EditModeWrapper,
    layers: &mut Mat<RonEdit<Layer>>,
    serial: &mut SerialState,
) {
    let SerialState {
        scan_task: _,
        dev: _,
        reader: _,
        logs: _,
        active_layer,
    } = serial;

    let GuiSettings { u1, margin } = gui_settings;

    egui::CentralPanel::default().show(ctx, |ui| {
        ScrollArea::both().show(ui, |ui| {
            if let Some(edit) = edit_mode.edit_mode.as_mut() {
                ui.label("Edit mode active");

                if ui.button("deselect").clicked() {
                    edit.selected_button = None;
                }
            }

            for (i, layer) in layers.iter_cf().enumerate() {
                let x = (i / layers.height()) as u16;
                let y = (i % layers.height()) as u16;

                if Some((x, y)) == *active_layer {
                    ui.visuals_mut().widgets.inactive.bg_stroke = (1.0, Color32::DARK_GREEN).into();
                }

                Frame::none().show(ui, |ui| {
                    for (i, geometry) in layout.t.buttons.iter().enumerate() {
                        let margin = *margin * *u1;
                        let size = Vec2::new(
                            *u1 * geometry.w - margin * 2.0,
                            *u1 * geometry.h - margin * 2.0,
                        );
                        let offset =
                            Vec2::new(*u1 * geometry.x + margin, *u1 * geometry.y + margin);
                        let rect = Rect::from_min_size(ui.min_rect().min + offset, size);

                        let mut button = if let Some(button) = layer.t.buttons.get(i) {
                            Button::new(button.to_string())
                        } else {
                            Button::new("")
                        };

                        let current_button_selected = edit_mode
                            .edit_mode
                            .as_ref()
                            .map(|edit| {
                                edit.selected_button
                                    .as_ref()
                                    .map(|button_ident| {
                                        button_ident.layer_x == x
                                            && button_ident.layer_y == y
                                            && button_ident.index == i
                                    })
                                    .unwrap_or(false)
                            })
                            .unwrap_or(false);

                        if current_button_selected {
                            button = button.stroke(Stroke::new(2.0, Color32::YELLOW));
                        }

                        let ui_button = ui.put(rect, button);

                        if let Some(edit) = edit_mode.edit_mode.as_mut() {
                            if ui_button.clicked() {
                                edit.selected_button = Some(ButtonIdentifier {
                                    layer_x: x,
                                    layer_y: y,
                                    index: i,
                                })
                            }
                        }
                    }
                });

                ui.visuals_mut().widgets = Default::default();
                ui.separator();
            }
        })
    });
}
