// SPDX-License-Identifier: LGPL-2.1-or-later
//! Manipulador da montagem (B261): setas, quadrados e anéis com referência mundo, peça ou face, pivô
//! escolhido, passo, colisão com escopo e respeito às restrições, numa barrinha junto dele. Cada alça
//! tem um rótulo que aceita o valor digitado. As regras moram em `fenkra_montagem::manipulador`.

use crate::app::App;
use crate::asm_ui::AsmState;
use crate::i18n::tr;
use crate::picking::Pick;
use crate::render::Camera;
use eframe::egui::{self, Pos2, Rect, RichText, Stroke};
use fenkra_core::Id;
use fenkra_montagem::manipulador::{eixos, girado, levar, movido, ponto_de_giro, trava, Pivo, Referencia, Trava};
use fenkra_montagem::solve::Liberdade;
use fenkra_montagem::*;
use glam::Vec3;

/// O que a colisão olha enquanto a peça se move.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum Escopo {
    #[default]
    Todas,
    Selecionadas,
}

/// O próximo clique no 3D define isto.
#[derive(Clone, Copy, PartialEq)]
enum Espera {
    Face,
    Pivo,
}

/// Alça com rótulo em edição.
#[derive(Clone, Copy, PartialEq)]
enum Alca {
    Mover(usize),
    Girar(usize),
}

/// Opções e estado do manipulador.
pub struct Manip {
    pub referencia: Referencia,
    pub pivo: Pivo,
    /// passo do arrasto (0: pelo zoom)
    pub passo: f64,
    pub passo_giro: f64,
    pub escopo: Escopo,
    /// normal da face de referência, no sistema da peça
    pub face: Option<[f64; 3]>,
    espera: Option<Espera>,
    digitando: Option<(Alca, String)>,
    /// deslocamento do arrasto em curso, nos eixos do manipulador
    off: [f64; 3],
    /// eixos no começo do arrasto (girar a peça não pode girar a régua)
    eixos_arrasto: Option<[[f64; 3]; 3]>,
    /// última conta das alças: a chave é o que muda a resposta
    cache: Option<(Chave, Option<Liberdade>)>,
}

type Chave = (Id, [[f64; 3]; 3], Placement);

impl Default for Manip {
    fn default() -> Self {
        Manip { referencia: Referencia::Mundo, pivo: Pivo::Centro, passo: 0., passo_giro: 1., escopo: Escopo::Todas, face: None, espera: None, digitando: None, off: [0.; 3], eixos_arrasto: None, cache: None }
    }
}

impl Manip {
    /// As restrições mudaram: a conta das alças não vale mais.
    pub fn esquecer(&mut self) {
        self.cache = None;
    }
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

fn tudo_livre() -> Liberdade {
    Liberdade { mover: [true; 3], girar: [true; 3], pivo: [None; 3] }
}

/// Que alças aparecem. Fixa não tem; com "respeitar restrições" desligado a peça é levada à mão.
fn liberdade(st: &mut AsmState, id: Id, e: [[f64; 3]; 3], centro: [f64; 3], p: Placement) -> Option<Liberdade> {
    let restrita = !st.asm.mates.is_empty() || !st.asm.joints.is_empty() || !st.asm.rigid.is_empty();
    if st.asm.node(id).is_some_and(|n| n.fixed) {
        return None;
    }
    if !st.respect || !restrita {
        return Some(tudo_livre());
    }
    let chave: Chave = (id, e, p);
    match &st.manip.cache {
        // no arrasto a peça anda: as alças são as do começo
        Some((c, l)) if *c == chave || st.drag_start.is_some() => *l,
        _ => {
            let l = solve::liberdade(&st.asm, id, e, centro);
            st.manip.cache = Some((chave, l));
            l
        }
    }
}

fn motivo(t: Trava) -> String {
    match t {
        Trava::Fixa => tr("Peça fixa").to_string(),
        Trava::Grupo => tr("Presa a uma peça fixa pela fixação conjunta").to_string(),
        Trava::Restricoes(n) => format!("{n} {}", tr("restrição(ões) prendem a peça")),
    }
}

fn cadeado(ui: &egui::Ui, rect: Rect, c: Pos2, t: Trava) {
    let v = ui.visuals();
    let p = ui.painter_at(rect);
    let g = p.layout_no_wrap(format!("🔒 {}", motivo(t)), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.5)), v.text_color());
    let caixa = Rect::from_min_size(c + egui::vec2(14., -10.), g.size() + egui::vec2(12., 6.));
    p.rect_filled(caixa, 4., v.window_fill.gamma_multiply(0.92));
    p.rect_stroke(caixa, 4., Stroke::new(1., crate::theme::st_warn()), egui::StrokeKind::Inside);
    p.galley(caixa.min + egui::vec2(6., 3.), g, v.text_color());
    p.circle_stroke(c, 5., Stroke::new(1.5, crate::theme::st_warn()));
}

/// Barrinha junto do manipulador.
fn barra(st: &mut AsmState, ui: &mut egui::Ui, rect: Rect, c: Pos2) {
    let m = &mut st.manip;
    let lugar = Pos2::new((c.x + 120.).clamp(rect.left() + 8., (rect.right() - 340.).max(rect.left() + 8.)), (c.y - 160.).clamp(rect.top() + 8., rect.bottom() - 90.));
    let mut mudou = false;
    egui::Area::new(egui::Id::unique("asm_barra_manipulador")).order(egui::Order::Middle).fixed_pos(lugar).show(ui.ctx(), |ui| {
        egui::Frame::popup(ui.style()).inner_margin(6.).show(ui, |ui| {
            ui.set_max_width(330.);
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Eixos")).small());
                for (r, nome, dica) in [(Referencia::Mundo, "Mundo", "Eixos da montagem"), (Referencia::Peca, "Peça", "Eixos da própria peça"), (Referencia::Face, "Face", "Z na normal de uma face: o próximo clique no 3D escolhe")] {
                    if ui.selectable_label(m.referencia == r, tr(nome)).on_hover_text(tr(dica)).clicked() {
                        m.referencia = r;
                        m.espera = (r == Referencia::Face).then_some(Espera::Face);
                        mudou = true;
                    }
                }
                ui.separator();
                ui.label(RichText::new(tr("Pivô")).small());
                for (p, nome, dica) in [(Pivo::Centro, "Centro", "Centro da caixa envolvente, ou o eixo que as restrições mandam"), (Pivo::OrigemDaPeca, "Origem", "Origem da peça")] {
                    if ui.selectable_label(m.pivo == p, tr(nome)).on_hover_text(tr(dica)).clicked() {
                        m.pivo = p;
                        m.espera = None;
                    }
                }
                let ponto = matches!(m.pivo, Pivo::Ponto(_)) || m.espera == Some(Espera::Pivo);
                if ui.selectable_label(ponto, tr("Ponto")).on_hover_text(tr("O próximo clique no 3D escolhe o pivô")).clicked() {
                    m.espera = Some(Espera::Pivo);
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Passo")).small());
                ui.add(crate::ui_ext::dv(&mut m.passo).range(0.0..=1000.0).speed(0.1).suffix(" mm").dica("Passo do arrasto; zero segue o zoom"));
                ui.add(crate::ui_ext::dv(&mut m.passo_giro).range(0.0..=90.0).speed(0.1).suffix("°").dica("Passo do giro"));
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut st.respect, tr("Respeitar restrições")).on_hover_text(tr("Só o que as restrições deixam acompanha o mouse"));
                ui.checkbox(&mut st.stop_collision, tr("Colisão")).on_hover_text(tr("Para no primeiro contato entre sólidos"));
                if st.stop_collision {
                    for (e, nome, dica) in [(Escopo::Todas, "Todas", "Contra todas as peças visíveis"), (Escopo::Selecionadas, "Selecionadas", "Só contra as peças da seleção")] {
                        if ui.selectable_label(m.escopo == e, tr(nome)).on_hover_text(tr(dica)).clicked() {
                            m.escopo = e;
                        }
                    }
                }
            });
            if let Some(e) = m.espera {
                let t = if e == Espera::Face { "Clique na face de referência" } else { "Clique no pivô" };
                ui.label(RichText::new(tr(t)).small().color(crate::theme::st_warn()));
            }
        });
    });
    if mudou {
        st.manip.esquecer();
    }
}

/// Cliques que a barrinha pediu: consome a seleção. Devolve se usou.
pub fn apos_viewport(app: &mut App) -> bool {
    let Some(st) = app.asm.as_mut() else { return false };
    let Some(e) = st.manip.espera else { return false };
    let Some(p) = app.picks.last().copied() else { return false };
    app.picks.clear();
    let Some(id) = st.sel else { return true };
    let Some(pl) = st.asm.node(id).map(|n| n.placement) else { return true };
    match e {
        Espera::Face => match st.mate_ref(p).map(|r| r.geom) {
            Ok(Geom::Plane { n, .. } | Geom::Axis { d: n, .. }) if st.mate_ref(p).is_ok_and(|r| r.comp == id) => {
                st.manip.face = Some(n);
                st.manip.espera = None;
                st.manip.esquecer();
            }
            Ok(Geom::Plane { n, .. } | Geom::Axis { d: n, .. }) => {
                // face de outra peça: a direção vale no mundo, e fica guardada no sistema desta
                let dono = st.mate_ref(p).ok().and_then(|r| st.asm.lugar(r.comp));
                if let Some(q) = dono {
                    st.manip.face = Some(pl.dir_to_local(q.dir(n)));
                }
                st.manip.espera = None;
                st.manip.esquecer();
            }
            _ => {}
        },
        Espera::Pivo => {
            let mundo = match p {
                Pick::Vertex { pos, .. } => Some(pos),
                Pick::Edge { mid, .. } => Some(mid),
                Pick::Face { point, .. } => Some(point),
                _ => None,
            };
            if let Some(q) = mundo {
                st.manip.pivo = Pivo::Ponto(pl.to_local(q.map(|v| v as f64)));
                st.manip.espera = None;
            }
        }
    }
    true
}

/// A peça vai para `alvo` de uma vez: é o valor digitado no rótulo.
fn aplicar(st: &mut AsmState, id: Id, alvo: Placement, escopo: &[Id]) {
    let mut copia = st.asm.clone();
    if st.respect {
        levar(&mut copia, id, alvo);
    } else if let Some(n) = copia.node_mut(id) {
        n.placement = alvo;
    }
    if colide(st, &copia, id, escopo) {
        st.contact = true;
        return;
    }
    st.push_undo();
    st.asm = copia;
    if st.respect {
        st.solve();
    }
    st.dirty = true;
}

fn colide(st: &mut AsmState, a: &Assembly, id: Id, picks: &[Id]) -> bool {
    if !st.stop_collision {
        return false;
    }
    let dir = st.dir();
    let alvos = (st.manip.escopo == Escopo::Selecionadas).then_some(picks);
    view::collides_com(a, id, dir.as_deref(), &mut st.cache, 0.01, alvos)
}

/// Pílula clicável de uma alça.
fn pilula(ui: &mut egui::Ui, rect: Rect, at: Pos2, txt: &str, cor: egui::Color32, chave: (&str, u64, usize), dica: &str) -> bool {
    let v = ui.visuals().clone();
    let g = ui.painter().layout_no_wrap(txt.to_string(), egui::FontId::proportional(crate::acessibilidade::fonte_3d(11.)), v.text_color());
    let caixa = Rect::from_center_size(at, g.size() + egui::vec2(8., 4.));
    let r = ui.interact(caixa, egui::Id::unique(chave), egui::Sense::click()).on_hover_text(tr(dica));
    let p = ui.painter_at(rect);
    p.rect_filled(caixa, 3., v.window_fill.gamma_multiply(if r.hovered() { 1. } else { 0.8 }));
    p.rect_stroke(caixa, 3., Stroke::new(1., cor), egui::StrokeKind::Inside);
    p.galley(caixa.min + egui::vec2(4., 2.), g, v.text_color());
    r.clicked()
}

/// Caixa de texto do rótulo: Enter aplica, Esc desiste. Aceita "12", "1 in", "37°" e fórmula simples.
fn digitar(st: &mut AsmState, ui: &mut egui::Ui, em: Pos2) -> Option<(Alca, f64)> {
    let (alca, mut txt) = st.manip.digitando.clone()?;
    let angulo = matches!(alca, Alca::Girar(_));
    let mut fim = None;
    egui::Area::new(egui::Id::unique("asm_digitar_manipulador")).order(egui::Order::Foreground).fixed_pos(em).show(ui.ctx(), |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut txt).desired_width(70.).hint_text(if angulo { "°" } else { "mm" }));
                if !r.has_focus() && !r.lost_focus() {
                    r.request_focus();
                }
                let (enter, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
                if esc || (r.lost_focus() && !enter) {
                    fim = Some(None);
                } else if enter {
                    fim = Some(crate::ui_ext::parse_com_unidade(&txt, angulo));
                }
            });
        });
    });
    match fim {
        None => {
            st.manip.digitando = Some((alca, txt));
            None
        }
        Some(v) => {
            st.manip.digitando = None;
            v.map(|x| (alca, x))
        }
    }
}

/// Manipulador do componente escolhido.
pub fn desenhar(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let picks_comps: Vec<Id>;
    let Some(st) = app.asm.as_mut() else { return };
    if st.view_only || st.cmd.is_some() {
        return;
    }
    let Some(id) = st.sel else { return };
    let Some(n) = st.asm.node(id) else { return };
    if matches!(n.kind, NodeKind::Group) {
        return;
    }
    let p = n.placement;
    let caixa = app.result.as_ref().and_then(|r| view::centro_caixa(r.bodies.iter().filter(|b| b.collector == id).map(|b| &b.mesh)));
    let centro = caixa.unwrap_or(p.origin);
    let c = v3(centro);
    if st.explode_mode {
        let off = st.live.offset(id);
        if let Some(v) = crate::gizmo::translate_gizmo(ui, rect, cam, id.0 ^ 0x5eed, c, [Vec3::X, Vec3::Y, Vec3::Z], off, [false; 3]) {
            st.exibicao.seta.get_or_insert((id, off));
            st.live.set_offset(id, v);
            st.dirty = true;
        }
        // soltou a seta: o que ela andou vira um passo
        if ui.input(|i| !i.pointer.any_down()) {
            if let Some((peca, inicio)) = st.exibicao.seta.take() {
                let escolhidas = st.comps_of(&app.picks).into_iter().chain(st.multi.iter().copied()).collect();
                crate::asm_exibicao::soltar_seta(st, peca, inicio, escolhidas);
            }
        }
        return;
    }
    picks_comps = st.comps_of(&app.picks).into_iter().chain(st.multi.iter().copied()).collect();
    let Some(cs) = cam.project(rect, c) else { return };
    let e = st.manip.eixos_arrasto.unwrap_or_else(|| eixos(st.manip.referencia, &p, st.manip.face));
    let lib = liberdade(st, id, e, centro, p);
    if let Some(t) = trava(&st.asm, id, lib.as_ref()) {
        cadeado(ui, rect, cs, t);
        if t != Trava::Fixa {
            barra(st, ui, rect, cs);
        }
        return;
    }
    let lib = lib.unwrap_or_else(tudo_livre);
    barra(st, ui, rect, cs);
    let ax = e.map(v3);
    let inicio = st.drag_start.unwrap_or(p);
    let mut cand: Option<Placement> = None;
    if let Some(v) = crate::gizmo::translate_alcas(ui, rect, cam, id.0, c, ax, st.manip.off, [false; 3], lib.mover) {
        let passo = st.manip.passo;
        let d = v.map(|x| if passo > 0. { (x / passo).round() * passo } else { x });
        st.manip.off = d;
        cand = Some(movido(&inicio, &e, d));
    }
    // anéis: cada um passa pelo pivô do eixo dele
    let ppm = crate::gizmo::screen_axis(cam, rect, c, cam.basis().0).map_or(1., |t| t.2.max(1e-3));
    let rad = 95. / ppm;
    let mut pontas = [None; 3];
    for k in (0..3).filter(|k| lib.girar[*k]) {
        let ponto = st.drag_center.filter(|_| st.rot_drag.is_some_and(|r| r.0 == k)).unwrap_or_else(|| ponto_de_giro(st.manip.pivo, &inicio, centro, lib.pivo[k]));
        let o = v3(ponto);
        let dentro = (ax[(k + 1) % 3] + ax[(k + 2) % 3]).normalize();
        let src = o + dentro * rad;
        pontas[k] = Some((src, ponto));
        let atual = st.rot_drag.filter(|r| r.0 == k).map(|r| r.1).unwrap_or(0.);
        if let Some(v) = crate::gizmo::rotate_arc_col(ui, rect, cam, id.0.wrapping_mul(31).wrapping_add(k as u64 + 7), o, ax[k], src, atual, false, crate::gizmo::AXIS_COL[k]) {
            let passo = st.manip.passo_giro;
            let v = if passo > 0. { (v / passo).round() * passo } else { v };
            st.drag_center.get_or_insert(ponto);
            st.rot_drag = Some((k, v));
            cand = Some(girado(&inicio, e[k], v, ponto));
        }
    }
    // rótulos: o valor digitado anda ou gira sem passar pelo painel
    let nomes = ["X", "Y", "Z"];
    let mut abrir = None;
    let mut canto = cs + egui::vec2(16., 16.);
    for k in 0..3 {
        if !lib.mover[k] {
            continue;
        }
        if let Some((_, tela, _)) = crate::gizmo::screen_axis(cam, rect, c, ax[k]) {
            let at = cs + tela * (crate::gizmo::ARM + 30.);
            if pilula(ui, rect, at, nomes[k], crate::gizmo::AXIS_COL[k], ("asm_rotulo_mover", id.0, k), "Clique e digite quanto mover neste eixo") {
                abrir = Some(Alca::Mover(k));
            }
            if st.manip.digitando.as_ref().is_some_and(|d| d.0 == Alca::Mover(k)) {
                canto = at + egui::vec2(12., 8.);
            }
        }
    }
    for (k, ponta) in pontas.iter().enumerate() {
        let Some((src, _)) = ponta else { continue };
        if let Some(at) = cam.project(rect, *src) {
            let at = at + (at - cs).normalized() * 18.;
            if pilula(ui, rect, at, &format!("⟳{}", nomes[k]), crate::gizmo::AXIS_COL[k], ("asm_rotulo_girar", id.0, k), "Clique e digite quanto girar neste eixo") {
                abrir = Some(Alca::Girar(k));
            }
            if st.manip.digitando.as_ref().is_some_and(|d| d.0 == Alca::Girar(k)) {
                canto = at + egui::vec2(12., 8.);
            }
        }
    }
    if let Some(a) = abrir {
        st.manip.digitando = Some((a, String::new()));
    }
    if let Some((alca, v)) = digitar(st, ui, canto) {
        let alvo = match alca {
            Alca::Mover(k) => {
                let mut d = [0.; 3];
                d[k] = v;
                movido(&p, &e, d)
            }
            Alca::Girar(k) => girado(&p, e[k], v, pontas[k].map_or(p.origin, |x| x.1)),
        };
        aplicar(st, id, alvo, &picks_comps);
    }
    // arrasto: tentativa numa cópia, para a colisão poder recusar
    let solto = ui.input(|i| i.pointer.any_released());
    if let Some(q) = cand {
        if st.drag_start.is_none() {
            st.drag_start = Some(p);
            st.manip.eixos_arrasto = Some(e);
            st.snapshot = Some(st.asm.clone());
        }
        let mut copia = st.asm.clone();
        if st.respect {
            levar(&mut copia, id, q);
        } else if let Some(n) = copia.node_mut(id) {
            n.placement = q;
        }
        st.contact = colide(st, &copia, id, &picks_comps);
        if !st.contact {
            st.asm = copia;
            st.dirty = true;
        }
    } else if solto {
        st.rot_drag = None;
        st.drag_center = None;
        st.manip.off = [0.; 3];
        st.manip.eixos_arrasto = None;
        if let (Some(_), Some(snap)) = (st.drag_start.take(), st.snapshot.take()) {
            st.undo.push(snap);
            st.modified = true;
            if st.respect && !st.asm.mates.is_empty() {
                st.solve();
            }
            st.dirty = true;
        }
        st.contact = false;
    }
    if st.contact {
        ui.painter_at(rect).text(cs + egui::vec2(18., 18.), egui::Align2::LEFT_TOP, tr("⛔ contato — o movimento parou na colisão"), egui::FontId::proportional(crate::acessibilidade::fonte_3d(13.)), crate::theme::st_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_montagem::kin::{JFrame, JRef, JointKind};

    const MUNDO: [[f64; 3]; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];

    fn estado() -> (AsmState, Id, Id) {
        let mut a = Assembly::new("par");
        let f = a.add_component("base", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        a.node_mut(f).unwrap().fixed = true;
        let m = a.add_component("tampa", NodeKind::Part { path: "b.fkp".into() }, Placement { origin: [100., 0., 0.], ..Default::default() }, None);
        (AsmState::new(a, None), f, m)
    }

    fn com_dobradica(st: &mut AsmState, f: Id, m: Id) {
        let (fa, fb) = (JFrame::from_axis([40., 0., 0.], [0., 0., 1.]), JFrame::from_axis([-60., 0., 0.], [0., 0., 1.]));
        st.asm.add_joint(JointKind::Revolute, JRef { comp: f, frame: fa }, JRef { comp: m, frame: fb });
    }

    #[test]
    fn so_aparecem_as_alcas_que_as_restricoes_deixam() {
        let (mut st, f, m) = estado();
        let p = st.asm.node(m).unwrap().placement;
        assert_eq!(liberdade(&mut st, m, MUNDO, p.origin, p).map(|l| l.mover), Some([true; 3]), "solta: todas");
        com_dobradica(&mut st, f, m);
        st.manip.esquecer();
        let l = liberdade(&mut st, m, MUNDO, p.origin, p).unwrap();
        assert_eq!((l.mover, l.girar), ([false; 3], [false, false, true]));
        st.respect = false;
        assert_eq!(liberdade(&mut st, m, MUNDO, p.origin, p).map(|l| l.mover), Some([true; 3]), "sem respeitar, a peça vai à mão");
    }

    #[test]
    fn peca_fixa_tem_cadeado_com_o_motivo() {
        let (mut st, f, _) = estado();
        let p = Placement::default();
        let l = liberdade(&mut st, f, MUNDO, p.origin, p);
        assert!(l.is_none());
        assert_eq!(trava(&st.asm, f, l.as_ref()), Some(Trava::Fixa));
        assert_eq!(motivo(Trava::Fixa), "Peça fixa");
        assert!(motivo(Trava::Restricoes(2)).starts_with("2 "));
    }

    #[test]
    fn digitar_37_graus_vai_pela_junta_sem_usar_o_painel() {
        let (mut st, f, m) = estado();
        com_dobradica(&mut st, f, m);
        let p = st.asm.node(m).unwrap().placement;
        let l = liberdade(&mut st, m, MUNDO, p.origin, p).unwrap();
        let ponto = ponto_de_giro(st.manip.pivo, &p, p.origin, l.pivo[2]);
        aplicar(&mut st, m, girado(&p, MUNDO[2], 37., ponto), &[]);
        let q = st.asm.node(m).unwrap().placement;
        assert!((q.dir(MUNDO[0])[0] - 37f64.to_radians().cos()).abs() < 1e-6);
        assert_eq!(st.undo.len(), 1, "um desfazer");
    }

    #[test]
    fn mudar_as_restricoes_esquece_a_conta_das_alcas() {
        let (mut st, f, m) = estado();
        let p = st.asm.node(m).unwrap().placement;
        com_dobradica(&mut st, f, m);
        liberdade(&mut st, m, MUNDO, p.origin, p);
        assert!(st.manip.cache.is_some());
        st.solve();
        assert!(st.manip.cache.is_none());
    }
}
