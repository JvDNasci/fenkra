// SPDX-License-Identifier: LGPL-2.1-or-later
//! B549: polígono de controle da spline na viewport do sketch.
//!
//! A conta fica em `fenkra_core::sketch_controle`; aqui só o desenho e o arraste. O polígono aparece
//! tracejado, os vértices são quadrados vazados que se arrastam, e as pontas (que são também pontos
//! de passagem) ficam cheias. Cores do tema: o polígono no tom fraco do texto, os vértices no destaque
//! e o vértice sob o mouse na cor de seleção.

use crate::i18n::tr;
use eframe::egui::{self, Pos2, Rect, Stroke};
use fenkra_core::sketch::{SketchData, SolveReport};
use fenkra_core::Id;
use std::collections::HashMap;

/// Resultado de um quadro: algum vértice sob o mouse ou arrastado (a viewport não seleciona nada)
/// e o relatório do solver, se o arraste mudou a curva.
#[derive(Default)]
pub(crate) struct Quadro {
    pub ocupado: bool,
    pub relatorio: Option<SolveReport>,
}

/// Desenha o polígono de controle de `curva` e trata o arraste dos vértices.
pub(crate) fn poligono(ui: &egui::Ui, painter: &egui::Painter, sk: &mut SketchData, curva: Id, to_screen: &dyn Fn([f64; 2]) -> Pos2, to_uv: &dyn Fn(Pos2) -> Option<[f64; 2]>, vars: &HashMap<String, f64>) -> Quadro {
    let mut q = Quadro::default();
    let d = sk.spline_controle(curva);
    if d.len() < 4 {
        return q;
    }
    let vis = ui.visuals();
    let (fraco, destaque, quente) = (vis.weak_text_color(), crate::theme::accent(vis), vis.selection.stroke.color);
    let tela: Vec<Pos2> = d.iter().map(|p| to_screen(*p)).collect();
    painter.extend(egui::Shape::dashed_line(&tela, Stroke::new(1.2, fraco), 5., 4.));
    let ponta = |k: usize| k == 0 || k == d.len() - 1;
    for (k, s) in tela.iter().enumerate() {
        let r = ui.interact(Rect::from_center_size(*s, egui::vec2(14., 14.)), egui::Id::new(("controle_spline", curva.0, k)), egui::Sense::drag());
        let ativo = r.hovered() || r.dragged();
        let cor = if ativo { quente } else { destaque };
        let caixa = Rect::from_center_size(*s, egui::vec2(9., 9.));
        if ponta(k) {
            painter.rect_filled(caixa, 1.5, cor);
        } else {
            painter.rect_filled(caixa, 1.5, vis.extreme_bg_color);
            painter.rect_stroke(caixa, 1.5, Stroke::new(1.6, cor), egui::StrokeKind::Inside);
        }
        if r.hovered() && !r.dragged() {
            let dica = if ponta(k) { "ponta da curva: arraste para mover" } else { "ponto de controle: arraste para dar forma à curva" };
            painter.text(*s + egui::vec2(10., -10.), egui::Align2::LEFT_BOTTOM, tr(dica), egui::FontId::proportional(crate::acessibilidade::fonte_3d(11.)), quente);
        }
        q.ocupado |= ativo;
        if r.dragged() {
            if let Some(alvo) = r.interact_pointer_pos().and_then(to_uv) {
                // sem solução a curva para onde estava, como no arraste do corpo da spline
                q.relatorio = sk.mover_controle(curva, k, alvo, vars).or(q.relatorio);
            }
        }
    }
    q
}
