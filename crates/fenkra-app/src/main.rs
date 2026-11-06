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
mod sup_analise;
mod sup_loft;
mod sup_spline;
mod picking;
mod selecao;
mod selecao_regras;
mod alca_estado;
mod viewcube;
mod sketch_ui;
mod sketch_vinculos;
mod geo_ui;
mod keymap;
mod feat_ui;
mod handles;
mod chapa_manip;
mod chapa_manip_ui;
mod gizmo;
mod asm_ui;
mod asm_cmd;
mod asm_manip;
mod demo_foto;
mod asm_selecao;
mod asm_simbolos;
mod asm_limites;
mod asm_exibicao;
mod st_ui;
mod pmi_ui;
mod pmi_smart;
mod pmi_handles;
mod pmi_minibar;
mod pmi_gdt_ui;
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
mod roda;
mod textos_campos;
mod midia;
mod textos_ajuda;
mod textos_biblioteca;
mod alca_cota;
mod engrenagens_ui;
mod eixo_ui;
mod textos_biblioteca3;
mod textos_conversor;
mod textos_intercambio;
mod textos_projeto;
mod textos_desenho;
mod textos_desenho_cotas;
mod textos_desenho_tabelas;
mod textos_estrutural;
mod textos_solda;
mod textos_dobra_tubo;
mod st_dobra_ui;
mod st_dobra_corte_ui;
mod st_cantos_ui;
mod st_montagem_ui;
mod textos_chapa;
mod chapa_estilo_ui;
mod chapa_laco_ui;
mod chapa_canto_ui;
mod chapa_unif_ui;
mod chapa_b35_ui;
mod st_alcas;
mod st_secao;
mod st_painel;
mod st_placas_ui;
mod st_solda_ui;
mod biblioteca_ui;
mod textos_montagem;
mod textos_montagem3;
mod textos_montagem4;
mod asm_inserir;
mod asm_interf;
mod asm_repeticao;
mod asm_fixadores;
mod asm_encaixe;
mod textos_plm;
mod textos_curso;
mod curso;
mod curso_exemplos;
mod mapa_macro;
mod compat;
mod curso_ui;
mod textos_pmi;
mod textos_superficies;
mod textos_solidos;
mod tabela_projeto_ui;
mod arestas_ui;
mod textos_forma;
mod forma_edit;
mod forma_ui;
mod sketch_fluxo;
mod sketch_cadeia;
mod sketch_canto;
mod sketch_cota;
mod sketch_relimita;
mod sketch_arraste;
mod sketch_barra;
mod sketch_demo;
mod sketch_sombra;
mod sketch_painel;
mod sketch_snap;
mod sketch_textos;
mod perfil;
mod relatorio;
mod param_ui;
mod config_camadas;
mod config_ui;
mod textos_config;
mod acessibilidade;
mod camera_nav;
mod arabesco;
mod mouse3d;
mod mouse3d_hid;
mod textos_mouse3d;
mod textos_acessibilidade;
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
                // a cena inteira vai num buffer só: com o limite padrão de 256 MiB, uma montagem grande
                // derrubava o programa ao subir a malha. Vale o que a placa aceita
                required_limits: wgpu::Limits { max_texture_dimension_2d: 8192, max_buffer_size: adapter.limits().max_buffer_size, ..base },
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
