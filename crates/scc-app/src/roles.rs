//! Entradas do comando em edição mostradas no 3D: cada elemento escolhido brilha e recebe um
//! rótulo com o papel dele ("Perfil", "Guia", "Eixo"…). Funciona para qualquer comando ou
//! elemento de geometria lendo os campos (nome do campo = papel).

use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use glam::Vec3;
use scc_core::*;
use scc_solidos::RecomputeResult;
use serde_json::Value;

/// Nome do papel pelo nome do campo.
fn role(key: &str) -> String {
    let k = match key {
        "sketch" | "profile" => "Perfil",
        "spine" => "Guia (spine)",
        "guide" | "guide2" => "Guia",
        "axis" => "Eixo",
        "dir" | "dir2" => "Direção",
        "base" => "Base",
        "plane" | "neutral" | "support" => "Plano",
        "curve" => "Curva",
        "point" | "near" => "Ponto",
        "center" | "origin" => "Centro",
        "surface" | "sa" | "sb" => "Superfície",
        "a" => "1º",
        "b" => "2º",
        "c" => "3º",
        "sections" => "Seção",
        "edge" | "edges" => "Aresta",
        "face" | "faces" => "Face",
        "tool" | "tools" => "Ferramenta",
        "sources" => "Origem",
        "paths" => "Linha guia",
        "points" => "Ponto",
        "curves" => "Curva",
        "items" => "Elemento",
        "at" => "Ponto",
        "source" => "Origem",
        other => return other.to_string(),
    };
    crate::i18n::tr(k).to_string()
}

fn num3(v: &Value) -> Option<[f64; 3]> {
    let a = v.as_array()?;
    if a.len() != 3 {
        return None;
    }
    Some([a[0].as_f64()?, a[1].as_f64()?, a[2].as_f64()?])
}

/// Posição 3D representativa de um id (referência, sketch).
fn pos_of_id(doc: &Document, r: &RecomputeResult, id: Id) -> Option<([f64; 3], Option<Id>)> {
    if let Some(p) = r.points.iter().find(|p| p.id == id) {
        return Some((p.p, Some(id)));
    }
    if let Some(c) = r.curves.iter().find(|c| c.id == id) {
        return c.pts.get(c.pts.len() / 2).map(|p| (*p, Some(id)));
    }
    if let Some(pl) = r.planes.iter().find(|p| p.id == Some(id)) {
        return Some((pl.frame.origin, Some(id)));
    }
    if let Some(s) = r.surfaces.iter().find(|s| s.id == id) {
        let m = &s.mesh;
        let n = m.positions.len() / 3;
        return (n > 0).then(|| {
            let k = (n / 2) * 3;
            ([m.positions[k] as f64, m.positions[k + 1] as f64, m.positions[k + 2] as f64], Some(id))
        });
    }
    if let Some(v) = r.sketches.iter().find(|v| v.feature == id) {
        let p = v.lines.get(v.lines.len() / 2).or(v.points.first()).map(|p| p.map(|x| x as f64)).unwrap_or(v.frame.origin);
        let _ = doc;
        return Some((p, None));
    }
    None
}

/// (posição, rótulo, referência a realçar) de cada entrada do valor (campos do comando).
fn walk(doc: &Document, r: &RecomputeResult, key: &str, v: &Value, out: &mut Vec<([f64; 3], String, Option<Id>)>, idx: Option<usize>) {
    let label = |k: &str| match idx {
        Some(i) => format!("{} {}", role(k), i + 1),
        None => role(k),
    };
    match v {
        // ids soltos só nos campos que guardam ids (sketch, seções, origens, ferramenta)
        Value::Number(n) if matches!(key, "sketch" | "sections" | "sources" | "tool" | "@id") => {
            if let Some(u) = n.as_u64() {
                let id = Id(u);
                if !crate::slots::is_unset(id) && (doc.reference(id).is_some() || doc.feature(id).is_some()) {
                    if let Some((p, hl)) = pos_of_id(doc, r, id) {
                        let k = if key == "@id" { "" } else { key };
                        out.push((p, if k.is_empty() { doc.name_of(id) } else { format!("{}: {}", label(k), doc.name_of(id)) }, hl));
                    }
                }
            }
        }
        Value::Array(a) => {
            if let Some(p) = num3(v) {
                // coordenadas guardadas (aresta/face clicada, ponto fixo)
                if !matches!(key, "x" | "normal" | "offset" | "anchors") && p != [0.; 3] {
                    out.push((p, label(key), None));
                }
                return;
            }
            // polilinha de aresta: o ponto do meio
            if a.len() >= 2 && a.iter().all(|x| num3(x).is_some()) && key == "Edge" {
                if let Some(p) = num3(&a[a.len() / 2]) {
                    out.push((p, crate::i18n::tr("Aresta").to_string(), None));
                }
                return;
            }
            for (i, x) in a.iter().enumerate() {
                walk(doc, r, key, x, out, if a.len() > 1 { Some(i) } else { idx });
            }
        }
        Value::Object(m) => {
            // variantes: {"Ref": id}, {"Datum": id}, {"Fixed": [..]}, {"Face": {...}}, {"Edge": [...]}
            if m.len() == 1 {
                let (k, x) = m.iter().next().unwrap();
                match k.as_str() {
                    "Fixed" => return walk(doc, r, key, x, out, idx),
                    "Ref" | "Datum" => {
                        let mut tmp = vec![];
                        walk(doc, r, "@id", x, &mut tmp, None);
                        for (p, n, h) in tmp {
                            out.push((p, format!("{}: {n}", label(key)), h));
                        }
                        return;
                    }
                    "Face" => {
                        if let Some(p) = x.get("point").and_then(num3) {
                            out.push((p, format!("{} ({})", label(key), crate::i18n::tr("face")), None));
                        }
                        return;
                    }
                    "Edge" => {
                        if let Some(a) = x.as_array() {
                            if let Some(p) = a.get(a.len() / 2).and_then(num3) {
                                out.push((p, format!("{} ({})", label(key), crate::i18n::tr("aresta")), None));
                            }
                        }
                        return;
                    }
                    _ => {}
                }
            }
            for (k, x) in m {
                // o campo dá o papel; dentro de variantes sem nome (ex.: Extent::UpTo) mantém o de fora
                let kk = if k.chars().next().is_some_and(|c| c.is_uppercase()) { key } else { k.as_str() };
                walk(doc, r, kk, x, out, idx);
            }
        }
        _ => {}
    }
}

/// Entradas do comando/elemento em edição.
pub fn inputs(doc: &Document, r: &RecomputeResult, id: Id) -> Vec<([f64; 3], String, Option<Id>)> {
    let v = if let Some(f) = doc.feature(id) {
        serde_json::to_value(&f.kind).ok()
    } else {
        doc.reference(id).and_then(|x| serde_json::to_value(&x.kind).ok())
    };
    let mut out = vec![];
    if let Some(v) = v {
        walk(doc, r, "", &v, &mut out, None);
    }
    // sem repetir o mesmo ponto com o mesmo rótulo
    out.dedup_by(|a, b| a.1 == b.1 && a.0 == b.0);
    out
}

/// Desenha os rótulos (e realça as referências) do comando em edição.
pub fn draw(painter: &egui::Painter, doc: &Document, r: &RecomputeResult, cam: &Camera, rect: Rect, id: Id) {
    let col = Color32::from_rgb(255, 196, 64);
    let mut placed: Vec<Rect> = vec![];
    for (p, label, hl) in inputs(doc, r, id) {
        if let Some(h) = hl {
            crate::picking::draw(painter, r, cam, rect, &crate::picking::Pick::Geo(h), true);
        }
        let Some(s) = cam.project(rect, Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)) else { continue };
        if !rect.contains(s) {
            continue;
        }
        painter.circle_filled(s, 4., col);
        let g = painter.layout_no_wrap(label, egui::FontId::proportional(12.), Color32::BLACK);
        // afasta o rótulo dos já colocados
        let mut at: Pos2 = s + egui::vec2(14., -22.);
        let mut box_r = Rect::from_min_size(at, g.size() + egui::vec2(10., 4.));
        while placed.iter().any(|q| q.intersects(box_r)) {
            at.y -= box_r.height() + 2.;
            box_r = Rect::from_min_size(at, g.size() + egui::vec2(10., 4.));
        }
        placed.push(box_r);
        painter.line_segment([s, box_r.left_bottom()], Stroke::new(1.2, col));
        painter.rect_filled(box_r, 3., col);
        painter.galley(box_r.min + egui::vec2(5., 2.), g, Color32::BLACK);
    }
}
