// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painéis do projeto que vivem fora do `app.rs` (B47, B93, B94, B265).
//!
//! A decisão (máscara, plano, conferência, relações) mora em `fenkra_project`; aqui fica só a
//! interface. O estado de cada diálogo fica na memória temporária do egui, para não crescer o `App`.
//! Cores só por tema (`ui.visuals()`), como manda o padrão de interface.

use super::*;
use fenkra_project::pn::{self, AmbientePn};

/// Campos em edição do ambiente de numeração.
#[derive(Clone, Default)]
struct BufAmbiente {
    origem: String,
    nome: String,
    prefixo: String,
    mascara: String,
}

impl App {
    /// Seção "Numeração de PN" do painel do projeto: ambiente ativo e máscara de cada ambiente (B47).
    pub(super) fn painel_pn(&mut self, ui: &mut egui::Ui) {
        let Some(p) = self.project.as_ref() else { return };
        let ambientes: Vec<AmbientePn> = p.index.pn_ambientes.clone();
        let ativo = p.index.pn_ambiente.clone();
        let previsto = p.pn_previsto();
        let mut escolher: Option<String> = None;
        let mut salvar: Option<AmbientePn> = None;
        egui::CollapsingHeader::new(tr("Numeração de PN")).id_salt("pn_painel").show(ui, |ui| {
            ui.label(RichText::new(format!("{}: {}", tr("Próximo PN"), previsto)).weak());
            ui.horizontal(|ui| {
                ui.label(tr("Ambiente"));
                egui::ComboBox::from_id_salt("pn_amb").selected_text(if ativo.is_empty() { tr("(simples)").to_string() } else { ativo.clone() }).show_ui(ui, |ui| {
                    if ui.selectable_label(ativo.is_empty(), tr("(simples)")).clicked() {
                        escolher = Some(String::new());
                    }
                    for a in &ambientes {
                        if ui.selectable_label(a.nome == ativo, &a.nome).clicked() {
                            escolher = Some(a.nome.clone());
                        }
                    }
                });
            })
            .response
            .on_hover_text(tr("Cada ambiente (produção, protótipo...) tem máscara e contador próprios"));

            let id = ui.make_persistent_id("pn_buf");
            let mut buf: BufAmbiente = ui.data(|d| d.get_temp(id)).unwrap_or_default();
            // ao trocar o ambiente ativo, o formulário passa a mostrar o dele
            if buf.origem != ativo {
                buf = match ambientes.iter().find(|a| a.nome == ativo) {
                    Some(a) => BufAmbiente { origem: ativo.clone(), nome: a.nome.clone(), prefixo: a.prefixo.clone(), mascara: a.mascara.clone() },
                    None => BufAmbiente { origem: ativo.clone(), nome: String::new(), prefixo: "FK".into(), mascara: "{PREFIXO}-{TIPO}-{SEQ:4}".into() },
                };
            }
            ui.horizontal(|ui| {
                ui.label(tr("Nome"));
                ui.add(egui::TextEdit::singleline(&mut buf.nome).desired_width(110.).hint_text(tr("novo ambiente")));
                ui.label(tr("Prefixo"));
                ui.add(egui::TextEdit::singleline(&mut buf.prefixo).desired_width(50.));
            });
            let r = ui.add(egui::TextEdit::singleline(&mut buf.mascara).desired_width(f32::INFINITY).hint_text("{PREFIXO}-{ANO}-{SEQ:4}"));
            let valida = pn::validar(&buf.mascara);
            let exemplo = pn::exemplo(&buf.mascara);
            r.on_hover_text(format!(
                "{}\n{}: {}",
                tr("Campos: {PREFIXO} {ANO} {TIPO} {AMB} {CAMPO:nome} {SEQ:N}. Um {SEQ:N} é obrigatório."),
                tr("Exemplo"),
                exemplo.clone().unwrap_or_else(|e| e.to_string())
            ));
            match (&valida, &exemplo) {
                (Err(e), _) => {
                    ui.label(RichText::new(e.to_string()).color(ui.visuals().error_fg_color).small());
                }
                (_, Ok(ex)) => {
                    ui.label(RichText::new(format!("{}: {}", tr("Exemplo"), ex)).weak().small());
                }
                _ => {}
            }
            if ui.add_enabled(valida.is_ok() && !buf.nome.trim().is_empty(), egui::Button::new(tr("Salvar ambiente"))).clicked() {
                salvar = Some(AmbientePn::novo(buf.nome.trim(), buf.prefixo.trim(), buf.mascara.trim()));
            }
            ui.data_mut(|d| d.insert_temp(id, buf));
        });
        if let Some(a) = salvar {
            let nome = a.nome.clone();
            let p = self.project.as_mut().unwrap();
            match p.definir_ambiente_pn(a).and_then(|_| p.ativar_ambiente_pn(&nome)) {
                Ok(()) => self.toast(format!("Ambiente {nome} salvo"), false),
                Err(e) => self.toast(e.to_string(), true),
            }
        } else if let Some(n) = escolher {
            if let Err(e) = self.project.as_mut().unwrap().ativar_ambiente_pn(&n) {
                self.toast(e.to_string(), true);
            }
        }
    }
}
