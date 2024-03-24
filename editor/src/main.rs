mod mat;
mod serial;

use clap::Parser;
use egui::{Button, Color32, Frame, Rect, ScrollArea, Slider, TextEdit, Vec2};
use eyre::eyre;
use mat::Mat;
use serde::{Deserialize, Serialize};
use serial::{connect_to_serial, scan_for_serial};
use std::{collections::VecDeque, path::PathBuf};
use tangentbord1::{
    layer::Layer,
    layout::Layout,
    serial_proto::owned::{ChangeLayer, DeviceMsg, LogRecord},
};
use tokio::{
    runtime::Runtime,
    sync::{
        mpsc::{self, Receiver},
        oneshot::{self, error::TryRecvError},
    },
    task::spawn_blocking,
};

#[derive(Parser)]
struct Opt {
    /// Path to device serial port
    device: Option<PathBuf>,
}

fn main() -> eyre::Result<()> {
    let _opt = Opt::parse();

    pretty_env_logger::init();

    let rt = Runtime::new().unwrap();
    let _enter_rt = rt.enter();

    log::info!("starting application");
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "tgnt keyboard editor",
        native_options,
        Box::new(|cc| Box::new(App::new(cc))),
    )
    .map_err(|e| eyre!("Failed to run program: {e:?}"))?;

    rt.shutdown_background();

    Ok(())
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Deserialize, Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    layout: RonEdit<Layout>,

    layers: Mat<RonEdit<Layer>>,

    u1: f32,
    margin: f32,

    #[serde(skip)]
    serial: SerialState,
}

struct SerialState {
    scan_task: Option<oneshot::Receiver<Result<Option<PathBuf>, String>>>,
    dev: Result<Option<PathBuf>, String>,
    reader: Option<Receiver<DeviceMsg>>,
    logs: VecDeque<LogRecord>,
    active_layer: Option<(u16, u16)>,
}

#[derive(Deserialize, Serialize, Clone)]
struct RonEdit<T> {
    pub t: T,
    pub ron: String,
    pub error: String,
}

impl<T: Serialize> RonEdit<T> {
    pub fn new(t: T) -> Self {
        RonEdit {
            ron: ron::ser::to_string_pretty(&t, Default::default()).unwrap(),
            error: String::new(),
            t,
        }
    }
}

impl<T: Default + Serialize> Default for RonEdit<T> {
    fn default() -> Self {
        RonEdit::new(Default::default())
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            layers: Default::default(),
            layout: Default::default(),
            u1: 75.0,
            margin: 0.05,

            serial: Default::default(),
        }
    }
}

impl Default for SerialState {
    fn default() -> Self {
        Self {
            scan_task: Default::default(),
            dev: Ok(None),
            reader: Default::default(),
            logs: Default::default(),
            active_layer: None,
        }
    }
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        let layer_str = include_str!("default_layer.ron");
        let layer: Layer = ron::from_str(&layer_str).expect("Failed to deserialize default layer");

        let layout_str = include_str!("default_layout.ron");
        let layout: Layout =
            ron::from_str(&layout_str).expect("Failed to deserialize default layout");

        let mut layers = Mat::default();
        layers.push_row(RonEdit::new(layer));

        Self {
            layers,
            layout: RonEdit::new(layout),
            ..Self::default()
        }
    }
}

impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            margin,
            u1,
            layout,
            layers,
            serial:
                SerialState {
                    scan_task: scan_serial_task,
                    dev: serial_devs,
                    reader: serial_reader,
                    logs: serial_logs,
                    active_layer,
                },
        } = self;

        while let Some(rx) = scan_serial_task {
            match rx.try_recv() {
                Ok(r) => {
                    *serial_devs = r;
                    *scan_serial_task = None;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Closed) => {
                    *scan_serial_task = None;
                    break;
                }
            }
        }

        while let Some(rx) = serial_reader {
            match rx.try_recv() {
                Ok(DeviceMsg::Log(record)) => {
                    serial_logs.push_back(record);
                    if serial_logs.len() > 10 {
                        serial_logs.pop_front();
                    }
                }
                Ok(DeviceMsg::ChangeLayer(ChangeLayer { x, y })) => {
                    *active_layer = Some((x, y));
                }
                Ok(_) => {}
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    *serial_reader = None;
                    break;
                }
            }
        }

        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        todo!("implement quit")
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel")
            .resizable(true)
            .width_range(250.0..=500.0)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Side Panel");

                    ui.collapsing("Serial", |ui| {
                        match serial_devs {
                            Ok(Some(dev)) => {
                                if serial_reader.is_none()
                                    && ui.button(format!("Connect to {dev:?}")).clicked()
                                {
                                    *serial_reader =
                                        Some(connect_to_serial(dev.clone(), ctx.clone()));
                                };
                            }
                            Ok(None) => {
                                ui.label("No devices found.");
                            }
                            Err(e) => {
                                ui.code_editor(e);
                            }
                        }

                        if ui.button("scan for serial device").clicked()
                            && scan_serial_task.is_none()
                        {
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

                    ui.label("u1");
                    ui.add(Slider::new(u1, 20.0..=150.0));
                    ui.label("margin");
                    ui.add(Slider::new(margin, 0.0..=0.4));

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
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::both().show(ui, |ui| {
                for (i, layer) in layers.iter_cf().enumerate() {
                    let x = (i / layers.height()) as u16;
                    let y = (i % layers.height()) as u16;

                    if Some((x, y)) == *active_layer {
                        ui.visuals_mut().widgets.inactive.bg_stroke =
                            (1.0, Color32::DARK_GREEN).into();
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

                            let button = if let Some(button) = layer.t.buttons.get(i) {
                                Button::new(button.to_string())
                            } else {
                                Button::new("")
                            };

                            ui.put(rect, button);
                        }
                    });

                    ui.visuals_mut().widgets = Default::default();
                    ui.separator();
                }
            })
        });
    }
}
