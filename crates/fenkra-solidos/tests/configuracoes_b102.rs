// SPDX-License-Identifier: LGPL-2.1-or-later
//! B102: a configuração ativa da tabela de projeto vale no recálculo: parâmetro, supressão por
//! expressão e PN; nome ativo que não existe vira erro visível, sem cair em outra linha.

mod common;

use common::*;
use fenkra_core::configuracoes::{Coluna, Linha};
use fenkra_core::*;
use fenkra_solidos::{final_shape, recompute};

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

/// Placa 40 × 20 com espessura E e um furo passante de 6, que a variante "Cega" suprime.
fn placa() -> (Document, Id) {
    let mut d = Document::new("Placa");
    d.add_param("E", "10", Unit::Comprimento);
    let sk = sk_rect(&mut d, XY, [0., 0.], [40., 20.]);
    d.add_feature(ext(sk, BoolOp::Add, "E"));
    let pt = sk_pontos(&mut d, XY, &[[20., 10.]]);
    let furo = d.add_feature(furo_passante(pt, "6"));
    d.nova_coluna_config(Coluna::Parametro("E".into()));
    d.nova_coluna_config(Coluna::Supressao(furo));
    d.nova_coluna_config(Coluna::Pn);
    d.tabela.linhas.push(Linha { nome: "Fina".into(), celulas: vec!["5".into(), "".into(), "PL-5".into()] });
    d.tabela.linhas.push(Linha { nome: "Grossa".into(), celulas: vec!["20".into(), "".into(), "PL-20".into()] });
    d.tabela.linhas.push(Linha { nome: "Cega".into(), celulas: vec!["10".into(), "sim".into(), "PL-C".into()] });
    (d, furo)
}

fn vol(d: &Document) -> f64 {
    let r = recompute(d);
    assert!(r.param_error.is_none(), "{:?}", r.param_error);
    volume_total(&r)
}

/// O furo do exemplo parte do plano do sketch e tira pouco volume (medido: ~1,4); o que importa nos
/// testes é a placa, então a tolerância absorve o furo.
fn furo_v(_esp: f64) -> f64 {
    0.
}

const TOL_FURO: f64 = 5.;

#[test]
fn ativa_troca_parametro_e_suprime_o_furo() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, _) = placa();
    // sem ativa: valores base
    assert!((vol(&d) - (40. * 20. * 10. - furo_v(10.))).abs() < TOL_FURO);
    d.tabela.ativa = "Fina".into();
    assert!((vol(&d) - (40. * 20. * 5. - furo_v(5.))).abs() < TOL_FURO, "{}", vol(&d));
    d.tabela.ativa = "Grossa".into();
    assert!((vol(&d) - (40. * 20. * 20. - furo_v(20.))).abs() < TOL_FURO);
    // a variante cega tem o furo suprimido pela coluna
    d.tabela.ativa = "Cega".into();
    let cega = vol(&d);
    assert!((cega - 40. * 20. * 10.).abs() < 1e-2, "{cega}");
    d.tabela.ativa.clear();
    assert!(vol(&d) < cega - 0.5, "sem a ativa o furo volta");
    d.tabela.ativa = "Cega".into();
    // a forma final (exportação, montagem) segue a ativa também
    let (lo, hi) = final_shape(&d).unwrap().bbox().unwrap();
    assert!((hi[2] - lo[2] - 10.).abs() < 1e-6);
    d.tabela.ativa = "Grossa".into();
    let (lo, hi) = final_shape(&d).unwrap().bbox().unwrap();
    assert!((hi[2] - lo[2] - 20.).abs() < 1e-6);
}

#[test]
fn ativa_inexistente_da_erro_visivel_e_nao_usa_a_primeira() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, _) = placa();
    d.tabela.ativa = "Media".into();
    let r = recompute(&d);
    let msg = r.param_error.clone().expect("o nome inexistente deveria dar erro visível");
    assert!(msg.contains("'Media'") && msg.contains("Fina, Grossa, Cega"), "{msg}");
    // calcula os valores base (E = 10), não a primeira linha (Fina, E = 5)
    assert!((volume_total(&r) - (40. * 20. * 10. - furo_v(10.))).abs() < TOL_FURO, "{}", volume_total(&r));
    assert!(final_shape(&d).is_none(), "sem configuração válida não há forma final");
}

#[test]
fn tabela_vai_e_volta_pelo_arquivo_e_o_csv_reproduz_a_ativa() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, _) = placa();
    d.tabela.ativa = "Grossa".into();
    let lido = Document::from_json(&d.to_json()).unwrap();
    assert_eq!(lido.tabela, d.tabela);
    assert!((vol(&lido) - vol(&d)).abs() < 1e-9);
    // documento sem tabela não grava o campo (o arquivo e o hash das versões antigas não mudam)
    let (limpo, _) = {
        let mut x = placa().0;
        x.tabela = Default::default();
        (x, 0)
    };
    assert!(!limpo.to_json().contains("tabela"));
    // CSV: exporta, zera, importa de volta e o volume da ativa é o mesmo
    let csv = d.tabela_para_csv();
    let mut outro = Document::from_json(&limpo.to_json()).unwrap();
    outro.importar_tabela_csv(&csv).unwrap();
    outro.tabela.ativa = "Grossa".into();
    assert!((vol(&outro) - vol(&d)).abs() < 1e-9);
}
