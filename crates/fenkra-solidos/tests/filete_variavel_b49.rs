// SPDX-License-Identifier: LGPL-2.1-or-later
//! B49: filete variável com pontos de controle sobre a aresta (posição e raio próprios) e transição
//! suave. A caixa 40 x 30 x 20 perde, ao longo da aresta de cima da frente, (1 - pi/4) * r(x)^2 por
//! milímetro: o volume tirado confere o raio em cada trecho.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};
use std::f64::consts::PI;

const CAIXA: f64 = 40. * 30. * 20.;

fn caixa() -> Document {
    let mut d = Document::new("caixa");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    d
}

fn var(d: &mut Document, r0: &str, r1: &str, controles: &[(&str, &str)], suave: bool) -> Id {
    let extra = EdgeExtra { controls: controles.iter().map(|(a, r)| ControlePonto { at: (*a).into(), radius: (*r).into() }).collect(), smooth: suave, ..Default::default() };
    d.add_feature(FeatureKind::Fillet(EdgeOp {
        value: r0.into(),
        edges: EdgeSel::Edges(vec![[0., -15., 20.]]),
        mode: EdgeMode::Variable(r1.into()),
        prop: EdgeProp::Tangente,
        keys: vec![],
        extra,
    }))
}

/// Volume tirado pelo filete, em mm3.
fn tirado(d: &Document) -> f64 {
    CAIXA - volume_total(&recompute(d))
}

/// Tirado por um raio que varia em retas entre pontos (posição 0 a 1, raio): integral exata de r^2.
fn esperado_linear(pts: &[(f64, f64)]) -> f64 {
    let mut media = 0.;
    for w in pts.windows(2) {
        let ((u0, a), (u1, b)) = (w[0], w[1]);
        media += (u1 - u0) * (a * a + a * b + b * b) / 3.;
    }
    (1. - PI / 4.) * media * 40.
}

fn perto(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs()
}

#[test]
fn ponto_de_controle_no_meio_da_aresta_muda_o_volume_como_manda_a_lei() {
    if !kernel_disponivel() {
        return;
    }
    // extremos 2 e 6, ponto de controle com raio 5 a 25% ou a 75% da aresta (retas entre eles)
    for (pos, pts) in [("25", vec![(0., 2.), (0.25, 5.), (1., 6.)]), ("75", vec![(0., 2.), (0.75, 5.), (1., 6.)])] {
        let mut d = caixa();
        let f = var(&mut d, "2", "6", &[(pos, "5")], false);
        let r = recompute(&d);
        assert!(matches!(r.status[&f], Status::Ok), "{pos}%: {:?}", r.status[&f]);
        let v = tirado(&d);
        let e = esperado_linear(&pts);
        assert!(perto(v, e, 0.04), "controle a {pos}%: tirou {v:.1} e a lei linear manda {e:.1}");
        // o filete só arredonda: a caixa envolvente é a da caixa
        // (a caixa exata do kernel conta os polos da superfície do filete; a da malha segue a peça)
        let (lo, hi) = caixa_da_malha(&r);
        assert!((hi[0] - lo[0] - 40.).abs() < 1e-3 && (hi[1] - lo[1] - 30.).abs() < 1e-3 && (hi[2] - lo[2] - 20.).abs() < 1e-3, "{lo:?} {hi:?}");
    }
}

#[test]
fn sem_ponto_de_controle_continua_a_lei_linear_de_dois_raios() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = caixa();
    let f = var(&mut d, "2", "6", &[], false);
    assert!(matches!(recompute(&d).status[&f], Status::Ok));
    let e = esperado_linear(&[(0., 2.), (1., 6.)]);
    assert!(perto(tirado(&d), e, 0.04), "{} contra {e}", tirado(&d));
}

#[test]
fn transicao_suave_passa_pelos_pontos_e_difere_das_retas() {
    if !kernel_disponivel() {
        return;
    }
    let mut ret = caixa();
    let f1 = var(&mut ret, "2", "2", &[("50", "5")], false);
    let mut suave = caixa();
    let f2 = var(&mut suave, "2", "2", &[("50", "5")], true);
    for (d, f) in [(&ret, f1), (&suave, f2)] {
        let r = recompute(d);
        assert!(matches!(r.status[&f], Status::Ok), "{:?}", r.status[&f]);
    }
    let (vr, vs) = (tirado(&ret), tirado(&suave));
    assert!(perto(vr, esperado_linear(&[(0., 2.), (0.5, 5.), (1., 2.)]), 0.04), "retas: {vr:.1}");
    // a curva suave sobe mais depressa até o raio 5 do meio: tira mais que as retas, mas bem menos que o raio 5 todo
    let const5 = (1. - PI / 4.) * 25. * 40.;
    let const2 = (1. - PI / 4.) * 4. * 40.;
    assert!(vs > const2 && vs < const5, "suave {vs:.1} entre {const2:.1} e {const5:.1}");
    assert!((vs - vr).abs() > 0.02 * vr, "suave {vs:.1} e retas {vr:.1} não deveriam coincidir");
}

#[test]
fn ponto_de_controle_invalido_falha_com_mensagem() {
    if !kernel_disponivel() {
        return;
    }
    for (pos, raio, trecho) in [("0", "5", "entre 0 e 100"), ("100", "5", "entre 0 e 100"), ("50", "0", "maior que zero")] {
        let mut d = caixa();
        let f = var(&mut d, "2", "3", &[(pos, raio)], false);
        match &recompute(&d).status[&f] {
            Status::Error(m) => assert!(m.contains(trecho), "{pos}/{raio}: {m}"),
            o => panic!("{pos}/{raio}: esperava erro, veio {o:?}"),
        }
    }
    // dois pontos na mesma posição
    let mut d = caixa();
    let f = var(&mut d, "2", "3", &[("50", "4"), ("50", "5")], false);
    assert!(matches!(recompute(&d).status[&f], Status::Error(_)));
}
