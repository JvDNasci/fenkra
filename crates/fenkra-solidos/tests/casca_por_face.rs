// SPDX-License-Identifier: LGPL-2.1-or-later
//! B06: casca com espessura própria em algumas faces.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

fn s(x: &str) -> String {
    x.to_string()
}

/// Caixa 60 × 40 × 30 aberta em cima, parede de 2 mm; `lista` traz as faces com parede própria.
fn caixa(lista: Vec<EspessuraFace>) -> (Document, Id) {
    let mut d = Document::new("t");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [60., 40.]);
    d.add_feature(ext(sk, BoolOp::Add, "30"));
    let id = d.add_feature(FeatureKind::Shell(Shell { espessuras: lista, faces: vec![[30., 20., 30.]], thickness: s("2"), outward: false, face_keys: vec![], closed: false, regras: vec![] }));
    (d, id)
}

#[test]
fn fundo_mais_grosso_tira_menos_material_na_conta_certa() {
    let (d, _) = caixa(vec![]);
    let r = recompute(&d);
    let geral = r.bodies[0].mass.volume;
    let (d, id) = caixa(vec![EspessuraFace { ponto: [30., 20., 0.], valor: s("6") }]);
    let r = recompute(&d);
    assert!(matches!(r.status[&id], Status::Ok), "{:?}", r.status[&id]);
    // o fundo sobe 4 mm por dentro: a cavidade perde 56 × 36 × 4
    let esperado = geral + 56. * 36. * 4.;
    let v = r.bodies[0].mass.volume;
    assert!((v - esperado).abs() < 1., "volume {v}, esperado {esperado}");
    let (lo, hi) = caixa_da_malha(&r);
    assert!((hi[0] - lo[0] - 60.).abs() < 0.01 && (hi[2] - lo[2] - 30.).abs() < 0.01, "{lo:?} {hi:?}");
}

#[test]
fn espessura_por_face_precisa_de_face_aberta() {
    let (mut d, id) = caixa(vec![EspessuraFace { ponto: [30., 20., 0.], valor: s("6") }]);
    let FeatureKind::Shell(c) = &mut d.feature_mut(id).unwrap().kind else { panic!() };
    c.faces.clear();
    c.closed = true;
    let r = recompute(&d);
    assert!(matches!(&r.status[&id], Status::Error(m) if m.contains("face aberta")), "{:?}", r.status[&id]);
}
