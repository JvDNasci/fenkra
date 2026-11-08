// SPDX-License-Identifier: LGPL-2.1-or-later
//! B352: sketch no desenho, dentro de uma vista ou solto na folha.
//!
//! Serve para o detalhe a mais e a geometria de apoio que não vêm do 3D. O conteúdo é um
//! `SketchData` comum, com as restrições e o resolvedor do sketch de peça.
//!
//! - **Na vista**: as coordenadas são as do plano da vista, em mm do modelo (as mesmas da
//!   projeção). O esboço anda com a vista e segue a escala dela: a cota mostra a medida do modelo.
//! - **Na folha**: as coordenadas são mm da folha, sem escala.
//!
//! Cada curva tem um tipo de linha (visível, fina, oculta, de centro); a de construção não sai na
//! folha. As cotas do sketch (distância, horizontal, vertical, comprimento, raio, diâmetro) saem
//! como cotas do desenho. Tudo vira primitiva da folha em `layout`, então entra na exportação.

use crate::source::Models;
use crate::{Drawing, Id, V2};
use fenkra_core::sketch::{Curve, Kind};
use fenkra_core::SketchData;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Onde o esboço fica preso.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DonoEsboco {
    /// solto na folha (mm da folha)
    Folha(Id),
    /// dentro da vista (mm do modelo no plano da vista)
    Vista(Id),
}

/// Tipo de linha de uma curva do esboço.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoLinha {
    #[default]
    Visivel,
    Fina,
    Oculta,
    Centro,
}

impl TipoLinha {
    pub const TODOS: [TipoLinha; 4] = [TipoLinha::Visivel, TipoLinha::Fina, TipoLinha::Oculta, TipoLinha::Centro];
    pub fn label(self) -> &'static str {
        match self {
            TipoLinha::Visivel => "Visível",
            TipoLinha::Fina => "Fina",
            TipoLinha::Oculta => "Oculta",
            TipoLinha::Centro => "De centro",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EsbocoDesenho {
    pub id: Id,
    pub nome: String,
    pub dono: DonoEsboco,
    pub dados: SketchData,
    /// tipo de linha de cada curva; a que não está aqui é visível
    #[serde(default)]
    pub tipos: Vec<(Id, TipoLinha)>,
    /// mostra as cotas do sketch na folha
    #[serde(default = "sim")]
    pub cotas: bool,
}

fn sim() -> bool {
    true
}

impl EsbocoDesenho {
    pub fn tipo(&self, curva: Id) -> TipoLinha {
        self.tipos.iter().find(|(c, _)| *c == curva).map_or(TipoLinha::Visivel, |(_, t)| *t)
    }

    pub fn definir_tipo(&mut self, curva: Id, t: TipoLinha) {
        self.tipos.retain(|(c, _)| *c != curva);
        if t != TipoLinha::Visivel {
            self.tipos.push((curva, t));
        }
    }

    /// Resolve as restrições (cotas sem variáveis do documento). Devolve se convergiu.
    pub fn resolver(&mut self) -> bool {
        self.dados.solve(&HashMap::new(), &[]).converged
    }
}

/// Traço de uma curva no plano do esboço: polilinha e tipo.
pub fn tracos(e: &EsbocoDesenho) -> Vec<(Vec<V2>, TipoLinha)> {
    let s = &e.dados;
    let mut out = vec![];
    for c in s.curves.iter().filter(|c| !c.construction) {
        let pts: Vec<V2> = match &c.geom {
            Curve::Line { a, b } => vec![s.pos(*a), s.pos(*b)],
            Curve::Circle { c: cc, r } => arco(s.pos(*cc), *r, 0., std::f64::consts::TAU),
            Curve::Arc { c: cc, s: a, e: b } => {
                let (o, p, q) = (s.pos(*cc), s.pos(*a), s.pos(*b));
                let r = ((p[0] - o[0]).powi(2) + (p[1] - o[1]).powi(2)).sqrt();
                let t0 = (p[1] - o[1]).atan2(p[0] - o[0]);
                let mut t1 = (q[1] - o[1]).atan2(q[0] - o[0]);
                if t1 <= t0 {
                    t1 += std::f64::consts::TAU;
                }
                arco(o, r, t0, t1)
            }
            Curve::Spline { .. } => s.spline_uv(c.id),
            Curve::Ellipse { c: cc, major, minor, t0, t1, .. } => {
                let o = s.pos(*cc);
                let a = (major[0].powi(2) + major[1].powi(2)).sqrt().max(1e-12);
                let u = [major[0] / a, major[1] / a];
                let v = [-u[1], u[0]];
                let n = 64;
                (0..=n)
                    .map(|i| {
                        let t = t0 + (t1 - t0) * i as f64 / n as f64;
                        let (sn, cs) = t.sin_cos();
                        [o[0] + u[0] * a * cs + v[0] * minor * sn, o[1] + u[1] * a * cs + v[1] * minor * sn]
                    })
                    .collect()
            }
            Curve::Point { .. } => continue,
        };
        out.push((pts, e.tipo(c.id)));
    }
    out
}

fn arco(c: V2, r: f64, t0: f64, t1: f64) -> Vec<V2> {
    let n = (((t1 - t0).abs() / std::f64::consts::TAU) * 72.).ceil().max(8.) as usize;
    (0..=n)
        .map(|i| {
            let t = t0 + (t1 - t0) * i as f64 / n as f64;
            [c[0] + r * t.cos(), c[1] + r * t.sin()]
        })
        .collect()
}

/// Cota do esboço já no plano dele: tipo, pontos (como os da cota do desenho) e valor medido.
#[derive(Clone, Debug, PartialEq)]
pub struct CotaEsboco {
    pub tipo: crate::DimKind,
    pub pts: [V2; 2],
    pub valor: f64,
    pub referencia: bool,
}

/// Cotas do sketch que viram cotas da folha. O valor é o medido (a geometria já resolvida).
pub fn cotas(e: &EsbocoDesenho) -> Vec<CotaEsboco> {
    use crate::DimKind;
    let s = &e.dados;
    let dist = |a: V2, b: V2| ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    let linha = |id: Id| s.curves.iter().find(|c| c.id == id).and_then(|c| if let Curve::Line { a, b } = c.geom { Some((s.pos(a), s.pos(b))) } else { None });
    let circulo = |id: Id| {
        s.curves.iter().find(|c| c.id == id).and_then(|c| match c.geom {
            Curve::Circle { c: o, r } => Some((s.pos(o), r)),
            Curve::Arc { c: o, s: a, .. } => Some((s.pos(o), dist(s.pos(o), s.pos(a)))),
            _ => None,
        })
    };
    let mut out = vec![];
    for k in &s.constraints {
        let c = match &k.kind {
            Kind::Distance(a, b, _) => Some((DimKind::Aligned, [s.pos(*a), s.pos(*b)])),
            Kind::DistanceX(a, b, _) => Some((DimKind::Horizontal, [s.pos(*a), s.pos(*b)])),
            Kind::DistanceY(a, b, _) => Some((DimKind::Vertical, [s.pos(*a), s.pos(*b)])),
            Kind::Length(l, _) => linha(*l).map(|(a, b)| (DimKind::Aligned, [a, b])),
            Kind::Radius(c, _) => circulo(*c).map(|(o, r)| (DimKind::Radius, [o, [o[0] + r * 0.7071, o[1] + r * 0.7071]])),
            Kind::Diameter(c, _) => circulo(*c).map(|(o, r)| (DimKind::Diameter, [o, [o[0] + r * 0.7071, o[1] + r * 0.7071]])),
            _ => None,
        };
        let Some((tipo, pts)) = c else { continue };
        let valor = match tipo {
            DimKind::Horizontal => (pts[1][0] - pts[0][0]).abs(),
            DimKind::Vertical => (pts[1][1] - pts[0][1]).abs(),
            DimKind::Diameter => dist(pts[0], pts[1]) * 2.,
            _ => dist(pts[0], pts[1]),
        };
        out.push(CotaEsboco { tipo, pts, valor, referencia: k.reference });
    }
    out
}

/// Ponto do esboço na folha: na vista, pela escala e posição dela; na folha, ele mesmo. `None`
/// se o dono sumiu ou a vista ainda não tem projeção.
pub fn na_folha(d: &Drawing, e: &EsbocoDesenho, m: &mut Models) -> Option<Box<dyn Fn(V2) -> V2>> {
    match e.dono {
        DonoEsboco::Folha(_) => Some(Box::new(|q| q)),
        DonoEsboco::Vista(id) => {
            let v = d.view(id)?.clone();
            let p = m.proj(d, id);
            if p.calculando {
                return None;
            }
            Some(Box::new(move |q| crate::layout::to_sheet(&v, &p, q)))
        }
    }
}

/// Folha em que o esboço aparece.
pub fn folha(d: &Drawing, e: &EsbocoDesenho) -> Option<Id> {
    match e.dono {
        DonoEsboco::Folha(s) => Some(s),
        DonoEsboco::Vista(v) => d.view(v).map(|v| v.sheet),
    }
}

/// Fator da folha para o modelo: a cota na vista mostra a medida do modelo.
pub fn escala(d: &Drawing, e: &EsbocoDesenho) -> f64 {
    match e.dono {
        DonoEsboco::Folha(_) => 1.,
        DonoEsboco::Vista(v) => d.view(v).map_or(1., |v| v.scale),
    }
}
