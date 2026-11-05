// SPDX-License-Identifier: LGPL-2.1-or-later
//! Relimitação do sketch (B18): aparar e estender em linha, arco, círculo e spline.
//!
//! Toda curva vira um `Traco`, um caminho parametrizado de 0 a 1. Cruzar dois traços dá os pontos de
//! corte; o trecho sob o cursor é o que fica entre os cortes vizinhos. A prévia e o clique usam o
//! mesmo plano (`plano_aparar`, `plano_estender`): o que aparece em vermelho é o que sai de verdade.
//! A ponta nova guarda o vínculo com a curva que a cortou (`PointOnCurve`), então ela acompanha o
//! limite quando o sketch muda.

use crate::sketch_ui::Tool;
use fenkra_core::sketch::{spline_eval, spline_tangents, Curve, Kind, SkCurve};
use fenkra_core::{Id, SketchData};
use std::collections::HashSet;
use std::f64::consts::TAU;

type P = [f64; 2];

/// Folga de comprimento para dizer que um ponto toca a curva: o solver entrega pontas a ~1e-9 do
/// limite, e uma folga exata perderia o corte num encontro em T resolvido.
const FOLGA: f64 = 1e-6;
/// Amostras por trecho de spline nos cruzamentos.
const POR_TRECHO: usize = 24;

fn dist(a: P, b: P) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn gira(x: f64) -> f64 {
    x.rem_euclid(TAU)
}

/// Caminho parametrizado de 0 a 1 de uma curva do sketch.
#[derive(Clone, Debug)]
pub enum Traco {
    Reta { a: P, b: P },
    /// `varre` com sinal: positivo é anti-horário. Círculo: `varre` = uma volta.
    Arco { c: P, r: f64, a0: f64, varre: f64 },
    /// Polilinha densa (spline); `acum` é o comprimento acumulado em cada ponto.
    Poli { pts: Vec<P>, acum: Vec<f64> },
}

impl Traco {
    pub fn poli(pts: Vec<P>) -> Self {
        let mut acum = vec![0.];
        for w in pts.windows(2) {
            acum.push(acum.last().unwrap() + dist(w[0], w[1]));
        }
        Traco::Poli { pts, acum }
    }

    pub fn fechado(&self) -> bool {
        matches!(self, Traco::Arco { varre, .. } if varre.abs() >= TAU - 1e-12)
    }

    pub fn comprimento(&self) -> f64 {
        match self {
            Traco::Reta { a, b } => dist(*a, *b),
            Traco::Arco { r, varre, .. } => r * varre.abs(),
            Traco::Poli { acum, .. } => *acum.last().unwrap_or(&0.),
        }
    }

    /// Folga em unidades do parâmetro.
    fn folga_u(&self) -> f64 {
        FOLGA / self.comprimento().max(1e-9)
    }

    pub fn em(&self, u: f64) -> P {
        match self {
            Traco::Reta { a, b } => [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u],
            Traco::Arco { c, r, a0, varre } => {
                let t = a0 + varre * u;
                [c[0] + r * t.cos(), c[1] + r * t.sin()]
            }
            Traco::Poli { pts, acum } => {
                let (total, n) = (*acum.last().unwrap_or(&0.), pts.len());
                if n < 2 || total <= 0. {
                    return pts.first().copied().unwrap_or([0., 0.]);
                }
                let s = u.clamp(0., 1.) * total;
                let k = acum.partition_point(|x| *x < s).clamp(1, n - 1);
                let d = (acum[k] - acum[k - 1]).max(1e-15);
                let t = (s - acum[k - 1]) / d;
                [pts[k - 1][0] + (pts[k][0] - pts[k - 1][0]) * t, pts[k - 1][1] + (pts[k][1] - pts[k - 1][1]) * t]
            }
        }
    }

    /// Parâmetro do ponto do traço mais perto de `p`. Fora do arco, passa de 1.
    pub fn param(&self, p: P) -> f64 {
        match self {
            Traco::Reta { a, b } => {
                let d = [b[0] - a[0], b[1] - a[1]];
                ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]).max(1e-18)
            }
            Traco::Arco { c, a0, varre, .. } => {
                let ang = (p[1] - c[1]).atan2(p[0] - c[0]);
                let mut d = if *varre >= 0. { gira(ang - a0) } else { gira(a0 - ang) };
                // um fio antes do início é o próprio início
                if TAU - d < 1e-9 {
                    d = 0.;
                }
                d / varre.abs().max(1e-12)
            }
            Traco::Poli { pts, acum } => {
                let (mut melhor, mut u) = (f64::MAX, 0.);
                for (k, w) in pts.windows(2).enumerate() {
                    let t = Traco::Reta { a: w[0], b: w[1] }.param(p).clamp(0., 1.);
                    let q = [w[0][0] + (w[1][0] - w[0][0]) * t, w[0][1] + (w[1][1] - w[0][1]) * t];
                    let d = dist(p, q);
                    if d < melhor {
                        melhor = d;
                        u = acum[k] + (acum[k + 1] - acum[k]) * t;
                    }
                }
                u / acum.last().copied().unwrap_or(1.).max(1e-12)
            }
        }
    }

    /// O ponto `p`, já sobre a curva de base, está dentro do trecho?
    fn contem(&self, p: P) -> bool {
        let (u, f) = (self.param(p), self.folga_u());
        match self {
            Traco::Arco { .. } if self.fechado() => true,
            // pelo fim do arco, o parâmetro de um ponto logo antes do início passa de 1
            Traco::Arco { varre, .. } => u <= 1. + f || (TAU / varre.abs() - u) <= f,
            _ => (-f..=1. + f).contains(&u),
        }
    }

    /// Polilinha do trecho de `lo` a `hi`. Num traço fechado, `hi < lo` dá a volta pelo início.
    pub fn trecho(&self, lo: f64, hi: f64) -> Vec<P> {
        let hi = if hi < lo { hi + 1. } else { hi };
        match self {
            Traco::Reta { .. } => vec![self.em(lo), self.em(hi)],
            Traco::Arco { varre, .. } => {
                let n = ((varre.abs() * (hi - lo)) / 0.08).ceil().clamp(2., 256.) as usize;
                (0..=n).map(|i| self.em(lo + (hi - lo) * i as f64 / n as f64)).collect()
            }
            Traco::Poli { pts, acum } => {
                let total = acum.last().copied().unwrap_or(0.).max(1e-12);
                let mut v = vec![self.em(lo)];
                v.extend(pts.iter().zip(acum).filter(|(_, s)| **s / total > lo && **s / total < hi).map(|(p, _)| *p));
                v.push(self.em(hi));
                v
            }
        }
    }
}

/// Traço da curva. Ponto isolado não tem caminho.
pub fn traco(sk: &SketchData, id: Id) -> Option<Traco> {
    Some(match &sk.curve(id)?.geom {
        Curve::Line { a, b } => Traco::Reta { a: sk.pos(*a), b: sk.pos(*b) },
        Curve::Circle { c, r } => Traco::Arco { c: sk.pos(*c), r: *r, a0: 0., varre: TAU },
        Curve::Arc { c, s, e } => {
            let (pc, ps, pe) = (sk.pos(*c), sk.pos(*s), sk.pos(*e));
            let a0 = (ps[1] - pc[1]).atan2(ps[0] - pc[0]);
            let a1 = (pe[1] - pc[1]).atan2(pe[0] - pc[0]);
            let v = gira(a1 - a0);
            Traco::Arco { c: pc, r: dist(ps, pc), a0, varre: if v < 1e-12 { TAU } else { v } }
        }
        Curve::Spline { .. } => Traco::poli(spline_eval(&sk.spline_uv(id), &sk.own_tangents(id), POR_TRECHO)),
        Curve::Point { .. } => return None,
    })
}

fn reta_reta(a: P, b: P, c: P, d: P) -> Option<P> {
    let (r, s) = ([b[0] - a[0], b[1] - a[1]], [d[0] - c[0], d[1] - c[1]]);
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-14 * (r[0].hypot(r[1]) * s[0].hypot(s[1])).max(1e-30) {
        return None;
    }
    let t = ((c[0] - a[0]) * s[1] - (c[1] - a[1]) * s[0]) / den;
    Some([a[0] + r[0] * t, a[1] + r[1] * t])
}

fn reta_circulo(a: P, b: P, c: P, r: f64) -> Vec<P> {
    let d = [b[0] - a[0], b[1] - a[1]];
    let l = d[0].hypot(d[1]);
    if l < 1e-15 {
        return vec![];
    }
    let u = [d[0] / l, d[1] / l];
    let t0 = (c[0] - a[0]) * u[0] + (c[1] - a[1]) * u[1];
    let pe = [a[0] + u[0] * t0, a[1] + u[1] * t0];
    let h = dist(pe, c);
    if h > r + FOLGA {
        return vec![];
    }
    // tangente: um ponto só, mesmo com o erro de arredondamento
    if (h - r).abs() <= FOLGA {
        return vec![pe];
    }
    let m = (r * r - h * h).sqrt();
    vec![[pe[0] - u[0] * m, pe[1] - u[1] * m], [pe[0] + u[0] * m, pe[1] + u[1] * m]]
}

fn circulo_circulo(c1: P, r1: f64, c2: P, r2: f64) -> Vec<P> {
    let d = dist(c1, c2);
    if d < 1e-12 || d > r1 + r2 + FOLGA || d < (r1 - r2).abs() - FOLGA {
        return vec![];
    }
    let a = (d * d + r1 * r1 - r2 * r2) / (2. * d);
    let h2 = r1 * r1 - a * a;
    let u = [(c2[0] - c1[0]) / d, (c2[1] - c1[1]) / d];
    let m = [c1[0] + u[0] * a, c1[1] + u[1] * a];
    if h2 <= FOLGA * FOLGA {
        return vec![m];
    }
    let h = h2.sqrt();
    vec![[m[0] - u[1] * h, m[1] + u[0] * h], [m[0] + u[1] * h, m[1] - u[0] * h]]
}

fn caixa(pts: &[P]) -> (P, P) {
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for p in pts {
        for i in 0..2 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    (lo, hi)
}

/// Pontos em que os dois traços se encontram.
pub fn cruzamentos(x: &Traco, y: &Traco) -> Vec<P> {
    match (x, y) {
        (Traco::Poli { pts, .. }, o) | (o, Traco::Poli { pts, .. }) => {
            // o outro lado, se também for polilinha, cai aqui de novo em cada trecho
            let caixa_o = match o {
                Traco::Poli { pts: q, .. } => Some(caixa(q)),
                _ => None,
            };
            let mut v = vec![];
            for w in pts.windows(2) {
                if let Some((lo, hi)) = caixa_o {
                    let (a, b) = caixa(w);
                    if a[0] > hi[0] + FOLGA || a[1] > hi[1] + FOLGA || b[0] < lo[0] - FOLGA || b[1] < lo[1] - FOLGA {
                        continue;
                    }
                }
                v.extend(cruzamentos(&Traco::Reta { a: w[0], b: w[1] }, o));
            }
            v
        }
        (Traco::Reta { a, b }, Traco::Reta { a: c, b: d }) => reta_reta(*a, *b, *c, *d).into_iter().filter(|p| x.contem(*p) && y.contem(*p)).collect(),
        (Traco::Reta { a, b }, Traco::Arco { c, r, .. }) | (Traco::Arco { c, r, .. }, Traco::Reta { a, b }) => reta_circulo(*a, *b, *c, *r).into_iter().filter(|p| x.contem(*p) && y.contem(*p)).collect(),
        (Traco::Arco { c: c1, r: r1, .. }, Traco::Arco { c: c2, r: r2, .. }) => circulo_circulo(*c1, *r1, *c2, *r2).into_iter().filter(|p| x.contem(*p) && y.contem(*p)).collect(),
    }
}

/// Um ponto em que outra curva corta a curva dada.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corte {
    pub u: f64,
    pub p: P,
    /// a curva que corta
    pub por: Id,
}

/// Cortes de um traço pelas outras curvas do sketch, em ordem de parâmetro, sem repetição.
fn cortes_em(sk: &SketchData, t: &Traco, ignora: Id) -> Vec<Corte> {
    let mut v: Vec<Corte> = vec![];
    for c in sk.curves.iter().filter(|c| c.id != ignora) {
        let Some(o) = traco(sk, c.id) else { continue };
        v.extend(cruzamentos(t, &o).into_iter().map(|p| Corte { u: t.param(p), p, por: c.id }));
    }
    v.sort_by(|a, b| a.u.total_cmp(&b.u));
    v.dedup_by(|b, a| dist(a.p, b.p) < FOLGA);
    v
}

/// Pontos de corte da curva. As pontas de um traço aberto não cortam nada.
pub fn cortes(sk: &SketchData, curva: Id) -> Vec<Corte> {
    let Some(t) = traco(sk, curva) else { return vec![] };
    let f = t.folga_u();
    let mut v = cortes_em(sk, &t, curva);
    if !t.fechado() {
        v.retain(|c| c.u > f && c.u < 1. - f);
    }
    v
}

/// O que o aparar vai fazer no trecho sob o cursor.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanoAparar {
    pub curva: Id,
    /// parâmetros do trecho que sai. Num círculo, `hi < lo` quando ele passa pelo início.
    pub lo: f64,
    pub hi: f64,
    /// polilinha do trecho que sai
    pub sai: Vec<P>,
    /// todos os pontos de corte da curva
    pub cortes: Vec<P>,
    /// os cortes que limitam o trecho: de 0 (a curva sai inteira) a 2
    pub limites: Vec<Corte>,
}

/// O trecho entre os cortes vizinhos do cursor.
pub fn plano_aparar(sk: &SketchData, curva: Id, cursor: P) -> Option<PlanoAparar> {
    let t = traco(sk, curva)?;
    let cs = cortes(sk, curva);
    let pontos: Vec<P> = cs.iter().map(|c| c.p).collect();
    let u = t.param(cursor).clamp(0., 1.);
    if t.fechado() {
        // com menos de dois cortes não há trecho: a volta sai inteira
        if cs.len() < 2 {
            return Some(PlanoAparar { curva, lo: 0., hi: 1., sai: t.trecho(0., 1.), cortes: pontos, limites: vec![] });
        }
        let antes = *cs.iter().rev().find(|c| c.u <= u).unwrap_or(cs.last()?);
        let depois = *cs.iter().find(|c| c.u > u).unwrap_or(cs.first()?);
        return Some(PlanoAparar { curva, lo: antes.u, hi: depois.u, sai: t.trecho(antes.u, depois.u), cortes: pontos, limites: vec![antes, depois] });
    }
    let antes = cs.iter().rev().find(|c| c.u < u).copied();
    let depois = cs.iter().find(|c| c.u > u).copied();
    let (lo, hi) = (antes.map_or(0., |c| c.u), depois.map_or(1., |c| c.u));
    Some(PlanoAparar { curva, lo, hi, sai: t.trecho(lo, hi), cortes: pontos, limites: antes.into_iter().chain(depois).collect() })
}

/// Remove os pontos que ninguém mais usa (os fixos ficam) e as restrições que ficaram sem alvo.
fn limpa_soltos(sk: &mut SketchData) {
    let usados: HashSet<Id> = sk.curves.iter().flat_map(|c| SketchData::curve_points(&c.geom)).collect();
    sk.points.retain(|p| p.fixed || usados.contains(&p.id));
    sk.prune_constraints();
}

fn troca_geom(sk: &mut SketchData, curva: Id, geom: Curve) {
    if let Some(c) = sk.curves.iter_mut().find(|c| c.id == curva) {
        c.geom = geom;
    }
}

/// A linha passa a ter estas pontas: a cota de comprimento antiga não vale mais.
fn troca_linha(sk: &mut SketchData, linha: Id, a: Id, b: Id) {
    troca_geom(sk, linha, Curve::Line { a, b });
    sk.constraints.retain(|k| !matches!(k.kind, Kind::Length(l, _) if l == linha));
}

/// Ponto novo no corte, preso à curva que cortou: a ponta segue o limite quando ele se mexe.
/// Spline não tem equação de ponto sobre a curva no solver: lá a ponta fica só na posição.
fn ponto_no_corte(sk: &mut SketchData, al: &mut impl FnMut() -> Id, c: Corte) -> Id {
    let id = sk.add_point(al, c.p[0], c.p[1]);
    if !matches!(sk.curve(c.por).map(|x| &x.geom), Some(Curve::Spline { .. })) {
        sk.add_constraint(al, Kind::PointOnCurve(id, c.por));
    }
    id
}

/// A curva nova herda da antiga as relações de direção: um trecho de linha horizontal segue horizontal.
fn herda_direcao(sk: &mut SketchData, al: &mut impl FnMut() -> Id, de: Id, para: Id) {
    let novas: Vec<Kind> = sk
        .constraints
        .iter()
        .filter_map(|k| match k.kind {
            Kind::Horizontal(l) if l == de => Some(Kind::Horizontal(para)),
            Kind::Vertical(l) if l == de => Some(Kind::Vertical(para)),
            Kind::Parallel(a, b) if a == de || b == de => Some(Kind::Parallel(para, if a == de { b } else { a })),
            Kind::Perpendicular(a, b) if a == de || b == de => Some(Kind::Perpendicular(para, if a == de { b } else { a })),
            _ => None,
        })
        .collect();
    for k in novas {
        sk.add_constraint(al, k);
    }
}

/// Apara o trecho sob o cursor. É sempre o trecho do `plano_aparar`.
pub fn aparar(sk: &mut SketchData, al: &mut impl FnMut() -> Id, curva: Id, cursor: P) -> Result<(), &'static str> {
    let Some(plano) = plano_aparar(sk, curva, cursor) else { return Err("esse elemento não tem trecho para aparar") };
    let Some(SkCurve { geom, construction, .. }) = sk.curve(curva).cloned() else { return Err("esse elemento não tem trecho para aparar") };
    let antes = plano.limites.iter().find(|c| c.u == plano.lo).copied();
    let depois = plano.limites.iter().find(|c| c.u == plano.hi).copied();
    match (geom, antes, depois) {
        (Curve::Line { .. } | Curve::Arc { .. } | Curve::Circle { .. } | Curve::Spline { .. }, None, None) => sk.remove_curve(curva),
        (Curve::Line { a, .. }, Some(c), None) => {
            let n = ponto_no_corte(sk, al, c);
            troca_linha(sk, curva, a, n);
        }
        (Curve::Line { b, .. }, None, Some(c)) => {
            let n = ponto_no_corte(sk, al, c);
            troca_linha(sk, curva, n, b);
        }
        (Curve::Line { a, b }, Some(c1), Some(c2)) => {
            let (n1, n2) = (ponto_no_corte(sk, al, c1), ponto_no_corte(sk, al, c2));
            troca_linha(sk, curva, a, n1);
            let nova = sk.add_curve(al, Curve::Line { a: n2, b }, construction);
            herda_direcao(sk, al, curva, nova);
        }
        (Curve::Arc { c, s, .. }, Some(k), None) => {
            let n = ponto_no_corte(sk, al, k);
            troca_geom(sk, curva, Curve::Arc { c, s, e: n });
        }
        (Curve::Arc { c, e, .. }, None, Some(k)) => {
            let n = ponto_no_corte(sk, al, k);
            troca_geom(sk, curva, Curve::Arc { c, s: n, e });
        }
        (Curve::Arc { c, s, e }, Some(k1), Some(k2)) => {
            let (n1, n2) = (ponto_no_corte(sk, al, k1), ponto_no_corte(sk, al, k2));
            troca_geom(sk, curva, Curve::Arc { c, s, e: n1 });
            let nova = sk.add_curve(al, Curve::Arc { c, s: n2, e }, construction);
            // o raio do arco é implícito: os dois trechos seguem no mesmo círculo
            sk.add_constraint(al, Kind::Equal(curva, nova));
        }
        // o círculo vira o arco que sobra, do fim do trecho que sai até o começo dele
        (Curve::Circle { c, .. }, Some(k1), Some(k2)) => {
            let (n1, n2) = (ponto_no_corte(sk, al, k1), ponto_no_corte(sk, al, k2));
            troca_geom(sk, curva, Curve::Arc { c, s: n2, e: n1 });
        }
        (Curve::Spline { pts }, antes, depois) => aparar_spline(sk, al, curva, &pts, construction, antes, depois),
        (Curve::Circle { .. }, ..) | (Curve::Point { .. }, ..) => return Err("esse elemento não tem trecho para aparar"),
    }
    limpa_soltos(sk);
    Ok(())
}

/// A spline perde o trecho entre os cortes. As tangentes dos pontos que ficam são gravadas, porque a
/// tangente automática depende dos vizinhos e mudaria o que sobrou. A forma só muda no trecho que foi
/// cortado ao meio: a tangente é uma por ponto e o pedaço menor pediria outra escala.
fn aparar_spline(sk: &mut SketchData, al: &mut impl FnMut() -> Id, curva: Id, pts: &[Id], construction: bool, antes: Option<Corte>, depois: Option<Corte>) {
    let uv: Vec<P> = pts.iter().map(|q| sk.pos(*q)).collect();
    let m = spline_tangents(&uv, &sk.own_tangents(curva));
    let Some(t @ Traco::Poli { .. }) = traco(sk, curva) else { return };
    let Traco::Poli { acum, .. } = &t else { return };
    let total = acum.last().copied().unwrap_or(1.).max(1e-12);
    let u_no: Vec<f64> = (0..pts.len()).map(|k| acum[k * POR_TRECHO] / total).collect();
    // tangente da ponta nova: direção da curva, com o tamanho da corda até o ponto vizinho
    let ponta = |c: &Corte, viz: P| {
        let (a, b) = (t.em((c.u - 1e-4).max(0.)), t.em((c.u + 1e-4).min(1.)));
        let l = dist(a, b).max(1e-12);
        let k = dist(c.p, viz) / l;
        [(b[0] - a[0]) * k, (b[1] - a[1]) * k]
    };
    // o ponto vizinho ao corte serve a um trecho inteiro e a um encurtado: a tangente fica na média
    // das duas escalas, para o erro se dividir em vez de cair todo no trecho menor
    let ajusta = |tan: [f64; 2], no: P, longe: P, c: &Corte| {
        let k = (1. + (dist(no, c.p) / dist(no, longe).max(1e-12)).min(1.)) / 2.;
        [tan[0] * k, tan[1] * k]
    };
    sk.spline_tan.retain(|x| x.curve != curva);
    let mut pedacos: Vec<Vec<(Id, [f64; 2])>> = vec![];
    if let Some(c) = antes {
        let fica = u_no.iter().filter(|u| **u < c.u).count().max(1);
        let mut v: Vec<(Id, [f64; 2])> = (0..fica).map(|k| (pts[k], m[k])).collect();
        v[fica - 1].1 = ajusta(m[fica - 1], uv[fica - 1], uv[fica.min(uv.len() - 1)], &c);
        let tan = ponta(&c, uv[fica - 1]);
        v.push((ponto_no_corte(sk, al, c), tan));
        pedacos.push(v);
    }
    if let Some(c) = depois {
        let de = u_no.iter().position(|u| *u > c.u).unwrap_or(pts.len() - 1);
        let mut v: Vec<(Id, [f64; 2])> = (de..pts.len()).map(|k| (pts[k], m[k])).collect();
        v[0].1 = ajusta(m[de], uv[de], uv[de.saturating_sub(1)], &c);
        let tan = ponta(&c, uv[de]);
        v.insert(0, (ponto_no_corte(sk, al, c), tan));
        pedacos.push(v);
    }
    for (i, p) in pedacos.into_iter().enumerate() {
        let ids: Vec<Id> = p.iter().map(|x| x.0).collect();
        let alvo = if i == 0 {
            troca_geom(sk, curva, Curve::Spline { pts: ids });
            curva
        } else {
            sk.add_curve(al, Curve::Spline { pts: ids }, construction)
        };
        for (id, tan) in p {
            sk.set_spline_tangent(alvo, id, Some(tan));
        }
    }
}

/// O que o estender vai fazer na ponta perto do cursor.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanoEstender {
    pub curva: Id,
    /// ponta que sai do lugar
    pub ponta: Id,
    /// onde ela chega
    pub destino: P,
    /// a curva que a recebe
    pub alvo: Id,
    /// polilinha do trecho que nasce
    pub novo: Vec<P>,
}

/// Distância que basta para sair de qualquer curva do sketch, olhando de `de`.
fn alcance(sk: &SketchData, de: P) -> f64 {
    let longe = sk.points.iter().map(|p| dist(de, [p.x, p.y])).fold(0., f64::max);
    let raio = sk.curves.iter().filter_map(|c| if let Curve::Circle { r, .. } = c.geom { Some(r) } else { None }).fold(0., f64::max);
    2. * (longe + raio) + 1.
}

/// A ponta mais perto do cursor vai até a primeira curva no caminho, seguindo a linha ou o círculo.
pub fn plano_estender(sk: &SketchData, curva: Id, cursor: P) -> Result<PlanoEstender, &'static str> {
    let (ponta, caminho) = match sk.curve(curva).map(|c| &c.geom) {
        Some(Curve::Line { a, b }) => {
            let (pa, pb) = (sk.pos(*a), sk.pos(*b));
            let fim = dist(cursor, pb) < dist(cursor, pa);
            let (de, para, ponta) = if fim { (pa, pb, *b) } else { (pb, pa, *a) };
            let l = dist(de, para).max(1e-12);
            let k = alcance(sk, para) / l;
            (ponta, Traco::Reta { a: para, b: [para[0] + (para[0] - de[0]) * k, para[1] + (para[1] - de[1]) * k] })
        }
        Some(Curve::Arc { s, e, .. }) => {
            let Some(Traco::Arco { c, r, a0, varre }) = traco(sk, curva) else { return Err("estender funciona em linha e arco") };
            let (ps, pe) = (sk.pos(*s), sk.pos(*e));
            // o que falta para fechar a volta, no sentido da ponta
            let falta = TAU - varre;
            if dist(cursor, pe) < dist(cursor, ps) {
                (*e, Traco::Arco { c, r, a0: a0 + varre, varre: falta })
            } else {
                (*s, Traco::Arco { c, r, a0, varre: -falta })
            }
        }
        _ => return Err("estender funciona em linha e arco"),
    };
    let f = caminho.folga_u();
    let Some(alvo) = cortes_em(sk, &caminho, curva).into_iter().find(|c| c.u > f) else {
        return Err("não há curva no caminho da ponta para estender até ela");
    };
    Ok(PlanoEstender { curva, ponta, destino: alvo.p, alvo: alvo.por, novo: caminho.trecho(0., alvo.u) })
}

/// Estende a ponta perto do cursor. É sempre o que o `plano_estender` mostra.
pub fn estender(sk: &mut SketchData, al: &mut impl FnMut() -> Id, curva: Id, cursor: P) -> Result<(), &'static str> {
    let plano = plano_estender(sk, curva, cursor)?;
    let n = ponto_no_corte(sk, al, Corte { u: 0., p: plano.destino, por: plano.alvo });
    match sk.curve(curva).map(|c| c.geom.clone()) {
        Some(Curve::Line { a, b }) => {
            let (na, nb) = if b == plano.ponta { (a, n) } else { (n, b) };
            troca_linha(sk, curva, na, nb);
        }
        Some(Curve::Arc { c, s, e }) => {
            let (ns, ne) = if e == plano.ponta { (s, n) } else { (n, e) };
            troca_geom(sk, curva, Curve::Arc { c, s: ns, e: ne });
        }
        _ => {}
    }
    limpa_soltos(sk);
    Ok(())
}

/// Com Shift segurado, Aparar age como Estender e Estender como Aparar.
pub fn efetiva(t: Tool, shift: bool) -> Tool {
    match (t, shift) {
        (Tool::Trim, true) => Tool::Extend,
        (Tool::Extend, true) => Tool::Trim,
        _ => t,
    }
}

/// O que um clique faz com a ferramenta de relimitar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gesto {
    Nada,
    Agir,
    /// clique no vazio ou botão direito: a ferramenta encerra
    Encerrar,
}

/// Decide o gesto. Fora da seleção prévia o clique é recusado, não encerra: há curva ali.
pub fn gesto(sobre_curva: bool, fora_da_selecao: bool, esquerdo: bool, direito: bool) -> Gesto {
    if direito {
        Gesto::Encerrar
    } else if !esquerdo || fora_da_selecao {
        Gesto::Nada
    } else if sobre_curva {
        Gesto::Agir
    } else {
        Gesto::Encerrar
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashMap;

    fn ids() -> impl FnMut() -> Id {
        let mut n = 1000u64;
        move || {
            n += 1;
            Id(n)
        }
    }

    fn linha(sk: &mut SketchData, al: &mut impl FnMut() -> Id, a: P, b: P) -> Id {
        let (p, q) = (sk.add_point(al, a[0], a[1]), sk.add_point(al, b[0], b[1]));
        sk.add_curve(al, Curve::Line { a: p, b: q }, false)
    }

    fn circulo(sk: &mut SketchData, al: &mut impl FnMut() -> Id, c: P, r: f64) -> Id {
        let p = sk.add_point(al, c[0], c[1]);
        sk.add_curve(al, Curve::Circle { c: p, r }, false)
    }

    fn arco(sk: &mut SketchData, al: &mut impl FnMut() -> Id, c: P, s: P, e: P) -> Id {
        let (pc, ps, pe) = (sk.add_point(al, c[0], c[1]), sk.add_point(al, s[0], s[1]), sk.add_point(al, e[0], e[1]));
        sk.add_curve(al, Curve::Arc { c: pc, s: ps, e: pe }, false)
    }

    fn perto(a: P, b: P) -> bool {
        dist(a, b) < 1e-6
    }

    #[test]
    fn linha_entre_dois_cortes_vira_dois_trechos_presos_aos_limites() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let h = linha(&mut sk, &mut al, [-20., 0.], [20., 0.]);
        sk.add_constraint(&mut al, Kind::Horizontal(h));
        let v1 = linha(&mut sk, &mut al, [-5., -9.], [-5., 9.]);
        let v2 = linha(&mut sk, &mut al, [5., -9.], [5., 9.]);
        let p = plano_aparar(&sk, h, [1., 0.]).unwrap();
        assert_eq!(p.limites.len(), 2);
        assert_eq!(p.cortes.len(), 2);
        assert!(perto(p.sai[0], [-5., 0.]) && perto(*p.sai.last().unwrap(), [5., 0.]));
        aparar(&mut sk, &mut al, h, [1., 0.]).unwrap();
        assert_eq!(sk.curves.len(), 4);
        // as pontas novas seguem as verticais, e os dois trechos continuam horizontais
        assert_eq!(sk.constraints.iter().filter(|k| matches!(k.kind, Kind::PointOnCurve(_, c) if c == v1 || c == v2)).count(), 2);
        assert_eq!(sk.constraints.iter().filter(|k| matches!(k.kind, Kind::Horizontal(_))).count(), 2);
        let rel = sk.solve(&HashMap::new(), &[]);
        assert!(rel.failing.is_empty());
    }

    #[test]
    fn arco_perde_so_o_trecho_clicado() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let a = arco(&mut sk, &mut al, [0., 0.], [10., 0.], [-10., 0.]);
        linha(&mut sk, &mut al, [0., 0.], [0., 20.]);
        let meio = 2f64.sqrt() * 5.;
        // fora do cruzamento em (0, 10): o lado direito sai
        let p = plano_aparar(&sk, a, [meio, meio]).unwrap();
        assert!(perto(p.sai[0], [10., 0.]) && perto(*p.sai.last().unwrap(), [0., 10.]));
        aparar(&mut sk, &mut al, a, [meio, meio]).unwrap();
        let t = traco(&sk, a).unwrap();
        assert!(perto(t.em(0.), [0., 10.]) && perto(t.em(1.), [-10., 0.]));
        assert!((t.comprimento() - 10. * std::f64::consts::FRAC_PI_2).abs() < 1e-6);
    }

    #[test]
    fn arco_cortado_duas_vezes_fica_em_dois_com_o_mesmo_raio() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let a = arco(&mut sk, &mut al, [0., 0.], [10., 0.], [-10., 0.]);
        let meio = 2f64.sqrt() * 5.;
        linha(&mut sk, &mut al, [0., 0.], [20., 20.]);
        linha(&mut sk, &mut al, [0., 0.], [-20., 20.]);
        aparar(&mut sk, &mut al, a, [0., 10.]).unwrap();
        let arcos: Vec<Id> = sk.curves.iter().filter(|c| matches!(c.geom, Curve::Arc { .. })).map(|c| c.id).collect();
        assert_eq!(arcos.len(), 2);
        assert!(sk.constraints.iter().any(|k| matches!(k.kind, Kind::Equal(x, y) if x == arcos[0] && y == arcos[1])));
        assert!(perto(traco(&sk, arcos[0]).unwrap().em(1.), [meio, meio]));
        assert!(perto(traco(&sk, arcos[1]).unwrap().em(0.), [-meio, meio]));
    }

    #[test]
    fn circulo_cortado_vira_o_arco_que_sobra() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let c = circulo(&mut sk, &mut al, [0., 0.], 10.);
        sk.add_constraint(&mut al, Kind::Diameter(c, "20".into()));
        linha(&mut sk, &mut al, [-20., 0.], [20., 0.]);
        // clique em cima: sai a metade de cima, e fica a de baixo
        let p = plano_aparar(&sk, c, [0., 10.]).unwrap();
        assert_eq!(p.limites.len(), 2);
        aparar(&mut sk, &mut al, c, [0., 10.]).unwrap();
        let t = traco(&sk, c).unwrap();
        assert!(matches!(sk.curve(c).unwrap().geom, Curve::Arc { .. }));
        assert!(perto(t.em(0.5), [0., -10.]), "{:?}", t.em(0.5));
        // a cota do círculo continua valendo no arco
        assert!(sk.constraints.iter().any(|k| matches!(k.kind, Kind::Diameter(x, _) if x == c)));
        assert!(sk.solve(&HashMap::new(), &[]).failing.is_empty());
    }

    #[test]
    fn circulo_cortado_passando_pelo_inicio_da_volta() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let c = circulo(&mut sk, &mut al, [0., 0.], 10.);
        linha(&mut sk, &mut al, [0., -20.], [0., 20.]);
        // o ângulo 0 fica dentro do trecho clicado: lo > hi
        let p = plano_aparar(&sk, c, [10., 0.]).unwrap();
        assert!(p.hi < p.lo);
        assert!(p.sai.iter().any(|q| perto(*q, [10., 0.])) || p.sai.iter().any(|q| dist(*q, [10., 0.]) < 0.5));
        aparar(&mut sk, &mut al, c, [10., 0.]).unwrap();
        assert!(perto(traco(&sk, c).unwrap().em(0.5), [-10., 0.]));
    }

    #[test]
    fn circulo_sem_dois_cortes_sai_inteiro() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let c = circulo(&mut sk, &mut al, [0., 0.], 10.);
        // tangente: um corte só
        linha(&mut sk, &mut al, [-20., 10.], [20., 10.]);
        assert_eq!(cortes(&sk, c).len(), 1);
        assert!(plano_aparar(&sk, c, [0., -10.]).unwrap().limites.is_empty());
        aparar(&mut sk, &mut al, c, [0., -10.]).unwrap();
        assert!(sk.curve(c).is_none());
        assert_eq!(sk.points.len(), 2);
    }

    #[test]
    fn linha_cortada_por_spline_e_spline_cortada_por_linha() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let pts: Vec<Id> = [[-20., 0.], [-10., 4.], [0., 0.], [10., -4.], [20., 0.]].map(|q| sk.add_point(&mut al, q[0], q[1])).to_vec();
        let s = sk.add_curve(&mut al, Curve::Spline { pts }, false);
        let v = linha(&mut sk, &mut al, [5., -20.], [5., 20.]);
        let antes = traco(&sk, s).unwrap();
        // o corte da linha é na spline
        let cl = cortes(&sk, v);
        assert_eq!(cl.len(), 1);
        assert!(cl[0].por == s && dist(cl[0].p, antes.em(antes.param(cl[0].p))) < 1e-6);
        // apara a spline à direita: o que fica termina na vertical e segue por onde passava
        aparar(&mut sk, &mut al, s, [15., -3.]).unwrap();
        let t = traco(&sk, s).unwrap();
        assert!((t.em(1.)[0] - 5.).abs() < 1e-6);
        for u in [0., 0.25, 0.5, 0.75] {
            let q = t.em(u);
            let d = dist(q, antes.em(antes.param(q)));
            assert!(d < 0.2, "desviou em {u}: {q:?}");
        }
        assert_eq!(sk.curves.iter().filter(|c| matches!(c.geom, Curve::Spline { .. })).count(), 1);
    }

    #[test]
    fn spline_cortada_em_dois_pontos_vira_duas() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let pts: Vec<Id> = [[-20., 0.], [0., 4.], [20., 0.]].map(|q| sk.add_point(&mut al, q[0], q[1])).to_vec();
        let s = sk.add_curve(&mut al, Curve::Spline { pts }, false);
        linha(&mut sk, &mut al, [-5., -20.], [-5., 20.]);
        linha(&mut sk, &mut al, [5., -20.], [5., 20.]);
        aparar(&mut sk, &mut al, s, [0., 4.]).unwrap();
        let sp: Vec<Id> = sk.curves.iter().filter(|c| matches!(c.geom, Curve::Spline { .. })).map(|c| c.id).collect();
        assert_eq!(sp.len(), 2);
        assert!((traco(&sk, sp[0]).unwrap().em(1.)[0] + 5.).abs() < 1e-6);
        assert!((traco(&sk, sp[1]).unwrap().em(0.)[0] - 5.).abs() < 1e-6);
    }

    #[test]
    fn estender_linha_mostra_o_alvo_e_prende_a_ponta() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let h = linha(&mut sk, &mut al, [0., 0.], [5., 0.]);
        let c = circulo(&mut sk, &mut al, [20., 0.], 5.);
        let p = plano_estender(&sk, h, [4., 0.]).unwrap();
        assert_eq!(p.alvo, c);
        assert!(perto(p.destino, [15., 0.]));
        assert!(perto(p.novo[0], [5., 0.]) && perto(*p.novo.last().unwrap(), [15., 0.]));
        estender(&mut sk, &mut al, h, [4., 0.]).unwrap();
        assert!((traco(&sk, h).unwrap().comprimento() - 15.).abs() < 1e-6);
        assert!(sk.constraints.iter().any(|k| matches!(k.kind, Kind::PointOnCurve(_, x) if x == c)));
    }

    #[test]
    fn estender_arco_segue_o_circulo_nos_dois_sentidos() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let a = arco(&mut sk, &mut al, [0., 0.], [10., 0.], [0., 10.]);
        linha(&mut sk, &mut al, [-20., 0.], [-5., 0.]);
        linha(&mut sk, &mut al, [0., -20.], [0., -5.]);
        // ponta final: anti-horário até a linha em (-10, 0)
        let p = plano_estender(&sk, a, [1., 10.]).unwrap();
        assert!(perto(p.destino, [-10., 0.]), "{:?}", p.destino);
        // ponta inicial: horário até (0, -10)
        let q = plano_estender(&sk, a, [10., 1.]).unwrap();
        assert!(perto(q.destino, [0., -10.]), "{:?}", q.destino);
        estender(&mut sk, &mut al, a, [1., 10.]).unwrap();
        let t = traco(&sk, a).unwrap();
        assert!(perto(t.em(1.), [-10., 0.]) && (t.comprimento() - 10. * std::f64::consts::PI).abs() < 1e-6);
    }

    #[test]
    fn estender_recusa_antes_do_clique() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let c = circulo(&mut sk, &mut al, [0., 0.], 5.);
        let h = linha(&mut sk, &mut al, [10., 0.], [20., 0.]);
        assert!(plano_estender(&sk, c, [5., 0.]).is_err());
        // a ponta perto de x = 20 olha para fora: nada no caminho
        assert!(plano_estender(&sk, h, [19., 0.]).is_err());
        assert!(plano_estender(&sk, h, [11., 0.]).is_ok());
    }

    #[test]
    fn encontro_em_t_resolvido_ainda_corta() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let h = linha(&mut sk, &mut al, [-10., 0.], [10., 0.]);
        // a vertical para a um fio da horizontal, como depois do solver
        linha(&mut sk, &mut al, [0., 1e-8], [0., 10.]);
        assert_eq!(cortes(&sk, h).len(), 1);
    }

    #[test]
    fn shift_alterna_e_gestos_encerram() {
        assert_eq!(efetiva(Tool::Trim, true), Tool::Extend);
        assert_eq!(efetiva(Tool::Extend, true), Tool::Trim);
        assert_eq!(efetiva(Tool::Trim, false), Tool::Trim);
        assert_eq!(gesto(true, false, true, false), Gesto::Agir);
        assert_eq!(gesto(false, false, true, false), Gesto::Encerrar);
        assert_eq!(gesto(true, false, false, true), Gesto::Encerrar);
        assert_eq!(gesto(false, true, true, false), Gesto::Nada);
        assert_eq!(gesto(true, false, false, false), Gesto::Nada);
    }
}
