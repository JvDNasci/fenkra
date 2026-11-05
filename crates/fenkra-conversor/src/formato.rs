// SPDX-License-Identifier: LGPL-2.1-or-later
//! Formatos que o conversor conhece, reconhecidos pelo conteúdo. A extensão é só uma dica: um STEP
//! renomeado continua sendo STEP, e o relatório avisa da divergência.

use crate::ErroConv;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Formato {
    Step,
    Brep,
    /// árvore paramétrica aberta (B266): JSON, sem geometria
    Arvore,
}

impl Formato {
    pub fn nome(self) -> &'static str {
        match self {
            Formato::Step => "STEP",
            Formato::Brep => "BREP",
            Formato::Arvore => "Árvore paramétrica",
        }
    }

    pub fn extensoes(self) -> &'static [&'static str] {
        match self {
            Formato::Step => &["step", "stp"],
            Formato::Brep => &["brep"],
            Formato::Arvore => &["fkx"],
        }
    }

    fn por_extensao(caminho: &Path) -> Option<Formato> {
        let e = caminho.extension()?.to_string_lossy().to_lowercase();
        [Formato::Step, Formato::Brep, Formato::Arvore].into_iter().find(|f| f.extensoes().contains(&e.as_str()))
    }

    fn por_conteudo(conteudo: &[u8]) -> Option<Formato> {
        let texto = String::from_utf8_lossy(&conteudo[..conteudo.len().min(512)]);
        let texto = texto.trim_start_matches('\u{feff}').trim_start();
        if texto.starts_with("ISO-10303-21") {
            Some(Formato::Step)
        } else if texto.starts_with("DBRep_DrawableShape") || texto.starts_with("CASCADE Topology") {
            Some(Formato::Brep)
        } else if texto.starts_with('{') && tem_marca_da_arvore(conteudo) {
            Some(Formato::Arvore)
        } else {
            None
        }
    }
}

/// O campo `formato` pode estar em qualquer lugar do objeto: quem escreve o JSON escolhe a ordem.
fn tem_marca_da_arvore(conteudo: &[u8]) -> bool {
    let marca = crate::arvore::NOME_FORMATO.as_bytes();
    conteudo.windows(marca.len()).any(|w| w == marca)
}

/// Formato do arquivo pelo conteúdo, e se a extensão diz outra coisa.
pub fn detectar(caminho: &Path, conteudo: &[u8]) -> Result<(Formato, bool), ErroConv> {
    let Some(f) = Formato::por_conteudo(conteudo) else {
        return Err(ErroConv::Formato(format!(
            "'{}' não parece STEP, BREP nem árvore paramétrica; o Fenkra lê formatos neutros: exporte STEP AP214 ou AP242 no programa de origem",
            caminho.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
        )));
    };
    Ok((f, Formato::por_extensao(caminho) != Some(f)))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn conteudo_manda_e_extensao_diferente_e_avisada() {
        assert_eq!(detectar(Path::new("a.step"), b"ISO-10303-21;\nHEADER;").unwrap(), (Formato::Step, false));
        assert_eq!(detectar(Path::new("a.dat"), b"ISO-10303-21;\nHEADER;").unwrap(), (Formato::Step, true));
        assert_eq!(detectar(Path::new("a.brep"), b"DBRep_DrawableShape\n\nCASCADE Topology V3").unwrap(), (Formato::Brep, false));
    }

    #[test]
    fn arvore_e_reconhecida_com_o_campo_em_qualquer_lugar() {
        let tardio = format!("{{\"pecas\": [], \"gerador\": \"{}\", \"formato\": \"fenkra-arvore\"}}", "x".repeat(600));
        assert_eq!(detectar(Path::new("a.fkx"), tardio.as_bytes()).unwrap(), (Formato::Arvore, false));
        assert_eq!(detectar(Path::new("a.json"), tardio.as_bytes()).unwrap(), (Formato::Arvore, true));
        assert!(detectar(Path::new("a.json"), b"{\"outro\": 1}").is_err());
    }

    #[test]
    fn texto_qualquer_e_recusado_com_orientacao() {
        let e = detectar(Path::new("peca.sldprt"), b"qualquer coisa").unwrap_err();
        assert!(e.to_string().contains("STEP AP214"), "{e}");
    }
}
