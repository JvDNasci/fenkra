// SPDX-License-Identifier: LGPL-2.1-or-later
//! B12: o loft sólido no 3D. Cada perfil fica numerado na ordem, uma linha de conexão liga os pontos
//! de partida e clicar noutro vértice troca o ponto de partida daquele perfil, para tirar a torção.
//! A seta da condição de ponta usa a alça comum de `handles.rs`.

use crate::app::App;
use crate::editor::expr;
use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, Pos2, Rect, Stroke};
use fenkra_core::*;
use fenkra_solidos::RecomputeResult;
use glam::Vec3;
use std::collections::HashMap;

/// Campos das setas de ponta em `handles::write_slot`.
pub const CAMPO_INICIO: u8 = 1;
pub const CAMPO_FIM: u8 = 2;

fn vec3(p: [f32; 3]) -> Vec3 {
    Vec3::from(p)
}

/// Vértices do perfil no mundo e o centro deles.
fn vertices(r: &RecomputeResult, sketch: Id) -> Option<(Vec<Vec3>, Vec3)> {
    let s = r.sketches.iter().find(|s| s.feature == sketch)?;
    // os traços do contorno, na ordem dele: cada ponta entra uma vez
    let mut v: Vec<Vec3> = vec![];
    for p in s.lines.iter().map(|p| vec3(*p)) {
        if !v.iter().any(|q| (*q - p).length() < 1e-3) {
            v.push(p);
        }
    }
    let c = v.iter().fold(Vec3::ZERO, |a, p| a + *p) / v.len().max(1) as f32;
    (!v.is_empty()).then_some((v, c))
}

/// Índice do ponto de partida do perfil `i`, dentro dos `n` vértices dele.
pub fn partida(l: &Loft, i: usize, n: usize) -> usize {
    l.torcao.get(i).copied().unwrap_or(0).rem_euclid(n.max(1) as i32) as usize
}

/// Grava o ponto de partida, criando a lista até o perfil `i`.
pub fn definir_partida(l: &mut Loft, i: usize, v: usize) {
    if l.torcao.len() <= i {
        l.torcao.resize(i + 1, 0);
    }
    l.torcao[i] = v as i32;
}

/// Seta de cada ponta com condição Normal: (campo, base, direção, mm por unidade, expressão).
pub fn setas(r: &RecomputeResult, l: &Loft) -> Vec<(u8, Vec3, Vec3, f32, String)> {
    let [a, .., b] = l.sections[..] else { return vec![] };
    let (Some((_, ca)), Some((_, cb))) = (vertices(r, a), vertices(r, b)) else { return vec![] };
    let vao = (cb - ca).length();
    let Some(d) = (cb - ca).try_normalize() else { return vec![] };
    let expr_de = |p: &PontaLoft| if p.intensidade.trim().is_empty() { "1".to_string() } else { p.intensidade.clone() };
    let mut out = vec![];
    if l.inicio.condicao == CondicaoPonta::Normal {
        out.push((CAMPO_INICIO, ca, d, vao / 3., expr_de(&l.inicio)));
    }
    if l.fim.condicao == CondicaoPonta::Normal {
        out.push((CAMPO_FIM, cb, -d, vao / 3., expr_de(&l.fim)));
    }
    out
}

/// Números, linha de conexão e vértices clicáveis.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let Some(FeatureKind::Loft(l)) = app.open.doc.feature(id).map(|f| f.kind.clone()) else { return };
    let Some(r) = app.result.as_ref() else { return };
    let cor = crate::theme::accent(ui.visuals());
    let fundo = ui.visuals().extreme_bg_color;
    let pintor = ui.painter_at(rect);
    let mut conexao: Vec<Pos2> = vec![];
    let mut troca = None;
    for (i, sk) in l.sections.iter().enumerate() {
        let Some((vs, c)) = vertices(r, *sk) else { continue };
        let ini = partida(&l, i, vs.len());
        for (k, v) in vs.iter().enumerate() {
            let Some(q) = cam.project(rect, *v) else { continue };
            if k == ini {
                conexao.push(q);
            }
            let hr = ui.interact(Rect::from_center_size(q, egui::vec2(18., 18.)), egui::Id::unique(("loft_vertice", id.0, i, k)), egui::Sense::click());
            pintor.circle_filled(q, if k == ini { 6. } else { 4. }, if k == ini { cor } else { crate::theme::st_warn() });
            if hr.on_hover_text(tr("Clique para o perfil começar neste vértice e desfazer a torção.")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                troca = Some((i, k));
            }
        }
        // ao lado do centro: a seta da ponta nasce nele
        if let Some(q) = cam.project(rect, c).map(|q| q + egui::vec2(-22., -22.)) {
            pintor.circle_filled(q, 11., fundo);
            pintor.circle_stroke(q, 11., Stroke::new(2., cor));
            pintor.text(q, egui::Align2::CENTER_CENTER, format!("{}", i + 1), egui::FontId::proportional(12.), ui.visuals().strong_text_color());
        }
    }
    pintor.add(egui::Shape::line(conexao, Stroke::new(2., cor)));
    if let (Some((i, k)), Some(FeatureKind::Loft(l))) = (troca, app.open.doc.feature_mut(id).map(|f| &mut f.kind)) {
        definir_partida(l, i, k);
    }
}

/// Condição de uma ponta no painel.
fn ponta_ui(ui: &mut egui::Ui, rotulo: &str, p: &mut PontaLoft, vars: &HashMap<String, f64>) {
    ui.horizontal(|ui| {
        ui.label(tr(rotulo));
        ui.selectable_value(&mut p.condicao, CondicaoPonta::Livre, tr("Livre"));
        ui.selectable_value(&mut p.condicao, CondicaoPonta::Normal, tr("Normal ao perfil"));
    });
    if p.condicao == CondicaoPonta::Normal {
        if p.intensidade.trim().is_empty() {
            p.intensidade = "1".into();
        }
        expr(ui, "Intensidade", &mut p.intensidade, vars, "");
    }
}

pub fn painel(ui: &mut egui::Ui, l: &mut Loft, vars: &HashMap<String, f64>) {
    ponta_ui(ui, "Início", &mut l.inicio, vars);
    ponta_ui(ui, "Fim", &mut l.fim, vars);
    ui.label(egui::RichText::new(tr("Clique nos vértices do 3D para desfazer a torção; arraste a seta para mudar a intensidade.")).weak().small());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loft() -> Loft {
        Loft { sections: vec![Id(1), Id(2)], ruled: false, closed: false, op: BoolOp::Add, torcao: vec![], inicio: Default::default(), fim: Default::default() }
    }

    #[test]
    fn partida_do_perfil_anda_pelos_vertices_e_grava_so_o_pedido() {
        let mut l = loft();
        assert_eq!(partida(&l, 1, 4), 0);
        definir_partida(&mut l, 1, 3);
        assert_eq!(l.torcao, vec![0, 3]);
        assert_eq!(partida(&l, 1, 4), 3);
        assert_eq!(partida(&l, 1, 2), 1, "menos vértices: dá a volta");
    }
}
