// SPDX-License-Identifier: LGPL-2.1-or-later
//! B549: spline editada pelos pontos de controle.
//!
//! A spline do sketch guarda os pontos de passagem e, quando há, a tangente própria de cada um (forma
//! de Hermite cúbica, um trecho por par de pontos, parâmetro uniforme). Uma B-spline cúbica com nós
//! uniformes e pontas presas (nós repetidos quatro vezes nas pontas) é exatamente uma dessas curvas
//! quando as tangentes internas fazem a curva contínua em curvatura (C2). Por isso o modo de pontos de
//! controle não cria um tipo novo de curva:
//!
//! 1. ao entrar, as tangentes internas viram as da spline C2 (`tangentes_c2`), mantendo as das pontas;
//! 2. o polígono de controle sai das passagens e tangentes (`controle_de_hermite`), com n + 2 vértices
//!    para n pontos de passagem;
//! 3. arrastar um vértice muda o polígono, e a volta (`hermite_de_controle`) dá as passagens e
//!    tangentes novas, gravadas como tangentes próprias.
//!
//! Assim o perfil, o solver, aparar, estender e as cotas continuam vendo a mesma spline de sempre,
//! e o kernel recebe a curva que a tela mostra. Cada vértice mexe só nos trechos vizinhos (até quatro),
//! que é o controle local esperado de uma B-spline.
//!
//! Relação entre o polígono `d` e a forma de Bézier de cada trecho `i` (pontos b0, b1, b2, b3), tirada
//! do florescimento (blossom) da B-spline com nós 0,0,0,0,1,…,L−1,L,L,L,L:
//! - trecho do meio: b1 = (2·d[i+1] + d[i+2]) / 3 e b2 = (d[i+1] + 2·d[i+2]) / 3;
//! - primeiro trecho: b1 = d[1] e b2 = (d[1] + d[2]) / 2; último: b1 = (d[n−1] + d[n]) / 2 e b2 = d[n];
//! - com um trecho só, o polígono é o próprio Bézier.
//!
//! O ponto de passagem interno é o meio entre b2 do trecho anterior e b1 do seguinte, e a tangente é
//! 3·(b1 − passagem).

use crate::sketch::{spline_tangents, Curve, SketchData, SolveReport};
use crate::Id;
use std::collections::HashMap;

type P2 = [f64; 2];

fn add(a: P2, b: P2) -> P2 {
    [a[0] + b[0], a[1] + b[1]]
}
fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}
fn mul(a: P2, k: f64) -> P2 {
    [a[0] * k, a[1] * k]
}

/// Tangentes da spline cúbica C2 de parâmetro uniforme pelos pontos `p`, com as tangentes das pontas
/// dadas. Cada ponto interno cumpre m[i−1] + 4·m[i] + m[i+1] = 3·(p[i+1] − p[i−1]), a condição de
/// segunda derivada igual dos dois lados; o sistema tridiagonal sai pelo algoritmo de Thomas.
pub fn tangentes_c2(p: &[P2], m0: P2, mn: P2) -> Vec<P2> {
    let n = p.len();
    match n {
        0 => return vec![],
        1 => return vec![m0],
        2 => return vec![m0, mn],
        _ => {}
    }
    // incógnitas m[1..n−1]; diagonal 4, vizinhas 1
    let k = n - 2;
    let mut rhs: Vec<P2> = (1..n - 1).map(|i| mul(sub(p[i + 1], p[i - 1]), 3.)).collect();
    rhs[0] = sub(rhs[0], m0);
    rhs[k - 1] = sub(rhs[k - 1], mn);
    let mut c = vec![0.; k];
    let mut b = 4.;
    c[0] = 1. / b;
    rhs[0] = mul(rhs[0], 1. / b);
    for i in 1..k {
        b = 4. - c[i - 1];
        c[i] = 1. / b;
        rhs[i] = mul(sub(rhs[i], rhs[i - 1]), 1. / b);
    }
    for i in (0..k - 1).rev() {
        rhs[i] = sub(rhs[i], mul(rhs[i + 1], c[i]));
    }
    let mut m = vec![m0];
    m.extend(rhs);
    m.push(mn);
    m
}

/// Polígono de controle (n + 2 vértices) da spline C2 com passagens `p` e tangentes `m`.
pub fn controle_de_hermite(p: &[P2], m: &[P2]) -> Vec<P2> {
    let n = p.len();
    if n < 2 || m.len() != n {
        return p.to_vec();
    }
    let b1 = |i: usize| add(p[i], mul(m[i], 1. / 3.));
    let b2 = |i: usize| sub(p[i + 1], mul(m[i + 1], 1. / 3.));
    let mut d = vec![p[0], b1(0)];
    // d[j] = 2·b2(j−2) − b1(j−2): vale do primeiro trecho até o penúltimo
    for j in 2..n {
        d.push(sub(mul(b2(j - 2), 2.), b1(j - 2)));
    }
    d.push(b2(n - 2));
    d.push(p[n - 1]);
    d
}

/// Passagens e tangentes da spline cujo polígono de controle é `d` (pelo menos quatro vértices).
pub fn hermite_de_controle(d: &[P2]) -> (Vec<P2>, Vec<P2>) {
    if d.len() < 4 {
        return (d.to_vec(), vec![[0., 0.]; d.len()]);
    }
    let n = d.len() - 2;
    let trechos = n - 1;
    // pontos internos de Bézier de cada trecho
    let bez: Vec<(P2, P2)> = (0..trechos)
        .map(|i| {
            if trechos == 1 {
                (d[1], d[2])
            } else if i == 0 {
                (d[1], mul(add(d[1], d[2]), 0.5))
            } else if i == trechos - 1 {
                (mul(add(d[n - 1], d[n]), 0.5), d[n])
            } else {
                (mul(add(mul(d[i + 1], 2.), d[i + 2]), 1. / 3.), mul(add(d[i + 1], mul(d[i + 2], 2.)), 1. / 3.))
            }
        })
        .collect();
    let mut p = vec![d[0]];
    for i in 1..n - 1 {
        p.push(mul(add(bez[i - 1].1, bez[i].0), 0.5));
    }
    p.push(d[n + 1]);
    let mut m: Vec<P2> = (0..n - 1).map(|i| mul(sub(bez[i].0, p[i]), 3.)).collect();
    m.push(mul(sub(p[n - 1], bez[trechos - 1].1), 3.));
    (p, m)
}

impl SketchData {
    /// Passagens e tangentes em vigor da spline (próprias ou automáticas).
    fn spline_hermite(&self, curve: Id) -> Option<(Vec<Id>, Vec<P2>, Vec<P2>)> {
        let Some(Curve::Spline { pts }) = self.curve(curve).map(|c| c.geom.clone()) else { return None };
        if pts.len() < 2 {
            return None;
        }
        let p: Vec<P2> = pts.iter().map(|q| self.pos(*q)).collect();
        let m = spline_tangents(&p, &self.own_tangents(curve));
        Some((pts, p, m))
    }

    /// Deixa a spline contínua em curvatura (C2), mantendo as passagens e as tangentes das pontas: é a
    /// forma que o polígono de controle descreve sem perda. As tangentes viram próprias.
    pub fn spline_c2(&mut self, curve: Id) -> bool {
        let Some((pts, p, m)) = self.spline_hermite(curve) else { return false };
        let novas = tangentes_c2(&p, m[0], m[m.len() - 1]);
        for (id, t) in pts.iter().zip(novas) {
            self.set_spline_tangent(curve, *id, Some(t));
        }
        true
    }

    /// Polígono de controle da spline, na ordem (n + 2 vértices para n pontos de passagem). Com as
    /// tangentes internas fora da forma C2, é o polígono da spline C2 de mesmas passagens e pontas.
    pub fn spline_controle(&self, curve: Id) -> Vec<P2> {
        let Some((_, p, m)) = self.spline_hermite(curve) else { return vec![] };
        controle_de_hermite(&p, &tangentes_c2(&p, m[0], m[m.len() - 1]))
    }

    /// Leva o vértice `k` do polígono de controle a `alvo`. As passagens e tangentes saem do polígono
    /// novo e o solver confere as restrições com as passagens presas no lugar novo; se isso conflitar,
    /// deixa-as escorregar. Sem solução, nada muda e devolve `None`.
    pub fn mover_controle(&mut self, curve: Id, k: usize, alvo: P2, vars: &HashMap<String, f64>) -> Option<SolveReport> {
        let mut d = self.spline_controle(curve);
        if k >= d.len() {
            return None;
        }
        let (pts, ..) = self.spline_hermite(curve)?;
        d[k] = alvo;
        let (p, m) = hermite_de_controle(&d);
        let mut g = self.clone();
        let mut movidos = vec![];
        for (i, id) in pts.iter().enumerate() {
            if let Some(q) = g.point_mut(*id) {
                if !q.fixed {
                    (q.x, q.y) = (p[i][0], p[i][1]);
                    movidos.push(*id);
                }
            }
            g.set_spline_tangent(curve, *id, Some(m[i]));
        }
        for pins in [movidos.as_slice(), &[]] {
            let mut t = g.clone();
            let rep = t.solve(vars, pins);
            if rep.converged {
                *self = t;
                return Some(rep);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sketch::spline_eval;

    fn perto(a: P2, b: P2, tol: f64) -> bool {
        (a[0] - b[0]).hypot(a[1] - b[1]) < tol
    }

    /// B-spline cúbica de pontas presas e nós uniformes avaliada por de Boor: a referência
    /// independente da conversão por Bézier.
    fn de_boor(d: &[P2], u: f64) -> P2 {
        let n = d.len() - 1;
        let l = (d.len() - 3) as f64;
        let mut nos = vec![0.; 4];
        nos.extend((1..d.len() - 3).map(|i| i as f64));
        nos.extend([l; 4]);
        let u = u.clamp(0., l);
        let mut k = 3;
        while k < n && nos[k + 1] <= u {
            k += 1;
        }
        let mut v: Vec<P2> = (0..4).map(|j| d[j + k - 3]).collect();
        for r in 1..4 {
            for j in (r..4).rev() {
                let i = j + k - 3;
                let den = nos[i + 4 - r] - nos[i];
                let a = if den.abs() < 1e-12 { 0. } else { (u - nos[i]) / den };
                v[j] = add(mul(v[j - 1], 1. - a), mul(v[j], a));
            }
        }
        v[3]
    }

    fn poligono() -> Vec<P2> {
        vec![[0., 0.], [10., 25.], [30., 30.], [45., 5.], [60., 20.], [80., 0.]]
    }

    #[test]
    fn controle_ida_e_volta_sem_perda() {
        for d in [poligono(), poligono()[..4].to_vec(), poligono()[..5].to_vec()] {
            let (p, m) = hermite_de_controle(&d);
            assert_eq!(p.len(), d.len() - 2);
            let volta = controle_de_hermite(&p, &m);
            assert_eq!(volta.len(), d.len());
            for (a, b) in d.iter().zip(&volta) {
                assert!(perto(*a, *b, 1e-9), "{a:?} != {b:?}");
            }
        }
    }

    #[test]
    fn hermite_do_poligono_e_a_mesma_curva_da_bspline() {
        let d = poligono();
        let (p, m) = hermite_de_controle(&d);
        let own: Vec<Option<P2>> = m.iter().map(|x| Some(*x)).collect();
        let por = 10;
        let s = spline_eval(&p, &own, por);
        for (k, q) in s.iter().enumerate() {
            let u = k as f64 / por as f64;
            assert!(perto(*q, de_boor(&d, u), 1e-9), "u = {u}: {q:?} x {:?}", de_boor(&d, u));
        }
        // a curva passa pela primeira e pela última ponta do polígono, e não pelos vértices internos
        assert!(perto(p[0], d[0], 1e-12) && perto(p[p.len() - 1], d[d.len() - 1], 1e-12));
        assert!(!s.iter().any(|q| perto(*q, d[2], 0.5)));
    }

    #[test]
    fn tangentes_c2_tem_curvatura_continua() {
        let p = [[0., 0.], [10., 8.], [25., 2.], [33., 15.], [50., 10.]];
        let m = tangentes_c2(&p, [10., 10.], [10., -5.]);
        // segunda derivada de Hermite no fim do trecho i e no começo do i+1
        for i in 0..p.len() - 2 {
            let fim = [0, 1].map(|j| 6. * p[i][j] + 2. * m[i][j] - 6. * p[i + 1][j] + 4. * m[i + 1][j]);
            let ini = [0, 1].map(|j| -6. * p[i + 1][j] - 4. * m[i + 1][j] + 6. * p[i + 2][j] - 2. * m[i + 2][j]);
            assert!(perto(fim, ini, 1e-9), "junta {}: {fim:?} x {ini:?}", i + 1);
        }
        assert_eq!(m[0], [10., 10.]);
        assert_eq!(m[4], [10., -5.]);
    }

    #[test]
    fn vertice_mexe_so_nos_trechos_vizinhos() {
        let d = poligono();
        let mut e = d.clone();
        e[5] = [90., 30.];
        // o vértice 5 (último) só alcança o último trecho; o vértice 2 só os três primeiros
        let antes = |d: &[P2], u: f64| de_boor(d, u);
        for k in 0..=20 {
            let u = k as f64 * 0.1;
            assert!(perto(antes(&d, u), antes(&e, u), 1e-9), "u = {u} mudou");
        }
        let (p0, _) = hermite_de_controle(&d);
        let (p1, _) = hermite_de_controle(&e);
        assert!(perto(p0[1], p1[1], 1e-9) && perto(p0[2], p1[2], 1e-9));
        assert!(!perto(p0[3], p1[3], 1e-6));
    }

    fn sk_spline(pts: &[P2]) -> (SketchData, Id, Vec<Id>) {
        let mut sk = SketchData::default();
        let mut n = 0u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let ids: Vec<Id> = pts.iter().map(|q| sk.add_point(&mut al, q[0], q[1])).collect();
        let c = sk.add_curve(&mut al, Curve::Spline { pts: ids.clone() }, false);
        (sk, c, ids)
    }

    #[test]
    fn entrar_no_modo_mantem_passagens_e_pontas() {
        let pts = [[0., 0.], [20., 15.], [40., 5.], [60., 20.]];
        let (mut sk, c, ids) = sk_spline(&pts);
        let antes = spline_tangents(&pts, &sk.own_tangents(c));
        assert!(sk.spline_c2(c));
        let d = sk.spline_controle(c);
        assert_eq!(d.len(), pts.len() + 2);
        for (id, q) in ids.iter().zip(pts) {
            assert_eq!(sk.pos(*id), q, "a passagem não se move ao entrar no modo");
        }
        let own = sk.own_tangents(c);
        assert_eq!(own[0], Some(antes[0]));
        assert_eq!(own[3], Some(antes[3]));
        // o polígono mostrado bate com a curva gravada
        let (p, m) = hermite_de_controle(&d);
        for i in 0..4 {
            assert!(perto(p[i], pts[i], 1e-9) && perto(m[i], own[i].unwrap(), 1e-9));
        }
    }

    #[test]
    fn arrastar_vertice_de_controle_muda_a_curva_e_respeita_ponto_fixo() {
        let pts = [[0., 0.], [20., 15.], [40., 5.], [60., 20.], [80., 0.]];
        let (mut sk, c, ids) = sk_spline(&pts);
        sk.point_mut(ids[0]).unwrap().fixed = true;
        sk.spline_c2(c);
        let vars = HashMap::new();
        let alvo = [42., 40.];
        let rep = sk.mover_controle(c, 3, alvo, &vars).expect("solução");
        assert!(rep.converged);
        let d = sk.spline_controle(c);
        assert!(perto(d[3], alvo, 1e-6), "o vértice foi aonde o mouse pediu: {:?}", d[3]);
        assert_eq!(sk.pos(ids[0]), [0., 0.], "ponto fixo continua no lugar");
        assert!(sk.pos(ids[2])[1] > 5. + 1., "a passagem perto do vértice subiu: {:?}", sk.pos(ids[2]));
        assert!(perto(sk.pos(ids[4]), [80., 0.], 1e-9), "longe do vértice nada muda");
        assert!(sk.mover_controle(c, 99, alvo, &vars).is_none());
    }
}
