// SPDX-License-Identifier: LGPL-2.1-or-later
//! B259: o cálculo cancelável. Booleana e malha do OpenCASCADE abandonam o trabalho quando a
//! bandeira sobe; o recálculo para sem sujar o cache; e mudar uma cota durante um cálculo longo
//! cancela o anterior e entrega o mesmo resultado de um cálculo do zero.

mod common;

use common::*;
use fenkra_core::trabalho::Fila;
use fenkra_core::*;
use fenkra_kernel::{Frame, Shape};
use fenkra_solidos::{recompute, recompute_cached, Cache, RecomputeResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

fn quadro() -> Frame {
    Frame { origin: [0., 0., 0.], x: [1., 0., 0.], normal: [0., 0., 1.] }
}

/// Peça com vários comandos: base, furos (cortes) e uma segunda extrusão que lê a cota `L`.
fn peca(l: &str) -> Document {
    let mut d = Document::new("cancelável");
    d.add_param("L", l, Unit::Comprimento).unwrap();
    let sk = sk_rect(&mut d, XY, [0., 0.], [40., 40.]);
    d.add_feature(ext(sk, BoolOp::Add, "L"));
    let sk2 = sk_rect(&mut d, XY, [5., 5.], [15., 15.]);
    d.add_feature(ext(sk2, BoolOp::Add, "L + 5"));
    let sk3 = sk_rect(&mut d, XY, [25., 25.], [35., 35.]);
    d.add_feature(ext(sk3, BoolOp::Add, "L + 8"));
    d
}

#[test]
fn booleana_e_malha_abandonam_com_a_bandeira_levantada() {
    if !kernel_disponivel() {
        return;
    }
    let a = Shape::primitive(0, &quadro(), &[20., 20., 20.]).unwrap();
    let b = Shape::primitive(0, &Frame { origin: [5., 5., 5.], ..quadro() }, &[20., 20., 20.]).unwrap();
    let bandeira = Arc::new(AtomicBool::new(true));
    {
        let _g = fenkra_kernel::ligar_cancelamento(bandeira.clone());
        let e = a.boolean(&b, fenkra_kernel::Bool::Fuse).err().expect("a booleana deveria abandonar");
        assert!(e.0.contains("cancelado"), "{}", e.0);
        let e = a.mesh(0.1).err().expect("a malha deveria abandonar");
        assert!(e.0.contains("cancelado"), "{}", e.0);
    }
    // sem a guarda, a thread volta a ser incancelável e o mesmo cálculo termina
    let ok = a.boolean(&b, fenkra_kernel::Bool::Fuse).expect("sem cancelamento a booleana termina");
    assert!(ok.mass().volume > 8000.0);
    // com a guarda mas a bandeira baixa, também termina
    bandeira.store(false, Ordering::Relaxed);
    let _g = fenkra_kernel::ligar_cancelamento(bandeira);
    assert!(a.boolean(&b, fenkra_kernel::Bool::Fuse).is_ok());
    assert!(a.mesh(0.1).is_ok());
}

#[test]
fn recalculo_cancelado_nao_suja_o_cache_e_o_proximo_igual_ao_do_zero() {
    if !kernel_disponivel() {
        return;
    }
    let d = peca("10");
    let do_zero = volume_total(&recompute(&d));
    assert!(do_zero > 0.0);

    let mut cache = Cache::default();
    let bandeira = Arc::new(AtomicBool::new(true));
    cache.cancelar = Some(bandeira.clone());
    let r: RecomputeResult = recompute_cached(&d, &mut cache);
    assert!(r.cancelado, "o cálculo percebeu o pedido de cancelamento");
    assert_eq!(r.desatualizados.len(), d.features.len(), "nenhum comando ficou como calculado");
    assert!(volume_total(&r) < 1.0);

    // mesma cache, bandeira baixa: refaz tudo e dá o resultado do zero
    bandeira.store(false, Ordering::Relaxed);
    let r2 = recompute_cached(&d, &mut cache);
    assert!(!r2.cancelado);
    assert!((volume_total(&r2) - do_zero).abs() < 1e-6, "{} x {do_zero}", volume_total(&r2));
}

#[test]
fn mudar_a_cota_durante_o_calculo_cancela_o_anterior_e_o_final_e_o_do_zero() {
    if !kernel_disponivel() {
        return;
    }
    // a mesma montagem da fila do aplicativo: thread com cache próprio, um pedido novo cancela o anterior
    let fila: Fila<Document, (String, RecomputeResult)> = Fila::nova(
        "teste-recompute",
        |_, novo| novo,
        || {
            let mut cache = Cache::default();
            Box::new(move |doc: Document, ctl: &fenkra_core::trabalho::Controle| {
                cache.cancelar = Some(ctl.bandeira());
                cache.progresso = Some(ctl.clone());
                let r = recompute_cached(&doc, &mut cache);
                Some((doc.params[0].expr.clone(), r))
            })
        },
        || {},
    );
    // pedido 1 (L = 10) seguido, sem esperar, do pedido 2 (L = 17)
    fila.enviar(peca("10"));
    let t = Instant::now();
    fila.enviar(peca("17"));
    let esperado = volume_total(&recompute(&peca("17")));
    let (expr, r) = loop {
        if let Some(x) = fila.receber() {
            break x;
        }
        assert!(t.elapsed() < Duration::from_secs(60), "o cálculo não voltou");
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(expr, "17", "só o resultado do pedido mais novo chega");
    assert!(!r.cancelado);
    assert!((volume_total(&r) - esperado).abs() < 1e-6, "{} x {esperado}", volume_total(&r));
    // e nenhum resultado do pedido cancelado aparece depois
    std::thread::sleep(Duration::from_millis(300));
    assert!(fila.receber().is_none());
}
