// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cordão de solda visível no PMI (B369): a aresta que o símbolo de solda marca ganha uma linha
//! engrossada do tamanho da perna definida na anotação, como se fosse a solda. É só exibição: não
//! muda o sólido, a massa nem a lista de corte, e pode ser escondida no painel da solda.

use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use fenkra_core::pmi::{weld_leg_mm, Pmi, PmiKind};
use fenkra_solidos::RecomputeResult;
use glam::Vec3;

/// Segmentos (extremos no modelo) da aresta cujo ponto médio é `q`.
pub fn segmentos_da_aresta(r: &RecomputeResult, q: [f64; 3]) -> Vec<([f32; 3], [f32; 3])> {
    let perto = |v: &[f32]| (0..3).all(|i| (v[i] as f64 - q[i]).abs() < 1e-3);
    for b in &r.bodies {
        let m = &b.mesh;
        if let Some(e) = m.edge_mids.chunks_exact(3).position(perto) {
            let Some(&s0) = m.edge_ranges.get(e) else { continue };
            let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(m.edges.len() / 6);
            return m.edges.chunks_exact(6).take(s1).skip(s0 as usize).map(|s| ([s[0], s[1], s[2]], [s[3], s[4], s[5]])).collect();
        }
    }
    vec![]
}

/// Perna e aresta marcada por uma anotação de solda; `None` se não é solda, se o cordão está
/// escondido ou se a medida não tem número.
pub fn cordao_de(r: &RecomputeResult, pm: &Pmi, ancoras: &[[f64; 3]]) -> Option<(f64, Vec<([f32; 3], [f32; 3])>)> {
    let PmiKind::Weld { size, extra, .. } = &pm.kind else { return None };
    if extra.hide_bead {
        return None;
    }
    let perna = weld_leg_mm(size)?;
    let segs = segmentos_da_aresta(r, *ancoras.first()?);
    (!segs.is_empty()).then_some((perna, segs))
}

/// Pinta o cordão: traço largo e translúcido (a largura em tela acompanha o zoom: perna em mm) e
/// um fio no centro. A cor vem do tema.
pub fn desenhar(p: &egui::Painter, cam: &Camera, rect: Rect, perna: f64, segs: &[([f32; 3], [f32; 3])], cor: Color32) {
    let Some(meio) = segs.get(segs.len() / 2).map(|s| (Vec3::from(s.0) + Vec3::from(s.1)) * 0.5) else { return };
    let (rt, _) = cam.basis();
    let ppm = match (cam.project(rect, meio), cam.project(rect, meio + rt)) {
        (Some(a), Some(b)) => (b - a).length(),
        _ => return,
    };
    let larg = (perna as f32 * ppm).clamp(3., 90.);
    let largo = Stroke::new(larg, cor.gamma_multiply(0.45));
    let fio = Stroke::new(1.5, cor);
    for (a, b) in segs {
        let (Some(a), Some(b)) = (cam.project(rect, Vec3::from(*a)), cam.project(rect, Vec3::from(*b))) else { continue };
        p.line_segment([a, b], largo);
        p.line_segment([a, b], fio);
        // pontas arredondadas: a linha do egui termina reta
        for q in [a, b] {
            p.circle_filled(q as Pos2, larg / 2., largo.color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::pmi::{WeldExtra, WeldKind};
    use fenkra_core::Id;

    fn solda(size: &str, extra: WeldExtra, ancora: [f64; 3]) -> Pmi {
        Pmi {
            id: Id(1),
            name: "Solda 1".into(),
            kind: PmiKind::Weld { kind: WeldKind::Filete, size: size.into(), all_around: false, extra },
            anchors: vec![ancora],
            offset: [10., 0., 10.],
            text: String::new(),
            tol_plus: String::new(),
            tol_minus: String::new(),
            visible: true,
            plane: None,
            style: Default::default(),
            folder: String::new(),
            targets: vec![],
        }
    }

    fn peca() -> RecomputeResult {
        let doc = fenkra_solidos::example_document();
        fenkra_solidos::recompute(&doc)
    }

    #[test]
    fn cordao_segue_a_aresta_marcada_e_a_perna_da_anotacao() {
        let r = peca();
        let m = &r.bodies[0].mesh;
        let mid = [m.edge_mids[0] as f64, m.edge_mids[1] as f64, m.edge_mids[2] as f64];
        let s = solda("z5", WeldExtra::default(), mid);
        let (perna, segs) = cordao_de(&r, &s, &s.anchors).expect("a aresta marcada vira cordão");
        assert_eq!(perna, 5.);
        assert!(!segs.is_empty());
        // os segmentos são da aresta: algum deles passa pelo ponto marcado (o meio fica sobre o traçado)
        let dist = |a: [f32; 3], b: [f32; 3]| (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f32>().sqrt();
        let q = [mid[0] as f32, mid[1] as f32, mid[2] as f32];
        assert!(segs.iter().any(|(a, b)| dist(*a, q) + dist(q, *b) - dist(*a, *b) < 0.05), "o ponto marcado está sobre o cordão");
        // a garganta a4 dá a perna 5,66
        let g = solda("a4", WeldExtra::default(), mid);
        assert!((cordao_de(&r, &g, &g.anchors).unwrap().0 - 5.657).abs() < 0.01);
    }

    #[test]
    fn cordao_escondido_ou_sem_medida_nao_aparece_e_nao_toca_no_modelo() {
        let r = peca();
        let m = &r.bodies[0].mesh;
        let mid = [m.edge_mids[0] as f64, m.edge_mids[1] as f64, m.edge_mids[2] as f64];
        let escondido = solda("z5", WeldExtra { hide_bead: true, ..Default::default() }, mid);
        assert!(cordao_de(&r, &escondido, &escondido.anchors).is_none());
        let sem = solda("", WeldExtra::default(), mid);
        assert!(cordao_de(&r, &sem, &sem.anchors).is_none());
        let solta = solda("z5", WeldExtra::default(), [1e6, 0., 0.]);
        assert!(cordao_de(&r, &solta, &solta.anchors).is_none(), "ponto fora de qualquer aresta");
        // exibição: o documento recalculado tem o mesmo volume e a mesma caixa com ou sem cordão
        let mut d = fenkra_solidos::example_document();
        let antes: f64 = fenkra_solidos::recompute(&d).bodies.iter().map(|b| b.mass.volume).sum();
        d.pmi.push(solda("z5", WeldExtra::default(), mid));
        let depois: f64 = fenkra_solidos::recompute(&d).bodies.iter().map(|b| b.mass.volume).sum();
        assert_eq!(antes, depois);
    }
}
