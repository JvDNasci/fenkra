// SPDX-License-Identifier: LGPL-2.1-or-later
//! B534: Extrair guarda a face pela identidade (dono + papel + assinatura, ADR 0010), com o ponto só de
//! reserva. Aceita face de sólido, de superfície de referência e de forma livre; a propagação tangente
//! junta as faces do filete; a extração sobrevive à mudança de parâmetro do recurso de origem, inclusive
//! com o cache do app; arquivo antigo, só com o ponto, continua abrindo e ganha a chave.

mod common;

use common::*;
use fenkra_core::subd::Gaiola;
use fenkra_core::*;
use fenkra_solidos::{recompute, recompute_cached, regioes::area_da_malha, Cache, Promoted, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.into()
}

fn ok(r: &RecomputeResult, d: &Document, id: Id) {
    assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{}: {:?}", d.name_of(id), r.status.get(&id));
}

/// Área da superfície `id` como o resultado a mostra (malha da referência ou da forma).
fn area(r: &RecomputeResult, id: Id) -> f64 {
    area_da_malha(&r.surfaces.iter().find(|x| x.id == id).unwrap_or_else(|| panic!("superfície {id:?} no resultado")).mesh)
}

/// Faces que a extração alcançou.
fn faces(r: &RecomputeResult, id: Id) -> u32 {
    r.notes[&id].count.expect("contagem").0
}

fn extrair(d: &mut Document, face: [f64; 3], owner: Option<Id>, key: Option<FaceKey>, prop: Propagation) -> Id {
    d.add_geometry(RefKind::SurfExtract { face, owner, prop, complement: false, expected: None, key })
}

// ------------------------------------------------------------------ forma livre

fn folha() -> Gaiola {
    let mut g = Gaiola::plano([0., 0., 30.], [100., 100.], [3, 3]);
    for n in &mut g.nos {
        n.p[2] += (n.p[0] * 0.03).sin() * 6.;
    }
    g
}

fn forma(g: Gaiola) -> FeatureKind {
    FeatureKind::Forma(Forma { gaiola: g, niveis: 3, congelada: false, conversao: Some(ConversaoForma { continuidade: 1, tol: s("0.01"), pausada: false }), vinculos: Default::default() })
}

#[test]
fn extrair_face_de_forma_livre_por_chave_e_por_ponto() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("b534");
    let fm = d.add_feature(forma(folha()));
    let r = recompute(&d);
    let sv = r.surfaces.iter().find(|x| x.id == fm).expect("a forma é superfície");
    let (p, key) = (sv.mesh.face_info[0].point, sv.face_key(0).expect("chave da face da forma"));
    let area_forma = area(&r, fm);
    // como o comando grava agora: dona = a forma, chave = o índice da face
    let ex = extrair(&mut d, p, Some(fm), Some(key), Propagation::Nenhuma);
    let tan = extrair(&mut d, p, Some(fm), Some(key), Propagation::Tangente);
    // arquivo antigo: só o ponto
    let velho = extrair(&mut d, p, None, None, Propagation::Nenhuma);
    let r = recompute(&d);
    for x in [ex, tan, velho] {
        ok(&r, &d, x);
    }
    // a forma convertida tem várias faces, lisas entre si (continuidade G1): uma só é um pedaço dela, e a
    // propagação tangente junta a forma inteira
    let a = area(&r, ex);
    assert_eq!(faces(&r, ex), 1);
    assert!(a > 0. && a < area_forma, "área extraída {a} contra a da forma {area_forma}");
    assert!(faces(&r, tan) > 1, "faces da tangente {}", faces(&r, tan));
    assert!((area(&r, tan) - area_forma).abs() / area_forma < 0.02, "tangente {} contra {area_forma}", area(&r, tan));
    assert!((area(&r, velho) - a).abs() < 1e-6 * a.max(1.));
    // o ponto sozinho ganha a chave da forma para o app gravar
    match r.promoted.get(&velho) {
        Some(Promoted::Referencia(ChaveAncora::Face(k))) => assert_eq!((k.owner, k.role), (fm, PAPEL_SUPERFICIE)),
        x => panic!("promoção {x:?}"),
    }
}

// ------------------------------------------------------------------ bloco com filete

/// Bloco de 20 × 20 × `altura` com as quatro arestas de cima arredondadas (raio 3), cada aresta por
/// dono e papel. Devolve a extrusão.
fn bloco_com_filete(d: &mut Document, altura: &str) -> Id {
    let sk = sk_rect(d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [20., 20.]);
    let ex = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s(altura)), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let r = recompute(d);
    let b = &r.bodies[0];
    let z = altura.parse::<f64>().unwrap();
    let (mut mids, mut keys) = (vec![], vec![]);
    for (e, m) in b.mesh.edge_mids.chunks_exact(3).enumerate() {
        if (m[2] as f64 - z).abs() < 1e-3 {
            mids.push([m[0] as f64, m[1] as f64, m[2] as f64]);
            keys.push(Some(b.edge_key(e).expect("chave da aresta de cima")));
        }
    }
    assert_eq!(mids.len(), 4, "arestas de cima");
    d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("3"), edges: EdgeSel::Edges(mids), mode: Default::default(), prop: Default::default(), keys, extra: Default::default() }));
    ex
}

/// A face plana de cima: ponto e chave.
fn tampa(r: &RecomputeResult) -> ([f64; 3], FaceKey) {
    let b = &r.bodies[0];
    let f = (0..b.mesh.face_info.len()).find(|j| b.mesh.face_info[*j].kind == 0 && b.mesh.face_info[*j].dir[2] > 0.99).expect("tampa");
    (b.mesh.face_info[f].point, b.face_key(f).expect("papel da tampa"))
}

#[test]
fn propagacao_tangente_junta_as_faces_do_filete_e_segue_a_altura() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("b534");
    let cx = bloco_com_filete(&mut d, "30");
    let r = recompute(&d);
    let b = &r.bodies[0];
    let total: f64 = b.mesh.face_info.iter().map(|f| f.area).sum();
    let nfaces = b.mesh.face_info.len() as u32;
    let (p, key) = tampa(&r);
    let so = extrair(&mut d, p, Some(cx), Some(key), Propagation::Nenhuma);
    let tan = extrair(&mut d, p, Some(cx), Some(key), Propagation::Tangente);
    let mut cache = Cache::default();
    let r = recompute_cached(&d, &mut cache);
    for x in [so, tan] {
        ok(&r, &d, x);
    }
    // a tampa de cima ficou 14 × 14 (o filete come 3 mm de cada lado)
    assert!((area(&r, so) - 196.).abs() < 1., "tampa {}", area(&r, so));
    assert_eq!(faces(&r, so), 1);
    // tangente: a tampa, os quatro filetes e as paredes (lisas com o filete); só o fundo, de canto
    // vivo, fica de fora. A área é a soma dessas faces: o total menos o fundo de 20 × 20
    assert_eq!(faces(&r, tan), nfaces - 1, "faces da propagação");
    let a_tan = area(&r, tan);
    assert!((a_tan - (total - 400.)).abs() < 0.01 * total, "tangente {a_tan} contra {}", total - 400.);
    assert!(a_tan > area(&r, so) + 4. * 20. * 27., "inclui filetes e paredes");

    // a altura muda: a extração continua Ok, na mesma face (a tampa sobe para z = 50)
    let FeatureKind::Extrude(e) = &mut d.feature_mut(cx).unwrap().kind else { panic!() };
    e.extent = Extent::Blind(s("50"));
    let r = recompute_cached(&d, &mut cache);
    for x in [so, tan] {
        ok(&r, &d, x);
    }
    let zs = r.surfaces.iter().find(|x| x.id == so).unwrap().mesh.positions.chunks_exact(3).map(|q| q[2] as f64).fold(f64::MIN, f64::max);
    assert!((zs - 50.).abs() < 1e-3, "a tampa extraída está em z = {zs}");
    assert!((area(&r, so) - 196.).abs() < 1.);
    // as paredes cresceram 20 mm em volta (perímetro 80)
    assert!((area(&r, tan) - (a_tan + 1600.)).abs() < 0.01 * a_tan, "tangente depois {}", area(&r, tan));
}

// ------------------------------------------------------------------ superfície de referência

#[test]
fn extracao_de_superficie_de_referencia_refaz_quando_a_superficie_muda() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("b534");
    let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([20., 0., 0.]), start: s("0"), end: s("0") });
    let sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(l), dir: AxisRef::Origin(OriginAxis::Z), length: s("30"), length2: s("0"), until: None, until2: None, mais: vec![] });
    let r = recompute(&d);
    let sv = r.surfaces.iter().find(|x| x.id == sup).unwrap();
    let (p, key) = (sv.mesh.face_info[0].point, sv.face_key(0).expect("chave da face da superfície"));
    let ex = extrair(&mut d, p, Some(sup), Some(key), Propagation::Nenhuma);
    let velho = extrair(&mut d, p, None, None, Propagation::Nenhuma);
    let mut cache = Cache::default();
    let r = recompute_cached(&d, &mut cache);
    ok(&r, &d, ex);
    ok(&r, &d, velho);
    assert!((area(&r, ex) - 600.).abs() < 1e-3, "{}", area(&r, ex));
    assert!(matches!(r.promoted.get(&velho), Some(Promoted::Referencia(ChaveAncora::Face(k))) if k.owner == sup));
    // a superfície fica mais alta: com o mesmo cache, a extração pela chave refaz (o dono da chave
    // entra na assinatura) e segue a face
    let RefKind::SurfExtrude { length, .. } = &mut d.references.iter_mut().find(|x| x.id == sup).unwrap().kind else { panic!() };
    *length = s("50");
    let r = recompute_cached(&d, &mut cache);
    ok(&r, &d, ex);
    assert!((area(&r, ex) - 1000.).abs() < 1e-3, "a extração não acompanhou: {}", area(&r, ex));
}
