use egui::{Color32, ScrollArea, Slider, TextEdit, Ui};
use tk_pie::{button_layout::ButtonLayout, layer::Layer};
use tokio::{sync::oneshot, task::spawn_blocking};

use crate::{
    edit_mode::EditModeWrapper,
    layouts::{Layout, Layouts},
    mat::Mat,
    ron_utils::RonEdit,
    serial::{connect_to_serial, scan_for_serial, SerialState},
};

use super::GuiSettings;

pub fn side_panel(
    ctx: &egui::Context,
    serial: &mut SerialState,
    gui_settings: &mut GuiSettings,
    edit_mode: &mut EditModeWrapper,
    layouts: &mut Layouts,
) {
    let GuiSettings { u1, margin } = gui_settings;

    egui::SidePanel::left("side_panel")
        .resizable(true)
        .width_range(250.0..=500.0)
        .show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Side Panel");

                sidepanel_serial(ctx, serial, ui);

                ui.label("u1");
                ui.add(Slider::new(u1, 20.0..=150.0));
                ui.label("margin");
                ui.add(Slider::new(margin, 0.0..=0.4));

                if ui.button("Edit mode").clicked() {
                    edit_mode.toggle_edit_mode();
                }

                layouts_input(layouts, ui);
            });
        });
}

fn sidepanel_serial(ctx: &egui::Context, serial: &mut SerialState, ui: &mut Ui) {
    let SerialState {
        scan_task: scan_serial_task,
        dev: serial_devs,
        reader: serial_reader,
        logs: serial_logs,
        active_layer: _,
    } = serial;

    ui.collapsing("Serial", |ui| {
        match serial_devs {
            Ok(Some(dev)) => {
                if serial_reader.is_none() && ui.button(format!("Connect to {dev:?}")).clicked() {
                    *serial_reader = Some(connect_to_serial(dev.clone(), ctx.clone()));
                };
            }
            Ok(None) => {
                ui.label("No devices found.");
            }
            Err(e) => {
                ui.code_editor(e);
            }
        }

        if ui.button("scan for serial device").clicked() && scan_serial_task.is_none() {
            let (tx, rx) = oneshot::channel();
            let ctx = ctx.clone();
            spawn_blocking(move || {
                let r = scan_for_serial().map_err(|e| e.to_string());
                let _ = tx.send(r);
                ctx.request_repaint();
            });

            *scan_serial_task = Some(rx);
        }

        if scan_serial_task.is_some() {
            ui.label("Scanning...");
        }

        ScrollArea::both().show(ui, |ui| {
            for log in &*serial_logs {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(&log.level);
                        ui.label(&log.message);
                    });
                });
            }
        })
    });
}

fn layouts_input(layout: &mut Layouts, ui: &mut Ui) {
    ui.collapsing("Current layout", |ui| {
        let Layout {
            name,
            button_layout,
            layers,
        } = &mut layout.active;

        ui.text_edit_singleline(name);

        layout_input(button_layout, ui);

        layers_input(layers, ui);

        if ui.button("Add layer row").clicked() {
            layers.push_row(RonEdit::default());
        }

        ui.menu_button("Delete layer row", |ui| {
            for r in 0..layers.height() {
                if ui.button(format!("row {r}")).clicked() {
                    layers.remove_row(r);
                }
            }
        });

        if ui.button("Add layer column").clicked() {
            layers.push_col(RonEdit::default());
        }

        ui.menu_button("Delete layer column", |ui| {
            for c in 0..layers.width() {
                if ui.button(format!("column {c}")).clicked() {
                    layers.remove_col(c);
                }
            }
        });
    });

    ui.collapsing("Alternative layouts", |ui| {
        if layout.alternatives.is_empty() {
            ui.label("No alternative layouts");
        } else {
            for (num, alt) in layout.alternatives.iter().enumerate() {
                ui.label(format!("{num} - {}", alt.name));
            }
        }

        ui.menu_button("Change active", |ui| {
            for num in 0..layout.alternatives.len() {
                if ui.button(format!("layout {num}")).clicked() {
                    let new = layout.alternatives.remove(num);
                    let old = layout.switch_active(new);
                    layout.alternatives.push(old);
                }
            }
        });

        if ui.button("Add layout").clicked() {
            layout.alternatives.push(Layout::default());
        };

        ui.menu_button("Duplicate layout", |ui| {
            if ui.button("active layout").clicked() {
                let new = layout.active.duplicate();
                layout.alternatives.push(new);
            }
            for num in 0..layout.alternatives.len() {
                if ui.button(format!("layout {num}")).clicked() {
                    let new = layout.alternatives[num].duplicate();
                    layout.alternatives.push(new);
                }
            }
        });

        ui.menu_button("Remove layout", |ui| {
            for num in 0..layout.alternatives.len() {
                if ui.button(format!("layout {num}")).clicked() {
                    layout.alternatives.remove(num);
                }
            }
        });
    });
}

fn layout_input(layout: &mut RonEdit<ButtonLayout>, ui: &mut Ui) {
    ui.collapsing("Layout", |ui| {
        if ui
            .add(TextEdit::multiline(&mut layout.ron).code_editor())
            .changed()
        {
            layout.error.clear();
            match ron::from_str(&layout.ron) {
                Ok(new) => layout.t = new,
                Err(e) => layout.error = e.to_string(),
            }
        }

        if !layout.error.is_empty() {
            ui.add(
                TextEdit::multiline(&mut layout.error)
                    .interactive(false)
                    .text_color(Color32::RED),
            );
        }
    });
}

fn layers_input(layers: &mut Mat<RonEdit<Layer>>, ui: &mut Ui) {
    for x in 0..layers.width() {
        for y in 0..layers.height() {
            let layer = layers.get_mut(x, y).unwrap();
            ui.collapsing(format!("Layer {x},{y}"), |ui| {
                if ui
                    .add(TextEdit::multiline(&mut layer.ron).code_editor())
                    .changed()
                {
                    layer.error.clear();
                    match ron::from_str(&layer.ron) {
                        Ok(new) => layer.t = new,
                        Err(e) => layer.error = e.to_string(),
                    }
                }

                if !layer.error.is_empty() {
                    ui.add(
                        TextEdit::multiline(&mut layer.error)
                            .interactive(false)
                            .text_color(Color32::RED),
                    );
                }
            });
        }
    }
}
