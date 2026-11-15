// SPDX-License-Identifier: LGPL-2.1-or-later
//! B466: o que a lista ▾ de um campo de curva oferece além dos eixos da Origem e das curvas do
//! wireframe: as curvas dos sketches e as arestas dos corpos do modelo. Cada aresta vai como
//! `BodyEdge` (segue a aresta pela chave de dono e papel, ADR 0010) quando tem chave, senão como a
//! polilinha fixa, igual ao clique no 3D.

use crate::app::App;
use fenkra_core::*;
use fenkra_solidos::RecomputeResult;

/// Teto de arestas listadas: peças com milhares de arestas continuam pelo clique no 3D.
const LIMITE_DE_ARESTAS: usize = 600;

/// Curvas e arestas do modelo, com o rótulo da lista.
pub fn lista(app: &App) -> Vec<(String, CurveRef)> {
    lista_de(app.doc(), app.result.as_ref())
}

pub fn lista_de(d: &Document, r: Option<&RecomputeResult>) -> Vec<(String, CurveRef)> {
    let mut out = vec![];
    for f in &d.features {
        if let FeatureKind::Sketch(s) = &f.kind {
            for c in s.data.curves.iter().filter(|c| !c.construction) {
                out.push((format!("{} · {} #{}", f.name, crate::i18n::tr("curva"), c.id.0), CurveRef::SketchCurve { sketch: f.id, curve: c.id }));
            }
        }
    }
    let Some(r) = r else { return out };
    let mut n = 0;
    for (bi, b) in r.bodies.iter().enumerate() {
        let nome = d.name_of(b.collector);
        for e in 0..b.mesh.edge_ranges.len() {
            if n >= LIMITE_DE_ARESTAS {
                return out;
            }
            let pts = crate::picking::edge_polyline(r, bi, e);
            if pts.len() < 2 {
                continue;
            }
            let comp: f64 = pts.windows(2).map(|w| (0..3).map(|i| (w[1][i] - w[0][i]).powi(2)).sum::<f64>().sqrt()).sum();
            let rotulo = format!("{nome} · {} {} ({} mm)", crate::i18n::tr("aresta"), e + 1, crate::app::fmt(comp));
            let c = match b.edge_key(e) {
                Some(key) => CurveRef::BodyEdge { pts, key },
                None => CurveRef::Edge(pts),
            };
            out.push((rotulo, c));
            n += 1;
        }
    }
    out
}

#[cfg(test)]
#[path = "curvas_modelo_tests.rs"]
mod tests;
