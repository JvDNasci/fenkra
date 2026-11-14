// SPDX-License-Identifier: LGPL-2.1-or-later
//! B437: origem e eixos H/V do sketch como alvo de restrição, e sketch posicionado por referência.
//!
//! Bloco 40 × 30 × 20 com o canto em (0, 0, 0). Um plano na face de cima (z = 20), com a origem no
//! canto (40, 30, 20), recebe um sketch de retângulo 20 (H) × 10 (V) preso só à origem e aos eixos do
//! sketch; o retângulo é extrudado 10 para cima. Cada modo de origem e de orientação põe o retângulo
//! num lugar conhecido: o volume é sempre 24000 + 2000 e a caixa envolvente diz onde ele ficou.

mod common;

use common::*;
use fenkra_core::sketch::{Curve, Kind, LinkSrc, SketchPos};
use fenkra_core::sketch_origem::{OrigemSk, OrientaSk};
use fenkra_core::*;
use fenkra_solidos::{recompute, RecomputeResult};

const BLOCO: f64 = 40. * 30. * 20.;

/// Bloco e o plano da face de cima, com a origem no canto (40, 30, 20).
fn bloco() -> (Document, Id) {
    let mut d = Document::new("b437");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [40., 30.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let topo = d.add_geometry(RefKind::FacePlane { origin: [40., 30., 20.], x: [1., 0., 0.], normal: [0., 0., 1.], owner: None, key: None });
    (d, topo)
}

/// Retângulo 20 × 10 preso à origem e aos eixos: horizontal, vertical, paralela aos eixos, o centro
/// da diagonal coincidente com a origem e as distâncias aos eixos.
fn retangulo_nos_eixos(d: &mut Document, pos: SketchPos) -> SketchData {
    let mut s = SketchData::default();
    let mut al = || d.alloc();
    let e = s.garantir_eixos(&mut al);
    let p = [[-9., -4.], [11., -6.], [9., 6.], [-11., 4.]].map(|q| s.add_point(&mut al, q[0], q[1]));
    let l = [0, 1, 2, 3].map(|i| s.add_curve(&mut al, Curve::Line { a: p[i], b: p[(i + 1) % 4] }, false));
    let diag = s.add_curve(&mut al, Curve::Line { a: p[0], b: p[2] }, true);
    s.add_constraint(&mut al, Kind::Horizontal(l[0]));
    s.add_constraint(&mut al, Kind::Parallel(l[2], e.h));
    s.add_constraint(&mut al, Kind::Vertical(l[1]));
    s.add_constraint(&mut al, Kind::Parallel(l[3], e.v));
    s.add_constraint(&mut al, Kind::Midpoint(e.origem, diag));
    s.add_constraint(&mut al, Kind::PointLineDistance(p[0], e.h, "5".into()));
    s.add_constraint(&mut al, Kind::PointLineDistance(p[1], e.v, "10".into()));
    s.pos = pos;
    s
}

fn monta(pos: SketchPos) -> (Document, Id) {
    let (mut d, topo) = bloco();
    let data = retangulo_nos_eixos(&mut d, pos);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data }));
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    (d, sk)
}

/// Recalcula, confere status, volume e caixa; devolve o resultado.
fn confere_caixa(nome: &str, d: &Document, lo: [f64; 3], hi: [f64; 3]) -> RecomputeResult {
    let tam = [0, 1, 2].map(|i| hi[i] - lo[i]);
    confere(nome, d, &Esperado { vol: BLOCO + 2000., tol_vol: 1e-6, tam, tol_tam: 1e-3, minimo: Some(lo), via_malha: true });
    recompute(d)
}

fn frame_de(r: &RecomputeResult, sk: Id) -> fenkra_kernel::Frame {
    r.sketches.iter().find(|s| s.feature == sk).expect("sketch avaliado").frame
}

fn perto(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-6)
}

/// Caixa do bloco somada à do retângulo de centro `c` e meias medidas `m` (em x e y).
fn caixa(c: [f64; 2], m: [f64; 2]) -> ([f64; 3], [f64; 3]) {
    ([(c[0] - m[0]).min(0.), (c[1] - m[1]).min(0.), 0.], [(c[0] + m[0]).max(40.), (c[1] + m[1]).max(30.), 30.])
}

#[test]
fn sketch_preso_aos_eixos_fica_totalmente_definido() {
    if !kernel_disponivel() {
        return;
    }
    let (d, sk) = monta(SketchPos::default());
    let r = recompute(&d);
    let v = r.sketches.iter().find(|s| s.feature == sk).unwrap();
    assert!(v.converged && v.dof == 0, "dof {} convergiu {}", v.dof, v.converged);
    // automática: a origem do plano, no canto do bloco
    let (lo, hi) = caixa([40., 30.], [10., 5.]);
    confere_caixa("automática", &d, lo, hi);
}

#[test]
fn cada_modo_de_origem_poe_o_sketch_no_lugar_certo() {
    if !kernel_disponivel() {
        return;
    }
    let casos: Vec<(&str, OrigemSk, [f64; 2])> = vec![
        ("origem da peça", OrigemSk::OrigemPeca, [0., 0.]),
        ("ponto projetado", OrigemSk::Ponto(LinkSrc::Vertex { pos: [40., 0., 0.] }), [40., 0.]),
        // aresta de cima em y = 30 e aresta de cima em x = 0
        ("interseção", OrigemSk::Intersecao(LinkSrc::Edge { mid: [20., 30., 20.] }, LinkSrc::Edge { mid: [0., 15., 20.] }), [0., 30.]),
        ("ponto médio", OrigemSk::PontoMedio(LinkSrc::Edge { mid: [20., 0., 20.] }), [20., 0.]),
        // face da frente (y = 0): o centro (20, 0, 10) projetado na face de cima
        ("centro da face", OrigemSk::CentroFace(LinkSrc::Face { point: [20., 0., 10.] }), [20., 0.]),
    ];
    for (nome, modo, c) in casos {
        let (d, sk) = monta(SketchPos { modo, ..Default::default() });
        let (lo, hi) = caixa(c, [10., 5.]);
        let r = confere_caixa(nome, &d, lo, hi);
        let f = frame_de(&r, sk);
        assert!(perto(f.origin, [c[0], c[1], 20.]), "{nome}: origem {:?}", f.origin);
        assert!(r.status.values().all(|s| *s == fenkra_solidos::Status::Ok), "{nome}: {:?}", r.status);
    }
}

#[test]
fn deslocamento_soma_a_origem_escolhida() {
    if !kernel_disponivel() {
        return;
    }
    let (d, _) = monta(SketchPos { modo: OrigemSk::OrigemPeca, origem: [12., 7.], ..Default::default() });
    let (lo, hi) = caixa([12., 7.], [10., 5.]);
    confere_caixa("origem da peça com deslocamento", &d, lo, hi);
}

#[test]
fn cada_orientacao_gira_o_sketch() {
    if !kernel_disponivel() {
        return;
    }
    // origem da peça: H ao longo de Y deixa o lado de 20 em y
    let casos: Vec<(&str, OrientaSk, [f64; 3])> = vec![
        ("eixo Y", OrientaSk::Eixo(OriginAxis::Y), [0., 1., 0.]),
        ("eixo Z (perpendicular, fica a automática)", OrientaSk::Eixo(OriginAxis::Z), [1., 0., 0.]),
        ("linha", OrientaSk::Linha(LinkSrc::Edge { mid: [40., 15., 20.] }), [0., 1., 0.]),
        ("ponto", OrientaSk::Ponto(LinkSrc::Vertex { pos: [0., 30., 20.] }), [0., 1., 0.]),
        // o plano YZ corta a face de cima numa reta em Y
        ("plano", OrientaSk::Plano(PlaneRef::Origin(OriginPlane::YZ)), [0., 1., 0.]),
    ];
    for (nome, orienta, h) in casos {
        let (d, sk) = monta(SketchPos { modo: OrigemSk::OrigemPeca, orienta, ..Default::default() });
        let r = recompute(&d);
        let f = frame_de(&r, sk);
        assert!(perto(f.x, h) || perto(f.x, h.map(|x| -x)), "{nome}: H {:?}", f.x);
        // a normal continua para cima: a extrusão sobe
        assert!(perto(f.normal, [0., 0., 1.]), "{nome}: normal {:?}", f.normal);
        let m = if h[1].abs() > 0.5 { [5., 10.] } else { [10., 5.] };
        let (lo, hi) = caixa([0., 0.], m);
        confere_caixa(nome, &d, lo, hi);
        // a direção impossível avisa e não quebra a peça
        let aviso = matches!(r.status.get(&sk), Some(fenkra_solidos::Status::Warning(_)));
        assert_eq!(aviso, matches!(orienta, OrientaSk::Eixo(OriginAxis::Z)), "{nome}: {:?}", r.status.get(&sk));
    }
}

#[test]
fn inverter_e_trocar_valem_sobre_a_orientacao_escolhida() {
    if !kernel_disponivel() {
        return;
    }
    // H em Y, invertido: H aponta para -Y; V invertido junto mantém a normal
    let (d, sk) = monta(SketchPos { modo: OrigemSk::OrigemPeca, orienta: OrientaSk::Eixo(OriginAxis::Y), inverter_h: true, inverter_v: true, ..Default::default() });
    let r = recompute(&d);
    let f = frame_de(&r, sk);
    assert!(perto(f.x, [0., -1., 0.]) && perto(f.normal, [0., 0., 1.]), "{:?} {:?}", f.x, f.normal);
    // trocar H e V: H vira o V de antes (cross(Z, Y) = -X) e a normal desce
    let (d, sk) = monta(SketchPos { modo: OrigemSk::OrigemPeca, orienta: OrientaSk::Eixo(OriginAxis::Y), trocar: true, ..Default::default() });
    let f = frame_de(&recompute(&d), sk);
    assert!(perto(f.x, [-1., 0., 0.]) && perto(f.normal, [0., 0., -1.]), "{:?} {:?}", f.x, f.normal);
}

#[test]
fn referencia_perdida_volta_para_a_automatica_e_avisa() {
    if !kernel_disponivel() {
        return;
    }
    let (d, sk) = monta(SketchPos { modo: OrigemSk::Ponto(LinkSrc::Vertex { pos: [500., 500., 500.] }), ..Default::default() });
    let r = recompute(&d);
    assert!(matches!(r.status.get(&sk), Some(fenkra_solidos::Status::Warning(w)) if w.contains("origem do sketch")), "{:?}", r.status.get(&sk));
    let (lo, hi) = caixa([40., 30.], [10., 5.]);
    confere_caixa("referência perdida", &d, lo, hi);
}

/// Arquivo de antes do B437: sem eixos, sem modo, com a origem deslocada. Abre como origem automática
/// com deslocamento e a geometria fica idêntica, antes e depois de ganhar os eixos no editor.
#[test]
fn arquivo_antigo_abre_com_a_mesma_geometria() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, topo) = bloco();
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        data.add_rect(&mut al, [0., 0.], [20., 10.], false);
    }
    data.pos = SketchPos { origem: [-13., -9.], inverter_v: false, ..Default::default() };
    // o JSON como a versão antiga gravava
    let mut v = serde_json::to_value(&data).unwrap();
    v.as_object_mut().unwrap().remove("eixos");
    let pos = v["pos"].as_object_mut().unwrap();
    pos.remove("modo");
    pos.remove("orienta");
    assert_eq!(pos.len(), 4, "a posição antiga só tinha origem, trocar e inverter: {pos:?}");
    let antigo: SketchData = serde_json::from_value(v).unwrap();
    assert!(antigo.pos.e_automatica() && antigo.eixos.is_none());
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: antigo }));
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    // canto do plano (40, 30) + deslocamento (-13, -9): o retângulo vai de (27, 21) a (47, 31)
    let (lo, hi) = ([0., 0., 0.], [47., 31., 30.]);
    let r0 = confere_caixa("arquivo antigo", &d, lo, hi);
    // o editor dá os eixos ao abrir: nada muda na peça
    let mut d2 = d.clone();
    let mut s2 = match d2.feature(sk).map(|f| &f.kind) {
        Some(FeatureKind::Sketch(s)) => s.data.clone(),
        _ => unreachable!(),
    };
    {
        let mut al = || d2.alloc();
        s2.garantir_eixos(&mut al);
    }
    if let Some(FeatureKind::Sketch(s)) = d2.feature_mut(sk).map(|f| &mut f.kind) {
        s.data = s2;
    }
    let r1 = confere_caixa("arquivo antigo com eixos", &d2, lo, hi);
    assert_eq!(r0.bodies[0].mesh.positions, r1.bodies[0].mesh.positions, "a malha é a mesma");
    assert!(perto(frame_de(&r0, sk).origin, [27., 21., 20.]));
}

#[test]
fn circulo_coincidente_com_a_origem_no_centro_da_face() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, topo) = bloco();
    let mut s = SketchData::default();
    {
        let mut al = || d.alloc();
        let e = s.garantir_eixos(&mut al);
        let c = s.add_point(&mut al, 1., -1.);
        let circ = s.add_curve(&mut al, Curve::Circle { c, r: 4. }, false);
        s.add_constraint(&mut al, Kind::Coincident(c, e.origem));
        s.add_constraint(&mut al, Kind::Radius(circ, "5".into()));
    }
    // centro da face de cima
    s.pos = SketchPos { modo: OrigemSk::CentroFace(LinkSrc::Face { point: [10., 10., 20.] }), ..Default::default() };
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: s }));
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    let r = recompute(&d);
    assert!(perto(frame_de(&r, sk).origin, [20., 15., 20.]), "{:?}", frame_de(&r, sk).origin);
    let cil = std::f64::consts::PI * 25. * 10.;
    confere("círculo no centro da face", &d, &Esperado { vol: BLOCO + cil, tol_vol: 2e-3, tam: [40., 30., 30.], tol_tam: 1e-3, minimo: Some([0., 0., 0.]), via_malha: true });
    let v = r.sketches.iter().find(|x| x.feature == sk).unwrap();
    assert_eq!(v.dof, 0);
}
