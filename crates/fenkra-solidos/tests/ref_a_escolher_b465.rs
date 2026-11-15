// SPDX-License-Identifier: LGPL-2.1-or-later
//! B465: uma referência (varredura avançada, etc.) recém-aberta, com as guias ainda "a escolher",
//! não acusa "referência perdida (#id)": o campo vazio espera a seleção, inativo e sem erro.
//! Uma guia apagada de verdade continua sendo erro.

use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

fn conica(g1: Id, g2: Id, ap: Id) -> RefKind {
    RefKind::SurfSweepX {
        kind: SweepKind::ConicGuides { guide1: CurveRef::Ref(g1), guide2: CurveRef::Ref(g2), apex: CurveRef::Ref(ap), rho: "0.5".into(), radius: "0".into() },
        spine: None,
        solution: 0,
        smooth: false,
        sections: "12".into(),
    }
}

#[test]
fn varredura_sem_guias_espera_sem_erro() {
    let mut d = Document::new("t");
    let marcador = Id(u64::MAX - 12);
    let id = d.add_geometry(conica(marcador, marcador, marcador));
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Inactive, "{:?}", r.status[&id]);
    assert!(!r.status.values().any(|s| matches!(s, Status::Error(_))), "{:?}", r.status);
}

#[test]
fn guia_apagada_de_verdade_segue_erro() {
    let mut d = Document::new("t");
    let id = d.add_geometry(conica(Id(7), Id(8), Id(9)));
    let r = recompute(&d);
    assert!(matches!(&r.status[&id], Status::Error(m) if m.contains("referência perdida")), "{:?}", r.status[&id]);
}
