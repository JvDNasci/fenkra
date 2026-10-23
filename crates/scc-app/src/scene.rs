//! Converte o resultado do recompute em uma cena para a viewport.
//!
//! Visibilidade: cada item tem oculto/visível; o **ambiente de ocultos** inverte a regra (mostra só o
//! que está oculto, para reexibir). O **filtro** de exibição vale nos dois ambientes.

use crate::app::Sel;
use crate::render::{LineVertex, MeshVertex, Scene};
use crate::settings::Filter;
use eframe::egui;
use glam::Vec3;
use scc_core::{Appearance, Document, Id, OriginPlane, PlaneRef};
use scc_solidos::RecomputeResult;
use std::collections::{HashMap, HashSet};

const SKETCH: [f32; 4] = [0.35, 0.75, 1.0, 1.0];
const CONSTR: [f32; 4] = [0.85, 0.6, 0.3, 0.8];
const HILITE: [f32; 4] = [1.0, 0.85, 0.2, 1.0];

pub struct SceneOpts<'a> {
    /// sketch em edição: suas linhas são desenhadas pelo editor, não pela cena
    pub editing: Option<Id>,
    pub sel: &'a Sel,
    pub filter: &'a Filter,
    pub hidden_env: bool,
    pub dark_edges: bool,
    /// item realçado: o que está sob o mouse na árvore, ou o comando aberto
    pub hilite: Option<Id>,
    /// estilo padrão das curvas de referência (px e tipo de linha)
    pub line_w: f32,
    pub line_dash: u8,
    /// símbolo dos pontos: 0 = cruz 3D na cena; outros são desenhados na tela pela viewport
    pub point_style: u8,
}

fn lv(p: [f32; 3], col: [f32; 4]) -> LineVertex {
    LineVertex { pos: p, col }
}

fn grow(b: &mut Option<(Vec3, Vec3)>, p: Vec3) {
    *b = Some(match *b {
        None => (p, p),
        Some((lo, hi)) => (lo.min(p), hi.max(p)),
    });
}

/// Regra de exibição: no ambiente normal mostra os visíveis; no de ocultos, só os ocultos.
fn shows(hidden: bool, env: bool) -> bool {
    hidden == env
}

fn rgb(c: [u8; 3]) -> [f32; 3] {
    c.map(|x| x as f32 / 255.)
}

pub fn build(doc: &Document, r: &RecomputeResult, o: &SceneOpts) -> Scene {
    let mut s = Scene::default();
    let mut bounds = None;
    let env = o.hidden_env;

    // realce dinâmico: só o item sob o mouse (ou o comando em edição); a seleção sozinha não brilha
    let _ = o.sel;
    let mut hi_features: HashSet<Id> = HashSet::new();
    if let Some(id) = o.hilite {
        if let Some(f) = doc.feature(id) {
            hi_features.insert(id);
            hi_features.extend(Document::refs_of(&f.kind));
        }
    }
    let hi = |id: Id| o.hilite == Some(id);

    let feat_app: HashMap<Id, Appearance> = doc.features.iter().filter_map(|f| f.appearance.map(|a| (f.id, a))).collect();
    // só sketches se ocultam um a um; sólidos aparecem pelo corpo (a última feature junta tudo)
    let feat_hidden: HashSet<Id> = doc.features.iter().filter(|f| f.hidden && matches!(f.kind, scc_core::FeatureKind::Sketch(_))).map(|f| f.id).collect();
    let edge_col = if o.dark_edges { [0.06, 0.07, 0.09, 1.0] } else { [0.85, 0.88, 0.92, 1.0] };

    if o.filter.corpos {
        for b in &r.bodies {
            let Some(col) = doc.collectors.iter().find(|c| c.id == b.collector) else { continue };
            let body_sel = hi(b.collector);
            let base = doc.appearance.unwrap_or(Appearance { color: col.color, finish: col.finish, texture: col.texture });
            let m = &b.mesh;
            let nfaces = m.face_ranges.len();
            for fi in 0..nfaces {
                let owner = b.face_owner.get(fi).copied();
                let face_hidden = !col.visible || owner.is_some_and(|w| feat_hidden.contains(&w));
                if !shows(face_hidden, env) {
                    continue;
                }
                let app = owner.and_then(|w| feat_app.get(&w).copied()).unwrap_or(base);
                let (rough, metal, alpha) = app.finish.params();
                let mut c = rgb(app.color);
                if body_sel || owner.is_some_and(|w| hi_features.contains(&w)) {
                    c = [c[0] * 0.55 + 0.45, c[1] * 0.55 + 0.38, c[2] * 0.55 + 0.08];
                }
                let start = m.face_ranges[fi] as usize;
                let end = m.face_ranges.get(fi + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
                // duplica os vértices da face para poder colorir por face
                let mut remap: HashMap<u32, u32> = HashMap::new();
                for &vi in &m.indices[start..end] {
                    let ni = *remap.entry(vi).or_insert_with(|| {
                        let k = vi as usize * 3;
                        let p = [m.positions[k], m.positions[k + 1], m.positions[k + 2]];
                        grow(&mut bounds, Vec3::from(p));
                        s.mesh.push(MeshVertex {
                            pos: p,
                            nrm: [m.normals[k], m.normals[k + 1], m.normals[k + 2]],
                            col: [c[0], c[1], c[2], 1.0],
                            mat: [rough, metal, alpha, app.texture.filter(|t| (*t as u32) < crate::render::TEX_LAYERS).map(|t| t as f32 + 1.).unwrap_or(0.)],
                        });
                        (s.mesh.len() - 1) as u32
                    });
                    s.indices.push(ni);
                }
            }
            if shows(!col.visible, env) {
                for p in m.edges.chunks_exact(3) {
                    s.edges.push(lv([p[0], p[1], p[2]], edge_col));
                }
            }
        }
    }

    // sketches: mostrados conforme oculto/visível; o sketch do comando selecionado sempre aparece em destaque
    if o.filter.sketches {
        for sk in &r.sketches {
            let hidden = feat_hidden.contains(&sk.feature);
            let editing = o.editing == Some(sk.feature);
            let h = hi_features.contains(&sk.feature) && !editing;
            if !editing && !h && !shows(hidden, env) {
                continue;
            }
            // sombra dos perfis fechados
            let n = Vec3::from(sk.frame.normal.map(|v| v as f32));
            let fc = if editing { [0.3, 0.65, 1.0, 1.0] } else { [0.35, 0.7, 1.0, 1.0] };
            for p in &sk.fill {
                s.fill.push(MeshVertex { pos: *p, nrm: n.into(), col: fc, mat: [1.0, 0.0, if editing { 0.22 } else { 0.14 }, 0.] });
            }
            if editing {
                continue;
            }
            let dst = if h { &mut s.overlay } else { &mut s.lines };
            for p in &sk.lines {
                dst.push(lv(*p, if h { HILITE } else { SKETCH }));
                grow(&mut bounds, Vec3::from(*p));
            }
            for p in &sk.construction {
                dst.push(lv(*p, CONSTR));
            }
        }
    }

    // tamanho de referência para planos, pontos e eixos
    let size = bounds.map(|(lo, hi): (Vec3, Vec3)| (hi - lo).max_element().max(20.) * 0.6).unwrap_or(60.);

    if o.filter.sketches && o.filter.pontos {
        for sk in &r.sketches {
            let h = hi_features.contains(&sk.feature);
            if !h && !shows(feat_hidden.contains(&sk.feature), env) {
                continue;
            }
            let d = size * 0.025;
            if o.point_style != 0 {
                continue;
            }
            for p in &sk.points {
                let c = Vec3::from(*p);
                for ax in [Vec3::X, Vec3::Y, Vec3::Z] {
                    s.overlay.push(lv((c - ax * d).into(), if h { HILITE } else { SKETCH }));
                    s.overlay.push(lv((c + ax * d).into(), if h { HILITE } else { SKETCH }));
                }
            }
        }
    }

    // planos da Origem e das Referências
    for p in &r.planes {
        let (visible_flag, allowed) = match p.id {
            Some(id) => (doc.reference(id).is_some_and(|x| x.visible), o.filter.planos_ref),
            None => (doc.origin_visible, o.filter.planos_origem),
        };
        let selected = match (o.sel, p.id) {
            (_, Some(b)) => hi(b),
            (Sel::Origin(op), None) => p.name == plane_name(*op),
            _ => false,
        };
        if !selected && (!allowed || !shows(!visible_flag, env)) {
            continue;
        }
        let col = match (p.id, p.name.as_str()) {
            (Some(_), _) => [0.95, 0.7, 0.3, 0.6],
            (_, "XY") => [0.4, 0.55, 1.0, 0.5],
            (_, "XZ") => [0.4, 0.85, 0.45, 0.5],
            _ => [1.0, 0.45, 0.4, 0.5],
        };
        let f = &p.frame;
        let (o3, x, yv) = (
            Vec3::from(f.origin.map(|v| v as f32)),
            Vec3::from(f.x.map(|v| v as f32)),
            Vec3::from(f.y().map(|v| v as f32)),
        );
        let h = size * if p.id.is_some() { 0.45 } else { 0.5 };
        let c = [o3 + (x + yv) * h, o3 + (x - yv) * h, o3 + (-x - yv) * h, o3 + (-x + yv) * h];
        let dst = if selected { &mut s.overlay } else { &mut s.lines };
        for i in 0..4 {
            let colr = if selected { HILITE } else { col };
            dst.push(lv(c[i].into(), colr));
            dst.push(lv(c[(i + 1) % 4].into(), colr));
        }
    }

    // superfícies: objetos opacos (como os corpos), com cor própria e contorno
    if o.filter.planos_ref {
        for sf in &r.surfaces {
            let selected = hi(sf.id);
            let rf = doc.reference(sf.id);
            let vis = rf.is_some_and(|x| x.visible);
            if !selected && !shows(!vis, env) {
                continue;
            }
            let app = rf.and_then(|x| x.appearance).unwrap_or(Appearance { color: [222, 170, 110], finish: scc_core::Finish::Acetinado, texture: None });
            let (rough, metal, alpha) = app.finish.params();
            let mut c = rgb(app.color);
            if selected {
                c = [c[0] * 0.55 + 0.45, c[1] * 0.55 + 0.38, c[2] * 0.55 + 0.08];
            }
            let m = &sf.mesh;
            let base = s.mesh.len() as u32;
            for k in (0..m.positions.len()).step_by(3) {
                let p = [m.positions[k], m.positions[k + 1], m.positions[k + 2]];
                grow(&mut bounds, Vec3::from(p));
                s.mesh.push(MeshVertex { pos: p, nrm: [m.normals[k], m.normals[k + 1], m.normals[k + 2]], col: [c[0], c[1], c[2], 1.0], mat: [rough, metal, alpha, 0.] });
            }
            s.indices.extend(m.indices.iter().map(|i| base + i));
            for p in m.edges.chunks_exact(3) {
                s.edges.push(lv([p[0], p[1], p[2]], if selected { HILITE } else { edge_col }));
            }
        }
    }

    // pontos, linhas e curvas de referência
    let geo_col = [0.95, 0.7, 0.3, 1.0];
    for c in &r.curves {
        let selected = hi(c.id);
        let vis = doc.reference(c.id).is_some_and(|x| x.visible);
        if !selected && (!o.filter.planos_ref || !shows(!vis, env)) {
            continue;
        }
        // espessura e tipo de linha: o do elemento, senão o padrão das configurações
        let st = doc.line_style(c.id);
        let (w, dash) = st.map(|x| (x.width, x.dash)).unwrap_or((o.line_w, o.line_dash));
        let col = match (selected, doc.reference(c.id).and_then(|x| x.appearance)) {
            (true, _) => HILITE,
            (false, Some(a)) => {
                let k = rgb(a.color);
                [k[0], k[1], k[2], 1.0]
            }
            _ => geo_col,
        };
        let pts: Vec<[f32; 3]> = c.pts.iter().map(|p| p.map(|v| v as f32)).collect();
        crate::render::push_wide(&mut s.wide, &pts, col, if selected { w + 1.5 } else { w }, dash);
        if selected {
            for w2 in c.pts.windows(2) {
                s.overlay.push(lv(w2[0].map(|v| v as f32), HILITE));
                s.overlay.push(lv(w2[1].map(|v| v as f32), HILITE));
            }
        }
    }
    if o.filter.pontos && o.point_style == 0 {
        for p in &r.points {
            let selected = hi(p.id);
            let vis = doc.reference(p.id).is_some_and(|x| x.visible);
            if !selected && !shows(!vis, env) {
                continue;
            }
            let c = Vec3::from(p.p.map(|v| v as f32));
            let d = size * 0.03;
            for ax in [Vec3::X, Vec3::Y, Vec3::Z] {
                let col = if selected { HILITE } else { geo_col };
                s.overlay.push(lv((c - ax * d).into(), col));
                s.overlay.push(lv((c + ax * d).into(), col));
            }
        }
    }

    // eixos da Origem
    if o.filter.eixos && shows(!doc.origin_visible, env) {
        for (ax, col) in [(Vec3::X, [1., 0.3, 0.3, 1.]), (Vec3::Y, [0.3, 0.9, 0.3, 1.]), (Vec3::Z, [0.35, 0.55, 1., 1.])] {
            s.overlay.push(lv([0.; 3], col));
            s.overlay.push(lv((ax * size * 0.25).into(), col));
        }
    }

    s.bounds = bounds.or(Some((Vec3::splat(-size), Vec3::splat(size))));
    s
}

/// O comando é um sketch?
pub fn is_sketch_feature(doc: &Document, id: Id) -> bool {
    doc.feature(id).is_some_and(|f| matches!(f.kind, scc_core::FeatureKind::Sketch(_)))
}

pub fn plane_name(o: OriginPlane) -> &'static str {
    match o {
        OriginPlane::XY => "XY",
        OriginPlane::XZ => "XZ",
        OriginPlane::YZ => "YZ",
    }
}

pub fn plane_ref_name(doc: &Document, p: &PlaneRef) -> String {
    match p {
        PlaneRef::Origin(o) => format!("Origem · {}", plane_name(*o)),
        PlaneRef::Datum(id) => doc.reference(*id).map(|r| r.name.clone()).or_else(|| crate::slots::label(*id)).unwrap_or_else(|| format!("⚠ perdida #{}", id.0)),
    }
}

/// Símbolos dos pontos (referência e centros de sketch) em tamanho fixo na tela, no estilo
/// escolhido (asterisco, X, círculo, quadrado, ponto) — o estilo 0 (cruz 3D) fica na cena.
#[allow(clippy::too_many_arguments)]
pub fn draw_points(painter: &egui::Painter, doc: &Document, r: &RecomputeResult, cam: &crate::render::Camera, rect: egui::Rect, style: u8, size: f32, filter: &Filter, hidden_env: bool, editing: Option<Id>, hilite: Option<Id>, sel: &Sel) {
    if style == 0 || !filter.pontos {
        return;
    }
    let geo = egui::Color32::from_rgb(242, 178, 76);
    let sk_col = egui::Color32::from_rgb(90, 170, 255);
    let hl = egui::Color32::from_rgb(255, 205, 60);
    let draw = |p: [f32; 3], col: egui::Color32, st: u8, sz: f32| {
        let Some(c) = cam.project(rect, Vec3::from(p)) else { return };
        if !rect.contains(c) {
            return;
        }
        let h = sz / 2.;
        let stroke = egui::Stroke::new(1.6, col);
        match st {
            1 => {
                for k in 0..3 {
                    let a = k as f32 * std::f32::consts::PI / 3. + std::f32::consts::FRAC_PI_2;
                    let d = egui::vec2(a.cos(), -a.sin()) * h;
                    painter.line_segment([c - d, c + d], stroke);
                }
            }
            2 => {
                painter.line_segment([c + egui::vec2(-h, -h), c + egui::vec2(h, h)], stroke);
                painter.line_segment([c + egui::vec2(-h, h), c + egui::vec2(h, -h)], stroke);
            }
            3 => {
                painter.circle_stroke(c, h, stroke);
                painter.circle_filled(c, 1.2, col);
            }
            4 => {
                painter.rect_stroke(egui::Rect::from_center_size(c, egui::vec2(sz, sz)), 0., stroke, egui::StrokeKind::Middle);
            }
            _ => {
                painter.circle_filled(c, h * 0.5, col);
            }
        }
    };
    for p in &r.points {
        let rf = doc.reference(p.id);
        let vis = rf.is_some_and(|x| x.visible);
        let selected = hilite == Some(p.id) || matches!(sel, Sel::Ref(x) if *x == p.id);
        if !selected && !shows(!vis, hidden_env) {
            continue;
        }
        let st = doc.line_style(p.id);
        let col = match (selected, rf.and_then(|x| x.appearance)) {
            (true, _) => hl,
            (false, Some(a)) => egui::Color32::from_rgb(a.color[0], a.color[1], a.color[2]),
            _ => geo,
        };
        draw(p.p.map(|v| v as f32), col, st.map(|x| x.point).filter(|x| *x != 0).unwrap_or(style), st.map(|x| x.size).unwrap_or(size));
    }
    if filter.sketches {
        for sk in &r.sketches {
            if Some(sk.feature) == editing {
                continue;
            }
            let hidden = doc.feature(sk.feature).is_some_and(|f| f.hidden);
            if !shows(hidden, hidden_env) {
                continue;
            }
            let col = if hilite == Some(sk.feature) { hl } else { sk_col };
            for p in &sk.points {
                draw(*p, col, style, size);
            }
        }
    }
}
