// SPDX-License-Identifier: LGPL-2.1-or-later
//! Solucionador de posicionamento: cada corpo livre tem 6 graus de liberdade (translação e
//! rotação em torno do próprio centro); as restrições viram resíduos e o Levenberg–Marquardt
//! leva todos a zero. Componentes fixos não entram como incógnita; uma fixação conjunta põe
//! várias peças no mesmo corpo.
//!
//! A montagem não vai inteira para o sistema: ela é dividida em aglomerados, os grupos de corpos que
//! se influenciam, e cada um resolve sozinho. Dentro do aglomerado, o que já está travado entre si
//! vira um corpo só antes de começar (B103, ADR 0046).

use crate::*;
use std::collections::HashMap;

/// Resultado do solucionador.
#[derive(Clone, Debug, Default)]
pub struct SolveReport {
    /// resíduo final (0 = todas as restrições satisfeitas)
    pub residual: f64,
    pub iterations: usize,
    /// graus de liberdade que sobram (6 × corpos livres − posto do jacobiano)
    pub dof: usize,
    /// restrições não satisfeitas (impossíveis ou em conflito)
    pub failed: Vec<Id>,
    pub converged: bool,
    /// juntas paradas num limite: não é falha, e sim o batente segurando a peça
    pub no_limite: Vec<Id>,
    /// partes independentes da montagem que tiveram conta para fazer
    pub aglomerados: usize,
    /// incógnitas somadas de todos os sistemas, depois de reunir o que já está travado
    pub incognitas: usize,
}

/// Geometria no mundo.
#[derive(Clone, Copy, Debug)]
enum World {
    Plane(V3, V3),
    Axis(V3, V3),
    Point(V3),
}

fn world(g: &Geom, p: &Placement) -> World {
    match *g {
        Geom::Plane { p: o, n } => World::Plane(p.point(o), unit(p.dir(n))),
        Geom::Axis { p: o, d } => World::Axis(p.point(o), unit(p.dir(d))),
        Geom::Point(o) => World::Point(p.point(o)),
    }
}

/// Parte de `v` perpendicular à direção unitária `d`.
fn perp(v: V3, d: V3) -> V3 {
    let k = dot(v, d);
    [v[0] - d[0] * k, v[1] - d[1] * k, v[2] - d[2] * k]
}

/// Seno abaixo disto: o ângulo pedido é 0° ou 180° (ou 90° no par plano × eixo).
const SENO_ZERO: f64 = 1e-9;

/// Resíduos de uma restrição (vetor que deve ir a zero).
fn residuals(m: &Mate, a: World, b: World, vars: &HashMap<String, f64>, out: &mut Vec<f64>) {
    use World::*;
    let kind = &m.kind;
    let num = |e: &String| expr::eval_str(e, vars).unwrap_or(0.);
    // cota com sinal: "Inverter lado" é o mesmo que digitar o valor negativo
    let cota = |e: &String| if m.lado_invertido { -num(e) } else { num(e) };
    let par = |u: V3, v: V3, out: &mut Vec<f64>| match m.sentido {
        Sentido::Livre => out.extend(cross(u, v)),
        Sentido::Mesmo => out.extend([0, 1, 2].map(|i| u[i] - v[i])),
        Sentido::Oposto => out.extend([0, 1, 2].map(|i| u[i] + v[i])),
    };
    match (kind, a, b) {
        // ---------- coincidência
        (MateKind::Coincident { flip }, Plane(pa, na), Plane(pb, nb)) => {
            let s = if *flip { -1. } else { 1. };
            out.extend([0, 1, 2].map(|i| na[i] - s * nb[i]));
            out.push(dot(sub(pb, pa), na));
        }
        (MateKind::Coincident { flip }, Axis(pa, da), Axis(pb, db)) => {
            let s = if *flip { -1. } else { 1. };
            // eixos colineares; sentido só é imposto quando invertido/direto faz sentido (mesma linha)
            out.extend([0, 1, 2].map(|i| da[i] - s * db[i]));
            out.extend(perp(sub(pb, pa), da));
        }
        (MateKind::Coincident { .. } | MateKind::Contact, Point(pa), Point(pb)) => out.extend(sub(pb, pa)),
        (MateKind::Coincident { .. } | MateKind::Contact, Point(p), Plane(o, n)) | (MateKind::Coincident { .. } | MateKind::Contact, Plane(o, n), Point(p)) => {
            out.push(dot(sub(p, o), n))
        }
        (MateKind::Coincident { .. }, Point(p), Axis(o, d)) | (MateKind::Coincident { .. }, Axis(o, d), Point(p)) => out.extend(perp(sub(p, o), d)),
        (MateKind::Coincident { .. }, Axis(o, d), Plane(q, n)) | (MateKind::Coincident { .. }, Plane(q, n), Axis(o, d)) => {
            // o eixo fica deitado no plano
            out.push(dot(d, n));
            out.push(dot(sub(o, q), n));
        }
        // ---------- contato: faces de frente uma para a outra, sem folga
        (MateKind::Contact, Plane(pa, na), Plane(pb, nb)) => {
            out.extend([0, 1, 2].map(|i| na[i] + nb[i]));
            out.push(dot(sub(pb, pa), na));
        }
        (MateKind::Contact, Axis(pa, da), Axis(pb, db)) => {
            par(da, db, out);
            out.extend(perp(sub(pb, pa), da));
        }
        (MateKind::Contact, Axis(o, d), Plane(q, n)) | (MateKind::Contact, Plane(q, n), Axis(o, d)) => {
            out.push(dot(d, n));
            out.push(dot(sub(o, q), n));
        }
        // ---------- distância
        (MateKind::Distance(e), Plane(pa, na), Plane(pb, nb)) => {
            par(na, nb, out);
            out.push(dot(sub(pb, pa), na) - cota(e));
        }
        (MateKind::Distance(e), Point(p), Plane(o, n)) | (MateKind::Distance(e), Plane(o, n), Point(p)) => out.push(dot(sub(p, o), n) - cota(e)),
        (MateKind::Distance(e), Point(pa), Point(pb)) => out.push(norm(sub(pb, pa)) - num(e)),
        (MateKind::Distance(e), Axis(pa, da), Axis(pb, db)) => {
            par(da, db, out);
            out.push(norm(perp(sub(pb, pa), da)) - num(e));
        }
        (MateKind::Distance(e), Axis(o, d), Plane(q, n)) | (MateKind::Distance(e), Plane(q, n), Axis(o, d)) => {
            out.push(dot(d, n));
            out.push(dot(sub(o, q), n) - cota(e));
        }
        (MateKind::Distance(e), Point(p), Axis(o, d)) | (MateKind::Distance(e), Axis(o, d), Point(p)) => out.push(norm(perp(sub(p, o), d)) - num(e)),
        // ---------- orientação
        (k, a, b) => {
            let dir = |w: World| match w {
                Plane(_, n) => Some(n),
                Axis(_, d) => Some(d),
                Point(_) => None,
            };
            let (Some(u), Some(v)) = (dir(a), dir(b)) else { return };
            // plano × eixo: paralelo = eixo deitado no plano (normal ⟂ eixo)
            let mixed = matches!((a, b), (Plane(..), Axis(..)) | (Axis(..), Plane(..)));
            match k {
                MateKind::Parallel if mixed => out.push(dot(u, v)),
                MateKind::Parallel => par(u, v, out),
                MateKind::Perpendicular if mixed => par(u, v, out),
                MateKind::Perpendicular => out.push(dot(u, v)),
                // cos(θ) é plano em 0° e 180°, e |sen| em plano × eixo também: nesses valores a
                // derivada zera, o solver não sente o giro e o conta como livre. Ali vale a forma
                // vetorial, com derivada cheia.
                MateKind::Angle(e) if mixed => {
                    let t = num(e).to_radians();
                    if t.sin().abs() < SENO_ZERO {
                        out.push(dot(u, v));
                    } else if t.cos().abs() < SENO_ZERO {
                        out.extend(cross(u, v));
                    } else {
                        out.push(dot(u, v).abs() - t.sin().abs());
                    }
                }
                MateKind::Angle(e) => {
                    let t = num(e).to_radians();
                    let v = if m.sentido == Sentido::Oposto { v.map(|x| -x) } else { v };
                    if t.sin().abs() < SENO_ZERO {
                        let c = t.cos();
                        out.extend([0, 1, 2].map(|i| u[i] - c * v[i]));
                    } else {
                        out.push(dot(u, v) - t.cos());
                    }
                }
                _ => {}
            }
        }
    }
}

/// Medida atual entre dois elementos, na convenção dos resíduos: é o valor que a restrição teria
/// que ter para não mover nada. Distância com sinal quando medida ao longo de uma normal; ângulo em
/// graus. `None` se o par não tem essa medida.
pub fn medida(kind: &MateKind, ga: &Geom, pa: &Placement, gb: &Geom, pb: &Placement) -> Option<f64> {
    use World::*;
    if !par_valido(kind, ga, gb) {
        return None;
    }
    let (a, b) = (world(ga, pa), world(gb, pb));
    match kind {
        MateKind::Distance(_) => Some(match (a, b) {
            (Plane(pa, na), Plane(pb, _)) => dot(sub(pb, pa), na),
            (Point(p), Plane(o, n)) | (Plane(o, n), Point(p)) => dot(sub(p, o), n),
            (Point(pa), Point(pb)) => norm(sub(pb, pa)),
            (Axis(pa, da), Axis(pb, _)) => norm(perp(sub(pb, pa), da)),
            (Axis(o, _), Plane(q, n)) | (Plane(q, n), Axis(o, _)) => dot(sub(o, q), n),
            (Point(p), Axis(o, d)) | (Axis(o, d), Point(p)) => norm(perp(sub(p, o), d)),
        }),
        MateKind::Angle(_) => {
            let (u, v) = (axis_dir(a), axis_dir(b));
            // atan2 e não acos: o acos perde metade dos dígitos perto de 0° e 180°
            let t = norm(cross(u, v)).atan2(dot(u, v)).to_degrees();
            let mixed = matches!((a, b), (Plane(..), Axis(..)) | (Axis(..), Plane(..)));
            // plano × eixo: 0° = eixo deitado no plano, 90° = eixo na normal
            Some(if mixed { (90. - t).abs() } else { t })
        }
        _ => None,
    }
}

/// Como as normais (ou eixos) dos dois elementos estão agora.
pub fn sentido_atual(ga: &Geom, pa: &Placement, gb: &Geom, pb: &Placement) -> Sentido {
    if dot(axis_dir(world(ga, pa)), axis_dir(world(gb, pb))) < 0. {
        Sentido::Oposto
    } else {
        Sentido::Mesmo
    }
}

/// Fórmula do valor da restrição, quando ela tem uma.
fn formulas(k: &MateKind) -> Option<&String> {
    match k {
        MateKind::Distance(e) | MateKind::Angle(e) => Some(e),
        MateKind::Gear { ratio: e, .. } | MateKind::RackPinion { mm_per_rev: e, .. } => Some(e),
        _ => None,
    }
}

/// A fórmula da restrição não dá número? Avaliar como 0 faria a peça saltar para uma medida que
/// ninguém pediu: ela fica fora do sistema e aparece como não satisfeita.
fn formula_invalida(m: &Mate, vars: &HashMap<String, f64>) -> bool {
    formulas(&m.kind).is_some_and(|e| expr::eval_str(e, vars).is_err())
}

/// Sistema montado: quais componentes se movem juntos e as incógnitas de cada corpo.
struct Bodies {
    /// componente → corpo (None = fixo)
    of: HashMap<Id, Option<usize>>,
    /// corpo → (componente de referência, posição relativa de cada componente do corpo)
    rel: Vec<Vec<(Id, Placement)>>,
    base: Vec<Placement>,
}

fn bodies(a: &Assembly) -> Bodies {
    let mut of: HashMap<Id, Option<usize>> = HashMap::new();
    let mut rel: Vec<Vec<(Id, Placement)>> = vec![];
    let mut base = vec![];
    let fixed = |id: Id| a.node(id).is_some_and(|n| n.fixed);
    // grupos rígidos: se algum membro é fixo, todos ficam fixos
    for g in &a.rigid {
        let comps: Vec<Id> = g.comps.iter().copied().filter(|c| a.node(*c).is_some() && !of.contains_key(c)).collect();
        if comps.is_empty() {
            continue;
        }
        if comps.iter().any(|c| fixed(*c)) {
            for c in comps {
                of.insert(c, None);
            }
            continue;
        }
        let p0 = a.node(comps[0]).unwrap().placement;
        let inv = p0.inverse();
        let k = rel.len();
        rel.push(comps.iter().map(|c| (*c, inv.then(&a.node(*c).unwrap().placement))).collect());
        base.push(p0);
        for c in comps {
            of.insert(c, Some(k));
        }
    }
    for n in a.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)) {
        if of.contains_key(&n.id) {
            continue;
        }
        if n.fixed {
            of.insert(n.id, None);
        } else {
            let k = rel.len();
            rel.push(vec![(n.id, Placement::default())]);
            base.push(n.placement);
            of.insert(n.id, Some(k));
        }
    }
    Bodies { of, rel, base }
}

fn place(b: &Bodies, a: &Assembly, x: &[f64]) -> HashMap<Id, Placement> {
    let mut out: HashMap<Id, Placement> = HashMap::new();
    for n in a.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)) {
        let p = match b.of.get(&n.id).copied().flatten() {
            None => n.placement,
            Some(k) => {
                let q = &x[6 * k..6 * k + 6];
                let bp = b.base[k].moved([q[0], q[1], q[2]], [q[3], q[4], q[5]], b.base[k].origin);
                let r = b.rel[k].iter().find(|(c, _)| *c == n.id).map(|(_, r)| *r).unwrap_or_default();
                bp.then(&r)
            }
        };
        out.insert(n.id, p);
    }
    out.insert(ORIGEM, Placement::default());
    out
}

fn wrap180(a: f64) -> f64 {
    let mut a = a % 360.;
    if a > 180. {
        a -= 360.;
    }
    if a <= -180. {
        a += 360.;
    }
    a
}

/// Giro (graus) de um componente em torno de `axis` (mundo), medido pela direção de referência
/// local `l` em relação à mesma direção no mundo na criação (`r0`).
fn turn(l: V3, r0: V3, p: &Placement, axis: V3) -> f64 {
    let c = p.dir(l);
    dot(cross(r0, c), axis).atan2(dot(r0, c)).to_degrees()
}

/// Direção do eixo/normal de uma geometria no mundo.
fn axis_dir(w: World) -> V3 {
    match w {
        World::Axis(_, d) | World::Plane(_, d) => d,
        World::Point(_) => [0., 0., 1.],
    }
}

fn all_residuals(a: &Assembly, pl: &HashMap<Id, Placement>, vars: &HashMap<String, f64>, drives: bool, per: &mut Vec<(Id, usize, usize)>, e: &Escopo) -> Vec<f64> {
    let mut r = vec![];
    per.clear();
    let num = |e: &String| expr::eval_str(e, vars).unwrap_or(0.);
    for m in e.mates.iter().map(|i| &a.mates[*i]).filter(|m| !m.suppressed && !formula_invalida(m, vars)) {
        let (Some(pa), Some(pb)) = (pl.get(&m.a.comp), pl.get(&m.b.comp)) else { continue };
        let s = r.len();
        let (wa, wb) = (world(&m.a.geom, pa), world(&m.b.geom, pb));
        match &m.kind {
            // engrenagem/polia: giro de B = razão × giro de A
            MateKind::Gear { ratio, refs, .. } => {
                if refs[0] != [0.; 3] {
                    let ga = turn(refs[0], refs[1], pa, axis_dir(wa));
                    let gb = turn(refs[2], refs[3], pb, axis_dir(wb));
                    r.push(wrap180(gb - num(ratio) * ga) / 10.);
                }
            }
            // cremalheira: avanço de B ao longo da direção dela = mm por volta × voltas de A
            MateKind::RackPinion { mm_per_rev, refs } => {
                if refs[0] != [0.; 3] {
                    let ga = turn(refs[0], refs[1], pa, axis_dir(wa));
                    let o = match m.b.geom {
                        Geom::Axis { p, .. } | Geom::Plane { p, .. } | Geom::Point(p) => p,
                    };
                    let adv = dot(sub(pb.point(o), refs[2]), pb.dir(refs[3]));
                    r.push(adv - num(mm_per_rev) * ga / 360.);
                }
            }
            _ => residuals(m, wa, wb, vars, &mut r),
        }
        per.push((m.id, s, r.len()));
    }
    crate::kin::residuals(a, pl, vars, drives, &mut r, per, e);
    r
}

/// Resolve as restrições e grava as novas posições nos componentes livres.
pub fn solve(a: &mut Assembly) -> SolveReport {
    solve_with(a, &HashMap::new())
}

/// Restrições de orientação viradas ao contrário (ex.: depois de "inverter"): o LM parte de um
/// ponto em que o gradiente da direção é zero e não sai do lugar. Meia volta antes de resolver.
fn seed_flips(a: &mut Assembly) {
    let fixed_of = |a: &Assembly, c: Id| {
        a.node(c).is_none_or(|n| n.fixed) || a.rigid.iter().any(|g| g.comps.contains(&c) && g.comps.iter().any(|x| a.node(*x).is_some_and(|n| n.fixed)))
    };
    let mut vars = a.vars();
    vars.entry("t".into()).or_insert(0.);
    for i in 0..a.mates.len() {
        let m = a.mates[i].clone();
        if m.suppressed {
            continue;
        }
        if let MateKind::Angle(e) = &m.kind {
            let t = expr::eval_str(e, &vars).unwrap_or(90.).to_radians();
            let (Some(pa), Some(pb)) = (a.node(m.a.comp).map(|n| n.placement), a.node(m.b.comp).map(|n| n.placement)) else { continue };
            let (wa, wb) = (world(&m.a.geom, &pa), world(&m.b.geom, &pb));
            let (u, v) = (axis_dir(wa), axis_dir(wb));
            // alinhados e pedindo outro ângulo: o cosseno é plano aqui e o LM não sai do lugar. Um
            // empurrão fora das duas direções dá gradiente; o que outras restrições não deixam
            // girar volta sozinho, e a parte livre segue até o ângulo pedido.
            let mixed = matches!((wa, wb), (World::Plane(..), World::Axis(..)) | (World::Axis(..), World::Plane(..)));
            let (s, c) = (norm(cross(u, v)), dot(u, v).abs());
            let parado = if mixed { (c < 1e-7 && t.sin().abs() >= SENO_ZERO) || (s < 1e-7 && t.cos().abs() >= SENO_ZERO) } else { s < 1e-7 && t.sin().abs() >= SENO_ZERO };
            if parado {
                let comp = if !fixed_of(a, m.b.comp) {
                    m.b.comp
                } else if !fixed_of(a, m.a.comp) {
                    m.a.comp
                } else {
                    continue;
                };
                let (e1, e2) = (any_perp(v), cross(v, any_perp(v)));
                let w = [0, 1, 2].map(|i| 0.05 * (0.8 * e1[i] + 0.6 * e2[i]));
                let o = a.node(comp).map(|n| n.placement.origin).unwrap_or_default();
                let group: Vec<Id> = a.rigid.iter().find(|g| g.comps.contains(&comp)).map(|g| g.comps.clone()).unwrap_or_else(|| vec![comp]);
                for c in group {
                    if let Some(n) = a.node_mut(c) {
                        n.placement = n.placement.moved([0.; 3], w, o);
                    }
                }
                continue;
            }
        }
        let sinal = match m.sentido {
            Sentido::Livre => None,
            Sentido::Mesmo => Some(1.),
            Sentido::Oposto => Some(-1.),
        };
        let s = match (&m.kind, m.a.geom, m.b.geom) {
            (MateKind::Distance(_) | MateKind::Parallel, Geom::Plane { .. }, Geom::Plane { .. }) | (MateKind::Distance(_) | MateKind::Parallel, Geom::Axis { .. }, Geom::Axis { .. }) => {
                let Some(s) = sinal else { continue };
                s
            }
            // só nos extremos o ângulo vira sentido; no meio do caminho o gradiente existe
            (MateKind::Angle(e), Geom::Plane { .. }, Geom::Plane { .. }) | (MateKind::Angle(e), Geom::Axis { .. }, Geom::Axis { .. }) => {
                let t = expr::eval_str(e, &vars).unwrap_or(90.).to_radians();
                if t.sin().abs() >= SENO_ZERO {
                    continue;
                }
                t.cos().signum() * sinal.unwrap_or(1.)
            }
            (MateKind::Coincident { flip }, Geom::Plane { .. }, Geom::Plane { .. }) | (MateKind::Coincident { flip }, Geom::Axis { .. }, Geom::Axis { .. }) => {
                if *flip {
                    -1.
                } else {
                    1.
                }
            }
            (MateKind::Contact, Geom::Plane { .. }, Geom::Plane { .. }) => -1.,
            _ => continue,
        };
        let (Some(pa), Some(pb)) = (a.node(m.a.comp).map(|n| n.placement), a.node(m.b.comp).map(|n| n.placement)) else { continue };
        let (wa, wb) = (world(&m.a.geom, &pa), world(&m.b.geom, &pb));
        let (da, db) = (axis_dir(wa), axis_dir(wb));
        if dot(da, db) * s > -0.2 {
            continue;
        }
        let pt = |w: World| match w {
            World::Axis(p, _) | World::Plane(p, _) | World::Point(p) => p,
        };
        let (comp, d, o) = if !fixed_of(a, m.b.comp) {
            (m.b.comp, db, pt(wb))
        } else if !fixed_of(a, m.a.comp) {
            (m.a.comp, da, pt(wa))
        } else {
            continue;
        };
        let u = any_perp(d);
        let w = [u[0] * std::f64::consts::PI, u[1] * std::f64::consts::PI, u[2] * std::f64::consts::PI];
        let group: Vec<Id> = a.rigid.iter().find(|g| g.comps.contains(&comp)).map(|g| g.comps.clone()).unwrap_or_else(|| vec![comp]);
        for c in group {
            if let Some(n) = a.node_mut(c) {
                n.placement = n.placement.moved([0.; 3], w, o);
            }
        }
    }
}

/// Resolve com variáveis extras (ex.: `t` da simulação).
pub fn solve_with(a: &mut Assembly, extra: &HashMap<String, f64>) -> SolveReport {
    solve_inner(a, extra, None)
}

/// Resolve puxando o componente `id` para `target` (peso pequeno): é o arraste que respeita as
/// restrições — o que elas permitem acompanha o mouse, o resto fica.
///
/// A puxada é um resíduo a mais, então o mínimo dos quadrados divide o erro entre ela e as
/// restrições: na rotação os pesos se igualam e uma peça sem giro livre sairia do lugar. Por isso
/// um segundo passo, sem puxada, projeta o resultado de volta nas restrições.
pub fn solve_toward(a: &mut Assembly, id: Id, target: Placement, weight: f64) -> SolveReport {
    solve_inner(a, &HashMap::new(), Some((id, target, weight)));
    solve_inner(a, &HashMap::new(), None)
}

fn solve_inner(a: &mut Assembly, extra: &HashMap<String, f64>, pull: Option<(Id, Placement, f64)>) -> SolveReport {
    seed_flips(a);
    let mut vars = a.vars();
    vars.extend(extra.iter().map(|(k, v)| (k.clone(), *v)));
    vars.entry("t".into()).or_insert(0.);
    let (mut novos, mut custo, mut iteracoes, mut dof, mut incognitas) = (HashMap::new(), 0., 0, 0, 0);
    let b = bodies(a);
    let grupos = aglomerados(a, &b, pull.and_then(|p| b.of.get(&p.0).copied().flatten()));
    let dentro: usize = grupos.iter().map(|g| g.corpos.len()).sum();
    // corpo solto de tudo: seis graus de liberdade e nenhuma conta a fazer
    dof += 6 * (b.rel.len() - dentro);
    for g in &grupos {
        let s = Sistema::reduzido(a, &b, &vars, g, pull);
        let (x, c, it) = s.minimizar();
        custo += c;
        iteracoes = iteracoes.max(it);
        incognitas += x.len();
        dof += s.gdl(&x);
        novos.extend(s.lugares(&x));
    }
    novos.remove(&ORIGEM);
    for nd in a.nodes.iter_mut() {
        if let Some(p) = novos.get(&nd.id) {
            nd.placement = *p;
        }
    }
    // a conferência vê a montagem inteira: o que o sistema reduzido deixou de fora entra aqui
    let mut pl: HashMap<Id, Placement> = a.nodes.iter().map(|n| (n.id, n.placement)).collect();
    pl.insert(ORIGEM, Placement::default());
    let (tudo, mut per) = (Escopo::tudo(a), vec![]);
    let mut r2 = all_residuals(a, &pl, &vars, true, &mut per, &tudo);
    crate::kin::residuos_limite(a, &pl, &mut r2, &mut per, &tudo.juntas);
    let mut failed = vec![];
    for (id, s, e) in &per {
        if r2[*s..*e].iter().any(|v| v.abs() > 1e-6) && !failed.contains(id) {
            failed.push(*id);
        }
    }
    failed.extend(a.mates.iter().filter(|m| !m.suppressed && formula_invalida(m, &vars)).map(|m| m.id));
    // as repetições acompanham os originais nas posições novas
    a.update_patterns();
    let no_limite = a.joints.iter().filter(|j| crate::kin::no_batente(j, &pl)).map(|j| j.id).collect();
    SolveReport { residual: f64::sqrt(custo), iterations: iteracoes, dof, failed: failed.clone(), converged: failed.is_empty(), no_limite, aglomerados: grupos.len(), incognitas }
}

/// Restrições, juntas e acoplamentos de um pedaço da montagem (posições nas listas dela).
#[derive(Clone, Debug, Default)]
pub(crate) struct Escopo {
    pub mates: Vec<usize>,
    pub juntas: Vec<usize>,
    pub acoplamentos: Vec<usize>,
}

impl Escopo {
    pub fn tudo(a: &Assembly) -> Self {
        Escopo { mates: (0..a.mates.len()).collect(), juntas: (0..a.joints.len()).collect(), acoplamentos: (0..a.couplings.len()).collect() }
    }

    fn vazio(&self) -> bool {
        self.mates.is_empty() && self.juntas.is_empty() && self.acoplamentos.is_empty()
    }

    fn ids(&self, a: &Assembly) -> Vec<Id> {
        let m = self.mates.iter().map(|i| a.mates[*i].id);
        let j = self.juntas.iter().map(|i| a.joints[*i].id);
        m.chain(j).chain(self.acoplamentos.iter().map(|i| a.couplings[*i].id)).collect()
    }
}

/// Um item do sistema e os componentes que ele prende.
enum Item {
    Mate(usize),
    Junta(usize),
    Acoplamento(usize),
}

impl Item {
    fn entrar(&self, e: &mut Escopo) {
        match self {
            Item::Mate(i) => e.mates.push(*i),
            Item::Junta(i) => e.juntas.push(*i),
            Item::Acoplamento(i) => e.acoplamentos.push(*i),
        }
    }
}

/// O que entra no sistema, com os componentes de cada item. Item suprimido, ou com fórmula que não
/// dá número, não gera resíduo: ele não pode ligar um corpo ao outro.
fn itens(a: &Assembly, e: &Escopo, vars: &HashMap<String, f64>) -> Vec<(Item, Vec<Id>)> {
    let mut out = vec![];
    for i in &e.mates {
        let m = &a.mates[*i];
        if !m.suppressed && !formula_invalida(m, vars) {
            out.push((Item::Mate(*i), vec![m.a.comp, m.b.comp]));
        }
    }
    for i in &e.juntas {
        let j = &a.joints[*i];
        if !j.suppressed {
            out.push((Item::Junta(*i), vec![j.a.comp, j.b.comp]));
        }
    }
    for i in &e.acoplamentos {
        let c = &a.couplings[*i];
        let comps: Vec<Id> = a.joints.iter().filter(|j| j.id == c.a || j.id == c.b).flat_map(|j| [j.a.comp, j.b.comp]).collect();
        out.push((Item::Acoplamento(*i), comps));
    }
    out
}

/// Corpos que se influenciam e o que os liga. Um componente fixo não transmite movimento: duas peças
/// presas à mesma base resolvem cada uma por si.
struct Aglomerado {
    corpos: Vec<usize>,
    escopo: Escopo,
}

fn raiz(uf: &mut [usize], mut i: usize) -> usize {
    while uf[i] != i {
        uf[i] = uf[uf[i]];
        i = uf[i];
    }
    i
}

fn aglomerados_de(a: &Assembly, b: &Bodies, vars: &HashMap<String, f64>, puxado: Option<usize>) -> Vec<Aglomerado> {
    let corpo = |c: &Id| b.of.get(c).copied().flatten();
    let lista = itens(a, &Escopo::tudo(a), vars);
    let mut uf: Vec<usize> = (0..b.rel.len()).collect();
    for (_, comps) in &lista {
        let mut corpos = comps.iter().filter_map(corpo);
        if let Some(c0) = corpos.next() {
            for c in corpos {
                let (x, y) = (raiz(&mut uf, c0), raiz(&mut uf, c));
                uf[x] = y;
            }
        }
    }
    let mut mapa: std::collections::BTreeMap<usize, Aglomerado> = Default::default();
    for k in 0..b.rel.len() {
        let r = raiz(&mut uf, k);
        mapa.entry(r).or_insert_with(|| Aglomerado { corpos: vec![], escopo: Escopo::default() }).corpos.push(k);
    }
    for (item, comps) in &lista {
        if let Some(c) = comps.iter().find_map(corpo) {
            let r = raiz(&mut uf, c);
            if let Some(g) = mapa.get_mut(&r) {
                item.entrar(&mut g.escopo);
            }
        }
    }
    let puxado = puxado.map(|p| raiz(&mut uf, p));
    mapa.into_iter().filter(|(r, g)| !g.escopo.vazio() || Some(*r) == puxado).map(|(_, g)| g).collect()
}

fn aglomerados(a: &Assembly, b: &Bodies, puxado: Option<usize>) -> Vec<Aglomerado> {
    let mut vars = a.vars();
    vars.entry("t".into()).or_insert(0.);
    aglomerados_de(a, b, &vars, puxado)
}

/// Itens que dividem o aglomerado com `item`: só eles podem estar em conflito com ele. Se todas as
/// peças do item estão fixas, não há aglomerado e a lista vem vazia.
pub fn vizinhos_de(a: &Assembly, item: Id) -> Vec<Id> {
    let b = bodies(a);
    aglomerados(a, &b, None).iter().map(|g| g.escopo.ids(a)).find(|ids| ids.contains(&item)).map(|ids| ids.into_iter().filter(|i| *i != item).collect()).unwrap_or_default()
}

/// Corpos que andam como um só: seis incógnitas para todos.
struct Bloco {
    corpos: Vec<usize>,
    pivo: V3,
}

/// Dono de um corpo no sistema reduzido: o bloco, ou nenhum se ficou preso ao que está fixo.
const PRESO: usize = usize::MAX;

/// Sistema de um aglomerado.
struct Sistema<'a> {
    a: &'a Assembly,
    b: &'a Bodies,
    vars: &'a HashMap<String, f64>,
    blocos: Vec<Bloco>,
    /// lugar de partida de tudo que o sistema enxerga
    parados: HashMap<Id, Placement>,
    escopo: Escopo,
    puxada: Option<(Id, Placement, f64)>,
}

impl<'a> Sistema<'a> {
    fn novo(a: &'a Assembly, b: &'a Bodies, vars: &'a HashMap<String, f64>, blocos: Vec<Bloco>, escopo: Escopo, puxada: Option<(Id, Placement, f64)>) -> Self {
        let mut parados: HashMap<Id, Placement> = HashMap::new();
        for (_, comps) in itens(a, &escopo, vars) {
            for c in comps {
                if let Some(p) = a.lugar(c) {
                    parados.insert(c, p);
                }
            }
        }
        for bl in &blocos {
            for k in &bl.corpos {
                for (c, _) in &b.rel[*k] {
                    if let Some(p) = a.lugar(*c) {
                        parados.insert(*c, p);
                    }
                }
            }
        }
        Sistema { a, b, vars, blocos, parados, escopo, puxada }
    }

    /// Sistema do aglomerado com os pares já travados entre si reunidos num corpo só.
    ///
    /// Se as restrições diretas entre dois corpos estão satisfeitas e têm posto 6, nenhum movimento
    /// relativo sobra: os dois são um corpo rígido, e a solução deles é conhecida antes de começar.
    /// Reunir tira seis incógnitas e as linhas desse par, e é isso que deixa barata uma montagem cheia
    /// de fixadores. O mesmo vale contra o que está fixo: o corpo sai do sistema.
    fn reduzido(a: &'a Assembly, b: &'a Bodies, vars: &'a HashMap<String, f64>, g: &Aglomerado, puxada: Option<(Id, Placement, f64)>) -> Self {
        let mut dono: HashMap<usize, usize> = g.corpos.iter().map(|k| (*k, *k)).collect();
        let quem = |dono: &HashMap<usize, usize>, c: &Id| -> usize { b.of.get(c).copied().flatten().map_or(PRESO, |k| dono[&k]) };
        loop {
            let mut pares: std::collections::BTreeMap<(usize, usize), Escopo> = Default::default();
            for (item, comps) in itens(a, &g.escopo, vars) {
                let mut donos: Vec<usize> = comps.iter().map(|c| quem(&dono, c)).collect();
                donos.sort_unstable();
                donos.dedup();
                if let [i, j] = donos[..] {
                    item.entrar(pares.entry((i, j)).or_default());
                }
            }
            let mut mudou = false;
            let mut usados = std::collections::HashSet::new();
            for ((i, j), e) in pares {
                if usados.contains(&i) || usados.contains(&j) || !Self::travados(a, b, vars, &dono, i, e) {
                    continue;
                }
                // j é sempre o maior: PRESO absorve, e entre dois blocos tanto faz
                dono.values_mut().filter(|d| **d == i).for_each(|d| *d = j);
                usados.extend([i, j]);
                mudou = true;
            }
            if !mudou {
                break;
            }
        }
        let mut donos: Vec<usize> = dono.values().copied().filter(|d| *d != PRESO).collect();
        donos.sort_unstable();
        donos.dedup();
        let blocos = donos.iter().map(|d| {
            let corpos: Vec<usize> = g.corpos.iter().copied().filter(|k| dono[k] == *d).collect();
            Bloco { pivo: b.base[corpos[0]].origin, corpos }
        });
        let blocos: Vec<Bloco> = blocos.collect();
        // o que ficou dentro de um bloco está satisfeito e assim continua: sai das contas
        let mut escopo = Escopo::default();
        for (item, comps) in itens(a, &g.escopo, vars) {
            let mut d: Vec<usize> = comps.iter().map(|c| quem(&dono, c)).collect();
            d.sort_unstable();
            d.dedup();
            if d.len() > 1 {
                item.entrar(&mut escopo);
            }
        }
        // a peça puxada precisa estar à vista mesmo sem nenhuma linha
        Sistema::novo(a, b, vars, blocos, escopo, puxada)
    }

    /// As restrições `e`, entre o bloco `i` e outro, já estão satisfeitas e não deixam movimento?
    fn travados(a: &'a Assembly, b: &'a Bodies, vars: &'a HashMap<String, f64>, dono: &HashMap<usize, usize>, i: usize, e: Escopo) -> bool {
        let corpos: Vec<usize> = dono.iter().filter(|(_, d)| **d == i).map(|(k, _)| *k).collect();
        let pivo = b.base[corpos[0]].origin;
        let s = Sistema::novo(a, b, vars, vec![Bloco { corpos, pivo }], e, None);
        let (x, mut per) = (vec![0.; 6], vec![]);
        if s.residuos(&x, &mut per).iter().any(|v| v.abs() > 1e-9) {
            return false;
        }
        let r = s.igualdades(&x, &mut per);
        r.len() >= 6 && matrix_rank(&jacobian(&|x, per| s.igualdades(x, per), &x, &r), r.len(), 6) == 6
    }

    fn lugares(&self, x: &[f64]) -> HashMap<Id, Placement> {
        let mut pl = self.parados.clone();
        for (k, bl) in self.blocos.iter().enumerate() {
            let q = &x[6 * k..6 * k + 6];
            for c in &bl.corpos {
                let bp = self.b.base[*c].moved([q[0], q[1], q[2]], [q[3], q[4], q[5]], bl.pivo);
                for (id, rel) in &self.b.rel[*c] {
                    pl.insert(*id, bp.then(rel));
                }
            }
        }
        pl.insert(ORIGEM, Placement::default());
        pl
    }

    /// Só as igualdades: é o que conta para o posto.
    fn igualdades(&self, x: &[f64], per: &mut Vec<(Id, usize, usize)>) -> Vec<f64> {
        all_residuals(self.a, &self.lugares(x), self.vars, true, per, &self.escopo)
    }

    /// Tudo que o mínimos quadrados leva a zero: igualdades, batentes e a puxada do arraste.
    fn residuos(&self, x: &[f64], per: &mut Vec<(Id, usize, usize)>) -> Vec<f64> {
        let pl = self.lugares(x);
        let mut r = all_residuals(self.a, &pl, self.vars, true, per, &self.escopo);
        crate::kin::residuos_limite(self.a, &pl, &mut r, per, &self.escopo.juntas);
        if let Some((id, t, w)) = self.puxada {
            if let Some(p) = pl.get(&id) {
                r.extend((0..3).map(|i| (p.origin[i] - t.origin[i]) * w));
                for e in [[1., 0., 0.], [0., 1., 0.]] {
                    let (u, v) = (p.dir(e), t.dir(e));
                    r.extend((0..3).map(|i| (u[i] - v[i]) * w * 20.));
                }
            }
        }
        r
    }

    /// Graus de liberdade na posição `x`. O batente e a puxada não tiram nenhum.
    fn gdl(&self, x: &[f64]) -> usize {
        let mut per = vec![];
        let r = self.igualdades(x, &mut per);
        if x.is_empty() || r.is_empty() {
            return x.len();
        }
        x.len().saturating_sub(matrix_rank(&jacobian(&|x, per| self.igualdades(x, per), x, &r), r.len(), x.len()))
    }

    /// Levenberg–Marquardt: devolve as incógnitas, o custo final e as iterações.
    fn minimizar(&self) -> (Vec<f64>, f64, usize) {
        let n = 6 * self.blocos.len();
        let mut per = vec![];
        let mut x = vec![0.; n];
        let mut r = self.residuos(&x, &mut per);
        let cost = |r: &[f64]| r.iter().map(|v| v * v).sum::<f64>();
        let mut c = cost(&r);
        let mut lambda = 1e-3;
        let mut it = 0;
        if n == 0 || r.is_empty() {
            return (x, c, it);
        }
        for k in 0..200 {
            it = k + 1;
            if c < 1e-20 {
                break;
            }
            let j = jacobian(&|x, per| self.residuos(x, per), &x, &r);
            // (JᵀJ + λ diag) dx = -Jᵀ r
            let mut jtj = vec![0.; n * n];
            let mut jtr = vec![0.; n];
            for row in 0..r.len() {
                for p in 0..n {
                    let jp = j[row * n + p];
                    if jp == 0. {
                        continue;
                    }
                    jtr[p] += jp * r[row];
                    for q in 0..n {
                        jtj[p * n + q] += jp * j[row * n + q];
                    }
                }
            }
            let mut improved = false;
            for _ in 0..12 {
                let mut m = jtj.clone();
                for p in 0..n {
                    m[p * n + p] += lambda * (1. + jtj[p * n + p]);
                }
                let Some(dx) = solve_linear(m, jtr.iter().map(|v| -v).collect(), n) else { break };
                let xn: Vec<f64> = x.iter().zip(&dx).map(|(a, d)| a + d).collect();
                let rn = self.residuos(&xn, &mut per);
                let cn = cost(&rn);
                if cn < c {
                    x = xn;
                    r = rn;
                    c = cn;
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
        (x, c, it)
    }
}

/// Graus de liberdade de cada componente (os outros parados): 0 = totalmente definido, 6 = solto.
/// Componentes fixos ficam com 0.
pub fn dof_per_comp(a: &Assembly) -> HashMap<Id, usize> {
    let b = bodies(a);
    let n = 6 * b.rel.len();
    let mut out: HashMap<Id, usize> = a.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).map(|n| (n.id, 0)).collect();
    if n == 0 {
        return out;
    }
    let mut vars = a.vars();
    vars.entry("t".into()).or_insert(0.);
    let mut per = vec![];
    let tudo = Escopo::tudo(a);
    let res = |x: &[f64], per: &mut Vec<(Id, usize, usize)>| all_residuals(a, &place(&b, a, x), &vars, true, per, &tudo);
    let x = vec![0.; n];
    let r = res(&x, &mut per);
    let j = if r.is_empty() { vec![] } else { jacobian(&res, &x, &r) };
    let m = r.len();
    for (k, comps) in b.rel.iter().enumerate() {
        // só as 6 colunas deste corpo
        let mut sub = vec![0.; m * 6];
        for row in 0..m {
            for c in 0..6 {
                sub[row * 6 + c] = j[row * n + 6 * k + c];
            }
        }
        let free = 6 - if m == 0 { 0 } else { matrix_rank(&sub, m, 6) };
        for (c, _) in comps {
            out.insert(*c, free);
        }
    }
    out
}

/// O que as restrições deixam um componente fazer ao longo de três eixos.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Liberdade {
    pub mover: [bool; 3],
    /// gira em torno de uma reta paralela ao eixo (não necessariamente pelo centro da peça)
    pub girar: [bool; 3],
    /// ponto por onde passa a reta do giro, quando as restrições é que mandam nela
    pub pivo: [Option<V3>; 3],
}

/// Acima disso o sistema do mecanismo inteiro pesa a cada seleção: olha-se só o próprio corpo.
const MAX_INCOGNITAS: usize = 240;

/// Movimentos permitidos ao componente `id`, ao longo de `eixos` (unitários, no mundo). `None` se o
/// corpo dele não tem incógnitas, isto é, está fixo.
///
/// Um movimento vale se existe uma velocidade dos outros corpos que mantém todas as restrições: em
/// termos do jacobiano J, a coluna do movimento está na imagem das demais colunas. Olhar só o corpo
/// escolhido, com os vizinhos parados, travaria o elo do meio de qualquer mecanismo. No giro as
/// translações do próprio corpo entram entre as demais colunas: uma dobradiça longe do centro da peça
/// gira em torno dela mesma, e a velocidade que sai da conta diz onde ela está.
pub fn liberdade(a: &Assembly, id: Id, eixos: [V3; 3], centro: V3) -> Option<Liberdade> {
    let b = bodies(a);
    let k = b.of.get(&id).copied().flatten()?;
    let n = 6 * b.rel.len();
    let mut vars = a.vars();
    vars.entry("t".into()).or_insert(0.);
    let mut per = vec![];
    let tudo = Escopo::tudo(a);
    let res = |x: &[f64], per: &mut Vec<(Id, usize, usize)>| all_residuals(a, &place(&b, a, x), &vars, true, per, &tudo);
    let x = vec![0.; n];
    let r = res(&x, &mut per);
    if r.is_empty() {
        return Some(Liberdade { mover: [true; 3], girar: [true; 3], pivo: [None; 3] });
    }
    let j = jacobian(&res, &x, &r);
    let m = r.len();
    let inicio = 6 * k;
    // só o próprio corpo, se o sistema for grande demais
    let outros: Vec<usize> = if n > MAX_INCOGNITAS { vec![] } else { (0..n).filter(|c| !(inicio..inicio + 6).contains(c)).collect() };
    let coluna = |c: usize| (0..m).map(|row| j[row * n + c]).collect::<Vec<f64>>();
    // melhor combinação das colunas `cols` para anular `alvo`: devolve (sobra, coeficientes)
    let anular = |alvo: &[f64], cols: &[usize]| -> (f64, Vec<f64>) {
        let q = cols.len();
        if q == 0 {
            return (alvo.iter().map(|v| v * v).sum::<f64>().sqrt(), vec![]);
        }
        let cs: Vec<Vec<f64>> = cols.iter().map(|c| coluna(*c)).collect();
        let mut nrm = vec![0.; q * q];
        let mut rhs = vec![0.; q];
        for p in 0..q {
            rhs[p] = -cs[p].iter().zip(alvo).map(|(u, v)| u * v).sum::<f64>();
            for s in p..q {
                let d: f64 = cs[p].iter().zip(&cs[s]).map(|(u, v)| u * v).sum();
                nrm[p * q + s] = d;
                nrm[s * q + p] = d;
            }
            // regularização: colunas livres (zeradas) não podem deixar o sistema singular
            nrm[p * q + p] += 1e-9;
        }
        let Some(y) = solve_linear(nrm, rhs, q) else { return (f64::MAX, vec![]) };
        let sobra = (0..m).map(|row| alvo[row] + cols.iter().zip(&y).map(|(c, v)| j[row * n + c] * v).sum::<f64>()).map(|v| v * v).sum::<f64>().sqrt();
        (sobra, y)
    };
    let tol = |alvo: &[f64]| 1e-5 * (1. + alvo.iter().map(|v| v * v).sum::<f64>().sqrt());
    let combinar = |base: usize, e: V3| (0..m).map(|row| (0..3).map(|i| j[row * n + base + i] * e[i]).sum::<f64>()).collect::<Vec<f64>>();
    let mut out = Liberdade { mover: [false; 3], girar: [false; 3], pivo: [None; 3] };
    let o = b.base[k].origin;
    for (i, e) in eixos.iter().enumerate() {
        let alvo = combinar(inicio, *e);
        out.mover[i] = anular(&alvo, &outros).0 < tol(&alvo);
        // girar em volta do centro: a origem do corpo anda v = e × (o − c)
        let vc = cross(*e, sub(o, centro));
        let giro = combinar(inicio + 3, *e);
        let alvo: Vec<f64> = giro.iter().zip(combinar(inicio, vc)).map(|(u, v)| u + v).collect();
        if anular(&alvo, &outros).0 < tol(&alvo) {
            out.girar[i] = true;
            continue;
        }
        // não dá no centro: procura a reta paralela que as restrições deixam
        let cols: Vec<usize> = outros.iter().copied().chain(inicio..inicio + 3).collect();
        let (sobra, y) = anular(&giro, &cols);
        if sobra < tol(&giro) {
            out.girar[i] = true;
            // as três últimas incógnitas são a velocidade da origem do corpo, v = e × (o − p)
            let v = [y[y.len() - 3], y[y.len() - 2], y[y.len() - 1]];
            let c = cross(*e, v);
            out.pivo[i] = Some([o[0] + c[0], o[1] + c[1], o[2] + c[2]]);
        }
    }
    Some(out)
}

/// Graus de liberdade restantes sem mexer em nada.
pub fn dof(a: &Assembly) -> usize {
    dof_with(a, true)
}

/// Graus de liberdade com ou sem os comandos das juntas (análise do mecanismo).
pub fn dof_with(a: &Assembly, drives: bool) -> usize {
    let b = bodies(a);
    let n = 6 * b.rel.len();
    if n == 0 {
        return 0;
    }
    let mut vars = a.vars();
    vars.entry("t".into()).or_insert(0.);
    let mut per = vec![];
    let tudo = Escopo::tudo(a);
    let res = |x: &[f64], per: &mut Vec<(Id, usize, usize)>| all_residuals(a, &place(&b, a, x), &vars, drives, per, &tudo);
    let x = vec![0.; n];
    let r = res(&x, &mut per);
    if r.is_empty() {
        return n;
    }
    n - matrix_rank(&jacobian(&res, &x, &r), r.len(), n)
}

fn jacobian(res: &dyn Fn(&[f64], &mut Vec<(Id, usize, usize)>) -> Vec<f64>, x: &[f64], r0: &[f64]) -> Vec<f64> {
    let n = x.len();
    let m = r0.len();
    let mut j = vec![0.; m * n];
    let mut per = vec![];
    let mut xp = x.to_vec();
    for p in 0..n {
        let h = 1e-7;
        xp[p] += h;
        let rp = res(&xp, &mut per);
        xp[p] -= 2. * h;
        let rm = res(&xp, &mut per);
        xp[p] += h;
        for row in 0..m.min(rp.len()).min(rm.len()) {
            j[row * n + p] = (rp[row] - rm[row]) / (2. * h);
        }
    }
    j
}

/// Eliminação de Gauss com pivotamento parcial.
fn solve_linear(mut a: Vec<f64>, mut b: Vec<f64>, n: usize) -> Option<Vec<f64>> {
    for c in 0..n {
        let p = (c..n).max_by(|i, j| a[i * n + c].abs().total_cmp(&a[j * n + c].abs()))?;
        if a[p * n + c].abs() < 1e-14 {
            return None;
        }
        if p != c {
            for k in 0..n {
                a.swap(c * n + k, p * n + k);
            }
            b.swap(c, p);
        }
        for r in c + 1..n {
            let f = a[r * n + c] / a[c * n + c];
            if f == 0. {
                continue;
            }
            for k in c..n {
                a[r * n + k] -= f * a[c * n + k];
            }
            b[r] -= f * b[c];
        }
    }
    let mut x = vec![0.; n];
    for r in (0..n).rev() {
        let mut s = b[r];
        for k in r + 1..n {
            s -= a[r * n + k] * x[k];
        }
        x[r] = s / a[r * n + r];
    }
    Some(x)
}

/// Posto numérico (eliminação com tolerância).
fn matrix_rank(j: &[f64], m: usize, n: usize) -> usize {
    let mut a = j.to_vec();
    let scale = a.iter().fold(0f64, |s, v| s.max(v.abs())).max(1e-12);
    let tol = 1e-7 * scale;
    let mut rank = 0;
    let mut row = 0;
    for c in 0..n {
        if row >= m {
            break;
        }
        let p = (row..m).max_by(|i, k| a[i * n + c].abs().total_cmp(&a[k * n + c].abs())).unwrap();
        if a[p * n + c].abs() < tol {
            continue;
        }
        for k in 0..n {
            a.swap(row * n + k, p * n + k);
        }
        for r in row + 1..m {
            let f = a[r * n + c] / a[row * n + c];
            for k in c..n {
                a[r * n + k] -= f * a[row * n + k];
            }
        }
        row += 1;
        rank += 1;
    }
    rank
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cubo(a: &mut Assembly, name: &str, at: V3) -> Id {
        a.add_component(name, NodeKind::Part { path: format!("{name}.fkp") }, Placement { origin: at, ..Default::default() }, None)
    }

    /// Faces de um cubo 10×10×10 com a origem no canto.
    fn topo() -> Geom {
        Geom::Plane { p: [5., 5., 10.], n: [0., 0., 1.] }
    }
    fn base() -> Geom {
        Geom::Plane { p: [5., 5., 0.], n: [0., 0., -1.] }
    }

    /// Peça presa à `alvo` por eixo, face e trava de giro: 0 GDL entre as duas.
    fn travar(a: &mut Assembly, alvo: Id, peca: Id) {
        let eixo = |comp| MateRef { comp, geom: Geom::Axis { p: [0.; 3], d: [0., 0., 1.] }, pick: None };
        let plano = |comp, n: V3| MateRef { comp, geom: Geom::Plane { p: [0.; 3], n }, pick: None };
        a.add_mate(MateKind::Coincident { flip: false }, eixo(alvo), eixo(peca));
        a.add_mate(MateKind::Coincident { flip: false }, plano(alvo, [0., 0., 1.]), plano(peca, [0., 0., 1.]));
        a.add_mate(MateKind::Parallel, plano(alvo, [1., 0., 0.]), plano(peca, [1., 0., 0.]));
    }

    #[test]
    fn peca_fixa_separa_os_aglomerados() {
        let mut a = Assembly::new("t");
        let base = cubo(&mut a, "Base", [0.; 3]);
        a.node_mut(base).unwrap().fixed = true;
        let (p1, p2) = (cubo(&mut a, "P1", [0., 0., 30.]), cubo(&mut a, "P2", [0., 0., 60.]));
        cubo(&mut a, "Solta", [0., 90., 0.]);
        a.add_mate(MateKind::Contact, MateRef { comp: base, geom: topo(), pick: None }, MateRef { comp: p1, geom: super::tests::base(), pick: None });
        a.add_mate(MateKind::Contact, MateRef { comp: base, geom: topo(), pick: None }, MateRef { comp: p2, geom: super::tests::base(), pick: None });
        let rep = solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        assert_eq!(rep.aglomerados, 2, "a base fixa não liga P1 a P2, e a solta não tem conta");
        assert_eq!(rep.incognitas, 12);
        assert_eq!(rep.dof, 3 + 3 + 6, "cada contato deixa 3, e a solta segue com 6");
    }

    #[test]
    fn par_travado_vira_um_bloco_e_o_gdl_nao_muda() {
        let mut a = Assembly::new("fixadores");
        let placa = cubo(&mut a, "Placa", [0.; 3]);
        a.node_mut(placa).unwrap().fixed = false;
        // a placa fica solta de propósito, com vinte fixadores travados nela
        let ids: Vec<Id> = (0..20).map(|k| cubo(&mut a, &format!("F{k}"), [0.; 3])).collect();
        for f in &ids {
            travar(&mut a, placa, *f);
        }
        let completo = dof_with(&a, true);
        let rep = solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        assert_eq!(rep.incognitas, 6, "vinte e um corpos, um bloco só");
        assert_eq!(rep.dof, completo, "reunir não muda o que sobra de liberdade");
        assert_eq!(rep.dof, 6);
    }

    #[test]
    fn travado_na_peca_fixa_sai_do_sistema() {
        let mut a = Assembly::new("t");
        let base = cubo(&mut a, "Base", [0.; 3]);
        a.node_mut(base).unwrap().fixed = true;
        let f1 = cubo(&mut a, "F1", [0.; 3]);
        let f2 = cubo(&mut a, "F2", [0.; 3]);
        travar(&mut a, base, f1);
        travar(&mut a, f1, f2);
        let rep = solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        assert_eq!((rep.incognitas, rep.dof), (0, 0));
    }

    #[test]
    fn par_fora_do_lugar_nao_e_reunido() {
        // as restrições travam, mas a peça ainda precisa andar até elas: é conta, não atalho
        let mut a = Assembly::new("t");
        let base = cubo(&mut a, "Base", [0.; 3]);
        a.node_mut(base).unwrap().fixed = true;
        let p = cubo(&mut a, "P", [7., 3., 11.]);
        travar(&mut a, base, p);
        let rep = solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        assert_eq!(rep.incognitas, 6);
        assert!(norm(a.node(p).unwrap().placement.origin) < 1e-6);
    }

    #[test]
    fn mover_a_peca_leva_o_bloco_inteiro() {
        let mut a = Assembly::new("t");
        let base = cubo(&mut a, "Base", [0.; 3]);
        a.node_mut(base).unwrap().fixed = true;
        let (placa, f) = (cubo(&mut a, "Placa", [0.; 3]), cubo(&mut a, "F", [0.; 3]));
        travar(&mut a, placa, f);
        let d = a.add_mate(MateKind::Distance("0".into()), MateRef { comp: base, geom: topo(), pick: None }, MateRef { comp: placa, geom: topo(), pick: None });
        a.mates.iter_mut().find(|m| m.id == d).unwrap().sentido = Sentido::Mesmo;
        assert!(solve(&mut a).converged);
        if let MateKind::Distance(e) = &mut a.mates.iter_mut().find(|m| m.id == d).unwrap().kind {
            *e = "15".into();
        }
        let rep = solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        let (zp, zf) = (a.node(placa).unwrap().placement.origin[2], a.node(f).unwrap().placement.origin[2]);
        assert!((zp - 15.).abs() < 1e-6 && (zf - 15.).abs() < 1e-6, "{zp} {zf}");
    }

    #[test]
    fn puxar_peca_sem_restricao_ainda_move() {
        let mut a = Assembly::new("t");
        let base = cubo(&mut a, "Base", [0.; 3]);
        a.node_mut(base).unwrap().fixed = true;
        let p = cubo(&mut a, "P", [0.; 3]);
        let alvo = Placement { origin: [5., 0., 0.], ..Default::default() };
        solve_inner(&mut a, &HashMap::new(), Some((p, alvo, 1.)));
        assert!((a.node(p).unwrap().placement.origin[0] - 5.).abs() < 1e-6);
    }

    #[test]
    fn contato_e_coincidencias_empilham() {
        let mut a = Assembly::new("pilha");
        let c1 = cubo(&mut a, "A", [0.; 3]);
        let c2 = cubo(&mut a, "B", [30., 7., 3.]);
        // B girado um pouco
        a.node_mut(c2).unwrap().placement = a.node(c2).unwrap().placement.moved([0.; 3], [0.1, 0.2, 0.3], [30., 7., 3.]);
        a.add_mate(MateKind::Contact, MateRef { comp: c1, geom: topo(), pick: None }, MateRef { comp: c2, geom: base(), pick: None });
        // faces laterais coplanares (x = 0 e y = 0)
        let fx = Geom::Plane { p: [0., 5., 5.], n: [-1., 0., 0.] };
        let fy = Geom::Plane { p: [5., 0., 5.], n: [0., -1., 0.] };
        a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: c1, geom: fx, pick: None }, MateRef { comp: c2, geom: fx, pick: None });
        a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: c1, geom: fy, pick: None }, MateRef { comp: c2, geom: fy, pick: None });
        let rep = solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        assert_eq!(rep.dof, 0, "totalmente restrito");
        let p = a.node(c2).unwrap().placement;
        assert!(norm(sub(p.origin, [0., 0., 10.])) < 1e-6, "{:?}", p.origin);
        assert!(norm(sub(p.dir([1., 0., 0.]), [1., 0., 0.])) < 1e-6);
    }

    #[test]
    fn eixo_coincidente_deixa_girar_e_deslizar() {
        let mut a = Assembly::new("eixo");
        let c1 = cubo(&mut a, "A", [0.; 3]);
        let c2 = cubo(&mut a, "B", [20., 5., 1.]);
        let ax = Geom::Axis { p: [5., 5., 0.], d: [0., 0., 1.] };
        a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: c1, geom: ax, pick: None }, MateRef { comp: c2, geom: ax, pick: None });
        let rep = solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        assert_eq!(rep.dof, 2, "gira e desliza no eixo");
        // distância entre topo de A e base de B: 4 mm de folga (planos paralelos)
        a.add_mate(MateKind::Distance("4".into()), MateRef { comp: c1, geom: topo(), pick: None }, MateRef { comp: c2, geom: Geom::Plane { p: [5., 5., 0.], n: [0., 0., 1.] }, pick: None });
        let rep = solve(&mut a);
        assert!(rep.converged && rep.dof == 1, "{rep:?}");
        assert!((a.node(c2).unwrap().placement.origin[2] - 14.).abs() < 1e-6);
    }

    #[test]
    fn fixacao_conjunta_e_conflito() {
        let mut a = Assembly::new("rigido");
        let c1 = cubo(&mut a, "A", [0.; 3]);
        let c2 = cubo(&mut a, "B", [20., 0., 0.]);
        let c3 = cubo(&mut a, "C", [40., 0., 0.]);
        a.add_rigid(vec![c2, c3]);
        a.add_mate(MateKind::Contact, MateRef { comp: c1, geom: topo(), pick: None }, MateRef { comp: c2, geom: base(), pick: None });
        let rep = solve(&mut a);
        assert!(rep.converged);
        // C acompanha B (mesma distância relativa)
        let (p2, p3) = (a.node(c2).unwrap().placement, a.node(c3).unwrap().placement);
        assert!((norm(sub(p3.origin, p2.origin)) - 20.).abs() < 1e-6);
        // conflito: duas distâncias diferentes entre os mesmos planos
        a.add_mate(MateKind::Distance("5".into()), MateRef { comp: c1, geom: topo(), pick: None }, MateRef { comp: c2, geom: Geom::Plane { p: [5., 5., 0.], n: [0., 0., 1.] }, pick: None });
        let rep = solve(&mut a);
        assert!(!rep.converged && !rep.failed.is_empty(), "{rep:?}");
    }

    #[test]
    fn engrenagem_inverter_e_gdl_por_peca() {
        let mut a = Assembly::new("engrenagens");
        let base = cubo(&mut a, "Base", [0.; 3]);
        a.node_mut(base).unwrap().fixed = true;
        let g1 = cubo(&mut a, "G1", [0.; 3]);
        let g2 = cubo(&mut a, "G2", [30., 0., 0.]);
        // cada engrenagem gira no seu eixo z (fixo na base)
        let ax1 = Geom::Axis { p: [0., 0., 0.], d: [0., 0., 1.] };
        let ax2 = Geom::Axis { p: [30., 0., 0.], d: [0., 0., 1.] };
        let own = Geom::Axis { p: [0., 0., 0.], d: [0., 0., 1.] };
        a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: base, geom: ax1, pick: None }, MateRef { comp: g1, geom: own, pick: None });
        a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: base, geom: ax2, pick: None }, MateRef { comp: g2, geom: own, pick: None });
        let z0 = Geom::Plane { p: [0., 0., 0.], n: [0., 0., 1.] };
        a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: base, geom: z0, pick: None }, MateRef { comp: g1, geom: z0, pick: None });
        a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: base, geom: z0, pick: None }, MateRef { comp: g2, geom: z0, pick: None });
        assert!(solve(&mut a).converged);
        let dofs = dof_per_comp(&a);
        assert_eq!(dofs[&g1], 1, "só gira");
        assert_eq!(dofs[&base], 0);
        // engrenagem 1:−2 (G2 gira metade, ao contrário)
        let gid = a.add_mate(MateKind::Gear { ratio: "-0.5".into(), belt: false, refs: [[0.; 3]; 4] }, MateRef { comp: g1, geom: own, pick: None }, MateRef { comp: g2, geom: own, pick: None });
        a.init_motion(gid);
        // gira G1 40° e resolve puxando G1 para lá
        let p1 = a.node(g1).unwrap().placement;
        let target = p1.moved([0.; 3], [0., 0., 40f64.to_radians()], [0.; 3]);
        solve_toward(&mut a, g1, target, 0.05);
        let ang = |p: &Placement| p.dir([1., 0., 0.])[1].atan2(p.dir([1., 0., 0.])[0]).to_degrees();
        let (t1, t2) = (ang(&a.node(g1).unwrap().placement), ang(&a.node(g2).unwrap().placement));
        assert!(t1 > 10., "G1 girou: {t1}");
        assert!((t2 + 0.5 * t1).abs() < 0.5, "G2 = −G1/2: {t1} {t2}");
        // inverter a coincidência dos planos: a peça vira de verdade
        let flip = a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: base, geom: Geom::Plane { p: [0., 0., 0.], n: [1., 0., 0.] }, pick: None }, MateRef { comp: g2, geom: Geom::Plane { p: [0., 0., 0.], n: [1., 0., 0.] }, pick: None });
        let _ = flip;
        let mut b = Assembly::new("virar");
        let f = cubo(&mut b, "F", [0.; 3]);
        b.node_mut(f).unwrap().fixed = true;
        let c = cubo(&mut b, "C", [20., 0., 0.]);
        let m = b.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: f, geom: topo(), pick: None }, MateRef { comp: c, geom: topo(), pick: None });
        assert!(solve(&mut b).converged);
        if let MateKind::Coincident { flip } = &mut b.mates.iter_mut().find(|x| x.id == m).unwrap().kind {
            *flip = true;
        }
        let rep = solve(&mut b);
        assert!(rep.converged, "inverteu: {rep:?}");
        assert!(b.node(c).unwrap().placement.dir([0., 0., 1.])[2] < -0.99);
    }

    /// Duas placas com faces paralelas: A na origem, B 12,5 acima, girado 30° em torno de z.
    fn duas_placas() -> (Assembly, Id, Id, MateRef, MateRef) {
        let mut a = Assembly::new("medida");
        let c1 = cubo(&mut a, "A", [0.; 3]);
        let c2 = cubo(&mut a, "B", [3., 4., 22.5]);
        let (ra, rb) = (MateRef { comp: c1, geom: topo(), pick: None }, MateRef { comp: c2, geom: base(), pick: None });
        (a, c1, c2, ra, rb)
    }

    fn valor(a: &Assembly, id: Id) -> String {
        match &a.mates.iter().find(|m| m.id == id).unwrap().kind {
            MateKind::Distance(e) | MateKind::Angle(e) => e.clone(),
            k => panic!("{k:?}"),
        }
    }

    #[test]
    fn distancia_nasce_com_a_medida_atual_e_nada_salta() {
        let (mut a, _, c2, ra, rb) = duas_placas();
        let antes = a.node(c2).unwrap().placement;
        let id = a.add_mate(MateKind::Distance("0".into()), ra, rb);
        assert!(a.iniciar_medida(id));
        assert_eq!(valor(&a, id), "12.5");
        assert_eq!(a.mates[0].sentido, Sentido::Oposto, "topo de A contra base de B");
        let rep = solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        assert_eq!(a.node(c2).unwrap().placement, antes, "resíduo zero: a peça fica onde estava");

        // abaixo da referência: o valor segue positivo, e quem guarda o lado é a restrição
        let (mut b, _, c2, ra, rb) = duas_placas();
        b.node_mut(c2).unwrap().placement.origin[2] = -7.;
        let id = b.add_mate(MateKind::Distance("0".into()), ra, rb);
        assert!(b.iniciar_medida(id));
        assert_eq!(valor(&b, id), "17");
        assert!(b.mates[0].lado_invertido);
        solve(&mut b);
        assert!((b.node(c2).unwrap().placement.origin[2] + 7.).abs() < 1e-9);
    }

    #[test]
    fn medida_de_cada_par_de_elementos() {
        let (p0, p1) = (Placement::default(), Placement { origin: [3., 4., 0.], ..Default::default() });
        let z = Geom::Axis { p: [0.; 3], d: [0., 0., 1.] };
        let o = Geom::Point([0.; 3]);
        let pl = Geom::Plane { p: [0.; 3], n: [1., 0., 0.] };
        let d = MateKind::Distance("0".into());
        assert_eq!(medida(&d, &o, &p0, &o, &p1), Some(5.));
        assert_eq!(medida(&d, &z, &p0, &z, &p1), Some(5.));
        assert_eq!(medida(&d, &o, &p0, &z, &p1), Some(5.));
        assert_eq!(medida(&d, &pl, &p0, &o, &p1), Some(3.));
        assert_eq!(medida(&d, &pl, &p1, &o, &p0), Some(-3.));
        let t = MateKind::Angle("0".into());
        assert_eq!(medida(&t, &pl, &p0, &o, &p1), None, "ponto não tem direção");
        assert_eq!(medida(&t, &pl, &p0, &pl, &p1), Some(0.));
        assert_eq!(medida(&t, &pl, &p0, &z, &p1), Some(0.), "eixo deitado no plano");
        assert_eq!(valor_cota(12.34567), "12.3457");
        assert_eq!(valor_cota(-0.00001), "0");
        assert_eq!(valor_cota(30.), "30");
    }

    #[test]
    fn angulo_nasce_com_a_medida_atual_mesmo_em_zero() {
        // B girado 25° em torno de z, com o eixo e o contato prendendo o resto
        let ax = Geom::Axis { p: [0., 0., 0.], d: [0., 0., 1.] };
        let fy = Geom::Plane { p: [5., 0., 5.], n: [0., -1., 0.] };
        for graus in [25., 0., 180.] {
            let mut a = Assembly::new("giro");
            let c1 = cubo(&mut a, "A", [0.; 3]);
            let c2 = cubo(&mut a, "B", [0., 0., 10.]);
            a.node_mut(c2).unwrap().placement = a.node(c2).unwrap().placement.moved([0.; 3], [0., 0., f64::to_radians(graus)], [0., 0., 10.]);
            a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: c1, geom: ax, pick: None }, MateRef { comp: c2, geom: ax, pick: None });
            a.add_mate(MateKind::Contact, MateRef { comp: c1, geom: topo(), pick: None }, MateRef { comp: c2, geom: base(), pick: None });
            let antes = a.node(c2).unwrap().placement;
            let id = a.add_mate(MateKind::Angle("90".into()), MateRef { comp: c1, geom: fy, pick: None }, MateRef { comp: c2, geom: fy, pick: None });
            assert!(a.iniciar_medida(id));
            assert_eq!(valor(&a, id), valor_cota(graus));
            let rep = solve(&mut a);
            assert!(rep.converged, "{graus}: {rep:?}");
            // em 0° e 180° o cosseno era plano e o giro ficava contado como livre
            assert_eq!(rep.dof, 0, "{graus}: o ângulo tira o giro");
            assert!(norm(sub(a.node(c2).unwrap().placement.origin, antes.origin)) < 1e-9, "{graus}");
            assert!(norm(sub(a.node(c2).unwrap().placement.dir([1., 0., 0.]), antes.dir([1., 0., 0.]))) < 1e-6, "{graus}");
        }
    }

    #[test]
    fn inverter_lado_e_inverter_alinhamento_sao_coisas_diferentes() {
        let (mut a, _, c2, ra, rb) = duas_placas();
        let id = a.add_mate(MateKind::Distance("0".into()), ra, rb);
        a.iniciar_medida(id);
        assert!(solve(&mut a).converged);

        // lado: a peça passa para o outro lado da referência, sem virar
        assert!(a.inverter_lado(id));
        assert!(solve(&mut a).converged);
        let p = a.node(c2).unwrap().placement;
        assert!((p.point([5., 5., 0.])[2] - (10. - 12.5)).abs() < 1e-6, "{p:?}");
        assert!(p.dir([0., 0., 1.])[2] > 0.99, "não virou");

        // alinhamento: a peça vira, e a cota continua do mesmo lado
        assert!(a.inverter_alinhamento(id));
        assert_eq!(a.mates[0].sentido, Sentido::Mesmo);
        assert!(solve(&mut a).converged);
        let p = a.node(c2).unwrap().placement;
        assert!((p.point([5., 5., 0.])[2] - (10. - 12.5)).abs() < 1e-6, "{p:?}");
        assert!(p.dir([0., 0., 1.])[2] < -0.99, "virou");

        // e voltam
        assert!(a.inverter_alinhamento(id) && a.inverter_lado(id));
        assert!(solve(&mut a).converged);
        let p = a.node(c2).unwrap().placement;
        assert!((p.point([5., 5., 0.])[2] - 22.5).abs() < 1e-6 && p.dir([0., 0., 1.])[2] > 0.99, "{p:?}");
    }

    #[test]
    fn inverter_contato_vira_coincidencia_de_faces() {
        let (mut a, _, c2, ra, rb) = duas_placas();
        let id = a.add_mate(MateKind::Contact, ra, rb);
        assert!(solve(&mut a).converged);
        assert!(!a.inverter_lado(id), "contato não tem cota");
        assert!(a.inverter_alinhamento(id));
        assert_eq!(a.mates[0].kind, MateKind::Coincident { flip: false });
        assert!(solve(&mut a).converged);
        assert!(a.node(c2).unwrap().placement.dir([0., 0., 1.])[2] < -0.99);
    }

    #[test]
    fn angulo_com_alinhamento_invertido_vira_o_suplemento() {
        let fy = Geom::Plane { p: [5., 0., 5.], n: [0., -1., 0.] };
        let ax = Geom::Axis { p: [0., 0., 0.], d: [0., 0., 1.] };
        let mut a = Assembly::new("suplemento");
        let c1 = cubo(&mut a, "A", [0.; 3]);
        let c2 = cubo(&mut a, "B", [0., 0., 10.]);
        a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: c1, geom: ax, pick: None }, MateRef { comp: c2, geom: ax, pick: None });
        a.add_mate(MateKind::Contact, MateRef { comp: c1, geom: topo(), pick: None }, MateRef { comp: c2, geom: base(), pick: None });
        let id = a.add_mate(MateKind::Angle("30".into()), MateRef { comp: c1, geom: fy, pick: None }, MateRef { comp: c2, geom: fy, pick: None });
        assert!(solve(&mut a).converged);
        assert!(a.inverter_alinhamento(id));
        assert!(solve(&mut a).converged);
        let n2 = a.node(c2).unwrap().placement.dir([0., -1., 0.]);
        assert!((dot(n2, [0., -1., 0.]) - 150f64.to_radians().cos()).abs() < 1e-6, "{n2:?}");
    }

    #[test]
    fn angulo_digitado_sai_de_faces_alinhadas() {
        // nasceu em 0° e a pessoa digita 30: o giro parte do ponto em que o cosseno é plano
        let fy = Geom::Plane { p: [5., 0., 5.], n: [0., -1., 0.] };
        // eixo x, para o empurrão não cair por sorte na direção livre
        let ax = Geom::Axis { p: [0., 5., 5.], d: [1., 0., 0.] };
        let lado = Geom::Plane { p: [0., 5., 5.], n: [-1., 0., 0.] };
        let mut a = Assembly::new("alinhados");
        let c1 = cubo(&mut a, "A", [0.; 3]);
        let c2 = cubo(&mut a, "B", [0.; 3]);
        a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: c1, geom: ax, pick: None }, MateRef { comp: c2, geom: ax, pick: None });
        a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: c1, geom: lado, pick: None }, MateRef { comp: c2, geom: lado, pick: None });
        let id = a.add_mate(MateKind::Angle("0".into()), MateRef { comp: c1, geom: fy, pick: None }, MateRef { comp: c2, geom: fy, pick: None });
        a.iniciar_medida(id);
        assert_eq!(valor(&a, id), "0");
        assert!(solve(&mut a).converged);
        if let MateKind::Angle(e) = &mut a.mates[2].kind {
            *e = "30".into();
        }
        let rep = solve(&mut a);
        assert!(rep.converged && rep.dof == 0, "{rep:?}");
        let n2 = a.node(c2).unwrap().placement.dir([0., -1., 0.]);
        assert!((dot(n2, [0., -1., 0.]) - 30f64.to_radians().cos()).abs() < 1e-6, "{n2:?}");
    }

    #[test]
    fn formula_invalida_nao_move_a_peca_e_aparece_como_falha() {
        let (mut a, _, c2, ra, rb) = duas_placas();
        let id = a.add_mate(MateKind::Distance("0".into()), ra, rb);
        a.iniciar_medida(id);
        assert!(solve(&mut a).converged);
        let antes = a.node(c2).unwrap().placement;
        if let MateKind::Distance(e) = &mut a.mates[0].kind {
            *e = "folga +".into();
        }
        let rep = solve(&mut a);
        assert!(!rep.converged && rep.failed == vec![id], "{rep:?}");
        assert_eq!(a.node(c2).unwrap().placement, antes);
        // com o parâmetro certo a mesma fórmula funciona
        a.params.push(AsmParam { name: "folga".into(), expr: "4".into() });
        if let MateKind::Distance(e) = &mut a.mates[0].kind {
            *e = "folga + 1".into();
        }
        assert!(solve(&mut a).converged);
        assert!((a.node(c2).unwrap().placement.point([5., 5., 0.])[2] - 15.).abs() < 1e-6);
    }

    #[test]
    fn pares_sem_sentido_e_compatibilidade_com_arquivos_antigos() {
        let o = Geom::Point([0.; 3]);
        let z = Geom::Axis { p: [0.; 3], d: [0., 0., 1.] };
        assert!(!par_valido(&MateKind::Angle("0".into()), &o, &z));
        assert!(!par_valido(&MateKind::Parallel, &z, &o));
        assert!(!par_valido(&MateKind::Contact, &o, &z));
        assert!(par_valido(&MateKind::Coincident { flip: false }, &o, &z));
        assert!(par_valido(&MateKind::Distance("1".into()), &o, &z));

        // restrição gravada sem os campos novos abre com o comportamento de antes
        let (mut a, _, _, ra, rb) = duas_placas();
        a.add_mate(MateKind::Distance("5".into()), ra, rb);
        let json = a.to_json().replace("\"sentido\": \"Livre\",", "").replace("\"lado_invertido\": false", "\"lado_antigo\": 0");
        let b = Assembly::from_json(&json).unwrap();
        assert_eq!(b.mates[0].sentido, Sentido::Livre);
        assert!(!b.mates[0].lado_invertido);
    }

    #[test]
    fn angulo_entre_faces() {
        let mut a = Assembly::new("angulo");
        let c1 = cubo(&mut a, "A", [0.; 3]);
        let c2 = cubo(&mut a, "B", [20., 0., 0.]);
        let fy = Geom::Plane { p: [5., 0., 5.], n: [0., -1., 0.] };
        let ax = Geom::Axis { p: [0., 0., 0.], d: [0., 0., 1.] };
        a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: c1, geom: ax, pick: None }, MateRef { comp: c2, geom: ax, pick: None });
        a.add_mate(MateKind::Contact, MateRef { comp: c1, geom: topo(), pick: None }, MateRef { comp: c2, geom: base(), pick: None });
        a.add_mate(MateKind::Angle("30".into()), MateRef { comp: c1, geom: fy, pick: None }, MateRef { comp: c2, geom: fy, pick: None });
        let rep = solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        let n2 = a.node(c2).unwrap().placement.dir([0., -1., 0.]);
        assert!((dot(n2, [0., -1., 0.]) - 30f64.to_radians().cos()).abs() < 1e-6);
    }
}
