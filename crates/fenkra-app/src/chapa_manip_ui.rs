// SPDX-License-Identifier: LGPL-2.1-or-later
//! Interface 3D da chapa (B243 e B249): alças de ângulo, raio, largura e recuos, e a linha de dobra
//! desenhada na face com face fixa. As regras estão em `chapa_manip.rs`; aqui só se desenha e se lê
//! o mouse. Cores só do tema.

use crate::app::{fmt, App};
use crate::chapa_manip::{self, Forma, Manip};
use crate::i18n::tr;
use crate::picking::{Filter, Pick};
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use fenkra_core::*;
use fenkra_solidos::chapa_linha;
use fenkra_solidos::geo3d::{self, V3};
use glam::Vec3;

fn v3(p: V3) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

fn d3(p: Vec3) -> V3 {
    [p.x as f64, p.y as f64, p.z as f64]
}

/// Estado do desenho da linha enquanto o comando está sem linha.
#[derive(Clone, Debug, Default)]
struct Desenho {
    /// ponto da face fixa e normal dela
    fixo: Option<(V3, V3)>,
    /// primeira ponta já marcada
    primeiro: Option<V3>,
}

fn cores(ui: &egui::Ui) -> (Color32, Color32) {
    (crate::theme::accent(ui.visuals()), crate::theme::st_warn())
}

/// Chamado pela viewport com o comando aberto (`id`): alças de chapa e linha de dobra.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let Some(kind) = app.open.doc.feature(id).map(|f| f.kind.clone()) else { return };
    let linha = matches!(&kind, FeatureKind::SmBend(b) if b.sketch.is_none()) || matches!(&kind, FeatureKind::SmJoggle(j) if j.plane.is_none()) || matches!(&kind, FeatureKind::SmLouver(_));
    if linha {
        if !linha_overlay(app, ui, rect, cam, id, &kind) {
            // ainda sem linha: só o desenho dela
            return;
        }
    }
    let Some(r) = app.result.as_ref() else { return };
    let vars = app.vars();
    let lista = chapa_manip::manipuladores(&app.open.doc, r, id, &vars);
    for (i, m) in lista.into_iter().enumerate() {
        alca(app, ui, rect, cam, id, i, m, &vars);
    }
}

/// Uma alça: seta arrastável ou arco de ângulo, com rótulo do valor e clique para digitar.
#[allow(clippy::too_many_arguments)]
fn alca(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id, nth: usize, m: Manip, vars: &std::collections::HashMap<String, f64>) {
    let Ok(val) = fenkra_core::expr::eval_str(&m.expr, vars) else { return };
    let (normal, quente) = cores(ui);
    let painter = ui.painter_at(rect);
    let hid = egui::Id::new(("chapa_alca", id.0, nth));
    let (ponta, rot_pos) = match &m.forma {
        Forma::Seta { base, dir, k } => {
            let tip = geo3d::add(*base, geo3d::mul(*dir, val / *k as f64));
            let (Some(sb), Some(st)) = (cam.project(rect, v3(*base)), cam.project(rect, v3(tip))) else { return };
            let Some(s1) = cam.project(rect, v3(geo3d::add(*base, *dir))) else { return };
            let ppm = (s1 - sb).length().max(1e-3);
            let eixo = (s1 - sb) / ppm;
            let hr = ui.interact(Rect::from_center_size(st, egui::vec2(crate::handles::PEGA, crate::handles::PEGA)), hid, egui::Sense::click_and_drag());
            let c = if hr.hovered() || hr.dragged() { quente } else { normal };
            painter.line_segment([sb, st], Stroke::new(2.5, c));
            painter.circle_filled(sb, 3.5, c);
            painter.circle_stroke(st, crate::handles::PEGA / 2., Stroke::new(1.5, c));
            painter.circle_filled(st, 4., c);
            if hr.dragged() {
                if let Some(mouse) = ui.input(|i| i.pointer.interact_pos()) {
                    let t = (mouse - sb).dot(eixo) / ppm;
                    let passo = crate::ui_ext::passo_3d(ui, ppm) * *k;
                    let v = ((t as f64 * *k) / passo).round() * passo;
                    if v >= 0. {
                        chapa_manip::aplicar(&mut app.open.doc, id, m.campo, v, vars);
                    }
                }
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
            (st, st + egui::vec2(16., -22.))
        }
        Forma::Arco { centro, .. } => {
            let max_a = match &m.forma {
                Forma::Arco { max, .. } => *max,
                _ => 180.,
            };
            let a = val.clamp(0., max_a);
            let pts: Vec<Pos2> = (0..=40).filter_map(|i| chapa_manip::ponto_do_arco(&m.forma, a * i as f64 / 40.).and_then(|p| cam.project(rect, v3(p)))).collect();
            let Some(tip) = chapa_manip::ponto_do_arco(&m.forma, a).and_then(|p| cam.project(rect, v3(p))) else { return };
            let Some(c0) = cam.project(rect, v3(*centro)) else { return };
            let hr = ui.interact(Rect::from_center_size(tip, egui::vec2(crate::handles::PEGA, crate::handles::PEGA)), hid, egui::Sense::click_and_drag());
            let c = if hr.hovered() || hr.dragged() { quente } else { normal };
            painter.add(egui::Shape::line(pts, Stroke::new(2.5, c)));
            painter.line_segment([c0, tip], Stroke::new(1., c.gamma_multiply(0.5)));
            painter.circle_stroke(tip, 9., Stroke::new(1.5, c));
            painter.circle_filled(tip, 4., c);
            if hr.dragged() {
                if let Some(mouse) = ui.input(|i| i.pointer.interact_pos()) {
                    let proj = |p: V3| cam.project(rect, v3(p)).map(|q| [q.x, q.y]);
                    if let Some(ang) = chapa_manip::angulo_mais_perto(&m.forma, [mouse.x, mouse.y], &proj) {
                        let passo = if ui.input(|i| i.modifiers.shift) { 5. } else { 1. };
                        let v = ((ang / passo).round() * passo).clamp(1., max_a);
                        chapa_manip::aplicar(&mut app.open.doc, id, m.campo, v, vars);
                    }
                }
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
            (tip, tip + egui::vec2(14., -10.))
        }
    };
    // rótulo com o valor (e a expressão, se tiver variável); clique para digitar no próprio 3D
    let numerico = m.expr.trim().replace(',', ".").parse::<f64>().is_ok();
    let txt = if numerico { format!("{} {}{}", tr(m.campo.rotulo()), fmt(val), m.campo.unidade()) } else { format!("{} = {}{}", m.expr.trim(), fmt(val), m.campo.unidade()) };
    let fundo = ui.visuals().extreme_bg_color;
    let tcol = ui.visuals().text_color();
    let g = painter.layout_no_wrap(txt, egui::FontId::proportional(12.), tcol);
    let lr = Rect::from_min_size(rot_pos, g.size() + egui::vec2(10., 6.));
    painter.rect_filled(lr, 4., fundo);
    painter.rect_stroke(lr, 4., Stroke::new(1., normal), egui::StrokeKind::Inside);
    painter.galley(lr.min + egui::vec2(5., 3.), g, tcol);
    let resp = ui.interact(lr, egui::Id::new(("chapa_rotulo", id.0, nth)), egui::Sense::click());
    let chave = egui::Id::new(("chapa_edita", id.0, nth));
    let mut editando: Option<String> = ui.ctx().data(|d| d.get_temp(chave));
    if resp.clicked() {
        editando = Some(m.expr.clone());
    }
    if let Some(mut texto) = editando.take() {
        let mut fim: Option<bool> = None;
        egui::Area::new(egui::Id::new(("chapa_area", id.0, nth))).fixed_pos(lr.left_bottom() + egui::vec2(0., 4.)).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(egui::RichText::new(tr(m.campo.rotulo())).small());
                let r = ui.add(egui::TextEdit::singleline(&mut texto).desired_width(110.));
                r.request_focus();
                let ok = fenkra_core::expr::eval_str(&texto, vars).is_ok();
                if !ok {
                    ui.label(egui::RichText::new(tr("expressão inválida")).small().color(crate::theme::st_err()));
                }
                let (enter, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
                if enter && ok {
                    fim = Some(true);
                } else if esc {
                    fim = Some(false);
                }
            });
        });
        match fim {
            Some(true) => {
                let _ = chapa_manip::digitar(&mut app.open.doc, id, m.campo, &texto, vars);
                ui.ctx().data_mut(|d| d.remove::<String>(chave));
            }
            Some(false) => {
                ui.ctx().data_mut(|d| d.remove::<String>(chave));
            }
            None => {
                ui.ctx().data_mut(|d| d.insert_temp(chave, texto));
            }
        }
    }
    let _ = ponta;
}

/// Candidatos de encaixe da face: vértices e meios de aresta que caem no plano dela.
fn candidatos(app: &App, body: usize, origem: V3, n: V3) -> Vec<V3> {
    let Some(r) = app.result.as_ref() else { return vec![] };
    let Some(b) = r.bodies.get(body) else { return vec![] };
    let no_plano = |p: V3| geo3d::dot(geo3d::sub(p, origem), n).abs() < 1e-3;
    let mut out: Vec<V3> = b.mesh.vertices.chunks_exact(3).map(|p| [p[0] as f64, p[1] as f64, p[2] as f64]).filter(|p| no_plano(*p)).collect();
    for e in 0..b.mesh.edge_ranges.len() {
        let l = crate::picking::edge_polyline(r, body, e);
        if l.len() >= 2 && l.iter().all(|p| no_plano(*p)) {
            out.push(l[l.len() / 2]);
        }
    }
    out
}

/// Ponto do plano (origem, n) sob o mouse, encaixado nos candidatos a até 10 px.
fn ponto_no_plano(ui: &egui::Ui, rect: Rect, cam: &Camera, mouse: Pos2, origem: V3, n: V3, cands: &[V3]) -> Option<(V3, bool)> {
    let (o, d) = cam.ray(rect, mouse);
    let (o, d) = (d3(o), d3(d));
    let den = geo3d::dot(d, n);
    if den.abs() < 1e-9 {
        return None;
    }
    let t = geo3d::dot(geo3d::sub(origem, o), n) / den;
    let p = geo3d::add(o, geo3d::mul(d, t));
    let _ = ui;
    let mut melhor: Option<(f32, V3)> = None;
    for c in cands {
        if let Some(s) = cam.project(rect, v3(*c)) {
            let dist = (s - mouse).length();
            if dist < 10. && melhor.is_none_or(|(bd, _)| dist < bd) {
                melhor = Some((dist, *c));
            }
        }
    }
    Some(match melhor {
        Some((_, c)) => (c, true),
        None => (p, false),
    })
}

fn seta(painter: &egui::Painter, a: Pos2, b: Pos2, cor: Color32) {
    painter.line_segment([a, b], Stroke::new(2., cor));
    let d = (b - a).normalized();
    let n = egui::vec2(-d.y, d.x);
    painter.add(egui::Shape::convex_polygon(vec![b + d * 9., b + n * 5., b - n * 5.], cor, Stroke::NONE));
}

/// Desenha e edita a linha de dobra do comando. Devolve `true` quando a linha existe (as demais
/// alças só aparecem depois dela).
fn linha_overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id, kind: &FeatureKind) -> bool {
    let (a, b, flip) = match kind {
        FeatureKind::SmBend(x) => (x.a, x.b, x.flip),
        FeatureKind::SmJoggle(x) => (x.a, x.b, x.flip),
        FeatureKind::SmLouver(x) => (x.a, x.b, x.flip),
        _ => return false,
    };
    let (normal_cor, quente) = cores(ui);
    let painter = ui.painter_at(rect);
    let chave = egui::Id::new(("chapa_linha_desenho", id.0));
    let mut est: Desenho = ui.ctx().data(|d| d.get_temp(chave)).unwrap_or_default();

    if geo3d::dist(a, b) < 1e-6 {
        // sem linha ainda: 1) clicar a face fixa, 2) clicar as duas pontas da linha
        let mouse = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
        let dica;
        match est.fixo {
            None => {
                dica = tr("Clique na face fixa: o lado dela fica parado e o outro se move.");
                if let (Some(m), Some(r)) = (mouse, app.result.as_ref()) {
                    let f = Filter { faces: true, edges: false, vertices: false, planes: false, hidden_env: false, exclude_sketch: None, fantasma: Some(id) };
                    if let Some(p @ Pick::Face { point, normal, .. }) = crate::picking::pick(r, &app.open.doc, cam, rect, m, &f) {
                        crate::picking::draw(&painter, r, cam, rect, &p, false);
                        if ui.input(|i| i.pointer.primary_clicked()) {
                            est.fixo = Some((point.map(|x| x as f64), normal.map(|x| x as f64)));
                        }
                    }
                }
            }
            Some((fixo, n)) => {
                dica = tr("Clique nas duas pontas da linha de dobra na face (encaixa em vértices e meios de aresta). Esc recomeça.");
                let corpo = app.result.as_ref().and_then(|r| r.bodies.iter().position(|b| b.collector == app.open.doc.feature(id).map(|f| f.target).unwrap_or(Id(0))));
                let cands = corpo.map(|c| candidatos(app, c, fixo, n)).unwrap_or_default();
                let atual = mouse.and_then(|m| ponto_no_plano(ui, rect, cam, m, fixo, n, &cands));
                if let Some((p, preso)) = atual {
                    if let Some(sp) = cam.project(rect, v3(p)) {
                        painter.circle_stroke(sp, if preso { 8. } else { 5. }, Stroke::new(1.5, quente));
                    }
                    if let Some(p0) = est.primeiro {
                        if let (Some(s0), Some(s1)) = (cam.project(rect, v3(p0)), cam.project(rect, v3(p))) {
                            painter.extend(egui::Shape::dashed_line(&[s0, s1], Stroke::new(2., quente), 8., 5.));
                        }
                    }
                    if ui.input(|i| i.pointer.primary_clicked()) {
                        match est.primeiro {
                            None => est.primeiro = Some(p),
                            Some(p0) if geo3d::dist(p0, p) > 1e-3 => {
                                let novo_flip = chapa_linha::flip_para_fixo(p0, p, n, fixo).unwrap_or(false);
                                escreve_linha(&mut app.open.doc, id, p0, p, Some(novo_flip));
                                est = Desenho::default();
                            }
                            _ => {}
                        }
                    }
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    est = Desenho::default();
                }
            }
        }
        let g = painter.layout_no_wrap(dica.to_string(), egui::FontId::proportional(13.), ui.visuals().text_color());
        let r = Rect::from_center_size(rect.center_top() + egui::vec2(0., 40.), g.size() + egui::vec2(16., 8.));
        painter.rect_filled(r, 4., ui.visuals().extreme_bg_color);
        painter.rect_stroke(r, 4., Stroke::new(1., normal_cor), egui::StrokeKind::Inside);
        painter.galley(r.min + egui::vec2(8., 4.), g, ui.visuals().text_color());
        ui.ctx().data_mut(|d| d.insert_temp(chave, est));
        return false;
    }

    // linha pronta: metades coloridas, linha tracejada com setas de sentido, pontas arrastáveis
    let Some(r) = app.result.as_ref() else { return true };
    let alvo = app.open.doc.feature(id).map(|f| f.target).unwrap_or(Id(0));
    let mid = geo3d::lerp(a, b, 0.5);
    let Some(n) = chapa_manip::normal_perto(r, alvo, mid) else { return true };
    // triângulos da face da linha, recortados pelo lado fixo e pelo móvel
    if let Some(body) = r.bodies.iter().find(|x| x.collector == alvo) {
        if let Some((fi, _)) = crate::picking::face_e_distancia(&body.mesh, mid) {
            let m = &body.mesh;
            let i0 = m.face_ranges.get(fi).map(|x| *x as usize).unwrap_or(0);
            let i1 = m.face_ranges.get(fi + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
            let (cor_fixa, cor_movel) = (crate::theme::st_ok().gamma_multiply(0.35), normal_cor.gamma_multiply(0.4));
            for t in m.indices[i0..i1].chunks_exact(3) {
                let tri: Vec<V3> = t.iter().map(|i| [m.positions[*i as usize * 3] as f64, m.positions[*i as usize * 3 + 1] as f64, m.positions[*i as usize * 3 + 2] as f64]).collect();
                let (fixa, movel) = chapa_linha::recorta_metades(&tri, a, b, n, flip);
                for (poly, cor) in [(fixa, cor_fixa), (movel, cor_movel)] {
                    let pts: Vec<Pos2> = poly.iter().filter_map(|p| cam.project(rect, v3(*p))).collect();
                    if pts.len() >= 3 && pts.len() == poly.len() {
                        painter.add(egui::Shape::convex_polygon(pts, cor, Stroke::NONE));
                    }
                }
            }
        }
    }
    let (Some(sa), Some(sb)) = (cam.project(rect, v3(a)), cam.project(rect, v3(b))) else { return true };
    painter.extend(egui::Shape::dashed_line(&[sa, sb], Stroke::new(2.5, quente), 10., 5.));
    for (p, q) in chapa_linha::setas_de_sentido(a, b, n, flip, (geo3d::dist(a, b) * 0.12).clamp(4., 20.)) {
        if let (Some(p), Some(q)) = (cam.project(rect, v3(p)), cam.project(rect, v3(q))) {
            seta(&painter, p, q, normal_cor);
        }
    }
    // pontas arrastáveis: ficam sempre no plano da face
    let cands = r.bodies.iter().position(|x| x.collector == alvo).map(|c| candidatos(app, c, mid, n)).unwrap_or_default();
    let mut novo: Option<(V3, V3)> = None;
    for (ponta, (sp, p)) in [(sa, a), (sb, b)].into_iter().enumerate() {
        let hr = ui.interact(Rect::from_center_size(sp, egui::vec2(crate::handles::PEGA, crate::handles::PEGA)), egui::Id::new(("chapa_ponta", id.0, ponta)), egui::Sense::drag());
        let c = if hr.hovered() || hr.dragged() { quente } else { normal_cor };
        painter.circle_filled(sp, 5., c);
        painter.circle_stroke(sp, 9., Stroke::new(1.5, c));
        let _ = p;
        if hr.dragged() {
            if let Some(m) = ui.input(|i| i.pointer.interact_pos()) {
                if let Some((q, _)) = ponto_no_plano(ui, rect, cam, m, mid, n, &cands) {
                    novo = Some(chapa_linha::move_ponta(a, b, ponta as u8, q, mid, n));
                }
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
    }
    if let Some((na, nb)) = novo {
        escreve_linha(&mut app.open.doc, id, na, nb, None);
    }
    // inverter o lado móvel e, na dobra, a posição da linha
    let pos = Pos2::new((sa.x + sb.x) / 2., (sa.y + sb.y) / 2.) + egui::vec2(14., 14.);
    egui::Area::new(egui::Id::new(("chapa_linha_menu", id.0))).fixed_pos(pos).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.small_button("⇄").on_hover_text(tr("Trocar o lado que se move (a face fixa passa a ser a outra).")).clicked() {
                    escreve_linha(&mut app.open.doc, id, a, b, Some(!flip));
                }
                if ui.small_button("✎").on_hover_text(tr("Desenhar a linha de novo (escolhe a face fixa outra vez).")).clicked() {
                    // volta ao estado sem linha: as duas pontas iguais
                    escreve_linha(&mut app.open.doc, id, a, a, None);
                }
            });
            if let FeatureKind::SmBend(bd) = kind {
                for (i, nome) in fenkra_core::SM_BEND_POSITIONS.iter().enumerate() {
                    if ui.selectable_label(bd.position as usize == i, tr(nome)).clicked() {
                        if let Some(f) = app.open.doc.feature_mut(id) {
                            if let FeatureKind::SmBend(x) = &mut f.kind {
                                x.position = i as u8;
                            }
                        }
                    }
                }
            }
        });
    });
    true
}

/// Grava a linha (e o lado móvel, se vier) no comando.
fn escreve_linha(doc: &mut Document, id: Id, a: V3, b: V3, flip: Option<bool>) {
    let Some(f) = doc.feature_mut(id) else { return };
    match &mut f.kind {
        FeatureKind::SmBend(x) => {
            (x.a, x.b) = (a, b);
            if let Some(v) = flip {
                x.flip = v;
            }
        }
        FeatureKind::SmJoggle(x) => {
            (x.a, x.b) = (a, b);
            if let Some(v) = flip {
                x.flip = v;
            }
        }
        FeatureKind::SmLouver(x) => {
            (x.a, x.b) = (a, b);
            if let Some(v) = flip {
                x.flip = v;
            }
        }
        _ => {}
    }
}
