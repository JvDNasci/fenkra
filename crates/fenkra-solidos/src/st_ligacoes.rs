// SPDX-License-Identifier: LGPL-2.1-or-later
//! Ligações de aço: parafusos, solda de filete e chapa de base, com o cálculo à mão da NBR 8800:2008
//! (B135). Cada função devolve a resistência de cálculo e um texto com a conta, para o relatório.
//!
//! * Parafuso a cisalhamento, por plano de corte: `Fv,Rd = 0,4·Ab·fub/γa2` com a rosca no plano de
//!   corte, `0,5·Ab·fub/γa2` sem ela (γa2 = 1,35; Ab = área bruta do fuste).
//! * Pressão de contato no furo: `Fc,Rd = 1,2·lf·t·fu/γa2 ≤ 2,4·db·t·fu/γa2`, com `lf` a distância
//!   livre na direção da força (até a borda ou ao furo seguinte).
//! * Grupo de parafusos com força excêntrica no plano: método elástico (o momento se reparte pela
//!   distância de cada parafuso ao centro do grupo); o parafuso mais carregado manda.
//! * Solda de filete: metal da solda `Fw,Rd = 0,6·Aw·fw/γw2` (γw2 = 1,35; Aw = 0,7·a·l, a garganta
//!   efetiva de filete de pernas iguais) e metal base `0,6·AMB·fy/γa1` (γa1 = 1,10; AMB = a·l).
//! * Chapa de base com força de compressão centrada: pressão no concreto `σ = N/(B·L)` contra
//!   `fcd = fck/(γc·γn)` (γc = 1,4; γn = 1,4 sem confinamento), e espessura pela flexão do balanço `c`
//!   além da seção: `t ≥ c·√(2·σ·γa1/fy)`.
//!
//! O cálculo é de pré-dimensionamento: o relatório diz isso e as hipóteses. Unidades: N, mm, MPa.

pub const GAMA_A1: f64 = 1.10;
pub const GAMA_A2: f64 = 1.35;
pub const GAMA_W2: f64 = 1.35;

/// Um parafuso.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parafuso {
    /// diâmetro nominal (mm)
    pub d: f64,
    /// resistência à ruptura do material do parafuso (MPa)
    pub fub: f64,
    /// a rosca fica no plano de corte
    pub rosca_no_plano: bool,
    /// planos de corte
    pub planos: u32,
}

impl Parafuso {
    pub fn area_bruta(&self) -> f64 {
        std::f64::consts::PI * self.d * self.d / 4.
    }
    /// Resistência ao cisalhamento do parafuso (todos os planos).
    pub fn cisalhamento(&self) -> f64 {
        let k = if self.rosca_no_plano { 0.4 } else { 0.5 };
        k * self.area_bruta() * self.fub / GAMA_A2 * self.planos as f64
    }
}

/// Pressão de contato num furo: chapa de espessura `t`, ruptura `fu`, distância livre `lf`.
pub fn contato(d: f64, t: f64, fu: f64, lf: f64) -> f64 {
    (1.2 * lf * t * fu).min(2.4 * d * t * fu) / GAMA_A2
}

/// Resultado de um grupo de parafusos.
#[derive(Clone, Debug, PartialEq)]
pub struct Grupo {
    /// força no parafuso mais carregado (N)
    pub f_max: f64,
    /// resistência de um parafuso: o menor entre cisalhamento e contato
    pub f_rd: f64,
    pub utilizacao: f64,
    pub conta: String,
}

/// Grupo de parafusos em `pos` (mm, no plano da ligação) com força `(fx, fy)` aplicada em `ponto`.
/// `contato_rd` é a pressão de contato de um furo (a menor das chapas ligadas).
pub fn grupo(pos: &[[f64; 2]], p: Parafuso, fx: f64, fy: f64, ponto: [f64; 2], contato_rd: f64) -> Option<Grupo> {
    let n = pos.len();
    if n == 0 {
        return None;
    }
    let c = [pos.iter().map(|q| q[0]).sum::<f64>() / n as f64, pos.iter().map(|q| q[1]).sum::<f64>() / n as f64];
    // momento da força em torno do centro do grupo
    let m = (ponto[0] - c[0]) * fy - (ponto[1] - c[1]) * fx;
    let j: f64 = pos.iter().map(|q| (q[0] - c[0]).powi(2) + (q[1] - c[1]).powi(2)).sum();
    let mut f_max: f64 = 0.;
    for q in pos {
        let (rx, ry) = (q[0] - c[0], q[1] - c[1]);
        let (mx, my) = if j > 1e-12 { (-m * ry / j, m * rx / j) } else { (0., 0.) };
        f_max = f_max.max((fx / n as f64 + mx).hypot(fy / n as f64 + my));
    }
    let f_rd = p.cisalhamento().min(contato_rd);
    let conta = format!(
        "{n} parafusos Ø{:.0} (fub {:.0} MPa, {} plano(s), rosca {}): Fv,Rd = {:.0} N, contato {:.0} N; momento no grupo {:.0} N·mm; parafuso mais carregado {:.0} N",
        p.d,
        p.fub,
        p.planos,
        if p.rosca_no_plano { "no plano" } else { "fora do plano" },
        p.cisalhamento(),
        contato_rd,
        m,
        f_max
    );
    Some(Grupo { f_max, f_rd, utilizacao: f_max / f_rd, conta })
}

/// Solda de filete de perna `a` e comprimento `l` (mm), eletrodo `fw` e metal base `fy` (MPa):
/// (resistência, conta).
pub fn filete(a: f64, l: f64, fw: f64, fy: f64) -> (f64, String) {
    let aw = 0.7 * a * l;
    let solda = 0.6 * aw * fw / GAMA_W2;
    let base = 0.6 * a * l * fy / GAMA_A1;
    let r = solda.min(base);
    (r, format!("filete a{:.0} × {:.0} mm: metal da solda {:.0} N, metal base {:.0} N → {:.0} N", a, l, solda, base, r))
}

/// Chapa de base B × L (mm) sob uma seção de `bs` × `ls` com compressão `n` (N), concreto `fck` e
/// aço `fy` (MPa): (espessura mínima, utilização do concreto, conta).
pub fn chapa_de_base(n: f64, b: f64, l: f64, bs: f64, ls: f64, fck: f64, fy: f64) -> (f64, f64, String) {
    let sigma = n / (b * l);
    let fcd = fck / (1.4 * 1.4);
    let c = ((b - bs) / 2.).max((l - ls) / 2.).max(0.);
    let t = c * (2. * sigma * GAMA_A1 / fy).sqrt();
    (t, sigma / fcd, format!("chapa {:.0} × {:.0}: σ = {:.2} MPa contra fcd {:.2} MPa; balanço {:.1} mm; espessura mínima {:.1} mm", b, l, sigma, fcd, c, t))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perto(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * b.abs()
    }

    #[test]
    fn parafuso_m16_classe_8_8_a_cisalhamento() {
        // Ab = π·16²/4 = 201,06 mm²; 0,4·201,06·800/1,35 = 47 659 N
        let p = Parafuso { d: 16., fub: 800., rosca_no_plano: true, planos: 1 };
        assert!(perto(p.cisalhamento(), 0.4 * 201.0619 * 800. / 1.35, 1e-5));
        let sem_rosca = Parafuso { rosca_no_plano: false, planos: 2, ..p };
        assert!(perto(sem_rosca.cisalhamento(), 2. * 0.5 * 201.0619 * 800. / 1.35, 1e-5));
    }

    #[test]
    fn contato_limita_pela_borda_ou_pelo_diametro() {
        // borda curta: 1,2·20·8·400/1,35; longe: 2,4·16·8·400/1,35
        assert!(perto(contato(16., 8., 400., 20.), 1.2 * 20. * 8. * 400. / 1.35, 1e-12));
        assert!(perto(contato(16., 8., 400., 100.), 2.4 * 16. * 8. * 400. / 1.35, 1e-12));
    }

    #[test]
    fn grupo_centrado_divide_por_igual_e_excentrico_carrega_a_ponta() {
        let p = Parafuso { d: 16., fub: 800., rosca_no_plano: true, planos: 1 };
        let pos = [[0., 0.], [0., 70.], [0., 140.]];
        let g = grupo(&pos, p, 0., -30_000., [0., 70.], 1e9).unwrap();
        assert!(perto(g.f_max, 10_000., 1e-9));
        // com 100 mm de excentricidade: M = 3e6; J = 2·70² = 9800; o de ponta leva 3e6·70/9800 a mais
        let g = grupo(&pos, p, 0., -30_000., [100., 70.], 1e9).unwrap();
        let fm = 30_000. * 100. * 70. / 9800.;
        assert!(perto(g.f_max, 10_000f64.hypot(fm), 1e-9), "{}", g.f_max);
        assert!(g.utilizacao > 0. && g.conta.contains("3 parafusos"));
    }

    #[test]
    fn filete_e_chapa_de_base() {
        let (r, t) = filete(6., 100., 485., 250.);
        assert!(perto(r, (0.6f64 * 0.7 * 6. * 100. * 485. / 1.35).min(0.6 * 600. * 250. / 1.1), 1e-12));
        assert!(t.contains("a6"));
        // 300 kN numa chapa 300 × 300 sob um tubo 150: σ = 3,33 MPa, balanço 75 mm
        let (t, u, _) = chapa_de_base(300_000., 300., 300., 150., 150., 25., 250.);
        assert!(perto(t, 75. * (2f64 * 300_000. / 90_000. * 1.1 / 250.).sqrt(), 1e-12));
        assert!(perto(u, (300_000. / 90_000.) / (25. / 1.96), 1e-12));
    }
}
