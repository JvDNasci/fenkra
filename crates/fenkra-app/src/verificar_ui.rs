// SPDX-License-Identifier: LGPL-2.1-or-later
//! B418: painel "Verificar requisitos". A peça declara o que precisa cumprir (massa, furos, folga,
//! interferência, caixa, propagação de parâmetro) e o painel confere um a um, com o que foi medido.
//!
//! O painel é livre e encaixável, abre pelo botão da aba Análise e fecha no ✕. A lista é dado da peça
//! (`Document::requisitos`): muda com desfazer e vai junto no arquivo. A conferência roda só no clique,
//! porque folga e propagação recalculam cópias da peça.

use crate::app::App;
use crate::i18n::tr;
use crate::ui_ext::dv;
use eframe::egui::{self, Rect, RichText};
use fenkra_core::requisitos::{Medida, Requisito, TipoRequisito};
use fenkra_solidos::verificar::{verificar, Resultado};
use std::cell::RefCell;

#[derive(Default)]
struct Estado {
    aberto: bool,
    resultados: Vec<Resultado>,
    /// a lista mudou depois da última conferência
    velho: bool,
    demo_lido: bool,
}

thread_local! {
    static ST: RefCell<Estado> = RefCell::new(Estado::default());
}

/// Abre o painel (botão da barra).
pub fn abrir() {
    ST.with(|s| s.borrow_mut().aberto = true);
}

pub fn aberto() -> bool {
    ST.with(|s| s.borrow().aberto)
}

/// Requisitos de exemplo para a captura sem mouse (`FENKRA_DEMO_VERIFICAR=1`): um de cada tipo, com
/// valores que a peça de exemplo cumpre e um que ela não cumpre, para a captura mostrar os dois estados.
fn demo(doc: &mut fenkra_core::Document) {
    if !doc.requisitos.is_empty() {
        return;
    }
    let p = doc.params.first().map(|p| (p.name.clone(), p.expr.clone()));
    let mut l = vec![
        Requisito { nome: "Massa até 2 kg".into(), tipo: TipoRequisito::Massa { min: None, max: Some(2.) } },
        Requisito { nome: "Cabe em 100 mm".into(), tipo: TipoRequisito::Caixa { max: [100.; 3] } },
        Requisito { nome: "Furos Ø6,6".into(), tipo: TipoRequisito::Furos { diametro: 6.6, quantidade: 4, tol: 0.05 } },
        Requisito { nome: "Corpos sem cruzar".into(), tipo: TipoRequisito::SemInterferencia { tol: 1e-3 } },
    ];
    if let Some((nome, expr)) = p {
        let novo = expr.trim().parse::<f64>().map_or_else(|_| format!("({expr})*1.2"), |v| crate::app::fmt(v * 1.2).replace(',', "."));
        l.push(Requisito { nome: format!("{nome} chega na forma"), tipo: TipoRequisito::Propagacao { parametro: nome, valor: novo, medida: Medida::Volume, esperado: None, tol: 0.01 } });
    }
    doc.requisitos = l;
}

fn campos(ui: &mut egui::Ui, t: &mut TipoRequisito, params: &[String]) -> bool {
    let mut mudou = false;
    match t {
        TipoRequisito::Massa { min, max } => {
            for (rot, v, dica) in [(tr("mín."), min, tr("Massa mínima em kg; desligado, não há mínimo.")), (tr("máx."), max, tr("Massa máxima em kg; desligado, não há máximo."))] {
                ui.horizontal(|ui| {
                    let mut liga = v.is_some();
                    mudou |= ui.checkbox(&mut liga, rot).on_hover_text(dica).changed();
                    if liga != v.is_some() {
                        *v = liga.then_some(1.);
                        mudou = true;
                    }
                    if let Some(x) = v.as_mut() {
                        mudou |= ui.add(dv(x).speed(0.01).range(0.0..=1e6).suffix(" kg").dica(dica)).changed();
                    }
                });
            }
        }
        TipoRequisito::Furos { diametro, quantidade, tol } => {
            ui.horizontal(|ui| {
                mudou |= ui.add(dv(diametro).speed(0.1).range(0.1..=1e4).prefix("Ø ").suffix(" mm").dica(tr("Diâmetro do furo que a peça precisa ter. Para um parafuso, use o furo de passagem da biblioteca."))).changed();
                mudou |= ui.add(dv(quantidade).range(1..=10000).suffix(" ×").dica(tr("Quantos furos desse diâmetro, no mínimo."))).changed();
                mudou |= ui.add(dv(tol).speed(0.01).range(0.0..=10.).prefix("± ").suffix(" mm").dica(tr("Tolerância no diâmetro."))).changed();
            });
        }
        TipoRequisito::Folga { min } => {
            mudou |= ui.add(dv(min).speed(0.1).range(0.0..=1e4).rotulo(tr("mín.")).suffix(" mm").dica(tr("Menor distância aceita entre dois corpos da peça."))).changed();
        }
        TipoRequisito::SemInterferencia { tol } => {
            mudou |= ui.add(dv(tol).speed(0.01).range(0.0..=1e6).rotulo(tr("tolerância")).suffix(" mm³").dica(tr("Volume em comum aceito entre dois corpos, para descontar o ruído da booleana."))).changed();
        }
        TipoRequisito::Caixa { max } => {
            ui.horizontal(|ui| {
                for (k, e) in ["X", "Y", "Z"].iter().enumerate() {
                    mudou |= ui.add(dv(&mut max[k]).speed(1.).range(0.0..=1e6).prefix(format!("{e} ")).suffix(" mm").dica(tr("Tamanho máximo da peça neste eixo."))).changed();
                }
            });
        }
        TipoRequisito::Propagacao { parametro, valor, medida, esperado, tol } => {
            ui.horizontal(|ui| {
                let antes = parametro.clone();
                egui::ComboBox::from_id_salt(("verif_param", ui.id())).selected_text(if parametro.is_empty() { tr("parâmetro").to_string() } else { parametro.clone() }).show_ui(ui, |ui| {
                    for p in params {
                        ui.selectable_value(parametro, p.clone(), p);
                    }
                }).response.on_hover_text(tr("Parâmetro que a conferência troca numa cópia da peça."));
                mudou |= *parametro != antes;
                ui.label("=");
                mudou |= ui.add(egui::TextEdit::singleline(valor).desired_width(60.)).on_hover_text(tr("Valor de teste: número ou expressão. A peça aberta não muda.")).changed();
            });
            ui.horizontal(|ui| {
                let antes = *medida;
                egui::ComboBox::from_id_salt(("verif_medida", ui.id())).selected_text(tr(medida.rotulo())).show_ui(ui, |ui| {
                    for m in Medida::TODAS {
                        ui.selectable_value(medida, m, tr(m.rotulo()));
                    }
                }).response.on_hover_text(tr("O que é medido antes e depois da troca."));
                mudou |= *medida != antes;
                let mut liga = esperado.is_some();
                if ui.checkbox(&mut liga, tr("esperado")).on_hover_text(tr("Ligado: a medida tem de chegar a este valor. Desligado: basta mudar.")).changed() {
                    *esperado = liga.then_some(0.);
                    mudou = true;
                }
                if let Some(x) = esperado.as_mut() {
                    mudou |= ui.add(dv(x).speed(0.1).dica(tr("Valor que a medida deve ter depois da troca."))).changed();
                    mudou |= ui.add(dv(tol).speed(0.01).range(0.0..=1e6).prefix("± ").dica(tr("Tolerância da medida."))).changed();
                }
            });
        }
    }
    mudou
}

/// Painel livre da verificação. Só aparece com o painel aberto e uma peça na frente.
pub fn painel(app: &mut App, ctx: &egui::Context, area: Rect) {
    use crate::paineis::{self, Painel};
    ST.with(|st| {
        let mut e = st.borrow_mut();
        if !e.demo_lido {
            e.demo_lido = true;
            if std::env::var_os("FENKRA_DEMO_VERIFICAR").is_some() {
                demo(&mut app.open.doc);
                e.aberto = true;
                e.resultados = verificar(&app.open.doc, &app.open.doc.requisitos);
            }
        }
    });
    if !aberto() || app.sem_peca_em_edicao() {
        return;
    }
    let cfg = Painel { nome: "verificar_requisitos".into(), titulo: tr("Verificar requisitos").to_string(), padrao: paineis::abaixo_do_cubo(area, 380.), largura: 380., redimensiona: false };
    let params: Vec<String> = app.open.doc.params.iter().map(|p| p.name.clone()).collect();
    let mut lista = app.open.doc.requisitos.clone();
    let antes_doc = app.open.doc.clone();
    let (mut mudou, mut conferir, mut fechar) = (false, false, false);
    ST.with(|st| {
        let mut e = st.borrow_mut();
        let posicao = app.settings.painel(&cfg.nome, paineis::Encaixe::Livre);
        let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
            ui.label(RichText::new(tr("O que a peça precisa cumprir. A lista fica na peça e vai junto no arquivo.")).small().weak());
            if lista.is_empty() {
                ui.label(RichText::new(tr("Nenhum requisito. Use Acrescentar.")).italics());
            }
            ui.horizontal(|ui| {
                ui.menu_button(tr("+ Acrescentar"), |ui| {
                    for m in TipoRequisito::modelos() {
                        if ui.button(tr(m.rotulo())).clicked() {
                            let base = tr(m.rotulo()).to_string();
                            let n = (1..).map(|k| if k == 1 { base.clone() } else { format!("{base} {k}") }).find(|n| !lista.iter().any(|r| &r.nome == n)).unwrap_or(base);
                            lista.push(Requisito { nome: n, tipo: m });
                            mudou = true;
                            ui.close();
                        }
                    }
                }).response.on_hover_text(tr("Acrescenta um requisito: massa, furos, folga, interferência, caixa ou propagação de parâmetro."));
                let rot = if e.velho && !e.resultados.is_empty() { tr("Verificar de novo") } else { tr("Verificar") };
                if ui.add_enabled(!lista.is_empty(), egui::Button::new(RichText::new(rot).strong())).on_hover_text(tr("Confere cada requisito na peça e mostra o que foi medido. Folga e propagação recalculam cópias da peça.")).clicked() {
                    conferir = true;
                }
                if ui.button(tr("Fechar")).clicked() {
                    fechar = true;
                }
            });
            if !e.resultados.is_empty() && !e.velho {
                let falhas = e.resultados.iter().filter(|r| !r.ok).count();
                let (txt, cor) = if falhas == 0 { (tr("Todos os requisitos cumpridos.").to_string(), crate::theme::st_ok()) } else { (format!("{falhas} {}", tr("requisito(s) sem cumprir.")), crate::theme::st_err()) };
                ui.label(RichText::new(txt).strong().color(cor));
            }
            let mut tirar = None;
            egui::ScrollArea::vertical().max_height(300.).show(ui, |ui| {
                for (i, r) in lista.iter_mut().enumerate() {
                    let res = e.resultados.iter().find(|x| x.nome == r.nome && !e.velho);
                    ui.horizontal(|ui| {
                        let (marca, cor) = match res {
                            Some(x) if x.ok => ("✔", crate::theme::st_ok()),
                            Some(_) => ("✖", crate::theme::st_err()),
                            None => ("•", ui.visuals().weak_text_color()),
                        };
                        ui.label(RichText::new(marca).strong().color(cor)).on_hover_text(tr("Resultado da última conferência"));
                        mudou |= ui.add(egui::TextEdit::singleline(&mut r.nome).desired_width(150.)).on_hover_text(tr("Nome do requisito, como aparece no resultado.")).changed();
                        ui.label(RichText::new(tr(r.tipo.rotulo())).small().weak());
                        if ui.small_button("✕").on_hover_text(tr("Tirar este requisito")).clicked() {
                            tirar = Some(i);
                        }
                    });
                    ui.indent(("verif", i), |ui| {
                        mudou |= campos(ui, &mut r.tipo, &params);
                        if let Some(x) = res {
                            let cor = if x.ok { crate::theme::st_ok() } else { crate::theme::st_err() };
                            ui.label(RichText::new(crate::app::loc(format!("{} · {}", x.medido, x.detalhe))).small().color(cor));
                        }
                    });
                    ui.separator();
                }
            });
            if let Some(i) = tirar {
                lista.remove(i);
                mudou = true;
            }
        });
        if let Some(m) = saida.movido {
            app.settings.guardar_painel(&cfg.nome, m);
        }
        if mudou {
            e.velho = true;
        }
        if fechar {
            e.aberto = false;
        }
    });
    if mudou {
        app.open.doc.requisitos = lista;
        app.push_undo_pub(antes_doc);
    }
    if conferir {
        let r = verificar(&app.open.doc, &app.open.doc.requisitos);
        ST.with(|st| {
            let mut e = st.borrow_mut();
            e.resultados = r;
            e.velho = false;
        });
    }
}
