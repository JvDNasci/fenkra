// SPDX-License-Identifier: LGPL-2.1-or-later
//! B59 e B69: causa estável de cada falha, Atualizar esta feature e Cancelar real.
//!
//! Atualizar esta feature calcula só até um comando e lista os de depois como desatualizados;
//! Cancelar para antes do próximo comando e o que já foi feito fica no cache, então calcular de
//! novo continua de onde parou, sem refazer.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::erro::ErrorKind;
use fenkra_solidos::{recompute, recompute_cached, Cache, Status};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn caixa_com_furo() -> (Document, Id, Id, Id) {
    let mut d = Document::new("caixa");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    let base = d.add_feature(ext(sk, BoolOp::Add, "10"));
    let sk2 = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-5., -5.], [5., 5.]);
    let topo = d.add_feature(ext(sk2, BoolOp::Add, "20"));
    (d, base, sk2, topo)
}

fn volume(r: &fenkra_solidos::RecomputeResult) -> f64 {
    r.bodies.iter().map(|b| b.mass.volume).sum()
}

#[test]
fn erro_estavel_referencia_perdida() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("perdida");
    let id = d.add_feature(ext(Id(9999), BoolOp::Add, "10"));
    let r = recompute(&d);
    assert!(matches!(r.status[&id], Status::Error(_)));
    let k = r.tipos_de_erro(&d);
    assert_eq!(k[&id], ErrorKind::ReferenciaPerdida { ausente: Id(9999) });
    assert_eq!(k[&id].codigo(), "referencia_perdida");
}

#[test]
fn erro_estavel_aponta_o_primeiro_pai_que_falhou() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("pai");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    // saída (ângulo) só vale com término cego: o kernel nem é chamado, o comando falha por si
    let ruim = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Symmetric("10".into()), reverse: false, taper: "3".into(), thin: None, dir: None, thin_side: 0 }));
    let filho = d.add_feature(FeatureKind::Fillet(EdgeOp { value: "1".into(), edges: EdgeSel::CreatedBy(ruim), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![] }));
    let r = recompute(&d);
    assert!(matches!(r.status[&ruim], Status::Error(_)));
    let k = r.tipos_de_erro(&d);
    assert_eq!(k[&ruim], ErrorKind::Propria);
    if matches!(r.status[&filho], Status::Error(_)) {
        assert_eq!(k[&filho], ErrorKind::PaiFalhou { causa: ruim });
        assert_eq!(k[&filho].corrigir_primeiro(), Some(ruim));
    }
}

#[test]
fn atualizar_esta_feature_para_no_comando_e_lista_o_resto() {
    if !kernel_disponivel() {
        return;
    }
    let (d, base, _sk2, topo) = caixa_com_furo();
    let completo = recompute(&d);
    assert!(completo.desatualizados.is_empty() && !completo.cancelado);

    let mut cache = Cache::default();
    cache.ate = Some(base);
    let parcial = recompute_cached(&d, &mut cache);
    assert!(!parcial.cancelado);
    assert!(parcial.desatualizados.contains(&topo), "{:?}", parcial.desatualizados);
    assert!(!parcial.desatualizados.contains(&base));
    assert!(matches!(parcial.status[&base], Status::Ok));
    assert!(volume(&parcial) < volume(&completo), "sem o comando de cima o sólido é menor");
    assert!((volume(&parcial) - 40. * 30. * 10.).abs() < 1., "{}", volume(&parcial));
    assert!(cache.ate.is_none(), "o pedido vale uma vez só");

    // atualizar tudo depois continua de onde parou
    let todo = recompute_cached(&d, &mut cache);
    assert!(todo.desatualizados.is_empty());
    assert!(cache.reused >= 2, "os sketches e a base ficam no cache: {}", cache.reused);
    assert!((volume(&todo) - volume(&completo)).abs() < 1e-6);
}

#[test]
fn cancelar_para_antes_do_proximo_comando_e_o_cache_continua_valendo() {
    if !kernel_disponivel() {
        return;
    }
    let (d, base, _sk2, topo) = caixa_com_furo();
    let bandeira = Arc::new(AtomicBool::new(true));
    let mut cache = Cache::default();
    cache.cancelar = Some(bandeira.clone());
    let r = recompute_cached(&d, &mut cache);
    assert!(r.cancelado);
    assert!(r.desatualizados.contains(&base) && r.desatualizados.contains(&topo));
    assert!(r.bodies.is_empty(), "nada foi calculado");

    bandeira.store(false, Ordering::Relaxed);
    let r = recompute_cached(&d, &mut cache);
    assert!(!r.cancelado && r.desatualizados.is_empty());
    assert!(matches!(r.status[&topo], Status::Ok));
    assert!(volume(&r) > 40. * 30. * 10.);
}
