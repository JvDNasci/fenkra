// SPDX-License-Identifier: LGPL-2.1-or-later
//! Resistência estática do eixo escalonado (B52).
//!
//! Cada trecho é conferido com o torque `T` e o momento fletor `M` dados (o maior que o eixo vê; a
//! conta assume o mesmo `M` em todos os trechos, o que fica do lado da segurança). Fórmulas de livro
//! de resistência dos materiais para seção circular cheia:
//!
//! - flexão: `σ = 32 M / (π d³)`;
//! - torção: `τ = 16 T / (π d³)`;
//! - tensão equivalente de von Mises: `σeq = √(σ² + 3 τ²)`;
//! - segurança: `n = Sy / σeq`;
//! - diâmetro mínimo para a segurança exigida `n`: `d = (32 n / (π Sy) · √(M² + ¾ T²))^(1/3)`.
//!
//! A seção conferida é a **líquida**: o menor diâmetro que sobra no trecho depois dos recursos —
//! fundo da ranhura ou do anel (`d − 2·prof`), diâmetro menor da rosca métrica
//! (`d3 = d − 1,22687·P`, perfil básico ISO) e fundo do rasgo de chaveta (`d − t1`). Não entram fator
//! de concentração de tensão nem fadiga: o resultado é a verificação estática, e o relatório diz isso.

use crate::biblioteca::chaveta_para_eixo;
use crate::eixos::ParamEixo;
use crate::eixos_recursos::{RecursoEixo, TipoRecurso};
use std::f64::consts::PI;

/// Fator do diâmetro menor da rosca externa métrica: d3 = d − 1,22687·P.
const FATOR_D3: f64 = 1.226_87;

#[derive(Clone, Debug, PartialEq)]
pub struct EntradaEixo {
    /// torque transmitido (N·m)
    pub torque_nm: f64,
    /// maior momento fletor (N·m)
    pub momento_nm: f64,
    /// limite de escoamento do material (MPa)
    pub escoamento_mpa: f64,
    /// segurança exigida
    pub fs_exigido: f64,
}

impl Default for EntradaEixo {
    fn default() -> Self {
        // aço 1045 laminado: escoamento típico de 310 MPa
        EntradaEixo { torque_nm: 20., momento_nm: 10., escoamento_mpa: 310., fs_exigido: 2. }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrechoVerificado {
    pub trecho: usize,
    pub diametro: f64,
    /// diâmetro da seção conferida (o menor que sobra no trecho)
    pub diametro_liquido: f64,
    /// o que reduziu a seção (vazio = trecho cheio)
    pub motivo: String,
    pub flexao_mpa: f64,
    pub torcao_mpa: f64,
    pub equivalente_mpa: f64,
    pub seguranca: f64,
    pub ok: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResultadoEixo {
    pub trechos: Vec<TrechoVerificado>,
    /// diâmetro líquido mínimo para a segurança exigida
    pub diametro_minimo: f64,
    /// índice do trecho de menor segurança
    pub critico: usize,
}

/// Diâmetro que sobra no trecho `i` e o motivo da redução.
pub fn secao_liquida(p: &ParamEixo, recursos: &[RecursoEixo], i: usize) -> (f64, String) {
    let t = p.trechos[i];
    let mut melhor = (t.diametro, String::new());
    let mut ver = |d: f64, motivo: &str| {
        if d < melhor.0 {
            melhor = (d, motivo.to_string());
        }
    };
    if t.rasgo.is_some() {
        if let Some(c) = chaveta_para_eixo(t.diametro) {
            ver(t.diametro - c.t1, "rasgo de chaveta");
        }
    }
    for r in recursos.iter().filter(|r| r.trecho == i) {
        match r.tipo {
            TipoRecurso::Anel { profundidade, .. } => ver(t.diametro - 2. * profundidade, "anel elástico"),
            TipoRecurso::Ranhura { profundidade, .. } => ver(t.diametro - 2. * profundidade, "ranhura"),
            TipoRecurso::Rosca { passo, .. } => ver(t.diametro - FATOR_D3 * passo, "rosca"),
            TipoRecurso::Rasgo { .. } => {
                if let Some(c) = chaveta_para_eixo(t.diametro) {
                    ver(t.diametro - c.t1, "rasgo de chaveta");
                }
            }
            // o furo transversal tira uma faixa da seção; a conta da seção cheia não vale nele
            TipoRecurso::FuroTransversal { diametro, .. } => ver(t.diametro - diametro, "furo transversal"),
        }
    }
    melhor
}

/// Confere cada trecho com o torque e o momento dados.
pub fn verificar_eixo(p: &ParamEixo, recursos: &[RecursoEixo], e: &EntradaEixo) -> Result<ResultadoEixo, String> {
    for (v, n) in [(e.torque_nm, "torque"), (e.momento_nm, "momento fletor"), (e.escoamento_mpa, "escoamento"), (e.fs_exigido, "segurança exigida")] {
        if !v.is_finite() || v < 0. {
            return Err(format!("{n} inválido"));
        }
    }
    if e.escoamento_mpa <= 0. || e.fs_exigido <= 0. {
        return Err("o escoamento e a segurança exigida precisam ser positivos".into());
    }
    if p.trechos.is_empty() {
        return Err("o eixo não tem trechos".into());
    }
    // N·m → N·mm; tensões em MPa (N/mm²)
    let (m, t) = (e.momento_nm * 1000., e.torque_nm * 1000.);
    let mut trechos = vec![];
    for i in 0..p.trechos.len() {
        let (d, motivo) = secao_liquida(p, recursos, i);
        if d <= 0. {
            return Err(format!("trecho {}: os recursos tiram a seção inteira", i + 1));
        }
        let k = PI * d.powi(3);
        let (s, tau) = (32. * m / k, 16. * t / k);
        let eq = (s * s + 3. * tau * tau).sqrt();
        let n = if eq > 0. { e.escoamento_mpa / eq } else { f64::INFINITY };
        trechos.push(TrechoVerificado { trecho: i, diametro: p.trechos[i].diametro, diametro_liquido: d, motivo, flexao_mpa: s, torcao_mpa: tau, equivalente_mpa: eq, seguranca: n, ok: n >= e.fs_exigido });
    }
    let diametro_minimo = (32. * e.fs_exigido / (PI * e.escoamento_mpa) * (m * m + 0.75 * t * t).sqrt()).cbrt();
    let critico = trechos.iter().enumerate().min_by(|a, b| a.1.seguranca.total_cmp(&b.1.seguranca)).map(|(i, _)| i).unwrap_or(0);
    Ok(ResultadoEixo { trechos, diametro_minimo, critico })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eixos::TrechoEixo;
    use crate::eixos_recursos::Borda;

    fn eixo() -> ParamEixo {
        ParamEixo {
            trechos: vec![
                TrechoEixo { diametro: 20., comprimento: 40., chanfro_fim: 0., rasgo: None },
                TrechoEixo { diametro: 30., comprimento: 60., chanfro_fim: 0., rasgo: None },
            ],
            furo_centro: false,
        }
    }

    /// Torção pura em Ø20 com 100 N·m: τ = 16·100000/(π·8000) = 63,66 MPa; σeq = √3·τ.
    #[test]
    fn torcao_pura_bate_com_a_formula() {
        let e = EntradaEixo { torque_nm: 100., momento_nm: 0., escoamento_mpa: 300., fs_exigido: 1.5 };
        let r = verificar_eixo(&eixo(), &[], &e).unwrap();
        let t0 = &r.trechos[0];
        assert!((t0.torcao_mpa - 63.662).abs() < 1e-2, "{}", t0.torcao_mpa);
        assert!((t0.equivalente_mpa - 3f64.sqrt() * 63.662).abs() < 2e-2);
        assert_eq!(r.critico, 0);
        // d mínimo: com o d achado, a segurança fica exatamente na exigida
        let d = r.diametro_minimo;
        let tau = 16. * 100_000. / (PI * d.powi(3));
        assert!((300. / (3f64.sqrt() * tau) - 1.5).abs() < 1e-9);
    }

    /// A ranhura e a rosca reduzem a seção conferida; a do menor diâmetro manda.
    #[test]
    fn recursos_reduzem_a_secao() {
        let rec = |trecho, tipo| RecursoEixo { trecho, borda: Borda::Esquerda, posicao: 5., tipo };
        let recursos = [rec(1, TipoRecurso::Anel { largura: 1.6, profundidade: 1.5 }), rec(0, TipoRecurso::Rosca { comprimento: 10., passo: 2.5 })];
        let (d1, m1) = secao_liquida(&eixo(), &recursos, 1);
        assert_eq!((d1, m1.as_str()), (27., "anel elástico"));
        let (d0, m0) = secao_liquida(&eixo(), &recursos, 0);
        assert!((d0 - (20. - 1.22687 * 2.5)).abs() < 1e-9 && m0 == "rosca");
        let e = EntradaEixo { torque_nm: 50., momento_nm: 30., escoamento_mpa: 310., fs_exigido: 2. };
        let sem = verificar_eixo(&eixo(), &[], &e).unwrap();
        let com = verificar_eixo(&eixo(), &recursos, &e).unwrap();
        assert!(com.trechos[0].seguranca < sem.trechos[0].seguranca);
        assert!(com.trechos[1].seguranca < sem.trechos[1].seguranca);
    }

    #[test]
    fn entradas_invalidas() {
        assert!(verificar_eixo(&eixo(), &[], &EntradaEixo { escoamento_mpa: 0., ..Default::default() }).is_err());
        assert!(verificar_eixo(&eixo(), &[], &EntradaEixo { torque_nm: f64::NAN, ..Default::default() }).is_err());
        let r = verificar_eixo(&eixo(), &[], &EntradaEixo { torque_nm: 0., momento_nm: 0., ..Default::default() }).unwrap();
        assert!(r.trechos.iter().all(|t| t.ok));
    }
}
