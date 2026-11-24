// SPDX-License-Identifier: LGPL-2.1-or-later
//! B579: conformação da aba de linha de dobra curva (prensa com borracha, hidroconformação).
//!
//! Como funciona. A aba de linha de dobra reta sai da dobradeira: o desenvolvimento é o da dobra com o fator
//! K (`ba = ângulo · (r + k·t)`), e o material só dobra. Quando a linha de dobra é curva, a aba é uma parede
//! que acompanha a curva, e o desenvolvimento deixa de ser só dobra: a borda livre da parede e a borda
//! livre da faixa plana não têm o mesmo comprimento.
//!
//! Faixa plana: a linha neutra da dobra conserva o comprimento e cada fibra normal à linha de dobra mantém o
//! comprimento (a faixa plana é a curva de dobra deslocada, para fora, por `D = ba + comprimento`, o
//! "desenvolvimento geodésico" da aba). Na peça dobrada a borda livre fica a `x_f` da linha de dobra,
//! medido no plano da web:
//!
//! ```text
//! x_f = (r + k·t)·sen θ + comprimento·cos θ        (θ = ângulo da aba)
//! ```
//!
//! Uma curva paralela a uma curva de curvatura `κ` (positiva quando o centro fica do lado da web, o lado
//! convexo; negativa quando fica do lado da aba, o côncavo) tem o comprimento multiplicado por `1 + κ·d`
//! na distância `d`. O comprimento da borda livre vale então `1 + κ·x_f` na peça dobrada e `1 + κ·D` na
//! faixa plana, e a deformação circunferencial da borda é
//!
//! ```text
//! ε = (1 + κ·x_f) / (1 + κ·D) − 1   ≈ −κ·(D − x_f)
//! ```
//!
//! Convexo (κ > 0): `ε < 0`, flange de contração (a borda se comprime, a chapa espessa e pode enrugar).
//! Côncavo (κ < 0): `ε > 0`, flange de estiramento (a borda estica, a chapa afina e pode trincar). Para uma
//! aba reta de pequena altura `h` num arco de raio `R` dá `ε ≈ −h/R`. Com a constância de volume e o raio
//! da aba fixo, a espessura na borda vira `t' = t / (1 + ε)`.
//!
//! O processo (conformação com borracha ou hidroconformação) é quem faz a borda mudar de comprimento; a
//! análise abaixo só mede quanto, em cada ponto da linha de dobra, e classifica o risco:
//!
//! - **Compressão (contração)**: a borda livre da aba é uma placa fina apoiada na dobra e livre na outra
//!   borda; a deformação crítica de flambagem local dessa placa é `ε_cr = 0,425·π²/(12·(1 − ν²))·(t/h)²`
//!   (tensão crítica clássica de placa com uma borda apoiada e uma livre). Na conformação a borracha (ou a
//!   pressão) escora a aba, e o limite prático sobe por um fator de apoio `ψ` (20 por padrão: aba de 15 mm em chapa de 1,5 mm aguenta cerca de 6 % de compressão). A razão `ρ = |ε| / (ψ·ε_cr)`
//!   dá três faixas: abaixo de 0,5 sem risco, até 1 atenção, de 1 em diante enruga.
//! - **Tração (estiramento)**: `ρ = ε / ε_trinca`, com o limite de alongamento da borda do material
//!   (`ε_trinca`); as mesmas três faixas, e a última é trinca.
//!
//! Onde não há risco o resultado é o da dobradeira; onde há, o aviso diz o trecho, o valor e o que fazer
//! (aba mais baixa, alívios).

use crate::geo3d::{self, V3};

/// Parâmetros do material e do processo da análise. Os números são prudentes e podem ser trocados no
/// painel da chapa (`limite_conformacao` é o alongamento de trinca em %).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limites {
    /// alongamento máximo da borda esticada antes da trinca (fração; alumínio recozido ~ 0,15)
    pub trinca: f64,
    /// fator de apoio do processo contra a flambagem da borda comprimida (borracha/pressão)
    pub apoio: f64,
    /// coeficiente de Poisson
    pub poisson: f64,
}

impl Default for Limites {
    fn default() -> Self {
        Limites { trinca: 0.15, apoio: 20., poisson: 0.33 }
    }
}

/// Dados da aba para a análise.
#[derive(Clone, Copy, Debug)]
pub struct Aba {
    pub t: f64,
    pub r: f64,
    pub k: f64,
    /// ângulo da aba, em graus
    pub ang: f64,
    pub len: f64,
}

impl Aba {
    /// Desenvolvimento da aba: o quanto a faixa plana avança a partir da linha de dobra.
    pub fn desenvolvimento(&self) -> f64 {
        self.ang.to_radians() * (self.r + self.k * self.t) + self.len
    }
    /// Onde a borda livre fica na peça dobrada, medido da linha de dobra no plano da web.
    pub fn alcance_dobrado(&self) -> f64 {
        let a = self.ang.to_radians();
        (self.r + self.k * self.t) * a.sin() + self.len * a.cos()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risco {
    Ok,
    Atencao,
    /// compressão: enruga; tração: trinca
    Falha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Regime {
    Reto,
    Contracao,
    Estiramento,
}

/// A deformação num ponto da linha de dobra.
#[derive(Clone, Copy, Debug)]
pub struct PontoDef {
    pub p: V3,
    /// posição ao longo da linha (mm)
    pub s: f64,
    /// curvatura com sinal (1/mm): positiva no lado convexo
    pub kappa: f64,
    /// deformação circunferencial da borda livre
    pub eps: f64,
    /// espessura final da borda, em relação à inicial (t'/t)
    pub esp: f64,
    pub regime: Regime,
    pub risco: Risco,
    /// razão do critério (≥ 1: falha)
    pub razao: f64,
}

/// Canto vivo da linha de dobra (bordo de fuga agudo): a curvatura não vale ali, e a aba pede alívio de canto.
#[derive(Clone, Copy, Debug)]
pub struct Canto {
    pub p: V3,
    /// quanto a linha vira no canto (graus)
    pub giro: f64,
    pub convexo: bool,
}

/// Giro mínimo, num vértice, para ser canto vivo e não curva.
pub const GIRO_CANTO: f64 = 35.;

#[derive(Clone, Debug)]
pub struct Analise {
    pub pontos: Vec<PontoDef>,
    pub cantos: Vec<Canto>,
    pub aba: Aba,
    pub lim: Limites,
}

/// Deformação da borda livre para a curvatura `kappa`.
pub fn deformacao(kappa: f64, aba: &Aba) -> f64 {
    let (d, x) = (aba.desenvolvimento(), aba.alcance_dobrado());
    (1. + kappa * x) / (1. + kappa * d) - 1.
}

/// Deformação crítica de flambagem local da borda livre de altura `h` (elástica, sem apoio).
pub fn eps_critica(t: f64, h: f64, poisson: f64) -> f64 {
    0.425 * std::f64::consts::PI.powi(2) / (12. * (1. - poisson * poisson)) * (t / h).powi(2)
}

/// Classifica um ponto (`eps` já calculada).
pub fn classifica(eps: f64, aba: &Aba, lim: &Limites) -> (Regime, Risco, f64) {
    if eps.abs() < 1e-9 {
        return (Regime::Reto, Risco::Ok, 0.);
    }
    let faixa = |rho: f64| if rho >= 1. { Risco::Falha } else if rho >= 0.5 { Risco::Atencao } else { Risco::Ok };
    if eps < 0. {
        let h = (aba.len + aba.t).max(1e-6);
        let rho = -eps / (lim.apoio * eps_critica(aba.t, h, lim.poisson));
        (Regime::Contracao, faixa(rho), rho)
    } else {
        let rho = eps / lim.trinca;
        (Regime::Estiramento, faixa(rho), rho)
    }
}

/// Raio com sinal da curva por três pontos: `(1/R, vetor do ponto do meio ao centro)`.
fn curvatura3(a: V3, b: V3, c: V3) -> Option<(f64, V3)> {
    let (ab, ac) = (geo3d::sub(b, a), geo3d::sub(c, a));
    let cr = fenkra_kernel::cross(ab, ac);
    let area2 = geo3d::norm(cr);
    let (la, lb, lc) = (geo3d::norm(ab), geo3d::norm(geo3d::sub(c, b)), geo3d::norm(ac));
    if area2 < 1e-9 * (la * lb * lc).max(1e-12) {
        return None;
    }
    let k = 2. * area2 / (la * lb * lc);
    let t1 = geo3d::mul(fenkra_kernel::cross(cr, ab), geo3d::dot(ac, ac));
    let t2 = geo3d::mul(fenkra_kernel::cross(ac, cr), geo3d::dot(ab, ab));
    let centro = geo3d::add(a, geo3d::mul(geo3d::add(t1, t2), 1. / (2. * area2 * area2)));
    Some((k, geo3d::sub(centro, b)))
}

/// Analisa a aba ao longo da linha de dobra `pts` (polilinha no plano da web de normal `n`), com `outs` o
/// sentido de fora (da web para a aba) em cada ponto. A polilinha pode ser fechada (última = primeira).
pub fn analisar(pts: &[V3], outs: &[V3], n: V3, aba: Aba, lim: Limites) -> Analise {
    let m = pts.len();
    let mut pontos = vec![];
    if m < 3 || outs.len() != m {
        return Analise { pontos, cantos: vec![], aba, lim };
    }
    let fechada = geo3d::dist(pts[0], pts[m - 1]) < 1e-6 * (1. + geo3d::dist(pts[0], pts[m / 2]));
    let mut s = vec![0.];
    for w in pts.windows(2) {
        s.push(s.last().unwrap() + geo3d::dist(w[0], w[1]));
    }
    let total = *s.last().unwrap();
    let passo = total / (m - 1) as f64;
    // janela de três pontos: longa o bastante para não medir o ruído da amostragem
    let h = (3. * passo).max(0.75f64.min(total / 20.)).min(total / 4.);
    let u = if fechada { m - 1 } else { m };
    // cantos vivos: vértices em que a linha vira mais que `GIRO_CANTO` de uma vez
    let mut cantos: Vec<Canto> = vec![];
    for i in 0..m {
        let (a, c) = if fechada {
            (pts[(i + u - 1) % u], pts[(i + 1) % u])
        } else if i == 0 || i == m - 1 {
            continue;
        } else {
            (pts[i - 1], pts[i + 1])
        };
        let (d1, d2) = (geo3d::sub(pts[i], a), geo3d::sub(c, pts[i]));
        let (Some(e1), Some(e2)) = (geo3d::unit(d1), geo3d::unit(d2)) else { continue };
        let giro = geo3d::dot(e1, e2).clamp(-1., 1.).acos().to_degrees();
        if giro > GIRO_CANTO && !(fechada && i == m - 1) {
            let vira = fenkra_kernel::cross(e1, e2);
            // convexo: vira para o lado contrário ao de fora (o centro fica do lado da web)
            let para_dentro = geo3d::dot(fenkra_kernel::cross(n, e1), outs[i]) * geo3d::dot(vira, n);
            cantos.push(Canto { p: pts[i], giro, convexo: para_dentro < 0. || geo3d::dot(vira, n).abs() < 1e-12 });
        }
    }
    let perto_de_canto = |si: f64| -> bool {
        cantos.iter().any(|c| {
            let k = pts.iter().position(|q| geo3d::dist(*q, c.p) < 1e-9).unwrap_or(0);
            let d = (s[k] - si).abs();
            let d = if fechada { d.min(total - d) } else { d };
            d < h
        })
    };
    for i in 0..m {
        if !cantos.is_empty() && perto_de_canto(s[i]) {
            pontos.push(ponto(pts[i], s[i], 0., &aba, &lim));
            continue;
        }
        // vizinhos a uma janela de distância, dando a volta se a linha é fechada
        let busca = |dir: i64| -> Option<usize> {
            let mut j = i as i64;
            let mut acc = 0.;
            for _ in 0..m {
                let nj = j + dir;
                let nj = if fechada { nj.rem_euclid(u as i64) } else if nj < 0 || nj >= m as i64 { return (acc >= h / 3.).then_some(j as usize) } else { nj };
                acc += geo3d::dist(pts[j as usize], pts[nj as usize]);
                j = nj;
                if acc >= h {
                    return Some(j as usize);
                }
            }
            None
        };
        let (Some(a), Some(c)) = (busca(-1), busca(1)) else { continue };
        let Some((k, para_centro)) = curvatura3(pts[a], pts[i], pts[c]) else {
            pontos.push(ponto(pts[i], s[i], 0., &aba, &lim));
            continue;
        };
        let no_plano = geo3d::sub(para_centro, geo3d::mul(n, geo3d::dot(n, para_centro)));
        let sinal = if geo3d::dot(no_plano, outs[i]) > 0. { -1. } else { 1. };
        pontos.push(ponto(pts[i], s[i], sinal * k, &aba, &lim));
    }
    Analise { pontos, cantos, aba, lim }
}

fn ponto(p: V3, s: f64, kappa: f64, aba: &Aba, lim: &Limites) -> PontoDef {
    // o desenvolvimento passa do centro de curvatura: a faixa plana se cruza (a validação da aba por
    // superfície já avisa); a deformação não tem sentido físico, fica no limite
    let d = aba.desenvolvimento();
    let eps = if 1. + kappa * d <= 0.05 { 1. } else { deformacao(kappa, aba) };
    let (regime, risco, razao) = classifica(eps, aba, lim);
    PontoDef { p, s, kappa, eps, esp: 1. / (1. + eps), regime, risco, razao }
}

impl Analise {
    /// Pior contração (a mais negativa) e pior estiramento (o mais positivo).
    pub fn extremos(&self) -> (Option<&PontoDef>, Option<&PontoDef>) {
        let c = self.pontos.iter().filter(|p| p.eps < 0.).min_by(|a, b| a.eps.total_cmp(&b.eps));
        let e = self.pontos.iter().filter(|p| p.eps > 0.).max_by(|a, b| a.eps.total_cmp(&b.eps));
        (c, e)
    }

    pub fn pior_risco(&self) -> Risco {
        if !self.cantos.is_empty() {
            return Risco::Falha;
        }
        self.pontos.iter().map(|p| p.risco).max().unwrap_or(Risco::Ok)
    }

    /// Comprimento da linha de dobra (mm) em cada faixa de risco: `[ok, atenção, falha]`.
    pub fn comprimentos(&self) -> [f64; 3] {
        let mut c = [0.; 3];
        for w in self.pontos.windows(2) {
            let l = geo3d::dist(w[0].p, w[1].p);
            c[w[0].risco as usize] += l;
        }
        c
    }

    /// Maior comprimento de aba (até o atual) que deixa o ponto de pior razão na faixa de atenção.
    pub fn comprimento_seguro(&self) -> Option<f64> {
        let pior = self.pontos.iter().filter(|p| p.risco == Risco::Falha).max_by(|a, b| a.razao.total_cmp(&b.razao))?;
        let (mut lo, mut hi) = (0., self.aba.len);
        for _ in 0..40 {
            let mid = (lo + hi) / 2.;
            let aba = Aba { len: mid, ..self.aba };
            let eps = if 1. + pior.kappa * aba.desenvolvimento() <= 0.05 { 1. } else { deformacao(pior.kappa, &aba) };
            if classifica(eps, &aba, &self.lim).1 == Risco::Falha {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        Some(lo)
    }

    /// Onde o perfil fecha mais (canto vivo ou menor raio de curvatura), para explicar por que a aba não une.
    pub fn dica_falha(&self) -> String {
        if let Some(c) = self.cantos.iter().max_by(|a, b| a.giro.total_cmp(&b.giro)) {
            return format!("; o perfil tem um canto vivo {} de {:.0}° perto de ({:.1}, {:.1}, {:.1}): a aba não contorna um canto tão fechado, arredonde o canto ou divida a aba em dois trechos que terminam nele", if c.convexo { "convexo" } else { "côncavo" }, c.giro, c.p[0], c.p[1], c.p[2]);
        }
        match self.pontos.iter().filter(|p| p.kappa.abs() > 1e-9).max_by(|a, b| a.kappa.abs().total_cmp(&b.kappa.abs())) {
            Some(p) => format!("; o trecho mais fechado do perfil tem raio de curvatura de {:.1} mm perto de ({:.1}, {:.1}, {:.1}): aumente esse raio ou divida a aba nele", 1. / p.kappa.abs(), p.p[0], p.p[1], p.p[2]),
            None => String::new(),
        }
    }

    /// Aviso (não erro) quando algum trecho passa do limite do material; `None` sem risco de falha.
    pub fn aviso(&self) -> Option<String> {
        if self.pior_risco() != Risco::Falha {
            return None;
        }
        let c = self.comprimentos();
        let (contra, estira) = self.extremos();
        let mut partes = vec![];
        if let Some(p) = contra.filter(|p| p.risco == Risco::Falha) {
            partes.push(format!(
                "conformação: a borda livre da aba se comprime até {:.0} % perto de ({:.1}, {:.1}, {:.1}) (curva de raio {:.1} mm) e enruga; a chapa espessa {:.0} % ali",
                -p.eps * 100.,
                p.p[0],
                p.p[1],
                p.p[2],
                1. / p.kappa.abs().max(1e-9),
                (p.esp - 1.) * 100.
            ));
        }
        if let Some(p) = estira.filter(|p| p.risco == Risco::Falha) {
            partes.push(format!(
                "conformação: a borda livre da aba estica {:.0} % perto de ({:.1}, {:.1}, {:.1}) (curva de raio {:.1} mm), acima do limite de {:.0} % do material, e a chapa afina {:.0} %: risco de trinca",
                p.eps * 100.,
                p.p[0],
                p.p[1],
                p.p[2],
                1. / p.kappa.abs().max(1e-9),
                self.lim.trinca * 100.,
                (1. - p.esp) * 100.
            ));
        }
        for cn in self.cantos.iter().take(3) {
            partes.push(format!(
                "canto vivo {} de {:.0}° perto de ({:.1}, {:.1}, {:.1}): as duas paredes se encontram ou deixam vão, abra um alívio de canto",
                if cn.convexo { "convexo" } else { "côncavo" },
                cn.giro,
                cn.p[0],
                cn.p[1],
                cn.p[2]
            ));
        }
        let mut dica = String::new();
        if c[2] > 0. {
            dica = format!("{:.0} mm da linha de dobra passam do limite", c[2]);
            if let Some(h) = self.comprimento_seguro() {
                dica.push_str(&format!("; abaixe a aba nesse trecho para até {h:.1} mm ou abra alívios na borda livre"));
            } else {
                dica.push_str("; abra alívios na borda livre");
            }
        }
        let tudo = partes.join("; ");
        Some(if dica.is_empty() { tudo } else { format!("{tudo}; {dica}") })
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn arco(r: f64, n: usize, fechado: bool) -> Vec<V3> {
        let fim = if fechado { std::f64::consts::TAU } else { std::f64::consts::FRAC_PI_2 };
        (0..=n).map(|i| fim * i as f64 / n as f64).map(|a| [r * a.cos(), r * a.sin(), 0.]).collect()
    }

    fn fora_radial(p: &[V3], sinal: f64) -> Vec<V3> {
        p.iter().map(|q| geo3d::mul(geo3d::unit([q[0], q[1], 0.]).unwrap(), sinal)).collect()
    }

    #[test]
    fn contracao_no_arco_convexo_e_menos_h_sobre_r() {
        // aba de 10 mm num arco de raio 500: ε ≈ −h/R = −2 %
        let aba = Aba { t: 0.01, r: 0.01, k: 0.5, ang: 90., len: 10. };
        let pts = arco(500., 400, false);
        let outs = fora_radial(&pts, 1.);
        let a = analisar(&pts, &outs, [0., 0., 1.], aba, Limites::default());
        let p = &a.pontos[a.pontos.len() / 2];
        assert!(p.regime == Regime::Contracao, "{p:?}");
        assert!((p.kappa - 1. / 500.).abs() < 2e-5, "κ {}", p.kappa);
        // fórmula exata e aproximação −h/R
        let exata = (1. + 0.015 / 500.) / (1. + aba.desenvolvimento() / 500.) - 1.;
        assert!((p.eps - exata).abs() < 2e-5, "ε {} contra {exata}", p.eps);
        assert!((p.eps + 10. / 500.).abs() < 0.02 * 10. / 500. + 1e-4, "ε {} contra −h/R", p.eps);
        // a espessura sobe
        assert!(p.esp > 1.);
    }

    #[test]
    fn estiramento_no_arco_concavo() {
        // a aba sai para o centro do círculo (côncavo): ε > 0, espessura cai
        let aba = Aba { t: 1., r: 1., k: 0.44, ang: 90., len: 10. };
        let pts = arco(60., 200, false);
        let outs = fora_radial(&pts, -1.);
        let a = analisar(&pts, &outs, [0., 0., 1.], aba, Limites::default());
        let p = &a.pontos[a.pontos.len() / 2];
        assert!(p.regime == Regime::Estiramento && p.eps > 0., "{p:?}");
        assert!(p.esp < 1.);
        // a conta direta: a borda livre dobrada fica em R − x_f, a plana em R − D
        let (d, x) = (aba.desenvolvimento(), aba.alcance_dobrado());
        let direta = (60. - x) / (60. - d) - 1.;
        assert!((p.eps - direta).abs() < 2e-3, "{} contra {direta}", p.eps);
    }

    #[test]
    fn linha_reta_nao_deforma_e_fica_na_dobradeira() {
        let aba = Aba { t: 1., r: 1., k: 0.44, ang: 90., len: 20. };
        let pts: Vec<V3> = (0..=50).map(|i| [i as f64 * 4., 0., 0.]).collect();
        let outs = vec![[0., 1., 0.]; pts.len()];
        let a = analisar(&pts, &outs, [0., 0., 1.], aba, Limites::default());
        assert!(a.pontos.iter().all(|p| p.regime == Regime::Reto && p.risco == Risco::Ok), "reta deformou");
        assert!(a.aviso().is_none());
    }

    #[test]
    fn nariz_de_nervura_enruga_e_o_aviso_sugere_aba_mais_baixa() {
        // círculo de raio 24 (bordo de ataque), aba de 15 mm, chapa de 1,5 mm
        let aba = Aba { t: 1.5, r: 1.5, k: 0.44, ang: 90., len: 15. };
        let pts = arco(24., 240, true);
        let outs = fora_radial(&pts, 1.);
        let a = analisar(&pts, &outs, [0., 0., 1.], aba, Limites::default());
        assert_eq!(a.pior_risco(), Risco::Falha);
        let msg = a.aviso().expect("aviso");
        assert!(msg.contains("enruga") && msg.contains("alívios"), "{msg}");
        let h = a.comprimento_seguro().expect("comprimento seguro");
        assert!(h < 15. && h > 0., "{h}");
        // com esse comprimento o ponto sai da faixa de falha
        let b = Aba { len: h, ..aba };
        let a2 = analisar(&pts, &outs, [0., 0., 1.], b, Limites::default());
        assert_ne!(a2.pior_risco(), Risco::Falha);
        // a espessura da borda sobe muito: ε ≈ −h/R e t' = t/(1+ε)
        let p = a.extremos().0.unwrap();
        assert!(p.esp > 1.5, "{}", p.esp);
    }

    #[test]
    fn aba_baixa_em_curva_suave_fica_sem_risco() {
        let aba = Aba { t: 1.5, r: 1.5, k: 0.44, ang: 90., len: 15. };
        let pts = arco(2000., 400, false);
        let outs = fora_radial(&pts, 1.);
        let a = analisar(&pts, &outs, [0., 0., 1.], aba, Limites::default());
        assert!(a.pior_risco() < Risco::Falha, "{:?}", a.extremos().0);
        assert!(a.aviso().is_none());
    }
}
