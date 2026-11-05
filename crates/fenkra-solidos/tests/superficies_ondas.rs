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
    let f1 = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(c1), dir: ez, length: s("10"), length2: s("0") });
    let f2 = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(c2), dir: ez, length: s("10"), length2: s("0") });
    let longe = l(&mut d, [0., 50., 0.], [10., 50., 0.]);
    let j = d.add_geometry(RefKind::SurfJoin { items: vec![SurfRef::Ref(f1), SurfRef::Ref(f2)], tol: s("0.01"), curves: vec![CurveRef::Ref(longe)] });
    let r = recompute(&d);
    ok(&r, j);
    let rep = r.notes[&j].join.unwrap();
    assert_eq!(rep.domains, 2, "{rep:?}");
    assert_eq!(rep.free_edges, 6, "{rep:?}");
    assert!((rep.open_gap - 50.).abs() < 1e-6, "{rep:?}");
}
