// SPDX-License-Identifier: LGPL-2.1-or-later
//! B15 (fase 2): papéis de face na varredura, no loft, na extrusão até plano e nos furos, filete
//! "criado pelo comando" pelos papéis e candidatas do assistente de reparo.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, reparo, BodyResult, Promoted, RecomputeResult, Status};

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

fn corpo(r: &RecomputeResult) -> &BodyResult {
    r.bodies.iter().find(|b| !b.face_owner.is_empty()).expect("um corpo")
}

/// Papéis das faces do comando `id` no corpo, em ordem.
fn papeis_de(r: &RecomputeResult, id: Id) -> Vec<i32> {
    let b = corpo(r);
    let mut v: Vec<i32> = b.face_owner.iter().zip(&b.face_role).filter(|(o, _)| **o == id).map(|(_, r)| *r).collect();
    v.sort_unstable();
    v
}

fn chave_da_face(r: &RecomputeResult, owner: Id, role: i32) -> FaceKey {
    let b = corpo(r);
    let j = (0..b.face_owner.len()).find(|j| b.face_owner[*j] == owner && b.face_role[*j] == role).expect("face com o papel");
    b.face_key(j).expect("chave")
}

fn casca(face: [f64; 3], key: Option<FaceKey>) -> FeatureKind {
    FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![face], thickness: "1".into(), outward: false, face_keys: vec![key], closed: false, regras: vec![] })
}

/// Perfil quadrado 10 × 10 no XY varrido pela linha de (0, 0, 0) até (0, 0, `h`).
fn varredura(h: f64) -> (Document, Id, Id) {
    let mut d = Document::new("varredura");
    let sk = sk_rect(&mut d, XY, [-5., -5.], [5., 5.]);
    let ln = d.add_geometry(RefKind::LinePoints { a: PointRef::Origin, b: PointRef::Fixed([0., 0., h]), start: "0".into(), end: "0".into() });
    let sw = d.add_feature(FeatureKind::Sweep(Sweep { sketch: sk, spine: CurveRef::Ref(ln), mode: SweepMode::Corrigido, op: BoolOp::Add, thin: None }));
    (d, ln, sw)
}

#[test]
fn varredura_tem_tampas_e_laterais_e_a_casca_segue_a_tampa_final() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, ln, sw) = varredura(30.);
    let r = recompute(&d);
    assert_eq!(r.status[&sw], Status::Ok, "{:?}", r.status[&sw]);
    assert_eq!(papeis_de(&r, sw), vec![0, 1, 2, 3, 4, 5], "antes ficava tudo -1");
    let topo = chave_da_face(&r, sw, 1);
    // a guia cresce 10 mm: o ponto antigo do topo fica no ar, a chave acha a tampa nova
    d.add_feature(casca([0., 0., 30.], Some(topo)));
    if let Some(g) = d.references.iter_mut().find(|g| g.id == ln) {
        g.kind = RefKind::LinePoints { a: PointRef::Origin, b: PointRef::Fixed([0., 0., 40.]), start: "0".into(), end: "0".into() };
    }
    let r = recompute(&d);
    assert!(problema_de_status(&d, &r).is_none(), "{:?}", problema_de_status(&d, &r));
    // caixa aberta no topo: 10 × 10 × 40 menos o miolo 8 × 8 × 39
    let v = volume_total(&r);
    assert!((v - (4000. - 64. * 39.)).abs() < 1e-2, "{v}");
    let (lo, hi) = caixa_da_malha(&r);
    assert!((hi[2] - lo[2] - 40.).abs() < 1e-3);
}

#[test]
fn loft_e_extrusao_ate_plano_tem_papeis() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("loft");
    let a = sk_rect(&mut d, XY, [-10., -10.], [10., 10.]);
    let pl = d.add_offset_plane(XY, "20");
    let b = sk_rect(&mut d, PlaneRef::Datum(pl), [-5., -5.], [5., 5.]);
    let lf = d.add_feature(FeatureKind::Loft(Loft { sections: vec![a, b], ruled: true, closed: false, op: BoolOp::Add, torcao: vec![], inicio: Default::default(), fim: Default::default() }));
    let r = recompute(&d);
    assert_eq!(r.status[&lf], Status::Ok);
    assert_eq!(papeis_de(&r, lf), vec![0, 1, 2, 3, 4, 5]);
    // a tampa final do loft é o quadrado de cima (área 100)
    assert!((chave_da_face(&r, lf, 1).area - 100.).abs() < 1e-6);

    let mut d = Document::new("ate plano");
    let sk = sk_rect(&mut d, XY, [-10., -10.], [10., 10.]);
    let pl = d.add_offset_plane(XY, "25");
    let ex = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::UpTo(PlaneRef::Datum(pl)), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let r = recompute(&d);
    assert_eq!(r.status[&ex], Status::Ok);
    assert_eq!(papeis_de(&r, ex), vec![0, 1, 2, 3, 4, 5]);
    assert!((volume_total(&r) - 400. * 25.).abs() < 1e-3);
}

#[test]
fn chanfro_na_boca_do_furo_segue_o_furo_pela_chave() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("furo");
    let base = sk_rect(&mut d, XY, [-20., -20.], [20., 20.]);
    let ex = d.add_feature(ext(base, BoolOp::Add, "10"));
    let pl = d.add_offset_plane(XY, "10");
    let pts = sk_pontos(&mut d, PlaneRef::Datum(pl), &[[0., 0.]]);
    let fu = d.add_feature(furo_passante(pts, "8"));
    let r = recompute(&d);
    assert_eq!(r.status[&fu], Status::Ok, "{:?}", r.status[&fu]);
    assert_eq!(papeis_de(&r, fu), vec![fenkra_solidos::papeis::PAPEL_FURO], "a parede do furo passante");
    // a aresta da boca: entre o topo da base e a parede do furo
    let b = corpo(&r);
    let e = aresta_perto(b, [4., 0., 10.]);
    let key = b.edge_key(e).expect("boca com chave");
    assert!(key.a.owner == ex || key.b.owner == ex);
    let ch = d.add_feature(chanfro("1", vec![ponto_medio(b, e)], vec![Some(key)]));
    assert_eq!(recompute(&d).status[&ch], Status::Ok);
    // o furo anda 6 mm: o ponto antigo fica longe, a chave acha a boca nova
    if let Some(FeatureKind::Sketch(s)) = d.feature_mut(pts).map(|f| &mut f.kind) {
        for p in s.data.points.iter_mut() {
            p.x = 6.;
        }
    }
    let r = recompute(&d);
    assert_eq!(r.status[&ch], Status::Ok, "{:?}", r.status[&ch]);
    let furo_e_chanfro = 1600. * 10. - std::f64::consts::PI * 16. * 10.;
    assert!(volume_total(&r) < furo_e_chanfro - 1., "o chanfro tirou material");
}

#[test]
fn filete_criado_pela_varredura_pega_so_as_arestas_dela() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("criado por");
    let base = sk_rect(&mut d, XY, [-20., -20.], [20., 20.]);
    d.add_feature(ext(base, BoolOp::Add, "5"));
    let pl = d.add_offset_plane(XY, "5");
    let sk = sk_rect(&mut d, PlaneRef::Datum(pl), [-5., -5.], [5., 5.]);
    let ln = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 5.]), b: PointRef::Fixed([0., 0., 25.]), start: "0".into(), end: "0".into() });
    let sw = d.add_feature(FeatureKind::Sweep(Sweep { sketch: sk, spine: CurveRef::Ref(ln), mode: SweepMode::Corrigido, op: BoolOp::Add, thin: None }));
    let fil = d.add_feature(FeatureKind::Fillet(EdgeOp { value: "1".into(), edges: EdgeSel::CreatedBy(sw), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
    let r = recompute(&d);
    assert_eq!(r.status[&fil], Status::Ok, "{:?}", r.status[&fil]);
    // 4 arestas do topo e 4 verticais (a base do pino encosta na placa e não é dele)
    let v0 = 1600. * 5. + 100. * 20.;
    let tirado = v0 - volume_total(&r);
    let por_metro = 1. - std::f64::consts::PI / 4.; // seção tirada por mm de aresta convexa com raio 1
    let esperado = por_metro * (4. * 10. + 4. * 20.);
    assert!((tirado - esperado).abs() < 0.2 * esperado, "tirou {tirado}, esperado ~{esperado}");
}

#[test]
fn assistente_de_reparo_mostra_candidatas_e_aplica_a_escolhida() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("reparo");
    let sk = sk_rect(&mut d, XY, [-20., -15.], [20., 15.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    // ponto antigo, sem chave, 3 mm acima do topo: referência perdida
    let sh = d.add_feature(casca([0., 0., 23.], None));
    let r = recompute(&d);
    assert!(matches!(&r.status[&sh], Status::Error(m) if m.contains("perdida")), "{:?}", r.status[&sh]);
    let f = d.features.iter().find(|f| f.id == sh).unwrap().clone();
    let lac = reparo::lacunas(&f, &r.bodies);
    assert_eq!(lac.len(), 1);
    assert_eq!(lac[0].alvo, reparo::Alvo::Face);
    assert!(!lac[0].candidatas.is_empty() && lac[0].candidatas.len() <= reparo::MAX_CANDIDATAS);
    // a mais provável é o topo (a face mais perto do último ponto)
    assert!((lac[0].candidatas[0][2] - 20.).abs() < 1e-6, "{:?}", lac[0].candidatas);
    let k = &mut d.feature_mut(sh).unwrap().kind;
    assert!(reparo::aplicar(k, lac[0].alvo, lac[0].indice, lac[0].candidatas[0]));
    let r = recompute(&d);
    assert_eq!(r.status[&sh], Status::Ok, "{:?}", r.status[&sh]);
    assert!(matches!(r.promoted.get(&sh), Some(Promoted::Faces(k)) if k[0].is_some()), "a chave nova vem no cálculo seguinte");
    let v = volume_total(&r);
    assert!((v - (40. * 30. * 20. - 38. * 28. * 19.)).abs() < 1e-2, "{v}");
}

#[test]
fn centro_fixo_do_furo_vira_ponto_na_face_e_acompanha_o_topo() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("furo fixo");
    let base = sk_rect(&mut d, XY, [-20., -20.], [20., 20.]);
    let ex = d.add_feature(ext(base, BoolOp::Add, "10"));
    let mut h = match furo_passante(Id(0), "6") {
        FeatureKind::Hole(h) => h,
        _ => unreachable!(),
    };
    h.points = vec![PointRef::Fixed([16., 5., 10.])];
    h.depth = Extent::Blind("6".into());
    let fu = d.add_feature(FeatureKind::Hole(h));
    let r = recompute(&d);
    assert_eq!(r.status[&fu], Status::Ok, "{:?}", r.status[&fu]);
    let Some(Promoted::Furo { pontos, dir }) = r.promoted.get(&fu).cloned() else { panic!("sem promoção: {:?}", r.promoted.get(&fu)) };
    assert!(dir.is_none());
    let PointRef::OnFace { key, .. } = &pontos[0] else { panic!("{:?}", pontos[0]) };
    assert_eq!((key.owner, key.role), (ex, 1), "a tampa final da extrusão");
    // altura do fundo do furo (papel "fundo" do furo 1)
    let fundo = |r: &RecomputeResult| {
        let b = corpo(r);
        let j = (0..b.face_owner.len()).find(|j| b.face_owner[*j] == fu && b.face_role[*j] == fenkra_solidos::papeis::PAPEL_FURO + 5).expect("fundo do furo");
        b.mesh.face_info[j].point[2]
    };
    assert!((fundo(&r) - 4.).abs() < 1e-6);
    // sem a promoção, a base crescer 5 mm deixa o centro mais perto da lateral x = 20 (4 mm) que do
    // topo novo (5 mm): o ponto fixo cai na lateral e o furo sai deitado
    let cresce = |d: &mut Document| {
        if let Some(FeatureKind::Extrude(x)) = d.feature_mut(ex).map(|f| &mut f.kind) {
            x.extent = Extent::Blind("15".into());
        }
    };
    let mut antigo = d.clone();
    cresce(&mut antigo);
    let z = fundo(&recompute(&antigo));
    assert!((z - 9.).abs() > 0.5, "o centro fixo caía na lateral (fundo em z = {z})");
    if let Some(FeatureKind::Hole(h)) = d.feature_mut(fu).map(|f| &mut f.kind) {
        h.points = pontos;
    }
    cresce(&mut d);
    let r = recompute(&d);
    assert_eq!(r.status[&fu], Status::Ok, "{:?}", r.status[&fu]);
    assert!((fundo(&r) - 9.).abs() < 1e-6, "o furo acompanha o topo: fundo em {}", fundo(&r));
    assert!((volume_total(&r) - (1600. * 15. - std::f64::consts::PI * 9. * 6.)).abs() < 1e-2, "{}", volume_total(&r));
    assert!(r.promoted.get(&fu).is_none(), "nada mais a promover");
}
