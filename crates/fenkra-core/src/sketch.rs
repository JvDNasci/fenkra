// SPDX-License-Identifier: LGPL-2.1-or-later
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
use std::time::Instant;

/// Lados que o polígono regular aceita.
pub const POLIGONO_LADOS_MIN: usize = 3;
pub const POLIGONO_LADOS_MAX: usize = 40;

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
    /// Spline que passa pelos pontos, na ordem (B-spline interpolada no perfil).
    Spline { pts: Vec<Id> },
    /// Elipse exata trazida de fora do sketch (`crate::projecao`). A forma vem da origem e o solver
    /// não a muda. `major`: vetor do centro à ponta do semieixo maior. De `t0` a `t1`, anti-horário;
    /// `s` e `e` são as pontas, e na elipse inteira os dois são o próprio centro.
    Ellipse { c: Id, s: Id, e: Id, major: [f64; 2], minor: f64, t0: f64, t1: f64 },
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
    /// o suplemento do ângulo entre as direções: o outro quadrante formado pelas duas linhas
    AngleSupp(Id, Id, String),
    PointLineDistance(Id, Id, String),
    /// ângulo da linha, de `a` para `b`, com a horizontal do sketch (com sinal)
    LineAngle(Id, String),
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
            Kind::Angle(..) | Kind::AngleSupp(..) => "Ângulo",
            Kind::PointLineDistance(..) => "Distância ponto-linha",
            Kind::LineAngle(..) => "Ângulo com a horizontal",
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
            | Kind::AngleSupp(_, _, v)
            | Kind::PointLineDistance(_, _, v)
            | Kind::LineAngle(_, v) => Some(v),
            _ => None,
        }
    }
    pub fn is_dimension(&self) -> bool {
        self.clone().value_mut().is_some()
    }
    /// Cotas em que um valor negativo significa "do outro lado".
    fn has_side(&self) -> bool {
        matches!(self, Kind::Distance(..) | Kind::DistanceX(..) | Kind::DistanceY(..) | Kind::Length(..) | Kind::Angle(..) | Kind::AngleSupp(..) | Kind::PointLineDistance(..))
    }
    /// Ids referenciados (pontos ou curvas).
    pub fn refs(&self) -> Vec<Id> {
        match self {
            Kind::Horizontal(a) | Kind::Vertical(a) | Kind::Fixed(a) | Kind::Length(a, _) | Kind::Radius(a, _) | Kind::Diameter(a, _) | Kind::LineAngle(a, _) => vec![*a],
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
            | Kind::AngleSupp(a, b, _)
            | Kind::PointLineDistance(a, b, _) => vec![*a, *b],
            Kind::Symmetric(a, b, c) => vec![*a, *b, *c],
        }
    }
    fn refs_mut(&mut self) -> Vec<&mut Id> {
        match self {
            Kind::Horizontal(a) | Kind::Vertical(a) | Kind::Fixed(a) | Kind::Length(a, _) | Kind::Radius(a, _) | Kind::Diameter(a, _) | Kind::LineAngle(a, _) => vec![a],
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
            | Kind::AngleSupp(a, b, _)
            | Kind::PointLineDistance(a, b, _) => vec![a, b],
            Kind::Symmetric(a, b, c) => vec![a, b, c],
        }
    }
    /// A mesma restrição com os alvos trocados por `f`; `None` se algum alvo não tem correspondente.
    pub fn remapeada(&self, f: impl Fn(Id) -> Option<Id>) -> Option<Kind> {
        let mut k = self.clone();
        for r in k.refs_mut() {
            *r = f(*r)?;
        }
        Some(k)
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Constraint {
    pub id: Id,
    pub kind: Kind,
    /// posição do texto da cota em relação à âncora (meio, centro ou vértice), no plano do sketch
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<[f64; 2]>,
    /// cota de referência: mostra a medida e não comanda a geometria
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reference: bool,
    /// a expressão da cota vale menos que zero e a geometria já está do outro lado
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub flipped: bool,
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
    /// como a geometria ainda pode se mover sem violar as restrições
    pub liberdade: Liberdade,
    /// `solve_ate`: o prazo acabou antes do fim e o sketch ficou como estava
    pub estourou: bool,
}

/// Movimentos que as restrições ainda deixam: o espaço nulo do Jacobiano no ponto resolvido. Um vetor
/// desse espaço é uma velocidade dos pontos que, em primeira ordem, não muda nenhuma equação.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Liberdade {
    /// direções unitárias de cada ponto: uma se ele só desliza, duas se anda no plano. Ponto
    /// determinado não aparece.
    pub pontos: HashMap<Id, Vec<[f64; 2]>>,
    /// círculos cujo raio ainda pode mudar
    pub raios: HashSet<Id>,
}

impl SolveReport {
    pub fn status(&self) -> &'static str {
        if !self.converged {
            "Conflito entre restrições"
        } else if self.redundant > 0 {
            "Redundante"
        } else if self.dof == 0 {
            "Totalmente restrito"
        } else {
            "Sub-restrito"
        }
    }
}

/// Segmento de perfil resolvido (coordenadas do plano).
#[derive(Clone, Debug, PartialEq)]
pub enum Seg {
    Line([f64; 2], [f64; 2]),
    Arc([f64; 2], [f64; 2], [f64; 2]),
    Circle([f64; 2], f64),
    /// spline pelos pontos (na ordem)
    Spline(Vec<[f64; 2]>),
    /// centro, semieixo maior, semieixo menor, parâmetros inicial e final (`t0 > t1`: percurso invertido)
    Ellipse([f64; 2], [f64; 2], f64, f64, f64),
}

/// Divide uma polilinha nos cantos vivos (giro > ~26°): cada trecho é uma lista de índices em
/// `p` (o último de um é o primeiro do seguinte). Fechada sem cantos vira duas metades.
/// Trechos com 4+ pontos são suaves (viram spline); os outros, retas.
pub fn smooth_runs(p: &[[f64; 2]], closed: bool) -> Vec<Vec<usize>> {
    let n = p.len();
    if n < 2 {
        return vec![];
    }
    let turn = |i: usize| {
        let (a, b, c) = (p[(i + n - 1) % n], p[i], p[(i + 1) % n]);
        let (u, v) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
        let (lu, lv) = ((u[0] * u[0] + u[1] * u[1]).sqrt(), (v[0] * v[0] + v[1] * v[1]).sqrt());
        if lu < 1e-12 || lv < 1e-12 {
            return 0.;
        }
        ((u[0] * v[0] + u[1] * v[1]) / (lu * lv)).clamp(-1., 1.).acos()
    };
    let lim = 0.45;
    if !closed {
        let mut br = vec![0];
        br.extend((1..n - 1).filter(|i| turn(*i) > lim));
        br.push(n - 1);
        return br.windows(2).map(|w| (w[0]..=w[1]).collect()).collect();
    }
    let mut br: Vec<usize> = (0..n).filter(|i| turn(*i) > lim).collect();
    if br.is_empty() {
        br = vec![0, n / 2];
    }
    (0..br.len())
        .map(|k| {
            let (s, e) = (br[k], br[(k + 1) % br.len()]);
            let mut v = vec![s];
            let mut i = s;
            loop {
                i = (i + 1) % n;
                v.push(i);
                if i == e {
                    break;
                }
            }
            v
        })
        .collect()
}

/// Amostra uma spline que passa pelos pontos (Catmull-Rom centrípeta), para desenhar.
pub fn spline_samples(p: &[[f64; 2]], per_seg: usize) -> Vec<[f64; 2]> {
    let n = p.len();
    if n < 3 {
        return p.to_vec();
    }
    let get = |i: isize| -> [f64; 2] {
        if i < 0 {
            // ponto fantasma refletido nas pontas
            [2. * p[0][0] - p[1][0], 2. * p[0][1] - p[1][1]]
        } else if i as usize >= n {
            [2. * p[n - 1][0] - p[n - 2][0], 2. * p[n - 1][1] - p[n - 2][1]]
        } else {
            p[i as usize]
        }
    };
    let mut out = vec![p[0]];
    for i in 0..n - 1 {
        let (p0, p1, p2, p3) = (get(i as isize - 1), get(i as isize), get(i as isize + 1), get(i as isize + 2));
        let tj = |a: [f64; 2], b: [f64; 2], t: f64| t + ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt().sqrt().max(1e-9);
        let t0 = 0.;
        let t1 = tj(p0, p1, t0);
        let t2 = tj(p1, p2, t1);
        let t3 = tj(p2, p3, t2);
        for k in 1..=per_seg {
            let t = t1 + (t2 - t1) * k as f64 / per_seg as f64;
            let l = |a: [f64; 2], b: [f64; 2], ta: f64, tb: f64| {
                let w = (t - ta) / (tb - ta);
                [a[0] + (b[0] - a[0]) * w, a[1] + (b[1] - a[1]) * w]
            };
            let a1 = l(p0, p1, t0, t1);
            let a2 = l(p1, p2, t1, t2);
            let a3 = l(p2, p3, t2, t3);
            let b1 = l(a1, a2, t0, t2);
            let b2 = l(a2, a3, t1, t3);
            out.push(l(b1, b2, t1, t2));
        }
    }
    out
}

/// Tangente própria num ponto de spline (alça tipo Bézier: o ponto ± vetor/3).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SplineTan {
    pub curve: Id,
    pub point: Id,
    /// vetor tangente (derivada no ponto, em mm por segmento)
    pub dir: [f64; 2],
}

/// Tangentes automáticas (Catmull-Rom) ou as dadas, em cada ponto.
pub fn spline_tangents(p: &[[f64; 2]], own: &[Option<[f64; 2]>]) -> Vec<[f64; 2]> {
    let n = p.len();
    (0..n)
        .map(|i| {
            if let Some(Some(t)) = own.get(i) {
                return *t;
            }
            if n < 2 {
                return [0., 0.];
            }
            let (a, b) = if i == 0 {
                (p[0], p[1])
            } else if i == n - 1 {
                (p[n - 2], p[n - 1])
            } else {
                (p[i - 1], p[i + 1])
            };
            let k = if i == 0 || i == n - 1 { 1. } else { 0.5 };
            [(b[0] - a[0]) * k, (b[1] - a[1]) * k]
        })
        .collect()
}

/// Curva de Hermite cúbica pelos pontos com as tangentes (amostrada).
pub fn spline_eval(p: &[[f64; 2]], own: &[Option<[f64; 2]>], per_seg: usize) -> Vec<[f64; 2]> {
    let n = p.len();
    if n < 2 {
        return p.to_vec();
    }
    let m = spline_tangents(p, own);
    let mut out = vec![p[0]];
    for i in 0..n - 1 {
        for k in 1..=per_seg {
            let t = k as f64 / per_seg as f64;
            let (t2, t3) = (t * t, t * t * t);
            let h00 = 2. * t3 - 3. * t2 + 1.;
            let h10 = t3 - 2. * t2 + t;
            let h01 = -2. * t3 + 3. * t2;
            let h11 = t3 - t2;
            out.push([0, 1].map(|j| h00 * p[i][j] + h10 * m[i][j] + h01 * p[i + 1][j] + h11 * m[i + 1][j]));
        }
    }
    out
}

/// Ponto da curva de Hermite no parâmetro `u` (0 no primeiro ponto, 1 no segundo, e assim por diante).
fn hermite_at(p: &[[f64; 2]], m: &[[f64; 2]], u: f64) -> [f64; 2] {
    let i = (u.max(0.) as usize).min(p.len() - 2);
    let t = u - i as f64;
    let (t2, t3) = (t * t, t * t * t);
    let (h00, h10, h01, h11) = (2. * t3 - 3. * t2 + 1., t3 - 2. * t2 + t, -2. * t3 + 3. * t2, t3 - t2);
    [0, 1].map(|j| h00 * p[i][j] + h10 * m[i][j] + h01 * p[i + 1][j] + h11 * m[i + 1][j])
}

/// Máximo de `f` em `[a, b]` por seção áurea. Refinar assim mantém o resíduo suave para o Jacobiano
/// numérico: a amostra mais próxima daria saltos do tamanho do passo da amostragem.
fn golden_max(f: impl Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    const R: f64 = 0.618_033_988_749_895;
    let (mut c, mut d) = (b - R * (b - a), a + R * (b - a));
    let (mut fc, mut fd) = (f(c), f(d));
    for _ in 0..48 {
        if fc > fd {
            b = d;
            (d, fd) = (c, fc);
            c = b - R * (b - a);
            fc = f(c);
        } else {
            a = c;
            (c, fc) = (d, fd);
            d = a + R * (b - a);
            fd = f(d);
        }
    }
    (a + b) / 2.
}

/// Pontos em que uma polilinha 3D encontra um plano (`normal` unitária): todas as travessias, os
/// vértices que caem sobre o plano e as duas pontas de um trecho deitado nele. Um perfil de outro
/// plano costuma ter vértices exatamente no plano do sketch, por isso a tolerância: comparar só o
/// sinal perderia a ponta que termina ali.
pub fn plane_crossings(pts: &[[f64; 3]], origin: [f64; 3], normal: [f64; 3]) -> Vec<[f64; 3]> {
    const TOL: f64 = 1e-7;
    let d: Vec<f64> = pts.iter().map(|p| (0..3).map(|k| (p[k] - origin[k]) * normal[k]).sum()).collect();
    let mut out: Vec<[f64; 3]> = vec![];
    let mut push = |q: [f64; 3]| {
        if !out.iter().any(|o| (0..3).all(|k| (o[k] - q[k]).abs() < 1e-6)) {
            out.push(q);
        }
    };
    for i in 0..pts.len() {
        let on = d[i].abs() <= TOL;
        let (before, after) = (i > 0 && d[i - 1].abs() <= TOL, i + 1 < pts.len() && d[i + 1].abs() <= TOL);
        if on && !(before && after) {
            push(pts[i]);
        }
        if i + 1 < pts.len() && !on && d[i + 1].abs() > TOL && d[i] * d[i + 1] < 0. {
            let t = d[i] / (d[i] - d[i + 1]);
            push([0, 1, 2].map(|k| pts[i][k] + (pts[i + 1][k] - pts[i][k]) * t));
        }
    }
    out
}

/// Pontos de inflexão (troca do lado da curvatura) de uma polilinha amostrada.
pub fn inflections(s: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let cross = |i: usize| {
        let (a, b, c) = (s[i - 1], s[i], s[i + 1]);
        (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])
    };
    let mut out = vec![];
    let mut last = 0f64;
    for i in 1..s.len().saturating_sub(1) {
        let c = cross(i);
        if c.abs() < 1e-12 {
            continue;
        }
        if last != 0. && c.signum() != last.signum() {
            out.push(s[i]);
        }
        last = c;
    }
    out
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
    /// tangentes próprias nos pontos das splines (alças Bézier)
    #[serde(default)]
    pub spline_tan: Vec<SplineTan>,
    /// política da geometria externa nova deste sketch
    #[serde(default)]
    pub link_policy: LinkPolicy,
    /// origem e orientação próprias
    #[serde(default)]
    pub pos: SketchPos,
}

impl SketchData {
    /// Pontos da spline como o perfil os usa: os próprios pontos (interpolação do kernel) ou,
    /// com alguma tangente própria, a curva de Hermite bem amostrada.
    pub fn spline_uv(&self, curve: Id) -> Vec<[f64; 2]> {
        let Some(Curve::Spline { pts }) = self.curves.iter().find(|c| c.id == curve).map(|c| &c.geom) else { return vec![] };
        let p: Vec<[f64; 2]> = pts.iter().map(|q| self.pos(*q)).collect();
        // com tangência, o perfil precisa ser a mesma curva que o solver enxerga
        let tangent = self.constraints.iter().any(|c| matches!(c.kind, Kind::Tangent(a, b) if a == curve || b == curve));
        if !tangent && !self.spline_tan.iter().any(|t| t.curve == curve) {
            return p;
        }
        spline_eval(&p, &self.own_tangents(curve), 16)
    }
    /// Tangente própria (ou nenhuma) de cada ponto da spline, na ordem.
    pub fn own_tangents(&self, curve: Id) -> Vec<Option<[f64; 2]>> {
        let Some(Curve::Spline { pts }) = self.curves.iter().find(|c| c.id == curve).map(|c| &c.geom) else { return vec![] };
        pts.iter().map(|q| self.spline_tan.iter().find(|t| t.curve == curve && t.point == *q).map(|t| t.dir)).collect()
    }
    /// Define (ou tira, com `None`) a tangente própria num ponto da spline.
    pub fn set_spline_tangent(&mut self, curve: Id, point: Id, dir: Option<[f64; 2]>) {
        self.spline_tan.retain(|t| !(t.curve == curve && t.point == point));
        if let Some(d) = dir {
            self.spline_tan.push(SplineTan { curve, point, dir: d });
        }
    }
    /// Acrescenta um ponto na spline perto de `at` (no trecho mais próximo).
    pub fn spline_insert(&mut self, al: &mut impl FnMut() -> Id, curve: Id, at: [f64; 2]) -> Option<Id> {
        let pts = match self.curves.iter().find(|c| c.id == curve).map(|c| &c.geom) {
            Some(Curve::Spline { pts }) => pts.clone(),
            _ => return None,
        };
        let p: Vec<[f64; 2]> = pts.iter().map(|q| self.pos(*q)).collect();
        let s = spline_eval(&p, &self.own_tangents(curve), 20);
        let mut best = (f64::MAX, 0usize);
        for (k, q) in s.iter().enumerate() {
            let d = (q[0] - at[0]).powi(2) + (q[1] - at[1]).powi(2);
            if d < best.0 {
                best = (d, k);
            }
        }
        let seg = (best.1.saturating_sub(1) / 20).min(pts.len() - 2);
        let q = s[best.1];
        let id = self.add_point(al, q[0], q[1]);
        if let Some(Curve::Spline { pts }) = self.curves.iter_mut().find(|c| c.id == curve).map(|c| &mut c.geom) {
            pts.insert(seg + 1, id);
        }
        Some(id)
    }
    /// Tira um ponto da spline (ficam pelo menos dois).
    pub fn spline_remove(&mut self, curve: Id, point: Id) -> bool {
        let mut ok = false;
        if let Some(Curve::Spline { pts }) = self.curves.iter_mut().find(|c| c.id == curve).map(|c| &mut c.geom) {
            if pts.len() > 2 {
                if let Some(i) = pts.iter().position(|q| *q == point) {
                    pts.remove(i);
                    ok = true;
                }
            }
        }
        if ok {
            self.spline_tan.retain(|t| !(t.curve == curve && t.point == point));
            // o ponto some se nada mais o usa
            let used = self.curves.iter().any(|c| Self::curve_points(&c.geom).contains(&point));
            if !used {
                self.points.retain(|p| p.id != point);
                self.constraints.retain(|k| !k.kind.refs().contains(&point));
            }
        }
        ok
    }
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
    /// polilinha de pontos fixos (na ordem); `isolated`: pontos soltos, sem linhas entre eles
    /// (as várias travessias de uma curva pelo plano do sketch)
    Poly {
        points: Vec<Id>,
        closed: bool,
        #[serde(default)]
        isolated: bool,
    },
    /// círculo: centro fixo e raio da curva acompanhando
    Circle { center: Id, curve: Id },
    /// linha exata
    Line { a: Id, b: Id, curve: Id },
    /// arco exato
    Arc { center: Id, s: Id, e: Id, curve: Id },
    /// elipse exata (inteira: `s` e `e` são o centro)
    Ellipse { center: Id, s: Id, e: Id, curve: Id },
}

/// O que a geometria externa faz quando a origem muda.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum LinkPolicy {
    /// acompanha a origem
    #[default]
    Associativo,
    /// guarda a origem, mas não acompanha: volta a acompanhar sem escolher de novo
    Isolado,
    /// só copia: nasce como geometria comum, sem vínculo
    Instantaneo,
}

/// Posição própria do sketch sobre o plano-base. Mudar H ou V de um lado só troca a normal, porque
/// H × V é sempre a normal: o sketch passa a ser visto pelo outro lado.
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct SketchPos {
    /// origem do sketch, em coordenadas do plano-base
    #[serde(default)]
    pub origem: [f64; 2],
    /// H e V trocam de lugar
    #[serde(default)]
    pub trocar: bool,
    #[serde(default)]
    pub inverter_h: bool,
    #[serde(default)]
    pub inverter_v: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SkLink {
    pub id: Id,
    pub source: LinkSrc,
    /// interseção com o plano do sketch (senão: projeção ortogonal no plano)
    pub intersect: bool,
    pub shape: LinkShape,
    #[serde(default)]
    pub policy: LinkPolicy,
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
        let shape = self.monta_poli(al, uv, Default::default(), construction);
        let id = al();
        self.links.push(SkLink { id, source, intersect, shape, policy: self.link_policy });
        Some(id)
    }

    /// Põe no sketch os pontos soltos em que uma curva externa corta o plano.
    pub fn add_link_points(&mut self, al: &mut impl FnMut() -> Id, source: LinkSrc, uv: &[[f64; 2]], construction: bool) -> Option<Id> {
        if uv.is_empty() {
            return None;
        }
        let points: Vec<Id> = uv
            .iter()
            .map(|q| {
                let id = self.add_point(al, q[0], q[1]);
                if let Some(p) = self.point_mut(id) {
                    p.fixed = true;
                }
                self.add_curve(al, Curve::Point { p: id }, construction);
                id
            })
            .collect();
        let id = al();
        self.links.push(SkLink { id, source, intersect: true, shape: LinkShape::Poly { points, closed: false, isolated: true }, policy: self.link_policy });
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
            LinkShape::Poly { points, isolated: true, .. } => {
                // cada ponto segue a travessia correspondente; se o 3D mudou e sobraram pontos, eles ficam
                for (id, p) in points.iter().zip(uv) {
                    if let Some(pt) = self.point_mut(*id) {
                        (pt.x, pt.y) = (p[0], p[1]);
                    }
                }
            }
            LinkShape::Poly { points, closed, .. } => {
                let n = points.len() + usize::from(closed);
                let q = resample(uv, n);
                for (id, p) in points.iter().zip(q) {
                    if let Some(pt) = self.point_mut(*id) {
                        (pt.x, pt.y) = (p[0], p[1]);
                    }
                }
            }
            // forma exata sem fórmula na origem: só as pontas seguem a polilinha
            LinkShape::Line { a, b, .. } | LinkShape::Arc { s: a, e: b, .. } | LinkShape::Ellipse { s: a, e: b, .. } => {
                if let (Some(p), Some(q)) = (uv.first(), uv.last()) {
                    for (id, r) in [(a, p), (b, q)] {
                        if let Some(pt) = self.point_mut(id) {
                            (pt.x, pt.y) = (r[0], r[1]);
                        }
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
            Curve::Spline { pts } => pts.clone(),
            Curve::Point { p } => vec![*p],
            Curve::Ellipse { c, s, e, .. } if s == c => {
                debug_assert!(s == e);
                vec![*c]
            }
            Curve::Ellipse { c, s, e, .. } => vec![*c, *s, *e],
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
        self.constraints.push(Constraint { id, kind, label: None, reference: false, flipped: false });
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

    /// Polígono regular inscrito em círculo de construção, com um vértice no ângulo `a0` (radianos).
    pub fn add_polygon(&mut self, al: &mut impl FnMut() -> Id, c: [f64; 2], r: f64, n: usize, a0: f64) -> Id {
        self.poligono_regular(al, c, r, n, a0).0
    }

    /// Polígono regular em volta de um círculo de construção de raio `r`, com o meio de um lado no
    /// ângulo `a0` (radianos). Devolve esse círculo, que é o que recebe a cota.
    ///
    /// Tangência em todos os lados não bastaria: com número par de lados, lados iguais em volta de um
    /// círculo ainda deixam o polígono abrir como um losango. Por isso a regularidade continua vindo
    /// do círculo dos vértices, e o círculo interno só amarra o apótema, tangente a um lado.
    pub fn add_polygon_circunscrito(&mut self, al: &mut impl FnMut() -> Id, c: [f64; 2], r: f64, n: usize, a0: f64) -> Id {
        let meio = std::f64::consts::PI / n.clamp(POLIGONO_LADOS_MIN, POLIGONO_LADOS_MAX) as f64;
        let (fora, lado) = self.poligono_regular(al, c, r / meio.cos(), n, a0 - meio);
        let Some(Curve::Circle { c: pc, .. }) = self.curve(fora).map(|k| k.geom.clone()) else { return fora };
        let dentro = self.add_curve(al, Curve::Circle { c: pc, r }, true);
        self.add_constraint(al, Kind::Tangent(lado, dentro));
        dentro
    }

    /// Círculo dos vértices e primeiro lado. Vértices no círculo com lados iguais fixam a forma.
    fn poligono_regular(&mut self, al: &mut impl FnMut() -> Id, c: [f64; 2], r: f64, n: usize, a0: f64) -> (Id, Id) {
        let n = n.clamp(POLIGONO_LADOS_MIN, POLIGONO_LADOS_MAX);
        let pc = self.add_point(al, c[0], c[1]);
        let circ = self.add_curve(al, Curve::Circle { c: pc, r }, true);
        let pts: Vec<Id> = (0..n)
            .map(|i| {
                let a = a0 + 2. * std::f64::consts::PI * i as f64 / n as f64;
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
        (circ, lines[0])
    }

    /// Repete nas cópias as restrições que só falam de elementos copiados (`map`: original e cópia).
    /// `Fixed` fica de fora: ele prende um lugar do plano, e a cópia está em outro.
    pub fn copiar_restricoes(&mut self, al: &mut impl FnMut() -> Id, map: &HashMap<Id, Id>) {
        let novas: Vec<Constraint> = self
            .constraints
            .iter()
            .filter(|c| !matches!(c.kind, Kind::Fixed(_)))
            .filter_map(|c| Some(Constraint { kind: c.kind.remapeada(|r| map.get(&r).copied())?, ..c.clone() }))
            .collect();
        for mut c in novas {
            c.id = al();
            self.constraints.push(c);
        }
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
                Curve::Arc { c, s, e } | Curve::Ellipse { c, s, e, .. } => {
                    swap(c);
                    swap(s);
                    swap(e)
                }
                Curve::Point { p } => swap(p),
                Curve::Spline { pts } => pts.iter_mut().for_each(swap),
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

    /// Cota recém-criada: se ela sobre-restringe o sketch (nada acrescenta ou entra em conflito), vira
    /// referência. Medir antes e depois é o que separa a cota nova da redundância que já existia.
    pub fn settle_dimension(&mut self, id: Id, vars: &Vars) -> bool {
        let Some(i) = self.constraints.iter().position(|c| c.id == id) else { return false };
        let mut without = self.clone();
        without.constraints[i].reference = true;
        let before = without.solve(vars, &[]);
        let after = self.clone().solve(vars, &[]);
        let over = (before.converged && !after.converged) || after.redundant > before.redundant;
        if over {
            self.constraints[i].reference = true;
        }
        over
    }

    /// Alterna a cota entre comandar e referência. Devolve false se ela não pode comandar, porque
    /// sobre-restringiria o sketch. Ao voltar a comandar, parte da medida atual: nada salta.
    pub fn toggle_reference(&mut self, id: Id, vars: &Vars) -> bool {
        let Some(i) = self.constraints.iter().position(|c| c.id == id) else { return false };
        if !self.constraints[i].reference {
            self.constraints[i].reference = true;
            self.constraints[i].flipped = false;
            return true;
        }
        let m = self.measure(&self.constraints[i].kind).unwrap_or(0.);
        self.constraints[i].reference = false;
        if let Some(v) = self.constraints[i].kind.value_mut() {
            *v = ((m * 1e6).round() / 1e6).to_string();
        }
        !self.settle_dimension(id, vars)
    }

    /// Valor negativo inverte o lado. O solver só enxerga o módulo, que tem solução nos dois lados e
    /// fica no mais próximo; então a virada acontece aqui, uma vez, na troca de sinal. Número digitado
    /// fica positivo depois de virar; expressão guarda o estado para voltar quando o sinal voltar.
    fn apply_sides(&mut self, vals: &HashMap<Id, f64>, fixed: &HashSet<Id>) {
        for i in 0..self.constraints.len() {
            let c = &self.constraints[i];
            let Some(&v) = vals.get(&c.id) else { continue };
            if !c.kind.has_side() {
                continue;
            }
            if (v < 0.) != c.flipped {
                let kind = c.kind.clone();
                self.flip_side(&kind, fixed);
                self.constraints[i].flipped = v < 0.;
            }
            let c = &mut self.constraints[i];
            if let (true, Some(t)) = (v < 0., c.kind.value_mut()) {
                if t.trim().replace(',', ".").parse::<f64>().is_ok() {
                    *t = t.trim().trim_start_matches('-').to_string();
                    c.flipped = false;
                }
            }
        }
    }

    fn move_to(&mut self, id: Id, q: [f64; 2], fixed: &HashSet<Id>) -> bool {
        if fixed.contains(&id) {
            return false;
        }
        if let Some(p) = self.point_mut(id) {
            (p.x, p.y) = (q[0], q[1]);
        }
        true
    }

    /// Passa a geometria para o outro lado da cota, mexendo no que estiver livre.
    fn flip_side(&mut self, k: &Kind, fixed: &HashSet<Id>) {
        let through = |p: [f64; 2], c: [f64; 2]| [2. * c[0] - p[0], 2. * c[1] - p[1]];
        let ends = |s: &Self, l: Id| match s.curve(l).map(|c| &c.geom) {
            Some(Curve::Line { a, b }) => Some((*a, *b)),
            _ => None,
        };
        // espelha `p` na reta que passa por `o` com direção `d`
        let across = |p: [f64; 2], o: [f64; 2], d: [f64; 2]| {
            let t = dot(sub(p, o), d) / dot(d, d).max(1e-18);
            through(p, [o[0] + d[0] * t, o[1] + d[1] * t])
        };
        match *k {
            Kind::Length(l, _) => {
                if let Some((a, b)) = ends(self, l) {
                    self.flip_side(&Kind::Distance(a, b, String::new()), fixed);
                }
            }
            Kind::Distance(a, b, _) => {
                let (pa, pb) = (self.pos(a), self.pos(b));
                if !self.move_to(b, through(pb, pa), fixed) {
                    self.move_to(a, through(pa, pb), fixed);
                }
            }
            Kind::DistanceX(a, b, _) | Kind::DistanceY(a, b, _) => {
                let i = usize::from(matches!(k, Kind::DistanceY(..)));
                let (pa, pb) = (self.pos(a), self.pos(b));
                let (mut qa, mut qb) = (pa, pb);
                qb[i] = 2. * pa[i] - pb[i];
                qa[i] = 2. * pb[i] - pa[i];
                if !self.move_to(b, qb, fixed) {
                    self.move_to(a, qa, fixed);
                }
            }
            Kind::PointLineDistance(p, l, _) => {
                let Some((a, b)) = ends(self, l) else { return };
                let (pp, pa, pb) = (self.pos(p), self.pos(a), self.pos(b));
                let q = across(pp, pa, sub(pb, pa));
                if !self.move_to(p, q, fixed) {
                    // ponto preso: a linha é que passa para o outro lado dele
                    let d = sub(pp, q);
                    self.move_to(a, [pa[0] + d[0], pa[1] + d[1]], fixed);
                    self.move_to(b, [pb[0] + d[0], pb[1] + d[1]], fixed);
                }
            }
            Kind::Angle(l1, l2, _) | Kind::AngleSupp(l1, l2, _) => {
                let (Some((a, b)), Some((c, d))) = (ends(self, l1), ends(self, l2)) else { return };
                // gira a segunda linha em torno da ponta comum; se ela estiver presa, gira a primeira
                let (pa, pb, pc, pd) = (self.pos(a), self.pos(b), self.pos(c), self.pos(d));
                let (pivot2, far2) = if c == a || c == b { (c, d) } else { (d, c) };
                let (pivot1, far1) = if a == c || a == d { (a, b) } else { (b, a) };
                let (o2, o1) = (self.pos(pivot2), self.pos(pivot1));
                if !self.move_to(far2, across(self.pos(far2), o2, sub(pb, pa)), fixed) {
                    self.move_to(far1, across(self.pos(far1), o1, sub(pd, pc)), fixed);
                }
            }
            _ => {}
        }
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

    /// Folga entre a spline e a curva `other` no extremo local da distância. Tangência sem ponto
    /// comum é isso: a distância com sinal tem um extremo que vale zero. Sem extremo interno, só pode
    /// tocar numa ponta, e aí a inclinação entra na conta para encostar sem estar tangente não contar.
    fn spline_gap(&self, x: &[f64], spline: Id, other: Id) -> f64 {
        let Some(Curve::Spline { pts }) = self.sk.curve(spline).map(|c| &c.geom) else { return 0. };
        if pts.len() < 2 {
            return 0.;
        }
        let p: Vec<[f64; 2]> = pts.iter().map(|q| self.p(x, *q)).collect();
        let m = spline_tangents(&p, &self.sk.own_tangents(spline));
        let (line, center) = (self.line(x, other), self.center(x, other));
        let r = self.radius(x, other);
        let gap = |u: f64| {
            let q = hermite_at(&p, &m, u);
            match (line, center) {
                (Some((a, b)), _) => cross(sub(b, a), sub(q, a)) / len2(sub(b, a)).max(1e-12),
                (_, Some(c)) => len2(sub(q, c)) - r,
                _ => 0.,
            }
        };
        let last = (p.len() - 1) as f64;
        let n = 16 * (p.len() - 1);
        let u = |k: usize| last * k as f64 / n as f64;
        let g: Vec<f64> = (0..=n).map(|k| gap(u(k))).collect();
        let best = (1..n).filter(|k| (g[*k] - g[k - 1]) * (g[k + 1] - g[*k]) <= 0.).min_by(|a, b| g[*a].abs().total_cmp(&g[*b].abs()));
        if let Some(k) = best {
            let side = if g[k] >= g[k - 1] { 1. } else { -1. };
            return gap(golden_max(|t| side * gap(t), u(k - 1), u(k + 1)));
        }
        let (e, inner) = if g[0].abs() <= g[n].abs() { (0., u(1)) } else { (last, u(n - 1)) };
        gap(e).abs() + (gap(inner) - gap(e)).abs()
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
                    let spline = |c: Id| matches!(self.sk.curve(c).map(|k| &k.geom), Some(Curve::Spline { .. }));
                    if spline(*c1) || spline(*c2) {
                        // com ponto comum, a tangente própria resolve depois (`apply_spline_tangency`)
                        // e nenhum ponto precisa se mexer: não há equação
                        let (s, o) = if spline(*c1) { (*c1, *c2) } else { (*c2, *c1) };
                        if self.sk.spline_contact(s, o).is_none() {
                            out.push(self.spline_gap(x, s, o));
                        }
                        return;
                    }
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
                Kind::Angle(l1, l2, _) | Kind::AngleSupp(l1, l2, _) => {
                    if let (Some((a, b)), Some((c, d))) = (ln(*l1), ln(*l2)) {
                        let (u, mut w) = (sub(b, a), sub(d, c));
                        // o suplemento é o ângulo com a segunda direção invertida
                        if matches!(k, Kind::AngleSupp(..)) {
                            w = [-w[0], -w[1]];
                        }
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
                Kind::LineAngle(l, _) => {
                    if let Some((a, b)) = ln(*l) {
                        let (d, t) = (sub(b, a), v.to_radians());
                        out.push((d[1] * t.cos() - d[0] * t.sin()) / len2(d).max(1e-12));
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

    /// Jacobiano numérico, ou `None` se o prazo acabar no meio: em sketch grande, é ele que pesa.
    fn jacobian(&self, x: &[f64], m: usize, prazo: Option<Instant>) -> Option<Vec<Vec<f64>>> {
        let n = x.len();
        let mut j = vec![vec![0.; n]; m];
        let mut xp = x.to_vec();
        for c in 0..n {
            if prazo.is_some_and(|t| Instant::now() >= t) {
                return None;
            }
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
        Some(j)
    }
}

/// Direções de um ponto, a partir das suas velocidades nos vetores do espaço nulo: os autovetores da
/// matriz 2x2 Σ v vᵀ. Autovalor menor desprezível quer dizer que todas apontam para o mesmo lado.
fn direcoes(vel: impl Iterator<Item = [f64; 2]>) -> Vec<[f64; 2]> {
    let (mut a, mut b, mut c) = (0., 0., 0.);
    for v in vel {
        a += v[0] * v[0];
        b += v[0] * v[1];
        c += v[1] * v[1];
    }
    let meio = (a + c) / 2.;
    let raiz = (((a - c) / 2.).powi(2) + b * b).sqrt();
    let (l1, l2) = (meio + raiz, meio - raiz);
    if l1 < 1e-14 {
        return vec![];
    }
    if l2 > 1e-8 * l1 {
        return vec![[1., 0.], [0., 1.]];
    }
    let d = if b.abs() > 1e-14 { [l1 - c, b] } else if a >= c { [1., 0.] } else { [0., 1.] };
    let l = len2(d);
    vec![[d[0] / l, d[1] / l]]
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
        for c in self.constraints.iter().filter(|c| !c.reference) {
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
        self.solve_ate(vars, pinned, None)
    }

    /// Como `solve`, mas desiste se passar de `prazo`: devolve `estourou` e deixa os pontos como
    /// estavam. É o que permite resolver a cada movimento do mouse sem travar em sketch grande.
    pub fn solve_ate(&mut self, vars: &Vars, pinned: &[Id], prazo: Option<Instant>) -> SolveReport {
        let mut rep = SolveReport::default();
        let vals = self.values(vars, &mut rep.errors);

        let fixed: HashSet<Id> = self.fixed_ids().into_iter().chain(pinned.iter().copied()).collect();
        self.apply_sides(&vals, &fixed);

        let mut x0 = vec![];
        let mut pvar = HashMap::new();
        for p in &self.points {
            if !fixed.contains(&p.id) {
                pvar.insert(p.id, x0.len());
                x0.push(p.x);
                x0.push(p.y);
            }
        }
        // o raio do círculo externo vem da origem: não é incógnita, senão sobraria um grau de liberdade
        let externos: HashSet<Id> = self.links.iter().filter_map(|l| if let LinkShape::Circle { curve, .. } = l.shape { Some(curve) } else { None }).collect();
        let mut rvar = HashMap::new();
        for c in &self.curves {
            if let (Curve::Circle { r, .. }, false) = (&c.geom, externos.contains(&c.id)) {
                let r = *r;
                rvar.insert(c.id, x0.len());
                x0.push(r);
            }
        }
        let mut eqs: Vec<(Id, Eq)> = vec![];
        for c in &self.constraints {
            if c.kind.is_dimension() && !vals.contains_key(&c.id) {
                continue; // cota de referência ou com expressão inválida
            }
            let v = vals.get(&c.id).copied().unwrap_or(0.);
            eqs.push((c.id, Eq::K(c.kind.clone(), if c.kind.has_side() { v.abs() } else { v })));
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
                let Some(j) = sys.jacobian(&x, m, prazo) else {
                    rep.estourou = true;
                    return rep;
                };
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
            let Some(j) = sys.jacobian(&x, m, prazo) else {
                rep.estourou = true;
                return rep;
            };
            let (piv, red) = rref(j, n, 1e-7);
            rep.dof = n - piv.len();
            rep.redundant = m.saturating_sub(piv.len());
            let free: Vec<usize> = (0..n).filter(|c| !piv.contains(c)).collect();
            let mut determined = vec![false; n];
            for (row, &pc) in piv.iter().enumerate() {
                determined[pc] = free.iter().all(|&fc| red[row][fc].abs() < 1e-7);
            }
            // cada coluna livre dá um vetor do espaço nulo: 1 nela e o oposto da linha nas colunas pivô
            let mut linha = vec![None; n];
            for (row, &pc) in piv.iter().enumerate() {
                linha[pc] = Some(row);
            }
            let nulo = |i: usize, f: usize| match linha[i] {
                Some(row) => -red[row][f],
                None => f64::from(i == f),
            };
            for (id, &i) in &pvar {
                if determined[i] && determined[i + 1] {
                    rep.fixed_points.insert(*id);
                } else {
                    let d = direcoes(free.iter().map(|&f| [nulo(i, f), nulo(i + 1, f)]));
                    if !d.is_empty() {
                        rep.liberdade.pontos.insert(*id, d);
                    }
                }
            }
            for (id, &i) in &rvar {
                if !determined[i] {
                    rep.liberdade.raios.insert(*id);
                }
            }
        } else {
            // nada livre: toda equação que sobrou é redundante
            rep.dof = 0;
            rep.redundant = m;
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
        self.apply_spline_tangency();
        rep
    }

    /// Leva o arco a passar por `alvo` e deixa o solver decidir o resto: um ponto temporário preso ao
    /// arco e parado no alvo é o que puxa. Assim tangências e cotas continuam valendo, e o arco muda
    /// só o que elas deixam. Sem solução, nada muda e devolve `None`: quem arrasta para no último
    /// lugar válido, em vez de ver a restrição quebrar.
    pub fn arrastar_arco(&mut self, arco: Id, alvo: [f64; 2], vars: &Vars) -> Option<SolveReport> {
        let mut g = self.clone();
        let mut n = u64::MAX;
        let mut al = || {
            n -= 1;
            Id(n)
        };
        let pega = g.add_point(&mut al, alvo[0], alvo[1]);
        let sobre = g.add_constraint(&mut al, Kind::PointOnCurve(pega, arco));
        let rep = g.solve(vars, &[pega]);
        if !rep.converged {
            return None;
        }
        g.constraints.retain(|c| c.id != sobre);
        g.points.retain(|p| p.id != pega);
        *self = g;
        Some(rep)
    }

    /// Parâmetro da spline mais perto de `q` (0 no primeiro ponto, 1 no segundo, e assim por diante).
    pub fn spline_param(&self, curve: Id, q: [f64; 2]) -> f64 {
        let Some(Curve::Spline { pts }) = self.curve(curve).map(|c| &c.geom) else { return 0. };
        let p: Vec<[f64; 2]> = pts.iter().map(|x| self.pos(*x)).collect();
        if p.len() < 2 {
            return 0.;
        }
        let m = spline_tangents(&p, &self.own_tangents(curve));
        let n = 24 * (p.len() - 1);
        let u = |k: usize| (p.len() - 1) as f64 * k as f64 / n as f64;
        (0..=n).map(u).min_by(|a, b| len2(sub(hermite_at(&p, &m, *a), q)).total_cmp(&len2(sub(hermite_at(&p, &m, *b), q)))).unwrap_or(0.)
    }

    /// Arrasta a spline pelo parâmetro `u`: os dois pontos do trecho seguem na proporção da distância
    /// até eles, até a curva passar por `alvo`. Depois o solver mantém as restrições, primeiro com os
    /// dois parados no lugar pedido e, se isso conflitar (ponto preso a outra curva), deixando-os
    /// escorregar. Sem solução, nada muda e devolve `None`.
    pub fn arrastar_spline(&mut self, curve: Id, u: f64, alvo: [f64; 2], vars: &Vars) -> Option<SolveReport> {
        let Some(Curve::Spline { pts }) = self.curve(curve).map(|c| c.geom.clone()) else { return None };
        if pts.len() < 2 {
            return None;
        }
        let presos = self.fixed_ids();
        let i = (u.max(0.) as usize).min(pts.len() - 2);
        let t = (u - i as f64).clamp(0., 1.);
        let peso = |id: Id, w: f64| if presos.contains(&id) { 0. } else { w };
        let w = [peso(pts[i], 1. - t), peso(pts[i + 1], t)];
        if w[0] + w[1] < 1e-9 {
            return None;
        }
        let base: Vec<[f64; 2]> = pts.iter().map(|x| self.pos(*x)).collect();
        let own = self.own_tangents(curve);
        // ganho do deslocamento na curva: as funções de Hermite dos dois pontos, nos pesos deles
        let h01 = t * t * (3. - 2. * t);
        let ganho = (w[0] * (1. - h01) + w[1] * h01).max(0.1);
        let mut d = [0., 0.];
        let mut p = base.clone();
        for _ in 0..8 {
            let m = spline_tangents(&p, &own);
            let e = sub(alvo, hermite_at(&p, &m, u));
            if len2(e) < 1e-9 {
                break;
            }
            d = [d[0] + e[0] / ganho, d[1] + e[1] / ganho];
            for (k, wk) in [(i, w[0]), (i + 1, w[1])] {
                p[k] = [base[k][0] + d[0] * wk, base[k][1] + d[1] * wk];
            }
        }
        let mut g = self.clone();
        let mut movidos = vec![];
        for k in [i, i + 1] {
            if let Some(q) = g.point_mut(pts[k]) {
                (q.x, q.y) = (p[k][0], p[k][1]);
            }
            if !presos.contains(&pts[k]) {
                movidos.push(pts[k]);
            }
        }
        for pins in [movidos.as_slice(), &[]] {
            let mut tentativa = g.clone();
            let rep = tentativa.solve(vars, pins);
            if rep.converged {
                *self = tentativa;
                return Some(rep);
            }
        }
        None
    }

    /// Pontos que o solver nunca move: fixos ou com restrição de fixar.
    fn fixed_ids(&self) -> HashSet<Id> {
        self.points.iter().filter(|p| p.fixed).map(|p| p.id).chain(self.constraints.iter().filter_map(|c| if let Kind::Fixed(p) = &c.kind { Some(*p) } else { None })).collect()
    }

    /// Ponto em que a spline toca a outra curva por construção: ponto compartilhado, coincidência ou
    /// ponto da spline preso à curva.
    pub fn spline_contact(&self, spline: Id, other: Id) -> Option<Id> {
        let Some(Curve::Spline { pts }) = self.curve(spline).map(|c| &c.geom) else { return None };
        let ends = match &self.curve(other)?.geom {
            Curve::Line { a, b } => vec![*a, *b],
            Curve::Arc { s, e, .. } => vec![*s, *e],
            Curve::Spline { pts } => pts.clone(),
            _ => vec![],
        };
        pts.iter().copied().find(|p| {
            ends.contains(p)
                || self.constraints.iter().any(|k| match k.kind {
                    Kind::Coincident(a, b) => (a == *p && ends.contains(&b)) || (b == *p && ends.contains(&a)),
                    Kind::PointOnCurve(q, c) => q == *p && c == other,
                    _ => false,
                })
        })
    }

    /// Direção da curva no ponto dado (linha, tangente do círculo ou do arco, tangente da spline).
    fn direction_at(&self, curve: Id, at: Id) -> Option<[f64; 2]> {
        match &self.curve(curve)?.geom {
            Curve::Line { a, b } => Some(sub(self.pos(*b), self.pos(*a))),
            Curve::Circle { c, .. } | Curve::Arc { c, .. } => {
                let r = sub(self.pos(at), self.pos(*c));
                Some([-r[1], r[0]])
            }
            Curve::Spline { pts } => {
                let i = pts.iter().position(|p| *p == at)?;
                let p: Vec<[f64; 2]> = pts.iter().map(|q| self.pos(*q)).collect();
                spline_tangents(&p, &self.own_tangents(curve)).get(i).copied()
            }
            Curve::Point { .. } | Curve::Ellipse { .. } => None,
        }
    }

    /// Tangência da spline num ponto de contato: a tangente própria dela passa a seguir a outra curva,
    /// mantendo o sentido e a intensidade atuais. A tangente própria não é variável do solver, então
    /// isso acontece depois de resolver os pontos; a direção da outra curva não depende dela.
    fn apply_spline_tangency(&mut self) {
        let pairs: Vec<(Id, Id)> = self.constraints.iter().filter_map(|c| if let Kind::Tangent(a, b) = c.kind { Some((a, b)) } else { None }).collect();
        for (a, b) in pairs {
            let spline = |s: &Self, c: Id| matches!(s.curve(c).map(|k| &k.geom), Some(Curve::Spline { .. }));
            // entre duas splines, a segunda acompanha a primeira
            let (sp, other) = if spline(self, b) { (b, a) } else { (a, b) };
            let Some(at) = self.spline_contact(sp, other) else { continue };
            let (Some(d), Some(now)) = (self.direction_at(other, at), self.direction_at(sp, at)) else { continue };
            let (ld, ln) = (len2(d), len2(now));
            if ld < 1e-12 || ln < 1e-12 {
                continue;
            }
            let k = ln / ld * if dot(d, now) < 0. { -1. } else { 1. };
            self.set_spline_tangent(sp, at, Some([d[0] * k, d[1] * k]));
        }
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
        let (l, o) = self.chains();
        (l, o.iter().map(|c| c.len()).sum())
    }

    /// Perfis fechados e cadeias abertas (de ponta a ponta, na ordem).
    pub fn chains(&self) -> (Vec<Vec<Seg>>, Vec<Vec<Seg>>) {
        let (l, o) = self.percursos();
        (l.into_iter().map(|x| x.0).collect(), o)
    }

    /// Laços fechados com as curvas de cada um, na mesma ordem de `loops`: quem clica numa região
    /// precisa voltar do contorno às entidades que o formam.
    pub fn loops_com_curvas(&self) -> Vec<(Vec<Seg>, Vec<Id>)> {
        self.percursos().0
    }

    fn percursos(&self) -> (Vec<(Vec<Seg>, Vec<Id>)>, Vec<Vec<Seg>>) {
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
        // pontas no mesmo lugar (snap sem restrição, spline desenhada até a ponta de uma linha)
        // também se unem: o perfil fecha pela geometria
        let ends: Vec<Id> = self
            .curves
            .iter()
            .filter(|c| !c.construction)
            .flat_map(|c| match &c.geom {
                Curve::Line { a, b } => vec![*a, *b],
                Curve::Arc { s, e, .. } => vec![*s, *e],
                Curve::Spline { pts } if pts.len() >= 2 => vec![pts[0], pts[pts.len() - 1]],
                Curve::Ellipse { c, s, e, .. } if s != c => vec![*s, *e],
                _ => vec![],
            })
            .collect();
        let scale = ends.iter().map(|p| self.pos(*p)).fold(0f64, |m, q| m.max(q[0].abs()).max(q[1].abs())).max(1.);
        let tol = scale * 1e-7 + 1e-9;
        for i in 0..ends.len() {
            for j in i + 1..ends.len() {
                let (a, b) = (self.pos(ends[i]), self.pos(ends[j]));
                if (a[0] - b[0]).abs() <= tol && (a[1] - b[1]).abs() <= tol {
                    let (ra, rb) = (find(&mut parent, ends[i]), find(&mut parent, ends[j]));
                    if ra != rb {
                        parent.insert(ra, rb);
                    }
                }
            }
        }
        let mut loops = vec![];
        // arestas: (extremo1, extremo2, segmento, curva)
        let mut edges: Vec<(Id, Id, Seg, Id)> = vec![];
        for c in self.curves.iter().filter(|c| !c.construction) {
            if let Curve::Spline { pts } = &c.geom {
                if pts.len() >= 2 {
                    let v: Vec<[f64; 2]> = self.spline_uv(c.id);
                    edges.push((find(&mut parent, pts[0]), find(&mut parent, pts[pts.len() - 1]), Seg::Spline(v), c.id));
                }
                continue;
            }
            match c.geom {
                Curve::Circle { c: ctr, r } => loops.push((vec![Seg::Circle(self.pos(ctr), r)], vec![c.id])),
                Curve::Line { a, b } => edges.push((find(&mut parent, a), find(&mut parent, b), Seg::Line(self.pos(a), self.pos(b)), c.id)),
                Curve::Arc { c: ctr, s, e } => {
                    let (pc, ps, pe) = (self.pos(ctr), self.pos(s), self.pos(e));
                    edges.push((find(&mut parent, s), find(&mut parent, e), Seg::Arc(ps, Self::arc_mid(pc, ps, pe), pe), c.id));
                }
                Curve::Ellipse { c: ctr, s, e, major, minor, t0, t1 } => {
                    let seg = Seg::Ellipse(self.pos(ctr), major, minor, t0, t1);
                    if s == ctr {
                        loops.push((vec![seg], vec![c.id]));
                    } else {
                        edges.push((find(&mut parent, s), find(&mut parent, e), seg, c.id));
                    }
                }
                Curve::Point { .. } | Curve::Spline { .. } => {}
            }
        }
        let rev = |s: Seg| match s {
            Seg::Line(a, b) => Seg::Line(b, a),
            Seg::Arc(a, m, b) => Seg::Arc(b, m, a),
            Seg::Ellipse(c, m, r, t0, t1) => Seg::Ellipse(c, m, r, t1, t0),
            Seg::Spline(mut v) => {
                v.reverse();
                Seg::Spline(v)
            }
            c => c,
        };
        let mut open = vec![];
        while let Some((s, e, seg, id)) = edges.pop() {
            let mut lp = vec![seg];
            let mut curvas = vec![id];
            let mut end = e;
            let mut closed = s == e;
            while !closed {
                let Some(i) = edges.iter().position(|(a, b, ..)| *a == end || *b == end) else { break };
                let (a, b, sg, id) = edges.swap_remove(i);
                curvas.push(id);
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
                loops.push((lp, curvas));
            } else {
                // aberta: cresce também pelo começo
                let mut start = s;
                while let Some(i) = edges.iter().position(|(a, b, ..)| *a == start || *b == start) {
                    let (a, b, sg, _) = edges.swap_remove(i);
                    if b == start {
                        lp.insert(0, sg);
                        start = a;
                    } else {
                        lp.insert(0, rev(sg));
                        start = b;
                    }
                }
                open.push(lp);
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
            Kind::LineAngle(l, _) => {
                let (a, b) = sys.line(&x, *l)?;
                let d = sub(b, a);
                return Some(d[1].atan2(d[0]).to_degrees());
            }
            Kind::Angle(l1, l2, _) | Kind::AngleSupp(l1, l2, _) => {
                let (a, b) = sys.line(&x, *l1)?;
                let (c, d) = sys.line(&x, *l2)?;
                let (u, w) = (sub(b, a), sub(d, c));
                let m = cross(u, w).abs().atan2(dot(u, w)).to_degrees();
                return Some(if matches!(k, Kind::AngleSupp(..)) { 180. - m } else { m });
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
    fn laco_sabe_as_curvas_que_o_formam() {
        let mut n = 0u64;
        let mut al = move || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        let (_, l) = s.add_rect(&mut al, [0., 0.], [40., 40.], false);
        let c = s.add_point(&mut al, 20., 20.);
        let circ = s.add_curve(&mut al, Curve::Circle { c, r: 5. }, false);
        // aberta e de construção não fazem laço
        let (a, b) = (s.add_point(&mut al, 50., 0.), s.add_point(&mut al, 60., 0.));
        s.add_curve(&mut al, Curve::Line { a, b }, false);
        let d = s.add_point(&mut al, 20., 20.);
        s.add_curve(&mut al, Curve::Circle { c: d, r: 9. }, true);

        let v = s.loops_com_curvas();
        assert_eq!(v.iter().map(|x| x.0.clone()).collect::<Vec<_>>(), s.loops().0, "mesmos laços, na mesma ordem");
        let mut grupos: Vec<Vec<Id>> = v.into_iter().map(|x| x.1).collect();
        grupos.iter_mut().for_each(|g| g.sort());
        grupos.sort();
        let mut quadrado = l.to_vec();
        quadrado.sort();
        let mut esperado = vec![quadrado, vec![circ]];
        esperado.sort();
        assert_eq!(grupos, esperado);
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

    /// Prende centro e giro do polígono e confere que só falta a cota do círculo que comanda.
    fn poligono_preso(s: &mut SketchData, al: &mut impl FnMut() -> Id, circ: Id) -> f64 {
        let Some(Curve::Circle { c, .. }) = s.curve(circ).map(|k| k.geom.clone()) else { panic!() };
        s.add_constraint(al, Kind::Fixed(c));
        let Curve::Line { a, .. } = s.curves.iter().find(|k| matches!(k.geom, Curve::Line { .. })).unwrap().geom else { panic!() };
        s.add_constraint(al, Kind::HorizontalPts(c, a));
        let r = s.solve(&HashMap::new(), &[]);
        assert!(r.converged && r.failing.is_empty(), "{r:?}");
        assert_eq!(r.dof, 1, "só o tamanho fica livre");
        s.add_constraint(al, Kind::Radius(circ, "20".into()));
        let r = s.solve(&HashMap::new(), &[]);
        assert!(r.converged && r.dof == 0, "{r:?}");
        s.radius_of(circ).unwrap()
    }

    fn lados(s: &SketchData) -> Vec<f64> {
        s.curves.iter().filter_map(|c| if let Curve::Line { a, b } = c.geom { Some(len2(sub(s.pos(a), s.pos(b)))) } else { None }).collect()
    }

    #[test]
    fn poligono_inscrito_passa_pelos_vertices_e_aceita_de_3_a_40_lados() {
        let mut n = 0;
        let mut al = || {
            n += 1;
            Id(n)
        };
        for (pedidos, feitos) in [(3, 3), (40, 40), (2, 3), (64, 40)] {
            let mut s = SketchData::default();
            let circ = s.add_polygon(&mut al, [5., 7.], 10., pedidos, 0.);
            assert_eq!(lados(&s).len(), feitos);
            assert!((poligono_preso(&mut s, &mut al, circ) - 20.).abs() < 1e-6);
            let l = 2. * 20. * (std::f64::consts::PI / feitos as f64).sin();
            assert!(lados(&s).iter().all(|x| (x - l).abs() < 1e-6), "{feitos}: {:?}", lados(&s));
        }
    }

    #[test]
    fn poligono_circunscrito_tem_o_circulo_tangente_aos_lados() {
        let mut n = 0;
        let mut al = || {
            n += 1;
            Id(n)
        };
        // par de lados incluído: é onde só tangência deixaria virar losango
        for lados_n in [3usize, 4, 6, 7, 40] {
            let mut s = SketchData::default();
            let circ = s.add_polygon_circunscrito(&mut al, [0., 0.], 10., lados_n, 0.3);
            // o meio do primeiro lado fica sobre o raio em `a0`, a 10 do centro
            let Curve::Line { a, b } = s.curves.iter().find(|k| matches!(k.geom, Curve::Line { .. })).unwrap().geom else { panic!() };
            let m = [(s.pos(a)[0] + s.pos(b)[0]) / 2., (s.pos(a)[1] + s.pos(b)[1]) / 2.];
            assert!((m[0] - 10. * 0.3f64.cos()).abs() < 1e-9 && (m[1] - 10. * 0.3f64.sin()).abs() < 1e-9, "{m:?}");
            assert!((poligono_preso(&mut s, &mut al, circ) - 20.).abs() < 1e-6);
            // lado de um polígono com apótema 20: 2 r tan(pi/n)
            let l = 2. * 20. * (std::f64::consts::PI / lados_n as f64).tan();
            assert!(lados(&s).iter().all(|x| (x - l).abs() < 1e-6), "{lados_n}: {:?}", lados(&s));
            assert_eq!(s.loops().0.len(), 1, "os círculos de construção não entram no perfil");
        }
    }

    #[test]
    fn copiar_restricoes_so_leva_as_internas() {
        let mut n = 0;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        let p: Vec<Id> = [[0., 0.], [10., 0.], [50., 0.]].iter().map(|q| s.add_point(&mut al, q[0], q[1])).collect();
        let l = s.add_curve(&mut al, Curve::Line { a: p[0], b: p[1] }, false);
        let fora = s.add_curve(&mut al, Curve::Line { a: p[1], b: p[2] }, false);
        s.add_constraint(&mut al, Kind::Horizontal(l));
        let cota = s.add_constraint(&mut al, Kind::Length(l, "10".into()));
        s.constraints.iter_mut().find(|c| c.id == cota).unwrap().label = Some([1., 2.]);
        s.add_constraint(&mut al, Kind::Fixed(p[0]));
        s.add_constraint(&mut al, Kind::Parallel(l, fora));
        let (qa, qb) = (s.add_point(&mut al, 0., 9.), s.add_point(&mut al, 10., 9.));
        let c = s.add_curve(&mut al, Curve::Line { a: qa, b: qb }, false);
        let total = s.constraints.len();
        s.copiar_restricoes(&mut al, &HashMap::from([(l, c), (p[0], qa), (p[1], qb)]));
        assert_eq!(s.constraints.len(), total + 2, "horizontal e cota; a fixa e a que olha para fora ficam");
        let nova = s.constraints.iter().find(|k| matches!(&k.kind, Kind::Length(x, _) if *x == c)).unwrap();
        assert_eq!(nova.label, Some([1., 2.]), "a cota copiada leva o lugar do texto");
        assert_ne!(nova.id, cota);
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
        assert_eq!(r.redundant, 0, "o rasgo novo não pode nascer redundante");
        let (loops, open) = s.loops();
        assert_eq!((loops.len(), open), (1, 0));
        assert_eq!(loops[0].len(), 4);
    }

    /// Linha presa em (0,0)-(30,0) e arco tangente saindo da ponta, de 90°, raio 10.
    fn arco_tangente() -> (SketchData, Id, Id, [Id; 3]) {
        let mut s = SketchData::default();
        let mut al = alloc();
        let a = s.add_point(&mut al, 0., 0.);
        let b = s.add_point(&mut al, 30., 0.);
        let c = s.add_point(&mut al, 30., 10.);
        let e = s.add_point(&mut al, 40., 10.);
        let l = s.add_curve(&mut al, Curve::Line { a, b }, false);
        let arc = s.add_curve(&mut al, Curve::Arc { c, s: b, e }, false);
        s.add_constraint(&mut al, Kind::Fixed(a));
        s.add_constraint(&mut al, Kind::Fixed(b));
        s.add_constraint(&mut al, Kind::Tangent(l, arc));
        (s, l, arc, [c, b, e])
    }

    /// B272: arrastar o arco por um ponto qualquer não quebra a tangência.
    #[test]
    fn arrastar_arco_tangente_mantem_a_tangencia() {
        let (mut s, _, arc, [c, b, e]) = arco_tangente();
        let vars = Vars::new();
        assert!(s.solve(&vars, &[]).converged);
        let alvo = [38., 2.5];
        for k in 1..=5 {
            let t = k as f64 / 5.;
            let meio = SketchData::arc_mid(s.pos(c), s.pos(b), s.pos(e));
            let q = [meio[0] + (alvo[0] - meio[0]) * t, meio[1] + (alvo[1] - meio[1]) * t];
            let rep = s.arrastar_arco(arc, q, &vars).expect("o arco tem liberdade para seguir");
            assert!(rep.converged);
        }
        let (pc, pb, pe) = (s.pos(c), s.pos(b), s.pos(e));
        // tangente: o raio no ponto comum é perpendicular à linha, que é horizontal
        assert!((pc[0] - 30.).abs() < 1e-6, "tangência perdida: centro em {pc:?}");
        assert_eq!(pb, [30., 0.], "o ponto preso não se move");
        let r = len2(sub(pb, pc));
        assert!((len2(sub(pe, pc)) - r).abs() < 1e-6, "o arco deixou de ser arco");
        assert!((len2(sub(alvo, pc)) - r).abs() < 1e-6, "o arco não chegou ao cursor");
        assert_eq!(s.points.len(), 4, "o ponto temporário não fica no sketch");
        assert_eq!(s.constraints.len(), 3);
    }

    #[test]
    fn arco_sem_folga_nao_muda_ao_arrastar() {
        let (mut s, _, arc, [c, _, e]) = arco_tangente();
        let mut n = 500;
        let mut al = || {
            n += 1;
            Id(n)
        };
        s.add_constraint(&mut al, Kind::Fixed(c));
        s.add_constraint(&mut al, Kind::Fixed(e));
        let vars = Vars::new();
        s.solve(&vars, &[]);
        let antes = s.clone();
        assert!(s.arrastar_arco(arc, [60., 60.], &vars).is_none());
        assert_eq!(s, antes);
    }

    /// B272: a spline segue pelo trecho agarrado e o ponto preso a uma linha continua nela.
    #[test]
    fn arrastar_spline_pela_curva_respeita_as_restricoes() {
        let mut s = SketchData::default();
        let mut al = alloc();
        let pts: Vec<Id> = [[0., 0.], [20., 10.], [40., 0.], [60., 10.]].iter().map(|q| s.add_point(&mut al, q[0], q[1])).collect();
        let sp = s.add_curve(&mut al, Curve::Spline { pts: pts.clone() }, false);
        let a = s.add_point(&mut al, 40., -20.);
        let b = s.add_point(&mut al, 40., 20.);
        let trilho = s.add_curve(&mut al, Curve::Line { a, b }, false);
        s.add_constraint(&mut al, Kind::Fixed(a));
        s.add_constraint(&mut al, Kind::Fixed(b));
        s.add_constraint(&mut al, Kind::PointOnCurve(pts[2], trilho));
        s.add_constraint(&mut al, Kind::Fixed(pts[0]));
        let vars = Vars::new();
        assert!(s.solve(&vars, &[]).converged);

        // num trecho livre, a curva passa exatamente pelo cursor
        let u = s.spline_param(sp, [20., 10.]);
        assert!((u - 1.).abs() < 0.05, "parâmetro {u}");
        let alvo = [22., 16.];
        assert!(s.arrastar_spline(sp, u, alvo, &vars).is_some());
        let p: Vec<[f64; 2]> = pts.iter().map(|x| s.pos(*x)).collect();
        let m = spline_tangents(&p, &s.own_tangents(sp));
        assert!(len2(sub(hermite_at(&p, &m, u), alvo)) < 0.5, "a curva não chegou ao cursor");
        assert_eq!(s.pos(pts[0]), [0., 0.], "ponto fixo não se move");

        // no trecho do ponto preso ao trilho: ele escorrega, mas fica no trilho
        assert!(s.arrastar_spline(sp, 2., [47., 5.], &vars).is_some());
        assert!((s.pos(pts[2])[0] - 40.).abs() < 1e-6, "saiu do trilho: {:?}", s.pos(pts[2]));
    }

    /// B272: o que as setas mostram vem do espaço nulo do Jacobiano.
    #[test]
    fn liberdade_diz_como_cada_ponto_pode_mover() {
        let mut s = SketchData::default();
        let mut al = alloc();
        let a = s.add_point(&mut al, 0., 0.);
        let b = s.add_point(&mut al, 30., 0.);
        let l = s.add_curve(&mut al, Curve::Line { a, b }, false);
        let vars = Vars::new();

        // linha solta: as duas pontas andam no plano
        let lib = s.solve(&vars, &[]).liberdade;
        assert_eq!(lib.pontos[&a].len(), 2);
        assert_eq!(lib.pontos[&b].len(), 2);

        // ponta presa e linha horizontal: a outra só desliza na horizontal
        s.add_constraint(&mut al, Kind::Fixed(a));
        s.add_constraint(&mut al, Kind::Horizontal(l));
        let lib = s.solve(&vars, &[]).liberdade;
        assert!(!lib.pontos.contains_key(&a));
        let d = &lib.pontos[&b];
        assert_eq!(d.len(), 1);
        assert!((d[0][0].abs() - 1.).abs() < 1e-6 && d[0][1].abs() < 1e-6, "{d:?}");

        // com a cota, a seta some
        s.add_constraint(&mut al, Kind::Length(l, "30".into()));
        let rep = s.solve(&vars, &[]);
        assert_eq!(rep.dof, 0);
        assert!(rep.liberdade.pontos.is_empty());

        // círculo: centro e raio, cada um com a sua liberdade
        let c = s.add_point(&mut al, 80., 0.);
        let k = s.add_curve(&mut al, Curve::Circle { c, r: 5. }, false);
        let lib = s.solve(&vars, &[]).liberdade;
        assert!(lib.raios.contains(&k) && lib.pontos.contains_key(&c));
        s.add_constraint(&mut al, Kind::Diameter(k, "10".into()));
        assert!(!s.solve(&vars, &[]).liberdade.raios.contains(&k));
    }

    /// A direção de quem desliza numa linha inclinada acompanha a linha.
    #[test]
    fn liberdade_de_ponto_sobre_linha_inclinada() {
        let mut s = SketchData::default();
        let mut al = alloc();
        let a = s.add_point(&mut al, 0., 0.);
        let b = s.add_point(&mut al, 30., 40.);
        let l = s.add_curve(&mut al, Curve::Line { a, b }, false);
        let p = s.add_point(&mut al, 15., 20.);
        s.add_constraint(&mut al, Kind::Fixed(a));
        s.add_constraint(&mut al, Kind::Fixed(b));
        s.add_constraint(&mut al, Kind::PointOnCurve(p, l));
        let lib = s.solve(&Vars::new(), &[]).liberdade;
        let d = lib.pontos[&p][0];
        assert!(cross(d, [0.6, 0.8]).abs() < 1e-6, "{d:?}");
    }

    /// B273: sem prazo sobrando, o solver desiste sem mexer no sketch.
    #[test]
    fn solve_com_prazo_vencido_nao_muda_nada() {
        let mut s = SketchData::default();
        let mut al = alloc();
        let a = s.add_point(&mut al, 0., 0.);
        let b = s.add_point(&mut al, 30., 5.);
        let l = s.add_curve(&mut al, Curve::Line { a, b }, false);
        s.add_constraint(&mut al, Kind::Horizontal(l));
        let antes = s.clone();
        let rep = s.solve_ate(&Vars::new(), &[], Some(Instant::now()));
        assert!(rep.estourou && !rep.converged);
        assert_eq!(s, antes);
        assert!(!s.solve_ate(&Vars::new(), &[], Some(Instant::now() + std::time::Duration::from_secs(60))).estourou);
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
    fn spline_com_tangente_e_pontos() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let a = s.add_point(&mut al, 0., 0.);
        let b = s.add_point(&mut al, 10., 5.);
        let c = s.add_point(&mut al, 20., 0.);
        let sp = s.add_curve(&mut al, Curve::Spline { pts: vec![a, b, c] }, false);
        assert_eq!(s.spline_uv(sp).len(), 3);
        // tangente horizontal no meio: a curva passa pelo ponto com derivada (k, 0)
        s.set_spline_tangent(sp, b, Some([10., 0.]));
        let u = s.spline_uv(sp);
        assert!(u.len() > 10 && u.iter().any(|q| (q[0] - 10.).abs() < 1e-9 && (q[1] - 5.).abs() < 1e-9));
        // em forma de S (tangentes iguais nas pontas): uma inflexão no meio
        let p = [[0., 0.], [10., 0.]];
        let ev = spline_eval(&p, &[Some([10., 10.]), Some([10., 10.])], 40);
        assert_eq!(inflections(&ev).len(), 1);
        // acrescentar e tirar ponto
        let np = s.spline_insert(&mut al, sp, [15., 3.]).unwrap();
        assert!(matches!(&s.curves.iter().find(|x| x.id == sp).unwrap().geom, Curve::Spline { pts } if pts.len() == 4 && pts[2] == np));
        assert!(s.spline_remove(sp, np));
        assert!(s.points.iter().all(|p| p.id != np));
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
        // pontas no mesmo lugar já fecham o perfil (mesmo sem juntar os pontos)
        assert_eq!(s.loops().0.len(), 1);
        s.merge_points(b2, b);
        s.merge_points(a2, a);
        assert_eq!(s.loops(), (s.loops().0.clone(), 0));
        assert_eq!(s.loops().0.len(), 1);
    }

    fn duas_linhas(al: &mut impl FnMut() -> Id, s: &mut SketchData, graus: f64) -> (Id, Id, Id) {
        let o = s.add_point(al, 0., 0.);
        s.point_mut(o).unwrap().fixed = true;
        let a = s.add_point(al, 20., 0.);
        s.point_mut(a).unwrap().fixed = true;
        let t = graus.to_radians();
        let b = s.add_point(al, 20. * t.cos(), 20. * t.sin());
        let l1 = s.add_curve(al, Curve::Line { a: o, b: a }, false);
        let l2 = s.add_curve(al, Curve::Line { a: o, b }, false);
        (l1, l2, b)
    }

    #[test]
    fn cota_que_sobre_restringe_vira_referencia() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let (p, l) = s.add_rect(&mut al, [0., 0.], [30., 20.], false);
        s.add_constraint(&mut al, Kind::Fixed(p[0]));
        let w = s.add_constraint(&mut al, Kind::Length(l[0], "30".into()));
        assert!(!s.settle_dimension(w, &Vars::new()), "a primeira largura comanda");
        s.add_constraint(&mut al, Kind::Length(l[1], "20".into()));
        // o lado oposto já está definido: com a mesma medida é redundante; com outra, conflito
        for v in ["30", "45"] {
            let d = s.add_constraint(&mut al, Kind::Length(l[2], v.into()));
            assert!(s.settle_dimension(d, &Vars::new()), "{v}");
            let r = s.solve(&Vars::new(), &[]);
            assert!(r.converged && r.redundant == 0 && r.dof == 0, "{r:?}");
            assert_eq!(r.status(), "Totalmente restrito");
            // a referência não comanda: mostra a medida, seja qual for o texto
            let k = s.constraints.iter().find(|c| c.id == d).unwrap().kind.clone();
            assert!((s.measure(&k).unwrap() - 30.).abs() < 1e-9);
            assert!(!s.toggle_reference(d, &Vars::new()), "não pode comandar sobre-restringindo");
            assert!(s.constraints.iter().find(|c| c.id == d).unwrap().reference);
            s.constraints.retain(|c| c.id != d);
        }
    }

    #[test]
    fn menu_alterna_comandar_e_referencia() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let (p, l) = s.add_rect(&mut al, [0., 0.], [30., 20.], false);
        s.add_constraint(&mut al, Kind::Fixed(p[0]));
        let w = s.add_constraint(&mut al, Kind::Length(l[0], "50".into()));
        s.solve(&Vars::new(), &[]);
        assert!((s.pos(p[1])[0] - 50.).abs() < 1e-6);
        assert!(s.toggle_reference(w, &Vars::new()));
        s.point_mut(p[1]).unwrap().x = 35.;
        s.point_mut(p[2]).unwrap().x = 35.;
        let r = s.solve(&Vars::new(), &[]);
        assert!(r.converged && r.dof == 2, "{r:?}");
        // volta a comandar com a medida de agora, sem saltar
        assert!(s.toggle_reference(w, &Vars::new()));
        s.solve(&Vars::new(), &[]);
        assert!((s.pos(p[1])[0] - 35.).abs() < 1e-6, "{:?}", s.pos(p[1]));
        assert!(matches!(&s.constraints.iter().find(|c| c.id == w).unwrap().kind, Kind::Length(_, v) if v == "35"));
    }

    #[test]
    fn referencia_ignora_expressao_invalida_e_le_arquivo_antigo() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let (_, l) = s.add_rect(&mut al, [0., 0.], [30., 20.], false);
        let d = s.add_constraint(&mut al, Kind::Length(l[0], "naoexiste + 1".into()));
        assert!(!s.solve(&Vars::new(), &[]).errors.is_empty());
        s.constraints.iter_mut().find(|c| c.id == d).unwrap().reference = true;
        assert!(s.solve(&Vars::new(), &[]).errors.is_empty());
        let velho = r#"{"id":7,"kind":{"Horizontal":3}}"#;
        let c: Constraint = serde_json::from_str(velho).unwrap();
        assert!(!c.reference && !c.flipped);
        let txt = serde_json::to_string(&c).unwrap();
        assert!(!txt.contains("reference") && !txt.contains("flipped"), "{txt}");
    }

    #[test]
    fn negativo_inverte_comprimento_e_fica_positivo() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let (p, l) = s.add_rect(&mut al, [0., 0.], [30., 20.], false);
        s.add_constraint(&mut al, Kind::Fixed(p[0]));
        let c = s.add_constraint(&mut al, Kind::Length(l[0], "-30".into()));
        let r = s.solve(&Vars::new(), &[]);
        assert!(r.converged, "{r:?}");
        assert!((s.pos(p[1])[0] + 30.).abs() < 1e-6 && (s.pos(p[2])[0] + 30.).abs() < 1e-6, "{:?}", s.pos(p[1]));
        let k = s.constraints.iter().find(|x| x.id == c).unwrap();
        assert!(matches!(&k.kind, Kind::Length(_, v) if v == "30") && !k.flipped);
        // resolver de novo não vira outra vez
        s.solve(&Vars::new(), &[]);
        assert!((s.pos(p[1])[0] + 30.).abs() < 1e-6);
    }

    #[test]
    fn negativo_inverte_distancias() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let a = s.add_point(&mut al, 0., 0.);
        s.point_mut(a).unwrap().fixed = true;
        let b = s.add_point(&mut al, 10., 5.);
        s.add_constraint(&mut al, Kind::DistanceX(a, b, "-10".into()));
        s.add_constraint(&mut al, Kind::DistanceY(a, b, "-5".into()));
        assert!(s.solve(&Vars::new(), &[]).converged);
        assert!((s.pos(b)[0] + 10.).abs() < 1e-6 && (s.pos(b)[1] + 5.).abs() < 1e-6, "{:?}", s.pos(b));
        // ponta presa: quem vira é a outra
        let mut s = SketchData::default();
        let a = s.add_point(&mut al, 0., 0.);
        let b = s.add_point(&mut al, 10., 0.);
        s.point_mut(b).unwrap().fixed = true;
        s.add_constraint(&mut al, Kind::Distance(a, b, "-10".into()));
        assert!(s.solve(&Vars::new(), &[]).converged);
        assert!((s.pos(a)[0] - 20.).abs() < 1e-6);
        // ponto e linha
        let mut s = SketchData::default();
        let (l1, _, _) = duas_linhas(&mut al, &mut s, 90.);
        let p = s.add_point(&mut al, 5., 7.);
        s.add_constraint(&mut al, Kind::PointLineDistance(p, l1, "-7".into()));
        assert!(s.solve(&Vars::new(), &[]).converged);
        assert!((s.pos(p)[1] + 7.).abs() < 1e-6, "{:?}", s.pos(p));
    }

    #[test]
    fn negativo_inverte_angulo() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let (l1, l2, b) = duas_linhas(&mut al, &mut s, 30.);
        s.add_constraint(&mut al, Kind::Length(l2, "20".into()));
        s.add_constraint(&mut al, Kind::Angle(l1, l2, "-30".into()));
        let r = s.solve(&Vars::new(), &[]);
        assert!(r.converged, "{r:?}");
        let q = s.pos(b);
        assert!((q[1] + 10.).abs() < 1e-6 && (q[0] - 17.320508).abs() < 1e-5, "{q:?}");
    }

    #[test]
    fn expressao_negativa_vira_e_volta_com_o_sinal() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let a = s.add_point(&mut al, 0., 0.);
        s.point_mut(a).unwrap().fixed = true;
        let b = s.add_point(&mut al, 10., 0.);
        let c = s.add_constraint(&mut al, Kind::DistanceX(a, b, "L".into()));
        let mut v = Vars::new();
        for (l, x, virada) in [(10., 10., false), (-12., -12., true), (-14., -14., true), (9., 9., false)] {
            v.insert("L".into(), l);
            assert!(s.solve(&v, &[]).converged);
            assert!((s.pos(b)[0] - x).abs() < 1e-6, "{l}: {:?}", s.pos(b));
            let k = s.constraints.iter().find(|x| x.id == c).unwrap();
            assert!(k.flipped == virada && matches!(&k.kind, Kind::DistanceX(_, _, t) if t == "L"));
        }
    }

    #[test]
    fn angulo_com_a_horizontal_tem_sinal() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let a = s.add_point(&mut al, 0., 0.);
        s.point_mut(a).unwrap().fixed = true;
        let b = s.add_point(&mut al, 30., 10.);
        let l = s.add_curve(&mut al, Curve::Line { a, b }, false);
        s.add_constraint(&mut al, Kind::Length(l, "20".into()));
        let c = s.add_constraint(&mut al, Kind::LineAngle(l, "30".into()));
        let r = s.solve(&Vars::new(), &[]);
        assert!(r.converged && r.dof == 0, "{r:?}");
        assert!((s.pos(b)[1] - 10.).abs() < 1e-6);
        let k = s.constraints.iter().find(|x| x.id == c).unwrap().kind.clone();
        assert!((s.measure(&k).unwrap() - 30.).abs() < 1e-6);
        *s.constraints.iter_mut().find(|x| x.id == c).unwrap().kind.value_mut().unwrap() = "-30".into();
        assert!(s.solve(&Vars::new(), &[]).converged);
        assert!((s.pos(b)[1] + 10.).abs() < 1e-6, "{:?}", s.pos(b));
    }

    #[test]
    fn nada_livre_conta_redundancia() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let a = s.add_point(&mut al, 0., 0.);
        let b = s.add_point(&mut al, 10., 0.);
        s.point_mut(a).unwrap().fixed = true;
        s.point_mut(b).unwrap().fixed = true;
        let d = s.add_constraint(&mut al, Kind::Distance(a, b, "10".into()));
        assert!(s.settle_dimension(d, &Vars::new()));
    }

    fn spline_arco(al: &mut impl FnMut() -> Id, s: &mut SketchData) -> (Id, [Id; 3]) {
        let a = s.add_point(al, 0., 0.);
        let b = s.add_point(al, 10., 5.);
        let c = s.add_point(al, 20., 0.);
        (s.add_curve(al, Curve::Spline { pts: vec![a, b, c] }, false), [a, b, c])
    }

    #[test]
    fn spline_tangente_a_linha_no_ponto_comum() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let (sp, p) = spline_arco(&mut al, &mut s);
        let q = s.add_point(&mut al, -10., -10.);
        let l = s.add_curve(&mut al, Curve::Line { a: q, b: p[0] }, false);
        let dof = s.solve(&Vars::new(), &[]).dof;
        s.add_constraint(&mut al, Kind::Tangent(sp, l));
        let r = s.solve(&Vars::new(), &[]);
        assert!(r.converged, "{r:?}");
        assert_eq!(r.dof, dof, "a tangência no ponto comum não prende nenhum ponto");
        // a curva sai da ponta na direção da linha (45°), e o perfil usa essa mesma curva
        let u = s.spline_uv(sp);
        let d = [u[1][0] - u[0][0], u[1][1] - u[0][1]];
        assert!(u.len() > 10 && (d[1] / d[0] - 1.).abs() < 0.08, "{d:?}");
        // a linha gira e a spline acompanha
        s.point_mut(q).unwrap().x = -10.;
        s.point_mut(q).unwrap().y = 0.;
        s.solve(&Vars::new(), &[]);
        let t = s.own_tangents(sp)[0].unwrap();
        assert!(t[1].abs() < 1e-9 && t[0] > 0., "{t:?}");
    }

    #[test]
    fn spline_tangente_a_arco_e_a_outra_spline() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let (sp, p) = spline_arco(&mut al, &mut s);
        // arco de centro (0, -10) que termina em (0, 0): tangente horizontal ali
        let c = s.add_point(&mut al, 0., -10.);
        let i = s.add_point(&mut al, 10., -10.);
        let arc = s.add_curve(&mut al, Curve::Arc { c, s: i, e: p[0] }, false);
        s.add_constraint(&mut al, Kind::Tangent(arc, sp));
        let e = s.add_point(&mut al, 30., 8.);
        let f = s.add_point(&mut al, 40., 0.);
        let sp2 = s.add_curve(&mut al, Curve::Spline { pts: vec![p[2], e, f] }, false);
        s.add_constraint(&mut al, Kind::Tangent(sp, sp2));
        assert!(s.solve(&Vars::new(), &[]).converged);
        let t = s.own_tangents(sp)[0].unwrap();
        assert!(t[1].abs() < 1e-9 && t[0] > 0., "{t:?}");
        let (t1, t2) = (s.direction_at(sp, p[2]).unwrap(), s.own_tangents(sp2)[0].unwrap());
        assert!(cross(t1, t2).abs() < 1e-9 && dot(t1, t2) > 0., "{t1:?} {t2:?}");
    }

    #[test]
    fn spline_encosta_tangente_sem_ponto_comum() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let (sp, p) = spline_arco(&mut al, &mut s);
        s.point_mut(p[0]).unwrap().fixed = true;
        s.point_mut(p[2]).unwrap().fixed = true;
        let a = s.add_point(&mut al, -5., 8.);
        let b = s.add_point(&mut al, 25., 8.);
        s.point_mut(a).unwrap().fixed = true;
        s.point_mut(b).unwrap().fixed = true;
        let l = s.add_curve(&mut al, Curve::Line { a, b }, false);
        let dof = s.solve(&Vars::new(), &[]).dof;
        s.add_constraint(&mut al, Kind::Tangent(l, sp));
        let r = s.solve(&Vars::new(), &[]);
        assert!(r.converged, "{r:?}");
        assert_eq!(r.dof, dof - 1);
        let top = s.spline_uv(sp).iter().map(|q| q[1]).fold(f64::MIN, f64::max);
        assert!((top - 8.).abs() < 1e-2, "topo {top}");

        // círculo acima da spline: ela sobe até encostar nele
        let mut s = SketchData::default();
        let (sp, p) = spline_arco(&mut al, &mut s);
        s.point_mut(p[0]).unwrap().fixed = true;
        s.point_mut(p[2]).unwrap().fixed = true;
        let c = s.add_point(&mut al, 10., 20.);
        s.point_mut(c).unwrap().fixed = true;
        let ci = s.add_curve(&mut al, Curve::Circle { c, r: 9. }, false);
        s.add_constraint(&mut al, Kind::Radius(ci, "9".into()));
        s.add_constraint(&mut al, Kind::Tangent(sp, ci));
        let r = s.solve(&Vars::new(), &[]);
        assert!(r.converged, "{r:?}");
        let near = s.spline_uv(sp).iter().map(|q| len2(sub(*q, [10., 20.]))).fold(f64::MAX, f64::min);
        assert!((near - 9.).abs() < 1e-2, "{near}");
    }

    #[test]
    fn spline_cruzando_sem_tangenciar_nao_passa_por_tangente() {
        // sobe sem parar e as pontas são fixas: não há como ficar tangente à linha que ela cruza
        let mut al = alloc();
        let mut s = SketchData::default();
        let a = s.add_point(&mut al, 0., 0.);
        let b = s.add_point(&mut al, 10., 10.);
        s.point_mut(a).unwrap().fixed = true;
        s.point_mut(b).unwrap().fixed = true;
        let sp = s.add_curve(&mut al, Curve::Spline { pts: vec![a, b] }, false);
        let (c, d) = (s.add_point(&mut al, -5., 5.), s.add_point(&mut al, 15., 5.));
        s.point_mut(c).unwrap().fixed = true;
        s.point_mut(d).unwrap().fixed = true;
        let l = s.add_curve(&mut al, Curve::Line { a: c, b: d }, false);
        s.add_constraint(&mut al, Kind::Tangent(sp, l));
        assert!(!s.solve(&Vars::new(), &[]).converged);
    }

    #[test]
    fn plano_acha_todos_os_pontos() {
        let (o, n) = ([0., 0., 0.], [0., 0., 1.]);
        // círculo de outro plano: duas travessias
        let c: Vec<[f64; 3]> = (0..=48).map(|i| {
            let t = i as f64 / 48. * std::f64::consts::TAU + 0.1;
            [10. * t.cos(), 0., 10. * t.sin()]
        }).collect();
        assert_eq!(plane_crossings(&c, o, n).len(), 2);
        // retângulo em pé com um lado deitado no plano: as duas pontas do lado
        let r = [[0., 0., 0.], [10., 0., 0.], [10., 0., 10.], [0., 0., 10.], [0., 0., 0.]];
        let q = plane_crossings(&r, o, n);
        assert_eq!(q.len(), 2, "{q:?}");
        // polilinha que termina exatamente no plano
        assert_eq!(plane_crossings(&[[0., 0., -5.], [0., 0., 0.]], o, n), [[0., 0., 0.]]);
        // zigue-zague: três travessias
        assert_eq!(plane_crossings(&[[0., 0., -1.], [1., 0., 1.], [2., 0., -1.], [3., 0., 1.]], o, n).len(), 3);
        assert!(plane_crossings(&[[0., 0., 1.], [1., 0., 2.]], o, n).is_empty());
    }

    #[test]
    fn pontos_de_intersecao_ficam_soltos_e_acompanham() {
        let mut al = alloc();
        let mut s = SketchData::default();
        let id = s.add_link_points(&mut al, LinkSrc::Ref(Id(1)), &[[1., 2.], [5., 6.]], false).unwrap();
        assert_eq!(s.hole_points().len(), 2);
        assert!(!s.curves.iter().any(|c| matches!(c.geom, Curve::Line { .. })), "sem linha entre os pontos");
        s.update_link(id, &[[3., 3.], [7., 7.]]);
        assert_eq!(s.hole_points(), [[3., 3.], [7., 7.]]);
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
