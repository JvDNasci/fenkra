// SPDX-License-Identifier: LGPL-2.1-or-later
//! Formatos abertos de malha e visualização (B210).
//!
//! Todo formato daqui passa por uma [`Cena`]: malhas, materiais e uma árvore de nós, sempre em
//! milímetros e com Z para cima. Cada leitor traz o arquivo para esse acordo e cada gravador sai dele,
//! então N formatos custam N leitores e N gravadores, e não N x N conversões.
//!
//! A cena só vira [`ModeloNeutro`] quando alguém precisa de B-rep. Malha para malha não passa pelo
//! kernel: costurar milhões de triângulos para gravar de novo só gastaria tempo e perderia a cor por
//! vértice. O caminho até o B-rep tem teto, porque cada triângulo vira uma face.

mod amf;
mod collada;
mod fbx;
pub(crate) mod corpo;
mod gltf;
mod ifc;
mod nos3d;
mod ponte;
mod usd;
mod vrml;
mod x3d;

pub use fenkra_malha::{Cena, Malha, Material, No, ITENS_FORA};
pub(crate) use fenkra_malha::{mm_da_unidade, obj, ply, stl, tmf, xml, zip};
pub(crate) use ponte::LIMITE_BREP;

use crate::{Capacidades, ErroConv, Formato, ModeloNeutro, OpcoesExportar, OpcoesImportar, Provedor};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Como a geometria exata vira triângulos, e como a malha vai para o arquivo.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OpcoesMalha {
    /// quanto a malha pode se afastar da superfície, em mm
    pub desvio: f64,
    /// ângulo máximo entre normais vizinhas, em graus
    pub angulo: f64,
    /// STL e PLY têm versão texto e binária; ligado usa a binária
    pub binario: bool,
    /// grava as normais nos formatos que têm esse campo
    pub normais: bool,
}

impl Default for OpcoesMalha {
    fn default() -> Self {
        OpcoesMalha { desvio: 0.05, angulo: 15., binario: true, normais: true }
    }
}

/// Lê a cena de um arquivo, já em milímetros e com Z para cima.
pub(crate) fn ler_cena(formato: Formato, caminho: &Path) -> Result<Cena, ErroConv> {
    let bytes = std::fs::read(caminho).map_err(|e| ErroConv::Leitura(format!("não foi possível ler '{}': {e}", caminho.display())))?;
    let nome = caminho.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let mut cena = match formato {
        Formato::Stl => stl::ler(&bytes).map_err(ErroConv::from),
        Formato::Obj => obj::ler(&bytes, caminho).map_err(ErroConv::from),
        Formato::Ply => ply::ler(&bytes).map_err(ErroConv::from),
        Formato::Tmf => tmf::ler(&bytes).map_err(ErroConv::from),
        Formato::Amf => amf::ler(&bytes),
        Formato::Gltf | Formato::Glb => gltf::ler(&bytes, caminho),
        Formato::Vrml => vrml::ler(&bytes),
        Formato::X3d => x3d::ler(&bytes),
        Formato::Collada => collada::ler(&bytes),
        Formato::Ifc => ifc::ler(&bytes),
        Formato::Fbx => fbx::ler(&bytes),
        _ => Err(ErroConv::Formato(format!("o Fenkra grava {} mas não lê", formato.nome()))),
    }?;
    if cena.nome.is_empty() {
        cena.nome = nome;
    }
    cena.enxugar();
    if cena.malhas.iter().all(|m| m.tris.is_empty()) {
        return Err(ErroConv::Leitura(format!("o arquivo {} não tem triângulos", formato.nome())));
    }
    Ok(cena)
}

/// Grava a cena. O gravador recebe milímetros; a unidade pedida só muda os números do arquivo.
pub(crate) fn gravar_cena(formato: Formato, cena: &Cena, caminho: &Path, o: &OpcoesExportar) -> Result<(), ErroConv> {
    let (mm, nome) = unidade_de_saida(o);
    match formato {
        Formato::Stl => escrever(caminho, &stl::gravar(cena, o.malha.binario, mm)),
        Formato::Obj => obj::gravar(cena, caminho, mm, o.malha.normais).map_err(ErroConv::from),
        Formato::Ply => escrever(caminho, &ply::gravar(cena, o.malha.binario, mm)),
        Formato::Tmf => escrever(caminho, &tmf::gravar(cena, mm, nome)?),
        Formato::Amf => escrever(caminho, amf::gravar(cena, mm, nome).as_bytes()),
        Formato::Gltf => escrever(caminho, &gltf::gravar_texto(cena, o.malha.normais)?),
        Formato::Glb => escrever(caminho, &gltf::gravar_binario(cena, o.malha.normais)?),
        Formato::Vrml => escrever(caminho, vrml::gravar(cena).as_bytes()),
        Formato::X3d => escrever(caminho, x3d::gravar(cena).as_bytes()),
        Formato::Collada => escrever(caminho, collada::gravar(cena, mm, nome).as_bytes()),
        Formato::Usda => escrever(caminho, usd::gravar_texto(cena, mm).as_bytes()),
        Formato::Usdz => escrever(caminho, &usd::gravar_pacote(cena, mm)?),
        Formato::Ifc => escrever(caminho, ifc::gravar(cena, mm, caminho).as_bytes()),
        _ => Err(ErroConv::Formato(format!("{} não é formato de malha", formato.nome()))),
    }
}

/// Milímetros por unidade do arquivo e o nome dela, no vocabulário comum a 3MF, AMF e Collada.
fn unidade_de_saida(o: &OpcoesExportar) -> (f64, &'static str) {
    match o.unidade {
        crate::UnidadeSaida::Milimetro => (1., "millimeter"),
        crate::UnidadeSaida::Polegada => (25.4, "inch"),
        crate::UnidadeSaida::Metro => (1000., "meter"),
    }
}

fn escrever(caminho: &Path, bytes: &[u8]) -> Result<(), ErroConv> {
    std::fs::write(caminho, bytes).map_err(|e| ErroConv::Gravacao(format!("não foi possível gravar '{}': {e}", caminho.display())))
}

/// Falha de leitura com o nome do formato na frente.
fn erro(formato: &str, motivo: impl std::fmt::Display) -> ErroConv {
    ErroConv::Leitura(format!("{formato}: {motivo}"))
}

/// Arquivo FBX de prova feito do modelo, para a sonda conferir a leitura de um formato que só se lê.
pub(crate) fn amostra_fbx(modelo: &ModeloNeutro, caminho: &Path, o: &OpcoesExportar) -> Result<(), ErroConv> {
    escrever(caminho, &fbx::amostra_binaria(&ponte::de_modelo(modelo, o)?, true))
}

/// O conteúdo é FBX binário? (assinatura no começo)
pub(crate) fn fbx_binario(b: &[u8]) -> bool {
    fbx::e_binario(b)
}

/// O conteúdo é FBX em texto?
pub(crate) fn fbx_texto(b: &[u8]) -> bool {
    fbx::e_texto(b)
}

/// Provedor dos formatos de malha, em Rust puro: nenhum deles precisa do kernel para ser lido.
pub struct ProvedorMalha;

impl Provedor for ProvedorMalha {
    fn id(&self) -> &'static str {
        "Fenkra"
    }

    fn formatos(&self) -> &'static [Formato] {
        &Formato::MALHA
    }

    fn capacidades(&self, formato: Formato) -> Capacidades {
        let base = Capacidades { ler: true, gravar: true, malha: true, ..Default::default() };
        match formato {
            Formato::Stl => base,
            Formato::Obj | Formato::Ply => Capacidades { cor: true, ..base },
            Formato::Tmf | Formato::Amf | Formato::Gltf | Formato::Glb | Formato::Vrml | Formato::X3d | Formato::Collada => Capacidades { cor: true, montagem: true, unidade: true, ..base },
            Formato::Usda | Formato::Usdz => Capacidades { ler: false, cor: true, montagem: true, unidade: true, ..base },
            Formato::Ifc => Capacidades { cor: true, unidade: true, ..base },
            Formato::Fbx => Capacidades { gravar: false, cor: true, montagem: true, unidade: true, ..base },
            _ => Capacidades::default(),
        }
    }

    fn ler(&self, formato: Formato, caminho: &Path, o: &OpcoesImportar) -> Result<ModeloNeutro, ErroConv> {
        ponte::para_modelo(ler_cena(formato, caminho)?, formato, o)
    }

    fn gravar(&self, formato: Formato, modelo: &ModeloNeutro, caminho: &Path, o: &OpcoesExportar) -> Result<(), ErroConv> {
        gravar_cena(formato, &ponte::de_modelo(modelo, o)?, caminho, o)
    }
}

/// Lê a cena direto do arquivo, sem subir ao B-rep: é o que os testes conferem.
pub fn ler_para_teste(formato: Formato, caminho: &Path) -> Result<Cena, ErroConv> {
    ler_cena(formato, caminho)
}

/// Grava uma cena pronta, com as opções padrão.
pub fn gravar_para_teste(formato: Formato, cena: &Cena, caminho: &Path) -> Result<(), ErroConv> {
    gravar_cena(formato, cena, caminho, &OpcoesExportar::default())
}

/// Malha para malha, sem passar pelo B-rep.
pub(crate) fn converter(formato: Formato, origem: &Path, destino: Formato, caminho: &Path, o: &OpcoesImportar, saida: &OpcoesExportar) -> Result<(), ErroConv> {
    let mut cena = ler_cena(formato, origem)?;
    if let Some(mm) = o.unidade.em_mm() {
        cena.escalar(mm / crate::importar::declarada_em_mm(&cena.unidade));
    }
    if !o.cores || !saida.cores {
        cena.sem_cores();
    }
    gravar_cena(destino, &cena, caminho, saida)
}
