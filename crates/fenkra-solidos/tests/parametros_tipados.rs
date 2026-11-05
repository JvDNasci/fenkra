// SPDX-License-Identifier: LGPL-2.1-or-later
//! B101: unidades reais, tipos, limites e supressão por expressão chegam ao recálculo.

mod common;

use common::*;
use fenkra_core::parametros::{ParamTipo, UnidadeComp};
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

#[test]
fn polegada_na_expressao_da_o_mesmo_solido_que_os_milimetros() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("pol");
    d.add_param("H", "1 in", Unit::Comprimento).unwrap();
    let sk = sk_rect(&mut d, XY, [0., 0.], [20., 10.]);
    let ex = d.add_feature(ext(sk, BoolOp::Add, "H"));
    let r = recompute(&d);
    assert_eq!(r.status[&ex], Status::Ok, "{:?}", r.status[&ex]);
    let em_pol = volume_total(&r);
    assert!((em_pol - 20. * 10. * 25.4).abs() < 1e-3, "volume {em_pol}");
    // o mesmo valor em mm, e trocando só a unidade de exibição: a geometria não se move
    definir_param(&mut d, "H", "25.4");
    assert!((volume_total(&recompute(&d)) - em_pol).abs() < 1e-6);
    d.params[0].exibir = UnidadeComp::Pol;
    assert!((volume_total(&recompute(&d)) - em_pol).abs() < 1e-6);
}

#[test]
fn comando_some_do_solido_enquanto_a_expressao_de_supressao_vale() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("suprime por expressão");
    d.add_param("ComFuro", "true", Unit::Numero).unwrap();
    d.params[0].tipo = ParamTipo::Booleano;
    let sk = sk_rect(&mut d, XY, [0., 0.], [40., 40.]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    // o corte parte do topo (plano deslocado 10) e desce pelo corpo inteiro
    let topo = d.add_offset_plane(XY, "10");
    let furo_sk = sk_circulos(&mut d, PlaneRef::Datum(topo), &[([20., 20.], 5.)]);
    let furo = d.add_feature(FeatureKind::Extrude(Extrude {
        sketch: furo_sk,
        op: BoolOp::Cut,
        extent: Extent::Blind("10".into()),
        reverse: false,
        taper: "0".into(),
        thin: None,
        dir: None,
        thin_side: 0,
        ends: Default::default(),
    }));
    d.feature_mut(furo).unwrap().suppress_if = "1 - ComFuro".into();

    let cheio = 40. * 40. * 10.;
    let com_furo = cheio - std::f64::consts::PI * 25. * 10.;
    let r = recompute(&d);
    assert_eq!(r.status[&furo], Status::Ok, "{:?}", r.status[&furo]);
    assert!((volume_total(&r) - com_furo).abs() < 0.5, "com furo: {}", volume_total(&r));

    // desliga o booleano: o furo é suprimido, sem tocar na marca manual
    definir_param(&mut d, "ComFuro", "false");
    let r = recompute(&d);
    assert_eq!(r.status[&furo], Status::Inactive);
    assert!((volume_total(&r) - cheio).abs() < 1e-3, "sem furo: {}", volume_total(&r));
    assert!(!d.feature(furo).unwrap().suppressed);

    // ligar de novo traz o furo de volta, igual ao cálculo do zero
    definir_param(&mut d, "ComFuro", "true");
    assert!((volume_total(&recompute(&d)) - com_furo).abs() < 0.5);
}

#[test]
fn limite_violado_avisa_mas_o_solido_recalcula() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("limite");
    let p = d.add_param("E", "3", Unit::Comprimento).unwrap();
    d.params[0].min = "5".into();
    let sk = sk_rect(&mut d, XY, [0., 0.], [10., 10.]);
    let ex = d.add_feature(ext(sk, BoolOp::Add, "E"));
    let r = recompute(&d);
    assert_eq!(r.status[&ex], Status::Ok);
    assert!((volume_total(&r) - 300.).abs() < 1e-3);
    assert_eq!(d.avisos_do_param(p).len(), 1);
}
