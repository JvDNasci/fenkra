// SPDX-License-Identifier: LGPL-2.1-or-later
//! B468: o Esc de um comando aberto desfaz um nível por vez. Primeiro a lista aberta de um campo
//! (o próprio egui a fecha), depois a seleção feita no 3D, e só então o comando. Antes, um Esc com
//! a lista ou uma face selecionada cancelava o comando inteiro e perdia o que já estava preenchido.

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) enum Esc {
    /// há uma lista aberta: o Esc é dela
    FechaLista,
    /// há seleção no 3D: o Esc a limpa e o comando continua
    LimpaSelecao,
    /// nada a desfazer antes: o Esc cancela o comando
    Cancela,
}

pub(super) fn decidir(lista_aberta: bool, tem_selecao: bool) -> Esc {
    if lista_aberta {
        Esc::FechaLista
    } else if tem_selecao {
        Esc::LimpaSelecao
    } else {
        Esc::Cancela
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::app::App;
    use crate::pmi_nota_cmd::tests::{faces, SERIE};
    use eframe::egui;

    #[test]
    fn a_ordem_do_esc() {
        assert_eq!(decidir(true, true), Esc::FechaLista);
        assert_eq!(decidir(false, true), Esc::LimpaSelecao);
        assert_eq!(decidir(false, false), Esc::Cancela);
    }

    fn quadro(app: &mut App, ctx: &egui::Context, frame: &mut eframe::Frame, t: &mut f64, eventos: Vec<egui::Event>) {
        *t += 0.1;
        let raw = egui::RawInput { time: Some(*t), screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280., 800.))), events: eventos, ..Default::default() };
        let mut saida = ctx.run_ui(raw, |ui| eframe::App::ui(app, ui, frame));
        saida.textures_delta.clear();
    }

    fn esc() -> Vec<egui::Event> {
        vec![egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }]
    }

    /// Comando aberto com uma face selecionada: o primeiro Esc limpa só a seleção, o seguinte fecha.
    #[test]
    fn esc_com_selecao_limpa_a_selecao_e_o_seguinte_fecha_o_comando() {
        let _g = SERIE.lock().unwrap_or_else(|e| e.into_inner());
        let ctx = egui::Context::default();
        let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        let e = crate::examples::all().into_iter().find(|e| e.name == "Placa paramétrica").expect("exemplo");
        app.load_doc_pub(e.doc, None);
        app.started = true;
        app.result = Some(fenkra_solidos::recompute(&app.open.doc));
        let mut frame = eframe::Frame::_new_kittest();
        let mut t = 0.;
        quadro(&mut app, &ctx, &mut frame, &mut t, vec![]);
        let id = app.open.doc.features.iter().find(|f| matches!(f.kind, fenkra_core::FeatureKind::Extrude(_))).map(|f| f.id).expect("extrusão");
        app.start_session(id, false, None);
        assert!(app.session.is_some());
        app.picks = faces(&app, 1);
        assert!(!app.picks.is_empty());
        quadro(&mut app, &ctx, &mut frame, &mut t, esc());
        assert!(app.session.is_some(), "o Esc com seleção não fecha o comando");
        assert!(app.picks.is_empty(), "o Esc limpou a seleção");
        quadro(&mut app, &ctx, &mut frame, &mut t, vec![]);
        quadro(&mut app, &ctx, &mut frame, &mut t, esc());
        assert!(app.session.is_none(), "o Esc seguinte fecha o comando");
    }
}
