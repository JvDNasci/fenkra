// SPDX-License-Identifier: LGPL-2.1-or-later
//! B559: definição única da spline por pontos.
//!
//! A spline que passa pelos pontos é a interpolação cúbica que o kernel faz (`GeomAPI_Interpolate`):
//! parâmetro pelo comprimento das cordas, curva C2 nos pontos internos e, nas pontas, a derivada do
//! polinômio de Lagrange pelos quatro primeiros (ou últimos) pontos. Com três pontos é a parábola por
//! eles; com dois, a reta. Fechada (periódica), é a cúbica C2 periódica com a corda de volta.
//!
//! O desenho do sketch, a prévia, o inserir ponto e o wireframe usam esta mesma curva, e o kernel recebe
//! os mesmos pontos para interpolar: o que se vê é o que vira sólido. (Aparar e Estender seguem a
//! Hermite do solver, ver ADR 0108.) O teste
//! `spline_unica_igual_ao_kernel` (fenkra-solidos) confere a réplica contra o kernel.
//!
//! A curva é guardada na forma de Hermite (pontos, derivadas e parâmetros); qualquer Hermite cúbica é
//! uma B-spline cúbica com nós duplos nos pontos (`Hermite::bspline`), que é como uma spline com
//! tangentes fixadas vai ao kernel sem amostrar.

/// Spline cúbica de Hermite por trechos: em cada vão [u_i, u_i+1] o polinômio cúbico que passa por
/// `p[i]` e `p[i+1]` com as derivadas `m[i]` e `m[i+1]` (em relação a `u`).
#[derive(Clone, Debug, PartialEq)]
pub struct Hermite<const D: usize> {
    pub u: Vec<f64>,
    pub p: Vec<[f64; D]>,
    pub m: Vec<[f64; D]>,
}

/// B-spline não racional: grau, polos e nós achatados (com as repetições).
#[derive(Clone, Debug, PartialEq)]
pub struct BSpline<const D: usize> {
    pub grau: usize,
    pub polos: Vec<[f64; D]>,
    pub nos: Vec<f64>,
}

fn sub<const D: usize>(a: [f64; D], b: [f64; D]) -> [f64; D] {
    std::array::from_fn(|k| a[k] - b[k])
}
fn add<const D: usize>(a: [f64; D], b: [f64; D]) -> [f64; D] {
    std::array::from_fn(|k| a[k] + b[k])
}
fn mul<const D: usize>(a: [f64; D], s: f64) -> [f64; D] {
    std::array::from_fn(|k| a[k] * s)
}
fn dist<const D: usize>(a: [f64; D], b: [f64; D]) -> f64 {
    (0..D).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}

/// Parâmetros pelo comprimento acumulado das cordas (como o kernel). Pontos repetidos seguidos
/// avançam um passo mínimo para o parâmetro continuar crescente.
pub fn parametros_corda<const D: usize>(p: &[[f64; D]]) -> Vec<f64> {
    let mut u = vec![0.];
    for w in p.windows(2) {
        let last = *u.last().unwrap();
        u.push(last + dist(w[0], w[1]).max(1e-12));
    }
    u
}

/// Derivada em `u[i0]` do polinômio de Lagrange pelos pontos `idx` (grau = idx.len() − 1).
fn derivada_lagrange<const D: usize>(u: &[f64], p: &[[f64; D]], idx: &[usize], i0: usize) -> [f64; D] {
    let x = u[i0];
    let mut d = [0.; D];
    for &j in idx {
        // L_j'(x) = Σ_{k≠j} 1/(u_j−u_k) · Π_{l≠j,k} (x−u_l)/(u_j−u_l)
        let mut lj = 0.;
        for &k in idx {
            if k == j {
                continue;
            }
            let mut t = 1. / (u[j] - u[k]);
            for &l in idx {
                if l != j && l != k {
                    t *= (x - u[l]) / (u[j] - u[l]);
                }
            }
            lj += t;
        }
        d = add(d, mul(p[j], lj));
    }
    d
}

/// Resolve o sistema tridiagonal a·x[i−1] + b·x[i] + c·x[i+1] = r (Thomas).
fn thomas<const D: usize>(a: &[f64], b: &[f64], c: &[f64], r: &[[f64; D]]) -> Vec<[f64; D]> {
    let n = b.len();
    let mut cp = vec![0.; n];
    let mut rp = vec![[0.; D]; n];
    cp[0] = c[0] / b[0];
    rp[0] = mul(r[0], 1. / b[0]);
    for i in 1..n {
        let den = b[i] - a[i] * cp[i - 1];
        cp[i] = c[i] / den;
        rp[i] = mul(sub(r[i], mul(rp[i - 1], a[i])), 1. / den);
    }
    let mut x = vec![[0.; D]; n];
    x[n - 1] = rp[n - 1];
    for i in (0..n - 1).rev() {
        x[i] = sub(rp[i], mul(x[i + 1], cp[i]));
    }
    x
}

/// Tridiagonal cíclico (cantos a[0] e c[n−1]) por Sherman–Morrison.
fn thomas_ciclico<const D: usize>(a: &[f64], b: &[f64], c: &[f64], r: &[[f64; D]]) -> Vec<[f64; D]> {
    let n = b.len();
    let gamma = -b[0];
    let mut bb = b.to_vec();
    bb[0] = b[0] - gamma;
    bb[n - 1] = b[n - 1] - c[n - 1] * a[0] / gamma;
    let x = thomas(a, &bb, c, r);
    let mut uu = vec![[0.; 1]; n];
    uu[0] = [gamma];
    uu[n - 1] = [c[n - 1]];
    let z = thomas(a, &bb, c, &uu);
    let fact_num: [f64; D] = add(x[0], mul(x[n - 1], a[0] / gamma));
    let den = 1. + z[0][0] + a[0] * z[n - 1][0] / gamma;
    (0..n).map(|i| sub(x[i], mul(fact_num, z[i][0] / den))).collect()
}

/// Condição nas pontas da cúbica C2 aberta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pontas {
    /// derivada do polinômio de Lagrange pelos quatro pontos da ponta
    Lagrange,
    /// segunda derivada nula
    Natural,
    /// terceira derivada contínua no segundo e no penúltimo ponto ("sem nó")
    SemNo,
}

/// Derivadas da cúbica C2 aberta em todos os pontos (sistema tridiagonal de primeira derivada).
fn derivadas_c2<const D: usize>(u: &[f64], p: &[[f64; D]], pontas: Pontas) -> Vec<[f64; D]> {
    let n = p.len();
    let h: Vec<f64> = u.windows(2).map(|w| w[1] - w[0]).collect();
    let delta: Vec<[f64; D]> = (0..n - 1).map(|k| mul(sub(p[k + 1], p[k]), 1. / h[k])).collect();
    let (mut a, mut b, mut c, mut r) = (vec![0.; n], vec![0.; n], vec![0.; n], vec![[0.; D]; n]);
    for i in 1..n - 1 {
        a[i] = h[i];
        b[i] = 2. * (h[i - 1] + h[i]);
        c[i] = h[i - 1];
        r[i] = mul(add(mul(delta[i - 1], h[i]), mul(delta[i], h[i - 1])), 3.);
    }
    match pontas {
        Pontas::Lagrange => {
            b[0] = 1.;
            r[0] = derivada_lagrange(u, p, &[0, 1, 2, 3], 0);
            b[n - 1] = 1.;
            r[n - 1] = derivada_lagrange(u, p, &[n - 4, n - 3, n - 2, n - 1], n - 1);
        }
        Pontas::Natural => {
            b[0] = 2.;
            c[0] = 1.;
            r[0] = mul(delta[0], 3.);
            a[n - 1] = 1.;
            b[n - 1] = 2.;
            r[n - 1] = mul(delta[n - 2], 3.);
        }
        Pontas::SemNo => {
            let (h0, h1) = (h[0], h[1]);
            b[0] = h1;
            c[0] = h0 + h1;
            r[0] = mul(add(mul(delta[0], (h0 + 2. * (h0 + h1)) * h1), mul(delta[1], h0 * h0)), 1. / (h0 + h1));
            let (hl, hp) = (h[n - 2], h[n - 3]);
            a[n - 1] = hl + hp;
            b[n - 1] = hp;
            r[n - 1] = mul(add(mul(delta[n - 3], hl * hl), mul(delta[n - 2], (2. * (hl + hp) + hl) * hp)), 1. / (hl + hp));
        }
    }
    thomas(&a, &b, &c, &r)
}

/// A condição nas pontas que o kernel usa (conferida em `spline_unica_igual_ao_kernel`).
const PONTAS: Pontas = Pontas::Lagrange;

impl<const D: usize> Hermite<D> {
    /// Spline que passa pelos pontos, como o kernel interpola (ver o módulo). `fechada` faz a curva
    /// periódica pela corda de volta; o último ponto não precisa repetir o primeiro (se repetir, sai).
    pub fn interpolada(pts: &[[f64; D]], fechada: bool) -> Option<Hermite<D>> {
        let mut p = pts.to_vec();
        if fechada && p.len() > 2 && dist(p[0], *p.last().unwrap()) < 1e-12 {
            p.pop();
        }
        let n = p.len();
        if n < 2 {
            return None;
        }
        if fechada && n >= 3 {
            // periódica: n vãos, o último volta ao primeiro ponto
            let mut q = p.clone();
            q.push(p[0]);
            let u = parametros_corda(&q);
            let h: Vec<f64> = u.windows(2).map(|w| w[1] - w[0]).collect();
            let delta: Vec<[f64; D]> = (0..n).map(|k| mul(sub(q[k + 1], q[k]), 1. / h[k])).collect();
            let (mut a, mut b, mut c, mut r) = (vec![0.; n], vec![0.; n], vec![0.; n], vec![[0.; D]; n]);
            for i in 0..n {
                let hp = h[(i + n - 1) % n];
                let hi = h[i];
                a[i] = hi;
                b[i] = 2. * (hp + hi);
                c[i] = hp;
                r[i] = mul(add(mul(delta[(i + n - 1) % n], hi), mul(delta[i], hp)), 3.);
            }
            let mut m = thomas_ciclico(&a, &b, &c, &r);
            m.push(m[0]);
            return Some(Hermite { u, p: q, m });
        }
        let u = parametros_corda(&p);
        if n == 2 {
            let d = mul(sub(p[1], p[0]), 1. / (u[1] - u[0]));
            return Some(Hermite { u, p, m: vec![d, d] });
        }
        if n == 3 {
            // parábola pelos três pontos (o kernel usa grau 2 aqui)
            let idx = [0, 1, 2];
            let m = (0..3).map(|i| derivada_lagrange(&u, &p, &idx, i)).collect();
            return Some(Hermite { u, p, m });
        }
        let m = derivadas_c2(&u, &p, PONTAS);
        Some(Hermite { u, p, m })
    }

    /// A mesma interpolação aberta com outra condição nas pontas (para comparar com o kernel).
    #[doc(hidden)]
    pub fn interpolada_pontas(pts: &[[f64; D]], pontas: Pontas) -> Option<Hermite<D>> {
        if pts.len() < 4 {
            return Self::interpolada(pts, false);
        }
        let u = parametros_corda(pts);
        let m = derivadas_c2(&u, pts, pontas);
        Some(Hermite { u, p: pts.to_vec(), m })
    }

    /// Mesma spline com algumas derivadas trocadas (tangentes fixadas). O resto não muda.
    pub fn com_derivadas(mut self, fixas: &[Option<[f64; D]>]) -> Hermite<D> {
        for (i, f) in fixas.iter().enumerate() {
            if let (Some(d), Some(m)) = (f, self.m.get_mut(i)) {
                *m = *d;
            }
        }
        self
    }

    pub fn dominio(&self) -> (f64, f64) {
        (self.u[0], *self.u.last().unwrap())
    }

    fn vao(&self, t: f64) -> usize {
        let n = self.u.len();
        match self.u.binary_search_by(|x| x.total_cmp(&t)) {
            Ok(i) => i.min(n - 2),
            Err(i) => i.saturating_sub(1).min(n - 2),
        }
    }

    /// Ponto no parâmetro `t` (limitado ao domínio).
    pub fn ponto(&self, t: f64) -> [f64; D] {
        let (a, b) = self.dominio();
        let t = t.clamp(a, b);
        let i = self.vao(t);
        let h = self.u[i + 1] - self.u[i];
        let s = (t - self.u[i]) / h;
        let (s2, s3) = (s * s, s * s * s);
        let (h00, h10, h01, h11) = (2. * s3 - 3. * s2 + 1., s3 - 2. * s2 + s, -2. * s3 + 3. * s2, s3 - s2);
        add(add(mul(self.p[i], h00), mul(self.m[i], h10 * h)), add(mul(self.p[i + 1], h01), mul(self.m[i + 1], h11 * h)))
    }

    /// Derivada em relação ao parâmetro.
    pub fn derivada(&self, t: f64) -> [f64; D] {
        let (a, b) = self.dominio();
        let t = t.clamp(a, b);
        let i = self.vao(t);
        let h = self.u[i + 1] - self.u[i];
        let s = (t - self.u[i]) / h;
        let s2 = s * s;
        let (d00, d10, d01, d11) = (6. * s2 - 6. * s, 3. * s2 - 4. * s + 1., -6. * s2 + 6. * s, 3. * s2 - 2. * s);
        add(add(mul(self.p[i], d00 / h), mul(self.m[i], d10)), add(mul(self.p[i + 1], d01 / h), mul(self.m[i + 1], d11)))
    }

    /// Amostras para desenhar: `por_vao` pontos por vão, começando no primeiro ponto.
    pub fn amostra(&self, por_vao: usize) -> Vec<[f64; D]> {
        let por_vao = por_vao.max(1);
        let mut out = vec![self.p[0]];
        for i in 0..self.u.len() - 1 {
            for k in 1..=por_vao {
                let t = self.u[i] + (self.u[i + 1] - self.u[i]) * k as f64 / por_vao as f64;
                out.push(if k == por_vao { self.p[i + 1] } else { self.ponto(t) });
            }
        }
        out
    }

    /// A mesma curva como B-spline cúbica: em cada vão a forma de Bézier (p, p + m·h/3, q − m'·h/3, q),
    /// com nós duplos nos pontos internos (a curva é C1 pela construção, então o nó duplo é exato).
    pub fn bspline(&self) -> BSpline<D> {
        let n = self.u.len();
        let mut polos = vec![self.p[0]];
        let mut nos = vec![self.u[0]; 4];
        for i in 0..n - 1 {
            let h = self.u[i + 1] - self.u[i];
            polos.push(add(self.p[i], mul(self.m[i], h / 3.)));
            polos.push(sub(self.p[i + 1], mul(self.m[i + 1], h / 3.)));
            if i + 1 < n - 1 {
                nos.push(self.u[i + 1]);
                nos.push(self.u[i + 1]);
            }
        }
        polos.push(self.p[n - 1]);
        nos.extend([self.u[n - 1]; 4]);
        BSpline { grau: 3, polos, nos }
    }
}

impl<const D: usize> BSpline<D> {
    /// Ponto no parâmetro `t` (de Boor).
    pub fn ponto(&self, t: f64) -> [f64; D] {
        let p = self.grau;
        let n = self.polos.len();
        let (a, b) = (self.nos[p], self.nos[n]);
        let t = t.clamp(a, b);
        let mut k = p;
        while k + 1 < n && self.nos[k + 1] <= t {
            k += 1;
        }
        let mut d: Vec<[f64; D]> = (0..=p).map(|j| self.polos[j + k - p]).collect();
        for r in 1..=p {
            for j in (r..=p).rev() {
                let i = j + k - p;
                let den = self.nos[i + p + 1 - r] - self.nos[i];
                let al = if den.abs() < 1e-300 { 0. } else { (t - self.nos[i]) / den };
                d[j] = add(mul(d[j - 1], 1. - al), mul(d[j], al));
            }
        }
        d[p]
    }

    pub fn dominio(&self) -> (f64, f64) {
        (self.nos[self.grau], self.nos[self.polos.len()])
    }

    /// Amostras uniformes no parâmetro (`n` intervalos).
    pub fn amostra(&self, n: usize) -> Vec<[f64; D]> {
        let (a, b) = self.dominio();
        let n = n.max(1);
        (0..=n).map(|i| self.ponto(a + (b - a) * i as f64 / n as f64)).collect()
    }
}

/// Amostras da spline pelos pontos, como o perfil e o desenho a veem (`por_vao` por vão). Dois pontos
/// ou menos voltam como vieram.
pub fn amostra_interpolada(p: &[[f64; 2]], por_vao: usize) -> Vec<[f64; 2]> {
    if p.len() < 3 {
        return p.to_vec();
    }
    match Hermite::interpolada(p, false) {
        Some(h) => h.amostra(por_vao),
        None => p.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passa_pelos_pontos_e_reproduz_a_cubica() {
        // pontos de uma cúbica no parâmetro da corda não ficam na cúbica, mas a curva passa por eles
        let p: Vec<[f64; 2]> = (0..7).map(|i| [i as f64 * 10., ((i as f64) * 0.7).sin() * 20.]).collect();
        let h = Hermite::interpolada(&p, false).unwrap();
        for (i, q) in p.iter().enumerate() {
            assert!(dist(h.ponto(h.u[i]), *q) < 1e-9);
        }
        // a B-spline é a mesma curva
        let b = h.bspline();
        for k in 0..=50 {
            let t = h.u[0] + (h.u[6] - h.u[0]) * k as f64 / 50.;
            assert!(dist(h.ponto(t), b.ponto(t)) < 1e-9, "t {t}");
        }
    }

    #[test]
    fn e_c2_nos_pontos_internos() {
        let p: Vec<[f64; 2]> = vec![[0., 0.], [10., 5.], [25., 2.], [40., 12.], [50., 0.], [70., -4.]];
        let h = Hermite::interpolada(&p, false).unwrap();
        for i in 1..p.len() - 1 {
            let e = 1e-6;
            let d2 = |t: f64| mul(sub(h.derivada(t + e), h.derivada(t - e)), 0.5 / e);
            let (l, r) = (d2(h.u[i] - 1e-4), d2(h.u[i] + 1e-4));
            assert!(dist(l, r) < 1e-3 * (1. + dist(l, [0.; 2])), "ponto {i}: {l:?} {r:?}");
        }
    }

    #[test]
    fn fechada_e_periodica() {
        let p: Vec<[f64; 3]> = (0..8).map(|i| {
            let a = i as f64 / 8. * std::f64::consts::TAU;
            [30. * a.cos(), 20. * a.sin(), 0.]
        }).collect();
        let h = Hermite::interpolada(&p, true).unwrap();
        assert_eq!(h.p.len(), 9);
        assert!(dist(h.m[0], h.m[8]) < 1e-9);
        assert!(dist(h.ponto(h.u[8]), p[0]) < 1e-9);
        // C2 em todos os pontos, inclusive na emenda (segunda derivada pelos dois lados)
        let d2 = |i: usize, s: f64| {
            let (u0, u1) = (h.u[i], h.u[i + 1]);
            let hh = u1 - u0;
            // segunda derivada da Hermite no começo (s = 0) ou no fim (s = 1) do vão i
            let (p0, p1, m0, m1) = (h.p[i], h.p[i + 1], h.m[i], h.m[i + 1]);
            let f = |k: usize| {
                let (a, b, c, d) = if s < 0.5 { (-6., 6., -4., -2.) } else { (6., -6., 2., 4.) };
                (a * p0[k] + b * p1[k]) / (hh * hh) + (c * m0[k] + d * m1[k]) / hh
            };
            [f(0), f(1), f(2)]
        };
        for i in 0..8 {
            let (fim, comeco) = (d2(i, 1.), d2((i + 1) % 8, 0.));
            assert!(dist(fim, comeco) < 1e-9 * (1. + dist(fim, [0.; 3])), "ponto {}: {fim:?} {comeco:?}", i + 1);
        }
    }

    #[test]
    fn tres_pontos_e_parabola() {
        let p = [[0., 0.], [1., 1.], [2., 0.]];
        let h = Hermite::interpolada(&p, false).unwrap();
        // simétrica: a derivada no meio é horizontal
        let d = h.derivada(h.u[1]);
        assert!(d[1].abs() < 1e-9);
    }
}
