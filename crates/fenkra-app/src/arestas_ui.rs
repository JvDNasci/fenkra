// SPDX-License-Identifier: LGPL-2.1-or-later
//! Apoio de interface do filete e do chanfro (B289): memória do último valor usado.

use fenkra_core::{Document, FeatureKind};

/// Último valor de raio (filete) ou distância (chanfro) usado no documento: o comando novo começa
/// por ele. Sem comando anterior, 1.
pub fn ultimo_valor_aresta(doc: &Document, fillet: bool) -> String {
    doc.features
        .iter()
        .rev()
        .find_map(|f| match &f.kind {
            FeatureKind::Fillet(o) if fillet => Some(o.value.clone()),
            FeatureKind::Chamfer(o) if !fillet => Some(o.value.clone()),
            _ => None,
        })
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "1".to_string())
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_core::{EdgeMode, EdgeOp, EdgeSel};

    fn op(v: &str) -> EdgeOp {
        EdgeOp { value: v.into(), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }
    }

    #[test]
    fn comando_novo_comeca_pelo_ultimo_valor_de_cada_tipo() {
        let mut d = Document::new("m");
        assert_eq!(ultimo_valor_aresta(&d, true), "1");
        d.add_feature(FeatureKind::Fillet(op("4")));
        d.add_feature(FeatureKind::Chamfer(op("2.5")));
        d.add_feature(FeatureKind::Fillet(op("raio")));
        assert_eq!(ultimo_valor_aresta(&d, true), "raio");
        assert_eq!(ultimo_valor_aresta(&d, false), "2.5");
    }
}
