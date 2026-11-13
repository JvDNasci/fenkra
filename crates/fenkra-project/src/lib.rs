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

pub mod comparacao;
pub mod diario;
pub mod estado_doc;
pub mod estados;
pub mod importacao;
pub mod pacote;
pub mod plano;
pub mod pn;
pub mod qualidade_modelo;
pub mod qualidade_projeto;
pub mod ramos;
pub mod referencias;
pub mod relacoes;
pub mod salvar_como;
pub mod seguro;
pub mod sobreposicao;
pub mod visualizacao;

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
    #[error("lista de conferência reprovada: {0}")]
    Conferencia(String),
}

pub type PResult<T> = Result<T, ProjectError>;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DocState {
    EmTrabalho,
    /// pedido de aprovação em andamento (só nos ciclos que usam este estado)
    EmAprovacao,
    Liberado,
    Obsoleto,
}

impl DocState {
    pub fn label(self) -> &'static str {
        match self {
            DocState::EmTrabalho => "Em trabalho",
            DocState::EmAprovacao => "Em aprovação",
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
    /// estado do ciclo de vida em que a versão ficou ao ser gravada (B214); vazio nas versões de antes
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub estado: String,
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
    /// ambientes de numeração (B47); vazio = só o prefixo e o contador acima
    #[serde(default)]
    pub pn_ambientes: Vec<pn::AmbientePn>,
    /// nome do ambiente em uso; vazio ou desconhecido = numeração simples
    #[serde(default)]
    pub pn_ambiente: String,
    /// pastas extras onde procurar quem usa os documentos do projeto (relativas à raiz ou absolutas);
    /// a própria pasta do projeto sempre entra (B94)
    #[serde(default)]
    pub pastas_busca: Vec<String>,
    /// ciclo de vida: estados, transições permitidas e esquema de revisão (B265)
    #[serde(default)]
    pub ciclo: estados::Ciclo,
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
    /// papéis de quem opera (B265): decidem quais transições do ciclo estão liberadas. Numa estação
    /// local sem controle de acesso o usuário acumula os papéis padrão.
    pub papeis: Vec<String>,
}

fn papeis_padrao() -> Vec<String> {
    vec!["projetista".into(), "aprovador".into()]
}

/// Autor padrão: o usuário do sistema operacional.
fn autor_do_sistema() -> String {
    std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default()
}

/// Código curto do tipo de peça para a máscara de PN.
fn codigo_tipo(t: fenkra_core::PartType) -> &'static str {
    use fenkra_core::PartType::*;
    match t {
        Fabricada => "FAB",
        Comprada => "COM",
        Hardware => "HW",
        Chapa => "CHP",
        MembroEstrutural => "EST",
        Fantasma => "FAN",
    }
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
                pn_ambientes: vec![],
                pn_ambiente: String::new(),
                pastas_busca: vec![],
                ciclo: estados::Ciclo::padrao(),
                next_id: 1,
                documents: vec![],
            },
            aviso_formato: None,
            autor: autor_do_sistema(),
            papeis: papeis_padrao(),
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
        Ok(Project { root: root.into(), index: serde_json::from_value(prep.valor)?, aviso_formato, autor: autor_do_sistema(), papeis: papeis_padrao() })
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
    /// chamador garante a gravação; `add_document` usa [`Project::pn_previsto_para`] e só avança o
    /// contador depois que o documento foi gravado (B25).
    pub fn next_pn(&mut self) -> String {
        let (pn, seq) = self.pn_livre(&Document::new(""));
        self.gravar_seq(seq);
        pn
    }

    /// O próximo PN sem consumi-lo, para um documento padrão.
    pub fn pn_previsto(&self) -> String {
        self.pn_livre(&Document::new("")).0
    }

    /// O PN que `doc` receberia agora (sem consumir). Usa o ambiente ativo, a máscara dele e os campos
    /// do documento (tipo, propriedades personalizadas) (B47).
    pub fn pn_previsto_para(&self, doc: &Document) -> String {
        self.pn_livre(doc).0
    }

    /// Prévia ao vivo em segmentos, para a interface pintar de cinza o que ainda falta preencher.
    pub fn pn_previa(&self, doc: &Document) -> Result<Vec<pn::Segmento>, pn::ErroMascara> {
        let (mascara, campos, seq) = self.regra_pn(doc);
        pn::previa(&mascara, &campos, seq)
    }

    /// Máscara em uso (a do ambiente ativo ou a padrão).
    pub fn pn_mascara_ativa(&self) -> String {
        self.ambiente_ativo().map(|a| a.mascara.clone()).unwrap_or_else(|| pn::MASCARA_PADRAO.into())
    }

    fn ambiente_ativo(&self) -> Option<&pn::AmbientePn> {
        self.index.pn_ambientes.iter().find(|a| a.nome == self.index.pn_ambiente)
    }

    /// Cria ou atualiza um ambiente de numeração. A máscara é validada; o contador de um ambiente que
    /// já existe é preservado.
    pub fn definir_ambiente_pn(&mut self, mut amb: pn::AmbientePn) -> PResult<()> {
        self.pode_gravar()?;
        pn::validar(&amb.mascara).map_err(|e| ProjectError::Recusado(e.to_string()))?;
        if amb.nome.trim().is_empty() {
            return Err(ProjectError::Recusado("o ambiente precisa de um nome".into()));
        }
        let antes = self.index.pn_ambientes.clone();
        match self.index.pn_ambientes.iter_mut().find(|a| a.nome == amb.nome) {
            Some(a) => {
                amb.proximo = a.proximo;
                *a = amb;
            }
            None => self.index.pn_ambientes.push(amb),
        }
        self.save_index().inspect_err(|_| self.index.pn_ambientes = antes)
    }

    /// Escolhe o ambiente em uso; nome vazio volta à numeração simples.
    pub fn ativar_ambiente_pn(&mut self, nome: &str) -> PResult<()> {
        self.pode_gravar()?;
        if !nome.is_empty() && !self.index.pn_ambientes.iter().any(|a| a.nome == nome) {
            return Err(ProjectError::Recusado(format!("não existe o ambiente {nome}")));
        }
        let antes = std::mem::replace(&mut self.index.pn_ambiente, nome.into());
        self.save_index().inspect_err(|_| self.index.pn_ambiente = antes)
    }

    fn regra_pn(&self, doc: &Document) -> (String, pn::CamposPn, u32) {
        let (mascara, prefixo, ambiente, seq) = match self.ambiente_ativo() {
            Some(a) => (a.mascara.clone(), a.prefixo.clone(), a.nome.clone(), a.proximo),
            None => (pn::MASCARA_PADRAO.to_string(), self.index.pn_prefix.clone(), String::new(), self.index.next_pn),
        };
        let campos = pn::CamposPn {
            prefixo,
            ambiente,
            ano: Some(pn::ano_atual()),
            tipo: codigo_tipo(doc.props.part_type).into(),
            personalizados: doc.props.custom.clone(),
        };
        (mascara, campos, seq)
    }

    /// Primeiro PN livre a partir do contador, e o número de sequência que ele usou.
    fn pn_livre(&self, doc: &Document) -> (String, u32) {
        let (mascara, campos, mut seq) = self.regra_pn(doc);
        loop {
            let pn = pn::previa(&mascara, &campos, seq).map(|s| pn::texto(&s)).unwrap_or_else(|_| format!("{}{:04}", self.index.pn_prefix, seq));
            if !self.index.documents.iter().any(|d| d.pn == pn) || seq == u32::MAX {
                return (pn, seq);
            }
            seq += 1;
        }
    }

    /// Marca `seq` como usada: o contador do ambiente ativo passa a `seq + 1`.
    fn gravar_seq(&mut self, seq: u32) {
        let ativo = self.index.pn_ambiente.clone();
        match self.index.pn_ambientes.iter_mut().find(|a| a.nome == ativo) {
            Some(a) => a.proximo = seq + 1,
            None => self.index.next_pn = seq + 1,
        }
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
        let mut seq_usada = 0;
        if consome_pn {
            let (pn, seq) = self.pn_livre(&doc);
            doc.props.pn = pn;
            seq_usada = seq;
        }
        self.check_pn(None, &doc.props.pn)?;
        let (next_id_antes, next_pn_antes, ambientes_antes) = (self.index.next_id, self.index.next_pn, self.index.pn_ambientes.clone());
        let id = self.index.next_id;
        self.index.next_id += 1;
        if consome_pn {
            self.gravar_seq(seq_usada);
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
            self.index.pn_ambientes = ambientes_antes;
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
        let mut estado_antes_edicao = None;
        let nova = e.versions.last().map(|v| v.hash != hash).unwrap_or(true);
        if nova {
            e.versions.push(Version { hash: hash.clone(), time: now(), note: note.into(), author: autor, comment: comentario.into(), restored_from: None, estado: String::new() });
            if matches!(e.state, DocState::Liberado | DocState::EmAprovacao) {
                estado_antes_edicao = Some(e.state);
                e.state = DocState::EmTrabalho;
                volta_a_trabalho = true;
            }
            // B214: o estado em que a versão ficou, para a comparação de versões mostrar
            let estado = e.state.label().to_string();
            if let Some(v) = e.versions.last_mut() {
                v.estado = estado;
            }
        }
        self.save_index()?;
        if nova {
            self.registrar(diario::TipoEvento::Versao, id, "", &hash, note)?;
        }
        if volta_a_trabalho {
            let de = estado_antes_edicao.unwrap_or(DocState::Liberado);
            self.registrar(diario::TipoEvento::Transicao, id, de.label(), DocState::EmTrabalho.label(), "automático: o documento foi editado")?;
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
            estado: String::new(),
        });
        let antes_estado = e.state;
        if matches!(e.state, DocState::Liberado | DocState::EmAprovacao) && e.dirty_since_release() {
            e.state = DocState::EmTrabalho;
        }
        let depois_estado = e.state;
        if let Some(v) = e.versions.last_mut() {
            v.estado = depois_estado.label().to_string();
        }
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

    /// Transições que o usuário pode fazer agora a partir do estado do documento (botões da interface).
    pub fn transicoes_disponiveis(&self, id: u64) -> PResult<Vec<estados::Transicao>> {
        let e = self.entry(id)?;
        Ok(self.index.ciclo.disponiveis(e.state, &self.papeis).into_iter().cloned().collect())
    }

    /// Troca o ciclo de vida do projeto (B265). O ciclo é validado; os documentos que estão num
    /// estado que o ciclo novo não conhece voltam para o estado inicial, e cada volta é registrada.
    pub fn definir_ciclo(&mut self, ciclo: estados::Ciclo) -> PResult<()> {
        self.pode_gravar()?;
        ciclo.validar().map_err(ProjectError::Recusado)?;
        let antes = self.index.clone();
        let mut ajustados = vec![];
        for d in &mut self.index.documents {
            if !ciclo.estados.contains(&d.state) {
                ajustados.push((d.id, d.state));
                d.state = ciclo.inicial;
            }
        }
        self.index.ciclo = ciclo;
        if let Err(e) = self.save_index() {
            self.index = antes;
            return Err(e);
        }
        for (id, estado) in ajustados {
            self.registrar(diario::TipoEvento::Transicao, id, estado.label(), self.index.ciclo.inicial.label(), "automático: o estado não existe no ciclo novo")?;
        }
        Ok(())
    }

    /// Troca o ciclo a partir do JSON (arquivo do usuário).
    pub fn definir_ciclo_json(&mut self, json: &str) -> PResult<()> {
        let c = estados::Ciclo::de_json(json).map_err(ProjectError::Recusado)?;
        self.definir_ciclo(c)
    }

    /// Muda o estado do documento conforme a tabela de transições do ciclo (B265). Toda mudança fora
    /// da tabela, sem o papel exigido ou sem motivo (quando a regra pede) é recusada com a razão.
    /// Liberar não passa por aqui: [`Project::liberar`] confere a lista e cria a letra.
    pub fn transicionar(&mut self, id: u64, para: DocState, motivo: &str) -> PResult<()> {
        self.pode_gravar()?;
        if para == DocState::Liberado {
            return Err(ProjectError::Recusado("para liberar use Liberar revisão: ela roda a lista de conferência e cria a letra".into()));
        }
        let de = self.entry(id)?.state;
        self.index.ciclo.verificar(de, para, &self.papeis, motivo).map_err(ProjectError::Recusado)?;
        self.entry_mut(id)?.state = para;
        if let Err(e) = self.save_index() {
            self.entry_mut(id)?.state = de;
            return Err(e);
        }
        self.registrar(diario::TipoEvento::Transicao, id, de.label(), para.label(), motivo)
    }

    /// Lista de conferência da liberação: recálculo sem erro, referências resolvidas, PN definido e
    /// desenho atualizado. `ext` traz o que só quem tem o documento aberto sabe (recálculo).
    pub fn conferir_liberacao(&self, id: u64, ext: &estados::Externos) -> PResult<estados::Conferencia> {
        use estados::{EstadoItem, ItemConferencia};
        let e = self.entry(id)?;
        let versao = e.versions.last().ok_or(ProjectError::NotFound)?;
        fn item(chave: &'static str, rotulo: &'static str, estado: EstadoItem, detalhe: String) -> ItemConferencia {
            ItemConferencia { chave, rotulo, estado, detalhe }
        }
        let mut itens = vec![];

        itens.push(match &ext.recalculo {
            Some(Ok(())) => item("recalculo", "Recalcula sem erro", EstadoItem::Ok, String::new()),
            Some(Err(m)) => item("recalculo", "Recalcula sem erro", EstadoItem::Falha, m.clone()),
            None => item("recalculo", "Recalcula sem erro", EstadoItem::NaoVerificado, "o documento não está aberto para recalcular".into()),
        });

        // referências do conteúdo que será congelado, resolvidas a partir da pasta do documento
        let dir = self.root.join(&e.file).parent().map(Path::to_path_buf).unwrap_or_else(|| self.root.clone());
        let busca = self.pastas_de_busca();
        itens.push(match std::fs::read_to_string(self.root.join(VERSIONS).join(format!("{}.fkp", versao.hash))) {
            Err(_) => item("referencias", "Referências resolvidas", EstadoItem::Falha, format!("a versão {} não está no projeto", versao.hash)),
            Ok(json) => {
                let quebradas: Vec<String> = relacoes::referencias_do_json(&json)
                    .into_iter()
                    .filter(|r| {
                        let ref_ = referencias::Ref { uid: r.uid.clone(), caminho: r.caminho.clone() };
                        !matches!(referencias::resolver(&dir, &ref_, &busca), referencias::Resolucao::Encontrado(_))
                    })
                    .map(|r| r.caminho)
                    .collect();
                if quebradas.is_empty() {
                    item("referencias", "Referências resolvidas", EstadoItem::Ok, String::new())
                } else {
                    item("referencias", "Referências resolvidas", EstadoItem::Falha, format!("{} sem resolver ({}); use Reparar links", quebradas.len(), quebradas.join(", ")))
                }
            }
        });

        let pn_ok = !e.pn.trim().is_empty() && self.check_pn(Some(id), &e.pn).is_ok();
        itens.push(if pn_ok {
            item("pn", "PN definido", EstadoItem::Ok, String::new())
        } else if e.pn.trim().is_empty() {
            item("pn", "PN definido", EstadoItem::Falha, "o documento está sem PN".into())
        } else {
            item("pn", "PN definido", EstadoItem::Falha, format!("o PN {} já é de outro documento", e.pn))
        });

        itens.push(match ext.desenho_atualizado {
            Some(true) => item("desenho", "Desenho atualizado", EstadoItem::Ok, String::new()),
            Some(false) => item("desenho", "Desenho atualizado", EstadoItem::Falha, "o desenho está desatualizado".into()),
            None => {
                // sem informação de quem tem o desenho aberto: compara a data de gravação de cada
                // desenho que usa a peça com a data da última versão
                let desenhos: Vec<relacoes::Uso> = self.onde_usado(id)?.into_iter().filter(|u| u.arquivo.extension().and_then(|x| x.to_str()) == Some("fkd")).collect();
                if desenhos.is_empty() {
                    item("desenho", "Desenho atualizado", EstadoItem::NaoAplicavel, "nenhum desenho usa esta peça".into())
                } else {
                    let velhos: Vec<String> = desenhos
                        .iter()
                        .filter(|u| {
                            let gravado = std::fs::metadata(&u.arquivo).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
                            gravado < versao.time
                        })
                        .map(|u| u.arquivo.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
                        .collect();
                    if velhos.is_empty() {
                        item("desenho", "Desenho atualizado", EstadoItem::Ok, String::new())
                    } else {
                        item("desenho", "Desenho atualizado", EstadoItem::Falha, format!("gravado antes da última versão da peça: {}", velhos.join(", ")))
                    }
                }
            }
        });
        // qualidade do modelo (B305): só bloqueia o que o perfil marca como bloqueio e que não tem exceção
        itens.push(match &ext.qualidade {
            None => item("qualidade", "Qualidade do modelo", EstadoItem::NaoVerificado, "o documento não está aberto para medir".into()),
            Some(m) => {
                let r = self.avaliar_qualidade(id, m, qualidade_modelo::Momento::Revisar)?;
                if r.dispensado {
                    item("qualidade", "Qualidade do modelo", EstadoItem::NaoAplicavel, "o perfil não confere na revisão".into())
                } else if r.bloqueia() {
                    item("qualidade", "Qualidade do modelo", EstadoItem::Falha, r.resumo_bloqueios())
                } else {
                    item("qualidade", "Qualidade do modelo", EstadoItem::Ok, String::new())
                }
            }
        });
        Ok(estados::Conferencia { itens })
    }

    /// Libera uma revisão a partir da última versão salva e a congela (RF-GPR-011). Antes da letra:
    /// a transição para `Liberado` precisa existir na tabela para o papel do usuário e a lista de
    /// conferência não pode ter falha (B265). A letra segue o esquema de revisão do ciclo.
    pub fn liberar(&mut self, id: u64, comment: &str, ext: &estados::Externos) -> PResult<String> {
        self.pode_gravar()?;
        if !self.entry(id)?.dirty_since_release() {
            return Err(ProjectError::NothingToRelease);
        }
        let estado_antes = self.entry(id)?.state;
        self.index.ciclo.verificar(estado_antes, DocState::Liberado, &self.papeis, comment).map_err(ProjectError::Recusado)?;
        let conferencia = self.conferir_liberacao(id, ext)?;
        if conferencia.bloqueia() {
            return Err(ProjectError::Conferencia(conferencia.resumo()));
        }
        let antes = self.entry(id)?.clone();
        let esquema = self.index.ciclo.esquema_revisao;
        let e = self.entry_mut(id)?;
        let hash = e.versions.last().unwrap().hash.clone();
        let label = esquema.proxima(e.revisions.last().map(|r| r.label.as_str()));
        e.revisions.push(Revision { label: label.clone(), hash, time: now(), comment: comment.into() });
        e.state = DocState::Liberado;
        if let Err(err) = self.save_index() {
            *self.entry_mut(id)? = antes;
            return Err(err);
        }
        self.registrar(diario::TipoEvento::Revisao, id, "", &label, comment)?;
        self.registrar(diario::TipoEvento::Transicao, id, estado_antes.label(), DocState::Liberado.label(), comment)?;
        Ok(label)
    }

    /// Atalho para quem não tem o documento aberto: libera sem informar o recálculo (o item aparece
    /// como não verificado na lista). A interface usa [`Project::liberar`] com o recálculo.
    pub fn release(&mut self, id: u64, comment: &str) -> PResult<String> {
        self.liberar(id, comment, &estados::Externos::default())
    }

    /// Pastas varridas para achar montagens e desenhos que usam os documentos do projeto.
    pub fn pastas_de_busca(&self) -> Vec<PathBuf> {
        let mut v = vec![self.root.clone()];
        for p in &self.index.pastas_busca {
            let c = PathBuf::from(p);
            v.push(if c.is_absolute() { c } else { self.root.join(c) });
        }
        v
    }

    /// Quem usa o documento `id`: montagens (com o nó) e desenhos (com a vista). A lista vem dos
    /// cabeçalhos e das referências gravadas, sem abrir nem recalcular nenhum documento (B94).
    pub fn onde_usado(&self, id: u64) -> PResult<Vec<relacoes::Uso>> {
        let e = self.entry(id)?;
        Ok(relacoes::onde_usado(&self.pastas_de_busca(), &self.root.join(&e.file), &e.uid))
    }

    /// Cabeçalho (uid, nome, PN) de cada documento do projeto lido do arquivo de trabalho, sem abrir
    /// o documento inteiro. Serve para conferir se o índice e os arquivos ainda concordam.
    pub fn cabecalhos(&self) -> Vec<(u64, Option<relacoes::Cabecalho>)> {
        self.index.documents.iter().map(|d| (d.id, relacoes::ler_cabecalho(&self.root.join(&d.file)))).collect()
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
        p.transicionar(id, DocState::Obsoleto, "substituída pelo modelo 2").unwrap();
        let (ev, puladas) = diario::ler(&root);
        assert_eq!(puladas, 0);
        let t = ev.iter().find(|e| e.tipo == diario::TipoEvento::Transicao).expect("transição registrada");
        assert_eq!((t.autor.as_str(), t.de.as_str(), t.para.as_str()), ("ana", "Em trabalho", "Obsoleto"));
        assert_eq!(t.motivo, "substituída pelo modelo 2");
        assert!(t.tempo > 0 && t.doc_id == id && t.uid.len() == 32);
        // mudar de estado de novo só acrescenta: as linhas anteriores ficam idênticas
        let antes = std::fs::read_to_string(diario::caminho(&root)).unwrap();
        p.transicionar(id, DocState::EmTrabalho, "voltou").unwrap();
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
        // o aplicativo copia o PN que o projeto atribuiu para o documento aberto; o teste faz o mesmo
        d.props.pn = p.entry(id).unwrap().pn.clone();
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

    #[test]
    fn mascara_e_ambientes_numeram_separado_e_guardam_o_contador() {
        let root = tmp("amb");
        let mut p = Project::create(&root, "T").unwrap();
        p.definir_ambiente_pn(pn::AmbientePn::novo("Produção", "FK", "{PREFIXO}-{TIPO}-{SEQ:4}")).unwrap();
        p.definir_ambiente_pn(pn::AmbientePn::novo("Protótipo", "PR", "{PREFIXO}/{ANO}/{SEQ:3}")).unwrap();
        assert!(p.definir_ambiente_pn(pn::AmbientePn::novo("Ruim", "X", "sem sequência")).is_err());
        p.ativar_ambiente_pn("Produção").unwrap();
        assert_eq!(p.pn_mascara_ativa(), "{PREFIXO}-{TIPO}-{SEQ:4}");
        let a = p.add_document(Document::new("a")).unwrap();
        let b = p.add_document(Document::new("b")).unwrap();
        assert_eq!(p.entry(a).unwrap().pn, "FK-FAB-0001");
        assert_eq!(p.entry(b).unwrap().pn, "FK-FAB-0002");
        // o protótipo tem contador próprio
        p.ativar_ambiente_pn("Protótipo").unwrap();
        let c = p.add_document(Document::new("c")).unwrap();
        assert_eq!(p.entry(c).unwrap().pn, format!("PR/{}/001", pn::ano_atual()));
        // voltando à produção, a sequência continua de onde parou
        p.ativar_ambiente_pn("Produção").unwrap();
        assert_eq!(p.pn_previsto(), "FK-FAB-0003");
        // o tipo da peça entra na máscara
        let mut comprada = Document::new("parafuso");
        comprada.props.part_type = fenkra_core::PartType::Comprada;
        assert_eq!(p.pn_previsto_para(&comprada), "FK-COM-0003");
        // reabrir preserva ambientes e contadores
        let p2 = Project::open(&root).unwrap();
        assert_eq!(p2.index.pn_ambiente, "Produção");
        assert_eq!(p2.pn_previsto(), "FK-FAB-0003");
        assert!(matches!(p.ativar_ambiente_pn("Inexistente"), Err(ProjectError::Recusado(_))));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn falha_de_gravacao_nao_avanca_a_sequencia_do_ambiente() {
        let root = tmp("amb-falha");
        let mut p = Project::create(&root, "T").unwrap();
        p.definir_ambiente_pn(pn::AmbientePn::novo("Produção", "FK", "{PREFIXO}-{SEQ:4}")).unwrap();
        p.ativar_ambiente_pn("Produção").unwrap();
        p.add_document(Document::new("a")).unwrap();
        std::fs::remove_dir_all(root.join(VERSIONS)).unwrap();
        std::fs::write(root.join(VERSIONS), b"x").unwrap();
        assert!(p.add_document(Document::new("b")).is_err());
        assert_eq!(p.pn_previsto(), "FK-0002", "a sequência do ambiente não avançou");
        assert_eq!(p.index.pn_ambientes[0].proximo, 2);
        assert_eq!(Project::open(&root).unwrap().index.pn_ambientes[0].proximo, 2, "nem no disco");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn pn_ocupado_por_outro_documento_e_pulado_sem_erro() {
        let root = tmp("pn-pula");
        let mut p = Project::create(&root, "T").unwrap();
        let mut manual = Document::new("manual");
        manual.props.pn = "FK-0001".into();
        p.add_document(manual).unwrap();
        let id = p.add_document(Document::new("auto")).unwrap();
        assert_eq!(p.entry(id).unwrap().pn, "FK-0002");
        assert_eq!(p.pn_previsto(), "FK-0003");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn previa_ao_vivo_marca_o_que_falta() {
        let root = tmp("previa");
        let mut p = Project::create(&root, "T").unwrap();
        p.definir_ambiente_pn(pn::AmbientePn::novo("Cliente", "FK", "{PREFIXO}-{CAMPO:cliente}-{SEQ:3}")).unwrap();
        p.ativar_ambiente_pn("Cliente").unwrap();
        let mut d = Document::new("x");
        let s = p.pn_previa(&d).unwrap();
        assert_eq!(pn::texto(&s), "FK-CLIENTE-001");
        assert!(!pn::completo(&s), "falta preencher o cliente");
        d.props.custom.push(("Cliente".into(), "ACME".into()));
        let s = p.pn_previa(&d).unwrap();
        assert_eq!(pn::texto(&s), "FK-ACME-001");
        assert!(pn::completo(&s));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn onde_usado_lista_montagem_no_e_desenho_do_documento_do_projeto() {
        let root = tmp("usado-proj");
        let mut p = Project::create(&root, "T").unwrap();
        let id = p.add_document(Document::new("Eixo")).unwrap();
        let outro = p.add_document(Document::new("Mancal")).unwrap();
        let caminho = p.entry(id).unwrap().file.clone();
        std::fs::write(
            root.join("conj.fka"),
            format!(r#"{{"format_version": 1, "name": "Conjunto", "nodes": [{{"id": 4, "name": "Eixo 1", "kind": {{"Part": {{"path": "{caminho}"}}}}}}]}}"#),
        )
        .unwrap();
        std::fs::create_dir_all(root.join("desenhos")).unwrap();
        std::fs::write(
            root.join("desenhos").join("d.fkd"),
            format!(r#"{{"name": "Des", "views": [{{"id": 1, "name": "Frente", "source": {{"path": "../{caminho}"}}}}]}}"#),
        )
        .unwrap();
        let usos = p.onde_usado(id).unwrap();
        assert_eq!(usos.len(), 2);
        let m = usos.iter().find(|u| u.tipo == relacoes::TipoRef::Peca).unwrap();
        assert_eq!((m.no, m.nome_no.as_str(), m.nome.as_str()), (Some(4), "Eixo 1", "Conjunto"));
        assert!(usos.iter().any(|u| u.tipo == relacoes::TipoRef::Vista));
        assert!(p.onde_usado(outro).unwrap().is_empty(), "o outro documento não é usado por ninguém");
        // pasta fora do projeto só entra se for cadastrada
        let fora = tmp("usado-fora");
        std::fs::create_dir_all(&fora).unwrap();
        std::fs::write(
            fora.join("externa.fka"),
            format!(r#"{{"name": "Externa", "nodes": [{{"id": 1, "name": "E", "kind": {{"Part": {{"path": {}}}}}}}]}}"#, serde_json::to_string(&root.join(&caminho).display().to_string()).unwrap()),
        )
        .unwrap();
        assert_eq!(p.onde_usado(id).unwrap().len(), 2);
        p.index.pastas_busca.push(fora.display().to_string());
        assert_eq!(p.onde_usado(id).unwrap().len(), 3);
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(fora);
    }

    #[test]
    fn cabecalhos_do_projeto_concordam_com_o_indice_sem_abrir_os_documentos() {
        let root = tmp("cabec");
        let mut p = Project::create(&root, "T").unwrap();
        let id = p.add_document(Document::new("Placa")).unwrap();
        let cabs = p.cabecalhos();
        let c = cabs.iter().find(|(i, _)| *i == id).unwrap().1.clone().unwrap();
        assert_eq!((c.nome.as_str(), c.pn.as_str()), ("Placa", p.entry(id).unwrap().pn.as_str()));
        assert_eq!(c.uid, p.entry(id).unwrap().uid);
        let _ = std::fs::remove_dir_all(root);
    }

    /// Cria uma versão com conteúdo arbitrário (para montar documentos com referências no teste).
    fn versao_manual(p: &mut Project, id: u64, nome: &str, json: &str) {
        std::fs::write(p.root.join(VERSIONS).join(format!("{nome}.fkp")), json).unwrap();
        let e = p.entry_mut(id).unwrap();
        e.versions.push(Version { hash: nome.into(), time: now(), note: "manual".into(), author: String::new(), comment: String::new(), restored_from: None, estado: String::new() });
    }

    #[test]
    fn transicao_fora_da_tabela_e_recusada_com_motivo_e_o_estado_nao_muda() {
        let root = tmp("trans");
        let mut p = Project::create(&root, "T").unwrap();
        let id = p.add_document(Document::new("a")).unwrap();
        // pedir motivo
        let e = p.transicionar(id, DocState::Obsoleto, "").unwrap_err().to_string();
        assert!(e.contains("exige um motivo"), "{e}");
        assert_eq!(p.entry(id).unwrap().state, DocState::EmTrabalho);
        p.transicionar(id, DocState::Obsoleto, "descontinuada").unwrap();
        // Obsoleto -> Liberado não existe na tabela: a revisão não é criada
        let mut d = Document::new("a");
        d.props.description = "mudou".into();
        p.save(id, &d, "v2").unwrap();
        let e = p.liberar(id, "x", &estados::Externos::default()).unwrap_err().to_string();
        assert!(e.contains("não existe no ciclo") && e.contains("Obsoleto"), "{e}");
        assert!(p.entry(id).unwrap().revisions.is_empty());
        assert_eq!(p.entry(id).unwrap().state, DocState::Obsoleto);
        // ir para Liberado só pelo liberar
        p.transicionar(id, DocState::EmTrabalho, "voltou").unwrap();
        assert!(p.transicionar(id, DocState::Liberado, "x").unwrap_err().to_string().contains("Liberar revisão"));
        // a tabela vale também para o que sobe: mudar para o mesmo estado é recusado
        assert!(p.transicionar(id, DocState::EmTrabalho, "x").is_err());
        // o diário só tem as transições que aconteceram
        let (ev, _) = diario::ler(&root);
        let trans: Vec<(&str, &str)> = ev.iter().filter(|e| e.tipo == diario::TipoEvento::Transicao).map(|e| (e.de.as_str(), e.para.as_str())).collect();
        assert_eq!(trans, vec![("Em trabalho", "Obsoleto"), ("Obsoleto", "Em trabalho")]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn liberar_com_referencia_quebrada_e_bloqueado_e_depois_de_reparar_libera() {
        let root = tmp("lib-ref");
        let mut p = Project::create(&root, "T").unwrap();
        let id = p.add_document(Document::new("a")).unwrap();
        versao_manual(&mut p, id, "comref", r#"{"format_version":1,"uid":"x","name":"a","links":[{"uid":"u-alvo","caminho":"../alvo.fkp"}]}"#);
        let ext = estados::Externos { recalculo: Some(Ok(())), desenho_atualizado: None, qualidade: None };
        let conf = p.conferir_liberacao(id, &ext).unwrap();
        assert!(conf.bloqueia());
        let f = conf.falhas().next().unwrap();
        assert_eq!(f.chave, "referencias");
        assert!(f.detalhe.contains("../alvo.fkp") && f.detalhe.contains("Reparar links"), "{}", f.detalhe);
        let e = p.liberar(id, "primeira", &ext).unwrap_err();
        assert!(matches!(e, ProjectError::Conferencia(_)), "{e}");
        assert!(p.entry(id).unwrap().revisions.is_empty(), "nenhuma letra foi criada");
        assert_eq!(p.entry(id).unwrap().state, DocState::EmTrabalho);
        // a referência passa a resolver (o arquivo existe com o uid certo)
        std::fs::write(root.join("alvo.fkp"), r#"{"format_version":1,"uid":"u-alvo","name":"alvo"}"#).unwrap();
        assert!(!p.conferir_liberacao(id, &ext).unwrap().bloqueia());
        assert_eq!(p.liberar(id, "primeira", &ext).unwrap(), "A");
        assert_eq!(p.entry(id).unwrap().state, DocState::Liberado);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn referencia_movida_tambem_bloqueia_porque_o_caminho_gravado_esta_errado() {
        let root = tmp("lib-movida");
        let mut p = Project::create(&root, "T").unwrap();
        let id = p.add_document(Document::new("a")).unwrap();
        versao_manual(&mut p, id, "movida", r#"{"format_version":1,"uid":"x","name":"a","links":[{"uid":"u-alvo","caminho":"../antigo.fkp"}]}"#);
        std::fs::create_dir_all(root.join("novo")).unwrap();
        std::fs::write(root.join("novo/alvo.fkp"), r#"{"format_version":1,"uid":"u-alvo","name":"alvo"}"#).unwrap();
        let e = p.release(id, "x").unwrap_err();
        assert!(matches!(e, ProjectError::Conferencia(m) if m.contains("antigo.fkp")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn falha_de_recalculo_e_desenho_desatualizado_bloqueiam() {
        let root = tmp("lib-conf");
        let mut p = Project::create(&root, "T").unwrap();
        let id = p.add_document(Document::new("a")).unwrap();
        let ok = estados::Externos { recalculo: Some(Ok(())), desenho_atualizado: None, qualidade: None };
        // recálculo com erro
        let ruim = estados::Externos { recalculo: Some(Err("extrusão sem perfil".into())), desenho_atualizado: None, qualidade: None };
        let e = p.liberar(id, "x", &ruim).unwrap_err().to_string();
        assert!(e.contains("Recalcula sem erro") && e.contains("extrusão sem perfil"), "{e}");
        // sem desenho: não se aplica
        let conf = p.conferir_liberacao(id, &ok).unwrap();
        assert_eq!(conf.itens.iter().find(|i| i.chave == "desenho").unwrap().estado, estados::EstadoItem::NaoAplicavel);
        // desenho gravado antes da última versão da peça
        let caminho = p.entry(id).unwrap().file.clone();
        let desenho = root.join("d.fkd");
        std::fs::write(&desenho, format!(r#"{{"name":"D","views":[{{"id":1,"name":"F","source":{{"path":"{caminho}"}}}}]}}"#)).unwrap();
        let velho = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        std::fs::File::options().write(true).open(&desenho).unwrap().set_modified(velho).unwrap();
        let e = p.liberar(id, "x", &ok).unwrap_err().to_string();
        assert!(e.contains("Desenho atualizado") && e.contains("d.fkd"), "{e}");
        // quem tem o desenho aberto pode afirmar que está em dia
        let em_dia = estados::Externos { recalculo: Some(Ok(())), desenho_atualizado: Some(true), qualidade: None };
        assert_eq!(p.liberar(id, "x", &em_dia).unwrap(), "A");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn modelo_com_aprovacao_exige_o_papel_e_edicao_durante_aprovacao_volta_a_trabalho() {
        let root = tmp("aprov");
        let mut p = Project::create(&root, "T").unwrap();
        p.definir_ciclo(estados::Ciclo::com_aprovacao()).unwrap();
        p.papeis = vec!["projetista".into()];
        let mut d = Document::new("a");
        let id = p.add_document(d.clone()).unwrap();
        // o aplicativo copia o PN que o projeto atribuiu para o documento aberto; o teste faz o mesmo
        d.props.pn = p.entry(id).unwrap().pn.clone();
        let e = p.release(id, "x").unwrap_err().to_string();
        assert!(e.contains("exige o papel aprovador"), "{e}");
        p.transicionar(id, DocState::EmAprovacao, "").unwrap();
        assert_eq!(p.entry(id).unwrap().state, DocState::EmAprovacao);
        assert!(p.release(id, "x").unwrap_err().to_string().contains("aprovador"));
        // quem edita durante a aprovação invalida o pedido
        d.props.description = "mexi".into();
        p.save(id, &d, "v2").unwrap();
        assert_eq!(p.entry(id).unwrap().state, DocState::EmTrabalho);
        p.transicionar(id, DocState::EmAprovacao, "").unwrap();
        // o aprovador libera
        p.papeis.push("aprovador".into());
        assert_eq!(p.release(id, "aprovado").unwrap(), "A");
        // o disponível a partir de Liberado para um projetista sem o papel de aprovador é só reabrir
        p.papeis = vec!["projetista".into()];
        let destinos: Vec<DocState> = p.transicoes_disponiveis(id).unwrap().iter().map(|t| t.para).collect();
        assert_eq!(destinos, vec![DocState::EmTrabalho]);
        // o ciclo escolhido sobrevive à reabertura
        assert_eq!(Project::open(&root).unwrap().index.ciclo.nome, "Com aprovação");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn esquema_de_revisao_configuravel_numeros_e_previa() {
        let root = tmp("esquema");
        let mut p = Project::create(&root, "T").unwrap();
        let mut ciclo = estados::Ciclo::padrao();
        ciclo.esquema_revisao = estados::EsquemaRevisao::Previa;
        p.definir_ciclo(ciclo.clone()).unwrap();
        let mut d = Document::new("a");
        let id = p.add_document(d.clone()).unwrap();
        // o aplicativo copia o PN que o projeto atribuiu para o documento aberto; o teste faz o mesmo
        d.props.pn = p.entry(id).unwrap().pn.clone();
        assert_eq!(p.release(id, "").unwrap(), "0.1");
        d.props.description = "2".into();
        p.save(id, &d, "v2").unwrap();
        assert_eq!(p.release(id, "").unwrap(), "0.2");
        // vira para letras no meio do caminho: recomeça em A
        ciclo.esquema_revisao = estados::EsquemaRevisao::Letras;
        p.definir_ciclo(ciclo.clone()).unwrap();
        d.props.description = "3".into();
        p.save(id, &d, "v3").unwrap();
        assert_eq!(p.release(id, "").unwrap(), "A");
        ciclo.esquema_revisao = estados::EsquemaRevisao::Numeros;
        p.definir_ciclo(ciclo).unwrap();
        d.props.description = "4".into();
        p.save(id, &d, "v4").unwrap();
        assert_eq!(p.release(id, "").unwrap(), "1");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn ciclo_novo_sem_o_estado_do_documento_devolve_ao_inicial_e_registra() {
        let root = tmp("ciclo-troca");
        let mut p = Project::create(&root, "T").unwrap();
        let id = p.add_document(Document::new("a")).unwrap();
        p.transicionar(id, DocState::Obsoleto, "fim").unwrap();
        let json = r#"{"nome":"Dois","estados":["EmTrabalho","Liberado"],"inicial":"EmTrabalho","transicoes":[{"de":"EmTrabalho","para":"Liberado"}]}"#;
        p.definir_ciclo_json(json).unwrap();
        assert_eq!(p.entry(id).unwrap().state, DocState::EmTrabalho);
        let (ev, _) = diario::ler(&root);
        assert!(ev.iter().any(|e| e.motivo.contains("não existe no ciclo novo")));
        // ciclo inválido não troca nada
        assert!(p.definir_ciclo_json(r#"{"nome":"x","estados":[],"inicial":"EmTrabalho","transicoes":[]}"#).is_err());
        assert_eq!(p.index.ciclo.nome, "Dois");
        // índice antigo, sem ciclo, abre com o modelo padrão
        let antigo = INDICE_ALPHA28;
        let r2 = tmp("ciclo-antigo");
        std::fs::create_dir_all(&r2).unwrap();
        std::fs::write(r2.join(INDEX), antigo).unwrap();
        assert_eq!(Project::open(&r2).unwrap().index.ciclo, estados::Ciclo::padrao());
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(r2);
    }
}
