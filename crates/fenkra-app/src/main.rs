// SPDX-License-Identifier: LGPL-2.1-or-later
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
mod entradas;
mod previa;
mod sup3d;
mod picking;
mod selecao;
mod viewcube;
mod sketch_ui;
mod geo_ui;
mod keymap;
mod feat_ui;
mod handles;
mod chapa_manip;
mod chapa_manip_ui;
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
mod tree_ui;
mod historico;
mod diagnostico;
mod paineis;
mod partida;
mod ajuda_cmd;
mod textos_ajuda;
mod textos_biblioteca;
mod biblioteca_ui;
mod sketch_fluxo;
mod perfil;
mod relatorio;
#[cfg(test)]
mod padrao;

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

fn opcoes(modo: partida::Modo) -> eframe::NativeOptions {
    let mut wgpu_options = wgpu_options();
    partida::aplicar_modo(&mut wgpu_options, modo);
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1360., 780.]).with_maximized(true).with_min_inner_size([900., 600.]).with_title(concat!("Fenkra CAD ", env!("CARGO_PKG_VERSION")))
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/app/fenix-256.png")).unwrap_or_default()),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options,
        ..Default::default()
    }
}

/// Abre a janela tentando, na ordem, o adaptador padrão, Vulkan, OpenGL e o modo seguro. Cada falha
/// vai para o `partida.log`; se nenhuma funcionar, uma caixa de mensagem explica o que aconteceu.
fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    partida::iniciar();
    if args.iter().any(|a| a == "--diagnostico") {
        partida::diagnostico();
        return Ok(());
    }
    let seguro = args.iter().any(|a| a == "--seguro");
    let forcado = std::env::var("WGPU_BACKEND").ok().filter(|v| !v.is_empty());
    let mut erros: Vec<(partida::Modo, String)> = Vec::new();
    for modo in partida::Modo::tentativas(seguro, forcado.is_some()) {
        partida::registrar("tentativa", modo.nome());
        let r = eframe::run_native(
            "Fenkra",
            opcoes(modo),
            Box::new(move |cc| {
                let mut app = app::App::new(cc);
                if let Some(rs) = &cc.wgpu_render_state {
                    let hw = perfil::detectar(rs);
                    app.avaliar_perfil(&hw);
                    if partida::registrar_adaptador(rs, modo) {
                        crate::i18n::set_english(app.settings.english);
                        app.toast(crate::i18n::tr("Adaptador gráfico por software: o Fenkra abre, mas fica lento. Atualize o driver da placa de vídeo."), true);
                    }
                }
                Ok(Box::new(app))
            }),
        );
        match r {
            Ok(()) => {
                partida::registrar("fim", "encerrado normalmente");
                return Ok(());
            }
            Err(e) => erros.push((modo, e.to_string())),
        }
    }
    partida::falha_de_gpu(&erros, forcado);
    match erros.pop() {
        Some((_, e)) => Err(eframe::Error::AppCreation(e.into())),
        None => Ok(()),
    }
}
