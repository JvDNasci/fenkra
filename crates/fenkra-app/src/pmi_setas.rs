// SPDX-License-Identifier: LGPL-2.1-or-later
//! Setas de notas e legendas (B368): ponto de saída na borda da caixa (visível e arrastável),
//! toque duplo na seta, propriedades de cada seta (alvos, ancoragem, ponta) e várias setas de uma
//! vez com Ctrl e Shift. Os dados e as contas ficam em `fenkra_core::pmi_leader`.

use crate::i18n::tr;
use crate::pmi_draw::{Pen, HL};
use crate::pmi_handles::{Out, Req};
use crate::render::Camera;
use eframe::egui::{self, Pos2, Rect, Stroke};
use fenkra_core::pmi::{Pmi, PmiArrow, PmiKind, PmiTarget};
use fenkra_core::Id;
use glam::Vec3;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Seta pedida pelo toque duplo no 3D: o painel da nota abre com ela expandida.
static ABERTA: Mutex<Option<(Id, usize)>> = Mutex::new(None);
/// Ctrl ou Shift estavam apertados neste quadro (o comando "Adicionar setas" espera pelo Enter).
static MODIFICADOR: AtomicBool = AtomicBool::new(false);

pub fn pedir_abrir(id: Id, k: usize) {
    *ABERTA.lock().unwrap_or_else(|e| e.into_inner()) = Some((id, k));
}

fn consumir(id: Id) -> Option<usize> {
    let mut g = ABERTA.lock().unwrap_or_else(|e| e.into_inner());
    match *g {
        Some((i, k)) if i == id => {
            *g = None;
            Some(k)
        }
        _ => None,
    }
}

pub fn marcar_modificador(ui: &egui::Ui) {
    let m = ui.input(|i| i.modifiers.command || i.modifiers.shift);
    MODIFICADOR.store(m, Ordering::Relaxed);
}

pub fn modificador() -> bool {
    MODIFICADOR.load(Ordering::Relaxed)
}

pub fn tem_setas(k: &PmiKind) -> bool {
    matches!(k, PmiKind::Note | PmiKind::FeatureNote { .. })
}

/// Caminho de cada seta na tela: vértices intermediários e, por último, a ponta.
pub fn rotas(cam: &Camera, rect: Rect, pm: &Pmi, a: &[[f64; 3]]) -> Vec<Vec<Pos2>> {
    let v3 = |p: [f64; 3]| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
    (0..a.len())
        .filter_map(|k| {
            let mut v: Vec<Pos2> = pm.style.leader_via.get(k).map(|w| w.iter().filter_map(|q| cam.project(rect, v3(*q))).collect()).unwrap_or_default();
            v.push(cam.project(rect, v3(a[k]))?);
            Some(v)
        })
        .collect()
}

fn dist_seg(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let l2 = ab.length_sq();
    let t = if l2 < 1e-6 { 0. } else { ((p - a).dot(ab) / l2).clamp(0., 1.) };
    (p - (a + ab * t)).length()
}

/// Alça do início de cada seta (nota selecionada) e toque duplo na seta. `lr` é a caixa do texto
/// sem girar, como a chamada a vê.
#[allow(clippy::too_many_arguments)]
pub fn interagir(ui: &mut egui::Ui, painter: &egui::Painter, pen: &Pen, cam: &Camera, rect: Rect, pm: &Pmi, a: &[[f64; 3]], lr: Rect, selecionada: bool, out: &mut Out, abrir: &mut Option<(Id, usize)>) {
    if !pen.leader_on || a.is_empty() {
        return;
    }
    let rs = rotas(cam, rect, pm, a);
    let duplo = ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
    let ptr = ui.input(|i| i.pointer.interact_pos()).filter(|q| rect.contains(*q));
    for (k, rt) in rs.iter().enumerate() {
        let lado = fenkra_core::pmi_leader::side(pm, k);
        let (att, knee, _) = pen.leader_start(lr, rt[0], lado);
        if duplo {
            if let Some(q) = ptr {
                let mut pts = vec![att, knee];
                pts.extend(rt.iter().copied());
                if pts.windows(2).any(|w| dist_seg(q, w[0], w[1]) < 6.) && !pen.outer(lr).contains(q) {
                    *abrir = Some((pm.id, k));
                }
            }
        }
        if !selecionada {
            continue;
        }
        let h = ui.interact(Rect::from_center_size(att, egui::vec2(16., 16.)), egui::Id::unique(("pmi_seta_ini", pm.id.0, k)), egui::Sense::click_and_drag());
        if h.hovered() || h.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        let caixa = Rect::from_center_size(att, egui::vec2(9., 9.));
        painter.rect_filled(caixa, 2., ui.visuals().extreme_bg_color);
        painter.rect_stroke(caixa, 2., Stroke::new(2., HL), egui::StrokeKind::Middle);
        let h = h.on_hover_text(tr("início da seta: arraste para a esquerda, direita, topo ou base da nota; botão direito escolhe o lado"));
        if h.drag_started() {
            out.begin = true;
        }
        if h.drag_stopped() {
            out.end = true;
        }
        if let (true, Some(m)) = (h.dragged(), h.interact_pointer_pos()) {
            // prévia: os quatro pontos possíveis, o escolhido em destaque
            let local = if pen.turned() { pen.xf_inv(m, lr.center()) } else { m };
            let novo = fenkra_core::pmi_leader::side_nearest([lr.left() as f64, lr.top() as f64, lr.right() as f64, lr.bottom() as f64], [local.x as f64, local.y as f64]);
            for s in 1..=4u8 {
                let (p, _, _) = pen.leader_start(lr, rt[0], s);
                painter.circle_stroke(p, if s == novo { 6. } else { 3.5 }, Stroke::new(if s == novo { 2.5 } else { 1.2 }, HL));
            }
            out.reqs.push(Req::Lado(pm.id, k, novo));
        }
        h.context_menu(|ui| {
            for (l, n) in LADOS {
                if ui.selectable_label(l == lado, tr(n)).clicked() {
                    out.begin = true;
                    out.reqs.push(Req::Lado(pm.id, k, l));
                    out.end = true;
                    ui.close();
                }
            }
        });
    }
}

const LADOS: [(u8, &str); 5] = [(0, "Automático"), (1, "Esquerda"), (2, "Direita"), (3, "Topo"), (4, "Base")];
const PONTAS: [(PmiArrow, &str); 6] = [(PmiArrow::Padrao, "Padrão"), (PmiArrow::Cheia, "Cheia"), (PmiArrow::Aberta, "Aberta"), (PmiArrow::Ponto, "Ponto"), (PmiArrow::Barra, "Barra"), (PmiArrow::Nenhuma, "Nenhuma")];

/// O que o painel pede ao app depois de editar as setas.
#[derive(Default)]
pub struct Pedido {
    /// remover a seta k
    pub remover: Option<usize>,
    /// apontar a seta k para a seleção atual (as demais escolhidas viram setas novas)
    pub apontar: Option<usize>,
    /// entrar no modo "adicionar setas" (Ctrl/Shift somam, Enter conclui)
    pub adicionar: bool,
}

/// Submenu "Setas" da edição da nota: uma seção por seta, com alvo, ancoragem e ponta.
pub fn secao(ui: &mut egui::Ui, p: &mut Pmi, alvos: &[String], com_selecao: usize) -> Pedido {
    let mut ped = Pedido::default();
    let abrir = consumir(p.id);
    egui::CollapsingHeader::new(format!("{} ({})", tr("Setas"), p.anchors.len())).default_open(true).id_salt(("pmi_setas", p.id.0)).show(ui, |ui| {
        for k in 0..p.anchors.len() {
            let mut cab = egui::CollapsingHeader::new(format!("{} {}", tr("Seta"), k + 1)).id_salt(("pmi_seta", p.id.0, k)).default_open(p.anchors.len() == 1);
            if abrir == Some(k) {
                cab = cab.open(Some(true));
            }
            cab.show(ui, |ui| {
                ui.label(format!("{}: {}", tr("Alvo"), alvos.get(k).cloned().unwrap_or_else(|| tr("ponto livre").to_string())));
                let mut lado = fenkra_core::pmi_leader::side(p, k);
                ui.horizontal_wrapped(|ui| {
                    ui.label(tr("Ancoragem na nota")).on_hover_text(tr("De que lado da nota sai esta seta. Cada seta tem a sua; também se arrasta o quadrado no início da seta, no 3D."));
                    for (l, n) in LADOS {
                        ui.selectable_value(&mut lado, l, tr(n));
                    }
                });
                fenkra_core::pmi_leader::set_side(p, k, lado);
                let mut ponta = fenkra_core::pmi_leader::arrow(p, k);
                ui.horizontal(|ui| {
                    ui.label(tr("Ponta")).on_hover_text(tr("Formato da ponta desta seta; Padrão usa a do estilo da nota."));
                    egui::ComboBox::from_id_salt(("pmi_seta_ponta", p.id.0, k)).selected_text(tr(PONTAS.iter().find(|x| x.0 == ponta).map_or("Padrão", |x| x.1))).show_ui(ui, |ui| {
                        for (a, n) in PONTAS {
                            ui.selectable_value(&mut ponta, a, tr(n));
                        }
                    });
                });
                fenkra_core::pmi_leader::set_arrow(p, k, ponta);
                ui.horizontal(|ui| {
                    if ui.add_enabled(com_selecao > 0, egui::Button::new(tr("⊙ Apontar para a seleção"))).on_hover_text(tr("Escolha no 3D uma ou várias faces ou arestas (Ctrl e Shift somam): a primeira troca o alvo desta seta, as outras viram setas novas.")).clicked() {
                        ped.apontar = Some(k);
                    }
                    if p.anchors.len() > 1 && ui.button(tr("Remover esta seta")).clicked() {
                        ped.remover = Some(k);
                    }
                });
            });
        }
        if ui.button(tr("＋ Adicionar setas…")).on_hover_text(tr("Clique faces ou arestas no 3D; com Ctrl ou Shift você soma várias e conclui com Enter.")).clicked() {
            ped.adicionar = true;
        }
    });
    ped
}

/// Texto do alvo de cada seta para a lista do painel.
pub fn rotulos(p: &Pmi, nome: &dyn Fn(&PmiTarget) -> String) -> Vec<String> {
    (0..p.anchors.len()).map(|k| p.targets.get(k).copied().flatten().map_or_else(|| tr("ponto livre").to_string(), |t| nome(&t))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toque_na_linha_usa_distancia_ao_segmento() {
        assert!(dist_seg(Pos2::new(5., 3.), Pos2::new(0., 0.), Pos2::new(10., 0.)) < 3.1);
        assert!(dist_seg(Pos2::new(5., 30.), Pos2::new(0., 0.), Pos2::new(10., 0.)) > 29.);
        assert_eq!(LADOS.len(), 5);
    }
}
