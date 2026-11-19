// SPDX-License-Identifier: LGPL-2.1-or-later
//! B522 e B553: converter sólido maciço em chapa e corpo único da parede base.
//!
//! Cada caso monta o documento em código, recalcula e confere status, volume e caixa envolvente.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn cubo(lado: f64) -> Document {
    let mut d = Document::new("cubo");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [lado, lado]);
    d.add_feature(ext(sk, BoolOp::Add, &format!("{lado}")));
    d
}

fn converte(d: &mut Document, espessura: &str, abertas: Vec<[f64; 3]>) -> Id {
    d.add_feature(FeatureKind::SmConvert(SmConvert { face: [0.; 3], thickness: s(espessura), open_faces: abertas, solido: true }))
}

fn estado(d: &Document, id: Id) -> Status {
    recompute(d).status[&id].clone()
}

/// Cubo de 100 mm, parede de 2 mm e a face de cima aberta: caixa de chapa com cinco paredes, que
/// dobra e planifica.
#[test]
fn cubo_com_uma_face_aberta_vira_caixa_de_chapa_planificavel() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = cubo(100.);
    let id = converte(&mut d, "2", vec![[50., 50., 100.]]);
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    assert_eq!(r.bodies.len(), 1);
    // cinco paredes (fundo e quatro laterais): quatro dobras no mapa
    assert_eq!(r.bend_lines.len(), 4);
    // cinco paredes de 2 mm: 100³ menos a cavidade de 96×96×98, e menos o material dos rasgos
    let v = r.bodies[0].mass.volume;
    let casca = 1_000_000. - 96. * 96. * 98.;
    assert!(v > 0.9 * casca && v < 1.01 * casca, "volume {v} contra casca {casca}");
    let (lo, hi) = caixa_da_malha(&r);
    for i in 0..3 {
        assert!((hi[i] - lo[i] - 100.).abs() < 0.5, "eixo {i}: {}", hi[i] - lo[i]);
    }
    // planificada: uma chapa plana de 2 mm, com todas as paredes desdobradas
    let mut p = d.clone();
    p.sheet.as_mut().unwrap().flat = true;
    let rp = recompute(&p);
    assert_eq!(rp.status[&id], Status::Ok, "{:?}", rp.status[&id]);
    assert_eq!(rp.bodies.len(), 1);
    let (lo, hi) = caixa_da_malha(&rp);
    assert!((hi[2] - lo[2] - 2.).abs() < 0.05, "z plano {}", hi[2] - lo[2]);
    assert!(hi[0] - lo[0] > 150. || hi[1] - lo[1] > 150., "planificada pequena: {lo:?} {hi:?}");
}

#[test]
fn cubo_sem_face_aberta_diz_o_que_escolher() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = cubo(100.);
    let id = converte(&mut d, "2", vec![]);
    match estado(&d, id) {
        Status::Error(m) => assert!(m.contains("face a abrir"), "{m}"),
        o => panic!("esperava erro, veio {o:?}"),
    }
    // sem espessura também orienta
    let mut d = cubo(100.);
    let id = converte(&mut d, "", vec![]);
    match estado(&d, id) {
        Status::Error(m) => assert!(m.contains("informe a espessura") && m.contains("face a abrir"), "{m}"),
        o => panic!("esperava erro, veio {o:?}"),
    }
    // espessura que não cabe no corpo
    let mut d = cubo(100.);
    let id = converte(&mut d, "60", vec![[50., 50., 100.]]);
    match estado(&d, id) {
        Status::Error(m) => assert!(m.contains("grande demais"), "{m}"),
        o => panic!("esperava erro, veio {o:?}"),
    }
}

fn parede(d: &mut Document, sk: Id) -> Id {
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }))
}

fn chapa_vazia() -> Document {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    d
}

fn dois_retangulos(d: &mut Document, a: ([f64; 2], [f64; 2]), b: ([f64; 2], [f64; 2])) -> Id {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    data.add_rect(&mut al, a.0, a.1, false);
    data.add_rect(&mut al, b.0, b.1, false);
    d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }))
}

/// Uma peça de chapa tem um só corpo: dois contornos externos disjuntos dão erro claro.
#[test]
fn parede_base_com_dois_contornos_disjuntos_e_recusada() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = chapa_vazia();
    let sk = dois_retangulos(&mut d, ([0., 0.], [50., 50.]), ([100., 0.], [150., 50.]));
    let w = parede(&mut d, sk);
    let r = recompute(&d);
    match &r.status[&w] {
        Status::Error(m) => assert!(m.contains("um só contorno externo") && m.contains("furos"), "{m}"),
        o => panic!("esperava erro, veio {o:?}"),
    }
    assert!(r.bodies.is_empty(), "nenhum corpo deveria nascer: {}", r.bodies.len());
}

/// Contorno interno continua sendo furo.
#[test]
fn parede_base_com_furo_interno_continua_valida() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = chapa_vazia();
    let sk = dois_retangulos(&mut d, ([0., 0.], [100., 50.]), ([20., 10.], [40., 30.]));
    let w = parede(&mut d, sk);
    let r = recompute(&d);
    assert_eq!(r.status[&w], Status::Ok, "{:?}", r.status[&w]);
    assert_eq!(r.bodies.len(), 1);
    let v = r.bodies[0].mass.volume;
    assert!((v - (100. * 50. - 20. * 20.) * 2.).abs() < 1e-3, "volume {v}");
}

/// Uma segunda parede base longe da primeira daria um segundo corpo de chapa: recusada.
#[test]
fn segunda_parede_base_disjunta_e_recusada() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = chapa_vazia();
    let a = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [50., 50.]);
    let w1 = parede(&mut d, a);
    let b = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [100., 0.], [150., 50.]);
    let w2 = parede(&mut d, b);
    let r = recompute(&d);
    assert_eq!(r.status[&w1], Status::Ok);
    match &r.status[&w2] {
        Status::Error(m) => assert!(m.contains("segunda parede base") && m.contains("encostar"), "{m}"),
        o => panic!("esperava erro, veio {o:?}"),
    }
    assert_eq!(r.bodies.len(), 1);
    assert!((r.bodies[0].mass.volume - 50. * 50. * 2.).abs() < 1e-3);
}

/// Uma segunda parede que encosta na primeira continua sendo o mesmo corpo.
#[test]
fn segunda_parede_base_encostada_continua_valida() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = chapa_vazia();
    let a = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [50., 50.]);
    parede(&mut d, a);
    let b = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [40., 0.], [90., 50.]);
    let w2 = parede(&mut d, b);
    let r = recompute(&d);
    assert_eq!(r.status[&w2], Status::Ok, "{:?}", r.status[&w2]);
    assert_eq!(r.bodies.len(), 1);
}
