// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painel do fechamento de canto (B293): o canto escolhido, o tipo (topo, sobreposto ou sob) com
//! miniaturas, a folga entre as faces, a razão de sobreposição e as opções de canto estreito e
//! propagação para os demais cantos da chapa.

use crate::editor::{check, expr_lim, icon_choice, section};
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::SmCornerClose;
use std::collections::HashMap;

type Vars = HashMap<String, f64>;

/// (ícone, nome, dica) de cada tipo, na ordem de `SmCornerClose::kind`.
const TIPOS: [(&str, &str, &str); 3] = [
    ("sm_canto_topo", "De topo", "As duas pontas avançam até uma diagonal no meio do canto e param separadas pela folga. Nenhuma cobre a outra."),
    ("sm_canto_sobreposto", "Sobreposto", "A primeira aba avança e cobre a ponta da segunda; a segunda para na folga, junto à face de dentro da primeira."),
    ("sm_canto_sob", "Sob", "A segunda aba avança e cobre a ponta da primeira; a primeira para na folga, junto à face de dentro da segunda."),
];

pub fn painel(ui: &mut egui::Ui, cc: &mut SmCornerClose, vars: &Vars) {
    section(ui, "Fechamento de canto");
    ui.label(RichText::new(format!("{} {} — {}", tr("canto em"), crate::app::p3(cc.at, 1), tr("clique outro vértice para trocar"))).small());
    ui.label(RichText::new(tr("Tipo do canto")).weak());
    let opts: Vec<(&str, &str, &str, bool)> = TIPOS.iter().enumerate().map(|(i, (ic, n, d))| (*ic, tr(n), tr(d), cc.kind as usize == i)).collect();
    if let Some(i) = icon_choice(ui, &opts) {
        cc.kind = i as u8;
    }
    expr_lim(ui, "Folga do canto", &mut cc.gap, vars, "mm", Some((0., 10.)));
    // só quem cobre a outra tem razão de sobreposição: o topo não cobre
    if cc.kind != 0 {
        expr_lim(ui, "Razão de sobreposição", &mut cc.ratio, vars, "", Some((0., 1.)));
    }
    section(ui, "Opções do canto");
    if cc.kind != 0 {
        check(ui, &mut cc.invert, "inverter qual aba cobre").on_hover_text(tr("A primeira aba é a que chega ao canto andando com o interior da chapa à esquerda (anti-horário visto de cima). Ligado, troca."));
    }
    check(ui, &mut cc.narrow, "canto estreito").on_hover_text(tr("Limita a cobertura à metade da espessura: para quando a peça vizinha não deixa espaço para cobrir tudo."));
    check(ui, &mut cc.propagate, "propagar a todos os cantos da chapa").on_hover_text(tr("Fecha do mesmo jeito todos os cantos entre abas de 90° da chapa, cada aba cobrindo a seguinte."));
}
