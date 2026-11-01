//! Fenkra — CAD paramétrico open source.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod examples;
mod theme;
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
mod handles;
mod gizmo;
mod asm_ui;
mod st_ui;
mod pmi_ui;
mod slots;
mod geom_fit;
mod extract;
mod pmi_draw;
mod palette;
mod roles;
mod ui_ext;
mod sketch_ext;
mod dwg_ui;
mod measure;
mod i18n;

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};

/// Configuração padrão do egui, mais os formatos específicos do adaptador no dispositivo: sem eles o
/// anti-serrilhamento só pode ser 1× ou 4× (2× e 8× dependem dessa feature).
fn wgpu_options() -> egui_wgpu::WgpuConfiguration {
    let mut cfg = egui_wgpu::WgpuConfiguration::default();
    if let egui_wgpu::WgpuSetup::CreateNew(novo) = &mut cfg.wgpu_setup {
        novo.device_descriptor = std::sync::Arc::new(|adapter| {
            let base = if adapter.get_info().backend == wgpu::Backend::Gl { wgpu::Limits::downlevel_webgl2_defaults() } else { wgpu::Limits::default() };
            wgpu::DeviceDescriptor {
                label: Some("fenkra wgpu device"),
                required_features: adapter.features() & wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                required_limits: wgpu::Limits { max_texture_dimension_2d: 8192, ..base },
                ..Default::default()
            }
        });
    }
    cfg
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1360., 780.]).with_maximized(true).with_min_inner_size([900., 600.]).with_title(concat!("Fenkra CAD ", env!("CARGO_PKG_VERSION")))
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/app/fenix-256.png")).unwrap_or_default()),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: wgpu_options(),
        ..Default::default()
    };
    eframe::run_native("Fenkra", opts, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
