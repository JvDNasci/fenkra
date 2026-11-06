// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painéis dos comandos de chapa do B35: aba entre perfis (lofted), desdobrar e redobrar parcial e
//! converter sólido em chapa.

use crate::editor::{check, expr_lim, section};
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::{Document, Id, SmConvert, SmLoftFlange, SmUnfoldPartial};
use fenkra_solidos::chapa_desdobra::e_dobra;
use std::collections::HashMap;

type Vars = HashMap<String, f64>;

/// Aba entre dois perfis: os dois contornos fechados, o tipo de transição e o lado da espessura.
pub fn painel_perfis(ui: &mut egui::Ui, doc: &Document, idx: usize, lf: &mut SmLoftFlange) {
    section(ui, "Perfis");
    crate::feat_ui::sketch_pick(ui, "slf_a", "Perfil 1", doc, idx, &mut lf.a);
    crate::feat_ui::sketch_pick(ui, "slf_b", "Perfil 2", doc, idx, &mut lf.b);
    ui.label(RichText::new(tr("Dois contornos fechados em planos diferentes. O perfil é a face de fora da chapa; a espessura vem dos parâmetros da chapa.")).weak().small());
    section(ui, "Transição");
    check(ui, &mut lf.ruled, "transição facetada (dobradeira)").on_hover_text(tr("Ligado: lados retos entre os vértices correspondentes dos dois perfis, como numa dobradeira. Desligado: transição lisa, como numa estampa."));
    crate::chapa_unif_ui::inverter(ui, &mut lf.flip);
    ui.label(RichText::new(tr("Sem planificação exata: no estado planificado a aba aparece dobrada, com aviso.")).weak().small());
}

/// Desdobrar ou redobrar só algumas abas: lista as abas que vêm antes deste comando.
pub fn painel_desdobra(ui: &mut egui::Ui, doc: &Document, idx: usize, u: &mut SmUnfoldPartial) {
    section(ui, if u.refold { "Abas a redobrar" } else { "Abas a desdobrar" });
    let candidatas: Vec<(Id, String)> = doc.features.iter().take(idx).filter(|f| e_dobra(&f.kind)).map(|f| (f.id, f.name.clone())).collect();
    if candidatas.is_empty() {
        ui.label(RichText::new(tr("⚠ nenhuma aba antes deste comando")).small());
    }
    for (id, nome) in candidatas {
        let mut on = u.features.contains(&id);
        if ui.checkbox(&mut on, nome).changed() {
            if on {
                u.features.push(id);
            } else {
                u.features.retain(|x| *x != id);
            }
        }
    }
    ui.horizontal(|ui| {
        if ui.button(tr("Redobrar")).clicked() && !u.refold {
            u.refold = true;
        }
        if ui.button(tr("Desdobrar")).clicked() && u.refold {
            u.refold = false;
        }
    });
    ui.label(RichText::new(tr(if u.refold { "Redobrar volta a dobrar as abas marcadas; as outras ficam como estão." } else { "As abas marcadas ficam planas e o resto da peça continua dobrado. Para voltar, use Redobrar parcial ou desative este comando." })).weak().small());
}

/// Converter sólido em chapa: face fixa, espessura esperada e a medição do corpo.
pub fn painel_converte(ui: &mut egui::Ui, doc: &Document, me: Id, c: &mut SmConvert, vars: &Vars) {
    section(ui, "Converter em chapa");
    ui.label(RichText::new(if c.face == [0.; 3] { tr("Face fixa: nenhuma (clique numa face plana da chapa para escolher)").to_string() } else { format!("{} {} — {}", tr("face fixa em"), crate::app::p3(c.face, 1), tr("clique outra face para trocar")) }).small());
    // vazio mede a espessura do corpo; com valor, o comando confere que o corpo tem essa espessura
    let mut medir = c.thickness.trim().is_empty();
    if check(ui, &mut medir, "medir a espessura do corpo").changed() {
        c.thickness = if medir { String::new() } else { "2".into() };
    }
    if !medir {
        expr_lim(ui, "Espessura esperada", &mut c.thickness, vars, "mm", Some((0.01, f64::INFINITY)));
    }
    let medir = ui.button(tr("Medir o corpo")).on_hover_text(tr("Reconhece a espessura e as dobras do corpo antes deste comando."));
    let chave = ui.id().with(("converte_medida", me.0));
    if medir.clicked() {
        let r = fenkra_solidos::chapa_converte::analisar_corpo(doc, me).map(|a| {
            c.thickness = format!("{:.3}", a.espessura).trim_end_matches('0').trim_end_matches('.').to_string();
            a.resumo()
        });
        ui.ctx().data_mut(|d| d.insert_temp(chave, r));
    }
    if let Some(r) = ui.ctx().data(|d| d.get_temp::<Result<String, String>>(chave)) {
        match r {
            Ok(t) => {
                ui.label(RichText::new(t).small());
                ui.label(RichText::new(tr("Ajuste a espessura em Chapa › Parâmetros para a mesma medida.")).weak().small());
            }
            Err(e) => {
                ui.label(RichText::new(e).small().color(crate::theme::st_err()));
            }
        }
    }
}
