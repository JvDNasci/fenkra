//! Geometria externa no sketch: converter arestas/vértices/curvas do 3D, interseção de faces e
//! superfícies com o plano do sketch, projeção de outros sketches e de referências. O vínculo
//! fica guardado no sketch (associativo): o cálculo reposiciona a geometria quando o 3D muda.

use crate::picking::Pick;
use scc_core::sketch::{Curve, LinkSrc};
use scc_core::*;
use scc_kernel::Frame;
use scc_solidos::RecomputeResult;

type V3 = [f64; 3];

/// Ponto 3D → coordenadas do plano do sketch.
pub fn uv_of(f: &Frame, p: V3) -> [f64; 2] {
    let y = f.y();
    let d = [p[0] - f.origin[0], p[1] - f.origin[1], p[2] - f.origin[2]];
    [d[0] * f.x[0] + d[1] * f.x[1] + d[2] * f.x[2], d[0] * y[0] + d[1] * y[1] + d[2] * y[2]]
}

/// Polilinha 3D → plano do sketch, com no máximo 64 pontos (círculos ficam fechados).
pub fn to_uv(f: &Frame, pts: &[V3]) -> Vec<[f64; 2]> {
    let uv: Vec<[f64; 2]> = pts.iter().map(|p| uv_of(f, *p)).collect();
    if uv.len() > 64 {
        scc_core::sketch::resample(&uv, 64)
    } else {
        uv
    }
}

fn sd(f: &Frame, p: V3) -> f64 {
    (p[0] - f.origin[0]) * f.normal[0] + (p[1] - f.origin[1]) * f.normal[1] + (p[2] - f.origin[2]) * f.normal[2]
}

fn lerp(a: V3, b: V3, t: f64) -> V3 {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Primeiro ponto em que a polilinha atravessa o plano.
fn crossing(f: &Frame, pts: &[V3]) -> Option<V3> {
    pts.windows(2).find_map(|w| {
        let (a, b) = (sd(f, w[0]), sd(f, w[1]));
        ((a <= 0. && b > 0.) || (a > 0. && b <= 0.)).then(|| lerp(w[0], w[1], a / (a - b)))
    })
}

/// Triângulos (posições, índices) cortados pelo plano → maior polilinha encadeada.
fn mesh_cut(f: &Frame, pos: &[f32], idx: &[u32]) -> Vec<V3> {
    let p = |i: u32| [pos[i as usize * 3] as f64, pos[i as usize * 3 + 1] as f64, pos[i as usize * 3 + 2] as f64];
    let mut segs: Vec<(V3, V3)> = vec![];
    for t in idx.chunks_exact(3) {
        let v = [p(t[0]), p(t[1]), p(t[2])];
        let mut hit = vec![];
        for k in 0..3 {
            let (a, b) = (v[k], v[(k + 1) % 3]);
            let (da, db) = (sd(f, a), sd(f, b));
            if (da <= 0. && db > 0.) || (da > 0. && db <= 0.) {
                hit.push(lerp(a, b, da / (da - db)));
            }
        }
        if hit.len() == 2 {
            segs.push((hit[0], hit[1]));
        }
    }
    chain(segs)
}

/// Encadeia segmentos soltos pelas pontas; devolve a cadeia mais longa.
fn chain(mut segs: Vec<(V3, V3)>) -> Vec<V3> {
    let close = |a: V3, b: V3| (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs() < 1e-4;
    let mut best: Vec<V3> = vec![];
    while let Some((a, b)) = segs.pop() {
        let mut c = vec![a, b];
        loop {
            let last = *c.last().unwrap();
            let first = c[0];
            if let Some(i) = segs.iter().position(|s| close(s.0, last) || close(s.1, last)) {
                let s = segs.swap_remove(i);
                c.push(if close(s.0, last) { s.1 } else { s.0 });
            } else if let Some(i) = segs.iter().position(|s| close(s.0, first) || close(s.1, first)) {
                let s = segs.swap_remove(i);
                c.insert(0, if close(s.0, first) { s.1 } else { s.0 });
            } else {
                break;
            }
        }
        let len = |c: &Vec<V3>| c.windows(2).map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2) + (w[1][2] - w[0][2]).powi(2)).sqrt()).sum::<f64>();
        if len(&c) > len(&best) {
            best = c;
        }
    }
    best
}

/// Origem e polilinha 3D do item clicado (converter = projetar; interseção = corte pelo plano).
pub fn world(r: &RecomputeResult, p: &Pick, intersect: bool, f: &Frame) -> Option<(LinkSrc, Vec<V3>)> {
    match *p {
        Pick::Edge { body, edge, mid } => {
            let pts = crate::picking::edge_polyline(r, body, edge);
            let src = LinkSrc::Edge { mid: mid.map(|v| v as f64) };
            if intersect {
                crossing(f, &pts).map(|q| (src, vec![q]))
            } else {
                Some((src, pts))
            }
        }
        Pick::Vertex { pos, .. } | Pick::SkPoint { pos, .. } | Pick::CurveVtx { pos, .. } => {
            let q = pos.map(|v| v as f64);
            Some((LinkSrc::Vertex { pos: q }, vec![q]))
        }
        // curva de outro sketch: projeção (ou interseção) associativa
        Pick::SkCurve { sketch, curve, .. } => {
            let s = r.sketches.iter().find(|s| s.feature == sketch)?;
            let (_, _, pts) = s.ents.iter().find(|(i, ..)| *i == curve)?;
            let p3: Vec<V3> = pts.iter().map(|q| q.map(|v| v as f64)).collect();
            let src = LinkSrc::SketchCurve { sketch, curve };
            if intersect {
                crossing(f, &p3).map(|q| (src, vec![q]))
            } else {
                Some((src, p3))
            }
        }
        Pick::Face { body, face, point, .. } if intersect => {
            let m = &r.bodies.get(body)?.mesh;
            let s0 = *m.face_ranges.get(face)? as usize;
            let s1 = m.face_ranges.get(face + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
            let pts = mesh_cut(f, &m.positions, &m.indices[s0..s1]);
            (pts.len() >= 2).then(|| (LinkSrc::Face { point: point.map(|v| v as f64) }, pts))
        }
        Pick::Geo(id) => {
            if let Some(pt) = r.points.iter().find(|x| x.id == id) {
                return Some((LinkSrc::Ref(id), vec![pt.p]));
            }
            if let Some(c) = r.curves.iter().find(|x| x.id == id) {
                return if intersect { crossing(f, &c.pts).map(|q| (LinkSrc::Ref(id), vec![q])) } else { Some((LinkSrc::Ref(id), c.pts.clone())) };
            }
            if let Some(s) = r.surfaces.iter().find(|x| x.id == id) {
                let pts = mesh_cut(f, &s.mesh.positions, &s.mesh.indices);
                return (pts.len() >= 2).then(|| (LinkSrc::Ref(id), pts));
            }
            None
        }
        _ => None,
    }
}

/// Curvas de outro sketch no mundo: (id da curva, polilinha).
pub fn sketch_curves(doc: &Document, r: &RecomputeResult, sketch: Id) -> Vec<(Id, Vec<V3>)> {
    let Some(FeatureKind::Sketch(s)) = doc.feature(sketch).map(|f| &f.kind) else { return vec![] };
    let Some(fr) = r.sketches.iter().find(|v| v.feature == sketch).map(|v| v.frame) else { return vec![] };
    let d = &s.data;
    let to3 = |q: [f64; 2]| fr.point(q[0], q[1]);
    d.curves
        .iter()
        .filter_map(|c| {
            let pts2: Vec<[f64; 2]> = match c.geom {
                Curve::Spline { .. } => scc_core::sketch::spline_samples(&d.spline_uv(c.id), 12),
                Curve::Line { a, b } => vec![d.pos(a), d.pos(b)],
                Curve::Point { p } => vec![d.pos(p)],
                Curve::Circle { c: ctr, r } => {
                    let cc = d.pos(ctr);
                    (0..=48).map(|i| {
                        let t = i as f64 / 48. * std::f64::consts::TAU;
                        [cc[0] + r * t.cos(), cc[1] + r * t.sin()]
                    })
                    .collect()
                }
                Curve::Arc { c: ctr, s, e } => {
                    let (pc, ps, pe) = (d.pos(ctr), d.pos(s), d.pos(e));
                    let rr = ((ps[0] - pc[0]).powi(2) + (ps[1] - pc[1]).powi(2)).sqrt();
                    let a0 = (ps[1] - pc[1]).atan2(ps[0] - pc[0]);
                    let mut a1 = (pe[1] - pc[1]).atan2(pe[0] - pc[0]);
                    while a1 <= a0 {
                        a1 += std::f64::consts::TAU;
                    }
                    (0..=32).map(|i| {
                        let t = a0 + (a1 - a0) * i as f64 / 32.;
                        [pc[0] + rr * t.cos(), pc[1] + rr * t.sin()]
                    })
                    .collect()
                }
            };
            Some((c.id, pts2.into_iter().map(to3).collect()))
        })
        .collect()
}
