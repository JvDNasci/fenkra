// SPDX-License-Identifier: LGPL-2.1-or-later
//! K-08 do kart: superfície de varredura com perfil circular numa spline 3D (linha de freio).
//! Antes falhava com "não foi possível varrer o perfil ao longo da guia" nos modos Corrigido e Frenet;
//! a varredura sólida na mesma guia já usava a escada de tentativas (triedro discreto, guia dividida).

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

fn linha_de_freio(mode: SweepMode) -> (Document, Id, Id, Id) {
    let mut d = Document::new("ex12");
    let pts = [[600., -120., 60.], [420., -300., 60.], [260., -330., 90.], [120., -280., 120.], [60., -160., 110.]];
    let c = d.add_geometry(RefKind::Spline { points: pts.iter().map(|p| PointRef::Fixed(*p)).collect(), closed: false, tangents: vec![] });
    let sup = d.add_geometry(RefKind::SurfSweep { profile: SweepProfile::Circle("2.5".into()), spine: CurveRef::Ref(c), mode });
    let f = d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(sup), t1: "1".into(), t2: "0".into(), op: BoolOp::Add }));
    (d, c, sup, f)
}

#[test]
fn tubo_por_spline_3d_varre_em_corrigido() {
    if !kernel_disponivel() {
        return;
    }
    let (d, c, sup, f) = linha_de_freio(SweepMode::Corrigido);
    let r = recompute(&d);
    assert_eq!(r.status[&sup], Status::Ok, "varredura {:?}", r.status[&sup]);
    // Engrossar não pode falhar (era aqui que o K-08 aparecia); o aviso de validade é permitido
    assert!(!matches!(r.status[&f], Status::Error(_)), "engrossar {:?}", r.status[&f]);
    // o engrosse de 1 mm entra no tubo (normal para dentro): V = comprimento · π(2,5² − 1,5²)
    let comp: f64 = r.curves.iter().find(|x| x.id == c).unwrap().pts.windows(2).map(|w| geo3d_dist(w[0], w[1])).sum();
    let esperado = comp * std::f64::consts::PI * (2.5f64.powi(2) - 1.5f64.powi(2));
    let v = volume_total(&r);
    assert!((v / esperado - 1.).abs() < 0.1, "volume {v} esperado {esperado}");
}

/// Frenet puro nesta guia não tem solução estável: o erro precisa dizer o motivo e o caminho.
#[test]
fn tubo_por_spline_3d_em_frenet_explica_a_falha_ou_varre() {
    if !kernel_disponivel() {
        return;
    }
    let (d, _, sup, _) = linha_de_freio(SweepMode::Frenet);
    let r = recompute(&d);
    match &r.status[&sup] {
        Status::Ok => {}
        Status::Error(m) => assert!(m.contains("Frenet") && m.contains("Corrigido"), "{m}"),
        outro => panic!("{outro:?}"),
    }
}

fn geo3d_dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}
