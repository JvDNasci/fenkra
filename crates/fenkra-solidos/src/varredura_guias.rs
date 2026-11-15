// SPDX-License-Identifier: LGPL-2.1-or-later
//! B40: seção cônica da varredura por três guias, com continuidade G2 exata nas pontas.
//!
//! Como funciona no kernel: em cada plano normal à spine, a guia 1 e a guia 2 dão as pontas P0 e P2
//! da seção e a guia do ápice dá o ponto de controle P1. A seção é uma cônica racional quadrática
//! (pesos 1, w, 1) e `w = ρ / (1 - ρ)`. O kernel costura as seções em superfície (`loft_surface`).
//!
//! G2 exato: a curvatura de uma racional quadrática na ponta é `κ = h / (2 w² a²)`, em que `a` é a
//! distância P0-P1 e `h` é a distância de P2 à reta P0-P1. Invertendo, o peso que dá a curvatura
//! pedida é `w = sqrt(h / (2 κ a²))`. Em vez de ajustar ρ por tentativa, calculamos w por seção.

use crate::geo3d::{cross, dist, dot, norm, sub, V3};

/// ρ (0 a 1) que dá raio de curvatura `raio` em P0 para a cônica de controle P0, P1, P2.
/// `None` se P1 estiver na reta P0-P2 ou colado em P0 (não há curva), ou se o raio for inválido.
pub fn rho_para_raio(p0: V3, p1: V3, p2: V3, raio: f64) -> Option<f64> {
    if raio <= 0. {
        return None;
    }
    let a = dist(p0, p1);
    if a < 1e-12 {
        return None;
    }
    let t = [(p1[0] - p0[0]) / a, (p1[1] - p0[1]) / a, (p1[2] - p0[2]) / a];
    let v = sub(p2, p0);
    let h = norm(cross(t, v));
    if h < 1e-12 {
        return None;
    }
    let w = (h * raio / (2. * a * a)).sqrt();
    Some(w / (1. + w))
}

/// Curvatura da cônica racional (pesos 1, w, 1) na ponta P0. Serve de conferência de `rho_para_raio`.
pub fn curvatura_na_ponta(p0: V3, p1: V3, p2: V3, rho: f64) -> f64 {
    let w = rho / (1. - rho);
    let a = dist(p0, p1);
    let t = [(p1[0] - p0[0]) / a, (p1[1] - p0[1]) / a, (p1[2] - p0[2]) / a];
    let h = norm(cross(t, sub(p2, p0)));
    h / (2. * w * w * a * a)
}

/// Pontos da seção cônica, de P0 a P2, em `n` amostras (n >= 2).
pub fn secao_conica(p0: V3, p1: V3, p2: V3, rho: f64, n: usize) -> Vec<V3> {
    let w = rho / (1. - rho);
    (0..n)
        .map(|k| {
            let t = k as f64 / (n - 1) as f64;
            let (b0, b1, b2) = ((1. - t) * (1. - t), 2. * t * (1. - t) * w, t * t);
            let den = b0 + b1 + b2;
            [0, 1, 2].map(|j| (b0 * p0[j] + b1 * p1[j] + b2 * p2[j]) / den)
        })
        .collect()
}

/// Ângulo (graus) entre a tangente de duas seções vizinhas na mesma ponta: mede o salto G1 entre
/// elas. Zero em seções que variam suavemente.
pub fn salto_de_tangente(a: &[V3], b: &[V3]) -> f64 {
    let t = |s: &[V3]| {
        let d = sub(s[1], s[0]);
        let n = norm(d);
        [d[0] / n, d[1] / n, d[2] / n]
    };
    dot(t(a), t(b)).clamp(-1., 1.).acos().to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// curvatura numérica no início, por três pontos bem próximos (círculo por eles)
    fn curv_num(s: impl Fn(f64) -> V3) -> f64 {
        let (a, b, c) = (s(0.), s(1e-3), s(2e-3));
        let (ab, bc, ca) = (dist(a, b), dist(b, c), dist(c, a));
        let area2 = norm(cross(sub(b, a), sub(c, a)));
        2. * area2 / (ab * bc * ca)
    }

    #[test]
    fn rho_por_raio_reproduz_a_curvatura_pedida() {
        let (p0, p1, p2) = ([0., 0., 0.], [10., 8., 0.], [30., 0., 0.]);
        for raio in [5., 12., 40.] {
            let rho = rho_para_raio(p0, p1, p2, raio).expect("rho");
            assert!(rho > 0. && rho < 1.);
            assert!((1. / curvatura_na_ponta(p0, p1, p2, rho) - raio).abs() < 1e-9, "analítico");
            let w = rho / (1. - rho);
            let s = |t: f64| {
                let (b0, b1, b2) = ((1. - t) * (1. - t), 2. * t * (1. - t) * w, t * t);
                let den = b0 + b1 + b2;
                [0, 1, 2].map(|j| (b0 * p0[j] + b1 * p1[j] + b2 * p2[j]) / den)
            };
            let k = curv_num(s);
            assert!((1. / k - raio).abs() < 0.02 * raio, "numérico {} contra {raio}", 1. / k);
        }
    }

    #[test]
    fn quarto_de_circulo_tem_rho_de_raiz_de_dois() {
        // quarto de círculo de raio 1: w = √2/2, ρ = w/(1+w)
        let rho = rho_para_raio([1., 0., 0.], [1., 1., 0.], [0., 1., 0.], 1.).unwrap();
        let w = std::f64::consts::FRAC_1_SQRT_2;
        assert!((rho - w / (1. + w)).abs() < 1e-12);
        let s = secao_conica([1., 0., 0.], [1., 1., 0.], [0., 1., 0.], rho, 17);
        for p in &s {
            assert!((norm(*p) - 1.).abs() < 1e-9, "fora do círculo: {p:?}");
        }
    }

    #[test]
    fn casos_degenerados_nao_dao_rho() {
        assert!(rho_para_raio([0.; 3], [5., 0., 0.], [10., 0., 0.], 3.).is_none(), "colinear");
        assert!(rho_para_raio([0.; 3], [0.; 3], [10., 0., 0.], 3.).is_none(), "ápice em cima da ponta");
        assert!(rho_para_raio([0.; 3], [1., 1., 0.], [2., 0., 0.], 0.).is_none(), "raio nulo");
    }
}
