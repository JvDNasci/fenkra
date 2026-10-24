//! Manipulador 3D (gizmo) do elemento em edição: setas lineares por eixo, quadrados planares
//! (arrastam duas coordenadas), arco de rotação e alça de escala. Serve aos pontos por
//! coordenadas e às transformações de geometria (mover, girar, escalar).
//! Arrastar muda o valor em passos de 1 mm / 1° (Shift: 0,1 mm / 5°); valores ligados a
//! parâmetros ficam travados (🔒) para não quebrar o vínculo.

use crate::app::{fmt, App};
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use glam::Vec3;
use scc_core::*;

const AXIS_COL: [Color32; 3] = [Color32::from_rgb(235, 80, 80), Color32::from_rgb(90, 200, 90), Color32::from_rgb(80, 140, 255)];
const HOT: Color32 = Color32::from_rgb(255, 190, 40);
/// comprimento das setas na tela (px)
const ARM: f32 = 70.;

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

fn num(e: &str) -> Option<f64> {
    e.trim().parse::<f64>().ok()
}

/// Centro do elemento calculado (ponto, curva, plano ou superfície).
fn center_of(r: &scc_solidos::RecomputeResult, id: Id) -> Option<Vec3> {
    if let Some(p) = r.points.iter().find(|p| p.id == id) {
        return Some(v3(p.p));
    }
    if let Some(c) = r.curves.iter().find(|c| c.id == id) {
        return Some(c.pts.iter().fold(Vec3::ZERO, |a, p| a + v3(*p)) / c.pts.len().max(1) as f32);
    }
    if let Some(p) = r.planes.iter().find(|p| p.id == Some(id)) {
        return Some(v3(p.frame.origin));
    }
    let s = r.surfaces.iter().find(|s| s.id == id)?;
    let n = (s.mesh.positions.len() / 3).max(1) as f32;
    Some(s.mesh.positions.chunks_exact(3).fold(Vec3::ZERO, |a, p| a + Vec3::new(p[0], p[1], p[2])) / n)
}

/// Eixo (origem, direção unitária) já calculado.
fn axis_of(app: &App, a: &AxisRef) -> Option<(Vec3, Vec3)> {
    let r = app.result.as_ref()?;
    match a {
        AxisRef::Origin(x) => Some((Vec3::ZERO, v3(scc_solidos::origin_axis(*x)))),
        AxisRef::Fixed { origin, dir } => Some((v3(*origin), v3(*dir).try_normalize()?)),
        AxisRef::Ref(id) => {
            let c = r.curves.iter().find(|c| c.id == *id)?;
            let (p, q) = (v3(c.pts[0]), v3(*c.pts.last()?));
            Some((p, (q - p).try_normalize()?))
        }
        AxisRef::SketchLine { .. } => None,
    }
}

fn point_of(app: &App, p: &PointRef) -> Option<Vec3> {
    crate::picking::point_pos(app.result.as_ref()?, p).map(v3)
}

/// Estado do arrasto guardado entre quadros: posição inicial do mouse e valores iniciais.
#[derive(Clone)]
struct Drag {
    start: Pos2,
    values: Vec<f64>,
}

fn drag_state(ui: &egui::Ui, hid: egui::Id, r: &egui::Response, values: &[f64]) -> Option<(Drag, Pos2)> {
    let key = hid.with("estado");
    if r.drag_started() {
        let start = r.interact_pointer_pos()?;
        ui.ctx().data_mut(|m| m.insert_temp(key, Drag { start, values: values.to_vec() }));
    }
    if !r.dragged() {
        return None;
    }
    let st: Drag = ui.ctx().data(|m| m.get_temp(key))?;
    Some((st, ui.input(|i| i.pointer.interact_pos())?))
}

fn step(ui: &egui::Ui, coarse: f64, fine: f64) -> f64 {
    if ui.input(|i| i.modifiers.shift) {
        fine
    } else {
        coarse
    }
}

fn snap(v: f64, s: f64) -> f64 {
    (v / s).round() * s
}

fn arrow(p: &egui::Painter, a: Pos2, b: Pos2, c: Color32) {
    p.line_segment([a, b], Stroke::new(2.5, c));
    let d = if (b - a).length() > 1. { (b - a).normalized() } else { Vec2::X };
    let n = egui::vec2(-d.y, d.x);
    p.add(egui::Shape::convex_polygon(vec![b + d * 12., b + n * 6., b - n * 6.], c, Stroke::new(1., Color32::from_black_alpha(120))));
}

fn label(p: &egui::Painter, at: Pos2, txt: String, c: Color32) {
    let g = p.layout_no_wrap(txt, egui::FontId::proportional(12.5), Color32::WHITE);
    let r = Rect::from_min_size(at, g.size() + egui::vec2(10., 6.));
    p.rect_filled(r, 4., Color32::from_rgba_unmultiplied(20, 40, 70, 225));
    p.rect_stroke(r, 4., Stroke::new(1., c), egui::StrokeKind::Inside);
    p.galley(r.min + egui::vec2(5., 3.), g, Color32::WHITE);
}

/// Pixels por mm ao longo de `dir` a partir de `o`, e a direção na tela.
fn screen_axis(cam: &Camera, rect: Rect, o: Vec3, dir: Vec3) -> Option<(Pos2, Vec2, f32)> {
    let a = cam.project(rect, o)?;
    let b = cam.project(rect, o + dir)?;
    let ppm = (b - a).length();
    (ppm > 1e-4).then(|| (a, (b - a) / ppm, ppm))
}

/// Gizmo completo de translação: 3 setas e 3 quadrados planares. `vals` = x, y, z editáveis.
/// Devolve os novos valores quando arrastado.
pub(crate) fn translate_gizmo(ui: &mut egui::Ui, rect: Rect, cam: &Camera, salt: u64, o: Vec3, axes: [Vec3; 3], vals: [f64; 3], locked: [bool; 3]) -> Option<[f64; 3]> {
    let painter = ui.painter_at(rect);
    let sa: Vec<(Pos2, Vec2, f32)> = axes.iter().map(|d| screen_axis(cam, rect, o, *d)).collect::<Option<_>>()?;
    let c0 = sa[0].0;
    let mut out = None;
    // quadrados planares (no meio, entre dois eixos)
    for (k, (i, j)) in [(0usize, 1usize), (1, 2), (0, 2)].into_iter().enumerate() {
        if locked[i] || locked[j] {
            continue;
        }
        let q = c0 + sa[i].1 * ARM * 0.3 + sa[j].1 * ARM * 0.3;
        let hid = egui::Id::unique(("gizmo_plano", salt, k));
        let r = ui.interact(Rect::from_center_size(q, egui::vec2(16., 16.)), hid, egui::Sense::drag());
        let hot = r.hovered() || r.dragged();
        let col = if hot { HOT } else { AXIS_COL[3 - i - j].gamma_multiply(0.85) };
        let quad = [c0 + sa[i].1 * ARM * 0.2 + sa[j].1 * ARM * 0.2, c0 + sa[i].1 * ARM * 0.4 + sa[j].1 * ARM * 0.2, c0 + sa[i].1 * ARM * 0.4 + sa[j].1 * ARM * 0.4, c0 + sa[i].1 * ARM * 0.2 + sa[j].1 * ARM * 0.4];
        painter.add(egui::Shape::convex_polygon(quad.to_vec(), col.gamma_multiply(0.45), Stroke::new(1.2, col)));
        if let Some((st, m)) = drag_state(ui, hid, &r, &vals) {
            // decompõe o deslocamento da tela nas duas direções projetadas (mm)
            let d = m - st.start;
            let (a, b) = (sa[i].1 * sa[i].2, sa[j].1 * sa[j].2);
            let det = a.x * b.y - a.y * b.x;
            if det.abs() > 1e-6 {
                let u = (d.x * b.y - d.y * b.x) / det;
                let w = (a.x * d.y - a.y * d.x) / det;
                let s = step(ui, 1., 0.1);
                let mut v = [st.values[0], st.values[1], st.values[2]];
                v[i] = snap(st.values[i] + u as f64, s);
                v[j] = snap(st.values[j] + w as f64, s);
                out = Some(v);
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
    }
    // setas lineares
    for i in 0..3 {
        let tip = c0 + sa[i].1 * ARM;
        let hid = egui::Id::unique(("gizmo_seta", salt, i));
        let sense = if locked[i] { egui::Sense::hover() } else { egui::Sense::drag() };
        let r = ui.interact(Rect::from_center_size(tip, egui::vec2(22., 22.)), hid, sense);
        let hot = r.hovered() || r.dragged();
        let col = if locked[i] { Color32::GRAY } else if hot { HOT } else { AXIS_COL[i] };
        arrow(&painter, c0, tip, col);
        if let Some((st, m)) = drag_state(ui, hid, &r, &vals) {
            let t = (m - st.start).dot(sa[i].1) / sa[i].2;
            let mut v = [st.values[0], st.values[1], st.values[2]];
            v[i] = snap(st.values[i] + t as f64, step(ui, 1., 0.1));
            out = Some(v);
            label(&painter, tip + egui::vec2(12., -8.), format!("{} mm", fmt(v[i])), col);
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if hot {
            let name = ["X", "Y", "Z"][i];
            let txt = if locked[i] { format!("🔒 {name} ligado a parâmetro") } else { format!("{name} = {}", fmt(vals[i])) };
            label(&painter, tip + egui::vec2(12., -8.), txt, col);
        }
    }
    painter.circle_filled(c0, 4., Color32::WHITE);
    out
}

/// Arco de rotação em volta de `axis` passando por `p`; devolve o novo ângulo (graus).
fn rotate_arc(ui: &mut egui::Ui, rect: Rect, cam: &Camera, salt: u64, origin: Vec3, axis: Vec3, src: Vec3, angle: f64, locked: bool) -> Option<f64> {
    let foot = origin + axis * (src - origin).dot(axis);
    let mut r0 = src - foot;
    if r0.length() < 1e-3 {
        // elemento no eixo: arco de tamanho fixo na tela
        let ppm = screen_axis(cam, rect, foot, axis.any_orthonormal_vector())?.2;
        r0 = axis.any_orthonormal_vector() * (ARM / ppm);
    }
    let at = |a: f64| foot + glam::Quat::from_axis_angle(axis, a.to_radians() as f32) * r0;
    let painter = ui.painter_at(rect);
    let ring: Vec<Pos2> = (0..=64).filter_map(|i| cam.project(rect, at(i as f64 * 360. / 64.))).collect();
    let tip = cam.project(rect, at(angle))?;
    let hid = egui::Id::unique(("gizmo_arco", salt));
    let r = ui.interact(Rect::from_center_size(tip, egui::vec2(22., 22.)), hid, if locked { egui::Sense::hover() } else { egui::Sense::drag() });
    let hot = r.hovered() || r.dragged();
    let col = if locked { Color32::GRAY } else if hot { HOT } else { Color32::from_rgb(120, 200, 255) };
    painter.add(egui::Shape::line(ring, Stroke::new(1.2, col.gamma_multiply(0.5))));
    let sweep: Vec<Pos2> = (0..=32).filter_map(|i| cam.project(rect, at(angle * i as f64 / 32.))).collect();
    painter.add(egui::Shape::line(sweep, Stroke::new(3., col)));
    painter.circle_filled(tip, 6., col);
    let mut out = None;
    if let Some((_, m)) = drag_state(ui, hid, &r, &[angle]) {
        let mut best = (f32::MAX, angle);
        for k in -180..=180 {
            if let Some(p) = cam.project(rect, at(k as f64)) {
                let d = (p - m).length();
                if d < best.0 {
                    best = (d, k as f64);
                }
            }
        }
        let v = snap(best.1, step(ui, 1., 5.));
        out = Some(v);
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if hot || out.is_some() {
        label(&painter, tip + egui::vec2(12., -8.), format!("{}°", fmt(out.unwrap_or(angle))), col);
    }
    out
}

/// Alça de escala: quadrado na diagonal do centro; a razão das distâncias ao centro dá o fator.
fn scale_handle(ui: &mut egui::Ui, rect: Rect, cam: &Camera, salt: u64, center: Vec3, factor: f64, locked: bool) -> Option<f64> {
    let c = cam.project(rect, center)?;
    let q = c + egui::vec2(ARM * 0.75, -ARM * 0.75);
    let painter = ui.painter_at(rect);
    let hid = egui::Id::unique(("gizmo_escala", salt));
    let r = ui.interact(Rect::from_center_size(q, egui::vec2(18., 18.)), hid, if locked { egui::Sense::hover() } else { egui::Sense::drag() });
    let hot = r.hovered() || r.dragged();
    let col = if locked { Color32::GRAY } else if hot { HOT } else { Color32::from_rgb(230, 160, 255) };
    painter.line_segment([c, q], Stroke::new(1.5, col));
    painter.rect_filled(Rect::from_center_size(q, egui::vec2(12., 12.)), 2., col);
    painter.circle_stroke(c, 5., Stroke::new(1.5, col));
    let mut out = None;
    if let Some((st, m)) = drag_state(ui, hid, &r, &[factor]) {
        let d0 = (st.start - c).length().max(1.);
        let k = (m - c).length() / d0;
        out = Some(snap((st.values[0] * k as f64).max(0.01), step(ui, 0.1, 0.01)));
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if hot || out.is_some() {
        label(&painter, q + egui::vec2(12., -8.), format!("× {}", fmt(out.unwrap_or(factor))), col);
    }
    out
}

/// Desenha o manipulador do elemento em edição. Devolve `true` se tratou o elemento.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) -> bool {
    let Some(rf) = app.open.doc.reference(id).cloned() else { return false };
    let Some(r) = app.result.as_ref() else { return false };
    let vars = app.vars();
    let ev = |e: &str| scc_core::expr::eval_str(e, &vars).ok();
    match &rf.kind {
        RefKind::PointCoords { x, y, z, frame: None } => {
            let Some(o) = center_of(r, id) else { return true };
            let vals = [ev(x).unwrap_or(0.), ev(y).unwrap_or(0.), ev(z).unwrap_or(0.)];
            let locked = [num(x).is_none(), num(y).is_none(), num(z).is_none()];
            if let Some(v) = translate_gizmo(ui, rect, cam, id.0, o, [Vec3::X, Vec3::Y, Vec3::Z], vals, locked) {
                if let Some(RefKind::PointCoords { x, y, z, .. }) = app.open.doc.references.iter_mut().find(|q| q.id == id).map(|q| &mut q.kind) {
                    for (e, val) in [x, y, z].into_iter().zip(v) {
                        if num(e).is_some() {
                            *e = fmt(val);
                        }
                    }
                }
            }
            true
        }
        RefKind::GeoTransform { xf, .. } => {
            let Some(c) = center_of(r, id) else { return true };
            match xf {
                GeoXform::Translate { dir, dist } => {
                    let Some((_, d)) = axis_of(app, dir) else { return true };
                    let Some(val) = ev(dist) else { return true };
                    // seta única ao longo da direção, ancorada na posição original
                    let base = c - d * val as f32;
                    let Some((sb, sd, ppm)) = screen_axis(cam, rect, base, d) else { return true };
                    let tip = sb + sd * (val as f32 * ppm).clamp(-4000., 4000.);
                    let painter = ui.painter_at(rect);
                    let hid = egui::Id::unique(("gizmo_mover", id.0));
                    let locked = num(dist).is_none();
                    let resp = ui.interact(Rect::from_center_size(tip, egui::vec2(22., 22.)), hid, if locked { egui::Sense::hover() } else { egui::Sense::drag() });
                    let hot = resp.hovered() || resp.dragged();
                    let col = if locked { Color32::GRAY } else if hot { HOT } else { AXIS_COL[0] };
                    painter.line_segment([sb, tip], Stroke::new(1., col.gamma_multiply(0.6)));
                    arrow(&painter, tip, tip + sd * 40., col);
                    painter.circle_filled(tip, 5., col);
                    if let Some((st, m)) = drag_state(ui, hid, &resp, &[val]) {
                        let v = snap(st.values[0] + ((m - st.start).dot(sd) / ppm) as f64, step(ui, 1., 0.1));
                        set_xf(app, id, |x| {
                            if let GeoXform::Translate { dist, .. } = x {
                                *dist = fmt(v)
                            }
                        });
                    }
                    if hot {
                        label(&painter, tip + egui::vec2(12., -8.), format!("{} mm", fmt(val)), col);
                    }
                }
                GeoXform::Rotate { axis, angle } => {
                    let Some((o, d)) = axis_of(app, axis) else { return true };
                    let Some(a) = ev(angle) else { return true };
                    // posição original = centro atual girado de volta
                    let src = o + glam::Quat::from_axis_angle(d, -(a.to_radians() as f32)) * (c - o);
                    if let Some(v) = rotate_arc(ui, rect, cam, id.0, o, d, src, a, num(angle).is_none()) {
                        set_xf(app, id, |x| {
                            if let GeoXform::Rotate { angle, .. } = x {
                                *angle = fmt(v)
                            }
                        });
                    }
                }
                GeoXform::Scale { center, factor } => {
                    let Some(p) = point_of(app, center) else { return true };
                    let Some(f) = ev(factor) else { return true };
                    if let Some(v) = scale_handle(ui, rect, cam, id.0, p, f, num(factor).is_none()) {
                        set_xf(app, id, |x| {
                            if let GeoXform::Scale { factor, .. } = x {
                                *factor = fmt(v)
                            }
                        });
                    }
                }
                GeoXform::Mirror { .. } => {}
            }
            true
        }
        _ => false,
    }
}

fn set_xf(app: &mut App, id: Id, f: impl FnOnce(&mut GeoXform)) {
    if let Some(RefKind::GeoTransform { xf, .. }) = app.open.doc.references.iter_mut().find(|q| q.id == id).map(|q| &mut q.kind) {
        f(xf);
    }
}
