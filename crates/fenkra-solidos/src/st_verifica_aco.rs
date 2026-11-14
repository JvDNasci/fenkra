// SPDX-License-Identifier: LGPL-2.1-or-later
//! Verificação de barras de aço pela NBR 8800:2008 (B115), sobre os esforços da análise de pórtico
//! (`st_portico`, B114).
//!
//! Como funciona: para cada barra, a resistência de cálculo vem da seção e do aço, e a utilização é
//! o esforço solicitante dividido pela resistência (1,0 = no limite).
//! * Tração: `Nt,Rd = A·fy / γa1`, com γa1 = 1,10 (escoamento da seção bruta).
//! * Compressão: `Nc,Rd = χ·Q·A·fy / γa1`, com `λ0 = √(Q·A·fy / Ne)`, `Ne = π²·E·I / (K·L)²` no eixo
//!   de menor inércia, `χ = 0,658^(λ0²)` até λ0 = 1,5 e `χ = 0,877 / λ0²` acima.
//! * Flambagem local na compressão (anexo F): `Q = Qs·Qa`. Elementos apoiados nas duas bordas (alma
//!   do I, paredes do tubo retangular) perdem largura: `bef = 1,92·t·√(E/σ)·[1 − ca/(b/t)·√(E/σ)] ≤ b`,
//!   com σ = fy (o lado seguro que a norma permite), `Qa = Aef/Ag` e `Aef = Ag − Σ(b − bef)·t`. As
//!   mesas do I são tratadas como de perfil soldado (`kc = 4/√(h/tw)` entre 0,35 e 0,76;
//!   `Qs = 1,415 − 0,65·(b/t)·√(fy/(kc·E))` até `1,17·√(E/(fy/kc))` e `0,90·E·kc/(fy·(b/t)²)` acima), o
//!   que também cobre o laminado pelo lado seguro. Tubo redondo: `Q = 0,038·E/(fy·D/t) + 2/3` entre
//!   `0,11·E/fy` e `0,45·E/fy`; acima disso a norma não prevê a seção e a barra não passa.
//! * Flexão no eixo de maior inércia do I, com flambagem lateral com torção (FLT, anexo G):
//!   `λ = Lb/ry`, `λp = 1,76·√(E/fy)`, `β1 = (fy − σr)·W/(E·J)` com σr = 0,3·fy,
//!   `λr = 1,38·√(Iy·J)/(ry·J·β1)·√(1 + √(1 + 27·Cw·β1²/Iy))`, `Mr = (fy − σr)·W`, `Mpl = Z·fy`.
//!   Até λp vale Mpl; entre λp e λr, `Cb·[Mpl − (Mpl − Mr)·(λ − λp)/(λr − λp)]`; acima de λr,
//!   `Mcr = Cb·π²·E·Iy/Lb²·√(Cw/Iy·(1 + 0,039·J·Lb²/Cw))`; sempre até Mpl, dividido por γa1.
//!   `Cb = 12,5·Mmax/(2,5·Mmax + 3·MA + 4·MB + 3·MC) ≤ 3,0`, com MA, MB e MC a 1/4, 1/2 e 3/4 do
//!   trecho; 1,0 em balanço. `Cw = Iy·(d − tf)²/4`; `J = (2·b·tf³ + hw·tw³)/3` (parede fina).
//!   Lb é o comprimento da barra (sem travamento lateral no meio dela).
//! * Flexão nos outros casos: `MRd = Z·fy / γa1` (seção compacta).
//! * Cortante: `VRd = 0,6·fy·Aw / γa1`, com a área de alma da seção.
//! * Flexo-compressão (ou tração): `N/NRd ≥ 0,2 → N/NRd + 8/9·(My/MyRd + Mz/MzRd)`, senão
//!   `N/(2·NRd) + (My/MyRd + Mz/MzRd)`.
//!
//! Limites, ditos no relatório: só tubos redondos, tubos retangulares e perfis I duplamente simétricos;
//! sem flambagem local na flexão (FLM, FLA), sem torção combinada e com K = 1 (comprimento de
//! flambagem igual ao da barra) quando não se diz outro. EN 1993 e AISC ficam para depois.
//! Unidades: N, mm, MPa. Fontes conferidas em `docs/arquitetura/procedencia.md`.

use crate::st_portico::{Portico, Resultado, ResultadoBarra};
use std::f64::consts::PI;

pub const GAMA_A1: f64 = 1.10;

/// Dados da flambagem lateral com torção de um I duplamente simétrico (mm, mm⁴, mm⁶, mm³).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flt {
    /// raio de giração no eixo de menor inércia
    pub ry: f64,
    /// constante de torção
    pub j: f64,
    /// constante de empenamento
    pub cw: f64,
    /// módulo elástico no eixo de maior inércia
    pub w: f64,
}

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
    /// fator de flambagem local da compressão; `None` quando a norma não prevê a seção
    pub q: Option<f64>,
    /// FLT na flexão em `mz` (só o I)
    pub flt: Option<Flt>,
}

/// Largura efetiva de um elemento apoiado nas duas bordas (anexo F), com σ = fy.
pub fn largura_efetiva(b: f64, t: f64, e: f64, sigma: f64, ca: f64) -> f64 {
    let r = (e / sigma).sqrt();
    (1.92 * t * r * (1. - ca / (b / t) * r)).min(b)
}

/// Coeficiente kc da mesa de perfil soldado: 4/√(h/tw), entre 0,35 e 0,76.
pub fn kc(hw: f64, tw: f64) -> f64 {
    (4. / (hw / tw).sqrt()).clamp(0.35, 0.76)
}

/// Qs da mesa de perfil I soldado (elemento apoiado numa borda só), com b/t = (bf/2)/tf.
pub fn qs_mesa_soldada(bt: f64, kc: f64, e: f64, fy: f64) -> f64 {
    let r = (e / (fy / kc)).sqrt();
    if bt <= 0.64 * r {
        1.
    } else if bt <= 1.17 * r {
        1.415 - 0.65 * bt * (fy / (kc * e)).sqrt()
    } else {
        0.90 * e * kc / (fy * bt * bt)
    }
}

/// Q do tubo redondo pela relação D/t; `None` acima de 0,45·E/fy.
pub fn q_tubo_redondo(dt: f64, e: f64, fy: f64) -> Option<f64> {
    if dt <= 0.11 * e / fy {
        Some(1.)
    } else if dt <= 0.45 * e / fy {
        Some(0.038 * e / (fy * dt) + 2. / 3.)
    } else {
        None
    }
}

/// Tubo redondo D × t. Limite local da compressão: D/t ≤ 0,11·E/fy.
pub fn tubo_redondo(d: f64, t: f64, e: f64, fy: f64) -> SecaoAco {
    let di = d - 2. * t;
    let area = PI / 4. * (d * d - di * di);
    let i = PI / 64. * (d.powi(4) - di.powi(4));
    let z = (d.powi(3) - di.powi(3)) / 6.;
    SecaoAco { area, iy: i, iz: i, zy: z, zz: z, awy: area / 2., awz: area / 2., esbeltez_local: d / t, limite_local: 0.11 * e / fy, q: q_tubo_redondo(d / t, e, fy), flt: None }
}

/// Tubo retangular b × h × t; `h` é a altura, que trabalha na flexão de `iz`. Limite local das
/// paredes: b/t ≤ 1,40·√(E/fy), com b = largura plana (lado − 3t). Parede esbelta perde largura
/// com ca = 0,38.
pub fn tubo_retangular(b: f64, h: f64, t: f64, e: f64, fy: f64) -> SecaoAco {
    let (bi, hi) = (b - 2. * t, h - 2. * t);
    let area = b * h - bi * hi;
    let iz = (b * h.powi(3) - bi * hi.powi(3)) / 12.;
    let iy = (h * b.powi(3) - hi * bi.powi(3)) / 12.;
    let zz = (b * h * h - bi * hi * hi) / 4.;
    let zy = (h * b * b - hi * bi * bi) / 4.;
    let plano = (b.max(h) - 3. * t) / t;
    let lim = 1.40 * (e / fy).sqrt();
    // duas paredes de cada lado
    let mut aef = area;
    for lado in [b, h] {
        let bp = lado - 3. * t;
        if bp / t > lim {
            aef -= 2. * (bp - largura_efetiva(bp, t, e, fy, 0.38)) * t;
        }
    }
    SecaoAco { area, iy, iz, zy, zz, awy: 2. * h * t, awz: 2. * b * t, esbeltez_local: plano, limite_local: lim, q: Some(aef / area), flt: None }
}

/// Perfil I duplamente simétrico (h, b, tw, tf). Limite local: alma h/tw ≤ 1,49·√(E/fy) na
/// compressão; mesa b/2tf ≤ 0,64·√(E/(fy/kc)) (perfil soldado). Vale o pior dos dois.
pub fn perfil_i(h: f64, b: f64, tw: f64, tf: f64, e: f64, fy: f64) -> SecaoAco {
    let hw = h - 2. * tf;
    let area = 2. * b * tf + hw * tw;
    let iz = (b * h.powi(3) - (b - tw) * hw.powi(3)) / 12.;
    let iy = (2. * tf * b.powi(3) + hw * tw.powi(3)) / 12.;
    let zz = b * tf * (h - tf) + tw * hw * hw / 4.;
    let zy = tf * b * b / 2. + hw * tw * tw / 4.;
    let r = (e / fy).sqrt();
    let k = kc(hw, tw);
    let lim_alma = 1.49 * r;
    let lim_mesa = 0.64 * (e / (fy / k)).sqrt();
    let (alma, mesa) = ((hw / tw) / lim_alma, (b / (2. * tf)) / lim_mesa);
    let (esb, lim) = if alma >= mesa { (hw / tw, lim_alma) } else { (b / (2. * tf), lim_mesa) };
    let aef = if hw / tw > lim_alma { area - (hw - largura_efetiva(hw, tw, e, fy, 0.34)) * tw } else { area };
    let q = qs_mesa_soldada(b / (2. * tf), k, e, fy) * aef / area;
    let flt = Flt { ry: (iy / area).sqrt(), j: (2. * b * tf.powi(3) + hw * tw.powi(3)) / 3., cw: iy * (h - tf).powi(2) / 4., w: iz / (h / 2.) };
    SecaoAco { area, iy, iz, zy, zz, awy: h * tw, awz: 2. * b * tf, esbeltez_local: esb, limite_local: lim, q: Some(q), flt: Some(flt) }
}

/// Fator de redução da compressão (NBR 8800, 5.3.3).
pub fn chi(lambda0: f64) -> f64 {
    if lambda0 <= 1.5 {
        0.658f64.powf(lambda0 * lambda0)
    } else {
        0.877 / (lambda0 * lambda0)
    }
}

/// Fator do diagrama de momento não uniforme, pelos módulos do momento máximo e dos momentos a 1/4,
/// 1/2 e 3/4 do trecho destravado.
pub fn cb(mmax: f64, ma: f64, mb: f64, mc: f64) -> f64 {
    let den = 2.5 * mmax + 3. * ma + 4. * mb + 3. * mc;
    if den <= 0. {
        1.
    } else {
        (12.5 * mmax / den).min(3.)
    }
}

/// Resultado da FLT: momento resistente de cálculo e os parâmetros de esbeltez.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResultadoFlt {
    pub m_rd: f64,
    pub lambda: f64,
    pub lambda_p: f64,
    pub lambda_r: f64,
}

/// Momento resistente de cálculo à FLT do I duplamente simétrico (anexo G), limitado a Mpl/γa1.
pub fn m_rd_flt(f: &Flt, iy: f64, z: f64, lb: f64, cb: f64, e: f64, fy: f64) -> ResultadoFlt {
    let mpl = z * fy;
    let sr = 0.3 * fy;
    let mr = (fy - sr) * f.w;
    let lambda = lb / f.ry;
    let lambda_p = 1.76 * (e / fy).sqrt();
    let b1 = (fy - sr) * f.w / (e * f.j);
    let lambda_r = 1.38 * (iy * f.j).sqrt() / (f.ry * f.j * b1) * (1. + (1. + 27. * f.cw * b1 * b1 / iy).sqrt()).sqrt();
    let mrk = if lambda <= lambda_p {
        mpl
    } else if lambda <= lambda_r {
        cb * (mpl - (mpl - mr) * (lambda - lambda_p) / (lambda_r - lambda_p))
    } else {
        cb * PI * PI * e * iy / (lb * lb) * (f.cw / iy * (1. + 0.039 * f.j * lb * lb / f.cw)).sqrt()
    };
    ResultadoFlt { m_rd: mrk.min(mpl) / GAMA_A1, lambda, lambda_p, lambda_r }
}

/// Módulo de `mz` na posição `x` da barra, interpolado no diagrama.
fn mz_em(rb: &ResultadoBarra, x: f64) -> f64 {
    let d = &rb.diagrama;
    if d.is_empty() {
        return 0.;
    }
    for w in d.windows(2) {
        if x >= w[0].x && x <= w[1].x {
            let t = if w[1].x - w[0].x > 1e-12 { (x - w[0].x) / (w[1].x - w[0].x) } else { 0. };
            return (w[0].mz + t * (w[1].mz - w[0].mz)).abs();
        }
    }
    let mais_perto = d.iter().min_by(|a, b| (a.x - x).abs().total_cmp(&(b.x - x).abs())).map(|q| q.mz).unwrap_or(0.);
    mais_perto.abs()
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
    /// fator de flambagem local usado na compressão
    pub q: f64,
    /// FLT na flexão em `mz` (só o I) e o Cb usado
    pub flt: Option<ResultadoFlt>,
    pub cb: f64,
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
        let q = match s.q {
            Some(q) => {
                if q < 1. - 1e-9 {
                    avisos.push(format!("seção esbelta: {:.1} passa do limite {:.1}; flambagem local na compressão com Q = {q:.3}", s.esbeltez_local, s.limite_local));
                }
                q
            }
            None => {
                avisos.push(format!("seção esbelta demais: {:.1} passa do que a norma prevê; a barra não pode ser usada comprimida", s.esbeltez_local));
                0.
            }
        };
        // esforço axial mais desfavorável: compressão manda se houver
        let n_min = rb.diagrama.iter().map(|x| x.n).fold(f64::MAX, f64::min);
        let n_max = rb.diagrama.iter().map(|x| x.n).fold(f64::MIN, f64::max);
        let l = rb.comprimento * k;
        let i_min = s.iy.min(s.iz);
        let ne = PI * PI * e * i_min / (l * l);
        let lambda0 = (q * s.area * fy / ne).sqrt();
        let x = chi(lambda0);
        let nc_rd = x * q * s.area * fy / GAMA_A1;
        let nt_rd = s.area * fy / GAMA_A1;
        let comprime = n_min < 0. && -n_min >= n_max;
        let (n_sd, n_rd) = if comprime { (-n_min, nc_rd) } else { (n_max.max(0.), nt_rd) };
        let my_rd = s.zy * fy / GAMA_A1;
        // FLT: o trecho destravado é a barra; em balanço (ponta livre que só esta barra toca) Cb = 1
        let lb = rb.comprimento;
        let livre = |n: usize| !p.nos[n].apoio.iter().any(|a| *a) && p.barras.iter().filter(|o| o.i == n || o.j == n).count() == 1;
        let c_b = if livre(b.i) || livre(b.j) { 1. } else { cb(rb.maximo(|q| q.mz), mz_em(rb, lb / 4.), mz_em(rb, lb / 2.), mz_em(rb, 3. * lb / 4.)) };
        let flt = s.flt.map(|f| m_rd_flt(&f, s.iy, s.zz, lb, c_b, e, fy));
        let mz_rd = flt.map(|f| f.m_rd).unwrap_or(s.zz * fy / GAMA_A1);
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
        // sem Q a seção comprimida não tem resistência; tracionada, ainda vale o escoamento
        let utilizacao = if s.q.is_none() && comprime { f64::INFINITY } else { inter.max(cortante).max(n_sd / n_rd) };
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
            q,
            flt,
            cb: c_b,
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
    t += "Q pelo anexo F; FLT do I pelo anexo G com Lb = barra; sem FLM e FLA na flexão; K dado. Não substitui o cálculo de um engenheiro.\n";
    for x in v {
        let estado = if x.utilizacao <= 1. { "ok" } else { "NÃO PASSA" };
        t += &format!(
            "barra {}: {estado} · utilização {:.2} · N {:.0}/{:.0} N (λ0 {:.2}, χ {:.3}, Q {:.3}) · My {:.0}/{:.0} · Mz {:.0}/{:.0} N·mm · V {:.0}/{:.0} N\n",
            x.barra + 1,
            x.utilizacao,
            x.n_sd,
            x.n_rd,
            x.lambda0,
            x.chi,
            x.q,
            x.my_sd,
            x.my_rd,
            x.mz_sd,
            x.mz_rd,
            x.v_sd,
            x.v_rd
        );
        if let Some(f) = x.flt {
            let regime = if f.lambda <= f.lambda_p {
                "plástico"
            } else if f.lambda <= f.lambda_r {
                "inelástico"
            } else {
                "elástico"
            };
            t += &format!("  FLT: λ {:.1} (λp {:.1}, λr {:.1}), {regime}, Cb {:.2}\n", f.lambda, f.lambda_p, f.lambda_r, x.cb);
        }
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
        assert_eq!(v.q, 1.);
    }

    /// Viga biapoiada de perfil I curta (travada pela própria esbeltez baixa) com carga uniforme: a
    /// utilização é M/MRd com o módulo plástico (λ ≤ λp na FLT).
    #[test]
    fn viga_em_flexao_usa_o_modulo_plastico() {
        let s = perfil_i(200., 100., 5.6, 8.5, E, FY);
        let l = 1000.;
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
        // ry = √(Iy/A) ≈ 22,1 mm; λ = 1000/22,1 ≈ 45 < λp = 1,76·√(800) = 49,8
        let f = v.flt.unwrap();
        assert!(f.lambda < f.lambda_p, "{f:?}");
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

    /// Exemplos publicados, refeitos: largura efetiva da alma 625 × 8 com fy = 300 MPa dá 352,0 mm e
    /// Qs da mesa com b/t = 12 e kc = 4/√78,13 dá 0,96 (TCC da UFMG citado na procedência); o Q do
    /// tubo redondo emenda com 1 no limite 0,11·E/fy.
    #[test]
    fn q_confere_com_os_exemplos() {
        assert!((largura_efetiva(625., 8., E, 300., 0.34) - 352.0).abs() < 0.2, "{}", largura_efetiva(625., 8., E, 300., 0.34));
        let k = kc(625., 8.);
        assert!((k - 0.4525).abs() < 1e-3);
        assert!((qs_mesa_soldada(12., k, E, 300.) - 0.96).abs() < 0.006, "{}", qs_mesa_soldada(12., k, E, 300.));
        // continuidade das faixas
        let r = (E / (300. / k)).sqrt();
        assert!((qs_mesa_soldada(0.64 * r + 1e-9, k, E, 300.) - 1.).abs() < 0.002);
        assert!((qs_mesa_soldada(1.17 * r - 1e-9, k, E, 300.) - qs_mesa_soldada(1.17 * r + 1e-9, k, E, 300.)).abs() < 0.004);
        assert!((q_tubo_redondo(0.11 * E / FY + 1e-9, E, FY).unwrap() - 1.).abs() < 0.013);
        // D/t = 200 com fy 250: 0,038·800/200 + 2/3
        assert!(perto(q_tubo_redondo(200., E, FY).unwrap(), 0.038 * 800. / 200. + 2. / 3., 1e-12));
        assert!(q_tubo_redondo(400., E, FY).is_none());
    }

    /// Tubo quadrado 200×200×2 (b/t plano = 97 > 39,6): conta à mão das quatro paredes.
    #[test]
    fn tubo_de_parede_fina_perde_area_efetiva() {
        let s = tubo_retangular(200., 200., 2., E, FY);
        let r = (E / FY).sqrt();
        let bp: f64 = 194.;
        let bef = 1.92 * 2. * r * (1. - 0.38 / (bp / 2.) * r);
        assert!((bef - 96.58).abs() < 0.05, "{bef}");
        let ag: f64 = 200. * 200. - 196. * 196.;
        let qa = (ag - 4. * (bp - bef) * 2.) / ag;
        assert!(perto(s.q.unwrap(), qa, 1e-12));
        assert!((qa - 0.508).abs() < 0.002, "{qa}");
        // no pilar, a resistência cai com Q: λ0 e Nc,Rd com Q·A·fy
        let mut topo = No::livre([0., 0., 3000.]);
        topo.apoio = [true, true, false, true, false, false];
        topo.carga[2] = -50_000.;
        let mut base = No::rotula([0., 0., 0.]);
        base.apoio[5] = true;
        let sec = Secao { e: E, g: 77_000., area: s.area, iy: s.iy, iz: s.iz, j: 2. * s.iy };
        let p = crate::st_portico::Portico { nos: vec![base, topo], barras: vec![Barra { i: 0, j: 1, secao: sec, ref_y: None, q: [0.; 3] }] };
        let res = analisar(&p).unwrap();
        let v = &verificar(&p, &res, &[s], FY, 1.)[0];
        let ne = PI * PI * E * s.iy / 3000f64.powi(2);
        let l0 = (qa * ag * FY / ne).sqrt();
        assert!(perto(v.lambda0, l0, 1e-9));
        assert!(perto(v.n_rd, chi(l0) * qa * ag * FY / 1.1, 1e-9));
        assert!(v.avisos.iter().any(|a| a.contains("Q = 0.508")), "{:?}", v.avisos);
    }

    /// Exemplo publicado de FLT (apostila da UFPR, cap. 5, ex. 5.2: perfil laminado 550×106, Lb =
    /// 2,5 m, A36), com as propriedades de tabela dele: λr = 158,1; com Cb = 1,09 a reta dá
    /// 672,99 kN·m, acima de Mpl/γa1 = 631,82 kN·m (vale o limite); com Cb = 1, 617,4 kN·m.
    #[test]
    fn flt_confere_com_o_exemplo_publicado() {
        let f = Flt { ry: 44.5, j: 124e4, cw: 1_884_000e6, w: 2440e3 };
        let (iy, z) = (2670e4, 2780e3);
        let r = m_rd_flt(&f, iy, z, 2500., 1.09, E, FY);
        assert!((r.lambda_p - 49.78).abs() < 0.01, "{r:?}");
        assert!((r.lambda_r - 158.1).abs() < 0.2, "{r:?}");
        assert!((r.m_rd / 1e6 - 631.82).abs() < 0.01, "{r:?}");
        let r1 = m_rd_flt(&f, iy, z, 2500., 1., E, FY);
        let a_mao = (695. - (695. - 427.) * (2500. / 44.5 - 49.78) / (r.lambda_r - 49.78)) / 1.1;
        assert!((r1.m_rd / 1e6 - a_mao).abs() < 0.01, "{} {a_mao}", r1.m_rd / 1e6);
        assert!((r1.m_rd / 1e6 - 617.4).abs() < 0.3);
        // Cb do mesmo exemplo: carga no meio do vão de 7,5 m, trecho do meio de 2,5 m
        assert!((cb(562.5, 468.75, 562.5, 468.75) - 1.087).abs() < 0.001);
    }

    /// Viga biapoiada longa de I com carga uniforme: Cb = 12,5/11 e FLT em regime elástico,
    /// conferidos à mão com as fórmulas do anexo G.
    #[test]
    fn viga_longa_cai_em_flt_elastica() {
        let (h, bf, tw, tf) = (200., 100., 5.6, 8.5);
        let s = perfil_i(h, bf, tw, tf, E, FY);
        let l = 8000.;
        let mut a = No::rotula([0., 0., 0.]);
        a.apoio[3] = true;
        let mut b = No::rotula([l, 0., 0.]);
        b.apoio[0] = false;
        let sec = Secao { e: E, g: 77_000., area: s.area, iy: s.iy, iz: s.iz, j: 1e4 };
        let p = crate::st_portico::Portico { nos: vec![a, b], barras: vec![Barra { i: 0, j: 1, secao: sec, ref_y: None, q: [0., 0., -2.] }] };
        let r = analisar(&p).unwrap();
        let v = &verificar(&p, &r, &[s], FY, 1.)[0];
        assert!((v.cb - 12.5 / 11.).abs() < 0.01, "{}", v.cb);
        // propriedades à mão
        let hw = h - 2. * tf;
        let iy = (2. * tf * bf.powi(3) + hw * tw.powi(3)) / 12.;
        let area = 2. * bf * tf + hw * tw;
        let iz = (bf * h.powi(3) - (bf - tw) * hw.powi(3)) / 12.;
        let j = (2. * bf * tf.powi(3) + hw * tw.powi(3)) / 3.;
        let cw = iy * (h - tf).powi(2) / 4.;
        let ry = (iy / area).sqrt();
        let w = iz / (h / 2.);
        let b1 = 0.7 * FY * w / (E * j);
        let lr = 1.38 * (iy * j).sqrt() / (ry * j * b1) * (1. + (1. + 27. * cw * b1 * b1 / iy).sqrt()).sqrt();
        assert!(l / ry > lr, "λ {} λr {lr}", l / ry);
        let mcr = v.cb * PI * PI * E * iy / (l * l) * (cw / iy * (1. + 0.039 * j * l * l / cw)).sqrt();
        assert!(perto(v.mz_rd, mcr / 1.1, 1e-9), "{} {}", v.mz_rd, mcr / 1.1);
        assert!(v.mz_rd < s.zz * FY / 1.1);
        assert!(perto(v.interacao, 2. * l * l / 8. / v.mz_rd, 1e-6));
        assert!(relatorio(&[v.clone()], FY).contains("elástico"));
    }
}
