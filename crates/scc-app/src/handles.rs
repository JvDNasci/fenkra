//! "Fast 3D": alças na viewport para o comando aberto — seta arrastável e cota editável
//! (extrusão/recorte, plano deslocado, altura das primitivas, profundidade do reforço).
//! Arrastar muda o valor em passos de 1 mm (Shift: 0,1 mm); duplo clique na cota abre a edição,
//! que aceita expressões. Valores ligados a parâmetros ficam travados (🔒) para não quebrar o vínculo.

use crate::app::{fmt, App};
use crate::render::Camera;
use eframe::egui::{self, Color32, Rect, RichText, Stroke};
use glam::Vec3;
use scc_core::*;

/// Alça linear: valor = distância ao longo de `dir` a partir de `base` (vezes `k`).
struct Linear {
    base: Vec3,
    dir: Vec3,
    /// valor exibido = comprimento da seta × k (plano médio: k = 2)
    k: f32,
    /// expressão editada
    expr: String,
    label: &'static str,
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Origem e normal de um plano já calculado.
fn plane_of(r: &scc_solidos::RecomputeResult, p: &PlaneRef) -> Option<(Vec3, Vec3)> {
    let f = match p {
        PlaneRef::Origin(o) => scc_solidos::origin_frame(*o),
        PlaneRef::Datum(id) => r.planes.iter().find(|x| x.id == Some(*id))?.frame,
    };
    Some((v3(f.origin), v3(f.normal)))
}

/// Centro do perfil de um sketch e a normal do plano dele.
fn sketch_center(r: &scc_solidos::RecomputeResult, id: Id) -> Option<(Vec3, Vec3)> {
    let s = r.sketches.iter().find(|s| s.feature == id)?;
    let n = v3(s.frame.normal);
    let c = if s.lines.is_empty() {
        v3(s.frame.origin)
    } else {
        s.lines.iter().fold(Vec3::ZERO, |a, p| a + Vec3::from(*p)) / s.lines.len() as f32
    };
    Some((c, n))
}

fn handle_for(app: &App, id: Id) -> Option<Linear> {
    let r = app.result.as_ref()?;
    let doc = &app.open.doc;
    if let Some(f) = doc.feature(id) {
        return match &f.kind {
            FeatureKind::Extrude(e) => {
                let (c, n) = sketch_center(r, e.sketch)?;
                let into = (e.op == BoolOp::Cut) != e.reverse;
                let dir = if into { -n } else { n };
                match &e.extent {
                    Extent::Blind(d) => Some(Linear { base: c, dir, k: 1., expr: d.clone(), label: "Distância" }),
                    Extent::Symmetric(d) => Some(Linear { base: c, dir, k: 2., expr: d.clone(), label: "Distância total" }),
                    Extent::TwoSides(a, _) => Some(Linear { base: c, dir, k: 1., expr: a.clone(), label: "Distância 1" }),
                    _ => None,
                }
            }
            FeatureKind::Primitive(p) => {
                let (o, n) = plane_of(r, &p.plane)?;
                let h = match p.kind {
                    PrimKind::Caixa | PrimKind::Cone | PrimKind::Prisma | PrimKind::Cunha => p.c.clone(),
                    PrimKind::Cilindro => p.b.clone(),
                    _ => return None,
                };
                Some(Linear { base: o, dir: n, k: 1., expr: h, label: "Altura" })
            }
            FeatureKind::Stiffener(s) => {
                let (c, n) = sketch_center(r, s.sketch)?;
                Some(Linear { base: c, dir: if s.reverse { n } else { -n }, k: 1., expr: s.depth.clone(), label: "Profundidade" })
            }
            _ => None,
        };
    }
    let rf = doc.reference(id)?;
    match &rf.kind {
        RefKind::OffsetPlane { base, offset } => {
            let (o, n) = plane_of(r, base)?;
            // a seta sai do centro do plano deslocado projetado no plano base
            let me = r.planes.iter().find(|x| x.id == Some(id)).map(|x| v3(x.frame.origin)).unwrap_or(o);
            let b = me - n * (me - o).dot(n);
            Some(Linear { base: b, dir: n, k: 1., expr: offset.clone(), label: "Deslocamento" })
        }
        _ => None,
    }
}

/// Grava o novo valor na expressão da alça.
fn write_value(app: &mut App, id: Id, value: String) {
    let doc = &mut app.open.doc;
    if let Some(f) = doc.feature_mut(id) {
        match &mut f.kind {
            FeatureKind::Extrude(e) => match &mut e.extent {
                Extent::Blind(d) | Extent::Symmetric(d) | Extent::TwoSides(d, _) => *d = value,
                _ => {}
            },
            FeatureKind::Primitive(p) => match p.kind {
                PrimKind::Caixa | PrimKind::Cone | PrimKind::Prisma | PrimKind::Cunha => p.c = value,
                PrimKind::Cilindro => p.b = value,
                _ => {}
            },
            FeatureKind::Stiffener(s) => s.depth = value,
            _ => {}
        }
        return;
    }
    if let Some(RefElement { kind: RefKind::OffsetPlane { offset, .. }, .. }) = doc.references.iter_mut().find(|r| r.id == id) {
        *offset = value;
    }
}

/// Desenha e trata a alça do comando aberto. Chamado pela viewport fora do modo sketch.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: Camera) {
    let Some(id) = app.session.as_ref().map(|s| s.id) else { return };
    let Some(h) = handle_for(app, id) else { return };
    let vars = app.vars();
    let Ok(val) = scc_core::expr::eval_str(&h.expr, &vars) else { return };
    let numeric = h.expr.trim().parse::<f64>().is_ok();
    let len = val as f32 / h.k;
    let tip = h.base + h.dir * len;
    let (Some(sb), Some(st)) = (cam.project(rect, h.base), cam.project(rect, tip)) else { return };
    // escala da tela: pixels por mm ao longo do eixo
    let Some(s1) = cam.project(rect, h.base + h.dir) else { return };
    let ppm = (s1 - sb).length().max(1e-3);
    let axis = (s1 - sb) / ppm;

    let painter = ui.painter_at(rect);
    let col = Color32::from_rgb(90, 200, 255);
    let hot = Color32::from_rgb(255, 190, 40);
    let hid = egui::Id::unique(("alca_3d", id.0));
    let hr = ui.interact(Rect::from_center_size(st, egui::vec2(22., 22.)), hid, if numeric { egui::Sense::click_and_drag() } else { egui::Sense::hover() });
    let active = hr.hovered() || hr.dragged();
    let c = if active { hot } else { col };

    // haste e ponta
    painter.line_segment([sb, st], Stroke::new(2.5, c));
    let d = if (st - sb).length() > 1. { (st - sb).normalized() } else { axis };
    let n = egui::vec2(-d.y, d.x);
    painter.add(egui::Shape::convex_polygon(vec![st + d * 14., st + n * 7., st - n * 7.], c, Stroke::new(1., Color32::from_black_alpha(120))));
    painter.circle_filled(sb, 3.5, c);
    painter.circle_stroke(st, 9., Stroke::new(1.5, c));

    // arrastar ao longo do eixo
    if hr.dragged() {
        if let Some(m) = ui.input(|i| i.pointer.interact_pos()) {
            let t = (m - sb).dot(axis) / ppm;
            let fine = ui.input(|i| i.modifiers.shift);
            let step = if fine { 0.1 } else { 1. };
            let mut v = ((t * h.k) / step).round() * step;
            if v.abs() < step {
                v = step * v.signum().max(1.);
            }
            write_value(app, id, fmt(v as f64));
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if hr.hovered() && numeric {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }

    // cota editável junto da ponta
    let key = egui::Id::unique(("alca_edit", id.0));
    let mut editing: Option<String> = ui.ctx().data(|m| m.get_temp(key));
    let txt = if numeric { format!("{} mm", fmt(val)) } else { format!("🔒 {} = {}", h.expr.trim(), fmt(val)) };
    let lp = st + n * 22. + d * 10.;
    let galley = painter.layout_no_wrap(txt, egui::FontId::proportional(13.), Color32::WHITE);
    let lr = Rect::from_min_size(lp, galley.size() + egui::vec2(10., 6.));
    painter.rect_filled(lr, 4., Color32::from_rgba_unmultiplied(20, 60, 110, 220));
    painter.rect_stroke(lr, 4., Stroke::new(1., c), egui::StrokeKind::Inside);
    painter.galley(lr.min + egui::vec2(5., 3.), galley, Color32::WHITE);
    let lresp = ui.interact(lr, egui::Id::unique(("alca_cota", id.0)), egui::Sense::click());
    let lresp = lresp.on_hover_text(format!("{} · arraste a seta (Shift = 0,1 mm) ou dê duplo clique para digitar (aceita expressão)", h.label));
    if lresp.double_clicked() || lresp.clicked() {
        editing = Some(h.expr.clone());
    }
    if let Some(mut text) = editing.take() {
        let mut done: Option<bool> = None;
        egui::Area::new(egui::Id::unique(("alca_area", id.0))).fixed_pos(lr.left_bottom() + egui::vec2(0., 4.)).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(RichText::new(h.label).small());
                let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(110.));
                r.request_focus();
                let ok = scc_core::expr::eval_str(&text, &vars).is_ok();
                if !ok {
                    ui.label(RichText::new("expressão inválida").small().color(Color32::from_rgb(255, 120, 100)));
                }
                let (enter, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
                if enter && ok {
                    done = Some(true);
                } else if esc {
                    done = Some(false);
                }
            });
        });
        match done {
            Some(true) => {
                // a cota guarda o comprimento total; a seta do plano médio mostra metade
                write_value(app, id, text.trim().to_string());
                ui.ctx().data_mut(|m| m.remove::<String>(key));
            }
            Some(false) => {
                ui.ctx().data_mut(|m| m.remove::<String>(key));
            }
            None => {
                ui.ctx().data_mut(|m| m.insert_temp(key, text));
            }
        }
    }
}
