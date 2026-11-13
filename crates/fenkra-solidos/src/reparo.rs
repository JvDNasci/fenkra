// SPDX-License-Identifier: LGPL-2.1-or-later
//! Candidatas no 3D para o assistente de reparo (B15, ADR 0010 fase 2).
//!
//! Quando a face ou a aresta de um comando some (referência perdida) ou fica ambígua, o recompute
//! para com o erro e nunca escolhe em silêncio. Aqui o assistente acha, no corpo que a pessoa está
//! vendo, as candidatas: as do mesmo dono e papel (caso ambíguo) ou as mais perto do último ponto
//! conhecido. A pessoa clica numa delas no 3D e o ponto do comando passa a ser o dela; a chave v2 é
//! refeita no cálculo seguinte (promoção), como num clique novo.

use crate::refs::{self, View};
use crate::BodyResult;
use fenkra_core::{EdgeKey, EdgeMode, EdgeSel, FaceKey, Feature, FeatureKind};
use fenkra_kernel::{Topology, NO_FACE};

/// Quantas candidatas mostrar por referência perdida.
pub const MAX_CANDIDATAS: usize = 5;

/// O que a referência perdida escolhia.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alvo {
    Face,
    Aresta,
}

/// Uma referência do comando que não resolve no corpo atual, com as candidatas.
#[derive(Clone, Debug, PartialEq)]
pub struct Lacuna {
    pub alvo: Alvo,
    /// posição na lista de faces ou arestas do comando
    pub indice: usize,
    /// último ponto conhecido
    pub atual: [f64; 3],
    /// ponto de cada candidata (sobre a face, ou o ponto médio da aresta), a mais provável primeiro
    pub candidatas: Vec<[f64; 3]>,
}

enum Chave {
    Face(Option<FaceKey>),
    Aresta(Option<EdgeKey>),
}

/// Referências de face e aresta que o comando grava (ponto e chave), na ordem do recompute.
fn referencias(k: &FeatureKind) -> Vec<([f64; 3], Chave)> {
    let faces = |pts: &[[f64; 3]], keys: &[Option<FaceKey>]| pts.iter().enumerate().map(|(i, p)| (*p, Chave::Face(keys.get(i).cloned().flatten()))).collect::<Vec<_>>();
    match k {
        FeatureKind::Shell(s) => faces(&s.faces, &s.face_keys),
        FeatureKind::Draft(d) => faces(&d.faces, &d.face_keys),
        FeatureKind::Thickness(t) => faces(&t.faces, &t.face_keys),
        FeatureKind::DirectEdit(d) => faces(&d.faces, &d.face_keys),
        FeatureKind::Thread(t) => vec![(t.face, Chave::Face(t.face_key.clone()))],
        FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => match (&o.mode, &o.edges) {
            (EdgeMode::FaceFace(a, b), _) => faces(&[*a, *b], &o.extra.face_keys),
            (EdgeMode::FullRound { mid, a, b }, _) => faces(&[*mid, *a, *b], &o.extra.face_keys),
            (_, EdgeSel::Edges(m)) => m.iter().enumerate().map(|(i, p)| (*p, Chave::Aresta(o.keys.get(i).cloned().flatten()))).collect(),
            _ => vec![],
        },
        _ => vec![],
    }
}

/// Topologia de um corpo do resultado (a malha guarda faces, pares de faces e pontos médios).
fn topologia(b: &BodyResult) -> Topology {
    let m = &b.mesh;
    let n = m.edge_faces.len().min(m.edge_mids.len() / 3);
    Topology {
        faces: m.face_info.clone(),
        edge_mids: (0..n).map(|e| [m.edge_mids[3 * e] as f64, m.edge_mids[3 * e + 1] as f64, m.edge_mids[3 * e + 2] as f64]).collect(),
        edge_faces: m.edge_faces[..n].to_vec(),
    }
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

/// Distância do ponto ao segmento ab.
fn ao_segmento(p: [f64; 3], a: [f64; 3], b: [f64; 3]) -> f64 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let l2 = ab.iter().map(|x| x * x).sum::<f64>();
    let t = if l2 < 1e-18 { 0. } else { (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1] + (p[2] - a[2]) * ab[2]) / l2).clamp(0., 1.) };
    dist(p, [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t])
}

/// Distância do ponto à superfície da malha (vértices e arestas dos triângulos; basta para dizer se
/// o ponto ficou a mais de 1 mm de qualquer face).
fn a_malha(b: &BodyResult, p: [f64; 3]) -> f64 {
    let m = &b.mesh;
    let v = |i: u32| {
        let i = i as usize * 3;
        [m.positions[i] as f64, m.positions[i + 1] as f64, m.positions[i + 2] as f64]
    };
    m.indices
        .chunks_exact(3)
        .map(|t| {
            let (a, b2, c) = (v(t[0]), v(t[1]), v(t[2]));
            // dentro do triângulo: distância ao plano; fora: à borda mais perto
            let n = {
                let (u, w) = ([b2[0] - a[0], b2[1] - a[1], b2[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
                [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]]
            };
            let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            let borda = ao_segmento(p, a, b2).min(ao_segmento(p, b2, c)).min(ao_segmento(p, c, a));
            if nl < 1e-18 {
                return borda;
            }
            let n = [n[0] / nl, n[1] / nl, n[2] / nl];
            let h = (p[0] - a[0]) * n[0] + (p[1] - a[1]) * n[1] + (p[2] - a[2]) * n[2];
            let q = [p[0] - n[0] * h, p[1] - n[1] * h, p[2] - n[2] * h];
            let lado = |x: [f64; 3], y: [f64; 3]| {
                let (u, w) = ([y[0] - x[0], y[1] - x[1], y[2] - x[2]], [q[0] - x[0], q[1] - x[1], q[2] - x[2]]);
                let c = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
                c[0] * n[0] + c[1] * n[1] + c[2] * n[2] >= -1e-12
            };
            if lado(a, b2) && lado(b2, c) && lado(c, a) {
                h.abs()
            } else {
                borda
            }
        })
        .fold(f64::MAX, f64::min)
}

/// Referências do comando que não resolvem no corpo de destino, com as candidatas de cada uma.
/// Vazio quando tudo resolve (o erro do comando é de outra natureza) ou não há corpo.
pub fn lacunas(f: &Feature, bodies: &[BodyResult]) -> Vec<Lacuna> {
    let Some(b) = bodies.iter().find(|b| b.collector == f.target) else { return vec![] };
    let topo = topologia(b);
    let v = View { topo: &topo, owners: &b.face_owner, roles: &b.face_role };
    let mut out = vec![];
    for (i, (p, chave)) in referencias(&f.kind).into_iter().enumerate() {
        let (alvo, cand) = match chave {
            Chave::Face(key) => {
                let ok = match &key {
                    Some(k) => refs::find_face(&v, k, Some(p)).is_ok(),
                    None => a_malha(b, p) <= 1.0,
                };
                if ok {
                    continue;
                }
                (Alvo::Face, refs::candidatas_face(&v, key.as_ref(), p, MAX_CANDIDATAS).into_iter().map(|j| topo.faces[j].point).collect::<Vec<_>>())
            }
            Chave::Aresta(key) => {
                let ok = match &key {
                    Some(k) => refs::find_edge(&v, k, Some(p)).is_ok(),
                    None => refs::edge_near(&v, p, 0.1).is_some(),
                };
                if ok {
                    continue;
                }
                let c = refs::candidatas_aresta(&v, key.as_ref(), p, MAX_CANDIDATAS);
                (Alvo::Aresta, c.into_iter().filter(|e| topo.edge_faces[*e] != [NO_FACE, NO_FACE]).map(|e| topo.edge_mids[e]).collect())
            }
        };
        if !cand.is_empty() {
            out.push(Lacuna { alvo, indice: i, atual: p, candidatas: cand });
        }
    }
    out
}

/// Troca o ponto da referência `indice` pelo da candidata escolhida e apaga a chave dela (o cálculo
/// seguinte grava a chave nova). Devolve `false` se o comando não tem essa referência.
pub fn aplicar(k: &mut FeatureKind, alvo: Alvo, indice: usize, ponto: [f64; 3]) -> bool {
    fn troca(pts: &mut [[f64; 3]], keys: &mut [Option<FaceKey>], i: usize, p: [f64; 3]) -> bool {
        let Some(q) = pts.get_mut(i) else { return false };
        *q = p;
        if let Some(c) = keys.get_mut(i) {
            *c = None;
        }
        true
    }
    match (k, alvo) {
        (FeatureKind::Shell(s), Alvo::Face) => troca(&mut s.faces, &mut s.face_keys, indice, ponto),
        (FeatureKind::Draft(d), Alvo::Face) => troca(&mut d.faces, &mut d.face_keys, indice, ponto),
        (FeatureKind::Thickness(t), Alvo::Face) => troca(&mut t.faces, &mut t.face_keys, indice, ponto),
        (FeatureKind::DirectEdit(d), Alvo::Face) => troca(&mut d.faces, &mut d.face_keys, indice, ponto),
        (FeatureKind::Thread(t), Alvo::Face) if indice == 0 => {
            t.face = ponto;
            t.face_key = None;
            true
        }
        (FeatureKind::Fillet(o) | FeatureKind::Chamfer(o), alvo) => {
            let slot = match (&mut o.mode, alvo) {
                (EdgeMode::FaceFace(a, b), Alvo::Face) => [Some(a), Some(b), None].into_iter().nth(indice).flatten(),
                (EdgeMode::FullRound { mid, a, b }, Alvo::Face) => [mid, a, b].into_iter().nth(indice),
                _ => None,
            };
            if let Some(q) = slot {
                *q = ponto;
                if let Some(c) = o.extra.face_keys.get_mut(indice) {
                    *c = None;
                }
                return true;
            }
            match (&mut o.edges, alvo) {
                (EdgeSel::Edges(m), Alvo::Aresta) if indice < m.len() => {
                    m[indice] = ponto;
                    if let Some(c) = o.keys.get_mut(indice) {
                        *c = None;
                    }
                    true
                }
                _ => false,
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{EdgeOp, Id, Shell};

    #[test]
    fn aplicar_troca_o_ponto_e_apaga_a_chave() {
        let key = FaceKey { owner: Id(3), role: 1, kind: 0, normal_or_axis: [0., 0., 1.], area: 4. };
        let mut k = FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0.; 3], [1.; 3]], thickness: "1".into(), outward: false, face_keys: vec![Some(key.clone()), Some(key)], closed: false, regras: vec![] });
        assert!(aplicar(&mut k, Alvo::Face, 1, [5., 5., 5.]));
        let FeatureKind::Shell(s) = &k else { unreachable!() };
        assert_eq!(s.faces[1], [5., 5., 5.]);
        assert!(s.face_keys[0].is_some() && s.face_keys[1].is_none());
        assert!(!aplicar(&mut k, Alvo::Face, 9, [0.; 3]));
        assert!(!aplicar(&mut k, Alvo::Aresta, 0, [0.; 3]));
        let mut f = FeatureKind::Fillet(EdgeOp { value: "1".into(), edges: EdgeSel::Edges(vec![[0.; 3]]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![None], extra: Default::default() });
        assert!(aplicar(&mut f, Alvo::Aresta, 0, [2., 0., 0.]));
        assert!(!aplicar(&mut f, Alvo::Face, 0, [2., 0., 0.]));
    }
}
