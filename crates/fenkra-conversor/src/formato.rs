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
}

impl Formato {
    pub fn nome(self) -> &'static str {
        match self {
            Formato::Step => "STEP",
            Formato::Brep => "BREP",
        }
    }

    pub fn extensoes(self) -> &'static [&'static str] {
        match self {
            Formato::Step => &["step", "stp"],
            Formato::Brep => &["brep"],
        }
    }

    fn por_extensao(caminho: &Path) -> Option<Formato> {
        let e = caminho.extension()?.to_string_lossy().to_lowercase();
        [Formato::Step, Formato::Brep].into_iter().find(|f| f.extensoes().contains(&e.as_str()))
    }

    fn por_conteudo(inicio: &[u8]) -> Option<Formato> {
        let texto = String::from_utf8_lossy(&inicio[..inicio.len().min(512)]);
        let texto = texto.trim_start_matches('\u{feff}').trim_start();
        if texto.starts_with("ISO-10303-21") {
            Some(Formato::Step)
        } else if texto.starts_with("DBRep_DrawableShape") || texto.starts_with("CASCADE Topology") {
            Some(Formato::Brep)
        } else {
            None
        }
    }
}

/// Formato do arquivo pelo conteúdo, e se a extensão diz outra coisa.
pub fn detectar(caminho: &Path, conteudo: &[u8]) -> Result<(Formato, bool), ErroConv> {
    let Some(f) = Formato::por_conteudo(conteudo) else {
        return Err(ErroConv::Formato(format!(
            "'{}' não parece STEP nem BREP; o Fenkra lê formatos neutros: exporte STEP AP214 ou AP242 no programa de origem",
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
    fn texto_qualquer_e_recusado_com_orientacao() {
        let e = detectar(Path::new("peca.sldprt"), b"qualquer coisa").unwrap_err();
        assert!(e.to_string().contains("STEP AP214"), "{e}");
    }
}
