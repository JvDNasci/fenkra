// SPDX-License-Identifier: LGPL-2.1-or-later
//! Caixa de comandos própria (B502, R1.5): a pessoa monta uma barra arrastando ícones da lista de
//! comandos para dentro da caixa. Arrastar um ícone dentro da caixa muda a ordem; arrastá-lo para fora
//! tira da caixa. Comando de outra bancada entra sem duplicar código: a caixa guarda só a chave dele
//! (`toolbar::entry_key`), e o botão chama a mesma ação.
//!
//! Chave que não existe mais (comando removido numa versão nova) não derruba a caixa: o item aparece
//! como indisponível, e a pessoa decide se tira.

use crate::barras_layout::{por_na_caixa, tirar_da_caixa};
use crate::i18n::tr;
use crate::toolbar::{entry_key, entry_title, Entry};
use eframe::egui::{self, RichText};

/// O que vai preso ao cursor: a chave do comando e, se veio da caixa, a posição dele lá.
#[derive(Clone, Debug)]
struct Arraste {
    chave: String,
    de: Option<usize>,
}

/// Lado do ícone na caixa e na lista.
const ICONE: f32 = 24.;

/// A caixa e a lista de comandos. `filtro` guarda o texto da busca entre quadros.
pub fn caixa(ui: &mut egui::Ui, itens: &mut Vec<String>, todos: &[Entry], filtro: &mut String) {
    ui.label(RichText::new(tr("Caixa de comandos")).strong());
    ui.label(RichText::new(tr("Arraste os ícones da lista para a caixa. Arraste um ícone da caixa para mudar a ordem, ou para fora dela para tirá-lo.")).weak().small());
    let ponteiro = ui.ctx().pointer_hover_pos();
    let mut onde: Option<usize> = None;
    let quadro = egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(6));
    let (zona, solto) = ui.dnd_drop_zone::<Arraste, ()>(quadro, |ui| {
        ui.set_min_size(egui::vec2(ui.available_width(), ICONE + 8.));
        ui.horizontal_wrapped(|ui| {
            if itens.is_empty() {
                ui.label(RichText::new(tr("Vazia: arraste comandos para cá.")).weak().italics());
            }
            for (i, k) in itens.iter().enumerate() {
                let r = match todos.iter().find(|e| entry_key(e) == *k) {
                    Some(e) => {
                        let (icone, frase) = entry_title(e);
                        ui.dnd_drag_source(egui::Id::unique(("caixa_item", i, k.as_str())), Arraste { chave: k.clone(), de: Some(i) }, |ui| {
                            ui.add(egui::Image::new(crate::icons::get(icone)).fit_to_exact_size(egui::vec2(ICONE, ICONE)));
                        })
                        .response
                        .on_hover_text(tr(frase))
                    }
                    None => ui
                        .dnd_drag_source(egui::Id::unique(("caixa_item", i, k.as_str())), Arraste { chave: k.clone(), de: Some(i) }, |ui| {
                            ui.label(RichText::new("?").weak());
                        })
                        .response
                        .on_hover_text(tr("Comando indisponível nesta versão. Arraste para fora da caixa para tirá-lo.")),
                };
                // onde o ícone arrastado vai entrar: antes ou depois do ícone sob o cursor
                if let Some(p) = ponteiro.filter(|p| r.rect.expand(2.).contains(*p)) {
                    onde = Some(if p.x < r.rect.center().x { i } else { i + 1 });
                }
            }
        });
    });
    if let Some(a) = solto {
        por_na_caixa(itens, &a.chave, onde);
    } else if ui.input(|i| i.pointer.any_released()) && !zona.response.contains_pointer() {
        // ícone da caixa solto fora dela: sai da caixa
        if let Some(a) = egui::DragAndDrop::payload::<Arraste>(ui.ctx()) {
            if let Some(i) = a.de.filter(|i| itens.get(*i) == Some(&a.chave)) {
                tirar_da_caixa(itens, i);
            }
        }
    }
    ui.add_space(6.);
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Comandos")).strong());
        ui.add(egui::TextEdit::singleline(filtro).hint_text(tr("buscar")).desired_width(160.));
    });
    let busca = filtro.to_lowercase();
    egui::ScrollArea::vertical().id_salt("caixa_lista").max_height(220.).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for e in todos {
                let (icone, frase) = entry_title(e);
                if !busca.is_empty() && !tr(frase).to_lowercase().contains(&busca) {
                    continue;
                }
                let k = entry_key(e);
                let ja = itens.contains(&k);
                ui.dnd_drag_source(egui::Id::unique(("caixa_lista", k.as_str())), Arraste { chave: k.clone(), de: None }, |ui| {
                    let img = egui::Image::new(crate::icons::get(icone)).fit_to_exact_size(egui::vec2(ICONE, ICONE));
                    ui.add(if ja { img.tint(ui.visuals().weak_text_color()) } else { img });
                })
                .response
                .on_hover_text(tr(frase));
            }
        });
    });
}
