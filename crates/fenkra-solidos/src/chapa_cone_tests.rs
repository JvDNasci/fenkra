// SPDX-License-Identifier: LGPL-2.1-or-later
//! B40: dobra cônica no documento: recalcula, conferida dobrada e planificada, e a volta.

use super::*;
use std::f64::consts::TAU;

fn s(x: &str) -> String {
    x.to_string()
}

const T: f64 = 2.;

/// Chapa de 2 mm (K = 0,5 para o volume do setor bater com o girado) com um tronco r1 = 20, r2 = 40, h = 50.
fn doc_cone(r2: &str, k: &str, center: [f64; 3]) -> (Document, Id) {
    let mut d = Document::new("cone");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s(k), flat: false, ..Default::default() });
    let id = d.add_feature(FeatureKind::SmCone(SmCone { r1: s("20"), r2: s(r2), height: s("50"), center, k: None }));
    (d, id)
}

fn plano(d: &Document) -> Document {
    let mut f = d.clone();
    f.sheet.as_mut().unwrap().flat = true;
    f
}

fn status(d: &Document, id: Id) -> Status {
    recompute(d).status[&id].clone()
}

fn caixa(d: &Document) -> ([f64; 3], [f64; 3]) {
    final_shape(d).expect("corpo").bbox().expect("caixa")
}

fn volume(d: &Document) -> f64 {
    final_shape(d).expect("corpo").mass().volume
}

/// Pappus do perfil do tronco: área = geratriz × espessura, centróide a meia espessura da face interna.
fn volume_analitico(r1: f64, r2: f64, h: f64) -> f64 {
    let l = (r2 - r1).hypot(h);
    let cos_a = h / l;
    l * T * TAU * ((r1 + r2) / 2. + T / 2. * cos_a)
}

#[test]
fn dobrada_tem_o_volume_e_a_caixa_do_tronco() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, id) = doc_cone("40", "0.5", [0.; 3]);
    assert!(matches!(status(&d, id), Status::Ok), "{:?}", status(&d, id));
    let v = volume(&d);
    let esperado = volume_analitico(20., 40., 50.);
    assert!((v - esperado).abs() < 0.002 * esperado, "volume {v} contra {esperado}");
    let (lo, hi) = caixa(&d);
    let sa = 20. / (20f64).hypot(50.);
    let r_max = 40. + T * (1. - sa * sa).sqrt();
    assert!((hi[0] - r_max).abs() < 0.05 && (lo[1] + r_max).abs() < 0.05, "{lo:?} {hi:?}");
    assert!((hi[2] - 50.).abs() < 0.05 && (lo[2] + T * sa).abs() < 0.05, "z {lo:?} {hi:?}");
}

#[test]
fn planificada_e_o_setor_do_desenvolvimento_e_o_volume_confere() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, id) = doc_cone("40", "0.5", [0.; 3]);
    let p = plano(&d);
    assert!(matches!(status(&p, id), Status::Ok), "{:?}", status(&p, id));
    let c = chapa_cone::desenvolve(20., 40., 50., T, 0.5).unwrap();
    // mesmo material nos dois estados (K = 0,5) e igual à área do setor × espessura
    let (vd, vp) = (volume(&d), volume(&p));
    assert!((vd - vp).abs() < 0.002 * vd, "dobrado {vd} contra plano {vp}");
    assert!((vp - c.area() * T).abs() < 0.002 * vp);
    // caixa: espessura em Z e raio externo do setor no plano
    let (lo, hi) = caixa(&p);
    assert!((hi[2] - T).abs() < 1e-6 && lo[2].abs() < 1e-6, "{lo:?} {hi:?}");
    let largura = hi[0].max(hi[1]);
    assert!(largura <= c.rho[1] + 1e-6 && largura >= c.rho[1] * 0.99, "{largura} contra {}", c.rho[1]);
    // o blank da planificação exata: área com a espessura em mm² e contorno de arcos e retas
    let pl = planificada::planificar(&d).expect("planificar");
    assert!((pl.propriedades.volume - vp).abs() < 0.002 * vp, "{:?}", pl.propriedades);
    assert!((pl.propriedades.area - c.area()).abs() < 0.002 * c.area(), "{:?} contra {}", pl.propriedades, c.area());
    assert!(pl.valida(), "{:?}", pl.problemas());
}

#[test]
fn planificar_e_voltar_devolve_o_mesmo_corpo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, _) = doc_cone("40", "0.5", [0.; 3]);
    let antes = volume(&d);
    let (lo0, hi0) = caixa(&d);
    let mut d2 = plano(&d);
    assert!((volume(&d2) - antes).abs() < 0.002 * antes);
    d2.sheet.as_mut().unwrap().flat = false;
    assert!((volume(&d2) - antes).abs() < 1e-6 * antes);
    let (lo1, hi1) = caixa(&d2);
    for j in 0..3 {
        assert!((lo0[j] - lo1[j]).abs() < 1e-6 && (hi0[j] - hi1[j]).abs() < 1e-6);
    }
}

#[test]
fn fator_k_diferente_de_meio_avisa_da_diferenca_de_volume() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, id) = doc_cone("40", "0.3", [0.; 3]);
    let Status::Warning(m) = status(&d, id) else { panic!("esperava aviso, veio {:?}", status(&d, id)) };
    assert!(m.contains("fator K 0.30"), "{m}");
    // a razão anunciada é a real: a planificada tem um pouco menos de material que o dobrado
    let (vd, vp) = (volume(&d), volume(&plano(&d)));
    let cos_a = 50. / (20f64).hypot(50.);
    let rm = 30.;
    let razao = (rm + 0.3 * T * cos_a) / (rm + T / 2. * cos_a);
    assert!((vp / vd - razao).abs() < 0.002, "{} contra {razao}", vp / vd);
}

#[test]
fn centro_desloca_o_corpo_e_parametros_valem() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, id) = doc_cone("raio_maior", "0.5", [100., -30., 5.]);
    d.add_param("raio_maior", "40", fenkra_core::Unit::Comprimento).unwrap();
    assert!(matches!(status(&d, id), Status::Ok), "{:?}", status(&d, id));
    let (lo, hi) = caixa(&d);
    assert!((hi[2] - 55.).abs() < 0.05, "{lo:?} {hi:?}");
    assert!(((hi[0] + lo[0]) / 2. - 100.).abs() < 0.05 && ((hi[1] + lo[1]) / 2. + 30.).abs() < 0.05);
}

#[test]
fn entradas_invalidas_dao_erro_no_comando() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, id) = doc_cone("20", "0.5", [0.; 3]);
    assert!(matches!(status(&d, id), Status::Error(_)), "cilindro não é cone: {:?}", status(&d, id));
    let (d, id) = doc_cone("10", "0.5", [0.; 3]);
    assert!(matches!(status(&d, id), Status::Error(_)), "boca menor maior que a maior");
}

#[test]
fn salva_e_abre_sem_perder_o_comando() {
    let (d, id) = doc_cone("40", "0.5", [1., 2., 3.]);
    let lido = Document::from_json(&d.to_json()).expect("abrir");
    assert_eq!(lido.feature(id).map(|f| f.kind.clone()), d.feature(id).map(|f| f.kind.clone()));
    if fenkra_kernel::AVAILABLE {
        assert!((volume(&lido) - volume(&d)).abs() < 1e-9);
    }
}
