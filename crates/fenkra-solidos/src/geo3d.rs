//! Geometria 3D de referência: pontos, curvas (como polilinhas densas, com dados analíticos
//! quando existem) e planos construídos a partir deles. Tudo em Rust, sem o kernel.

pub use fenkra_kernel::cross;
use fenkra_kernel::Frame;
use std::f64::consts::{PI, TAU};

pub type V3 = [f64; 3];

pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn mul(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}
pub fn dist(a: V3, b: V3) -> f64 {
    norm(sub(a, b))
}
pub fn lerp(a: V3, b: V3, t: f64) -> V3 {
    add(a, mul(sub(b, a), t))
}
pub fn unit(a: V3) -> Option<V3> {
    let n = norm(a);
    (n > 1e-12).then(|| mul(a, 1. / n))
}
/// Gira `v` em torno do eixo unitário `k` (Rodrigues).
pub fn rotate(v: V3, k: V3, a: f64) -> V3 {
    let (c, s) = (a.cos(), a.sin());
    let kv = cross(k, v);
    let kd = dot(k, v);
    [0, 1, 2].map(|i| v[i] * c + kv[i] * s + k[i] * kd * (1. - c))
}

/// Direção perpendicular qualquer (estável).
pub fn any_perp(n: V3) -> V3 {
    let h = if n[0].abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] };
    unit(sub(h, mul(n, dot(h, n)))).unwrap_or([0., 1., 0.])
}

/// Sistema do plano a partir da origem e da normal; o X segue `hint` projetado no plano.
pub fn frame_from(origin: V3, normal: V3, hint: Option<V3>) -> Result<Frame, String> {
    let n = unit(normal).ok_or("normal nula: os elementos não definem um plano")?;
    let x = hint.and_then(|h| unit(sub(h, mul(n, dot(h, n))))).unwrap_or_else(|| {
        // X global quando possível (planos "de pé" usam o X da Origem como as vistas)
        let gx = [1., 0., 0.];
        unit(sub(gx, mul(n, dot(gx, n)))).unwrap_or_else(|| any_perp(n))
    });
    Ok(Frame { origin, x, normal: n })
}

/// Curva avaliada: polilinha densa + dados analíticos de círculo, se houver.
#[derive(Clone, Debug, PartialEq)]
pub struct Curve3 {
    pub pts: Vec<V3>,
    pub closed: bool,
    /// (centro, normal, raio) quando a curva é um círculo/arco
    pub circle: Option<(V3, V3, f64)>,
}

impl Curve3 {
    pub fn poly(pts: Vec<V3>, closed: bool) -> Result<Curve3, String> {
        if pts.len() < 2 {
            return Err("curva com menos de dois pontos".into());
        }
        let mut pts = pts;
        if closed && dist(pts[0], *pts.last().unwrap()) > 1e-9 {
            pts.push(pts[0]);
        }
        Ok(Curve3 { pts, closed, circle: None })
    }
    pub fn line(a: V3, b: V3) -> Result<Curve3, String> {
        if dist(a, b) < 1e-9 {
            return Err("linha de comprimento zero (pontos coincidentes)".into());
        }
        Curve3::poly(vec![a, b], false)
    }
    pub fn start(&self) -> V3 {
        self.pts[0]
    }
    pub fn end(&self) -> V3 {
        *self.pts.last().unwrap()
    }
    pub fn length(&self) -> f64 {
        self.pts.windows(2).map(|w| dist(w[0], w[1])).sum()
    }
    pub fn is_straight(&self) -> bool {
        let (a, b) = (self.start(), self.end());
        let Some(d) = unit(sub(b, a)) else { return false };
        let tol = 1e-6 * self.length().max(1.);
        self.pts.iter().all(|p| norm(cross(sub(*p, a), d)) < tol)
    }
    /// Ponto e tangente unitária a uma distância `s` do início (limitada à curva).
    pub fn at_dist(&self, s: f64) -> (V3, V3) {
        let mut acc = 0.;
        let n = self.pts.len();
        for i in 0..n - 1 {
            let (a, b) = (self.pts[i], self.pts[i + 1]);
            let l = dist(a, b);
            if acc + l >= s || i == n - 2 {
                let t = if l > 1e-12 { ((s - acc) / l).clamp(0., 1.) } else { 0. };
                return (lerp(a, b, t), self.tangent_seg(i));
            }
            acc += l;
        }
        (self.end(), self.tangent_seg(n - 2))
    }
    fn tangent_seg(&self, i: usize) -> V3 {
        // média dos segmentos vizinhos suaviza a tangente da polilinha
        let d = |j: usize| unit(sub(self.pts[j + 1], self.pts[j])).unwrap_or([1., 0., 0.]);
        d(i)
    }
    pub fn at_ratio(&self, t: f64) -> (V3, V3) {
        self.at_dist(t * self.length())
    }
    /// Distância ao longo da curva do ponto mais próximo de `p`, e o ponto.
    pub fn nearest(&self, p: V3) -> (f64, V3) {
        let mut best = (f64::MAX, 0., self.start());
        let mut acc = 0.;
        for w in self.pts.windows(2) {
            let (a, b) = (w[0], w[1]);
            let ab = sub(b, a);
            let l2 = dot(ab, ab).max(1e-24);
            let t = (dot(sub(p, a), ab) / l2).clamp(0., 1.);
            let q = lerp(a, b, t);
            let d = dist(p, q);
            if d < best.0 {
                best = (d, acc + t * l2.sqrt(), q);
            }
            acc += l2.sqrt();
        }
        (best.1, best.2)
    }
    /// Tangente unitária na ponta, apontando para fora da curva.
    pub fn end_tangent(&self, at_end: bool) -> V3 {
        let n = self.pts.len();
        if at_end {
            unit(sub(self.pts[n - 1], self.pts[n - 2])).unwrap_or([1., 0., 0.])
        } else {
            unit(sub(self.pts[0], self.pts[1])).unwrap_or([1., 0., 0.])
        }
    }
    /// Vetor curvatura (d²p/ds²) aproximado na ponta.
    pub fn end_curvature(&self, at_end: bool) -> V3 {
        let n = self.pts.len();
        if n < 3 || self.circle.is_none() && self.is_straight() {
            return [0.; 3];
        }
        let (p0, p1, p2) = if at_end { (self.pts[n - 3], self.pts[n - 2], self.pts[n - 1]) } else { (self.pts[2], self.pts[1], self.pts[0]) };
        let (h1, h2) = (dist(p0, p1).max(1e-12), dist(p1, p2).max(1e-12));
        let d1 = mul(sub(p1, p0), 1. / h1);
        let d2 = mul(sub(p2, p1), 1. / h2);
        mul(sub(d2, d1), 2. / (h1 + h2))
    }
    /// Terceira derivada (d³p/ds³) aproximada na ponta, no sentido para fora da curva.
    pub fn end_jerk(&self, at_end: bool) -> V3 {
        let n = self.pts.len();
        if n < 4 || self.circle.is_none() && self.is_straight() {
            return [0.; 3];
        }
        let p: [V3; 4] = if at_end { [self.pts[n - 4], self.pts[n - 3], self.pts[n - 2], self.pts[n - 1]] } else { [self.pts[3], self.pts[2], self.pts[1], self.pts[0]] };
        let h = (dist(p[0], p[1]) + dist(p[1], p[2]) + dist(p[2], p[3])) / 3.;
        if h < 1e-12 {
            return [0.; 3];
        }
        [0, 1, 2].map(|k| (p[3][k] - 3. * p[2][k] + 3. * p[1][k] - p[0][k]) / (h * h * h))
    }
    /// Plano que contém a curva (ajuste), se ela for planar.
    pub fn plane(&self) -> Result<Frame, String> {
        if let Some((c, n, _)) = self.circle {
            return frame_from(c, n, Some(sub(self.start(), c)));
        }
        if self.is_straight() {
            return Err("a curva é reta: não define um plano sozinha".into());
        }
        let (f, dev) = fit_plane(&self.pts)?;
        let size = self.length().max(1.);
        if dev > 1e-4 * size + 1e-6 {
            return Err(format!("a curva não é planar (desvio {:.3} mm)", dev));
        }
        Ok(f)
    }
}

/// Plano de melhor ajuste (mínimos quadrados) e o maior desvio dos pontos.
pub fn fit_plane(pts: &[V3]) -> Result<(Frame, f64), String> {
    if pts.len() < 3 {
        return Err("são precisos pelo menos três pontos".into());
    }
    let n = pts.len() as f64;
    let c = mul(pts.iter().fold([0.; 3], |a, p| add(a, *p)), 1. / n);
    let mut m = [[0.; 3]; 3];
    for p in pts {
        let d = sub(*p, c);
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += d[i] * d[j];
            }
        }
    }
    let (vals, vecs) = jacobi(m);
    let mut idx = [0, 1, 2];
    idx.sort_by(|a, b| vals[*a].total_cmp(&vals[*b]));
    if vals[idx[1]] < 1e-12 {
        return Err("os pontos estão alinhados: não definem um plano".into());
    }
    let normal = [vecs[0][idx[0]], vecs[1][idx[0]], vecs[2][idx[0]]];
    let major = [vecs[0][idx[2]], vecs[1][idx[2]], vecs[2][idx[2]]];
    let f = frame_from(c, normal, Some(major))?;
    let dev = pts.iter().map(|p| dot(sub(*p, c), f.normal).abs()).fold(0., f64::max);
    Ok((f, dev))
}

/// Autovalores/autovetores de matriz simétrica 3×3 (Jacobi). Vetores nas colunas.
fn jacobi(mut a: [[f64; 3]; 3]) -> ([f64; 3], [[f64; 3]; 3]) {
    let mut v = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for _ in 0..50 {
        let (mut p, mut q, mut big) = (0, 1, 0.);
        for i in 0..3 {
            for j in i + 1..3 {
                if a[i][j].abs() > big {
                    big = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if big < 1e-15 {
            break;
        }
        let th = 0.5 * (2. * a[p][q]).atan2(a[q][q] - a[p][p]);
        let (c, s) = (th.cos(), th.sin());
        for k in 0..3 {
            let (akp, akq) = (a[k][p], a[k][q]);
            a[k][p] = c * akp - s * akq;
            a[k][q] = s * akp + c * akq;
        }
        for k in 0..3 {
            let (apk, aqk) = (a[p][k], a[q][k]);
            a[p][k] = c * apk - s * aqk;
            a[q][k] = s * apk + c * aqk;
        }
        for k in 0..3 {
            let (vkp, vkq) = (v[k][p], v[k][q]);
            v[k][p] = c * vkp - s * vkq;
            v[k][q] = s * vkp + c * vkq;
        }
    }
    ([a[0][0], a[1][1], a[2][2]], v)
}

/// Círculo pelos três pontos: (centro, normal, raio).
pub fn circle3(a: V3, b: V3, c: V3) -> Result<(V3, V3, f64), String> {
    let (ab, ac) = (sub(b, a), sub(c, a));
    let n = cross(ab, ac);
    let n2 = dot(n, n);
    if n2 < 1e-18 {
        return Err("os três pontos estão alinhados".into());
    }
    let t = add(mul(cross(n, ab), dot(ac, ac)), mul(cross(ac, n), dot(ab, ab)));
    let ctr = add(a, mul(t, 1. / (2. * n2)));
    Ok((ctr, unit(n).unwrap(), dist(ctr, a)))
}

/// Círculo que melhor descreve a polilinha (centro, normal, raio), se ela for circular.
pub fn fit_circle(c: &Curve3) -> Result<(V3, V3, f64), String> {
    if let Some(ci) = c.circle {
        return Ok(ci);
    }
    let n = c.pts.len();
    if n < 3 {
        return Err("a curva não é um círculo nem um arco".into());
    }
    let (ctr, nn, r) = circle3(c.pts[0], c.pts[n / 3], c.pts[(2 * n) / 3])?;
    let ok = c.pts.iter().all(|p| (dist(*p, ctr) - r).abs() < 1e-3 * r.max(1.));
    if !ok {
        return Err("a curva não é um círculo nem um arco".into());
    }
    Ok((ctr, nn, r))
}

/// Arco de `a0` a `a1` (rad) no plano (centro, x, y).
pub fn arc_pts(ctr: V3, x: V3, y: V3, r: f64, a0: f64, a1: f64) -> Vec<V3> {
    let n = (((a1 - a0).abs() / TAU) * 128.).ceil().max(8.) as usize;
    (0..=n)
        .map(|i| {
            let t = a0 + (a1 - a0) * i as f64 / n as f64;
            add(ctr, add(mul(x, r * t.cos()), mul(y, r * t.sin())))
        })
        .collect()
}

pub fn circle_curve(ctr: V3, f: &Frame, r: f64, a0: f64, a1: f64) -> Result<Curve3, String> {
    if r <= 0. {
        return Err("raio precisa ser positivo".into());
    }
    let full = (a1 - a0).abs() >= TAU - 1e-9;
    let pts = arc_pts(ctr, f.x, f.y(), r, a0, if full { a0 + TAU } else { a1 });
    let mut c = Curve3::poly(pts, full)?;
    c.circle = Some((ctr, f.normal, r));
    Ok(c)
}

/// Círculo ou arco (do 1º ao 3º passando pelo 2º) por três pontos.
pub fn circle3_curve(a: V3, b: V3, c: V3, full: bool) -> Result<Curve3, String> {
    let (ctr, n, r) = circle3(a, b, c)?;
    let f = frame_from(ctr, n, Some(sub(a, ctr)))?;
    let ang = |p: V3| {
        let d = sub(p, ctr);
        let t = dot(d, f.y()).atan2(dot(d, f.x));
        if t < 0. { t + TAU } else { t }
    };
    if full {
        return circle_curve(ctr, &f, r, 0., TAU);
    }
    // a normal vem de (b−a)×(c−a): o sentido anti-horário passa por b antes de c
    let (tb, tc) = (ang(b), ang(c));
    let end = if tb < tc { tc } else { tc + TAU };
    circle_curve(ctr, &f, r, 0., end)
}

/// Spline interpolada (Catmull-Rom centrípeta) pelos pontos.
pub fn spline(pts: &[V3], closed: bool) -> Result<Curve3, String> {
    if pts.len() < 2 {
        return Err("a spline precisa de pelo menos dois pontos".into());
    }
    if pts.len() == 2 {
        return Curve3::line(pts[0], pts[1]);
    }
    let n = pts.len();
    let get = |i: isize| -> V3 {
        if closed {
            pts[i.rem_euclid(n as isize) as usize]
        } else if i < 0 {
            // ponta: reflete para tangente natural
            sub(mul(pts[0], 2.), pts[1])
        } else if i as usize >= n {
            sub(mul(pts[n - 1], 2.), pts[n - 2])
        } else {
            pts[i as usize]
        }
    };
    let spans = if closed { n } else { n - 1 };
    let mut out = vec![];
    for s in 0..spans as isize {
        let (p0, p1, p2, p3) = (get(s - 1), get(s), get(s + 1), get(s + 2));
        let kt = |a: V3, b: V3| dist(a, b).sqrt().max(1e-9);
        let t0 = 0.;
        let t1 = t0 + kt(p0, p1);
        let t2 = t1 + kt(p1, p2);
        let t3 = t2 + kt(p2, p3);
        let steps = 24;
        for k in 0..steps {
            let t = t1 + (t2 - t1) * k as f64 / steps as f64;
            let a1 = add(mul(p0, (t1 - t) / (t1 - t0)), mul(p1, (t - t0) / (t1 - t0)));
            let a2 = add(mul(p1, (t2 - t) / (t2 - t1)), mul(p2, (t - t1) / (t2 - t1)));
            let a3 = add(mul(p2, (t3 - t) / (t3 - t2)), mul(p3, (t - t2) / (t3 - t2)));
            let b1 = add(mul(a1, (t2 - t) / (t2 - t0)), mul(a2, (t - t0) / (t2 - t0)));
            let b2 = add(mul(a2, (t3 - t) / (t3 - t1)), mul(a3, (t - t1) / (t3 - t1)));
            out.push(add(mul(b1, (t2 - t) / (t2 - t1)), mul(b2, (t - t1) / (t2 - t1))));
        }
    }
    out.push(if closed { pts[0] } else { pts[n - 1] });
    Curve3::poly(out, closed)
}

/// Hermite cúbica (G1) ou quíntica (G2) entre dois pontos.
pub fn hermite(p0: V3, d0: V3, a0: V3, p1: V3, d1: V3, a1: V3, quintic: bool) -> Vec<V3> {
    let n = 64;
    (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let (t2, t3) = (t * t, t * t * t);
            if quintic {
                let (t4, t5) = (t3 * t, t3 * t2);
                let h0 = 1. - 10. * t3 + 15. * t4 - 6. * t5;
                let h1 = t - 6. * t3 + 8. * t4 - 3. * t5;
                let h2 = 0.5 * t2 - 1.5 * t3 + 1.5 * t4 - 0.5 * t5;
                let h3 = 0.5 * t3 - t4 + 0.5 * t5;
                let h4 = -4. * t3 + 7. * t4 - 3. * t5;
                let h5 = 10. * t3 - 15. * t4 + 6. * t5;
                [0, 1, 2].map(|k| h0 * p0[k] + h1 * d0[k] + h2 * a0[k] + h3 * a1[k] + h4 * d1[k] + h5 * p1[k])
            } else {
                let h00 = 2. * t3 - 3. * t2 + 1.;
                let h10 = t3 - 2. * t2 + t;
                let h01 = -2. * t3 + 3. * t2;
                let h11 = t3 - t2;
                [0, 1, 2].map(|k| h00 * p0[k] + h10 * d0[k] + h01 * p1[k] + h11 * d1[k])
            }
        })
        .collect()
}

/// Hermite de grau 7: posição, 1ª, 2ª e 3ª derivadas nas duas pontas (continuidade G3).
pub fn hermite7(p0: V3, d0: V3, a0: V3, j0: V3, p1: V3, d1: V3, a1: V3, j1: V3) -> Vec<V3> {
    // c0..c3 saem da ponta inicial; c4..c7 resolvem as quatro condições da ponta final
    let m = [[1., 1., 1., 1.], [4., 5., 6., 7.], [12., 20., 30., 42.], [24., 60., 120., 210.]];
    let mut coef = [[0.; 8]; 3];
    for k in 0..3 {
        let c = [p0[k], d0[k], a0[k] / 2., j0[k] / 6.];
        let rhs = [
            p1[k] - (c[0] + c[1] + c[2] + c[3]),
            d1[k] - (c[1] + 2. * c[2] + 3. * c[3]),
            a1[k] - (2. * c[2] + 6. * c[3]),
            j1[k] - 6. * c[3],
        ];
        let x = solve4(m, rhs);
        coef[k] = [c[0], c[1], c[2], c[3], x[0], x[1], x[2], x[3]];
    }
    let n = 64;
    (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            [0, 1, 2].map(|k| coef[k].iter().rev().fold(0., |acc, c| acc * t + c))
        })
        .collect()
}

fn solve4(mut a: [[f64; 4]; 4], mut b: [f64; 4]) -> [f64; 4] {
    for i in 0..4 {
        let p = (i..4).max_by(|x, y| a[*x][i].abs().total_cmp(&a[*y][i].abs())).unwrap();
        a.swap(i, p);
        b.swap(i, p);
        for r in i + 1..4 {
            let f = a[r][i] / a[i][i];
            for c in i..4 {
                a[r][c] -= f * a[i][c];
            }
            b[r] -= f * b[i];
        }
    }
    let mut x = [0.; 4];
    for i in (0..4).rev() {
        x[i] = (b[i] - (i + 1..4).map(|c| a[i][c] * x[c]).sum::<f64>()) / a[i][i];
    }
    x
}

/// Hélice: origem e eixo, raio inicial, passo, altura, conicidade (graus), sentido.
pub fn helix(origin: V3, axis: V3, r: f64, pitch: f64, height: f64, taper_deg: f64, left: bool) -> Result<Curve3, String> {
    if r <= 0. || pitch <= 0. || height <= 0. {
        return Err("raio, passo e altura precisam ser positivos".into());
    }
    let ax = unit(axis).ok_or("eixo nulo")?;
    let x = any_perp(ax);
    let y = cross(ax, x);
    let turns = height / pitch;
    let n = ((turns * 64.).ceil() as usize).clamp(16, 20000);
    let tan = taper_deg.to_radians().tan();
    let sgn = if left { -1. } else { 1. };
    let pts = (0..=n)
        .map(|i| {
            let s = i as f64 / n as f64;
            let h = s * height;
            let a = sgn * s * turns * TAU;
            let rr = r + h * tan;
            add(origin, add(mul(ax, h), add(mul(x, rr * a.cos()), mul(y, rr * a.sin()))))
        })
        .collect();
    Curve3::poly(pts, false)
}

pub fn spiral(ctr: V3, f: &Frame, r0: f64, r1: f64, turns: f64) -> Result<Curve3, String> {
    if turns <= 0. {
        return Err("número de voltas precisa ser positivo".into());
    }
    let n = ((turns * 96.).ceil() as usize).clamp(16, 20000);
    let pts = (0..=n)
        .map(|i| {
            let s = i as f64 / n as f64;
            let a = s * turns * TAU;
            let r = r0 + (r1 - r0) * s;
            add(ctr, add(mul(f.x, r * a.cos()), mul(f.y(), r * a.sin())))
        })
        .collect();
    Curve3::poly(pts, false)
}

/// Cônica no plano: elipse (a, b), parábola (y = x²/4a, |x| ≤ extent), hipérbole (ramo direito, |y| ≤ extent).
pub fn conic(ctr: V3, f: &Frame, kind: fenkra_core::ConicKind, a: f64, b: f64, extent: f64) -> Result<Curve3, String> {
    use fenkra_core::ConicKind::*;
    if a <= 0. || (b <= 0. && kind != Parabola) {
        return Err("parâmetros a e b precisam ser positivos".into());
    }
    let p2 = |u: f64, v: f64| add(ctr, add(mul(f.x, u), mul(f.y(), v)));
    let n = 128;
    let (pts, closed) = match kind {
        Elipse => ((0..=n).map(|i| {
            let t = i as f64 / n as f64 * TAU;
            p2(a * t.cos(), b * t.sin())
        }).collect(), true),
        Parabola => {
            if extent <= 0. {
                return Err("extensão precisa ser positiva".into());
            }
            ((0..=n).map(|i| {
                let x = -extent + 2. * extent * i as f64 / n as f64;
                p2(x, x * x / (4. * a))
            }).collect(), false)
        }
        Hiperbole => {
            if extent <= 0. {
                return Err("extensão precisa ser positiva".into());
            }
            let tm = (extent / b).asinh();
            ((0..=n).map(|i| {
                let t = -tm + 2. * tm * i as f64 / n as f64;
                p2(a * t.cosh(), b * t.sinh())
            }).collect(), false)
        }
    };
    Curve3::poly(pts, closed)
}

/// Concordância entre duas linhas (só o arco): (curva, ponto de tangência em a, em b).
pub fn corner(a: &Curve3, b: &Curve3, r: f64) -> Result<Curve3, String> {
    if r <= 0. {
        return Err("raio precisa ser positivo".into());
    }
    if !a.is_straight() || !b.is_straight() {
        return corner_curves(a, b, r);
    }
    let (pa, da) = (a.start(), unit(sub(a.end(), a.start())).unwrap());
    let (pb, db) = (b.start(), unit(sub(b.end(), b.start())).unwrap());
    // ponto mais próximo entre as retas (vértice)
    let w = sub(pa, pb);
    let (bb, d, e) = (dot(da, db), dot(da, w), dot(db, w));
    let den = 1. - bb * bb;
    if den.abs() < 1e-9 {
        return Err("as linhas são paralelas".into());
    }
    let s = (bb * e - d) / den;
    let t = (e - bb * d) / den;
    let (qa, qb) = (add(pa, mul(da, s)), add(pb, mul(db, t)));
    if dist(qa, qb) > 1e-4 * (a.length() + b.length()) + 1e-6 {
        return Err("as linhas não são coplanares".into());
    }
    let v = lerp(qa, qb, 0.5);
    // ramos: do vértice para o lado onde fica a maior parte de cada linha
    let side = |c: &Curve3, dd: V3| {
        let far = if dist(c.start(), v) > dist(c.end(), v) { c.start() } else { c.end() };
        if dot(sub(far, v), dd) >= 0. { dd } else { mul(dd, -1.) }
    };
    let (u1, u2) = (side(a, da), side(b, db));
    let th = dot(u1, u2).clamp(-1., 1.).acos();
    if th < 1e-6 || (PI - th) < 1e-6 {
        return Err("as linhas são colineares".into());
    }
    let tl = r / (th / 2.).tan();
    let bis = unit(add(u1, u2)).ok_or("bissetriz indefinida")?;
    let ctr = add(v, mul(bis, r / (th / 2.).sin()));
    let t1 = add(v, mul(u1, tl));
    let n = unit(cross(u1, u2)).ok_or("linhas colineares")?;
    let x = unit(sub(t1, ctr)).unwrap();
    let y = cross(n, x);
    let sweep = PI - th;
    // o arco vai de t1 até t2 girando em torno de n; verifica o sentido
    let t2 = add(v, mul(u2, tl));
    let end_dir = unit(sub(t2, ctr)).unwrap();
    let sgn = if dot(cross(x, end_dir), n) >= 0. { 1. } else { -1. };
    let pts = arc_pts(ctr, x, mul(y, sgn), r, 0., sweep);
    let mut c = Curve3::poly(pts, false)?;
    c.circle = Some((ctr, mul(n, sgn), r));
    Ok(c)
}

/// Canto entre duas curvas coplanares quaisquer (linha, spline, arco): o centro está onde as
/// curvas deslocadas de `r` se cruzam; fica a solução mais perto do encontro das curvas.
fn corner_curves(a: &Curve3, b: &Curve3, r: f64) -> Result<Curve3, String> {
    let mut all = a.pts.clone();
    all.extend(b.pts.iter().copied());
    let (f, dev) = fit_plane(&all).map_err(|_| "as curvas estão alinhadas numa reta: não há canto".to_string())?;
    let size = a.length() + b.length();
    if dev > 1e-4 * size + 1e-6 {
        return Err("as curvas não são coplanares".into());
    }
    let y = f.y();
    let uv = |p: V3| {
        let d = sub(p, f.origin);
        [dot(d, f.x), dot(d, y)]
    };
    let to3 = |q: [f64; 2]| add(f.origin, add(mul(f.x, q[0]), mul(y, q[1])));
    let pa: Vec<[f64; 2]> = a.pts.iter().map(|p| uv(*p)).collect();
    let pb: Vec<[f64; 2]> = b.pts.iter().map(|p| uv(*p)).collect();
    // segmentos deslocados de um lado (s = ±1)
    let offs = |p: &[[f64; 2]], s: f64| -> Vec<([f64; 2], [f64; 2])> {
        p.windows(2)
            .filter_map(|w| {
                let d = [w[1][0] - w[0][0], w[1][1] - w[0][1]];
                let l = (d[0] * d[0] + d[1] * d[1]).sqrt();
                (l > 1e-12).then(|| {
                    let n = [-d[1] / l * s * r, d[0] / l * s * r];
                    ([w[0][0] + n[0], w[0][1] + n[1]], [w[1][0] + n[0], w[1][1] + n[1]])
                })
            })
            .collect()
    };
    let seg_x = |p: ([f64; 2], [f64; 2]), q: ([f64; 2], [f64; 2])| -> Option<[f64; 2]> {
        let (r1, r2) = ([p.1[0] - p.0[0], p.1[1] - p.0[1]], [q.1[0] - q.0[0], q.1[1] - q.0[1]]);
        let den = r1[0] * r2[1] - r1[1] * r2[0];
        if den.abs() < 1e-14 {
            return None;
        }
        let w = [q.0[0] - p.0[0], q.0[1] - p.0[1]];
        let t = (w[0] * r2[1] - w[1] * r2[0]) / den;
        let u = (w[0] * r1[1] - w[1] * r1[0]) / den;
        (-1e-9..=1. + 1e-9).contains(&t).then_some(())?;
        (-1e-9..=1. + 1e-9).contains(&u).then(|| [p.0[0] + r1[0] * t, p.0[1] + r1[1] * t])
    };
    let foot = |p: &[[f64; 2]], c: [f64; 2]| -> ([f64; 2], f64) {
        let mut best = (p[0], f64::MAX);
        for w in p.windows(2) {
            let d = [w[1][0] - w[0][0], w[1][1] - w[0][1]];
            let l2 = (d[0] * d[0] + d[1] * d[1]).max(1e-24);
            let t = (((c[0] - w[0][0]) * d[0] + (c[1] - w[0][1]) * d[1]) / l2).clamp(0., 1.);
            let q = [w[0][0] + d[0] * t, w[0][1] + d[1] * t];
            let dd = ((q[0] - c[0]).powi(2) + (q[1] - c[1]).powi(2)).sqrt();
            if dd < best.1 {
                best = (q, dd);
            }
        }
        best
    };
    // encontro das curvas (ou o par de pontos mais próximo)
    let meet = {
        let mut best = (pa[0], f64::MAX);
        for q in &pb {
            let (fq, d) = foot(&pa, *q);
            if d < best.1 {
                best = (fq, d);
            }
        }
        for q in &pa {
            let (fq, d) = foot(&pb, *q);
            if d < best.1 {
                best = (fq, d);
            }
        }
        best.0
    };
    let mut cand: Option<([f64; 2], f64)> = None;
    for sa in [1., -1.] {
        for sb in [1., -1.] {
            let (oa, ob) = (offs(&pa, sa), offs(&pb, sb));
            for p in &oa {
                for q in &ob {
                    if let Some(c) = seg_x(*p, *q) {
                        // o centro precisa ficar a r das duas curvas (não só dos deslocamentos)
                        let (da, db) = (foot(&pa, c).1, foot(&pb, c).1);
                        if (da - r).abs() > 1e-3 * r + 1e-6 || (db - r).abs() > 1e-3 * r + 1e-6 {
                            continue;
                        }
                        let dm = ((c[0] - meet[0]).powi(2) + (c[1] - meet[1]).powi(2)).sqrt();
                        if cand.is_none_or(|(_, b)| dm < b) {
                            cand = Some((c, dm));
                        }
                    }
                }
            }
        }
    }
    let (c2, _) = cand.ok_or("não cabe um canto com esse raio entre as curvas")?;
    let (ta, tb) = (foot(&pa, c2).0, foot(&pb, c2).0);
    let ctr = to3(c2);
    let x = unit(sub(to3(ta), ctr)).ok_or("canto degenerado")?;
    let e = unit(sub(to3(tb), ctr)).ok_or("canto degenerado")?;
    let sweep = dot(x, e).clamp(-1., 1.).acos();
    let sgn = if dot(cross(x, e), f.normal) >= 0. { 1. } else { -1. };
    let yy = mul(cross(f.normal, x), sgn);
    let mut out = Curve3::poly(arc_pts(ctr, x, yy, r, 0., sweep), false)?;
    out.circle = Some((ctr, mul(f.normal, sgn), r));
    Ok(out)
}

/// Pontos onde a tangente da curva planar é paralela a `dir`: extremos da projeção na perpendicular.
pub fn tangent_points(c: &Curve3, dir: V3) -> Result<[V3; 2], String> {
    let f = c.plane()?;
    let d = unit(sub(dir, mul(f.normal, dot(dir, f.normal)))).ok_or("a direção é perpendicular ao plano da curva")?;
    let perp = cross(f.normal, d);
    let mut lo = (f64::MAX, c.start());
    let mut hi = (f64::MIN, c.start());
    for p in &c.pts {
        let k = dot(*p, perp);
        if k > hi.0 {
            hi = (k, *p);
        }
        if k < lo.0 {
            lo = (k, *p);
        }
    }
    Ok([hi.1, lo.1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plano_medio_e_circulo() {
        let pts = [[0., 0., 1.], [10., 0., 1.], [10., 10., 1.], [0., 10., 1.]];
        let (f, dev) = fit_plane(&pts).unwrap();
        assert!(dev < 1e-9 && f.normal[2].abs() > 0.999);
        let c = circle3_curve([10., 0., 0.], [0., 10., 0.], [-10., 0., 0.], true).unwrap();
        let (ctr, _, r) = fit_circle(&c).unwrap();
        assert!(norm(ctr) < 1e-9 && (r - 10.).abs() < 1e-9);
        assert!((c.length() - TAU * 10.).abs() < 0.1);
    }

    #[test]
    fn canto_entre_linhas() {
        let a = Curve3::line([0., 0., 0.], [20., 0., 0.]).unwrap();
        let b = Curve3::line([0., 0., 0.], [0., 20., 0.]).unwrap();
        let c = corner(&a, &b, 5.).unwrap();
        let (ctr, _, r) = c.circle.unwrap();
        assert!((r - 5.).abs() < 1e-9 && dist(ctr, [5., 5., 0.]) < 1e-9);
        assert!(dist(c.start(), [5., 0., 0.]) < 1e-6 && dist(c.end(), [0., 5., 0.]) < 1e-6);
    }

    #[test]
    fn spline_passa_pelos_pontos_e_helice() {
        let p = [[0., 0., 0.], [10., 5., 0.], [20., 0., 3.], [30., 8., 0.]];
        let s = spline(&p, false).unwrap();
        for q in p {
            assert!(s.pts.iter().any(|x| dist(*x, q) < 1e-9));
        }
        let h = helix([0.; 3], [0., 0., 1.], 5., 2., 10., 0., false).unwrap();
        assert!((h.end()[2] - 10.).abs() < 1e-9 && (dist(h.end(), [0., 0., 10.]) - 5.).abs() < 1e-6);
    }
}

/// Aplica a transformação a um ponto.
pub fn xform_point(x: &fenkra_kernel::Xform, p: V3) -> V3 {
    use fenkra_kernel::Xform as X;
    match *x {
        X::Translate(d) => add(p, d),
        X::Rotate(o, k, a) => add(o, rotate(sub(p, o), unit(k).unwrap_or([0., 0., 1.]), a.to_radians())),
        X::Mirror(o, n) => {
            let n = unit(n).unwrap_or([0., 0., 1.]);
            sub(p, mul(n, 2. * dot(sub(p, o), n)))
        }
        X::Scale(o, k) => add(o, mul(sub(p, o), k)),
    }
}

/// Aplica a transformação a uma direção (sem translação nem escala).
pub fn xform_dir(x: &fenkra_kernel::Xform, v: V3) -> V3 {
    use fenkra_kernel::Xform as X;
    match *x {
        X::Translate(_) | X::Scale(..) => v,
        X::Rotate(_, k, a) => rotate(v, unit(k).unwrap_or([0., 0., 1.]), a.to_radians()),
        X::Mirror(_, n) => {
            let n = unit(n).unwrap_or([0., 0., 1.]);
            sub(v, mul(n, 2. * dot(v, n)))
        }
    }
}

/// Reamostra a curva em `n` pontos igualmente espaçados pelo comprimento.
pub fn resample(c: &Curve3, n: usize) -> Vec<V3> {
    (0..n).map(|i| c.at_ratio(i as f64 / (n - 1).max(1) as f64).0).collect()
}
