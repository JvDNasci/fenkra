// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estilos de chapa na interface (B283): seletor no topo da janela de parâmetros, "salvar como
//! estilo" e o cadeado de sobrescrita, na janela e nos comandos.
//!
//! A janela edita uma cópia efetiva (estilo já aplicado) e devolve as mudanças em [`Acoes`]; quem
//! grava no documento é [`aplica`], para a lógica de herança ficar toda em `fenkra_core::estilo_chapa`.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::{Document, EstiloRegras};

/// O que a janela pediu neste quadro.
#[derive(Default)]
pub struct Acoes {
    /// estilo escolhido no seletor (vazio = sem estilo)
    pub escolher: Option<String>,
    /// nome para salvar os valores atuais como estilo
    pub salvar: Option<String>,
    /// chaves cujo cadeado foi clicado (voltam ao valor do estilo)
    pub restaurar: Vec<String>,
}

/// Seletor de estilo e "salvar como estilo", no topo da janela de parâmetros.
pub fn topo(ui: &mut egui::Ui, estilos: &[EstiloRegras], atual: &str, acoes: &mut Acoes) {
    crate::editor::section(ui, "Estilo da chapa");
    ui.horizontal(|ui| {
        let texto = if atual.is_empty() { tr("(sem estilo)").to_string() } else { atual.to_string() };
        egui::ComboBox::from_id_salt("chapa_estilo").selected_text(texto).width(200.).show_ui(ui, |ui| {
            if ui.selectable_label(atual.is_empty(), tr("(sem estilo)")).clicked() {
                acoes.escolher = Some(String::new());
            }
            for e in estilos {
                if ui.selectable_label(e.nome == atual, &e.nome).clicked() {
                    acoes.escolher = Some(e.nome.clone());
                }
            }
        }).response.on_hover_text(tr("Estilo salvo com material, espessura, raio por fórmula, compensação e alívios. Trocar o estilo recalcula a peça e a tabela de dobras."));
    });
    let k = egui::Id::new("estilo_chapa_nome");
    let mut nome: String = ui.ctx().data(|m| m.get_temp(k)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut nome).hint_text(tr("Nome do estilo")).desired_width(150.));
        let ok = !nome.trim().is_empty();
        if ui.add_enabled(ok, egui::Button::new(tr("Salvar como estilo"))).on_hover_text(tr("Guarda os valores atuais da chapa num estilo com este nome (substitui se já existir) e passa a usá-lo.")).clicked() {
            acoes.salvar = Some(nome.trim().to_string());
            nome.clear();
        }
    });
    ui.ctx().data_mut(|m| m.insert_temp(k, nome));
    if !atual.is_empty() {
        ui.label(RichText::new(tr("Campos com cadeado 🔒 sobrescrevem o estilo; clique no cadeado para voltar ao valor dele.")).weak().small());
    }
}

/// Cadeado ao lado de um campo que sobrescreve o estilo (nada se o campo é herdado).
pub fn cadeado(ui: &mut egui::Ui, atual: &str, sobrescritas: &[String], chave: &str, acoes: &mut Acoes) {
    if atual.is_empty() || !sobrescritas.iter().any(|c| c == chave) {
        return;
    }
    if ui.small_button("🔒").on_hover_text(format!("{} '{}'. {}", tr("Este campo sobrescreve o estilo"), atual, tr("Clique para voltar ao valor do estilo."))).clicked() {
        acoes.restaurar.push(chave.to_string());
    }
}

/// Cadeado de um comando que sobrescreve o estilo da chapa (raio ou alívio próprio da aba).
/// Devolve `true` quando foi clicado: o comando deve voltar a herdar.
pub fn cadeado_comando(ui: &mut egui::Ui, doc: &Document, sobrescreve: bool) -> bool {
    let Some(s) = &doc.sheet else { return false };
    if !sobrescreve || s.style.is_empty() {
        return false;
    }
    ui.small_button("🔒").on_hover_text(format!("{} '{}'. {}", tr("Este campo sobrescreve o estilo"), s.style, tr("Clique para voltar ao valor do estilo."))).clicked()
}

/// Grava no documento o que a janela pediu.
pub fn aplica(doc: &mut Document, acoes: Acoes) {
    if let Some(s) = doc.sheet.as_mut() {
        for c in &acoes.restaurar {
            s.restaura(c);
        }
        if let Some(nome) = &acoes.escolher {
            let existe = nome.is_empty() || doc.sheet_styles.iter().any(|e| &e.nome == nome);
            if existe {
                if let Some(s) = doc.sheet.as_mut() {
                    s.escolhe_estilo(nome);
                }
            }
        }
    }
    if let Some(nome) = acoes.salvar {
        doc.salva_estilo_chapa(&nome);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::SheetParams;

    #[test]
    fn acoes_gravam_estilo_e_restauram_cadeado() {
        let mut d = Document::new("x");
        d.sheet = Some(SheetParams { thickness: "2".into(), radius: "1.5*espessura".into(), ..Default::default() });
        aplica(&mut d, Acoes { salvar: Some("Meu".into()), ..Default::default() });
        assert_eq!(d.sheet.as_ref().unwrap().style, "Meu");
        d.sheet.as_mut().unwrap().overrides.push("raio".into());
        aplica(&mut d, Acoes { restaurar: vec!["raio".into()], ..Default::default() });
        assert!(d.sheet.as_ref().unwrap().overrides.is_empty());
        // estilo inexistente é ignorado; vazio tira o estilo
        aplica(&mut d, Acoes { escolher: Some("Fantasma".into()), ..Default::default() });
        assert_eq!(d.sheet.as_ref().unwrap().style, "Meu");
        aplica(&mut d, Acoes { escolher: Some(String::new()), ..Default::default() });
        assert_eq!(d.sheet.as_ref().unwrap().style, "");
    }
}
