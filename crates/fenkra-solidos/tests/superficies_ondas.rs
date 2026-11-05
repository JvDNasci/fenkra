//! Superfícies e wireframe: preenchimento, junção e demais comandos, com o documento montado em
//! código, recalculado e conferido por status e medidas.

use fenkra_core::*;
use fenkra_kernel::Shape;
use fenkra_solidos::{recompute, surface_brep, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.into()
}

fn ok(r: &RecomputeResult, id: Id) {
    assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{:?}", r.status[&id]);
}

/// Forma exata de uma superfície.
fn forma(d: &Document, id: Id) -> Shape {
    Shape::from_brep(&surface_brep(d, id).expect("superfície")).unwrap()
}

fn plano_x(d: &mut Document, x: &str) -> SurfRef {
    let id = d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::YZ), offset: s(x) });
    SurfRef::Plane(PlaneRef::Datum(id))
}

fn plano_y(d: &mut Document, y: &str) -> SurfRef {
    // a normal do plano XZ da Origem aponta para −Y
    let id = d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::XZ), offset: s(y) });
    SurfRef::Plane(PlaneRef::Datum(id))
}

/// Quatro faces de uma esfera em volta de um buraco quadrado no polo. Devolve (faces, arcos do
/// contorno). Cada face é a calota além de um plano, aparada pelos dois planos vizinhos.
fn buraco_em_esfera(d: &mut Document) -> (Vec<Id>, Vec<Id>) {
    let esfera = d.add_geometry(RefKind::SurfSphere { center: PointRef::Origin, radius: s("100") });
    let xy = SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY));
    let topo = d.add_geometry(RefKind::SurfSplit { surface: SurfRef::Ref(esfera), tool: xy, flip: false, more: vec![], curve: None });
    let (xp, xm) = (plano_x(d, "30"), plano_x(d, "-30"));
    let (yp, ym) = (plano_y(d, "-30"), plano_y(d, "30"));
    let (mut faces, mut arcos) = (vec![], vec![]);
    // (plano do lado, lado que fica; planos vizinhos, lados que ficam)
    for (corte, f, a, fa, b, fb) in [(xp, false, yp, false, ym, true), (xm, true, yp, false, ym, true), (yp, true, xp, true, xm, false), (ym, false, xp, true, xm, false)] {
        let lado = d.add_geometry(RefKind::SurfSplit { surface: SurfRef::Ref(topo), tool: corte, flip: f, more: vec![], curve: None });
        let m1 = d.add_geometry(RefKind::SurfSplit { surface: SurfRef::Ref(lado), tool: a, flip: fa, more: vec![], curve: None });
        let m2 = d.add_geometry(RefKind::SurfSplit { surface: SurfRef::Ref(m1), tool: b, flip: fb, more: vec![], curve: None });
        let arco = d.add_geometry(RefKind::CurveIntersect { a: SurfRef::Ref(m2), b: corte, which: 0 });
        faces.push(m2);
        arcos.push(arco);
    }
    (faces, arcos)
}

/// Maior desvio angular (graus) entre o preenchimento e as vizinhas, em toda a volta da junção.
fn desvio_na_juncao(d: &Document, r: &RecomputeResult, fill: Id, faces: &[Id], arcos: &[Id]) -> f64 {
    let sh = forma(d, fill);
    let mut pior = 0f64;
    for (f, a) in faces.iter().zip(arcos) {
        let vizinha = forma(d, *f);
        let pts = &r.curves.iter().find(|c| c.id == *a).unwrap().pts;
        for p in pts.iter().step_by((pts.len() / 8).max(1)) {
            let (_, n1) = sh.surface_near(*p).unwrap();
            let (_, n2) = vizinha.surface_near(*p).unwrap();
            let c = (n1[0] * n2[0] + n1[1] * n2[1] + n1[2] * n2[2]).abs().min(1.);
            pior = pior.max(c.acos().to_degrees());
        }
    }
    pior
}

#[test]
fn preenchimento_g1_entre_quatro_faces_curvas() {
    let mut d = Document::new("t");
    let (faces, arcos) = buraco_em_esfera(&mut d);
    let curvas: Vec<CurveRef> = arcos.iter().map(|a| CurveRef::Ref(*a)).collect();
    let conds = |peso: &str| faces.iter().map(|f| FillCond { cont: Continuity::G1, support: Some(SurfRef::Ref(*f)), weight: s(peso) }).collect::<Vec<_>>();
    let fill = d.add_geometry(RefKind::SurfFill { curves: curvas.clone(), conds: conds("1"), points: vec![], tol: s("0.01") });
    // controle: sem tangência a junção tem de quebrar, senão a medida não prova nada
    let solto = d.add_geometry(RefKind::SurfFill { curves: curvas.clone(), conds: vec![], points: vec![], tol: s("0.01") });
    let pesado = d.add_geometry(RefKind::SurfFill { curves: curvas, conds: conds("3"), points: vec![], tol: s("0.01") });
    let r = recompute(&d);
    for x in faces.iter().chain(&arcos).chain([&fill, &solto, &pesado]) {
        ok(&r, *x);
    }
    let note = &r.notes[&fill];
    assert!(note.contour.as_ref().unwrap().open_ends.is_empty());
    // o preenchimento reproduz a esfera no polo
    let (q, _) = forma(&d, fill).surface_near([0., 0., 100.]).unwrap();
    assert!((q[2] - 100.).abs() < 0.5, "polo em {q:?}");
    let g1 = desvio_na_juncao(&d, &r, fill, &faces, &arcos);
    let g0 = desvio_na_juncao(&d, &r, solto, &faces, &arcos);
    assert!(g1 < 0.5, "desvio de {g1:.3} graus");
    assert!(g0 > 3. * g1 && g0 > 0.5, "controle G0 com {g0:.3} graus, G1 com {g1:.3}");
    assert!(note.g1_deg.unwrap() < 0.5, "{note:?}");
    // o peso leva a tangente mais para dentro: o polo sobe
    let (qp, _) = forma(&d, pesado).surface_near([0., 0., 120.]).unwrap();
    assert!((qp[2] - q[2]).abs() > 0.05, "peso sem efeito: {qp:?} {q:?}");
}

#[test]
fn g1_sem_face_de_apoio_nao_escolhe_em_silencio() {
    let mut d = Document::new("t");
    let p = |x: f64, y: f64| PointRef::Fixed([x, y, 0.]);
    let lados: Vec<CurveRef> = [(0., 0., 10., 0.), (10., 0., 10., 10.), (10., 10., 0., 10.), (0., 10., 0., 0.)]
        .iter()
        .map(|(a, b, c, e)| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: p(*a, *b), b: p(*c, *e), start: s("0"), end: s("0") })))
        .collect();
    let conds = vec![FillCond { cont: Continuity::G1, support: None, weight: s("1") }];
    let fill = d.add_geometry(RefKind::SurfFill { curves: lados, conds, points: vec![], tol: s("0.01") });
    assert!(matches!(&recompute(&d).status[&fill], Status::Error(m) if m.contains("face de apoio")));
}

#[test]
fn contorno_aberto_conta_pontas_e_folga_fecha_pela_tolerancia() {
    let mut d = Document::new("t");
    let p = |x: f64, y: f64| PointRef::Fixed([x, y, 0.]);
    // a última linha para 0,05 mm antes de fechar
    let lados: Vec<CurveRef> = [(0., 0., 10., 0.), (10., 0., 10., 10.), (10., 10., 0., 10.), (0., 10., 0., 0.05)]
        .iter()
        .map(|(a, b, c, e)| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: p(*a, *b), b: p(*c, *e), start: s("0"), end: s("0") })))
        .collect();
    let fill = d.add_geometry(RefKind::SurfFill { curves: lados.clone(), conds: vec![], points: vec![], tol: s("0.01") });
    let r = recompute(&d);
    assert!(matches!(&r.status[&fill], Status::Error(m) if m.contains("contorno aberto")), "{:?}", r.status[&fill]);
    let c = r.notes[&fill].contour.clone().unwrap();
    assert_eq!(c.open_ends.len(), 2);
    assert!((c.max_gap - 0.05).abs() < 1e-9 && !c.gaps[0].closed);
    let RefKind::SurfFill { tol, .. } = &mut d.references.iter_mut().find(|r| r.id == fill).unwrap().kind else { panic!() };
    *tol = s("0.1");
    let r = recompute(&d);
    ok(&r, fill);
    assert!(r.notes[&fill].contour.as_ref().unwrap().gaps[0].closed);
    let m = forma(&d, fill).mass();
    assert!((m.area - 100.).abs() < 0.5, "{}", m.area);
}

#[test]
fn ponto_de_passagem_prende_a_superficie() {
    let mut d = Document::new("t");
    let p = |x: f64, y: f64| PointRef::Fixed([x, y, 0.]);
    let lados: Vec<CurveRef> = [(0., 0., 10., 0.), (10., 0., 10., 10.), (10., 10., 0., 10.), (0., 10., 0., 0.)]
        .iter()
        .map(|(a, b, c, e)| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: p(*a, *b), b: p(*c, *e), start: s("0"), end: s("0") })))
        .collect();
    let fill = d.add_geometry(RefKind::SurfFill { curves: lados, conds: vec![], points: vec![PointRef::Fixed([5., 5., 3.])], tol: s("0.01") });
    let r = recompute(&d);
    ok(&r, fill);
    let (q, _) = forma(&d, fill).surface_near([5., 5., 3.]).unwrap();
    assert!((q[2] - 3.).abs() < 0.05, "{q:?}");
}

/// Quatro linhas de um sketch: o caso que não juntava.
fn sketch_quadrado(d: &mut Document) -> (Id, [Id; 4]) {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    let (_, linhas) = data.add_rect(&mut al, [0., 0.], [20., 20.], false);
    let id = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    (id, linhas)
}

#[test]
fn juntar_e_preencher_linhas_de_sketch() {
    let mut d = Document::new("t");
    let (sk, linhas) = sketch_quadrado(&mut d);
    let curvas: Vec<CurveRef> = linhas.iter().map(|l| CurveRef::SketchCurve { sketch: sk, curve: *l }).collect();
    let junta = d.add_geometry(RefKind::CurveJoin { curves: curvas.clone() });
    let fill = d.add_geometry(RefKind::SurfFill { curves: curvas, conds: vec![], points: vec![], tol: s("0.01") });
    let r = recompute(&d);
    ok(&r, junta);
    ok(&r, fill);
    let c = r.curves.iter().find(|c| c.id == junta).unwrap();
    assert!(fenkra_solidos::geo3d::dist(c.pts[0], *c.pts.last().unwrap()) < 1e-6);
    assert!((forma(&d, fill).mass().area - 400.).abs() < 1.);
}

#[test]
fn juntar_faces_e_curvas_relata_dominios_bordas_e_folga() {
    let mut d = Document::new("t");
    let l = |d: &mut Document, a: [f64; 3], b: [f64; 3]| d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") });
    let (c1, c2) = (l(&mut d, [0., 0., 0.], [10., 0., 0.]), l(&mut d, [10., 0., 0.], [20., 0., 0.]));
    let ez = AxisRef::Origin(OriginAxis::Z);
    let f1 = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(c1), dir: ez, length: s("10"), length2: s("0"), until: None, until2: None });
    let f2 = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(c2), dir: ez, length: s("10"), length2: s("0"), until: None, until2: None });
    let longe = l(&mut d, [0., 50., 0.], [10., 50., 0.]);
    let j = d.add_geometry(RefKind::SurfJoin { items: vec![SurfRef::Ref(f1), SurfRef::Ref(f2)], tol: s("0.01"), curves: vec![CurveRef::Ref(longe)] });
    let r = recompute(&d);
    ok(&r, j);
    let rep = r.notes[&j].join.unwrap();
    assert_eq!(rep.domains, 2, "{rep:?}");
    assert_eq!(rep.free_edges, 6, "{rep:?}");
    assert!((rep.open_gap - 50.).abs() < 1e-6, "{rep:?}");
}

/// Caixa com todas as arestas arredondadas: 6 faces planas, 12 de filete e 8 de canto.
fn caixa_arredondada(d: &mut Document) -> Id {
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("40"), b: s("40"), c: s("40"), d: s("0"), op: BoolOp::Add }));
    d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("5"), edges: EdgeSel::AllOfBody, mode: Default::default(), prop: Default::default(), keys: vec![] }))
}

/// Centro da face de cima do primeiro corpo.
fn topo_de(r: &RecomputeResult) -> [f64; 3] {
    let p = &r.bodies[0].mesh.positions;
    let ext = |k: usize| {
        let v = p.iter().skip(k).step_by(3).map(|x| *x as f64);
        (v.clone().fold(f64::MAX, f64::min), v.fold(f64::MIN, f64::max))
    };
    let ((a, b), (c, e), (_, z)) = (ext(0), ext(1), ext(2));
    [(a + b) / 2., (c + e) / 2., z]
}

fn extrair(d: &mut Document, face: [f64; 3], prop: Propagation, complement: bool, expected: Option<u32>) -> Id {
    d.add_geometry(RefKind::SurfExtract { face, owner: None, prop, complement, expected })
}

#[test]
fn extrair_refaz_a_propagacao_e_conta_as_faces() {
    let mut d = Document::new("t");
    caixa_arredondada(&mut d);
    let r = recompute(&d);
    let topo = topo_de(&r);
    let so = extrair(&mut d, topo, Propagation::Nenhuma, false, None);
    let tan = extrair(&mut d, topo, Propagation::Tangente, false, None);
    let pto = extrair(&mut d, topo, Propagation::Ponto, false, None);
    let tudo = extrair(&mut d, topo, Propagation::Completo, false, None);
    let resto = extrair(&mut d, topo, Propagation::Nenhuma, true, None);
    let r = recompute(&d);
    for x in [so, tan, pto, tudo, resto] {
        ok(&r, x);
    }
    let n = |x: Id| r.notes[&x].count.unwrap().0;
    assert_eq!((n(so), n(tan), n(tudo)), (1, 26, 26));
    assert_eq!(n(resto), 25);
    // só a face plana de cima: 40 − 2·5 de lado
    assert!((forma(&d, so).mass().area - 900.).abs() < 1.);
    let area = |x: Id| forma(&d, x).mass().area;
    assert!((area(so) + area(resto) - area(tudo)).abs() < 1e-3 * area(tudo));
}

#[test]
fn extrair_avisa_quando_a_contagem_muda() {
    let mut d = Document::new("t");
    caixa_arredondada(&mut d);
    let r = recompute(&d);
    let topo = topo_de(&r);
    let x = extrair(&mut d, topo, Propagation::Tangente, false, Some(30));
    let r = recompute(&d);
    assert!(matches!(&r.status[&x], Status::Warning(m) if m.contains("30 faces") && m.contains("26")), "{:?}", r.status[&x]);
}

#[test]
fn contorno_propaga_por_aresta_tangente_e_laco() {
    let mut d = Document::new("t");
    // sketch-L extrudado não entra aqui: uma superfície plana quadrada tem 4 arestas e 4 cantos
    let p = |x: f64, y: f64| PointRef::Fixed([x, y, 0.]);
    let lados: Vec<CurveRef> = [(0., 0., 10., 0.), (10., 0., 10., 10.), (10., 10., 0., 10.), (0., 10., 0., 0.)]
        .iter()
        .map(|(a, b, c, e)| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: p(*a, *b), b: p(*c, *e), start: s("0"), end: s("0") })))
        .collect();
    let f = d.add_geometry(RefKind::SurfFill { curves: lados, conds: vec![], points: vec![], tol: s("0.01") });
    let borda = |d: &mut Document, prop| d.add_geometry(RefKind::CurveBoundary { surface: SurfRef::Ref(f), which: 0, prop, near: Some([5., 0., 0.]), expected: None });
    let (a, b, c, e) = (borda(&mut d, Propagation::Nenhuma), borda(&mut d, Propagation::Tangente), borda(&mut d, Propagation::Ponto), borda(&mut d, Propagation::Completo));
    let r = recompute(&d);
    let n = |x: Id| r.notes[&x].count.unwrap().0;
    assert_eq!((n(a), n(b), n(c), n(e)), (1, 1, 4, 4));
    let compr = |x: Id| fenkra_solidos::geo3d::Curve3::poly(r.curves.iter().find(|c| c.id == x).unwrap().pts.clone(), false).unwrap().length();
    assert!((compr(a) - 10.).abs() < 1e-6 && (compr(c) - 40.).abs() < 1e-6);
}

/// Extrusão de um sketch: é ela que dá papel às faces (tampas e laterais).
fn extrusao(d: &mut Document, sketch: Id, altura: &str) -> Id {
    d.add_feature(FeatureKind::Extrude(Extrude { sketch, op: BoolOp::Add, extent: Extent::Blind(s(altura)), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0 }))
}

fn caixa(d: &mut Document, altura: &str) -> Id {
    let (sk, _) = sketch_quadrado(d);
    extrusao(d, sk, altura)
}

fn muda_altura(d: &mut Document, id: Id, altura: &str) {
    let FeatureKind::Extrude(e) = &mut d.feature_mut(id).unwrap().kind else { panic!() };
    e.extent = Extent::Blind(s(altura));
}

#[test]
fn plano_de_face_acompanha_e_cai_em_erro_quando_a_face_some() {
    let mut d = Document::new("t");
    let cx = caixa(&mut d, "30");
    let r = recompute(&d);
    let topo = topo_de(&r);
    let b = &r.bodies[0];
    let f = (0..b.mesh.face_info.len()).find(|j| b.mesh.face_info[*j].dir[2] > 0.99).unwrap();
    let key = b.face_key(f).expect("papel da tampa");
    assert_eq!(key.owner, cx);
    let com = d.add_geometry(RefKind::FacePlane { origin: topo, x: [1., 0., 0.], normal: [0., 0., 1.], owner: Some(cx), key: Some(key) });
    let sem = d.add_geometry(RefKind::FacePlane { origin: topo, x: [1., 0., 0.], normal: [0., 0., 1.], owner: Some(cx), key: None });
    let r = recompute(&d);
    ok(&r, com);
    ok(&r, sem);
    // a caixa cresce: o plano com chave vai junto; o sem chave perdeu a face e diz isso
    muda_altura(&mut d, cx, "50");
    let r = recompute(&d);
    ok(&r, com);
    let z = r.planes.iter().find(|p| p.id == Some(com)).unwrap().frame.origin[2];
    assert!((z - (topo[2] + 20.)).abs() < 1e-6, "{z}");
    assert!(matches!(&r.status[&sem], Status::Error(m) if m.starts_with("referência perdida")), "{:?}", r.status[&sem]);
    // a tampa some de vez: chanfro em toda a caixa não mata a tampa, mas apagar a caixa sim
    d.delete(cx, false);
    let r = recompute(&d);
    assert!(matches!(&r.status[&com], Status::Error(m) if m.starts_with("referência perdida")), "{:?}", r.status[&com]);
}

#[test]
fn plano_na_curva_por_ponto_contido_ou_projetado() {
    let mut d = Document::new("t");
    let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([100., 0., 0.]), start: s("0"), end: s("0") });
    let pl = |d: &mut Document, q: [f64; 3], project| d.add_geometry(RefKind::PlaneNormalCurve { curve: CurveRef::Ref(l), at: OnCurve::Ratio(s("0.5")), reverse: false, point: Some(PointRef::Fixed(q)), project });
    let (a, b, c) = (pl(&mut d, [30., 0., 0.], false), pl(&mut d, [30., 7., 0.], true), pl(&mut d, [30., 7., 0.], false));
    let r = recompute(&d);
    let f = |x: Id| r.planes.iter().find(|p| p.id == Some(x)).unwrap().frame;
    assert!((f(a).origin[0] - 30.).abs() < 1e-9 && f(a).normal[0].abs() > 0.999);
    assert!((f(b).origin[0] - 30.).abs() < 1e-9 && f(b).origin[1].abs() < 1e-9);
    assert!(matches!(&r.status[&c], Status::Error(m) if m.contains("projetado")));
}

#[test]
fn eixo_de_cilindro_aresta_e_planos_acompanha_o_modelo() {
    let mut d = Document::new("t");
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    let c = data.add_point(&mut al, 0., 0.);
    data.add_curve(&mut al, sketch::Curve::Circle { c, r: 10. }, false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    let cil = extrusao(&mut d, sk, "30");
    let r = recompute(&d);
    let b = &r.bodies[0];
    let lat = (0..b.mesh.face_info.len()).find(|j| b.mesh.face_info[*j].kind == 1).unwrap();
    let key = b.face_key(lat).expect("papel da lateral");
    let ponto = b.mesh.face_info[lat].point;
    let e1 = d.add_geometry(RefKind::AxisLine { axis: AxisRef::Face { point: ponto, key: Some(key) }, length: String::new() });
    let e2 = d.add_geometry(RefKind::AxisLine { axis: AxisRef::Planes { a: PlaneRef::Origin(OriginPlane::XZ), b: PlaneRef::Origin(OriginPlane::YZ) }, length: s("10") });
    let r = recompute(&d);
    ok(&r, e1);
    ok(&r, e2);
    let c = |r: &RecomputeResult, x: Id| r.curves.iter().find(|c| c.id == x).unwrap().pts.clone();
    let p = c(&r, e1);
    assert!((fenkra_solidos::geo3d::dist(p[0], p[1]) - 30.).abs() < 1e-6, "{p:?}");
    assert!(p[0][0].abs() < 1e-6 && p[0][1].abs() < 1e-6);
    let q = c(&r, e2);
    assert!(q[0][0].abs() < 1e-9 && q[0][1].abs() < 1e-9 && (q[1][2] - q[0][2]).abs() > 9.99);
    // a altura muda: o eixo acompanha, sem nada fixo
    muda_altura(&mut d, cil, "45");
    let r = recompute(&d);
    ok(&r, e1);
    let p = c(&r, e1);
    assert!((fenkra_solidos::geo3d::dist(p[0], p[1]) - 45.).abs() < 1e-6, "{p:?}");
}

#[test]
fn ponto_em_vertice_e_aresta_seguem_a_chave() {
    let mut d = Document::new("t");
    let cx = caixa(&mut d, "30");
    let r = recompute(&d);
    let b = &r.bodies[0];
    // aresta de cima, entre a tampa e uma lateral
    let e = (0..b.mesh.edge_faces.len()).find(|e| b.edge_key(*e).is_some_and(|k| k.a.role == 1 || k.b.role == 1)).unwrap();
    let key = b.edge_key(e).unwrap();
    let m = &b.mesh.edge_mids;
    let mid = [m[3 * e] as f64, m[3 * e + 1] as f64, m[3 * e + 2] as f64];
    let linha = d.add_geometry(RefKind::CurveCopy { curve: CurveRef::BodyEdge { pts: vec![mid, mid], key } });
    let ponto = d.add_geometry(RefKind::PointBetween { a: PointRef::Vertex { point: mid, edge: key }, b: PointRef::Origin, ratio: s("0") });
    let r = recompute(&d);
    ok(&r, linha);
    ok(&r, ponto);
    muda_altura(&mut d, cx, "50");
    let r = recompute(&d);
    ok(&r, linha);
    let c = r.curves.iter().find(|c| c.id == linha).unwrap();
    assert!(c.pts.iter().all(|q| (q[2] - (mid[2] + 20.)).abs() < 1e-6), "{:?}", c.pts);
    assert!((r.points.iter().find(|p| p.id == ponto).unwrap().p[2] - (mid[2] + 20.)).abs() < 1e-6);
}

/// Faixa plana de 20 × 30 no plano XZ, y = 0, x de 0 a 20 e z de 0 a 30.
fn faixa(d: &mut Document) -> Id {
    let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([20., 0., 0.]), start: s("0"), end: s("0") });
    d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(l), dir: AxisRef::Origin(OriginAxis::Z), length: s("30"), length2: s("0"), until: None, until2: None })
}

fn plano_z(d: &mut Document, z: &str) -> SurfRef {
    SurfRef::Plane(PlaneRef::Datum(d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::XY), offset: s(z) })))
}

fn caixa_de(d: &Document, id: Id) -> ([f64; 3], [f64; 3]) {
    forma(d, id).bbox().unwrap()
}

#[test]
fn superficie_extrudada_ate_elemento_dos_dois_lados() {
    let mut d = Document::new("t");
    let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([20., 0., 0.]), start: s("0"), end: s("0") });
    let (alto, baixo) = (plano_z(&mut d, "12"), plano_z(&mut d, "-7"));
    let z = AxisRef::Origin(OriginAxis::Z);
    let ate = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(l), dir: z, length: s("0"), length2: s("0"), until: Some(alto), until2: Some(baixo) });
    let misto = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(l), dir: z, length: s("0"), length2: s("5"), until: Some(alto), until2: None });
    let r = recompute(&d);
    ok(&r, ate);
    ok(&r, misto);
    let (a, b) = caixa_de(&d, ate);
    assert!((a[2] + 7.).abs() < 1e-4 && (b[2] - 12.).abs() < 1e-4, "{a:?} {b:?}");
    assert!((forma(&d, ate).mass().area - 20. * 19.).abs() < 1e-3);
    assert!((forma(&d, misto).mass().area - 20. * 17.).abs() < 1e-3);
}

#[test]
fn superficie_de_revolucao_com_dois_angulos() {
    let mut d = Document::new("t");
    let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([10., 0., 0.]), b: PointRef::Fixed([10., 0., 20.]), start: s("0"), end: s("0") });
    let rv = d.add_geometry(RefKind::SurfRevolve { curve: CurveRef::Ref(l), axis: AxisRef::Origin(OriginAxis::Z), angle: s("90"), angle2: s("30") });
    let demais = d.add_geometry(RefKind::SurfRevolve { curve: CurveRef::Ref(l), axis: AxisRef::Origin(OriginAxis::Z), angle: s("300"), angle2: s("100") });
    let r = recompute(&d);
    ok(&r, rv);
    let area = 120f64.to_radians() * 10. * 20.;
    assert!((forma(&d, rv).mass().area - area).abs() < 1e-3 * area);
    // 30 graus para trás: o giro chega a y negativo
    assert!((caixa_de(&d, rv).0[1] + 10. * 30f64.to_radians().sin()).abs() < 1e-3);
    assert!(matches!(&r.status[&demais], Status::Error(m) if m.contains("uma volta")));
}

fn borda_de_cima() -> CurveRef {
    CurveRef::Edge(vec![[0., 0., 30.], [20., 0., 30.]])
}

fn extrapolar(d: &mut Document, f: Id, edges: Vec<CurveRef>, len: &str, until: Option<SurfRef>, normal_edge: bool) -> Id {
    d.add_geometry(RefKind::SurfExtrapolate { surface: SurfRef::Ref(f), near: PointRef::Origin, length: s(len), curvature: false, more: vec![], edges, until, normal_edge, tangent_prop: false })
}

#[test]
fn extrapolar_pela_borda_clicada_estende_apara_e_vai_ate_elemento() {
    let mut d = Document::new("t");
    let f = faixa(&mut d);
    let mais = extrapolar(&mut d, f, vec![borda_de_cima()], "10", None, false);
    let menos = extrapolar(&mut d, f, vec![borda_de_cima()], "-10", None, false);
    let alvo = plano_z(&mut d, "47");
    let ate = extrapolar(&mut d, f, vec![borda_de_cima()], "0", Some(alvo), false);
    let normal = extrapolar(&mut d, f, vec![borda_de_cima()], "10", None, true);
    let longe = extrapolar(&mut d, f, vec![CurveRef::Edge(vec![[0., 40., 30.], [20., 40., 30.]])], "10", None, false);
    let r = recompute(&d);
    for x in [mais, menos, ate, normal] {
        ok(&r, x);
    }
    let topo = |x: Id| caixa_de(&d, x).1[2];
    assert!((topo(mais) - 40.).abs() < 1e-3, "{}", topo(mais));
    assert!((topo(menos) - 20.).abs() < 1e-3, "{}", topo(menos));
    // a caixa de uma face aparada em B-spline vem dos polos: quem mede o corte é a área
    assert!((forma(&d, ate).mass().area - 940.).abs() < 1e-2, "{}", forma(&d, ate).mass().area);
    assert!((topo(normal) - 40.).abs() < 1e-3, "{}", topo(normal));
    assert!((forma(&d, menos).mass().area - 400.).abs() < 1e-3);
    assert!(matches!(&r.status[&longe], Status::Error(m) if m.contains("não é borda")));
}
