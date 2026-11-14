// SPDX-License-Identifier: LGPL-2.1-or-later
//! Fenkra — CAD paramétrico open source.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// a tabela de traduções é uma cadeia longa de `chain`; o tipo dela passa da profundidade padrão
// a tabela de traduções encadeia muitas listas (`i18n.rs`): o tipo do iterador passa do limite padrão
// a tabela de traduções encadeia dezenas de `textos_*`: o tipo do iterador passa do limite padrão
// a tabela de traduções de `i18n.rs` encadeia uma fatia por bancada; com elas todas, o rustc passa do
// limite padrão de profundidade ao calcular o tipo do iterador (a compilação release falhava)
#![recursion_limit = "256"]

mod app;
mod examples;
mod theme;
mod icons;
mod settings;
mod toolbar;
mod editor;
mod render;
mod demo_polimento;
mod scene;
mod entradas;
mod previa;
mod previa_leve_ui;
mod rosca_ui;
mod furo_painel;
mod textos_furo_rosca;
mod demo_rosca;
mod sup3d;
mod sup_analise;
mod loft_3d;
mod sup_demo;
mod sup_loft;
mod sup_spline;
mod picking;
mod realce_face;
mod analise_janela;
mod textos_analise;
mod selecao;
mod selecao_regras;
mod continuar;
mod ajuda_f1;
mod scan;
mod textos_ger8;
mod alca_estado;
mod viewcube;
mod sketch_ui;
mod sketch_vinculos;
mod geo_ui;
mod keymap;
mod feat_ui;
mod varredura_ui;
mod demo_solidos;
mod demo_chapa;
mod edicao_direta_ui;
mod handles;
mod chapa_manip;
mod chapa_manip_ui;
mod gizmo;
mod asm_ui;
mod asm_abrir;
mod textos_abrir;
mod textos_validade;
mod sentido_corte_ui;
mod asm_cmd;
mod asm_manip;
mod demo_dica;
mod demo_foto;
mod teclado;
mod asm_selecao;
mod asm_simbolos;
mod asm_limites;
mod asm_exibicao;
mod st_ui;
mod pmi_ui;
mod pmi_smart;
mod pmi_handles;
mod pmi_minibar;
mod pmi_nota_cmd;
mod pmi_setas;
mod pmi_cordao;
mod pmi_consultor_3d;
mod pmi_empilhamento;
mod pmi_inspecao_3d;
mod textos_pmi_consultor;
mod pmi_arvore;
mod pmi_legivel;
mod pmi_sobrepoe;
mod pmi_feat_asm;
mod pmi_baloes_ui;
mod pmi_alvos_ui;
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
mod arvore_soltura_ui;
mod historico;
mod diagnostico;
mod paineis;
mod paineis_demo;
mod partida;
mod abertura;
mod demo_estado;
mod rotulos_3d;
mod arvore_nomes;
mod barra_estado;
mod contraste;
mod massa_fmt;
mod dica_anim;
mod erro_acao;
mod arvore_grupos;
mod chapa_estado;
mod textos_interface2;
mod textos_interface3;
mod textos_interface4;
mod textos_verificar;
mod verificar_ui;
mod comparar_ui;
mod forma_progresso;
mod ajuda_cmd;
mod roda;
mod textos_campos;
mod midia;
mod textos_ajuda;
mod textos_biblioteca;
mod alca_cota;
mod engrenagens_abas_ui;
mod engrenagens_ui;
mod engrenagens_verificar_ui;
mod transmissao_ui;
mod textos_transmissao;
mod eixo_ui;
mod textos_biblioteca3;
mod textos_biblioteca4;
mod textos_biblioteca5;
mod textos_biblioteca6;
mod textos_biblioteca7;
mod textos_conversor;
mod textos_dwg_dxf;
mod textos_formatos;
mod textos_visualizacao;
mod assoc_fkv;
mod assoc_docs;
mod textos_assoc;
mod textos_malha;
mod malha_ui;
mod impressao_ui;
mod textos_intercambio;
mod textos_projeto;
mod textos_desenho;
mod textos_desenho_cotas;
mod textos_desenho_tabelas;
mod textos_desenho_esboco;
mod textos_desenho_inspecao;
mod textos_estrutural;
mod textos_solda;
mod textos_dobra_tubo;
mod st_dobra_ui;
mod st_dobra_corte_ui;
mod st_cantos_ui;
mod st_acesso_ui;
mod st_tocha_ui;
mod st_corte_tubo_ui;
mod st_limite_ui;
mod textos_st_limite;
mod st_encaixe_ui;
mod textos_encaixe;
mod textos_conjunto;
mod st_montagem_ui;
mod st_corte_liga;
mod st_maquina_ui;
mod textos_st_corte;
mod textos_chapa;
mod chapa_estilo_ui;
mod chapa_laco_ui;
mod chapa_canto_ui;
mod chapa_unif_ui;
mod chapa_b35_ui;
mod chapa_fab_ui;
mod chapa_anim_ui;
mod textos_chapa_fab;
mod st_alcas;
mod st_esqueleto;
mod st_secao;
mod st_painel;
mod st_placas_ui;
mod st_solda_ui;
mod biblioteca_ui;
mod reparo_ui;
#[cfg(test)]
mod checklist_solidos;
mod textos_montagem;
mod textos_montagem3;
mod textos_montagem4;
mod asm_inserir;
mod asm_inserir_projeto;
mod asm_interf;
mod asm_repeticao;
mod asm_resolver;
mod asm_fixadores;
mod asm_conjunto;
mod asm_chaveta;
mod calculo_ui;
mod textos_calculo;
mod textos_chaveta;
mod asm_encaixe;
mod textos_plm;
mod textos_curso;
mod curso;
mod curso_exemplos;
mod mapa_macro;
mod compat;
mod curso_ui;
mod tutoriais;
mod figura_cota;
mod textos_pmi;
mod textos_pmi_step;
mod textos_pmi_captura;
mod textos_pmi_nota;
mod textos_superficies;
mod textos_solidos;
mod textos_edicao_direta;
mod textos_dfm;
mod textos_paineis;
mod textos_arvore_soltura;
mod dfm_ui;
mod sup_inclinacao;
mod textos_features;
mod tabela_projeto_ui;
mod arestas_ui;
mod arestas_previa;
mod textos_forma;
mod forma_edit;
mod forma_sel;
mod forma_vinculo_ui;
mod forma_ui;
mod sketch_fluxo;
mod sketch_cadeia;
mod sketch_gesto;
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
mod sketch_pub;
mod rotulo_ligado;
mod perfil;
mod relatorio;
mod param_ui;
mod config_camadas;
mod config_externos;
mod camada3d;
mod config_ui;
mod textos_config;
mod acessibilidade;
mod camera_nav;
mod arabesco;
mod inicio_tema;
mod props_geometria;
mod asm_fundo;
mod instancias;
mod medir_quadros;
mod desempenho;
mod textos_desempenho;
mod textos_montagem5;
mod textos_montagem6;
mod textos_montagem7;
mod textos_fechamont;
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
    // B393: o desenho projeta cada vista de montagem num processo deste mesmo executável
    if let Some(codigo) = fenkra_desenho::hlr_processos::principal(&args) {
        std::process::exit(codigo);
    }
    if let Ok(exe) = std::env::current_exe() {
        fenkra_desenho::hlr_processos::definir_executavel(exe);
    }
    // duplo clique num .fkv (B365): abre o pacote no navegador e sai, sem subir a janela
    if assoc_fkv::tratar_argumentos(&args) {
        return Ok(());
    }
    partida::iniciar();
    if args.iter().any(|a| a == "--diagnostico") {
        partida::diagnostico();
        return Ok(());
    }
    let seguro = args.iter().any(|a| a == "--seguro");
    // `.fkp`, `.fka` ou `.fkd` na linha de comando (ou duplo clique) abre o arquivo (B389)
    let arquivo = abertura::arquivo_da_linha(&args);
    let forcado = std::env::var("WGPU_BACKEND").ok().filter(|v| !v.is_empty());
    let mut erros: Vec<(partida::Modo, String)> = Vec::new();
    for modo in partida::Modo::tentativas(seguro, forcado.is_some()) {
        partida::registrar("tentativa", modo.nome());
        let r = eframe::run_native(
            "Fenkra",
            opcoes(modo),
            Box::new({
                let arquivo = arquivo.clone();
                move |cc| {
                let mut app = app::App::new(cc);
                if let Some(p) = &arquivo {
                    app.open_path(p);
                }
                if let Some(rs) = &cc.wgpu_render_state {
                    let hw = perfil::detectar(rs);
                    app.avaliar_perfil(&hw);
                    if partida::registrar_adaptador(rs, modo) {
                        crate::i18n::set_english(app.settings.english);
                        app.toast(crate::i18n::tr("Adaptador gráfico por software: o Fenkra abre, mas fica lento. Atualize o driver da placa de vídeo."), true);
                    }
                }
                Ok(Box::new(app))
                }
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
