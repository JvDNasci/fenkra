// SPDX-License-Identifier: LGPL-2.1-or-later
//! Chamada de feature na montagem (B186). Na peça, a face dá a feature que a gerou (`face_owner`).
//! Na montagem a vista tem um corpo por componente e o dono de toda face é o próprio componente;
//! a feature de verdade está no resultado da peça, cujas malhas a vista concatena na mesma ordem.
//! Daí a conta: face `f` do corpo do componente -> corpo e face dentro da peça -> feature dona.
//!
//! Só vale para componente que é peça, sem configuração da tabela de projeto e sem recorte de
//! recurso de montagem (a malha recortada não tem a numeração de faces da peça): nos outros casos
//! a função devolve `None` e a interface diz que a chamada não está disponível.

use crate::app::App;
use crate::asm_ui::AsmState;
use fenkra_core::pmi_feature::Callout;
use fenkra_core::Id;
use fenkra_montagem::NodeKind;
use std::path::PathBuf;

/// Caminho completo e peça carregada de um componente (sem configuração).
fn parte(st: &AsmState, comp: Id) -> Option<(&fenkra_montagem::load::LoadedPart, PathBuf)> {
    if !st.asm.config_de(comp).is_empty() {
        return None;
    }
    let node = st.asm.node(comp)?;
    let NodeKind::Part { path } = &node.kind else { return None };
    let full = fenkra_montagem::load::full_path(st.dir().as_deref(), path);
    let lp = st.cache.peek(&full)?;
    Some((lp, full))
}

/// Feature dona da face `face` do corpo `body` da vista da montagem: `(componente, feature)`.
pub fn dono_da_face(st: &AsmState, body: usize, face: usize) -> Option<(Id, Id)> {
    let comp = st.comp_de(body)?;
    let (lp, _) = parte(st, comp)?;
    let vista = st.view.as_ref()?.bodies.get(body)?;
    // a vista e a peça precisam concordar no total de faces: recorte de montagem muda a numeração
    let total: usize = lp.result.bodies.iter().map(|b| b.mesh.face_ranges.len()).sum();
    if total != vista.mesh.face_ranges.len() {
        return None;
    }
    let mut resto = face;
    for b in &lp.result.bodies {
        let n = b.mesh.face_ranges.len();
        if resto < n {
            let dono = *b.face_owner.get(resto)?;
            return Some((comp, dono));
        }
        resto -= n;
    }
    None
}

/// Chamada da feature `feature` do componente `comp`, com o mesmo gerador da peça e do desenho.
pub fn chamada(st: &AsmState, comp: Id, feature: Id, internal: bool) -> Option<Callout> {
    let (lp, _) = parte(st, comp)?;
    fenkra_core::pmi_feature::callout(&lp.doc, feature, internal)
}

/// Chamada da feature no nível aberto: da peça aberta, ou do componente quando é montagem.
pub fn chamada_de(app: &App, comp: Id, feature: Id, internal: bool) -> Option<Callout> {
    if let (true, Some(st)) = (app.in_asm(), app.asm.as_ref()) {
        chamada(st, comp, feature, internal)
    } else {
        fenkra_core::pmi_feature::callout(&app.open.doc, feature, internal)
    }
}

/// Garante que a peça do corpo escolhido esteja no cache da interface (a leitura de `dono_da_face`
/// só espia o cache). Chamado quando a seleção chega, com acesso mutável ao aplicativo.
pub fn aquecer(app: &mut App, body: usize) {
    let Some(st) = app.asm.as_mut() else { return };
    let Some(comp) = st.comp_de(body) else { return };
    if !st.asm.config_de(comp).is_empty() {
        return;
    }
    let Some(NodeKind::Part { path }) = st.asm.node(comp).map(|n| n.kind.clone()) else { return };
    let full = fenkra_montagem::load::full_path(st.dir().as_deref(), &path);
    let _ = st.cache.get(&full, false);
}

/// Componente (`Id(0)` na peça) e feature dona da face escolhida, no nível aberto.
pub fn dono_do_pick(app: &App, p: &crate::picking::Pick) -> Option<(Id, Id)> {
    let crate::picking::Pick::Face { body, face, owner, .. } = *p else { return None };
    if let (true, Some(st)) = (app.in_asm(), app.asm.as_ref()) {
        dono_da_face(st, body, face)
    } else {
        Some((Id(0), owner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_montagem::{Assembly, Placement};

    /// Duas cópias da placa de exemplo numa montagem, com a vista composta.
    fn montagem() -> AsmState {
        let dir = std::env::temp_dir().join(format!("fenkra_pmi_feat_asm_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
        let mut a = Assembly::new("Conjunto");
        a.add_component("Base", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
        a.add_component("Tampa", NodeKind::Part { path: "placa.fkp".into() }, Placement { origin: [200., 0., 0.], ..Default::default() }, None);
        let f = dir.join("conjunto.fka");
        std::fs::write(&f, a.to_json()).unwrap();
        let mut st = AsmState::new(a, Some(f));
        let mut erros = vec![];
        let dir = st.dir();
        let (doc, r) = fenkra_montagem::view::compose(&st.asm, dir.as_deref(), &mut st.cache, &mut erros);
        assert!(erros.is_empty(), "{erros:?}");
        st.view_doc = doc;
        st.view = Some(r);
        st
    }

    #[test]
    fn face_do_furo_na_montagem_acha_a_feature_e_gera_a_mesma_chamada_da_peca() {
        let st = montagem();
        let doc = fenkra_solidos::example_document();
        let esperado = doc.features.iter().find_map(|f| fenkra_core::pmi_feature::callout(&doc, f.id, true).filter(|c| c.hole_diameter.is_some()).map(|c| (f.id, c))).expect("a placa tem furo");
        // acha, em cada componente, uma face cuja dona é a feature do furo
        let r = st.view.as_ref().unwrap();
        for (body, b) in r.bodies.iter().enumerate() {
            let achou = (0..b.mesh.face_ranges.len()).find_map(|face| dono_da_face(&st, body, face).filter(|(_, feat)| *feat == esperado.0));
            let (comp, feat) = achou.unwrap_or_else(|| panic!("corpo {body}: nenhuma face do furo"));
            assert_eq!(comp, b.collector);
            let c = chamada(&st, comp, feat, true).expect("chamada do furo");
            assert_eq!(c.text, esperado.1.text, "montagem e peça falam a mesma chamada");
            assert!(c.text.contains('Ø'), "{}", c.text);
        }
    }
}
