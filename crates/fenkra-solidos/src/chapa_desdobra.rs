// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desdobrar e redobrar parcial (B35).
//!
//! A chapa tem um estado global (dobrada ou planificada). O desdobrar parcial deixa algumas abas
//! planificadas enquanto o resto fica dobrado, para editar uma parte (um recorte que atravessa a
//! dobra, uma medida na aba) sem planificar a peça inteira. O estado de cada aba sai de todos os
//! comandos `SmUnfoldPartial` da árvore, na ordem: desdobrar põe a aba no conjunto, redobrar tira.
//! Cada aba é refeita no estado dela: a dobra vira a tira plana do comprimento desenvolvido.

use super::*;
use fenkra_core::SmUnfoldPartial;
use std::cell::Cell;
use std::collections::HashSet;

thread_local! {
    /// o comando em execução, para `Ctx::sheet` saber se ele está desdobrado
    static ATUAL: Cell<u64> = const { Cell::new(0) };
}

pub(super) fn marca(id: Id) {
    ATUAL.with(|c| c.set(id.0));
}

pub(super) fn atual() -> Id {
    Id(ATUAL.with(|c| c.get()))
}

/// As abas desdobradas ao fim da árvore.
pub fn desdobradas(doc: &Document) -> HashSet<Id> {
    let mut s = HashSet::new();
    for (_, f) in doc.active_features() {
        if let FeatureKind::SmUnfoldPartial(u) = &f.kind {
            for id in &u.features {
                if u.refold {
                    s.remove(id);
                } else {
                    s.insert(*id);
                }
            }
        }
    }
    s
}

/// O comando em execução é uma aba desdobrada pelo desdobrar parcial?
pub(super) fn desdobrada_agora(doc: &Document) -> bool {
    doc.features.iter().any(|f| matches!(f.kind, FeatureKind::SmUnfoldPartial(_))) && desdobradas(doc).contains(&atual())
}

/// O comando é uma aba que dobra (e portanto pode ser desdobrada)?
pub fn e_dobra(k: &FeatureKind) -> bool {
    matches!(k, FeatureKind::SmFlange(_) | FeatureKind::SmFlangeLoop(_) | FeatureKind::SmCurveFlange(_) | FeatureKind::SmProfileFlange(_) | FeatureKind::SmSurfFlange(_) | FeatureKind::SmHem(_))
}

impl Ctx<'_> {
    /// O comando em si não muda o corpo: confere que cada aba existe e dobra. O efeito está em
    /// refazer cada aba no estado dela (`desdobrada_agora`).
    pub(super) fn run_unfold_partial(&mut self, f: &Feature, u: &SmUnfoldPartial) -> Result<Option<String>, String> {
        if u.features.is_empty() {
            return Err(if u.refold { "escolha pelo menos uma aba para redobrar".into() } else { "escolha pelo menos uma aba para desdobrar".into() });
        }
        let aqui = self.doc.features.iter().position(|g| g.id == f.id).unwrap_or(usize::MAX);
        for id in &u.features {
            let Some(pos) = self.doc.features.iter().position(|g| g.id == *id) else { return Err(format!("a aba #{} não existe mais: reescolha as abas", id.0)) };
            if !e_dobra(&self.doc.features[pos].kind) {
                return Err(format!("'{}' não é uma aba com dobra: só abas e bainhas desdobram", self.doc.features[pos].name));
            }
            if pos > aqui {
                return Err(format!("'{}' vem depois deste comando: desdobre só abas que já existem", self.doc.features[pos].name));
            }
        }
        Ok(None)
    }
}
