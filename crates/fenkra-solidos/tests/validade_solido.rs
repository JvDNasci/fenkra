// SPDX-License-Identifier: LGPL-2.1-or-later
//! B413 e K-10 do kart: sólido inválido não passa como "ok".

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

fn xy() -> PlaneRef {
    PlaneRef::Origin(OriginPlane::XY)
}

/// Aro do kart (RD-02): revolução, alma, furos em padrão circular e o pino do talão extrudado na
/// direção X, a partir de um sketch no plano XY.
fn aro(dir_pino: Option<AxisRef>) -> (Document, Id, Id) {
    let mut d = Document::new("aro");
    let l = 210.;
    let h = l / 2.;
    let perfil = sk_poligono(&mut d, xy(), &[[58., -h], [68.1, -h], [68.1, -h + 5.], [63.1, -h + 8.], [63.1, h - 8.], [68.1, h - 5.], [68.1, h], [58., h]]);
    d.add_feature(FeatureKind::Revolve(Revolve { sketch: perfil, axis: AxisRef::Origin(OriginAxis::Y), angle: "360".into(), symmetric: false, op: BoolOp::Add, thin: None, thin_side: 0, angle2: None }));
    let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XZ), "0");
    let alma = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XZ), &[([0., 0.], 59.), ([0., 0.], 27.)]);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: alma, op: BoolOp::Add, extent: Extent::Symmetric("8".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let sf = sk_pontos(&mut d, PlaneRef::Datum(pl), &[[42., 0.]]);
    let furo = d.add_feature(furo_passante(sf, "8.5"));
    d.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Y), count: "4".into(), angle: "360".into() }, sources: vec![furo], skip: vec![] }));
    let pp = d.add_offset_plane(xy(), "0");
    let sp = sk_circulos(&mut d, PlaneRef::Datum(pp), &[([63.1, -h + 12.], 1.5)]);
    let pino = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sp, op: BoolOp::Add, extent: Extent::Symmetric("6".into()), reverse: false, taper: "0".into(), thin: None, dir: dir_pino, thin_side: 0, ends: Default::default() }));
    let pad = d.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Y), count: "3".into(), angle: "360".into() }, sources: vec![pino], skip: vec![] }));
    (d, pino, pad)
}

#[test]
fn aro_do_kart_com_direcao_normal_ao_plano_segue_valido() {
    if !kernel_disponivel() {
        return;
    }
    let (d, pino, pad) = aro(None);
    let r = recompute(&d);
    for f in &d.features {
        assert_eq!(r.status[&f.id], Status::Ok, "{:?}", f.id);
    }
    assert_eq!(r.status[&pino], Status::Ok);
    assert_eq!(r.status[&pad], Status::Ok);
    // os pinos entram no volume: o aro sem eles tem 499.477 mm³ (RD-02A)
    assert!(volume_total(&r) > 499_477. + 30., "{}", volume_total(&r));
}

/// K-10, causa: o pino do kart era extrudado na direção X de um sketch no plano XY. A direção está
/// dentro do plano, o perfil varre a si mesmo e sobra um sólido sem espessura, inválido, que o status
/// "ok" não via. Os padrões de depois herdavam o defeito e o volume comum saía igual ao da peça inteira.
#[test]
fn extrusao_com_direcao_dentro_do_plano_do_sketch_e_erro_claro() {
    if !kernel_disponivel() {
        return;
    }
    let (d, pino, pad) = aro(Some(AxisRef::Origin(OriginAxis::X)));
    let r = recompute(&d);
    match &r.status[&pino] {
        Status::Error(m) => assert!(m.contains("plano do sketch"), "{m}"),
        outro => panic!("esperava erro, veio {outro:?}"),
    }
    // o padrão do pino não é refeito sobre um comando em erro
    assert!(matches!(r.status[&pad], Status::Error(_) | Status::Warning(_)), "{:?}", r.status[&pad]);
    // o aro continua o de antes do pino
    assert!((volume_total(&r) - 499_477.).abs() < 100., "{}", volume_total(&r));
}

/// K-13: filete por ponto perto de uma aresta circular, mas fora dela (0,5 mm para fora do círculo).
/// Nunca vale a aresta vizinha; o erro diz qual é o ponto médio da aresta mais próxima, e usar esse
/// ponto funciona. Um ponto SOBRE a aresta, longe do médio, vale desde o B391 (ver aresta_por_ponto).
#[test]
fn filete_em_aresta_circular_por_ponto_fora_do_medio_sugere_o_ponto_certo() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("cil");
    let sk = sk_circulos(&mut d, xy(), &[([0., 0.], 10.)]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let r = recompute(&d);
    let b = &r.bodies[0];
    // aresta circular do topo (z = 20): ponto médio do lado oposto da costura
    let e = (0..b.mesh.edge_mids.len() / 3).find(|e| (b.mesh.edge_mids[e * 3 + 2] as f64 - 20.).abs() < 1e-3).expect("aresta do topo");
    let medio = ponto_medio(b, e);
    // um ponto a 90° do médio, 0,5 mm para fora do círculo do topo
    let ang = medio[1].atan2(medio[0]) + std::f64::consts::FRAC_PI_2;
    let fora = [10.5 * ang.cos(), 10.5 * ang.sin(), 20.];
    let mut ruim = d.clone();
    ruim.add_feature(filete("1", vec![fora], vec![None]));
    let id = ruim.features.last().unwrap().id;
    let r2 = recompute(&ruim);
    match &r2.status[&id] {
        Status::Error(m) => assert!(m.contains("ponto médio em") && m.contains("a mais próxima"), "{m}"),
        outro => panic!("esperava erro com sugestão, veio {outro:?}"),
    }
    let mut bom = d.clone();
    bom.add_feature(filete("1", vec![medio], vec![None]));
    let id = bom.features.last().unwrap().id;
    let r3 = recompute(&bom);
    assert_eq!(r3.status[&id], Status::Ok, "{:?}", r3.status[&id]);
    assert!(volume_total(&r3) < volume_total(&r), "o filete tira material");
}
