// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cota inteligente (B268): o tipo da cota sai dos elementos escolhidos, na ordem dos cliques.
//!
//! A decisão é função pura dos alvos (`PmiTarget`) e, para arestas retas, do segmento delas:
//! - uma aresta circular ou face cilíndrica pede diâmetro (Tab alterna com raio);
//! - dois círculos pedem a distância entre eles (Tab: centros, interior, exterior);
//! - duas faces planas paralelas pedem a distância entre os planos;
//! - duas arestas retas concorrentes pedem o ângulo entre elas; paralelas, a distância;
//! - dois ou três vértices: distância (Tab: ao longo de X, Y, Z); o terceiro vértice vira ângulo,
//!   com vértice no 1º clique e pontos 1 e 2 nos seguintes.
//!
//! O resultado diz se a seleção já basta (`Complete`), se serve mas ainda aceita outro clique
//! (`Provisional`, confirmado com Enter), se falta elemento (`Pending`) ou se é inválida, com o
//! motivo e o clique responsável.

use crate::pmi::{GapMode, PmiKind, PmiTarget, TargetClass};

type P = [f64; 3];

/// Elemento escolhido: o alvo e, para aresta reta, as pontas.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Item {
    pub t: PmiTarget,
    pub seg: Option<(P, P)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// o elemento não tem medida (face curva livre, excesso de cliques)
    NotMeasurable,
    /// faces planas não paralelas: a distância não existe
    PlanesNotParallel,
    /// pontos alinhados ou repetidos: não formam ângulo. `vertex` é o clique que é o vértice (0-based)
    AngleDegenerate { vertex: usize },
    /// as duas arestas estão na mesma reta
    EdgesCollinear,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// nada escolhido ainda
    Empty,
    /// falta elemento
    Pending,
    /// a seleção já vale uma cota, mas outro clique a transformaria em outra (Enter confirma)
    Provisional,
    /// cria sozinha
    Complete,
    Invalid(Reason),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Inference {
    pub verdict: Verdict,
    /// variantes que o Tab percorre; a primeira é a inferida
    pub variants: Vec<PmiKind>,
}

impl Inference {
    pub fn kind(&self, tab: usize) -> Option<PmiKind> {
        (!self.variants.is_empty()).then(|| self.variants[tab % self.variants.len()].clone())
    }
    pub fn is_invalid(&self) -> bool {
        matches!(self.verdict, Verdict::Invalid(_))
    }
}

impl Reason {
    /// Mensagem em português (a interface traduz).
    pub fn message(self) -> &'static str {
        match self {
            Reason::NotMeasurable => "este elemento não tem medida para a cota",
            Reason::PlanesNotParallel => "faces não paralelas: escolha duas arestas para o ângulo",
            Reason::AngleDegenerate { .. } => "pontos alinhados ou repetidos: o 1º clique é o vértice e os outros dois precisam sair da mesma reta",
            Reason::EdgesCollinear => "as duas arestas estão na mesma reta",
        }
    }
}

/// Nome do papel do clique `k` (0-based) numa cota angular.
pub fn angle_role(k: usize) -> &'static str {
    match k {
        0 => "vértice",
        1 => "ponto 1",
        _ => "ponto 2",
    }
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: P, b: P) -> P {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn len(a: P) -> f64 {
    dot(a, a).sqrt()
}

fn circ(t: &PmiTarget) -> bool {
    matches!(t.class, TargetClass::ArestaCircular | TargetClass::FaceCilindrica)
}

/// Os três pontos formam um ângulo com `vértice, ponto 1, ponto 2`? Nenhum ponto repetido e os
/// vetores a partir do vértice não alinhados (seno > 0,02, cerca de 1,1°).
pub fn check_angle(v: P, p: P, q: P) -> Result<(), Reason> {
    let (u, w) = (sub(p, v), sub(q, v));
    let (lu, lw) = (len(u), len(w));
    if lu < 1e-6 || lw < 1e-6 || len(cross(u, w)) / (lu * lw) < 0.02 {
        return Err(Reason::AngleDegenerate { vertex: 0 });
    }
    Ok(())
}

/// Âncoras (`vértice, ponto 1, ponto 2`) do ângulo entre duas arestas retas: o vértice é o
/// ponto de maior aproximação das retas e cada ponto é a ponta da aresta mais longe dele.
/// `None` se as retas são paralelas.
pub fn angle_from_segments(a: (P, P), b: (P, P)) -> Option<[P; 3]> {
    let (d1, d2) = (sub(a.1, a.0), sub(b.1, b.0));
    let n = cross(d1, d2);
    let nn = dot(n, n);
    if nn.sqrt() / (len(d1) * len(d2)).max(1e-12) < 1e-3 {
        return None;
    }
    let r = sub(b.0, a.0);
    let t = dot(cross(r, d2), n) / nn;
    let s = dot(cross(r, d1), n) / nn;
    let c1 = [a.0[0] + d1[0] * t, a.0[1] + d1[1] * t, a.0[2] + d1[2] * t];
    let c2 = [b.0[0] + d2[0] * s, b.0[1] + d2[1] * s, b.0[2] + d2[2] * s];
    let v = [(c1[0] + c2[0]) / 2., (c1[1] + c2[1]) / 2., (c1[2] + c2[2]) / 2.];
    let far = |s: (P, P)| if len(sub(s.0, v)) >= len(sub(s.1, v)) { s.0 } else { s.1 };
    Some([v, far(a), far(b)])
}

fn parallel_segments(a: (P, P), b: (P, P)) -> bool {
    let (d1, d2) = (sub(a.1, a.0), sub(b.1, b.0));
    len(cross(d1, d2)) / (len(d1) * len(d2)).max(1e-12) < 1e-3
}

fn point_variants() -> Vec<PmiKind> {
    vec![PmiKind::Linear, PmiKind::Ordinate { axis: 0 }, PmiKind::Ordinate { axis: 1 }, PmiKind::Ordinate { axis: 2 }]
}

fn out(verdict: Verdict, variants: Vec<PmiKind>) -> Inference {
    Inference { verdict, variants }
}

/// Infere o tipo de cota a partir dos elementos escolhidos, em ordem de clique.
pub fn infer(items: &[Item]) -> Inference {
    let bad = |r| out(Verdict::Invalid(r), vec![]);
    if items.iter().any(|i| i.t.class == TargetClass::FaceOutra) {
        return bad(Reason::NotMeasurable);
    }
    match items {
        [] => out(Verdict::Empty, vec![]),
        [a] if circ(&a.t) => out(Verdict::Provisional, vec![PmiKind::Diameter, PmiKind::Radius]),
        [_] => out(Verdict::Pending, vec![PmiKind::Linear]),
        [a, b] => {
            use TargetClass::*;
            let (ca, cb) = (a.t.class, b.t.class);
            if circ(&a.t) && circ(&b.t) {
                return out(Verdict::Complete, vec![PmiKind::Gap { mode: GapMode::Centers }, PmiKind::Gap { mode: GapMode::Inner }, PmiKind::Gap { mode: GapMode::Outer }]);
            }
            if ca == FacePlana && cb == FacePlana {
                return if dot(a.t.dir, b.t.dir).abs() > 0.99995 { out(Verdict::Complete, vec![PmiKind::Linear]) } else { bad(Reason::PlanesNotParallel) };
            }
            if let (Aresta, Aresta, Some(s1), Some(s2)) = (ca, cb, a.seg, b.seg) {
                if parallel_segments(s1, s2) {
                    // retas paralelas: a distância vale; sobrepostas, não há o que medir
                    let d = sub(s2.0, s1.0);
                    let u = sub(s1.1, s1.0);
                    let perp = len(cross(d, u)) / len(u).max(1e-12);
                    return if perp < 1e-6 { bad(Reason::EdgesCollinear) } else { out(Verdict::Complete, vec![PmiKind::Linear]) };
                }
                return out(Verdict::Complete, vec![PmiKind::Angle]);
            }
            if ca == Vertice && cb == Vertice {
                return out(Verdict::Provisional, point_variants());
            }
            out(Verdict::Complete, point_variants())
        }
        [a, b, c] if [a, b, c].iter().all(|i| i.t.class == TargetClass::Vertice) => match check_angle(a.t.point, b.t.point, c.t.point) {
            Ok(()) => out(Verdict::Complete, vec![PmiKind::Angle]),
            Err(r) => bad(r),
        },
        _ => bad(Reason::NotMeasurable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(class: TargetClass, point: P, dir: P, radius: f64) -> PmiTarget {
        PmiTarget { class, point, dir, radius, concave: false }
    }
    fn vert(x: f64, y: f64, z: f64) -> Item {
        Item { t: t(TargetClass::Vertice, [x, y, z], [0.; 3], 0.), seg: None }
    }
    fn plano(z: f64, n: P) -> Item {
        Item { t: t(TargetClass::FacePlana, [0., 0., z], n, 0.), seg: None }
    }
    fn furo(x: f64) -> Item {
        Item { t: t(TargetClass::FaceCilindrica, [x, 0., 0.], [0., 0., 1.], 3.), seg: None }
    }
    fn reta(a: P, b: P) -> Item {
        Item { t: t(TargetClass::Aresta, [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2., (a[2] + b[2]) / 2.], [0.; 3], 0.), seg: Some((a, b)) }
    }

    #[test]
    fn aresta_circular_vira_diametro_e_tab_troca_para_raio() {
        let r = infer(&[furo(0.)]);
        assert_eq!(r.verdict, Verdict::Provisional);
        assert_eq!(r.kind(0), Some(PmiKind::Diameter));
        assert_eq!(r.kind(1), Some(PmiKind::Radius));
        assert_eq!(r.kind(2), Some(PmiKind::Diameter));
    }

    #[test]
    fn dois_furos_viram_distancia_entre_circulos_com_tres_modos() {
        let r = infer(&[furo(0.), furo(20.)]);
        assert_eq!(r.verdict, Verdict::Complete);
        assert_eq!(r.kind(0), Some(PmiKind::Gap { mode: GapMode::Centers }));
        assert_eq!(r.kind(1), Some(PmiKind::Gap { mode: GapMode::Inner }));
        assert_eq!(r.kind(2), Some(PmiKind::Gap { mode: GapMode::Outer }));
    }

    #[test]
    fn faces_paralelas_dao_distancia_e_nao_paralelas_sao_invalidas() {
        let r = infer(&[plano(0., [0., 0., -1.]), plano(5., [0., 0., 1.])]);
        assert_eq!((r.verdict, r.kind(0)), (Verdict::Complete, Some(PmiKind::Linear)));
        let r = infer(&[plano(0., [0., 0., 1.]), plano(5., [1., 0., 0.])]);
        assert_eq!(r.verdict, Verdict::Invalid(Reason::PlanesNotParallel));
        assert!(r.kind(0).is_none());
    }

    #[test]
    fn duas_arestas_concorrentes_dao_angulo_e_paralelas_dao_distancia() {
        let r = infer(&[reta([0., 0., 0.], [10., 0., 0.]), reta([0., 0., 0.], [0., 10., 0.])]);
        assert_eq!((r.verdict, r.kind(0)), (Verdict::Complete, Some(PmiKind::Angle)));
        let r = infer(&[reta([0., 0., 0.], [10., 0., 0.]), reta([0., 5., 0.], [10., 5., 0.])]);
        assert_eq!((r.verdict, r.kind(0)), (Verdict::Complete, Some(PmiKind::Linear)));
        let r = infer(&[reta([0., 0., 0.], [10., 0., 0.]), reta([2., 0., 0.], [8., 0., 0.])]);
        assert_eq!(r.verdict, Verdict::Invalid(Reason::EdgesCollinear));
    }

    #[test]
    fn angulo_de_arestas_acha_vertice_e_pontas() {
        let [v, p, q] = angle_from_segments(([0., 0., 0.], [10., 0., 0.]), ([0., 0., 0.], [0., 7., 0.])).unwrap();
        assert!(len(v) < 1e-9);
        assert_eq!((p, q), ([10., 0., 0.], [0., 7., 0.]));
        // arestas que só se encontram no prolongamento: o vértice é a interseção das retas
        let [v, ..] = angle_from_segments(([2., 0., 0.], [10., 0., 0.]), ([0., 3., 0.], [0., 9., 0.])).unwrap();
        assert!(len(v) < 1e-9);
        assert!(angle_from_segments(([0.; 3], [1., 0., 0.]), ([0., 1., 0.], [3., 1., 0.])).is_none());
    }

    #[test]
    fn vertices_dao_distancia_provisoria_e_o_terceiro_vira_angulo() {
        let r = infer(&[vert(0., 0., 0.)]);
        assert_eq!(r.verdict, Verdict::Pending);
        let r = infer(&[vert(0., 0., 0.), vert(10., 0., 0.)]);
        assert_eq!((r.verdict, r.kind(0), r.kind(1)), (Verdict::Provisional, Some(PmiKind::Linear), Some(PmiKind::Ordinate { axis: 0 })));
        let r = infer(&[vert(0., 0., 0.), vert(10., 0., 0.), vert(0., 10., 0.)]);
        assert_eq!((r.verdict, r.kind(0)), (Verdict::Complete, Some(PmiKind::Angle)));
    }

    #[test]
    fn angulo_com_pontos_alinhados_diz_qual_clique_e_o_vertice() {
        let r = infer(&[vert(0., 0., 0.), vert(5., 0., 0.), vert(10., 0., 0.)]);
        assert_eq!(r.verdict, Verdict::Invalid(Reason::AngleDegenerate { vertex: 0 }));
        assert!(Reason::AngleDegenerate { vertex: 0 }.message().contains("1º clique é o vértice"));
        let r = infer(&[vert(0., 0., 0.), vert(0., 0., 0.), vert(10., 0., 0.)]);
        assert!(r.is_invalid());
        assert_eq!((angle_role(0), angle_role(1), angle_role(2)), ("vértice", "ponto 1", "ponto 2"));
    }

    #[test]
    fn face_curva_livre_e_excesso_de_cliques_sao_invalidos() {
        let curva = Item { t: t(TargetClass::FaceOutra, [0.; 3], [0.; 3], 0.), seg: None };
        assert_eq!(infer(&[curva]).verdict, Verdict::Invalid(Reason::NotMeasurable));
        let v = vert(0., 0., 0.);
        assert!(infer(&[v, v, v, v]).is_invalid());
    }

    #[test]
    fn furo_e_vertice_viram_linear_completo() {
        let r = infer(&[furo(0.), vert(10., 0., 0.)]);
        assert_eq!((r.verdict, r.kind(0)), (Verdict::Complete, Some(PmiKind::Linear)));
    }
}
