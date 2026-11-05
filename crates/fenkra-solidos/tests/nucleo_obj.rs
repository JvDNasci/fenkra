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
    let ruim = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Symmetric("10".into()), reverse: false, taper: "3".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let filho = d.add_feature(FeatureKind::Fillet(EdgeOp { value: "1".into(), edges: EdgeSel::CreatedBy(ruim), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
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

// ------------------------------------------------------------------ B12: loft com torção e condição de ponta

fn loft_de(base: [f64; 2], topo: [f64; 2], ajusta: impl FnOnce(&mut Loft)) -> (f64, Status) {
    let mut d = Document::new("loft");
    let s0 = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-base[0] / 2., -base[1] / 2.], [base[0] / 2., base[1] / 2.]);
    let alto = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30");
    let s1 = sk_rect(&mut d, PlaneRef::Datum(alto), [-topo[0] / 2., -topo[1] / 2.], [topo[0] / 2., topo[1] / 2.]);
    let mut l = Loft { sections: vec![s0, s1], ruled: false, closed: false, op: BoolOp::Add, torcao: vec![], inicio: Default::default(), fim: Default::default() };
    ajusta(&mut l);
    let id = d.add_feature(FeatureKind::Loft(l));
    let r = recompute(&d);
    (volume(&r), r.status[&id].clone())
}

#[test]
fn loft_torcao_pelo_ponto_de_partida_muda_o_solido() {
    if !kernel_disponivel() {
        return;
    }
    let (reto, st) = loft_de([40., 20.], [40., 20.], |_| {});
    assert!(matches!(st, Status::Ok), "{st:?}");
    assert!((reto - 40. * 20. * 30.).abs() < 1., "{reto}");
    let (torcido, st) = loft_de([40., 20.], [40., 20.], |l| l.torcao = vec![0, 1]);
    assert!(matches!(st, Status::Ok), "{st:?}");
    assert!((torcido - reto).abs() > 1., "a torção de um vértice deve mudar o volume: {torcido} x {reto}");
}

#[test]
fn loft_condicao_normal_na_ponta_e_validada() {
    if !kernel_disponivel() {
        return;
    }
    let (livre, st) = loft_de([40., 40.], [16., 16.], |_| {});
    assert!(matches!(st, Status::Ok), "{st:?}");
    let ponta = PontaLoft { condicao: CondicaoPonta::Normal, intensidade: "1".into() };
    let (normal, st) = loft_de([40., 40.], [16., 16.], |l| l.inicio = ponta.clone());
    assert!(matches!(st, Status::Ok), "{st:?}");
    assert!(normal > livre + 1., "sair na normal enche o começo: {normal} x {livre}");
    let (zero, st) = loft_de([40., 40.], [16., 16.], |l| l.inicio = PontaLoft { condicao: CondicaoPonta::Normal, intensidade: "0".into() });
    assert!(matches!(st, Status::Ok) && (zero - livre).abs() < 1e-6, "{zero} x {livre}");
    let (_, st) = loft_de([40., 40.], [16., 16.], |l| l.inicio = PontaLoft { condicao: CondicaoPonta::Normal, intensidade: "9".into() });
    assert!(matches!(&st, Status::Error(m) if m.contains("entre 0 e 3")), "{st:?}");
    let (_, st) = loft_de([40., 40.], [16., 16.], |l| {
        l.ruled = true;
        l.fim = ponta.clone();
    });
    assert!(matches!(&st, Status::Error(m) if m.contains("loft suave")), "{st:?}");
}

#[test]
fn loft_antigo_sem_torcao_nem_ponta_abre() {
    let j = r#"{"sections":[1,2],"ruled":false,"closed":false,"op":"Add"}"#;
    let l: Loft = serde_json::from_str(j).unwrap();
    assert!(l.torcao.is_empty() && l.inicio.e_livre() && l.fim.e_livre());
    let volta = serde_json::to_string(&l).unwrap();
    assert!(!volta.contains("torcao") && !volta.contains("inicio"), "{volta}");
}
