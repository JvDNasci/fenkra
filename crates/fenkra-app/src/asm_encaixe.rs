// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pontos de encaixe na vista, com o comando de restrição ou junta aberto (B99).
//!
//! Passar o mouse numa face mostra os lugares em que dá para prender a peça, e o mais perto do cursor
//! mostra o sistema de eixos dele. Clicar perto de um deles usa o ponto em vez da face inteira. A tecla
//! L trava o ponto: dali em diante o clique usa o travado, mesmo com o cursor longe, o que ajuda quando
//! a peça de trás atrapalha a mira.
//!
//! A conta dos candidatos mora em `fenkra_montagem::encaixe`. O ponto entra no comando como um
//! elemento comum (plano no ponto ou eixo), então o solucionador não precisa saber dele.

use crate::app::App;
use crate::asm_ui::AsmState;
use crate::i18n::tr;
use crate::picking::Pick;
use crate::render::Camera;
use eframe::egui::{self, Rect, Stroke};
use fenkra_montagem::encaixe::{self, Candidato};
use fenkra_montagem::proposta::{Classe, Elemento};
use fenkra_montagem::MateRef;
use glam::Vec3;

/// Distância do cursor, em pontos, para um candidato acender.
const ALCANCE: f32 = 22.;
/// Comprimento dos eixos desenhados, em pontos.
const EIXO_PX: f32 = 44.;

#[derive(Default)]
pub struct Encaixe {
    /// corpo e face dos candidatos de `lista`
    face: Option<(usize, usize)>,
    pub lista: Vec<Candidato>,
    /// candidato sob o cursor
    pub quente: Option<(usize, Candidato)>,
    /// candidato preso pela tecla L
    pub travado: Option<(usize, Candidato)>,
}

impl Encaixe {
    /// O ponto que o próximo clique usa.
    pub fn escolhido(&self) -> Option<(usize, Candidato)> {
        self.travado.or(self.quente)
    }

    /// Tecla de travar: prende o candidato sob o cursor, ou solta o preso.
    pub fn alternar_trava(&mut self) {
        self.travado = if self.travado.is_some() { None } else { self.quente };
    }
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Candidato mais perto do cursor, dentro do alcance.
pub fn mais_perto(lista: &[Candidato], projetar: impl Fn(Vec3) -> Option<egui::Pos2>, cursor: egui::Pos2) -> Option<Candidato> {
    lista.iter().filter_map(|c| projetar(v3(c.p)).map(|s| (s.distance(cursor), *c))).filter(|(d, _)| *d <= ALCANCE).min_by(|a, b| a.0.total_cmp(&b.0)).map(|x| x.1)
}

/// O elemento do comando a partir do ponto escolhido, no sistema da peça.
pub fn elemento(st: &AsmState, corpo: usize, c: &Candidato) -> Option<Elemento> {
    let comp = st.comp_de(corpo)?;
    let pl = st.asm.node(comp)?.placement;
    Some(Elemento::novo(MateRef { comp, geom: encaixe::geom_local(c, &pl), pick: None }, Classe::Face))
}

fn eixos(p: &egui::Painter, rect: Rect, cam: &Camera, c: &Candidato, v: &egui::Visuals) {
    let Some(o) = cam.project(rect, v3(c.p)) else { return };
    let y = [c.z[1] * c.x[2] - c.z[2] * c.x[1], c.z[2] * c.x[0] - c.z[0] * c.x[2], c.z[0] * c.x[1] - c.z[1] * c.x[0]];
    // o comprimento na tela é fixo: a escala em mm vem de um passo pequeno na direção do eixo
    for (d, cor, nome) in [(c.x, crate::theme::st_err(), "X"), (y, crate::theme::st_ok(), "Y"), (c.z, crate::theme::accent(v), "Z")] {
        let Some((_, na_tela, _)) = crate::gizmo::screen_axis(cam, rect, v3(c.p), v3(d)) else { continue };
        let ponta = o + na_tela * EIXO_PX;
        crate::gizmo::arrow(p, o, ponta, cor);
        p.text(ponta + na_tela * 8., egui::Align2::CENTER_CENTER, nome, egui::FontId::proportional(11.), cor);
    }
}

/// Atualiza os candidatos da face sob o cursor e desenha. Chamado com a viewport.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let hover = app.hover_pick;
    let vista = app.result.as_ref();
    let Some(st) = app.asm.as_mut() else { return };
    let Some(pen) = st.cmd.as_mut() else { return };
    if pen.c.completo() {
        return;
    }
    let e = &mut pen.encaixe;
    let cursor = ui.ctx().input(|i| i.pointer.latest_pos()).filter(|p| rect.contains(*p));
    let face = match hover {
        Some(Pick::Face { body, face, .. }) => Some((body, face)),
        _ => None,
    };
    if face != e.face {
        e.face = face;
        e.lista = face.and_then(|(b, f)| vista.and_then(|r| r.bodies.get(b)).map(|c| encaixe::candidatos(&c.mesh, f))).unwrap_or_default();
    }
    e.quente = match (face, cursor) {
        (Some((b, _)), Some(c)) => mais_perto(&e.lista, |q| cam.project(rect, q), c).map(|x| (b, x)),
        _ => None,
    };
    let v = ui.visuals().clone();
    let painter = ui.painter_at(rect);
    let cor = crate::theme::accent(&v);
    for c in &e.lista {
        let Some(s) = cam.project(rect, v3(c.p)) else { continue };
        painter.circle_stroke(s, 4., Stroke::new(1.5, cor));
    }
    if let Some((_, c)) = e.quente.filter(|_| e.travado.is_none()) {
        eixos(&painter, rect, cam, &c, &v);
        if let Some(s) = cam.project(rect, v3(c.p)) {
            painter.circle_filled(s, 5., cor);
            painter.text(s + egui::vec2(12., 14.), egui::Align2::LEFT_TOP, format!("{}  ·  {}", tr(c.tipo.label()), tr("L trava")), egui::FontId::proportional(12.), cor);
        }
    }
    if let Some((_, c)) = e.travado {
        eixos(&painter, rect, cam, &c, &v);
        if let Some(s) = cam.project(rect, v3(c.p)) {
            let aviso = crate::theme::st_warn();
            painter.circle_filled(s, 6., aviso);
            painter.circle_stroke(s, 10., Stroke::new(2., aviso));
            painter.text(s + egui::vec2(14., 14.), egui::Align2::LEFT_TOP, format!("{} {}", tr(c.tipo.label()), tr("travado: clique para usar, L solta")), egui::FontId::proportional(12.), aviso);
        }
    }
}

/// Tecla L com o comando aberto.
pub fn teclas(app: &mut App, ctx: &egui::Context) {
    let Some(pen) = app.asm.as_mut().and_then(|s| s.cmd.as_mut()) else { return };
    if !ctx.egui_wants_keyboard_input() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::L)) {
        pen.encaixe.alternar_trava();
    }
}

/// `FENKRA_DEMO_ASM=encaixe`: candidatos da face de cima da placa, com o centro do furo aceso.
pub fn demo(st: &mut AsmState) {
    st.dirty = true;
    st.refresh();
    let Some(vista) = st.view.as_ref() else { return };
    let Some(lista) = vista.bodies.get(1).and_then(|b| (0..b.mesh.face_ranges.len()).map(|f| encaixe::candidatos(&b.mesh, f)).max_by_key(Vec::len)) else { return };
    let Some(pen) = st.cmd.as_mut() else { return };
    let quente = lista.iter().find(|c| c.tipo == encaixe::TipoEncaixe::CentroFuro).or(lista.first()).copied();
    pen.encaixe.travado = quente.map(|c| (1, c));
    pen.encaixe.lista = lista;
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_montagem::encaixe::TipoEncaixe;

    fn cand(p: [f64; 3]) -> Candidato {
        Candidato { tipo: TipoEncaixe::Canto, p, z: [0., 0., 1.], x: [1., 0., 0.] }
    }

    #[test]
    fn acende_o_mais_perto_dentro_do_alcance() {
        let l = [cand([0., 0., 0.]), cand([15., 0., 0.]), cand([300., 0., 0.])];
        let projetar = |q: Vec3| Some(egui::pos2(q.x, q.y));
        assert_eq!(mais_perto(&l, projetar, egui::pos2(11., 0.)).unwrap().p[0], 15.);
        assert!(mais_perto(&l, projetar, egui::pos2(100., 100.)).is_none());
    }

    #[test]
    fn trava_prende_o_que_estava_aceso_e_sobrevive_ao_cursor_sair() {
        let mut e = Encaixe { quente: Some((3, cand([1., 2., 3.]))), ..Default::default() };
        e.alternar_trava();
        e.quente = None;
        assert_eq!(e.escolhido().unwrap().1.p, [1., 2., 3.]);
        e.alternar_trava();
        assert!(e.escolhido().is_none(), "a tecla solta");
    }
}
