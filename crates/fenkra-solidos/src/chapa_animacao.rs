// SPDX-License-Identifier: LGPL-2.1-or-later
//! Dobrar e desdobrar animado (B304): a peça entre dobrada (0) e planificada (1).
//!
//! Como funciona. Os dois extremos são os estados de verdade: em 0 o documento volta igual, em 1
//! só liga `flat` (a planificada exata, com o comprimento desenvolvido). No meio, cada dobra que
//! tem ângulo no comando abre na mesma proporção: o ângulo vira `(ângulo) * (1 - f)`, escrito como
//! expressão para preservar as variáveis, e a peça é recalculada como qualquer outra. Como o arco da
//! dobra (BA) é proporcional ao ângulo, a perna reta da aba ganha `f * BA`: o comprimento
//! desenvolvido não muda no caminho, e o 100% emenda com a planificada. Recortes,
//! furos e abas feitos depois seguem o mapa de dobras, então andam junto com a aba.
//!
//! O que não tem ângulo no comando só muda no fim: bainha (sempre fechada), aba por perfil (as
//! dobras estão no sketch), aba entre perfis e corpo convertido. `fixos` lista esses comandos para a
//! interface dizer isso ao lado do controle.

use fenkra_core::*;

/// Fração máxima no meio do caminho: acima dela a dobra quase plana fica abaixo do ângulo mínimo
/// aceito (0,01°); o 100% é a planificada de verdade.
pub const MAX_MEIO: f64 = 0.995;

fn escala(expr: &str, fator: f64) -> String {
    format!("({})*{:.6}", expr.trim(), fator)
}

fn soma(expr: &str, mais: f64) -> String {
    format!("({})+{:.6}", expr.trim(), mais)
}

/// Documento da peça com as dobras abertas na fração `f` (0 dobrada, 1 planificada).
pub fn doc_parcial(doc: &Document, f: f64) -> Document {
    let mut d = doc.clone();
    let f = f.clamp(0., 1.);
    if f <= 0. {
        if let Some(sp) = d.sheet.as_mut() {
            sp.flat = false;
        }
        return d;
    }
    if f >= 1. {
        if let Some(sp) = d.sheet.as_mut() {
            sp.flat = true;
        }
        return d;
    }
    if let Some(sp) = d.sheet.as_mut() {
        sp.flat = false;
    }
    let fator = 1. - f.min(MAX_MEIO);
    // o arco de uma aba encolhe com o ângulo (BA proporcional a ele): a perna reta ganha o que o arco
    // perdeu, assim o desenvolvido (e o volume) fica o mesmo do começo ao fim
    let ba: std::collections::HashMap<Id, f64> = crate::bend_table(doc).into_iter().map(|r| (r.feature, r.allowance)).collect();
    let ganho = |id: Id| ba.get(&id).copied().unwrap_or(0.) * (1. - fator);
    for feat in d.features.iter_mut() {
        let g = ganho(feat.id);
        match &mut feat.kind {
            FeatureKind::SmFlange(x) => {
                x.angle = escala(&x.angle, fator);
                x.length = soma(&x.length, g);
            }
            FeatureKind::SmFlangeLoop(x) => {
                x.angle = escala(&x.angle, fator);
                x.length = soma(&x.length, g);
            }
            FeatureKind::SmBend(x) => x.angle = escala(&x.angle, fator),
            FeatureKind::SmCurveFlange(x) => {
                x.angle = escala(if x.angle.trim().is_empty() { "90" } else { &x.angle }, fator);
                x.length = soma(&x.length, g);
            }
            FeatureKind::SmSurfFlange(x) if x.mode == 2 => x.angle = escala(&x.angle, fator),
            // joggle: a rampa abaixa junto (profundidade proporcional)
            FeatureKind::SmJoggle(x) => {
                x.depth = escala(&x.depth, fator);
                if let Some(a) = x.angle.as_mut() {
                    *a = escala(a, fator);
                }
            }
            _ => {}
        }
    }
    d
}

/// Comandos de chapa que não abrem no meio do caminho (só mudam no 100%).
pub fn fixos(doc: &Document) -> Vec<Id> {
    doc.active_features()
        .map(|(_, f)| f)
        .filter(|f| matches!(&f.kind, FeatureKind::SmHem(_) | FeatureKind::SmProfileFlange(_) | FeatureKind::SmLoftFlange(_) | FeatureKind::SmConvert(_)) || matches!(&f.kind, FeatureKind::SmSurfFlange(x) if x.mode != 2))
        .map(|f| f.id)
        .collect()
}
