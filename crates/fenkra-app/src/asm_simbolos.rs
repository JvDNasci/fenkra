// SPDX-License-Identifier: LGPL-2.1-or-later
//! Símbolos de restrição e junta no 3D (B245). A posição e o que cabe na tela vêm de
//! `fenkra_montagem::simbolos`; aqui ficam o desenho, a cota e o arco arrastáveis, o rótulo editável,
//! o realce das duas referências e o menu que oculta por tipo.

use crate::app::{fmt, App};
use crate::asm_ui::AsmState;
use crate::i18n::tr;
use crate::picking::Pick;
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use fenkra_core::Id;
use fenkra_montagem::simbolos::{arranjar, simbolos, Medida, Movimento, Simbolo};
use fenkra_montagem::*;
use glam::Vec3;
use std::collections::BTreeSet;

/// Distância mínima entre dois símbolos na tela (px).
const CELULA: f32 = 26.;
/// Acima disso, cotas e eixos só aparecem no item destacado.
const DETALHE_ATE: usize = 40;

/// O que a pessoa escolheu ver, e o que está acontecendo com os símbolos agora.
pub struct Exibicao {
    pub ligado: bool,
    pub ocultos: BTreeSet<&'static str>,
    /// símbolo sob o mouse: a linha da árvore acende junto
    pub hover: Option<Id>,
    /// rótulo em edição e o texto digitado
    editando: Option<(Id, String)>,
    /// cota em arrasto e o valor no começo
    arrasto: Option<(Id, f64)>,
}

impl Default for Exibicao {
    fn default() -> Self {
        Exibicao { ligado: true, ocultos: BTreeSet::new(), hover: None, editando: None, arrasto: None }
    }
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

fn menu_tipos(st: &mut AsmState, ui: &mut egui::Ui, rect: Rect) {
    let mut tipos: BTreeSet<&'static str> = st.asm.mates.iter().map(|m| m.kind.label()).collect();
    tipos.extend(st.asm.joints.iter().map(|j| j.kind.label()));
    if tipos.is_empty() {
        return;
    }
    let e = &mut st.simbolos;
    egui::Area::new(egui::Id::unique("asm_menu_simbolos")).order(egui::Order::Middle).fixed_pos(rect.right_bottom() + egui::vec2(-104., -62.)).show(ui.ctx(), |ui| {
        ui.menu_button(tr("Símbolos"), |ui| {
            ui.checkbox(&mut e.ligado, tr("Mostrar no modelo")).on_hover_text(tr("Desenha um símbolo em cada restrição e junta, preso à geometria"));
            ui.separator();
            for t in tipos {
                let mut ver = !e.ocultos.contains(t);
                if ui.add_enabled(e.ligado, egui::Checkbox::new(&mut ver, tr(t))).changed() {
                    if ver {
                        e.ocultos.remove(t);
                    } else {
                        e.ocultos.insert(t);
                    }
                }
            }
        })
        .response
        .on_hover_text(tr("Liga e desliga os símbolos por tipo"));
    });
}

/// Pílula de valor, clicável.
fn rotulo(ui: &mut egui::Ui, rect: Rect, centro: Pos2, txt: &str, cor: Color32, chave: (&str, u64)) -> egui::Response {
    let v = ui.visuals().clone();
    let g = ui.painter().layout_no_wrap(txt.to_string(), egui::FontId::proportional(12.), v.text_color());
    let caixa = Rect::from_center_size(centro, g.size() + egui::vec2(10., 5.));
    let p = ui.painter_at(rect);
    p.rect_filled(caixa, 4., v.window_fill.gamma_multiply(0.92));
    p.rect_stroke(caixa, 4., Stroke::new(1., cor), egui::StrokeKind::Inside);
    p.galley(caixa.min + egui::vec2(5., 2.5), g, v.text_color());
    ui.interact(caixa, egui::Id::unique(chave), egui::Sense::click())
}

/// Grava o valor: o rótulo aceita fórmula, a alça só número.
fn gravar_texto(st: &mut AsmState, id: Id, txt: String) {
    st.push_undo();
    if let Some(m) = st.asm.mates.iter_mut().find(|m| m.id == id) {
        if let MateKind::Distance(e) | MateKind::Angle(e) = &mut m.kind {
            *e = txt;
        }
    }
    st.solve();
}

fn arrastar(st: &mut AsmState, id: Id, r: &egui::Response, atual: f64, novo: impl FnOnce(f64) -> f64) {
    if r.drag_started() {
        st.push_undo();
        st.simbolos.arrasto = Some((id, atual));
    }
    if let Some((_, v0)) = st.simbolos.arrasto.filter(|a| a.0 == id && r.dragged()) {
        let v = novo(v0);
        if (v - atual).abs() > 1e-9 && st.asm.definir_medida(id, v) {
            // no arrasto só as posições; o resto (links, GDL) espera soltar
            st.report = solve::solve(&mut st.asm);
            st.dirty = true;
        }
    }
    if r.drag_stopped() && st.simbolos.arrasto.is_some_and(|a| a.0 == id) {
        st.simbolos.arrasto = None;
        st.solve();
    }
}

fn pega(ui: &mut egui::Ui, rect: Rect, at: Pos2, cor: Color32, chave: (&str, u64), editavel: bool, dica: &str) -> egui::Response {
    let sentido = if editavel { egui::Sense::drag() } else { egui::Sense::hover() };
    let r = ui.interact(Rect::from_center_size(at, egui::vec2(crate::handles::PEGA, crate::handles::PEGA)), egui::Id::unique(chave), sentido).on_hover_text(tr(dica));
    let quente = r.hovered() || r.dragged();
    ui.painter_at(rect).circle_filled(at, if quente { 6.5 } else { 4.5 }, if editavel { cor } else { cor.gamma_multiply(0.5) });
    r
}

fn cota(st: &mut AsmState, ui: &mut egui::Ui, rect: Rect, cam: &Camera, s: &Simbolo, cor: Color32) {
    let Some(Medida::Distancia { de, ate, sentido, valor, sinal }) = s.medida.clone() else { return };
    let (Some(a), Some(b)) = (cam.project(rect, v3(de)), cam.project(rect, v3(ate))) else { return };
    let p = ui.painter_at(rect);
    let traco = Stroke::new(1.4, cor);
    p.line_segment([a, b], traco);
    if (b - a).length() > 18. {
        crate::gizmo::arrow(&p, b - (b - a).normalized() * 12., b, cor);
        crate::gizmo::arrow(&p, a + (b - a).normalized() * 12., a, cor);
    }
    let Some((_, na_tela, ppm)) = crate::gizmo::screen_axis(cam, rect, v3(ate), v3(sentido)) else { return };
    let dica = if s.editavel { "Arraste para mudar a distância" } else { "O valor vem de uma fórmula: edite pelo rótulo" };
    let r = pega(ui, rect, b + na_tela * 14., cor, ("asm_cota", s.id.0), s.editavel, dica);
    let passo = crate::ui_ext::passo_3d(ui, ppm);
    let ponteiro = ui.input(|i| i.pointer.interact_pos());
    let inicio = ui.input(|i| i.pointer.press_origin());
    arrastar(st, s.id, &r, valor, |v0| {
        let t = match (ponteiro, inicio) {
            (Some(m), Some(o)) => ((m - o).dot(na_tela) / ppm) as f64,
            _ => 0.,
        };
        let v = ((v0 + t) / passo).round() * passo;
        if sinal {
            v
        } else {
            v.max(0.)
        }
    });
    if rotulo(ui, rect, a + (b - a) / 2., &format!("{} mm", fmt(valor)), cor, ("asm_rotulo", s.id.0)).on_hover_text(tr("Clique para digitar o valor")).clicked() {
        st.simbolos.editando = Some((s.id, fmt(valor)));
    }
}

fn arco(st: &mut AsmState, ui: &mut egui::Ui, rect: Rect, cam: &Camera, s: &Simbolo, cor: Color32) {
    let Some(Medida::Angulo { centro, u, v, valor }) = s.medida.clone() else { return };
    let n = cross(u, v);
    let n = if norm(n) > 1e-6 { unit(n) } else { any_perp(u) };
    let w = cross(n, u);
    let Some((_, _, ppm)) = crate::gizmo::screen_axis(cam, rect, v3(centro), v3(u)) else { return };
    let raio = (52. / ppm.max(1e-3)) as f64;
    let em = |g: f64| {
        let (c, sn) = (g.to_radians().cos(), g.to_radians().sin());
        [0, 1, 2].map(|i| centro[i] + raio * (u[i] * c + w[i] * sn))
    };
    let linha: Vec<Pos2> = (0..=32).filter_map(|k| cam.project(rect, v3(em(valor * k as f64 / 32.)))).collect();
    let (Some(c), Some(ponta), Some(meio), Some(zero)) = (cam.project(rect, v3(centro)), cam.project(rect, v3(em(valor))), cam.project(rect, v3(em(valor / 2.))), cam.project(rect, v3(em(0.)))) else { return };
    let p = ui.painter_at(rect);
    p.add(egui::Shape::line(linha, Stroke::new(1.6, cor)));
    p.line_segment([c, zero], Stroke::new(1., cor.gamma_multiply(0.6)));
    p.line_segment([c, ponta], Stroke::new(1., cor.gamma_multiply(0.6)));
    let dica = if s.editavel { "Arraste para mudar o ângulo" } else { "O valor vem de uma fórmula: edite pelo rótulo" };
    let r = pega(ui, rect, ponta, cor, ("asm_arco", s.id.0), s.editavel, dica);
    let ponteiro = ui.input(|i| i.pointer.interact_pos());
    let passo = ui.input(|i| crate::ui_ext::mult_teclas(i.modifiers));
    arrastar(st, s.id, &r, valor, |_| {
        let Some(m) = ponteiro else { return valor };
        let melhor = (0..=180).filter_map(|g| cam.project(rect, v3(em(g as f64))).map(|q| ((q - m).length(), g as f64))).min_by(|a, b| a.0.total_cmp(&b.0));
        let g = melhor.map_or(valor, |x| x.1);
        (g / passo).round() * passo
    });
    let fora = meio + (meio - c).normalized() * 16.;
    if rotulo(ui, rect, fora, &format!("{}°", fmt(valor)), cor, ("asm_rotulo", s.id.0)).on_hover_text(tr("Clique para digitar o valor")).clicked() {
        st.simbolos.editando = Some((s.id, fmt(valor)));
    }
}

/// Seta reta com ponta nos dois lados.
fn trilho(p: &egui::Painter, c: Pos2, d: Vec2, cor: Color32) {
    crate::gizmo::arrow(p, c, c + d * 22., cor);
    crate::gizmo::arrow(p, c, c - d * 22., cor);
}

/// Seta curva em volta do eixo.
fn volta(p: &egui::Painter, rect: Rect, cam: &Camera, o: [f64; 3], z: [f64; 3], x: [f64; 3], cor: Color32) {
    let Some((_, _, ppm)) = crate::gizmo::screen_axis(cam, rect, v3(o), v3(x)) else { return };
    let raio = (24. / ppm.max(1e-3)) as f64;
    let y = cross(z, x);
    let em = |g: f64| {
        let (c, s) = (g.to_radians().cos(), g.to_radians().sin());
        [0, 1, 2].map(|i| o[i] + raio * (x[i] * c + y[i] * s))
    };
    let linha: Vec<Pos2> = (0..=24).filter_map(|k| cam.project(rect, v3(em(k as f64 * 270. / 24.)))).collect();
    if let (Some(a), Some(b)) = (linha.get(linha.len().wrapping_sub(2)).copied(), linha.last().copied()) {
        crate::gizmo::arrow(p, a, b, cor);
    }
    p.add(egui::Shape::line(linha, Stroke::new(1.8, cor)));
}

fn eixos(ui: &egui::Ui, rect: Rect, cam: &Camera, s: &Simbolo, cor: Color32) {
    let Some((o, z, x)) = s.eixos else { return };
    let p = ui.painter_at(rect);
    let Some(c) = cam.project(rect, v3(o)) else { return };
    let tela = |d: [f64; 3]| crate::gizmo::screen_axis(cam, rect, v3(o), v3(d)).map(|t| t.1);
    let (Some(tz), Some(tx)) = (tela(z), tela(x)) else { return };
    let fraco = ui.visuals().weak_text_color();
    p.line_segment([c - tz * 34., c + tz * 34.], Stroke::new(1.2, cor.gamma_multiply(0.7)));
    p.line_segment([c, c + tx * 20.], Stroke::new(1.2, fraco));
    match s.movimento {
        Movimento::Curva => volta(&p, rect, cam, o, z, x, cor),
        Movimento::Reta => trilho(&p, c + tx * 8., tz, cor),
        Movimento::RetaCurva => {
            volta(&p, rect, cam, o, z, x, cor);
            trilho(&p, c + tx * 30., tz, cor);
        }
        Movimento::Esfera => {
            p.circle_stroke(c, 18., Stroke::new(1.8, cor));
        }
        Movimento::Plano => {
            trilho(&p, c, tx, cor);
            if let Some(ty) = tela(cross(z, x)) {
                trilho(&p, c, ty, cor);
            }
        }
        Movimento::Trajetoria | Movimento::Nenhum => {}
    }
}

/// Acende a face de uma referência.
fn realcar(app: &App, painter: &egui::Painter, rect: Rect, cam: &Camera, comp: Id, ponto: [f64; 3]) {
    let Some(r) = app.result.as_ref() else { return };
    let Some(body) = r.bodies.iter().position(|b| b.collector == comp) else { return };
    if let Some(face) = crate::picking::face_at(r, body, ponto) {
        let point = ponto.map(|v| v as f32);
        crate::picking::draw(painter, r, cam, rect, &Pick::Face { body, face, owner: comp, point, normal: [0., 0., 1.] }, true);
    }
}

fn editor_de_rotulo(st: &mut AsmState, ui: &mut egui::Ui, em: Pos2) {
    let Some((id, mut txt)) = st.simbolos.editando.clone() else { return };
    let mut fim = None;
    egui::Area::new(egui::Id::unique("asm_rotulo_edicao")).order(egui::Order::Foreground).fixed_pos(em).show(ui.ctx(), |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut txt).desired_width(90.));
            if !r.has_focus() && !r.lost_focus() {
                r.request_focus();
            }
            let (enter, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
            if esc {
                fim = Some(false);
            } else if enter || r.lost_focus() {
                fim = Some(true);
            }
        });
    });
    match fim {
        Some(true) => {
            st.simbolos.editando = None;
            gravar_texto(st, id, txt);
        }
        Some(false) => st.simbolos.editando = None,
        None => st.simbolos.editando = Some((id, txt)),
    }
}

/// Símbolos do modelo. Passar o mouse acende as duas referências e a linha da árvore; dois cliques
/// abrem a edição.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(st) = app.asm.as_mut() else { return };
    menu_tipos(st, ui, rect);
    if !st.simbolos.ligado || st.cmd.is_some() || st.editing_part.is_some() {
        st.simbolos.hover = None;
        return;
    }
    let todos = simbolos(&st.asm, &st.report.failed);
    let vis: Vec<(Simbolo, Pos2)> = todos.into_iter().filter(|s| !st.simbolos.ocultos.contains(s.tipo)).filter_map(|s| cam.project(rect, v3(s.centro())).filter(|p| rect.contains(*p)).map(|p| (s, p))).collect();
    let pontos: Vec<[f32; 2]> = vis.iter().map(|v| [v.1.x, v.1.y]).collect();
    let (sel, hover) = (st.sel, st.simbolos.hover);
    let destaque = |i: usize| Some(vis[i].0.id) == sel || Some(vis[i].0.id) == hover;
    let arr = arranjar(&pontos, destaque, CELULA);
    let detalhe = arr.mostrar.len() <= DETALHE_ATE;
    let v = ui.visuals().clone();
    let acc = crate::theme::accent(&v);
    let mut novo_hover = None;
    let mut abrir = None;
    let mut rotulo_em = None;
    for &i in &arr.mostrar {
        let (s, c) = &vis[i];
        let cor = if s.falhou {
            crate::theme::st_err()
        } else if s.suprimido {
            v.weak_text_color()
        } else if s.eixos.is_some() {
            crate::theme::st_ok()
        } else {
            acc
        };
        if detalhe || destaque(i) {
            eixos(ui, rect, cam, s, cor);
            match s.medida {
                Some(Medida::Distancia { .. }) => cota(st, ui, rect, cam, s, cor),
                Some(Medida::Angulo { .. }) => arco(st, ui, rect, cam, s, cor),
                None => {}
            }
        }
        if st.simbolos.editando.as_ref().is_some_and(|e| e.0 == s.id) {
            rotulo_em = Some(*c + egui::vec2(12., -12.));
        }
        let r = ui.interact(Rect::from_center_size(*c, egui::vec2(20., 20.)), egui::Id::unique(("asm_simbolo", s.id.0)), egui::Sense::click());
        let p = ui.painter_at(rect);
        let quente = r.hovered() || destaque(i);
        p.circle_filled(*c, if quente { 11. } else { 9. }, v.window_fill.gamma_multiply(0.9));
        p.circle_stroke(*c, if quente { 11. } else { 9. }, Stroke::new(if quente { 2. } else { 1.2 }, cor));
        p.text(*c, egui::Align2::CENTER_CENTER, s.icone, egui::FontId::proportional(12.), cor);
        if r.hovered() {
            novo_hover = Some(s.id);
        }
        if r.double_clicked() {
            abrir = Some(s.id);
        }
        let r = r.on_hover_ui(|ui| {
            ui.label(RichText::new(tr(s.tipo)).strong());
            ui.label(RichText::new(tr("2 cliques: editar")).weak().small());
        });
        drop(r);
    }
    let p = ui.painter_at(rect);
    for (i, n) in &arr.grupos {
        p.text(vis[*i].1 + egui::vec2(10., 9.), egui::Align2::LEFT_TOP, format!("+{n}"), egui::FontId::proportional(10.), v.weak_text_color());
    }
    let alvo = novo_hover.or(st.simbolos.arrasto.map(|a| a.0));
    st.simbolos.hover = alvo;
    if let Some(id) = abrir {
        st.sel = Some(id);
        st.props = false;
    }
    if let Some(em) = rotulo_em {
        editor_de_rotulo(st, ui, em);
    }
    if let Some(s) = alvo.and_then(|id| vis.iter().find(|x| x.0.id == id)).map(|x| &x.0) {
        let (a, b) = (s.comps.0, s.comps.1);
        realcar(app, &p, rect, cam, a, s.a);
        realcar(app, &p, rect, cam, b, s.b);
        for (ponto, cor) in [(s.a, acc), (s.b, crate::theme::st_ok())] {
            if let Some(q) = cam.project(rect, v3(ponto)) {
                p.circle_stroke(q, 7., Stroke::new(2., cor));
            }
        }
        app.set_tree_hover(Some(b));
    }
}
