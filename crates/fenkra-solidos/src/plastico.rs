// SPDX-License-Identifier: LGPL-2.1-or-later
//! B198: diagnóstico da regra plástica do documento na árvore.
//!
//! Depois do recálculo, cada recurso que a regra governa tem o valor dele comparado com os limites
//! (`RegraPlastica::confere`): casca e parede fina (parede), nervura, saída, filete (raio) e os
//! recursos de plástico (boss, snap-fit, grelha, lip). O que sai da regra vira aviso no status do
//! comando, somado ao aviso que ele já tiver. Erro e comando inativo ficam como estão.
//!
//! O diagnóstico roda fora do cache dos comandos: mudar só um limite da regra (sem mudar valor
//! nenhum que um comando lê) já atualiza os avisos, sem refazer geometria.

use crate::Status;
use fenkra_core::regra_plastica::{Grandeza, RegraPlastica};
use fenkra_core::{expr, Document, EdgeMode, FeatureKind, Id};
use std::collections::HashMap;

/// O que de cada comando a regra confere: (grandeza, fórmula do campo, nome do campo).
pub fn campos(k: &FeatureKind) -> Vec<(Grandeza, &str, &'static str)> {
    match k {
        FeatureKind::Shell(s) => {
            let mut v = vec![(Grandeza::Parede, s.thickness.as_str(), "espessura")];
            v.extend(s.espessuras.iter().map(|e| (Grandeza::Parede, e.valor.as_str(), "espessura da face")));
            v
        }
        FeatureKind::Extrude(e) => e.thin.iter().map(|t| (Grandeza::Parede, t.as_str(), "parede fina")).collect(),
        FeatureKind::Stiffener(s) => vec![(Grandeza::Nervura, s.thickness.as_str(), "espessura")],
        FeatureKind::Draft(d) => vec![(Grandeza::Saida, d.angle.as_str(), "ângulo")],
        FeatureKind::Fillet(f) if matches!(f.mode, EdgeMode::Constant) => vec![(Grandeza::Raio, f.value.as_str(), "raio")],
        // B148: boss, snap-fit, grelha e lip
        FeatureKind::Plastico(p) => p.campos_regra(),
        _ => vec![],
    }
}

/// Avisos de um comando contra a regra.
pub fn avisos(regra: &RegraPlastica, k: &FeatureKind, vars: &HashMap<String, f64>) -> Vec<String> {
    let mut out = vec![];
    for (g, f, nome) in campos(k) {
        let Ok(v) = expr::eval_str(f, vars) else { continue };
        if let Some(m) = regra.confere(g, v) {
            out.push(format!("{nome}: {m}"));
        }
    }
    out
}

/// Soma os avisos da regra plástica ao status de cada comando.
pub fn diagnosticar(doc: &Document, vars: &HashMap<String, f64>, status: &mut HashMap<Id, Status>) {
    let Some(regra) = &doc.plastico else { return };
    for f in &doc.features {
        let Some(st) = status.get_mut(&f.id) else { continue };
        if matches!(st, Status::Error(_) | Status::Inactive) {
            continue;
        }
        let a = avisos(regra, &f.kind, vars);
        if a.is_empty() {
            continue;
        }
        let msg = format!("Regra plástica {}: {}", regra.nome, a.join("; "));
        *st = match std::mem::replace(st, Status::Ok) {
            Status::Warning(w) => Status::Warning(format!("{w}; {msg}")),
            _ => Status::Warning(msg),
        };
    }
}
