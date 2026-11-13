// SPDX-License-Identifier: LGPL-2.1-or-later
//! `FENKRA_DEMO_DICA=<ícone>` e `FENKRA_DEMO_DICA_CAMPO=<rótulo>`: desenham, fixas na tela, a dica
//! em camadas de um comando (camada 2, com a mídia ou a animação feita pelo programa) e a dica de
//! um campo numérico, para a captura conferir sem mouse o que o pairar mostra (B278). O corpo é o
//! mesmo do pairar (`icons::conteudo_dica` e `editor::dica_do_campo`), não uma cópia.

use crate::i18n::tr;
use eframe::egui;

/// Instante da animação na captura: depois do meio do ciclo, com a cena já em movimento.
const T_ANIM: f64 = 1.4;

pub fn quadro(ctx: &egui::Context) {
    let icone = std::env::var("FENKRA_DEMO_DICA").ok();
    let campo = std::env::var("FENKRA_DEMO_DICA_CAMPO").ok();
    if icone.is_none() && campo.is_none() {
        return;
    }
    let tela = ctx.content_rect();
    if let Some(icone) = icone {
        if let Some(it) = crate::ajuda_cmd::catalogo().iter().find(|i| i.icon == icone) {
            let atalho = crate::ajuda_cmd::atalho_do_comando(ctx, it.icon);
            egui::Area::new(egui::Id::new("demo_dica_cmd")).fixed_pos(tela.center() - egui::vec2(340., 220.)).order(egui::Order::Tooltip).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    crate::icons::conteudo_dica(ui, it.icon, tr(it.name), tr(it.brief), tr(it.advanced), atalho.as_deref(), Some(T_ANIM), false);
                });
            });
        }
    }
    if let Some(rotulo) = campo {
        egui::Area::new(egui::Id::new("demo_dica_campo")).fixed_pos(tela.center() + egui::vec2(20., -220.)).order(egui::Order::Tooltip).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(320.);
                ui.label(egui::RichText::new(tr(&rotulo)).strong());
                crate::editor::dica_do_campo(ui, &rotulo, "mm");
                ui.label(egui::RichText::new(tr(crate::editor::EXPR_HINT)).weak().small());
            });
        });
    }
}

#[cfg(test)]
mod tests {
    /// O que a captura pede existe: um comando sem GIF no pacote cai na animação do programa, e o
    /// campo escolhido tem dica própria e figura.
    #[test]
    fn caso_da_captura_existe() {
        assert!(crate::ajuda_cmd::catalogo().iter().any(|i| i.icon == "chanfro"));
        assert!(crate::textos_campos::dica("Profundidade").is_some());
        assert!(crate::figura_cota::do_rotulo("Profundidade").is_some());
    }
}
