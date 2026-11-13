// SPDX-License-Identifier: LGPL-2.1-or-later
//! B15 (fase 2): rosca, filete face-face e filete completo guardam a face por dono + papel, com o ponto
//! clicado como reserva. Arquivo antigo (só o ponto) é promovido no primeiro cálculo; depois a face é
//! achada pela chave mesmo quando a geometria anterior muda de lugar, e o ponto sozinho vira
//! "referência perdida" em vez de pegar outra face em silêncio.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Promoted, Status};

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

fn faces_promovidas(r: &fenkra_solidos::RecomputeResult, id: Id) -> Vec<Option<FaceKey>> {
    match r.promoted.get(&id) {
        Some(Promoted::Faces(k)) => k.clone(),
        outro => panic!("sem promoção de faces: {outro:?} / {:?}", r.status[&id]),
    }
}

fn perdida(st: &Status) -> bool {
    matches!(st, Status::Error(m) if m.contains("perdida"))
}

/// Cilindro de raio 5 e altura 30 com a rosca na face lateral, clicada em (5, 0, 15).
fn cilindro_roscado() -> (Document, Id, Id) {
    let mut d = Document::new("rosca");
    let sk = sk_circulos(&mut d, XY, &[([0., 0.], 5.)]);
    d.add_feature(ext(sk, BoolOp::Add, "30"));
    let th = d.add_feature(FeatureKind::Thread(Thread { face: [5., 0., 15.], spec: "M10x1.5".into(), length: "12".into(), modeled: false, end: None, start: String::new(), face_key: None }));
    (d, sk, th)
}

#[test]
fn rosca_segue_a_face_pela_chave_quando_o_cilindro_muda_de_lugar() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, sk, th) = cilindro_roscado();
    let r = recompute(&d);
    assert_eq!(r.status[&th], Status::Ok, "{:?}", r.status[&th]);
    let k = faces_promovidas(&r, th);
    assert_eq!(k.len(), 1);
    let key = k[0].expect("a face lateral da extrusão tem dono e papel");
    assert_eq!(key.role, 2, "lateral gerada pela aresta 0 do perfil");
    // sem chave (arquivo antigo), o cilindro andar 8 mm deixa o ponto a 2 mm da face: referência perdida
    set_circulo(&mut d, sk, [8., 0.], 5.);
    let st = recompute(&d).status[&th].clone();
    assert!(perdida(&st), "{st:?}");
    // com a chave, a rosca acompanha a face e fica no eixo novo
    if let Some(FeatureKind::Thread(t)) = d.feature_mut(th).map(|f| &mut f.kind) {
        t.face_key = Some(key);
    }
    let r = recompute(&d);
    assert_eq!(r.status[&th], Status::Ok, "{:?}", r.status[&th]);
    let rosca = r.roscas.iter().find(|x| x.feature == th).expect("rosca desenhada");
    assert!((rosca.origin[0] - 8.).abs() < 1e-6 && rosca.origin[1].abs() < 1e-6, "eixo da rosca em {:?}", rosca.origin);
    assert!((rosca.radius - 5.).abs() < 1e-6);
    // o documento continua válido: volume do cilindro inteiro
    assert!((volume_total(&r) - std::f64::consts::PI * 25. * 30.).abs() < 1e-3, "{}", volume_total(&r));
}

/// Caixa 40 × 30 × 20 com filete face-face entre o topo e a lateral y = 15.
fn caixa_face_face(mode: impl Fn([f64; 3], [f64; 3], [f64; 3]) -> EdgeMode) -> (Document, Id, Id) {
    let mut d = Document::new("face-face");
    let sk = sk_rect(&mut d, XY, [-20., -15.], [20., 15.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let op = EdgeOp { value: "3".into(), edges: EdgeSel::AllOfBody, mode: mode([0., 0., 20.], [0., 15., 10.], [0., -15., 10.]), prop: Default::default(), keys: vec![], extra: Default::default() };
    let id = d.add_feature(FeatureKind::Fillet(op));
    (d, sk, id)
}

fn poe_chaves(d: &mut Document, id: Id, k: Vec<Option<FaceKey>>) {
    if let Some(FeatureKind::Fillet(o)) = d.feature_mut(id).map(|f| &mut f.kind) {
        o.extra.face_keys = k;
    }
}

#[test]
fn filete_face_face_segue_as_faces_pela_chave() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, sk, id) = caixa_face_face(|a, b, _| EdgeMode::FaceFace(a, b));
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    let k = faces_promovidas(&r, id);
    assert_eq!(k.len(), 2);
    assert!(k.iter().all(Option::is_some));
    // filete r = 3 numa aresta de 40: tira (1 - π/4) · 9 · 40
    let tirado = (1. - std::f64::consts::PI / 4.) * 9. * 40.;
    assert!((volume_total(&r) - (24000. - tirado)).abs() < 0.05, "{}", volume_total(&r));
    // a caixa engorda para y = ±25: a lateral sai de y = 15 e o ponto antigo fica a 10 mm dela
    set_rect(&mut d, sk, [-20., -25.], [20., 25.]);
    assert!(perdida(&recompute(&d).status[&id]));
    poe_chaves(&mut d, id, k);
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    assert!((volume_total(&r) - (40000. - tirado)).abs() < 0.05, "o filete segue na aresta nova: {}", volume_total(&r));
    let (lo, hi) = caixa_envolvente(&d);
    assert!((hi[1] - lo[1] - 50.).abs() < 1e-6 && (hi[2] - lo[2] - 20.).abs() < 1e-6, "{lo:?} {hi:?}");
}

#[test]
fn filete_completo_promove_as_tres_faces_e_as_segue() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, sk, id) = caixa_face_face(|a, b, c| EdgeMode::FullRound { mid: a, a: b, b: c });
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    let k = faces_promovidas(&r, id);
    assert_eq!(k.len(), 3);
    assert!(k.iter().all(Option::is_some));
    let antes = volume_total(&r);
    // a peça fica mais alta (o topo vai de z = 20 a 26): o ponto do topo fica a 6 mm dele
    if let Some(FeatureKind::Extrude(e)) = d.features.iter_mut().find(|f| matches!(f.kind, FeatureKind::Extrude(_))).map(|f| &mut f.kind) {
        e.extent = Extent::Blind("26".into());
    }
    assert!(perdida(&recompute(&d).status[&id]));
    poe_chaves(&mut d, id, k);
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    // o arredondamento completo é o mesmo meio-cilindro em cima de uma caixa 6 mm mais alta
    assert!((volume_total(&r) - antes - 40. * 30. * 6.).abs() < 0.05, "{} {antes}", volume_total(&r));
    let _ = sk;
}

/// Arquivo antigo, sem os campos novos: abre igual (chaves vazias).
#[test]
fn arquivo_antigo_sem_chaves_abre() {
    let (d, _, th) = cilindro_roscado();
    let mut j: serde_json::Value = serde_json::from_str(&d.to_json()).unwrap();
    assert!(j.to_string().contains("\"face\""));
    assert!(!j.to_string().contains("face_key"), "chave vazia não é gravada");
    j["format_version"] = 3.into();
    let d2 = Document::from_json(&j.to_string()).unwrap();
    assert!(matches!(&d2.feature(th).unwrap().kind, FeatureKind::Thread(t) if t.face_key.is_none()));
}
