// SPDX-License-Identifier: LGPL-2.1-or-later
//! Causa estável de cada falha de recálculo (B69).
//!
//! A mensagem de um comando que falhou é texto para a pessoa e pode mudar de redação; o painel de
//! Diagnóstico, os testes e a automação precisam de algo que não mude. Por isso cada comando com
//! erro recebe um [`ErrorKind`], decidido pela **estrutura do documento** (o que o comando cita e
//! se isso existe ou falhou), não pelo texto.

use crate::Status;
use fenkra_core::doc::{Document, Id};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// o comando cita algo que não existe mais no documento (apagado ou nunca criado)
    ReferenciaPerdida { ausente: Id },
    /// um pai do comando falhou antes dele; `causa` é o primeiro pai que falhou na linha do tempo
    PaiFalhou { causa: Id },
    /// o comando resolveu todas as referências, mas a face/aresta que ele nomeava deixou de
    /// existir na geometria depois de um recálculo (nomeação topológica)
    GeometriaPerdida,
    /// o comando falhou por si: entrada inválida ou operação que o kernel não completou
    Propria,
}

impl ErrorKind {
    /// Código curto que não muda entre versões (serve a testes, relatórios e à automação).
    pub fn codigo(&self) -> &'static str {
        match self {
            ErrorKind::ReferenciaPerdida { .. } => "referencia_perdida",
            ErrorKind::PaiFalhou { .. } => "pai_falhou",
            ErrorKind::GeometriaPerdida => "geometria_perdida",
            ErrorKind::Propria => "propria",
        }
    }

    /// Comando para o qual a correção deve ir primeiro (`None` = o próprio).
    pub fn corrigir_primeiro(&self) -> Option<Id> {
        match self {
            ErrorKind::PaiFalhou { causa } => Some(*causa),
            _ => None,
        }
    }
}

fn existe(doc: &Document, id: Id) -> bool {
    doc.feature(id).is_some() || doc.reference(id).is_some() || doc.collectors.iter().any(|c| c.id == id) || doc.folders.iter().any(|f| f.id == id) || doc.external.iter().any(|e| e.id == id)
}

fn pais(doc: &Document, id: Id) -> Vec<Id> {
    if let Some(f) = doc.feature(id) {
        return Document::refs_of(&f.kind).into_iter().filter(|x| *x != id && (doc.feature(*x).is_some() || doc.reference(*x).is_some())).collect();
    }
    if let Some(r) = doc.reference(id) {
        return r.kind.deps().into_iter().filter(|x| *x != id).collect();
    }
    vec![]
}

fn ancestrais(doc: &Document, id: Id) -> Vec<Id> {
    let mut out: Vec<Id> = vec![];
    let mut fila = pais(doc, id);
    while let Some(p) = fila.pop() {
        if !out.contains(&p) {
            out.push(p);
            fila.extend(pais(doc, p));
        }
    }
    out
}

/// Classifica cada comando com `Status::Error`.
pub fn classificar(doc: &Document, status: &HashMap<Id, Status>, _param_error: Option<&str>) -> HashMap<Id, ErrorKind> {
    let falhou = |id: Id| matches!(status.get(&id), Some(Status::Error(_)));
    let mut out = HashMap::new();
    for f in doc.features.iter().filter(|f| falhou(f.id)) {
        let ausente = Document::refs_of(&f.kind).into_iter().find(|x| !existe(doc, *x));
        let causa = ancestrais(doc, f.id).into_iter().filter(|a| falhou(*a)).min_by_key(|a| doc.feature_index(*a).map_or(0, |i| i + 1));
        let kind = if let Some(ausente) = ausente {
            ErrorKind::ReferenciaPerdida { ausente }
        } else if let Some(causa) = causa {
            ErrorKind::PaiFalhou { causa }
        } else {
            match status.get(&f.id) {
                Some(Status::Error(m)) if m.contains("não existe mais") || m.contains("referência perdida") => ErrorKind::GeometriaPerdida,
                _ => ErrorKind::Propria,
            }
        };
        out.insert(f.id, kind);
    }
    out
}
