//! Módulo Modelagem Sólida: executa a linha de tempo do documento sobre o kernel.
//!
//! Regras (RF-NUC-008, RF-NUC-024): um comando com erro **não** é apagado nem invalida o resto;
//! o corpo continua do estado anterior e o comando fica marcado com a mensagem do erro.

pub mod geo3d;
pub mod hardware;
pub mod profile;

use profile::{ev, eval_sketch, to3d, EvalSketch, Vars};
use scc_core::sketch::{Curve, Kind};
use scc_core::*;
use scc_kernel::{Bool, Frame, HoleShape, KernelError, Mesh, MassProps, Shape, Xform};
use geo3d::{Curve3, V3};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Ok,
    Warning(String),
    Error(String),
    /// Depois do marcador de rollback ou suprimido.
    Inactive,
}

impl Status {
    pub fn message(&self) -> Option<&str> {
        match self {
            Status::Warning(m) | Status::Error(m) => Some(m),
            _ => None,
        }
    }
}

pub struct BodyResult {
    pub collector: Id,
    pub color: [u8; 3],
    pub mesh: Mesh,
    pub mass: MassProps,
    pub faces: u32,
    /// Comando que gerou cada face (mesma ordem de `mesh.face_ranges`).
    pub face_owner: Vec<Id>,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn unit(v: [f64; 3]) -> [f64; 3] {
    let l = dot(v, v).sqrt().max(1e-12);
    v.map(|x| x / l)
}
/// Diagonal aproximada dos perfis (para dimensionar ferramentas).
fn loops_size(loops: &[scc_kernel::Loop2]) -> f64 {
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    let mut add = |p: [f64; 2], r: f64| {
        for i in 0..2 {
            lo[i] = lo[i].min(p[i] - r);
            hi[i] = hi[i].max(p[i] + r);
        }
    };
    for l in loops {
        for sgm in l {
            match *sgm {
                scc_kernel::Seg2::Line(a, b) => {
                    add(a, 0.);
                    add(b, 0.)
                }
                scc_kernel::Seg2::Arc(a, m, b) => {
                    add(a, 0.);
                    add(m, 0.);
                    add(b, 0.)
                }
                scc_kernel::Seg2::Circle(c, r) => add(c, r),
            }
        }
    }
    if lo[0] > hi[0] {
        return 1.;
    }
    ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2)).sqrt()
}

fn tags_for(owners: &[Id]) -> Vec<i32> {
    (0..owners.len() as i32).collect()
}

/// Converte as etiquetas devolvidas pelo kernel em donos; -1 = face nova do comando `fid`.
fn remap(owners: &[Id], out: &[i32], fid: Id) -> Vec<Id> {
    out.iter().map(|t| if *t >= 0 { owners.get(*t as usize).copied().unwrap_or(fid) } else { fid }).collect()
}

/// Geometria de exibição de um sketch (segmentos 3D).
pub struct SketchViz {
    pub feature: Id,
    /// plano do sketch (para alinhar a câmera)
    pub frame: Frame,
    pub lines: Vec<[f32; 3]>,
    pub construction: Vec<[f32; 3]>,
    pub points: Vec<[f32; 3]>,
    /// triângulos da sombra dos perfis fechados (lista sem índices)
    pub fill: Vec<[f32; 3]>,
    /// graus de liberdade restantes e se o solver convergiu
    pub dof: usize,
    pub converged: bool,
}

/// Plano exibível (Origem ou Referências).
pub struct PlaneViz {
    pub id: Option<Id>,
    pub name: String,
    pub frame: Frame,
}

/// Ponto de referência exibível.
pub struct PointViz {
    pub id: Id,
    pub name: String,
    pub p: [f64; 3],
}

/// Linha/curva de referência exibível (polilinha).
pub struct CurveViz {
    pub id: Id,
    pub name: String,
    pub pts: Vec<[f64; 3]>,
}

/// Valor de um elemento de geometria de referência.
#[derive(Clone)]
pub enum Geo {
    Plane(Frame),
    Point(V3),
    Curve(Curve3),
    Surface(Shape),
}

/// Superfície de referência exibível (malha).
pub struct SurfaceViz {
    pub id: Id,
    pub name: String,
    pub mesh: Mesh,
}

pub struct RecomputeResult {
    pub bodies: Vec<BodyResult>,
    pub sketches: Vec<SketchViz>,
    pub planes: Vec<PlaneViz>,
    pub points: Vec<PointViz>,
    pub curves: Vec<CurveViz>,
    pub surfaces: Vec<SurfaceViz>,
    pub status: HashMap<Id, Status>,
    pub params: HashMap<String, f64>,
    pub param_error: Option<String>,
    pub elapsed_ms: f64,
    pub kernel: String,
}

pub fn origin_frame(p: OriginPlane) -> Frame {
    match p {
        OriginPlane::XY => Frame { origin: [0.; 3], x: [1., 0., 0.], normal: [0., 0., 1.] },
        OriginPlane::XZ => Frame { origin: [0.; 3], x: [1., 0., 0.], normal: [0., -1., 0.] },
        OriginPlane::YZ => Frame { origin: [0.; 3], x: [0., 1., 0.], normal: [1., 0., 0.] },
    }
}

pub fn origin_axis(a: OriginAxis) -> [f64; 3] {
    match a {
        OriginAxis::X => [1., 0., 0.],
        OriginAxis::Y => [0., 1., 0.],
        OriginAxis::Z => [0., 0., 1.],
    }
}

struct Ctx<'a> {
    doc: &'a Document,
    vars: Vars,
    bodies: HashMap<Id, (Shape, Vec<Id>)>,
    sketches: HashMap<Id, (Frame, EvalSketch)>,
    /// sólido gerado por cada comando (base para filete/chanfro por comando)
    status: HashMap<Id, Status>,
    /// ferramenta e operação de cada comando de forma (para padrões e espelho)
    tools: HashMap<Id, (Shape, BoolOp)>,
}

/// Pontos mais próximos entre os segmentos ab e cd.
fn seg_closest(a: V3, b: V3, c: V3, d: V3) -> (V3, V3) {
    use geo3d::*;
    let (u, v, w) = (sub(b, a), sub(d, c), sub(a, c));
    let (aa, bb, cc, dd, ee) = (dot(u, u), dot(u, v), dot(v, v), dot(u, w), dot(v, w));
    let den = aa * cc - bb * bb;
    let mut s = if den > 1e-12 { ((bb * ee - cc * dd) / den).clamp(0., 1.) } else { 0. };
    let mut t = if cc > 1e-12 { ((bb * s + ee) / cc).clamp(0., 1.) } else { 0. };
    if aa > 1e-12 {
        s = ((bb * t - dd) / aa).clamp(0., 1.);
    }
    if cc > 1e-12 {
        t = ((bb * s + ee) / cc).clamp(0., 1.);
    }
    (add(a, mul(u, s)), add(c, mul(v, t)))
}

/// B-spline cúbica uniforme pelo polígono de controle (aberta: presa nas pontas).
fn bspline(ctrl: &[V3], closed: bool) -> Result<Curve3, String> {
    use geo3d::*;
    if ctrl.len() < 3 {
        return Err("o polígono de controle precisa de pelo menos 3 pontos".into());
    }
    let mut p: Vec<V3> = ctrl.to_vec();
    if closed {
        p.extend_from_slice(&ctrl[..3]);
    } else {
        p.insert(0, ctrl[0]);
        p.insert(0, ctrl[0]);
        p.push(*ctrl.last().unwrap());
        p.push(*ctrl.last().unwrap());
    }
    let mut out = vec![];
    for i in 0..p.len() - 3 {
        for k in 0..24 {
            let t = k as f64 / 24.;
            let (t2, t3) = (t * t, t * t * t);
            let b0 = (1. - t).powi(3) / 6.;
            let b1 = (3. * t3 - 6. * t2 + 4.) / 6.;
            let b2 = (-3. * t3 + 3. * t2 + 3. * t + 1.) / 6.;
            let b3 = t3 / 6.;
            out.push(add(add(mul(p[i], b0), mul(p[i + 1], b1)), add(mul(p[i + 2], b2), mul(p[i + 3], b3))));
        }
    }
    if closed {
        out.push(out[0]);
    } else {
        out.push(*ctrl.last().unwrap());
    }
    Curve3::poly(out, closed)
}

impl Ctx<'_> {
    fn plane(&self, p: &PlaneRef, depth: u32) -> Result<Frame, String> {
        match p {
            PlaneRef::Origin(o) => Ok(origin_frame(*o)),
            PlaneRef::Datum(id) => match self.geo(*id, depth + 1)? {
                Geo::Plane(f) => Ok(f),
                _ => Err(format!("'{}' não é um plano", self.doc.name_of(*id))),
            },
        }
    }

    /// Eixo (ponto, direção unitária).
    fn axis(&self, a: &AxisRef) -> Result<([f64; 3], [f64; 3]), String> {
        self.axis_d(a, 0)
    }

    fn axis_d(&self, a: &AxisRef, depth: u32) -> Result<([f64; 3], [f64; 3]), String> {
        match *a {
            AxisRef::Origin(ax) => Ok(([0.; 3], origin_axis(ax))),
            AxisRef::SketchLine { sketch, entity } => {
                let (af, ask) = self.sketch(sketch)?;
                let (a, b) = ask.lines.get(&entity).ok_or("linha do eixo não existe mais — reedite o eixo")?;
                let (pa, pb) = (af.point(a[0], a[1]), af.point(b[0], b[1]));
                Ok((pa, unit([pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]])))
            }
            AxisRef::Ref(id) => {
                let c = self.curve(&CurveRef::Ref(id), depth + 1)?;
                let d = if c.is_straight() { geo3d::sub(c.end(), c.start()) } else { c.at_dist(0.).1 };
                Ok((c.start(), geo3d::unit(d).ok_or("eixo de comprimento zero")?))
            }
            AxisRef::Fixed { origin, dir } => Ok((origin, geo3d::unit(dir).ok_or("eixo de comprimento zero")?)),
        }
    }

    fn point(&self, p: &PointRef, depth: u32) -> Result<V3, String> {
        match *p {
            PointRef::Origin => Ok([0.; 3]),
            PointRef::Fixed(q) => Ok(q),
            PointRef::Ref(id) => match self.geo(id, depth + 1)? {
                Geo::Point(q) => Ok(q),
                _ => Err(format!("'{}' não é um ponto", self.doc.name_of(id))),
            },
        }
    }

    fn points(&self, ps: &[PointRef], depth: u32) -> Result<Vec<V3>, String> {
        ps.iter().map(|p| self.point(p, depth)).collect()
    }

    fn curve(&self, c: &CurveRef, depth: u32) -> Result<Curve3, String> {
        match c {
            CurveRef::Axis(a) => {
                let d = origin_axis(*a);
                Curve3::line(geo3d::mul(d, -500.), geo3d::mul(d, 500.))
            }
            CurveRef::Edge(pts) => {
                let closed = pts.len() > 2 && geo3d::dist(pts[0], *pts.last().unwrap()) < 1e-6;
                Curve3::poly(pts.clone(), closed)
            }
            CurveRef::SketchLine { sketch, entity } => {
                let (af, ask) = self.sketch(*sketch)?;
                let (a, b) = ask.lines.get(entity).ok_or("a linha do sketch não existe mais — reedite")?;
                Curve3::line(af.point(a[0], a[1]), af.point(b[0], b[1]))
            }
            CurveRef::Ref(id) => match self.geo(*id, depth + 1)? {
                Geo::Curve(c) => Ok(c),
                _ => Err(format!("'{}' não é uma linha ou curva", self.doc.name_of(*id))),
            },
        }
    }

    /// Ponto da superfície mais próximo de `p` (projeção no plano tangente) e a normal.
    fn surf_at(&self, sf: &SurfRef, p: V3, depth: u32) -> Result<(V3, V3), String> {
        let (o, n) = match sf {
            SurfRef::Plane(pr) => {
                let f = self.plane(pr, depth)?;
                (f.origin, f.normal)
            }
            SurfRef::Face { point, normal, .. } => (*point, geo3d::unit(*normal).ok_or("normal da face nula")?),
            SurfRef::Ref(id) => return self.surface(*id, depth)?.surface_near(p).map_err(|e| e.0),
        };
        Ok((geo3d::sub(p, geo3d::mul(n, geo3d::dot(geo3d::sub(p, o), n))), n))
    }

    fn surface(&self, id: Id, depth: u32) -> Result<Shape, String> {
        match self.geo(id, depth + 1)? {
            Geo::Surface(s) => Ok(s),
            _ => Err(format!("'{}' não é uma superfície", self.doc.name_of(id))),
        }
    }

    /// Forma da superfície de entrada (face do corpo, superfície de referência).
    fn surface_shape(&self, sf: &SurfRef, depth: u32) -> Result<Shape, String> {
        match sf {
            SurfRef::Ref(id) => self.surface(*id, depth),
            SurfRef::Face { point, .. } => self.face_of_bodies(*point),
            SurfRef::Plane(_) => Err("use 'Plano deslocado' para deslocar um plano".into()),
        }
    }

    /// Cópia da face de algum corpo que passa pelo ponto.
    fn face_of_bodies(&self, p: V3) -> Result<Shape, String> {
        for (b, _) in self.bodies.values() {
            if let Ok((q, _)) = b.surface_near(p) {
                if geo3d::dist(q, p) < 1e-3 {
                    return b.face_copy(p).map_err(|e| e.0);
                }
            }
        }
        Err("a face não existe mais no modelo — reedite".into())
    }

    fn surf_frame(&self, sf: &SurfRef, depth: u32) -> Result<Frame, String> {
        match sf {
            SurfRef::Plane(pr) => self.plane(pr, depth),
            SurfRef::Face { point, normal, .. } => geo3d::frame_from(*point, *normal, None),
            SurfRef::Ref(id) => {
                let sh = self.surface(*id, depth)?;
                let (q, n) = sh.surface_near(sh.mass().center).map_err(|e| e.0)?;
                geo3d::frame_from(q, n, None)
            }
        }
    }

    /// Ponto e tangente numa posição da curva.
    fn on_curve(&self, c: &Curve3, at: &OnCurve, reverse: bool, name: &str) -> Result<(V3, V3), String> {
        let len = c.length();
        let mut s = match at {
            OnCurve::Distance(e) => ev(e, &self.vars, &format!("{name} · distância"))?,
            OnCurve::Ratio(e) => ev(e, &self.vars, &format!("{name} · proporção"))? * len,
        };
        if s < -1e-9 || s > len + 1e-9 {
            return Err(format!("posição fora da curva (comprimento {:.3})", len));
        }
        if reverse {
            s = len - s;
        }
        Ok(c.at_dist(s))
    }

    /// Avalia um elemento de geometria de referência (plano, ponto ou curva).
    fn geo(&self, id: Id, depth: u32) -> Result<Geo, String> {
        use geo3d::*;
        use RefKind::*;
        if depth > 48 {
            return Err("referências em ciclo".into());
        }
        let r = self.doc.reference(id).ok_or_else(|| format!("referência perdida (#{}) — reedite ou substitua", id.0))?;
        let e = |x: &String, what: &str| ev(x, &self.vars, &format!("{} · {what}", r.name));
        let d = depth + 1;
        let line_of = |c: &Curve3| -> Result<(V3, V3), String> {
            if !c.is_straight() {
                return Err("esperava uma linha reta".into());
            }
            Ok((c.start(), geo3d::unit(geo3d::sub(c.end(), c.start())).ok_or("linha de comprimento zero")?))
        };
        Ok(match &r.kind {
            OffsetPlane { base, offset } => Geo::Plane(self.plane(base, d)?.offset(e(offset, "deslocamento")?)),
            FacePlane { origin, x, normal, .. } => Geo::Plane(Frame { origin: *origin, x: *x, normal: *normal }),
            AnglePlane { base, axis, angle } => {
                let b = self.plane(base, d)?;
                let (o, dd) = self.axis_d(axis, d)?;
                let a = e(angle, "ângulo")?.to_radians();
                let rot = |v: [f64; 3]| geo3d::rotate(v, dd, a);
                // o eixo é levado para dentro do plano base (giro em torno da linha no plano base)
                let h = geo3d::dot(geo3d::sub(b.origin, o), b.normal);
                let o = [0, 1, 2].map(|i| o[i] + b.normal[i] * h);
                Geo::Plane(Frame { origin: o, x: rot(b.x), normal: rot(b.normal) })
            }
            PlaneParallelPoint { base, point } => Geo::Plane(Frame { origin: self.point(point, d)?, ..self.plane(base, d)? }),
            PlaneThreePoints { a, b, c } => {
                let (pa, pb, pc) = (self.point(a, d)?, self.point(b, d)?, self.point(c, d)?);
                let n = cross(geo3d::sub(pb, pa), geo3d::sub(pc, pa));
                if norm(n) < 1e-9 {
                    return Err("os três pontos estão alinhados ou coincidentes".into());
                }
                Geo::Plane(frame_from(pa, n, Some(geo3d::sub(pb, pa)))?)
            }
            PlaneTwoLines { a, b } => {
                let (pa, da) = line_of(&self.curve(a, d)?)?;
                let (pb, db) = line_of(&self.curve(b, d)?)?;
                let mut n = cross(da, db);
                if norm(n) < 1e-9 {
                    n = cross(da, geo3d::sub(pb, pa));
                    if norm(n) < 1e-9 {
                        return Err("as linhas são a mesma reta".into());
                    }
                }
                let nu = geo3d::unit(n).unwrap();
                if geo3d::dot(geo3d::sub(pb, pa), nu).abs() > 1e-6 * (1. + dist(pa, pb)) {
                    return Err("as linhas não são coplanares".into());
                }
                Geo::Plane(frame_from(pa, nu, Some(da))?)
            }
            PlanePointLine { point, line } => {
                let p = self.point(point, d)?;
                let (o, dd) = line_of(&self.curve(line, d)?)?;
                let n = cross(dd, geo3d::sub(p, o));
                if norm(n) < 1e-9 {
                    return Err("o ponto está sobre a linha".into());
                }
                Geo::Plane(frame_from(o, n, Some(dd))?)
            }
            PlaneCurve { curve } => Geo::Plane(self.curve(curve, d)?.plane()?),
            PlaneNormalCurve { curve, at, reverse } => {
                let c = self.curve(curve, d)?;
                let (p, t) = self.on_curve(&c, at, *reverse, &r.name)?;
                Geo::Plane(frame_from(p, t, None)?)
            }
            PlaneTangentSurface { surface, point } => {
                let (p, n) = self.surf_at(surface, self.point(point, d)?, d)?;
                Geo::Plane(frame_from(p, n, None)?)
            }
            PlaneEquation { a, b, c, d: dd } => {
                let n = [e(a, "A")?, e(b, "B")?, e(c, "C")?];
                let l2 = geo3d::dot(n, n);
                if l2 < 1e-18 {
                    return Err("A, B e C não podem ser todos zero".into());
                }
                Geo::Plane(frame_from(mul(n, e(dd, "D")? / l2), n, None)?)
            }
            PlaneMeanPoints { points } => Geo::Plane(fit_plane(&self.points(points, d)?)?.0),

            PointCoords { x, y, z, frame } => {
                let (x, y, z) = (e(x, "X")?, e(y, "Y")?, e(z, "Z")?);
                Geo::Point(match frame {
                    Some(p) => {
                        let f = self.plane(p, d)?;
                        add(f.point(x, y), mul(f.normal, z))
                    }
                    None => [x, y, z],
                })
            }
            PointOnCurve { curve, at, reverse } => Geo::Point(self.on_curve(&self.curve(curve, d)?, at, *reverse, &r.name)?.0),
            PointOnPlane { plane, u, v } => Geo::Point(self.plane(plane, d)?.point(e(u, "u")?, e(v, "v")?)),
            PointOnSurface { surface, u, v } => Geo::Point(self.surf_frame(surface, d)?.point(e(u, "u")?, e(v, "v")?)),
            PointCenter { curve } => Geo::Point(fit_circle(&self.curve(curve, d)?)?.0),
            PointTangent { curve, dir, which } => {
                let (_, dv) = self.axis_d(dir, d)?;
                Geo::Point(tangent_points(&self.curve(curve, d)?, dv)?[(*which).min(1)])
            }
            PointBetween { a, b, ratio } => Geo::Point(lerp(self.point(a, d)?, self.point(b, d)?, e(ratio, "proporção")?)),

            LinePoints { a, b, start, end } => {
                let (pa, pb) = (self.point(a, d)?, self.point(b, d)?);
                let dd = geo3d::unit(geo3d::sub(pb, pa)).ok_or("os dois pontos coincidem")?;
                Geo::Curve(Curve3::line(geo3d::sub(pa, mul(dd, e(start, "prolongar início")?)), add(pb, mul(dd, e(end, "prolongar fim")?)))?)
            }
            LineDir { point, dir, start, end } => {
                let p = self.point(point, d)?;
                let (_, dv) = self.axis_d(dir, d)?;
                Geo::Curve(Curve3::line(add(p, mul(dv, e(start, "início")?)), add(p, mul(dv, e(end, "fim")?)))?)
            }
            LineAngle { curve, point, support, angle, start, end } => {
                let c = self.curve(curve, d)?;
                let (s, q) = c.nearest(self.point(point, d)?);
                let (_, t) = c.at_dist(s);
                let n = self.plane(support, d)?.normal;
                let tp = geo3d::unit(geo3d::sub(t, mul(n, geo3d::dot(t, n)))).ok_or("a curva é perpendicular ao plano de apoio nesse ponto")?;
                let dv = geo3d::rotate(tp, n, e(angle, "ângulo")?.to_radians());
                Geo::Curve(Curve3::line(add(q, mul(dv, e(start, "início")?)), add(q, mul(dv, e(end, "fim")?)))?)
            }
            LineNormalSurface { surface, point, start, end } => {
                let (q, n) = self.surf_at(surface, self.point(point, d)?, d)?;
                Geo::Curve(Curve3::line(add(q, mul(n, e(start, "início")?)), add(q, mul(n, e(end, "fim")?)))?)
            }
            Polyline { points, closed } => Geo::Curve(Curve3::poly(self.points(points, d)?, *closed)?),
            Circle { center, plane, radius, start, end } => {
                let c = self.point(center, d)?;
                let f = Frame { origin: c, ..self.plane(plane, d)? };
                Geo::Curve(circle_curve(c, &f, e(radius, "raio")?, e(start, "ângulo inicial")?.to_radians(), e(end, "ângulo final")?.to_radians())?)
            }
            Circle3 { a, b, c, full } => Geo::Curve(circle3_curve(self.point(a, d)?, self.point(b, d)?, self.point(c, d)?, *full)?),
            Spline { points, closed } => Geo::Curve(spline(&self.points(points, d)?, *closed)?),
            Helix { origin, axis, radius, pitch, height, taper, left } => {
                let (_, ax) = self.axis_d(axis, d)?;
                Geo::Curve(helix(self.point(origin, d)?, ax, e(radius, "raio")?, e(pitch, "passo")?, e(height, "altura")?, e(taper, "conicidade")?, *left)?)
            }
            Spiral { center, plane, r0, r1, turns } => {
                let c = self.point(center, d)?;
                let f = Frame { origin: c, ..self.plane(plane, d)? };
                Geo::Curve(spiral(c, &f, e(r0, "raio inicial")?, e(r1, "raio final")?, e(turns, "voltas")?)?)
            }
            Conic { center, plane, kind, a, b, extent } => {
                let c = self.point(center, d)?;
                let f = Frame { origin: c, ..self.plane(plane, d)? };
                Geo::Curve(conic(c, &f, *kind, e(a, "a")?, e(b, "b")?, e(extent, "extensão")?)?)
            }
            Connect { a, a_end, b, b_end, cont, tension } => {
                let (ca, cb) = (self.curve(a, d)?, self.curve(b, d)?);
                let pa = if *a_end { ca.end() } else { ca.start() };
                let pb = if *b_end { cb.end() } else { cb.start() };
                let l = dist(pa, pb) * e(tension, "tensão")?;
                if dist(pa, pb) < 1e-9 {
                    return Err("as pontas escolhidas já se tocam".into());
                }
                let (ta, tb) = (ca.end_tangent(*a_end), cb.end_tangent(*b_end));
                let pts = match cont {
                    Continuity::G0 => vec![pa, pb],
                    Continuity::G1 => hermite(pa, mul(ta, l), [0.; 3], pb, mul(tb, -l), [0.; 3], false),
                    Continuity::G2 => hermite(pa, mul(ta, l), mul(ca.end_curvature(*a_end), l * l), pb, mul(tb, -l), mul(cb.end_curvature(*b_end), l * l), true),
                };
                Geo::Curve(Curve3::poly(pts, false)?)
            }
            Corner { a, b, radius } => Geo::Curve(corner(&self.curve(a, d)?, &self.curve(b, d)?, e(radius, "raio")?)?),
            PointCurvePlane { curve, plane, which } => {
                let c = self.curve(curve, d)?;
                let f = self.plane(plane, d)?;
                let h = |q: V3| geo3d::dot(geo3d::sub(q, f.origin), f.normal);
                let mut hits = vec![];
                for w in c.pts.windows(2) {
                    let (ha, hb) = (h(w[0]), h(w[1]));
                    if ha.abs() < 1e-12 {
                        hits.push(w[0]);
                    } else if ha * hb < 0. {
                        hits.push(lerp(w[0], w[1], ha / (ha - hb)));
                    }
                }
                if h(c.end()).abs() < 1e-12 {
                    hits.push(c.end());
                }
                if hits.is_empty() {
                    return Err("a curva não cruza o plano".into());
                }
                Geo::Point(hits[(*which).min(hits.len() - 1)])
            }
            PointCurveCurve { a, b } => {
                let (ca, cb) = (self.curve(a, d)?, self.curve(b, d)?);
                let mut best = (f64::MAX, [0.; 3]);
                for wa in ca.pts.windows(2) {
                    for wb in cb.pts.windows(2) {
                        let (p, q) = seg_closest(wa[0], wa[1], wb[0], wb[1]);
                        let dd = dist(p, q);
                        if dd < best.0 {
                            best = (dd, lerp(p, q, 0.5));
                        }
                    }
                }
                Geo::Point(best.1)
            }
            PointProject { point, surface } => Geo::Point(self.surf_at(surface, self.point(point, d)?, d)?.0),
            PointProjectCurve { point, curve } => Geo::Point(self.curve(curve, d)?.nearest(self.point(point, d)?).1),
            PointOffset { point, dir, dist: dd } => {
                let (_, v) = self.axis_d(dir, d)?;
                Geo::Point(add(self.point(point, d)?, mul(v, e(dd, "distância")?)))
            }
            LinePlanes { a, b, length } => {
                let (fa, fb) = (self.plane(a, d)?, self.plane(b, d)?);
                let u = cross(fa.normal, fb.normal);
                let uu = dot(u, u);
                if uu < 1e-12 {
                    return Err("os planos são paralelos".into());
                }
                let (da, db) = (dot(fa.normal, fa.origin), dot(fb.normal, fb.origin));
                let p0 = mul(add(mul(cross(fb.normal, u), da), mul(cross(u, fa.normal), db)), 1. / uu);
                let dir = geo3d::unit(u).unwrap();
                // centra perto das origens dos planos
                let m = lerp(fa.origin, fb.origin, 0.5);
                let c = add(p0, mul(dir, dot(sub(m, p0), dir)));
                let l = e(length, "comprimento")? / 2.;
                Geo::Curve(Curve3::line(sub(c, mul(dir, l)), add(c, mul(dir, l)))?)
            }
            LineBisector { a, b, length } => {
                let (ca, cb) = (self.curve(a, d)?, self.curve(b, d)?);
                let (pa, da) = line_of(&ca)?;
                let (pb, db) = line_of(&cb)?;
                let (p, q) = seg_closest(sub(pa, mul(da, 1e4)), add(pa, mul(da, 1e4)), sub(pb, mul(db, 1e4)), add(pb, mul(db, 1e4)));
                let v = lerp(p, q, 0.5);
                let side = |c: &Curve3, dd: V3| {
                    let far = if dist(c.start(), v) > dist(c.end(), v) { c.start() } else { c.end() };
                    if dot(sub(far, v), dd) >= 0. { dd } else { mul(dd, -1.) }
                };
                let bis = geo3d::unit(add(side(&ca, da), side(&cb, db))).ok_or("as linhas são opostas")?;
                Geo::Curve(Curve3::line(v, add(v, mul(bis, e(length, "comprimento")?)))?)
            }
            LineAxis { face } => {
                for (b, _) in self.bodies.values() {
                    if let Ok(cy) = b.cylinder_at(*face) {
                        let o = add(cy.origin, mul(cy.dir, cy.h0));
                        let t = add(cy.origin, mul(cy.dir, cy.h1.max(cy.h0 + 1.)));
                        return Ok(Geo::Curve(Curve3::line(o, t)?));
                    }
                    if let Ok(fp) = b.face_props(*face) {
                        if fp.kind == 2 {
                            return Ok(Geo::Curve(Curve3::line(fp.origin, add(fp.origin, mul(fp.dir, 2. * fp.radius.max(5.))))?));
                        }
                    }
                }
                return Err("a face escolhida não é cilíndrica nem cônica".into());
            }
            PlanePointNormal { point, line } => {
                let c = self.curve(line, d)?;
                let p = self.point(point, d)?;
                let (s0, _) = c.nearest(p);
                let (_, t) = c.at_dist(s0);
                Geo::Plane(frame_from(p, t, None)?)
            }
            PlaneMid { a, b } => {
                let (fa, fb) = (self.plane(a, d)?, self.plane(b, d)?);
                let nb = if dot(fa.normal, fb.normal) < 0. { mul(fb.normal, -1.) } else { fb.normal };
                let n = geo3d::unit(add(fa.normal, nb)).ok_or("planos opostos")?;
                let proj = sub(fa.origin, mul(nb, dot(sub(fa.origin, fb.origin), nb)));
                Geo::Plane(frame_from(lerp(fa.origin, proj, 0.5), n, Some(fa.x))?)
            }
            CurveProject { curve, plane } => {
                let c = self.curve(curve, d)?;
                let f = self.plane(plane, d)?;
                let pts = c.pts.iter().map(|q| sub(*q, mul(f.normal, dot(sub(*q, f.origin), f.normal)))).collect();
                Geo::Curve(Curve3::poly(pts, c.closed)?)
            }
            SplineControl { points, closed } => Geo::Curve(bspline(&self.points(points, d)?, *closed)?),
            CurveCopy { curve } => Geo::Curve(self.curve(curve, d)?),
            SurfExtract { face, .. } => Geo::Surface(self.face_of_bodies(*face)?),
            SurfOffset { surface, dist: dd } => {
                let sh = self.surface_shape(surface, d)?;
                Geo::Surface(sh.offset_surface(e(dd, "distância")?).map_err(|x| x.0)?)
            }
            SurfFill { curves } => {
                let cs: Vec<Vec<V3>> = curves.iter().map(|c| self.curve(c, d).map(|c| c.pts)).collect::<Result<_, _>>()?;
                Geo::Surface(Shape::fill_surface(&cs).map_err(|x| x.0)?)
            }
            SurfExtrude { curve, dir, length } => {
                let c = self.curve(curve, d)?;
                let (_, v) = self.axis_d(dir, d)?;
                Geo::Surface(Shape::extrude_curve(&c.pts, v, e(length, "comprimento")?).map_err(|x| x.0)?)
            }
            SurfRevolve { curve, axis, angle } => {
                let c = self.curve(curve, d)?;
                let (o, v) = self.axis_d(axis, d)?;
                Geo::Surface(Shape::revolve_curve(&c.pts, o, v, e(angle, "ângulo")?).map_err(|x| x.0)?)
            }
            Linked { ext, .. } => {
                let x = self.doc.external.iter().find(|x| x.id == *ext).ok_or("link perdido")?;
                match x.snapshot.as_ref().ok_or("link sem cópia — atualize os links")? {
                    LinkedGeo::Plane { origin, x, normal } => Geo::Plane(Frame { origin: *origin, x: *x, normal: *normal }),
                    LinkedGeo::Point(p) => Geo::Point(*p),
                    LinkedGeo::Curve(pts) => Geo::Curve(Curve3::poly(pts.clone(), false)?),
                    LinkedGeo::Value(_) => return Err("publicação de valor vira parâmetro, não geometria".into()),
                }
            }
            Spine { planes } => {
                let fs: Vec<Frame> = planes.iter().map(|p| self.plane(p, d)).collect::<Result<_, _>>()?;
                if fs.len() < 2 {
                    return Err("a spine precisa de pelo menos dois planos".into());
                }
                let mut pts: Vec<V3> = vec![];
                for w in fs.windows(2) {
                    let (p0, p1) = (w[0].origin, w[1].origin);
                    let chord = geo3d::sub(p1, p0);
                    let l = norm(chord);
                    if l < 1e-9 {
                        return Err("dois planos seguidos com a mesma origem".into());
                    }
                    // normais orientadas no sentido do percurso
                    let orient = |n: V3| if geo3d::dot(n, chord) < 0. { mul(n, -1.) } else { n };
                    let seg = hermite(p0, mul(orient(w[0].normal), l), [0.; 3], p1, mul(orient(w[1].normal), l), [0.; 3], false);
                    if !pts.is_empty() {
                        pts.pop();
                    }
                    pts.extend(seg);
                }
                Geo::Curve(Curve3::poly(pts, false)?)
            }
        })
    }

    fn sketch(&self, id: Id) -> Result<&(Frame, EvalSketch), String> {
        self.sketches.get(&id).ok_or_else(|| {
            match self.doc.feature(id) {
                Some(f) => format!("o sketch '{}' está inativo ou com erro", f.name),
                None => format!("referência perdida: sketch #{} não existe — reedite ou substitua a referência", id.0),
            }
        })
    }

    fn body(&self, target: Id) -> Option<&Shape> {
        self.body_owned(target).map(|(s, _)| s)
    }

    /// Distância "passante": maior que o corpo inteiro.
    fn through(&self, target: Id) -> Result<f64, String> {
        let b = self.body(target).ok_or("'Passante' precisa de um corpo existente no coletor de destino")?;
        let (lo, hi) = b.bbox().ok_or("corpo sem dimensões")?;
        let mut d2 = 0.;
        for i in 0..3 {
            d2 += (hi[i] - lo[i]).powi(2);
        }
        Ok(d2.sqrt() * 2. + 10.)
    }

    fn extent(&self, e: &Extent, target: Id) -> Result<(f64, f64), String> {
        Ok(match e {
            Extent::Blind(d) => (ev(d, &self.vars, "distância")?, 0.),
            Extent::Symmetric(d) => {
                let d = ev(d, &self.vars, "distância")?;
                (d / 2., d / 2.)
            }
            Extent::TwoSides(a, b) => (ev(a, &self.vars, "distância 1")?, ev(b, &self.vars, "distância 2")?),
            Extent::ThroughAll => (self.through(target)?, 0.),
            Extent::UpTo(_) => return Err("'até plano' só vale para extrusão e recorte".into()),
            Extent::UpToNext => return Err("'até o próximo' só vale para extrusão".into()),
        })
    }

    /// Corpo + dono (comando) de cada face, na ordem das faces do kernel.
    fn body_owned(&self, target: Id) -> Option<&(Shape, Vec<Id>)> {
        self.bodies.get(&target).filter(|(s, _)| !s.is_null())
    }

    /// Aplica a ferramenta ao corpo do coletor; faces novas passam a pertencer ao comando `fid`.
    fn apply(&mut self, target: Id, tool: Shape, op: BoolOp, fid: Id) -> Result<(), String> {
        let k = |e: KernelError| e.0;
        // guarda a ferramenta (furos com rosca somam várias)
        let kept = match self.tools.remove(&fid) {
            Some((t, o)) if o == op => t.compound(&tool),
            _ => tool.clone(),
        };
        self.tools.insert(fid, (kept, op));
        let next = match (self.body_owned(target), op) {
            (None, BoolOp::Add) => {
                let n = tool.face_count() as usize;
                (tool, vec![fid; n])
            }
            (None, _) => return Err("não há corpo no coletor de destino para esta operação".into()),
            (Some((b, owners)), op) => {
                let kop = match op {
                    BoolOp::Add => Bool::Fuse,
                    BoolOp::Cut => Bool::Cut,
                    BoolOp::Intersect => Bool::Common,
                };
                let (s, out) = b.boolean_tagged(&tool, kop, &tags_for(owners)).map_err(k)?;
                let own = remap(owners, &out, fid);
                (s, own)
            }
        };
        self.bodies.insert(target, next);
        Ok(())
    }

    /// Repete as ferramentas dos comandos `sources` (ou o corpo inteiro) pelas transformações.
    fn replicate(&mut self, f: &Feature, sources: &[Id], xs: &[Xform]) -> Result<(), String> {
        let k = |e: KernelError| e.0;
        if sources.is_empty() {
            let b = self.body(f.target).ok_or("não há corpo para repetir")?.clone();
            let mut acc = Shape::empty();
            for x in xs {
                let c = b.transformed(*x).map_err(k)?;
                acc = if acc.is_null() { c } else { acc.compound(&c) };
            }
            return self.apply(f.target, acc, BoolOp::Add, f.id);
        }
        for src in sources {
            let (tool, op) = self.tools.get(src).cloned().ok_or_else(|| format!("'{}' não gera forma para repetir (ou vem depois)", self.doc.name_of(*src)))?;
            let mut acc = Shape::empty();
            for x in xs {
                let c = tool.transformed(*x).map_err(k)?;
                acc = if acc.is_null() { c } else { acc.compound(&c) };
            }
            self.apply(f.target, acc, op, f.id)?;
        }
        Ok(())
    }

    fn run(&mut self, f: &Feature) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        match &f.kind {
            FeatureKind::Sketch(s) => {
                let frame = self.plane(&s.plane, 0)?;
                let es = eval_sketch(&s.data, &self.vars)?;
                let warn = (!es.warnings.is_empty()).then(|| es.warnings.join("; "));
                self.sketches.insert(f.id, (frame, es));
                Ok(warn)
            }
            FeatureKind::Extrude(e) => {
                let (frame, sk) = self.sketch(e.sketch)?;
                if sk.loops.is_empty() {
                    return Err("o sketch não tem perfis fechados".into());
                }
                let mut face = Shape::face(frame, &sk.loops).map_err(k)?;
                if let Some(t) = &e.thin {
                    face = face.thin(ev(t, &self.vars, "espessura da parede")?).map_err(k)?;
                }
                let taper = ev(&e.taper, &self.vars, "ângulo de saída")?;
                // recorte entra no material (contra a normal); "inverter" troca o sentido
                let into = (e.op == BoolOp::Cut) != e.reverse;
                let n = frame.normal;
                let dir = if into { n.map(|x| -x) } else { n };
                let tool = if let Extent::UpTo(p) = &e.extent {
                    let target = self.plane(p, 0)?;
                    let n = target.normal;
                    let den = dot(dir, n);
                    if den.abs() < 1e-9 {
                        return Err("o plano de término é paralelo à direção da extrusão".into());
                    }
                    let t = dot(sub(target.origin, frame.origin), n) / den;
                    if t <= 1e-9 {
                        return Err("o plano de término está do outro lado — use 'inverter sentido'".into());
                    }
                    let size = loops_size(&sk.loops);
                    face.extrude_until(dir, t * 4. + size * 10. + 100., target.origin, n).map_err(k)?
                } else if e.extent == Extent::UpToNext {
                    if e.op != BoolOp::Add {
                        return Err("'até o próximo' é para extrusão que acrescenta material".into());
                    }
                    let body = self.body(f.target).ok_or("'até o próximo' precisa de um corpo no coletor")?;
                    face.extrude_to_next(dir, self.through(f.target)?, body).map_err(k)?
                } else if taper.abs() > 1e-9 {
                    let Extent::Blind(d) = &e.extent else {
                        return Err("o ângulo de saída vale só com término 'cego'".into());
                    };
                    face.extrude_taper(dir, ev(d, &self.vars, "distância")?, taper).map_err(k)?
                } else {
                    let (front, back) = self.extent(&e.extent, f.target)?;
                    face.extrude(dir, front, back).map_err(k)?
                };
                self.apply(f.target, tool, e.op, f.id)?;
                Ok(None)
            }
            FeatureKind::Revolve(r) => {
                let (frame, sk) = self.sketch(r.sketch)?;
                if sk.loops.is_empty() {
                    return Err("o sketch não tem perfis fechados".into());
                }
                let (origin, dir) = self.axis(&r.axis)?;
                let mut face = Shape::face(frame, &sk.loops).map_err(k)?;
                if let Some(t) = &r.thin {
                    face = face.thin(ev(t, &self.vars, "espessura da parede")?).map_err(k)?;
                }
                let ang = ev(&r.angle, &self.vars, "ângulo")?;
                let start = if r.symmetric { -ang / 2. } else { 0. };
                let tool = face.revolve(origin, dir, ang, start).map_err(k)?;
                self.apply(f.target, tool, r.op, f.id)?;
                Ok(None)
            }
            FeatureKind::Hole(h) => {
                let (frame, sk) = self.sketch(h.sketch)?;
                if sk.points.is_empty() {
                    return Err("o sketch do furo precisa de pelo menos um ponto (centro)".into());
                }
                let (frame, pts) = (*frame, sk.points.clone());
                let d = ev(&h.diameter, &self.vars, "diâmetro")?;
                let (depth, _) = self.extent(&h.depth, f.target)?;
                let shape = match &h.kind {
                    HoleKind::Simples => HoleShape::Simple,
                    HoleKind::Rebaixado { diameter, depth } => HoleShape::Counterbore {
                        diameter: ev(diameter, &self.vars, "diâmetro do rebaixo")?,
                        depth: ev(depth, &self.vars, "profundidade do rebaixo")?,
                    },
                    HoleKind::Escareado { diameter, angle } => HoleShape::Countersink {
                        diameter: ev(diameter, &self.vars, "diâmetro do escareado")?,
                        angle: ev(angle, &self.vars, "ângulo do escareado")?,
                    },
                };
                let dir = frame.normal.map(|x| -x);
                let mut tool = Shape::empty();
                for p in &pts {
                    let t = Shape::hole_tool(frame.point(p[0], p[1]), dir, d, depth, shape).map_err(k)?;
                    tool = if tool.is_null() { t } else { tool.compound(&t) };
                }
                self.apply(f.target, tool, BoolOp::Cut, f.id)?;
                let Some(t) = &h.thread else { return Ok(None) };
                if !h.thread_modeled {
                    return Ok(Some(format!("rosca cosmética {t}")));
                }
                // rosca modelada: sulco helicoidal subtraído em cada furo, até a espessura do material
                let spec = hardware::thread(t).ok_or_else(|| format!("rosca '{t}' não encontrada na tabela de normas"))?;
                let (lo, hi) = self.body(f.target).and_then(|b| b.bbox()).ok_or("corpo sem dimensões")?;
                for p in &pts {
                    let o = frame.point(p[0], p[1]);
                    // extensão do material a partir do ponto, nos dois sentidos do eixo do furo:
                    // a rosca vai para o lado onde está a peça (o sketch pode estar na outra face)
                    let (mut fwd, mut back): (f64, f64) = (0., 0.);
                    for c in 0..8 {
                        let q = [if c & 1 == 0 { lo[0] } else { hi[0] }, if c & 2 == 0 { lo[1] } else { hi[1] }, if c & 4 == 0 { lo[2] } else { hi[2] }];
                        let t = dot(sub(q, o), dir);
                        fwd = fwd.max(t);
                        back = back.max(-t);
                    }
                    let (tdir, ext) = if fwd >= back { (dir, fwd) } else { (dir.map(|x| -x), back) };
                    // a rosca começa abaixo do rebaixo/escareado (começar no vazio faz a booleana falhar)
                    let s0 = if fwd >= back {
                        match shape {
                            HoleShape::Counterbore { depth, .. } => depth,
                            HoleShape::Countersink { diameter, angle } => ((diameter - d) / 2.).max(0.) / (angle.to_radians() / 2.).tan().max(1e-6),
                            HoleShape::Simple => 0.,
                        }
                    } else {
                        0.
                    };
                    let len = depth.min(ext) - s0;
                    if len < spec.pitch {
                        continue;
                    }
                    let o = [0, 1, 2].map(|i| o[i] + tdir[i] * s0);
                    let tool = Shape::thread_tool(o, tdir, spec.major, spec.pitch, len, spec.angle).map_err(k)?;
                    self.apply(f.target, tool, BoolOp::Cut, f.id)?;
                }
                Ok(None)
            }
            FeatureKind::Sweep(w) => {
                let (frame, sk) = self.sketch(w.sketch)?;
                if sk.loops.is_empty() {
                    return Err("o sketch do perfil não tem contorno fechado".into());
                }
                let mut face = Shape::face(frame, &sk.loops).map_err(k)?;
                if let Some(t) = &w.thin {
                    face = face.thin(ev(t, &self.vars, "espessura da parede")?).map_err(k)?;
                }
                let spine = self.curve(&w.spine, 0)?;
                let mode = match w.mode {
                    SweepMode::Corrigido => 0,
                    SweepMode::Fixo => 1,
                    SweepMode::Frenet => 2,
                };
                let tool = face.sweep(&spine.pts, mode).map_err(k)?;
                self.apply(f.target, tool, w.op, f.id)?;
                Ok(None)
            }
            FeatureKind::Loft(l) => {
                let mut comp = Shape::empty();
                for id in &l.sections {
                    let (frame, sk) = self.sketch(*id)?;
                    if sk.loops.is_empty() {
                        return Err(format!("o sketch '{}' não tem contorno fechado", self.doc.name_of(*id)));
                    }
                    let face = Shape::face(frame, &sk.loops[..1]).map_err(k)?;
                    comp = if comp.is_null() { face } else { comp.compound(&face) };
                }
                let tool = comp.loft(l.ruled, l.closed).map_err(k)?;
                self.apply(f.target, tool, l.op, f.id)?;
                Ok(None)
            }
            FeatureKind::Primitive(pr) => {
                let fr = self.plane(&pr.plane, 0)?;
                let names = pr.kind.params();
                let mut vals = vec![];
                for (e, n) in [&pr.a, &pr.b, &pr.c, &pr.d].into_iter().zip(names) {
                    vals.push(if n.is_empty() { 0. } else { ev(e, &self.vars, n)? });
                }
                let kind = PrimKind::ALL.iter().position(|x| *x == pr.kind).unwrap_or(0) as u8;
                let tool = Shape::primitive(kind, &fr, &vals).map_err(k)?;
                self.apply(f.target, tool, pr.op, f.id)?;
                Ok(None)
            }
            FeatureKind::Shell(sh) => {
                let t = ev(&sh.thickness, &self.vars, "espessura")?;
                let (b, owners) = self.body_owned(f.target).ok_or("não há corpo para esvaziar")?;
                let (s, out) = b.shell_tagged(&sh.faces, t, sh.outward, &tags_for(owners)).map_err(k)?;
                let own = remap(owners, &out, f.id);
                self.bodies.insert(f.target, (s, own));
                Ok(None)
            }
            FeatureKind::Draft(d) => {
                let a = ev(&d.angle, &self.vars, "ângulo")?;
                let neutral = self.plane(&d.neutral, 0)?;
                let pull = if d.reverse { neutral.normal.map(|x| -x) } else { neutral.normal };
                let (b, owners) = self.body_owned(f.target).ok_or("não há corpo para aplicar")?;
                let (s, out) = b.draft_tagged(&d.faces, pull, a, &neutral, &tags_for(owners)).map_err(k)?;
                let own = remap(owners, &out, f.id);
                self.bodies.insert(f.target, (s, own));
                Ok(None)
            }
            FeatureKind::Thickness(t) => {
                let v = ev(&t.value, &self.vars, "espessura")?;
                if v.abs() < 1e-9 {
                    return Ok(Some("espessura zero: nada a fazer".into()));
                }
                let b = self.body(f.target).ok_or("não há corpo para aplicar")?;
                let tool = b.faces_offset_tool(&t.faces, v).map_err(k)?;
                self.apply(f.target, tool, if v > 0. { BoolOp::Add } else { BoolOp::Cut }, f.id)?;
                Ok(None)
            }
            FeatureKind::Pattern(pt) => {
                let mut xs: Vec<Xform> = vec![];
                match &pt.kind {
                    PatternKind::Linear { dir, count, spacing, dir2, count2, spacing2 } => {
                        let n1 = ev(count, &self.vars, "quantidade")?.round().max(1.) as usize;
                        let d1 = ev(spacing, &self.vars, "espaçamento")?;
                        let (_, u1) = self.axis(dir)?;
                        let (n2, d2, u2) = match dir2 {
                            Some(a) => (ev(count2, &self.vars, "quantidade 2")?.round().max(1.) as usize, ev(spacing2, &self.vars, "espaçamento 2")?, self.axis(a)?.1),
                            None => (1, 0., [0.; 3]),
                        };
                        for i in 0..n1 {
                            for j in 0..n2 {
                                if i + j == 0 {
                                    continue;
                                }
                                xs.push(Xform::Translate([0, 1, 2].map(|c| u1[c] * d1 * i as f64 + u2[c] * d2 * j as f64)));
                            }
                        }
                    }
                    PatternKind::Circular { axis, count, angle } => {
                        let n = ev(count, &self.vars, "quantidade")?.round().max(1.) as usize;
                        let total = ev(angle, &self.vars, "ângulo")?;
                        let (o, d) = self.axis(axis)?;
                        // volta completa: divide por n; arco: divide por n-1 (primeira e última nas pontas)
                        let step = if (total.abs() - 360.).abs() < 1e-6 { total / n as f64 } else { total / (n.max(2) - 1) as f64 };
                        for i in 1..n {
                            xs.push(Xform::Rotate(o, d, step * i as f64));
                        }
                    }
                    PatternKind::Points { points, base } => {
                        let b0 = self.point(base, 0)?;
                        for q in points {
                            let p = self.point(q, 0)?;
                            xs.push(Xform::Translate([p[0] - b0[0], p[1] - b0[1], p[2] - b0[2]]));
                        }
                    }
                }
                if xs.is_empty() {
                    return Ok(Some("padrão com uma só instância".into()));
                }
                self.replicate(f, &pt.sources, &xs)?;
                Ok(None)
            }
            FeatureKind::Mirror(m) => {
                let pl = self.plane(&m.plane, 0)?;
                self.replicate(f, &m.sources, &[Xform::Mirror(pl.origin, pl.normal)])?;
                Ok(None)
            }
            FeatureKind::Scale(sc) => {
                let c = self.point(&sc.center, 0)?;
                let kf = ev(&sc.factor, &self.vars, "fator")?;
                let (b, owners) = self.body_owned(f.target).ok_or("não há corpo para escalar")?;
                let (s, out) = b.transformed_tagged(Xform::Scale(c, kf), &tags_for(owners)).map_err(k)?;
                let own = remap(owners, &out, f.id);
                self.bodies.insert(f.target, (s, own));
                Ok(None)
            }
            FeatureKind::Stiffener(st) => {
                let (frame, sk) = self.sketch(st.sketch)?;
                let frame = *frame;
                let segs: Vec<([f64; 3], [f64; 3])> = sk.lines.values().map(|(a, b)| (frame.point(a[0], a[1]), frame.point(b[0], b[1]))).collect();
                if segs.is_empty() {
                    return Err("o sketch do reforço precisa de linhas".into());
                }
                let t = ev(&st.thickness, &self.vars, "espessura")?;
                let depth = ev(&st.depth, &self.vars, "profundidade")?;
                let face = Shape::thick_segments(&segs, frame.normal, t).map_err(k)?;
                // entra no material como um furo (contra a normal); "inverter" troca o lado
                let dir = if st.reverse { frame.normal } else { frame.normal.map(|x| -x) };
                let tool = if st.to_body {
                    let body = self.body(f.target).ok_or("'até o corpo' precisa de um corpo no coletor")?;
                    face.extrude_to_next(dir, self.through(f.target)?, body).map_err(k)?
                } else {
                    face.extrude(dir, depth, 0.).map_err(k)?
                };
                self.apply(f.target, tool, BoolOp::Add, f.id)?;
                Ok(None)
            }
            FeatureKind::Thread(t) => {
                let spec = hardware::thread(&t.spec).ok_or_else(|| format!("rosca '{}' não encontrada na tabela de normas", t.spec))?;
                let b = self.body(f.target).ok_or("não há corpo para roscar")?;
                let cyl = b.cylinder_at(t.face).map_err(k)?;
                let len = ev(&t.length, &self.vars, "comprimento")?.min(cyl.h1 - cyl.h0);
                let d = 2. * cyl.radius;
                let nominal = if cyl.internal { spec.tap_drill } else { spec.major };
                let note = ((d - nominal).abs() > 0.3 * spec.pitch).then(|| format!("Ø da face {d:.2} ≠ {nominal:.2} da rosca {}", spec.name));
                if !t.modeled {
                    return Ok(Some(note.unwrap_or_else(|| format!("rosca cosmética {}", spec.name))));
                }
                if len < spec.pitch {
                    return Err("comprimento menor que um passo".into());
                }
                let o = [0, 1, 2].map(|i| cyl.origin[i] + cyl.dir[i] * cyl.h0);
                let tool = if cyl.internal {
                    Shape::thread_tool(o, cyl.dir, spec.major, spec.pitch, len, spec.angle)
                } else {
                    Shape::thread_tool_ext(o, cyl.dir, spec.major, spec.pitch, len, spec.angle)
                }
                .map_err(k)?;
                self.apply(f.target, tool, BoolOp::Cut, f.id)?;
                Ok(note)
            }
            FeatureKind::Combine(c) => {
                if c.tool == f.target {
                    return Err("o corpo ferramenta precisa ser de outro coletor".into());
                }
                let tool = self.body(c.tool).cloned().ok_or_else(|| format!("o coletor '{}' não tem corpo (ou vem depois)", self.doc.name_of(c.tool)))?;
                self.apply(f.target, tool, c.op, f.id)?;
                if !c.keep_tool {
                    self.bodies.remove(&c.tool);
                }
                Ok(None)
            }
            FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => {
                let v = ev(&o.value, &self.vars, "valor")?;
                let (b, owners) = self.body_owned(f.target).ok_or("não há corpo para aplicar")?;
                let tags = tags_for(owners);
                let chamfer = matches!(f.kind, FeatureKind::Chamfer(_));
                match &o.mode {
                    EdgeMode::FaceFace(a, c) => {
                        let (s, out) = b.face_face_fillet_tagged(*a, *c, v, &tags).map_err(k)?;
                        let own = remap(owners, &out, f.id);
                        self.bodies.insert(f.target, (s, own));
                        return Ok(None);
                    }
                    EdgeMode::FullRound { mid, a, b: c } => {
                        let (fa, fc) = (b.face_props(*a).map_err(k)?, b.face_props(*c).map_err(k)?);
                        if fa.kind != 0 || fc.kind != 0 {
                            return Err("o filete completo precisa de duas faces laterais planas".into());
                        }
                        let width = geo3d::dot(geo3d::sub(fa.origin, fc.origin), fa.dir).abs();
                        let (s, out) = b.full_round_tagged(*mid, width, &tags).map_err(k)?;
                        let own = remap(owners, &out, f.id);
                        self.bodies.insert(f.target, (s, own));
                        return Ok(None);
                    }
                    _ => {}
                }
                if o.mode != EdgeMode::Constant {
                    let EdgeSel::Edges(mids) = &o.edges else {
                        return Err("esta variação precisa das arestas escolhidas na viewport".into());
                    };
                    let tol = b.bbox().map(|(a, c)| (0..3).map(|i| (c[i] - a[i]).powi(2)).sum::<f64>().sqrt() * 0.05).unwrap_or(1.).max(1.);
                    let (mode, v2) = match &o.mode {
                        EdgeMode::Variable(x) => (0, ev(x, &self.vars, "raio final")?),
                        EdgeMode::TwoDist(x) => (1, ev(x, &self.vars, "segunda distância")?),
                        EdgeMode::DistAngle(x) => (2, ev(x, &self.vars, "ângulo")?),
                        _ => unreachable!(),
                    };
                    let (s, out) = b.edges_mode_tagged(mode, v, v2, mids, tol, &tags).map_err(k)?;
                    let own = remap(owners, &out, f.id);
                    self.bodies.insert(f.target, (s, own));
                    return Ok(None);
                }
                let r = match &o.edges {
                    EdgeSel::Edges(mids) if !mids.is_empty() => {
                        let tol = b.bbox().map(|(a, c)| (0..3).map(|i| (c[i] - a[i]).powi(2)).sum::<f64>().sqrt() * 0.05).unwrap_or(1.).max(1.);
                        b.edges_tagged(v, mids, tol, chamfer, &tags)
                    }
                    _ if chamfer => b.chamfer_tagged(v, &tags),
                    _ => b.fillet_tagged(v, &tags),
                };
                let (s, out) = r.map_err(k)?;
                let own = remap(owners, &out, f.id);
                self.bodies.insert(f.target, (s, own));
                Ok(match &o.edges {
                    EdgeSel::CreatedBy(_) => Some("seleção por comando ainda não implementada: aplicado a todas as arestas".into()),
                    EdgeSel::Edges(m) if m.is_empty() => Some("nenhuma aresta escolhida: aplicado a todas".into()),
                    _ => None,
                })
            }
        }
    }
}

/// Recalcula o documento inteiro. Roda fora da thread da interface (RNF-001).
pub fn recompute(doc: &Document) -> RecomputeResult {
    let t0 = Instant::now();
    let (vars, param_error) = match doc.eval_params() {
        Ok(v) => (v, None),
        Err(e) => (HashMap::new(), Some(e.to_string())),
    };
    let mut cx = Ctx { doc, vars, bodies: HashMap::new(), sketches: HashMap::new(), status: HashMap::new(), tools: HashMap::new() };

    // 1º: sketches livres (pastas de geometria, sem linha do tempo); 2º: comandos dos coletores em ordem
    let order: Vec<usize> = (0..doc.features.len())
        .filter(|i| doc.is_free(&doc.features[*i]))
        .chain((0..doc.features.len()).filter(|i| !doc.is_free(&doc.features[*i])))
        .collect();
    for i in order {
        let f = &doc.features[i];
        if !doc.is_active(i) {
            cx.status.insert(f.id, Status::Inactive);
            continue;
        }
        let st = match cx.run(f) {
            Ok(None) => Status::Ok,
            Ok(Some(w)) => Status::Warning(w),
            Err(e) => Status::Error(e),
        };
        cx.status.insert(f.id, st);
    }

    // planos: Origem + Referências
    let mut planes = vec![];
    if doc.origin_visible {
        for (p, n) in [(OriginPlane::XY, "XY"), (OriginPlane::XZ, "XZ"), (OriginPlane::YZ, "YZ")] {
            planes.push(PlaneViz { id: None, name: n.into(), frame: origin_frame(p) });
        }
    }
    let (mut points, mut curves, mut surfaces) = (vec![], vec![], vec![]);
    for r in &doc.references {
        match cx.geo(r.id, 0) {
            Ok(g) => {
                cx.status.insert(r.id, Status::Ok);
                if !r.visible {
                    continue;
                }
                match g {
                    Geo::Plane(fr) => planes.push(PlaneViz { id: Some(r.id), name: r.name.clone(), frame: fr }),
                    Geo::Point(p) => points.push(PointViz { id: r.id, name: r.name.clone(), p }),
                    Geo::Curve(c) => curves.push(CurveViz { id: r.id, name: r.name.clone(), pts: c.pts }),
                    Geo::Surface(sh) => {
                        if let Ok(mesh) = sh.mesh(0.05) {
                            surfaces.push(SurfaceViz { id: r.id, name: r.name.clone(), mesh });
                        }
                    }
                }
            }
            Err(e) => {
                cx.status.insert(r.id, Status::Error(e));
            }
        }
    }

    // sketches para exibição
    let mut sketches = vec![];
    for (id, (fr, es)) in &cx.sketches {
        let mut v = SketchViz {
            feature: *id,
            frame: *fr,
            lines: vec![],
            construction: vec![],
            points: vec![],
            fill: vec![],
            dof: es.report.dof,
            converged: es.report.converged,
        };
        // sombra: a face real (com furos internos) tesselada pelo kernel
        if !es.loops.is_empty() {
            if let Ok(m) = Shape::face(fr, &es.loops).and_then(|f| f.mesh(0.2)) {
                for t in m.indices.chunks_exact(3) {
                    for &i in t {
                        let k = i as usize * 3;
                        v.fill.push([m.positions[k], m.positions[k + 1], m.positions[k + 2]]);
                    }
                }
            }
        }
        for (cons, pts) in &es.viz {
            let dst = if *cons { &mut v.construction } else { &mut v.lines };
            for w in pts.windows(2) {
                dst.push(to3d(fr, w[0]));
                dst.push(to3d(fr, w[1]));
            }
        }
        v.points = es.points.iter().map(|p| to3d(fr, *p)).collect();
        sketches.push(v);
    }

    // malhas dos corpos
    let mut bodies = vec![];
    for c in &doc.collectors {
        let Some((s, owners)) = cx.body_owned(c.id) else { continue };
        let defl = s
            .bbox()
            .map(|(a, b)| ((0..3).map(|i| (b[i] - a[i]).powi(2)).sum::<f64>().sqrt() * 0.0015).clamp(0.005, 1.0))
            .unwrap_or(0.1);
        match s.mesh(defl) {
            Ok(mesh) => bodies.push(BodyResult {
                collector: c.id,
                color: c.color,
                mass: s.mass(),
                faces: s.face_count(),
                face_owner: owners.clone(),
                mesh,
            }),
            Err(e) => {
                cx.status.insert(c.id, Status::Error(e.0));
            }
        }
    }

    RecomputeResult {
        bodies,
        sketches,
        planes,
        points,
        curves,
        surfaces,
        status: cx.status,
        params: cx.vars,
        param_error,
        elapsed_ms: t0.elapsed().as_secs_f64() * 1000.,
        kernel: scc_kernel::version(),
    }
}

/// Publicações da peça (elementos e parâmetros marcados como publicados), já calculadas,
/// para outras peças copiarem por link.
pub fn publications(doc: &Document) -> Vec<(String, LinkedGeo)> {
    let r = recompute(doc);
    let mut out = vec![];
    for e in doc.references.iter().filter(|e| e.published) {
        if let Some(pl) = r.planes.iter().find(|p| p.id == Some(e.id)) {
            out.push((e.name.clone(), LinkedGeo::Plane { origin: pl.frame.origin, x: pl.frame.x, normal: pl.frame.normal }));
        } else if let Some(pt) = r.points.iter().find(|p| p.id == e.id) {
            out.push((e.name.clone(), LinkedGeo::Point(pt.p)));
        } else if let Some(c) = r.curves.iter().find(|c| c.id == e.id) {
            out.push((e.name.clone(), LinkedGeo::Curve(c.pts.clone())));
        }
    }
    for p in doc.params.iter().filter(|p| p.published) {
        if let Some(v) = r.params.get(&p.name) {
            out.push((p.name.clone(), LinkedGeo::Value(*v)));
        }
    }
    out
}

/// Exporta o corpo de um coletor (ou de todos) após recalcular.
pub fn export(doc: &Document, collector: Option<Id>, path: &str) -> Result<(), String> {
    let vars = doc.eval_params().map_err(|e| e.to_string())?;
    let mut cx = Ctx { doc, vars, bodies: HashMap::new(), sketches: HashMap::new(), status: HashMap::new(), tools: HashMap::new() };
    for f in doc.active_features().map(|(_, f)| f) {
        let _ = cx.run(f);
    }
    let mut all = Shape::empty();
    for c in &doc.collectors {
        if collector.is_some_and(|x| x != c.id) {
            continue;
        }
        if let Some(b) = cx.body(c.id) {
            all = if all.is_null() { b.clone() } else { all.compound(b) };
        }
    }
    if all.is_null() {
        return Err("nada para exportar".into());
    }
    let lower = path.to_lowercase();
    let r = if lower.ends_with(".stl") { all.export_stl(path) } else { all.export_step(path) };
    r.map_err(|e| e.0)
}

/// Ponto fixo de construção na origem do sketch (referência para cotas).
pub fn origin_point(sk: &mut SketchData, al: &mut impl FnMut() -> Id) -> Id {
    let o = sk.add_point(al, 0., 0.);
    sk.point_mut(o).unwrap().fixed = true;
    sk.add_curve(al, Curve::Point { p: o }, true);
    o
}

/// Exemplo usado pelo comando "Nova peça de exemplo" e pelos testes:
/// placa parametrizada com rasgo, furos rebaixados M6 e filete.
pub fn example_document() -> Document {
    let s = |v: &str| v.to_string();
    let mut d = Document::new("Placa de exemplo");
    d.props.pn = s("SCC-0001");
    d.props.description = s("Placa base");
    d.add_param("L", "120", Unit::Comprimento).unwrap();
    d.add_param("W", "L*0.6", Unit::Comprimento).unwrap();
    d.add_param("esp", "10", Unit::Comprimento).unwrap();
    d.add_param("margem", "12", Unit::Comprimento).unwrap();

    // sketch base: retângulo L × W centrado na origem + rasgo central, tudo por restrições
    let mut sk = SketchData::default();
    {
        let mut al = || d.alloc();
        let o = origin_point(&mut sk, &mut al);
        let (p, l) = sk.add_rect(&mut al, [-60., -36.], [60., 36.], false);
        let diag = sk.add_curve(&mut al, Curve::Line { a: p[0], b: p[2] }, true);
        sk.add_constraint(&mut al, Kind::Midpoint(o, diag));
        sk.add_constraint(&mut al, Kind::Length(l[0], s("L")));
        sk.add_constraint(&mut al, Kind::Length(l[1], s("W")));
        let (c1, c2, arc) = sk.add_slot(&mut al, [-20., 0.], [20., 0.], 6.);
        let axis = sk.curves.last().unwrap().id;
        sk.add_constraint(&mut al, Kind::Midpoint(o, axis));
        sk.add_constraint(&mut al, Kind::HorizontalPts(c1, c2));
        sk.add_constraint(&mut al, Kind::Distance(c1, c2, s("L/3")));
        sk.add_constraint(&mut al, Kind::Radius(arc, s("6")));
    }
    let base = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: sk }));
    d.feature_mut(base).unwrap().name = s("Sketch base");
    let ex = d.add_feature(FeatureKind::Extrude(Extrude { sketch: base, op: BoolOp::Add, extent: Extent::Blind(s("esp")), reverse: false, taper: s("0"), thin: None }));
    d.feature_mut(ex).unwrap().name = s("Placa");

    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "esp");
    d.references.last_mut().unwrap().name = s("PlanoTopo");
    // centros dos furos: quatro pontos cotados a partir da origem
    let mut sk = SketchData::default();
    {
        let mut al = || d.alloc();
        let o = origin_point(&mut sk, &mut al);
        for (sx, sy) in [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)] {
            let p = sk.add_point(&mut al, sx * 48., sy * 24.);
            sk.add_curve(&mut al, Curve::Point { p }, false);
            sk.add_constraint(&mut al, Kind::DistanceX(o, p, s("L/2-margem")));
            sk.add_constraint(&mut al, Kind::DistanceY(o, p, s("W/2-margem")));
        }
    }
    let skf = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: sk }));
    d.feature_mut(skf).unwrap().name = s("Centros dos furos");
    // sketches consumidos por comandos ficam ocultos, como nos CADs usuais
    d.feature_mut(base).unwrap().hidden = true;
    d.feature_mut(skf).unwrap().hidden = true;
    let m6 = hardware::metric("M6").unwrap();
    let furo = d.add_feature(FeatureKind::Hole(Hole {
        sketch: skf,
        kind: HoleKind::Rebaixado { diameter: m6.cbore_diameter.to_string(), depth: m6.cbore_depth.to_string() },
        diameter: m6.clearance[1].to_string(),
        depth: Extent::ThroughAll,
        thread: None,
        thread_modeled: false,
        standard: Some(s("M6 passagem normal")),
    }));
    d.feature_mut(furo).unwrap().name = s("Furos M6");
    let fol = d.add_folder("Acabamentos");
    let fil = d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("1"), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant }));
    d.feature_mut(fil).unwrap().folder = Some(fol);
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modos_extras_superficies_links_e_booleanas() {
        let mut d = Document::new("t");
        let s_ = |x: &str| x.to_string();
        // linha de interseção XY ∩ XZ = eixo X
        let li = d.add_geometry(RefKind::LinePlanes { a: PlaneRef::Origin(OriginPlane::XY), b: PlaneRef::Origin(OriginPlane::XZ), length: s_("40") });
        let p10 = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
        let pm = d.add_geometry(RefKind::PlaneMid { a: PlaneRef::Origin(OriginPlane::XY), b: PlaneRef::Datum(p10) });
        let sp = d.add_geometry(RefKind::SplineControl { points: vec![PointRef::Origin, PointRef::Fixed([10., 10., 0.]), PointRef::Fixed([20., 0., 5.])], closed: false });
        let ci = d.add_geometry(RefKind::PointCurvePlane { curve: CurveRef::Ref(sp), plane: PlaneRef::Origin(OriginPlane::YZ), which: 0 });
        let off = d.add_geometry(RefKind::PointOffset { point: PointRef::Origin, dir: AxisRef::Origin(OriginAxis::Z), dist: s_("7") });
        let bis = d.add_geometry(RefKind::LineBisector { a: CurveRef::Axis(OriginAxis::X), b: CurveRef::Axis(OriginAxis::Y), length: s_("10") });
        let r = recompute(&d);
        for id in [li, pm, sp, ci, off, bis] {
            assert_eq!(r.status[&id], Status::Ok, "{}: {:?}", d.name_of(id), r.status[&id]);
        }
        let c = r.curves.iter().find(|c| c.id == li).unwrap();
        assert!(c.pts.iter().all(|p| p[1].abs() < 1e-9 && p[2].abs() < 1e-9));
        assert!((r.planes.iter().find(|p| p.id == Some(pm)).unwrap().frame.origin[2] - 5.).abs() < 1e-9);
        assert!(geo3d::dist(r.points.iter().find(|p| p.id == off).unwrap().p, [0., 0., 7.]) < 1e-9);
        let b = r.curves.iter().find(|c| c.id == bis).unwrap();
        let e = b.pts.last().unwrap();
        assert!((e[0].abs() - e[1].abs()).abs() < 1e-6);

        if !scc_kernel::AVAILABLE {
            return;
        }
        // superfícies: extrusão de curva, preenchimento, deslocamento
        let se = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(sp), dir: AxisRef::Origin(OriginAxis::Z), length: s_("10") });
        let so = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(se), dist: s_("2") });
        let sq = d.add_geometry(RefKind::Polyline { points: vec![PointRef::Origin, PointRef::Fixed([10., 0., 0.]), PointRef::Fixed([10., 10., 3.]), PointRef::Fixed([0., 10., 0.])], closed: true });
        let sf = d.add_geometry(RefKind::SurfFill { curves: vec![CurveRef::Ref(sq)] });
        let pp = d.add_geometry(RefKind::PointProject { point: PointRef::Fixed([5., 5., 20.]), surface: SurfRef::Ref(sf) });
        let r = recompute(&d);
        for id in [se, so, sq, sf, pp] {
            assert_eq!(r.status[&id], Status::Ok, "{}: {:?}", d.name_of(id), r.status[&id]);
        }
        assert_eq!(r.surfaces.len(), 3);
        assert!(r.points.iter().find(|p| p.id == pp).unwrap().p[2] < 5.);

        // links: publica um plano e um parâmetro e importa noutra peça
        let mut src = example_document();
        let pl = src.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "33");
        src.references.iter_mut().find(|r| r.id == pl).unwrap().published = true;
        src.params[0].published = true;
        let pubs = publications(&src);
        assert_eq!(pubs.len(), 2);
        let mut dst = Document::new("dst");
        for (n, g) in pubs {
            dst.add_link("Placa", "placa.sccpart", &n, g);
        }
        let r = recompute(&dst);
        assert!(dst.references.iter().all(|x| r.status[&x.id] == Status::Ok));
        assert!(r.planes.iter().any(|p| (p.frame.origin[2] - 33.).abs() < 1e-9));
        assert_eq!(dst.params.len(), 1);

        // booleana entre corpos e até o próximo
        let mut d = caixa();
        let c2 = d.add_collector("Ferramenta");
        d.active_collector = c2;
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Origin(OriginPlane::XY), a: s_("5"), b: s_("40"), c: s_("0"), d: s_("0"), op: BoolOp::Add }));
        let c1 = d.collectors[0].id;
        d.active_collector = c1;
        d.add_feature(FeatureKind::Combine(Combine { tool: c2, op: BoolOp::Cut, keep_tool: false }));
        let r = recompute(&d);
        assert_eq!(r.bodies.len(), 1);
        assert!((r.bodies[0].mass.volume - (24000. - std::f64::consts::PI * 25. * 20.)).abs() < 1.);

        // filete completo: nervura 4 mm de largura vira meia-cana
        let mut d = Document::new("t");
        let s1 = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-2., -20.], [2., 20.]);
        d.add_feature(ext(s1, BoolOp::Add, "10"));
        d.add_feature(FeatureKind::Fillet(EdgeOp {
            value: s_("1"),
            edges: EdgeSel::AllOfBody,
            mode: EdgeMode::FullRound { mid: [0., 0., 10.], a: [2., 0., 5.], b: [-2., 0., 5.] },
        }));
        let v = vol(&d);
        assert!(v < 4. * 40. * 10. && v > 4. * 40. * 8., "filete completo {v}");
    }

    fn sk_rect(d: &mut Document, plane: PlaneRef, a: [f64; 2], b: [f64; 2]) -> Id {
        let mut data = SketchData::default();
        let mut n = 1_000_000u64 + d.features.len() as u64 * 1000;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, a, b, false);
        d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
    }

    fn sk_lines(d: &mut Document, plane: PlaneRef, segs: &[([f64; 2], [f64; 2])]) -> Id {
        let mut data = SketchData::default();
        let mut n = 2_000_000u64 + d.features.len() as u64 * 1000;
        let mut al = || {
            n += 1;
            Id(n)
        };
        for (a, b) in segs {
            let pa = data.add_point(&mut al, a[0], a[1]);
            let pb = data.add_point(&mut al, b[0], b[1]);
            data.add_curve(&mut al, scc_core::sketch::Curve::Line { a: pa, b: pb }, false);
        }
        d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
    }

    fn ext(sketch: Id, op: BoolOp, d: &str) -> FeatureKind {
        FeatureKind::Extrude(Extrude { sketch, op, extent: Extent::Blind(d.into()), reverse: false, taper: "0".into(), thin: None })
    }

    /// Caixa 40 × 30 × 20 centrada na origem (base em z = 0).
    fn caixa() -> Document {
        let mut d = Document::new("t");
        let s = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
        d.add_feature(ext(s, BoolOp::Add, "20"));
        d
    }

    fn vol(d: &Document) -> f64 {
        let r = recompute(d);
        for f in &d.features {
            // sketch de linhas abertas (reforço) avisa que não há perfil fechado
            let sketch_warn = matches!(f.kind, FeatureKind::Sketch(_)) && matches!(r.status[&f.id], Status::Warning(_));
            assert!(r.status[&f.id] == Status::Ok || sketch_warn, "{}: {:?}", f.name, r.status[&f.id]);
        }
        r.bodies.iter().map(|b| b.mass.volume).sum()
    }

    #[test]
    fn features_de_forma_acabamento_e_padroes() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let v0 = vol(&caixa());
        assert!(near(v0, 24000., 1e-6));

        // casca abrindo o topo
        let mut d = caixa();
        d.add_feature(FeatureKind::Shell(Shell { faces: vec![[0., 0., 20.]], thickness: "2".into(), outward: false }));
        let v = vol(&d);
        assert!(v < v0 * 0.5 && v > 0., "casca {v}");

        // ângulo de saída numa face lateral
        let mut d = caixa();
        d.add_feature(FeatureKind::Draft(Draft { faces: vec![[20., 0., 10.]], neutral: PlaneRef::Origin(OriginPlane::XY), angle: "5".into(), reverse: false }));
        assert!(!near(vol(&d), v0, 1e-6), "saída sem efeito");

        // espessura +5 no topo
        let mut d = caixa();
        d.add_feature(FeatureKind::Thickness(Thickness { faces: vec![[0., 0., 20.]], value: "5".into() }));
        assert!(near(vol(&d), v0 + 6000., 1e-6));

        // padrão linear de um bolso 4×4×5: 3 cópias ao longo de X
        let mut d = caixa();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let sp = sk_rect(&mut d, PlaneRef::Datum(top), [-16., -2.], [-12., 2.]);
        let pocket = d.add_feature(ext(sp, BoolOp::Cut, "5"));
        let v1 = vol(&d);
        assert!(near(v1, v0 - 80., 1e-6));
        d.add_feature(FeatureKind::Pattern(Pattern {
            kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: "3".into(), spacing: "8".into(), dir2: None, count2: "1".into(), spacing2: "0".into() },
            sources: vec![pocket],
        }));
        assert!(near(vol(&d), v0 - 240., 1e-6));
        // espelho do bolso pelo YZ e padrão circular de 4 em volta de Z
        let mut e = d.clone();
        e.add_feature(FeatureKind::Mirror(Mirror { plane: PlaneRef::Origin(OriginPlane::YZ), sources: vec![pocket] }));
        assert!(near(vol(&e), v0 - 320., 1e-6));
        let mut e = d.clone();
        e.features.pop();
        e.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Z), count: "2".into(), angle: "360".into() }, sources: vec![pocket] }));
        assert!(near(vol(&e), v0 - 160., 1e-6));

        // primitiva: cilindro r5 h10 sobre o topo
        let mut d = caixa();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Datum(top), a: "5".into(), b: "10".into(), c: "0".into(), d: "0".into(), op: BoolOp::Add }));
        assert!(near(vol(&d), v0 + std::f64::consts::PI * 250., 1e-3));
        // rosca externa modelada no cilindro (M10)
        let mut e = d.clone();
        e.add_feature(FeatureKind::Thread(Thread { face: [5., 0., 25.], spec: "M10x1.5".into(), length: "8".into(), modeled: true }));
        let vt = vol(&e);
        assert!(vt < vol(&d) - 1., "rosca externa {vt}");

        // escala ×2
        let mut d = caixa();
        d.add_feature(FeatureKind::Scale(Scale { center: PointRef::Origin, factor: "2".into() }));
        assert!(near(vol(&d), v0 * 8., 1e-6));

        // extrusão com ângulo e parede fina
        let mut d = Document::new("t");
        let s = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-10., -10.], [10., 10.]);
        let id = d.add_feature(ext(s, BoolOp::Add, "10"));
        if let Some(FeatureKind::Extrude(x)) = d.feature_mut(id).map(|f| &mut f.kind) {
            x.taper = "5".into();
        }
        let v = vol(&d);
        assert!(v < 4000. && v > 3000., "cônica {v}");
        if let Some(FeatureKind::Extrude(x)) = d.feature_mut(id).map(|f| &mut f.kind) {
            x.taper = "0".into();
            x.thin = Some("2".into());
        }
        let v = vol(&d);
        assert!(near(v, 80. * 2. * 10., 0.02), "parede fina {v}");

        // multi-seções: quadrado 20 em z=0 até quadrado 10 em z=20
        let mut d = Document::new("t");
        let a = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-10., -10.], [10., 10.]);
        let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let b = sk_rect(&mut d, PlaneRef::Datum(pl), [-5., -5.], [5., 5.]);
        d.add_feature(FeatureKind::Loft(Loft { sections: vec![a, b], ruled: true, closed: false, op: BoolOp::Add }));
        // tronco de pirâmide: h/3 (A1 + A2 + sqrt(A1 A2))
        assert!(near(vol(&d), 20. / 3. * (400. + 100. + 200.), 1e-3));

        // nervura: perfil 4×4 no YZ varrido 50 mm ao longo de X
        let mut d = Document::new("t");
        let s = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::YZ), [-2., -2.], [2., 2.]);
        let ln = d.add_geometry(RefKind::LinePoints { a: PointRef::Origin, b: PointRef::Fixed([50., 0., 0.]), start: "0".into(), end: "0".into() });
        d.add_feature(FeatureKind::Sweep(Sweep { sketch: s, spine: CurveRef::Ref(ln), mode: SweepMode::Corrigido, op: BoolOp::Add, thin: None }));
        assert!(near(vol(&d), 16. * 50., 1e-3));

        // reforço: linha de 20 mm, espessura 2, profundidade 10 (não precisa de corpo)
        let mut d = Document::new("t");
        let s = sk_lines(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([-10., 0.], [10., 0.])]);
        d.add_feature(FeatureKind::Stiffener(Stiffener { sketch: s, thickness: "2".into(), depth: "10".into(), reverse: true, to_body: false }));
        let v = vol(&d);
        assert!(near(v, (20. * 2. + std::f64::consts::PI) * 10., 1e-3), "reforço {v}");

        // filete variável e chanfro por distância e ângulo numa aresta do topo
        for mode in [EdgeMode::Variable("4".into()), EdgeMode::TwoDist("3".into()), EdgeMode::DistAngle("30".into())] {
            let mut d = caixa();
            let op = EdgeOp { value: "2".into(), edges: EdgeSel::Edges(vec![[0., 15., 20.]]), mode: mode.clone() };
            d.add_feature(if matches!(mode, EdgeMode::Variable(_)) { FeatureKind::Fillet(op) } else { FeatureKind::Chamfer(op) });
            assert!(vol(&d) < v0, "{mode:?}");
        }
    }

    #[test]
    fn rosca_modelada_remove_material() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        let mut d = example_document();
        let id = d.features.iter().find(|f| matches!(f.kind, FeatureKind::Hole(_))).unwrap().id;
        if let Some(FeatureKind::Hole(h)) = d.feature_mut(id).map(|f| &mut f.kind) {
            h.thread = Some("M6".into());
            h.diameter = "5".into();
            h.thread_modeled = false;
        }
        let v0 = recompute(&d).bodies[0].mass.volume;
        if let Some(FeatureKind::Hole(h)) = d.feature_mut(id).map(|f| &mut f.kind) {
            h.thread_modeled = true;
        }
        let r = recompute(&d);
        assert_eq!(r.status[&id], Status::Ok);
        assert!(r.bodies[0].mass.volume < v0 - 1., "a rosca não tirou material");
    }

    #[test]
    fn geometria_de_referencia_planos_pontos_curvas() {
        let mut d = Document::new("geo");
        let x = |v: f64| v.to_string();
        let p1 = d.add_geometry(RefKind::PointCoords { x: x(10.), y: x(0.), z: x(0.), frame: None });
        let p2 = d.add_geometry(RefKind::PointCoords { x: x(0.), y: x(10.), z: x(0.), frame: None });
        let p3 = d.add_geometry(RefKind::PointCoords { x: x(0.), y: x(0.), z: x(10.), frame: None });
        let pl = d.add_geometry(RefKind::PlaneThreePoints { a: PointRef::Ref(p1), b: PointRef::Ref(p2), c: PointRef::Ref(p3) });
        let ln = d.add_geometry(RefKind::LinePoints { a: PointRef::Ref(p1), b: PointRef::Ref(p2), start: "0".into(), end: "0".into() });
        let mid = d.add_geometry(RefKind::PointOnCurve { curve: CurveRef::Ref(ln), at: OnCurve::Ratio("0.5".into()), reverse: false });
        let nc = d.add_geometry(RefKind::PlaneNormalCurve { curve: CurveRef::Ref(ln), at: OnCurve::Ratio("0.5".into()), reverse: false });
        let ci = d.add_geometry(RefKind::Circle3 { a: PointRef::Ref(p1), b: PointRef::Ref(p2), c: PointRef::Ref(p3), full: true });
        let ce = d.add_geometry(RefKind::PointCenter { curve: CurveRef::Ref(ci) });
        let eq = d.add_geometry(RefKind::PlaneEquation { a: "0".into(), b: "0".into(), c: "2".into(), d: "10".into() });
        let hx = d.add_geometry(RefKind::Helix { origin: PointRef::Origin, axis: AxisRef::Origin(OriginAxis::Z), radius: "5".into(), pitch: "2".into(), height: "10".into(), taper: "0".into(), left: false });
        let ruim = d.add_geometry(RefKind::PlaneThreePoints { a: PointRef::Ref(p1), b: PointRef::Ref(p1), c: PointRef::Ref(p2) });
        let r = recompute(&d);
        for id in [p1, pl, ln, mid, nc, ci, ce, eq, hx] {
            assert_eq!(r.status[&id], Status::Ok, "{}", d.name_of(id));
        }
        assert!(matches!(r.status[&ruim], Status::Error(_)));
        let pt = |id: Id| r.points.iter().find(|p| p.id == id).unwrap().p;
        assert!(geo3d::dist(pt(mid), [5., 5., 0.]) < 1e-9);
        let c = 10. / 3.;
        assert!(geo3d::dist(pt(ce), [c, c, c]) < 1e-6);
        let fr = |id: Id| r.planes.iter().find(|p| p.id == Some(id)).unwrap().frame;
        assert!((fr(eq).origin[2] - 5.).abs() < 1e-9);
        let n = fr(pl).normal;
        assert!((n[0] - n[1]).abs() < 1e-9 && (n[1] - n[2]).abs() < 1e-9);
        // apagar o ponto leva junto o que depende dele (dependentes)
        assert!(d.dependents(p1).contains(&ce));
    }

    #[test]
    fn exemplo_recalcula_sem_erros() {
        let d = example_document();
        let r = recompute(&d);
        assert!(r.param_error.is_none());
        if !scc_kernel::AVAILABLE {
            return;
        }
        for f in &d.features {
            assert_eq!(r.status[&f.id], Status::Ok, "{}: {:?}", f.name, r.status[&f.id]);
        }
        assert_eq!(r.bodies.len(), 1);
        let b = &r.bodies[0];
        assert!(b.mass.volume > 0.);
        // cada face tem dono; furo, filete e placa aparecem como donos
        assert_eq!(b.face_owner.len(), b.mesh.face_ranges.len());
        let owners: std::collections::HashSet<Id> = b.face_owner.iter().copied().collect();
        for name in ["Placa", "Furos M6", "Filete1"] {
            let id = d.features.iter().find(|f| f.name == name).unwrap().id;
            assert!(owners.contains(&id), "{name} sem faces");
        }
    }

    #[test]
    fn extrusao_ate_plano_inclinado_e_filete_em_aresta() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        let mut d = example_document();
        // plano a 30° em torno do eixo X, deslocado 20 mm acima, e extrusão do sketch base até ele
        let base = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let inclinado = d.add_angle_plane(PlaneRef::Datum(base), AxisRef::Origin(OriginAxis::X), "30");
        let sk = d.features[0].id;
        if let Some(FeatureKind::Extrude(e)) = d.feature_mut(d.features[1].id).map(|f| &mut f.kind) {
            assert_eq!(e.sketch, sk);
            e.extent = Extent::UpTo(PlaneRef::Datum(inclinado));
        }
        let r = recompute(&d);
        let placa = &d.features[1];
        assert_eq!(r.status[&placa.id], Status::Ok, "{:?}", r.status[&placa.id]);
        // filete só em uma aresta escolhida (aresta inferior frontal: y = -36, z = 0)
        let fil = d.features.iter().find(|f| matches!(f.kind, FeatureKind::Fillet(_))).unwrap().id;
        if let Some(FeatureKind::Fillet(o)) = d.feature_mut(fil).map(|f| &mut f.kind) {
            o.edges = EdgeSel::Edges(vec![[0., -36., 0.]]);
        }
        let r = recompute(&d);
        assert_eq!(r.status[&fil], Status::Ok, "{:?}", r.status[&fil]);
    }

    #[test]
    fn referencia_perdida_nao_apaga_comando() {
        let mut d = example_document();
        let sk = d.features[0].id;
        d.delete(sk, false);
        let r = recompute(&d);
        let ex = &d.features[0];
        assert!(matches!(&r.status[&ex.id], Status::Error(m) if m.contains("referência perdida")));
    }

    #[test]
    fn parametro_muda_volume() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        let mut d = example_document();
        let v1 = recompute(&d).bodies[0].mass.volume;
        d.params[2].expr = "20".into(); // esp
        let v2 = recompute(&d).bodies[0].mass.volume;
        assert!(v2 > v1 * 1.8);
    }
}
