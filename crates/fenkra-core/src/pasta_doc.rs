// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pasta do documento na sessão (B454): de onde os arquivos externos da peça, a malha importada e as
//! imagens de textura, são lidos quando o caminho gravado é relativo.
//!
//! Por que existe: o recálculo só recebe o [`crate::Document`], sem saber onde o arquivo mora. Antes
//! a malha com caminho relativo era lida a partir da pasta de trabalho do programa, que muda conforme
//! quem abre o Fenkra (atalho, terminal, outra máquina). No pacote e na cópia para o projeto o caminho
//! é reescrito relativo à peça, então ele só vale visto da pasta da peça.
//!
//! A pasta não é gravada no arquivo e não conta na igualdade (como `app_version`): o mesmo documento
//! aberto de dois lugares continua igual. Quem lê a peça do disco chama [`crate::Document::na_pasta`].

use std::path::{Path, PathBuf};

/// Pasta onde o documento está gravado, conhecida só durante a sessão.
#[derive(Clone, Debug, Default)]
pub struct PastaDoc(pub Option<PathBuf>);

impl PartialEq for PastaDoc {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// Arquivo externo (malha, textura) que um caminho gravado na peça indica, em ordem:
/// 1. caminho absoluto vale como está (arquivos antigos gravavam assim);
/// 2. relativo à pasta do documento, se lá existir;
/// 3. relativo à pasta de trabalho, se lá existir (como o comando lia antes do B454);
/// 4. senão, o lugar ao lado do documento (a mensagem de erro aponta onde ele deveria estar).
///
/// É a mesma ordem de `fenkra_project::relacoes::arquivo_da_ref`, que a cópia para o projeto, o
/// pacote e o onde-usado seguem: quem copia e quem lê acham o mesmo arquivo.
pub fn resolver(pasta: Option<&Path>, caminho: &str) -> PathBuf {
    let p = Path::new(caminho);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    let Some(dir) = pasta else { return p.to_path_buf() };
    let perto = dir.join(p);
    if !perto.is_file() && p.is_file() {
        return p.to_path_buf();
    }
    perto
}

impl crate::Document {
    /// Marca a pasta do arquivo de onde o documento foi lido (ou para onde foi gravado).
    pub fn na_pasta(mut self, arquivo: &Path) -> Self {
        self.definir_pasta(arquivo);
        self
    }

    /// Como [`Self::na_pasta`], para quem já tem o documento em mão (Salvar como, abrir).
    pub fn definir_pasta(&mut self, arquivo: &Path) {
        self.pasta = PastaDoc(arquivo.parent().map(|d| std::path::absolute(d).unwrap_or_else(|_| d.to_path_buf())));
    }

    pub fn pasta(&self) -> Option<&Path> {
        self.pasta.0.as_deref()
    }

    /// Arquivo externo indicado por `caminho` (ver [`resolver`]).
    pub fn arquivo_externo(&self, caminho: &str) -> PathBuf {
        resolver(self.pasta(), caminho)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(nome: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fenkra-pasta-doc-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn relativo_le_ao_lado_do_documento_e_absoluto_vale_como_esta() {
        let d = tmp("rel");
        std::fs::create_dir_all(d.join("recursos")).unwrap();
        std::fs::write(d.join("recursos").join("a.stl"), b"x").unwrap();
        let doc = crate::Document::new("P").na_pasta(&d.join("p.fkp"));
        assert_eq!(doc.arquivo_externo("recursos/a.stl"), std::path::absolute(&d).unwrap().join("recursos/a.stl"));
        let abs = d.join("recursos").join("a.stl");
        assert_eq!(doc.arquivo_externo(&abs.to_string_lossy()), abs);
        // sem pasta (documento novo, nunca gravado): o caminho fica como está
        assert_eq!(crate::Document::new("Q").arquivo_externo("recursos/a.stl"), Path::new("recursos/a.stl"));
    }

    #[test]
    fn pasta_fica_fora_do_arquivo_e_da_igualdade() {
        let a = crate::Document::new("P").na_pasta(Path::new("/um/lugar/p.fkp"));
        let mut b = a.clone();
        b.definir_pasta(Path::new("/outro/p.fkp"));
        assert_eq!(a, b);
        assert!(!a.to_json().contains("/um/lugar"));
        assert!(crate::Document::from_json(&a.to_json()).unwrap().pasta().is_none());
    }
}
