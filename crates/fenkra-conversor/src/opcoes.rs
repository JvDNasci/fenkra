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

/// O que um arquivo com várias peças vira.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Destino {
    /// um documento por peça e uma montagem com as ocorrências: a peça repetida existe uma vez
    #[default]
    Montagem,
    /// um documento só, com um corpo por ocorrência
    UmaPeca,
}

impl Destino {
    pub const TODOS: [Destino; 2] = [Destino::Montagem, Destino::UmaPeca];

    pub fn nome(self) -> &'static str {
        match self {
            Destino::Montagem => "Montagem",
            Destino::UmaPeca => "Uma peça",
        }
    }
}

/// Como uma montagem vai para o arquivo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Estrutura {
    /// produtos e instâncias, como na árvore
    #[default]
    Arvore,
    /// um produto só com tudo no lugar
    Achatada,
}

impl Estrutura {
    pub const TODAS: [Estrutura; 2] = [Estrutura::Arvore, Estrutura::Achatada];

    pub fn nome(self) -> &'static str {
        match self {
            Estrutura::Arvore => "Com estrutura",
            Estrutura::Achatada => "Achatada",
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
    pub destino: Destino,
}

impl Default for OpcoesImportar {
    fn default() -> Self {
        OpcoesImportar { cores: true, nomes: true, camadas: true, reparo: Reparo::default(), tolerancia: 0.01, unidade: UnidadeArquivo::default(), destino: Destino::default() }
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

/// Como o IGES leva a geometria.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModoIges {
    /// topologia completa: o sólido chega sólido em quem lê BRep
    #[default]
    Brep,
    /// faces aparadas soltas: o que todo programa antigo lê, e chega como superfícies
    Faces,
}

impl ModoIges {
    pub const TODOS: [ModoIges; 2] = [ModoIges::Brep, ModoIges::Faces];

    pub fn nome(self) -> &'static str {
        match self {
            ModoIges::Brep => "BRep",
            ModoIges::Faces => "Faces",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OpcoesExportar {
    pub esquema: EsquemaSaida,
    pub unidade: UnidadeSaida,
    pub cores: bool,
    pub nomes: bool,
    pub camadas: bool,
    /// liga a escrita de PMI do gravador; exige AP242 para ser semântico
    pub pmi: bool,
    pub iges: ModoIges,
    pub estrutura: Estrutura,
    /// desvio, ângulo e codificação dos formatos de malha
    pub malha: crate::OpcoesMalha,
}

impl Default for OpcoesExportar {
    fn default() -> Self {
        OpcoesExportar { esquema: EsquemaSaida::default(), unidade: UnidadeSaida::default(), cores: true, nomes: true, camadas: true, pmi: false, iges: ModoIges::default(), estrutura: Estrutura::default(), malha: crate::OpcoesMalha::default() }
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
