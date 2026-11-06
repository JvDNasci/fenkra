// SPDX-License-Identifier: LGPL-2.1-or-later
//! Interface de provedor: quem sabe ler e gravar um formato. O OpenCASCADE é o provedor padrão; outro
//! provedor entra no registro sem mudar diálogos nem relatórios.

use crate::{ErroConv, Formato, ModeloNeutro, OpcoesExportar, OpcoesImportar, ProvedorOcct};
use std::path::Path;

pub trait Provedor: Send + Sync {
    fn id(&self) -> &'static str;
    fn formatos(&self) -> &'static [Formato];
    /// Lê o arquivo, com a geometria em milímetros na unidade que ele declara.
    fn ler(&self, formato: Formato, caminho: &Path, opcoes: &OpcoesImportar) -> Result<ModeloNeutro, ErroConv>;
    fn gravar(&self, formato: Formato, modelo: &ModeloNeutro, caminho: &Path, opcoes: &OpcoesExportar) -> Result<(), ErroConv>;
}

/// Provedores disponíveis; o primeiro que aceita o formato atende.
pub struct Registro {
    provedores: Vec<Box<dyn Provedor>>,
}

impl Default for Registro {
    fn default() -> Self {
        Registro { provedores: vec![Box::new(ProvedorOcct)] }
    }
}

impl Registro {
    pub fn vazio() -> Registro {
        Registro { provedores: Vec::new() }
    }

    pub fn registrar(&mut self, p: Box<dyn Provedor>) {
        self.provedores.push(p);
    }

    pub fn formatos(&self) -> Vec<Formato> {
        let mut v: Vec<Formato> = self.provedores.iter().flat_map(|p| p.formatos().iter().copied()).collect();
        v.dedup();
        v
    }

    pub(crate) fn provedor(&self, formato: Formato) -> Result<&dyn Provedor, ErroConv> {
        self.provedores
            .iter()
            .find(|p| p.formatos().contains(&formato))
            .map(|p| p.as_ref())
            .ok_or_else(|| ErroConv::Formato(format!("nenhum provedor lê {}", formato.nome())))
    }

    /// Grava no formato pedido pela extensão do destino. Extensão desconhecida é erro: gravar STEP em
    /// `.xyz` faria o outro programa falhar sem dizer por quê.
    pub fn gravar(&self, modelo: &ModeloNeutro, caminho: &Path, opcoes: &OpcoesExportar) -> Result<(), ErroConv> {
        let extensao = caminho.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let formato = self
            .formatos()
            .into_iter()
            .find(|f| f.extensoes().contains(&extensao.as_str()))
            .ok_or_else(|| ErroConv::Formato(format!("extensão desconhecida: '{extensao}'")))?;
        if opcoes.estrutura == crate::Estrutura::Achatada && modelo.tem_montagem() {
            return self.provedor(formato)?.gravar(formato, &modelo.achatado()?, caminho, opcoes);
        }
        self.provedor(formato)?.gravar(formato, modelo, caminho, opcoes)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    struct SoRecusa;

    impl Provedor for SoRecusa {
        fn id(&self) -> &'static str {
            "teste"
        }
        fn formatos(&self) -> &'static [Formato] {
            &[Formato::Brep]
        }
        fn ler(&self, _: Formato, _: &Path, _: &OpcoesImportar) -> Result<ModeloNeutro, ErroConv> {
            Err(ErroConv::Leitura("recusado".into()))
        }
        fn gravar(&self, _: Formato, _: &ModeloNeutro, _: &Path, _: &OpcoesExportar) -> Result<(), ErroConv> {
            Err(ErroConv::Gravacao("recusado".into()))
        }
    }

    #[test]
    fn provedor_registrado_atende_so_o_seu_formato() {
        let mut r = Registro::vazio();
        r.registrar(Box::new(SoRecusa));
        assert_eq!(r.formatos(), vec![Formato::Brep]);
        assert_eq!(r.provedor(Formato::Brep).ok().map(|p| p.id()), Some("teste"));
        assert!(r.provedor(Formato::Step).is_err());
    }

    #[test]
    fn registro_padrao_lista_formatos_e_registro_vazio_explica() {
        assert_eq!(Registro::default().formatos(), vec![Formato::Step, Formato::Iges, Formato::Brep]);
        let e = Registro::vazio().provedor(Formato::Step).err().unwrap();
        assert!(e.to_string().contains("nenhum provedor"), "{e}");
    }
}
