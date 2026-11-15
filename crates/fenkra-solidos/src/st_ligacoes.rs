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
//! * Parafuso ou chumbador a tração: `Ft,Rd = 0,75·Ab·fub/γa2` (barra rosqueada: até `Ab·fy/γa1`);
//!   com cisalhamento junto, `(Ft,Sd/Ft,Rd)² + (Fv,Sd/Fv,Rd)² ≤ 1`.
//! * Chapa de base com compressão e momento: bloco de tensão uniforme no concreto, excentricidade
//!   crítica e tração nos chumbadores pelo equilíbrio (`chapa_de_base_com_momento`).
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

/// Parafuso ou chumbador (barra redonda rosqueada) a tração: `Ft,Rd = 0,75·Ab·fub/γa2`, com a área
/// efetiva tomada como 75 % da bruta; na barra rosqueada, nunca acima de `Ab·fy/γa1` (`fy` dado).
pub fn tracao(d: f64, fub: f64, fy: Option<f64>) -> f64 {
    let ab = std::f64::consts::PI * d * d / 4.;
    let r = 0.75 * ab * fub / GAMA_A2;
    match fy {
        Some(fy) => r.min(ab * fy / GAMA_A1),
        None => r,
    }
}

/// Tração com cisalhamento no mesmo parafuso: `(Ft,Sd/Ft,Rd)² + (Fv,Sd/Fv,Rd)²`, que deve ficar até 1.
pub fn interacao_tracao_cisalhamento(ft: f64, ft_rd: f64, fv: f64, fv_rd: f64) -> f64 {
    (ft / ft_rd).powi(2) + (fv / fv_rd).powi(2)
}

/// Chapa de base com compressão e momento, pelo bloco de tensão uniforme no concreto.
#[derive(Clone, Debug, PartialEq)]
pub struct BaseComMomento {
    /// excentricidade M/N e a crítica, até a qual o concreto sozinho equilibra (mm)
    pub e: f64,
    pub e_critica: f64,
    /// comprimento comprimido do concreto (mm)
    pub y: f64,
    /// tensão no concreto (MPa) e utilização dela contra fcd
    pub sigma: f64,
    pub utilizacao_concreto: f64,
    /// tração no conjunto de chumbadores do lado tracionado (N)
    pub tracao: f64,
    /// espessura mínima da chapa: pela pressão do concreto e pela tração dos chumbadores (mm)
    pub espessura: f64,
    pub conta: String,
}

/// Chapa de base `b` (largura) × `l` (na direção do momento) sob um pilar `bs` × `ls`, com
/// compressão `n` (N, positiva) e momento `m` (N·mm); `f` é a distância do centro da chapa à linha
/// de chumbadores do lado tracionado; concreto `fck` e aço da chapa `fy` (MPa).
///
/// Como funciona: o concreto reage com tensão uniforme `σ` num trecho `Y` a partir da borda
/// comprimida, até `fcd = fck/(1,4·1,4)`, isto é, até `qmax = fcd·b` por mm. Com `e = M/N` até
/// `e_crit = L/2 − N/(2·qmax)`, só o concreto equilibra: `Y = L − 2e` e `σ = N/(b·Y)`. Acima, o
/// concreto trabalha no limite e os chumbadores tracionam: das somas de forças e de momentos em
/// torno da linha de chumbadores, `Y = (f + L/2) − √((f + L/2)² − 2·N·(e + f)/qmax)` e
/// `T = qmax·Y − N`. Sem raiz real, a chapa é pequena demais: volta `None`. A espessura vem da
/// flexão plástica de uma faixa de 1 mm em balanço além do pilar, `t = √(4·Mpl·γa1/fy)`: do lado do
/// concreto com o momento da tensão no balanço `c` (todo comprimido, `σ·c²/2`; parte, `σ·Y·(c − Y/2)`)
/// e do lado tracionado com `T·(f − ls/2)/b`.
#[allow(clippy::too_many_arguments)]
pub fn chapa_de_base_com_momento(n: f64, m: f64, b: f64, l: f64, bs: f64, ls: f64, f: f64, fck: f64, fy: f64) -> Option<BaseComMomento> {
    if n <= 0. || b <= 0. || l <= 0. {
        return None;
    }
    let fcd = fck / (1.4 * 1.4);
    let qmax = fcd * b;
    let e = m.abs() / n;
    let e_critica = l / 2. - n / (2. * qmax);
    let (y, sigma, tracao) = if e <= e_critica {
        let y = l - 2. * e;
        (y, n / (b * y), 0.)
    } else {
        let a = f + l / 2.;
        let disc = a * a - 2. * n * (e + f) / qmax;
        if disc < 0. {
            return None;
        }
        let y = a - disc.sqrt();
        (y, fcd, qmax * y - n)
    };
    let c = ((b - bs) / 2.).max((l - ls) / 2.).max(0.);
    let m_conc = if y >= c { sigma * c * c / 2. } else { sigma * y * (c - y / 2.) };
    let braco = (f - ls / 2.).max(0.);
    let m_trac = tracao * braco / b;
    let t = |mm: f64| (4. * mm * GAMA_A1 / fy).sqrt();
    let espessura = t(m_conc).max(t(m_trac));
    let conta = format!(
        "chapa {:.0} × {:.0}: e = {:.1} mm (crítica {:.1}); concreto comprimido em {:.1} mm com σ = {:.2} MPa contra fcd {:.2} MPa; tração nos chumbadores {:.0} N; espessura mínima {:.1} mm",
        b, l, e, e_critica, y, sigma, fcd, tracao, espessura
    );
    Some(BaseComMomento { e, e_critica, y, sigma, utilizacao_concreto: sigma / fcd, tracao, espessura, conta })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chumbador Ø20 ASTM A36 (fub 400, fy 250): 0,75·314,16·400/1,35 = 69 813 N, abaixo de
    /// Ab·fy/γa1 = 71 400 N; parafuso Ø16 classe 8.8 a tração e cisalhamento juntos.
    #[test]
    fn tracao_de_chumbador_e_interacao() {
        let ab = std::f64::consts::PI * 100.;
        assert!(perto(tracao(20., 400., Some(250.)), 0.75 * ab * 400. / 1.35, 1e-12));
        assert!((tracao(20., 400., Some(250.)) - 69_813.).abs() < 1.);
        // barra de aço fraco e ruptura alta: manda o escoamento
        assert!(perto(tracao(20., 600., Some(250.)), ab * 250. / 1.1, 1e-12));
        let p = Parafuso { d: 16., fub: 800., rosca_no_plano: true, planos: 1 };
        let (ft_rd, fv_rd) = (tracao(16., 800., None), p.cisalhamento());
        let u = interacao_tracao_cisalhamento(0.6 * ft_rd, ft_rd, 0.6 * fv_rd, fv_rd);
        assert!(perto(u, 0.72, 1e-12));
    }

    /// Chapa 400 × 500 com chumbadores a 200 do centro, pilar 200 × 300, C25 e aço 250:
    /// - 200 kN e 60 kN·m: e = 300 > e_crit = 250 − 200 000/(2·5 102) = 230,4, há tração;
    ///   as duas equações de equilíbrio fecham com o Y e o T calculados;
    /// - 200 kN e 20 kN·m: e = 100, Y = 300 e σ = 200 000/(400·300) = 1,667 MPa, sem tração.
    #[test]
    fn chapa_de_base_com_momento_fecha_o_equilibrio() {
        let fcd = 25. / 1.96;
        let r = chapa_de_base_com_momento(200_000., 60e6, 400., 500., 200., 300., 200., 25., 250.).unwrap();
        assert!(perto(r.e, 300., 1e-12));
        assert!(perto(r.e_critica, 250. - 200_000. / (2. * fcd * 400.), 1e-12));
        assert!(r.tracao > 0. && perto(r.sigma, fcd, 1e-12));
        // forças: compressão no concreto = N + T; momentos em torno do centro da chapa
        let c = fcd * 400. * r.y;
        assert!(perto(c, 200_000. + r.tracao, 1e-9));
        assert!(perto(c * (250. - r.y / 2.) + r.tracao * 200., 200_000. * 300., 1e-9), "{r:?}");
        // conta à mão: Y = 450 − √(450² − 2·200 000·500/5 102,04)
        let y = 450. - (450f64.powi(2) - 2. * 200_000. * 500. / (fcd * 400.)).sqrt();
        assert!(perto(r.y, y, 1e-12) && (r.y - 45.9).abs() < 0.1, "{}", r.y);
        // espessura: balanço c = max(100, 100) = 100 > Y → σ·Y·(c − Y/2); tração: T·(200 − 150)/400
        let m_conc = fcd * y * (100. - y / 2.);
        let m_trac = r.tracao * 50. / 400.;
        assert!(perto(r.espessura, (4. * m_conc.max(m_trac) * 1.1 / 250.).sqrt(), 1e-12));
        let s = chapa_de_base_com_momento(200_000., 20e6, 400., 500., 200., 300., 200., 25., 250.).unwrap();
        assert_eq!(s.tracao, 0.);
        assert!(perto(s.y, 300., 1e-12) && perto(s.sigma, 200_000. / 120_000., 1e-12));
        // chapa pequena para o momento: sem solução
        assert!(chapa_de_base_com_momento(200_000., 1000e6, 400., 500., 200., 300., 200., 25., 250.).is_none());
    }

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
