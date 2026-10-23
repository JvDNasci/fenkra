//! Sketch paramétrico: pontos, curvas e restrições, com solver próprio.
//!
//! - **Geometria livre:** as ferramentas criam pontos e curvas sem cotas obrigatórias.
//! - **Restrições geométricas** (horizontal, paralela, tangente...) e **dimensionais** (cotas),
//!   cujos valores são expressões que aceitam parâmetros.
//! - **Solver** Levenberg–Marquardt com Jacobiano numérico; análise de graus de liberdade por
//!   eliminação (RREF) para saber o que está totalmente restrito.
//! - Pontos compartilhados entre curvas (mesmo `Id`) definem a conectividade dos perfis.

use crate::expr;
use crate::Id;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SkPoint {
    pub id: Id,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub fixed: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Curve {
    Line { a: Id, b: Id },
    Circle { c: Id, r: f64 },
    /// Arco anti-horário de `s` até `e` em torno de `c` (raio implícito: |s−c| = |e−c|).
    Arc { c: Id, s: Id, e: Id },
    /// Ponto isolado (centro de furo, referência).
    Point { p: Id },
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SkCurve {
    pub id: Id,
    pub construction: bool,
    pub geom: Curve,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Kind {
    Coincident(Id, Id),
    Horizontal(Id),
    Vertical(Id),
    Parallel(Id, Id),
    Perpendicular(Id, Id),
    Equal(Id, Id),
    Tangent(Id, Id),
    Concentric(Id, Id),
    Midpoint(Id, Id),
    PointOnCurve(Id, Id),
    Fixed(Id),
    Symmetric(Id, Id, Id),
    /// Pontos alinhados na horizontal / vertical.
    HorizontalPts(Id, Id),
    VerticalPts(Id, Id),
    // ---- cotas (valor = expressão)
    Distance(Id, Id, String),
    DistanceX(Id, Id, String),
    DistanceY(Id, Id, String),
    Length(Id, String),
    Radius(Id, String),
    Diameter(Id, String),
    Angle(Id, Id, String),
    PointLineDistance(Id, Id, String),
}

impl Kind {
    pub fn label(&self) -> &'static str {
        match self {
            Kind::Coincident(..) => "Coincidente",
            Kind::Horizontal(_) | Kind::HorizontalPts(..) => "Horizontal",
            Kind::Vertical(_) | Kind::VerticalPts(..) => "Vertical",
            Kind::Parallel(..) => "Paralela",
            Kind::Perpendicular(..) => "Perpendicular",
            Kind::Equal(..) => "Igual",
            Kind::Tangent(..) => "Tangente",
            Kind::Concentric(..) => "Concêntrica",
            Kind::Midpoint(..) => "Ponto médio",
            Kind::PointOnCurve(..) => "Sobre a curva",
            Kind::Fixed(_) => "Fixa",
            Kind::Symmetric(..) => "Simétrica",
            Kind::Distance(..) => "Distância",
            Kind::DistanceX(..) => "Distância horizontal",
            Kind::DistanceY(..) => "Distância vertical",
            Kind::Length(..) => "Comprimento",
            Kind::Radius(..) => "Raio",
            Kind::Diameter(..) => "Diâmetro",
            Kind::Angle(..) => "Ângulo",
            Kind::PointLineDistance(..) => "Distância ponto-linha",
        }
    }
    /// Valor (expressão) se for cota.
    pub fn value_mut(&mut self) -> Option<&mut String> {
        match self {
            Kind::Distance(_, _, v)
            | Kind::DistanceX(_, _, v)
            | Kind::DistanceY(_, _, v)
            | Kind::Length(_, v)
            | Kind::Radius(_, v)
            | Kind::Diameter(_, v)
            | Kind::Angle(_, _, v)
            | Kind::PointLineDistance(_, _, v) => Some(v),
            _ => None,
        }
    }
    pub fn is_dimension(&self) -> bool {
        self.clone().value_mut().is_some()
    }
    /// Ids referenciados (pontos ou curvas).
    pub fn refs(&self) -> Vec<Id> {
        match self {
            Kind::Horizontal(a) | Kind::Vertical(a) | Kind::Fixed(a) | Kind::Length(a, _) | Kind::Radius(a, _) | Kind::Diameter(a, _) => vec![*a],
            Kind::Coincident(a, b)
            | Kind::Parallel(a, b)
            | Kind::Perpendicular(a, b)
            | Kind::Equal(a, b)
            | Kind::Tangent(a, b)
            | Kind::Concentric(a, b)
            | Kind::Midpoint(a, b)
            | Kind::PointOnCurve(a, b)
            | Kind::HorizontalPts(a, b)
            | Kind::VerticalPts(a, b)
            | Kind::Distance(a, b, _)
            | Kind::DistanceX(a, b, _)
            | Kind::DistanceY(a, b, _)
            | Kind::Angle(a, b, _)
            | Kind::PointLineDistance(a, b, _) => vec![*a, *b],
            Kind::Symmetric(a, b, c) => vec![*a, *b, *c],
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Constraint {
    pub id: Id,
    pub kind: Kind,
    /// posição do texto da cota em relação à âncora (meio, centro ou vértice), no plano do sketch
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<[f64; 2]>,
}

/// Estado do sketch após resolver.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct SolveReport {
    pub converged: bool,
    pub residual: f64,
    /// graus de liberdade restantes
    pub dof: usize,
    /// pontos totalmente determinados
    pub fixed_points: HashSet<Id>,
    /// restrições que não foram satisfeitas (conflito)
    pub failing: Vec<Id>,
    /// restrições redundantes (não acrescentam informação)
    pub redundant: usize,
    pub errors: Vec<String>,
}

impl SolveReport {
    pub fn status(&self) -> &'static str {
        if !self.converged {
            "Conflito entre restrições"
        } else if self.dof == 0 {
            "Totalmente restrito"
        } else {
            "Sub-restrito"
        }
    }
}

/// Segmento de perfil resolvido (coordenadas do plano).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Line([f64; 2], [f64; 2]),
    Arc([f64; 2], [f64; 2], [f64; 2]),
    Circle([f64; 2], f64),
}

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct SketchData {
    pub points: Vec<SkPoint>,
    pub curves: Vec<SkCurve>,
    pub constraints: Vec<Constraint>,
    /// geometria externa trazida para o sketch (converter, projetar, interseção): pontos fixos
    /// que o cálculo reposiciona a cada vez — muda o 3D, muda o sketch
    #[serde(default)]
    pub links: Vec<SkLink>,
}

/// Origem de uma geometria externa do sketch.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum LinkSrc {
    /// ponto, linha/curva ou superfície de referência (inclusive importada por link)
    Ref(Id),
    /// curva de outro sketch
    SketchCurve { sketch: Id, curve: Id },
    /// aresta de um corpo (reencontrada pelo ponto médio)
    Edge { mid: [f64; 3] },
    /// vértice de um corpo
    Vertex { pos: [f64; 3] },
    /// face de um corpo (para a interseção com o plano do sketch)
    Face { point: [f64; 3] },
}

/// Como a geometria externa vira geometria do sketch.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum LinkShape {
    /// polilinha de pontos fixos (na ordem)
    Poly { points: Vec<Id>, closed: bool },
    /// círculo: centro fixo e raio da curva acompanhando
    Circle { center: Id, curve: Id },
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SkLink {
    pub id: Id,
    pub source: LinkSrc,
    /// interseção com o plano do sketch (senão: projeção ortogonal no plano)
    pub intersect: bool,
    pub shape: LinkShape,
}

/// Reamostra uma polilinha em `n` pontos igualmente espaçados pelo comprimento.
pub fn resample(pts: &[[f64; 2]], n: usize) -> Vec<[f64; 2]> {
    if pts.is_empty() || n == 0 {
        return vec![];
    }
    if n == 1 || pts.len() == 1 {
        return vec![pts[0]; n];
    }
    let mut acc = vec![0.];
    for w in pts.windows(2) {
        let l = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
        acc.push(acc.last().unwrap() + l);
    }
    let total = *acc.last().unwrap();
    (0..n)
        .map(|i| {
            let t = total * i as f64 / (n - 1) as f64;
            let k = acc.partition_point(|a| *a < t).clamp(1, pts.len() - 1);
            let (a0, a1) = (acc[k - 1], acc[k]);
            let u = if a1 > a0 { (t - a0) / (a1 - a0) } else { 0. };
            [pts[k - 1][0] + (pts[k][0] - pts[k - 1][0]) * u, pts[k - 1][1] + (pts[k][1] - pts[k - 1][1]) * u]
        })
        .collect()
}

/// Círculo que passa pelos pontos (centro, raio), se a polilinha fechada for um círculo.
pub fn fit_circle2(pts: &[[f64; 2]]) -> Option<([f64; 2], f64)> {
    let n = pts.len();
    if n < 8 {
        return None;
    }
    let closed = ((pts[0][0] - pts[n - 1][0]).powi(2) + (pts[0][1] - pts[n - 1][1]).powi(2)).sqrt() < 1e-4 * (1. + pts[0][0].abs());
    if !closed {
        return None;
    }
    let (a, b, c) = (pts[0], pts[n / 3], pts[2 * n / 3]);
    let d = 2. * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() < 1e-12 {
        return None;
    }
    let sq = |p: [f64; 2]| p[0] * p[0] + p[1] * p[1];
    let ux = (sq(a) * (b[1] - c[1]) + sq(b) * (c[1] - a[1]) + sq(c) * (a[1] - b[1])) / d;
    let uy = (sq(a) * (c[0] - b[0]) + sq(b) * (a[0] - c[0]) + sq(c) * (b[0] - a[0])) / d;
    let r = ((a[0] - ux).powi(2) + (a[1] - uy).powi(2)).sqrt();
    let ok = pts.iter().all(|p| (((p[0] - ux).powi(2) + (p[1] - uy).powi(2)).sqrt() - r).abs() < r * 0.01 + 1e-4);
    ok.then_some(([ux, uy], r))
}

impl SketchData {
    /// Põe a geometria externa no sketch: círculo (se a polilinha fechada for um) ou polilinha de
    /// pontos fixos ligados por linhas. `al` cria ids.
    pub fn add_link(&mut self, al: &mut impl FnMut() -> Id, source: LinkSrc, intersect: bool, uv: &[[f64; 2]], construction: bool) -> Option<Id> {
        if uv.is_empty() {
            return None;
        }
        let shape = if let Some((c, r)) = fit_circle2(uv) {
            let center = self.add_point(al, c[0], c[1]);
            if let Some(p) = self.point_mut(center) {
                p.fixed = true;
            }
            let curve = self.add_curve(al, Curve::Circle { c: center, r }, construction);
            LinkShape::Circle { center, curve }
        } else {
            let closed = uv.len() > 2 && ((uv[0][0] - uv[uv.len() - 1][0]).powi(2) + (uv[0][1] - uv[uv.len() - 1][1]).powi(2)).sqrt() < 1e-4;
            let open: Vec<[f64; 2]> = if closed { uv[..uv.len() - 1].to_vec() } else { uv.to_vec() };
            let ids: Vec<Id> = open
                .iter()
                .map(|q| {
                    let id = self.add_point(al, q[0], q[1]);
                    if let Some(p) = self.point_mut(id) {
                        p.fixed = true;
                    }
                    id
                })
                .collect();
            if ids.len() == 1 {
                self.add_curve(al, Curve::Point { p: ids[0] }, construction);
            }
            for w in ids.windows(2) {
                self.add_curve(al, Curve::Line { a: w[0], b: w[1] }, construction);
            }
            if closed && ids.len() > 2 {
                self.add_curve(al, Curve::Line { a: ids[ids.len() - 1], b: ids[0] }, construction);
            }
            LinkShape::Poly { points: ids, closed }
        };
        let id = al();
        self.links.push(SkLink { id, source, intersect, shape });
        Some(id)
    }

    /// Reposiciona a geometria externa com a polilinha nova (no plano do sketch).
    pub fn update_link(&mut self, link: Id, uv: &[[f64; 2]]) {
        let Some(l) = self.links.iter().find(|l| l.id == link).cloned() else { return };
        match l.shape {
            LinkShape::Circle { center, curve } => {
                let (c, r) = fit_circle2(uv).unwrap_or_else(|| {
                    let n = uv.len().max(1) as f64;
                    let c = [uv.iter().map(|p| p[0]).sum::<f64>() / n, uv.iter().map(|p| p[1]).sum::<f64>() / n];
                    let r = uv.iter().map(|p| ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2)).sqrt()).sum::<f64>() / n;
                    (c, r)
                });
                if let Some(p) = self.point_mut(center) {
                    (p.x, p.y) = (c[0], c[1]);
                }
                if let Some(cv) = self.curves.iter_mut().find(|x| x.id == curve) {
                    if let Curve::Circle { r: rr, .. } = &mut cv.geom {
                        *rr = r;
                    }
                }
            }
            LinkShape::Poly { points, closed } => {
                let n = points.len() + usize::from(closed);
                let q = resample(uv, n);
                for (id, p) in points.iter().zip(q) {
                    if let Some(pt) = self.point_mut(*id) {
                        (pt.x, pt.y) = (p[0], p[1]);
                    }
                }
            }
        }
    }
}

type Vars = HashMap<String, f64>;

// ---------------------------------------------------------------- consultas e construção

impl SketchData {
    pub fn point(&self, id: Id) -> Option<&SkPoint> {
        self.points.iter().find(|p| p.id == id)
    }
    pub fn point_mut(&mut self, id: Id) -> Option<&mut SkPoint> {
        self.points.iter_mut().find(|p| p.id == id)
    }
    pub fn curve(&self, id: Id) -> Option<&SkCurve> {
        self.curves.iter().find(|c| c.id == id)
    }
    pub fn pos(&self, id: Id) -> [f64; 2] {
        self.point(id).map(|p| [p.x, p.y]).unwrap_or([0., 0.])
    }

    /// Pontos que uma curva usa.
    pub fn curve_points(c: &Curve) -> Vec<Id> {
        match c {
            Curve::Line { a, b } => vec![*a, *b],
            Curve::Circle { c, .. } => vec![*c],
            Curve::Arc { c, s, e } => vec![*c, *s, *e],
            Curve::Point { p } => vec![*p],
        }
    }

    pub fn radius_of(&self, id: Id) -> Option<f64> {
        match &self.curve(id)?.geom {
            Curve::Circle { r, .. } => Some(*r),
            Curve::Arc { c, s, .. } => {
                let (c, s) = (self.pos(*c), self.pos(*s));
                Some(((s[0] - c[0]).powi(2) + (s[1] - c[1]).powi(2)).sqrt())
            }
            _ => None,
        }
    }

    pub fn add_point(&mut self, alloc: &mut impl FnMut() -> Id, x: f64, y: f64) -> Id {
        let id = alloc();
        self.points.push(SkPoint { id, x, y, fixed: false });
        id
    }
    pub fn add_curve(&mut self, alloc: &mut impl FnMut() -> Id, geom: Curve, construction: bool) -> Id {
        let id = alloc();
        self.curves.push(SkCurve { id, construction, geom });
        id
    }
    pub fn add_constraint(&mut self, alloc: &mut impl FnMut() -> Id, kind: Kind) -> Id {
        let id = alloc();
        self.constraints.push(Constraint { id, kind, label: None });
        id
    }

    /// Retângulo com cantos compartilhados e restrições horizontal/vertical. Devolve (pontos, linhas).
    pub fn add_rect(&mut self, al: &mut impl FnMut() -> Id, a: [f64; 2], b: [f64; 2], construction: bool) -> ([Id; 4], [Id; 4]) {
        let p = [
            self.add_point(al, a[0], a[1]),
            self.add_point(al, b[0], a[1]),
            self.add_point(al, b[0], b[1]),
            self.add_point(al, a[0], b[1]),
        ];
        let l = [0, 1, 2, 3].map(|i| self.add_curve(al, Curve::Line { a: p[i], b: p[(i + 1) % 4] }, construction));
        self.add_constraint(al, Kind::Horizontal(l[0]));
        self.add_constraint(al, Kind::Horizontal(l[2]));
        self.add_constraint(al, Kind::Vertical(l[1]));
        self.add_constraint(al, Kind::Vertical(l[3]));
        (p, l)
    }

    /// Rasgo (centro a centro): dois arcos e duas linhas tangentes. Devolve (centro1, centro2, arco).
    pub fn add_slot(&mut self, al: &mut impl FnMut() -> Id, c1: [f64; 2], c2: [f64; 2], r: f64) -> (Id, Id, Id) {
        let d = [c2[0] - c1[0], c2[1] - c1[1]];
        let len = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-9);
        let n = [-d[1] / len * r, d[0] / len * r];
        let pc1 = self.add_point(al, c1[0], c1[1]);
        let pc2 = self.add_point(al, c2[0], c2[1]);
        let a1 = self.add_point(al, c1[0] + n[0], c1[1] + n[1]);
        let b1 = self.add_point(al, c1[0] - n[0], c1[1] - n[1]);
        let a2 = self.add_point(al, c2[0] + n[0], c2[1] + n[1]);
        let b2 = self.add_point(al, c2[0] - n[0], c2[1] - n[1]);
        let l1 = self.add_curve(al, Curve::Line { a: b1, b: b2 }, false);
        let arc2 = self.add_curve(al, Curve::Arc { c: pc2, s: b2, e: a2 }, false);
        let l2 = self.add_curve(al, Curve::Line { a: a2, b: a1 }, false);
        let arc1 = self.add_curve(al, Curve::Arc { c: pc1, s: a1, e: b1 }, false);
        self.add_curve(al, Curve::Line { a: pc1, b: pc2 }, true);
        for (l, a) in [(l1, arc2), (l2, arc2), (l1, arc1), (l2, arc1)] {
            self.add_constraint(al, Kind::Tangent(l, a));
        }
        self.add_constraint(al, Kind::Equal(arc1, arc2));
        (pc1, pc2, arc1)
    }

    /// Polígono regular inscrito em círculo de construção.
    pub fn add_polygon(&mut self, al: &mut impl FnMut() -> Id, c: [f64; 2], r: f64, n: usize) -> Id {
        let n = n.max(3);
        let pc = self.add_point(al, c[0], c[1]);
        let circ = self.add_curve(al, Curve::Circle { c: pc, r }, true);
        let pts: Vec<Id> = (0..n)
            .map(|i| {
                let a = std::f64::consts::FRAC_PI_2 + 2. * std::f64::consts::PI * i as f64 / n as f64;
                self.add_point(al, c[0] + r * a.cos(), c[1] + r * a.sin())
            })
            .collect();
        let lines: Vec<Id> = (0..n).map(|i| self.add_curve(al, Curve::Line { a: pts[i], b: pts[(i + 1) % n] }, false)).collect();
        for p in &pts {
            self.add_constraint(al, Kind::PointOnCurve(*p, circ));
        }
        for i in 1..n {
            self.add_constraint(al, Kind::Equal(lines[0], lines[i]));
        }
        circ
    }

    /// Remove curva e pontos que só ela usava, além das restrições que ficam sem alvo.
    pub fn remove_curve(&mut self, id: Id) {
        let Some(c) = self.curve(id).cloned() else { return };
        self.curves.retain(|x| x.id != id);
        let used: HashSet<Id> = self.curves.iter().flat_map(|c| Self::curve_points(&c.geom)).collect();
        for p in Self::curve_points(&c.geom) {
            if !used.contains(&p) {
                self.points.retain(|x| x.id != p);
            }
        }
        self.prune_constraints();
    }

    pub fn prune_constraints(&mut self) {
        let ids: HashSet<Id> = self.points.iter().map(|p| p.id).chain(self.curves.iter().map(|c| c.id)).collect();
        self.constraints.retain(|c| c.kind.refs().iter().all(|r| ids.contains(r)));
    }

    /// Junta o ponto `from` em `into` (coincidência por compartilhamento).
    pub fn merge_points(&mut self, from: Id, into: Id) {
        if from == into {
            return;
        }
        let swap = |x: &mut Id| {
            if *x == from {
                *x = into
            }
        };
        for c in &mut self.curves {
            match &mut c.geom {
                Curve::Line { a, b } => {
                    swap(a);
                    swap(b)
                }
                Curve::Circle { c, .. } => swap(c),
                Curve::Arc { c, s, e } => {
                    swap(c);
                    swap(s);
                    swap(e)
                }
                Curve::Point { p } => swap(p),
            }
        }
        for k in &mut self.constraints {
            match &mut k.kind {
                Kind::Coincident(a, b)
                | Kind::Midpoint(a, b)
                | Kind::PointOnCurve(a, b)
                | Kind::HorizontalPts(a, b)
                | Kind::VerticalPts(a, b)
                | Kind::Distance(a, b, _)
                | Kind::DistanceX(a, b, _)
                | Kind::DistanceY(a, b, _)
                | Kind::PointLineDistance(a, b, _) => {
                    swap(a);
                    swap(b)
                }
                Kind::Fixed(a) => swap(a),
                Kind::Symmetric(a, b, _) => {
                    swap(a);
                    swap(b)
                }
                _ => {}
            }
        }
        self.points.retain(|p| p.id != from);
        // remove curvas degeneradas (linha com os dois extremos iguais)
        self.curves.retain(|c| !matches!(c.geom, Curve::Line { a, b } if a == b));
        self.prune_constraints();
    }

    /// Todas as expressões das cotas.
    pub fn exprs_mut(&mut self) -> impl Iterator<Item = &mut String> {
        self.constraints.iter_mut().filter_map(|c| c.kind.value_mut())
    }
}

// ---------------------------------------------------------------- solver

struct Sys<'a> {
    sk: &'a SketchData,
    /// índice da variável x de cada ponto livre
    pvar: HashMap<Id, usize>,
    /// índice da variável raio de cada círculo
    rvar: HashMap<Id, usize>,
    eqs: Vec<(Id, Eq)>,
}

#[derive(Clone)]
enum Eq {
    K(Kind, f64),
    ArcRadius(Id, Id, Id),
}

fn len2(v: [f64; 2]) -> f64 {
    (v[0] * v[0] + v[1] * v[1]).sqrt()
}
fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

impl Sys<'_> {
    fn p(&self, x: &[f64], id: Id) -> [f64; 2] {
        match self.pvar.get(&id) {
            Some(&i) => [x[i], x[i + 1]],
            None => self.sk.pos(id),
        }
    }
    fn radius(&self, x: &[f64], curve: Id) -> f64 {
        if let Some(&i) = self.rvar.get(&curve) {
            return x[i];
        }
        match self.sk.curve(curve).map(|c| &c.geom) {
            Some(Curve::Circle { r, .. }) => *r,
            Some(Curve::Arc { c, s, .. }) => len2(sub(self.p(x, *s), self.p(x, *c))),
            _ => 0.,
        }
    }
    fn center(&self, x: &[f64], curve: Id) -> Option<[f64; 2]> {
        match self.sk.curve(curve).map(|c| &c.geom) {
            Some(Curve::Circle { c, .. }) | Some(Curve::Arc { c, .. }) => Some(self.p(x, *c)),
            _ => None,
        }
    }
    fn line(&self, x: &[f64], curve: Id) -> Option<([f64; 2], [f64; 2])> {
        match self.sk.curve(curve).map(|c| &c.geom) {
            Some(Curve::Line { a, b }) => Some((self.p(x, *a), self.p(x, *b))),
            _ => None,
        }
    }

    fn residual(&self, x: &[f64], e: &Eq, out: &mut Vec<f64>) {
        let ln = |c| self.line(x, c);
        match e {
            Eq::ArcRadius(c, s, en) => {
                let (c, s, en) = (self.p(x, *c), self.p(x, *s), self.p(x, *en));
                out.push(len2(sub(s, c)) - len2(sub(en, c)));
            }
            Eq::K(k, v) => match k {
                Kind::Coincident(a, b) | Kind::Concentric(a, b) => {
                    let (pa, pb) = match k {
                        Kind::Concentric(..) => (self.center(x, *a).unwrap_or_default(), self.center(x, *b).unwrap_or_default()),
                        _ => (self.p(x, *a), self.p(x, *b)),
                    };
                    out.push(pa[0] - pb[0]);
                    out.push(pa[1] - pb[1]);
                }
                Kind::Horizontal(l) => {
                    if let Some((a, b)) = ln(*l) {
                        out.push(a[1] - b[1])
                    }
                }
                Kind::Vertical(l) => {
                    if let Some((a, b)) = ln(*l) {
                        out.push(a[0] - b[0])
                    }
                }
                Kind::HorizontalPts(a, b) => out.push(self.p(x, *a)[1] - self.p(x, *b)[1]),
                Kind::VerticalPts(a, b) => out.push(self.p(x, *a)[0] - self.p(x, *b)[0]),
                Kind::Parallel(l1, l2) | Kind::Perpendicular(l1, l2) => {
                    if let (Some((a, b)), Some((c, d))) = (ln(*l1), ln(*l2)) {
                        let (u, w) = (sub(b, a), sub(d, c));
                        let n = (len2(u) * len2(w)).max(1e-12);
                        out.push(if matches!(k, Kind::Parallel(..)) { cross(u, w) / n } else { dot(u, w) / n });
                    }
                }
                Kind::Equal(c1, c2) => match (ln(*c1), ln(*c2)) {
                    (Some((a, b)), Some((c, d))) => out.push(len2(sub(b, a)) - len2(sub(d, c))),
                    _ => out.push(self.radius(x, *c1) - self.radius(x, *c2)),
                },
                Kind::Tangent(c1, c2) => {
                    let (l, c) = if ln(*c1).is_some() { (*c1, *c2) } else { (*c2, *c1) };
                    // extremo compartilhado entre a linha e o arco: direção da linha ⟂ raio nesse ponto
                    // (forma bem condicionada; a distância centro-linha fica degenerada no ponto de tangência)
                    let shared = match (self.sk.curve(l).map(|k| &k.geom), self.sk.curve(c).map(|k| &k.geom)) {
                        (Some(Curve::Line { a, b }), Some(Curve::Arc { s, e, .. })) => [*a, *b].into_iter().find(|p| p == s || p == e),
                        _ => None,
                    };
                    if let (Some(p), Some((a, b)), Some(ctr)) = (shared, ln(l), self.center(x, c)) {
                        let (d, rr) = (sub(b, a), sub(self.p(x, p), ctr));
                        out.push(dot(d, rr) / (len2(d) * len2(rr)).max(1e-12));
                        return;
                    }
                    match (ln(l), self.center(x, c)) {
                        (Some((a, b)), Some(ctr)) => {
                            let d = sub(b, a);
                            out.push(cross(d, sub(ctr, a)).abs() / len2(d).max(1e-12) - self.radius(x, c));
                        }
                        _ => {
                            // círculo–círculo (tangência externa)
                            if let (Some(p), Some(q)) = (self.center(x, *c1), self.center(x, *c2)) {
                                out.push(len2(sub(p, q)) - self.radius(x, *c1) - self.radius(x, *c2));
                            }
                        }
                    }
                }
                Kind::Midpoint(p, l) => {
                    if let Some((a, b)) = ln(*l) {
                        let q = self.p(x, *p);
                        out.push(q[0] - (a[0] + b[0]) / 2.);
                        out.push(q[1] - (a[1] + b[1]) / 2.);
                    }
                }
                Kind::PointOnCurve(p, c) => {
                    let q = self.p(x, *p);
                    match (ln(*c), self.center(x, *c)) {
                        (Some((a, b)), _) => {
                            let d = sub(b, a);
                            out.push(cross(d, sub(q, a)) / len2(d).max(1e-12));
                        }
                        (None, Some(ctr)) => out.push(len2(sub(q, ctr)) - self.radius(x, *c)),
                        _ => {}
                    }
                }
                Kind::Fixed(_) => {}
                Kind::Symmetric(p, q, l) => {
                    if let Some((a, b)) = ln(*l) {
                        let (pp, qq) = (self.p(x, *p), self.p(x, *q));
                        let m = [(pp[0] + qq[0]) / 2., (pp[1] + qq[1]) / 2.];
                        let d = sub(b, a);
                        let n = len2(d).max(1e-12);
                        out.push(cross(d, sub(m, a)) / n);
                        out.push(dot(d, sub(qq, pp)) / n);
                    }
                }
                Kind::Distance(a, b, _) => out.push(len2(sub(self.p(x, *b), self.p(x, *a))) - v),
                Kind::DistanceX(a, b, _) => out.push((self.p(x, *b)[0] - self.p(x, *a)[0]).abs() - v),
                Kind::DistanceY(a, b, _) => out.push((self.p(x, *b)[1] - self.p(x, *a)[1]).abs() - v),
                Kind::Length(l, _) => {
                    if let Some((a, b)) = ln(*l) {
                        out.push(len2(sub(b, a)) - v)
                    }
                }
                Kind::Radius(c, _) => out.push(self.radius(x, *c) - v),
                Kind::Diameter(c, _) => out.push(2. * self.radius(x, *c) - v),
                Kind::Angle(l1, l2, _) => {
                    if let (Some((a, b)), Some((c, d))) = (ln(*l1), ln(*l2)) {
                        let (u, w) = (sub(b, a), sub(d, c));
                        let n = (len2(u) * len2(w)).max(1e-12);
                        let t = v.to_radians();
                        // zero quando o ângulo (sem sinal) entre as direções vale v
                        out.push((cross(u, w).abs() * t.cos() - dot(u, w) * t.sin()) / n);
                    }
                }
                Kind::PointLineDistance(p, l, _) => {
                    if let Some((a, b)) = ln(*l) {
                        let d = sub(b, a);
                        out.push(cross(d, sub(self.p(x, *p), a)).abs() / len2(d).max(1e-12) - v);
                    }
                }
            },
        }
    }

    fn all(&self, x: &[f64]) -> Vec<f64> {
        let mut r = vec![];
        for (_, e) in &self.eqs {
            self.residual(x, e, &mut r);
        }
        r
    }

    fn jacobian(&self, x: &[f64], m: usize) -> Vec<Vec<f64>> {
        let n = x.len();
        let mut j = vec![vec![0.; n]; m];
        let mut xp = x.to_vec();
        for c in 0..n {
            let h = 1e-7 * (1. + x[c].abs());
            xp[c] = x[c] + h;
            let rp = self.all(&xp);
            xp[c] = x[c] - h;
            let rm = self.all(&xp);
            xp[c] = x[c];
            for i in 0..m.min(rp.len()).min(rm.len()) {
                j[i][c] = (rp[i] - rm[i]) / (2. * h);
            }
        }
        j
    }
}

/// Resolve (A + λI) δ = b por eliminação de Gauss com pivotamento parcial.
fn solve_dense(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let piv = (col..n).max_by(|&i, &k| a[i][col].abs().total_cmp(&a[k][col].abs()))?;
        if a[piv][col].abs() < 1e-14 {
            return None;
        }
        a.swap(col, piv);
        b.swap(col, piv);
        for row in col + 1..n {
            let f = a[row][col] / a[col][col];
            if f != 0. {
                for k in col..n {
                    a[row][k] -= f * a[col][k];
                }
                b[row] -= f * b[col];
            }
        }
    }
    let mut x = vec![0.; n];
    for i in (0..n).rev() {
        let s: f64 = (i + 1..n).map(|k| a[i][k] * x[k]).sum();
        x[i] = (b[i] - s) / a[i][i];
    }
    Some(x)
}

/// Forma escalonada reduzida; devolve (colunas pivô, matriz reduzida).
fn rref(mut m: Vec<Vec<f64>>, cols: usize, tol: f64) -> (Vec<usize>, Vec<Vec<f64>>) {
    let rows = m.len();
    let mut pivots = vec![];
    let mut r = 0;
    for c in 0..cols {
        if r >= rows {
            break;
        }
        let Some(piv) = (r..rows).max_by(|&i, &k| m[i][c].abs().total_cmp(&m[k][c].abs())) else { break };
        if m[piv][c].abs() < tol {
            continue;
        }
        m.swap(r, piv);
        let d = m[r][c];
        for k in 0..cols {
            m[r][k] /= d;
        }
        for i in 0..rows {
            if i != r {
                let f = m[i][c];
                if f.abs() > 0. {
                    for k in 0..cols {
                        m[i][k] -= f * m[r][k];
                    }
                }
            }
        }
        pivots.push(c);
        r += 1;
    }
    (pivots, m)
}

impl SketchData {
    /// Avalia as cotas (expressões) com os parâmetros.
    fn values(&self, vars: &Vars, errors: &mut Vec<String>) -> HashMap<Id, f64> {
        let mut out = HashMap::new();
        for c in &self.constraints {
            if let Some(v) = c.kind.clone().value_mut() {
                match expr::eval_str(v, vars) {
                    Ok(x) => {
                        out.insert(c.id, x);
                    }
                    Err(e) => errors.push(format!("{} = '{v}': {e}", c.kind.label())),
                }
            }
        }
        out
    }

    /// Resolve o sketch no lugar. `pinned`: pontos mantidos fixos (ex.: o ponto sendo arrastado).
    pub fn solve(&mut self, vars: &Vars, pinned: &[Id]) -> SolveReport {
        let mut rep = SolveReport::default();
        let vals = self.values(vars, &mut rep.errors);

        let fixed: HashSet<Id> = self
            .points
            .iter()
            .filter(|p| p.fixed)
            .map(|p| p.id)
            .chain(self.constraints.iter().filter_map(|c| if let Kind::Fixed(p) = &c.kind { Some(*p) } else { None }))
            .chain(pinned.iter().copied())
            .collect();

        let mut x0 = vec![];
        let mut pvar = HashMap::new();
        for p in &self.points {
            if !fixed.contains(&p.id) {
                pvar.insert(p.id, x0.len());
                x0.push(p.x);
                x0.push(p.y);
            }
        }
        let mut rvar = HashMap::new();
        for c in &self.curves {
            if let Curve::Circle { r, .. } = c.geom {
                rvar.insert(c.id, x0.len());
                x0.push(r);
            }
        }
        let mut eqs: Vec<(Id, Eq)> = vec![];
        for c in &self.constraints {
            if c.kind.is_dimension() && !vals.contains_key(&c.id) {
                continue; // cota com expressão inválida
            }
            eqs.push((c.id, Eq::K(c.kind.clone(), vals.get(&c.id).copied().unwrap_or(0.))));
        }
        for c in &self.curves {
            if let Curve::Arc { c: ctr, s, e } = c.geom {
                eqs.push((c.id, Eq::ArcRadius(ctr, s, e)));
            }
        }

        let sys = Sys { sk: self, pvar: pvar.clone(), rvar: rvar.clone(), eqs };
        let n = x0.len();
        let mut x = x0.clone();
        let mut r = sys.all(&x);
        let m = r.len();
        let norm = |r: &[f64]| r.iter().map(|v| v * v).sum::<f64>();
        let mut f = norm(&r);
        let mut lambda = 1e-3;
        if n > 0 && m > 0 {
            for _ in 0..100 {
                if f < 1e-20 {
                    break;
                }
                let j = sys.jacobian(&x, m);
                // (JᵀJ + λ diag) δ = −Jᵀ r, com um pequeno termo que prende ao ponto atual
                let mut a = vec![vec![0.; n]; n];
                let mut g = vec![0.; n];
                for i in 0..m {
                    for c in 0..n {
                        if j[i][c] == 0. {
                            continue;
                        }
                        g[c] -= j[i][c] * r[i];
                        for k in 0..n {
                            a[c][k] += j[i][c] * j[i][k];
                        }
                    }
                }
                let mut improved = false;
                for _ in 0..12 {
                    let mut al = a.clone();
                    for c in 0..n {
                        al[c][c] += lambda * (1. + a[c][c]) + 1e-12;
                    }
                    let Some(d) = solve_dense(al, g.clone()) else { break };
                    let xn: Vec<f64> = x.iter().zip(&d).map(|(a, b)| a + b).collect();
                    let rn = sys.all(&xn);
                    let fnew = norm(&rn);
                    if fnew < f {
                        x = xn;
                        r = rn;
                        f = fnew;
                        lambda = (lambda * 0.3).max(1e-12);
                        improved = true;
                        break;
                    }
                    lambda *= 10.;
                }
                if !improved {
                    break;
                }
            }
        }
        rep.residual = f.sqrt();
        rep.converged = rep.residual < 1e-6;

        // restrições que falharam
        if !rep.converged {
            for (cid, e) in &sys.eqs {
                let mut rr = vec![];
                sys.residual(&x, e, &mut rr);
                if rr.iter().any(|v| v.abs() > 1e-5) {
                    rep.failing.push(*cid);
                }
            }
        }

        // graus de liberdade e pontos determinados
        if n > 0 {
            let j = sys.jacobian(&x, m);
            let (piv, red) = rref(j, n, 1e-7);
            rep.dof = n - piv.len();
            rep.redundant = m.saturating_sub(piv.len());
            let free: Vec<usize> = (0..n).filter(|c| !piv.contains(c)).collect();
            let mut determined = vec![false; n];
            for (row, &pc) in piv.iter().enumerate() {
                determined[pc] = free.iter().all(|&fc| red[row][fc].abs() < 1e-7);
            }
            for (id, &i) in &pvar {
                if determined[i] && determined[i + 1] {
                    rep.fixed_points.insert(*id);
                }
            }
        } else {
            rep.dof = 0;
        }
        rep.fixed_points.extend(fixed.iter().copied());

        // aplica a solução (mesmo parcial: fica o melhor ajuste)
        let pv: Vec<(Id, usize)> = pvar.into_iter().collect();
        let rv: Vec<(Id, usize)> = rvar.into_iter().collect();
        for (id, i) in pv {
            if let Some(p) = self.point_mut(id) {
                p.x = x[i];
                p.y = x[i + 1];
            }
        }
        for (id, i) in rv {
            if let Some(SkCurve { geom: Curve::Circle { r, .. }, .. }) = self.curves.iter_mut().find(|c| c.id == id) {
                *r = x[i].abs();
            }
        }
        rep
    }

    // ------------------------------------------------------------ perfis

    /// Ponto médio (no sentido anti-horário) de um arco.
    pub fn arc_mid(c: [f64; 2], s: [f64; 2], e: [f64; 2]) -> [f64; 2] {
        let r = len2(sub(s, c));
        let a0 = (s[1] - c[1]).atan2(s[0] - c[0]);
        let mut a1 = (e[1] - c[1]).atan2(e[0] - c[0]);
        while a1 <= a0 {
            a1 += 2. * std::f64::consts::PI;
        }
        let m = (a0 + a1) / 2.;
        [c[0] + r * m.cos(), c[1] + r * m.sin()]
    }

    /// Laços fechados (perfis) formados pelas curvas que não são de construção.
    /// Devolve (laços, quantidade de curvas abertas).
    pub fn loops(&self) -> (Vec<Vec<Seg>>, usize) {
        // une pontos ligados por restrição de coincidência
        let mut parent: HashMap<Id, Id> = HashMap::new();
        fn find(p: &mut HashMap<Id, Id>, x: Id) -> Id {
            let mut r = x;
            while let Some(&q) = p.get(&r) {
                if q == r {
                    break;
                }
                r = q;
            }
            r
        }
        for c in &self.constraints {
            if let Kind::Coincident(a, b) = c.kind {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                if ra != rb {
                    parent.insert(ra, rb);
                }
            }
        }
        let mut loops = vec![];
        // arestas: (extremo1, extremo2, segmento)
        let mut edges: Vec<(Id, Id, Seg)> = vec![];
        for c in self.curves.iter().filter(|c| !c.construction) {
            match c.geom {
                Curve::Circle { c: ctr, r } => loops.push(vec![Seg::Circle(self.pos(ctr), r)]),
                Curve::Line { a, b } => edges.push((find(&mut parent, a), find(&mut parent, b), Seg::Line(self.pos(a), self.pos(b)))),
                Curve::Arc { c: ctr, s, e } => {
                    let (pc, ps, pe) = (self.pos(ctr), self.pos(s), self.pos(e));
                    edges.push((find(&mut parent, s), find(&mut parent, e), Seg::Arc(ps, Self::arc_mid(pc, ps, pe), pe)));
                }
                Curve::Point { .. } => {}
            }
        }
        let rev = |s: Seg| match s {
            Seg::Line(a, b) => Seg::Line(b, a),
            Seg::Arc(a, m, b) => Seg::Arc(b, m, a),
            c => c,
        };
        let mut open = 0;
        while let Some((s, e, seg)) = edges.pop() {
            let mut lp = vec![seg];
            let mut end = e;
            let mut closed = s == e;
            while !closed {
                let Some(i) = edges.iter().position(|(a, b, _)| *a == end || *b == end) else { break };
                let (a, b, sg) = edges.swap_remove(i);
                if a == end {
                    lp.push(sg);
                    end = b;
                } else {
                    lp.push(rev(sg));
                    end = a;
                }
                closed = end == s;
            }
            if closed {
                loops.push(lp);
            } else {
                open += lp.len();
            }
        }
        (loops, open)
    }

    /// Pontos isolados (curvas `Point` que não são de construção): centros de furos.
    pub fn hole_points(&self) -> Vec<[f64; 2]> {
        self.curves
            .iter()
            .filter(|c| !c.construction)
            .filter_map(|c| if let Curve::Point { p } = c.geom { Some(self.pos(p)) } else { None })
            .collect()
    }

    /// Medida atual de uma cota (para criar a cota com o valor existente).
    pub fn measure(&self, k: &Kind) -> Option<f64> {
        let x: Vec<f64> = vec![];
        let sys = Sys { sk: self, pvar: HashMap::new(), rvar: HashMap::new(), eqs: vec![] };
        let mut r = vec![];
        let probe = match k {
            Kind::Distance(a, b, _) => Kind::Distance(*a, *b, String::new()),
            Kind::DistanceX(a, b, _) => Kind::DistanceX(*a, *b, String::new()),
            Kind::DistanceY(a, b, _) => Kind::DistanceY(*a, *b, String::new()),
            Kind::Length(l, _) => Kind::Length(*l, String::new()),
            Kind::Radius(c, _) => Kind::Radius(*c, String::new()),
            Kind::Diameter(c, _) => Kind::Diameter(*c, String::new()),
            Kind::PointLineDistance(p, l, _) => Kind::PointLineDistance(*p, *l, String::new()),
            Kind::Angle(l1, l2, _) => {
                let (a, b) = sys.line(&x, *l1)?;
                let (c, d) = sys.line(&x, *l2)?;
                let (u, w) = (sub(b, a), sub(d, c));
                return Some(cross(u, w).abs().atan2(dot(u, w)).to_degrees());
            }
            _ => return None,
        };
        sys.residual(&x, &Eq::K(probe, 0.), &mut r);
        r.first().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alloc() -> impl FnMut() -> Id {
        let mut n = 100;
        move || {
            n += 1;
            Id(n)
        }
    }

    #[test]
    fn retangulo_com_cotas_fica_totalmente_restrito() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let (p, l) = s.add_rect(&mut al, [1., 2.], [30., 17.], false);
        s.point_mut(p[0]).unwrap().fixed = true;
        s.add_constraint(&mut al, Kind::Length(l[0], "L".into()));
        s.add_constraint(&mut al, Kind::Length(l[1], "L/2".into()));
        let mut v = Vars::new();
        v.insert("L".into(), 40.);
        let r = s.solve(&v, &[]);
        assert!(r.converged, "{r:?}");
        assert_eq!(r.dof, 0);
        assert_eq!(r.fixed_points.len(), 4);
        let c = s.pos(p[2]);
        assert!((c[0] - 41.).abs() < 1e-6 && (c[1] - 22.).abs() < 1e-6, "{c:?}");
        let (loops, open) = s.loops();
        assert_eq!((loops.len(), open), (1, 0));
    }

    #[test]
    fn sub_restrito_e_conflito() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let (_, l) = s.add_rect(&mut al, [0., 0.], [10., 5.], false);
        let r = s.solve(&Vars::new(), &[]);
        assert!(r.converged);
        assert_eq!(r.dof, 4); // posição (2) + largura + altura
        s.add_constraint(&mut al, Kind::Length(l[0], "10".into()));
        s.add_constraint(&mut al, Kind::Length(l[2], "20".into())); // lados opostos diferentes: conflito
        let r = s.solve(&Vars::new(), &[]);
        assert!(!r.converged);
        assert!(!r.failing.is_empty());
    }

    #[test]
    fn rasgo_tangente_e_circulo() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let (c1, c2, arc) = s.add_slot(&mut al, [0., 0.], [30., 0.], 5.);
        s.add_constraint(&mut al, Kind::Fixed(c1));
        s.add_constraint(&mut al, Kind::HorizontalPts(c1, c2));
        s.add_constraint(&mut al, Kind::Distance(c1, c2, "40".into()));
        s.add_constraint(&mut al, Kind::Radius(arc, "6".into()));
        let r = s.solve(&Vars::new(), &[]);
        assert!(r.converged, "{r:?}");
        assert!((s.pos(c2)[0].abs() - 40.).abs() < 1e-6);
        assert_eq!(r.dof, 0, "rasgo cotado deve ficar totalmente restrito");
        let (loops, open) = s.loops();
        assert_eq!((loops.len(), open), (1, 0));
        assert_eq!(loops[0].len(), 4);
    }

    #[test]
    fn arrastar_ponto_preso_mantem_restricoes() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let (p, _) = s.add_rect(&mut al, [0., 0.], [10., 5.], false);
        s.point_mut(p[2]).unwrap().x = 20.;
        s.point_mut(p[2]).unwrap().y = 9.;
        let r = s.solve(&Vars::new(), &[p[2]]);
        assert!(r.converged);
        assert!((s.pos(p[1])[0] - 20.).abs() < 1e-6 && (s.pos(p[3])[1] - 9.).abs() < 1e-6);
    }

    #[test]
    fn juntar_pontos_fecha_perfil() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let a = s.add_point(&mut al, 0., 0.);
        let b = s.add_point(&mut al, 10., 0.);
        let b2 = s.add_point(&mut al, 10., 0.);
        let c = s.add_point(&mut al, 0., 10.);
        let a2 = s.add_point(&mut al, 0., 0.);
        s.add_curve(&mut al, Curve::Line { a, b }, false);
        s.add_curve(&mut al, Curve::Line { a: b2, b: c }, false);
        s.add_curve(&mut al, Curve::Line { a: c, b: a2 }, false);
        assert_eq!(s.loops().0.len(), 0);
        s.merge_points(b2, b);
        s.merge_points(a2, a);
        assert_eq!(s.loops(), (s.loops().0.clone(), 0));
        assert_eq!(s.loops().0.len(), 1);
    }

    #[test]
    fn medir_cotas() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let (p, l) = s.add_rect(&mut al, [0., 0.], [30., 40.], false);
        assert!((s.measure(&Kind::Distance(p[0], p[2], String::new())).unwrap() - 50.).abs() < 1e-9);
        assert!((s.measure(&Kind::Angle(l[0], l[1], String::new())).unwrap() - 90.).abs() < 1e-9);
    }
}
