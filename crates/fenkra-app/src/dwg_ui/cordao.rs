// SPDX-License-Identifier: LGPL-2.1-or-later
//! Símbolo de solda a partir do cordão do modelo (B285): lê os cordões da peça da vista e preenche
//! a nota (medida `z`, contorno e `n×l (e)`) com um clique, sem digitar nada.

use super::*;
use fenkra_desenho::simbolos::normalizar;
use fenkra_desenho::solda_cordao::{comprimento, medida, solda_do_cordao};
use fenkra_desenho::source::Model;
use fenkra_solidos::{weld_beads, CordaoCalculado};
use std::sync::Arc;

/// Linha da lista: `z5`, com o comprimento do símbolo e o total de solda.
fn rotulo(c: &CordaoCalculado) -> String {
    let n = comprimento(c);
    let mut s = medida(c);
    if !n.is_empty() {
        s += &format!(" · {n}");
    }
    format!("{s} · {} mm", crate::app::fmt(c.comprimento_soldado()))
}

pub(super) fn do_modelo(ui: &mut egui::Ui, models: &mut Models, views: &[View], n: &mut Note) {
    let Some(src) = n.view.and_then(|v| views.iter().find(|x| x.id == v)).or(views.first()).map(|v| v.source.clone()) else { return };
    let chave = egui::Id::new(("cordoes_do_modelo", n.id.0));
    ui.separator();
    ui.label(RichText::new(tr("Cordão do modelo")).strong());
    if ui.button(tr("Ler os cordões do modelo")).on_hover_text(tr("Lê os cordões de solda da peça da vista e deixa escolher um para preencher o símbolo.")).clicked() {
        let lista = match &models.load(&src).model {
            Model::Part(doc) => weld_beads(doc),
            Model::Asm(_) => vec![],
        };
        ui.ctx().data_mut(|d| d.insert_temp(chave, Arc::new(lista)));
    }
    match ui.ctx().data(|d| d.get_temp::<Arc<Vec<CordaoCalculado>>>(chave)) {
        None => {}
        Some(l) if l.is_empty() => {
            ui.label(RichText::new(tr("A peça da vista não tem cordão de solda (a montagem não é lida).")).weak().small());
        }
        Some(l) => {
            for c in l.iter() {
                if ui.button(rotulo(c)).on_hover_text(tr("Preenche o símbolo com este cordão: medida z, contorno e n×l (e).")).clicked() {
                    let (texto, solda) = solda_do_cordao(c);
                    n.text = texto;
                    n.weld = Some(solda);
                    normalizar(n);
                }
            }
        }
    }
}
