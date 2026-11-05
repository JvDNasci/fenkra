// SPDX-License-Identifier: LGPL-2.1-or-later
//! Opções serializáveis: viajam no trabalho do processo filho e ficam nas preferências.

use fenkra_kernel::{EsquemaStep, UnidadeStep};
use serde::{Deserialize, Serialize};

/// Quanto o conversor mexe na geometria lida.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reparo {
    /// só analisa e relata
    Nenhum,
    /// repara dentro da tolerância e costura o que está solto
    #[default]
    Padrao,
    /// deixa o reparo subir a tolerância em até 100 vezes
    Agressivo,
}

impl Reparo {
    pub const TODOS: [Reparo; 3] = [Reparo::Nenhum, Reparo::Padrao, Reparo::Agressivo];

    pub fn nome(self) -> &'static str {
        match self {
            Reparo::Nenhum => "Nenhum",
            Reparo::Padrao => "Padrão",
            Reparo::Agressivo => "Agressivo",
        }
    }

    pub(crate) fn nivel(self) -> u8 {
        self as u8
    }
}

/// Em que unidade ler os números do arquivo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnidadeArquivo {
    /// a que o arquivo declara; sem declaração, milímetros
    #[default]
    Declarada,
    Milimetro,
    Centimetro,
    Metro,
    Polegada,
}

impl UnidadeArquivo {
    pub const TODAS: [UnidadeArquivo; 5] = [UnidadeArquivo::Declarada, UnidadeArquivo::Milimetro, UnidadeArquivo::Centimetro, UnidadeArquivo::Metro, UnidadeArquivo::Polegada];

    pub fn nome(self) -> &'static str {
        match self {
            UnidadeArquivo::Declarada => "A do arquivo",
            UnidadeArquivo::Milimetro => "Milímetro",
            UnidadeArquivo::Centimetro => "Centímetro",
            UnidadeArquivo::Metro => "Metro",
            UnidadeArquivo::Polegada => "Polegada",
        }
    }

    /// Milímetros por unidade; `None` quando vale a do arquivo.
    pub(crate) fn em_mm(self) -> Option<f64> {
        match self {
            UnidadeArquivo::Declarada => None,
            UnidadeArquivo::Milimetro => Some(1.),
            UnidadeArquivo::Centimetro => Some(10.),
            UnidadeArquivo::Metro => Some(1000.),
            UnidadeArquivo::Polegada => Some(25.4),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OpcoesImportar {
    pub cores: bool,
    pub nomes: bool,
    pub camadas: bool,
    pub reparo: Reparo,
    /// tolerância do reparo e da costura, em mm
    pub tolerancia: f64,
    pub unidade: UnidadeArquivo,
}

impl Default for OpcoesImportar {
    fn default() -> Self {
        OpcoesImportar { cores: true, nomes: true, camadas: true, reparo: Reparo::default(), tolerancia: 0.01, unidade: UnidadeArquivo::default() }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EsquemaSaida {
    Ap203,
    #[default]
    Ap214,
    Ap242,
}

impl From<EsquemaSaida> for EsquemaStep {
    fn from(e: EsquemaSaida) -> Self {
        match e {
            EsquemaSaida::Ap203 => EsquemaStep::Ap203,
            EsquemaSaida::Ap214 => EsquemaStep::Ap214,
            EsquemaSaida::Ap242 => EsquemaStep::Ap242,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnidadeSaida {
    #[default]
    Milimetro,
    Polegada,
    Metro,
}

impl From<UnidadeSaida> for UnidadeStep {
    fn from(u: UnidadeSaida) -> Self {
        match u {
            UnidadeSaida::Milimetro => UnidadeStep::Milimetro,
            UnidadeSaida::Polegada => UnidadeStep::Polegada,
            UnidadeSaida::Metro => UnidadeStep::Metro,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OpcoesExportar {
    pub esquema: EsquemaSaida,
    pub unidade: UnidadeSaida,
    pub cores: bool,
    pub nomes: bool,
    pub camadas: bool,
    /// liga a escrita de PMI do gravador; exige AP242 para ser semântico
    pub pmi: bool,
}

impl Default for OpcoesExportar {
    fn default() -> Self {
        OpcoesExportar { esquema: EsquemaSaida::default(), unidade: UnidadeSaida::default(), cores: true, nomes: true, camadas: true, pmi: false }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn opcoes_voltam_iguais_e_campo_novo_nao_quebra_preferencia_antiga() {
        let o = OpcoesImportar { reparo: Reparo::Agressivo, unidade: UnidadeArquivo::Polegada, tolerancia: 0.05, ..Default::default() };
        assert_eq!(serde_json::from_str::<OpcoesImportar>(&serde_json::to_string(&o).unwrap()).unwrap(), o);
        let antiga: OpcoesImportar = serde_json::from_str(r#"{"cores":false}"#).unwrap();
        assert_eq!(antiga, OpcoesImportar { cores: false, ..Default::default() });
    }
}
