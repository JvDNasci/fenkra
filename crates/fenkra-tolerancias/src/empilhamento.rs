// SPDX-License-Identifier: LGPL-2.1-or-later
//! Empilhamento de tolerâncias em uma direção (B126): pior caso, RSS e Monte Carlo.
//!
//! Cada elo é uma cota da cadeia com o seu nominal, os desvios superior e inferior e o sentido
//! (+1 soma, −1 subtrai, conforme a cota vai a favor ou contra a direção da cadeia). O resultado é a
//! folga ou a medida que fecha a cadeia.
//!
//! * **Pior caso**: soma dos extremos. Garante 100% das peças, ao custo de tolerâncias apertadas.
//! * **RSS** (raiz da soma dos quadrados): cada elo vira uma distribuição centrada no meio da sua
//!   zona, com a meia-largura como 3σ; o resultado é o meio somado ± √Σ(meia-largura²).
//! * **Monte Carlo**: sorteia cada elo com essa mesma normal (σ = meia-largura / 3), soma, e conta
//!   média, desvio e a fração fora dos limites pedidos. Gerador próprio com semente, para o mesmo
//!   resultado em qualquer máquina.
//!
//! A contribuição de cada elo é a parte dele na variação: meia-largura sobre a soma (pior caso) e
//! meia-largura² sobre a soma dos quadrados (RSS).

/// Um elo da cadeia.
#[derive(Clone, Debug, PartialEq)]
pub struct Elo {
    pub nome: String,
    pub nominal: f64,
    /// desvio superior (mm, com sinal): +0,1
    pub sup: f64,
    /// desvio inferior (mm, com sinal): −0,05
    pub inf: f64,
    /// +1 soma, −1 subtrai
    pub sentido: f64,
}

impl Elo {
    fn meio(&self) -> f64 {
        self.sentido * (self.nominal + (self.sup + self.inf) / 2.)
    }
    fn meia(&self) -> f64 {
        (self.sup - self.inf).abs() / 2.
    }
}

/// Estatística do Monte Carlo.
#[derive(Clone, Debug, PartialEq)]
pub struct MonteCarlo {
    pub amostras: usize,
    pub media: f64,
    pub desvio: f64,
    pub min: f64,
    pub max: f64,
    /// fração das amostras fora dos limites pedidos (0 sem limites)
    pub fora: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resultado {
    /// soma dos nominais com sentido
    pub nominal: f64,
    /// extremos do pior caso
    pub pior: (f64, f64),
    /// meio e meia-largura do RSS
    pub rss_meio: f64,
    pub rss_meia: f64,
    /// contribuição de cada elo (0 a 1) no pior caso e no RSS
    pub contrib_pior: Vec<f64>,
    pub contrib_rss: Vec<f64>,
    pub mc: Option<MonteCarlo>,
}

/// Gerador xorshift64* com semente: leve e repetível.
struct Sorteio(u64);

impl Sorteio {
    fn u(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    /// normal padrão por Box-Muller
    fn normal(&mut self) -> f64 {
        let (a, b) = (self.u(), self.u());
        (-2. * a.ln()).sqrt() * (std::f64::consts::TAU * b).cos()
    }
}

/// Analisa a cadeia. `amostras` = 0 desliga o Monte Carlo; `limites` são os da especificação do
/// resultado (mínimo, máximo), para contar o que fica fora.
pub fn analisar(elos: &[Elo], amostras: usize, semente: u64, limites: Option<(f64, f64)>) -> Resultado {
    let nominal: f64 = elos.iter().map(|e| e.sentido * e.nominal).sum();
    let (mut lo, mut hi) = (0., 0.);
    for e in elos {
        let (a, b) = (e.sentido * (e.nominal + e.inf), e.sentido * (e.nominal + e.sup));
        lo += a.min(b);
        hi += a.max(b);
    }
    let rss_meio: f64 = elos.iter().map(Elo::meio).sum();
    let soma_q: f64 = elos.iter().map(|e| e.meia().powi(2)).sum();
    let soma: f64 = elos.iter().map(Elo::meia).sum();
    let contrib_pior = elos.iter().map(|e| if soma > 0. { e.meia() / soma } else { 0. }).collect();
    let contrib_rss = elos.iter().map(|e| if soma_q > 0. { e.meia().powi(2) / soma_q } else { 0. }).collect();
    let mc = (amostras > 0).then(|| {
        let mut s = Sorteio(semente.max(1));
        let (mut m, mut q, mut mn, mut mx, mut fora) = (0., 0., f64::MAX, f64::MIN, 0usize);
        for _ in 0..amostras {
            let x: f64 = elos.iter().map(|e| e.meio() + e.sentido * e.meia() / 3. * s.normal()).sum();
            m += x;
            q += x * x;
            mn = f64::min(mn, x);
            mx = f64::max(mx, x);
            if limites.is_some_and(|(a, b)| x < a || x > b) {
                fora += 1;
            }
        }
        let n = amostras as f64;
        let media = m / n;
        MonteCarlo { amostras, media, desvio: (q / n - media * media).max(0.).sqrt(), min: mn, max: mx, fora: fora as f64 / n }
    });
    Resultado { nominal, pior: (lo, hi), rss_meio, rss_meia: soma_q.sqrt(), contrib_pior, contrib_rss, mc }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn elo(nome: &str, nominal: f64, sup: f64, inf: f64, sentido: f64) -> Elo {
        Elo { nome: nome.into(), nominal, sup, inf, sentido }
    }

    /// Folga de um eixo de 3 peças numa caixa: caixa 60 ±0,1 menos três peças de 19,9 ±0,05.
    #[test]
    fn folga_pior_caso_rss_e_monte_carlo() {
        let c = vec![elo("caixa", 60., 0.1, -0.1, 1.), elo("a", 19.9, 0.05, -0.05, -1.), elo("b", 19.9, 0.05, -0.05, -1.), elo("c", 19.9, 0.05, -0.05, -1.)];
        let r = analisar(&c, 20_000, 7, Some((0., 0.5)));
        assert!((r.nominal - 0.3).abs() < 1e-9);
        // pior caso: 0,3 ± (0,1 + 3·0,05) = 0,05 a 0,55
        assert!((r.pior.0 - 0.05).abs() < 1e-9 && (r.pior.1 - 0.55).abs() < 1e-9, "{:?}", r.pior);
        // RSS: √(0,1² + 3·0,05²) = 0,1323
        assert!((r.rss_meia - (0.01f64 + 3. * 0.0025).sqrt()).abs() < 1e-9);
        assert!(r.rss_meia < 0.25, "o RSS é mais estreito que o pior caso");
        // contribuições somam 1; a caixa é a maior parte da variação no RSS (0,01/0,0175)
        assert!((r.contrib_rss.iter().sum::<f64>() - 1.).abs() < 1e-9);
        assert!((r.contrib_rss[0] - 0.01 / 0.0175).abs() < 1e-9);
        assert!((r.contrib_pior.iter().sum::<f64>() - 1.).abs() < 1e-9);
        // Monte Carlo: σ do resultado ≈ RSS / 3, média no nominal, quase nada acima de 0,5
        let mc = r.mc.unwrap();
        assert!((mc.media - 0.3).abs() < 0.003, "{mc:?}");
        assert!((mc.desvio - r.rss_meia / 3.).abs() < 0.003, "{mc:?}");
        assert!(mc.fora < 0.01, "{mc:?}");
        assert!(mc.min > r.pior.0 - 1e-9 || mc.min > r.rss_meio - 4. * r.rss_meia);
        // mesma semente, mesmo resultado
        assert_eq!(analisar(&c, 1000, 7, None).mc, analisar(&c, 1000, 7, None).mc);
    }

    #[test]
    fn desvio_assimetrico_desloca_o_meio() {
        // furo H7 de 20 (0/+0,021) menos eixo g6 de 20 (−0,020/−0,007): folga 0,007 a 0,041
        let r = analisar(&[elo("furo", 20., 0.021, 0., 1.), elo("eixo", 20., -0.007, -0.020, -1.)], 0, 1, None);
        assert!((r.pior.0 - 0.007).abs() < 1e-9 && (r.pior.1 - 0.041).abs() < 1e-9, "{:?}", r.pior);
        assert!((r.rss_meio - 0.024).abs() < 1e-9);
        assert!(r.mc.is_none());
    }
}
