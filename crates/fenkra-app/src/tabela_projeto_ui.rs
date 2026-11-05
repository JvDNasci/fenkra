// SPDX-License-Identifier: LGPL-2.1-or-later
//! Tabela de projeto e configurações da peça (B102): linhas = variantes, colunas = parâmetros,
//! supressão de comandos e PN. Fica no painel de Parâmetros, logo abaixo da lista deles.
//!
//! A configuração ativa vale no recálculo (`Document::com_configuracao`); nome ativo que não existe
//! aparece em vermelho aqui e no aviso de parâmetros, e a peça calcula os valores base, nunca "a
//! primeira linha". O CSV é lido e escrito direto, sem planilha instalada.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::configuracoes::Coluna;
use fenkra_core::Document;

/// Título curto de uma coluna.
fn titulo(doc: &Document, c: &Coluna) -> String {
    match c {
        Coluna::Pn => "PN".into(),
        Coluna::Parametro(n) => n.clone(),
        Coluna::Supressao(id) => format!("{} {}", tr("suprimir"), doc.name_of(*id)),
    }
}

fn aviso(ui: &egui::Ui) -> egui::Id {
    ui.make_persistent_id("config_aviso")
}

/// Painel da tabela de projeto.
pub fn tabela_projeto(ui: &mut egui::Ui, doc: &mut Document) {
    ui.add_space(8.);
    ui.separator();
    ui.heading(tr("Tabela de projeto"));
    ui.label(RichText::new(tr("Cada linha é uma variante da peça. Célula vazia usa o valor base. A montagem escolhe a variante pelo nome.")).weak().small());

    // configuração ativa
    let nomes = doc.tabela.nomes();
    let existe = doc.tabela.ativa.is_empty() || nomes.contains(&doc.tabela.ativa);
    ui.horizontal(|ui| {
        ui.label(tr("Ativa"));
        let atual = if doc.tabela.ativa.is_empty() { tr("(valores base)").to_string() } else { doc.tabela.ativa.clone() };
        egui::ComboBox::from_id_salt("config_ativa").selected_text(atual).show_ui(ui, |ui| {
            ui.selectable_value(&mut doc.tabela.ativa, String::new(), tr("(valores base)"));
            for n in &nomes {
                ui.selectable_value(&mut doc.tabela.ativa, n.clone(), n);
            }
        });
    });
    if !existe {
        let msg = doc.com_configuracao(&doc.tabela.ativa).err().map(|e| e.to_string()).unwrap_or_default();
        ui.label(RichText::new(format!("⚠ {msg}")).color(crate::theme::st_err()));
    }

    // grade: uma linha por configuração
    let ncol = doc.tabela.colunas.len();
    let mut tirar_linha = None;
    let mut tirar_coluna = None;
    if ncol > 0 || !doc.tabela.linhas.is_empty() {
        let titulos: Vec<String> = doc.tabela.colunas.iter().map(|c| titulo(doc, c)).collect();
        egui::ScrollArea::horizontal().id_salt("config_grade").show(ui, |ui| {
            egui::Grid::new("config_tabela").striped(true).num_columns(ncol + 2).show(ui, |ui| {
                ui.label(RichText::new(tr("Configuração")).strong());
                for (i, t) in titulos.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t).strong());
                        if ui.small_button("✕").on_hover_text(tr("Tirar esta coluna")).clicked() {
                            tirar_coluna = Some(i);
                        }
                    });
                }
                ui.label("");
                ui.end_row();
                for (li, l) in doc.tabela.linhas.iter_mut().enumerate() {
                    ui.add(egui::TextEdit::singleline(&mut l.nome).desired_width(90.));
                    l.celulas.resize(ncol, String::new());
                    for c in &mut l.celulas {
                        ui.add(egui::TextEdit::singleline(c).desired_width(70.));
                    }
                    if ui.small_button("🗑").on_hover_text(tr("Apagar esta configuração")).clicked() {
                        tirar_linha = Some(li);
                    }
                    ui.end_row();
                }
            });
        });
    }
    if let Some(i) = tirar_coluna {
        doc.tabela.colunas.remove(i);
        for l in &mut doc.tabela.linhas {
            if i < l.celulas.len() {
                l.celulas.remove(i);
            }
        }
    }
    if let Some(i) = tirar_linha {
        let nome = doc.tabela.linhas.remove(i).nome;
        if doc.tabela.ativa == nome {
            doc.tabela.ativa.clear();
        }
    }

    // nova configuração
    let chave = egui::Id::new("config_nova_nome");
    let mut nome: String = ui.ctx().data(|d| d.get_temp(chave)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut nome).hint_text(tr("nome da nova configuração")).desired_width(150.));
        let livre = !nome.trim().is_empty() && doc.tabela.linha(nome.trim()).is_none();
        if ui.add_enabled(livre, egui::Button::new(tr("➕ Configuração"))).on_hover_text(tr("Cria a linha com os valores atuais da peça para você mudar só o que varia.")).clicked() && doc.nova_configuracao(nome.trim()) {
            nome.clear();
        }
        // nova coluna
        ui.menu_button(tr("➕ Coluna"), |ui| {
            if ui.button("PN").clicked() {
                doc.nova_coluna_config(Coluna::Pn);
                ui.close();
            }
            ui.separator();
            let params: Vec<String> = doc.params.iter().map(|p| p.name.clone()).collect();
            for p in params {
                if ui.button(&p).clicked() {
                    doc.nova_coluna_config(Coluna::Parametro(p));
                    ui.close();
                }
            }
            ui.separator();
            let cmds: Vec<(fenkra_core::Id, String)> = doc.features.iter().filter(|f| !matches!(f.kind, fenkra_core::FeatureKind::Sketch(_))).map(|f| (f.id, f.name.clone())).collect();
            for (id, n) in cmds {
                if ui.button(format!("{} {n}", tr("suprimir"))).clicked() {
                    doc.nova_coluna_config(Coluna::Supressao(id));
                    ui.close();
                }
            }
        });
    });
    ui.ctx().data_mut(|d| d.insert_temp(chave, nome));

    // CSV
    let mut msg: Option<(bool, String)> = ui.ctx().data(|d| d.get_temp(aviso(ui)));
    ui.horizontal(|ui| {
        if ui.button(tr("⬆ Exportar CSV")).on_hover_text(tr("Grava a tabela em CSV (separador ;), que qualquer planilha abre.")).clicked() {
            if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name("tabela_de_projeto.csv").save_file() {
                msg = Some(match std::fs::write(&p, doc.tabela_para_csv()) {
                    Ok(()) => (true, format!("{} {}", tr("Gravado em"), p.display())),
                    Err(e) => (false, format!("{}: {e}", p.display())),
                });
            }
        }
        if ui.button(tr("⬇ Importar CSV")).on_hover_text(tr("Troca a tabela pelo CSV. Cabeçalho: Configuração; PN; nome de parâmetro; suprimir:<nome do comando>.")).clicked() {
            if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv", "txt"]).pick_file() {
                msg = Some(match std::fs::read_to_string(&p) {
                    Ok(t) => match doc.importar_tabela_csv(&t) {
                        Ok(()) => (true, format!("{} {}", tr("Importado de"), p.display())),
                        Err(e) => (false, e.to_string()),
                    },
                    Err(e) => (false, format!("{}: {e}", p.display())),
                });
            }
        }
    });
    if let Some((ok, m)) = &msg {
        let cor = if *ok { crate::theme::st_ok() } else { crate::theme::st_err() };
        ui.label(RichText::new(m).color(cor).small());
    }
    let chave_aviso = aviso(ui);
    ui.ctx().data_mut(|d| {
        if let Some(m) = msg {
            d.insert_temp(chave_aviso, m);
        }
    });
}
