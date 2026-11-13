// SPDX-License-Identifier: LGPL-2.1-or-later
//! Terceira camada da dica (B60): F1 sobre um botão abre a ajuda do comando, com o nome, o atalho,
//! a frase, a animação maior e o passo a passo inteiro. Também guarda o atraso das dicas, que a
//! pessoa escolhe nas configurações.

use crate::i18n::tr;
use eframe::egui;
use std::sync::atomic::{AtomicU32, Ordering};

/// Atraso padrão da primeira camada, em milissegundos.
pub const ATRASO_PADRAO_MS: u32 = 500;
/// A camada 2 (animação e passo a passo) vem este tanto depois da primeira.
pub const DEPOIS_DA_PRIMEIRA_S: f64 = 0.7;

static ATRASO_MS: AtomicU32 = AtomicU32::new(ATRASO_PADRAO_MS);

/// Publica o atraso escolhido (a cada quadro) e o aplica às dicas do egui.
pub fn definir_atraso(ctx: &egui::Context, ms: u32) {
    let ms = ms.clamp(100, 3000);
    if ATRASO_MS.swap(ms, Ordering::Relaxed) != ms || ctx.global_style().interaction.tooltip_delay != ms as f32 / 1000. {
        ctx.global_style_mut(|s| s.interaction.tooltip_delay = ms as f32 / 1000.);
    }
}

/// Segundos até a camada 2 abrir, contados do começo do pairar.
pub fn estagio_2() -> f64 {
    ATRASO_MS.load(Ordering::Relaxed) as f64 / 1000. + DEPOIS_DA_PRIMEIRA_S
}

fn chave() -> egui::Id {
    egui::Id::new("fenkra_ajuda_f1")
}

/// Chamado pela dica do botão: F1 com o mouse em cima abre a ajuda daquele comando.
pub fn talvez_abrir(ctx: &egui::Context, icone: &str) {
    if !icone.is_empty() && ctx.input(|i| i.key_pressed(egui::Key::F1)) {
        abrir(ctx, icone);
    }
}

pub fn abrir(ctx: &egui::Context, icone: &str) {
    ctx.data_mut(|d| d.insert_temp(chave(), icone.to_string()));
}

/// O comando do catálogo da barra com este ícone.
pub fn comando(icone: &str) -> Option<crate::toolbar::Item> {
    crate::ajuda_cmd::catalogo().iter().find(|i| i.icon == icone).cloned()
}

/// Janela da ajuda do comando, quando aberta.
pub fn janela(ctx: &egui::Context) {
    let Some(icone) = ctx.data(|d| d.get_temp::<String>(chave())) else { return };
    let Some(it) = comando(&icone) else {
        ctx.data_mut(|d| d.remove::<String>(chave()));
        return;
    };
    let mut aberta = true;
    // o relógio sai antes: ler a entrada dentro de `data_mut` travaria o contexto
    let agora = ctx.input(|i| i.time);
    let inicio = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new(("fenkra_ajuda_f1_t", icone.as_str())), || agora));
    let t = agora - inicio;
    egui::Window::new(format!("{} — {}", tr("Ajuda do comando"), tr(it.name))).id(egui::Id::new("janela_ajuda_f1")).open(&mut aberta).default_width(380.).default_pos([360., 140.]).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::Image::new(crate::icons::get(it.icon)).fit_to_exact_size(egui::vec2(32., 32.)));
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(tr(it.name)).strong().size(16.));
                if let Some(a) = crate::ajuda_cmd::atalho_do_comando(ui.ctx(), it.icon) {
                    ui.label(egui::RichText::new(format!("{}: {a}", tr("Atalho"))).weak().small());
                }
            });
        });
        if !it.brief.is_empty() {
            ui.label(tr(it.brief));
        }
        ui.separator();
        if crate::midia::exibir::tem(it.icon) {
            crate::midia::exibir::desenhar(ui, it.icon, t.max(0.), 360.);
        } else {
            crate::dica_anim::desenhar(ui, it.icon, t.max(0.), 360.);
        }
        ui.add_space(4.);
        crate::ajuda_cmd::mini_fluxo(ui, it.advanced);
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
        ui.separator();
        ui.label(egui::RichText::new(tr("F1 sobre qualquer botão abre a ajuda dele. Esc fecha.")).weak().small());
    });
    if !aberta || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        ctx.data_mut(|d| d.remove::<String>(chave()));
    }
}

/// Textos novos (inglês), somados à tabela de `i18n.rs`.
pub static TEXTOS: &[(&str, &str)] = &[
    ("Ajuda do comando", "Command help"),
    ("Atalho", "Shortcut"),
    ("F1 sobre qualquer botão abre a ajuda dele. Esc fecha.", "F1 over any button opens its help. Esc closes."),
    ("(F1: ajuda do comando)", "(F1: command help)"),
    ("atraso das dicas (ms)", "tooltip delay (ms)"),
    ("Quanto o mouse fica parado sobre um botão antes da dica aparecer. A animação e o passo a passo vêm 0,7 s depois.", "How long the mouse rests on a button before the tooltip shows. The animation and the steps come 0.7 s later."),
    ("Buscar no catálogo (nome ou sinônimo, PT/EN)", "Search the catalog (name or synonym, PT/EN)"),
];

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn atraso_vale_para_as_duas_camadas_e_fica_na_faixa() {
        let ctx = egui::Context::default();
        definir_atraso(&ctx, 800);
        assert!((estagio_2() - 1.5).abs() < 1e-9);
        assert!((ctx.global_style().interaction.tooltip_delay - 0.8).abs() < 1e-6);
        definir_atraso(&ctx, 5);
        assert!((estagio_2() - (0.1 + DEPOIS_DA_PRIMEIRA_S)).abs() < 1e-9, "atraso mínimo de 100 ms");
        definir_atraso(&ctx, ATRASO_PADRAO_MS);
        assert!((estagio_2() - crate::ajuda_cmd::ESTAGIO_2).abs() < 1e-9, "o padrão é o de antes: 1,2 s");
    }

    #[test]
    fn f1_guarda_o_comando_e_nome_desconhecido_fecha() {
        let ctx = egui::Context::default();
        abrir(&ctx, "filete");
        assert_eq!(ctx.data(|d| d.get_temp::<String>(chave())).as_deref(), Some("filete"));
        let icone = crate::ajuda_cmd::catalogo().iter().find(|i| !i.advanced.is_empty()).map(|i| i.icon).expect("comando com passo a passo");
        assert!(comando(icone).is_some_and(|i| !i.advanced.is_empty()), "a ajuda tem o passo a passo");
        assert!(comando("nao_existe").is_none());
    }
}
