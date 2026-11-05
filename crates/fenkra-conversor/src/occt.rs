// SPDX-License-Identifier: LGPL-2.1-or-later
//! Provedor padrão: STEP pela ponte XDE do kernel e BREP pelo texto nativo do OpenCASCADE.

use crate::{texto_do_caminho, ErroConv, Formato, ModeloNeutro, OpcoesExportar, OpcoesImportar, Provedor};
use fenkra_kernel::{ModeloXde, OpcoesGravarXde, OpcoesLerXde, Shape};
use std::path::Path;

pub struct ProvedorOcct;

/// A árvore é receita, não forma: quem a lê é [`crate::arvore`], e nenhum provedor de geometria.
fn sem_geometria() -> ErroConv {
    ErroConv::Formato("árvore paramétrica não passa pelo provedor de geometria".into())
}

impl Provedor for ProvedorOcct {
    fn id(&self) -> &'static str {
        "OpenCASCADE"
    }

    fn formatos(&self) -> &'static [Formato] {
        &[Formato::Step, Formato::Brep]
    }

    fn ler(&self, formato: Formato, caminho: &Path, o: &OpcoesImportar) -> Result<ModeloNeutro, ErroConv> {
        match formato {
            Formato::Step => {
                let opcoes = OpcoesLerXde { cores: o.cores, nomes: o.nomes, camadas: o.camadas, pmi: true };
                Ok(ModeloNeutro { formato, xde: ModeloXde::ler(texto_do_caminho(caminho)?, opcoes)? })
            }
            Formato::Brep => {
                let texto = std::fs::read_to_string(caminho).map_err(|e| ErroConv::Leitura(format!("não foi possível ler o BREP: {e}")))?;
                let nome = caminho.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                Ok(ModeloNeutro::de_forma(formato, &nome, Shape::from_brep(&texto)?))
            }
            Formato::Arvore => Err(sem_geometria()),
        }
    }

    fn gravar(&self, formato: Formato, modelo: &ModeloNeutro, caminho: &Path, o: &OpcoesExportar) -> Result<(), ErroConv> {
        let falha = |e: fenkra_kernel::KernelError| ErroConv::Gravacao(e.0);
        match formato {
            Formato::Step => {
                let opcoes = OpcoesGravarXde { esquema: o.esquema.into(), unidade: o.unidade.into(), cores: o.cores, nomes: o.nomes, camadas: o.camadas, pmi: o.pmi };
                modelo.xde.gravar(texto_do_caminho(caminho)?, opcoes).map_err(falha)
            }
            Formato::Arvore => Err(sem_geometria()),
            // o BREP não tem estrutura: as peças vão soltas, cada uma no seu lugar
            Formato::Brep => {
                let mut tudo: Option<Shape> = None;
                for c in modelo.ocorrencias() {
                    let Some(forma) = modelo.xde.produtos[c.produto].forma.as_ref() else { continue };
                    let posta = forma.com_matriz(&c.local).map_err(falha)?;
                    tudo = Some(match tudo {
                        Some(t) => t.compound(&posta),
                        None => posta,
                    });
                }
                let tudo = tudo.ok_or_else(|| ErroConv::Gravacao("o modelo não tem forma para gravar".into()))?;
                std::fs::write(caminho, tudo.to_brep().map_err(falha)?).map_err(|e| ErroConv::Gravacao(format!("não foi possível gravar o BREP: {e}")))
            }
        }
    }
}
