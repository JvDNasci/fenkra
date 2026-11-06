// SPDX-License-Identifier: LGPL-2.1-or-later
//! Furo-guia (B90): com o comando Furo aberto em modo "pontos 3D", o contorno do furo segue o cursor
//! sobre a face, alinhado à normal dela, com o diâmetro e a profundidade que o painel tem agora.
//! Clicar na face grava o ponto no comando (`feat_ui::add_face`) e o furo-guia continua para o
//! próximo furo.
//!
//! Como funciona: o picking já devolve o ponto e a normal de saída da face sob o cursor. O guia é o
//! cilindro do furo desenhado em linhas: um anel na boca, outro no fundo e quatro arestas ligando os
//! dois, mais o eixo. Boca escareada ou rebaixada ganha um anel maior.

use super::*;

/// Geometria do furo-guia em 3D: base ortonormal da face, raios e profundidade.
#[derive(Debug, PartialEq)]
pub(crate) struct Guia {
    pub(crate) ponto: Vec3,
    pub(crate) normal: Vec3,
    /// raio do furo e da boca (a boca é o maior)
    pub(crate) raio: f32,
    pub(crate) raio_boca: f32,
    pub(crate) fundo: f32,
}

/// Profundidade desenhada quando o furo é passante ou "até": um valor de convite, não a medida.
const FUNDO_CONVITE: f32 = 12.;

/// O comando Furo está esperando o clique numa face (modo pontos 3D, ou ainda sem centros)?
pub(crate) fn espera_clique(h: &Hole) -> bool {
    !h.points.is_empty() || crate::slots::is_unset(h.sketch)
}

/// Monta o guia a partir do comando e do ponto sob o cursor. `None` com valores que não valem.
pub(crate) fn montar(h: &Hole, vars: &std::collections::HashMap<String, f64>, ponto: Vec3, normal: Vec3) -> Option<Guia> {
    let eval = |s: &str| fenkra_core::expr::eval_str(s, vars).ok().filter(|v| v.is_finite() && *v > 0.);
    let d = eval(&h.diameter)? as f32;
    let boca = match &h.kind {
        HoleKind::Simples => d,
        HoleKind::Rebaixado { diameter, .. } | HoleKind::Escareado { diameter, .. } => eval(diameter).map(|b| b as f32).unwrap_or(d).max(d),
        _ => d,
    };
    let fundo = match &h.depth {
        Extent::Blind(p) => eval(p).map(|v| v as f32).unwrap_or(FUNDO_CONVITE),
        _ => FUNDO_CONVITE,
    };
    let normal = normal.try_normalize()?;
    // sentido do furo: para dentro do material, ou o contrário se o comando inverte
    Some(Guia { ponto, normal: if h.reverse { -normal } else { normal }, raio: d / 2., raio_boca: boca / 2., fundo })
}

impl Guia {
    /// Pontos de um anel de raio `r` à distância `z` da boca, para dentro do material.
    pub(crate) fn anel(&self, r: f32, z: f32, n: usize) -> Vec<Vec3> {
        let u = self.normal.any_orthonormal_vector();
        let v = self.normal.cross(u);
        (0..=n)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / n as f32;
                self.ponto - self.normal * z + (u * a.cos() + v * a.sin()) * r
            })
            .collect()
    }
}

/// Desenha o furo-guia sob o cursor.
pub(super) fn guia(app: &App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let Some(FeatureKind::Hole(h)) = app.open.doc.feature(id).map(|f| &f.kind) else { return };
    if !espera_clique(h) {
        return;
    }
    let (ponto, normal) = match app.hover_pick.as_ref() {
        Some(crate::picking::Pick::Face { point, normal, body, .. }) if *body < crate::picking::SURF_BASE => (Vec3::from(*point), Vec3::from(*normal)),
        Some(_) => return,
        // captura sem mouse: com `FENKRA_DEMO_PREVIA=furo_guia` o guia pousa no centro da face de cima
        None if std::env::var("FENKRA_DEMO_PREVIA").as_deref() == Ok("furo_guia") => (Vec3::new(0., 0., 20.), Vec3::Z),
        None => return,
    };
    let Some(g) = montar(h, &app.vars(), ponto, normal) else { return };
    let cor = crate::theme::accent(ui.visuals());
    let painter = ui.painter_at(rect);
    let proj = |ps: Vec<Vec3>| -> Vec<egui::Pos2> { ps.into_iter().filter_map(|p| cam.project(rect, p)).collect() };
    // contorno da boca (se escareada ou rebaixada, o anel maior) e o fundo do furo
    if g.raio_boca > g.raio + 1e-4 {
        painter.add(egui::Shape::line(proj(g.anel(g.raio_boca, 0., 40)), Stroke::new(1.5, cor)));
    }
    painter.add(egui::Shape::line(proj(g.anel(g.raio, 0., 40)), Stroke::new(2.5, cor)));
    painter.add(egui::Shape::line(proj(g.anel(g.raio, g.fundo, 40)), Stroke::new(1.5, cor.gamma_multiply(0.7))));
    for q in [0, 10, 20, 30] {
        let (a, b) = (g.anel(g.raio, 0., 40)[q], g.anel(g.raio, g.fundo, 40)[q]);
        if let (Some(pa), Some(pb)) = (cam.project(rect, a), cam.project(rect, b)) {
            painter.line_segment([pa, pb], Stroke::new(1., cor.gamma_multiply(0.7)));
        }
    }
    if let (Some(pa), Some(pb)) = (cam.project(rect, g.ponto), cam.project(rect, g.ponto - g.normal * (g.fundo + 2.))) {
        painter.line_segment([pa, pb], Stroke::new(1., cor));
        let texto = format!("Ø{} · {}", crate::app::fmt(g.raio as f64 * 2.), tr("clique para furar aqui"));
        let gl = painter.layout_no_wrap(texto, egui::FontId::proportional(12.), cor);
        let caixa = Rect::from_min_size(pa + egui::vec2(14., -22.), gl.size() + egui::vec2(8., 4.));
        painter.rect_filled(caixa, 3., ui.visuals().extreme_bg_color.gamma_multiply(0.92));
        painter.galley(caixa.min + egui::vec2(4., 2.), gl, cor);
    }
    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn furo(kind: HoleKind, depth: Extent) -> Hole {
        Hole { sketch: crate::slots::UNSET_SKETCH, kind, diameter: "6.6".into(), depth, thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default() }
    }

    #[test]
    fn guia_segue_a_normal_da_face_com_o_diametro_e_a_profundidade_do_comando() {
        let h = furo(HoleKind::Simples, Extent::Blind("8".into()));
        let g = montar(&h, &HashMap::new(), Vec3::new(1., 2., 3.), Vec3::new(0., 0., 2.)).unwrap();
        assert!((g.normal - Vec3::Z).length() < 1e-6 && (g.raio - 3.3).abs() < 1e-6 && (g.fundo - 8.).abs() < 1e-6);
        // o anel da boca está na face, a 3,3 do ponto; o do fundo, 8 mm abaixo
        let boca = g.anel(g.raio, 0., 16);
        assert!(boca.iter().all(|p| (p.z - 3.).abs() < 1e-5 && (((p.x - 1.).powi(2) + (p.y - 2.).powi(2)).sqrt() - 3.3).abs() < 1e-4));
        let fundo = g.anel(g.raio, g.fundo, 16);
        assert!(fundo.iter().all(|p| (p.z + 5.).abs() < 1e-5));
        // normal inclinada: o anel fica no plano da face, não no plano XY
        let incl = montar(&h, &HashMap::new(), Vec3::ZERO, Vec3::new(1., 0., 1.)).unwrap();
        assert!(incl.anel(incl.raio, 0., 16).iter().all(|p| p.dot(incl.normal).abs() < 1e-5));
        // inverter o furo vira o guia
        let mut inv = h.clone();
        inv.reverse = true;
        assert!((montar(&inv, &HashMap::new(), Vec3::ZERO, Vec3::Z).unwrap().normal + Vec3::Z).length() < 1e-6);
    }

    #[test]
    fn boca_escareada_ganha_anel_maior_e_passante_usa_profundidade_de_convite() {
        let h = furo(HoleKind::Escareado { diameter: "13".into(), angle: "90".into() }, Extent::ThroughAll);
        let g = montar(&h, &HashMap::new(), Vec3::ZERO, Vec3::Z).unwrap();
        assert!((g.raio_boca - 6.5).abs() < 1e-6 && g.fundo == FUNDO_CONVITE);
        // diâmetro inválido: sem guia
        let mut ruim = h.clone();
        ruim.diameter = "0".into();
        assert!(montar(&ruim, &HashMap::new(), Vec3::ZERO, Vec3::Z).is_none());
    }

    #[test]
    fn so_espera_clique_em_modo_pontos_ou_sem_centros() {
        let mut h = furo(HoleKind::Simples, Extent::ThroughAll);
        assert!(espera_clique(&h));
        h.sketch = Id(5);
        assert!(!espera_clique(&h));
        h.points = vec![PointRef::Fixed([0.; 3])];
        assert!(espera_clique(&h));
    }
}
