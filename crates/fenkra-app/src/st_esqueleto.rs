// SPDX-License-Identifier: LGPL-2.1-or-later
//! B226: enquanto o comando Membro está aberto, cada linha escolhida fica realçada e numerada no 3D,
//! e a seção aparece ampliada junto do cursor, para ver o perfil antes de confirmar.

use crate::app::App;
use crate::render::Camera;
use crate::st_secao::{lado_maior, miniatura};
use eframe::egui::{self, Pos2, Rect, Stroke};
use fenkra_core::*;
use glam::Vec3;

/// Lado da seção ampliada, em pixels.
const LADO_SECAO: f32 = 112.;
/// Distância entre o cursor e a seção, para ela não ficar debaixo dele.
const FOLGA_CURSOR: f32 = 24.;

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Texto do número de cada linha: a ordem em que entraram no membro, a partir de 1.
pub fn numeros(m: &StructMember) -> Vec<String> {
    (1..=m.paths.len()).map(|i| i.to_string()).collect()
}

/// Canto de cima à esquerda da seção: junto do cursor e sempre dentro da vista.
pub fn lugar_da_secao(cursor: Pos2, vista: Rect) -> Pos2 {
    let mut p = cursor + egui::vec2(FOLGA_CURSOR, FOLGA_CURSOR);
    if p.x + LADO_SECAO > vista.right() {
        p.x = cursor.x - FOLGA_CURSOR - LADO_SECAO;
    }
    if p.y + LADO_SECAO > vista.bottom() {
        p.y = cursor.y - FOLGA_CURSOR - LADO_SECAO;
    }
    Pos2::new(p.x.max(vista.left()), p.y.max(vista.top()))
}

pub fn overlay(app: &App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, m: &StructMember) {
    let Some(r) = app.result.as_ref() else { return };
    let cor = crate::theme::accent(ui.visuals());
    let painter = ui.painter_at(rect);
    let mut ancora = None;
    for (c, n) in m.paths.iter().zip(numeros(m)) {
        let Some(p) = crate::picking::curve_pts(r, c) else { continue };
        let tela: Vec<Pos2> = p.iter().filter_map(|q| cam.project(rect, v3(*q))).collect();
        painter.add(egui::Shape::line(tela.clone(), Stroke::new(3., cor)));
        let Some(meio) = tela.get(tela.len() / 2).copied() else { continue };
        ancora.get_or_insert(meio);
        painter.circle_filled(meio, 10., ui.visuals().extreme_bg_color);
        painter.circle_stroke(meio, 10., Stroke::new(2., cor));
        painter.text(meio, egui::Align2::CENTER_CENTER, n, egui::FontId::proportional(12.), ui.visuals().strong_text_color());
    }
    let dims: Vec<f64> = m.dims.iter().filter_map(|e| fenkra_core::expr::eval_str(e, &app.vars()).ok()).collect();
    let (Some(escala), Some(perto)) = (lado_maior(m.kind, &dims), ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p)).or(ancora)) else { return };
    egui::Area::new(egui::Id::new("st_secao_cursor")).fixed_pos(lugar_da_secao(perto, rect)).order(egui::Order::Foreground).interactable(false).show(ui.ctx(), |ui| {
        miniatura(ui, m.kind, &dims, escala, LADO_SECAO, false);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secao_fica_dentro_da_vista_e_fora_do_cursor() {
        let vista = Rect::from_min_size(Pos2::ZERO, egui::vec2(800., 600.));
        let p = lugar_da_secao(Pos2::new(100., 100.), vista);
        assert_eq!(p, Pos2::new(124., 124.));
        let canto = lugar_da_secao(Pos2::new(790., 590.), vista);
        let caixa = Rect::from_min_size(canto, egui::vec2(LADO_SECAO, LADO_SECAO));
        assert!(vista.contains_rect(caixa), "{caixa:?}");
        assert!(!caixa.contains(Pos2::new(790., 590.)));
    }
}
