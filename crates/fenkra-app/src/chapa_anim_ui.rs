// SPDX-License-Identifier: LGPL-2.1-or-later
//! Dobrar e desdobrar animado e ordem de dobra editável (B304).
//!
//! O controle de 0 a 100% fica na janela de chapa. Nos extremos ele só troca o estado da peça
//! (dobrada ou planificada, os cálculos de verdade); no meio manda ao worker a peça com as dobras
//! abertas na fração (`fenkra_solidos::chapa_animacao`), sem mexer no documento. Os glifos
//! numerados ficam sobre as linhas de dobra no 3D: arrastar para os lados muda o número (repetir
//! vale), e o número vai para a tabela de dobras, a planificada e o DXF.

use crate::app::App;
use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, Rect, RichText, Stroke};
use fenkra_core::*;

/// Pixels de arraste por número.
pub const PASSO_PX: f32 = 24.;

const K_PEDIDO: &str = "chapa_anim_pedido";
const K_PCT: &str = "chapa_anim_pct";
const K_PLAY: &str = "chapa_anim_play";

/// Número novo do glifo depois de arrastar `dx` pixels a partir de `inicio` (nunca abaixo de 1).
pub fn numero_arrastado(inicio: u32, dx: f32) -> u32 {
    (inicio as i64 + (dx / PASSO_PX).round() as i64).max(1) as u32
}

/// Para cada linha de dobra do resultado: o comando e a posição dela entre as linhas dele.
pub fn chaves(linhas: &[fenkra_solidos::BendLine]) -> Vec<(Id, u32)> {
    let mut visto: std::collections::HashMap<Id, u32> = Default::default();
    linhas
        .iter()
        .map(|l| {
            let k = visto.entry(l.feature).or_default();
            *k += 1;
            (l.feature, *k - 1)
        })
        .collect()
}

/// Controle da janela de chapa. `flat` é o estado da cópia editada pela janela; `fixos` são os
/// nomes dos comandos que só mudam no fim.
pub fn controle(ui: &mut egui::Ui, flat: &mut bool, fixos: &[String], animando: Option<f64>) {
    crate::editor::section(ui, "Dobrar e desdobrar");
    let ctx = ui.ctx().clone();
    let mut pct: f64 = ctx.data(|m| m.get_temp(egui::Id::new(K_PCT))).unwrap_or(if *flat { 100. } else { 0. });
    if animando.is_none() && !ctx.data(|m| m.get_temp::<bool>(egui::Id::new(K_PLAY))).unwrap_or(false) {
        // fora da animação o controle mostra o estado da peça
        pct = if *flat { 100. } else { 0. };
    }
    let mut play: bool = ctx.data(|m| m.get_temp(egui::Id::new(K_PLAY))).unwrap_or(false);
    ui.horizontal(|ui| {
        let r = ui
            .add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix(" %").fixed_decimals(0).text(tr("planificada")))
            .on_hover_text(tr("Arraste para abrir as dobras aos poucos: 0% é a peça dobrada, 100% a planificada exata. No meio, cada dobra abre na mesma proporção."));
        if r.changed() {
            play = false;
            pede(&ctx, pct, flat);
        }
        let rotulo = if play { "⏸" } else { "▶" };
        if ui.button(rotulo).on_hover_text(tr("Anima a peça de dobrada até planificada (ou volta, se já estiver plana).")).clicked() {
            play = !play;
            if play && pct >= 100. {
                pct = 0.;
                pede(&ctx, pct, flat);
            }
        }
    });
    if !fixos.is_empty() {
        ui.label(RichText::new(format!("{} {}", tr("Só mudam no fim (sem ângulo no comando):"), fixos.join(", "))).small().weak());
    }
    ui.label(RichText::new(tr("Ordem de dobra: arraste o número sobre cada linha de dobra, no 3D, para os lados. Número repetido vale (mesmo golpe).")).small().weak());
    ctx.data_mut(|m| {
        m.insert_temp(egui::Id::new(K_PCT), pct);
        m.insert_temp(egui::Id::new(K_PLAY), play);
    });
}

fn pede(ctx: &egui::Context, pct: f64, flat: &mut bool) {
    if pct <= 0. {
        *flat = false;
    } else if pct >= 100. {
        *flat = true;
    }
    ctx.data_mut(|m| m.insert_temp(egui::Id::new(K_PEDIDO), pct));
}

/// Depois da janela: manda o pedido do controle e avança a animação quando o cálculo anterior
/// chegou. Os extremos voltam ao cálculo normal do documento.
pub fn processa(app: &mut App, ctx: &egui::Context) {
    let play: bool = ctx.data(|m| m.get_temp(egui::Id::new(K_PLAY))).unwrap_or(false);
    let mut pedido: Option<f64> = ctx.data_mut(|m| m.remove_temp(egui::Id::new(K_PEDIDO)));
    if play && pedido.is_none() && !app.calculando() {
        let atual: f64 = ctx.data(|m| m.get_temp(egui::Id::new(K_PCT))).unwrap_or(0.);
        let prox = (atual + 5.).min(100.);
        ctx.data_mut(|m| m.insert_temp(egui::Id::new(K_PCT), prox));
        if prox >= 100. {
            ctx.data_mut(|m| m.insert_temp(egui::Id::new(K_PLAY), false));
            if let Some(sp) = app.open.doc.sheet.as_mut() {
                sp.flat = true;
            }
        }
        pedido = Some(prox);
    }
    if play {
        ctx.request_repaint();
    }
    let Some(pct) = pedido else { return };
    ctx.data_mut(|m| m.insert_temp(egui::Id::new(K_PCT), pct));
    if pct <= 0. || pct >= 100. {
        app.request_recompute();
    } else {
        app.enviar_animacao(pct / 100.);
    }
}

/// Glifos numerados sobre as linhas de dobra. Arrastar para os lados muda o número.
pub fn glifos(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let Some(r) = app.result.as_ref() else { return };
    if r.bend_lines.is_empty() || app.open.doc.sheet.is_none() {
        return;
    }
    let tabela = fenkra_solidos::tabela_dobras(&app.open.doc, &r.bend_lines);
    let chaves = chaves(&r.bend_lines);
    let v = |p: [f64; 3]| glam::Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
    let painter = ui.painter_at(rect);
    let vis = ui.visuals().clone();
    let cor = crate::theme::accent(&vis);
    let mut grava: Option<(Id, u32, u32)> = None;
    for (i, b) in r.bend_lines.iter().enumerate() {
        let meio = if b.curva.len() > 2 { b.curva[b.curva.len() / 2] } else { [(b.center[0][0] + b.center[1][0]) / 2., (b.center[0][1] + b.center[1][1]) / 2., (b.center[0][2] + b.center[1][2]) / 2.] };
        // o glifo fica perto da ponta inicial da linha, longe do rótulo do meio
        let ponta = if b.curva.len() > 2 { b.curva[1] } else { b.center[0] };
        let lugar = [meio[0] * 0.3 + ponta[0] * 0.7, meio[1] * 0.3 + ponta[1] * 0.7, meio[2] * 0.3 + ponta[2] * 0.7];
        let Some(c) = cam.project(rect, v(lugar)) else { continue };
        let (feature, k) = chaves[i];
        let ordem = tabela.get(i).map_or(i + 1, |t| t.ordem) as u32;
        let id = egui::Id::new(("glifo_dobra", feature.0, k));
        let raio = 11.;
        let resp = ui.interact(Rect::from_center_size(c, egui::vec2(raio * 2. + 4., raio * 2. + 4.)), id, egui::Sense::drag());
        let k_ini = id.with("inicio");
        if resp.drag_started() {
            ui.ctx().data_mut(|m| m.insert_temp(k_ini, ordem));
        }
        let mut mostra = ordem;
        if resp.dragged() {
            let ini: u32 = ui.ctx().data(|m| m.get_temp(k_ini)).unwrap_or(ordem);
            let dx = resp.total_drag_delta().map_or(0., |d| d.x);
            mostra = numero_arrastado(ini, dx);
            if mostra != ordem {
                grava = Some((feature, k, mostra));
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        } else if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        let quente = resp.hovered() || resp.dragged();
        painter.circle_filled(c, raio, if quente { cor } else { vis.extreme_bg_color });
        painter.circle_stroke(c, raio, Stroke::new(2., cor));
        let txt = if quente { vis.strong_text_color() } else { vis.text_color() };
        painter.text(c, egui::Align2::CENTER_CENTER, mostra.to_string(), egui::FontId::proportional(crate::acessibilidade::fonte_3d(13.)), txt);
        resp.on_hover_text(tr("Ordem de dobra: arraste para a direita ou para a esquerda para mudar o número. Número repetido vale."));
    }
    if let Some((f, k, n)) = grava {
        if let Some(sp) = app.open.doc.sheet.as_mut() {
            sp.define_ordem(f, k, n);
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn arrastar_muda_o_numero_e_nunca_fica_abaixo_de_um() {
        assert_eq!(numero_arrastado(2, 0.), 2);
        assert_eq!(numero_arrastado(2, PASSO_PX * 1.4), 3);
        assert_eq!(numero_arrastado(2, PASSO_PX * 3.), 5);
        assert_eq!(numero_arrastado(2, -PASSO_PX * 5.), 1);
    }
}
