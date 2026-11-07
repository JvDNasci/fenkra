// SPDX-License-Identifier: LGPL-2.1-or-later
//! B373: o trecho do painel de furo que cuida da entrada e do fim da rosca modelada.
//!
//! O painel de furo segue uma hierarquia: tipo do furo (boca), medida, rosca (nenhuma, cosmética ou
//! modelada) e, só quando a rosca é modelada, entrada e fim: folga em cima, chanfro de entrada, folga
//! no fundo, profundidade da rosca e ângulo de início. O resto (fundo em V, conicidade, posição e
//! direção) fica abaixo, fora do caminho de quem só quer o furo roscado.
//!
//! Cada campo de medida aceita milímetros, passos da rosca ou porcentagem, e tem dica própria. O
//! ângulo de início conta a partir de uma referência: automática (direção X do mundo, projetada no
//! plano do furo) ou uma linha, aresta ou eixo escolhido.

use crate::editor::{check, section, Vars};
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::*;

/// Ângulos de início de uso comum (graus): o terço de volta e o quarto de volta.
const ANGULOS: [f64; 6] = [0., 90., 120., 180., 240., 270.];

fn nome_da_unidade(u: UnidadeMedida) -> &'static str {
    match u {
        UnidadeMedida::Mm => "mm",
        UnidadeMedida::Passos => "passos",
        UnidadeMedida::Porcento => "%",
    }
}

/// Linha de um valor com unidade escolhida (mm, passos ou %), dica própria e o valor calculado.
fn campo_medida(ui: &mut egui::Ui, rotulo: &str, dica: &str, texto: &mut String, unidade: &mut UnidadeMedida, vars: &Vars, com_passos: bool) {
    ui.horizontal(|ui| {
        ui.add_sized([118., 18.], egui::Label::new(tr(rotulo)).truncate()).on_hover_text(tr(dica));
        let r = fenkra_core::expr::eval_str(texto, vars);
        let vazio = texto.trim().is_empty();
        let mut te = egui::TextEdit::singleline(texto).desired_width(70.);
        if !vazio && r.is_err() {
            te = te.text_color(crate::theme::st_err());
        }
        let resp = ui.add(te).on_hover_ui(|ui| {
            ui.label(tr(dica));
            ui.label(RichText::new(tr(crate::editor::EXPR_HINT)).weak().small());
        });
        crate::ui_ext::wheel_text(ui, &resp, texto);
        egui::ComboBox::from_id_salt((rotulo, "unidade")).selected_text(tr(nome_da_unidade(*unidade))).width(64.).show_ui(ui, |ui| {
            ui.selectable_value(unidade, UnidadeMedida::Mm, tr("mm")).on_hover_text(tr("Medida em milímetros."));
            if com_passos {
                ui.selectable_value(unidade, UnidadeMedida::Passos, tr("passos")).on_hover_text(tr("Medida em passos da rosca: 1 passo é a distância entre dois filetes."));
            }
            ui.selectable_value(unidade, UnidadeMedida::Porcento, tr("%")).on_hover_text(tr("Porcentagem de uma medida de referência, explicada na dica do campo."));
        });
        if !vazio && r.is_err() {
            ui.label(RichText::new("⚠").color(crate::theme::st_err()));
        }
    });
}

/// "Entrada e fim" da rosca modelada de um furo. `com_boca`: o furo tem rebaixo ou escareado (a
/// entrada já é a boca larga, então o chanfro de entrada não se aplica).
pub(crate) fn entrada_e_fim(ui: &mut egui::Ui, doc: &Document, h: &mut Hole, vars: &Vars, seed: &crate::geo_ui::Seed, com_boca: bool) {
    section(ui, "Entrada e fim");
    // compatível com arquivos antigos: a folga no fundo era sempre em passos
    let mut un_fundo = h.extra.thread_relief_unit.unwrap_or(UnidadeMedida::Passos);
    if !com_boca {
        campo_medida(
            ui,
            "Chanfro de entrada",
            "Cone de 45 graus na boca do furo, para o parafuso entrar liso. Zero ou vazio: sem chanfro. Em %, é a porcentagem da profundidade do filete: 100% chega ao diâmetro externo da rosca.",
            &mut h.extra.thread_chamfer,
            &mut h.extra.thread_chamfer_unit,
            vars,
            true,
        );
    }
    campo_medida(
        ui,
        "Folga em cima",
        "Trecho liso entre a boca (ou o fim do rebaixo) e o começo do sulco. Vazio: o sulco começa na boca e atravessa a entrada. Em %, é a porcentagem da profundidade útil do furo.",
        &mut h.extra.thread_top_gap,
        &mut h.extra.thread_top_gap_unit,
        vars,
        true,
    );
    campo_medida(
        ui,
        "Folga embaixo",
        "Trecho liso entre o fim do sulco e o fundo do furo, para o parafuso ou o macho não bater. Em passos da rosca, mm ou % da profundidade útil do furo. Vale quando a profundidade da rosca está vazia.",
        &mut h.extra.thread_relief,
        &mut un_fundo,
        vars,
        true,
    );
    h.extra.thread_relief_unit = (un_fundo != UnidadeMedida::Passos).then_some(un_fundo);
    crate::editor::expr(ui, "Profundidade da rosca", &mut h.extra.thread_depth, vars, "mm");
    ui.label(RichText::new(tr("Vazio na profundidade da rosca = a do furo menos a folga embaixo.")).weak().small());

    ui.add_space(4.);
    crate::editor::expr(ui, "Ângulo de início", &mut h.extra.thread_start_angle, vars, "°");
    ui.horizontal_wrapped(|ui| {
        let atual = fenkra_core::expr::eval_str(&h.extra.thread_start_angle, vars).unwrap_or(0.);
        for a in ANGULOS {
            let on = (atual.rem_euclid(360.) - a).abs() < 1e-9;
            if ui.selectable_label(on, format!("{a}°")).on_hover_text(tr("Ângulo onde o sulco cruza a boca, contado a partir da referência (regra da mão direita em torno do eixo do furo).")).clicked() {
                h.extra.thread_start_angle = format!("{a}");
            }
        }
    });
    let mut propria = h.extra.thread_start_ref.is_some();
    if check(ui, &mut propria, "referência do ângulo: linha, aresta ou eixo").changed() {
        h.extra.thread_start_ref = propria.then(|| seed.curves.first().map(crate::geo_ui::axis_of).unwrap_or(AxisRef::Origin(OriginAxis::X)));
    }
    if let Some(a) = &mut h.extra.thread_start_ref {
        crate::geo_ui::axis_slot(ui, "rosca_ref", "Referência", doc, Id(u64::MAX), a, seed);
    } else {
        ui.label(RichText::new(tr("Referência automática: a direção X do mundo, projetada no plano do furo.")).weak().small());
    }
}
