// SPDX-License-Identifier: LGPL-2.1-or-later
//! Candidatas no 3D do assistente de reparo (B15, ADR 0010 fase 2).
//!
//! Com o assistente aberto, cada face ou aresta perdida de um comando com erro mostra as candidatas
//! na viewport, numeradas (a mais provável é a 1). Clicar no número, no 3D ou na lista do assistente,
//! troca o ponto do comando pelo da candidata; a chave v2 é refeita no cálculo seguinte.

use crate::app::App;
use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, Rect, RichText, Sense, Stroke};
use fenkra_core::Id;
use fenkra_solidos::reparo::{self, Alvo, Lacuna};
use fenkra_solidos::Status;
use glam::Vec3;

/// Candidata sob o ponteiro (no 3D ou na lista): comando, referência e número da candidata.
type Destaque = (u64, usize, usize);

fn chave_destaque() -> egui::Id {
    egui::Id::new("reparo_destaque")
}

/// Lacunas de todos os comandos com erro (só com o assistente aberto).
pub fn lacunas(app: &App) -> Vec<(Id, Vec<Lacuna>)> {
    if !app.wizard_open {
        return vec![];
    }
    let Some(r) = &app.result else { return vec![] };
    app.open
        .doc
        .features
        .iter()
        .filter(|f| matches!(r.status.get(&f.id), Some(Status::Error(_))))
        .map(|f| (f.id, reparo::lacunas(f, &r.bodies)))
        .filter(|(_, l)| !l.is_empty())
        .collect()
}

fn nome(l: &Lacuna) -> String {
    match l.alvo {
        Alvo::Face => format!("{} {}", tr("Face"), l.indice + 1),
        Alvo::Aresta => format!("{} {}", tr("Aresta"), l.indice + 1),
    }
}

/// Linhas do assistente para um comando: as referências perdidas e os botões das candidatas.
pub fn linhas(ui: &mut egui::Ui, id: Id, lac: &[Lacuna], escolha: &mut Option<(Id, Lacuna, usize)>) {
    let k = chave_destaque();
    for l in lac {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("📍 {} — {}", nome(l), tr("candidatas no 3D:"))).small());
            for (i, _) in l.candidatas.iter().enumerate() {
                let b = ui.small_button(format!("{}", i + 1)).on_hover_text(tr("Usar esta candidata: o comando passa a apontar para ela (a mais provável é a 1)."));
                if b.hovered() {
                    ui.ctx().data_mut(|m| m.insert_temp::<Destaque>(k, (id.0, l.indice, i)));
                }
                if b.clicked() {
                    *escolha = Some((id, l.clone(), i));
                }
            }
        });
    }
}

/// Troca a referência pela candidata `i` e guarda o desfazer.
pub fn aplicar(app: &mut App, id: Id, l: &Lacuna, i: usize) {
    let Some(p) = l.candidatas.get(i).copied() else { return };
    let antes = app.open.doc.clone();
    let ok = app.open.doc.feature_mut(id).is_some_and(|f| reparo::aplicar(&mut f.kind, l.alvo, l.indice, p));
    if ok {
        app.push_undo_pub(antes);
    }
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Marcadores numerados das candidatas na viewport; clique aplica.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let todas = lacunas(app);
    if todas.is_empty() {
        return;
    }
    let k = chave_destaque();
    let destaque = ui.ctx().data_mut(|m| m.remove_temp::<Destaque>(k));
    let painter = ui.painter_at(rect);
    let acento = crate::theme::accent(&ui.ctx().global_style().visuals);
    let erro = crate::theme::st_err();
    let texto = ui.visuals().strong_text_color();
    let mut escolha: Option<(Id, Lacuna, usize)> = None;
    for (id, lac) in &todas {
        for l in lac {
            // último ponto conhecido: um x vermelho
            if let Some(a) = cam.project(rect, v3(l.atual)) {
                let s = 5.;
                painter.line_segment([a + egui::vec2(-s, -s), a + egui::vec2(s, s)], Stroke::new(2., erro));
                painter.line_segment([a + egui::vec2(-s, s), a + egui::vec2(s, -s)], Stroke::new(2., erro));
            }
            for (i, c) in l.candidatas.iter().enumerate() {
                let Some(a) = cam.project(rect, v3(*c)) else { continue };
                let aceso = destaque == Some((id.0, l.indice, i));
                let r = ui.interact(Rect::from_center_size(a, egui::vec2(20., 20.)), egui::Id::new(("reparo_cand", *id, l.indice, i)), Sense::click());
                let forte = aceso || r.hovered();
                painter.circle(a, if forte { 11. } else { 9. }, if forte { acento } else { acento.gamma_multiply(0.55) }, Stroke::new(1.5, texto));
                painter.text(a, egui::Align2::CENTER_CENTER, format!("{}", i + 1), egui::FontId::proportional(11.), texto);
                if r.clicked() {
                    escolha = Some((*id, l.clone(), i));
                }
                r.on_hover_text(format!("{} · {}", nome(l), tr("clique para usar esta candidata")));
            }
        }
    }
    if let Some((id, l, i)) = escolha {
        aplicar(app, id, &l, i);
    }
}
