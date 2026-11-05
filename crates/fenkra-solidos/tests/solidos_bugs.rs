// SPDX-License-Identifier: LGPL-2.1-or-later
//! B05, B06, B07 e B11: início da rosca, casca fechada, espessura com botão de remover e reforço
//! com lado da parede.

mod common;

use common::*;
use fenkra_core::sketch::Curve;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

fn caixa() -> Document {
    let mut d = Document::new("caixa");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    d
}

fn volume(d: &Document) -> f64 {
    volume_total(&recompute(d))
}

#[test]
fn espessura_com_botao_remover_tira_material() {
    if !kernel_disponivel() {
        return;
    }
    let v0 = volume(&caixa());
    let t = Thickness { faces: vec![[0., 0., 20.]], value: "5".into(), face_keys: vec![], remove: true };
    let mut d = caixa();
    let f = d.add_feature(FeatureKind::Thickness(t.clone()));
    let r = recompute(&d);
    assert!(problema_de_status(&d, &r).is_none(), "{:?}", r.status[&f]);
    let v = volume_total(&r);
    assert!((v - (v0 - 40. * 30. * 5.)).abs() < 1., "remover {v} vs {v0}");
    // sem o botão o mesmo valor acrescenta
    let mut d = caixa();
    d.add_feature(FeatureKind::Thickness(Thickness { remove: false, ..t }));
    assert!((volume(&d) - (v0 + 40. * 30. * 5.)).abs() < 1.);
}

#[test]
fn casca_fechada_cria_cavidade_sem_abrir_face() {
    if !kernel_disponivel() {
        return;
    }
    let v0 = volume(&caixa());
    let mut d = caixa();
    let sh = d.add_feature(FeatureKind::Shell(Shell { faces: vec![], thickness: "2".into(), outward: false, face_keys: vec![], closed: true }));
    let r = recompute(&d);
    assert!(matches!(r.status[&sh], Status::Ok), "{:?}", r.status[&sh]);
    let esperado = v0 - 36. * 26. * 16.;
    let v = volume_total(&r);
    assert!((v - esperado).abs() < 2., "casca fechada {v} vs {esperado}");
    // a parede é por dentro: a caixa envolvente não muda
    let (lo, hi) = caixa_envolvente(&d);
    assert!((hi[2] - lo[2] - 20.).abs() < 1e-3 && (hi[0] - lo[0] - 40.).abs() < 1e-3);
}

#[test]
fn reforco_um_lado_desloca_a_parede() {
    if !kernel_disponivel() {
        return;
    }
    let mut caixas = vec![];
    for side in [0u8, 1, 2] {
        let mut d = Document::new("r");
        let mut data = SketchData::default();
        let mut al = || d.alloc();
        let a = data.add_point(&mut al, -10., 0.);
        let b = data.add_point(&mut al, 10., 0.);
        data.add_curve(&mut al, Curve::Line { a, b }, false);
        let s = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::Stiffener(Stiffener { sketch: s, thickness: "2".into(), depth: "10".into(), reverse: true, to_body: false, side }));
        caixas.push(caixa_envolvente(&d));
    }
    // dois lados: simétrico em torno de y = 0
    assert!(caixas[0].0[1] < -0.9 && caixas[0].1[1] > 0.9, "{:?}", caixas[0]);
    // um lado: a espessura inteira de um só lado, e cada valor de `side` vai para um lado
    for c in &caixas[1..] {
        let (lo, hi) = (c.0[1], c.1[1]);
        assert!((lo.abs() < 0.1 && hi > 1.9) || (hi.abs() < 0.1 && lo < -1.9), "{c:?}");
    }
    assert!((caixas[1].0[1] - caixas[2].0[1]).abs() > 1., "{caixas:?}");
}

#[test]
fn rosca_com_inicio_deslocado_e_fora_da_face() {
    if !kernel_disponivel() {
        return;
    }
    let mk = |start: &str| {
        let mut d = Document::new("eixo");
        let sk = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([0., 0.], 5.)]);
        d.add_feature(ext(sk, BoolOp::Add, "30"));
        let th = d.add_feature(FeatureKind::Thread(Thread { face: [5., 0., 25.], spec: "M10x1.5".into(), length: "8".into(), modeled: true, end: None, start: start.into() }));
        (d, th)
    };
    let (sem, _) = {
        let mut d = Document::new("eixo");
        let sk = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([0., 0.], 5.)]);
        d.add_feature(ext(sk, BoolOp::Add, "30"));
        (d, ())
    };
    let v_sem = volume(&sem);
    let (v0, v1) = (volume(&mk("").0), volume(&mk("10").0));
    // a rosca tira material nas duas posições; o início só desloca o sulco no eixo (na borda a rosca sai pela face e tira um pouco mais)
    assert!(v0 < v_sem - 1. && v1 < v_sem - 1., "{v_sem} {v0} {v1}");
    assert!((v0 - v1).abs() < 0.2 * (v_sem - v0), "{v0} {v1}");
    // perto da ponta o comprimento é limitado pelo que sobra da face: tira menos que a rosca inteira
    let v2 = volume(&mk("25").0);
    assert!(v2 > v1 + 1. && v2 < v_sem - 1., "{v1} {v2}");
    let (d2, th) = mk("40");
    let r = recompute(&d2);
    assert!(matches!(&r.status[&th], Status::Error(m) if m.contains("início")), "{:?}", r.status[&th]);
}
