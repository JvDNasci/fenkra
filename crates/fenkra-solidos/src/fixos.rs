// SPDX-License-Identifier: LGPL-2.1-or-later
//! Ponto e eixo fixos que vieram de um clique na peça passam a seguir a geometria de origem (B15,
//! ADR 0010 fase 2).
//!
//! `PointRef::Fixed` e `AxisRef::Fixed` guardam só coordenadas: quando a face ou a aresta clicada
//! muda de lugar, o furo fica onde estava, no ar ou dentro do material. No primeiro cálculo que dá
//! certo, o ponto do furo que está sobre uma face com dono e papel vira `PointRef::OnFace`, e a
//! direção fixa que coincide com uma aresta reta (ou com a normal de uma face plana, no mesmo
//! sentido) vira `AxisRef::Edge` (ou `AxisRef::Face`). Coordenada que não está sobre a peça
//! continua fixa: a pessoa pode ter digitado de propósito.

use crate::refs::{self, View};
use fenkra_core::{AxisRef, FaceKey, Id, PointRef};
use fenkra_kernel::Shape;

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let l = dot(a, a).sqrt();
    (l > 1e-12).then(|| [a[0] / l, a[1] / l, a[2] / l])
}

/// Chave da face sobre a qual `p` está (a até 1e-3 mm), se ela tiver papel.
fn face_sob(b: &Shape, v: &View, p: [f64; 3]) -> Option<FaceKey> {
    let j = b.face_index(p, 1e-3).ok()??;
    refs::promote_face(v, j)
}

/// O ponto fixo sobre uma face com papel vira ponto na face.
pub fn ponto(b: &Shape, owners: &[Id], roles: &[i32], p: &PointRef) -> Option<PointRef> {
    let PointRef::Fixed(q) = p else { return None };
    let topo = b.topology().ok()?;
    let v = View { topo: &topo, owners, roles };
    face_sob(b, &v, *q).map(|key| PointRef::OnFace { point: *q, key })
}

/// O eixo fixo que coincide com uma aresta reta (mesmo sentido do início ao fim) ou com a normal de
/// uma face plana (mesmo sentido) vira referência a ela.
pub fn eixo(b: &Shape, owners: &[Id], roles: &[i32], a: &AxisRef) -> Option<AxisRef> {
    let AxisRef::Fixed { origin, dir } = a else { return None };
    let d = unit(*dir)?;
    let topo = b.topology().ok()?;
    let v = View { topo: &topo, owners, roles };
    // aresta reta que passa pela origem, na direção do eixo
    for e in b.edges_through(*origin, 1e-3).ok()? {
        let Some(m) = topo.edge_mids.get(e).copied() else { continue };
        let Ok(pts) = b.edge_at(m) else { continue };
        let (Some(p0), Some(p1)) = (pts.first(), pts.last()) else { continue };
        let Some(t) = unit([p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]]) else { continue };
        // reta: o meio está na corda; mesmo sentido do eixo guardado
        let meio = [m[0] - p0[0], m[1] - p0[1], m[2] - p0[2]];
        let fora = dot(meio, meio) - dot(meio, t).powi(2);
        if fora.max(0.).sqrt() > 1e-6 || dot(t, d) < 1. - 1e-9 {
            continue;
        }
        if let Some(key) = refs::promote_edge(&v, e) {
            return Some(AxisRef::Edge { mid: m, key: Some(key) });
        }
    }
    // face plana sob a origem com a normal no sentido do eixo
    let j = b.face_index(*origin, 1e-3).ok()??;
    let f = topo.faces.get(j)?;
    if f.kind == 0 && dot(f.dir, d) > 1. - 1e-9 {
        let key = refs::promote_face(&v, j)?;
        return Some(AxisRef::Face { point: *origin, key: Some(key) });
    }
    None
}
