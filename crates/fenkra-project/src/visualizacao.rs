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

/// Subpasta dos pacotes de montagens e de arquivos fora do índice.
pub const PASTA_ARQUIVOS: &str = "arquivos";

/// Nome do arquivo sem a extensão, só com letras, números e sublinhado.
fn nome_seguro(p: &std::path::Path) -> String {
    let s: String = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default().chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if s.is_empty() {
        "arquivo".into()
    } else {
        s
    }
}

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

    /// Onde fica o pacote de um nó da árvore do produto (B364, B339): a peça do índice usa o pacote
    /// da versão atual; a montagem e o arquivo de fora, um pacote nomeado pela assinatura do nó, que
    /// muda quando qualquer peça de baixo muda.
    pub fn caminho_visualizacao_no(&self, n: &crate::arvore_produto::NoProduto) -> PathBuf {
        match (n.id, &n.versao) {
            (Some(id), Some(h)) => self.caminho_visualizacao(id, h),
            _ => self.root.join(PASTA).join(PASTA_ARQUIVOS).join(format!("{}-{}.fkv", nome_seguro(&n.arquivo), n.assinatura)),
        }
    }

    /// O pacote do nó, se já foi gerado.
    pub fn visualizacao_do_no(&self, n: &crate::arvore_produto::NoProduto) -> Option<PathBuf> {
        let p = self.caminho_visualizacao_no(n);
        p.is_file().then_some(p)
    }

    /// Pacote anterior do nó, para comparar com o atual (B339): na peça do índice, o da versão de
    /// antes que ainda tem pacote; na montagem, o mais novo do mesmo arquivo com outra assinatura.
    pub fn visualizacao_anterior_do_no(&self, n: &crate::arvore_produto::NoProduto) -> Option<PathBuf> {
        if let (Some(id), Some(h)) = (n.id, &n.versao) {
            let e = self.entry(id).ok()?;
            return e.versions.iter().rev().map(|v| v.hash.as_str()).filter(|x| x != h).find_map(|x| self.visualizacao_da_versao(id, x));
        }
        let atual = self.caminho_visualizacao_no(n);
        let prefixo = format!("{}-", nome_seguro(&n.arquivo));
        std::fs::read_dir(self.root.join(PASTA).join(PASTA_ARQUIVOS))
            .ok()?
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().strip_prefix(&prefixo).is_some_and(|r| r.len() == 20 && r.ends_with(".fkv")))
            .filter(|e| e.path() != atual)
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .max()
            .map(|(_, c)| c)
    }

    /// Grava o pacote de um nó. Fora do índice, apaga os antigos do mesmo arquivo e deixa os
    /// [`GUARDAR_POR_DOCUMENTO`] mais novos.
    pub fn gravar_visualizacao_no(&self, n: &crate::arvore_produto::NoProduto, bytes: &[u8]) -> PResult<PathBuf> {
        if let (Some(id), Some(h)) = (n.id, &n.versao) {
            return self.gravar_visualizacao(id, h, bytes);
        }
        self.pode_gravar()?;
        let destino = self.caminho_visualizacao_no(n);
        gravar_atomico(&destino, bytes)?;
        let prefixo = format!("{}-", nome_seguro(&n.arquivo));
        let mut antigos: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(self.root.join(PASTA).join(PASTA_ARQUIVOS))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().strip_prefix(&prefixo).is_some_and(|r| r.len() == 20 && r.ends_with(".fkv")))
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .filter(|(_, c)| *c != destino)
            .collect();
        antigos.sort();
        let sobra = antigos.len().saturating_sub(GUARDAR_POR_DOCUMENTO - 1);
        for (_, c) in antigos.into_iter().take(sobra) {
            let _ = std::fs::remove_file(c);
        }
        Ok(destino)
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
    fn pacote_de_montagem_pela_assinatura_e_poda_por_arquivo() {
        let d = pasta("montagem");
        let mut p = Project::create(&d, "P").unwrap();
        let id = p.add_document(peca("40")).unwrap();
        let f = p.entry(id).unwrap().file.replace('\\', "/");
        let conj = format!(r#"{{"format_version": 1, "name": "Conj", "nodes": [{{"id": 1, "name": "Base", "kind": {{"Part": {{"path": "{f}"}}}}}}]}}"#);
        std::fs::write(d.join("conj-1.fka"), conj).unwrap();
        let a = p.arvore_produto();
        let n = &a.nos[a.no_do_arquivo(&d.join("conj-1.fka")).unwrap()];
        let c = p.caminho_visualizacao_no(n);
        assert!(c.ends_with(format!("arquivos/conj_1-{}.fkv", n.assinatura)), "{}", c.display());
        assert!(p.visualizacao_do_no(n).is_none());
        assert_eq!(p.gravar_visualizacao_no(n, b"um").unwrap(), c);
        assert!(p.visualizacao_do_no(n).is_some());
        // a peça do índice usa o pacote da versão, o mesmo de "Ver no navegador"
        let np = &a.nos[a.no_do_documento(id).unwrap()];
        assert_eq!(p.caminho_visualizacao_no(np), p.caminho_visualizacao(id, &p.hash_da_versao(id, 1).unwrap()));
        // muitas edições da peça: assinaturas novas da montagem, e a pasta não cresce sem fim
        for k in 0..(GUARDAR_POR_DOCUMENTO + 3) {
            p.save(id, &peca(&format!("{}", 50 + k)), "edição").unwrap();
            let a = p.arvore_produto();
            let n = &a.nos[a.no_do_arquivo(&d.join("conj-1.fka")).unwrap()];
            p.gravar_visualizacao_no(n, b"x").unwrap();
            assert!(p.visualizacao_do_no(n).is_some(), "o mais novo nunca sai");
        }
        let total = std::fs::read_dir(d.join(PASTA).join(PASTA_ARQUIVOS)).unwrap().count();
        assert!(total <= GUARDAR_POR_DOCUMENTO, "{total} pacotes da montagem");
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
