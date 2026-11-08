// SPDX-License-Identifier: LGPL-2.1-or-later
//! B352: edição do esboço do desenho, sem interface.
//!
//! A ferramenta da folha (`fenkra-app/src/dwg_ui/esboco_ui.rs`) só traduz cliques em chamadas
//! daqui. Tudo trabalha no plano do esboço (mm do modelo na vista, mm da folha quando solto), então
//! a mesma regra vale nos dois casos e os testes não precisam da tela.
//!
//! Formas: linha, retângulo (por cantos ou pelo centro), círculo (centro e raio ou três pontos),
//! arco (três pontos ou centro) e ponto. A linha quase horizontal ou vertical ganha a restrição,
//! como no sketch de peça. As cotas nascem com a medida atual e mandam na geometria quando o
//! valor muda; um valor que o resolvedor não alcança volta atrás em vez de deixar o esboço torto.

use crate::esboco::{EsbocoDesenho, TipoLinha};
use crate::V2;
use fenkra_core::sketch::{Curve, Kind, SkCurve};
use fenkra_core::{Id, SketchData};
use std::collections::HashMap;

/// O que a ferramenta de desenho cria.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forma {
    Linha,
    Retangulo,
    RetanguloCentro,
    Circulo,
    Circulo3,
    Arco3,
    ArcoCentro,
    Ponto,
}

impl Forma {
    pub const TODAS: [Forma; 8] = [
        Forma::Linha,
        Forma::Retangulo,
        Forma::RetanguloCentro,
        Forma::Circulo,
        Forma::Circulo3,
        Forma::Arco3,
        Forma::ArcoCentro,
        Forma::Ponto,
    ];

    /// Cliques que a forma pede.
    pub fn cliques(self) -> usize {
        match self {
            Forma::Ponto => 1,
            Forma::Linha | Forma::Retangulo | Forma::RetanguloCentro | Forma::Circulo => 2,
            Forma::Circulo3 | Forma::Arco3 | Forma::ArcoCentro => 3,
        }
    }
}

/// Um clique no plano do esboço: a posição e o ponto que o snap pegou. `fixo`: o clique caiu num
/// vértice da vista, e o ponto novo fica preso ali (geometria de apoio que vem do 3D).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clique {
    pub uv: V2,
    pub ponto: Option<Id>,
    pub fixo: bool,
}

impl Clique {
    pub fn solto(uv: V2) -> Self {
        Clique {
            uv,
            ponto: None,
            fixo: false,
        }
    }
}

/// Ids novos do esboço, depois do maior que ele já usa.
pub fn alocador(s: &SketchData) -> impl FnMut() -> Id {
    let mut n = s
        .points
        .iter()
        .map(|p| p.id.0)
        .chain(s.curves.iter().map(|c| c.id.0))
        .chain(s.constraints.iter().map(|c| c.id.0))
        .max()
        .unwrap_or(0);
    move || {
        n += 1;
        Id(n)
    }
}

fn dist(a: V2, b: V2) -> f64 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
}

fn ponto(s: &mut SketchData, al: &mut impl FnMut() -> Id, c: Clique) -> Id {
    if let Some(p) = c.ponto.filter(|p| s.point(*p).is_some()) {
        return p;
    }
    let p = s.add_point(al, c.uv[0], c.uv[1]);
    if c.fixo {
        if let Some(q) = s.point_mut(p) {
            q.fixed = true;
        }
    }
    p
}

/// Linha quase horizontal ou vertical (até 2°) ganha a restrição.
fn alinhar(s: &mut SketchData, al: &mut impl FnMut() -> Id, linha: Id, a: V2, b: V2) {
    let (dx, dy) = ((b[0] - a[0]).abs(), (b[1] - a[1]).abs());
    let t = 2f64.to_radians().tan();
    if dx > 1e-9 && dy <= dx * t {
        s.add_constraint(al, Kind::Horizontal(linha));
    } else if dy > 1e-9 && dx <= dy * t {
        s.add_constraint(al, Kind::Vertical(linha));
    }
}

/// Centro e raio do círculo pelos três pontos (`None` se alinhados).
pub fn circulo3(a: V2, b: V2, c: V2) -> Option<(V2, f64)> {
    let d = 2. * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() < 1e-9 {
        return None;
    }
    let sq = |p: V2| p[0] * p[0] + p[1] * p[1];
    let ux = (sq(a) * (b[1] - c[1]) + sq(b) * (c[1] - a[1]) + sq(c) * (a[1] - b[1])) / d;
    let uy = (sq(a) * (c[0] - b[0]) + sq(b) * (a[0] - c[0]) + sq(c) * (b[0] - a[0])) / d;
    Some(([ux, uy], dist([ux, uy], a)))
}

/// `m` está no arco anti-horário de `s` até `e` em torno de `c`?
fn no_arco(c: V2, s: V2, e: V2, m: V2) -> bool {
    let ang = |p: V2| (p[1] - c[1]).atan2(p[0] - c[0]);
    let tau = std::f64::consts::TAU;
    let (a0, a1, am) = (ang(s), ang(e), ang(m));
    let span = (a1 - a0).rem_euclid(tau);
    (am - a0).rem_euclid(tau) <= span
}

/// Cria a forma com os cliques dados. Devolve as curvas novas, ou o motivo de não criar.
pub fn criar(
    e: &mut EsbocoDesenho,
    forma: Forma,
    c: &[Clique],
    construcao: bool,
    tipo: TipoLinha,
) -> Result<Vec<Id>, &'static str> {
    if c.len() < forma.cliques() {
        return Err("faltam cliques");
    }
    let s = &mut e.dados;
    let mut al = alocador(s);
    let mut novas = vec![];
    match forma {
        Forma::Linha => {
            if dist(c[0].uv, c[1].uv) < 1e-9 || (c[0].ponto.is_some() && c[0].ponto == c[1].ponto) {
                return Err("os dois pontos coincidem");
            }
            let a = ponto(s, &mut al, c[0]);
            let b = ponto(s, &mut al, c[1]);
            let l = s.add_curve(&mut al, Curve::Line { a, b }, construcao);
            let (pa, pb) = (s.pos(a), s.pos(b));
            alinhar(s, &mut al, l, pa, pb);
            novas.push(l);
        }
        Forma::Retangulo | Forma::RetanguloCentro => {
            let (a, b) = if forma == Forma::Retangulo {
                (c[0].uv, c[1].uv)
            } else {
                (
                    [2. * c[0].uv[0] - c[1].uv[0], 2. * c[0].uv[1] - c[1].uv[1]],
                    c[1].uv,
                )
            };
            if (a[0] - b[0]).abs() < 1e-9 || (a[1] - b[1]).abs() < 1e-9 {
                return Err("o retângulo não tem área");
            }
            let (p, l) = s.add_rect(&mut al, a, b, construcao);
            novas.extend(l);
            if forma == Forma::Retangulo {
                if let Some(q) = c[0].ponto.filter(|q| s.point(*q).is_some()) {
                    s.merge_points(p[0], q);
                }
            } else {
                // centro preso ao meio da diagonal de construção
                let centro = ponto(s, &mut al, c[0]);
                let diag = s.add_curve(&mut al, Curve::Line { a: p[0], b: p[2] }, true);
                s.add_constraint(&mut al, Kind::Midpoint(centro, diag));
                novas.push(diag);
            }
            if let Some(q) = c[1].ponto.filter(|q| s.point(*q).is_some()) {
                s.merge_points(p[2], q);
            }
        }
        Forma::Circulo => {
            let r = dist(c[0].uv, c[1].uv);
            if r < 1e-9 {
                return Err("o raio é zero");
            }
            let o = ponto(s, &mut al, c[0]);
            novas.push(s.add_curve(&mut al, Curve::Circle { c: o, r }, construcao));
        }
        Forma::Circulo3 => {
            let (o, r) =
                circulo3(c[0].uv, c[1].uv, c[2].uv).ok_or("os três pontos estão alinhados")?;
            let o = s.add_point(&mut al, o[0], o[1]);
            novas.push(s.add_curve(&mut al, Curve::Circle { c: o, r }, construcao));
        }
        Forma::Arco3 => {
            // início, fim e um ponto do meio: o arco passa pelos três
            let (o, _) =
                circulo3(c[0].uv, c[1].uv, c[2].uv).ok_or("os três pontos estão alinhados")?;
            let (i, f) = if no_arco(o, c[0].uv, c[1].uv, c[2].uv) {
                (c[0], c[1])
            } else {
                (c[1], c[0])
            };
            let po = s.add_point(&mut al, o[0], o[1]);
            let ps = ponto(s, &mut al, i);
            let pe = ponto(s, &mut al, f);
            novas.push(s.add_curve(
                &mut al,
                Curve::Arc {
                    c: po,
                    s: ps,
                    e: pe,
                },
                construcao,
            ));
        }
        Forma::ArcoCentro => {
            let (o, i) = (c[0].uv, c[1].uv);
            let r = dist(o, i);
            let d = dist(o, c[2].uv);
            if r < 1e-9 || d < 1e-9 {
                return Err("o raio é zero");
            }
            // o fim cai no raio do início, na direção do terceiro clique
            let f = [
                o[0] + (c[2].uv[0] - o[0]) * r / d,
                o[1] + (c[2].uv[1] - o[1]) * r / d,
            ];
            let po = ponto(s, &mut al, c[0]);
            let ps = ponto(s, &mut al, c[1]);
            let pe = s.add_point(&mut al, f[0], f[1]);
            novas.push(s.add_curve(
                &mut al,
                Curve::Arc {
                    c: po,
                    s: ps,
                    e: pe,
                },
                construcao,
            ));
        }
        Forma::Ponto => {
            let p = ponto(s, &mut al, c[0]);
            novas.push(s.add_curve(&mut al, Curve::Point { p }, construcao));
        }
    }
    for id in &novas {
        let construida = e.dados.curve(*id).is_some_and(|c| c.construction);
        if !construida {
            e.definir_tipo(*id, tipo);
        }
    }
    Ok(novas)
}

/// Ponto do fim da última linha criada: a polilinha continua dele.
pub fn fim_da_linha(e: &EsbocoDesenho, linha: Id) -> Option<Id> {
    match e.dados.curve(linha)?.geom {
        Curve::Line { b, .. } => Some(b),
        _ => None,
    }
}

/// Tipo de cota que a ferramenta cria.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cota {
    /// distância entre dois pontos
    Distancia(Id, Id),
    Horizontal(Id, Id),
    Vertical(Id, Id),
    /// comprimento da linha
    Comprimento(Id),
    Raio(Id),
    Diametro(Id),
}

/// Número curto para a expressão da cota (ponto decimal, até 3 casas).
pub fn numero(v: f64) -> String {
    let s = format!("{:.3}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

/// Cota nova com a medida de agora: não mexe na geometria. Devolve o id da restrição.
pub fn cotar(e: &mut EsbocoDesenho, c: Cota) -> Option<Id> {
    let s = &mut e.dados;
    let k = match c {
        Cota::Distancia(a, b) => Kind::Distance(a, b, String::new()),
        Cota::Horizontal(a, b) => Kind::DistanceX(a, b, String::new()),
        Cota::Vertical(a, b) => Kind::DistanceY(a, b, String::new()),
        Cota::Comprimento(l) => Kind::Length(l, String::new()),
        Cota::Raio(r) => Kind::Radius(r, String::new()),
        Cota::Diametro(r) => Kind::Diameter(r, String::new()),
    };
    let v = medida_de(s, &k)?;
    let mut k = k;
    *k.value_mut()? = numero(v);
    let mut al = alocador(s);
    Some(s.add_constraint(&mut al, k))
}

fn medida_de(s: &SketchData, k: &Kind) -> Option<f64> {
    let par = |a: Id, b: Id| {
        Some((
            s.point(a).map(|p| [p.x, p.y])?,
            s.point(b).map(|p| [p.x, p.y])?,
        ))
    };
    let v = match k {
        Kind::Distance(a, b, _) => par(*a, *b).map(|(p, q)| dist(p, q)),
        Kind::DistanceX(a, b, _) => par(*a, *b).map(|(p, q)| (q[0] - p[0]).abs()),
        Kind::DistanceY(a, b, _) => par(*a, *b).map(|(p, q)| (q[1] - p[1]).abs()),
        Kind::Length(l, _) => match s.curve(*l)?.geom {
            Curve::Line { a, b } => Some(dist(s.pos(a), s.pos(b))),
            _ => None,
        },
        Kind::Radius(c, _) => s.radius_of(*c),
        Kind::Diameter(c, _) => s.radius_of(*c).map(|r| 2. * r),
        _ => None,
    }?;
    (v > 1e-9).then_some(v)
}

/// Medida atual da cota `id` (o que a geometria mede hoje).
pub fn medida(e: &EsbocoDesenho, id: Id) -> Option<f64> {
    let k = &e.dados.constraints.iter().find(|c| c.id == id)?.kind;
    medida_de(&e.dados, k)
}

/// Valor (expressão) da cota `id`.
pub fn valor(e: &EsbocoDesenho, id: Id) -> Option<String> {
    let mut k = e
        .dados
        .constraints
        .iter()
        .find(|c| c.id == id)?
        .kind
        .clone();
    k.value_mut().map(|v| v.clone())
}

/// Troca o valor da cota e resolve. Se não convergir, tudo volta como estava e devolve `false`.
pub fn definir_valor(e: &mut EsbocoDesenho, id: Id, texto: &str) -> bool {
    let antes = e.dados.clone();
    let t = texto.trim().replace(',', ".");
    // expressão que não vale um número positivo não entra: o resolvedor a ignoraria calado
    if !fenkra_core::expr::eval_str(&t, &HashMap::new()).is_ok_and(|v| v.is_finite() && v > 0.) {
        return false;
    }
    let Some(k) = e.dados.constraints.iter_mut().find(|c| c.id == id) else {
        return false;
    };
    let Some(v) = k.kind.value_mut() else {
        return false;
    };
    *v = t;
    if !e.resolver() {
        e.dados = antes;
        return false;
    }
    true
}

/// Aplica uma mudança no esboço e resolve; se o esboço resolvia e deixa de resolver, desfaz.
pub fn com_resolucao(e: &mut EsbocoDesenho, f: impl FnOnce(&mut SketchData) -> bool) -> bool {
    let antes = e.dados.clone();
    let resolvia = antes.clone().solve(&HashMap::new(), &[]).converged;
    if !f(&mut e.dados) {
        e.dados = antes;
        return false;
    }
    if !e.resolver() && resolvia {
        e.dados = antes;
        return false;
    }
    true
}

/// Arrasta o ponto até `alvo` mantendo as restrições. Devolve se o resolvedor alcançou.
pub fn arrastar(e: &mut EsbocoDesenho, p: Id, alvo: V2) -> bool {
    let antes = e.dados.clone();
    let Some(q) = e.dados.point_mut(p) else {
        return false;
    };
    if q.fixed {
        return false;
    }
    q.x = alvo[0];
    q.y = alvo[1];
    if !e.dados.solve(&HashMap::new(), &[p]).converged {
        e.dados = antes;
        return false;
    }
    true
}

/// Desloca o esboço todo (arrastar na folha).
pub fn mover(e: &mut EsbocoDesenho, d: V2) {
    for p in &mut e.dados.points {
        p.x += d[0];
        p.y += d[1];
    }
}

/// Polilinha de uma curva no plano do esboço, a de construção também.
pub fn curva_pts(s: &SketchData, c: &SkCurve) -> Vec<V2> {
    crate::esboco::polilinha(s, c)
}

/// O que está sob o cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alvo {
    Ponto(Id),
    Curva(Id),
}

fn seg(p: V2, a: V2, b: V2) -> f64 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let l2 = (ab[0] * ab[0] + ab[1] * ab[1]).max(1e-18);
    let t = (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / l2).clamp(0., 1.);
    dist(p, [a[0] + ab[0] * t, a[1] + ab[1] * t])
}

/// Ponto ou curva mais perto de `q` dentro de `tol` (unidades do esboço). O ponto ganha da curva.
pub fn mais_perto(e: &EsbocoDesenho, q: V2, tol: f64) -> Option<Alvo> {
    let s = &e.dados;
    let usados: Vec<Id> = s
        .curves
        .iter()
        .flat_map(|c| SketchData::curve_points(&c.geom))
        .collect();
    let p = s
        .points
        .iter()
        .filter(|p| usados.contains(&p.id))
        .map(|p| (dist([p.x, p.y], q), p.id))
        .filter(|(d, _)| *d <= tol)
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, id)) = p {
        return Some(Alvo::Ponto(id));
    }
    s.curves
        .iter()
        .filter_map(|c| {
            let pts = curva_pts(s, c);
            let d = if pts.len() == 1 {
                dist(pts[0], q)
            } else {
                pts.windows(2)
                    .map(|w| seg(q, w[0], w[1]))
                    .fold(f64::MAX, f64::min)
            };
            (d <= tol).then_some((d, c.id))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| Alvo::Curva(id))
}

/// Cota pedida pela ferramenta inteligente para o que foi clicado: linha dá o comprimento, círculo
/// o diâmetro, arco o raio e dois pontos a distância.
pub fn cota_inteligente(e: &EsbocoDesenho, alvos: &[Alvo]) -> Option<Cota> {
    let s = &e.dados;
    match alvos {
        [Alvo::Curva(c)] => match s.curve(*c)?.geom {
            Curve::Line { .. } => Some(Cota::Comprimento(*c)),
            Curve::Circle { .. } => Some(Cota::Diametro(*c)),
            Curve::Arc { .. } => Some(Cota::Raio(*c)),
            _ => None,
        },
        [Alvo::Ponto(a), Alvo::Ponto(b)] if a != b => Some(Cota::Distancia(*a, *b)),
        _ => None,
    }
}

/// Os dois pontos que a cota horizontal ou vertical usa: os da linha, ou os dois clicados.
pub fn pontos_da_cota(e: &EsbocoDesenho, alvos: &[Alvo]) -> Option<(Id, Id)> {
    match alvos {
        [Alvo::Curva(c)] => match e.dados.curve(*c)?.geom {
            Curve::Line { a, b } => Some((a, b)),
            _ => None,
        },
        [Alvo::Ponto(a), Alvo::Ponto(b)] if a != b => Some((*a, *b)),
        _ => None,
    }
}
