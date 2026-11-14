// SPDX-License-Identifier: LGPL-2.1-or-later
//! Calculadoras de rolamento e de estriado (B111). A de engrenagem reaproveita a verificação do par
//! (`engrenagens_resistencia`, Lewis e Hertz).
//!
//! **Cálculo preliminar** (ver [`crate::calculo_projeto::AVISO_PRELIMINAR`]).
//!
//! - **Vida do rolamento** (Lundberg e Palmgren, 1947, a forma básica que as normas de vida adotam):
//!   `L10 = a1 · (C / P)^p` milhões de voltas, com `p = 3` para esferas e `p = 10/3` para rolos;
//!   em horas, `L10h = L10 · 10⁶ / (60 · n)`. A carga equivalente é `P = X·Fr + Y·Fa`, com `X` e `Y`
//!   do catálogo do fabricante. A segurança estática é `s0 = C0 / P0`, `P0 = X0·Fr + Y0·Fa`.
//! - **Dados do fabricante como camada**: as capacidades `C` e `C0` e os fatores `X`, `Y`, `X0`, `Y0`
//!   mudam de um fabricante para outro e não são dado de norma gratuita; por isso **não vêm
//!   embutidos**. Ficam numa camada [`CamadaFabricante`] que a pessoa preenche (ou carrega de um
//!   arquivo JSON do fabricante) e que se aplica por cima das medidas de contorno da biblioteca
//!   (`rolamentos`, ISO 15). O fator de confiabilidade `a1` é um campo, 1 para 90 %.
//! - **Estriado de flancos retos** (pressão nos flancos, como nos livros de elementos de máquinas):
//!   altura útil `h = (D − d)/2 − 2·c` (c = chanfro), raio médio `rm = (D + d)/4`, e o torque que os
//!   `n` flancos aguentam com a pressão admissível `p_adm`, comprimento `L` e fator de divisão de
//!   carga `φ` (nem todos os flancos encostam; 0,75 é o valor típico):
//!   `T = φ · n · h · L · p_adm · rm`. O núcleo do eixo é conferido à torção no diâmetro menor:
//!   `τ = 16 T / (π d³)`.

use std::f64::consts::PI;

fn positivo(v: f64, nome: &str) -> Result<(), String> {
    if v.is_finite() && v > 0. {
        Ok(())
    } else {
        Err(format!("{nome} precisa ser positivo"))
    }
}

/// Tipo de corpo rolante: decide o expoente da vida.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum CorpoRolante {
    #[default]
    Esferas,
    Rolos,
}

impl CorpoRolante {
    pub fn expoente(self) -> f64 {
        match self {
            CorpoRolante::Esferas => 3.,
            CorpoRolante::Rolos => 10. / 3.,
        }
    }

    pub fn rotulo(self) -> &'static str {
        match self {
            CorpoRolante::Esferas => "Esferas",
            CorpoRolante::Rolos => "Rolos",
        }
    }
}

/// Dados de um rolamento que vêm do fabricante: a camada que a pessoa preenche por cima da biblioteca.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DadosFabricante {
    /// designação, como "6005"
    pub designacao: String,
    /// de onde veio o dado (fabricante, catálogo, página)
    pub fonte: String,
    pub corpo: CorpoRolante,
    /// capacidade de carga dinâmica (N)
    pub c: f64,
    /// capacidade de carga estática (N)
    pub c0: f64,
    pub x: f64,
    pub y: f64,
    pub x0: f64,
    pub y0: f64,
}

impl Default for DadosFabricante {
    fn default() -> Self {
        DadosFabricante { designacao: String::new(), fonte: String::new(), corpo: CorpoRolante::Esferas, c: 0., c0: 0., x: 1., y: 0., x0: 1., y0: 0. }
    }
}

/// Camada de dados do fabricante: lista por designação, gravável em JSON.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CamadaFabricante {
    pub linhas: Vec<DadosFabricante>,
}

impl CamadaFabricante {
    pub fn de_json(s: &str) -> Result<CamadaFabricante, String> {
        let c: CamadaFabricante = serde_json::from_str(s).map_err(|e| format!("camada do fabricante ilegível: {e}"))?;
        for l in &c.linhas {
            if l.designacao.trim().is_empty() {
                return Err("toda linha da camada precisa de designação".into());
            }
            positivo(l.c, &format!("{}: C", l.designacao))?;
            positivo(l.c0, &format!("{}: C0", l.designacao))?;
        }
        Ok(c)
    }

    pub fn para_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn buscar(&self, designacao: &str) -> Option<&DadosFabricante> {
        self.linhas.iter().find(|l| l.designacao == designacao)
    }

    /// Põe ou troca a linha da designação.
    pub fn gravar(&mut self, d: DadosFabricante) {
        match self.linhas.iter_mut().find(|l| l.designacao == d.designacao) {
            Some(l) => *l = d,
            None => self.linhas.push(d),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntradaRolamento {
    pub dados: DadosFabricante,
    /// carga radial e axial (N)
    pub fr: f64,
    pub fa: f64,
    /// rotação (rpm)
    pub rpm: f64,
    /// fator de confiabilidade (1 = 90 %)
    pub a1: f64,
    /// vida exigida (h) e segurança estática exigida
    pub vida_exigida_h: f64,
    pub s0_exigido: f64,
}

impl Default for EntradaRolamento {
    fn default() -> Self {
        EntradaRolamento { dados: DadosFabricante { designacao: "6005".into(), c: 0., c0: 0., ..Default::default() }, fr: 1000., fa: 0., rpm: 1500., a1: 1., vida_exigida_h: 20_000., s0_exigido: 1. }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResultadoRolamento {
    pub p: f64,
    pub p0: f64,
    /// vida em milhões de voltas e em horas
    pub l10: f64,
    pub l10h: f64,
    pub s0: f64,
    /// carga equivalente máxima para a vida exigida (N)
    pub p_max: f64,
    pub avisos: Vec<String>,
}

pub fn calcular_rolamento(e: &EntradaRolamento) -> Result<ResultadoRolamento, String> {
    let d = &e.dados;
    if !(d.c > 0. && d.c0 > 0.) {
        return Err("preencha C e C0 do fabricante: a capacidade de carga não vem embutida".into());
    }
    positivo(e.rpm, "a rotação")?;
    positivo(e.a1, "o fator a1")?;
    for (v, n) in [(e.fr, "a carga radial"), (e.fa, "a carga axial"), (d.x, "X"), (d.y, "Y"), (d.x0, "X0"), (d.y0, "Y0")] {
        if !(v.is_finite() && v >= 0.) {
            return Err(format!("{n} não pode ser negativa"));
        }
    }
    let p = d.x * e.fr + d.y * e.fa;
    let p0 = (d.x0 * e.fr + d.y0 * e.fa).max(e.fr);
    positivo(p, "a carga equivalente")?;
    let k = d.corpo.expoente();
    let l10 = e.a1 * (d.c / p).powf(k);
    let l10h = l10 * 1e6 / (60. * e.rpm);
    let s0 = d.c0 / p0;
    // P que dá a vida exigida: (L / a1)^(1/k) = C / P
    let l_exig = e.vida_exigida_h * 60. * e.rpm / 1e6;
    let p_max = if l_exig > 0. { d.c / (l_exig / e.a1).powf(1. / k) } else { f64::INFINITY };
    let mut avisos = vec![];
    if e.fa > 0. && d.y == 0. {
        avisos.push("há carga axial e Y = 0: confira X e Y no catálogo do fabricante".into());
    }
    if p > 0.5 * d.c {
        avisos.push("P acima de 0,5·C: fora da faixa usual da fórmula de vida".into());
    }
    if l10h < e.vida_exigida_h {
        avisos.push(format!("vida abaixo da exigida: a carga equivalente precisa cair para {p_max:.0} N"));
    }
    if d.fonte.trim().is_empty() {
        avisos.push("informe a fonte de C e C0 (fabricante e catálogo)".into());
    }
    Ok(ResultadoRolamento { p, p0, l10, l10h, s0, p_max, avisos })
}

// ---------------------------------------------------------------- estriado

#[derive(Clone, Debug, PartialEq)]
pub struct EntradaEstriado {
    /// número de estrias
    pub n: u32,
    /// diâmetro menor e maior (mm)
    pub d: f64,
    pub d_maior: f64,
    /// chanfro dos cantos (mm)
    pub chanfro: f64,
    /// comprimento de contato (mm)
    pub comprimento: f64,
    /// pressão admissível nos flancos (MPa)
    pub p_adm: f64,
    /// fator de divisão de carga (parcela dos flancos que trabalha)
    pub phi: f64,
    /// torque exigido (N·m)
    pub torque: f64,
    /// tensão de cisalhamento admissível do eixo (MPa)
    pub tau_adm: f64,
    pub fs_exigido: f64,
}

impl Default for EntradaEstriado {
    fn default() -> Self {
        EntradaEstriado { n: 6, d: 26., d_maior: 30., chanfro: 0.3, comprimento: 40., p_adm: 50., phi: 0.75, torque: 100., tau_adm: 120., fs_exigido: 1.5 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResultadoEstriado {
    pub altura_util: f64,
    pub raio_medio: f64,
    /// torque que os flancos aguentam (N·m)
    pub torque_flancos: f64,
    /// pressão nos flancos com o torque exigido (MPa)
    pub pressao: f64,
    pub fs_flancos: f64,
    /// cisalhamento no núcleo (MPa) e o fator dele
    pub tau_nucleo: f64,
    pub fs_nucleo: f64,
    pub avisos: Vec<String>,
}

pub fn calcular_estriado(e: &EntradaEstriado) -> Result<ResultadoEstriado, String> {
    if e.n < 3 {
        return Err("o estriado precisa de pelo menos 3 estrias".into());
    }
    for (v, n) in [(e.d, "o diâmetro menor"), (e.d_maior, "o diâmetro maior"), (e.comprimento, "o comprimento"), (e.p_adm, "a pressão admissível"), (e.phi, "o fator de divisão"), (e.tau_adm, "a tensão admissível"), (e.fs_exigido, "o fator exigido")] {
        positivo(v, n)?;
    }
    if e.d_maior <= e.d {
        return Err("o diâmetro maior precisa passar o menor".into());
    }
    if e.phi > 1. {
        return Err("o fator de divisão vai de 0 a 1".into());
    }
    let h = (e.d_maior - e.d) / 2. - 2. * e.chanfro;
    positivo(h, "a altura útil (sobra depois dos chanfros)")?;
    let rm = (e.d_maior + e.d) / 4.;
    let t_flancos = e.phi * e.n as f64 * h * e.comprimento * e.p_adm * rm / 1000.;
    let pressao = if e.torque > 0. { e.torque * 1000. / (e.phi * e.n as f64 * h * e.comprimento * rm) } else { 0. };
    let fs_flancos = if e.torque > 0. { t_flancos / e.torque } else { f64::INFINITY };
    let tau = 16. * e.torque * 1000. / (PI * e.d.powi(3));
    let fs_nucleo = if tau > 0. { e.tau_adm / tau } else { f64::INFINITY };
    let mut avisos = vec![];
    if e.comprimento > 2. * e.d_maior {
        avisos.push("comprimento acima de 2·D: a carga não se divide por igual ao longo do estriado".into());
    }
    if fs_flancos < e.fs_exigido {
        avisos.push(format!("flancos abaixo do exigido: aumente o comprimento para {:.0} mm", e.comprimento * e.fs_exigido / fs_flancos));
    }
    Ok(ResultadoEstriado { altura_util: h, raio_medio: rm, torque_flancos: t_flancos, pressao, fs_flancos, tau_nucleo: tau, fs_nucleo, avisos })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// À mão: C = 14 000 N, P = 2000 N, esferas: L10 = 7³ = 343 milhões de voltas; a 1500 rpm,
    /// L10h = 343·10⁶ / 90 000 = 3811,1 h. C0 = 6950 N e P0 = 2000 → s0 = 3,475. Para 20 000 h a
    /// 1500 rpm são 1800 milhões de voltas e P máx = 14 000 / 1800^(1/3) = 1150,9 N.
    #[test]
    fn vida_l10_do_exemplo_a_mao() {
        let dados = DadosFabricante { designacao: "teste".into(), fonte: "exemplo do teste".into(), c: 14_000., c0: 6950., ..Default::default() };
        let e = EntradaRolamento { dados: dados.clone(), fr: 2000., ..Default::default() };
        let r = calcular_rolamento(&e).unwrap();
        assert!((r.l10 - 343.).abs() < 1e-9 && (r.l10h - 3811.11).abs() < 0.01, "{r:?}");
        assert!((r.s0 - 3.475).abs() < 1e-9);
        assert!((r.p_max - 1150.9).abs() < 0.1, "{}", r.p_max);
        assert!(r.avisos.iter().any(|a| a.contains("abaixo da exigida")));
        // rolos: expoente 10/3 → 7^(10/3) = 656,1
        let rolos = calcular_rolamento(&EntradaRolamento { dados: DadosFabricante { corpo: CorpoRolante::Rolos, ..dados.clone() }, ..e.clone() }).unwrap();
        assert!((rolos.l10 - 7_f64.powf(10. / 3.)).abs() < 1e-9 && (rolos.l10 - 656.14).abs() < 0.01);
        // carga combinada com X = 0,56 e Y = 1,8 (valores do catálogo, dados pela pessoa)
        let comb = calcular_rolamento(&EntradaRolamento { dados: DadosFabricante { x: 0.56, y: 1.8, ..dados.clone() }, fr: 2000., fa: 500., ..e.clone() }).unwrap();
        assert!((comb.p - (0.56 * 2000. + 1.8 * 500.)).abs() < 1e-9);
        // a1 = 0,5 corta a vida pela metade
        let meia = calcular_rolamento(&EntradaRolamento { a1: 0.5, ..e.clone() }).unwrap();
        assert!((meia.l10 - 171.5).abs() < 1e-9);
        // sem dado do fabricante não calcula
        assert!(calcular_rolamento(&EntradaRolamento::default()).unwrap_err().contains("fabricante"));
    }

    #[test]
    fn camada_do_fabricante_grava_le_e_recusa_linha_ruim() {
        let mut c = CamadaFabricante::default();
        c.gravar(DadosFabricante { designacao: "6005".into(), fonte: "catálogo X".into(), c: 1., c0: 1., ..Default::default() });
        c.gravar(DadosFabricante { designacao: "6005".into(), fonte: "catálogo Y".into(), c: 2., c0: 1., ..Default::default() });
        assert_eq!(c.linhas.len(), 1);
        let lida = CamadaFabricante::de_json(&c.para_json()).unwrap();
        assert_eq!(lida.buscar("6005").unwrap().fonte, "catálogo Y");
        assert!(CamadaFabricante::de_json(r#"{"linhas":[{"designacao":"6005","c":0,"c0":1}]}"#).is_err());
        assert!(CamadaFabricante::de_json(r#"{"linhas":[{"designacao":"","c":1,"c0":1}]}"#).is_err());
    }

    /// À mão (6 × 26 × 30, chanfro 0,3, L 40, p 50 MPa, φ 0,75): h = 2 − 0,6 = 1,4; rm = 14;
    /// T = 0,75 · 6 · 1,4 · 40 · 50 · 14 = 176 400 N·mm = 176,4 N·m. Com 100 N·m: FS 1,764 e
    /// p = 100 000 / (0,75 · 6 · 1,4 · 40 · 14) = 28,34 MPa; τ = 16·100 000/(π·26³) = 28,98 MPa.
    #[test]
    fn estriado_do_exemplo_a_mao() {
        let r = calcular_estriado(&EntradaEstriado::default()).unwrap();
        assert!((r.altura_util - 1.4).abs() < 1e-12 && (r.raio_medio - 14.).abs() < 1e-12);
        assert!((r.torque_flancos - 176.4).abs() < 1e-9, "{}", r.torque_flancos);
        assert!((r.fs_flancos - 1.764).abs() < 1e-9);
        assert!((r.pressao - 28.345).abs() < 0.001, "{}", r.pressao);
        assert!((r.tau_nucleo - 28.98).abs() < 0.01, "{}", r.tau_nucleo);
        assert!(calcular_estriado(&EntradaEstriado { d_maior: 26., ..Default::default() }).is_err());
        assert!(calcular_estriado(&EntradaEstriado { chanfro: 1.2, ..Default::default() }).is_err());
        let curto = calcular_estriado(&EntradaEstriado { comprimento: 20., ..Default::default() }).unwrap();
        assert!(curto.avisos.iter().any(|a| a.contains("flancos abaixo")));
    }
}
