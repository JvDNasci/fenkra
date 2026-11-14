// SPDX-License-Identifier: LGPL-2.1-or-later
//! B112: aceite do motor de regras. Uma peça com furo a 0,5 mm da borda e parede de 0,3 mm gera as
//! duas violações com os identificadores certos, e mudar o limite no JSON da pasta do usuário muda o
//! resultado sem recompilar nada.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::dfm::{self, Perfil};
use fenkra_solidos::final_shape;
use std::f64::consts::PI;

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

/// Extrusão para baixo do plano XY: a peça fica de z = -h a 0 e o furo, que entra contra a normal
/// do plano do sketch, atravessa a peça.
fn baixo(sk: Id, op: BoolOp, h: &str) -> FeatureKind {
    let mut k = ext(sk, op, h);
    if let FeatureKind::Extrude(e) = &mut k {
        e.reverse = true;
    }
    k
}

/// Bloco 30 x 20 x 10 (z de -10 a 0) com um furo de 4 mm centrado a 2,5 mm da face x = 0 (parede de 0,5 mm) e um
/// rasgo passante de x = 10 a 29,7 que deixa uma parede de 0,3 mm na face x = 30.
fn peca() -> Document {
    let mut d = Document::new("furo_na_borda");
    let a = sk_rect(&mut d, XY, [0., 0.], [30., 20.]);
    d.add_feature(baixo(a, BoolOp::Add, "10"));
    let p = sk_pontos(&mut d, XY, &[[2.5, 10.]]);
    d.add_feature(furo_passante(p, "4"));
    let r = sk_rect(&mut d, XY, [10., 5.], [29.7, 15.]);
    d.add_feature(ext(r, BoolOp::Cut, "10"));
    d
}

fn fresagem() -> Perfil {
    dfm::perfis_padrao().into_iter().find(|p| p.processo == "cnc").unwrap()
}

#[test]
fn furo_a_meio_milimetro_da_borda_e_parede_de_0_3_geram_as_duas_violacoes() {
    if !kernel_disponivel() {
        return;
    }
    let d = peca();
    let vol = 30. * 20. * 10. - PI * 4. * 10. - 19.7 * 10. * 10.;
    confere("furo na borda", &d, &Esperado { vol, tol_vol: 1e-4, tam: [30., 20., 10.], tol_tam: 1e-3, minimo: Some([0., 0., -10.]), via_malha: false });
    let s = final_shape(&d).unwrap();
    let r = dfm::varrer(&s, &fresagem()).unwrap();
    // o furo: a parede até x = 0 tem 0,5 mm (a tesselação do furo deixa a parede um pouco maior)
    let b: Vec<_> = r.das_regras("cnc.furo.borda").collect();
    assert_eq!(b.len(), 1, "{:#?}", r.violacoes);
    assert!((b[0].valor - 0.5).abs() < 0.02, "{:?}", b[0]);
    assert!(b[0].ponto[0] < 1., "o ponto fica do lado da borda: {:?}", b[0].ponto);
    assert!(!b[0].faces.is_empty(), "a violação cita a face do furo para o realce");
    // a parede de 0,3 mm entre o rasgo e a face x = 30
    let w: Vec<_> = r.das_regras("cnc.parede.espessura").filter(|v| v.ponto[0] > 29.).collect();
    assert!(!w.is_empty(), "{:#?}", r.violacoes);
    assert!(w.iter().all(|v| (v.valor - 0.3).abs() < 0.03), "{w:#?}");
    // os dois identificadores certos, e nenhuma regra estranha ao perfil
    for v in &r.violacoes {
        assert!(fresagem().regra(&v.regra).is_some(), "{}", v.regra);
    }
}

#[test]
fn furo_longe_da_borda_nao_acusa() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("furo_no_meio");
    let a = sk_rect(&mut d, XY, [0., 0.], [30., 20.]);
    d.add_feature(baixo(a, BoolOp::Add, "10"));
    let p = sk_pontos(&mut d, XY, &[[15., 10.]]);
    d.add_feature(furo_passante(p, "4"));
    confere("furo no meio", &d, &Esperado::prisma(6000. - PI * 40., [30., 20., 10.], [0., 0., -10.]));
    let r = dfm::varrer(&final_shape(&d).unwrap(), &fresagem()).unwrap();
    assert_eq!(r.das_regras("cnc.furo.borda").count(), 0, "{:#?}", r.violacoes);
}

#[test]
fn limite_editado_no_json_do_usuario_muda_o_resultado_sem_recompilar() {
    if !kernel_disponivel() {
        return;
    }
    let s = final_shape(&peca()).unwrap();
    let dir = std::env::temp_dir().join(format!("dfm_b112_borda_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // limite de 0,4 mm: a parede de 0,5 passa
    let mut p = fresagem();
    p.ajusta("cnc.furo.borda", Some(0.4), None);
    std::fs::write(dir.join("fresagem.json"), p.para_json()).unwrap();
    let (v, erros) = dfm::carregar_pasta(&dir);
    assert!(erros.is_empty(), "{erros:?}");
    let lido = v.iter().find(|q| q.processo == "cnc").unwrap();
    assert_eq!(dfm::varrer(&s, lido).unwrap().das_regras("cnc.furo.borda").count(), 0);
    // de volta a 2 mm, editando só o texto do arquivo
    let t = std::fs::read_to_string(dir.join("fresagem.json")).unwrap().replace("\"min\": 0.4", "\"min\": 2.0");
    std::fs::write(dir.join("fresagem.json"), t).unwrap();
    let (v, _) = dfm::carregar_pasta(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lido = v.iter().find(|q| q.processo == "cnc").unwrap();
    let r = dfm::varrer(&s, lido).unwrap();
    let b: Vec<_> = r.das_regras("cnc.furo.borda").collect();
    assert_eq!(b.len(), 1);
    assert!((b[0].limite - 2.).abs() < 1e-12);
}

#[test]
fn chapa_compara_a_parede_do_furo_com_a_espessura() {
    if !kernel_disponivel() {
        return;
    }
    // chapa de 2 mm, furo de 3 mm com centro a 3 mm da borda: parede de 1,5 mm = 0,75 espessura
    let mut d = Document::new("chapa_furo_borda");
    let a = sk_rect(&mut d, XY, [0., 0.], [60., 40.]);
    d.add_feature(baixo(a, BoolOp::Add, "2"));
    let p = sk_pontos(&mut d, XY, &[[3., 20.]]);
    d.add_feature(furo_passante(p, "3"));
    confere("chapa furo borda", &d, &Esperado::prisma(60. * 40. * 2. - PI * 2.25 * 2., [60., 40., 2.], [0., 0., -2.]));
    let chapa = dfm::perfis_padrao().into_iter().find(|p| p.processo == "chapa").unwrap();
    let r = dfm::varrer(&final_shape(&d).unwrap(), &chapa).unwrap();
    let b: Vec<_> = r.das_regras("chapa.furo.borda").collect();
    assert_eq!(b.len(), 1, "{:#?}", r.violacoes);
    assert!((b[0].valor - 0.75).abs() < 0.03, "{:?}", b[0]);
}
