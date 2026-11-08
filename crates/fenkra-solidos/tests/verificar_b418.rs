// SPDX-License-Identifier: LGPL-2.1-or-later
//! B418: requisitos declarados na peça, conferidos um a um. Cada caso monta a peça em código, confere
//! status, volume e caixa, e depois o que `verificar` diz de cada requisito.

mod common;

use common::*;
use fenkra_core::requisitos::{Medida, Requisito, TipoRequisito};
use fenkra_core::*;
use fenkra_solidos::verificar::verificar;

fn req(nome: &str, tipo: TipoRequisito) -> Requisito {
    Requisito { nome: nome.into(), tipo }
}

/// Placa 100 × 60 × h (h é o parâmetro "esp") com quatro furos Ø6,6 passantes.
fn placa_com_furos() -> Document {
    let mut d = Document::new("placa");
    d.props.material = "Aço 1020".into();
    d.add_param("esp", "10", Unit::Comprimento).unwrap();
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 60.]);
    d.add_feature(ext(sk, BoolOp::Add, "esp"));
    let p = sk_pontos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[[10., 10.], [90., 10.], [10., 50.], [90., 50.]]);
    // o furo parte do plano do sketch contra a normal: invertido, atravessa a placa
    let mut furo = furo_passante(p, "6.6");
    if let FeatureKind::Hole(h) = &mut furo {
        h.reverse = true;
    }
    d.add_feature(furo);
    d
}

#[test]
fn placa_cumpre_massa_furos_caixa_e_propagacao_e_reprova_o_que_nao_cumpre() {
    if !kernel_disponivel() {
        return;
    }
    let d = placa_com_furos();
    let r = fenkra_solidos::recompute(&d);
    assert!(problema_de_status(&d, &r).is_none(), "{:?}", problema_de_status(&d, &r));
    let furo = std::f64::consts::PI * 3.3 * 3.3 * 10.;
    let v = 100. * 60. * 10. - 4. * furo;
    assert!((volume_total(&r) - v).abs() < 1e-3, "volume {}", volume_total(&r));
    let (lo, hi) = caixa_envolvente(&d);
    assert!((hi[0] - lo[0] - 100.).abs() < 1e-6 && (hi[2] - lo[2] - 10.).abs() < 1e-6);
    let kg = v * materiais::densidade("Aço 1020") * 1e-6;

    let lista = vec![
        req("massa", TipoRequisito::Massa { min: Some(kg - 0.01), max: Some(kg + 0.01) }),
        req("leve demais", TipoRequisito::Massa { min: None, max: Some(0.1) }),
        req("4 furos M6", TipoRequisito::Furos { diametro: 6.6, quantidade: 4, tol: 0.05 }),
        req("6 furos M6", TipoRequisito::Furos { diametro: 6.6, quantidade: 6, tol: 0.05 }),
        req("cabe", TipoRequisito::Caixa { max: [100., 60., 12.] }),
        req("não cabe", TipoRequisito::Caixa { max: [90., 60., 12.] }),
        req("espessura segue", TipoRequisito::Propagacao { parametro: "esp".into(), valor: "15".into(), medida: Medida::CaixaZ, esperado: Some(15.), tol: 1e-3 }),
        req("parâmetro solto", TipoRequisito::Propagacao { parametro: "nao_existe".into(), valor: "1".into(), medida: Medida::Volume, esperado: None, tol: 1e-3 }),
        req("um corpo", TipoRequisito::SemInterferencia { tol: 1e-3 }),
    ];
    let res = verificar(&d, &lista);
    let ok: Vec<(&str, bool)> = res.iter().map(|r| (r.nome.as_str(), r.ok)).collect();
    assert_eq!(
        ok,
        vec![("massa", true), ("leve demais", false), ("4 furos M6", true), ("6 furos M6", false), ("cabe", true), ("não cabe", false), ("espessura segue", true), ("parâmetro solto", false), ("um corpo", true)],
        "{res:#?}"
    );
    assert_eq!(res[6].medido, "10 → 15", "{:?}", res[6]);
    assert!(res[5].detalhe.contains('X'), "{:?}", res[5]);
    // a peça aberta não mudou com a propagação
    assert_eq!(d.params[0].expr, "10");
}

/// Dois corpos: blocos de 10 mm com 2 mm entre eles, e depois cruzados.
#[test]
fn folga_e_interferencia_entre_corpos() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("dois");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [10., 10.]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    let b = d.add_collector("B");
    d.active_collector = b;
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [12., 0.], [22., 10.]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    let r = fenkra_solidos::recompute(&d);
    assert!(problema_de_status(&d, &r).is_none());
    assert_eq!(r.bodies.len(), 2);
    assert!((volume_total(&r) - 2000.).abs() < 1e-3);

    let lista = vec![
        req("folga 1", TipoRequisito::Folga { min: 1. }),
        req("folga 3", TipoRequisito::Folga { min: 3. }),
        req("livre", TipoRequisito::SemInterferencia { tol: 1e-3 }),
    ];
    let res = verificar(&d, &lista);
    assert_eq!(res.iter().map(|r| r.ok).collect::<Vec<_>>(), vec![true, false, true], "{res:#?}");
    assert_eq!(res[0].medido, "2 mm", "{:?}", res[0]);

    // o segundo bloco entra 3 mm no primeiro: 3 × 10 × 10 em comum
    let sk2 = d.features.iter().filter(|f| matches!(f.kind, FeatureKind::Sketch(_))).nth(1).unwrap().id;
    set_rect(&mut d, sk2, [7., 0.], [17., 10.]);
    let res = verificar(&d, &lista);
    assert_eq!(res.iter().map(|r| r.ok).collect::<Vec<_>>(), vec![false, false, false], "{res:#?}");
    assert_eq!(res[2].medido, "300 mm³", "{:?}", res[2]);
}

#[test]
fn peca_sem_corpo_reprova_massa_sem_menos_zero() {
    let d = Document::new("vazia");
    let res = verificar(&d, &[req("massa", TipoRequisito::Massa { min: None, max: Some(1.) })]);
    assert!(!res[0].ok);
    assert_eq!(res[0].medido, "—");
}
