// SPDX-License-Identifier: LGPL-2.1-or-later
//! B40: dobra cônica (chapa enrolada em tronco de cone) e seu desenvolvimento exato.
//!
//! Como funciona no kernel: o tronco de cone é um perfil retangular (comprimento da geratriz × espessura)
//! girado 360° em torno do eixo; a planificada é um setor de coroa circular com o mesmo volume. A
//! geratriz fica com o comprimento da linha neutra e as pontas do perfil são normais a ela, então o
//! setor termina em arcos. Um tronco de cone é desenvolvível: a geratriz de semiângulo `α` desenvolve
//! um raio `ρ = r / sin α` e o setor abre `θ = 2π·sin α`. A linha neutra fica a `K·t` da face interna,
//! medida na normal da parede (não na horizontal).
//!
//! O que aqui é exato: o desenvolvimento (raios e ângulo do setor), o comprimento do arco neutro
//! (igual ao perímetro neutro do cone) e o volume (Pappus). Fica de fora: a aba cônica numa borda
//! circular de chapa plana (ela não se desenvolve sem esticar o material; veja `dobra por linha em curva`).

use fenkra_kernel::{Frame, Loop2, Seg2, Shape};

/// Desenvolvimento do tronco de cone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cone {
    /// semiângulo do cone, em relação ao eixo (rad)
    pub alfa: f64,
    /// comprimento da geratriz (mm), igual na face interna, na neutra e na externa
    pub geratriz: f64,
    /// raios neutros horizontais das duas bocas (menor, maior)
    pub r_neutro: [f64; 2],
    /// raios do setor planificado na linha neutra (menor, maior)
    pub rho: [f64; 2],
    /// abertura do setor (rad)
    pub theta: f64,
}

impl Cone {
    /// Comprimento do arco neutro do setor numa das bocas: igual ao perímetro neutro do cone.
    pub fn arco(&self, boca: usize) -> f64 {
        self.rho[boca] * self.theta
    }

    /// Área neutra planificada (mm²).
    pub fn area(&self) -> f64 {
        self.theta / 2. * (self.rho[1] * self.rho[1] - self.rho[0] * self.rho[0])
    }
}

/// `r1` e `r2`: raios internos da boca menor e da maior; `h`: altura; `t`: espessura; `k`: fator K.
pub fn desenvolve(r1: f64, r2: f64, h: f64, t: f64, k: f64) -> Result<Cone, String> {
    if r1 <= 0. || h <= 0. || t <= 0. {
        return Err("o cone precisa de raio, altura e espessura maiores que zero".into());
    }
    if r2 <= r1 + 1e-9 {
        return Err("o raio da boca maior precisa passar o da menor (para cilindro use a dobra comum)".into());
    }
    if !(0.0..=1.0).contains(&k) {
        return Err("o fator K fica entre 0 e 1".into());
    }
    let geratriz = (r2 - r1).hypot(h);
    let alfa = ((r2 - r1) / geratriz).asin();
    let desloca = k * t * alfa.cos();
    let r_neutro = [r1 + desloca, r2 + desloca];
    let rho = [r_neutro[0] / alfa.sin(), r_neutro[1] / alfa.sin()];
    Ok(Cone { alfa, geratriz, r_neutro, rho, theta: std::f64::consts::TAU * alfa.sin() })
}

/// O tronco dobrado: eixo em Z, boca menor em z = 0.
pub fn corpo_dobrado(r1: f64, r2: f64, h: f64, t: f64) -> Result<Shape, String> {
    let k = |e: fenkra_kernel::KernelError| e.0;
    let l = (r2 - r1).hypot(h);
    let (s, c) = ((r2 - r1) / l, h / l);
    // normal da parede, para fora: (cos α, −sin α) com α o semiângulo (sin α = (r2−r1)/L)
    let n = (c, -s);
    let (a, b) = ([r1, 0.], [r2, h]);
    let (a2, b2) = ([a[0] + t * n.0, a[1] + t * n.1], [b[0] + t * n.0, b[1] + t * n.1]);
    let perfil: Loop2 = vec![Seg2::Line(a, b), Seg2::Line(b, b2), Seg2::Line(b2, a2), Seg2::Line(a2, a)];
    let plano = Frame { origin: [0.; 3], x: [1., 0., 0.], normal: [0., -1., 0.] };
    Shape::face(&plano, &[perfil]).and_then(|f| f.revolve([0.; 3], [0., 0., 1.], 360., 0.)).map_err(k)
}

/// A planificada: setor de coroa na linha neutra, em XY, com a espessura em Z.
pub fn corpo_plano(c: &Cone, t: f64) -> Result<Shape, String> {
    let k = |e: fenkra_kernel::KernelError| e.0;
    let p = |rho: f64, a: f64| [rho * a.cos(), rho * a.sin()];
    let th = c.theta;
    let [r0, r1] = c.rho;
    let perfil: Loop2 = vec![
        Seg2::Arc(p(r0, 0.), p(r0, th / 2.), p(r0, th)),
        Seg2::Line(p(r0, th), p(r1, th)),
        Seg2::Arc(p(r1, th), p(r1, th / 2.), p(r1, 0.)),
        Seg2::Line(p(r1, 0.), p(r0, 0.)),
    ];
    let plano = Frame { origin: [0.; 3], x: [1., 0., 0.], normal: [0., 0., 1.] };
    Shape::face(&plano, &[perfil]).and_then(|f| f.extrude([0., 0., 1.], t, 0.)).map_err(k)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{PI, TAU};

    #[test]
    fn desenvolvimento_analitico() {
        // cone de 30° de semiângulo, geratriz de 40: boca menor 20, maior 40, altura = 40·cos30°
        let (r1, r2) = (20., 40.);
        let h = (r2 - r1) / 30f64.to_radians().tan();
        let c = desenvolve(r1, r2, h, 2., 0.5).unwrap();
        assert!((c.alfa - 30f64.to_radians()).abs() < 1e-12);
        assert!((c.geratriz - 40.).abs() < 1e-9);
        // o setor abre 2π·sin 30° = π e o arco neutro bate com o perímetro neutro do cone
        assert!((c.theta - PI).abs() < 1e-12);
        for i in 0..2 {
            assert!((c.arco(i) - TAU * c.r_neutro[i]).abs() < 1e-9, "arco {i}");
        }
        // diferença dos raios do setor = geratriz
        assert!((c.rho[1] - c.rho[0] - c.geratriz).abs() < 1e-9);
        // a linha neutra sai da face interna na normal: K = 0 não desloca
        let c0 = desenvolve(r1, r2, h, 2., 0.).unwrap();
        assert_eq!(c0.r_neutro, [r1, r2]);
    }

    #[test]
    fn entradas_invalidas_dao_erro() {
        assert!(desenvolve(20., 20., 30., 2., 0.44).is_err(), "cilindro não é cone");
        assert!(desenvolve(20., 10., 30., 2., 0.44).is_err(), "boca menor maior que a maior");
        assert!(desenvolve(0., 10., 30., 2., 0.44).is_err());
        assert!(desenvolve(20., 40., 30., 2., 1.5).is_err());
    }

    #[test]
    fn tronco_dobrado_e_setor_plano_tem_o_mesmo_volume_e_a_caixa_certa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (r1, r2, h, t) = (20., 40., 50., 2.);
        // K = 0,5: a linha neutra passa pelo meio da parede, que é onde o volume girado a mede
        let c = desenvolve(r1, r2, h, t, 0.5).unwrap();
        let dobrado = corpo_dobrado(r1, r2, h, t).unwrap();
        let plano = corpo_plano(&c, t).unwrap();
        let (vd, vp) = (dobrado.mass().volume, plano.mass().volume);
        assert!((vd - vp).abs() < 0.002 * vp, "dobrado {vd} contra plano {vp}");
        assert!((vp - c.area() * t).abs() < 0.002 * vp, "volume do setor = área × t");
        // caixa do dobrado: diâmetro externo da boca maior e altura (a parede inclinada sobe um pouco)
        let (lo, hi) = dobrado.bbox().unwrap();
        let sa = (r2 - r1) / (r2 - r1).hypot(h);
        let r_max = r2 + t * (1. - sa * sa).sqrt();
        assert!((hi[0] - r_max).abs() < 0.05 && (lo[0] + r_max).abs() < 0.05, "x {lo:?} {hi:?}");
        assert!((lo[2] + t * sa).abs() < 0.05 && (hi[2] - h).abs() < 0.05, "z {lo:?} {hi:?}");
        // caixa do plano: o setor (θ < π aqui) fica entre o raio externo e a espessura
        let (plo, phi) = plano.bbox().unwrap();
        assert!((phi[2] - t).abs() < 1e-6 && plo[2].abs() < 1e-6);
        assert!(phi[0].max(phi[1]) <= c.rho[1] + 1e-6 && phi[0].max(phi[1]) >= c.rho[1] * 0.99);
    }
}
