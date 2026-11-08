// SPDX-License-Identifier: LGPL-2.1-or-later
//! B404 (K-21 do kart): rosca cosmética é o caso normal e fica com status ok. Antes todo comando com
//! rosca cosmética vinha como aviso ("rosca cosmética M24x3"), o que ensina a ignorar aviso. Medida
//! errada e rosca fora da tabela continuam avisando.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

fn eixo(spec: &str, raio: f64) -> (Document, Id) {
    let mut d = Document::new("eixo");
    let sk = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([0., 0.], raio)]);
    d.add_feature(ext(sk, BoolOp::Add, "60"));
    let th = d.add_feature(FeatureKind::Thread(Thread { face: [raio, 0., 30.], spec: spec.into(), length: "40".into(), modeled: false, end: None, start: String::new() }));
    (d, th)
}

#[test]
fn rosca_cosmetica_na_medida_fica_ok_e_nao_corta() {
    if !kernel_disponivel() {
        return;
    }
    let (d, th) = eixo("M24x3", 12.);
    let r = recompute(&d);
    assert_eq!(r.status[&th], Status::Ok);
    let v = volume_total(&r);
    let esperado = std::f64::consts::PI * 144. * 60.;
    assert!((v - esperado).abs() < 1e-6 * esperado, "cosmética não tira material: {v}");
    // o dado da rosca continua no resultado (textura e desenho)
    assert!(r.roscas.iter().any(|x| x.feature == th && !x.modelada));
}

#[test]
fn rosca_cosmetica_com_medida_errada_continua_avisando() {
    if !kernel_disponivel() {
        return;
    }
    // Ø20 com rosca M24: o aviso é de verdade
    let (d, th) = eixo("M24x3", 10.);
    let r = recompute(&d);
    assert!(matches!(&r.status[&th], Status::Warning(m) if m.contains("Ø da face")), "{:?}", r.status[&th]);
}

#[test]
fn furo_com_rosca_cosmetica_fora_da_tabela_avisa() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("bloco");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -20.], [20., 20.]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
    let c = sk_pontos(&mut d, PlaneRef::Datum(pl), &[[0., 0.]]);
    let mut furo = furo_passante(c, "6.8");
    if let FeatureKind::Hole(h) = &mut furo {
        h.thread = Some("XPTO 8".into());
        h.thread_modeled = false;
    }
    let id = d.add_feature(furo);
    let r = recompute(&d);
    assert!(matches!(&r.status[&id], Status::Warning(m) if m.contains("fora da tabela")), "{:?}", r.status[&id]);
}

/// A face interna (furo) e a externa (eixo) são reconhecidas pela normal de verdade: a interna
/// compara com a broca de rosca, a externa com o diâmetro maior. Antes a orientação da face sozinha
/// marcava o eixo extrudado de um círculo como "interno" e a rosca certa avisava.
#[test]
fn rosca_cosmetica_em_furo_na_medida_da_broca_fica_ok() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("bloco");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-30., -30.], [30., 30.]);
    d.add_feature(ext(sk, BoolOp::Add, "40"));
    let f = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([0., 0.], 10.5)]);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: f, op: BoolOp::Cut, extent: Extent::ThroughAll, reverse: true, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let th = d.add_feature(FeatureKind::Thread(Thread { face: [10.5, 0., 20.], spec: "M24x3".into(), length: "30".into(), modeled: false, end: None, start: String::new() }));
    let r = recompute(&d);
    assert_eq!(r.status[&th], Status::Ok);
    assert!(r.roscas.iter().any(|x| x.feature == th && x.interna));
    // e o eixo é externo
    let (d, th) = eixo("M24x3", 12.);
    let r = recompute(&d);
    assert!(r.roscas.iter().any(|x| x.feature == th && !x.interna));
}
