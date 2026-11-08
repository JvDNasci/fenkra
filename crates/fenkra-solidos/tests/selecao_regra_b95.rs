// SPDX-License-Identifier: LGPL-2.1-or-later
//! B95: a regra de seleção gravada no filete (tangentes, cadeia, concêntricas) é refeita a cada
//! recompute. O filete guarda só a aresta semente; as arestas que a regra alcança no modelo atual
//! entram sozinhas, mesmo depois de uma mudança que move ou cria arestas.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::recompute;

/// Caixa 40 × 30 × 20 com centro na origem e as quatro arestas verticais arredondadas com raio `R`
/// (parâmetro): o contorno de cima fica com quatro retas e quatro arcos tangentes.
fn caixa(r: &str) -> Document {
    let mut d = Document::new("caixa");
    d.add_param("R", r, Unit::Comprimento).unwrap();
    let s = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    d.add_feature(ext(s, BoolOp::Add, "20"));
    d.add_feature(filete("R", vec![[20., 15., 10.], [-20., 15., 10.], [-20., -15., 10.], [20., -15., 10.]], vec![None; 4]));
    d
}

const SEMENTE: [f64; 3] = [0., -15., 20.];

fn com_regra(d: &mut Document, regra: RegraSelecao) {
    let mut k = filete("1", vec![SEMENTE], vec![None]);
    if let FeatureKind::Fillet(o) = &mut k {
        o.extra.regras = vec![RegraAresta { semente: 0, regra }];
    }
    d.add_feature(k);
}

/// Filete de 1 na semente com a propagação tangente do próprio comando: a referência do que a regra
/// "tangentes" tem de dar.
fn com_propagacao(d: &mut Document) {
    let mut k = filete("1", vec![SEMENTE], vec![None]);
    if let FeatureKind::Fillet(o) = &mut k {
        o.prop = EdgeProp::Tangente;
    }
    d.add_feature(k);
}

fn volume_ok(d: &Document) -> f64 {
    let r = recompute(d);
    assert_eq!(problema_de_status(d, &r), None);
    volume_total(&r)
}

#[test]
fn regra_tangentes_acha_as_oito_arestas_do_contorno() {
    if !kernel_disponivel() {
        return;
    }
    let d = caixa("3");
    let corpo = fenkra_solidos::final_shape(&d).unwrap();
    let regra = [RegraAresta { semente: 0, regra: RegraSelecao::Tangentes }];
    let todas = fenkra_solidos::selecao_regras::expandir_arestas(&corpo, &[SEMENTE], &regra).unwrap();
    assert_eq!(todas.len(), 8, "{todas:?}");
    assert!(todas.iter().all(|p| (p[2] - 20.).abs() < 1e-3), "só o contorno de cima: {todas:?}");
    // sem regra, só a semente
    assert_eq!(fenkra_solidos::selecao_regras::expandir_arestas(&corpo, &[SEMENTE], &[]).unwrap().len(), 1);
}

#[test]
fn regra_concentricas_gravada_pega_os_quatro_circulos_do_anel() {
    if !kernel_disponivel() {
        return;
    }
    // anel Ø40 com furo Ø16, altura 10: quatro círculos coaxiais e nenhuma outra aresta de filete
    let anel = || {
        let mut d = Document::new("anel");
        let s = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([0., 0.], 20.), ([0., 0.], 8.)]);
        d.add_feature(ext(s, BoolOp::Add, "10"));
        d
    };
    let semente = [20., 0., 10.];
    let mut a = anel();
    let mut k = filete("1", vec![semente], vec![None]);
    if let FeatureKind::Fillet(o) = &mut k {
        o.extra.regras = vec![RegraAresta { semente: 0, regra: RegraSelecao::Concentricas }];
    }
    a.add_feature(k);
    let mut todas = anel();
    todas.add_feature(FeatureKind::Fillet(EdgeOp { value: "1".into(), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
    let mut so = anel();
    so.add_feature(filete("1", vec![semente], vec![None]));
    let (va, vt, vs) = (volume_ok(&a), volume_ok(&todas), volume_ok(&so));
    assert!((va - vt).abs() < 1e-6 * vt, "regra {va} × todas {vt}");
    assert!(va < vs - 1., "a regra arredonda os quatro círculos, não só a semente: {va} × {vs}");
    let (lo, hi) = caixa_envolvente(&a);
    // a caixa do kernel em peça redonda é folgada (envolve os polos das curvas): só a altura é exata
    assert!(lo[0] <= -20. + 1e-6 && lo[0] > -23. && (hi[2] - 10.).abs() < 1e-5 && lo[2].abs() < 1e-5, "{lo:?} {hi:?}");
}

#[test]
fn regra_refeita_no_recompute_acompanha_o_parametro() {
    if !kernel_disponivel() {
        return;
    }
    // com R = 3 a regra acha as oito arestas; ao mudar R para 5 os arcos mudam de lugar e a regra,
    // refeita, acha as novas, igual à propagação tangente
    let mut a = caixa("3");
    com_regra(&mut a, RegraSelecao::Tangentes);
    volume_ok(&a);
    a.params.iter_mut().find(|p| p.name == "R").unwrap().expr = "5".into();
    let mut b = caixa("5");
    com_propagacao(&mut b);
    let (va, vb) = (volume_ok(&a), volume_ok(&b));
    assert!((va - vb).abs() < 1e-6 * vb, "{va} × {vb}");
    // a regra vai no arquivo e volta igual
    let volta = Document::from_json(&a.to_json()).unwrap();
    assert!((volume_ok(&volta) - va).abs() < 1e-6 * va);
    let FeatureKind::Fillet(o) = &volta.features.last().unwrap().kind else { panic!() };
    assert_eq!(o.extra.regras, vec![RegraAresta { semente: 0, regra: RegraSelecao::Tangentes }]);
}
