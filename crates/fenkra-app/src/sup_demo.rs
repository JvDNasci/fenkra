// SPDX-License-Identifier: LGPL-2.1-or-later
//! Casos de `FENKRA_DEMO_PREVIA` para conferir sem mouse o preenchimento (B09) e as alças dos valores
//! principais de superfície (B240). Só abrem comandos: nada daqui entra num documento de verdade.

use crate::previa::DemoPrevia::{self, Ref};
use fenkra_core::*;

fn t(x: &str) -> String {
    x.to_string()
}

fn reta(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: t("0"), end: t("0") }))
}

/// Folha plana: a reta `a`–`b` extrudada 30 mm em Z.
fn folha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> SurfRef {
    let curve = reta(d, a, b);
    SurfRef::Ref(d.add_geometry(RefKind::SurfExtrude { curve, dir: AxisRef::Origin(OriginAxis::Z), length: t("30"), length2: t("0"), until: None, until2: None }))
}

fn plano(d: &mut Document, base: OriginPlane, dist: &str) -> SurfRef {
    SurfRef::Plane(PlaneRef::Datum(d.add_offset_plane(PlaneRef::Origin(base), dist)))
}

/// Calota de esfera com um buraco quadrado: as quatro faces em volta e os quatro arcos do buraco.
fn buraco_em_esfera(d: &mut Document) -> (Vec<Id>, Vec<Id>) {
    let esfera = d.add_geometry(RefKind::SurfSphere { center: PointRef::Origin, radius: t("100") });
    let xy = SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY));
    let corta = |d: &mut Document, surface, tool, flip| d.add_geometry(RefKind::SurfSplit { surface, tool, flip, more: vec![], curve: None, more_flip: vec![] });
    let topo = corta(d, SurfRef::Ref(esfera), xy, false);
    let (xp, xm) = (plano(d, OriginPlane::YZ, "30"), plano(d, OriginPlane::YZ, "-30"));
    let (yp, ym) = (plano(d, OriginPlane::XZ, "-30"), plano(d, OriginPlane::XZ, "30"));
    let (mut faces, mut arcos) = (vec![], vec![]);
    for (c, f, a, fa, b, fb) in [(xp, false, yp, false, ym, true), (xm, true, yp, false, ym, true), (yp, true, xp, true, xm, false), (ym, false, xp, true, xm, false)] {
        let lado = corta(d, SurfRef::Ref(topo), c, f);
        let m1 = corta(d, SurfRef::Ref(lado), a, fa);
        let m2 = corta(d, SurfRef::Ref(m1), b, fb);
        arcos.push(d.add_geometry(RefKind::CurveIntersect { a: SurfRef::Ref(m2), b: c, which: 0, near: None }));
        faces.push(m2);
    }
    (faces, arcos)
}

/// Sketch retangular no plano dado.
fn retangulo(d: &mut Document, plane: PlaneRef, a: [f64; 2], b: [f64; 2]) -> Id {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    data.add_rect(&mut al, a, b, false);
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

/// Chapa 100 × 50 × 2 para os casos de chapa.
fn chapa(d: &mut Document) {
    d.sheet = Some(SheetParams { thickness: t("2"), radius: t("2"), k: t("0.44"), flat: false, ..Default::default() });
    let sketch = retangulo(d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch, reverse: false, offset: String::new(), mid: false, surface: None }));
}

pub fn demo(nome: &str, doc: &mut Document) -> Option<DemoPrevia> {
    let d = doc;
    if nome == "sup_chapa_curva" {
        chapa(d);
        let mut data = SketchData::default();
        let mut al = || d.alloc();
        let pts = [[50., 0.], [62., 25.], [50., 50.]].map(|p| data.add_point(&mut al, p[0], p[1])).to_vec();
        data.add_curve(&mut al, sketch::Curve::Spline { pts }, false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        return Some(DemoPrevia::Comando(FeatureKind::SmBend(SmBend { sketch: Some(sk), a: [0.; 3], b: [0.; 3], angle: t("90"), radius: None, flip: false, down: false, position: 0 })));
    }
    if nome == "sup_chapa_recorte" {
        chapa(d);
        d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: t("20"), angle: t("90"), radius: None, off1: t("0"), off2: t("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
        let sk = retangulo(d, PlaneRef::Origin(OriginPlane::XY), [95., 15.], [108., 35.]);
        return Some(DemoPrevia::Comando(FeatureKind::SmCut(SmCut { sketch: sk, mode: 0, offset: String::new(), draft: t("5"), height: t("4"), flip: false, normal_face: false })));
    }
    if nome == "sup_loft_solido" {
        let a = retangulo(d, PlaneRef::Origin(OriginPlane::XY), [-30., -20.], [30., 20.]);
        let p = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "70");
        let b = retangulo(d, PlaneRef::Datum(p), [-15., -15.], [25., 15.]);
        let normal = PontaLoft { condicao: CondicaoPonta::Normal, intensidade: t("1") };
        return Some(DemoPrevia::Comando(FeatureKind::Loft(Loft { sections: vec![a, b], ruled: false, closed: false, op: BoolOp::Add, torcao: vec![0, 0], inicio: normal.clone(), fim: normal })));
    }
    Some(Ref(match nome {
        "sup_preencher" | "sup_preencher_g0" => {
            let (faces, arcos) = buraco_em_esfera(d);
            let cont = if nome == "sup_preencher" { Continuity::G1 } else { Continuity::G0 };
            let conds = faces.iter().map(|f| FillCond { cont, support: Some(SurfRef::Ref(*f)), weight: t("1") }).collect();
            RefKind::SurfFill { curves: arcos.iter().map(|a| CurveRef::Ref(*a)).collect(), conds, points: vec![PointRef::Fixed([0., 0., 99.])], tol: t("0.01") }
        }
        // três lados e uma folga de 2 mm: bolas nas pontas abertas e rótulo da folga
        "sup_preencher_aberto" => {
            let c = [reta(d, [0., 0., 0.], [60., 0., 0.]), reta(d, [60., 0., 0.], [60., 60., 0.]), reta(d, [60., 60., 0.], [2., 60., 0.]), reta(d, [0., 58., 0.], [0., 10., 0.])];
            RefKind::SurfFill { curves: c.to_vec(), conds: vec![], points: vec![], tol: t("0.01") }
        }
        "sup_offset" | "sup_offset_neg" | "sup_offset_dois" | "sup_offset_curva" => {
            let surface = if nome == "sup_offset_curva" { SurfRef::Ref(d.add_geometry(RefKind::SurfSphere { center: PointRef::Origin, radius: t("40") })) } else { folha(d, [-40., 0., 0.], [40., 0., 0.]) };
            RefKind::SurfOffset { surface, dist: t(if nome == "sup_offset_neg" { "-15" } else { "15" }), both: nome == "sup_offset_dois", join: Default::default() }
        }
        "sup_extrusao" => {
            let curve = reta(d, [-40., 0., 0.], [40., 0., 0.]);
            RefKind::SurfExtrude { curve, dir: AxisRef::Origin(OriginAxis::Z), length: t("30"), length2: t("20"), until: None, until2: None }
        }
        "sup_revolucao" => {
            let curve = reta(d, [30., 0., 0.], [50., 0., 40.]);
            RefKind::SurfRevolve { curve, axis: AxisRef::Origin(OriginAxis::Z), angle: t("200"), angle2: t("0") }
        }
        "sup_cilindro" => RefKind::SurfCylinder { point: PointRef::Fixed([0.; 3]), dir: AxisRef::Origin(OriginAxis::Z), radius: t("25"), start: t("0"), end: t("60") },
        "sup_tubo" => {
            let spine = reta(d, [0., 0., 0.], [0., 0., 80.]);
            RefKind::SurfSweep { profile: SweepProfile::Circle(t("15")), spine, mode: Default::default() }
        }
        "sup_extrapolar" => {
            let surface = folha(d, [-40., 0., 0.], [40., 0., 0.]);
            RefKind::SurfExtrapolate { surface, near: PointRef::Origin, length: t("15"), curvature: false, more: vec![], edges: vec![CurveRef::Edge(vec![[-40., 0., 30.], [40., 0., 30.]])], until: None, normal_edge: false, tangent_prop: false }
        }
        "sup_ponto" => {
            let c = reta(d, [-40., 0., 0.], [40., 20., 0.]);
            RefKind::PointOnCurve { curve: c, at: OnCurve::Distance(t("50")), reverse: false }
        }
        _ => return None,
    }))
}
