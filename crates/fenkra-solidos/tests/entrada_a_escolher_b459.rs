// SPDX-License-Identifier: LGPL-2.1-or-later
//! B459: um comando recém-aberto, com o perfil ainda "a escolher", não acusa referência perdida.
//! O marcador do campo vazio (`Id::A_ESCOLHER`) não é um objeto apagado: o comando fica inativo,
//! sem erro, e o resto do documento segue igual. Um sketch apagado de verdade continua sendo erro.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

/// Bloco 60 × 40 × 30 e, depois dele, uma extrusão com o perfil `perfil`.
fn bloco_com_extrusao(perfil: Id) -> (Document, Id, Id) {
    let mut d = Document::new("t");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [60., 40.]);
    let base = d.add_feature(ext(sk, BoolOp::Add, "30"));
    let nova = d.add_feature(ext(perfil, BoolOp::Add, "10"));
    (d, base, nova)
}

#[test]
fn extrusao_sem_perfil_espera_sem_erro() {
    // o marcador de sketch "a escolher" que o painel põe no comando novo
    let a_escolher = Id(u64::MAX - 16);
    assert!(a_escolher.a_escolher());
    let (d, base, nova) = bloco_com_extrusao(a_escolher);
    let r = recompute(&d);
    assert_eq!(r.status[&base], Status::Ok);
    assert_eq!(r.status[&nova], Status::Inactive, "pede o perfil, sem erro: {:?}", r.status[&nova]);
    assert!(!r.status.values().any(|s| matches!(s, Status::Error(_))), "{:?}", r.status);
    assert!(r.tipos_de_erro(&d).is_empty(), "nada vai para o assistente de reparo");
    // o bloco segue inteiro
    assert_eq!(r.bodies.len(), 1);
    let v = r.bodies[0].mass.volume;
    assert!((v - 60. * 40. * 30.).abs() < 1., "volume {v}");
    let (lo, hi) = caixa_da_malha(&r);
    assert!((hi[0] - lo[0] - 60.).abs() < 0.01 && (hi[1] - lo[1] - 40.).abs() < 0.01 && (hi[2] - lo[2] - 30.).abs() < 0.01, "{lo:?} {hi:?}");
}

#[test]
fn todo_marcador_a_escolher_vale_e_ids_comuns_nao() {
    for k in 11..=16 {
        assert!(Id(u64::MAX - k).a_escolher(), "u64::MAX - {k}");
    }
    assert!(!Id(u64::MAX).a_escolher() && !Id(u64::MAX - 17).a_escolher() && !Id(7).a_escolher());
}

#[test]
fn sketch_apagado_continua_referencia_perdida() {
    let (d, _, nova) = bloco_com_extrusao(Id(987_654));
    let r = recompute(&d);
    assert!(matches!(&r.status[&nova], Status::Error(m) if m.starts_with("referência perdida")), "{:?}", r.status[&nova]);
}
