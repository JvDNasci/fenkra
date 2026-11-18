// SPDX-License-Identifier: LGPL-2.1-or-later
//! B516: referências por dono, papel e índice em todas as bancadas. Cada teste monta a peça em código
//! como um arquivo antigo (só o ponto clicado), recalcula, grava as chaves promovidas como o app faz,
//! muda o perfil base e confere que o comando continua na mesma entidade: status, volume e caixa.

use super::*;
use fenkra_core::{Ancora, ChaveAncora};
use std::f64::consts::PI;

fn s(x: &str) -> String {
    x.to_string()
}

fn ok(r: &RecomputeResult, id: Id) -> bool {
    matches!(r.status.get(&id), Some(Status::Ok | Status::Warning(_)))
}

fn erro(r: &RecomputeResult, id: Id) -> String {
    match r.status.get(&id) {
        Some(Status::Error(e)) => e.clone(),
        outro => panic!("devia dar erro: {outro:?}"),
    }
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn sk_data(a: [f64; 2], b: [f64; 2], base: u64) -> SketchData {
    let mut data = SketchData::default();
    let mut n = base;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, a, b, false);
    data
}

fn sk_rect(d: &mut Document, plane: PlaneRef, a: [f64; 2], b: [f64; 2]) -> Id {
    let base = 5_160_000 + d.features.len() as u64 * 1000;
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data: sk_data(a, b, base) }))
}

/// Troca o retângulo do sketch (o "perfil base" muda de cota).
fn set_rect(d: &mut Document, sk: Id, a: [f64; 2], b: [f64; 2]) {
    let base = 5_170_000 + sk.0 * 1000;
    if let Some(FeatureKind::Sketch(x)) = d.feature_mut(sk).map(|f| &mut f.kind) {
        x.data = sk_data(a, b, base);
    }
}

fn ext(sketch: Id, d: &str) -> FeatureKind {
    FeatureKind::Extrude(Extrude { sketch, op: BoolOp::Add, extent: Extent::Blind(d.into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() })
}

fn caixa(d: &Document) -> ([f64; 3], [f64; 3]) {
    final_shape(d).expect("corpo").bbox().expect("caixa")
}

fn volume(r: &RecomputeResult) -> f64 {
    r.bodies.iter().map(|b| b.mass.volume).sum()
}

/// O que o app faz com `RecomputeResult::promoted` (`App::apply_promoted`): grava cada chave nova no
/// comando ou na referência. Devolve quantos itens ganharam chave.
fn gravar(d: &mut Document, r: &RecomputeResult) -> usize {
    let mut n = 0;
    for (id, p) in &r.promoted {
        if let Some(f) = d.feature_mut(*id) {
            n += 1;
            if let Promoted::Ancoras(a) = p {
                f.guardar_ancoras(a);
                continue;
            }
            match (&mut f.kind, p) {
                (FeatureKind::Fillet(o) | FeatureKind::Chamfer(o), Promoted::Edges(k)) => o.keys = k.clone(),
                (FeatureKind::SmFlange(a), Promoted::Edges(k)) => a.edge_key = k[0],
                (FeatureKind::SmHem(a), Promoted::Edges(k)) => a.edge_key = k[0],
                (FeatureKind::Hole(h), Promoted::Ate(k)) => {
                    if let Extent::UpToSurface(SurfRef::Face { key, .. }) = &mut h.depth {
                        *key = Some(*k);
                    }
                }
                (FeatureKind::StructMember(m), Promoted::Caminhos(c)) => m.paths = c.clone(),
                (k, p) => panic!("promoção sem destino no teste: {k:?} {p:?}"),
            }
        } else if let (Some(rf), Promoted::Referencia(ChaveAncora::Face(k))) = (d.references.iter_mut().find(|x| x.id == *id), p) {
            n += 1;
            if let RefKind::FacePlane { key, .. } = &mut rf.kind {
                *key = Some(*k);
            }
        }
    }
    n
}

fn meio(b: &BodyResult, e: usize) -> [f64; 3] {
    [0, 1, 2].map(|i| b.mesh.edge_mids[e * 3 + i] as f64)
}

// ------------------------------------------------------------------ sólidos

#[test]
fn filete_de_arquivo_antigo_e_promovido_e_segue_a_aresta_quando_o_perfil_base_cresce() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b516 filete");
    let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [40., 30.]);
    d.add_feature(ext(base, "20"));
    // arquivo antigo: o filete só tem o ponto médio da aresta de cima em x = 40
    let fil = d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("2"), edges: EdgeSel::Edges(vec![[40., 15., 20.]]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
    // gravado e lido de novo, como um arquivo de antes da chave
    let mut d = Document::from_json(&d.to_json()).unwrap();
    let r0 = recompute(&d);
    assert!(ok(&r0, fil), "{:?}", r0.status.get(&fil));
    assert_eq!(gravar(&mut d, &r0), 1, "o ponto antigo vira chave no primeiro cálculo");
    let FeatureKind::Fillet(o) = &d.feature(fil).unwrap().kind else { unreachable!() };
    let k = o.keys[0].expect("chave");
    let nome = refs::nome_aresta(&d, &k, &|x| x.to_string());
    assert!(nome.starts_with(&format!("{} · aresta ", d.name_of(k.a.owner))) && nome.ends_with(" · tampa final"), "o nome é dono, papel e índice: {nome}");

    // o perfil base cresce 20 mm em x: a aresta vai para x = 60
    set_rect(&mut d, base, [0., 0.], [60., 30.]);
    let r1 = recompute(&d);
    assert!(ok(&r1, fil), "{:?}", r1.status.get(&fil));
    let esperado = 60. * 30. * 20. - (1. - PI / 4.) * 4. * 30.;
    assert!(perto(volume(&r1), esperado, 0.5), "{} x {esperado}", volume(&r1));
    let (lo, hi) = caixa(&d);
    assert!(perto(lo[0], 0., 1e-6) && perto(hi[0], 60., 1e-6) && perto(hi[2], 20., 1e-6), "{lo:?} {hi:?}");

    // sem a chave (só o ponto), o filete ficaria sem aresta: referência perdida, nunca a vizinha
    if let FeatureKind::Fillet(o) = &mut d.feature_mut(fil).unwrap().kind {
        o.keys.clear();
    }
    assert!(erro(&recompute(&d), fil).contains("referência perdida"));
}

// ------------------------------------------------------------------ chapa

fn placa(largura: f64) -> (Document, Id, Id) {
    let mut d = Document::new("b516 chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [largura, 50.]);
    let w = d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    (d, sk, w)
}

#[test]
fn parede_de_chapa_e_bainha_seguem_as_arestas_quando_a_parede_base_cresce() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, sk, _) = placa(100.);
    // aba na aresta de cima em x = 100, como num arquivo antigo (só o ponto)
    let aba = d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
    let r0 = recompute(&d);
    assert!(ok(&r0, aba), "{:?}", r0.status.get(&aba));
    assert_eq!(gravar(&mut d, &r0), 1);
    // a ponta da aba (a aresta mais alta e mais para fora) tem nome: a aba deu papel às faces dela
    let b = &r0.bodies[0];
    let alto = (0..b.mesh.edge_mids.len() / 3).map(|e| meio(b, e)).fold(f64::MIN, |m, p| m.max(p[2]));
    let e = (0..b.mesh.edge_mids.len() / 3).filter(|e| perto(meio(b, *e)[2], alto, 1e-3)).max_by(|a, c| meio(b, *a)[0].total_cmp(&meio(b, *c)[0])).unwrap();
    let ponta = b.edge_key(e).expect("a aresta da ponta da aba tem nome");
    let nome = refs::nome_aresta(&d, &ponta, &|x| x.to_string());
    assert!(nome.contains("ponta") && ponta.a.owner == aba && ponta.b.owner == aba, "{nome}");
    // bainha na ponta da aba, sem chave (arquivo antigo): promovida no cálculo
    let bainha = d.add_feature(FeatureKind::SmHem(SmHem { edge: meio(b, e), length: s("8"), gap: s("0"), flip: false, kind: 0, edge_key: None }));
    let r1 = recompute(&d);
    assert!(ok(&r1, bainha), "{:?}", r1.status.get(&bainha));
    gravar(&mut d, &r1);
    let FeatureKind::SmHem(h) = &d.feature(bainha).unwrap().kind else { unreachable!() };
    let papeis = |k: &EdgeKey| {
        let mut v = [(k.a.owner, k.a.role), (k.b.owner, k.b.role)];
        v.sort_by_key(|x| (x.0 .0, x.1));
        v
    };
    assert_eq!(h.edge_key.as_ref().map(papeis), Some(papeis(&ponta)), "a bainha guarda a ponta da aba pelo nome");
    let (lo1, hi1) = caixa(&d);
    let v1 = volume(&r1);

    // a parede base cresce 20 mm em x: a aresta da aba anda 20 mm, e a ponta da aba junto
    set_rect(&mut d, sk, [0., 0.], [120., 50.]);
    let r2 = recompute(&d);
    assert!(ok(&r2, aba) && ok(&r2, bainha), "{:?} {:?}", r2.status.get(&aba), r2.status.get(&bainha));
    let (lo2, hi2) = caixa(&d);
    assert!(perto(hi2[0], hi1[0] + 20., 1e-3) && perto(lo2[0], lo1[0], 1e-3) && perto(hi2[2], hi1[2], 1e-3), "{lo1:?} {hi1:?} -> {lo2:?} {hi2:?}");
    // só a parede ganhou material: 20 × 50 × 2
    assert!(perto(volume(&r2) - v1, 2000., 1.), "{v1} -> {}", volume(&r2));

    // sem as chaves, o ponto antigo da aba não acha aresta nenhuma em x = 100 (agora meio da face de
    // cima): o comando não escolhe outra aresta em silêncio
    if let FeatureKind::SmFlange(a) = &mut d.feature_mut(aba).unwrap().kind {
        a.edge_key = None;
    }
    let msg = erro(&recompute(&d), aba);
    assert!(msg.contains("referência perdida"), "{msg}");
}

#[test]
fn linha_de_dobra_segue_a_face_quando_a_chapa_engrossa_e_perdida_vira_erro_com_candidatas() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, _, w) = placa(100.);
    let dobra = d.add_feature(FeatureKind::SmBend(SmBend { a: [60., 0., 2.], b: [60., 50., 2.], angle: s("90"), radius: None, flip: false, down: false, sketch: None, position: 0, cega: None }));
    let r0 = recompute(&d);
    assert!(ok(&r0, dobra), "{:?}", r0.status.get(&dobra));
    assert_eq!(gravar(&mut d, &r0), 1);
    let f = d.feature(dobra).unwrap();
    let Some(ChaveAncora::Face(k)) = f.ancora("linha", [60., 25., 2.]) else { panic!("âncora: {:?}", f.ancoras) };
    assert_eq!((k.owner, k.role), (w, 1), "a linha está na tampa final da parede");
    assert_eq!(refs::nome_ancora(&d, &ChaveAncora::Face(*k), &|x| x.to_string()), format!("{} · tampa final", d.name_of(w)));
    let v0 = volume(&r0);

    // a chapa vai para 3 mm: a face de cima sobe 1 mm e a linha vai com ela
    d.sheet.as_mut().unwrap().thickness = s("3");
    let r1 = recompute(&d);
    assert!(ok(&r1, dobra), "{:?}", r1.status.get(&dobra));
    assert!(volume(&r1) > v0 * 1.4, "{v0} -> {}", volume(&r1));

    // âncora com um papel que não existe: referência perdida, com as candidatas pelo nome
    let f = d.feature_mut(dobra).unwrap();
    let mut falsa = f.ancoras[0].clone();
    if let ChaveAncora::Face(k) = &mut falsa.chave {
        k.role = 77;
    }
    f.ancoras = vec![falsa];
    let msg = erro(&recompute(&d), dobra);
    assert!(msg.contains("referência perdida") && msg.contains("candidatas"), "{msg}");
}

// ------------------------------------------------------------------ furo e plano de face

#[test]
fn plano_de_face_e_furo_seguem_a_tampa_quando_a_peca_engrossa() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b516 furo");
    let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [60., 40.]);
    let placa = d.add_feature(ext(base, "10"));
    // plano na face de cima como num arquivo antigo: o dono e o ponto (longe do furo), sem chave
    let pl = d.add_face_plane([10., 10., 10.], [1., 0., 0.], [0., 0., 1.], Some(placa));
    let mut data = SketchData::default();
    let mut n = 5_180_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let p = data.add_point(&mut al, 20., 10.);
    data.add_curve(&mut al, fenkra_core::sketch::Curve::Point { p }, false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(pl), data }));
    let furo = d.add_feature(FeatureKind::Hole(Hole {
        sketch: sk,
        kind: HoleKind::Simples,
        diameter: s("6"),
        depth: Extent::Blind(s("5")),
        thread: None,
        thread_modeled: false,
        standard: None,
        points: vec![],
        dir: None,
        reverse: false,
        tip: None,
        taper: s("0"),
        left_hand: false,
        extra: Default::default(),
    }));
    let mut d = Document::from_json(&d.to_json()).unwrap();
    let r0 = recompute(&d);
    assert!(ok(&r0, furo), "{:?}", r0.status.get(&furo));
    let furo_v = PI * 9. * 5.;
    assert!(perto(volume(&r0), 60. * 40. * 10. - furo_v, 0.5), "{}", volume(&r0));
    assert_eq!(gravar(&mut d, &r0), 1, "o plano de face antigo ganha a chave");
    let Some(RefKind::FacePlane { key: Some(k), .. }) = d.reference(pl).map(|r| &r.kind) else { panic!("sem chave") };
    assert_eq!((k.owner, k.role), (placa, 1), "tampa final da extrusão");

    // a placa vai para 20 mm: a tampa sobe 10 mm, o plano e o furo (5 mm de fundo) sobem junto
    if let FeatureKind::Extrude(e) = &mut d.feature_mut(placa).unwrap().kind {
        e.extent = Extent::Blind(s("20"));
    }
    let r1 = recompute(&d);
    assert!(ok(&r1, furo) && ok(&r1, pl), "{:?} {:?}", r1.status.get(&furo), r1.status.get(&pl));
    let plano = r1.planes.iter().find(|p| p.id == Some(pl)).expect("plano");
    assert!(perto(plano.frame.origin[2], 20., 1e-6), "o plano vai com a face: {:?}", plano.frame.origin);
    assert!(perto(volume(&r1), 60. * 40. * 20. - furo_v, 0.5), "o furo continua com 5 mm a partir da face de cima: {}", volume(&r1));
    let (lo, hi) = caixa(&d);
    assert!(perto(lo[2], 0., 1e-6) && perto(hi[2], 20., 1e-6), "{lo:?} {hi:?}");

    // a chave com um papel que não existe: o plano fica em erro com o nome, nunca na primeira face
    if let Some(RefKind::FacePlane { key: Some(k), .. }) = d.references.iter_mut().find(|r| r.id == pl).map(|r| &mut r.kind) {
        k.role = 55;
    }
    let r2 = recompute(&d);
    assert!(matches!(r2.status.get(&pl), Some(Status::Error(e)) if e.contains("referência perdida")), "{:?}", r2.status.get(&pl));
}

#[test]
fn termino_ate_a_face_de_arquivo_antigo_ganha_a_chave() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b516 até a face");
    let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [40., 40.]);
    let bloco = d.add_feature(ext(base, "10"));
    let mut data = SketchData::default();
    let mut n = 5_190_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let p = data.add_point(&mut al, 20., 20.);
    data.add_curve(&mut al, fenkra_core::sketch::Curve::Point { p }, false);
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data }));
    // furo até a face de baixo (tampa inicial), guardada só pelo ponto
    let furo = d.add_feature(FeatureKind::Hole(Hole {
        sketch: sk,
        kind: HoleKind::Simples,
        diameter: s("6"),
        depth: Extent::UpToSurface(SurfRef::Face { point: [5., 5., 0.], normal: [0., 0., -1.], owner: Some(bloco), key: None }),
        thread: None,
        thread_modeled: false,
        standard: None,
        points: vec![],
        dir: None,
        reverse: false,
        tip: None,
        taper: s("0"),
        left_hand: false,
        extra: Default::default(),
    }));
    let r0 = recompute(&d);
    assert!(ok(&r0, furo), "{:?}", r0.status.get(&furo));
    assert_eq!(gravar(&mut d, &r0), 1);
    let FeatureKind::Hole(h) = &d.feature(furo).unwrap().kind else { unreachable!() };
    let Extent::UpToSurface(SurfRef::Face { key: Some(k), .. }) = &h.depth else { panic!("sem chave: {:?}", h.depth) };
    assert_eq!((k.owner, k.role), (bloco, 0), "tampa inicial do bloco");
    let r1 = recompute(&d);
    assert!(ok(&r1, furo) && perto(volume(&r1), 40. * 40. * 10. - PI * 9. * 10., 0.5), "{}", volume(&r1));
}

// ------------------------------------------------------------------ estrutural

#[test]
fn placa_de_extremidade_segue_a_ponta_do_membro_quando_a_linha_muda() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b516 estrutural");
    let l1 = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 0., 500.]), start: s("0"), end: s("0") });
    let l2 = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 500.]), b: PointRef::Fixed([300., 0., 500.]), start: s("0"), end: s("0") });
    let membro = d.add_feature(FeatureKind::StructMember(StructMember {
        paths: vec![CurveRef::Ref(l1), CurveRef::Ref(l2)],
        family: s("Tubo quadrado"),
        designation: s("40x3"),
        kind: SectionKind::TuboQuadrado,
        dims: vec![s("40"), s("3")],
        anchor: 0,
        angle: s("0"),
        off_x: s("0"),
        off_y: s("0"),
        ext1: s("0"),
        ext2: s("0"),
        corner: CornerMode::MeiaEsquadria,
        gap: s("0"),
        joints: vec![],
        material: String::new(),
        mirror: false,
    }));
    // placa na ponta de baixo da primeira linha (arquivo antigo: só o ponto)
    let placa = d.add_feature(FeatureKind::StructPlate(StructPlate::nova([0., 0., 0.], "8", "5")));
    let r0 = recompute(&d);
    assert!(ok(&r0, placa), "{:?}", r0.status.get(&placa));
    assert_eq!(gravar(&mut d, &r0), 1);
    let f = d.feature(placa).unwrap();
    assert_eq!(f.ancora("at", [0., 0., 0.]), Some(&ChaveAncora::PontaMembro { membro, trecho: 0, fim: false }), "{:?}", f.ancoras);
    assert!(perto(caixa(&d).0[2], -8., 1e-3), "{:?}", caixa(&d));

    // a primeira linha desce até z = -600: pelo ponto antigo, a ponta mais perto de (0, 0, 0) seria o
    // canto de cima (a 500 mm); pela âncora, a placa vai com a ponta de baixo
    if let Some(r) = d.references.iter_mut().find(|r| r.id == l1) {
        r.kind = RefKind::LinePoints { a: PointRef::Fixed([0., 0., -600.]), b: PointRef::Fixed([0., 0., 500.]), start: s("0"), end: s("0") };
    }
    let r1 = recompute(&d);
    assert!(ok(&r1, placa), "{:?}", r1.status.get(&placa));
    let (lo, hi) = caixa(&d);
    assert!(perto(lo[2], -608., 1e-3), "a placa fica 8 mm além da ponta nova: {lo:?} {hi:?}");

    // a âncora de um trecho que não existe mais é referência perdida, com a ponta mais perto pelo nome
    let f = d.feature_mut(placa).unwrap();
    f.ancoras = vec![Ancora { campo: s("at"), ponto: [0., 0., 0.], chave: ChaveAncora::PontaMembro { membro, trecho: 9, fim: false } }];
    let msg = erro(&recompute(&d), placa);
    assert!(msg.contains("referência perdida") && msg.contains("mais perto"), "{msg}");
}
