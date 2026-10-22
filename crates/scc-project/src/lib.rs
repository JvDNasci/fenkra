//! Gestão de projeto simples (ADR 0009, RF-GPR).
//!
//! ```text
//! MeuProjeto/
//! ├── projeto.scc.json      índice: documentos, PN, revisões
//! ├── pecas/<id>.sccpart    documentos de trabalho
//! └── .versoes/<hash>.sccpart   versões imutáveis (endereçadas pelo conteúdo)
//! ```
//!
//! - Documentos são identificados por um ID estável, não pelo nome do arquivo (RF-GPR-021).
//! - Cada salvamento cria uma **versão** (RF-GPR-010).
//! - **Liberar revisão** marca a versão atual com uma letra e a congela (RF-GPR-011).

use scc_core::Document;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const INDEX: &str = "projeto.scc.json";
const PARTS: &str = "pecas";
const VERSIONS: &str = ".versoes";

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("erro de arquivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("arquivo inválido: {0}")]
    Json(#[from] serde_json::Error),
    #[error("documento não encontrado no projeto")]
    NotFound,
    #[error("PN já usado por outro documento: {0}")]
    DuplicatePn(String),
    #[error("não há alterações desde a última revisão")]
    NothingToRelease,
}

pub type PResult<T> = Result<T, ProjectError>;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DocState {
    EmTrabalho,
    Liberado,
    Obsoleto,
}

impl DocState {
    pub fn label(self) -> &'static str {
        match self {
            DocState::EmTrabalho => "Em trabalho",
            DocState::Liberado => "Liberado",
            DocState::Obsoleto => "Obsoleto",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Version {
    pub hash: String,
    pub time: u64,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Revision {
    pub label: String,
    pub hash: String,
    pub time: u64,
    pub comment: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocEntry {
    pub id: u64,
    pub file: String,
    pub name: String,
    pub pn: String,
    pub description: String,
    pub state: DocState,
    pub versions: Vec<Version>,
    pub revisions: Vec<Revision>,
}

impl DocEntry {
    pub fn current_revision(&self) -> Option<&Revision> {
        self.revisions.last()
    }
    /// Há alterações desde a última revisão liberada?
    pub fn dirty_since_release(&self) -> bool {
        match (self.versions.last(), self.revisions.last()) {
            (Some(v), Some(r)) => v.hash != r.hash,
            (Some(_), None) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Index {
    pub format_version: u32,
    pub name: String,
    pub pn_prefix: String,
    pub next_pn: u32,
    next_id: u64,
    pub documents: Vec<DocEntry>,
}

pub struct Project {
    pub root: PathBuf,
    pub index: Index,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Próxima letra de revisão: A, B, ..., Z, AA, AB...
pub fn next_label(prev: Option<&str>) -> String {
    let Some(p) = prev else { return "A".into() };
    let mut chars: Vec<u8> = p.bytes().collect();
    let mut i = chars.len();
    loop {
        if i == 0 {
            chars.insert(0, b'A');
            break;
        }
        i -= 1;
        if chars[i] == b'Z' {
            chars[i] = b'A';
        } else {
            chars[i] += 1;
            break;
        }
    }
    String::from_utf8(chars).unwrap()
}

impl Project {
    pub fn create(root: &Path, name: &str) -> PResult<Project> {
        std::fs::create_dir_all(root.join(PARTS))?;
        std::fs::create_dir_all(root.join(VERSIONS))?;
        let p = Project {
            root: root.into(),
            index: Index { format_version: 1, name: name.into(), pn_prefix: "SCC-".into(), next_pn: 1, next_id: 1, documents: vec![] },
        };
        p.save_index()?;
        Ok(p)
    }

    pub fn open(root: &Path) -> PResult<Project> {
        let s = std::fs::read_to_string(root.join(INDEX))?;
        Ok(Project { root: root.into(), index: serde_json::from_str(&s)? })
    }

    pub fn save_index(&self) -> PResult<()> {
        let tmp = self.root.join(format!("{INDEX}.tmp"));
        std::fs::write(&tmp, serde_json::to_string_pretty(&self.index)?)?;
        std::fs::rename(tmp, self.root.join(INDEX))?;
        Ok(())
    }

    pub fn entry(&self, id: u64) -> PResult<&DocEntry> {
        self.index.documents.iter().find(|d| d.id == id).ok_or(ProjectError::NotFound)
    }
    fn entry_mut(&mut self, id: u64) -> PResult<&mut DocEntry> {
        self.index.documents.iter_mut().find(|d| d.id == id).ok_or(ProjectError::NotFound)
    }

    /// Numeração automática de PN (RF-GPR-005).
    pub fn next_pn(&mut self) -> String {
        let pn = format!("{}{:04}", self.index.pn_prefix, self.index.next_pn);
        self.index.next_pn += 1;
        pn
    }

    /// Adiciona um documento novo ao projeto e grava a primeira versão.
    pub fn add_document(&mut self, mut doc: Document) -> PResult<u64> {
        if doc.props.pn.is_empty() {
            doc.props.pn = self.next_pn();
        }
        self.check_pn(None, &doc.props.pn)?;
        let id = self.index.next_id;
        self.index.next_id += 1;
        self.index.documents.push(DocEntry {
            id,
            file: format!("{PARTS}/{id:05}.sccpart"),
            name: doc.name.clone(),
            pn: doc.props.pn.clone(),
            description: doc.props.description.clone(),
            state: DocState::EmTrabalho,
            versions: vec![],
            revisions: vec![],
        });
        self.save(id, &doc, "criado")?;
        Ok(id)
    }

    fn check_pn(&self, except: Option<u64>, pn: &str) -> PResult<()> {
        if !pn.is_empty() && self.index.documents.iter().any(|d| Some(d.id) != except && d.pn == pn) {
            return Err(ProjectError::DuplicatePn(pn.into()));
        }
        Ok(())
    }

    /// Salva o documento de trabalho e cria uma versão (se o conteúdo mudou).
    pub fn save(&mut self, id: u64, doc: &Document, note: &str) -> PResult<()> {
        self.check_pn(Some(id), &doc.props.pn)?;
        let json = doc.to_json();
        let hash = blake3::hash(json.as_bytes()).to_hex()[..16].to_string();
        let vpath = self.root.join(VERSIONS).join(format!("{hash}.sccpart"));
        if !vpath.exists() {
            std::fs::write(&vpath, &json)?;
        }
        let file = self.root.join(&self.entry(id)?.file);
        std::fs::write(file, &json)?;
        let e = self.entry_mut(id)?;
        e.name = doc.name.clone();
        e.pn = doc.props.pn.clone();
        e.description = doc.props.description.clone();
        if e.versions.last().map(|v| v.hash != hash).unwrap_or(true) {
            e.versions.push(Version { hash, time: now(), note: note.into() });
            if e.state == DocState::Liberado {
                e.state = DocState::EmTrabalho;
            }
        }
        self.save_index()
    }

    pub fn load(&self, id: u64) -> PResult<Document> {
        let s = std::fs::read_to_string(self.root.join(&self.entry(id)?.file))?;
        Ok(Document::from_json(&s)?)
    }

    /// Abre uma versão imutável (ex.: revisão congelada).
    pub fn load_version(&self, hash: &str) -> PResult<Document> {
        let s = std::fs::read_to_string(self.root.join(VERSIONS).join(format!("{hash}.sccpart")))?;
        Ok(Document::from_json(&s)?)
    }

    /// Libera uma revisão a partir da última versão salva e a congela (RF-GPR-011).
    pub fn release(&mut self, id: u64, comment: &str) -> PResult<String> {
        let e = self.entry_mut(id)?;
        if !e.dirty_since_release() {
            return Err(ProjectError::NothingToRelease);
        }
        let hash = e.versions.last().unwrap().hash.clone();
        let label = next_label(e.revisions.last().map(|r| r.label.as_str()));
        e.revisions.push(Revision { label: label.clone(), hash, time: now(), comment: comment.into() });
        e.state = DocState::Liberado;
        self.save_index()?;
        Ok(label)
    }

    pub fn set_state(&mut self, id: u64, st: DocState) -> PResult<()> {
        self.entry_mut(id)?.state = st;
        self.save_index()
    }

    /// Cópia de um documento como novo item (novo ID e PN), sem histórico.
    pub fn duplicate(&mut self, id: u64) -> PResult<u64> {
        let mut d = self.load(id)?;
        d.name = format!("{} (cópia)", d.name);
        d.props.pn = String::new();
        self.add_document(d)
    }

    /// Exporta a estrutura de produto simples em CSV (RF-GPR-006).
    pub fn export_csv(&self, path: &Path) -> PResult<()> {
        let mut s = String::from("PN;Nome;Descrição;Estado;Revisão;Versões\n");
        for d in &self.index.documents {
            let rev = d.current_revision().map(|r| r.label.as_str()).unwrap_or("-");
            s += &format!("{};{};{};{};{};{}\n", d.pn, d.name, d.description, d.state.label(), rev, d.versions.len());
        }
        std::fs::write(path, s)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("scc-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn letras_de_revisao() {
        assert_eq!(next_label(None), "A");
        assert_eq!(next_label(Some("A")), "B");
        assert_eq!(next_label(Some("Z")), "AA");
        assert_eq!(next_label(Some("AZ")), "BA");
    }

    #[test]
    fn versoes_e_revisao_congelada() {
        let root = tmp("rev");
        let mut p = Project::create(&root, "Teste").unwrap();
        let mut d = Document::new("Placa");
        let id = p.add_document(d.clone()).unwrap();
        d.props.pn = p.entry(id).unwrap().pn.clone();
        assert_eq!(d.props.pn, "SCC-0001");

        assert_eq!(p.release(id, "primeira").unwrap(), "A");
        assert!(matches!(p.release(id, "de novo"), Err(ProjectError::NothingToRelease)));
        let hash_a = p.entry(id).unwrap().revisions[0].hash.clone();

        d.props.description = "alterada".into();
        p.save(id, &d, "editado").unwrap();
        assert_eq!(p.entry(id).unwrap().state, DocState::EmTrabalho);
        assert_eq!(p.entry(id).unwrap().versions.len(), 2);

        // revisão A continua exatamente como foi liberada
        assert_eq!(p.load_version(&hash_a).unwrap().props.description, "");
        assert_eq!(p.release(id, "segunda").unwrap(), "B");

        // reabrir o projeto
        let p2 = Project::open(&root).unwrap();
        assert_eq!(p2.entry(id).unwrap().revisions.len(), 2);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn pn_duplicado_bloqueado() {
        let root = tmp("pn");
        let mut p = Project::create(&root, "T").unwrap();
        let mut a = Document::new("a");
        a.props.pn = "X-1".into();
        p.add_document(a.clone()).unwrap();
        assert!(matches!(p.add_document(a), Err(ProjectError::DuplicatePn(_))));
        let _ = std::fs::remove_dir_all(root);
    }
}
