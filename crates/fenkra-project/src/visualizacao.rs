// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pré-visualização do projeto pelo pacote `.fkv` (B339, ADR 0066).
//!
//! O projeto guarda **só a opção e os arquivos**: quem gera o pacote é o aplicativo, que enxerga o
//! kernel (`fenkra-pacote` fica acima deste crate). Aqui moram o que o PLM decide sozinho: se o
//! pacote é gerado ao salvar, onde ele fica, qual versão ainda não tem pacote e o que apagar
//! para a pasta não crescer sem fim.
//!
//! ```text
//! MeuProjeto/
//! ├── .visualizacao.json              opção do projeto (desligada por padrão)
//! └── .visualizacao/<id>-<hash>.fkv   um pacote por versão, nomeado pelo conteúdo da versão
//! ```
//!
//! O nome leva o hash da versão, que é imutável: o pacote de uma versão nunca fica velho, e comparar
//! duas versões é abrir dois arquivos que já existem.

use crate::seguro::gravar_atomico;
use crate::{PResult, Project, ProjectError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const ARQUIVO_OPCOES: &str = ".visualizacao.json";
pub const PASTA: &str = ".visualizacao";

/// Quantos pacotes de versões de trabalho ficam por documento. Os de revisão liberada nunca saem.
pub const GUARDAR_POR_DOCUMENTO: usize = 6;

/// Opção do projeto, não do documento: vale para quem abre a pasta.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OpcoesVisualizacao {
    /// gera o pacote em segundo plano depois de gravar uma versão nova
    #[serde(default)]
    pub gerar_ao_salvar: bool,
    /// decima a malha e tira os pontos de captura exatos (protege a geometria)
    #[serde(default)]
    pub simplificada: bool,
}

impl Project {
    pub fn opcoes_visualizacao(&self) -> OpcoesVisualizacao {
        std::fs::read_to_string(self.root.join(ARQUIVO_OPCOES)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn definir_opcoes_visualizacao(&mut self, o: &OpcoesVisualizacao) -> PResult<()> {
        self.pode_gravar()?;
        gravar_atomico(&self.root.join(ARQUIVO_OPCOES), serde_json::to_string_pretty(o)?.as_bytes())?;
        Ok(())
    }

    /// Hash da versão `numero` (a partir de 1) do documento.
    pub fn hash_da_versao(&self, id: u64, numero: usize) -> PResult<String> {
        let e = self.entry(id)?;
        numero.checked_sub(1).and_then(|i| e.versions.get(i)).map(|v| v.hash.clone()).ok_or(ProjectError::NotFound)
    }

    pub fn caminho_visualizacao(&self, id: u64, hash: &str) -> PathBuf {
        self.root.join(PASTA).join(format!("{id}-{hash}.fkv"))
    }

    /// O pacote da versão, se já foi gerado.
    pub fn visualizacao_da_versao(&self, id: u64, hash: &str) -> Option<PathBuf> {
        let p = self.caminho_visualizacao(id, hash);
        p.is_file().then_some(p)
    }

    /// Hash da versão atual quando ela ainda não tem pacote. É o que o aplicativo consulta depois
    /// de gravar: `None` quando já existe (nada a fazer) ou quando o documento não tem versão.
    pub fn visualizacao_pendente(&self, id: u64) -> Option<String> {
        let hash = self.entry(id).ok()?.versions.last()?.hash.clone();
        self.visualizacao_da_versao(id, &hash).is_none().then_some(hash)
    }

    /// Grava o pacote da versão `hash` e apaga os mais antigos de versões de trabalho.
    pub fn gravar_visualizacao(&self, id: u64, hash: &str, bytes: &[u8]) -> PResult<PathBuf> {
        self.pode_gravar()?;
        let destino = self.caminho_visualizacao(id, hash);
        gravar_atomico(&destino, bytes)?;
        let _ = self.podar_visualizacoes(id);
        Ok(destino)
    }

    /// Mantém os pacotes das revisões liberadas e dos [`GUARDAR_POR_DOCUMENTO`] mais recentes.
    fn podar_visualizacoes(&self, id: u64) -> PResult<usize> {
        let e = self.entry(id)?;
        let liberadas: Vec<&str> = e.revisions.iter().map(|r| r.hash.as_str()).collect();
        // do mais novo para o mais antigo, pela ordem das versões (o tempo do arquivo não é confiável)
        let mut recentes: Vec<&str> = vec![];
        for v in e.versions.iter().rev() {
            if !recentes.contains(&v.hash.as_str()) {
                recentes.push(v.hash.as_str());
            }
            if recentes.len() == GUARDAR_POR_DOCUMENTO {
                break;
            }
        }
        let prefixo = format!("{id}-");
        let mut apagados = 0;
        let Ok(rd) = std::fs::read_dir(self.root.join(PASTA)) else { return Ok(0) };
        for entrada in rd.flatten() {
            let nome = entrada.file_name().to_string_lossy().to_string();
            let Some(hash) = nome.strip_prefix(&prefixo).and_then(|r| r.strip_suffix(".fkv")) else { continue };
            if !liberadas.contains(&hash) && !recentes.contains(&hash) && std::fs::remove_file(entrada.path()).is_ok() {
                apagados += 1;
            }
        }
        Ok(apagados)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{BoolOp, FeatureKind, OriginPlane, PlaneRef, PrimKind, Primitive};

    fn pasta(nome: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fenkra_viz_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn peca(lado: &str) -> fenkra_core::Document {
        let s = |x: &str| x.to_string();
        let mut d = fenkra_core::Document::new("Placa");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s(lado), b: s("20"), c: s("5"), d: s("0"), op: BoolOp::Add }));
        d
    }

    #[test]
    fn desligada_por_padrao_e_grava_no_projeto() {
        let d = pasta("opcao");
        let mut p = Project::create(&d, "P").unwrap();
        assert_eq!(p.opcoes_visualizacao(), OpcoesVisualizacao { gerar_ao_salvar: false, simplificada: false });
        p.definir_opcoes_visualizacao(&OpcoesVisualizacao { gerar_ao_salvar: true, simplificada: true }).unwrap();
        let reaberto = Project::open(&d).unwrap();
        assert!(reaberto.opcoes_visualizacao().gerar_ao_salvar && reaberto.opcoes_visualizacao().simplificada);
        // arquivo ilegível volta ao padrão em vez de travar o projeto
        std::fs::write(d.join(ARQUIVO_OPCOES), "{ lixo").unwrap();
        assert!(!Project::open(&d).unwrap().opcoes_visualizacao().gerar_ao_salvar);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn versao_nova_fica_pendente_ate_o_pacote_ser_gravado() {
        let d = pasta("pendente");
        let mut p = Project::create(&d, "P").unwrap();
        let id = p.add_document(peca("40")).unwrap();
        let h1 = p.visualizacao_pendente(id).expect("a primeira versão não tem pacote");
        assert_eq!(h1, p.hash_da_versao(id, 1).unwrap());
        let caminho = p.gravar_visualizacao(id, &h1, b"PK-um").unwrap();
        assert_eq!(std::fs::read(&caminho).unwrap(), b"PK-um");
        assert!(p.visualizacao_pendente(id).is_none(), "já tem pacote");
        // editar cria outra versão, que volta a ficar pendente; a antiga continua guardada
        p.save(id, &peca("50"), "editado").unwrap();
        let h2 = p.visualizacao_pendente(id).expect("a versão nova não tem pacote");
        assert_ne!(h1, h2);
        assert!(p.visualizacao_da_versao(id, &h1).is_some(), "o pacote da versão antiga serve para comparar");
        // um documento sem versão nunca fica pendente
        assert!(p.visualizacao_pendente(9999).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn poda_guarda_as_recentes_e_nunca_a_revisao_liberada() {
        let d = pasta("poda");
        let mut p = Project::create(&d, "P").unwrap();
        let id = p.add_document(peca("10")).unwrap();
        let primeira = p.hash_da_versao(id, 1).unwrap();
        p.gravar_visualizacao(id, &primeira, b"a").unwrap();
        p.release(id, "liberada").unwrap();
        // muitas versões de trabalho depois da revisão
        let mut hashes = vec![];
        for n in 0..(GUARDAR_POR_DOCUMENTO + 3) {
            p.save(id, &peca(&format!("{}", 20 + n)), "edição").unwrap();
            let h = p.visualizacao_pendente(id).unwrap();
            p.gravar_visualizacao(id, &h, b"x").unwrap();
            hashes.push(h);
        }
        assert!(p.visualizacao_da_versao(id, &primeira).is_some(), "a revisão liberada é imutável e fica");
        assert!(p.visualizacao_da_versao(id, hashes.last().unwrap()).is_some(), "a mais nova fica");
        assert!(p.visualizacao_da_versao(id, &hashes[0]).is_none(), "as mais antigas de trabalho saem");
        let total = std::fs::read_dir(d.join(PASTA)).unwrap().count();
        assert!(total <= GUARDAR_POR_DOCUMENTO + 1, "{total} pacotes");
        let _ = std::fs::remove_dir_all(&d);
    }
}
