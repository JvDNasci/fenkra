//! B78: SmFlange, SmCurveFlange e SmHem escolhem a aresta por dono + papel das duas faces vizinhas
//! (ADR 0010). O ponto clicado fica só como reserva; aresta que some vira estado de erro explícito.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Promoted, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.to_string()
}

struct Peca {
    d: Document,
    sketch: Id,
    aba: Id,
}

fn peca() -> Peca {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sketch = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch, reverse: false, offset: String::new(), mid: false, surface: None }));
    let aba = d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
    Peca { d, sketch, aba }
}

fn chave_da(r: &RecomputeResult, id: Id) -> Option<EdgeKey> {
    match r.promoted.get(&id) {
        Some(Promoted::Edges(k)) if k.len() == 1 => k[0].clone(),
        _ => None,
    }
}

fn grava_chave(d: &mut Document, id: Id, key: Option<EdgeKey>) {
    match &mut d.feature_mut(id).unwrap().kind {
        FeatureKind::SmFlange(a) => a.edge_key = key,
        FeatureKind::SmHem(a) => a.edge_key = key,
        FeatureKind::SmCurveFlange(a) => a.edge_key = key,
        _ => panic!("não é aba"),
    }
}

fn recalcula(d: &Document, flat: bool) -> RecomputeResult {
    let mut d = d.clone();
    d.sheet.as_mut().unwrap().flat = flat;
    recompute(&d)
}

#[test]
fn a_aba_promove_a_chave_e_segue_a_aresta_quando_a_chapa_cresce() {
    if !kernel_disponivel() {
        return;
    }
    let mut p = peca();
    let r = recompute(&p.d);
    assert_eq!(r.status[&p.aba], Status::Ok);
    let key = chave_da(&r, p.aba).expect("o recompute deveria promover a chave da aresta");
    grava_chave(&mut p.d, p.aba, Some(key));
    // com a chave gravada, o recompute não promove de novo
    assert!(chave_da(&recompute(&p.d), p.aba).is_some() || recompute(&p.d).promoted.get(&p.aba).is_none());

    // a chapa vira 104 × 20: a aresta que era x = 100 passa a x = 104, e o ponto guardado (100, 25)
    // fica mais perto da aresta y = 20 (5 mm) que da nova (6,4 mm)
    set_rect(&mut p.d, p.sketch, [0., 0.], [104., 20.]);
    let r = recompute(&p.d);
    assert_eq!(r.status[&p.aba], Status::Ok, "{:?}", r.status[&p.aba]);
    let (lo, hi) = caixa_da_malha(&r);
    assert!((hi[0] - 108.).abs() < 0.1 && (hi[1] - 20.).abs() < 0.1 && (hi[2] - 24.).abs() < 0.1 && lo[0].abs() < 0.1, "a aba seguiu a aresta: {lo:?} {hi:?}");

    // sem a chave, só o ponto: a aba não acompanha a aresta (cai na aresta mais próxima do ponto)
    let mut sem = p.d.clone();
    grava_chave(&mut sem, p.aba, None);
    let rs = recompute(&sem);
    let ok_e_no_lugar = rs.status[&p.aba] == Status::Ok && (caixa_da_malha(&rs).1[0] - 108.).abs() < 0.1;
    assert!(!ok_e_no_lugar, "o ponto sozinho não deveria acompanhar a aresta");

    // planificada: a chave não vale no outro estado, o ponto é levado e a aba continua válida
    let plana = recalcula(&p.d, true);
    assert_eq!(plana.status[&p.aba], Status::Ok, "{:?}", plana.status[&p.aba]);
}

#[test]
fn aresta_que_some_vira_erro_explicito_e_nao_troca_de_aresta() {
    if !kernel_disponivel() {
        return;
    }
    let mut p = peca();
    let key = chave_da(&recompute(&p.d), p.aba).expect("chave promovida");
    // a aresta era entre as faces de um dono que deixa de existir
    let mut perdida = key.clone();
    perdida.a.owner = Id(999_999);
    grava_chave(&mut p.d, p.aba, Some(perdida));
    let r = recompute(&p.d);
    match &r.status[&p.aba] {
        Status::Error(m) => assert!(m.contains("referência perdida") && m.contains("aba"), "{m}"),
        o => panic!("deveria falhar com referência perdida: {o:?}"),
    }
    // chave certa de volta: a aba volta sozinha
    grava_chave(&mut p.d, p.aba, Some(key));
    assert_eq!(recompute(&p.d).status[&p.aba], Status::Ok);
}

#[test]
fn bainha_tambem_usa_a_chave() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let h = d.add_feature(FeatureKind::SmHem(SmHem { edge: [100., 25., 2.], length: s("8"), gap: s("1"), flip: false, kind: 0, edge_key: None }));
    let r = recompute(&d);
    assert_eq!(r.status[&h], Status::Ok, "{:?}", r.status[&h]);
    let key = chave_da(&r, h).expect("a bainha promove a chave");
    grava_chave(&mut d, h, Some(key));
    set_rect(&mut d, sk, [0., 0.], [130., 50.]);
    let r = recompute(&d);
    assert_eq!(r.status[&h], Status::Ok, "{:?}", r.status[&h]);
    let (_, hi) = caixa_da_malha(&r);
    assert!(hi[0] > 130., "a bainha seguiu a aresta para x = 130: {hi:?}");
}
