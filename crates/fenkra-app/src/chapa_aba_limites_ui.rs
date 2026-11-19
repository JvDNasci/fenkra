// SPDX-License-Identifier: LGPL-2.1-or-later
//! B556: campos da aba por superfície para os limites dos lados 1 e 2, o limite da borda final e o
//! tratamento do canto (filete ou chanfro) de cada lado. Cada limite é opcional; o campo de seleção
//! é o padrão (clique no 3D, lista ou árvore) e aceita plano, face, superfície, curva, aresta, linha
//! do 3D, sketch ou elemento de construção.

use crate::editor::{check, expr_lim, section};
use crate::geo_ui::{self, Seed};
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::*;
use std::collections::HashMap;

type Vars = HashMap<String, f64>;

/// Seção "Limites" e "Cantos" do painel da aba por superfície.
pub fn painel(ui: &mut egui::Ui, doc: &Document, me: Id, lm: &mut LimitesAba, vars: &Vars, seed: &Seed) {
    section(ui, "Limites da aba");
    limite(
        ui,
        "smsf_lim1",
        "Limite do lado 1",
        "Limita a aba neste lado da linha de dobra. Pode ser plano, face plana, superfície, curva, aresta, linha do 3D, sketch ou elemento de construção. O comando estende o suporte até o limite e recorta a aba nele; curva e linha são projetadas no plano da web.",
        &mut lm.lado1,
        doc,
        me,
        seed,
    );
    limite(ui, "smsf_lim2", "Limite do lado 2", "Limita a aba neste lado da linha de dobra. Pode ser plano, face plana, superfície, curva, aresta, linha do 3D, sketch ou elemento de construção. O comando estende o suporte até o limite e recorta a aba nele; curva e linha são projetadas no plano da web.", &mut lm.lado2, doc, me, seed);
    limite(
        ui,
        "smsf_limp",
        "Limite da borda final",
        "Limita a borda final (a aresta livre) da aba. A aba é estendida ou recortada até o limite; uma curva é estendida normal à aba para virar superfície de corte.",
        &mut lm.ponta,
        doc,
        me,
        seed,
    );
    section(ui, "Cantos");
    canto(ui, "Canto do lado 1", &mut lm.canto1, vars);
    canto(ui, "Canto do lado 2", &mut lm.canto2, vars);
    ui.label(RichText::new(tr("O lado 1 é a ponta de menor coordenada (X, depois Y, depois Z) ao longo da linha de dobra; o lado 2, a outra. O canto arredonda ou chanfra a quina da ponta da aba, na borda final.")).weak().small());
    ui.label(RichText::new(tr("Sem limite a aba vai de ponta a ponta do suporte sobre a web, com o comprimento dado. Com limite na borda final, o comprimento vira o do limite.")).weak().small());
}

/// Um limite opcional: liga, escolhe o tipo (plano ou superfície; curva ou aresta) e o elemento.
#[allow(clippy::too_many_arguments)]
fn limite(ui: &mut egui::Ui, salt: &str, rotulo: &str, dica: &str, campo: &mut Option<LimiteAba>, doc: &Document, me: Id, seed: &Seed) {
    let mut ligado = campo.is_some();
    if check(ui, &mut ligado, rotulo).on_hover_text(tr(dica)).changed() {
        // o campo nasce vazio ("a escolher") e recebe a próxima seleção no 3D
        *campo = ligado.then(|| LimiteAba::Superficie(crate::slots::surf()));
    }
    let Some(l) = campo else { return };
    let mut curva = matches!(l, LimiteAba::Curva(_));
    ui.horizontal(|ui| {
        ui.add_space(18.);
        let a = ui.radio_value(&mut curva, false, tr("plano ou superfície")).changed();
        let b = ui.radio_value(&mut curva, true, tr("curva ou aresta")).changed();
        if a || b {
            *l = if curva { LimiteAba::Curva(crate::slots::curve()) } else { LimiteAba::Superficie(crate::slots::surf()) };
        }
    });
    match l {
        LimiteAba::Superficie(s) => geo_ui::surf_slot(ui, &format!("{salt}_s"), rotulo, doc, me, s, seed),
        LimiteAba::Curva(c) => geo_ui::curve_slot(ui, &format!("{salt}_c"), rotulo, doc, me, c, seed),
    }
}

/// Canto de uma ponta: nenhum, filete (raio) ou chanfro (distância).
fn canto(ui: &mut egui::Ui, rotulo: &str, c: &mut CantoAba, vars: &Vars) {
    ui.horizontal(|ui| {
        ui.label(tr(rotulo));
        let valor = match c {
            CantoAba::Nenhum => String::new(),
            CantoAba::Filete(v) | CantoAba::Chanfro(v) => v.clone(),
        };
        let v = if valor.is_empty() { "2".to_string() } else { valor };
        if ui.selectable_label(matches!(c, CantoAba::Nenhum), tr("Nenhum")).clicked() {
            *c = CantoAba::Nenhum;
        }
        if ui.selectable_label(matches!(c, CantoAba::Filete(_)), tr("filete")).on_hover_text(tr("Arredonda a quina da ponta da aba.")).clicked() && !matches!(c, CantoAba::Filete(_)) {
            *c = CantoAba::Filete(v.clone());
        }
        if ui.selectable_label(matches!(c, CantoAba::Chanfro(_)), tr("chanfro")).on_hover_text(tr("Corta a quina da ponta da aba em diagonal.")).clicked() && !matches!(c, CantoAba::Chanfro(_)) {
            *c = CantoAba::Chanfro(v);
        }
    });
    match c {
        CantoAba::Filete(v) => expr_lim(ui, "Raio do filete", v, vars, "mm", Some((0.01, f64::INFINITY))),
        CantoAba::Chanfro(v) => expr_lim(ui, "Distância do chanfro", v, vars, "mm", Some((0.01, f64::INFINITY))),
        CantoAba::Nenhum => {}
    }
}
