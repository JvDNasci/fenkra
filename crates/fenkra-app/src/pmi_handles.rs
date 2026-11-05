// SPDX-License-Identifier: LGPL-2.1-or-later
//! Alças 3D da anotação selecionada (B269): afastamento da linha de cota, extensão das linhas de
//! apoio e posição do texto, mais o encaixe dos vértices da chamada. As contas de encaixe
//! (1 mm, 15°, eixos do modelo) estão em `fenkra_core::pmi_snap`; aqui ficam o desenho das
//! alças, o cursor de cada uma, o arraste e o desfazer.
//!
//! O arraste só emite pedidos (`Req`); quem aplica é `apply`, depois do laço de desenho. Cada
//! gesto guarda o estado de antes ao começar e entrega um passo de desfazer ao terminar (a peça
//! usa o histórico do documento; a montagem, a pilha própria).

use crate::i18n::tr;
use crate::pmi_draw::HL;
use crate::render::Camera;
use eframe::egui::{self, Pos2, Rect, Stroke};
use fenkra_core::pmi::Pmi;
use fenkra_core::{pmi_snap, Id};
use glam::Vec3;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// O que uma alça pede para mudar.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Req {
    /// novo deslocamento da linha de cota (vetor do modelo)
    Offset(Id, [f64; 3]),
    /// novo avanço das linhas de apoio (mm)
    Ext(Id, f64),
    /// nova posição do texto ao longo da linha (mm a partir do meio)
    Text(Id, f64),
}

/// Resultado de um quadro: pedidos e marcas do gesto.
#[derive(Default)]
pub struct Out {
    pub reqs: Vec<Req>,
    pub begin: bool,
    pub end: bool,
}

/// Estado de antes de um gesto (peça: o documento; montagem: a montagem).
pub enum Antes {
    Peca(fenkra_core::Document),
    Montagem(Box<fenkra_montagem::Assembly>),
}

static ANTES: Mutex<Option<Antes>> = Mutex::new(None);
static MUDOU: AtomicBool = AtomicBool::new(false);

/// Foto do estado atual, para desfazer depois.
pub fn snapshot(app: &crate::app::App) -> Option<Antes> {
    if app.in_asm() {
        app.asm.as_ref().map(|s| Antes::Montagem(Box::new(s.asm.clone())))
    } else {
        Some(Antes::Peca(app.open.doc.clone()))
    }
}

/// Entrega a foto como passo de desfazer, se algo mudou desde o último `clear_mark`.
pub fn commit(app: &mut crate::app::App, antes: Option<Antes>) {
    if !MUDOU.swap(false, Ordering::Relaxed) {
        return;
    }
    match antes {
        Some(Antes::Peca(d)) => app.push_undo_pub(d),
        Some(Antes::Montagem(a)) => {
            if let Some(st) = app.asm.as_mut() {
                st.undo.push(*a);
                if st.undo.len() > 100 {
                    st.undo.remove(0);
                }
                st.redo.clear();
                st.modified = true;
            }
        }
        None => {}
    }
}

/// Marca que o gesto em curso mudou a anotação.
pub fn mark() {
    MUDOU.store(true, Ordering::Relaxed);
}

pub fn clear_mark() {
    MUDOU.store(false, Ordering::Relaxed);
}

/// Começa um gesto de alça: guarda o estado de antes para o desfazer.
pub fn begin(app: &crate::app::App) {
    *ANTES.lock().unwrap_or_else(|e| e.into_inner()) = snapshot(app);
    clear_mark();
}

/// Termina o gesto: se algo mudou, vira um passo de desfazer.
pub fn end(app: &mut crate::app::App) {
    let antes = ANTES.lock().unwrap_or_else(|e| e.into_inner()).take();
    commit(app, antes);
}

/// Aplica os pedidos de um quadro à lista de anotações.
pub fn apply(list: &mut [Pmi], reqs: &[Req]) -> bool {
    let mut mudou = false;
    for r in reqs {
        let (id, f): (Id, Box<dyn Fn(&mut Pmi) -> bool>) = match *r {
            Req::Offset(id, o) => (id, Box::new(move |p| std::mem::replace(&mut p.offset, o) != o)),
            Req::Ext(id, e) => (id, Box::new(move |p| std::mem::replace(&mut p.style.ext_over, e) != e)),
            Req::Text(id, t) => (id, Box::new(move |p| std::mem::replace(&mut p.style.text_along, t) != t)),
        };
        if let Some(p) = list.iter_mut().find(|p| p.id == id) {
            mudou |= f(p);
        }
    }
    if mudou {
        MUDOU.store(true, Ordering::Relaxed);
    }
    mudou
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

fn arr(v: Vec3) -> [f64; 3] {
    [v.x as f64, v.y as f64, v.z as f64]
}

/// Ponto do plano de normal `n` por `through` sob o cursor `m`.
fn plane_hit(cam: &Camera, rect: Rect, m: Pos2, through: Vec3, n: Vec3) -> Option<Vec3> {
    let (o, dir) = cam.ray(rect, m);
    let den = dir.dot(n);
    (den.abs() > 1e-6).then(|| o + dir * ((through - o).dot(n) / den))
}

/// Lado do quadrado da pega em px (alvo de clique confortável).
const PEGA: f32 = 20.;

/// Uma alça: desenha o símbolo, devolve a resposta do arraste. `forma` 0 quadrado, 1 losango, 2 círculo.
fn alca(ui: &mut egui::Ui, painter: &egui::Painter, at: Pos2, id: egui::Id, forma: u8, cursor: egui::CursorIcon, dica: &str) -> egui::Response {
    let r = ui.interact(Rect::from_center_size(at, egui::vec2(PEGA, PEGA)), id, egui::Sense::drag());
    let ativo = r.hovered() || r.dragged();
    let (fill, stroke) = (if ativo { HL } else { ui.visuals().window_fill() }, Stroke::new(2., HL));
    let s = if ativo { 6. } else { 5. };
    match forma {
        0 => {
            painter.rect_filled(Rect::from_center_size(at, egui::vec2(2. * s, 2. * s)), 1., fill);
            painter.rect_stroke(Rect::from_center_size(at, egui::vec2(2. * s, 2. * s)), 1., stroke, egui::StrokeKind::Middle);
        }
        1 => {
            let pts = vec![at + egui::vec2(0., -s - 1.), at + egui::vec2(s + 1., 0.), at + egui::vec2(0., s + 1.), at + egui::vec2(-s - 1., 0.)];
            painter.add(egui::Shape::convex_polygon(pts, fill, stroke));
        }
        _ => {
            painter.circle_filled(at, s, fill);
            painter.circle_stroke(at, s, stroke);
        }
    }
    if ativo {
        ui.ctx().set_cursor_icon(cursor);
    }
    r.on_hover_text(tr(dica))
}

fn valor(painter: &egui::Painter, at: Pos2, txt: String) {
    painter.text(at + egui::vec2(14., -18.), egui::Align2::LEFT_BOTTOM, txt, egui::FontId::proportional(12.), HL);
}

/// Alças de uma cota de linha (linear, ordenada, entre círculos) selecionada. `lbl` é o retângulo
/// do texto na tela. Os pedidos saem em `out`.
pub fn linear(ui: &mut egui::Ui, painter: &egui::Painter, cam: &Camera, rect: Rect, pm: &Pmi, a: &[[f64; 3]], lbl: Rect, out: &mut Out) {
    if a.len() < 2 {
        return;
    }
    let (p0, p1, off) = (v3(a[0]), v3(a[1]), v3(pm.offset));
    let (q0, q1) = (p0 + off, p1 + off);
    let (Some(s0), Some(t0), Some(t1)) = (cam.project(rect, p0), cam.project(rect, q0), cam.project(rect, q1)) else { return };
    let shift = ui.input(|i| i.modifiers.shift);
    // plano de trabalho: o da anotação ou o de frente para a tela
    let n = match pm.plane {
        Some((n, _)) => v3(n).normalize_or_zero(),
        None => -cam.ray(rect, rect.center()).1,
    };
    // 1. afastamento da linha de cota: alça a um quarto da linha
    let at = t0 + (t1 - t0) * 0.25;
    let r = alca(ui, painter, at, egui::Id::unique(("pmi_alca_off", pm.id.0)), 0, egui::CursorIcon::ResizeVertical, "arraste para afastar a linha de cota; encaixa em eixos e em 1 mm (Shift solta)");
    if r.drag_started() {
        out.begin = true;
    }
    if let (true, Some(m)) = (r.dragged(), r.interact_pointer_pos()) {
        if let Some(hit) = plane_hit(cam, rect, m, p0, n) {
            if let Some(o) = pmi_snap::offset_from_hit(a[0], a[1], arr(hit), arr(n), shift) {
                out.reqs.push(Req::Offset(pm.id, o));
                valor(painter, m, format!("{} {} mm", tr("afastamento"), fenkra_core::pmi::num(o.iter().map(|x| x * x).sum::<f64>().sqrt())));
            }
        }
    }
    if r.drag_stopped() {
        out.end = true;
    }
    // 2. extensão das linhas de apoio: losango na ponta da linha de apoio da primeira âncora
    let e3 = off.normalize_or_zero();
    if e3.length() > 0.5 {
        let fim = match cam.project(rect, q0 + e3 * pm.style.ext_over as f32) {
            Some(x) if pm.style.ext_over > 0. => x,
            _ => t0 + (t0 - s0).normalized() * 4.,
        };
        let r = alca(ui, painter, fim, egui::Id::unique(("pmi_alca_ext", pm.id.0)), 1, egui::CursorIcon::ResizeNeSw, "arraste para estender as linhas de apoio além da linha de cota (1 mm; Shift solta)");
        if r.drag_started() {
            out.begin = true;
        }
        if let (true, Some(m)) = (r.dragged(), r.interact_pointer_pos()) {
            if let Some(hit) = plane_hit(cam, rect, m, q0, n) {
                let e = pmi_snap::ext_from_hit(a[0], pm.offset, arr(hit), shift);
                out.reqs.push(Req::Ext(pm.id, e));
                valor(painter, m, format!("{} {} mm", tr("extensão"), fenkra_core::pmi::num(e)));
            }
        }
        if r.drag_stopped() {
            out.end = true;
        }
    }
    // 3. posição do texto ao longo da linha: círculo à direita do texto
    let r = alca(ui, painter, lbl.right_center() + egui::vec2(14., 0.), egui::Id::unique(("pmi_alca_txt", pm.id.0)), 2, egui::CursorIcon::ResizeHorizontal, "arraste para deslizar o texto ao longo da linha de cota (1 mm; Shift solta)");
    if r.drag_started() {
        out.begin = true;
    }
    if let (true, Some(m)) = (r.dragged(), r.interact_pointer_pos()) {
        if let Some(hit) = plane_hit(cam, rect, m, (q0 + q1) * 0.5, n) {
            let t = pmi_snap::text_along_from_hit(a[0], a[1], pm.offset, arr(hit), shift);
            out.reqs.push(Req::Text(pm.id, t));
            valor(painter, m, format!("{} {:+} mm", tr("texto"), t));
        }
    }
    if r.drag_stopped() {
        out.end = true;
    }
}

/// Vértice da chamada arrastado até `hit` (no plano da vista): encaixa em 1 mm e 15°.
pub fn snap_via(prev: [f64; 3], hit: Vec3, n: Vec3, shift: bool) -> [f64; 3] {
    pmi_snap::snap_polar(prev, arr(hit), arr(n), shift)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::pmi::{PmiKind, PmiStyle};

    fn cota() -> Pmi {
        Pmi { id: Id(7), name: "c".into(), kind: PmiKind::Linear, anchors: vec![[0.; 3], [40., 0., 0.]], offset: [0., 0., 12.], text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: PmiStyle::default(), folder: String::new(), targets: vec![] }
    }

    #[test]
    fn pedidos_das_alcas_gravam_afastamento_extensao_e_texto() {
        let mut l = vec![cota()];
        assert!(apply(&mut l, &[Req::Offset(Id(7), [0., 0., 20.]), Req::Ext(Id(7), 3.), Req::Text(Id(7), -5.)]));
        assert_eq!((l[0].offset, l[0].style.ext_over, l[0].style.text_along), ([0., 0., 20.], 3., -5.));
        // pedido repetido não conta como mudança (não polui o desfazer)
        assert!(!apply(&mut l, &[Req::Offset(Id(7), [0., 0., 20.]), Req::Ext(Id(7), 3.)]));
        // id inexistente é ignorado
        assert!(!apply(&mut l, &[Req::Ext(Id(99), 1.)]));
        // quem mudou marca o gesto para virar passo de desfazer
        assert!(MUDOU.load(Ordering::Relaxed));
    }

    #[test]
    fn vertice_da_chamada_encaixa_em_15_graus() {
        let p = snap_via([0.; 3], Vec3::new(10.4, 0., 2.7), Vec3::Y, false);
        let ang = p[2].atan2(p[0]).to_degrees().abs();
        assert!(((ang / 15.).round() * 15. - ang).abs() < 1e-6, "{ang}");
        assert!(((p[0] * p[0] + p[2] * p[2]).sqrt() - 11.).abs() < 1e-9);
    }
}
