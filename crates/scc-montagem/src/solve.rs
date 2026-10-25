//! Solucionador de posicionamento: cada corpo livre tem 6 graus de liberdade (translação e
//! rotação em torno do próprio centro); as restrições viram resíduos e o Levenberg–Marquardt
//! leva todos a zero. Componentes fixos não entram como incógnita; uma fixação conjunta põe
//! várias peças no mesmo corpo.

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

/// Resíduos de uma restrição (vetor que deve ir a zero).
fn residuals(kind: &MateKind, a: World, b: World, vars: &HashMap<String, f64>, out: &mut Vec<f64>) {
    use World::*;
    let num = |e: &String| expr::eval_str(e, vars).unwrap_or(0.);
    let par = |u: V3, v: V3, out: &mut Vec<f64>| out.extend(cross(u, v));
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
            out.push(dot(sub(pb, pa), na) - num(e));
        }
        (MateKind::Distance(e), Point(p), Plane(o, n)) | (MateKind::Distance(e), Plane(o, n), Point(p)) => out.push(dot(sub(p, o), n) - num(e)),
        (MateKind::Distance(e), Point(pa), Point(pb)) => out.push(norm(sub(pb, pa)) - num(e)),
        (MateKind::Distance(e), Axis(pa, da), Axis(pb, db)) => {
            par(da, db, out);
            out.push(norm(perp(sub(pb, pa), da)) - num(e));
        }
        (MateKind::Distance(e), Axis(o, d), Plane(q, n)) | (MateKind::Distance(e), Plane(q, n), Axis(o, d)) => {
            out.push(dot(d, n));
            out.push(dot(sub(o, q), n) - num(e));
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
                MateKind::Angle(e) => {
                    let c = num(e).to_radians().cos();
                    out.push(if mixed { dot(u, v).abs() - (std::f64::consts::FRAC_PI_2 - num(e).to_radians()).cos() } else { dot(u, v) - c });
                }
                _ => {}
            }
        }
    }
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

fn all_residuals(a: &Assembly, pl: &HashMap<Id, Placement>, vars: &HashMap<String, f64>, drives: bool, per: &mut Vec<(Id, usize, usize)>) -> Vec<f64> {
    let mut r = vec![];
    per.clear();
    let num = |e: &String| expr::eval_str(e, vars).unwrap_or(0.);
    for m in a.mates.iter().filter(|m| !m.suppressed) {
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
            k => residuals(k, wa, wb, vars, &mut r),
        }
        per.push((m.id, s, r.len()));
    }
    crate::kin::residuals(a, pl, vars, drives, &mut r, per);
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
    for i in 0..a.mates.len() {
        let m = a.mates[i].clone();
        if m.suppressed {
            continue;
        }
        let s = match (&m.kind, m.a.geom, m.b.geom) {
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
pub fn solve_toward(a: &mut Assembly, id: Id, target: Placement, weight: f64) -> SolveReport {
    solve_inner(a, &HashMap::new(), Some((id, target, weight)))
}

fn solve_inner(a: &mut Assembly, extra: &HashMap<String, f64>, pull: Option<(Id, Placement, f64)>) -> SolveReport {
    seed_flips(a);
    let b = bodies(a);
    let n = 6 * b.rel.len();
    let mut vars = a.vars();
    vars.extend(extra.iter().map(|(k, v)| (k.clone(), *v)));
    vars.entry("t".into()).or_insert(0.);
    let mut x = vec![0.; n];
    let mut per = vec![];
    let res = |x: &[f64], per: &mut Vec<(Id, usize, usize)>| {
        let pl = place(&b, a, x);
        let mut r = all_residuals(a, &pl, &vars, true, per);
        if let Some((id, t, w)) = pull {
            if let Some(p) = pl.get(&id) {
                r.extend((0..3).map(|i| (p.origin[i] - t.origin[i]) * w));
                for e in [[1., 0., 0.], [0., 1., 0.]] {
                    let (u, v) = (p.dir(e), t.dir(e));
                    r.extend((0..3).map(|i| (u[i] - v[i]) * w * 20.));
                }
            }
        }
        r
    };
    let mut r = res(&x, &mut per);
    let cost = |r: &[f64]| r.iter().map(|v| v * v).sum::<f64>();
    let mut c = cost(&r);
    let mut lambda = 1e-3;
    let mut it = 0;
    let mut rank = 0;
    if n > 0 && !r.is_empty() {
        for k in 0..200 {
            it = k + 1;
            if c < 1e-20 {
                break;
            }
            let j = jacobian(&res, &x, &r);
            rank = matrix_rank(&j, r.len(), n);
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
                let rn = res(&xn, &mut per);
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
    } else if !r.is_empty() {
        rank = 0;
    }
    if rank == 0 && n > 0 && !r.is_empty() {
        rank = matrix_rank(&jacobian(&res, &x, &r), r.len(), n);
    }
    // restrições que ficaram de fora (conflito)
    let pl = place(&b, a, &x);
    let mut failed = vec![];
    let r2 = all_residuals(a, &pl, &vars, true, &mut per);
    let _ = &res;
    for (id, s, e) in &per {
        if r2[*s..*e].iter().any(|v| v.abs() > 1e-6) {
            failed.push(*id);
        }
    }
    // grava as posições
    for nd in a.nodes.iter_mut() {
        if let Some(p) = pl.get(&nd.id) {
            nd.placement = *p;
        }
    }
    // as repetições acompanham os originais nas posições novas
    a.update_patterns();
    SolveReport { residual: c.sqrt(), iterations: it, dof: n.saturating_sub(rank), failed: failed.clone(), converged: failed.is_empty() }
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
    let res = |x: &[f64], per: &mut Vec<(Id, usize, usize)>| all_residuals(a, &place(&b, a, x), &vars, true, per);
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
    let res = |x: &[f64], per: &mut Vec<(Id, usize, usize)>| all_residuals(a, &place(&b, a, x), &vars, drives, per);
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
        a.add_component(name, NodeKind::Part { path: format!("{name}.sccpart") }, Placement { origin: at, ..Default::default() }, None)
    }

    /// Faces de um cubo 10×10×10 com a origem no canto.
    fn topo() -> Geom {
        Geom::Plane { p: [5., 5., 10.], n: [0., 0., 1.] }
    }
    fn base() -> Geom {
        Geom::Plane { p: [5., 5., 0.], n: [0., 0., -1.] }
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
