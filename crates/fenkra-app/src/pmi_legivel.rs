// SPDX-License-Identifier: LGPL-2.1-or-later
//! Legibilidade da anotação ao girar o modelo (B23). Uma chamada com várias pontas e vértices
//! intermediários é limpa numa vista e pode se cruzar noutra; o texto deitado num plano some de
//! perfil. Aqui se projeta o caminho na tela e se diz o que ficou ilegível, para a interface
//! avisar junto ao rótulo em vez de deixar a pessoa descobrir ao girar.

use crate::render::Camera;
use eframe::egui::{Pos2, Rect};
use fenkra_core::pmi::{Pmi, PmiKind};
use glam::Vec3;

/// O que ficou ilegível na vista atual.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Aviso {
    /// dois trechos da chamada se cruzam (ou uma ponta passa por cima de outra) na tela
    Cruza,
    /// o texto deitado no plano está quase de perfil para a câmera
    DePerfil,
}

impl Aviso {
    pub fn mensagem(self) -> &'static str {
        match self {
            Aviso::Cruza => "chamada se cruza nesta vista: gire ou mova um vértice",
            Aviso::DePerfil => "texto quase de perfil nesta vista: gire para ler",
        }
    }
}

/// Os segmentos `p1p2` e `p3p4` se cruzam na tela (ou ficam a menos de `tol` px um do outro)?
fn cruzam(p1: Pos2, p2: Pos2, p3: Pos2, p4: Pos2, tol: f32) -> bool {
    let o = |a: Pos2, b: Pos2, c: Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let (d1, d2, d3, d4) = (o(p3, p4, p1), o(p3, p4, p2), o(p1, p2, p3), o(p1, p2, p4));
    if ((d1 > 0.) != (d2 > 0.)) && ((d3 > 0.) != (d4 > 0.)) && d1 != 0. && d2 != 0. && d3 != 0. && d4 != 0. {
        return true;
    }
    let dist = |p: Pos2, a: Pos2, b: Pos2| {
        let ab = b - a;
        let l2 = ab.length_sq();
        let t = if l2 < 1e-9 { 0. } else { ((p - a).dot(ab) / l2).clamp(0., 1.) };
        (p - (a + ab * t)).length()
    };
    [dist(p1, p3, p4), dist(p2, p3, p4), dist(p3, p1, p2), dist(p4, p1, p2)].into_iter().fold(f32::MAX, f32::min) < tol
}

/// Caminhos já projetados (um por ponta, do rótulo para a ponta): algum cruza a si mesmo ou a outro?
pub fn caminhos_se_cruzam(caminhos: &[Vec<Pos2>], tol: f32) -> bool {
    let segs: Vec<(usize, usize, Pos2, Pos2)> = caminhos.iter().enumerate().flat_map(|(c, p)| p.windows(2).enumerate().map(move |(i, w)| (c, i, w[0], w[1]))).collect();
    for (x, a) in segs.iter().enumerate() {
        for b in &segs[x + 1..] {
            // trechos vizinhos do mesmo caminho dividem uma ponta; os primeiros trechos de todos os
            // caminhos nascem no mesmo rótulo e não contam como cruzamento
            let vizinhos = a.0 == b.0 && b.1 <= a.1 + 1;
            let mesmo_inicio = a.0 != b.0 && a.1 == 0 && b.1 == 0 && (a.2 - b.2).length() < 0.5;
            if vizinhos || mesmo_inicio {
                continue;
            }
            let folga = if a.0 != b.0 { 0.5 } else { tol };
            if cruzam(a.2, a.3, b.2, b.3, folga) {
                return true;
            }
        }
    }
    false
}

/// Aviso da anotação na vista: `a` são as âncoras já reencaixadas.
pub fn aviso(cam: &Camera, rect: Rect, pm: &Pmi, a: &[[f64; 3]]) -> Option<Aviso> {
    let pontas_e_vertices = a.len() + pm.style.leader_via.iter().map(Vec::len).sum::<usize>();
    if matches!(pm.kind, PmiKind::Note | PmiKind::FeatureNote { .. }) && pontas_e_vertices >= 3 {
        let first = a.first().copied()?;
        let label = [0, 1, 2].map(|i| first[i] + pm.offset[i]);
        let mut p = pm.clone();
        p.anchors = a.to_vec();
        let caminhos: Vec<Vec<Pos2>> = (0..a.len())
            .filter_map(|k| fenkra_core::pmi_leader::route(&p, label, k).iter().map(|q| cam.project(rect, Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32))).collect::<Option<Vec<_>>>())
            .collect();
        if caminhos_se_cruzam(&caminhos, 2.) {
            return Some(Aviso::Cruza);
        }
    }
    if let Some((n, _)) = pm.plane {
        let n = Vec3::new(n[0] as f32, n[1] as f32, n[2] as f32).normalize_or_zero();
        let (_, dir) = cam.ray(rect, rect.center());
        if n.length() > 0.5 && n.dot(dir).abs() < 0.2 {
            return Some(Aviso::DePerfil);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;
    use fenkra_core::pmi_leader::{add_tip, add_vertex};
    use fenkra_core::Id;

    fn nota() -> Pmi {
        Pmi { id: Id(1), name: "n".into(), kind: PmiKind::Note, anchors: vec![[0., 0., 0.]], offset: [0., 0., 30.], text: "x".into(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] }
    }

    fn vista(yaw: f32) -> Camera {
        Camera { target: Vec3::new(0., 0., 15.), yaw, pitch: 0., dist: 300., ortho: true }
    }

    #[test]
    fn x_na_tela_cruza_e_leque_nao() {
        let leque = vec![vec![pos2(0., 0.), pos2(-10., 20.)], vec![pos2(0., 0.), pos2(10., 20.)]];
        assert!(!caminhos_se_cruzam(&leque, 2.));
        let x = vec![vec![pos2(0., 0.), pos2(20., 20.)], vec![pos2(20., 0.), pos2(0., 20.)]];
        assert!(caminhos_se_cruzam(&x, 2.));
        let dobra = vec![vec![pos2(0., 0.), pos2(10., 10.), pos2(0., 10.), pos2(10., 0.)]];
        assert!(caminhos_se_cruzam(&dobra, 2.), "o último trecho cruza o primeiro");
    }

    #[test]
    fn nota_de_tres_pontas_e_dois_vertices_e_limpa_numa_vista_e_cruzada_noutra() {
        // ponta 0 desce em zigue-zague no plano XZ (dois vértices); a ponta 1 vai para a frente (Y) e a 2 para a esquerda
        let mut n = nota();
        let label = [0., 0., 30.];
        add_tip(&mut n, [0., 40., 0.], None);
        add_tip(&mut n, [-40., 0., 0.], None);
        add_vertex(&mut n, label, 0, [20., 0., 20.]).unwrap();
        add_vertex(&mut n, label, 0, [-20., 0., 10.]).unwrap();
        assert_eq!((n.anchors.len(), n.style.leader_via[0].len()), (3, 2));
        let a = n.anchors.clone();
        let rect = Rect::from_min_size(pos2(0., 0.), eframe::egui::vec2(800., 600.));
        let avisos: Vec<Option<Aviso>> = (0..24).map(|k| aviso(&vista(k as f32 * std::f32::consts::TAU / 24.), rect, &n, &a)).collect();
        assert!(avisos.iter().any(|v| *v == Some(Aviso::Cruza)), "alguma vista cruza a chamada: {avisos:?}");
        assert!(avisos.iter().any(|v| v.is_none()), "alguma vista deixa a chamada limpa: {avisos:?}");
    }

    #[test]
    fn texto_deitado_de_perfil_avisa() {
        let mut n = nota();
        n.plane = Some(([0., 1., 0.], [1., 0., 0.]));
        let a = n.anchors.clone();
        let rect = Rect::from_min_size(pos2(0., 0.), eframe::egui::vec2(800., 600.));
        // olhando ao longo de X: o plano de normal Y fica de perfil
        assert_eq!(aviso(&vista(0.), rect, &n, &a), Some(Aviso::DePerfil));
        // olhando ao longo de Y: de frente
        assert_eq!(aviso(&vista(-std::f32::consts::FRAC_PI_2), rect, &n, &a), None);
    }
}
