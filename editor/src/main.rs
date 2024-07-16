mod edit_mode;
mod gui;
mod key_mapper;
mod mat;
mod ron_utils;
mod serial;

use clap::Parser;
use edit_mode::EditModeWrapper;
use eyre::eyre;
use gui::GuiSettings;
use mat::Mat;
use ron_utils::RonEdit;
use serde::{Deserialize, Serialize};
use serial::SerialState;
use std::{borrow::BorrowMut, path::PathBuf};
use tk_pie::{layer::Layer, layout::Layout};
use tokio::runtime::Runtime;

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
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
    .map_err(|e| eyre!("Failed to run program: {e:?}"))?;

    rt.shutdown_background();

    Ok(())
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Deserialize, Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    layout: RonEdit<Layout>,

    layers: Mat<RonEdit<Layer>>,

    edit_mode: EditModeWrapper,
    gui_settings: GuiSettings,

    #[serde(skip)]
    serial: SerialState,
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
        let layer: Layer = ron::from_str(layer_str).expect("Failed to deserialize default layer");

        let layout_str = include_str!("default_layout.ron");
        let layout: Layout =
            ron::from_str(layout_str).expect("Failed to deserialize default layout");

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

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        self.edit_mode
            .handle_input(self.layers.borrow_mut(), raw_input);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            layout,
            gui_settings,
            edit_mode,
            layers,
            serial,
        } = self;

        serial.check_serial();

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

        gui::side_panel::side_panel(ctx, serial, gui_settings, edit_mode, layout, layers);

        gui::central_panel::central_panel(ctx, gui_settings, layout, edit_mode, layers, serial);
    }
}
