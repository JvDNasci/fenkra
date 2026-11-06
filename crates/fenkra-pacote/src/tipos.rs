// SPDX-License-Identifier: LGPL-2.1-or-later
//! Conversões pequenas do modelo para o JSON do manifesto.

use fenkra_core::doc::NamedView;
use fenkra_core::pmi::{PmiTarget, TargetClass};
use fenkra_montagem::display::DisplayState;
use serde_json::{json, Value};

fn r6(x: f64) -> f64 {
    let v = (x * 1e6).round() / 1e6;
    if v == 0. {
        0.
    } else {
        v
    }
}

pub fn camera(v: &NamedView) -> Value {
    json!({"alvo": v.target.map(r6), "yaw": r6(v.yaw), "pitch": r6(v.pitch), "distancia": r6(v.dist), "ortografica": v.ortho})
}

pub fn alvo(t: &PmiTarget) -> Value {
    let classe = match t.class {
        TargetClass::FacePlana => "face_plana",
        TargetClass::FaceCilindrica => "face_cilindrica",
        TargetClass::FaceOutra => "face",
        TargetClass::ArestaCircular => "aresta_circular",
        TargetClass::Aresta => "aresta",
        TargetClass::Vertice => "vertice",
    };
    json!({"classe": classe, "ponto": t.point.map(r6), "direcao": t.dir.map(r6), "raio": r6(t.radius), "furo": t.concave})
}

/// Estado de exibição. Os IDs são os da árvore do pacote (`n<id>`); a explosão vai como deslocamento
/// em milímetros, no mundo.
pub fn estado(s: &DisplayState) -> Value {
    json!({
        "nome": s.name,
        "ocultos": s.hidden.iter().map(|i| format!("n{}", i.0)).collect::<Vec<_>>(),
        "cores": s.colors.iter().map(|(i, c)| json!({"id": format!("n{}", i.0), "cor": c})).collect::<Vec<_>>(),
        "deslocamentos": s.offsets.iter().map(|(i, d)| json!({"id": format!("n{}", i.0), "d": d.map(r6)})).collect::<Vec<_>>(),
        "camera": s.camera.as_ref().map(|c| json!({"alvo": c.target.map(r6), "yaw": r6(c.yaw), "pitch": r6(c.pitch), "distancia": r6(c.dist), "ortografica": c.ortho})),
    })
}
