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

/// Uma seta como foi pintada na tela: a nota, o índice da seta, o caminho (do início na caixa até a
/// ponta) e a caixa do texto, que é um widget próprio e fica fora do teste da seta.
#[derive(Clone, Debug)]
pub struct SetaNaTela {
    pub id: Id,
    pub k: usize,
    pub pts: Vec<Pos2>,
    pub caixa: Rect,
}

/// Setas do último quadro pintado, com o número do quadro (B448).
///
/// Causa do defeito: a seleção do 3D (`hover_pick`) é calculada antes de o PMI ser pintado, e a
/// seta não é um widget do egui (é uma linha inclinada, não um retângulo). O duplo clique na seta
/// caía então na face de baixo e abria o comando que a criou. Guardando o caminho de cada seta na
/// tela, o teste de clique do 3D pergunta primeiro "há seta aqui?" e só depois procura a face.
static NA_TELA: Mutex<(u64, Vec<SetaNaTela>)> = Mutex::new((0, Vec::new()));
/// Distância em pixels até a linha que ainda conta como clique na seta.
pub const RAIO_CLIQUE: f32 = 6.;

/// Começa o registro das setas de um quadro.
pub fn novo_quadro(quadro: u64) {
    let mut g = NA_TELA.lock().unwrap_or_else(|e| e.into_inner());
    *g = (quadro, Vec::new());
}

fn registrar(s: SetaNaTela) {
    NA_TELA.lock().unwrap_or_else(|e| e.into_inner()).1.push(s);
}

/// A seta mais próxima do ponto `q`, se alguma passa a menos de `RAIO_CLIQUE` (a caixa do texto
/// não conta: ali o clique é do rótulo).
pub fn seta_sob(setas: &[SetaNaTela], q: Pos2) -> Option<(Id, usize)> {
    setas
        .iter()
        .filter(|s| !s.caixa.contains(q))
        .filter_map(|s| s.pts.windows(2).map(|w| dist_seg(q, w[0], w[1])).min_by(f32::total_cmp).map(|d| (d, s)))
        .filter(|(d, _)| *d < RAIO_CLIQUE)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, s)| (s.id, s.k))
}

/// Seta sob o cursor, pelas setas pintadas no quadro anterior ou neste (`quadro` é o atual). Fora
/// disso o registro é velho (o PMI não foi pintado, por exemplo no sketch) e nada é devolvido.
pub fn seta_em(quadro: u64, q: Pos2) -> Option<(Id, usize)> {
    let g = NA_TELA.lock().unwrap_or_else(|e| e.into_inner());
    if quadro > g.0 + 1 {
        return None;
    }
    seta_sob(&g.1, q)
}

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

/// Usado pelos testes, que não têm teclado: finge Ctrl ou Shift apertado.
#[cfg(test)]
pub fn fingir_modificador(m: bool) {
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

/// Alça do início de cada seta (nota selecionada) e registro do caminho de cada seta para o teste
/// de clique do 3D (`seta_em`). `lr` é a caixa do texto sem girar, como a chamada a vê.
#[allow(clippy::too_many_arguments)]
pub fn interagir(ui: &mut egui::Ui, painter: &egui::Painter, pen: &Pen, cam: &Camera, rect: Rect, pm: &Pmi, a: &[[f64; 3]], lr: Rect, selecionada: bool, out: &mut Out) {
    if !pen.leader_on || a.is_empty() {
        return;
    }
    let rs = rotas(cam, rect, pm, a);
    for (k, rt) in rs.iter().enumerate() {
        let lado = fenkra_core::pmi_leader::side(pm, k);
        let (att, knee, _) = pen.leader_start(lr, rt[0], lado);
        let mut pts = vec![att, knee];
        pts.extend(rt.iter().copied());
        registrar(SetaNaTela { id: pm.id, k, pts, caixa: pen.outer(lr) });
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

    #[test]
    fn seta_mais_proxima_ganha_e_caixa_do_texto_fica_fora() {
        let caixa = Rect::from_min_max(Pos2::new(100., 100.), Pos2::new(160., 120.));
        let setas = vec![
            SetaNaTela { id: Id(1), k: 0, pts: vec![Pos2::new(100., 110.), Pos2::new(90., 110.), Pos2::new(20., 40.)], caixa },
            SetaNaTela { id: Id(1), k: 1, pts: vec![Pos2::new(100., 110.), Pos2::new(90., 110.), Pos2::new(20., 180.)], caixa },
        ];
        // no meio da segunda seta (de (90,110) a (20,180)), 3 px ao lado
        assert_eq!(seta_sob(&setas, Pos2::new(55. + 2., 145. + 2.)), Some((Id(1), 1)));
        assert_eq!(seta_sob(&setas, Pos2::new(55., 75.)), Some((Id(1), 0)));
        // longe das duas linhas
        assert_eq!(seta_sob(&setas, Pos2::new(55., 110.)), None);
        // dentro da caixa do texto o clique é do rótulo
        assert_eq!(seta_sob(&setas, Pos2::new(101., 110.)), None);
    }

    /// B448: nota numa face da placa, vista de cima. Sob a ponta da seta há uma face (o picking a
    /// acha), mas o teste de clique do 3D vê a seta antes; o duplo clique ali abre a nota com a seta
    /// pedida, e não o comando da face.
    #[test]
    fn seta_fica_a_frente_da_face_e_duplo_clique_abre_a_nota() {
        use crate::app::{App, Sel};
        use crate::picking::{Filter, Pick};
        use crate::pmi_nota_cmd::tests::{app_com_placa, conferir, SERIE};
        let _g = SERIE.lock().unwrap_or_else(|e| e.into_inner());
        let mut app = app_com_placa();
        let r = app.result.as_ref().unwrap();
        let (vol, lo, hi) = conferir(r);
        assert!(vol > 0.);
        // uma face plana do topo (normal +Z, na altura máxima), no meio de um triângulo
        let b = &r.bodies[0];
        let topo = (0..b.mesh.face_ranges.len())
            .find_map(|face| {
                let p = Pick::Face { body: 0, face, owner: b.collector, point: [0.; 3], normal: [0.; 3] };
                let t = crate::pmi_ui::pick_target(r, &p)?;
                (t.class == fenkra_core::pmi::TargetClass::FacePlana && t.dir[2] > 0.99 && (t.point[2] as f32 - hi[2]).abs() < 1e-3).then_some(face)
            })
            .expect("a placa tem face no topo");
        let ini = b.mesh.face_ranges[topo] as usize;
        let tri = &b.mesh.indices[ini..ini + 3];
        let v = |i: u32| [0, 1, 2].map(|k| b.mesh.positions[i as usize * 3 + k]);
        let point = [0, 1, 2].map(|k| (v(tri[0])[k] + v(tri[1])[k] + v(tri[2])[k]) / 3.);
        app.picks = vec![Pick::Face { body: 0, face: topo, owner: b.collector, point, normal: [0., 0., 1.] }];
        crate::pmi_ui::command(&mut app, crate::pmi_nota_cmd::NOTA);
        let id = app.open.doc.pmi.last().expect("nota criada").id;
        app.sel = Sel::None;
        crate::pmi_minibar::drop_session();

        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(900., 700.));
        let mut cam = Camera { dist: 400., ..Default::default() };
        cam.fit((Vec3::from(lo), Vec3::from(hi)));
        cam.set_view(0., std::f32::consts::FRAC_PI_2 - 0.01);
        let ctx = egui::Context::default();
        let mut quadro = |app: &mut App, t: f64, evs: Vec<egui::Event>| {
            let raw = egui::RawInput { screen_rect: Some(rect), events: evs, time: Some(t), ..Default::default() };
            let mut s = ctx.run_ui(raw, |ui| crate::pmi_ui::overlay(app, ui, rect, &cam));
            s.textures_delta.clear();
        };
        quadro(&mut app, 0.0, vec![]);
        quadro(&mut app, 0.1, vec![]);
        // um ponto da seta perto da ponta: ali embaixo está a face do topo
        let pts = NA_TELA.lock().unwrap().1.iter().find(|s| s.id == id).expect("a seta foi registrada").pts.clone();
        let (a, ponta) = (pts[pts.len() - 2], *pts.last().unwrap());
        let q = ponta + (a - ponta).normalized() * 4.;
        let filt = Filter { faces: true, edges: false, vertices: false, planes: false, hidden_env: false, exclude_sketch: None, fantasma: None };
        let r = app.result.as_ref().unwrap();
        assert!(matches!(crate::picking::pick(r, &app.open.doc, &cam, rect, q, &filt), Some(Pick::Face { .. })), "sob a seta há uma face");
        assert_eq!(seta_em(ctx.cumulative_frame_nr(), q), Some((id, 0)), "o teste de clique vê a seta antes da face");
        // registro velho (PMI não pintado há quadros) não segura o clique
        assert_eq!(seta_em(ctx.cumulative_frame_nr() + 5, q), None);

        let mover = egui::Event::PointerMoved(q);
        let botao = |pressed| egui::Event::PointerButton { pos: q, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        quadro(&mut app, 1.00, vec![mover.clone()]);
        quadro(&mut app, 1.02, vec![botao(true)]);
        quadro(&mut app, 1.06, vec![botao(false)]);
        quadro(&mut app, 1.10, vec![botao(true)]);
        quadro(&mut app, 1.14, vec![botao(false)]);
        assert_eq!(app.sel, Sel::Pmi(id), "o duplo clique na seta abre a nota");
        assert_eq!(consumir(id), Some(0), "com a seta 1 pedida aberta no painel");
    }
}
