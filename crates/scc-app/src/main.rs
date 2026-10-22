//! SolidCadCraft — CAD paramétrico open source.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod icons;
mod settings;
mod toolbar;
mod editor;
mod render;
mod scene;
mod picking;
mod viewcube;
mod sketch_ui;
mod geo_ui;
mod keymap;
mod feat_ui;

use eframe::egui;

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1360., 780.]).with_maximized(true).with_min_inner_size([900., 600.]).with_title(concat!("SolidCadCraft ", env!("CARGO_PKG_VERSION")))
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/app/fenix-256.png")).unwrap_or_default()),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native("SolidCadCraft", opts, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
