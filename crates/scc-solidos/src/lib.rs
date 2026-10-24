//! Módulo Modelagem Sólida: executa a linha de tempo do documento sobre o kernel.
//!
//! Regras (RF-NUC-008, RF-NUC-024): um comando com erro **não** é apagado nem invalida o resto;
//! o corpo continua do estado anterior e o comando fica marcado com a mensagem do erro.

pub mod geo3d;
pub mod mesh_io;
pub mod structural;
pub mod hardware;
pub mod profile;

use profile::{ev, eval_sketch, to3d, EvalSketch, Vars};
use scc_core::sketch::{Curve, Kind};
use scc_core::*;
use scc_kernel::{Bool, Frame, HoleShape, KernelError, Mesh, MassProps, Shape, Xform};
use geo3d::{Curve3, V3};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
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
    /// posições atuais da geometria externa (o editor do sketch copia para o documento)
    pub link_pts: Vec<(Id, [f64; 2])>,
    pub link_r: Vec<(Id, f64)>,
    /// cada curva do sketch no mundo (id, construção, polilinha) — subelementos selecionáveis
    pub ents: Vec<(Id, bool, Vec<[f32; 3]>)>,
    /// cada ponto do sketch no mundo (id, posição)
    pub pt_ids: Vec<(Id, [f32; 3])>,
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
    /// vértices de definição (pontas da linha, pontos da polilinha/spline) — selecionáveis
    pub verts: Vec<[f64; 3]>,
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
    sketches: HashMap<Id, (Frame, Arc<EvalSketch>)>,
    /// sólido gerado por cada comando (base para filete/chanfro por comando)
    status: HashMap<Id, Status>,
    /// ferramenta e operação de cada comando de forma (para padrões e espelho)
    tools: HashMap<Id, (Shape, BoolOp)>,
    /// geometria de referência já avaliada neste cálculo (cadeias de referências não recalculam)
    memo: RefCell<HashMap<Id, Geo>>,
    /// assinatura do último passo que mudou o corpo de cada coletor (cache de malha)
    body_sig: HashMap<Id, u64>,
    /// medidas da última aba de chapa (para os alívios)
    last_sheet: Option<scc_kernel::SheetEdgeInfo>,
}

/// Resultado guardado de um comando: o que ele produziu, válido enquanto as entradas que ele lê
/// (corpo de destino, sketches, outros comandos, corpos citados, referências) não mudarem.
#[derive(Clone)]
struct FeatEntry {
    sig: u64,
    status: Status,
    /// corpos que o comando pode ter mudado (destino e corpos citados): estado depois dele
    bodies: Vec<(Id, Option<(Shape, Vec<Id>)>)>,
    sketch: Option<(Frame, Arc<EvalSketch>)>,
    tool: Option<(Shape, BoolOp)>,
    last_sheet: Option<scc_kernel::SheetEdgeInfo>,
}

/// Cache entre cálculos, por dependência: editar um comando refaz só ele e quem lê o que ele
/// produziu; o resto (outros corpos, outros ramos) vem pronto. Também guarda malhas.
#[derive(Default)]
pub struct Cache {
    feats: HashMap<Id, FeatEntry>,
    meshes: HashMap<(Id, u64), (Mesh, MassProps, u32)>,
    fills: HashMap<(Id, u64), Vec<[f32; 3]>>,
    /// geometria de referência já avaliada: (assinatura, resultado) — só refaz o que mudou
    refs: HashMap<Id, (u64, Result<RefViz, String>)>,
    /// quantos comandos foram reaproveitados no último cálculo (diagnóstico)
    pub reused: usize,
}

/// Valor exibível de uma referência (guardado no cache).
#[derive(Clone)]
enum RefViz {
    Plane(Frame),
    Point([f64; 3]),
    Curve(Vec<[f64; 3]>),
    Surface(Mesh),
}

/// Assinatura só da geometria de um comando: nome, visibilidade e aparência não mudam o sólido.
fn feature_sig(f: &Feature) -> String {
    let mut g = f.clone();
    g.name.clear();
    g.hidden = false;
    g.appearance = None;
    serde_json::to_string(&g).unwrap_or_default()
}

/// A referência (ou alguma entrada dela) lê faces/corpos? Então muda quando a linha do tempo muda.
fn ref_reads_bodies(doc: &Document, id: Id, depth: u32) -> bool {
    let Some(r) = doc.reference(id) else { return false };
    if depth > 24 {
        return true;
    }
    let js = serde_json::to_string(&r.kind).unwrap_or_default();
    if ["\"Face\"", "SurfExtract", "SurfMulti", "LineAxis", "\"Plane\""].iter().any(|k| js.contains(k)) {
        return true;
    }
    let mut ids = vec![];
    r.kind.clone().walk(&mut |x| ids.push(*x), &mut |_| {});
    ids.into_iter().any(|x| x != id && ref_reads_bodies(doc, x, depth + 1))
}

/// O texto cita o nome (palavra inteira)?
fn cites(text: &str, name: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(name).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + name.len()..].chars().next();
        !before.is_some_and(word) && !after.is_some_and(word)
    })
}

/// Valores dos parâmetros citados no texto (só eles entram na assinatura de quem os usa).
fn vars_sig(text: &str, vars: &Vars) -> String {
    let mut v: Vec<(&String, u64)> = vars.iter().filter(|(k, _)| cites(text, k)).map(|(k, v)| (k, v.to_bits())).collect();
    v.sort();
    format!("{v:?}")
}

/// Assinatura de uma referência: a definição dela, das entradas dela e dos parâmetros que usam.
fn ref_sig(doc: &Document, id: Id, vars: &Vars, depth: u32) -> String {
    let Some(r) = doc.reference(id) else { return format!("?{}", id.0) };
    let js = serde_json::to_string(&r.kind).unwrap_or_default();
    let mut out = format!("{}{}{}", id.0, js, vars_sig(&js, vars));
    if depth < 24 {
        let mut ids = vec![];
        r.kind.clone().walk(&mut |x| ids.push(*x), &mut |_| {});
        for x in ids.into_iter().filter(|x| *x != id) {
            out.push_str(&ref_sig(doc, x, vars, depth + 1));
        }
    }
    out
}

fn hash_of(parts: &[&str], prev: u64) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    prev.hash(&mut h);
    for p in parts {
        p.hash(&mut h);
    }
    h.finish()
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
            PointRef::SketchPoint { sketch, point } => {
                let (fr, es) = self.sketch(sketch)?;
                let q = es.pts.get(&point).ok_or("o ponto do sketch não existe mais — reedite")?;
                Ok(fr.point(q[0], q[1]))
            }
            PointRef::CurveVertex { curve, index } => {
                if depth > 32 {
                    return Err("referência circular".into());
                }
                // polilinhas/splines: o ponto de definição; demais: início, fim ou meio da curva
                if let Some(r) = self.doc.reference(curve) {
                    if let RefKind::Polyline { points, .. } | RefKind::Spline { points, .. } | RefKind::SplineControl { points, .. } = &r.kind {
                        let p = points.get(index).ok_or("o vértice da curva não existe mais")?;
                        return self.point(p, depth + 1);
                    }
                }
                let c = self.curve(&CurveRef::Ref(curve), depth + 1)?;
                Ok(match index {
                    0 => c.start(),
                    1 => c.end(),
                    _ => c.pts.get(c.pts.len() / 2).copied().unwrap_or(c.start()),
                })
            }
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
            CurveRef::SketchCurve { sketch, curve } => {
                let (fr, es) = self.sketch(*sketch)?;
                let pts = es.curves.get(curve).ok_or("a curva do sketch não existe mais — reedite")?;
                let p3: Vec<V3> = pts.iter().map(|q| fr.point(q[0], q[1])).collect();
                if p3.len() == 2 {
                    return Curve3::line(p3[0], p3[1]);
                }
                let closed = p3.len() > 2 && geo3d::dist(p3[0], *p3.last().unwrap()) < 1e-6;
                let p3 = if closed { p3[..p3.len() - 1].to_vec() } else { p3 };
                Curve3::poly(p3, closed)
            }
            CurveRef::Segment { curve, index } => {
                let a = self.point(&PointRef::CurveVertex { curve: *curve, index: *index }, depth + 1)?;
                let b = self.point(&PointRef::CurveVertex { curve: *curve, index: *index + 1 }, depth + 1)?;
                Curve3::line(a, b)
            }
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

    /// Tamanho que cobre o modelo inteiro (para planos usados como superfície de corte).
    fn model_size(&self) -> f64 {
        let mut m = 500.;
        for (b, _) in self.bodies.values() {
            if let Some((a, c)) = b.bbox() {
                m = f64::max(m, geo3d::dist(a, c) * 2. + 100.);
            }
        }
        m
    }

    /// Superfície de entrada como forma: plano vira face plana grande; face/superfície como estão.
    fn cut_shape(&self, sf: &SurfRef, depth: u32) -> Result<Shape, String> {
        match sf {
            SurfRef::Plane(p) => {
                let fr = self.plane(p, depth)?;
                Shape::plane_face(fr.origin, fr.normal, self.model_size()).map_err(|e| e.0)
            }
            _ => self.surface_shape(sf, depth),
        }
    }

    /// Transformação de geometria em coordenadas do kernel.
    fn xform(&self, x: &GeoXform, d: u32, e: &dyn Fn(&String, &str) -> Result<f64, String>) -> Result<Xform, String> {
        Ok(match x {
            GeoXform::Translate { dir, dist } => {
                let (_, v) = self.axis_d(dir, d)?;
                Xform::Translate(geo3d::mul(geo3d::unit(v).ok_or("direção nula")?, e(dist, "distância")?))
            }
            GeoXform::Rotate { axis, angle } => {
                let (o, v) = self.axis_d(axis, d)?;
                Xform::Rotate(o, v, e(angle, "ângulo")?)
            }
            GeoXform::Mirror { plane } => {
                let f = self.plane(plane, d)?;
                Xform::Mirror(f.origin, f.normal)
            }
            GeoXform::Scale { center, factor } => Xform::Scale(self.point(center, d)?, e(factor, "fator")?),
        })
    }

    /// Concordância entre duas bordas: seções de Hermite (G1 cúbica, G2 quíntica com curvatura
    /// nula nas pontas) que saem tangentes às superfícies de apoio; G0 liga em linha reta.
    fn blend(&self, a: &CurveRef, sa: Option<&SurfRef>, b: &CurveRef, sb: Option<&SurfRef>, cont: Continuity, tension: f64, d: u32) -> Result<Shape, String> {
        use geo3d::*;
        const N: usize = 24;
        let (ca, cb) = (self.curve(a, d)?, self.curve(b, d)?);
        let pa = resample(&ca, N);
        let mut pb = resample(&cb, N);
        // mesmo sentido nas duas bordas
        if dist(pa[0], pb[0]) + dist(pa[N - 1], pb[N - 1]) > dist(pa[0], pb[N - 1]) + dist(pa[N - 1], pb[0]) {
            pb.reverse();
        }
        let sha = match sa {
            Some(s) => Some(self.cut_shape(s, d)?),
            None => None,
        };
        let shb = match sb {
            Some(s) => Some(self.cut_shape(s, d)?),
            None => None,
        };
        let tan = |v: &[V3], i: usize| unit(sub(v[(i + 1).min(N - 1)], v[i.saturating_sub(1)])).unwrap_or([1., 0., 0.]);
        // direção que sai da borda, no plano tangente da superfície, apontando para o outro lado
        let out = |sh: &Option<Shape>, v: &[V3], i: usize, toward: V3| -> Result<Option<V3>, String> {
            let Some(sh) = sh else { return Ok(None) };
            let (_, n) = sh.surface_near(v[i]).map_err(|x| x.0)?;
            let mut w = unit(cross(n, tan(v, i))).ok_or("borda tangente à normal")?;
            if dot(w, sub(toward, v[i])) < 0. {
                w = mul(w, -1.);
            }
            Ok(Some(w))
        };
        let mut sections = vec![];
        for i in 0..N {
            let chord = dist(pa[i], pb[i]);
            if chord < 1e-6 {
                return Err("as bordas se tocam — a concordância precisa de afastamento".into());
            }
            let k = chord * tension;
            let sec = if cont == Continuity::G0 {
                vec![pa[i], pb[i]]
            } else {
                let da = out(&sha, &pa, i, pb[i])?.unwrap_or_else(|| unit(sub(pb[i], pa[i])).unwrap());
                let db = out(&shb, &pb, i, pa[i])?.unwrap_or_else(|| unit(sub(pa[i], pb[i])).unwrap());
                let pts = hermite(pa[i], mul(da, k), [0.; 3], pb[i], mul(db, -k), [0.; 3], cont == Continuity::G2);
                pts.into_iter().step_by(4).chain(std::iter::once(pb[i])).collect::<Vec<_>>()
            };
            sections.push(sec);
        }
        // as seções vão de uma borda à outra; o loft liga uma seção à seguinte
        Shape::loft_surface(&sections, cont == Continuity::G0, false, None).map_err(|x| x.0)
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

    /// Avalia um elemento de geometria de referência (plano, ponto, curva ou superfície), com memória.
    fn geo(&self, id: Id, depth: u32) -> Result<Geo, String> {
        if let Some(g) = self.memo.borrow().get(&id) {
            return Ok(g.clone());
        }
        let g = self.geo_eval(id, depth)?;
        self.memo.borrow_mut().insert(id, g.clone());
        Ok(g)
    }

    fn geo_eval(&self, id: Id, depth: u32) -> Result<Geo, String> {
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
            SurfExtrude { curve, dir, length, length2 } => {
                let c = self.curve(curve, d)?;
                let (_, v) = self.axis_d(dir, d)?;
                let v = unit(v).ok_or("direção nula")?;
                let (l1, l2) = (e(length, "comprimento")?, e(length2, "comprimento 2")?);
                // limite 2: a curva recua para o outro lado antes de extrudar
                let pts: Vec<V3> = c.pts.iter().map(|p| sub(*p, mul(v, l2))).collect();
                Geo::Surface(Shape::extrude_curve(&pts, v, l1 + l2).map_err(|x| x.0)?)
            }
            SurfRevolve { curve, axis, angle } => {
                let c = self.curve(curve, d)?;
                let (o, v) = self.axis_d(axis, d)?;
                Geo::Surface(Shape::revolve_curve(&c.pts, o, v, e(angle, "ângulo")?).map_err(|x| x.0)?)
            }
            SurfSphere { center, radius } => Geo::Surface(Shape::sphere_surface(self.point(center, d)?, e(radius, "raio")?).map_err(|x| x.0)?),
            SurfCylinder { point, dir, radius, start, end } => {
                let o = self.point(point, d)?;
                let (_, v) = self.axis_d(dir, d)?;
                let v = unit(v).ok_or("direção nula")?;
                Geo::Surface(Shape::cylinder_surface(o, v, e(radius, "raio")?, -e(start, "comprimento 1")?, e(end, "comprimento 2")?).map_err(|x| x.0)?)
            }
            SurfSweep { profile, spine, mode } => {
                let sp = self.curve(spine, d)?;
                let prof = match profile {
                    SweepProfile::Curve(c) => self.curve(c, d)?.pts,
                    SweepProfile::Circle(r) => {
                        // círculo normal à guia no início dela (tubo)
                        let fr = frame_from(sp.start(), sp.end_tangent(false), None)?;
                        let r = e(r, "raio")?;
                        let mut v: Vec<V3> = (0..48).map(|i| {
                            let a = i as f64 / 48. * std::f64::consts::TAU;
                            fr.point(r * a.cos(), r * a.sin())
                        }).collect();
                        v.push(v[0]);
                        v
                    }
                };
                let m = match mode {
                    SweepMode::Corrigido => 0,
                    SweepMode::Fixo => 1,
                    SweepMode::Frenet => 2,
                };
                Geo::Surface(Shape::sweep_surface(&prof, &sp.pts, m).map_err(|x| x.0)?)
            }
            SurfLoft { sections, guide, ruled, closed } => {
                let cs: Vec<Vec<V3>> = sections.iter().map(|c| self.curve(c, d).map(|c| c.pts)).collect::<Result<_, _>>()?;
                let g = match guide {
                    Some(g) => Some(self.curve(g, d)?.pts),
                    None => None,
                };
                Geo::Surface(Shape::loft_surface(&cs, *ruled, *closed, g.as_deref()).map_err(|x| x.0)?)
            }
            SurfBlend { a, sa, b, sb, cont, tension } => Geo::Surface(self.blend(a, sa.as_ref(), b, sb.as_ref(), *cont, e(tension, "tensão")?, d)?),
            SurfJoin { items, tol } => {
                let mut acc = Shape::empty();
                for it in items {
                    let x = self.cut_shape(it, d)?;
                    acc = if acc.is_null() { x } else { acc.compound(&x) };
                }
                if acc.is_null() {
                    return Err("escolha as superfícies a juntar".into());
                }
                Geo::Surface(acc.sewn(e(tol, "tolerância")?).map_err(|x| x.0)?)
            }
            SurfHeal { surface, tol } => Geo::Surface(self.cut_shape(surface, d)?.healed(e(tol, "tolerância")?).map_err(|x| x.0)?),
            SurfSplit { surface, tool, flip } => {
                let sh = self.cut_shape(surface, d)?;
                Geo::Surface(sh.split_side(&self.cut_shape(tool, d)?, !*flip, false).map_err(|x| x.0)?)
            }
            SurfTrim { a, b, flip_a, flip_b } => {
                let (sa, sb) = (self.cut_shape(a, d)?, self.cut_shape(b, d)?);
                let pa = sa.split_side(&sb, !*flip_a, false).map_err(|x| x.0)?;
                let pb = sb.split_side(&sa, !*flip_b, false).map_err(|x| x.0)?;
                Geo::Surface(pa.compound(&pb).sewn(1e-3).map_err(|x| x.0)?)
            }
            SurfMulti { faces } => {
                let mut acc = Shape::empty();
                for p in faces {
                    let x = self.face_of_bodies(*p)?;
                    acc = if acc.is_null() { x } else { acc.compound(&x) };
                }
                if acc.is_null() {
                    return Err("escolha as faces a extrair".into());
                }
                Geo::Surface(acc)
            }
            SurfPart { surface, index } => Geo::Surface(self.cut_shape(surface, d)?.nth_face(*index as u32).map_err(|x| x.0)?),
            SurfUntrim { surface } => Geo::Surface(self.cut_shape(surface, d)?.untrimmed().map_err(|x| x.0)?),
            SurfExtrapolate { surface, near, length, curvature } => {
                let sh = self.cut_shape(surface, d)?;
                Geo::Surface(sh.extrapolated(self.point(near, d)?, e(length, "comprimento")?, *curvature).map_err(|x| x.0)?)
            }
            GeoTransform { source, xf, .. } => {
                let x = self.xform(xf, d, &|s: &String, w: &str| e(s, w))?;
                match self.geo(*source, d)? {
                    Geo::Point(p) => Geo::Point(xform_point(&x, p)),
                    Geo::Plane(f) => Geo::Plane(Frame { origin: xform_point(&x, f.origin), x: xform_dir(&x, f.x), normal: xform_dir(&x, f.normal) }),
                    Geo::Curve(c) => Geo::Curve(Curve3::poly(c.pts.iter().map(|p| xform_point(&x, *p)).collect(), c.closed)?),
                    Geo::Surface(sh) => Geo::Surface(sh.transformed(x).map_err(|x| x.0)?),
                }
            }
            CurveIntersect { a, b, which } => {
                let cs = self.cut_shape(a, d)?.section(&self.cut_shape(b, d)?).map_err(|x| x.0)?;
                let c = cs.get(*which).or(cs.last()).ok_or("os elementos não se cruzam")?.clone();
                let closed = c.len() > 2 && dist(c[0], c[c.len() - 1]) < 1e-4;
                Geo::Curve(Curve3::poly(c, closed)?)
            }
            CurveProjectSurf { curve, surface, dir } => {
                let c = self.curve(curve, d)?;
                let along = match dir {
                    Some(a) => Some(unit(self.axis_d(a, d)?.1).ok_or("direção nula")?),
                    None => None,
                };
                let pts: Vec<V3> = match (surface, along) {
                    (SurfRef::Plane(p), None) => {
                        let f = self.plane(p, d)?;
                        c.pts.iter().map(|q| sub(*q, mul(f.normal, dot(sub(*q, f.origin), f.normal)))).collect()
                    }
                    (SurfRef::Plane(p), Some(v)) => {
                        let f = self.plane(p, d)?;
                        let den = dot(v, f.normal);
                        if den.abs() < 1e-9 {
                            return Err("a direção é paralela ao plano".into());
                        }
                        c.pts.iter().map(|q| add(*q, mul(v, dot(sub(f.origin, *q), f.normal) / den))).collect()
                    }
                    (_, None) => {
                        let sh = self.surface_shape(surface, d)?;
                        c.pts.iter().map(|q| sh.surface_near(*q).map(|(p, _)| p)).collect::<Result<_, _>>().map_err(|x| x.0)?
                    }
                    (_, Some(v)) => {
                        // cada ponto segue a direção até encontrar a superfície (os que não encontram ficam de fora)
                        let sh = self.surface_shape(surface, d)?;
                        let mut out = vec![];
                        for q in &c.pts {
                            if let Some((p, _)) = sh.ray_hit(*q, v).map_err(|x| x.0)? {
                                out.push(p);
                            }
                        }
                        if out.len() < 2 {
                            return Err("a curva não atinge a superfície nessa direção".into());
                        }
                        out
                    }
                };
                Geo::Curve(Curve3::poly(pts, c.closed)?)
            }
            CurveParallel { curve, support, d1, d2, flip } => {
                let c = self.curve(curve, d)?;
                let (a, b) = (e(d1, "distância inicial")?, e(d2, "distância final")?);
                let sh = match support {
                    SurfRef::Plane(_) => None,
                    _ => Some(self.surface_shape(support, d)?),
                };
                let total = c.length().max(1e-9);
                let n = c.pts.len();
                let mut s_acc = 0.;
                let mut pts = vec![];
                for i in 0..n {
                    if i > 0 {
                        s_acc += dist(c.pts[i - 1], c.pts[i]);
                    }
                    let q = c.pts[i];
                    let t = unit(sub(c.pts[(i + 1).min(n - 1)], c.pts[i.saturating_sub(1)])).ok_or("curva degenerada")?;
                    let nrm = match (&sh, support) {
                        (Some(sh), _) => sh.surface_near(q).map_err(|x| x.0)?.1,
                        (None, SurfRef::Plane(p)) => self.plane(p, d)?.normal,
                        _ => unreachable!(),
                    };
                    let mut side = unit(cross(nrm, t)).ok_or("a curva é normal ao apoio")?;
                    if *flip {
                        side = mul(side, -1.);
                    }
                    let dd = a + (b - a) * s_acc / total;
                    let mut p = add(q, mul(side, dd));
                    if let Some(sh) = &sh {
                        p = sh.surface_near(p).map_err(|x| x.0)?.0;
                    }
                    pts.push(p);
                }
                Geo::Curve(Curve3::poly(pts, c.closed)?)
            }
            CurveBoundary { surface, which } => {
                let cs = self.cut_shape(surface, d)?.boundary().map_err(|x| x.0)?;
                let c = cs.get(*which).or(cs.last()).ok_or("sem bordas")?.clone();
                let closed = c.len() > 2 && dist(c[0], c[c.len() - 1]) < 1e-4;
                Geo::Curve(Curve3::poly(c, closed)?)
            }
            Linked { ext, .. } => {
                let x = self.doc.external.iter().find(|x| x.id == *ext).ok_or("link perdido")?;
                if x.state == LinkState::Desativado {
                    return Err("link desativado provisoriamente — reative pelo botão direito".into());
                }
                linked_geo(x.snapshot.as_ref().ok_or("link sem cópia — atualize os links")?)?
            }
            Isolated { geo } => linked_geo(geo)?,
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

    fn sketch(&self, id: Id) -> Result<&(Frame, Arc<EvalSketch>), String> {
        self.sketches.get(&id).ok_or_else(|| {
            match self.doc.feature(id) {
                Some(f) => format!("o sketch '{}' está inativo ou com erro", f.name),
                None => format!("referência perdida: sketch #{} não existe — reedite ou substitua a referência", id.0),
            }
        })
    }

    /// Polilinha 3D da geometria externa de um sketch, no estado atual do modelo.
    fn link_world(&self, l: &scc_core::sketch::SkLink, frame: &Frame) -> Result<Vec<V3>, String> {
        use scc_core::sketch::LinkSrc;
        let k = |e: KernelError| e.0;
        // corte da forma pelo plano do sketch: o contorno mais longo
        let cut = |sh: &Shape| -> Result<Vec<V3>, String> {
            let face = Shape::plane_face(frame.origin, frame.normal, self.model_size()).map_err(k)?;
            let loops = sh.section(&face).map_err(k)?;
            let len = |l: &Vec<V3>| l.windows(2).map(|w| geo3d::dist(w[0], w[1])).sum::<f64>();
            Ok(loops.into_iter().max_by(|a, b| len(a).total_cmp(&len(b))).unwrap_or_default())
        };
        // pontos onde a polilinha atravessa o plano
        let crossings = |pts: &[V3]| -> Vec<V3> {
            let sd = |p: V3| geo3d::dot(geo3d::sub(p, frame.origin), frame.normal);
            pts.windows(2)
                .filter_map(|w| {
                    let (a, b) = (sd(w[0]), sd(w[1]));
                    ((a <= 0. && b > 0.) || (a > 0. && b <= 0.)).then(|| geo3d::lerp(w[0], w[1], a / (a - b)))
                })
                .collect()
        };
        match &l.source {
            LinkSrc::Ref(id) => match self.geo(*id, 0)? {
                Geo::Point(p) => Ok(vec![p]),
                Geo::Curve(c) if l.intersect => Ok(crossings(&c.pts)),
                Geo::Curve(c) => Ok(c.pts),
                Geo::Surface(sh) => cut(&sh),
                Geo::Plane(_) => Err("plano não se projeta num sketch".into()),
            },
            LinkSrc::SketchCurve { sketch, curve } => {
                let (fr, es) = self.sketch(*sketch)?;
                let pts = es.curves.get(curve).ok_or("a curva do outro sketch não existe mais")?;
                let p3: Vec<V3> = pts.iter().map(|q| fr.point(q[0], q[1])).collect();
                Ok(if l.intersect { crossings(&p3) } else { p3 })
            }
            LinkSrc::Edge { mid } | LinkSrc::Vertex { pos: mid } => {
                let vertex = matches!(l.source, LinkSrc::Vertex { .. });
                let mut best: Option<(f64, Vec<V3>)> = None;
                for (b, _) in self.bodies.values() {
                    let Some((lo, hi)) = b.bbox() else { continue };
                    let defl = (geo3d::dist(lo, hi) * 0.0015).clamp(0.005, 1.0);
                    let Ok(m) = b.mesh(defl) else { continue };
                    if vertex {
                        for v in m.vertices.chunks_exact(3) {
                            let p = [v[0] as f64, v[1] as f64, v[2] as f64];
                            let d = geo3d::dist(p, *mid);
                            if best.as_ref().is_none_or(|x| d < x.0) {
                                best = Some((d, vec![p]));
                            }
                        }
                        continue;
                    }
                    let ne = m.edge_mids.len() / 3;
                    for e in 0..ne {
                        let q = [m.edge_mids[3 * e] as f64, m.edge_mids[3 * e + 1] as f64, m.edge_mids[3 * e + 2] as f64];
                        let d = geo3d::dist(q, *mid);
                        if best.as_ref().is_none_or(|x| d < x.0) {
                            let s0 = m.edge_ranges[e] as usize;
                            let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(m.edges.len() / 6);
                            let mut pl: Vec<V3> = vec![];
                            for sg in m.edges.chunks_exact(6).take(s1).skip(s0) {
                                if pl.is_empty() {
                                    pl.push([sg[0] as f64, sg[1] as f64, sg[2] as f64]);
                                }
                                pl.push([sg[3] as f64, sg[4] as f64, sg[5] as f64]);
                            }
                            best = Some((d, pl));
                        }
                    }
                }
                let (d, pl) = best.ok_or("nenhum corpo com arestas")?;
                // a aresta pode ter mudado um pouco; longe demais = perdida
                if d > 5. {
                    return Err(format!("{} não encontrada (a {d:.1} mm do lugar)", if vertex { "vértice" } else { "aresta" }));
                }
                Ok(if l.intersect { crossings(&pl) } else { pl })
            }
            LinkSrc::Face { point } => cut(&self.face_of_bodies(*point)?),
        }
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
            Extent::UpToNext => return Err("'até o próximo' não vale para este comando".into()),
            Extent::UpToSurface(_) => return Err("'até a superfície' não vale para este comando".into()),
            Extent::UpToLast => return Err("'até o último' não vale para este comando".into()),
        })
    }

    /// Tamanho de referência do modelo (para meios-espaços e prolongamentos).
    fn size_hint(&self) -> f64 {
        self.model_size().max(1000.)
    }

    /// Acrescenta sólidos sem fundir (membros e placas continuam peças separadas no corpo).
    fn add_separate(&mut self, f: &Feature, s: Shape) {
        let n = s.face_count() as usize;
        let next = match self.bodies.remove(&f.target) {
            Some((b, mut own)) if !b.is_null() => {
                own.extend(std::iter::repeat_n(f.id, n));
                (b.compound(&s), own)
            }
            _ => (s, vec![f.id; n]),
        };
        self.bodies.insert(f.target, next);
    }

    /// Geometria de cada membro (eixos da seção, tamanho, prolongamentos — pontas ligadas ganham
    /// sobra para o tratamento de canto cortar).
    fn members(&self, m: &StructMember) -> Result<Vec<MemberGeom>, String> {
        let dims: Vec<f64> = m.dims.iter().map(|e| ev(e, &self.vars, "cota da seção")).collect::<Result<_, _>>()?;
        let (loops, w, h, cen) = structural::section(m.kind, &dims)?;
        let ang = ev(&m.angle, &self.vars, "rotação")?.to_radians();
        let off = [ev(&m.off_x, &self.vars, "deslocamento x")?, ev(&m.off_y, &self.vars, "deslocamento y")?];
        let a = structural::anchor_point(m.anchor, w, h, cen);
        let shift = [a[0] - off[0], a[1] - off[1]];
        let (e1, e2) = (ev(&m.ext1, &self.vars, "prolongamento 1")?, ev(&m.ext2, &self.vars, "prolongamento 2")?);
        let mut out = vec![];
        for p in &m.paths {
            let c = self.curve(p, 0)?;
            let (start, end) = (c.start(), c.end());
            let z = if c.is_straight() { geo3d::unit(geo3d::sub(end, start)).ok_or("linha guia sem comprimento")? } else { c.end_tangent(false) };
            let up = if z[2].abs() > 0.99 { [1., 0., 0.] } else { [0., 0., 1.] };
            let v = geo3d::unit(geo3d::sub(up, geo3d::mul(z, geo3d::dot(up, z)))).unwrap();
            let x0 = scc_kernel::cross(v, z);
            let x = geo3d::rotate(x0, z, ang);
            let y = scc_kernel::cross(z, x);
            out.push(MemberGeom { start, end, z, x, y, w, h, shift, ext: (e1, e2), loops: loops.clone(), pts: c.pts.clone(), straight: c.is_straight() });
        }
        // pontas ligadas a outro membro: sobra para o canto (diagonal da seção)
        if m.corner != CornerMode::Nenhum {
            let diag = (w * w + h * h).sqrt() + 1.;
            for i in 0..out.len() {
                for j in 0..out.len() {
                    if i == j {
                        continue;
                    }
                    let close = |a: V3, b: V3| geo3d::dist(a, b) < 1e-4;
                    let (s, e) = (out[i].start, out[i].end);
                    if close(s, out[j].start) || close(s, out[j].end) {
                        out[i].ext.0 = out[i].ext.0.max(diag);
                    }
                    if close(e, out[j].start) || close(e, out[j].end) {
                        out[i].ext.1 = out[i].ext.1.max(diag);
                    }
                }
            }
        }
        Ok(out)
    }

    fn member_shape(&self, g: &MemberGeom) -> Result<Shape, String> {
        let k = |e: KernelError| e.0;
        let loops = structural::shifted(&g.loops, g.shift);
        if g.straight {
            let o = geo3d::sub(g.start, geo3d::mul(g.z, g.ext.0));
            let fr = Frame { origin: o, x: g.x, normal: g.z };
            let face = Shape::face(&fr, &loops).map_err(k)?;
            face.extrude(g.z, geo3d::dist(g.start, g.end) + g.ext.0 + g.ext.1, 0.).map_err(k)
        } else {
            let fr = Frame { origin: g.start, x: g.x, normal: g.z };
            let face = Shape::face(&fr, &loops).map_err(k)?;
            face.sweep(&g.pts, 0).map_err(k)
        }
    }

    /// Membro (de comandos estruturais anteriores) com a ponta mais perto de `at`.
    fn member_end(&self, at: V3, me: &Feature) -> Result<(MemberGeom, bool), String> {
        let mut best: Option<(f64, MemberGeom, bool)> = None;
        for f in self.doc.features.iter().take_while(|g| g.id != me.id) {
            let FeatureKind::StructMember(m) = &f.kind else { continue };
            for g in self.members(m)? {
                for (p, s) in [(g.start, true), (g.end, false)] {
                    let dd = geo3d::dist(p, at);
                    if best.as_ref().is_none_or(|b| dd < b.0) {
                        best = Some((dd, g.clone(), s));
                    }
                }
            }
        }
        best.map(|(_, g, s)| (g, s)).ok_or_else(|| "nenhum membro estrutural antes deste comando".into())
    }

    /// Dois membros que se encontram perto de `at`: (geometria, direção para dentro de cada um) e o ponto.
    fn member_pair(&self, at: V3, me: &Feature) -> Result<((MemberGeom, V3), (MemberGeom, V3), V3), String> {
        let mut ends: Vec<(f64, MemberGeom, V3, V3)> = vec![];
        for f in self.doc.features.iter().take_while(|g| g.id != me.id) {
            let FeatureKind::StructMember(m) = &f.kind else { continue };
            for g in self.members(m)? {
                ends.push((geo3d::dist(g.start, at), g.clone(), g.z, g.start));
                ends.push((geo3d::dist(g.end, at), g.clone(), geo3d::mul(g.z, -1.), g.end));
            }
        }
        ends.sort_by(|a, b| a.0.total_cmp(&b.0));
        let first = ends.first().cloned().ok_or("nenhum membro estrutural antes deste comando")?;
        let second = ends.iter().skip(1).find(|e| geo3d::dist(e.3, first.3) < 1e-4).cloned().ok_or("o ponto precisa ser o encontro de dois membros")?;
        Ok(((first.1, first.2), (second.1, second.2), first.3))
    }

    /// Parâmetros de chapa avaliados.
    fn sheet(&self) -> Result<SheetValues, String> {
        let s = self.doc.sheet.as_ref().ok_or("defina a chapa (espessura, raio, fator K) em Chapa › Parâmetros")?;
        Ok(SheetValues {
            t: ev(&s.thickness, &self.vars, "espessura da chapa")?,
            r: ev(&s.radius, &self.vars, "raio de dobra")?,
            k: ev(&s.k, &self.vars, "fator K")?,
            flat: s.flat,
        })
    }

    /// Aba/bainha: no estado planificado, a aresta clicada precisa existir na chapa plana — senão
    /// avisa que o planificado não é exato (aba sobre aba) em vez de inventar a geometria.
    fn sheet_tool(&mut self, f: &Feature, edge: V3, p: scc_kernel::SheetFlange) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let b = self.body(f.target).ok_or("a aba precisa de uma parede de chapa antes")?.clone();
        if p.flat {
            let (q, _) = b.surface_near(edge).map_err(k)?;
            if geo3d::dist(q, edge) > 1e-3 {
                return Ok(Some("⚠ planificado não exato: esta aba nasce de outra aba dobrada — desdobre na ordem ou planifique por partes".into()));
            }
        }
        let (tool, info) = b.sm_flange(edge, p).map_err(k)?;
        self.last_sheet = Some(info);
        if (info.thickness - p.t).abs() > 1e-3 && !p.flat {
            // a face lateral não tem a espessura da chapa: a aresta não é de parede de chapa
            let _ = info;
        }
        self.apply(f.target, tool, BoolOp::Add, f.id)?;
        Ok(None)
    }

    /// "Até o último": distância de `o` até o ponto do corpo mais distante na direção `dir`.
    fn last_dist(&self, target: Id, o: V3, dir: V3) -> Result<f64, String> {
        let b = self.body(target).ok_or("'até o último' precisa de um corpo no coletor")?;
        let (lo, hi) = b.bbox().ok_or("corpo sem dimensões")?;
        let mut t = f64::MIN;
        for c in 0..8 {
            let q = [if c & 1 == 0 { lo[0] } else { hi[0] }, if c & 2 == 0 { lo[1] } else { hi[1] }, if c & 4 == 0 { lo[2] } else { hi[2] }];
            t = t.max(dot(sub(q, o), dir));
        }
        if t <= 1e-6 {
            return Err("o corpo está todo do outro lado — inverta o sentido".into());
        }
        // a caixa envolvente pode passar um pouco do corpo: divide e fica com o trecho até a última parede
        Ok(t)
    }

    /// Superfície de término (até plano / superfície / próximo) como forma de corte e se ela deve ser
    /// estendida; `None` = término por distância. Mesmo critério para extrusão, recorte, furo e rosca.
    fn end_cutter(&self, e: &Extent, target: Id) -> Result<Option<(Shape, bool)>, String> {
        let k = |e: KernelError| e.0;
        let size = self.through(target).unwrap_or(0.).max(1e4);
        Ok(match e {
            Extent::UpTo(p) | Extent::UpToSurface(SurfRef::Plane(p)) => {
                let fr = self.plane(p, 0)?;
                Some((Shape::plane_face(fr.origin, fr.normal, size).map_err(k)?, false))
            }
            Extent::UpToSurface(sf) => Some((self.surface_shape(sf, 0)?, true)),
            Extent::UpToNext => Some((self.body(target).ok_or("'até o próximo' precisa de um corpo no coletor")?.clone(), false)),
            _ => None,
        })
    }

    /// Comprimento de ferramenta que certamente passa da superfície de corte.
    fn reach(&self, cut: &Shape, from: V3, target: Id) -> f64 {
        let mut len = self.through(target).unwrap_or(0.);
        if let Some((a, b)) = cut.bbox() {
            for c in 0..8 {
                let q = [if c & 1 == 0 { a[0] } else { b[0] }, if c & 2 == 0 { a[1] } else { b[1] }, if c & 4 == 0 { a[2] } else { b[2] }];
                len = len.max(geo3d::dist(q, from) * 1.5 + 10.);
            }
        }
        len.min(1e5)
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
                // geometria externa (converter/projetar/interseção): reposiciona antes de resolver
                let mut link_warn = vec![];
                let data = if s.data.links.is_empty() {
                    std::borrow::Cow::Borrowed(&s.data)
                } else {
                    let mut d = s.data.clone();
                    for l in &s.data.links {
                        match self.link_world(l, &frame) {
                            Ok(p3) if !p3.is_empty() => {
                                let y = frame.y();
                                let uv: Vec<[f64; 2]> = p3
                                    .iter()
                                    .map(|p| {
                                        let v = geo3d::sub(*p, frame.origin);
                                        [geo3d::dot(v, frame.x), geo3d::dot(v, y)]
                                    })
                                    .collect();
                                d.update_link(l.id, &uv);
                            }
                            Ok(_) => link_warn.push("geometria externa sem interseção com o plano".to_string()),
                            Err(e) => link_warn.push(format!("geometria externa: {e}")),
                        }
                    }
                    std::borrow::Cow::Owned(d)
                };
                let mut es = eval_sketch(&data, &self.vars)?;
                es.warnings.extend(link_warn);
                for l in &data.links {
                    match &l.shape {
                        scc_core::sketch::LinkShape::Poly { points, .. } => es.link_pts.extend(points.iter().map(|p| (*p, data.pos(*p)))),
                        scc_core::sketch::LinkShape::Circle { center, curve } => {
                            es.link_pts.push((*center, data.pos(*center)));
                            if let Some(scc_core::sketch::Curve::Circle { r, .. }) = data.curve(*curve).map(|c| &c.geom) {
                                es.link_r.push((*curve, *r));
                            }
                        }
                    }
                }
                let warn = (!es.warnings.is_empty()).then(|| es.warnings.join("; "));
                self.sketches.insert(f.id, (frame, Arc::new(es)));
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
                let n = match &e.dir {
                    // direção escolhida (linha/aresta/eixo): o recorte entra no sentido dela
                    Some(a) => {
                        let v = geo3d::unit(self.axis(a)?.1).ok_or("direção da extrusão nula")?;
                        if e.op == BoolOp::Cut { v.map(|x| -x) } else { v }
                    }
                    None => frame.normal,
                };
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
                } else if e.extent == Extent::UpToLast {
                    let len = self.last_dist(f.target, frame.origin, dir)?;
                    face.extrude(dir, len, 0.).map_err(k)?
                } else if e.extent == Extent::UpToNext && e.op == BoolOp::Add {
                    let body = self.body(f.target).ok_or("'até o próximo' precisa de um corpo no coletor")?;
                    face.extrude_to_next(dir, self.through(f.target)?, body).map_err(k)?
                } else if let Some((cut, ext)) = self.end_cutter(&e.extent, f.target)? {
                    // até a superfície/face (ou recorte até o próximo): ferramenta longa dividida pela superfície
                    let start = face.surface_near(frame.origin).map_err(k)?.0;
                    let len = self.reach(&cut, frame.origin, f.target) + loops_size(&sk.loops);
                    face.extrude(dir, len, 0.).map_err(k)?.clip_until(&cut, start, ext).map_err(k)?
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
                // dois ângulos: o segundo vai para o outro lado do plano do sketch
                let (ang, start) = match &r.angle2 {
                    Some(a2) => {
                        let a2 = ev(a2, &self.vars, "ângulo 2")?;
                        if ang + a2 > 360. + 1e-9 {
                            return Err("os dois ângulos somam mais de 360°".into());
                        }
                        (ang + a2, -a2)
                    }
                    None if r.symmetric => (ang, -ang / 2.),
                    None => (ang, 0.),
                };
                let tool = face.revolve(origin, dir, ang, start).map_err(k)?;
                self.apply(f.target, tool, r.op, f.id)?;
                Ok(None)
            }
            FeatureKind::Hole(h) => {
                let d = ev(&h.diameter, &self.vars, "diâmetro")?;
                // direção escolhida (linha, aresta, eixo) ou automática
                let custom = match &h.dir {
                    Some(a) => Some(geo3d::unit(self.axis_d(a, 0)?.1).ok_or("direção do furo nula")?),
                    None => None,
                };
                // cada furo: boca e direção entrando no material
                let mut seats: Vec<(V3, V3)> = vec![];
                if h.points.is_empty() {
                    let (frame, sk) = self.sketch(h.sketch)?;
                    if sk.points.is_empty() {
                        return Err("o sketch do furo precisa de pelo menos um ponto (centro) — ou escolha pontos 3D".into());
                    }
                    let n = frame.normal.map(|x| -x);
                    for p in &sk.points {
                        seats.push((frame.point(p[0], p[1]), custom.unwrap_or(n)));
                    }
                } else {
                    let body = self.body(f.target).cloned();
                    for p in &h.points {
                        let o = self.point(p, 0)?;
                        let dir = match custom {
                            Some(v) => v,
                            None => {
                                // normal à superfície do corpo no ponto, para dentro do material
                                let b = body.as_ref().ok_or("furo por pontos 3D precisa de um corpo para achar a normal")?;
                                let (_, n) = b.surface_near(o).map_err(k)?;
                                n.map(|x| -x)
                            }
                        };
                        seats.push((o, dir));
                    }
                }
                if h.reverse {
                    for s in &mut seats {
                        s.1 = s.1.map(|x| -x);
                    }
                }
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
                    HoleKind::Contrafurado { diameter, depth, angle } => HoleShape::Counterdrill {
                        diameter: ev(diameter, &self.vars, "diâmetro do contrafuro")?,
                        depth: ev(depth, &self.vars, "profundidade do contrafuro")?,
                        angle: ev(angle, &self.vars, "ângulo do contrafuro")?,
                    },
                };
                let tip = match &h.tip {
                    Some(t) => ev(t, &self.vars, "ângulo da ponta")?,
                    None => 0.,
                };
                let taper = ev(&h.taper, &self.vars, "conicidade")?;
                let cut = self.end_cutter(&h.depth, f.target)?;
                let mut tool = Shape::empty();
                let mut depths = vec![];
                for (o, dir) in &seats {
                    let depth = match (&cut, &h.depth) {
                        (Some((c, _)), _) => self.reach(c, *o, f.target),
                        (None, Extent::UpToLast) => self.last_dist(f.target, *o, *dir)?,
                        (None, x) => self.extent(x, f.target)?.0,
                    };
                    depths.push(depth);
                    HOLE_LOG.with(|l| {
                        if let Some(v) = l.borrow_mut().as_mut() {
                            v.push(HoleRow { feature: f.id, pos: *o, dir: *dir, diameter: d, depth, through: matches!(h.depth, Extent::ThroughAll | Extent::UpToLast), thread: h.thread.clone().unwrap_or_default() });
                        }
                    });
                    let mut t = Shape::hole_tool_ext(*o, *dir, d, depth, shape, tip, taper).map_err(k)?;
                    if let Some((c, ext)) = &cut {
                        t = t.clip_until(c, *o, *ext).map_err(k)?;
                    }
                    tool = if tool.is_null() { t } else { tool.compound(&t) };
                }
                self.apply(f.target, tool, BoolOp::Cut, f.id)?;
                let Some(t) = &h.thread else { return Ok(None) };
                if !h.thread_modeled {
                    return Ok(Some(format!("rosca cosmética {t}{}", if h.left_hand { " (esquerda)" } else { "" })));
                }
                // rosca modelada: sulco helicoidal subtraído em cada furo, até a espessura do material
                let spec = hardware::thread(t).ok_or_else(|| format!("rosca '{t}' não encontrada na tabela de normas"))?;
                let (lo, hi) = self.body(f.target).and_then(|b| b.bbox()).ok_or("corpo sem dimensões")?;
                for ((o, dir), depth) in seats.iter().zip(depths) {
                    let (o, dir) = (*o, *dir);
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
                    let s0 = if fwd >= back { shape.head_depth(d) } else { 0. };
                    let len = depth.min(ext) - s0;
                    if len < spec.pitch {
                        continue;
                    }
                    let o = [0, 1, 2].map(|i| o[i] + tdir[i] * s0);
                    let mut tool = Shape::thread_tool(o, tdir, spec.major, spec.pitch, len, spec.angle).map_err(k)?;
                    if h.left_hand {
                        // espelhar por um plano que contém o eixo troca o sentido da hélice
                        tool = tool.transformed(Xform::Mirror(o, geo3d::any_perp(tdir))).map_err(k)?;
                    }
                    if let Some((c, ext)) = &cut {
                        // a rosca para na mesma superfície do furo
                        tool = tool.clip_until(c, o, *ext).map_err(k)?;
                    }
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
            FeatureKind::Shell(sh) if sh.faces.is_empty() => Ok(Some("escolha na viewport as faces a remover — até lá a casca não é aplicada".into())),
            FeatureKind::Draft(d) if d.faces.is_empty() => Ok(Some("escolha na viewport as faces que recebem a saída — até lá nada é aplicado".into())),
            FeatureKind::Thickness(t) if t.faces.is_empty() => Ok(Some("escolha na viewport as faces a engrossar — até lá nada é aplicado".into())),
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
                    PatternKind::Linear { dir, count, spacing, dir2, count2, spacing2, total } => {
                        let n1 = ev(count, &self.vars, "quantidade")?.round().max(1.) as usize;
                        let mut d1 = ev(spacing, &self.vars, "espaçamento")?;
                        if *total && n1 > 1 {
                            // instâncias e comprimento total
                            d1 /= (n1 - 1) as f64;
                        }
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
            FeatureKind::SmWall(w) => {
                let sp = self.sheet()?;
                let (frame, sk) = self.sketch(w.sketch)?;
                if sk.loops.is_empty() {
                    return Err("a parede base precisa de um perfil fechado".into());
                }
                let face = Shape::face(frame, &sk.loops).map_err(k)?;
                let dir = if w.reverse { frame.normal.map(|x| -x) } else { frame.normal };
                let tool = face.extrude(dir, sp.t, 0.).map_err(k)?;
                self.apply(f.target, tool, BoolOp::Add, f.id)?;
                Ok(None)
            }
            FeatureKind::SmFlange(fl) => {
                let sp = self.sheet()?;
                let r = match &fl.radius {
                    Some(e) => ev(e, &self.vars, "raio")?,
                    None => sp.r,
                };
                let p = scc_kernel::SheetFlange {
                    t: sp.t,
                    r,
                    angle: ev(&fl.angle, &self.vars, "ângulo")?,
                    length: ev(&fl.length, &self.vars, "comprimento")?,
                    off1: ev(&fl.off1, &self.vars, "recuo 1")?,
                    off2: ev(&fl.off2, &self.vars, "recuo 2")?,
                    flip: fl.flip,
                    flat: sp.flat,
                    k: sp.k,
                };
                let relief = match &fl.relief {
                    Some(w) => Some(ev(w, &self.vars, "largura do alívio")?),
                    None => None,
                };
                let out = self.sheet_tool(f, fl.edge, p)?;
                if let (Some(w), Some(info)) = (relief, self.last_sheet) {
                    // alívio de canto: corte retangular atrás da dobra em cada ponta recuada
                    let depth = p.r + p.t;
                    let e = geo3d::unit(geo3d::sub(info.ends[1], info.ends[0])).ok_or("aresta nula")?;
                    let mut cut = Shape::empty();
                    for (end, x, off) in [(info.ends[0], geo3d::mul(e, -1.), p.off1), (info.ends[1], e, p.off2)] {
                        if off <= 1e-9 {
                            continue;
                        }
                        let z = geo3d::mul(info.normal, -1.);
                        let y = scc_kernel::cross(z, x);
                        let o = if geo3d::dot(y, info.out) > 0. { geo3d::sub(end, geo3d::mul(info.out, depth)) } else { end };
                        let bx = Shape::box_tool(o, x, z, [w.min(off), depth, p.t]).map_err(k)?;
                        cut = if cut.is_null() { bx } else { cut.compound(&bx) };
                    }
                    if !cut.is_null() {
                        self.apply(f.target, cut, BoolOp::Cut, f.id)?;
                    }
                }
                Ok(out)
            }
            FeatureKind::SmJoggle(j) => {
                let sp = self.sheet()?;
                let depth = ev(&j.depth, &self.vars, "desvio")?;
                let run = ev(&j.run, &self.vars, "rampa")?;
                let b = self.body(f.target).ok_or("o joggle precisa de uma parede de chapa")?.clone();
                // normal da face no meio da linha (as pontas podem cair numa aresta da chapa)
                let (_, n) = b.surface_near(geo3d::lerp(j.a, j.b, 0.5)).map_err(k)?;
                let e = geo3d::unit(geo3d::sub(j.b, j.a)).ok_or("os dois pontos da linha coincidem")?;
                let mut side = geo3d::unit(scc_kernel::cross(n, e)).ok_or("linha normal à chapa")?;
                if j.flip {
                    side = geo3d::mul(side, -1.);
                }
                let (lo, hi) = b.bbox().ok_or("corpo sem dimensões")?;
                let big = geo3d::dist(lo, hi) * 2. + 10.;
                // parte da chapa além da linha (lado `side`)
                let start = geo3d::sub(j.a, geo3d::mul(e, big));
                let z = geo3d::mul(n, -1.);
                let y0 = scc_kernel::cross(z, e);
                let corner = |from: f64| {
                    // caixa ao longo de e (2·big), de `from` a `big` no lado, atravessando ±big na normal
                    let o = geo3d::add(geo3d::add(start, geo3d::mul(side, from)), geo3d::mul(n, big));
                    if geo3d::dot(y0, side) > 0. {
                        Shape::box_tool(o, e, z, [2. * big, big, 2. * big])
                    } else {
                        Shape::box_tool(geo3d::add(o, geo3d::mul(side, big)), e, z, [2. * big, big, 2. * big])
                    }
                };
                let beyond = corner(0.).map_err(k)?;
                let far = corner(run).map_err(k)?;
                let part = b.boolean(&beyond, scc_kernel::Bool::Common).map_err(k)?;
                let far_part = part.boolean(&far, scc_kernel::Bool::Common).map_err(k)?;
                let moved = far_part.transformed(Xform::Translate(geo3d::mul(n, -depth))).map_err(k)?;
                // rampa: paralelogramo (seção) na linha, só na largura da chapa ao longo da linha
                let (plo, phi) = part.bbox().ok_or("a linha do joggle não corta a chapa")?;
                let (mut e0, mut e1) = (f64::MAX, f64::MIN);
                for c in 0..8 {
                    let q = [if c & 1 == 0 { plo[0] } else { phi[0] }, if c & 2 == 0 { plo[1] } else { phi[1] }, if c & 4 == 0 { plo[2] } else { phi[2] }];
                    let t = geo3d::dot(geo3d::sub(q, j.a), e);
                    e0 = e0.min(t);
                    e1 = e1.max(t);
                }
                let top = geo3d::add(j.a, geo3d::mul(e, e0));
                let pts = [
                    top,
                    geo3d::sub(top, geo3d::mul(n, sp.t)),
                    geo3d::add(geo3d::sub(top, geo3d::mul(n, sp.t + depth)), geo3d::mul(side, run)),
                    geo3d::add(geo3d::sub(top, geo3d::mul(n, depth)), geo3d::mul(side, run)),
                ];
                let ramp = Shape::poly_prism(&pts, geo3d::mul(e, e1 - e0)).map_err(k)?;
                let n_own = b.face_count() as usize;
                let rest = b.boolean(&beyond, scc_kernel::Bool::Cut).map_err(k)?;
                let r = rest.boolean(&moved, scc_kernel::Bool::Fuse).map_err(k)?.boolean(&ramp, scc_kernel::Bool::Fuse).map_err(k)?;
                let nf = r.face_count() as usize;
                let _ = n_own;
                self.bodies.insert(f.target, (r, vec![f.id; nf]));
                Ok(None)
            }
            FeatureKind::SmBend(bd) => {
                let sp = self.sheet()?;
                if sp.flat {
                    // planificada: a dobra não se aplica (a chapa continua plana)
                    return Ok(Some("planificada: dobra desfeita".into()));
                }
                let ang = ev(&bd.angle, &self.vars, "ângulo")?;
                let r = match &bd.radius {
                    Some(x) => ev(x, &self.vars, "raio")?,
                    None => sp.r,
                };
                if !(0.1..=179.9).contains(&ang) {
                    return Err("ângulo da dobra entre 0,1° e 179,9°".into());
                }
                let b = self.body(f.target).ok_or("a dobra precisa de uma chapa")?.clone();
                let mid = geo3d::lerp(bd.a, bd.b, 0.5);
                // normal para fora da face clicada: o material fica do lado oposto
                let (_, n) = b.surface_near(mid).map_err(k)?;
                let e = geo3d::unit(geo3d::sub(bd.b, bd.a)).ok_or("os dois pontos da linha coincidem")?;
                let mut side = geo3d::unit(scc_kernel::cross(n, e)).ok_or("linha normal à chapa")?;
                if bd.flip {
                    side = geo3d::mul(side, -1.);
                }
                let (lo, hi) = b.bbox().ok_or("corpo sem dimensões")?;
                let big = geo3d::dist(lo, hi) * 2. + 10.;
                // comprimento desenvolvido da dobra, centrado na linha
                let ba = ang.to_radians() * (r + sp.k * sp.t);
                let half_space = |d: f64| {
                    let p = geo3d::add(bd.a, geo3d::mul(side, d));
                    let y = scc_kernel::cross(side, e);
                    let o = geo3d::sub(geo3d::sub(p, geo3d::mul(e, big)), geo3d::mul(y, big));
                    Shape::box_tool(o, e, side, [2. * big, 2. * big, big])
                };
                let fixed = b.boolean(&half_space(-ba / 2.).map_err(k)?, scc_kernel::Bool::Cut).map_err(k)?;
                let moving = b.boolean(&half_space(ba / 2.).map_err(k)?, scc_kernel::Bool::Common).map_err(k)?;
                let p0 = geo3d::sub(bd.a, geo3d::mul(side, ba / 2.));
                let section = b.boolean(&Shape::plane_face(p0, side, big).map_err(k)?, scc_kernel::Bool::Common).map_err(k)?;
                if section.is_null() || moving.is_null() {
                    return Err("a linha da dobra não atravessa a chapa".into());
                }
                // eixo: raio interno acima da face clicada (para cima) ou abaixo da outra face
                let (axis_p, axis_d) = if bd.down {
                    (geo3d::sub(p0, geo3d::mul(n, sp.t + r)), scc_kernel::cross(n, side))
                } else {
                    (geo3d::add(p0, geo3d::mul(n, r)), scc_kernel::cross(side, n))
                };
                let bend = section.revolve(axis_p, axis_d, ang, 0.).map_err(k)?;
                let moved = moving
                    .transformed(Xform::Translate(geo3d::mul(side, -ba)))
                    .map_err(k)?
                    .transformed(Xform::Rotate(axis_p, axis_d, ang))
                    .map_err(k)?;
                let out = fixed.boolean(&bend, scc_kernel::Bool::Fuse).map_err(k)?.boolean(&moved, scc_kernel::Bool::Fuse).map_err(k)?;
                let nf = out.face_count() as usize;
                self.bodies.insert(f.target, (out, vec![f.id; nf]));
                Ok(None)
            }
            FeatureKind::SmLouver(lv) => {
                let sp = self.sheet()?;
                if sp.flat {
                    return Ok(Some("planificada: veneziana mostrada só pelo corte".into()));
                }
                let w = ev(&lv.width, &self.vars, "largura")?;
                let h = ev(&lv.height, &self.vars, "altura")?;
                if h <= sp.t {
                    return Err("a altura da veneziana precisa ser maior que a espessura".into());
                }
                let b = self.body(f.target).ok_or("a veneziana precisa de uma chapa")?.clone();
                let mid = geo3d::lerp(lv.a, lv.b, 0.5);
                let (_, n0) = b.surface_near(mid).map_err(k)?;
                let e = geo3d::sub(lv.b, lv.a);
                let len = geo3d::norm(e);
                let e = geo3d::unit(e).ok_or("os dois pontos da linha coincidem")?;
                let mut side = geo3d::unit(scc_kernel::cross(n0, e)).ok_or("linha normal à chapa")?;
                if lv.flip {
                    side = geo3d::mul(side, -1.);
                }
                // levantar para fora da face clicada (ou para o outro lado: a origem vai para a outra face)
                let (o, n) = if lv.down { (geo3d::sub(lv.a, geo3d::mul(n0, sp.t)), geo3d::mul(n0, -1.)) } else { (lv.a, n0) };
                let at = |s: f64, z: f64| geo3d::add(geo3d::add(o, geo3d::mul(side, s)), geo3d::mul(n, z));
                // retira a chapa sob a rampa e põe a rampa (paralelogramo: o volume se conserva)
                let cut = Shape::poly_prism(&[at(0., 0.01), at(w, 0.01), at(w, -sp.t - 0.01), at(0., -sp.t - 0.01)], geo3d::mul(e, len)).map_err(k)?;
                let hood = Shape::poly_prism(&[at(0., h), at(w, 0.), at(w, -sp.t), at(0., h - sp.t)], geo3d::mul(e, len)).map_err(k)?;
                let out = b.boolean(&cut, scc_kernel::Bool::Cut).map_err(k)?.boolean(&hood, scc_kernel::Bool::Fuse).map_err(k)?;
                let nf = out.face_count() as usize;
                self.bodies.insert(f.target, (out, vec![f.id; nf]));
                Ok(None)
            }
            FeatureKind::SmStamp(st) => {
                let sp = self.sheet()?;
                let (frame, sk) = self.sketch(st.sketch)?;
                if sk.loops.is_empty() {
                    return Err("o estampo precisa de um perfil fechado".into());
                }
                let frame = *frame;
                let face = Shape::face(&frame, &sk.loops).map_err(k)?;
                let p = if st.flip { frame.normal } else { frame.normal.map(|x| -x) };
                let depth = ev(&st.depth, &self.vars, "profundidade")?;
                // copo: perfil deslocado da espessura até o fundo, menos a cavidade
                let outer = face.offset_face(sp.t).map_err(k)?.extrude(p, depth + sp.t, 0.).map_err(k)?;
                let inner = face.extrude(p, depth, 0.).map_err(k)?;
                self.apply(f.target, outer, BoolOp::Add, f.id)?;
                self.apply(f.target, inner, BoolOp::Cut, f.id)?;
                Ok(None)
            }
            FeatureKind::SmFlangedHole(h) => {
                let sp = self.sheet()?;
                let (frame, sk) = self.sketch(h.sketch)?;
                if sk.points.is_empty() {
                    return Err("o sketch precisa de pontos (centros dos furos)".into());
                }
                let (frame, pts) = (*frame, sk.points.clone());
                let p = if h.flip { frame.normal } else { frame.normal.map(|x| -x) };
                let dia = ev(&h.diameter, &self.vars, "diâmetro")?;
                let ht = ev(&h.height, &self.vars, "altura do colar")?;
                let (mut ring, mut cut) = (Shape::empty(), Shape::empty());
                for q in &pts {
                    let o = frame.point(q[0], q[1]);
                    let o2 = geo3d::add(o, geo3d::mul(p, sp.t));
                    let outer = Shape::hole_tool(o2, p, dia + 2. * sp.t, ht, HoleShape::Simple).map_err(k)?;
                    let inner = Shape::hole_tool(o2, p, dia, ht + 1., HoleShape::Simple).map_err(k)?;
                    let rg = outer.boolean(&inner, scc_kernel::Bool::Cut).map_err(k)?;
                    let c = Shape::hole_tool(o, p, dia, sp.t + ht + 1., HoleShape::Simple).map_err(k)?;
                    ring = if ring.is_null() { rg } else { ring.compound(&rg) };
                    cut = if cut.is_null() { c } else { cut.compound(&c) };
                }
                self.apply(f.target, ring, BoolOp::Add, f.id)?;
                self.apply(f.target, cut, BoolOp::Cut, f.id)?;
                Ok(None)
            }
            FeatureKind::SmHem(h) => {
                let sp = self.sheet()?;
                let gap = ev(&h.gap, &self.vars, "folga")?;
                let p = scc_kernel::SheetFlange {
                    t: sp.t,
                    r: (gap / 2.).max(0.01),
                    angle: 180.,
                    length: ev(&h.length, &self.vars, "comprimento")?,
                    off1: 0.,
                    off2: 0.,
                    flip: h.flip,
                    flat: sp.flat,
                    k: sp.k,
                };
                self.sheet_tool(f, h.edge, p)
            }
            FeatureKind::StructMember(m) => {
                let geo = self.members(m)?;
                let mut shapes: Vec<Shape> = vec![];
                for g in &geo {
                    shapes.push(self.member_shape(g)?);
                }
                // cantos: pares de membros que dividem uma ponta
                let gap = ev(&m.gap, &self.vars, "folga")?;
                let mid = |g: &MemberGeom| geo3d::lerp(g.start, g.end, 0.5);
                for i in 0..geo.len() {
                    for j in i + 1..geo.len() {
                        let Some((p, di, dj)) = shared_end(&geo[i], &geo[j]) else { continue };
                        match m.corner {
                            CornerMode::Nenhum => {}
                            CornerMode::MeiaEsquadria => {
                                let n = geo3d::unit(geo3d::sub(di, dj)).ok_or("membros colineares não fazem meia-esquadria")?;
                                shapes[i] = shapes[i].boolean(&half_space(geo3d::add(p, geo3d::mul(n, gap / 2.)), n, self.size_hint())?, scc_kernel::Bool::Common).map_err(k)?;
                                let nn = geo3d::mul(n, -1.);
                                shapes[j] = shapes[j].boolean(&half_space(geo3d::add(p, geo3d::mul(nn, gap / 2.)), nn, self.size_hint())?, scc_kernel::Bool::Common).map_err(k)?;
                            }
                            CornerMode::Aparar | CornerMode::Encaixe => {
                                // o posterior para no anterior (plano da face ou contorno, com folga)
                                let primary = if gap > 1e-9 { shapes[i].offset_surface(gap).map_err(k)? } else { shapes[i].clone() };
                                let extend = m.corner == CornerMode::Aparar;
                                shapes[j] = shapes[j].clip_until(&primary, mid(&geo[j]), extend).map_err(k)?;
                                // o anterior vai só até a face de fora do posterior (topo)
                                let hj = geo[j].w.max(geo[j].h) / 2.;
                                shapes[i] = shapes[i].boolean(&half_space(geo3d::sub(p, geo3d::mul(di, hj)), di, self.size_hint())?, scc_kernel::Bool::Common).map_err(k)?;
                            }
                        }
                    }
                }
                let mut all = Shape::empty();
                for sh in shapes {
                    all = if all.is_null() { sh } else { all.compound(&sh) };
                }
                self.tools.insert(f.id, (all.clone(), BoolOp::Add));
                self.add_separate(f, all);
                Ok(None)
            }
            FeatureKind::StructPlate(pl) => {
                let (g, end_at_start) = self.member_end(pl.at, f)?;
                let t = ev(&pl.thickness, &self.vars, "espessura")?;
                let mg = ev(&pl.margin, &self.vars, "sobra")?;
                let (e, out) = if end_at_start { (geo3d::sub(g.start, geo3d::mul(g.z, g.ext.0)), geo3d::mul(g.z, -1.)) } else { (geo3d::add(g.end, geo3d::mul(g.z, g.ext.1)), g.z) };
                let (w, h) = (g.w + 2. * mg, g.h + 2. * mg);
                // centro da seção (a linha guia pode estar num canto, pela ancoragem)
                let c = geo3d::add(geo3d::add(e, geo3d::mul(g.x, -g.shift[0])), geo3d::mul(g.y, -g.shift[1]));
                let y = scc_kernel::cross(out, g.x);
                let corner = geo3d::sub(geo3d::sub(c, geo3d::mul(g.x, w / 2.)), geo3d::mul(y, h / 2.));
                let plate = Shape::box_tool(corner, g.x, out, [w, h, t]).map_err(k)?;
                self.add_separate(f, plate);
                Ok(None)
            }
            FeatureKind::StructGusset(gs) => {
                let (a, b, p) = self.member_pair(gs.at, f)?;
                let leg = ev(&gs.leg, &self.vars, "aba")?;
                let t = ev(&gs.thickness, &self.vars, "espessura")?;
                let c = ev(&gs.chamfer, &self.vars, "chanfro")?;
                let (da, db) = (a.1, b.1);
                let n = geo3d::unit(scc_kernel::cross(da, db)).ok_or("membros colineares não levam reforço")?;
                // canto do reforço junto às faces dos membros, no plano das linhas guia
                let ha = a.0.w.max(a.0.h) / 2.;
                let hb = b.0.w.max(b.0.h) / 2.;
                let q = geo3d::add(geo3d::add(p, geo3d::mul(da, hb)), geo3d::mul(db, ha));
                let q = geo3d::sub(q, geo3d::mul(n, t / 2.));
                let mut pts = vec![];
                if c > 1e-9 {
                    pts.push(geo3d::add(q, geo3d::mul(da, c)));
                } else {
                    pts.push(q);
                }
                pts.push(geo3d::add(q, geo3d::mul(da, leg)));
                pts.push(geo3d::add(q, geo3d::mul(db, leg)));
                if c > 1e-9 {
                    pts.push(geo3d::add(q, geo3d::mul(db, c)));
                }
                let plate = Shape::poly_prism(&pts, geo3d::mul(n, t)).map_err(k)?;
                self.add_separate(f, plate);
                Ok(None)
            }
            FeatureKind::MoveBody(m) => {
                let (b, owners) = self.body_owned(f.target).ok_or("não há corpo para mover")?;
                let rigid = match &m.xf {
                    BodyXf::Translate { dir, dist } => {
                        let v = geo3d::unit(self.axis(dir)?.1).ok_or("direção nula")?;
                        Some(Xform::Translate(geo3d::mul(v, ev(dist, &self.vars, "distância")?)))
                    }
                    BodyXf::Rotate { axis, angle } => {
                        let (o, v) = self.axis(axis)?;
                        Some(Xform::Rotate(o, v, ev(angle, &self.vars, "ângulo")?))
                    }
                    BodyXf::Symmetry { plane } => {
                        let fr = self.plane(plane, 0)?;
                        Some(Xform::Mirror(fr.origin, fr.normal))
                    }
                    BodyXf::Affinity { .. } => None,
                };
                match (rigid, &m.xf) {
                    (Some(x), _) => {
                        let (s, out) = b.transformed_tagged(x, &tags_for(owners)).map_err(k)?;
                        let own = remap(owners, &out, f.id);
                        self.bodies.insert(f.target, (s, own));
                    }
                    (None, BodyXf::Affinity { plane, fx, fy, fz }) => {
                        let fr = self.plane(plane, 0)?;
                        let kf = [ev(fx, &self.vars, "fator X")?, ev(fy, &self.vars, "fator Y")?, ev(fz, &self.vars, "fator Z")?];
                        let s = b.affinity(fr, kf).map_err(k)?;
                        let n = s.face_count() as usize;
                        self.bodies.insert(f.target, (s, vec![f.id; n]));
                    }
                    _ => unreachable!(),
                }
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
                let full = cyl.h1 - cyl.h0;
                let len = match &t.end {
                    None => ev(&t.length, &self.vars, "comprimento")?.min(full),
                    Some(Extent::Blind(x)) => ev(x, &self.vars, "comprimento")?.min(full),
                    Some(Extent::UpToNext) => return Err("a rosca termina por comprimento, na face inteira ou até plano/superfície".into()),
                    // face inteira, com folga de 1/4 de passo (encostar na face vizinha faz a booleana falhar)
                    Some(_) => full - spec.pitch * 0.25,
                };
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
                let cut = match &t.end {
                    Some(e) => self.end_cutter(e, f.target)?,
                    None => None,
                };
                let tool = match cut {
                    Some((c, ext)) => tool.clip_until(&c, o, ext).map_err(k)?,
                    None => tool,
                };
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
            FeatureKind::SplitBody(sp) => {
                let b = self.body(f.target).ok_or("não há corpo para dividir")?.clone();
                let tool = self.cut_shape(&sp.surface, 0)?;
                let r = b.split_side(&tool, !sp.flip, matches!(sp.surface, SurfRef::Face { .. })).map_err(k)?;
                let n = r.face_count() as usize;
                self.bodies.insert(f.target, (r, vec![f.id; n]));
                Ok(None)
            }
            FeatureKind::ThickSurface(t) => {
                let sh = self.cut_shape(&t.surface, 0)?;
                let tool = sh.thicken(ev(&t.t1, &self.vars, "espessura 1")?, ev(&t.t2, &self.vars, "espessura 2")?).map_err(k)?;
                self.apply(f.target, tool, t.op, f.id)?;
                Ok(None)
            }
            FeatureKind::CloseSurface(c) => {
                let sh = self.cut_shape(&c.surface, 0)?;
                let tool = sh.closed_solid(ev(&c.tol, &self.vars, "tolerância")?).map_err(k)?;
                self.apply(f.target, tool, c.op, f.id)?;
                Ok(None)
            }
            FeatureKind::SewSurface(sw) => {
                let b = self.body(f.target).ok_or("não há corpo para costurar")?.clone();
                let sh = self.cut_shape(&sw.surface, 0)?;
                if sw.remove {
                    let r = b.split_side(&sh, !sw.flip, false).map_err(k)?;
                    let n = r.face_count() as usize;
                    self.bodies.insert(f.target, (r, vec![f.id; n]));
                } else {
                    // volumes fechados entre a superfície e o corpo
                    let cells = b.compound(&sh).volume_cells(&[]).map_err(k)?;
                    self.apply(f.target, cells, BoolOp::Add, f.id)?;
                }
                Ok(None)
            }
            FeatureKind::BoundaryFill(bf) => {
                let mut acc = Shape::empty();
                for t in &bf.tools {
                    let x = self.cut_shape(t, 0)?;
                    acc = if acc.is_null() { x } else { acc.compound(&x) };
                }
                if acc.is_null() {
                    return Err("escolha superfícies, faces ou planos que fechem volumes".into());
                }
                let cells = acc.volume_cells(&bf.exclude).map_err(k)?;
                self.apply(f.target, cells, bf.op, f.id)?;
                Ok(None)
            }
            FeatureKind::MeshImport(m) => {
                let tris = mesh_io::read(&m.path)?;
                let sh = Shape::from_mesh(&tris, m.solid).map_err(k)?;
                let n = tris.len() / 9;
                if sh.is_solid() {
                    self.apply(f.target, sh, BoolOp::Add, f.id)?;
                    Ok(Some(format!("{n} triângulos → sólido")))
                } else {
                    let r = match self.body(f.target) {
                        Some(b) => b.compound(&sh),
                        None => sh,
                    };
                    let nf = r.face_count() as usize;
                    self.bodies.insert(f.target, (r, vec![f.id; nf]));
                    Ok(Some(format!("{n} triângulos → superfície aberta (a malha não fecha um volume)")))
                }
            }
            // arestas escolhidas, mas nenhuma ainda: não aplica nada (todas = só pela opção "Todas")
            FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) if matches!(&o.edges, EdgeSel::Edges(m) if m.is_empty()) && matches!(o.mode, EdgeMode::Constant | EdgeMode::Variable(_) | EdgeMode::TwoDist(_) | EdgeMode::DistAngle(_)) => {
                Ok(Some("clique nas arestas na viewport (ou escolha 'Todas') — até lá nada é aplicado".into()))
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
                    _ => None,
                })
            }
        }
    }
}

/// Recalcula o documento inteiro. Roda fora da thread da interface (RNF-001).
pub fn recompute(doc: &Document) -> RecomputeResult {
    recompute_cached(doc, &mut Cache::default())
}

/// Recalcula reaproveitando o cache: os comandos antes da primeira mudança não são refeitos.
pub fn recompute_cached(doc: &Document, cache: &mut Cache) -> RecomputeResult {
    let t0 = Instant::now();
    let (vars, param_error) = match doc.eval_params() {
        Ok(v) => (v, None),
        Err(e) => (HashMap::new(), Some(e.to_string())),
    };
    let mut cx = Ctx {
        doc,
        vars,
        bodies: HashMap::new(),
        sketches: HashMap::new(),
        status: HashMap::new(),
        tools: HashMap::new(),
        memo: RefCell::new(HashMap::new()),
        body_sig: HashMap::new(),
        last_sheet: None,
    };

    // 1º: sketches livres (pastas de geometria, sem linha do tempo); 2º: comandos dos coletores em ordem
    let order: Vec<usize> = (0..doc.features.len())
        .filter(|i| doc.is_free(&doc.features[*i]))
        .chain((0..doc.features.len()).filter(|i| !doc.is_free(&doc.features[*i])))
        .collect();
    // assinatura global: só o que vale para todos (links de outras peças, chapa, coletores).
    // Parâmetros e referências entram na assinatura de quem os usa — mudar um parâmetro refaz
    // só os comandos que citam esse parâmetro (e quem depende deles).
    let sheet_vars = serde_json::to_string(&doc.sheet).unwrap_or_default();
    let global = hash_of(
        &[
            &serde_json::to_string(&doc.external).unwrap_or_default(),
            &sheet_vars,
            &vars_sig(&sheet_vars, &cx.vars),
            &serde_json::to_string(&doc.collectors.iter().map(|c| c.id).collect::<Vec<_>>()).unwrap_or_default(),
        ],
        0,
    );
    // assinatura de todos os corpos (para quem lê faces de qualquer corpo)
    let all_bodies = |bs: &HashMap<Id, u64>| {
        let mut v: Vec<(u64, u64)> = bs.iter().map(|(k, v)| (k.0, *v)).collect();
        v.sort();
        format!("{v:?}")
    };
    let mut feat_out: HashMap<Id, u64> = HashMap::new();
    let mut seen: std::collections::HashSet<Id> = Default::default();
    cache.reused = 0;
    for i in order {
        let f = &doc.features[i];
        seen.insert(f.id);
        if !doc.is_active(i) {
            cx.status.insert(f.id, Status::Inactive);
            continue;
        }
        // entradas: o próprio comando, o corpo de destino, os comandos/corpos/referências citados
        let js = feature_sig(f);
        let mut ids = vec![];
        f.kind.clone().walk(&mut |x| ids.push(*x), &mut |_| {});
        let mut parts: Vec<String> = vec![js.clone(), vars_sig(&js, &cx.vars), format!("t{}", cx.body_sig.get(&f.target).copied().unwrap_or(0)), format!("{:?}", cx.last_sheet)];
        if let FeatureKind::Sketch(sk) = &f.kind {
            if let PlaneRef::Datum(pid) = sk.plane {
                parts.push(ref_sig(doc, pid, &cx.vars, 0));
            }
        }
        let mut touched = vec![f.target];
        let mut all = ["\"Face\"", "\"Vertex\""].iter().any(|k| js.contains(k)) && !matches!(f.kind, FeatureKind::Sketch(_));
        if let FeatureKind::Sketch(sk) = &f.kind {
            // sketch: lê corpos se o plano ou a geometria externa vierem de faces/arestas
            let ext = serde_json::to_string(&sk.data.links).unwrap_or_default();
            all |= ext.contains("\"Edge\"") || ext.contains("\"Vertex\"") || ext.contains("\"Face\"");
            if let PlaneRef::Datum(pid) = sk.plane {
                all |= ref_reads_bodies(doc, pid, 0);
            }
        }
        for x in ids {
            if let Some(sg) = feat_out.get(&x) {
                parts.push(format!("f{}:{sg}", x.0));
            } else if doc.collectors.iter().any(|c| c.id == x) {
                if !touched.contains(&x) {
                    touched.push(x);
                }
                parts.push(format!("b{}:{}", x.0, cx.body_sig.get(&x).copied().unwrap_or(0)));
            } else if doc.reference(x).is_some() {
                parts.push(ref_sig(doc, x, &cx.vars, 0));
                all |= ref_reads_bodies(doc, x, 0);
            }
        }
        if all {
            parts.push(all_bodies(&cx.body_sig));
        }
        let sig = hash_of(&parts.iter().map(|x| x.as_str()).collect::<Vec<_>>(), global);
        match cache.feats.get(&f.id).filter(|e| e.sig == sig).cloned() {
            Some(e) => {
                // nada que ele lê mudou: reaproveita o que ele produziu
                cache.reused += 1;
                cx.status.insert(f.id, e.status);
                for (b, v) in e.bodies {
                    match v {
                        Some(v) => cx.bodies.insert(b, v),
                        None => cx.bodies.remove(&b),
                    };
                }
                if let Some(sk) = e.sketch {
                    cx.sketches.insert(f.id, sk);
                }
                match e.tool {
                    Some(t) => cx.tools.insert(f.id, t),
                    None => cx.tools.remove(&f.id),
                };
                cx.last_sheet = e.last_sheet;
            }
            None => {
                let st = match cx.run(f) {
                    Ok(None) => Status::Ok,
                    Ok(Some(w)) => Status::Warning(w),
                    Err(e) => Status::Error(e),
                };
                cx.status.insert(f.id, st.clone());
                cache.feats.insert(
                    f.id,
                    FeatEntry {
                        sig,
                        status: st,
                        bodies: touched.iter().map(|b| (*b, cx.bodies.get(b).cloned())).collect(),
                        sketch: cx.sketches.get(&f.id).cloned(),
                        tool: cx.tools.get(&f.id).cloned(),
                        last_sheet: cx.last_sheet,
                    },
                );
            }
        }
        cx.memo.borrow_mut().clear();
        feat_out.insert(f.id, sig);
        // o sketch não muda corpo nenhum: quem usa o sketch já lê a assinatura dele
        if !matches!(f.kind, FeatureKind::Sketch(_)) {
            for b in &touched {
                cx.body_sig.insert(*b, sig);
            }
        }
    }
    cache.feats.retain(|id, _| seen.contains(id));
    let h = hash_of(&[&all_bodies(&cx.body_sig)], global);

    // planos: Origem + Referências. Todas entram no resultado (ocultas também: a cena filtra pela
    // visibilidade atual, então mostrar/ocultar não exige recálculo). Só reavalia a referência cuja
    // geometria mudou — ou, se ela lê faces de corpos, quando a linha do tempo mudou.
    let mut planes = vec![];
    for (p, n) in [(OriginPlane::XY, "XY"), (OriginPlane::XZ, "XZ"), (OriginPlane::YZ, "YZ")] {
        planes.push(PlaneViz { id: None, name: n.into(), frame: origin_frame(p) });
    }
    let tail = h;
    let (mut points, mut curves, mut surfaces) = (vec![], vec![], vec![]);
    cache.refs.retain(|id, _| doc.reference(*id).is_some());
    for r in &doc.references {
        let key = hash_of(&[&ref_sig(doc, r.id, &cx.vars, 0)], if ref_reads_bodies(doc, r.id, 0) { tail } else { global });
        let v = match cache.refs.get(&r.id).filter(|(k, _)| *k == key) {
            Some((_, v)) => v.clone(),
            None => {
                let v = cx.geo(r.id, 0).and_then(|g| {
                    Ok(match g {
                        Geo::Plane(fr) => RefViz::Plane(fr),
                        Geo::Point(p) => RefViz::Point(p),
                        Geo::Curve(c) => RefViz::Curve(c.pts),
                        Geo::Surface(sh) => {
                            let defl = sh.bbox().map(|(a, b)| ((0..3).map(|i| (b[i] - a[i]).powi(2)).sum::<f64>().sqrt() * 0.002).clamp(0.01, 1.0)).unwrap_or(0.05);
                            RefViz::Surface(sh.mesh(defl).map_err(|e| e.0)?)
                        }
                    })
                });
                cache.refs.insert(r.id, (key, v.clone()));
                v
            }
        };
        match v {
            Ok(g) => {
                cx.status.insert(r.id, Status::Ok);
                match g {
                    RefViz::Plane(fr) => planes.push(PlaneViz { id: Some(r.id), name: r.name.clone(), frame: fr }),
                    RefViz::Point(p) => points.push(PointViz { id: r.id, name: r.name.clone(), p }),
                    RefViz::Curve(pts) => {
                        let verts = match &r.kind {
                            RefKind::Polyline { points, .. } | RefKind::Spline { points, .. } | RefKind::SplineControl { points, .. } => cx.points(points, 0).unwrap_or_default(),
                            _ => pts.first().copied().into_iter().chain(pts.last().copied()).collect(),
                        };
                        curves.push(CurveViz { id: r.id, name: r.name.clone(), pts, verts })
                    }
                    RefViz::Surface(mesh) => surfaces.push(SurfaceViz { id: r.id, name: r.name.clone(), mesh }),
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
            link_pts: es.link_pts.clone(),
            link_r: es.link_r.clone(),
            ents: es.curves.iter().map(|(cid, pts)| (*cid, es.cons.contains(cid), pts.iter().map(|q| to3d(fr, *q)).collect())).collect(),
            pt_ids: es.pts.iter().map(|(pid, q)| (*pid, to3d(fr, *q))).collect(),
        };
        // sombra: a face real (com furos internos) tesselada pelo kernel (com cache)
        if !es.loops.is_empty() {
            let key = (*id, hash_of(&[&format!("{:?}{:?}", es.loops, (fr.origin, fr.x, fr.normal))], 0));
            if let Some(f) = cache.fills.get(&key) {
                v.fill = f.clone();
            } else if let Ok(m) = Shape::face(fr, &es.loops).and_then(|f| f.mesh(0.2)) {
                for t in m.indices.chunks_exact(3) {
                    for &i in t {
                        let k = i as usize * 3;
                        v.fill.push([m.positions[k], m.positions[k + 1], m.positions[k + 2]]);
                    }
                }
                cache.fills.insert(key, v.fill.clone());
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
        let key = (c.id, cx.body_sig.get(&c.id).copied().unwrap_or(0));
        let cached = cache.meshes.get(&key).cloned();
        let r = match cached {
            Some(x) => Ok(x),
            None => {
                let defl = s
                    .bbox()
                    .map(|(a, b)| ((0..3).map(|i| (b[i] - a[i]).powi(2)).sum::<f64>().sqrt() * 0.0015).clamp(0.005, 1.0))
                    .unwrap_or(0.1);
                s.mesh(defl).map(|m| {
                    let x = (m, s.mass(), s.face_count());
                    cache.meshes.insert(key, x.clone());
                    x
                })
            }
        };
        match r {
            Ok((mesh, mass, faces)) => bodies.push(BodyResult {
                collector: c.id,
                color: c.color,
                mass,
                faces,
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
        } else if e.kind.class() == RefClass::Surface {
            // superfícies: a forma exata vai como texto B-rep
            if let Some(b) = surface_brep(doc, e.id) {
                out.push((e.name.clone(), LinkedGeo::Surface(b)));
            }
        }
    }
    for p in doc.params.iter().filter(|p| p.published) {
        if let Some(v) = r.params.get(&p.name) {
            out.push((p.name.clone(), LinkedGeo::Value(*v)));
        }
    }
    // corpos publicados: a forma exata (B-rep) — quem importa usa como superfície/corpo de referência
    if doc.collectors.iter().any(|c| c.published) {
        if let Ok(vars) = doc.eval_params() {
            let mut cx = Ctx {
                doc,
                vars,
                bodies: HashMap::new(),
                sketches: HashMap::new(),
                status: HashMap::new(),
                tools: HashMap::new(),
                memo: RefCell::new(HashMap::new()),
                body_sig: HashMap::new(),
                last_sheet: None,
            };
            for f in doc.active_features().map(|(_, f)| f) {
                let _ = cx.run(f);
            }
            for c in doc.collectors.iter().filter(|c| c.published) {
                if let Some(b) = cx.body(c.id).and_then(|b| b.to_brep().ok()) {
                    out.push((c.name.clone(), LinkedGeo::Surface(b)));
                }
            }
        }
    }
    out
}

/// Superfície de referência avaliada no fim da linha do tempo, em texto B-rep.
fn surface_brep(doc: &Document, id: Id) -> Option<String> {
    let vars = doc.eval_params().ok()?;
    let mut cx = Ctx {
        doc,
        vars,
        bodies: HashMap::new(),
        sketches: HashMap::new(),
        status: HashMap::new(),
        tools: HashMap::new(),
        memo: RefCell::new(HashMap::new()),
        body_sig: HashMap::new(),
        last_sheet: None,
    };
    for f in doc.active_features().map(|(_, f)| f) {
        let _ = cx.run(f);
    }
    match cx.geo(id, 0).ok()? {
        Geo::Surface(s) => s.to_brep().ok(),
        _ => None,
    }
}

/// Geometria de um membro estrutural.
#[derive(Clone, Debug)]
struct MemberGeom {
    start: V3,
    end: V3,
    /// eixo do membro e eixos da seção (x, y)
    z: V3,
    x: V3,
    y: V3,
    w: f64,
    h: f64,
    /// deslocamento da seção (ancoragem − deslocamentos)
    shift: [f64; 2],
    ext: (f64, f64),
    loops: Vec<scc_kernel::Loop2>,
    pts: Vec<V3>,
    straight: bool,
}

/// Ponta comum de dois membros: (ponto, direção para dentro do primeiro, do segundo).
fn shared_end(a: &MemberGeom, b: &MemberGeom) -> Option<(V3, V3, V3)> {
    let close = |p: V3, q: V3| geo3d::dist(p, q) < 1e-4;
    let mz = |v: V3| geo3d::mul(v, -1.);
    for (pa, da) in [(a.start, a.z), (a.end, mz(a.z))] {
        for (pb, db) in [(b.start, b.z), (b.end, mz(b.z))] {
            if close(pa, pb) {
                return Some((pa, da, db));
            }
        }
    }
    None
}

/// Meio-espaço (caixa grande) do lado de `n` a partir do ponto `p`.
fn half_space(p: V3, n: V3, big: f64) -> Result<Shape, String> {
    let x = geo3d::any_perp(n);
    let y = scc_kernel::cross(n, x);
    let o = geo3d::sub(geo3d::sub(p, geo3d::mul(x, big)), geo3d::mul(y, big));
    Shape::box_tool(o, x, n, [2. * big, 2. * big, big]).map_err(|e| e.0)
}

/// Linha da lista de corte.
#[derive(Clone, Debug, PartialEq)]
pub struct CutRow {
    pub feature: Id,
    pub designation: String,
    pub length: f64,
    pub count: usize,
}

/// Lista de corte dos membros estruturais (comprimento de cada barra, agrupado por perfil e medida).
pub fn cut_list(doc: &Document) -> Vec<CutRow> {
    let Ok(vars) = doc.eval_params() else { return vec![] };
    let cx = Ctx {
        doc,
        vars,
        bodies: HashMap::new(),
        sketches: HashMap::new(),
        status: HashMap::new(),
        tools: HashMap::new(),
        memo: RefCell::new(HashMap::new()),
        body_sig: HashMap::new(),
        last_sheet: None,
    };
    let mut rows: Vec<CutRow> = vec![];
    for f in doc.active_features().map(|(_, f)| f) {
        let FeatureKind::StructMember(m) = &f.kind else { continue };
        let Ok(geo) = cx.members(m) else { continue };
        let explicit = (ev(&m.ext1, &cx.vars, "").unwrap_or(0.), ev(&m.ext2, &cx.vars, "").unwrap_or(0.));
        for g in geo {
            let len = if g.straight { geo3d::dist(g.start, g.end) + explicit.0 + explicit.1 } else { g.pts.windows(2).map(|w| geo3d::dist(w[0], w[1])).sum() };
            let len = (len * 10.).round() / 10.;
            let name = format!("{} {}", m.family, m.designation);
            match rows.iter_mut().find(|r| r.designation == name && (r.length - len).abs() < 0.05) {
                Some(r) => r.count += 1,
                None => rows.push(CutRow { feature: f.id, designation: name, length: len, count: 1 }),
            }
        }
    }
    rows
}

/// Parâmetros de chapa já avaliados.
#[derive(Clone, Copy, Debug)]
struct SheetValues {
    t: f64,
    r: f64,
    k: f64,
    flat: bool,
}

thread_local! {
    /// coleta dos furos durante `hole_table` (fora dela fica `None`)
    static HOLE_LOG: RefCell<Option<Vec<HoleRow>>> = const { RefCell::new(None) };
}

/// Linha da tabela de furos: centro da boca, direção, diâmetro, profundidade e rosca.
#[derive(Clone, Debug, PartialEq)]
pub struct HoleRow {
    pub feature: Id,
    pub pos: V3,
    pub dir: V3,
    pub diameter: f64,
    pub depth: f64,
    /// passante (atravessa tudo / até a última face)
    pub through: bool,
    pub thread: String,
}

/// Todos os furos da peça (um por centro), na ordem da árvore.
pub fn hole_table(doc: &Document) -> Vec<HoleRow> {
    if !doc.active_features().any(|(_, f)| matches!(f.kind, FeatureKind::Hole(_))) {
        return vec![];
    }
    HOLE_LOG.with(|l| *l.borrow_mut() = Some(vec![]));
    let _ = final_shape(doc);
    HOLE_LOG.with(|l| l.borrow_mut().take()).unwrap_or_default()
}

/// Linha da tabela de dobras.
#[derive(Clone, Debug, PartialEq)]
pub struct BendRow {
    pub feature: Id,
    pub name: String,
    pub angle: f64,
    pub radius: f64,
    /// comprimento da dobra na linha neutra (bend allowance)
    pub allowance: f64,
    /// dedução de dobra (2 × recuo externo − allowance), para quem cota por fora
    pub deduction: f64,
}

/// Tabela de dobras da peça de chapa (ângulo, raio, comprimento desenvolvido e dedução).
pub fn bend_table(doc: &Document) -> Vec<BendRow> {
    let Some(s) = &doc.sheet else { return vec![] };
    let Ok(vars) = doc.eval_params() else { return vec![] };
    let num = |e: &String| ev(e, &vars, "").unwrap_or(0.);
    let (t, r0, kf) = (num(&s.thickness), num(&s.radius), num(&s.k));
    let mut out = vec![];
    for f in doc.active_features().map(|(_, f)| f) {
        let (ang, r) = match &f.kind {
            FeatureKind::SmFlange(fl) => (num(&fl.angle), fl.radius.as_ref().map(num).unwrap_or(r0)),
            FeatureKind::SmHem(h) => (180., (num(&h.gap) / 2.).max(0.01)),
            FeatureKind::SmBend(b) => (num(&b.angle), b.radius.as_ref().map(num).unwrap_or(r0)),
            _ => continue,
        };
        let a = ang.to_radians();
        let ba = a * (r + kf * t);
        let ossb = if ang < 179.9 { (a / 2.).tan() * (r + t) } else { r + t };
        out.push(BendRow { feature: f.id, name: f.name.clone(), angle: ang, radius: r, allowance: ba, deduction: 2. * ossb - ba });
    }
    out
}

/// Geometria guardada de um link (ou cópia isolada).
fn linked_geo(g: &LinkedGeo) -> Result<Geo, String> {
    Ok(match g {
        LinkedGeo::Plane { origin, x, normal } => Geo::Plane(Frame { origin: *origin, x: *x, normal: *normal }),
        LinkedGeo::Point(p) => Geo::Point(*p),
        LinkedGeo::Curve(pts) => Geo::Curve(Curve3::poly(pts.clone(), false)?),
        LinkedGeo::Surface(b) => Geo::Surface(Shape::from_brep(b).map_err(|e| e.0)?),
        LinkedGeo::Value(_) => return Err("publicação de valor vira parâmetro, não geometria".into()),
    })
}

/// Exporta o corpo de um coletor (ou de todos) após recalcular.
pub fn export(doc: &Document, collector: Option<Id>, path: &str) -> Result<(), String> {
    let vars = doc.eval_params().map_err(|e| e.to_string())?;
    let mut cx = Ctx {
        doc,
        vars,
        bodies: HashMap::new(),
        sketches: HashMap::new(),
        status: HashMap::new(),
        tools: HashMap::new(),
        memo: RefCell::new(HashMap::new()),
        body_sig: HashMap::new(),
        last_sheet: None,
    };
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

/// Forma final da peça (todos os corpos visíveis juntos) — usada pela montagem para achar faces
/// e eixos das restrições.
pub fn final_shape(doc: &Document) -> Option<Shape> {
    let vars = doc.eval_params().ok()?;
    let mut cx = Ctx {
        doc,
        vars,
        bodies: HashMap::new(),
        sketches: HashMap::new(),
        status: HashMap::new(),
        tools: HashMap::new(),
        memo: RefCell::new(HashMap::new()),
        body_sig: HashMap::new(),
        last_sheet: None,
    };
    for f in doc.active_features().map(|(_, f)| f) {
        let _ = cx.run(f);
    }
    let mut all = Shape::empty();
    for c in doc.collectors.iter().filter(|c| c.visible) {
        if let Some(b) = cx.body(c.id) {
            all = if all.is_null() { b.clone() } else { all.compound(b) };
        }
    }
    (!all.is_null()).then_some(all)
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
    let ex = d.add_feature(FeatureKind::Extrude(Extrude { sketch: base, op: BoolOp::Add, extent: Extent::Blind(s("esp")), reverse: false, taper: s("0"), thin: None, dir: None }));
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
        points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false,
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
    fn estrutural_quadro_cantos_placa_reforco() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let s = |x: &str| x.to_string();
        let quadro = |corner: CornerMode| {
            let mut d = Document::new("quadro");
            let pts = [[0., 0., 0.], [400., 0., 0.], [400., 300., 0.], [0., 300., 0.]];
            let mut paths = vec![];
            for i in 0..4 {
                let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(pts[i]), b: PointRef::Fixed(pts[(i + 1) % 4]), start: s("0"), end: s("0") });
                paths.push(CurveRef::Ref(l));
            }
            d.add_feature(FeatureKind::StructMember(StructMember {
                paths,
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
                corner,
                gap: s("0"),
            }));
            d
        };
        let a = 40. * 40. - 34. * 34.;
        // meia-esquadria conserva o comprimento da linha de centro
        let d = quadro(CornerMode::MeiaEsquadria);
        let v = vol(&d);
        assert!(near(v, a * 1400., 1e-3), "meia-esquadria {v}");
        // encaixe/aparar: os posteriores param nos anteriores (menos volume que as barras inteiras + sobras)
        for c in [CornerMode::Aparar, CornerMode::Encaixe] {
            let v2 = vol(&quadro(c));
            assert!(v2 > a * 1300. && v2 < a * 1500., "{c:?} {v2}");
        }
        let cl = cut_list(&d);
        assert_eq!(cl.iter().map(|r| r.count).sum::<usize>(), 4);
        assert_eq!(cl.len(), 2, "duas medidas: 400 e 300");
        // placa de extremidade e reforço num L aberto
        let mut d = Document::new("l");
        let l1 = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 0., 500.]), start: s("0"), end: s("0") });
        let l2 = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 500.]), b: PointRef::Fixed([300., 0., 500.]), start: s("0"), end: s("0") });
        d.add_feature(FeatureKind::StructMember(StructMember {
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
        }));
        let v0 = vol(&d);
        d.add_feature(FeatureKind::StructPlate(StructPlate { at: [0., 0., 0.], thickness: s("8"), margin: s("5") }));
        let v1 = vol(&d);
        assert!(near(v1 - v0, 50. * 50. * 8., 1e-3), "placa {}", v1 - v0);
        d.add_feature(FeatureKind::StructGusset(StructGusset { at: [0., 0., 500.], leg: s("60"), thickness: s("6"), chamfer: s("0") }));
        let v2 = vol(&d);
        assert!(near(v2 - v1, 0.5 * 60. * 60. * 6., 1e-3), "reforço {}", v2 - v1);
    }

    #[test]
    fn sketch_com_geometria_externa_associativa() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        use scc_core::sketch::LinkSrc;
        let s = |x: &str| x.to_string();
        let mut d = Document::new("assoc");
        let lado = d.add_param("L", "40", Unit::Comprimento).unwrap();
        let _ = lado;
        // linha de referência de (0,0,0) a (L,0,0)
        let la = d.add_geometry(RefKind::LinePoints { a: PointRef::Origin, b: PointRef::Fixed([40., 0., 0.]), start: s("0"), end: s("0") });
        let mut data = SketchData::default();
        let mut n = 5_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let link = data.add_link(&mut al, LinkSrc::Ref(la), false, &[[0., 0.], [40., 0.]], true).unwrap();
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        // a referência muda: o ponto final no sketch acompanha
        if let Some(r) = d.references.iter_mut().find(|r| r.id == la) {
            r.kind = RefKind::LinePoints { a: PointRef::Origin, b: PointRef::Fixed([70., 0., 0.]), start: s("0"), end: s("0") };
        }
        let r = recompute(&d);
        assert!(matches!(r.status[&sk], Status::Ok | Status::Warning(_)), "{:?}", r.status[&sk]);
        let v = r.sketches.iter().find(|v| v.feature == sk).unwrap();
        let xmax = v.construction.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
        assert!((xmax - 70.).abs() < 1e-3, "a linha projetada foi para 70: {xmax}");
        let _ = link;
    }

    #[test]
    fn chapa_joggle_estampo_furo_flangeado_alivio() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        use std::f64::consts::PI;
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let s = |x: &str| x.to_string();
        let base = || {
            let mut d = Document::new("chapa");
            d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false });
            let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
            d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false }));
            d
        };
        let check = |d: &Document| {
            let r = recompute(d);
            for f in &d.features {
                assert_eq!(r.status[&f.id], Status::Ok, "{}: {:?}", f.name, r.status[&f.id]);
            }
            r
        };
        // joggle: o volume se conserva e a parte além da linha desce 3 mm
        let mut d = base();
        d.add_feature(FeatureKind::SmJoggle(SmJoggle { a: [60., 0., 2.], b: [60., 50., 2.], depth: s("3"), run: s("5"), flip: true }));
        let r = check(&d);
        assert!(near(r.bodies[0].mass.volume, 10000., 1e-3), "joggle {}", r.bodies[0].mass.volume);
        let zmin = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MAX, f32::min);
        assert!((zmin + 3.).abs() < 0.01, "desvio {zmin}");
        // dobra por linha a 90°: volume conservado (comprimento desenvolvido) e a ponta sobe
        let mut d = base();
        d.add_feature(FeatureKind::SmBend(SmBend { a: [60., 0., 2.], b: [60., 50., 2.], angle: s("90"), radius: None, flip: false, down: false }));
        let r = check(&d);
        assert!(near(r.bodies[0].mass.volume, 10000., 2e-3), "dobra por linha {}", r.bodies[0].mass.volume);
        let zmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max);
        assert!(zmax > 30., "a aba subiu {zmax}");
        // veneziana: volume conservado e abertura de 5 mm
        let mut d = base();
        d.add_feature(FeatureKind::SmLouver(SmLouver { a: [30., 10., 2.], b: [30., 40., 2.], width: s("12"), height: s("5"), flip: false, down: false }));
        let r = check(&d);
        assert!(near(r.bodies[0].mass.volume, 10000., 1e-3), "veneziana {}", r.bodies[0].mass.volume);
        let zmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max);
        assert!((zmax - 7.).abs() < 0.01, "altura da veneziana {zmax}");
        // estampo 20×20 com 3 mm de profundidade
        let mut d = base();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "2");
        let sq = sk_rect(&mut d, PlaneRef::Datum(top), [40., 15.], [60., 35.]);
        d.add_feature(FeatureKind::SmStamp(SmStamp { sketch: sq, depth: s("3"), flip: false }));
        let r = check(&d);
        let outer = 400. + 80. * 2. + PI * 4.;
        assert!(near(r.bodies[0].mass.volume, 10000. + outer * 3. - 400. * 3., 1e-3), "estampo {}", r.bodies[0].mass.volume);
        // furo flangeado Ø10 com colar de 6 mm
        let mut d = base();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "2");
        let mut data = SketchData::default();
        let mut n = 7_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let p0 = data.add_point(&mut al, 50., 25.);
        data.add_curve(&mut al, scc_core::sketch::Curve::Point { p: p0 }, false);
        let sp = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(top), data }));
        d.add_feature(FeatureKind::SmFlangedHole(SmFlangedHole { sketch: sp, diameter: s("10"), height: s("6"), flip: false }));
        let r = check(&d);
        assert!(near(r.bodies[0].mass.volume, 10000. + PI * 24. * 6. - PI * 25. * 2., 1e-3), "furo flangeado {}", r.bodies[0].mass.volume);
        // aba recuada 10 mm de cada lado com alívio de 3 mm
        let mut d = base();
        d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: None, off1: s("10"), off2: s("10"), flip: false, relief: None }));
        let v_sem = check(&d).bodies[0].mass.volume;
        if let FeatureKind::SmFlange(fl) = &mut d.features.last_mut().unwrap().kind {
            fl.relief = Some(s("3"));
        }
        let v_com = check(&d).bodies[0].mass.volume;
        assert!(near(v_sem - v_com, 2. * 3. * 4. * 2., 1e-3), "alívio {}", v_sem - v_com);
    }

    #[test]
    fn chapa_parede_aba_bainha_e_planificado() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        use std::f64::consts::PI;
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let s = |x: &str| x.to_string();
        let mut d = Document::new("chapa");
        d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false });
        let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
        d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false }));
        d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None }));
        let r = recompute(&d);
        for f in &d.features {
            assert!(matches!(r.status[&f.id], Status::Ok | Status::Warning(_)), "{}: {:?}", f.name, r.status[&f.id]);
        }
        let v = r.bodies[0].mass.volume;
        let bend = PI / 4. * (16. - 4.) * 50.;
        assert!(near(v, 10000. + bend + 2000., 1e-3), "dobrada {v}");
        // a aba sobe: topo em z = 2 + R + 20
        let zmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max);
        assert!((zmax - 24.).abs() < 0.05, "altura {zmax}");
        // planificado: tira com o comprimento desenvolvido
        d.sheet.as_mut().unwrap().flat = true;
        let r = recompute(&d);
        let ba = PI / 2. * (2. + 0.44 * 2.);
        let vf = r.bodies[0].mass.volume;
        assert!(near(vf, 10000. + (ba + 20.) * 2. * 50., 1e-3), "plana {vf}");
        let tab = bend_table(&d);
        assert_eq!(tab.len(), 1);
        assert!((tab[0].allowance - ba).abs() < 1e-9);
        // bainha esmagada na aresta oposta (x = 0), dobrando para baixo
        d.sheet.as_mut().unwrap().flat = false;
        d.add_feature(FeatureKind::SmHem(SmHem { edge: [0., 25., 2.], length: s("8"), gap: s("0"), flip: true }));
        let r = recompute(&d);
        let st = &r.status[&d.features.last().unwrap().id];
        assert_eq!(*st, Status::Ok, "{st:?}");
        assert!(r.bodies[0].mass.volume > v + 8. * 2. * 50. * 0.9, "bainha {}", r.bodies[0].mass.volume);
    }

    #[test]
    fn publicar_superficie_link_e_isolar() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        let mut src = Document::new("Mestre");
        let sph = src.add_geometry(RefKind::SurfSphere { center: PointRef::Origin, radius: "10".into() });
        src.references.iter_mut().find(|r| r.id == sph).unwrap().published = true;
        let pubs = publications(&src);
        let (n, g) = pubs.iter().find(|(_, g)| matches!(g, LinkedGeo::Surface(_))).expect("superfície publicada");
        let mut d = Document::new("Filha");
        let id = d.add_link("Mestre", "mestre.sccpart", n, g.clone());
        let r = recompute(&d);
        assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
        // desativado: inativo até reativar
        let ext = d.external[0].id;
        d.external[0].state = LinkState::Desativado;
        assert!(matches!(recompute(&d).status[&id], Status::Error(_)));
        d.external[0].state = LinkState::Ativo;
        d.isolate_link(ext);
        assert!(d.external.is_empty());
        let r = recompute(&d);
        assert_eq!(r.status[&id], Status::Ok, "isolada: {:?}", r.status[&id]);
        assert!(r.surfaces.iter().any(|s| s.id == id));
    }

    #[test]
    fn entradas_consumidas_e_renomear() {
        let mut d = Document::new("c");
        let s = |x: &str| x.to_string();
        let a = d.add_geometry(RefKind::SurfSphere { center: PointRef::Origin, radius: s("10") });
        let b = d.add_geometry(RefKind::SurfSphere { center: PointRef::Fixed([15., 0., 0.]), radius: s("10") });
        let j = d.add_geometry(RefKind::SurfJoin { items: vec![SurfRef::Ref(a), SurfRef::Ref(b)], tol: s("0.01") });
        d.hide_consumed(j);
        assert!(!d.reference(a).unwrap().visible && !d.reference(b).unwrap().visible);
        d.delete(j, false);
        assert!(d.reference(a).unwrap().visible && d.reference(b).unwrap().visible, "apagar a operação reexibe as entradas");
        d.rename_item(a, "Bola").unwrap();
        assert_eq!(d.name_of(a), "Bola");
        assert!(d.rename_item(b, "Bola").is_err(), "nome repetido");
    }

    #[test]
    fn furos_e_terminos_novos() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        use std::f64::consts::PI;
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let s = |x: &str| x.to_string();
        let v0 = vol(&caixa());
        let furo = |pts: Vec<PointRef>, kind: HoleKind, tip: Option<String>, depth: Extent| Hole {
            sketch: Id(0),
            kind,
            diameter: s("6"),
            depth,
            thread: None,
            thread_modeled: false,
            standard: None,
            points: pts,
            dir: None,
            reverse: false,
            tip,
            taper: s("0"),
            left_hand: false,
        };
        // dois furos Ø6 × 5 por pontos 3D no topo (normal à superfície, para baixo)
        let topo = vec![PointRef::Fixed([-10., 0., 20.]), PointRef::Fixed([10., 0., 20.])];
        let mut d = caixa();
        d.add_feature(FeatureKind::Hole(furo(topo.clone(), HoleKind::Simples, None, Extent::Blind(s("5")))));
        let vf = vol(&d);
        assert!(near(vf, v0 - 2. * PI * 9. * 5., 1e-3), "furos 3D {vf}");
        // fundo em V tira mais que o fundo plano
        let mut d = caixa();
        d.add_feature(FeatureKind::Hole(furo(topo.clone(), HoleKind::Simples, Some(s("118")), Extent::Blind(s("5")))));
        assert!(vol(&d) < vf - 1., "fundo em V");
        // furo numa face lateral (x = 20): a normal da face manda o furo para -X
        let mut d = caixa();
        d.add_feature(FeatureKind::Hole(furo(vec![PointRef::Fixed([20., 0., 10.])], HoleKind::Simples, None, Extent::Blind(s("5")))));
        assert!(near(vol(&d), v0 - PI * 9. * 5., 1e-3), "furo lateral {}", vol(&d));
        // contrafurado passante
        let mut d = caixa();
        d.add_feature(FeatureKind::Hole(furo(vec![topo[0]], HoleKind::Contrafurado { diameter: s("10"), depth: s("3"), angle: s("90") }, None, Extent::ThroughAll)));
        let vc = vol(&d);
        assert!(vc < v0 - PI * 9. * 20. - PI * 16. * 3. + 1., "contrafurado {vc}");

        // extrusão até o último: do plano z = 30 para baixo até a base (z = 0)
        let mut d = caixa();
        let alto = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30");
        let sk = sk_rect(&mut d, PlaneRef::Datum(alto), [-5., -5.], [5., 5.]);
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::UpToLast, reverse: true, taper: s("0"), thin: None, dir: None }));
        assert!(near(vol(&d), v0 + 100. * 10., 1e-6), "até o último {}", vol(&d));

        // extrusão inclinada por uma direção: o volume do prisma oblíquo é área × altura
        let mut d = Document::new("e");
        let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [10., 10.]);
        let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Origin, b: PointRef::Fixed([10., 0., 10.]), start: s("0"), end: s("0") });
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("10")), reverse: false, taper: s("0"), thin: None, dir: Some(AxisRef::Ref(l)) }));
        assert!(near(vol(&d), 100. * 10. / 2f64.sqrt(), 1e-6), "oblíqua {}", vol(&d));

        // padrão linear por comprimento total: 3 bolsos em 20 mm = espaçamento 10
        let mut d = caixa();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let sp = sk_rect(&mut d, PlaneRef::Datum(top), [-16., -2.], [-12., 2.]);
        let pocket = d.add_feature(ext(sp, BoolOp::Cut, "5"));
        d.add_feature(FeatureKind::Pattern(Pattern {
            kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: s("3"), spacing: s("20"), dir2: None, count2: s("1"), spacing2: s("0"), total: true },
            sources: vec![pocket],
        }));
        assert!(near(vol(&d), v0 - 240., 1e-6), "padrão por total {}", vol(&d));

        // mover o corpo: translação mantém o volume; afinidade multiplica por fx·fy·fz
        let mut d = caixa();
        d.add_feature(FeatureKind::MoveBody(MoveBody { xf: BodyXf::Translate { dir: AxisRef::Origin(OriginAxis::X), dist: s("50") } }));
        let r = recompute(&d);
        assert!(near(r.bodies[0].mass.volume, v0, 1e-6) && (r.bodies[0].mass.center[0] - 50.).abs() < 1e-6, "translação");
        let mut d = caixa();
        d.add_feature(FeatureKind::MoveBody(MoveBody { xf: BodyXf::Affinity { plane: PlaneRef::Origin(OriginPlane::XY), fx: s("2"), fy: s("1"), fz: s("0.5") } }));
        assert!(near(vol(&d), v0, 1e-3), "afinidade 2·1·0,5 {}", vol(&d));
    }

    #[test]
    fn curva_paralela_e_projecao_por_direcao() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let mut d = Document::new("w");
        let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Origin, b: PointRef::Fixed([30., 0., 0.]), start: s("0"), end: s("0") });
        let p = d.add_geometry(RefKind::CurveParallel { curve: CurveRef::Ref(l), support: SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY)), d1: s("5"), d2: s("5"), flip: false });
        let alto = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
        let pr = d.add_geometry(RefKind::CurveProjectSurf { curve: CurveRef::Ref(l), surface: SurfRef::Plane(PlaneRef::Datum(alto)), dir: Some(AxisRef::Fixed { origin: [0.; 3], dir: [0., 1., 1.] }) });
        let r = recompute(&d);
        for id in [p, pr] {
            assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
        }
        let c = r.curves.iter().find(|c| c.id == p).unwrap();
        assert!(c.pts.iter().all(|q| (q[1].abs() - 5.).abs() < 1e-9), "paralela a 5 mm");
        let c = r.curves.iter().find(|c| c.id == pr).unwrap();
        assert!(c.pts.iter().all(|q| (q[2] - 10.).abs() < 1e-9 && (q[1] - 10.).abs() < 1e-9), "projeção oblíqua");
    }

    fn mesh_area(m: &Mesh) -> f64 {
        let p = |i: u32| {
            let k = i as usize * 3;
            [m.positions[k] as f64, m.positions[k + 1] as f64, m.positions[k + 2] as f64]
        };
        m.indices
            .chunks_exact(3)
            .map(|t| {
                let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
                geo3d::norm(geo3d::cross(geo3d::sub(b, a), geo3d::sub(c, a))) / 2.
            })
            .sum()
    }

    #[test]
    fn bancada_de_superficies() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        use std::f64::consts::PI;
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let xy = || SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY));
        let s = |x: &str| x.to_string();
        let mut d = Document::new("s");
        let sph = d.add_geometry(RefKind::SurfSphere { center: PointRef::Origin, radius: s("10") });
        let half = d.add_geometry(RefKind::SurfSplit { surface: SurfRef::Ref(sph), tool: xy(), flip: false });
        let circ = d.add_geometry(RefKind::CurveIntersect { a: SurfRef::Ref(sph), b: xy(), which: 0 });
        let cyl = d.add_geometry(RefKind::SurfCylinder { point: PointRef::Origin, dir: AxisRef::Origin(OriginAxis::Z), radius: s("10"), start: s("0"), end: s("20") });
        let bnd = d.add_geometry(RefKind::CurveBoundary { surface: SurfRef::Ref(cyl), which: 0 });
        let moved = d.add_geometry(RefKind::GeoTransform {
            source: sph,
            class: RefClass::Surface,
            xf: GeoXform::Translate { dir: AxisRef::Origin(OriginAxis::X), dist: s("50") },
        });
        let p1 = d.add_geometry(RefKind::PointCoords { x: s("1"), y: s("0"), z: s("0"), frame: None });
        let pmov = d.add_geometry(RefKind::GeoTransform {
            source: p1,
            class: RefClass::Point,
            xf: GeoXform::Rotate { axis: AxisRef::Origin(OriginAxis::Z), angle: s("90") },
        });
        // duas bordas paralelas: tubo por varredura, multi-seções e concordância G1 entre duas faixas planas
        let la = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 30., 0.]), start: s("0"), end: s("0") });
        let lb = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([20., 0., 10.]), b: PointRef::Fixed([20., 30., 10.]), start: s("0"), end: s("0") });
        let tube = d.add_geometry(RefKind::SurfSweep { profile: SweepProfile::Circle(s("2")), spine: CurveRef::Ref(la), mode: SweepMode::Corrigido });
        let loft = d.add_geometry(RefKind::SurfLoft { sections: vec![CurveRef::Ref(la), CurveRef::Ref(lb)], guide: None, ruled: false, closed: false });
        let fa = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(la), dir: AxisRef::Origin(OriginAxis::X), length: s("-10"), length2: "0".into() });
        let fb = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(lb), dir: AxisRef::Origin(OriginAxis::X), length: s("10"), length2: "0".into() });
        let blend = d.add_geometry(RefKind::SurfBlend {
            a: CurveRef::Ref(la),
            sa: Some(SurfRef::Ref(fa)),
            b: CurveRef::Ref(lb),
            sb: Some(SurfRef::Ref(fb)),
            cont: Continuity::G1,
            tension: s("1"),
        });
        let join = d.add_geometry(RefKind::SurfJoin { items: vec![SurfRef::Ref(fa), SurfRef::Ref(blend), SurfRef::Ref(fb)], tol: s("0.01") });
        let bounds = d.add_geometry(RefKind::CurveBoundary { surface: SurfRef::Ref(join), which: 0 });
        let r = recompute(&d);
        for id in [sph, half, circ, cyl, bnd, moved, pmov, tube, loft, fa, fb, blend, join, bounds] {
            assert_eq!(r.status[&id], Status::Ok, "{}: {:?}", d.name_of(id), r.status[&id]);
        }
        let area = |id: Id| mesh_area(&r.surfaces.iter().find(|s| s.id == id).unwrap().mesh);
        assert!(near(area(half), 2. * PI * 100., 0.02), "meia esfera {}", area(half));
        let len = |id: Id| r.curves.iter().find(|c| c.id == id).unwrap().pts.windows(2).map(|w| geo3d::dist(w[0], w[1])).sum::<f64>();
        assert!(near(len(circ), 2. * PI * 10., 0.02), "círculo {}", len(circ));
        assert!(near(len(bnd), 2. * PI * 10., 0.02), "borda {}", len(bnd));
        assert!(near(area(tube), 2. * PI * 2. * 30., 0.03), "tubo {}", area(tube));
        let p = r.points.iter().find(|p| p.id == pmov).unwrap().p;
        assert!(geo3d::dist(p, [0., 1., 0.]) < 1e-9, "{p:?}");
        let c = r.surfaces.iter().find(|s| s.id == moved).unwrap().mesh.positions.chunks(3).map(|v| v[0] as f64).sum::<f64>()
            / (r.surfaces.iter().find(|s| s.id == moved).unwrap().mesh.positions.len() / 3) as f64;
        assert!((c - 50.).abs() < 1., "esfera movida {c}");
        assert!(area(blend) > 22.36 * 30., "concordância G1 é mais longa que a reta {}", area(blend));

        // híbridos: engrossar, fechar, preencher fronteiras, dividir corpo
        let mut h = d.clone();
        h.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(cyl), t1: s("2"), t2: s("0"), op: BoolOp::Add }));
        let v = vol(&h);
        assert!(near(v, PI * (144. - 100.) * 20., 0.02) || near(v, PI * (100. - 64.) * 20., 0.02), "engrossar {v}");
        let mut h = d.clone();
        h.add_feature(FeatureKind::CloseSurface(CloseSurface { surface: SurfRef::Ref(cyl), tol: s("0.01"), op: BoolOp::Add }));
        assert!(near(vol(&h), PI * 100. * 20., 0.01), "fechar {}", vol(&h));
        let mut h = d.clone();
        h.add_feature(FeatureKind::BoundaryFill(BoundaryFill { tools: vec![SurfRef::Ref(sph), xy()], exclude: vec![[0., 0., -5.]], op: BoolOp::Add }));
        assert!(near(vol(&h), 2. / 3. * PI * 1000., 0.02), "preencher {}", vol(&h));
        let mut h = caixa();
        let meio = h.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "5");
        h.add_feature(FeatureKind::SplitBody(SplitBody { surface: SurfRef::Plane(PlaneRef::Datum(meio)), flip: false }));
        assert!(near(vol(&h), 24000. * 0.75, 1e-6), "dividir {}", vol(&h));

        // malha STL (cubo de 10) → sólido
        let dir = std::env::temp_dir().join("scc_mesh_test.stl");
        let mut t = String::from("solid c\n");
        let v = |i: usize| [(i & 1) as f64 * 10., ((i >> 1) & 1) as f64 * 10., ((i >> 2) & 1) as f64 * 10.];
        for f in [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]] {
            for tri in [[f[0], f[1], f[2]], [f[0], f[2], f[3]]] {
                t += "facet normal 0 0 0\nouter loop\n";
                for k in tri {
                    let p = v(k);
                    t += &format!("vertex {} {} {}\n", p[0], p[1], p[2]);
                }
                t += "endloop\nendfacet\n";
            }
        }
        t += "endsolid c\n";
        std::fs::write(&dir, t).unwrap();
        let mut m = Document::new("m");
        m.add_feature(FeatureKind::MeshImport(MeshImport { path: dir.to_string_lossy().into(), solid: true }));
        let r = recompute(&m);
        assert!(matches!(r.status[&m.features[0].id], Status::Warning(_)), "{:?}", r.status[&m.features[0].id]);
        assert!(near(r.bodies[0].mass.volume, 1000., 1e-6), "malha {}", r.bodies[0].mass.volume);
    }

    #[test]
    fn terminos_unificados() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let v0 = vol(&caixa());
        // extrusão de cima para baixo até a face do topo: pino de 10 × 10 × 20
        let mut d = caixa();
        let alto = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "40");
        let s = sk_rect(&mut d, PlaneRef::Datum(alto), [-5., -5.], [5., 5.]);
        d.add_feature(FeatureKind::Extrude(Extrude {
            sketch: s,
            op: BoolOp::Add,
            extent: Extent::UpToSurface(SurfRef::Face { point: [0., 0., 20.], normal: [0., 0., 1.], owner: None }),
            reverse: true,
            taper: "0".into(),
            thin: None, dir: None,
        }));
        assert!(near(vol(&d), v0 + 2000., 1e-6), "até a face {}", vol(&d));

        // recorte do topo até um plano (z = 12) como superfície
        let mut d = caixa();
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let fundo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "12");
        let s = sk_rect(&mut d, PlaneRef::Datum(topo), [-5., -5.], [5., 5.]);
        d.add_feature(FeatureKind::Extrude(Extrude {
            sketch: s,
            op: BoolOp::Cut,
            extent: Extent::UpToSurface(SurfRef::Plane(PlaneRef::Datum(fundo))),
            reverse: false,
            taper: "0".into(),
            thin: None, dir: None,
        }));
        assert!(near(vol(&d), v0 - 800., 1e-6), "recorte até plano {}", vol(&d));

        // furo Ø6 do topo até o plano z = 12
        let mut d = caixa();
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let fundo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "12");
        let mut data = SketchData::default();
        let mut n = 5_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let p0 = data.add_point(&mut al, 0., 0.);
        data.add_curve(&mut al, scc_core::sketch::Curve::Point { p: p0 }, false);
        let sp = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data }));
        let furo = Hole { sketch: sp, kind: HoleKind::Simples, diameter: "6".into(), depth: Extent::UpTo(PlaneRef::Datum(fundo)), thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false };
        let mut e = d.clone();
        e.add_feature(FeatureKind::Hole(furo.clone()));
        assert!(near(vol(&e), v0 - std::f64::consts::PI * 9. * 8., 1e-3), "furo até plano {}", vol(&e));
        // o mesmo furo "até o próximo" atravessa a parede inteira (20)
        let mut e = d.clone();
        e.add_feature(FeatureKind::Hole(Hole { depth: Extent::UpToNext, ..furo }));
        assert!(near(vol(&e), v0 - std::f64::consts::PI * 9. * 20., 1e-3), "furo até o próximo {}", vol(&e));

        // rosca: comprimento 4 < face inteira
        let mut d = caixa();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Datum(top), a: "5".into(), b: "10".into(), c: "0".into(), d: "0".into(), op: BoolOp::Add }));
        let th = |end: Option<Extent>| Thread { face: [5., 0., 25.], spec: "M10x1.5".into(), length: "4".into(), modeled: true, end };
        let mut a = d.clone();
        a.add_feature(FeatureKind::Thread(th(None)));
        let mut b = d.clone();
        b.add_feature(FeatureKind::Thread(th(Some(Extent::ThroughAll))));
        let (va, vb) = (vol(&a), vol(&b));
        assert!(vb < va - 1., "rosca completa {vb} vs 4 mm {va}");
    }

    #[test]
    fn cache_reaproveita_linha_do_tempo() {
        let mut d = Document::new("c");
        scc_core::macros::run(&mut d, scc_core::macros::EXAMPLE).unwrap();
        let mut cache = Cache::default();
        let a = recompute_cached(&d, &mut cache);
        assert_eq!(cache.reused, 0);
        let b = recompute_cached(&d, &mut cache);
        assert_eq!(cache.reused, d.features.len());
        assert_eq!(a.status, b.status);
        if scc_kernel::AVAILABLE {
            assert!((a.bodies[0].mass.volume - b.bodies[0].mass.volume).abs() < 1e-6);
        }
        // renomear não recalcula nada
        let n = d.features.len();
        d.features[n - 1].name.push('x');
        let c = recompute_cached(&d, &mut cache);
        assert_eq!(cache.reused, n);
        if scc_kernel::AVAILABLE {
            assert!((a.bodies[0].mass.volume - c.bodies[0].mass.volume).abs() < 1e-6);
        }
    }

    #[test]
    fn cache_por_dependencia_refaz_so_o_ramo_editado() {
        let s = |x: &str| x.to_string();
        let mut d = Document::new("dois corpos");
        let a_col = d.active_collector;
        let ska = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [20., 20.]);
        let exa = d.add_feature(FeatureKind::Extrude(Extrude { sketch: ska, op: BoolOp::Add, extent: Extent::Blind(s("10")), reverse: false, taper: s("0"), thin: None, dir: None }));
        let b_col = d.add_collector("B");
        d.active_collector = b_col;
        let skb = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [50., 0.], [70., 20.]);
        d.add_param("HB", "5", Unit::Comprimento).unwrap();
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: skb, op: BoolOp::Add, extent: Extent::Blind(s("HB")), reverse: false, taper: s("0"), thin: None, dir: None }));
        let _ = a_col;
        let mut cache = Cache::default();
        recompute_cached(&d, &mut cache);
        // editar a extrusão do corpo A: o sketch A, o sketch B e a extrusão B vêm do cache
        if let Some(FeatureKind::Extrude(e)) = d.feature_mut(exa).map(|f| &mut f.kind) {
            e.extent = Extent::Blind(s("15"));
        }
        let r = recompute_cached(&d, &mut cache);
        assert_eq!(cache.reused, 3, "só a extrusão editada recalcula");
        // mudar o parâmetro HB refaz só quem o usa (a extrusão B)
        d.params.iter_mut().find(|p| p.name == "HB").unwrap().expr = s("8");
        recompute_cached(&d, &mut cache);
        assert_eq!(cache.reused, 3, "parâmetro: só a extrusão B recalcula");
        if scc_kernel::AVAILABLE {
            let va = r.bodies.iter().find(|b| b.collector != b_col).unwrap().mass.volume;
            assert!((va - 20. * 20. * 15.).abs() < 1e-3, "{va}");
        }
    }

    #[test]
    fn macro_de_exemplo_recalcula() {
        let mut d = Document::new("m");
        scc_core::macros::run(&mut d, scc_core::macros::EXAMPLE).unwrap();
        let r = recompute(&d);
        for f in &d.features {
            assert_eq!(r.status[&f.id], Status::Ok, "{}: {:?}", f.name, r.status[&f.id]);
        }
        if scc_kernel::AVAILABLE {
            // placa 120 × 70 × 8 menos 4 furos Ø8 (com filete de 1 mm em tudo)
            let v = r.bodies[0].mass.volume;
            assert!(v < 120. * 70. * 8. && v > 120. * 70. * 8. - 4. * std::f64::consts::PI * 16. * 8. - 2000., "{v}");
        }
    }

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
        let se = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(sp), dir: AxisRef::Origin(OriginAxis::Z), length: s_("10"), length2: "0".into() });
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
        FeatureKind::Extrude(Extrude { sketch, op, extent: Extent::Blind(d.into()), reverse: false, taper: "0".into(), thin: None, dir: None })
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
            kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: "3".into(), spacing: "8".into(), dir2: None, count2: "1".into(), spacing2: "0".into(), total: false },
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
        e.add_feature(FeatureKind::Thread(Thread { face: [5., 0., 25.], spec: "M10x1.5".into(), length: "8".into(), modeled: true, end: None }));
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
