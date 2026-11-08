// SPDX-License-Identifier: LGPL-2.1-or-later
//! B391 (K-13 do kart): chanfro e filete por ponto. Qualquer ponto sobre a aresta a identifica; antes
//! só o ponto médio exato valia, e numa aresta circular ele fica do lado oposto da costura, onde quem
//! usa a API não tem como adivinhá-lo. Ponto fora da aresta ou no vértice entre duas continua erro.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

fn cilindro() -> Document {
    let mut d = Document::new("cil");
    let sk = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([0., 0.], 10.)]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    d
}

fn com(d: &Document, f: FeatureKind) -> (Document, Id) {
    let mut d = d.clone();
    let id = d.add_feature(f);
    (d, id)
}

#[test]
fn filete_e_chanfro_por_qualquer_ponto_da_aresta_circular() {
    if !kernel_disponivel() {
        return;
    }
    let d = cilindro();
    let r0 = recompute(&d);
    let b = &r0.bodies[0];
    let e = (0..b.mesh.edge_mids.len() / 3).find(|e| (b.mesh.edge_mids[e * 3 + 2] as f64 - 20.).abs() < 1e-3).expect("aresta do topo");
    let medio = ponto_medio(b, e);
    let a0 = medio[1].atan2(medio[0]);
    let (_, ref_medio) = {
        let (dd, id) = com(&d, filete("1", vec![medio], vec![None]));
        let r = recompute(&dd);
        assert_eq!(r.status[&id], Status::Ok);
        (id, volume_total(&r))
    };
    // a 90°, a 180° (em cima da costura) e a 30° do médio: todos sobre o círculo do topo
    for graus in [90., 180., 30.] {
        let a = a0 + f64::to_radians(graus);
        let p = [10. * a.cos(), 10. * a.sin(), 20.];
        let (dd, id) = com(&d, filete("1", vec![p], vec![None]));
        let r = recompute(&dd);
        assert_eq!(r.status[&id], Status::Ok, "filete a {graus}°: {:?}", r.status[&id]);
        assert!((volume_total(&r) - ref_medio).abs() < 1e-6 * ref_medio, "filete a {graus}° deve dar o mesmo sólido do médio");
        // o comando ganha a chave da aresta, para seguir a aresta quando a peça mudar
        assert!(r.promoted.contains_key(&id), "a aresta escolhida pelo ponto vira chave");
        let (dd, id) = com(&d, chanfro("1", vec![p], vec![None]));
        let r = recompute(&dd);
        assert_eq!(r.status[&id], Status::Ok, "chanfro a {graus}°: {:?}", r.status[&id]);
        assert!(volume_total(&r) < volume_total(&r0));
    }
}

#[test]
fn ponto_no_vertice_entre_arestas_e_ambiguo_e_ponto_fora_e_perdido() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("caixa");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [20., 10.]);
    d.add_feature(ext(sk, BoolOp::Add, "5"));
    // canto (20, 10, 5): três arestas passam por ele
    let (dd, id) = com(&d, filete("1", vec![[20., 10., 5.]], vec![None]));
    let r = recompute(&dd);
    match &r.status[&id] {
        Status::Error(m) => assert!(m.contains("candidatas") && m.contains("meio da aresta"), "{m}"),
        outro => panic!("vértice não pode escolher aresta: {outro:?}"),
    }
    // um ponto qualquer da aresta de cima, longe do médio (10, 10, 5)
    let (dd, id) = com(&d, filete("1", vec![[3., 10., 5.]], vec![None]));
    let r = recompute(&dd);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    // 0,5 mm fora da aresta: nunca a vizinha
    let (dd, id) = com(&d, filete("1", vec![[3., 10.5, 5.]], vec![None]));
    let r = recompute(&dd);
    match &r.status[&id] {
        Status::Error(m) => assert!(m.contains("referência perdida") && m.contains("ponto médio em"), "{m}"),
        outro => panic!("{outro:?}"),
    }
}
