// SPDX-License-Identifier: LGPL-2.1-or-later
//! Gestão de projeto simples (ADR 0009, RF-GPR).
//!
//! ```text
//! MeuProjeto/
//! ├── projeto.fenkra.json      índice: documentos, PN, revisões
//! ├── pecas/<id>.fkp    documentos de trabalho
//! ├── .versoes/<hash>.fkp   versões imutáveis (endereçadas pelo conteúdo)
//! └── .originais/<hash>.*   arquivos importados e relatórios de conversão
//! ```
//!
//! - Documentos são identificados por um ID estável, não pelo nome do arquivo (RF-GPR-021).
//! - Cada salvamento cria uma **versão** (RF-GPR-010).
//! - **Liberar revisão** marca a versão atual com uma letra e a congela (RF-GPR-011).

pub mod diario;
pub mod importacao;
pub mod plano;
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
    #[error("importação: {0}")]
    Importacao(String),
    #[error("recusado: {0}")]
    Recusado(String),
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
    /// quem gravou a versão (B25); vazio nas versões de antes do campo existir
    #[serde(default)]
    pub author: String,
    /// comentário livre da versão, além da nota automática
    #[serde(default)]
    pub comment: String,
    /// número (a partir de 1) da versão cujo conteúdo esta versão restaurou. O `hash` é o mesmo da
    /// versão antiga: a restauração é uma versão nova, o histórico nunca é reescrito.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restored_from: Option<usize>,
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
    /// selo do documento importado, copiado aqui para o painel listar sem abrir cada peça
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origem: Option<fenkra_core::SeloOrigem>,
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
    /// quem está operando o projeto: entra em versões e no diário (B25)
    pub autor: String,
}

/// Autor padrão: o usuário do sistema operacional.
fn autor_do_sistema() -> String {
    std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default()
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
            autor: autor_do_sistema(),
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
        Ok(Project { root: root.into(), index: serde_json::from_value(prep.valor)?, aviso_formato, autor: autor_do_sistema() })
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

    /// Numeração automática de PN (RF-GPR-005). **Consome** o número na hora: use só quando o
    /// chamador garante a gravação; `add_document` usa [`Project::pn_previsto`] e só avança o contador
    /// depois que o documento foi gravado (B25).
    pub fn next_pn(&mut self) -> String {
        let pn = self.pn_previsto();
        self.index.next_pn += 1;
        pn
    }

    /// O próximo PN sem consumi-lo (prévia e reserva).
    pub fn pn_previsto(&self) -> String {
        format!("{}{:04}", self.index.pn_prefix, self.index.next_pn)
    }

    fn registrar(&self, tipo: diario::TipoEvento, doc_id: u64, de: &str, para: &str, motivo: &str) -> PResult<()> {
        let uid = self.entry(doc_id).map(|e| e.uid.clone()).unwrap_or_default();
        diario::acrescentar(
            &self.root,
            &diario::Evento { tempo: now(), autor: self.autor.clone(), tipo, doc_id, uid, de: de.into(), para: para.into(), motivo: motivo.into() },
        )?;
        Ok(())
    }

    /// Adiciona um documento novo ao projeto e grava a primeira versão. O contador de PN e o de IDs
    /// só avançam se a gravação inteira deu certo: falha de disco não queima número (B25).
    pub fn add_document(&mut self, mut doc: Document) -> PResult<u64> {
        self.pode_gravar()?;
        let consome_pn = doc.props.pn.is_empty();
        if consome_pn {
            doc.props.pn = self.pn_previsto();
        }
        self.check_pn(None, &doc.props.pn)?;
        let (next_id_antes, next_pn_antes) = (self.index.next_id, self.index.next_pn);
        let id = self.index.next_id;
        self.index.next_id += 1;
        if consome_pn {
            self.index.next_pn += 1;
        }
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
            origem: doc.origem.clone(),
        });
        if let Err(e) = self.save(id, &doc, "criado") {
            // desfaz tudo: o projeto na memória volta a ser igual ao que está no disco
            self.index.documents.retain(|d| d.id != id);
            self.index.next_id = next_id_antes;
            self.index.next_pn = next_pn_antes;
            return Err(e);
        }
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
        self.save_comentado(id, doc, note, "")
    }

    /// Como [`Project::save`], com um comentário livre gravado na versão nova (B25).
    pub fn save_comentado(&mut self, id: u64, doc: &Document, note: &str, comentario: &str) -> PResult<()> {
        self.pode_gravar()?;
        let antes = self.entry(id)?.clone();
        match self.save_interno(id, doc, note, comentario) {
            Ok(()) => Ok(()),
            Err(e) => {
                // índice na memória volta ao que está no disco
                if let Ok(en) = self.entry_mut(id) {
                    *en = antes;
                }
                Err(e)
            }
        }
    }

    fn save_interno(&mut self, id: u64, doc: &Document, note: &str, comentario: &str) -> PResult<()> {
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
        let autor = self.autor.clone();
        let e = self.entry_mut(id)?;
        e.uid = uid;
        e.name = doc.name.clone();
        e.pn = doc.props.pn.clone();
        e.description = doc.props.description.clone();
        e.origem = doc.origem.clone();
        let mut volta_a_trabalho = false;
        let nova = e.versions.last().map(|v| v.hash != hash).unwrap_or(true);
        if nova {
            e.versions.push(Version { hash: hash.clone(), time: now(), note: note.into(), author: autor, comment: comentario.into(), restored_from: None });
            if e.state == DocState::Liberado {
                e.state = DocState::EmTrabalho;
                volta_a_trabalho = true;
            }
        }
        self.save_index()?;
        if nova {
            self.registrar(diario::TipoEvento::Versao, id, "", &hash, note)?;
        }
        if volta_a_trabalho {
            self.registrar(diario::TipoEvento::Transicao, id, DocState::Liberado.label(), DocState::EmTrabalho.label(), "automático: documento liberado foi editado")?;
        }
        Ok(())
    }

    /// Restaura a versão `numero` (a partir de 1): o conteúdo antigo vira o documento de trabalho e
    /// uma versão **nova** aponta para ele. Nenhuma versão anterior é alterada ou apagada (B25).
    pub fn restaurar_versao(&mut self, id: u64, numero: usize, motivo: &str) -> PResult<()> {
        self.pode_gravar()?;
        let antes = self.entry(id)?.clone();
        let alvo = antes.versions.get(numero.wrapping_sub(1)).cloned().ok_or(ProjectError::NotFound)?;
        if antes.versions.last().map(|v| v.hash == alvo.hash).unwrap_or(false) {
            return Err(ProjectError::Recusado("a versão escolhida já é a atual".into()));
        }
        let conteudo = std::fs::read(self.root.join(VERSIONS).join(format!("{}.fkp", alvo.hash)))?;
        match self.restaurar_interno(id, numero, &alvo.hash, &conteudo, motivo) {
            Ok(()) => Ok(()),
            Err(e) => {
                if let Ok(en) = self.entry_mut(id) {
                    *en = antes;
                }
                Err(e)
            }
        }
    }

    fn restaurar_interno(&mut self, id: u64, numero: usize, hash: &str, conteudo: &[u8], motivo: &str) -> PResult<()> {
        let file = self.root.join(&self.entry(id)?.file);
        gravar_atomico(&file, conteudo)?;
        let autor = self.autor.clone();
        let e = self.entry_mut(id)?;
        e.versions.push(Version {
            hash: hash.into(),
            time: now(),
            note: format!("restaurada da v{numero}"),
            author: autor,
            comment: motivo.into(),
            restored_from: Some(numero),
        });
        let antes_estado = e.state;
        if e.state == DocState::Liberado && e.dirty_since_release() {
            e.state = DocState::EmTrabalho;
        }
        let depois_estado = e.state;
        self.save_index()?;
        self.registrar(diario::TipoEvento::Restauracao, id, &format!("v{numero}"), hash, motivo)?;
        if antes_estado != depois_estado {
            self.registrar(diario::TipoEvento::Transicao, id, antes_estado.label(), depois_estado.label(), "automático: versão restaurada difere da revisão liberada")?;
        }
        Ok(())
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
        self.pode_gravar()?;
        let e = self.entry_mut(id)?;
        if !e.dirty_since_release() {
            return Err(ProjectError::NothingToRelease);
        }
        let hash = e.versions.last().unwrap().hash.clone();
        let label = next_label(e.revisions.last().map(|r| r.label.as_str()));
        let estado_antes = e.state;
        e.revisions.push(Revision { label: label.clone(), hash, time: now(), comment: comment.into() });
        e.state = DocState::Liberado;
        self.save_index()?;
        self.registrar(diario::TipoEvento::Revisao, id, "", &label, comment)?;
        self.registrar(diario::TipoEvento::Transicao, id, estado_antes.label(), DocState::Liberado.label(), comment)?;
        Ok(label)
    }

    pub fn set_state(&mut self, id: u64, st: DocState, motivo: &str) -> PResult<()> {
        self.pode_gravar()?;
        let e = self.entry_mut(id)?;
        let antes = e.state;
        e.state = st;
        self.save_index()?;
        self.registrar(diario::TipoEvento::Transicao, id, antes.label(), st.label(), motivo)
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

    #[test]
    fn transicao_grava_autor_data_e_motivo_no_diario() {
        let root = tmp("diario");
        let mut p = Project::create(&root, "T").unwrap();
        p.autor = "ana".into();
        let id = p.add_document(Document::new("Placa")).unwrap();
        p.set_state(id, DocState::Obsoleto, "substituída pelo modelo 2").unwrap();
        let (ev, puladas) = diario::ler(&root);
        assert_eq!(puladas, 0);
        let t = ev.iter().find(|e| e.tipo == diario::TipoEvento::Transicao).expect("transição registrada");
        assert_eq!((t.autor.as_str(), t.de.as_str(), t.para.as_str()), ("ana", "Em trabalho", "Obsoleto"));
        assert_eq!(t.motivo, "substituída pelo modelo 2");
        assert!(t.tempo > 0 && t.doc_id == id && t.uid.len() == 32);
        // mudar de estado de novo só acrescenta: as linhas anteriores ficam idênticas
        let antes = std::fs::read_to_string(diario::caminho(&root)).unwrap();
        p.set_state(id, DocState::EmTrabalho, "voltou").unwrap();
        assert!(std::fs::read_to_string(diario::caminho(&root)).unwrap().starts_with(&antes));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn versao_guarda_autor_e_comentario() {
        let root = tmp("autor");
        let mut p = Project::create(&root, "T").unwrap();
        p.autor = "bia".into();
        let mut d = Document::new("Placa");
        let id = p.add_document(d.clone()).unwrap();
        d.props.description = "mudou".into();
        p.save_comentado(id, &d, "editado", "ajustei a espessura").unwrap();
        let v = p.entry(id).unwrap().versions.last().unwrap().clone();
        assert_eq!((v.author.as_str(), v.comment.as_str()), ("bia", "ajustei a espessura"));
        // sobrevive à reabertura
        let p2 = Project::open(&root).unwrap();
        assert_eq!(p2.entry(id).unwrap().versions[1].author, "bia");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn restaurar_cria_versao_nova_e_nunca_reescreve_o_historico() {
        let root = tmp("restaura");
        let mut p = Project::create(&root, "T").unwrap();
        p.autor = "caio".into();
        let mut d = Document::new("Placa");
        let id = p.add_document(d.clone()).unwrap();
        d.props.description = "segunda".into();
        p.save(id, &d, "v2").unwrap();
        d.props.description = "terceira".into();
        p.save(id, &d, "v3").unwrap();
        let antes: Vec<(String, String)> = p.entry(id).unwrap().versions.iter().map(|v| (v.hash.clone(), v.note.clone())).collect();
        let hash_v1 = antes[0].0.clone();

        p.restaurar_versao(id, 1, "o cliente voltou atrás").unwrap();
        let e = p.entry(id).unwrap();
        assert_eq!(e.versions.len(), 4, "uma versão nova, nenhuma removida");
        let depois: Vec<(String, String)> = e.versions[..3].iter().map(|v| (v.hash.clone(), v.note.clone())).collect();
        assert_eq!(depois, antes, "as versões antigas ficam como estavam");
        let nova = &e.versions[3];
        assert_eq!((nova.hash.as_str(), nova.restored_from, nova.author.as_str()), (hash_v1.as_str(), Some(1), "caio"));
        assert_eq!(nova.comment, "o cliente voltou atrás");
        assert_eq!(p.load(id).unwrap().props.description, "", "o documento de trabalho tem o conteúdo antigo");
        // restaurar a versão que já é a atual não faz nada
        assert!(matches!(p.restaurar_versao(id, 4, ""), Err(ProjectError::Recusado(_))));
        assert!(matches!(p.restaurar_versao(id, 99, ""), Err(ProjectError::NotFound)));
        let (ev, _) = diario::ler(&root);
        assert!(ev.iter().any(|e| e.tipo == diario::TipoEvento::Restauracao && e.para == hash_v1 && e.motivo == "o cliente voltou atrás"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn restaurar_depois_de_liberar_volta_a_trabalho_e_registra() {
        let root = tmp("restaura-lib");
        let mut p = Project::create(&root, "T").unwrap();
        let mut d = Document::new("Placa");
        let id = p.add_document(d.clone()).unwrap();
        d.props.description = "nova".into();
        p.save(id, &d, "v2").unwrap();
        p.release(id, "A").unwrap();
        assert_eq!(p.entry(id).unwrap().state, DocState::Liberado);
        p.restaurar_versao(id, 1, "volta").unwrap();
        assert_eq!(p.entry(id).unwrap().state, DocState::EmTrabalho);
        assert!(p.entry(id).unwrap().dirty_since_release());
        // a revisão A continua congelada
        assert_eq!(p.load_version(&p.entry(id).unwrap().revisions[0].hash).unwrap().props.description, "nova");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn falha_na_gravacao_nao_avanca_o_contador_de_pn_nem_deixa_documento_fantasma() {
        let root = tmp("pn-falha");
        let mut p = Project::create(&root, "T").unwrap();
        p.add_document(Document::new("a")).unwrap();
        assert_eq!(p.pn_previsto(), "FK-0002");
        // um arquivo no lugar da pasta de versões faz a gravação falhar
        std::fs::remove_dir_all(root.join(VERSIONS)).unwrap();
        std::fs::write(root.join(VERSIONS), b"x").unwrap();
        assert!(p.add_document(Document::new("b")).is_err());
        assert_eq!(p.pn_previsto(), "FK-0002", "a sequência não avançou");
        assert_eq!(p.index.documents.len(), 1, "nada ficou registrado");
        // consertado o disco, o mesmo número é usado pelo próximo documento
        std::fs::remove_file(root.join(VERSIONS)).unwrap();
        std::fs::create_dir_all(root.join(VERSIONS)).unwrap();
        let id = p.add_document(Document::new("c")).unwrap();
        assert_eq!(p.entry(id).unwrap().pn, "FK-0002");
        assert_eq!(p.pn_previsto(), "FK-0003");
        // e o índice no disco concorda
        assert_eq!(Project::open(&root).unwrap().index.next_pn, 3);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn pn_informado_pelo_usuario_nao_consome_a_sequencia() {
        let root = tmp("pn-manual");
        let mut p = Project::create(&root, "T").unwrap();
        let mut d = Document::new("a");
        d.props.pn = "MEU-1".into();
        p.add_document(d).unwrap();
        assert_eq!(p.pn_previsto(), "FK-0001");
        let _ = std::fs::remove_dir_all(root);
    }
}
