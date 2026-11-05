// SPDX-License-Identifier: LGPL-2.1-or-later
//! B75: nada ignorado em silêncio nos comandos de sólido. Cada combinação que antes perdia parte do
//! pedido (saída com término não cego, furo simétrico pela metade, filete "criado por" aplicado ao
//! corpo todo, contornos descartados no loft e na varredura, revolução com eixo fora do plano ou
//! perfil cruzando o eixo) agora dá o resultado completo, um aviso que nomeia o descarte ou um erro.

mod common;

use common::*;
use fenkra_core::sketch::Curve;
use fenkra_core::*;
use fenkra_solidos::{recompute, RecomputeResult, Status};
use std::f64::consts::PI;

fn caixa(l: f64, w: f64, h: f64) -> (Document, Id) {
    let mut d = Document::new("caixa");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-l / 2., -w / 2.], [l / 2., w / 2.]);
    d.add_feature(ext(sk, BoolOp::Add, &h.to_string()));
    (d, sk)
}

fn erro(r: &RecomputeResult, id: Id) -> String {
    match &r.status[&id] {
        Status::Error(m) => m.clone(),
        outro => panic!("esperava erro, veio {outro:?}"),
    }
}

fn aviso(r: &RecomputeResult, id: Id) -> String {
    match &r.status[&id] {
        Status::Warning(m) => m.clone(),
        outro => panic!("esperava aviso, veio {outro:?}"),
    }
}

// ------------------------------------------------------------------ saída (taper) com qualquer término

#[test]
fn saida_com_termino_nao_cego_e_erro_nomeado() {
    if !kernel_disponivel() {
        return;
    }
    let termos = [Extent::Symmetric("10".into()), Extent::TwoSides("5".into(), "5".into()), Extent::ThroughAll, Extent::UpToLast, Extent::UpToNext];
    for t in termos {
        let (mut d, _) = caixa(40., 30., 10.);
        let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-5., -5.], [5., 5.]);
        let nome = format!("{t:?}");
        let id = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Cut, extent: t, reverse: false, taper: "3".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let r = recompute(&d);
        let m = erro(&r, id);
        assert!(m.contains("ângulo de saída") && m.contains("cego"), "{nome}: {m}");
    }
}

#[test]
fn saida_com_termino_cego_continua_valendo() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("saida");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-10., -10.], [10., 10.]);
    let id = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("20".into()), reverse: false, taper: "5".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok);
    // tronco de pirâmide: a face de cima encolhe 20·tan(5°) por lado
    let v = volume_total(&r);
    assert!(v < 20. * 20. * 20. && v > 0.7 * 20. * 20. * 20., "volume {v}");
}

// ------------------------------------------------------------------ furo simétrico completo

fn furo(sketch: Id, depth: Extent) -> FeatureKind {
    FeatureKind::Hole(Hole { sketch, kind: HoleKind::Simples, diameter: "6".into(), depth, thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default() })
}

#[test]
fn furo_simetrico_vai_dos_dois_lados_da_boca() {
    if !kernel_disponivel() {
        return;
    }
    // caixa de 20 de altura; a boca está no meio (z = 10) e o furo de 10 vai de z = 5 a z = 15
    for (nome, depth) in [("simétrico", Extent::Symmetric("10".into())), ("dois lados", Extent::TwoSides("3".into(), "7".into()))] {
        let (mut d, _) = caixa(40., 30., 20.);
        let meio = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
        let sp = sk_pontos(&mut d, PlaneRef::Datum(meio), &[[0., 0.]]);
        let id = d.add_feature(furo(sp, depth));
        let r = recompute(&d);
        assert_eq!(r.status[&id], Status::Ok, "{nome}: {:?}", r.status[&id]);
        let esperado = 40. * 30. * 20. - PI * 9. * 10.;
        let v = volume_total(&r);
        assert!((v - esperado).abs() < 5., "{nome}: volume {v:.2} esperado {esperado:.2} (metade do furo faltando?)");
    }
}

// ------------------------------------------------------------------ EdgeSel::CreatedBy só nas arestas do dono

#[test]
fn filete_por_comando_atua_so_nas_arestas_criadas_por_ele() {
    if !kernel_disponivel() {
        return;
    }
    // caixa 40 × 30 × 10 com um pino de raio 6 e altura 10 em cima; o filete "criado pelo pino" só pega
    // a borda circular de cima do pino (a de baixo é dele com a caixa e as da caixa não são dele)
    let (mut d, _) = caixa(40., 30., 10.);
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
    let sk = sk_circulos(&mut d, PlaneRef::Datum(topo), &[([0., 0.], 6.)]);
    let pino = d.add_feature(ext(sk, BoolOp::Add, "10"));
    let r0 = recompute(&d);
    assert_eq!(r0.status[&pino], Status::Ok);
    let v0 = volume_total(&r0);
    let fil = d.add_feature(FeatureKind::Fillet(EdgeOp { value: "1".into(), edges: EdgeSel::CreatedBy(pino), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![] }));
    let r = recompute(&d);
    assert_eq!(r.status[&fil], Status::Ok, "{:?}", r.status[&fil]);
    let v = volume_total(&r);
    // o filete tira só o material da borda circular de cima do pino
    let alvo = v0 - vol_filete_borda_circular(6., 1.);
    assert!((v - alvo).abs() < 0.05, "volume {v:.3} esperado {alvo:.3}");
    // a caixa continua com os cantos vivos: a caixa envolvente não muda
    let (a, b) = caixa_da_malha(&r);
    assert!((b[0] - a[0] - 40.).abs() < 1e-3 && (b[1] - a[1] - 30.).abs() < 1e-3, "{a:?} {b:?}");
}

#[test]
fn filete_por_comando_sem_arestas_do_dono_e_erro() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, sk) = caixa(40., 30., 10.);
    // o sketch não cria face nenhuma: nada para arredondar
    let fil = d.add_feature(FeatureKind::Fillet(EdgeOp { value: "1".into(), edges: EdgeSel::CreatedBy(sk), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![] }));
    let r = recompute(&d);
    let m = erro(&r, fil);
    assert!(m.contains("não criou nenhuma aresta"), "{m}");
}

// ------------------------------------------------------------------ loft e varredura avisam o que descartam

#[test]
fn loft_avisa_contornos_extras_da_secao() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("loft");
    let s0 = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([-10., 0.], 5.), ([15., 0.], 3.)]);
    let alto = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
    let s1 = sk_circulos(&mut d, PlaneRef::Datum(alto), &[([-10., 0.], 4.)]);
    let id = d.add_feature(FeatureKind::Loft(Loft { sections: vec![s0, s1], ruled: false, closed: false, op: BoolOp::Add, torcao: vec![], inicio: Default::default(), fim: Default::default() }));
    let r = recompute(&d);
    let m = aviso(&r, id);
    assert!(m.contains("contornos descartados") && m.contains("2 contornos"), "{m}");
}

#[test]
fn varredura_avisa_perfis_abertos_descartados() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("varredura");
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        data.add_rect(&mut al, [-3., -3.], [3., 3.], false);
        let (a, b) = (data.add_point(&mut al, 10., 10.), data.add_point(&mut al, 20., 10.));
        data.add_curve(&mut al, Curve::Line { a, b }, false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    let id = d.add_feature(FeatureKind::Sweep(Sweep { sketch: sk, spine: CurveRef::Axis(OriginAxis::Z), mode: SweepMode::Corrigido, op: BoolOp::Add, thin: None }));
    let r = recompute(&d);
    let m = aviso(&r, id);
    assert!(m.contains("aberto") && m.contains("descartados"), "{m}");
}

// ------------------------------------------------------------------ revolução valida o perfil

fn revolucao(plano: PlaneRef, a: [f64; 2], b: [f64; 2], eixo: OriginAxis) -> (RecomputeResult, Id) {
    let mut d = Document::new("revolucao");
    let sk = sk_rect(&mut d, plano, a, b);
    let id = d.add_feature(FeatureKind::Revolve(Revolve { sketch: sk, axis: AxisRef::Origin(eixo), angle: "360".into(), symmetric: false, op: BoolOp::Add, thin: None, thin_side: 0, angle2: None }));
    (recompute(&d), id)
}

#[test]
fn revolucao_com_perfil_de_um_lado_do_eixo_funciona() {
    if !kernel_disponivel() {
        return;
    }
    let (r, id) = revolucao(PlaneRef::Origin(OriginPlane::XY), [2., 0.], [6., 10.], OriginAxis::Y);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    let esperado = PI * (36. - 4.) * 10.;
    assert!((volume_total(&r) - esperado).abs() < 0.01 * esperado, "volume {}", volume_total(&r));
}

#[test]
fn revolucao_com_perfil_cruzando_o_eixo_e_erro() {
    if !kernel_disponivel() {
        return;
    }
    let (r, id) = revolucao(PlaneRef::Origin(OriginPlane::XY), [-3., 0.], [6., 10.], OriginAxis::Y);
    let m = erro(&r, id);
    assert!(m.contains("cruza o eixo"), "{m}");
}

#[test]
fn revolucao_com_eixo_fora_do_plano_do_perfil_e_erro() {
    if !kernel_disponivel() {
        return;
    }
    // perfil no plano XY e eixo Z: o eixo é normal ao plano do sketch
    let (r, id) = revolucao(PlaneRef::Origin(OriginPlane::XY), [2., 0.], [6., 10.], OriginAxis::Z);
    let m = erro(&r, id);
    assert!(m.contains("não está no plano"), "{m}");
}
