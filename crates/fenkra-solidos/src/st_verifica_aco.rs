// SPDX-License-Identifier: LGPL-2.1-or-later
//! Verificação de barras de aço pela NBR 8800:2008 (B115), sobre os esforços da análise de pórtico
//! (`st_portico`, B114).
//!
//! Como funciona: para cada barra, a resistência de cálculo vem da seção e do aço, e a utilização é
//! o esforço solicitante dividido pela resistência (1,0 = no limite).
//! * Tração: `Nt,Rd = A·fy / γa1`, com γa1 = 1,10 (escoamento da seção bruta).
//! * Compressão: `Nc,Rd = χ·Q·A·fy / γa1`, com `λ0 = √(Q·A·fy / Ne)`, `Ne = π²·E·I / (K·L)²` no eixo
//!   de menor inércia, `χ = 0,658^(λ0²)` até λ0 = 1,5 e `χ = 0,877 / λ0²` acima. Q = 1 (seção sem
//!   flambagem local): a seção que passa do limite de esbeltez local é avisada, não calculada.
//! * Flexão: `MRd = Z·fy / γa1` (seção compacta, sem flambagem lateral com torção).
//! * Cortante: `VRd = 0,6·fy·Aw / γa1`, com a área de alma da seção.
//! * Flexo-compressão (ou tração): `N/NRd ≥ 0,2 → N/NRd + 8/9·(My/MyRd + Mz/MzRd)`, senão
//!   `N/(2·NRd) + (My/MyRd + Mz/MzRd)`.
//!
//! Limites, ditos no relatório: só tubos redondos, tubos retangulares e perfis I duplamente simétricos;
//! sem flambagem lateral com torção, sem flambagem local (Q < 1), sem torção combinada e com K = 1
//! (comprimento de flambagem igual ao da barra) quando não se diz outro. EN 1993 e AISC ficam para
//! depois. Unidades: N, mm, MPa.

use crate::st_portico::{Portico, Resultado};
use std::f64::consts::PI;

pub const GAMA_A1: f64 = 1.10;

/// Dados de resistência de uma seção (mm, mm², mm³).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SecaoAco {
    pub area: f64,
    /// inércias (as mesmas da análise)
    pub iy: f64,
    pub iz: f64,
    /// módulos plásticos
    pub zy: f64,
    pub zz: f64,
    /// área de cisalhamento para Vy e Vz
    pub awy: f64,
    pub awz: f64,
    /// esbeltez local da parte mais esbelta (b/t ou D/t) e o limite dela na compressão
    pub esbeltez_local: f64,
    pub limite_local: f64,
}

/// Tubo redondo D × t. Limite local da compressão: D/t ≤ 0,11·E/fy.
pub fn tubo_redondo(d: f64, t: f64, e: f64, fy: f64) -> SecaoAco {
    let di = d - 2. * t;
    let area = PI / 4. * (d * d - di * di);
    let i = PI / 64. * (d.powi(4) - di.powi(4));
    let z = (d.powi(3) - di.powi(3)) / 6.;
    SecaoAco { area, iy: i, iz: i, zy: z, zz: z, awy: area / 2., awz: area / 2., esbeltez_local: d / t, limite_local: 0.11 * e / fy }
}

/// Tubo retangular b × h × t; `h` é a altura, que trabalha na flexão de `iz`. Limite local das
/// paredes: b/t ≤ 1,40·√(E/fy), com b = largura plana (lado − 3t).
pub fn tubo_retangular(b: f64, h: f64, t: f64, e: f64, fy: f64) -> SecaoAco {
    let (bi, hi) = (b - 2. * t, h - 2. * t);
    let area = b * h - bi * hi;
    let iz = (b * h.powi(3) - bi * hi.powi(3)) / 12.;
    let iy = (h * b.powi(3) - hi * bi.powi(3)) / 12.;
    let zz = (b * h * h - bi * hi * hi) / 4.;
    let zy = (h * b * b - hi * bi * bi) / 4.;
    let plano = (b.max(h) - 3. * t) / t;
    SecaoAco { area, iy, iz, zy, zz, awy: 2. * h * t, awz: 2. * b * t, esbeltez_local: plano, limite_local: 1.40 * (e / fy).sqrt() }
}

/// Perfil I duplamente simétrico (h, b, tw, tf). Limite local: alma h/tw ≤ 1,49·√(E/fy) na
/// compressão (o da mesa, b/2tf ≤ 0,56·√(E/fy), também é conferido: vale o pior).
pub fn perfil_i(h: f64, b: f64, tw: f64, tf: f64, e: f64, fy: f64) -> SecaoAco {
    let hw = h - 2. * tf;
    let area = 2. * b * tf + hw * tw;
    let iz = (b * h.powi(3) - (b - tw) * hw.powi(3)) / 12.;
    let iy = (2. * tf * b.powi(3) + hw * tw.powi(3)) / 12.;
    let zz = b * tf * (h - tf) + tw * hw * hw / 4.;
    let zy = tf * b * b / 2. + hw * tw * tw / 4.;
    let r = (e / fy).sqrt();
    let (alma, mesa) = ((hw / tw) / (1.49 * r), (b / (2. * tf)) / (0.56 * r));
    let (esb, lim) = if alma >= mesa { (hw / tw, 1.49 * r) } else { (b / (2. * tf), 0.56 * r) };
    SecaoAco { area, iy, iz, zy, zz, awy: h * tw, awz: 2. * b * tf, esbeltez_local: esb, limite_local: lim }
}

/// Fator de redução da compressão (NBR 8800, 5.3.3).
pub fn chi(lambda0: f64) -> f64 {
    if lambda0 <= 1.5 {
        0.658f64.powf(lambda0 * lambda0)
    } else {
        0.877 / (lambda0 * lambda0)
    }
}

/// Utilizações de uma barra (0..1 cabe).
#[derive(Clone, Debug, PartialEq)]
pub struct Verificacao {
    pub barra: usize,
    pub n_sd: f64,
    pub n_rd: f64,
    pub my_sd: f64,
    pub mz_sd: f64,
    pub my_rd: f64,
    pub mz_rd: f64,
    pub v_sd: f64,
    pub v_rd: f64,
    pub lambda0: f64,
    pub chi: f64,
    /// interação N + M
    pub interacao: f64,
    pub cortante: f64,
    /// a maior das utilizações
    pub utilizacao: f64,
    pub avisos: Vec<String>,
}

/// Verifica cada barra com o aço `fy` (MPa) e o coeficiente de flambagem `k` (1 = biarticulada).
/// `secoes[i]` é a seção da barra `i` do pórtico.
pub fn verificar(p: &Portico, r: &Resultado, secoes: &[SecaoAco], fy: f64, k: f64) -> Vec<Verificacao> {
    let mut out = vec![];
    for (bi, (b, rb)) in p.barras.iter().zip(&r.barras).enumerate() {
        let Some(s) = secoes.get(bi) else { continue };
        let e = b.secao.e;
        let mut avisos = vec![];
        if s.esbeltez_local > s.limite_local {
            avisos.push(format!("seção esbelta: {:.1} passa do limite {:.1}; a flambagem local (Q < 1) não está calculada", s.esbeltez_local, s.limite_local));
        }
        // esforço axial mais desfavorável: compressão manda se houver
        let n_min = rb.diagrama.iter().map(|x| x.n).fold(f64::MAX, f64::min);
        let n_max = rb.diagrama.iter().map(|x| x.n).fold(f64::MIN, f64::max);
        let l = rb.comprimento * k;
        let i_min = s.iy.min(s.iz);
        let ne = PI * PI * e * i_min / (l * l);
        let lambda0 = (s.area * fy / ne).sqrt();
        let x = chi(lambda0);
        let nc_rd = x * s.area * fy / GAMA_A1;
        let nt_rd = s.area * fy / GAMA_A1;
        let comprime = n_min < 0. && -n_min >= n_max;
        let (n_sd, n_rd) = if comprime { (-n_min, nc_rd) } else { (n_max.max(0.), nt_rd) };
        let my_rd = s.zy * fy / GAMA_A1;
        let mz_rd = s.zz * fy / GAMA_A1;
        // interação ponto a ponto (o pior ponto da barra)
        let mut inter: f64 = 0.;
        for q in &rb.diagrama {
            let n = if comprime { (-q.n).max(0.) } else { q.n.max(0.) };
            let r_n = n / n_rd;
            let m = q.my.abs() / my_rd + q.mz.abs() / mz_rd;
            let v = if r_n >= 0.2 { r_n + 8. / 9. * m } else { r_n / 2. + m };
            inter = inter.max(v);
        }
        let vy = rb.maximo(|q| q.vy);
        let vz = rb.maximo(|q| q.vz);
        let (vy_rd, vz_rd) = (0.6 * fy * s.awy / GAMA_A1, 0.6 * fy * s.awz / GAMA_A1);
        let cortante = (vy / vy_rd).max(vz / vz_rd);
        let (v_sd, v_rd) = if vy / vy_rd >= vz / vz_rd { (vy, vy_rd) } else { (vz, vz_rd) };
        if lambda0 > 0. && k * rb.comprimento / (i_min / s.area).sqrt() > 200. {
            avisos.push(format!("esbeltez KL/r = {:.0} passa de 200 (limite da norma para barra comprimida)", k * rb.comprimento / (i_min / s.area).sqrt()));
        }
        let utilizacao = inter.max(cortante).max(n_sd / n_rd);
        out.push(Verificacao {
            barra: bi,
            n_sd,
            n_rd,
            my_sd: rb.maximo(|q| q.my),
            mz_sd: rb.maximo(|q| q.mz),
            my_rd,
            mz_rd,
            v_sd,
            v_rd,
            lambda0,
            chi: x,
            interacao: inter,
            cortante,
            utilizacao,
            avisos,
        });
    }
    out
}

/// Relatório em texto da verificação.
pub fn relatorio(v: &[Verificacao], fy: f64) -> String {
    let mut t = format!("Verificação de barras de aço · NBR 8800:2008 · fy = {fy:.0} MPa · γa1 = {GAMA_A1:.2}\n");
    t += "Sem flambagem lateral com torção, sem flambagem local (Q = 1), K dado. Não substitui o cálculo de um engenheiro.\n";
    for x in v {
        let estado = if x.utilizacao <= 1. { "ok" } else { "NÃO PASSA" };
        t += &format!(
            "barra {}: {estado} · utilização {:.2} · N {:.0}/{:.0} N (λ0 {:.2}, χ {:.3}) · My {:.0}/{:.0} · Mz {:.0}/{:.0} N·mm · V {:.0}/{:.0} N\n",
            x.barra + 1,
            x.utilizacao,
            x.n_sd,
            x.n_rd,
            x.lambda0,
            x.chi,
            x.my_sd,
            x.my_rd,
            x.mz_sd,
            x.mz_rd,
            x.v_sd,
            x.v_rd
        );
        for a in &x.avisos {
            t += &format!("  aviso: {a}\n");
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::st_portico::{analisar, Barra, No, Secao};

    const E: f64 = 200_000.;
    const FY: f64 = 250.;

    fn perto(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * b.abs()
    }

    #[test]
    fn chi_segue_a_curva_da_norma() {
        assert!(perto(chi(0.), 1., 1e-12));
        assert!(perto(chi(1.), 0.658, 1e-12));
        assert!(perto(chi(1.5), 0.658f64.powf(2.25), 1e-12));
        assert!(perto(chi(2.), 0.877 / 4., 1e-12));
        // as duas expressões quase se encontram em 1,5 (a da norma é contínua a 0,1 %)
        assert!((0.658f64.powf(2.25) - 0.877 / 2.25).abs() < 0.002);
    }

    /// Pilar biarticulado de tubo 100×100×5 com 3 m, 100 kN: confere λ0, χ e a utilização à mão.
    #[test]
    fn pilar_comprimido_confere_com_a_conta_a_mao() {
        let s = tubo_retangular(100., 100., 5., E, FY);
        let mut topo = No::livre([0., 0., 3000.]);
        topo.apoio = [true, true, false, true, false, false];
        topo.carga[2] = -100_000.;
        let mut base = No::rotula([0., 0., 0.]);
        base.apoio[5] = true;
        let sec = Secao { e: E, g: 77_000., area: s.area, iy: s.iy, iz: s.iz, j: 2. * s.iy };
        let p = crate::st_portico::Portico { nos: vec![base, topo], barras: vec![Barra { i: 0, j: 1, secao: sec, ref_y: None, q: [0.; 3] }] };
        let r = analisar(&p).unwrap();
        let v = &verificar(&p, &r, &[s], FY, 1.)[0];
        let ne = PI * PI * E * s.iy / 3000f64.powi(2);
        let l0 = (s.area * FY / ne).sqrt();
        assert!(perto(v.lambda0, l0, 1e-9));
        let nrd = chi(l0) * s.area * FY / 1.1;
        assert!(perto(v.n_rd, nrd, 1e-9));
        assert!(perto(v.utilizacao, 100_000. / nrd, 1e-6), "{} {}", v.utilizacao, 100_000. / nrd);
        assert!(v.avisos.is_empty(), "{:?}", v.avisos);
    }

    /// Viga biapoiada de perfil I com carga uniforme: a utilização é M/MRd (sem força normal).
    #[test]
    fn viga_em_flexao_usa_o_modulo_plastico() {
        let s = perfil_i(200., 100., 5.6, 8.5, E, FY);
        let l = 4000.;
        let q = -10.;
        let mut a = No::rotula([0., 0., 0.]);
        a.apoio[3] = true;
        let mut b = No::rotula([l, 0., 0.]);
        b.apoio[0] = false;
        let sec = Secao { e: E, g: 77_000., area: s.area, iy: s.iy, iz: s.iz, j: 1e4 };
        let p = crate::st_portico::Portico { nos: vec![a, b], barras: vec![Barra { i: 0, j: 1, secao: sec, ref_y: None, q: [0., 0., q] }] };
        let r = analisar(&p).unwrap();
        let v = &verificar(&p, &r, &[s], FY, 1.)[0];
        let m = q.abs() * l * l / 8.;
        assert!(perto(v.mz_sd, m, 1e-9));
        assert!(perto(v.interacao, m / (s.zz * FY / 1.1), 1e-9));
        assert!(relatorio(&[v.clone()], FY).contains("barra 1"));
    }

    #[test]
    fn secao_esbelta_e_avisada() {
        let s = tubo_retangular(200., 200., 2., E, FY);
        assert!(s.esbeltez_local > s.limite_local);
        let s = tubo_redondo(60.3, 3.6, E, FY);
        assert!(s.esbeltez_local < s.limite_local);
        // módulo plástico do tubo redondo: (D³ − d³)/6
        assert!(perto(s.zz, (60.3f64.powi(3) - 53.1f64.powi(3)) / 6., 1e-12));
    }
}
