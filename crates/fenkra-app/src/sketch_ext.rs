// SPDX-License-Identifier: LGPL-2.1-or-later
//! Geometria externa no sketch: converter arestas/vértices/curvas do 3D, interseção de faces e
//! superfícies com o plano do sketch, projeção de outros sketches e de referências. O vínculo
//! fica guardado no sketch (associativo): o cálculo reposiciona a geometria quando o 3D muda.

use crate::picking::Pick;
use fenkra_core::projecao::{corta, projeta, reconhece, Curva3};
use fenkra_core::sketch::{Curve, LinkPolicy, LinkSrc};
use fenkra_core::*;
use fenkra_kernel::Frame;
use fenkra_solidos::RecomputeResult;

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
        fenkra_core::sketch::resample(&uv, 64)
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

/// Todos os pontos em que a curva encontra o plano do sketch. Com fórmula, o corte é exato; sem ela,
/// vale a travessia da polilinha, e o recálculo refina a aresta de corpo pelo kernel.
fn crossings(f: &Frame, curva: &Curva3, pts: &[V3]) -> Vec<V3> {
    corta(curva, &fenkra_solidos::profile::plano_do(f)).unwrap_or_else(|| fenkra_core::sketch::plane_crossings(pts, f.origin, f.normal))
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

/// O que o item clicado vira no sketch.
pub struct External {
    pub src: LinkSrc,
    pub pts: Vec<V3>,
    /// pontos soltos (travessias de uma curva pelo plano), não uma polilinha
    pub isolated: bool,
    /// a fórmula da curva, quando se conhece: o sketch recebe a forma exata
    pub curva: Curva3,
}

impl External {
    /// Polilinha exata (outro sketch, referência): a fórmula é reconhecida nos pontos.
    pub fn line(src: LinkSrc, pts: Vec<V3>) -> Self {
        let curva = reconhece(&pts);
        External { src, pts, isolated: false, curva }
    }
    /// Travessias da curva pelo plano; nenhuma, nada a trazer.
    fn cut(src: LinkSrc, f: &Frame, curva: Curva3, curve: &[V3]) -> Option<Self> {
        let pts = crossings(f, &curva, curve);
        (!pts.is_empty()).then_some(External { src, pts, isolated: true, curva })
    }
    /// Nome da forma que vai nascer, para a dica junto ao cursor.
    pub fn forma(&self, f: &Frame) -> Option<&'static str> {
        (!self.isolated).then(|| projeta(&self.curva, &fenkra_solidos::profile::plano_do(f))).flatten().map(|x| x.nome())
    }
    /// Grava no sketch como geometria externa.
    pub fn add_to(self, sk: &mut SketchData, al: &mut impl FnMut() -> Id, f: &Frame, intersect: bool, construction: bool) -> Option<Id> {
        let uv = to_uv(f, &self.pts);
        let exata = projeta(&self.curva, &fenkra_solidos::profile::plano_do(f));
        let id = match (self.isolated, exata) {
            (true, _) => sk.add_link_points(al, self.src, &uv, construction),
            (false, Some(forma)) => Some(sk.add_link_exata(al, self.src, intersect, &forma, construction)),
            (false, None) => sk.add_link(al, self.src, intersect, &uv, construction),
        };
        // instantâneo: nasce solta, como geometria comum
        if let (LinkPolicy::Instantaneo, Some(l)) = (sk.link_policy, id) {
            sk.release_link(l);
        }
        id
    }
}

/// Origem e geometria 3D do item clicado (converter = projetar; interseção = corte pelo plano).
pub fn world(doc: &Document, r: &RecomputeResult, p: &Pick, intersect: bool, f: &Frame) -> Option<External> {
    match *p {
        Pick::Edge { body, edge, mid } => {
            let pts = crate::picking::edge_polyline(r, body, edge);
            let src = LinkSrc::Edge { mid: mid.map(|v| v as f64) };
            // o kernel diz o tipo da aresta: a polilinha da malha é só uma aproximação dela
            let curva = match r.bodies.get(body).and_then(|b| b.mesh.edge_formula(edge)) {
                Some(fenkra_kernel::EdgeFormula::Line { a, b }) => Curva3::Reta { a, b },
                Some(fenkra_kernel::EdgeFormula::Conic { c, u, v, t0, t1 }) => Curva3::Conica { c, u, v, t0, t1 },
                None => Curva3::Outra,
            };
            if intersect {
                External::cut(src, f, curva, &pts)
            } else {
                Some(External { src, pts, isolated: false, curva })
            }
        }
        Pick::Vertex { pos, .. } | Pick::SkPoint { pos, .. } | Pick::CurveVtx { pos, .. } => {
            let q = pos.map(|v| v as f64);
            Some(External::line(LinkSrc::Vertex { pos: q }, vec![q]))
        }
        // curva de outro sketch: projeção (ou interseção) associativa
        Pick::SkCurve { sketch, curve, .. } => {
            // os pontos em precisão dupla: a fórmula é reconhecida neles
            let p3 = sketch_curves(doc, r, sketch).into_iter().find(|x| x.0 == curve)?.1;
            let src = LinkSrc::SketchCurve { sketch, curve };
            if intersect {
                External::cut(src, f, reconhece(&p3), &p3)
            } else {
                Some(External::line(src, p3))
            }
        }
        Pick::Face { body, face, point, .. } if intersect => {
            let m = &r.bodies.get(body)?.mesh;
            let s0 = *m.face_ranges.get(face)? as usize;
            let s1 = m.face_ranges.get(face + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
            let pts = mesh_cut(f, &m.positions, &m.indices[s0..s1]);
            (pts.len() >= 2).then(|| External::line(LinkSrc::Face { point: point.map(|v| v as f64) }, pts))
        }
        Pick::Geo(id) => {
            if let Some(pt) = r.points.iter().find(|x| x.id == id) {
                return Some(External::line(LinkSrc::Ref(id), vec![pt.p]));
            }
            if let Some(c) = r.curves.iter().find(|x| x.id == id) {
                return if intersect { External::cut(LinkSrc::Ref(id), f, reconhece(&c.pts), &c.pts) } else { Some(External::line(LinkSrc::Ref(id), c.pts.clone())) };
            }
            if let Some(s) = r.surfaces.iter().find(|x| x.id == id) {
                let pts = mesh_cut(f, &s.mesh.positions, &s.mesh.indices);
                return (pts.len() >= 2).then(|| External::line(LinkSrc::Ref(id), pts));
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
                Curve::Spline { .. } => fenkra_core::sketch::spline_samples(&d.spline_uv(c.id), 12),
                Curve::Line { a, b } => vec![d.pos(a), d.pos(b)],
                Curve::Point { p } => vec![d.pos(p)],
                Curve::Ellipse { c: ctr, major, minor, t0, t1, .. } => fenkra_core::projecao::elipse_amostras(d.pos(ctr), major, minor, t0, t1),
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

#[cfg(test)]
mod testes {
    use super::*;

    const XY: Frame = Frame { origin: [0., 0., 0.], x: [1., 0., 0.], normal: [0., 0., 1.] };

    fn ids() -> impl FnMut() -> Id {
        let mut n = 40u64;
        move || {
            n += 1;
            Id(n)
        }
    }

    /// Círculo de raio 10 em torno de z = 5, inclinado 60° em torno de x.
    fn inclinado() -> External {
        let (s, c) = (60f64.to_radians().sin(), 60f64.to_radians().cos());
        let curva = Curva3::Conica { c: [0., 0., 5.], u: [10., 0., 0.], v: [0., 10. * c, 10. * s], t0: 0., t1: std::f64::consts::TAU };
        External { src: LinkSrc::Edge { mid: [0.; 3] }, pts: vec![[10., 0., 5.], [0., 5., 13.]], isolated: false, curva }
    }

    #[test]
    fn aresta_com_formula_nasce_com_a_forma_exata() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        assert_eq!(inclinado().forma(&XY), Some("elipse"));
        let l = inclinado().add_to(&mut sk, &mut al, &XY, false, false).unwrap();
        assert!(sk.links.iter().any(|x| x.id == l));
        assert!(matches!(sk.curves[0].geom, Curve::Ellipse { minor, .. } if (minor - 5.).abs() < 1e-9));
    }

    #[test]
    fn politica_do_sketch_decide_o_vinculo_novo() {
        let (mut sk, mut al) = (SketchData { link_policy: LinkPolicy::Instantaneo, ..Default::default() }, ids());
        inclinado().add_to(&mut sk, &mut al, &XY, false, false).unwrap();
        assert!(sk.links.is_empty() && sk.points.iter().all(|p| !p.fixed), "instantânea nasce solta");
        let (mut sk, mut al) = (SketchData { link_policy: LinkPolicy::Isolado, ..Default::default() }, ids());
        inclinado().add_to(&mut sk, &mut al, &XY, false, false).unwrap();
        assert_eq!(sk.links[0].policy, LinkPolicy::Isolado);
    }

    #[test]
    fn curva_sem_formula_fica_como_polilinha() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let pts: Vec<V3> = (0..12).map(|i| [i as f64, (i as f64 * 0.8).sin() * 4., 0.]).collect();
        let x = External::line(LinkSrc::Edge { mid: [0.; 3] }, pts);
        assert_eq!(x.forma(&XY), None);
        x.add_to(&mut sk, &mut al, &XY, false, false).unwrap();
        assert!(matches!(sk.links[0].shape, fenkra_core::sketch::LinkShape::Poly { .. }));
    }

    #[test]
    fn interseccao_usa_a_formula_e_acha_todos_os_pontos() {
        let curva = Curva3::Conica { c: [0., 0., 0.], u: [10., 0., 0.], v: [0., 0., 10.], t0: 0., t1: std::f64::consts::TAU };
        let plano = Frame { origin: [0., 0., 6.], ..XY };
        // a polilinha dada é grosseira de propósito: quem responde é a fórmula
        let x = External::cut(LinkSrc::Edge { mid: [0.; 3] }, &plano, curva, &[[10., 0., 0.], [-10., 0., 0.]]).unwrap();
        assert!(x.isolated && x.pts.len() == 2);
        assert!(x.pts.iter().all(|p| (p[0].abs() - 8.).abs() < 1e-12));
    }
}
