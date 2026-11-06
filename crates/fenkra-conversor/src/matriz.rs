// SPDX-License-Identifier: LGPL-2.1-or-later
//! O que atravessa cada formato. Cada provedor declara as suas [`Capacidades`], e a tela e o diálogo só
//! leem o que ele declarou; nenhuma lista à parte pode ficar velha.

use crate::{Formato, Provedor, Registro};
use serde::{Deserialize, Serialize};

/// Uma linha da matriz: o que um formato lê, grava e carrega.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capacidades {
    pub ler: bool,
    pub gravar: bool,
    /// curvas e superfícies exatas, sem virar triângulo
    pub exata: bool,
    pub malha: bool,
    pub cor: bool,
    pub camadas: bool,
    pub montagem: bool,
    pub pmi: bool,
    pub parametros: bool,
    /// o arquivo diz em que unidade está
    pub unidade: bool,
}

impl Capacidades {
    /// Nome de cada coluna do que atravessa, com o valor nesta linha, na ordem da tela.
    pub fn colunas(&self) -> [(&'static str, bool); 8] {
        [
            ("Geometria exata", self.exata),
            ("Malha", self.malha),
            ("Cor", self.cor),
            ("Camadas", self.camadas),
            ("Montagem", self.montagem),
            ("PMI", self.pmi),
            ("Parâmetros", self.parametros),
            ("Unidade", self.unidade),
        ]
    }

    /// O que o formato carrega, para a linha de suporte do diálogo.
    pub fn leva(&self) -> Vec<&'static str> {
        self.colunas().into_iter().filter(|(_, v)| *v).map(|(n, _)| n).collect()
    }

    /// O que fica para trás. Malha só faz falta a quem não tem geometria nenhuma.
    pub fn nao_leva(&self) -> Vec<&'static str> {
        self.colunas().into_iter().filter(|(n, v)| !*v && *n != "Malha").map(|(n, _)| n).collect()
    }
}

/// Uma linha da tela "Formatos suportados".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinhaMatriz {
    pub formato: Formato,
    /// quem atende o formato
    pub provedor: String,
    pub capacidades: Capacidades,
}

/// Capacidades dos formatos que o próprio conversor atende, fora dos provedores de geometria.
fn internas(f: Formato) -> Option<Capacidades> {
    match f {
        Formato::Arvore => Some(Capacidades { ler: true, gravar: true, cor: true, montagem: true, parametros: true, unidade: true, ..Default::default() }),
        Formato::Dxf => Some(Capacidades { ler: true, exata: true, camadas: true, unidade: true, ..Default::default() }),
        _ => None,
    }
}

impl Registro {
    /// Matriz de todos os formatos com quem atende, na ordem da tela.
    pub fn matriz(&self) -> Vec<LinhaMatriz> {
        Formato::TODOS
            .into_iter()
            .filter_map(|formato| match self.provedor(formato) {
                Ok(p) => Some(LinhaMatriz { formato, provedor: p.id().into(), capacidades: p.capacidades(formato) }),
                Err(_) => internas(formato).map(|capacidades| LinhaMatriz { formato, provedor: "Fenkra".into(), capacidades }),
            })
            .collect()
    }

    /// Capacidades de um formato; tudo desligado se ninguém o atende.
    pub fn capacidades(&self, formato: Formato) -> Capacidades {
        self.matriz().into_iter().find(|l| l.formato == formato).map(|l| l.capacidades).unwrap_or_default()
    }

    /// Extensões de tudo o que se pode ler, para o filtro do seletor de arquivo.
    pub fn extensoes_de_leitura(&self) -> Vec<&'static str> {
        self.matriz().into_iter().filter(|l| l.capacidades.ler).flat_map(|l| l.formato.extensoes().iter().copied()).collect()
    }

    /// Formatos em que se pode gravar uma peça, na ordem da tela.
    pub fn formatos_de_gravacao(&self) -> Vec<Formato> {
        self.matriz().into_iter().filter(|l| l.capacidades.gravar).map(|l| l.formato).collect()
    }
}
