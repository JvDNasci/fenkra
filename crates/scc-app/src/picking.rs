//! Seleção na viewport: faces, arestas, vértices e planos (com filtro de seleção).
//! Prioridade: vértice > aresta > face/plano, respeitando o que está na frente.

use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use glam::Vec3;
use scc_core::{Document, Id, OriginPlane, PlaneRef};
use scc_solidos::RecomputeResult;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pick {
    Face { body: usize, face: usize, owner: Id, point: [f32; 3], normal: [f32; 3] },
    Edge { body: usize, edge: usize, mid: [f32; 3] },
    Vertex { body: usize, pos: [f32; 3] },
    Plane(PlaneRef),
    /// ponto, linha ou curva de referência
    Geo(Id),
}

impl Pick {
    pub fn label(&self, doc: &Document) -> String {
        match self {
            Pick::Face { owner, .. } => format!("Face de {}", doc.name_of(*owner)),
            Pick::Edge { edge, .. } => format!("Aresta #{edge}"),
            Pick::Vertex { pos, .. } => format!("Vértice ({:.1}, {:.1}, {:.1})", pos[0], pos[1], pos[2]),
            Pick::Plane(p) => crate::scene::plane_ref_name(doc, p),
            Pick::Geo(id) => doc.name_of(*id),
        }
    }
}

pub struct Filter {
    pub faces: bool,
    pub edges: bool,
    pub vertices: bool,
    pub planes: bool,
}

fn v3(p: &[f32]) -> Vec3 {
    Vec3::new(p[0], p[1], p[2])
}

fn seg_dist(p: Pos2, a: Pos2, b: Pos2) -> (f32, f32) {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0., 1.);
    ((a + ab * t - p).length(), t)
}

/// Interseção raio–triângulo (Möller–Trumbore).
fn ray_tri(o: Vec3, d: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let (e1, e2) = (b - a, c - a);
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1. / det;
    let s = o - a;
    let u = s.dot(p) * inv;
    if !(0. ..=1.).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < 0. || u + v > 1. {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 0.).then_some(t)
}

/// Tamanho de referência dos planos (igual ao usado na cena).
pub fn plane_size(r: &RecomputeResult) -> f32 {
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for b in &r.bodies {
        for p in b.mesh.positions.chunks_exact(3) {
            lo = lo.min(v3(p));
            hi = hi.max(v3(p));
        }
    }
    if lo.x > hi.x {
        60.
    } else {
        (hi - lo).max_element().max(20.) * 0.6
    }
}

pub fn pick(r: &RecomputeResult, doc: &Document, cam: &Camera, rect: Rect, pos: Pos2, f: &Filter) -> Option<Pick> {
    let (o, d) = cam.ray(rect, pos);
    let visible = |ci: usize| doc.collectors.iter().find(|c| c.id == r.bodies[ci].collector).is_none_or(|c| c.visible);

    // faces (também dão a profundidade para esconder o que está atrás)
    let mut best_face: Option<(f32, usize, usize)> = None;
    for (bi, b) in r.bodies.iter().enumerate() {
        if !visible(bi) {
            continue;
        }
        let m = &b.mesh;
        for (ti, t) in m.indices.chunks_exact(3).enumerate() {
            let p = |i: u32| v3(&m.positions[i as usize * 3..i as usize * 3 + 3]);
            if let Some(dist) = ray_tri(o, d, p(t[0]), p(t[1]), p(t[2])) {
                if best_face.is_none_or(|(bd, _, _)| dist < bd) {
                    best_face = Some((dist, bi, ti));
                }
            }
        }
    }
    let depth_limit = best_face.map(|(t, ..)| t * 1.002 + 0.05).unwrap_or(f32::MAX);
    let depth = |p: Vec3| (p - o).dot(d);

    if f.vertices {
        let mut best: Option<(f32, Pick)> = None;
        for (bi, b) in r.bodies.iter().enumerate() {
            if !visible(bi) {
                continue;
            }
            for p in b.mesh.vertices.chunks_exact(3) {
                let w = v3(p);
                let Some(s) = cam.project(rect, w) else { continue };
                let dd = (s - pos).length();
                if dd < 8. && depth(w) <= depth_limit && best.is_none_or(|(bd, _)| dd < bd) {
                    best = Some((dd, Pick::Vertex { body: bi, pos: [p[0], p[1], p[2]] }));
                }
            }
        }
        if let Some((_, p)) = best {
            return Some(p);
        }
    }
    // pontos e curvas de referência
    if f.planes {
        let mut best: Option<(f32, Pick)> = None;
        for p in &r.points {
            let w = Vec3::from(p.p.map(|v| v as f32));
            let Some(s) = cam.project(rect, w) else { continue };
            let dd = (s - pos).length();
            if dd < 8. && depth(w) <= depth_limit && best.is_none_or(|(bd, _)| dd < bd) {
                best = Some((dd, Pick::Geo(p.id)));
            }
        }
        if best.is_none() {
            for c in &r.curves {
                for w in c.pts.windows(2) {
                    let (a, b) = (Vec3::from(w[0].map(|v| v as f32)), Vec3::from(w[1].map(|v| v as f32)));
                    let (Some(sa), Some(sb)) = (cam.project(rect, a), cam.project(rect, b)) else { continue };
                    let (dd, t) = seg_dist(pos, sa, sb);
                    if dd < 6. && depth(a.lerp(b, t)) <= depth_limit && best.is_none_or(|(bd, _)| dd < bd) {
                        best = Some((dd, Pick::Geo(c.id)));
                    }
                }
            }
        }
        if let Some((_, p)) = best {
            return Some(p);
        }
        // superfícies de referência (raio contra os triângulos)
        let mut hit: Option<(f32, Id)> = None;
        for sf in &r.surfaces {
            let m = &sf.mesh;
            for t in m.indices.chunks_exact(3) {
                let p = |i: u32| v3(&m.positions[i as usize * 3..i as usize * 3 + 3]);
                if let Some(dist) = ray_tri(o, d, p(t[0]), p(t[1]), p(t[2])) {
                    if dist <= depth_limit && hit.is_none_or(|(bd, _)| dist < bd) {
                        hit = Some((dist, sf.id));
                    }
                }
            }
        }
        if let Some((_, id)) = hit {
            return Some(Pick::Geo(id));
        }
    }
    if f.edges {
        let mut best: Option<(f32, Pick)> = None;
        for (bi, b) in r.bodies.iter().enumerate() {
            if !visible(bi) {
                continue;
            }
            let m = &b.mesh;
            for (si, sgm) in m.edges.chunks_exact(6).enumerate() {
                let (a, c) = (v3(&sgm[0..3]), v3(&sgm[3..6]));
                let (Some(sa), Some(sc)) = (cam.project(rect, a), cam.project(rect, c)) else { continue };
                let (dd, t) = seg_dist(pos, sa, sc);
                if dd < 6. && depth(a.lerp(c, t)) <= depth_limit && best.is_none_or(|(bd, _)| dd < bd) {
                    let e = m.edge_ranges.partition_point(|&x| x as usize <= si).saturating_sub(1);
                    let mid = m.edge_mids.get(e * 3..e * 3 + 3).map(|x| [x[0], x[1], x[2]]).unwrap_or([0.; 3]);
                    best = Some((dd, Pick::Edge { body: bi, edge: e, mid }));
                }
            }
        }
        if let Some((_, p)) = best {
            return Some(p);
        }
    }
    if f.faces {
        if let Some((t, bi, ti)) = best_face {
            let b = &r.bodies[bi];
            let face = b.mesh.face_ranges.partition_point(|&x| x as usize <= ti * 3).saturating_sub(1);
            let owner = b.face_owner.get(face).copied().unwrap_or(Id(0));
            let idx = &b.mesh.indices[ti * 3..ti * 3 + 3];
            let p = |i: u32| v3(&b.mesh.positions[i as usize * 3..i as usize * 3 + 3]);
            let n = (p(idx[1]) - p(idx[0])).cross(p(idx[2]) - p(idx[0])).normalize_or_zero();
            let n = if n.dot(d) > 0. { -n } else { n };
            let hit = o + d * t;
            return Some(Pick::Face { body: bi, face, owner, point: hit.into(), normal: n.into() });
        }
    }
    if f.planes {
        // planos: clique dentro do quadrado desenhado
        let size = plane_size(r);
        let mut best: Option<(f32, Pick)> = None;
        for pl in &r.planes {
            let fr = &pl.frame;
            let n = Vec3::from(fr.normal.map(|x| x as f32));
            let po = Vec3::from(fr.origin.map(|x| x as f32));
            let den = d.dot(n);
            if den.abs() < 1e-6 {
                continue;
            }
            let t = (po - o).dot(n) / den;
            if t <= 0. || t > depth_limit {
                continue;
            }
            let hit = o + d * t - po;
            let (x, y) = (Vec3::from(fr.x.map(|v| v as f32)), Vec3::from(fr.y().map(|v| v as f32)));
            let h = size * if pl.id.is_some() { 0.45 } else { 0.5 };
            if hit.dot(x).abs() <= h && hit.dot(y).abs() <= h && best.is_none_or(|(bt, _)| t < bt) {
                let pr = match pl.id {
                    Some(id) => PlaneRef::Datum(id),
                    None => PlaneRef::Origin(match pl.name.as_str() {
                        "XY" => OriginPlane::XY,
                        "XZ" => OriginPlane::XZ,
                        _ => OriginPlane::YZ,
                    }),
                };
                best = Some((t, Pick::Plane(pr)));
            }
        }
        return best.map(|b| b.1);
    }
    None
}

/// Ponto mais próximo de `p` no triângulo abc (Ericson).
fn closest_on_tri(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0. && d2 <= 0. {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0. && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0. && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let den = 1. / (va + vb + vc);
    a + ab * (vb * den) + ac * (vc * den)
}

/// Face (índice na malha do corpo) que contém o ponto.
pub fn face_at(r: &RecomputeResult, body: usize, p: [f64; 3]) -> Option<usize> {
    let b = r.bodies.get(body)?;
    let m = &b.mesh;
    let q = Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
    let mut best: Option<(f32, usize)> = None;
    for (ti, t) in m.indices.chunks_exact(3).enumerate() {
        let v = |i: u32| v3(&m.positions[i as usize * 3..i as usize * 3 + 3]);
        let d = (closest_on_tri(q, v(t[0]), v(t[1]), v(t[2])) - q).length();
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, ti));
        }
    }
    let (_, ti) = best?;
    Some(m.face_ranges.partition_point(|&x| x as usize <= ti * 3).saturating_sub(1))
}

/// Os dois pontos estão na mesma face do corpo?
pub fn same_face(r: &RecomputeResult, body: usize, a: [f64; 3], b: [f64; 3]) -> bool {
    matches!((face_at(r, body, a), face_at(r, body, b)), (Some(x), Some(y)) if x == y)
}

/// Polilinha de uma aresta do corpo (na ordem dos segmentos da malha).
pub fn edge_polyline(r: &RecomputeResult, body: usize, edge: usize) -> Vec<[f64; 3]> {
    let Some(b) = r.bodies.get(body) else { return vec![] };
    let m = &b.mesh;
    let Some(&s0) = m.edge_ranges.get(edge) else { return vec![] };
    let s1 = m.edge_ranges.get(edge + 1).map(|x| *x as usize).unwrap_or(m.edges.len() / 6);
    let mut out: Vec<[f64; 3]> = vec![];
    for sgm in m.edges.chunks_exact(6).take(s1).skip(s0 as usize) {
        let a = [sgm[0] as f64, sgm[1] as f64, sgm[2] as f64];
        let c = [sgm[3] as f64, sgm[4] as f64, sgm[5] as f64];
        if out.is_empty() {
            out.push(a);
        }
        out.push(c);
    }
    out
}

/// Realce das seleções (cheio) e da pré-seleção (mais claro) por cima da viewport.
pub fn draw(painter: &egui::Painter, r: &RecomputeResult, cam: &Camera, rect: Rect, p: &Pick, strong: bool) {
    let col = if strong { Color32::from_rgb(255, 190, 40) } else { Color32::from_rgba_unmultiplied(120, 220, 255, 200) };
    let fillc = if strong { Color32::from_rgba_unmultiplied(255, 190, 40, 90) } else { Color32::from_rgba_unmultiplied(120, 220, 255, 60) };
    match *p {
        Pick::Face { body, face, .. } => {
            let Some(b) = r.bodies.get(body) else { return };
            let m = &b.mesh;
            let Some(&start) = m.face_ranges.get(face) else { return };
            let end = m.face_ranges.get(face + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
            let mut mesh = egui::Mesh::default();
            for t in m.indices[start as usize..end].chunks_exact(3) {
                let pts: Vec<Option<Pos2>> = t.iter().map(|i| cam.project(rect, v3(&m.positions[*i as usize * 3..*i as usize * 3 + 3]))).collect();
                if let [Some(a), Some(b2), Some(c)] = pts[..] {
                    let base = mesh.vertices.len() as u32;
                    for q in [a, b2, c] {
                        mesh.colored_vertex(q, fillc);
                    }
                    mesh.add_triangle(base, base + 1, base + 2);
                }
            }
            painter.add(egui::Shape::mesh(mesh));
        }
        Pick::Edge { body, edge, .. } => {
            let Some(b) = r.bodies.get(body) else { return };
            let m = &b.mesh;
            let Some(&s0) = m.edge_ranges.get(edge) else { return };
            let s1 = m.edge_ranges.get(edge + 1).map(|x| *x as usize).unwrap_or(m.edges.len() / 6);
            for sgm in m.edges.chunks_exact(6).take(s1).skip(s0 as usize) {
                if let (Some(a), Some(c)) = (cam.project(rect, v3(&sgm[0..3])), cam.project(rect, v3(&sgm[3..6]))) {
                    painter.line_segment([a, c], Stroke::new(if strong { 3.5 } else { 2.5 }, col));
                }
            }
        }
        Pick::Vertex { pos, .. } => {
            if let Some(s) = cam.project(rect, v3(&pos)) {
                painter.circle_filled(s, if strong { 5. } else { 4. }, col);
            }
        }
        Pick::Geo(id) => {
            if let Some(pt) = r.points.iter().find(|p| p.id == id) {
                if let Some(s) = cam.project(rect, Vec3::from(pt.p.map(|v| v as f32))) {
                    painter.circle_stroke(s, if strong { 7. } else { 6. }, Stroke::new(2.5, col));
                }
            }
            if let Some(c) = r.curves.iter().find(|c| c.id == id) {
                let pts: Vec<Pos2> = c.pts.iter().filter_map(|p| cam.project(rect, Vec3::from(p.map(|v| v as f32)))).collect();
                painter.add(egui::Shape::line(pts, Stroke::new(if strong { 3.5 } else { 2.5 }, col)));
            }
        }
        Pick::Plane(pr) => {
            let size = plane_size(r);
            let pl = r.planes.iter().find(|pl| match pr {
                PlaneRef::Datum(id) => pl.id == Some(id),
                PlaneRef::Origin(o) => pl.id.is_none() && pl.name == crate::scene::plane_name(o),
            });
            if let Some(pl) = pl {
                let fr = &pl.frame;
                let o = Vec3::from(fr.origin.map(|v| v as f32));
                let (x, y) = (Vec3::from(fr.x.map(|v| v as f32)), Vec3::from(fr.y().map(|v| v as f32)));
                let h = size * if pl.id.is_some() { 0.45 } else { 0.5 };
                let c: Vec<Pos2> = [x + y, x - y, -x - y, -x + y].iter().filter_map(|k| cam.project(rect, o + *k * h)).collect();
                if c.len() == 4 {
                    painter.add(egui::Shape::convex_polygon(c, fillc, Stroke::new(2., col)));
                }
            }
        }
    }
}
