//! Detalhamento 3D (PMI): criação pelas seleções da viewport, desenho sobre o modelo (linhas de
//! cota com setas, chamadas, símbolos) e painel de edição. Peça e montagem têm a própria lista.

use crate::app::{App, Sel};
use crate::i18n::tr;
use crate::picking::Pick;
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke};
use glam::Vec3;
use scc_core::pmi::*;
use scc_core::Id;
use scc_solidos::RecomputeResult;

const COL: Color32 = Color32::from_rgb(255, 150, 60);

/// Tipos na ordem dos comandos da barra.
pub fn kind_of(i: u8) -> PmiKind {
    match i {
        0 => PmiKind::Linear,
        1 => PmiKind::Radius,
        2 => PmiKind::Diameter,
        3 => PmiKind::Angle,
        4 => PmiKind::Note,
        5 => PmiKind::Roughness { ra: "1,6".into(), process: String::new() },
        6 => PmiKind::Weld { kind: WeldKind::Filete, size: "a4".into(), all_around: false },
        7 => PmiKind::Datum { letter: "A".into() },
        _ => PmiKind::Gdt { ch: GdtChar::Planeza, tol: "0,05".into(), datums: String::new() },
    }
}

/// Lista de PMI do nível atual (montagem aberta na bancada Montagem, senão a peça).
pub fn list(app: &App) -> &Vec<Pmi> {
    match app.asm.as_ref().filter(|_| app.in_asm()) {
        Some(st) => &st.asm.pmi,
        None => &app.open.doc.pmi,
    }
}

pub fn list_mut(app: &mut App) -> &mut Vec<Pmi> {
    if app.in_asm() {
        &mut app.asm.as_mut().unwrap().asm.pmi
    } else {
        &mut app.open.doc.pmi
    }
}

/// Âncoras a partir das seleções: vértice → posição; aresta → ponto médio (raio/diâmetro: centro
/// e um ponto do arco); face → ponto clicado.
fn anchors(app: &App, kind: &PmiKind, picks: &[Pick]) -> Option<Vec<[f64; 3]>> {
    let r = app.result.as_ref();
    let pt = |p: &Pick| -> Option<[f64; 3]> {
        Some(match *p {
            Pick::Vertex { pos, .. } => pos.map(|v| v as f64),
            Pick::Edge { mid, .. } => mid.map(|v| v as f64),
            Pick::Face { point, .. } => point.map(|v| v as f64),
            _ => return None,
        })
    };
    match kind {
        PmiKind::Radius | PmiKind::Diameter => {
            let Pick::Edge { body, edge, mid } = *picks.first()? else { return None };
            let pts = crate::picking::edge_polyline(r?, body, edge);
            let c = scc_solidos::geo3d::Curve3::poly(pts, false).ok()?;
            let (ctr, _, _) = scc_solidos::geo3d::fit_circle(&c).ok()?;
            Some(vec![ctr, mid.map(|v| v as f64)])
        }
        k => {
            let v: Vec<[f64; 3]> = picks.iter().filter_map(pt).collect();
            (v.len() >= k.anchors()).then(|| v[..k.anchors()].to_vec())
        }
    }
}

/// Cria a anotação com as seleções atuais; sem seleções suficientes, espera por elas.
pub fn command(app: &mut App, i: u8) {
    let kind = kind_of(i);
    let picks = app.picks.clone();
    match anchors(app, &kind, &picks) {
        Some(a) => create(app, kind, a),
        None => {
            app.pmi_pending = Some(i);
            let need = match &kind {
                PmiKind::Radius | PmiKind::Diameter => tr("uma aresta circular").to_string(),
                k => format!("{} {}", k.anchors(), tr("elemento(s): vértice, aresta ou face")),
            };
            app.toast(format!("{}: {} {need}", tr(kind.label()), tr("escolha")), false);
        }
    }
}

fn create(app: &mut App, kind: PmiKind, a: Vec<[f64; 3]>) {
    let id = Id(app.next_pmi_id());
    let n = list(app).iter().filter(|p| p.kind.label() == kind.label()).count() + 1;
    let name = format!("{} {n}", tr(kind.label()));
    let off = match kind {
        PmiKind::Linear | PmiKind::Angle => [0., 0., 12.],
        _ => [15., 0., 15.],
    };
    list_mut(app).push(Pmi { id, name, kind, anchors: a, offset: off, text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None });
    app.picks.clear();
    app.pmi_pending = None;
    if app.in_asm() {
        let st = app.asm.as_mut().unwrap();
        st.sel = Some(id);
        st.modified = true;
    } else {
        app.sel = Sel::Pmi(id);
    }
}

/// Seleções feitas com uma anotação pendente.
pub fn after_viewport(app: &mut App) {
    let Some(i) = app.pmi_pending else { return };
    let kind = kind_of(i);
    let picks = app.picks.clone();
    if let Some(a) = anchors(app, &kind, &picks) {
        create(app, kind, a);
    }
}

/// Reencaixa a âncora no vértice ou meio de aresta mais próximo (até 2 mm).
fn snap(r: &RecomputeResult, p: [f64; 3]) -> [f64; 3] {
    let mut best = (2.0f64, p);
    for b in &r.bodies {
        for q in b.mesh.vertices.chunks_exact(3).chain(b.mesh.edge_mids.chunks_exact(3)) {
            let q = [q[0] as f64, q[1] as f64, q[2] as f64];
            let d = ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) + (q[2] - p[2]).powi(2)).sqrt();
            if d < best.0 {
                best = (d, q);
            }
        }
    }
    best.1
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

fn arrow(p: &egui::Painter, tip: Pos2, from: Pos2) {
    let d = (tip - from).normalized();
    let n = egui::vec2(-d.y, d.x);
    p.add(egui::Shape::convex_polygon(vec![tip, tip - d * 9. + n * 3.5, tip - d * 9. - n * 3.5], COL, Stroke::NONE));
}

/// Rótulo deitado num plano: girado conforme a direção x do plano na tela e com altura de texto
/// em mm do modelo (acompanha o zoom). Devolve o retângulo (sem giro) para clique/arraste.
fn label_on_plane(p: &egui::Painter, cam: &Camera, rect: Rect, world: Vec3, x: Vec3, txt: &str, sel: bool) -> Option<Rect> {
    let at = cam.project(rect, world)?;
    let ax = cam.project(rect, world + x * 10.)?;
    let d = ax - at;
    let px_per_mm = d.length() / 10.;
    let size = (5. * px_per_mm).clamp(6., 60.);
    let angle = d.y.atan2(d.x);
    let g = p.layout_no_wrap(txt.to_string(), egui::FontId::proportional(size), Color32::WHITE);
    let (w, h) = (g.size().x + 10., g.size().y + 6.);
    // caixa girada: quatro cantos em volta do centro
    let (c, s) = (angle.cos(), angle.sin());
    let rot = |v: egui::Vec2| egui::vec2(v.x * c - v.y * s, v.x * s + v.y * c);
    let corners: Vec<Pos2> = [(-w / 2., -h / 2.), (w / 2., -h / 2.), (w / 2., h / 2.), (-w / 2., h / 2.)].iter().map(|(a, b)| at + rot(egui::vec2(*a, *b))).collect();
    p.add(egui::Shape::convex_polygon(corners.clone(), Color32::from_rgba_unmultiplied(60, 30, 10, 225), Stroke::new(if sel { 2. } else { 1. }, if sel { Color32::from_rgb(255, 220, 120) } else { COL })));
    let tl = at + rot(egui::vec2(-w / 2. + 5., -h / 2. + 3.));
    p.add(egui::epaint::TextShape::new(tl, g, Color32::WHITE).with_angle(angle));
    Some(Rect::from_center_size(at, egui::vec2(w.max(h), w.max(h)) * 0.8))
}

fn label(p: &egui::Painter, at: Pos2, txt: &str, sel: bool) -> Rect {
    let g = p.layout_no_wrap(txt.to_string(), egui::FontId::proportional(13.), Color32::WHITE);
    let r = Rect::from_center_size(at, g.size() + egui::vec2(10., 6.));
    p.rect_filled(r, 3., Color32::from_rgba_unmultiplied(60, 30, 10, 225));
    p.rect_stroke(r, 3., Stroke::new(if sel { 2. } else { 1. }, if sel { Color32::from_rgb(255, 220, 120) } else { COL }), egui::StrokeKind::Inside);
    p.galley(r.min + egui::vec2(5., 3.), g, Color32::WHITE);
    r
}

/// Desenha as anotações sobre a viewport (o clique num rótulo seleciona a anotação).
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let mut action: Option<(Id, u8)> = None;
    let mut drag: Option<(Id, [f64; 3])> = None;
    let Some(r) = app.result.as_ref() else { return };
    let sel = if app.in_asm() { app.asm.as_ref().and_then(|s| s.sel) } else if let Sel::Pmi(i) = app.sel { Some(i) } else { None };
    let painter = ui.painter_at(rect);
    let mut clicked = None;
    for pm in list(app).iter().filter(|p| p.visible) {
        let a: Vec<[f64; 3]> = pm.anchors.iter().map(|p| snap(r, *p)).collect();
        let Some(first) = a.first().copied() else { continue };
        let val = pm.measure(&a);
        let txt = pm.display(val);
        let is_sel = sel == Some(pm.id);
        let stroke = Stroke::new(if is_sel { 2. } else { 1.3 }, COL);
        let off = v3(pm.offset);
        let rect_lbl = match pm.kind {
            PmiKind::Linear if a.len() >= 2 => {
                // linhas de chamada e linha de cota afastadas pelo deslocamento
                let (p0, p1) = (v3(a[0]), v3(a[1]));
                let (q0, q1) = (p0 + off, p1 + off);
                let (Some(s0), Some(s1), Some(t0), Some(t1)) = (cam.project(rect, p0), cam.project(rect, p1), cam.project(rect, q0), cam.project(rect, q1)) else { continue };
                painter.line_segment([s0, t0], Stroke::new(1., COL));
                painter.line_segment([s1, t1], Stroke::new(1., COL));
                painter.line_segment([t0, t1], stroke);
                if (t1 - t0).length() > 4. {
                    arrow(&painter, t0, t1);
                    arrow(&painter, t1, t0);
                }
                label(&painter, t0 + (t1 - t0) * 0.5 + egui::vec2(0., -12.), &txt, is_sel)
            }
            PmiKind::Angle if a.len() >= 3 => {
                let (Some(sv), Some(s1), Some(s2)) = (cam.project(rect, v3(a[0])), cam.project(rect, v3(a[1])), cam.project(rect, v3(a[2]))) else { continue };
                painter.line_segment([sv, s1], Stroke::new(1., COL));
                painter.line_segment([sv, s2], Stroke::new(1., COL));
                let (d1, d2) = ((s1 - sv).normalized(), (s2 - sv).normalized());
                let rad = 30.;
                let pts: Vec<Pos2> = (0..=16).map(|k| {
                    let t = k as f32 / 16.;
                    sv + (d1 * (1. - t) + d2 * t).normalized() * rad
                }).collect();
                painter.add(egui::Shape::line(pts, stroke));
                label(&painter, sv + (d1 + d2).normalized() * (rad + 22.), &txt, is_sel)
            }
            _ => {
                // chamada: da âncora até o rótulo
                let (Some(s0), Some(st)) = (cam.project(rect, v3(first)), cam.project(rect, v3(first) + off)) else { continue };
                painter.line_segment([s0, st], stroke);
                painter.circle_filled(s0, 3., COL);
                if let (PmiKind::Radius | PmiKind::Diameter, Some(s1)) = (&pm.kind, a.get(1).and_then(|q| cam.project(rect, v3(*q)))) {
                    painter.line_segment([s0, s1], stroke);
                    arrow(&painter, s1, s0);
                }
                if let PmiKind::Datum { .. } = pm.kind {
                    let n = (st - s0).normalized();
                    let m = egui::vec2(-n.y, n.x);
                    painter.add(egui::Shape::convex_polygon(vec![s0, s0 + n * 10. + m * 6., s0 + n * 10. - m * 6.], COL, Stroke::NONE));
                }
                match pm.plane {
                    Some((_, x)) => label_on_plane(&painter, cam, rect, v3(first) + off, v3(x).normalize_or_zero(), &txt, is_sel).unwrap_or_else(|| label(&painter, st, &txt, is_sel)),
                    None => label(&painter, st, &txt, is_sel),
                }
            }
        };
        let resp = ui.interact(rect_lbl, egui::Id::unique(("pmi_rotulo", pm.id.0)), egui::Sense::click_and_drag());
        // arrastar o rótulo: move o texto no plano da anotação (ou de frente para a tela)
        if resp.dragged() {
            if let Some(m) = resp.interact_pointer_pos() {
                let base = v3(first) + off;
                let n = match pm.plane {
                    Some((n, _)) => v3(n).normalize_or_zero(),
                    None => -cam.ray(rect, rect.center()).1,
                };
                let (o, dir) = cam.ray(rect, m);
                let den = dir.dot(n);
                if den.abs() > 1e-6 {
                    let t = (base - o).dot(n) / den;
                    let hit = o + dir * t;
                    let new_off = hit - v3(first);
                    drag = Some((pm.id, [new_off.x as f64, new_off.y as f64, new_off.z as f64]));
                }
            }
        }
        if resp.clicked() || resp.secondary_clicked() {
            clicked = Some(pm.id);
        }
        // botão direito no rótulo: opções da anotação
        resp.context_menu(|ui| {
            ui.label(RichText::new(&pm.name).strong());
            if ui.button(tr("✏ Editar")).clicked() {
                action = Some((pm.id, 2));
                ui.close();
            }
            if ui.button(tr("Ocultar")).clicked() {
                action = Some((pm.id, 1));
                ui.close();
            }
            if ui.button(tr(if pm.plane.is_some() { "🖵 De frente para a tela" } else { "▱ Deitar no plano da vista atual" })).clicked() {
                action = Some((pm.id, 3));
                ui.close();
            }
            ui.separator();
            if ui.button(tr("🗑 Apagar   Del")).clicked() {
                action = Some((pm.id, 0));
                ui.close();
            }
        });
        resp.on_hover_text(&pm.name);
    }
    if let Some((id, off)) = drag {
        if let Some(p) = list_mut(app).iter_mut().find(|p| p.id == id) {
            p.offset = off;
        }
        clicked = Some(id);
    }
    if let Some((id, k)) = action {
        match k {
            0 => {
                list_mut(app).retain(|p| p.id != id);
                clicked = None;
                if app.sel == Sel::Pmi(id) {
                    app.sel = Sel::None;
                }
            }
            1 => {
                if let Some(p) = list_mut(app).iter_mut().find(|p| p.id == id) {
                    p.visible = false;
                }
            }
            3 => {
                // alterna: de frente ↔ deitado no plano da tela atual (fica fixo nele ao girar)
                let (o, d) = cam.ray(rect, rect.center());
                let (_, dx) = cam.ray(rect, rect.center() + egui::vec2(10., 0.));
                let n = -d;
                let x = (dx - d).normalize_or_zero();
                let _ = o;
                if let Some(p) = list_mut(app).iter_mut().find(|p| p.id == id) {
                    p.plane = match p.plane {
                        Some(_) => None,
                        None => Some(([n.x as f64, n.y as f64, n.z as f64], [x.x as f64, x.y as f64, x.z as f64])),
                    };
                }
            }
            _ => clicked = Some(id),
        }
    }
    if let Some(id) = clicked {
        if app.in_asm() {
            app.asm.as_mut().unwrap().sel = Some(id);
        } else {
            app.sel = Sel::Pmi(id);
        }
    }
}

/// Seção "PMI" na árvore (peça ou montagem).
pub fn tree_section(app: &mut App, ui: &mut egui::Ui) {
    let n = list(app).len();
    let mut select = None;
    let mut action: Option<(Id, u8)> = None;
    let sel = if app.in_asm() { app.asm.as_ref().and_then(|s| s.sel) } else if let Sel::Pmi(i) = app.sel { Some(i) } else { None };
    let mut create: Option<u8> = None;
    let h = egui::CollapsingHeader::new(format!("📐 {} ({n})", tr("PMI — detalhamento 3D"))).id_salt("arvore_pmi").default_open(n > 0).show(ui, |ui| {
        if n == 0 {
            ui.label(RichText::new(tr("Cotas, notas, rugosidade, solda e tolerâncias no modelo 3D (grupo Detalhamento 3D).")).weak().small());
        }
        for p in list(app) {
            let txt = format!("{} {}", if p.visible { "📐" } else { "·" }, p.name);
            let r = ui.selectable_label(sel == Some(p.id), if p.visible { RichText::new(txt) } else { RichText::new(txt).weak() });
            if r.clicked() {
                select = Some(p.id);
            }
            r.context_menu(|ui| {
                ui.label(RichText::new(&p.name).strong());
                if ui.button(tr("✏ Editar")).clicked() {
                    select = Some(p.id);
                    ui.close();
                }
                if ui.button(if p.visible { tr("Ocultar") } else { tr("Mostrar") }).clicked() {
                    action = Some((p.id, 1));
                    ui.close();
                }
                ui.separator();
                if ui.button(tr("🗑 Apagar")).clicked() {
                    action = Some((p.id, 0));
                    ui.close();
                }
            });
        }
    });
    // capturas (só na peça): vista + anotações visíveis
    let mut cap_act: Option<(usize, u8)> = None;
    if !app.in_asm() {
        let caps: Vec<String> = app.open.doc.pmi_captures.iter().map(|c| c.name.clone()).collect();
        if !caps.is_empty() {
            ui.indent("capturas_pmi", |ui| {
                for (i, n) in caps.iter().enumerate() {
                    let r = ui.selectable_label(false, format!("📷 {n}")).on_hover_text(tr("aplicar: câmera e anotações desta captura"));
                    if r.clicked() {
                        cap_act = Some((i, 0));
                    }
                    r.context_menu(|ui| {
                        if ui.button(tr("📷 Atualizar com a vista atual")).clicked() {
                            cap_act = Some((i, 1));
                            ui.close();
                        }
                        if ui.button(tr("🗑 Apagar captura")).clicked() {
                            cap_act = Some((i, 2));
                            ui.close();
                        }
                    });
                }
            });
        }
    }
    h.header_response.context_menu(|ui| {
        if !app.in_asm() && ui.button(tr("📷 Capturar vista (câmera + anotações visíveis)")).clicked() {
            cap_act = Some((usize::MAX, 1));
            ui.close();
        }
        ui.label(RichText::new(tr("Nova anotação 3D")).strong());
        for (i, n) in [(0u8, "↔ Cota linear"), (1, "R Raio"), (2, "Ø Diâmetro"), (3, "∠ Ângulo"), (4, "✎ Nota"), (5, "√ Rugosidade"), (6, "⌒ Solda"), (7, "▭ Referência (datum)"), (8, "⊞ Tolerância geométrica")] {
            if ui.button(tr(n)).clicked() {
                create = Some(i);
                ui.close();
            }
        }
        ui.separator();
        if ui.button(tr("👁 Mostrar todas")).clicked() {
            action = Some((Id(0), 2));
            ui.close();
        }
        if ui.button(tr("Ocultar todas")).clicked() {
            action = Some((Id(0), 3));
            ui.close();
        }
    });
    if let Some(id) = select {
        if app.in_asm() {
            app.asm.as_mut().unwrap().sel = Some(id);
        } else {
            app.sel = Sel::Pmi(id);
        }
    }
    if let Some(i) = create {
        command(app, i);
    }
    if let Some((i, k)) = cap_act {
        let view = app.camera_get().map(|c| scc_core::NamedView { name: String::new(), target: c.target, yaw: c.yaw, pitch: c.pitch, dist: c.dist, ortho: c.ortho });
        let shown: Vec<Id> = app.open.doc.pmi.iter().filter(|p| p.visible).map(|p| p.id).collect();
        match k {
            0 => {
                if let Some(c) = app.open.doc.pmi_captures.get(i).cloned() {
                    for p in &mut app.open.doc.pmi {
                        p.visible = c.shown.contains(&p.id);
                    }
                    app.apply_view(&c.view);
                }
            }
            1 => {
                if let Some(v) = view {
                    if i == usize::MAX {
                        let id = Id(app.next_pmi_id());
                        let n = app.open.doc.pmi_captures.len() + 1;
                        app.open.doc.pmi_captures.push(scc_core::pmi::PmiCapture { id, name: format!("{} {n}", tr("Captura")), view: v, shown });
                    } else if let Some(c) = app.open.doc.pmi_captures.get_mut(i) {
                        c.view = v;
                        c.shown = shown;
                    }
                }
            }
            _ => {
                if i < app.open.doc.pmi_captures.len() {
                    app.open.doc.pmi_captures.remove(i);
                }
            }
        }
    }
    if let Some((id, k)) = action {
        let l = list_mut(app);
        match k {
            0 => l.retain(|p| p.id != id),
            2 | 3 => l.iter_mut().for_each(|p| p.visible = k == 2),
            _ => {
                if let Some(p) = l.iter_mut().find(|p| p.id == id) {
                    p.visible = !p.visible;
                }
            }
        }
    }
}

/// Painel de edição da anotação selecionada. Devolve `true` se havia uma anotação.
pub fn panel(app: &mut App, ui: &mut egui::Ui, id: Id) -> bool {
    let seed_pts: Vec<[f64; 3]> = app.picks.iter().filter_map(|p| match *p {
        Pick::Vertex { pos, .. } => Some(pos.map(|v| v as f64)),
        Pick::Edge { mid, .. } => Some(mid.map(|v| v as f64)),
        Pick::Face { point, .. } => Some(point.map(|v| v as f64)),
        _ => None,
    }).collect();
    // plano da face/plano clicado (para deitar o texto nele)
    let seed_plane: Option<([f64; 3], [f64; 3])> = app.picks.iter().find_map(|p| match *p {
        Pick::Face { normal, .. } => {
            let n = Vec3::new(normal[0], normal[1], normal[2]).normalize_or_zero();
            let helper = if n.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
            let x = helper.cross(n).normalize_or_zero();
            Some(([n.x as f64, n.y as f64, n.z as f64], [x.x as f64, x.y as f64, x.z as f64]))
        }
        Pick::Plane(pr) => app.result.as_ref().and_then(|r| {
            r.planes.iter().find(|v| match pr {
                scc_core::PlaneRef::Datum(id) => v.id == Some(id),
                scc_core::PlaneRef::Origin(o) => v.id.is_none() && v.name == crate::scene::plane_name(o),
            })
        }).map(|v| (v.frame.normal, v.frame.x)),
        _ => None,
    });
    let l = list_mut(app);
    let Some(p) = l.iter_mut().find(|p| p.id == id) else { return false };
    ui.heading(format!("📐 {}", p.name));
    ui.label(RichText::new(tr(p.kind.label())).weak());
    ui.horizontal(|ui| {
        ui.label(tr("Nome"));
        ui.text_edit_singleline(&mut p.name);
    });
    match &mut p.kind {
        PmiKind::Note => {
            ui.label(tr("Texto"));
            ui.text_edit_multiline(&mut p.text);
        }
        PmiKind::Roughness { ra, process } => {
            ui.horizontal(|ui| {
                ui.label("Ra");
                ui.text_edit_singleline(ra);
            });
            ui.horizontal(|ui| {
                ui.label(tr("Processo"));
                ui.text_edit_singleline(process);
            });
        }
        PmiKind::Weld { kind, size, all_around } => {
            egui::ComboBox::from_id_salt("pmi_solda").selected_text(tr(kind.label())).show_ui(ui, |ui| {
                for k in WeldKind::ALL {
                    ui.selectable_value(kind, k, format!("{} {}", k.symbol(), tr(k.label())));
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("Medida"));
                ui.text_edit_singleline(size);
            });
            ui.checkbox(all_around, tr("solda em todo o contorno"));
        }
        PmiKind::Datum { letter } => {
            ui.horizontal(|ui| {
                ui.label(tr("Letra"));
                ui.text_edit_singleline(letter);
            });
        }
        PmiKind::Gdt { ch, tol, datums } => {
            egui::ComboBox::from_id_salt("pmi_gdt").selected_text(format!("{} {}", ch.symbol(), tr(ch.label()))).show_ui(ui, |ui| {
                for c in GdtChar::ALL {
                    ui.selectable_value(ch, c, format!("{} {}", c.symbol(), tr(c.label())));
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("Tolerância"));
                ui.text_edit_singleline(tol);
            });
            ui.horizontal(|ui| {
                ui.label(tr("Referências"));
                ui.text_edit_singleline(datums);
            });
        }
        _ => {
            ui.horizontal(|ui| {
                ui.label(tr("Texto (vazio = medido)"));
                ui.text_edit_singleline(&mut p.text);
            });
            ui.horizontal(|ui| {
                ui.label(tr("Tolerância +"));
                ui.add(egui::TextEdit::singleline(&mut p.tol_plus).desired_width(60.));
                ui.label("−");
                ui.add(egui::TextEdit::singleline(&mut p.tol_minus).desired_width(60.));
            });
        }
    }
    crate::editor::section(ui, "Posição do texto");
    ui.horizontal(|ui| {
        for (k, l) in ["X", "Y", "Z"].iter().enumerate() {
            ui.label(*l);
            { let __r = ui.add(egui::DragValue::new(&mut p.offset[k]).speed(0.5)); crate::ui_ext::wheel(ui, &__r, &mut p.offset[k]); __r };
        }
    });
    crate::editor::section(ui, "Orientação do texto");
    let mut on_plane = p.plane.is_some();
    ui.horizontal(|ui| {
        if ui.radio(!on_plane, tr("de frente para a tela")).clicked() {
            on_plane = false;
        }
        if ui.radio(on_plane, tr("deitado num plano")).clicked() {
            on_plane = true;
        }
    });
    if !on_plane {
        p.plane = None;
    } else {
        if p.plane.is_none() {
            p.plane = Some(([0., 0., 1.], [1., 0., 0.]));
        }
        ui.horizontal(|ui| {
            for (n, pl) in [("XY", ([0., 0., 1.], [1., 0., 0.])), ("XZ", ([0., -1., 0.], [1., 0., 0.])), ("YZ", ([1., 0., 0.], [0., 1., 0.]))] {
                if ui.selectable_label(p.plane == Some(pl), n).clicked() {
                    p.plane = Some(pl);
                }
            }
            if ui.add_enabled(seed_plane.is_some(), egui::Button::new(tr("⊙ Plano/face selecionado"))).on_disabled_hover_text(tr("clique numa face ou plano na viewport")).clicked() {
                p.plane = seed_plane;
            }
        });
    }
    ui.label(RichText::new(tr("Arraste o rótulo na viewport para mudar a posição.")).weak().small());
    ui.checkbox(&mut p.visible, tr("visível"));
    if seed_pts.len() >= p.kind.anchors() && ui.button(tr("⊙ Reancorar na seleção atual")).clicked() {
        p.anchors = seed_pts[..p.kind.anchors()].to_vec();
    }
    true
}
