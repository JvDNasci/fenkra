// SPDX-License-Identifier: LGPL-2.1-or-later
//! B393: a forma final da peça tirada do cache do cálculo que acabou de rodar.
//!
//! `final_shape` refaz todos os comandos do zero. Quem já chamou `recompute_cached` (a montagem lê
//! cada peça para a malha e para a forma) calculava a peça duas vezes. O cache guarda, comando a
//! comando, o estado dos corpos depois dele; percorrendo os comandos ativos em ordem chega-se aos
//! mesmos corpos que `final_shape` produziria, sem repetir nenhum cálculo.

use crate::{Cache, Shape};
use fenkra_core::{Document, Id};
use std::collections::HashMap;

/// Forma final (corpos visíveis juntos) a partir do cache de um `recompute_cached(doc, cache)`
/// recém-feito. `None` se faltou algum comando no cache (cálculo cancelado ou interrompido): quem
/// chama volta a `final_shape`.
pub fn forma_do_cache(doc: &Document, cache: &Cache) -> Option<Shape> {
    let efetivo = doc.efetivo().ok()?;
    let doc: &Document = &efetivo;
    let corpos = corpos_do_cache(doc, cache)?;
    let mut all = Shape::empty();
    for c in doc.collectors.iter().filter(|c| c.visible) {
        if let Some(b) = corpos.get(&c.id) {
            all = if all.is_null() { b.clone() } else { all.compound(b) };
        }
    }
    // as faces do cache já foram malhadas, e a malha engorda a caixa envolvente; `final_shape` devolve
    // a forma sem malha, e quem usa a caixa (restrições, filtros de colisão) conta com ela exata
    if !all.is_null() {
        all.discard_mesh().ok();
    }
    (!all.is_null()).then_some(all)
}

/// O corpo de cada coletor ao fim dos comandos ativos, tirado do cache (sem refazer nenhum comando).
/// `doc` já deve ser o efetivo (configuração ativa). Os corpos ainda trazem a malha do cache.
pub fn corpos_do_cache(doc: &Document, cache: &Cache) -> Option<HashMap<Id, Shape>> {
    let mut corpos: HashMap<Id, Shape> = HashMap::new();
    for f in doc.active_features().map(|(_, f)| f) {
        let entrada = cache.feats.get(&f.id)?;
        for (id, b) in &entrada.bodies {
            match b {
                Some((s, _, _)) if !s.is_null() => {
                    corpos.insert(*id, s.clone());
                }
                _ => {
                    corpos.remove(id);
                }
            }
        }
    }
    Some(corpos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volume(s: &Shape) -> f64 {
        s.mass().volume
    }

    #[test]
    fn forma_do_cache_e_a_mesma_do_final_shape() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let doc = crate::example_document();
        let mut c = Cache::default();
        let r = crate::recompute_cached(&doc, &mut c);
        assert!(!r.bodies.is_empty());
        let do_cache = forma_do_cache(&doc, &c).expect("forma do cache");
        let refeita = crate::final_shape(&doc).expect("final_shape");
        let (a, b) = (do_cache.bbox().unwrap(), refeita.bbox().unwrap());
        // sem a malha do cache, a caixa é a mesma exata do `final_shape`
        for k in 0..3 {
            assert!((a.0[k] - b.0[k]).abs() < 1e-6 && (a.1[k] - b.1[k]).abs() < 1e-6, "caixa {a:?} {b:?}");
        }
        assert!((volume(&do_cache) - volume(&refeita)).abs() < 1e-6 * volume(&refeita).abs().max(1.), "{} {}", volume(&do_cache), volume(&refeita));
    }

    #[test]
    fn cache_vazio_nao_inventa_forma() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let doc = crate::example_document();
        assert!(forma_do_cache(&doc, &Cache::default()).is_none(), "sem cálculo prévio, o chamador usa final_shape");
    }
}
