// SPDX-License-Identifier: LGPL-2.1-or-later
//! Análises de qualidade de superfície (B34). Os números vêm da geometria exata do kernel: a conexão
//! mede a junção nas bordas, o desvio projeta cada ponto na outra superfície e o pente sai da curva.
//! A tela só pinta o que está aqui, por isso nada depende de zoom ou de malha de exibição.

use crate::geo3d::*;
use fenkra_kernel::{AmostraConexao, Mesh, Shape};
use std::sync::Arc;

/// Junção de duas superfícies: as amostras e os piores valores.
#[derive(Clone, Debug, PartialEq)]
pub struct Conexao {
    pub amostras: Vec<AmostraConexao>,
    pub folga: f64,
    pub angulo: f64,
    pub dif_curvatura: f64,
    /// onde cada um dos três piores acontece, nessa ordem
    pub onde: [V3; 3],
}

/// Curva de nível de continuidade alcançada, do pior para o melhor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nivel {
    Aberta,
    G0,
    G1,
    G2,
}

/// Tolerâncias do veredito: a posição da modelagem, 0,5 grau e 2% da maior curvatura com piso de 0,001/mm.
pub const TOL_FOLGA: f64 = 0.01;
pub const TOL_ANGULO: f64 = 0.5;

impl Conexao {
    /// Resume as amostras. Sem amostra, não há junção a medir.
    pub fn de(amostras: Vec<AmostraConexao>) -> Option<Conexao> {
        let dk = |a: &AmostraConexao| (a.curvatura[0] - a.curvatura[1]).abs();
        let pior = |f: &dyn Fn(&AmostraConexao) -> f64| amostras.iter().max_by(|a, b| f(a).total_cmp(&f(b))).map(|a| (f(a), a.ponto));
        let (folga, p1) = pior(&|a| a.folga)?;
        let (angulo, p2) = pior(&|a| a.angulo)?;
        let (dif_curvatura, p3) = pior(&dk)?;
        Some(Conexao { folga, angulo, dif_curvatura, onde: [p1, p2, p3], amostras })
    }

    /// Continuidade que a junção atinge.
    pub fn nivel(&self) -> Nivel {
        let kmax = self.amostras.iter().flat_map(|a| a.curvatura).fold(0f64, |m, k| m.max(k.abs()));
        if self.folga > TOL_FOLGA {
            Nivel::Aberta
        } else if self.angulo > TOL_ANGULO {
            Nivel::G0
        } else if self.dif_curvatura > (kmax * 0.02).max(1e-3) {
            Nivel::G1
        } else {
            Nivel::G2
        }
    }
}

/// Distância de cada vértice da malha até a outra superfície.
#[derive(Clone, Debug)]
pub struct Desvio {
    pub malha: Arc<Mesh>,
    /// um valor com sinal por vértice, em mm
    pub valores: Vec<f32>,
    pub min: f64,
    pub max: f64,
    /// raiz da média dos quadrados
    pub rms: f64,
    pub pior: V3,
}

impl PartialEq for Desvio {
    fn eq(&self, o: &Self) -> bool {
        self.valores == o.valores
    }
}

impl Desvio {
    pub fn medir(malha: Mesh, b: &Shape) -> Result<Desvio, String> {
        let pts: Vec<V3> = malha.positions.chunks_exact(3).map(|q| [q[0] as f64, q[1] as f64, q[2] as f64]).collect();
        if pts.is_empty() {
            return Err("superfície sem malha".into());
        }
        let d = b.signed_distances(&pts).map_err(|x| x.0)?;
        let (mut min, mut max, mut q, mut pior, mut maior) = (f64::MAX, f64::MIN, 0., pts[0], -1.);
        for (p, v) in pts.iter().zip(&d) {
            min = min.min(*v);
            max = max.max(*v);
            q += v * v;
            if v.abs() > maior {
                (maior, pior) = (v.abs(), *p);
            }
        }
        let rms = (q / d.len() as f64).sqrt();
        Ok(Desvio { malha: Arc::new(malha), valores: d.iter().map(|x| *x as f32).collect(), min, max, rms, pior })
    }
}

/// Um dente do pente: da curva até a ponta, na direção oposta ao centro de curvatura.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dente {
    pub base: V3,
    pub ponta: V3,
    /// curvatura ali, em 1/mm
    pub k: f64,
}

/// Curvatura por três pontos a `h` de distância: o inverso do raio da circunferência que passa por eles.
fn menger(a: V3, b: V3, c: V3) -> (f64, V3) {
    let (ab, cb) = (sub(a, b), sub(c, b));
    let n = norm(cross(ab, cb));
    let den = norm(ab) * norm(cb) * dist(a, c);
    if den < 1e-18 || n < 1e-12 * den {
        return (0., [0.; 3]);
    }
    // o meio da corda aponta para o lado de dentro da curva
    (2. * n / den, unit(add(ab, cb)).unwrap_or([0.; 3]))
}

/// Pente de curvatura com `n` dentes. `escala`: mm de dente por 1/mm de curvatura.
pub fn pente(c: &Curve3, n: usize, escala: f64) -> Vec<Dente> {
    let n = n.max(4);
    let total = c.length();
    let fechada = c.closed;
    // a janela de medida cobre o passo do pente: mais curta, ela só veria a polilinha
    let h = total / n as f64;
    let em = |s: f64| c.at_dist(if fechada { s.rem_euclid(total) } else { s }).0;
    (0..=n)
        .map(|i| {
            let s = total * i as f64 / n as f64;
            let meio = if fechada { s } else { s.clamp(h, total - h) };
            let (k, dentro) = menger(em(meio - h), em(meio), em(meio + h));
            let base = em(s);
            Dente { base, ponta: sub(base, mul(dentro, k * escala)), k }
        })
        .collect()
}

/// Maior curvatura do pente e onde ela está.
pub fn pico(d: &[Dente]) -> Option<&Dente> {
    d.iter().max_by(|a, b| a.k.total_cmp(&b.k))
}

/// Resultado da análise que a tela desenha.
#[derive(Clone, Debug, PartialEq)]
pub enum Analise {
    Conexao(Conexao),
    Desvio(Desvio),
    Pente(Vec<Dente>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pente_do_circulo_tem_a_curvatura_do_raio() {
        let f = fenkra_kernel::Frame { origin: [0.; 3], x: [1., 0., 0.], normal: [0., 0., 1.] };
        let c = circle_curve([0.; 3], &f, 25., 0., 360.).unwrap();
        for d in pente(&c, 24, 100.) {
            assert!((d.k - 1. / 25.).abs() < 1e-3, "k = {}", d.k);
            // aponta para fora: a ponta fica mais longe do centro
            assert!(norm(d.ponta) > norm(d.base));
        }
    }

    #[test]
    fn pente_da_reta_e_zero() {
        let c = Curve3::line([0.; 3], [50., 0., 0.]).unwrap();
        assert!(pente(&c, 10, 100.).iter().all(|d| d.k == 0. && d.base == d.ponta));
    }

    #[test]
    fn veredito_da_conexao() {
        let a = |folga, angulo, k1| AmostraConexao { ponto: [0.; 3], folga, angulo, curvatura: [k1, 0.] };
        assert_eq!(Conexao::de(vec![a(0.5, 0., 0.)]).unwrap().nivel(), Nivel::Aberta);
        assert_eq!(Conexao::de(vec![a(0., 10., 0.)]).unwrap().nivel(), Nivel::G0);
        assert_eq!(Conexao::de(vec![a(0., 0.1, 0.05)]).unwrap().nivel(), Nivel::G1);
        assert_eq!(Conexao::de(vec![a(0., 0.1, 0.)]).unwrap().nivel(), Nivel::G2);
        assert!(Conexao::de(vec![]).is_none());
    }
}
