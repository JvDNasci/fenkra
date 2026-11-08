// SPDX-License-Identifier: LGPL-2.1-or-later
//! B407: rótulos de geometria ligada (link de outra peça) e o contador de uso na árvore.
//!
//! A geometria ligada leva o nome `origem_publicação` (`mestre_LONG_E_0`), que é a variável
//! estável das fórmulas, mas é longo para o 3D: com várias curvas ligadas, os rótulos se cobrem e a
//! vista fica suja. No 3D ela só ganha rótulo quando está selecionada, e com o nome curto da
//! publicação; o nome completo segue na árvore, no painel e nas fórmulas.

use fenkra_core::{Document, RefElement, RefKind};

/// Texto do rótulo 3D de um elemento de referência; `None` quando ele não leva rótulo.
pub fn rotulo_3d(doc: &Document, rf: &RefElement, selecionado: bool) -> Option<String> {
    let RefKind::Linked { ext, .. } = rf.kind else { return Some(rf.name.clone()) };
    if !selecionado {
        return None;
    }
    let curto = doc.external.iter().find(|e| e.id == ext).map(|e| e.publication.clone()).filter(|p| !p.is_empty());
    Some(format!("🔗 {}", curto.unwrap_or_else(|| rf.name.clone())))
}

/// Explicação do `·N` da árvore e, na geometria ligada, de onde ela vem.
pub fn dica_da_linha(doc: &Document, rf: &RefElement, usos: usize) -> Option<String> {
    let mut partes = vec![];
    if let RefKind::Linked { ext, .. } = rf.kind {
        if let Some(e) = doc.external.iter().find(|e| e.id == ext) {
            partes.push(format!("{} {}.{}", crate::i18n::tr("Ligado à publicação"), e.source, e.publication));
        }
    }
    if usos > 0 {
        partes.push(format!("·{usos} = {} {usos} {}", crate::i18n::tr("usado por"), crate::i18n::tr("elemento(s) ou comando(s)")));
    }
    (!partes.is_empty()).then(|| partes.join("\n"))
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_core::LinkedGeo;

    #[test]
    fn curva_ligada_so_tem_rotulo_curto_quando_selecionada() {
        let mut d = Document::new("peca");
        let id = d.add_link("mestre", "mestre.fkp", "LONG_E_0", LinkedGeo::Curve(vec![[0.; 3], [10., 0., 0.]]));
        let rf = d.reference(id).unwrap().clone();
        assert_eq!(rf.name, "mestre_LONG_E_0");
        assert_eq!(rotulo_3d(&d, &rf, false), None);
        assert_eq!(rotulo_3d(&d, &rf, true).as_deref(), Some("🔗 LONG_E_0"));
        let dica = dica_da_linha(&d, &rf, 1).unwrap();
        assert!(dica.contains("mestre.LONG_E_0") && dica.contains("·1"), "{dica}");
    }

    #[test]
    fn geometria_propria_mantem_o_rotulo_e_sem_uso_nao_tem_dica() {
        let mut d = Document::new("peca");
        let id = d.add_geometry(RefKind::PointCoords { x: "0".into(), y: "0".into(), z: "0".into(), frame: None });
        let rf = d.reference(id).unwrap().clone();
        assert_eq!(rotulo_3d(&d, &rf, false), Some(rf.name.clone()));
        assert_eq!(dica_da_linha(&d, &rf, 0), None);
    }
}
