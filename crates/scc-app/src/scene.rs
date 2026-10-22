//! Converte o resultado do recompute em uma cena para a viewport.

use crate::app::Sel;
use crate::render::{LineVertex, MeshVertex, Scene};
use glam::Vec3;
use scc_core::{Document, FeatureKind, Id, OriginPlane, PlaneRef};
use scc_solidos::RecomputeResult;
use std::collections::HashSet;

const EDGE: [f32; 4] = [0.06, 0.07, 0.09, 1.0];
const SKETCH: [f32; 4] = [0.35, 0.75, 1.0, 1.0];
const CONSTR: [f32; 4] = [0.85, 0.6, 0.3, 0.8];
const HILITE: [f32; 4] = [1.0, 0.85, 0.2, 1.0];

fn lv(p: [f32; 3], col: [f32; 4]) -> LineVertex {
    LineVertex { pos: p, col }
}

fn grow(b: &mut Option<(Vec3, Vec3)>, p: Vec3) {
    *b = Some(match *b {
        None => (p, p),
        Some((lo, hi)) => (lo.min(p), hi.max(p)),
    });
}

/// Sketch exibido: o selecionado, os usados pelo comando selecionado e os que ainda não foram consumidos.
fn shown_sketches(doc: &Document, sel: &Sel) -> (HashSet<Id>, HashSet<Id>) {
    let mut consumed = HashSet::new();
    for f in &doc.features {
        consumed.extend(Document::refs_of(&f.kind));
    }
    let mut hi = HashSet::new();
    if let Sel::Feature(id) = sel {
        if let Some(f) = doc.feature(*id) {
            if matches!(f.kind, FeatureKind::Sketch(_)) {
                hi.insert(*id);
            } else {
                hi.extend(Document::refs_of(&f.kind));
            }
        }
    }
    let shown = doc
        .features
        .iter()
        .filter(|f| matches!(f.kind, FeatureKind::Sketch(_)) && (!consumed.contains(&f.id) || hi.contains(&f.id)))
        .map(|f| f.id)
        .collect();
    (shown, hi)
}

pub fn build(doc: &Document, r: &RecomputeResult, sel: &Sel) -> Scene {
    let mut s = Scene::default();
    let mut bounds = None;

    for b in &r.bodies {
        let visible = doc.collectors.iter().find(|c| c.id == b.collector).is_none_or(|c| c.visible);
        if !visible {
            continue;
        }
        let hi = matches!(sel, Sel::Collector(c) if *c == b.collector);
        let k = |x: u8| x as f32 / 255.;
        let mut col = [k(b.color[0]), k(b.color[1]), k(b.color[2]), 1.0];
        if hi {
            col = [col[0] * 0.6 + 0.4, col[1] * 0.6 + 0.34, col[2] * 0.6 + 0.08, 1.0];
        }
        let base = s.mesh.len() as u32;
        for (p, n) in b.mesh.positions.chunks_exact(3).zip(b.mesh.normals.chunks_exact(3)) {
            s.mesh.push(MeshVertex { pos: [p[0], p[1], p[2]], nrm: [n[0], n[1], n[2]], col });
            grow(&mut bounds, Vec3::new(p[0], p[1], p[2]));
        }
        s.indices.extend(b.mesh.indices.iter().map(|i| i + base));
        for p in b.mesh.edges.chunks_exact(3) {
            s.lines.push(lv([p[0], p[1], p[2]], EDGE));
        }
    }

    let (shown, hi) = shown_sketches(doc, sel);
    for sk in &r.sketches {
        if !shown.contains(&sk.feature) {
            continue;
        }
        let h = hi.contains(&sk.feature);
        let dst = if h { &mut s.overlay } else { &mut s.lines };
        for p in &sk.lines {
            dst.push(lv(*p, if h { HILITE } else { SKETCH }));
            grow(&mut bounds, Vec3::from(*p));
        }
        for p in &sk.construction {
            dst.push(lv(*p, CONSTR));
        }
    }

    // tamanho de referência para planos, pontos e eixos
    let size = bounds.map(|(lo, hi): (Vec3, Vec3)| (hi - lo).max_element().max(20.) * 0.6).unwrap_or(60.);

    for sk in &r.sketches {
        if !shown.contains(&sk.feature) {
            continue;
        }
        let h = hi.contains(&sk.feature);
        let d = size * 0.025;
        for p in &sk.points {
            let c = Vec3::from(*p);
            for ax in [Vec3::X, Vec3::Y, Vec3::Z] {
                s.overlay.push(lv((c - ax * d).into(), if h { HILITE } else { SKETCH }));
                s.overlay.push(lv((c + ax * d).into(), if h { HILITE } else { SKETCH }));
            }
        }
    }

    // planos da Origem e Referências
    for p in &r.planes {
        let selected = match (sel, p.id) {
            (Sel::Ref(a), Some(b)) => *a == b,
            (Sel::Origin(o), None) => p.name == plane_name(*o),
            _ => false,
        };
        let col = match (p.id, p.name.as_str()) {
            (Some(_), _) => [0.95, 0.7, 0.3, 0.55],
            (_, "XY") => [0.4, 0.55, 1.0, 0.45],
            (_, "XZ") => [0.4, 0.85, 0.45, 0.45],
            _ => [1.0, 0.45, 0.4, 0.45],
        };
        let f = &p.frame;
        let y = f.y();
        let (o, x, yv) = (Vec3::from(f.origin.map(|v| v as f32)), Vec3::from(f.x.map(|v| v as f32)), Vec3::from(y.map(|v| v as f32)));
        let h = size * if p.id.is_some() { 0.45 } else { 0.5 };
        let c = [o + (x + yv) * h, o + (x - yv) * h, o + (-x - yv) * h, o + (-x + yv) * h];
        let dst = if selected { &mut s.overlay } else { &mut s.lines };
        for i in 0..4 {
            let colr = if selected { HILITE } else { col };
            dst.push(lv(c[i].into(), colr));
            dst.push(lv(c[(i + 1) % 4].into(), colr));
        }
    }

    // eixos da Origem
    if doc.origin_visible {
        for (ax, col) in [(Vec3::X, [1., 0.3, 0.3, 1.]), (Vec3::Y, [0.3, 0.9, 0.3, 1.]), (Vec3::Z, [0.35, 0.55, 1., 1.])] {
            s.overlay.push(lv([0.; 3], col));
            s.overlay.push(lv((ax * size * 0.25).into(), col));
        }
    }

    s.bounds = bounds.or(Some((Vec3::splat(-size), Vec3::splat(size))));
    s
}

pub fn plane_name(o: OriginPlane) -> &'static str {
    match o {
        OriginPlane::XY => "XY",
        OriginPlane::XZ => "XZ",
        OriginPlane::YZ => "YZ",
    }
}

pub fn plane_ref_name(doc: &Document, p: &PlaneRef) -> String {
    match p {
        PlaneRef::Origin(o) => format!("Origem · {}", plane_name(*o)),
        PlaneRef::Datum(id) => doc.reference(*id).map(|r| r.name.clone()).unwrap_or_else(|| format!("⚠ perdida #{}", id.0)),
    }
}
