// SPDX-License-Identifier: LGPL-2.1-or-later
//! B169: reconhecimento de primitivas numa malha e a descrição do B-rep híbrido.
//!
//! Como funciona:
//! 1. **Planos** por crescimento de regiões: a semente é o maior triângulo livre; entram vizinhos com
//!    normal a menos de 1° e vértices a menos da tolerância do plano ajustado. Uma faixa estreita de
//!    um cilindro também é plana, então a região só vale se tiver quatro triângulos ou mais, ou se a
//!    maior parte da borda dela for um canto vivo, de 35° ou mais (como um chanfro): a malha de um STL comum vira até 20° entre triângulos de uma face curva.
//! 2. **Curvas** (esfera, cilindro, cone, toro) no que sobrou: primeiro a componente inteira; se ela
//!    não couber numa primitiva só (filetes que se encontram, por exemplo), sementes de dois anéis de
//!    vizinhança crescem com a primitiva que melhor se ajusta. Cada ajuste começa por uma conta
//!    direta (eixo pelas normais, círculo e esfera algébricos, raio do toro por busca) e termina em
//!    Levenberg-Marquardt sobre a distância exata de cada vértice.
//! 3. **Encaixe**: cada vértice vai para a superfície da sua região; o vértice da borda entre duas
//!    regiões curvas ou planas vai para a interseção delas (projeções alternadas).
//! 4. Cada região vira uma lista de laços de borda, com a região vizinha de cada trecho. O kernel
//!    (`malha_hibrida`) monta a face analítica, usa a interseção das duas superfícies como aresta
//!    quando ela passa pelos vértices da borda, e deixa triangular o que não teve ajuste.
//!
//! Tudo é medido na malha original: o relatório de desvio compara a malha com o sólido final.

use crate::geo3d::*;
use fenkra_malha::Malha;
use std::collections::{HashMap, VecDeque};

/// Superfície reconhecida.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Primitiva {
    Plano { o: V3, n: V3 },
    Cilindro { o: V3, eixo: V3, r: f64 },
    /// `eixo` vai do ápice para a abertura; `ang` é o semiângulo, em radianos
    Cone { apice: V3, eixo: V3, ang: f64 },
    Esfera { c: V3, r: f64 },
    Toro { c: V3, eixo: V3, maior: f64, menor: f64 },
}

impl Primitiva {
    pub fn nome(&self) -> &'static str {
        match self {
            Primitiva::Plano { .. } => "plano",
            Primitiva::Cilindro { .. } => "cilindro",
            Primitiva::Cone { .. } => "cone",
            Primitiva::Esfera { .. } => "esfera",
            Primitiva::Toro { .. } => "toro",
        }
    }

    /// Código do tipo para o kernel (0 plano, 1 cilindro, 2 cone, 3 esfera, 4 toro).
    pub fn codigo(&self) -> usize {
        match self {
            Primitiva::Plano { .. } => 0,
            Primitiva::Cilindro { .. } => 1,
            Primitiva::Cone { .. } => 2,
            Primitiva::Esfera { .. } => 3,
            Primitiva::Toro { .. } => 4,
        }
    }

    /// Distância com sinal até a superfície e a normal (gradiente) no ponto.
    pub fn distancia(&self, p: V3) -> (f64, V3) {
        match *self {
            Primitiva::Plano { o, n } => (dot(sub(p, o), n), n),
            Primitiva::Cilindro { o, eixo, r } => {
                let v = sub(p, o);
                let rad = sub(v, mul(eixo, dot(v, eixo)));
                let rho = norm(rad);
                (rho - r, if rho > 1e-12 { mul(rad, 1. / rho) } else { any_perp(eixo) })
            }
            Primitiva::Esfera { c, r } => {
                let v = sub(p, c);
                let l = norm(v);
                (l - r, if l > 1e-12 { mul(v, 1. / l) } else { [0., 0., 1.] })
            }
            Primitiva::Cone { apice, eixo, ang } => {
                let v = sub(p, apice);
                let h = dot(v, eixo);
                let rad = sub(v, mul(eixo, h));
                let rho = norm(rad);
                let e = if rho > 1e-12 { mul(rad, 1. / rho) } else { any_perp(eixo) };
                let (s, c) = ang.sin_cos();
                (rho * c - h * s, sub(mul(e, c), mul(eixo, s)))
            }
            Primitiva::Toro { c, eixo, maior, menor } => {
                let v = sub(p, c);
                let h = dot(v, eixo);
                let rad = sub(v, mul(eixo, h));
                let rho = norm(rad);
                let e = if rho > 1e-12 { mul(rad, 1. / rho) } else { any_perp(eixo) };
                let q = add(c, mul(e, maior));
                let w = sub(p, q);
                let l = norm(w);
                (l - menor, if l > 1e-12 { mul(w, 1. / l) } else { e })
            }
        }
    }

    pub fn projetar(&self, p: V3) -> V3 {
        let (d, n) = self.distancia(p);
        sub(p, mul(n, d))
    }

    /// Parâmetros para o kernel: tipo, origem, eixo, direção de referência, dois números.
    pub fn para_kernel(&self) -> [f64; 12] {
        let (o, ax, a, b) = match *self {
            Primitiva::Plano { o, n } => (o, n, 0., 0.),
            Primitiva::Cilindro { o, eixo, r } => (o, eixo, r, 0.),
            Primitiva::Cone { apice, eixo, ang } => (apice, eixo, ang, 0.),
            Primitiva::Esfera { c, r } => (c, [0., 0., 1.], r, 0.),
            Primitiva::Toro { c, eixo, maior, menor } => (c, eixo, maior, menor),
        };
        let x = any_perp(ax);
        [self.codigo() as f64, o[0], o[1], o[2], ax[0], ax[1], ax[2], x[0], x[1], x[2], a, b]
    }

    fn vetor(&self) -> Vec<f64> {
        match *self {
            Primitiva::Plano { o, n } => vec![o[0], o[1], o[2], n[0], n[1], n[2]],
            Primitiva::Cilindro { o, eixo, r } => vec![o[0], o[1], o[2], eixo[0], eixo[1], eixo[2], r],
            Primitiva::Cone { apice, eixo, ang } => vec![apice[0], apice[1], apice[2], eixo[0], eixo[1], eixo[2], ang],
            Primitiva::Esfera { c, r } => vec![c[0], c[1], c[2], r],
            Primitiva::Toro { c, eixo, maior, menor } => vec![c[0], c[1], c[2], eixo[0], eixo[1], eixo[2], maior, menor],
        }
    }

    fn de_vetor(&self, x: &[f64]) -> Option<Primitiva> {
        let v = |i: usize| [x[i], x[i + 1], x[i + 2]];
        Some(match self {
            Primitiva::Plano { .. } => Primitiva::Plano { o: v(0), n: unit(v(3))? },
            Primitiva::Cilindro { .. } => Primitiva::Cilindro { o: v(0), eixo: unit(v(3))?, r: x[6] },
            Primitiva::Cone { .. } => Primitiva::Cone { apice: v(0), eixo: unit(v(3))?, ang: x[6] },
            Primitiva::Esfera { .. } => Primitiva::Esfera { c: v(0), r: x[3] },
            Primitiva::Toro { .. } => Primitiva::Toro { c: v(0), eixo: unit(v(3))?, maior: x[6], menor: x[7] },
        })
    }

    /// Forma canônica: origem do eixo perto de `perto` e números com sentido geométrico.
    fn canonica(self, perto: V3) -> Option<Primitiva> {
        let ok = |x: f64| x.is_finite();
        match self {
            Primitiva::Cilindro { o, eixo, r } if ok(r) && r.abs() > 1e-9 => Some(Primitiva::Cilindro { o: add(o, mul(eixo, dot(sub(perto, o), eixo))), eixo, r: r.abs() }),
            Primitiva::Esfera { c, r } if ok(r) && r.abs() > 1e-9 => Some(Primitiva::Esfera { c, r: r.abs() }),
            Primitiva::Cone { apice, eixo, ang } if ok(ang) => {
                // semiângulo entre 0 e 90°, com o eixo para o lado da peça
                let (eixo, ang) = if ang < 0. { (mul(eixo, -1.), -ang) } else { (eixo, ang) };
                let (eixo, ang) = if ang > std::f64::consts::FRAC_PI_2 { (mul(eixo, -1.), std::f64::consts::PI - ang) } else { (eixo, ang) };
                let eixo = if dot(sub(perto, apice), eixo) < 0. { mul(eixo, -1.) } else { eixo };
                (ang > 1e-3 && ang < std::f64::consts::FRAC_PI_2 - 1e-3).then_some(Primitiva::Cone { apice, eixo, ang })
            }
            Primitiva::Toro { c, eixo, maior, menor } if ok(maior) && ok(menor) => {
                (maior.abs() > menor.abs() + 1e-9 && menor.abs() > 1e-9).then_some(Primitiva::Toro { c, eixo, maior: maior.abs(), menor: menor.abs() })
            }
            Primitiva::Plano { o, n } => Some(Primitiva::Plano { o, n }),
            _ => None,
        }
    }
}

// ---------- álgebra pequena

/// Autovalores e autovetores de uma matriz simétrica 3×3 (Jacobi), em ordem crescente.
pub fn autos(m: [[f64; 3]; 3]) -> ([f64; 3], [V3; 3]) {
    let mut a = m;
    let mut v = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for _ in 0..60 {
        let (mut p, mut q, mut maior) = (0, 1, 0.);
        for i in 0..3 {
            for j in i + 1..3 {
                if a[i][j].abs() > maior {
                    (p, q, maior) = (i, j, a[i][j].abs());
                }
            }
        }
        if maior < 1e-15 * (a[0][0].abs() + a[1][1].abs() + a[2][2].abs()).max(1e-300) {
            break;
        }
        let th = 0.5 * (2. * a[p][q]).atan2(a[q][q] - a[p][p]);
        let (s, c) = th.sin_cos();
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
        for row in &mut v {
            let (vp, vq) = (row[p], row[q]);
            row[p] = c * vp - s * vq;
            row[q] = s * vp + c * vq;
        }
    }
    let mut idx = [0, 1, 2];
    idx.sort_by(|i, j| a[*i][*i].total_cmp(&a[*j][*j]));
    let vals = idx.map(|i| a[i][i]);
    let vecs = idx.map(|i| [v[0][i], v[1][i], v[2][i]]);
    (vals, vecs)
}

/// Resolve `a x = b` por eliminação com pivô; `None` se for singular.
fn resolver(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for c in 0..n {
        let p = (c..n).max_by(|i, j| a[*i][c].abs().total_cmp(&a[*j][c].abs()))?;
        if a[p][c].abs() < 1e-300 {
            return None;
        }
        a.swap(c, p);
        b.swap(c, p);
        for r in c + 1..n {
            let f = a[r][c] / a[c][c];
            for k in c..n {
                a[r][k] -= f * a[c][k];
            }
            b[r] -= f * b[c];
        }
    }
    let mut x = vec![0.; n];
    for c in (0..n).rev() {
        let s: f64 = (c + 1..n).map(|k| a[c][k] * x[k]).sum();
        x[c] = (b[c] - s) / a[c][c];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

fn centroide(p: &[V3]) -> V3 {
    let s = p.iter().fold([0.; 3], |s, q| add(s, *q));
    mul(s, 1. / p.len().max(1) as f64)
}

fn covariancia(p: &[V3], c: V3) -> [[f64; 3]; 3] {
    let mut m = [[0.; 3]; 3];
    for q in p {
        let d = sub(*q, c);
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += d[i] * d[j];
            }
        }
    }
    m
}

/// Círculo algébrico no plano (centro e raio) dos pontos já em 2D.
fn circulo2d(p: &[[f64; 2]]) -> Option<([f64; 2], f64)> {
    if p.len() < 3 {
        return None;
    }
    let mut a = vec![vec![0.; 3]; 3];
    let mut b = vec![0.; 3];
    for q in p {
        let l = [q[0], q[1], 1.];
        let z = -(q[0] * q[0] + q[1] * q[1]);
        for i in 0..3 {
            for j in 0..3 {
                a[i][j] += l[i] * l[j];
            }
            b[i] += l[i] * z;
        }
    }
    let x = resolver(a, b)?;
    let c = [-x[0] / 2., -x[1] / 2.];
    let r2 = c[0] * c[0] + c[1] * c[1] - x[2];
    (r2 > 0.).then(|| (c, r2.sqrt()))
}

fn base(n: V3) -> (V3, V3) {
    let x = unit(any_perp(n)).unwrap_or([1., 0., 0.]);
    (x, cross(n, x))
}

// ---------- ajustes

/// Pontos e normais de uma região: vértices (com a normal média das faces da região) e triângulos.
pub struct Amostra {
    pub pts: Vec<V3>,
    pub nrm: Vec<V3>,
}

fn ajuste_plano(a: &Amostra) -> Option<Primitiva> {
    let c = centroide(&a.pts);
    let (_, v) = autos(covariancia(&a.pts, c));
    let mut n = v[0];
    let media = a.nrm.iter().fold([0.; 3], |s, q| add(s, *q));
    if dot(n, media) < 0. {
        n = mul(n, -1.);
    }
    Some(Primitiva::Plano { o: c, n })
}

fn ajuste_esfera(a: &Amostra) -> Option<Primitiva> {
    if a.pts.len() < 4 {
        return None;
    }
    let mut m = vec![vec![0.; 4]; 4];
    let mut b = vec![0.; 4];
    for p in &a.pts {
        let l = [p[0], p[1], p[2], 1.];
        let z = -dot(*p, *p);
        for i in 0..4 {
            for j in 0..4 {
                m[i][j] += l[i] * l[j];
            }
            b[i] += l[i] * z;
        }
    }
    let x = resolver(m, b)?;
    let c = [-x[0] / 2., -x[1] / 2., -x[2] / 2.];
    let r2 = dot(c, c) - x[3];
    (r2 > 0.).then(|| Primitiva::Esfera { c, r: r2.sqrt() })
}

/// Eixo pelas normais: o cilindro tem normais perpendiculares a ele (menor autovetor de Σ n nᵀ); o
/// cone, normais num cone em volta dele (menor autovetor da covariância centrada).
fn eixo_das_normais(nrm: &[V3], centrada: bool) -> V3 {
    let c = if centrada { centroide(nrm) } else { [0.; 3] };
    autos(covariancia(nrm, c)).1[0]
}

fn ajuste_cilindro(a: &Amostra) -> Option<Primitiva> {
    let eixo = eixo_das_normais(&a.nrm, false);
    let (x, y) = base(eixo);
    let p2: Vec<[f64; 2]> = a.pts.iter().map(|p| [dot(*p, x), dot(*p, y)]).collect();
    let (c, r) = circulo2d(&p2)?;
    let h = centroide(&a.pts);
    let o = add(add(mul(x, c[0]), mul(y, c[1])), mul(eixo, dot(h, eixo)));
    Some(Primitiva::Cilindro { o, eixo, r })
}

fn ajuste_cone(a: &Amostra) -> Option<Primitiva> {
    let eixo = eixo_das_normais(&a.nrm, true);
    // as normais fazem o mesmo ângulo com o eixo: n·a = −sen(semiângulo)
    let s = a.nrm.iter().map(|n| dot(*n, eixo)).sum::<f64>() / a.nrm.len().max(1) as f64;
    // o ápice está no plano tangente de cada ponto: n·(A − p) = 0
    let mut m = vec![vec![0.; 3]; 3];
    let mut b = vec![0.; 3];
    for (p, n) in a.pts.iter().zip(&a.nrm) {
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += n[i] * n[j];
            }
            b[i] += n[i] * dot(*n, *p);
        }
    }
    let apice = resolver(m, b)?;
    let apice = [apice[0], apice[1], apice[2]];
    Primitiva::Cone { apice, eixo: mul(eixo, -1.), ang: (-s).clamp(-1., 1.).asin() }.canonica(centroide(&a.pts)).or_else(|| Primitiva::Cone { apice, eixo, ang: s.clamp(-1., 1.).asin() }.canonica(centroide(&a.pts)))
}

/// Toro: os centros do tubo `p − r·n` caem num círculo. Busca o raio menor `r` que deixa esses
/// centros mais perto de um círculo plano.
fn ajuste_toro(a: &Amostra, tamanho: f64) -> Option<Primitiva> {
    let custo = |r: f64| -> Option<(f64, Primitiva)> {
        let q: Vec<V3> = a.pts.iter().zip(&a.nrm).map(|(p, n)| sub(*p, mul(*n, r))).collect();
        let c = centroide(&q);
        let (vals, vecs) = autos(covariancia(&q, c));
        let eixo = vecs[0];
        let (x, y) = base(eixo);
        let p2: Vec<[f64; 2]> = q.iter().map(|p| [dot(sub(*p, c), x), dot(sub(*p, c), y)]).collect();
        let (cc, rr) = circulo2d(&p2)?;
        let erro_circ: f64 = p2.iter().map(|p| ((p[0] - cc[0]).hypot(p[1] - cc[1]) - rr).powi(2)).sum();
        let centro = add(c, add(mul(x, cc[0]), mul(y, cc[1])));
        Some((vals[0].abs() + erro_circ, Primitiva::Toro { c: centro, eixo, maior: rr, menor: r }))
    };
    let mut melhor: Option<(f64, f64, Primitiva)> = None;
    for i in 1..=60 {
        for s in [-1., 1.] {
            let r = s * tamanho * i as f64 / 60.;
            if let Some((e, t)) = custo(r) {
                if melhor.as_ref().is_none_or(|m| e < m.0) {
                    melhor = Some((e, r, t));
                }
            }
        }
    }
    let (_, mut r, _) = melhor?;
    // busca de seção áurea em volta do melhor ponto da grade
    let passo = tamanho / 60.;
    let (mut lo, mut hi) = (r - passo, r + passo);
    for _ in 0..50 {
        let (m1, m2) = (hi - (hi - lo) / 1.618, lo + (hi - lo) / 1.618);
        let (e1, e2) = (custo(m1).map_or(f64::MAX, |x| x.0), custo(m2).map_or(f64::MAX, |x| x.0));
        if e1 < e2 {
            hi = m2;
        } else {
            lo = m1;
        }
    }
    r = (lo + hi) / 2.;
    // com a normal para fora, o centro do tubo fica a `r` para dentro: r positivo; furo, negativo
    custo(r).map(|x| x.1)
}

/// Levenberg-Marquardt sobre a distância exata dos pontos. Volta o ajuste refinado.
fn refinar(p0: Primitiva, pts: &[V3]) -> Primitiva {
    if matches!(p0, Primitiva::Plano { .. }) || pts.is_empty() {
        return p0;
    }
    // no máximo 400 pontos: o ajuste fica rápido e não perde nada que importe
    let passo = (pts.len() / 400).max(1);
    let amostra: Vec<V3> = pts.iter().step_by(passo).copied().collect();
    let resid = |pr: &Primitiva| -> Vec<f64> { amostra.iter().map(|p| pr.distancia(*p).0).collect() };
    let custo = |r: &[f64]| r.iter().map(|x| x * x).sum::<f64>();
    let mut cur = p0;
    let mut r = resid(&cur);
    let mut c = custo(&r);
    let mut lambda = 1e-3;
    for _ in 0..40 {
        let x = cur.vetor();
        let k = x.len();
        let h = 1e-7 * x.iter().fold(1f64, |m, v| m.max(v.abs()));
        let mut jac = vec![vec![0.; k]; amostra.len()];
        for j in 0..k {
            let mut xp = x.clone();
            xp[j] += h;
            let Some(pp) = cur.de_vetor(&xp) else { return cur };
            for (i, rp) in resid(&pp).iter().enumerate() {
                jac[i][j] = (rp - r[i]) / h;
            }
        }
        let mut jtj = vec![vec![0.; k]; k];
        let mut jtr = vec![0.; k];
        for (i, linha) in jac.iter().enumerate() {
            for a in 0..k {
                jtr[a] -= linha[a] * r[i];
                for b in 0..k {
                    jtj[a][b] += linha[a] * linha[b];
                }
            }
        }
        let mut melhorou = false;
        for _ in 0..8 {
            let mut m = jtj.clone();
            for (a, linha) in m.iter_mut().enumerate() {
                linha[a] += lambda * (jtj[a][a] + 1e-12);
            }
            let Some(dx) = resolver(m, jtr.clone()) else { break };
            let xn: Vec<f64> = x.iter().zip(&dx).map(|(a, b)| a + b).collect();
            if let Some(pn) = cur.de_vetor(&xn) {
                let rn = resid(&pn);
                let cn = custo(&rn);
                if cn < c {
                    (cur, r, c) = (pn, rn, cn);
                    lambda = (lambda * 0.3).max(1e-12);
                    melhorou = true;
                    break;
                }
            }
            lambda *= 10.;
        }
        if !melhorou || c < 1e-24 {
            break;
        }
    }
    cur
}

/// Maior distância dos pontos à primitiva e o pior desacordo de normal (cosseno).
fn qualidade(p: &Primitiva, a: &Amostra) -> (f64, f64) {
    let mut dmax = 0f64;
    let mut cmin = 1f64;
    for (q, n) in a.pts.iter().zip(&a.nrm) {
        let (d, g) = p.distancia(*q);
        dmax = dmax.max(d.abs());
        cmin = cmin.min(dot(g, *n).abs());
    }
    (dmax, cmin)
}

/// Melhor primitiva curva para a amostra dentro da tolerância, da mais simples para a mais complexa.
fn ajustar_curva(a: &Amostra, tol: f64, tamanho: f64) -> Option<Primitiva> {
    let perto = centroide(&a.pts);
    let limite = 1e3 * tamanho;
    let candidatos: [&dyn Fn() -> Option<Primitiva>; 4] = [&|| ajuste_esfera(a), &|| ajuste_cilindro(a), &|| ajuste_cone(a), &|| ajuste_toro(a, tamanho)];
    for f in candidatos {
        let Some(p) = f() else { continue };
        let p = refinar(p, &a.pts);
        let Some(p) = p.canonica(perto) else { continue };
        let grande = match p {
            Primitiva::Esfera { r, .. } | Primitiva::Cilindro { r, .. } => r > limite,
            Primitiva::Toro { maior, .. } => maior > limite,
            _ => false,
        };
        if grande {
            continue;
        }
        let (d, c) = qualidade(&p, a);
        if d <= tol && c > 0.85 {
            return Some(p);
        }
    }
    None
}

// ---------- malha e regiões

/// Resultado do reconhecimento: a região de cada triângulo (ou nenhuma) e as primitivas.
#[derive(Clone, Debug, Default)]
pub struct Reconhecimento {
    pub regiao: Vec<Option<usize>>,
    pub primitivas: Vec<Primitiva>,
}

impl Reconhecimento {
    /// Quantas regiões de cada tipo (plano, cilindro, cone, esfera, toro) e triângulos sem ajuste.
    pub fn contagem(&self) -> ([usize; 5], usize) {
        let mut c = [0; 5];
        for p in &self.primitivas {
            c[p.codigo()] += 1;
        }
        (c, self.regiao.iter().filter(|r| r.is_none()).count())
    }

    pub fn resumo(&self) -> String {
        let (c, livres) = self.contagem();
        let nomes = ["planos", "cilindros", "cones", "esferas", "toros"];
        let mut partes: Vec<String> = c.iter().zip(nomes).filter(|(n, _)| **n > 0).map(|(n, s)| format!("{n} {s}")).collect();
        if livres > 0 {
            partes.push(format!("{livres} triângulos sem ajuste"));
        }
        partes.join(", ")
    }
}

struct Topo<'a> {
    m: &'a Malha,
    /// vizinho do triângulo por aresta (aresta i vai do vértice i ao i+1)
    viz: Vec<[Option<u32>; 3]>,
    nrm: Vec<V3>,
    area: Vec<f64>,
}

impl<'a> Topo<'a> {
    fn nova(m: &'a Malha) -> Topo<'a> {
        let mut arestas: HashMap<(u32, u32), Vec<(u32, usize)>> = HashMap::new();
        for (t, tri) in m.tris.iter().enumerate() {
            for i in 0..3 {
                let (a, b) = (tri[i], tri[(i + 1) % 3]);
                arestas.entry((a.min(b), a.max(b))).or_default().push((t as u32, i));
            }
        }
        let mut viz = vec![[None; 3]; m.tris.len()];
        for lista in arestas.values() {
            if lista.len() == 2 {
                let ((t0, i0), (t1, i1)) = (lista[0], lista[1]);
                viz[t0 as usize][i0] = Some(t1);
                viz[t1 as usize][i1] = Some(t0);
            }
        }
        let (mut nrm, mut area) = (vec![], vec![]);
        for t in &m.tris {
            let [a, b, c] = t.map(|i| m.pos[i as usize]);
            let n = cross(sub(b, a), sub(c, a));
            let l = norm(n);
            area.push(l / 2.);
            nrm.push(if l > 0. { mul(n, 1. / l) } else { [0., 0., 1.] });
        }
        Topo { m, viz, nrm, area }
    }

    /// Ângulo entre as normais dos dois triângulos da aresta `i` de `t`, em graus.
    fn diedro(&self, t: usize, i: usize) -> f64 {
        match self.viz[t][i] {
            Some(u) => dot(self.nrm[t], self.nrm[u as usize]).clamp(-1., 1.).acos().to_degrees(),
            None => 180.,
        }
    }

    fn comprimento(&self, t: usize, i: usize) -> f64 {
        let tri = self.m.tris[t];
        dist(self.m.pos[tri[i] as usize], self.m.pos[tri[(i + 1) % 3] as usize])
    }

    /// Vértices (sem repetir) e normais médias dos triângulos dados.
    fn amostra(&self, tris: &[usize]) -> Amostra {
        let mut idx: HashMap<u32, usize> = HashMap::new();
        let mut a = Amostra { pts: vec![], nrm: vec![] };
        for &t in tris {
            for v in self.m.tris[t] {
                let k = *idx.entry(v).or_insert_with(|| {
                    a.pts.push(self.m.pos[v as usize]);
                    a.nrm.push([0.; 3]);
                    a.pts.len() - 1
                });
                a.nrm[k] = add(a.nrm[k], mul(self.nrm[t], self.area[t]));
            }
        }
        for n in &mut a.nrm {
            *n = unit(*n).unwrap_or([0., 0., 1.]);
        }
        a
    }

    /// O triângulo cabe na primitiva: vértices na tolerância e normal de acordo.
    fn cabe(&self, p: &Primitiva, t: usize, tol: f64, cos_min: f64) -> bool {
        let tri = self.m.tris[t];
        let pts = tri.map(|i| self.m.pos[i as usize]);
        if pts.iter().any(|q| p.distancia(*q).0.abs() > tol) {
            return false;
        }
        let c = mul(add(add(pts[0], pts[1]), pts[2]), 1. / 3.);
        dot(p.distancia(c).1, self.nrm[t]).abs() >= cos_min
    }

    /// Cresce a partir de `semente` entre os triângulos livres enquanto couberem em `p`.
    fn crescer(&self, p: &Primitiva, semente: &[usize], livre: &dyn Fn(usize) -> bool, tol: f64, cos_min: f64) -> Vec<usize> {
        let mut dentro = vec![false; self.m.tris.len()];
        let mut fila: VecDeque<usize> = VecDeque::new();
        let mut out = vec![];
        for &t in semente {
            if !dentro[t] {
                dentro[t] = true;
                fila.push_back(t);
                out.push(t);
            }
        }
        while let Some(t) = fila.pop_front() {
            for u in self.viz[t].iter().flatten() {
                let u = *u as usize;
                if !dentro[u] && livre(u) && self.cabe(p, u, tol, cos_min) {
                    dentro[u] = true;
                    fila.push_back(u);
                    out.push(u);
                }
            }
        }
        out
    }
}

/// Tolerância automática: um décimo de milésimo da diagonal, com piso de um micrômetro.
pub fn tolerancia_automatica(m: &Malha) -> f64 {
    let (lo, hi) = m.pos.iter().fold(([f64::MAX; 3], [f64::MIN; 3]), |(lo, hi), p| ([0, 1, 2].map(|i| lo[i].min(p[i])), [0, 1, 2].map(|i| hi[i].max(p[i]))));
    (dist(lo, hi) * 1e-4).max(1e-3)
}

/// Reconhece planos e superfícies curvas numa malha soldada.
pub fn reconhecer(m: &Malha, tol: f64) -> Reconhecimento {
    let topo = Topo::nova(m);
    let n = m.tris.len();
    let (lo, hi) = m.pos.iter().fold(([f64::MAX; 3], [f64::MIN; 3]), |(lo, hi), p| ([0, 1, 2].map(|i| lo[i].min(p[i])), [0, 1, 2].map(|i| hi[i].max(p[i]))));
    let tamanho = dist(lo, hi).max(1e-9);
    let mut rec = Reconhecimento { regiao: vec![None; n], primitivas: vec![] };
    let mut tentado = vec![false; n];

    // 0. componentes lisas (separadas por cantos vivos) que cabem inteiras numa primitiva: a esfera, o
    // cilindro e o cone de uma peça simples saem aqui, antes que uma faixa da malha pareça plana
    let cos_curva = 25f64.to_radians().cos();
    // área da componente lisa de cada triângulo
    let mut area_comp = vec![0f64; n];
    {
        let mut visto = vec![false; n];
        for t0 in 0..n {
            if visto[t0] {
                continue;
            }
            let mut comp = vec![t0];
            visto[t0] = true;
            let mut i = 0;
            while i < comp.len() {
                let t = comp[i];
                i += 1;
                for (j, u) in topo.viz[t].iter().enumerate() {
                    if let Some(u) = u.map(|u| u as usize) {
                        if !visto[u] && topo.diedro(t, j) < 35. {
                            visto[u] = true;
                            comp.push(u);
                        }
                    }
                }
            }
            let ac: f64 = comp.iter().map(|t| topo.area[*t]).sum();
            for t in &comp {
                area_comp[*t] = ac;
            }
            if comp.len() < 6 {
                continue;
            }
            let a = topo.amostra(&comp);
            let plano = ajuste_plano(&a).filter(|p| qualidade(p, &a).0 <= tol);
            if let Some(p) = plano.or_else(|| ajustar_curva(&a, tol, tamanho)) {
                if comp.iter().all(|t| topo.cabe(&p, *t, tol, cos_curva)) {
                    let k = rec.primitivas.len();
                    rec.primitivas.push(p);
                    for t in comp {
                        rec.regiao[t] = Some(k);
                    }
                }
            }
        }
    }

    // 1. planos
    let mut ordem: Vec<usize> = (0..n).collect();
    ordem.sort_by(|a, b| topo.area[*b].total_cmp(&topo.area[*a]));
    let cos1 = 1f64.to_radians().cos();
    for &t in &ordem {
        if rec.regiao[t].is_some() || tentado[t] || topo.area[t] <= 0. {
            continue;
        }
        let p0 = Primitiva::Plano { o: m.pos[m.tris[t][0] as usize], n: topo.nrm[t] };
        let livre = |u: usize| rec.regiao[u].is_none();
        let r = topo.crescer(&p0, &[t], &livre, tol, cos1);
        let Some(p) = ajuste_plano(&topo.amostra(&r)) else { continue };
        let r = topo.crescer(&p, &r, &livre, tol, cos1);
        for &u in &r {
            tentado[u] = true;
        }
        // borda: quanto dela é canto vivo
        let (mut viva, mut total) = (0., 0.);
        let dentro: std::collections::HashSet<usize> = r.iter().copied().collect();
        for &u in &r {
            for i in 0..3 {
                if topo.viz[u][i].is_none_or(|w| !dentro.contains(&(w as usize))) {
                    let l = topo.comprimento(u, i);
                    total += l;
                    if topo.diedro(u, i) >= 35. {
                        viva += l;
                    }
                }
            }
        }
        // faixa de uma face curva também é plana: só vale a região de borda viva, ou a compacta
        // (área / perímetro² de um retângulo de lados até 1:6) que pesa na componente lisa dela
        let area: f64 = r.iter().map(|u| topo.area[*u]).sum();
        let compacta = total > 0. && area / (total * total) >= 0.02 && area >= 0.02 * area_comp[t];
        if (r.len() >= 2 && compacta) || (total > 0. && viva / total >= 0.5) {
            let k = rec.primitivas.len();
            rec.primitivas.push(p);
            for u in r {
                rec.regiao[u] = Some(k);
            }
        }
    }

    // 2. curvas: componentes do que sobrou
    let mut visto = vec![false; n];
    for t0 in 0..n {
        if rec.regiao[t0].is_some() || visto[t0] {
            continue;
        }
        let mut comp = vec![t0];
        visto[t0] = true;
        let mut i = 0;
        while i < comp.len() {
            let t = comp[i];
            i += 1;
            for u in topo.viz[t].iter().flatten() {
                let u = *u as usize;
                if !visto[u] && rec.regiao[u].is_none() {
                    visto[u] = true;
                    comp.push(u);
                }
            }
        }
        if comp.len() >= 3 {
            if let Some(p) = ajustar_curva(&topo.amostra(&comp), tol, tamanho) {
                if comp.iter().all(|t| topo.cabe(&p, *t, tol, cos_curva)) {
                    let k = rec.primitivas.len();
                    rec.primitivas.push(p);
                    for t in comp {
                        rec.regiao[t] = Some(k);
                    }
                    continue;
                }
            }
        }
        // a componente tem mais de uma superfície: sementes de dois anéis
        let no_comp: std::collections::HashSet<usize> = comp.iter().copied().collect();
        for &s in &comp {
            if rec.regiao[s].is_some() {
                continue;
            }
            let livre = |u: usize| rec.regiao[u].is_none() && no_comp.contains(&u);
            let mut semente = vec![s];
            for _ in 0..2 {
                let mut prox = semente.clone();
                for &t in &semente {
                    for u in topo.viz[t].iter().flatten() {
                        let u = *u as usize;
                        if livre(u) && !prox.contains(&u) {
                            prox.push(u);
                        }
                    }
                }
                semente = prox;
            }
            if semente.len() < 4 {
                continue;
            }
            let Some(p) = ajustar_curva(&topo.amostra(&semente), tol, tamanho) else { continue };
            let semente: Vec<usize> = semente.into_iter().filter(|t| topo.cabe(&p, *t, tol, cos_curva)).collect();
            if !semente.contains(&s) {
                continue;
            }
            let mut r = topo.crescer(&p, &semente, &livre, tol, cos_curva);
            let mut p = p;
            // reajusta com a região maior e cresce de novo
            for _ in 0..3 {
                let Some(q) = refinar(p, &topo.amostra(&r).pts).canonica(centroide(&topo.amostra(&r).pts)) else { break };
                if !r.iter().all(|t| topo.cabe(&q, *t, tol, cos_curva)) {
                    break;
                }
                p = q;
                let r2 = topo.crescer(&p, &r, &livre, tol, cos_curva);
                if r2.len() == r.len() {
                    break;
                }
                r = r2;
            }
            if r.len() >= 6 {
                let k = rec.primitivas.len();
                rec.primitivas.push(p);
                for t in r {
                    rec.regiao[t] = Some(k);
                }
            }
        }
    }
    rec
}

/// Vértices encaixados nas superfícies: um vértice de uma região vai para ela; o da borda entre
/// regiões vai para a interseção delas (projeções alternadas).
pub fn encaixar(m: &Malha, rec: &Reconhecimento, tol: f64) -> Vec<V3> {
    let mut regs: Vec<Vec<usize>> = vec![vec![]; m.pos.len()];
    for (t, tri) in m.tris.iter().enumerate() {
        if let Some(k) = rec.regiao[t] {
            for v in tri {
                if !regs[*v as usize].contains(&k) {
                    regs[*v as usize].push(k);
                }
            }
        }
    }
    m.pos
        .iter()
        .zip(&regs)
        .map(|(p, r)| match r.len() {
            0 => *p,
            1 => rec.primitivas[r[0]].projetar(*p),
            _ => {
                let mut q = *p;
                for _ in 0..60 {
                    for k in r {
                        q = rec.primitivas[*k].projetar(q);
                    }
                }
                // sem convergir (superfícies quase tangentes), fica na primeira e o kernel tolera
                if dist(q, *p) > 50. * tol {
                    rec.primitivas[r[0]].projetar(*p)
                } else {
                    q
                }
            }
        })
        .collect()
}

/// Laços de borda de cada região: (vértice, região do outro lado da aresta que sai dele, ou −1).
pub fn lacos(m: &Malha, rec: &Reconhecimento) -> Vec<Vec<Vec<(u32, i64)>>> {
    let mut saida: Vec<Vec<Vec<(u32, i64)>>> = vec![vec![]; rec.primitivas.len()];
    let mut dono: HashMap<(u32, u32), usize> = HashMap::new();
    for (t, tri) in m.tris.iter().enumerate() {
        for i in 0..3 {
            dono.insert((tri[i], tri[(i + 1) % 3]), t);
        }
    }
    let mut meias: Vec<HashMap<u32, Vec<(u32, i64)>>> = vec![HashMap::new(); rec.primitivas.len()];
    for (t, tri) in m.tris.iter().enumerate() {
        let Some(k) = rec.regiao[t] else { continue };
        for i in 0..3 {
            let (a, b) = (tri[i], tri[(i + 1) % 3]);
            let outro = dono.get(&(b, a)).and_then(|u| rec.regiao[*u]);
            if outro != Some(k) {
                meias[k].entry(a).or_default().push((b, outro.map_or(-1, |x| x as i64)));
            }
        }
    }
    for (k, mapa) in meias.iter_mut().enumerate() {
        while let Some(&ini) = mapa.keys().min() {
            let mut laco = vec![];
            let mut v = ini;
            loop {
                let Some(lista) = mapa.get_mut(&v) else { break };
                let Some((b, nb)) = lista.pop() else { break };
                if lista.is_empty() {
                    mapa.remove(&v);
                }
                laco.push((v, nb));
                v = b;
                if v == ini {
                    break;
                }
            }
            if laco.len() >= 2 {
                saida[k].push(laco);
            }
        }
    }
    saida
}

/// Tudo o que o kernel precisa para montar o B-rep híbrido, já achatado.
pub struct Descricao {
    pub vertices: Vec<f64>,
    pub facetas: Vec<u32>,
    pub regioes: Vec<f64>,
    pub lacos: Vec<i64>,
}

pub fn descrever(m: &Malha, rec: &Reconhecimento, tol: f64) -> Descricao {
    let pos = encaixar(m, rec, tol);
    let lc = lacos(m, rec);
    let mut d = Descricao { vertices: pos.iter().flatten().copied().collect(), facetas: vec![], regioes: vec![], lacos: vec![] };
    for (t, tri) in m.tris.iter().enumerate() {
        if rec.regiao[t].is_none() {
            d.facetas.extend_from_slice(tri);
        }
    }
    for (p, ls) in rec.primitivas.iter().zip(&lc) {
        d.regioes.extend_from_slice(&p.para_kernel());
        d.lacos.push(ls.len() as i64);
        for l in ls {
            d.lacos.push(l.len() as i64);
            for (v, nb) in l {
                d.lacos.push(*v as i64);
                d.lacos.push(*nb);
            }
        }
    }
    d
}

fn montar(m: &Malha, d: &Descricao, tol: f64, solido: bool) -> Result<fenkra_kernel::Shape, String> {
    let s = fenkra_kernel::Shape::from_mesh_hybrid(&d.vertices, &d.facetas, &d.regioes, &d.lacos, tol, solido).map_err(|e| e.0)?;
    if solido {
        // o sólido tem de guardar o volume da malha: a corda das faces curvas é a única diferença
        let (vm, vs) = (m.volume().abs(), s.mass().volume);
        if (vs - vm).abs() > 0.02 * vm.max(1e-12) {
            return Err(format!("o volume saiu {vs:.3} contra {vm:.3} da malha"));
        }
    }
    Ok(s)
}

/// Descrição sem região: todo triângulo fica plano, e as faces coplanares se fundem no kernel.
fn so_facetas(m: &Malha) -> Descricao {
    Descricao { vertices: m.pos.iter().flatten().copied().collect(), facetas: m.tris.iter().flatten().copied().collect(), regioes: vec![], lacos: vec![] }
}

/// Só as regiões dos tipos em `fica` (códigos de `Primitiva::codigo`): o caminho de volta quando um
/// tipo de face curva não fecha.
fn so_tipos(rec: &Reconhecimento, fica: &[usize]) -> Reconhecimento {
    let mut novo: Vec<Option<usize>> = vec![None; rec.primitivas.len()];
    let mut out = Reconhecimento { regiao: vec![], primitivas: vec![] };
    for (k, p) in rec.primitivas.iter().enumerate() {
        if fica.contains(&p.codigo()) {
            novo[k] = Some(out.primitivas.len());
            out.primitivas.push(*p);
        }
    }
    out.regiao = rec.regiao.iter().map(|r| r.and_then(|k| novo[k])).collect();
    out
}

/// Malha (uma componente soldada) para B-rep. Devolve a forma e o que foi reconhecido. No modo de
/// primitivas, se a montagem com as faces curvas falhar, tenta só com os planos e, por fim, facetado:
/// o resultado é sempre um B-rep válido.
pub fn converter(m: &Malha, modo: fenkra_core::malha::ModoBrep, tol: f64, solido: bool) -> Result<(fenkra_kernel::Shape, String), String> {
    let tol = if tol > 0. { tol } else { tolerancia_automatica(m) };
    let facetado = |motivo: Option<String>| -> Result<(fenkra_kernel::Shape, String), String> {
        let s = montar(m, &so_facetas(m), tol, solido)?;
        let n = s.face_count();
        Ok((s, match motivo {
            Some(x) => format!("facetado, {n} faces (primitivas recusadas: {x})"),
            None => format!("facetado, {n} faces"),
        }))
    };
    if modo == fenkra_core::malha::ModoBrep::Facetado {
        return facetado(None);
    }
    let rec = reconhecer(m, tol);
    let primeiro = match montar(m, &descrever(m, &rec, tol), tol, solido) {
        Ok(s) => return Ok((s, rec.resumo())),
        Err(e) => e,
    };
    // tira um tipo curvo por vez, do mais difícil de costurar ao mais simples, até fechar
    let ordem: [(&[usize], &str); 4] = [(&[0, 1, 2, 3], "toros"), (&[0, 1, 3], "toros e cones"), (&[0, 1], "toros, cones e esferas"), (&[0], "todas as curvas")];
    for (fica, nome) in ordem {
        let r = so_tipos(&rec, fica);
        if r.primitivas.len() == rec.primitivas.len() {
            continue;
        }
        if let Ok(s) = montar(m, &descrever(m, &r, tol), tol, solido) {
            return Ok((s, format!("{} ({nome} ficaram triangulares: {primeiro})", r.resumo())));
        }
    }
    facetado(Some(primeiro))
}

/// Maior distância entre a malha e o sólido, nos dois sentidos: da malha até o sólido tesselado fino
/// e do sólido até a malha. O sólido é tesselado a um vigésimo da tolerância da malha de origem.
pub fn desvio_maximo(m: &Malha, s: &fenkra_kernel::Shape) -> f64 {
    use fenkra_malha::desvio::desvio;
    let diag = s.bbox().map(|(a, b)| dist(a, b)).unwrap_or(1.);
    let Ok(fina) = crate::corpo_malha::malha_da_forma(s, (diag * 2e-5).max(1e-4), 5f64.to_radians()) else { return f64::NAN };
    desvio(m, &fenkra_malha::Bvh::nova(&fina)).maximo.max(desvio(&fina, &fenkra_malha::Bvh::nova(m)).maximo)
}

/// Peça de exemplo das capturas: placa com furo e todas as arestas arredondadas, em malha de STL.
pub fn malha_de_exemplo() -> Malha {
    use fenkra_core::*;
    let s = |x: &str| x.to_string();
    let mut d = Document::new("placa");
    let xy = PlaneRef::Origin(OriginPlane::XY);
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: xy.clone(), a: s("80"), b: s("50"), c: s("16"), d: s("0"), op: BoolOp::Add }));
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: xy, a: s("12"), b: s("16"), c: s("0"), d: s("0"), op: BoolOp::Cut }));
    d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("3"), edges: EdgeSel::AllOfBody, mode: Default::default(), prop: Default::default(), keys: vec![], extra: Default::default() }));
    crate::final_shape(&d).and_then(|f| crate::corpo_malha::malha_da_forma(&f, 0.05, 20f64.to_radians()).ok()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amostra_de(pts: Vec<V3>, nrm: Vec<V3>) -> Amostra {
        Amostra { pts, nrm }
    }

    #[test]
    fn autos_da_diagonal() {
        let (v, e) = autos([[3., 0., 0.], [0., 1., 0.], [0., 0., 2.]]);
        assert_eq!(v, [1., 2., 3.]);
        assert_eq!(e[0].map(f64::abs), [0., 1., 0.]);
    }

    #[test]
    fn ajusta_cilindro_cone_esfera_e_toro_exatos() {
        let mut pc = vec![];
        let mut nc = vec![];
        let mut pk = vec![];
        let mut nk = vec![];
        let mut pe = vec![];
        let mut ne = vec![];
        let mut pt = vec![];
        let mut nt = vec![];
        let (ang, apice) = (20f64.to_radians(), [1., 2., 3.]);
        for i in 0..12 {
            for j in 0..5 {
                let a = i as f64 * 0.5;
                let z = j as f64 * 3.;
                // cilindro r 7 em torno de (5, 0, z)
                pc.push([5. + 7. * a.cos(), 7. * a.sin(), z]);
                nc.push([a.cos(), a.sin(), 0.]);
                // cone com ápice (1,2,3), eixo +Z
                let h = 2. + z;
                let rho = h * ang.tan();
                pk.push([1. + rho * a.cos(), 2. + rho * a.sin(), 3. + h]);
                nk.push([ang.cos() * a.cos(), ang.cos() * a.sin(), -ang.sin()]);
                // esfera r 4 em (0, 0, 10)
                let b = -1. + j as f64 * 0.5;
                pe.push([4. * b.cos() * a.cos(), 4. * b.cos() * a.sin(), 10. + 4. * b.sin()]);
                ne.push([b.cos() * a.cos(), b.cos() * a.sin(), b.sin()]);
                // toro R 10, r 2, eixo Z
                let c = j as f64 * 1.1;
                let e = [a.cos(), a.sin(), 0.];
                let nn = add(mul(e, c.cos()), [0., 0., c.sin()]);
                pt.push(add(mul(e, 10.), mul(nn, 2.)));
                nt.push(nn);
            }
        }
        let tol = 1e-6;
        match ajustar_curva(&amostra_de(pc, nc), tol, 50.) {
            Some(Primitiva::Cilindro { r, eixo, .. }) => assert!((r - 7.).abs() < 1e-6 && eixo[2].abs() > 0.999999),
            x => panic!("cilindro: {x:?}"),
        }
        match ajustar_curva(&amostra_de(pk, nk), tol, 50.) {
            Some(Primitiva::Cone { apice: a, ang: g, .. }) => assert!(dist(a, apice) < 1e-5 && (g - ang).abs() < 1e-6, "{a:?} {g}"),
            x => panic!("cone: {x:?}"),
        }
        match ajustar_curva(&amostra_de(pe, ne), tol, 50.) {
            Some(Primitiva::Esfera { r, c }) => assert!((r - 4.).abs() < 1e-6 && dist(c, [0., 0., 10.]) < 1e-6),
            x => panic!("esfera: {x:?}"),
        }
        match ajustar_curva(&amostra_de(pt, nt), tol, 50.) {
            Some(Primitiva::Toro { maior, menor, .. }) => assert!((maior - 10.).abs() < 1e-5 && (menor - 2.).abs() < 1e-5, "{maior} {menor}"),
            x => panic!("toro: {x:?}"),
        }
    }

    #[test]
    fn projetar_cai_na_superficie() {
        let ps = [
            Primitiva::Cilindro { o: [0.; 3], eixo: [0., 0., 1.], r: 3. },
            Primitiva::Cone { apice: [0.; 3], eixo: [0., 0., 1.], ang: 0.4 },
            Primitiva::Esfera { c: [1., 1., 1.], r: 2. },
            Primitiva::Toro { c: [0.; 3], eixo: [0., 0., 1.], maior: 8., menor: 1.5 },
        ];
        for p in ps {
            let q = p.projetar([2.5, 1.7, 3.3]);
            assert!(p.distancia(q).0.abs() < 1e-9, "{p:?}");
        }
    }
}
