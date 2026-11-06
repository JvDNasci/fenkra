// SPDX-License-Identifier: LGPL-2.1-or-later
//! Conversor de arquivos (B87): formatos, opções, relatório e o processo filho que isola o kernel.
//!
//! Quem usa chama [`processo::Filho`], que roda a conversão em `fenkra-conv`: um arquivo hostil que
//! derrube o OpenCASCADE derruba o filho, e o programa recebe um erro. Dentro do filho, ou nos
//! testes, [`Registro`] escolhe o [`Provedor`] do formato e [`Registro::importar`] monta o documento.
//! O que atravessa os processos é só dado (documento, relatório, prévia), nunca uma forma do kernel.

pub mod arvore;
mod conjunto;
mod documento;
mod dxf;
mod formato;
mod importar;
pub mod lote;
pub mod malha;
mod matriz;
mod modelo;
mod occt;
mod opcoes;
mod plano;
pub mod processo;
mod provedor;
mod relatorio;
mod sonda;

pub use documento::montar_documento;
pub use formato::{detectar, Formato};
pub use importar::{unidade_suspeita, Importado, Previa, Suspeita};
pub use modelo::{InstanciaNeutra, ModeloNeutro, Ocorrencia, ProdutoNeutro};
pub use occt::ProvedorOcct;
pub use opcoes::{Destino, EsquemaSaida, Estrutura, ModoIges, OpcoesExportar, OpcoesImportar, Reparo, UnidadeArquivo, UnidadeSaida};
pub use malha::{Cena, OpcoesMalha, ProvedorMalha};
pub use matriz::{Capacidades, LinhaMatriz};
pub use provedor::{Provedor, Registro};
pub use relatorio::{Contagem, Defeito, LinhaReparo, Nota, Relatorio, TipoDefeito, NOTAS};

/// Por que uma conversão não terminou.
#[derive(Clone, Debug, PartialEq, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum ErroConv {
    /// o arquivo não é de um formato que algum provedor leia
    #[error("{0}")]
    Formato(String),
    #[error("{0}")]
    Leitura(String),
    #[error("{0}")]
    Gravacao(String),
    #[error("o conversor parou ao ler o arquivo (código {0}); o Fenkra segue aberto")]
    Queda(i32),
    #[error("a conversão passou do limite de {0} s e foi encerrada")]
    Tempo(u64),
    #[error("conversão cancelada")]
    Cancelado,
    #[error("programa do conversor não encontrado: {0}")]
    Ausente(String),
}

impl From<fenkra_malha::Erro> for ErroConv {
    fn from(e: fenkra_malha::Erro) -> Self {
        use fenkra_malha::Erro;
        match e {
            Erro::Formato(t) => ErroConv::Formato(t),
            Erro::Leitura(t) | Erro::Operacao(t) => ErroConv::Leitura(t),
            Erro::Gravacao(t) => ErroConv::Gravacao(t),
        }
    }
}

impl From<fenkra_kernel::KernelError> for ErroConv {
    fn from(e: fenkra_kernel::KernelError) -> Self {
        ErroConv::Leitura(e.0)
    }
}

/// Texto de um caminho para a ponte com o kernel, que recebe UTF-8.
fn texto_do_caminho(p: &std::path::Path) -> Result<&str, ErroConv> {
    p.to_str().ok_or_else(|| ErroConv::Leitura(format!("caminho com caracteres inválidos: {}", p.display())))
}
