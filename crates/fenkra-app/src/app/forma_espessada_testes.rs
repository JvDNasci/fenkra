// SPDX-License-Identifier: LGPL-2.1-or-later
//! B537: forma livre aberta com Espessar (e Deslocar) depois dela volta à edição pela árvore, com o
//! cálculo de verdade (worker) rodando entre os quadros. O comando abre a forma, a gaiola volta, quem
//! usa a forma espera e, ao concluir, o espessar recalcula com a forma nova.

use super::*;
use crate::pmi_nota_cmd::tests::SERIE;
use fenkra_core::subd::Gaiola;

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
        app.load_doc_pub(Document::new("b537"), None);
        app.started = true;
        let mut c = Cena { app, ctx, frame: eframe::Frame::_new_kittest(), t: 0. };
        c.passo();
        // a abertura (splash) só termina depois de ~2 s: pula o relógio para o app desenhar de fato
        c.t = 5.;
        c.calculado();
        c
    }

    fn passo(&mut self) {
        self.t += 0.1;
        let raw = egui::RawInput { time: Some(self.t), screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280., 800.))), ..Default::default() };
        let (app, frame) = (&mut self.app, &mut self.frame);
        let mut saida = self.ctx.run_ui(raw, |ui| eframe::App::ui(app, ui, frame));
        saida.textures_delta.clear();
    }

    /// Quadros até o worker devolver o último pedido.
    fn calculado(&mut self) {
        for _ in 0..1500 {
            self.passo();
            if self.app.gen_done >= self.app.gen_sent && self.app.result.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(self.app.gen_done >= self.app.gen_sent, "o cálculo não voltou");
        self.passo();
    }

    fn status(&self, id: Id) -> Option<Status> {
        self.app.result.as_ref().and_then(|r| r.status.get(&id)).cloned()
    }

    fn ok(&self, id: Id) {
        assert!(matches!(self.status(id), Some(Status::Ok | Status::Warning(_))), "{}: {:?}", self.app.open.doc.name_of(id), self.status(id));
    }

    fn volume(&self) -> f64 {
        self.app.result.as_ref().map_or(0., |r| r.bodies.iter().map(|b| b.mass.volume).sum())
    }
}

#[test]
fn forma_espessada_abre_pela_arvore_e_ao_concluir_refaz_o_espessar() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let _g = SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let mut c = Cena::nova();
    // forma 2 × 2 concluída: vira superfície comum (convertida, com borda)
    let mut g = Gaiola::plano([0.; 3], [60., 60.], [2, 2]);
    let meio = g.nos.iter().find(|n| n.p[0].abs() < 1e-9 && n.p[1].abs() < 1e-9).expect("nó do meio").id;
    g.no_mut(meio).unwrap().p[2] = 4.;
    let fm = crate::forma_ui::criar_feature(&mut c.app, g);
    c.app.conclude_session();
    c.calculado();
    c.ok(fm);

    // Espessar pela barra, com a forma escolhida na árvore, e Deslocar a mesma forma
    c.app.sel = Sel::Feature(fm);
    c.app.command(Cmd::ThickSurface);
    let esp = c.app.session.as_ref().map(|s| s.id).expect("espessar aberto");
    assert!(matches!(c.app.open.doc.feature(esp).map(|f| &f.kind), Some(FeatureKind::ThickSurface(t)) if t.surface == SurfRef::Ref(fm)), "o espessar usa a forma");
    c.passo();
    c.app.conclude_session();
    let snap = c.app.open.doc.clone();
    let off = c.app.open.doc.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(fm), dist: "5".into(), both: false, join: OffsetJoin::Automatico });
    c.app.start_session(off, true, Some(snap));
    c.app.conclude_session();
    c.calculado();
    c.ok(esp);
    c.ok(off);
    assert!(!c.app.open.doc.superficie_mostrada(fm), "o espessar consumiu a forma");
    let v0 = c.volume();
    assert!(v0 > 60. * 60. * 2. * 0.97, "volume {v0}");

    // duplo clique na árvore: abre a forma com o espessar e o deslocar depois dela
    c.app.tree_action(TreeAction::EditSketch(fm));
    c.calculado();
    assert_eq!(c.app.session.as_ref().map(|s| s.id), Some(fm), "a forma abriu para edição");
    // a gaiola abre direto (antes caía no relatório da conversão, com a forma oculta no sólido): quem
    // usa a forma espera, e a forma lisa aparece mesmo consumida
    assert!(pausada(&c.app, fm), "editar a forma convertida abre a gaiola");
    assert!(matches!(c.status(esp), Some(Status::Inactive)), "espessar espera a forma: {:?}", c.status(esp));
    assert!(matches!(c.status(off), Some(Status::Inactive)), "deslocar espera a forma: {:?}", c.status(off));
    assert!(c.app.result.as_ref().is_some_and(|r| r.formas.iter().any(|x| x.id == fm)), "a forma lisa aparece para editar");
    assert_eq!(c.app.session.as_ref().map(|s| s.id), Some(fm), "o comando continua aberto depois do cálculo");

    // muda um ponto da gaiola e conclui: forma, espessar e deslocar recalculam
    if let Some(n) = crate::forma_ui::forma_mut(&mut c.app, fm).and_then(|f| f.gaiola.no_mut(meio)) {
        n.p[2] = 20.;
    }
    c.app.request_recompute();
    c.calculado();
    c.app.conclude_session();
    c.calculado();
    assert!(c.app.session.is_none());
    c.ok(fm);
    c.ok(esp);
    c.ok(off);
    let v1 = c.volume();
    assert!(v1 > v0 * 1.02, "o espessar seguiu a forma nova: {v1} contra {v0}");
    assert!(!c.app.open.doc.superficie_mostrada(fm), "concluída, a forma volta a ficar consumida");

    // abrir de novo e cancelar devolve a forma convertida e o espessar como estavam
    c.app.tree_action(TreeAction::EditSketch(fm));
    c.calculado();
    assert!(pausada(&c.app, fm));
    c.app.cancel_session();
    c.calculado();
    assert!(!pausada(&c.app, fm), "o Cancelar tira a pausa");
    c.ok(esp);
    assert!((c.volume() - v1).abs() < 1e-6 * v1, "o sólido volta igual: {} contra {v1}", c.volume());
}

fn pausada(app: &App, id: Id) -> bool {
    crate::forma_ui::forma(app, id).and_then(|f| f.conversao.as_ref()).is_some_and(|c| c.pausada)
}

/// Congelada é só leitura: abrir não pausa a conversão nem tira o espessar.
#[test]
fn forma_congelada_abre_sem_pausar() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let _g = SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let mut c = Cena::nova();
    let fm = crate::forma_ui::criar_feature(&mut c.app, Gaiola::plano([0.; 3], [40., 40.], [2, 2]));
    c.app.conclude_session();
    if let Some(f) = crate::forma_ui::forma_mut(&mut c.app, fm) {
        f.congelada = true;
    }
    c.calculado();
    c.app.start_session(fm, false, None);
    assert!(!pausada(&c.app, fm));
    c.app.conclude_session();
}
