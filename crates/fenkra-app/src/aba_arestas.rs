// SPDX-License-Identifier: LGPL-2.1-or-later
//! B560: o clique numa aresta no modo único "Arestas" da aba. Monta, a partir da malha do corpo, o
//! que `slots::aresta_clicada_lista` precisa: a polilinha da aresta, a cadeia tangente (com as
//! arestas que a formam, meio e chave de cada uma) e a cadeia da aba curva que já estava aberta.

use crate::picking;
use crate::slots::CliqueAresta;
use fenkra_core::{EdgeKey, FeatureKind};
use fenkra_solidos::RecomputeResult;

fn meio(m: &fenkra_kernel::Mesh, e: usize) -> [f64; 3] {
    [m.edge_mids[e * 3] as f64, m.edge_mids[e * 3 + 1] as f64, m.edge_mids[e * 3 + 2] as f64]
}

/// O clique na aresta `edge` do corpo `body`, com o recurso `k` (a aba aberta) para achar a cadeia
/// que ele já guarda.
pub fn clique(r: &RecomputeResult, body: usize, edge: usize, mid: [f64; 3], k: &FeatureKind) -> CliqueAresta {
    let Some(b) = r.bodies.get(body) else { return CliqueAresta { mid, reta: true, ..Default::default() } };
    let m = &b.mesh;
    let so_pts = picking::edge_polyline_mesh(m, edge);
    let reta = crate::slots::polilinha_reta(&so_pts);
    let (cadeia_pts, ids) = picking::tangent_edges_mesh(m, edge);
    let com_chave = |ids: &[usize]| -> Vec<([f64; 3], Option<EdgeKey>)> { ids.iter().filter(|e| **e < m.edge_ranges.len()).map(|e| (meio(m, *e), b.edge_key(*e))).collect() };
    let mut cadeia = com_chave(&ids);
    if cadeia.is_empty() {
        cadeia = vec![(mid, b.edge_key(edge))];
    }
    // a aba curva aberta guarda só uma chave: a aresta dela dá a cadeia pelas tangentes
    let existente = match k {
        FeatureKind::SmCurveFlange(c) if !c.edge.is_empty() => {
            let semente = c.edge_key.and_then(|key| (0..m.edge_ranges.len()).find(|e| b.edge_key(*e).as_ref() == Some(&key)));
            match semente {
                Some(e) if c.so_aresta => com_chave(&[e]),
                Some(e) => com_chave(&picking::tangent_edges_mesh(m, e).1),
                None => vec![],
            }
        }
        _ => vec![],
    };
    CliqueAresta { mid, key: b.edge_key(edge), reta, so_pts, cadeia_pts, cadeia, existente }
}
