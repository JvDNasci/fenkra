// SPDX-License-Identifier: LGPL-2.1-or-later
//! B477: comando recém-aberto cuja face está em (0,0,0) ("a escolher", como Rosca, Casca e Saída)
//! espera a seleção: inativo e sem "referência perdida". Face em outro ponto que não existe segue erro.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

fn bloco() -> Document {
    let mut d = Document::new("t");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [60., 40.]);
    d.add_feature(ext(sk, BoolOp::Add, "30"));
    d
}

fn rosca(face: [f64; 3]) -> FeatureKind {
    FeatureKind::Thread(Thread { face, spec: "M10x1.5".into(), length: "15".into(), modeled: false, end: None, start: String::new(), face_key: None })
}

#[test]
fn rosca_sem_face_espera_sem_erro() {
    let mut d = bloco();
    let id = d.add_feature(rosca([0.; 3]));
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Inactive, "{:?}", r.status[&id]);
}

#[test]
fn casca_e_saida_sem_face_esperam_sem_erro() {
    let mut d = bloco();
    let c = d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0.; 3]], thickness: "2".into(), outward: false, face_keys: vec![], closed: false, regras: vec![] }));
    let r = recompute(&d);
    assert!(!matches!(r.status[&c], Status::Error(ref m) if m.contains("referência perdida")), "{:?}", r.status[&c]);
}

#[test]
fn face_que_nao_existe_de_verdade_segue_erro() {
    let mut d = bloco();
    let id = d.add_feature(rosca([500., 500., 500.]));
    let r = recompute(&d);
    assert!(matches!(&r.status[&id], Status::Error(m) if m.contains("referência perdida")), "{:?}", r.status[&id]);
}
