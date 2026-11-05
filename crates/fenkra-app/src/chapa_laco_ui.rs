// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painel da aba por laço e extensão lateral (B284): quais arestas, que trecho de cada uma recebe
//! a aba e os valores comuns (comprimento, ângulo, raio, lado, mitra, alívio).

use crate::editor::{check, expr, section};
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::{Document, ExtensaoAba, SmFlangeLoop};
use std::collections::HashMap;

type Vars = HashMap<String, f64>;

/// Modos da extensão lateral, na ordem do seletor.
const MODOS: [(&str, &str); 4] = [("Aresta inteira", "inteira"), ("Largura centrada", "centrada"), ("Largura deslocada", "deslocada"), ("Entre duas referências", "entre")];

fn modo(e: &ExtensaoAba) -> usize {
    match e {
        ExtensaoAba::Inteira => 0,
        ExtensaoAba::Centrada { .. } => 1,
        ExtensaoAba::Deslocada { .. } => 2,
        ExtensaoAba::Entre { .. } => 3,
    }
}

/// Extensão nova do modo `i`, aproveitando o que já havia (a largura passa de um modo a outro).
fn troca_modo(atual: &ExtensaoAba, i: usize) -> ExtensaoAba {
    let largura = match atual {
        ExtensaoAba::Centrada { largura } | ExtensaoAba::Deslocada { largura, .. } => largura.clone(),
        _ => "20".to_string(),
    };
    match i {
        1 => ExtensaoAba::Centrada { largura },
        2 => ExtensaoAba::Deslocada { inicio: "0".into(), largura },
        3 => ExtensaoAba::Entre { a: [0.; 3], b: [0.; 3] },
        _ => ExtensaoAba::Inteira,
    }
}

pub fn painel(ui: &mut egui::Ui, doc: &Document, lp: &mut SmFlangeLoop, vars: &Vars) {
    section(ui, "Arestas da aba");
    ui.label(RichText::new(format!("{} {}", lp.edges.len(), tr("aresta(s) escolhida(s)"))).small());
    check(ui, &mut lp.loop_all, "usar o laço da face inteira").on_hover_text(tr("Liga: a partir da primeira aresta escolhida, cria uma aba em cada aresta reta do contorno da face. Desliga: só as arestas escolhidas."));
    section(ui, "Extensão lateral");
    let mut m = modo(&lp.extent);
    ui.horizontal_wrapped(|ui| {
        for (i, (nome, _)) in MODOS.iter().enumerate() {
            ui.selectable_value(&mut m, i, tr(nome));
        }
    });
    if m != modo(&lp.extent) {
        lp.extent = troca_modo(&lp.extent, m);
    }
    match &mut lp.extent {
        ExtensaoAba::Inteira => {
            ui.label(RichText::new(tr("A aba cobre a aresta toda.")).weak().small());
        }
        ExtensaoAba::Centrada { largura } => {
            expr(ui, "Largura da aba", largura, vars, "mm");
        }
        ExtensaoAba::Deslocada { inicio, largura } => {
            expr(ui, "Início da aba", inicio, vars, "mm");
            expr(ui, "Largura da aba", largura, vars, "mm");
        }
        ExtensaoAba::Entre { a, b } => {
            ui.label(RichText::new(tr("Escolha dois vértices ou pontos no 3D: a aba cobre o trecho entre as projeções deles na aresta.")).weak().small());
            for (nome, p) in [("Referência 1", a), ("Referência 2", b)] {
                ui.horizontal(|ui| {
                    ui.label(tr(nome));
                    for v in p.iter_mut() {
                        ui.add(crate::ui_ext::dv(v).speed(0.1).dica("Coordenada do ponto de referência no 3D"));
                    }
                });
            }
        }
    }
    if !matches!(lp.extent, ExtensaoAba::Inteira | ExtensaoAba::Entre { .. }) {
        check(ui, &mut lp.invert, "medir a partir da outra ponta").on_hover_text(tr("A medida começa na ponta de menor X (depois Y, Z) de cada aresta; ligado, começa na outra."));
    }
    section(ui, "Aba");
    expr(ui, "Comprimento reto", &mut lp.length, vars, "mm");
    expr(ui, "Ângulo", &mut lp.angle, vars, "°");
    let mut own = lp.radius.is_some();
    ui.horizontal(|ui| {
        if check(ui, &mut own, "raio próprio (senão o da chapa)").changed() {
            lp.radius = own.then(|| "2".to_string());
        }
        if crate::chapa_estilo_ui::cadeado_comando(ui, doc, lp.radius.is_some()) {
            lp.radius = None;
        }
    });
    if let Some(r) = &mut lp.radius {
        expr(ui, "Raio interno", r, vars, "mm");
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Lado do material"));
        for (k, n) in [(0u8, "dobra por fora"), (1, "material por fora"), (2, "material por dentro")] {
            ui.selectable_value(&mut lp.side, k, tr(n));
        }
    });
    check(ui, &mut lp.flip, "dobrar para o outro lado");
    check(ui, &mut lp.miter, "pontas em meia-esquadria (45°) para a aba vizinha");
    let mut rel = lp.relief.is_some();
    ui.horizontal(|ui| {
        if check(ui, &mut rel, "alívio de canto nas pontas recuadas").changed() {
            lp.relief = rel.then(|| "2".to_string());
        }
        if crate::chapa_estilo_ui::cadeado_comando(ui, doc, lp.relief.is_some()) {
            lp.relief = None;
        }
    });
    if let Some(w) = &mut lp.relief {
        expr(ui, "Largura do alívio", w, vars, "mm");
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Tipo"));
            for (k, n) in [(0u8, "retangular"), (1, "redondo"), (2, "rasgo")] {
                ui.selectable_value(&mut lp.relief_kind, k, tr(n));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn troca_de_modo_aproveita_a_largura() {
        let c = ExtensaoAba::Centrada { largura: "35".into() };
        assert_eq!(troca_modo(&c, 2), ExtensaoAba::Deslocada { inicio: "0".into(), largura: "35".into() });
        assert_eq!(troca_modo(&c, 0), ExtensaoAba::Inteira);
        assert_eq!(modo(&troca_modo(&ExtensaoAba::Inteira, 3)), 3);
    }
}
