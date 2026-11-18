// SPDX-License-Identifier: LGPL-2.1-or-later
//! O realce de um item escolhido na árvore sai pelas mesmas portas do que veio do 3D: clique no
//! vazio do 3D, clique no vazio da árvore e Esc. Antes só `picks` era zerado, e `sel`/`multi` (que
//! também acendem a peça pelo conjunto único) ficavam presos.

use super::*;
use crate::pmi_nota_cmd::tests::{faces, SERIE};

fn esc() -> Vec<egui::Event> {
    vec![egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }]
}

fn botao(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() }
}

struct Cena {
    app: App,
    ctx: egui::Context,
    frame: eframe::Frame,
    t: f64,
}

impl Cena {
    fn nova() -> Cena {
        let ctx = egui::Context::default();
        let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        let e = crate::examples::all().into_iter().find(|e| e.name == "Placa paramétrica").expect("exemplo");
        app.load_doc_pub(e.doc, None);
        app.started = true;
        app.result = Some(fenkra_solidos::recompute(&app.open.doc));
        let mut c = Cena { app, ctx, frame: eframe::Frame::_new_kittest(), t: 0. };
        c.passo(vec![]);
        // a abertura (splash) só termina depois de ~2 s: pula o relógio para o app desenhar de fato
        // e o splash também espera o primeiro cálculo da peça, que roda em outra thread
        c.t = 5.;
        for _ in 0..500 {
            c.passo(vec![]);
            if c.app.gen_done >= c.app.gen_sent {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        c.t += 3.;
        c.passo(vec![]);
        c.passo(vec![]);
        c
    }

    fn passo(&mut self, eventos: Vec<egui::Event>) {
        self.t += 0.1;
        let raw = egui::RawInput { time: Some(self.t), screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280., 800.))), events: eventos, ..Default::default() };
        let (app, frame) = (&mut self.app, &mut self.frame);
        let mut saida = self.ctx.run_ui(raw, |ui| eframe::App::ui(app, ui, frame));
        saida.textures_delta.clear();
    }

    fn clicar(&mut self, pos: egui::Pos2) {
        self.passo(vec![egui::Event::PointerMoved(pos)]);
        self.passo(vec![botao(pos, true)]);
        self.passo(vec![botao(pos, false)]);
        self.passo(vec![]);
    }

    /// Escolhe um recurso como a árvore faz: item único + multisseleção, e o 3D passa a acendê-lo.
    fn selecionar_pela_arvore(&mut self) -> Id {
        let id = self.app.open.doc.features.iter().find(|f| matches!(f.kind, fenkra_core::FeatureKind::Extrude(_))).map(|f| f.id).expect("extrusão");
        self.app.sel = Sel::Feature(id);
        *self.app.multi.borrow_mut() = vec![id];
        self.passo(vec![]);
        self.passo(vec![]);
        assert_eq!(self.app.selecao_extra.visto, vec![id], "a peça acende o item da árvore");
        id
    }

    fn conferir_limpo(&mut self) {
        self.passo(vec![]);
        assert_eq!(self.app.sel, Sel::None, "item único da árvore");
        assert!(self.app.multi.borrow().is_empty(), "multisseleção da árvore");
        assert!(self.app.picks.is_empty(), "seleção do 3D");
        assert!(self.app.selecao_extra.visto.is_empty(), "sem realce na peça nem na árvore");
    }
}

/// O clique do 3D passa por `clique_no_3d` (o mesmo código da viewport, que no teste não desenha por
/// falta de wgpu): clicar no fundo vazio é `alvo = None`.
#[test]
fn arvore_e_clique_vazio_no_3d_limpam_tudo() {
    let _g = SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let mut c = Cena::nova();
    c.selecionar_pela_arvore();
    let App { picks, sel, multi, .. } = &mut c.app;
    App::clique_no_3d(picks, sel, multi, true, None, false, false);
    c.conferir_limpo();
}

/// Shift/Ctrl no vazio não derrubam a seleção (como já era no 3D).
#[test]
fn clique_vazio_com_shift_nao_limpa() {
    let _g = SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let mut c = Cena::nova();
    let id = c.selecionar_pela_arvore();
    let App { picks, sel, multi, .. } = &mut c.app;
    App::clique_no_3d(picks, sel, multi, true, None, false, true);
    assert_eq!(c.app.sel, Sel::Feature(id));
    assert_eq!(*c.app.multi.borrow(), vec![id]);
}

#[test]
fn arvore_e_esc_limpam_tudo() {
    let _g = SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let mut c = Cena::nova();
    c.selecionar_pela_arvore();
    c.passo(esc());
    c.conferir_limpo();
}

#[test]
fn arvore_e_clique_no_vazio_da_arvore_limpam_tudo() {
    let _g = SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let mut c = Cena::nova();
    c.selecionar_pela_arvore();
    c.clicar(egui::pos2(100., 760.));
    c.conferir_limpo();
}

#[test]
fn arvore_e_outro_item_no_3d_so_o_novo_fica() {
    let _g = SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let mut c = Cena::nova();
    let antigo = c.selecionar_pela_arvore();
    let p = faces(&c.app, 1);
    let App { picks, sel, multi, .. } = &mut c.app;
    App::clique_no_3d(picks, sel, multi, true, Some(p[0]), false, false);
    c.passo(vec![]);
    assert_eq!(c.app.sel, Sel::None, "o item da árvore saiu");
    assert!(c.app.multi.borrow().is_empty());
    assert_eq!(c.app.picks, vec![p[0]], "só o novo no 3D");
    let dono = crate::selecao::dono_na_arvore(&p[0], |b| c.app.result.as_ref().and_then(|r| r.bodies.get(b)).map(|x| x.collector));
    assert_eq!(c.app.selecao_extra.visto, dono.into_iter().collect::<Vec<_>>(), "o realce é só o dono da face nova");
    assert!(!c.app.selecao_extra.visto.contains(&antigo) || dono == Some(antigo));
}
