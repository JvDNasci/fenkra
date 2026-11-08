// SPDX-License-Identifier: LGPL-2.1-or-later
//! Ida e volta entre a montagem e a peça ativa (B362): o caso de captura e os testes, que rodam
//! quadros de verdade do aplicativo, sem placa de vídeo. As ações entram pelo meio do quadro, como o
//! clique: chamadas entre quadros não mostram a comparação do fim do quadro, que era o defeito.

use crate::app::App;
use fenkra_core::{Document, Extent, FeatureKind};
use fenkra_montagem::NodeKind;
use std::sync::atomic::{AtomicU8, Ordering};

/// Triplica a primeira extrusão cega: uma edição que muda volume e caixa.
fn engrossar(d: &mut Document) {
    for f in &mut d.features {
        if let FeatureKind::Extrude(e) = &mut f.kind {
            if let Extent::Blind(v) = &mut e.extent {
                *v = (v.parse::<f64>().unwrap_or(10.) * 3.).to_string();
                return;
            }
        }
    }
}

/// `FENKRA_DEMO_ABRIR=<caso>` com `FENKRA_DEMO_ASM=1`: percorre a ida e a volta sem mouse, um passo a
/// cada vez que o cálculo assenta. `ida` (peça ativa com o conjunto em volta), `volta`, `suja` (a
/// pergunta ao voltar), `editada` (edita, salva e volta) e `ausente` (o arquivo da peça sumiu).
pub fn demo_quadro(app: &mut App) {
    static ETAPA: AtomicU8 = AtomicU8::new(0);
    let Ok(caso) = std::env::var("FENKRA_DEMO_ABRIR") else { return };
    let Some(st) = app.asm.as_ref() else { return };
    if app.calculando() || st.compondo || st.dirty || st.envio_pendente || st.view.is_none() {
        return;
    }
    let Some(n) = st.asm.nodes.iter().find(|n| matches!(n.kind, NodeKind::Part { .. })) else { return };
    let id = n.id;
    let montagem = crate::settings::Bench::Montagem;
    match (ETAPA.load(Ordering::Relaxed), caso.as_str()) {
        (0, c) => {
            if c == "ausente" {
                if let (NodeKind::Part { path }, Some(d)) = (&n.kind, st.dir()) {
                    let _ = std::fs::remove_file(d.join(path));
                }
            }
            crate::asm_ui::edit_part(app, id);
            ETAPA.store(1, Ordering::Relaxed);
        }
        (1, "volta") => {
            app.settings.bench = montagem;
            ETAPA.store(9, Ordering::Relaxed);
        }
        (1, "suja") => {
            engrossar(&mut app.open.doc);
            ETAPA.store(2, Ordering::Relaxed);
        }
        (1, "editada") => {
            engrossar(&mut app.open.doc);
            app.save_pub();
            app.settings.bench = montagem;
            ETAPA.store(9, Ordering::Relaxed);
        }
        (2, _) => {
            app.settings.bench = montagem;
            ETAPA.store(9, Ordering::Relaxed);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::engrossar;
    use crate::app::App;
    use crate::asm_ui::{concluir_volta, edit_part, editar_geometria, AsmCtx, AsmState, Volta};
    use crate::settings::Bench;
    use eframe::egui;
    use fenkra_core::{BoolOp, Document, Extent, Extrude, FeatureKind, Id, OriginPlane, PlaneRef, Sketch, SketchData};
    use fenkra_montagem::{Assembly, NodeKind, Placement};
    use std::path::PathBuf;

    fn bloco(nome: &str, l: f64) -> Document {
        let mut d = Document::new(nome);
        let mut n = 7_200_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., 0.], [l, l], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(l.to_string()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        d
    }

    /// Aplicativo de verdade, sem janela, com a montagem A (bloco de 10) e B (bloco de 20).
    struct Bancada {
        app: App,
        ctx: egui::Context,
        frame: eframe::Frame,
        t: f64,
        dir: PathBuf,
        a: Id,
        b: Id,
    }

    impl Bancada {
        fn nova(nome: &str) -> Self {
            std::env::set_var("FENKRA_DEMO_TESTE", "1");
            let dir = std::env::temp_dir().join(format!("fenkra_abrir_{nome}_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("a.fkp"), bloco("A", 10.).to_json()).unwrap();
            std::fs::write(dir.join("b.fkp"), bloco("B", 20.).to_json()).unwrap();
            let mut m = Assembly::new("Conjunto");
            let a = m.add_component("A", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
            let b = m.add_component("B", NodeKind::Part { path: "b.fkp".into() }, Placement { origin: [50., 0., 0.], ..Default::default() }, None);
            let ctx = egui::Context::default();
            let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
            app.started = true;
            // o documento que estava aberto antes de entrar na montagem
            app.load_doc_pub(bloco("Fora", 5.), Some(dir.join("fora.fkp")));
            std::fs::write(dir.join("fora.fkp"), app.open.doc.to_json()).unwrap();
            app.asm = Some(AsmState::new(m, Some(dir.join("conjunto.fka"))));
            app.settings.bench = Bench::Montagem;
            let mut b = Bancada { app, ctx, frame: eframe::Frame::_new_kittest(), t: 0., dir, a, b };
            b.assentar();
            b
        }

        fn st(&mut self) -> &mut AsmState {
            self.app.asm.as_mut().unwrap()
        }

        fn quadro(&mut self) {
            self.t += 0.1;
            let raw = egui::RawInput { time: Some(self.t), screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280., 800.))), ..Default::default() };
            let (app, frame) = (&mut self.app, &mut self.frame);
            let mut saida = self.ctx.run_ui(raw, |ui| eframe::App::ui(app, ui, frame));
            // sem placa de vídeo ninguém consome as texturas, e o egui reclama delas ao soltar
            saida.textures_delta.clear();
        }

        /// Roda quadros até o cálculo e a composição terminarem.
        fn assentar(&mut self) {
            for _ in 0..6000 {
                self.quadro();
                let compondo = self.app.asm.as_ref().is_some_and(|s| s.compondo || s.dirty || s.envio_pendente);
                if !self.app.calculando() && !compondo {
                    self.quadro();
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            panic!("não assentou");
        }

        /// Como o menu do 3D: a ação roda no meio do quadro, depois da viewport.
        fn abrir(&mut self, id: Id) {
            self.st().ctx_act = Some(AsmCtx::Edit(id));
            self.assentar();
        }

        /// Como o clique na aba Montagem: a volta roda no meio do quadro.
        fn aba_montagem(&mut self) {
            self.app.settings.bench = Bench::Montagem;
            self.quadro();
        }

        fn editar(&mut self) {
            // entre quadros a comparação do fim do quadro não vê a edição: entra pelo caminho de fora
            let antes = self.app.open.doc.clone();
            engrossar(&mut self.app.open.doc);
            self.app.push_undo_pub(antes);
            self.assentar();
            assert!(self.app.open.modified, "a edição marca o documento");
        }

        fn arquivo(&self, nome: &str) -> PathBuf {
            self.dir.join(nome)
        }

        fn no_disco(&self, nome: &str) -> Document {
            Document::from_json(&std::fs::read_to_string(self.arquivo(nome)).unwrap()).unwrap()
        }

        fn volume(&mut self) -> f64 {
            self.st().view.as_ref().unwrap().bodies.iter().map(|b| b.mass.volume).sum()
        }

        /// Fora da peça, na montagem, com o documento de antes devolvido intacto.
        fn de_volta_na_montagem(&mut self) {
            assert!(self.st().editing_part.is_none(), "a peça fechou");
            assert!(self.st().volta.is_none(), "sem pergunta pendente");
            assert_eq!(self.app.settings.bench, Bench::Montagem);
            assert_eq!(self.app.open.doc, bloco("Fora", 5.));
            assert_eq!(self.app.open.file, Some(self.arquivo("fora.fkp")));
            assert!(!self.app.open.modified, "o documento de antes não fica sujo");
        }
    }

    fn perto(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-3 * b.abs().max(1.), "{a} != {b}");
    }

    #[test]
    fn ida_e_volta_sem_mudar_nada_nao_pergunta_nem_suja() {
        let mut b = Bancada::nova("limpa");
        perto(b.volume(), 9000.);
        let a = b.a;
        b.abrir(a);
        assert_eq!(b.app.open.file, Some(b.arquivo("a.fkp")));
        assert_eq!(b.app.open.doc, bloco("A", 10.));
        assert_eq!(b.app.settings.bench, Bench::Solidos);
        assert!(!b.app.open.modified, "abrir não é editar");
        assert!(!b.app.pode_desfazer(), "o desfazer não traz o documento de antes para dentro da peça");
        perto(b.app.result.as_ref().unwrap().bodies.iter().map(|c| c.mass.volume).sum(), 1000.);
        assert!(b.st().vista_contexto.is_some(), "o conjunto aparece em volta");
        b.aba_montagem();
        b.de_volta_na_montagem();
        assert!(!b.st().modified, "nada mudou na montagem");
        b.assentar();
        assert!(!b.app.open.modified && !b.st().modified, "continua limpo depois do cálculo");
        perto(b.volume(), 9000.);
    }

    #[test]
    fn abrir_nao_calcula_a_peca_na_thread_da_interface() {
        let mut b = Bancada::nova("sem_calculo");
        let (a, f) = (b.a, b.arquivo("a.fkp"));
        b.st().cache.forget(&f);
        edit_part(&mut b.app, a);
        assert!(b.st().editing_part.is_some());
        assert!(b.st().cache.peek(&f).is_none(), "abrir só lê o documento");
    }

    #[test]
    fn contexto_chega_quando_a_composicao_termina() {
        let mut b = Bancada::nova("contexto");
        let a = b.a;
        b.st().dirty = true;
        b.st().ctx_act = Some(AsmCtx::Edit(a));
        b.quadro();
        assert!(b.st().editing_part.is_some(), "abre sem esperar a composição");
        assert!(b.st().vista_contexto.is_none());
        b.assentar();
        assert_eq!(b.st().vista_contexto.as_ref().map(|v| v.bodies.len()), Some(1), "só B fica em volta");
    }

    #[test]
    fn edicao_salva_aparece_na_montagem_ao_voltar() {
        let mut b = Bancada::nova("edicao");
        let a = b.a;
        b.abrir(a);
        b.editar();
        b.app.save_pub();
        assert!(!b.app.open.modified);
        b.aba_montagem();
        b.de_volta_na_montagem();
        assert!(b.st().pecas_alteradas.contains(&a) && b.st().modified);
        b.assentar();
        assert!(b.st().errors.is_empty(), "{:?}", b.st().errors);
        perto(b.volume(), 11000.);
        let i = b.st().view.as_ref().unwrap().bodies.iter().position(|c| c.collector == a).unwrap();
        perto(b.st().caixas[i].1[2] as f64, 30.);
        perto(fenkra_solidos::recompute(&b.no_disco("a.fkp")).bodies[0].mass.volume, 3000.);
    }

    #[test]
    fn alteracoes_por_salvar_nos_dois_documentos() {
        let mut b = Bancada::nova("dois");
        let (a, id_b) = (b.a, b.b);
        // a montagem fica por salvar antes de entrar
        b.st().asm.node_mut(id_b).unwrap().placement.origin[0] = 60.;
        b.st().modified = true;
        b.st().dirty = true;
        b.abrir(a);
        b.editar();
        // voltar pergunta, e nada sai do lugar enquanto a pessoa decide
        b.aba_montagem();
        assert!(b.st().volta.is_some() && b.st().editing_part.is_some());
        assert_eq!(b.app.settings.bench, Bench::Solidos, "a montagem não aparece com a peça por trás");
        assert_eq!(b.app.open.file, Some(b.arquivo("a.fkp")));
        // descartar: o arquivo da peça fica como estava, e a montagem guarda o que tinha
        concluir_volta(&mut b.app, Volta::Pai, false);
        b.de_volta_na_montagem();
        assert_eq!(b.no_disco("a.fkp"), bloco("A", 10.));
        assert!(b.st().pecas_alteradas.is_empty());
        assert!(b.st().modified, "a montagem segue por salvar");
        assert_eq!(b.st().asm.node(id_b).unwrap().placement.origin[0], 60.);
        b.assentar();
        perto(b.volume(), 9000.);
        // salvar e voltar: grava a peça, e a montagem continua com o que tinha
        b.abrir(a);
        b.editar();
        b.aba_montagem();
        concluir_volta(&mut b.app, Volta::Pai, true);
        b.de_volta_na_montagem();
        assert_ne!(b.no_disco("a.fkp"), bloco("A", 10.));
        b.assentar();
        perto(b.volume(), 11000.);
        crate::asm_ui::save(&mut b.app);
        assert!(!b.st().modified);
        let salva = Assembly::from_json(&std::fs::read_to_string(b.arquivo("conjunto.fka")).unwrap()).unwrap();
        assert_eq!(salva.node(id_b).unwrap().placement.origin[0], 60.);
    }

    #[test]
    fn peca_aberta_por_salvar_pergunta_antes_de_sair_de_cena() {
        let mut b = Bancada::nova("fora_suja");
        let a = b.a;
        engrossar(&mut b.app.open.doc);
        b.app.open.modified = true;
        let antes = b.app.open.doc.clone();
        edit_part(&mut b.app, a);
        assert!(b.app.confirmando(), "pergunta Salvar / Descartar / Cancelar");
        assert!(b.st().editing_part.is_none());
        assert_eq!(b.app.open.doc, antes);
        assert!(b.app.open.modified);
    }

    #[test]
    fn peca_ausente_avisa_e_nao_mexe_em_nada() {
        let mut b = Bancada::nova("ausente");
        let (a, id_b) = (b.a, b.b);
        std::fs::remove_file(b.arquivo("a.fkp")).unwrap();
        b.abrir(a);
        b.de_volta_na_montagem();
        assert_eq!(b.st().referencias.len(), 1, "a janela Resolver dá a saída");
        assert_eq!(b.st().referencias[0].comp, a);
        // pedir de novo não repete a pendência, e a outra peça segue abrindo e voltando
        b.abrir(a);
        assert_eq!(b.st().referencias.len(), 1);
        b.abrir(id_b);
        assert_eq!(b.app.open.file, Some(b.arquivo("b.fkp")));
        assert!(!b.app.open.modified);
        b.aba_montagem();
        b.de_volta_na_montagem();
    }

    #[test]
    fn comando_aberto_nao_sobra_depois_da_volta() {
        let mut b = Bancada::nova("comando");
        let a = b.a;
        b.abrir(a);
        let id = b.app.open.doc.features[1].id;
        b.app.start_session(id, false, None);
        b.quadro();
        b.aba_montagem();
        assert!(b.st().volta.is_some(), "comando aberto é trabalho em andamento");
        concluir_volta(&mut b.app, Volta::Pai, false);
        assert!(b.app.session.is_none());
        b.de_volta_na_montagem();
        b.assentar();
        b.de_volta_na_montagem();
        // salvar e voltar conclui o comando e grava o que ele mudou
        b.abrir(a);
        let id = b.app.open.doc.features[1].id;
        b.app.start_session(id, false, None);
        engrossar(&mut b.app.open.doc);
        b.aba_montagem();
        concluir_volta(&mut b.app, Volta::Pai, true);
        assert!(b.app.session.is_none());
        b.de_volta_na_montagem();
        assert_ne!(b.no_disco("a.fkp"), bloco("A", 10.));
    }

    #[test]
    fn alternar_entre_pecas_passa_pela_montagem() {
        let mut b = Bancada::nova("alternar");
        let (a, id_b) = (b.a, b.b);
        for (id, nome) in [(a, "a.fkp"), (id_b, "b.fkp"), (a, "a.fkp")] {
            b.abrir(id);
            assert_eq!(b.app.open.file, Some(b.arquivo(nome)));
            assert!(!b.app.open.modified && !b.app.pode_desfazer());
            b.aba_montagem();
            b.de_volta_na_montagem();
        }
        // pedir a outra peça com a ativa por salvar pergunta, sem abrir por cima
        b.abrir(a);
        b.editar();
        let editado = b.app.open.doc.clone();
        edit_part(&mut b.app, id_b);
        assert!(b.st().volta.is_some());
        assert_eq!(b.app.open.doc, editado);
        assert_eq!(b.st().editing_part.as_ref().map(|p| p.0), Some(a));
    }

    #[test]
    fn outro_documento_aberto_solta_a_peca_ativa() {
        let mut b = Bancada::nova("solta");
        let a = b.a;
        b.abrir(a);
        let fora = b.arquivo("fora.fkp");
        b.app.open_path(&fora);
        b.quadro();
        assert!(b.st().editing_part.is_none());
        assert_eq!(b.app.open.file, Some(fora), "quem foi aberto fica");
        assert_eq!(b.app.settings.bench, Bench::Solidos);
    }

    #[test]
    fn volta_do_subconjunto_pedida_na_viewport_nao_troca_o_documento() {
        let mut b = Bancada::nova("sub");
        let mut sub = Assembly::new("Sub");
        sub.add_component("A", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        std::fs::write(b.arquivo("sub.fka"), sub.to_json()).unwrap();
        let id = b.st().asm.add_component("Sub", NodeKind::SubAssembly { path: "sub.fka".into() }, Placement { origin: [0., 80., 0.], ..Default::default() }, None);
        b.st().dirty = true;
        b.assentar();
        b.abrir(id);
        assert_eq!(b.st().pais.len(), 1);
        b.st().volta_adiada = Some(Volta::Pai);
        b.quadro();
        assert!(b.st().pais.is_empty());
        b.assentar();
        b.de_volta_na_montagem();
    }

    #[test]
    fn peca_sem_arquivo_guarda_na_memoria_e_nasce_ao_salvar() {
        let mut b = Bancada::nova("pendente");
        let f = b.arquivo("nova.fkp");
        let id = b.st().asm.add_component("Nova", NodeKind::Part { path: "nova.fkp".into() }, Placement::default(), None);
        b.st().cache.definir_pendente(&f, bloco("Nova", 10.));
        b.st().dirty = true;
        b.assentar();
        b.abrir(id);
        b.editar();
        b.app.save_pub();
        assert!(!b.app.open.modified && !f.exists());
        b.aba_montagem();
        b.de_volta_na_montagem();
        assert!(b.st().pecas_alteradas.contains(&id));
        crate::asm_ui::save(&mut b.app);
        assert!(f.exists(), "o arquivo nasce com o Salvar da montagem");
        // a que nasceu e não recebeu nada some sem deixar arquivo
        let vazia = b.arquivo("vazia.fkp");
        let id = b.st().asm.add_component("Vazia", NodeKind::Part { path: "vazia.fkp".into() }, Placement::default(), None);
        b.st().cache.definir_pendente(&vazia, Document::new("Vazia"));
        b.abrir(id);
        b.aba_montagem();
        b.de_volta_na_montagem();
        assert!(b.st().asm.node(id).is_none() && !vazia.exists());
    }

    /// Coloca a peça da mão e confere o componente novo na montagem composta.
    fn colocar_e_conferir(b: &mut Bancada, est: &mut crate::asm_inserir::Estado, volume: f64) {
        let st = b.app.asm.as_mut().unwrap();
        assert!(crate::asm_inserir::colocar_copia(st, est, [0., 120., 0.]).is_some());
        b.assentar();
        assert!(b.st().errors.is_empty(), "{:?}", b.st().errors);
        perto(b.volume(), volume);
    }

    #[test]
    fn inserir_a_peca_aberta_com_alteracoes_por_salvar() {
        let mut b = Bancada::nova("inserir_suja");
        let antes = b.app.open.doc.clone();
        engrossar(&mut b.app.open.doc);
        b.app.push_undo_pub(antes);
        b.assentar();
        let mut est = crate::asm_inserir::Estado::novo(false, None);
        crate::asm_inserir::pegar_aberta(&mut b.app, &mut est).unwrap();
        assert!(!b.app.open.modified, "pegar salva");
        assert_ne!(b.no_disco("fora.fkp"), bloco("Fora", 5.));
        // o bloco de 5 com 15 de altura soma 375 aos 9000
        colocar_e_conferir(&mut b, &mut est, 9375.);
    }

    #[test]
    fn inserir_a_peca_aberta_que_nunca_foi_salva() {
        let mut b = Bancada::nova("inserir_nova");
        std::env::set_var("FENKRA_DEMO_PASTA", &b.dir);
        b.app.load_doc_pub(bloco("Nunca salva", 5.), None);
        let mut est = crate::asm_inserir::Estado::novo(false, None);
        crate::asm_inserir::pegar_aberta(&mut b.app, &mut est).unwrap();
        assert_eq!(b.app.open.file, Some(b.arquivo("Nunca salva.fkp")));
        assert!(b.app.settings.recent_files.iter().any(|f| f.ends_with("Nunca salva.fkp")));
        colocar_e_conferir(&mut b, &mut est, 9125.);
    }

    #[test]
    fn inserir_a_peca_salva_e_ela_segue_aberta() {
        let mut b = Bancada::nova("inserir_salva");
        let mut est = crate::asm_inserir::Estado::novo(false, None);
        crate::asm_inserir::pegar_aberta(&mut b.app, &mut est).unwrap();
        colocar_e_conferir(&mut b, &mut est, 9125.);
        assert_eq!(b.app.open.doc, bloco("Fora", 5.));
    }

    #[test]
    fn documento_do_projeto_salvo_entra_nos_recentes_e_na_montagem() {
        let mut b = Bancada::nova("inserir_projeto");
        let mut p = fenkra_project::Project::create(&b.dir.join("proj"), "P").unwrap();
        let id = p.add_document(bloco("Do projeto", 5.)).unwrap();
        let doc = p.load(id).unwrap();
        let arquivo = p.root.join(&p.entry(id).unwrap().file);
        b.app.project = Some(p);
        b.app.load_doc_pub(doc, None);
        b.app.open.entry = Some(id);
        b.app.open.modified = true;
        let mut est = crate::asm_inserir::Estado::novo(false, None);
        crate::asm_inserir::pegar_aberta(&mut b.app, &mut est).unwrap();
        assert!(b.app.settings.recent_files.contains(&arquivo.to_string_lossy().to_string()));
        colocar_e_conferir(&mut b, &mut est, 9125.);
    }

    #[test]
    fn geometria_da_montagem_vai_e_volta() {
        let mut b = Bancada::nova("geometria");
        editar_geometria(&mut b.app);
        b.assentar();
        assert!(b.st().editing_part.is_some() && !b.app.open.modified);
        b.app.open.doc = bloco("Geometria", 10.);
        b.quadro();
        b.app.save_pub();
        b.aba_montagem();
        b.de_volta_na_montagem();
        assert_eq!(b.st().asm.geometria.as_ref().map(|g| g.features.len()), Some(2));
        assert!(b.st().modified);
    }

    #[test]
    fn abrir_pelo_caminho_atualiza_os_links_dos_subconjuntos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // B382: os recentes, a tela inicial e a linha de comando abrem por `open_path`, que não
        // atualizava os links; só o menu atualizava, e o kart não recebia o entre-eixos novo
        std::env::set_var("FENKRA_DEMO_TESTE", "1");
        let dir = std::env::temp_dir().join(format!("fenkra_abrir_links_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mestre = |ee: &str| {
            let mut d = Document::new("Mestre");
            let id = d.alloc();
            d.params.push(fenkra_core::Parameter { published: true, ..fenkra_core::Parameter::novo(id, "EE", ee, fenkra_core::Unit::Comprimento) });
            std::fs::write(dir.join("mestre.fkp"), d.to_json()).unwrap();
        };
        mestre("20");
        let mut filha = bloco("Filha", 10.);
        filha.add_link("Mestre", dir.join("mestre.fkp").to_str().unwrap(), "EE", fenkra_core::LinkedGeo::Value(20.));
        std::fs::write(dir.join("filha.fkp"), filha.to_json()).unwrap();
        let mut sub = Assembly::new("Sub");
        sub.add_component("Filha", NodeKind::Part { path: "filha.fkp".into() }, Placement::default(), None);
        std::fs::write(dir.join("sub.fka"), sub.to_json()).unwrap();
        let mut topo = Assembly::new("Topo");
        topo.add_component("Mestre", NodeKind::Part { path: "mestre.fkp".into() }, Placement::default(), None);
        topo.add_component("Sub", NodeKind::SubAssembly { path: "sub.fka".into() }, Placement::default(), None);
        std::fs::write(dir.join("topo.fka"), topo.to_json()).unwrap();
        mestre("35");

        let mut app = App::new(&eframe::CreationContext::_new_kittest(egui::Context::default()));
        app.open_path(&dir.join("topo.fka"));
        assert!(app.asm.is_some());
        assert!(app.settings.bench == Bench::Montagem);
        let d = Document::from_json(&std::fs::read_to_string(dir.join("filha.fkp")).unwrap()).unwrap();
        let valor = d.external.iter().find(|e| e.publication == "EE").and_then(|e| e.snapshot.clone());
        assert_eq!(valor, Some(fenkra_core::LinkedGeo::Value(35.)), "a peça do subconjunto recebe o valor novo do mestre");
        let _ = std::fs::remove_dir_all(dir);
    }
}
