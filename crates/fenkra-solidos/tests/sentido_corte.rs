// SPDX-License-Identifier: LGPL-2.1-or-later
//! B392 (K-14 do kart): o recorte que não remove nada diz com certeza se "inverter" resolve.
//! O recorte entra contra a normal do plano do sketch; a adição sai a favor dela.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::sentido_corte::{DESLIGUE_INVERTER, LIGUE_INVERTER, NAO_ALCANCA};
use fenkra_solidos::{recompute, Status};

/// Caixa 40 x 30 x 10 sobre o plano XY (z de 0 a 10) e um recorte circular num plano a `z`.
fn caixa_e_corte(z: &str, centro: [f64; 2], reverse: bool, extent: Extent) -> (Document, Id) {
    let mut d = Document::new("caixa");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [40., 30.]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), z);
    let c = sk_circulos(&mut d, PlaneRef::Datum(pl), &[(centro, 5.)]);
    let id = d.add_feature(FeatureKind::Extrude(Extrude { sketch: c, op: BoolOp::Cut, extent, reverse, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    (d, id)
}

const CHEIO: f64 = 40. * 30. * 10.;

#[test]
fn recorte_na_base_para_fora_manda_ligar_inverter_e_ligado_corta() {
    if !kernel_disponivel() {
        return;
    }
    // sketch na base (z = 0): contra a normal (para -Z) não há peça
    let (d, id) = caixa_e_corte("0", [20., 15.], false, Extent::ThroughAll);
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Warning(LIGUE_INVERTER.into()));
    assert!((volume_total(&r) - CHEIO).abs() < 1e-6);
    let (d, id) = caixa_e_corte("0", [20., 15.], true, Extent::ThroughAll);
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok);
    let furo = std::f64::consts::PI * 25. * 10.;
    assert!((volume_total(&r) - (CHEIO - furo)).abs() < 1e-3 * CHEIO, "{}", volume_total(&r));
}

/// O caso do kart: no topo, quem pensa como na adição liga "inverter" e o recorte sai da peça.
#[test]
fn recorte_no_topo_com_inverter_ligado_manda_desligar() {
    if !kernel_disponivel() {
        return;
    }
    let (d, id) = caixa_e_corte("10", [20., 15.], true, Extent::Blind("4".into()));
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Warning(DESLIGUE_INVERTER.into()));
    let (d, id) = caixa_e_corte("10", [20., 15.], false, Extent::Blind("4".into()));
    assert_eq!(recompute(&d).status[&id], Status::Ok);
}

#[test]
fn recorte_fora_da_peca_nos_dois_sentidos_nao_manda_inverter() {
    if !kernel_disponivel() {
        return;
    }
    let (d, id) = caixa_e_corte("10", [200., 15.], false, Extent::ThroughAll);
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Warning(NAO_ALCANCA.into()));
}
