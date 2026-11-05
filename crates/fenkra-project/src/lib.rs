//! Gestão de projeto simples (ADR 0009, RF-GPR).
//!
//! ```text
//! MeuProjeto/
//! ├── projeto.fenkra.json      índice: documentos, PN, revisões
//! ├── pecas/<id>.fkp    documentos de trabalho
//! └── .versoes/<hash>.fkp   versões imutáveis (endereçadas pelo conteúdo)
//! ```
//!
//! - Documentos são identificados por um ID estável, não pelo nome do arquivo (RF-GPR-021).
//! - Cada salvamento cria uma **versão** (RF-GPR-010).
//! - **Liberar revisão** marca a versão atual com uma letra e a congela (RF-GPR-011).

pub mod referencias;
pub mod seguro;

use fenkra_core::formato::{preparar, Tipo, APP_VERSION};
use seguro::gravar_atomico;
use fenkra_core::Document;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const INDEX: &str = "projeto.fenkra.json";
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
    #[error("projeto somente leitura: {0}")]
    SomenteLeitura(String),
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
    /// identidade estável do documento (B66); o mesmo uid está gravado dentro do arquivo
    #[serde(default)]
    pub uid: String,
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
    /// versão do programa que gravou o índice (informativo)
    #[serde(default)]
    pub app_version: String,
    pub name: String,
    pub pn_prefix: String,
    pub next_pn: u32,
    next_id: u64,
    pub documents: Vec<DocEntry>,
}

pub struct Project {
    pub root: PathBuf,
    pub index: Index,
    /// índice de formato mais novo que este programa entende: o projeto abre só para leitura (B64)
    pub aviso_formato: Option<String>,
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
            index: Index {
                format_version: Tipo::Indice.versao_atual(),
                app_version: APP_VERSION.into(),
                name: name.into(),
                pn_prefix: "FK-".into(),
                next_pn: 1,
                next_id: 1,
                documents: vec![],
            },
            aviso_formato: None,
        };
        p.save_index()?;
        Ok(p)
    }

    /// Abre o projeto de qualquer versão antiga (cadeia de migração). Índice de formato futuro abre
    /// tolerante, mas com [`Project::aviso_formato`] preenchido e toda gravação recusada.
    pub fn open(root: &Path) -> PResult<Project> {
        let s = std::fs::read_to_string(root.join(INDEX))?;
        let prep = preparar(Tipo::Indice, &s)?;
        let aviso_formato = prep.aviso(Tipo::Indice);
        Ok(Project { root: root.into(), index: serde_json::from_value(prep.valor)?, aviso_formato })
    }

    fn pode_gravar(&self) -> PResult<()> {
        match &self.aviso_formato {
            Some(a) => Err(ProjectError::SomenteLeitura(a.clone())),
            None => Ok(()),
        }
    }

    pub fn save_index(&self) -> PResult<()> {
        self.pode_gravar()?;
        let mut index = self.index.clone();
        index.app_version = APP_VERSION.into();
        gravar_atomico(&self.root.join(INDEX), serde_json::to_string_pretty(&index)?.as_bytes())?;
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
            uid: doc.uid.0.clone(),
            file: format!("{PARTS}/{id:05}.fkp"),
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
        self.pode_gravar()?;
        if doc.formato_futuro() {
            return Err(ProjectError::SomenteLeitura(fenkra_core::formato::aviso_futuro(Tipo::Peca, doc.format_version)));
        }
        self.check_pn(Some(id), &doc.props.pn)?;
        // documento antigo sem uid: o uid fica estável pelo registro do projeto (B66)
        let uid = if doc.uid.esta_vazio() {
            let e = self.entry(id)?;
            if e.uid.is_empty() { fenkra_core::formato::novo_uid() } else { e.uid.clone() }
        } else {
            doc.uid.0.clone()
        };
        let json = if doc.uid.esta_vazio() {
            let mut c = doc.clone();
            c.uid.0 = uid.clone();
            c.to_json()
        } else {
            doc.to_json()
        };
        // a versão do programa não faz parte da identidade do conteúdo: atualizar o Fenkra e salvar
        // sem editar não cria versão nova nem "suja" uma revisão liberada
        let sem_app: String = json.lines().filter(|l| !l.trim_start().starts_with("\"app_version\"")).collect::<Vec<_>>().join("\n");
        let hash = blake3::hash(sem_app.as_bytes()).to_hex()[..16].to_string();
        let vpath = self.root.join(VERSIONS).join(format!("{hash}.fkp"));
        if !vpath.exists() {
            gravar_atomico(&vpath, json.as_bytes())?;
        }
        let file = self.root.join(&self.entry(id)?.file);
        gravar_atomico(&file, json.as_bytes())?;
        let e = self.entry_mut(id)?;
        e.uid = uid;
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
        let e = self.entry(id)?;
        let s = std::fs::read_to_string(self.root.join(&e.file))?;
        let mut d = Document::from_json(&s)?;
        if d.uid.esta_vazio() {
            d.uid.0 = e.uid.clone();
        }
        Ok(d)
    }

    /// Abre uma versão imutável (ex.: revisão congelada).
    pub fn load_version(&self, hash: &str) -> PResult<Document> {
        let s = std::fs::read_to_string(self.root.join(VERSIONS).join(format!("{hash}.fkp")))?;
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
        // cópia é outro documento: nunca herda a identidade do original
        d.uid = fenkra_core::formato::UidDoc::novo();
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
        let p = std::env::temp_dir().join(format!("fenkra-test-{name}-{}", std::process::id()));
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
        assert_eq!(d.props.pn, "FK-0001");

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

    /// Índice gravado pelo alpha.28: sem `app_version`.
    const INDICE_ALPHA28: &str = r#"{
  "format_version": 1,
  "name": "Antigo",
  "pn_prefix": "FK-",
  "next_pn": 3,
  "next_id": 2,
  "documents": [
    { "id": 1, "file": "pecas/00001.fkp", "name": "Placa", "pn": "FK-0002", "description": "",
      "state": "EmTrabalho", "versions": [ { "hash": "abc", "time": 10, "note": "criado" } ], "revisions": [] }
  ]
}"#;

    #[test]
    fn indice_do_alpha28_abre_e_e_regravado_com_app_version() {
        let root = tmp("migra");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(INDEX), INDICE_ALPHA28).unwrap();
        let mut p = Project::open(&root).unwrap();
        assert!(p.aviso_formato.is_none());
        assert_eq!(p.index.documents.len(), 1);
        assert_eq!(p.index.documents[0].pn, "FK-0002");
        assert_eq!(p.next_pn(), "FK-0003");
        p.save_index().unwrap();
        let regravado = std::fs::read_to_string(root.join(INDEX)).unwrap();
        assert!(regravado.contains(&format!("\"app_version\": \"{APP_VERSION}\"")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn indice_de_formato_futuro_abre_somente_leitura_e_nao_e_regravado() {
        let root = tmp("futuro");
        std::fs::create_dir_all(&root).unwrap();
        let futuro = INDICE_ALPHA28.replace("\"format_version\": 1", "\"format_version\": 42").replace("\"documents\"", "\"campo_novo\": true, \"documents\"");
        std::fs::write(root.join(INDEX), &futuro).unwrap();
        let p = Project::open(&root).unwrap();
        assert!(p.aviso_formato.as_deref().unwrap().contains("v42"));
        assert!(matches!(p.save_index(), Err(ProjectError::SomenteLeitura(_))));
        assert_eq!(std::fs::read_to_string(root.join(INDEX)).unwrap(), futuro, "o arquivo do usuário não pode mudar");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn uid_do_documento_e_estavel_e_a_copia_ganha_outro() {
        let root = tmp("uid");
        let mut p = Project::create(&root, "T").unwrap();
        let d = Document::new("Placa");
        let uid = d.uid.0.clone();
        assert_eq!(uid.len(), 32);
        let id = p.add_document(d).unwrap();
        assert_eq!(p.entry(id).unwrap().uid, uid);
        assert_eq!(p.load(id).unwrap().uid.0, uid);
        let copia = p.duplicate(id).unwrap();
        assert_ne!(p.entry(copia).unwrap().uid, uid);

        // arquivo de alpha antigo: sem uid no JSON. O registro do projeto fixa um uid que não muda.
        let mut velho = Document::new("Velha");
        velho.uid.0.clear();
        let id2 = p.add_document(velho.clone()).unwrap();
        let u2 = p.entry(id2).unwrap().uid.clone();
        assert_eq!(u2.len(), 32, "o primeiro salvamento atribui o uid");
        p.save(id2, &velho, "de novo").unwrap();
        assert_eq!(p.entry(id2).unwrap().uid, u2, "salvar de novo não troca a identidade");
        assert_eq!(p.load(id2).unwrap().uid.0, u2);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn atualizar_o_programa_e_salvar_sem_editar_nao_cria_versao() {
        let root = tmp("appver");
        let mut p = Project::create(&root, "T").unwrap();
        let id = p.add_document(Document::new("Placa")).unwrap();
        p.release(id, "A").unwrap();
        let mut d = p.load(id).unwrap();
        d.app_version.0 = "0.0.1-antigo".into();
        p.save(id, &d, "regravado").unwrap();
        assert_eq!(p.entry(id).unwrap().versions.len(), 1);
        assert!(!p.entry(id).unwrap().dirty_since_release());
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
