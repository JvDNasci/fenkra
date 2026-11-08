// SPDX-License-Identifier: LGPL-2.1-or-later
//! B370: edição das setas de uma nota da folha, com o mesmo modelo do PMI: uma seção por seta com
//! posição, lado da nota de onde ela sai e formato da ponta; botão para acrescentar e para tirar.
//! Os dados e as regras vêm de `fenkra_desenho::setas_nota`.

use super::*;
use fenkra_core::pmi::PmiArrow;
use fenkra_desenho::setas_nota as setas;

const LADOS: [(u8, &str); 5] = [(0, "Automático"), (1, "Esquerda"), (2, "Direita"), (3, "Topo"), (4, "Base")];
const PONTAS: [(PmiArrow, &str); 6] = [(PmiArrow::Padrao, "Padrão"), (PmiArrow::Cheia, "Cheia"), (PmiArrow::Aberta, "Aberta"), (PmiArrow::Ponto, "Ponto"), (PmiArrow::Barra, "Barra"), (PmiArrow::Nenhuma, "Nenhuma")];

/// Seção "Setas" da nota. Devolve `true` se algo mudou.
pub fn secao(ui: &mut egui::Ui, n: &mut Note) -> bool {
    let mut mudou = false;
    let total = setas::pontas(n).len();
    let mut tirar = None;
    egui::CollapsingHeader::new(format!("{} ({})", tr("Setas"), total)).id_salt(("nota_setas", n.id.0)).default_open(true).show(ui, |ui| {
        for k in 0..total {
            egui::CollapsingHeader::new(format!("{} {}", tr("Seta"), k + 1)).id_salt(("nota_seta", n.id.0, k)).default_open(total == 1).show(ui, |ui| {
                let mut p = setas::pontas(n)[k];
                ui.horizontal(|ui| {
                    ui.label(tr("Ponta (x, y)"));
                    let a = ui.add(crate::ui_ext::dv(&mut p[0]).speed(0.5));
                    let b = ui.add(crate::ui_ext::dv(&mut p[1]).speed(0.5));
                    if a.changed() || b.changed() {
                        mudou |= setas::mover(n, k, p);
                    }
                });
                let mut lado = setas::lado(n, k);
                ui.horizontal_wrapped(|ui| {
                    ui.label(tr("Ancoragem na nota")).on_hover_text(tr("De que lado da nota sai esta seta. Cada seta tem a sua; também se arrasta o quadrado no início da seta, na folha."));
                    for (l, nome) in LADOS {
                        ui.selectable_value(&mut lado, l, tr(nome));
                    }
                });
                mudou |= setas::definir_lado(n, k, lado);
                let mut ponta = setas::seta(n, k);
                ui.horizontal(|ui| {
                    ui.label(tr("Ponta")).on_hover_text(tr("Formato da ponta desta seta; Padrão usa a do estilo da nota."));
                    egui::ComboBox::from_id_salt(("nota_seta_ponta", n.id.0, k)).selected_text(tr(PONTAS.iter().find(|x| x.0 == ponta).map_or("Padrão", |x| x.1))).show_ui(ui, |ui| {
                        for (a, nome) in PONTAS {
                            ui.selectable_value(&mut ponta, a, tr(nome));
                        }
                    });
                });
                mudou |= setas::definir_seta(n, k, ponta);
                if ui.button(tr("Remover seta")).on_hover_text(tr("Tira só esta seta; a nota e as outras setas ficam")).clicked() {
                    tirar = Some(k);
                }
            });
        }
        if ui.button(tr("Adicionar seta")).on_hover_text(tr("Acrescenta outra seta da nota, perto do texto; arraste ou edite a posição depois")).clicked() {
            let base = setas::pontas(n).last().copied().unwrap_or([n.pos[0] - 15., n.pos[1] - 10.]);
            setas::adicionar(n, [base[0], base[1] - 12.]);
            mudou = true;
        }
    });
    if let Some(k) = tirar {
        mudou |= setas::remover(n, k);
    }
    mudou
}
