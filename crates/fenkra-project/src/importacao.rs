// SPDX-License-Identifier: LGPL-2.1-or-later
//! Documentos importados no projeto (B229).
//!
//! O arquivo original e o relatório de conversão ficam em `.originais/`, endereçados pelo hash do
//! conteúdo, como as versões: dois documentos do mesmo arquivo dividem uma cópia, e a cópia não muda
//! depois de gravada. O selo dentro do documento diz de qual original ele nasceu; reimportar compara
//! o hash do arquivo de agora com o do selo.

use crate::seguro::gravar_atomico;
use crate::{PResult, Project};
use fenkra_core::{Document, RefKind};
use std::path::{Path, PathBuf};

const ORIGINAIS: &str = ".originais";
const MONTAGENS: &str = "montagens";

/// O que reimportar um arquivo significa para o projeto.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reimportacao {
    /// nenhum documento veio de um arquivo com este nome
    Nova,
    /// o documento `id` já veio deste mesmo conteúdo
    Igual { id: u64 },
    /// o documento `id` veio de um arquivo com este nome, mas o conteúdo mudou
    Mudou { id: u64, hash_anterior: String },
}

impl Project {
    fn pasta_originais(&self) -> PathBuf {
        self.root.join(ORIGINAIS)
    }

    /// Caminho do original guardado no cofre para um hash, com a extensão dada.
    pub fn original_no_cofre(&self, hash: &str, extensao: &str) -> PathBuf {
        self.pasta_originais().join(format!("{hash}.{extensao}"))
    }

    /// Caminho do relatório de conversão guardado no cofre.
    pub fn relatorio_no_cofre(&self, hash: &str) -> PathBuf {
        self.pasta_originais().join(format!("{hash}.relatorio.txt"))
    }

    fn guardar_original(&self, original: &Path, hash: &str, relatorio: &str) -> PResult<()> {
        self.pode_gravar()?;
        let extensao = original.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_else(|| "bin".into());
        let destino = self.original_no_cofre(hash, &extensao);
        if !destino.exists() {
            gravar_atomico(&destino, &std::fs::read(original)?)?;
        }
        gravar_atomico(&self.relatorio_no_cofre(hash), relatorio.as_bytes())?;
        Ok(())
    }

    /// Compara um arquivo pelo nome e pelo hash com o que o projeto já importou. Vale o documento
    /// mais recente com esse nome de origem, que é o que a pessoa atualizaria.
    pub fn reimportacao(&self, nome: &str, hash: &str) -> Reimportacao {
        let mesmos = self.index.documents.iter().rev().filter_map(|d| d.origem.as_ref().map(|o| (d.id, o))).filter(|(_, o)| o.nome == nome);
        let mut mais_recente = None;
        for (id, o) in mesmos {
            if o.hash == hash {
                return Reimportacao::Igual { id };
            }
            mais_recente.get_or_insert(Reimportacao::Mudou { id, hash_anterior: o.hash.clone() });
        }
        mais_recente.unwrap_or(Reimportacao::Nova)
    }

    /// Adiciona o documento importado e guarda o original e o relatório no cofre.
    pub fn adicionar_importado(&mut self, original: &Path, doc: Document, relatorio: &str) -> PResult<u64> {
        let hash = doc.origem.as_ref().map(|o| o.hash.clone()).unwrap_or_default();
        self.guardar_original(original, &hash, relatorio)?;
        self.add_document(doc)
    }

    /// Adiciona as peças de um arquivo de montagem. Cada uma ganha PN do motor, uma vez por peça: as
    /// ocorrências dela não são documentos. Devolve o nome de cada peça no conjunto e o caminho dela
    /// relativo à pasta das montagens, que é o que os nós da montagem guardam.
    pub fn adicionar_pecas_importadas(&mut self, original: &Path, pecas: Vec<(String, Document)>, relatorio: &str) -> PResult<Vec<(String, String)>> {
        let hash = pecas.first().and_then(|(_, d)| d.origem.as_ref()).map(|o| o.hash.clone()).unwrap_or_default();
        if !hash.is_empty() {
            self.guardar_original(original, &hash, relatorio)?;
        }
        let mut saida = Vec::with_capacity(pecas.len());
        for (nome, doc) in pecas {
            let id = self.add_document(doc)?;
            saida.push((nome, format!("../{}", self.entry(id)?.file)));
        }
        Ok(saida)
    }

    /// Caminho que uma montagem importada com este nome de arquivo tem no projeto.
    pub fn montagem_importada(&self, arquivo: &str) -> PathBuf {
        self.root.join(MONTAGENS).join(arquivo)
    }

    /// Grava a montagem de um arquivo importado na pasta das montagens.
    pub fn gravar_montagem_importada(&self, arquivo: &str, texto: &str) -> PResult<PathBuf> {
        self.pode_gravar()?;
        let destino = self.montagem_importada(arquivo);
        if let Some(pasta) = destino.parent() {
            std::fs::create_dir_all(pasta)?;
        }
        gravar_atomico(&destino, texto.as_bytes())?;
        Ok(destino)
    }

    /// Cria um documento novo a partir do arquivo mudado, ligado ao documento `de`, que fica como está.
    pub fn adicionar_derivado(&mut self, de: u64, original: &Path, mut doc: Document, relatorio: &str) -> PResult<u64> {
        let base = self.entry(de)?.uid.clone();
        if let Some(o) = &mut doc.origem {
            o.derivado_de = Some(base);
        }
        self.adicionar_importado(original, doc, relatorio)
    }

    /// Troca a geometria importada do documento `id` pela de `novo`, mantendo os comandos que a
    /// pessoa fez depois da importação, e grava o selo do arquivo novo.
    pub fn atualizar_importado(&mut self, id: u64, original: &Path, novo: &Document, relatorio: &str) -> PResult<()> {
        let mut doc = self.load(id)?;
        let (atuais, trocadas) = (geometria_importada(&doc), geometria_importada(novo));
        if atuais.is_empty() {
            return Err(crate::ProjectError::Importacao("o documento não tem corpo importado".into()));
        }
        // sem a mesma contagem não há como saber qual corpo virou qual: os comandos feitos depois
        // iriam parar em outra geometria sem ninguém ver
        if atuais.len() != trocadas.len() {
            return Err(crate::ProjectError::Importacao(format!("o arquivo tinha {} corpos e agora tem {}: crie um derivado", atuais.len(), trocadas.len())));
        }
        for (atual, trocada) in atuais.into_iter().zip(trocadas) {
            let kind = novo.references.iter().find(|r| r.id == trocada).map(|r| r.kind.clone());
            if let (Some(r), Some(kind)) = (doc.references.iter_mut().find(|r| r.id == atual), kind) {
                r.kind = kind;
            }
        }
        doc.origem = novo.origem.clone();
        let hash = doc.origem.as_ref().map(|o| o.hash.clone()).unwrap_or_default();
        self.guardar_original(original, &hash, relatorio)?;
        self.save(id, &doc, "reimportado")
    }
}

/// Elementos de referência com a geometria importada, na ordem do arquivo. Todo corpo e toda
/// superfície aberta nascem como geometria isolada.
fn geometria_importada(doc: &Document) -> Vec<fenkra_core::Id> {
    doc.references.iter().filter(|r| matches!(r.kind, RefKind::Isolated { .. })).map(|r| r.id).collect()
}
