// SPDX-License-Identifier: LGPL-2.1-or-later
//! Página "Camadas e perfil" das configurações (B100): de onde vem cada opção, o que está travado,
//! opções que ficam no arquivo do documento e exportar/importar o perfil de interface.

use crate::config_camadas as cc;
use crate::i18n::tr;
use crate::settings::Settings;
use eframe::egui::{self, RichText};
use fenkra_core::Document;

pub fn pagina(ui: &mut egui::Ui, s: &mut Settings, doc: &mut Document) {
    let msg_key = egui::Id::new("config_camadas_msg");

    ui.label(RichText::new(tr("Camadas")).strong());
    ui.label(RichText::new(tr("Da mais fraca para a mais forte: programa, empresa, projeto, usuário. Uma camada pode travar uma opção: dali para cima ninguém muda o valor.")).weak().small());
    let (tem_empresa, tem_projeto) = cc::camadas_presentes();
    egui::Grid::new("config_camadas_grade").num_columns(2).spacing([12., 4.]).show(ui, |ui| {
        let linha = |ui: &mut egui::Ui, nome: &str, onde: String, ativa: bool| {
            ui.label(tr(nome));
            ui.label(RichText::new(format!("{} · {onde}", tr(if ativa { "em uso" } else { "não encontrada" }))).small().weak());
            ui.end_row();
        };
        linha(ui, "Empresa", cc::caminho_empresa().map(|p| p.display().to_string()).unwrap_or_default(), tem_empresa);
        linha(ui, "Projeto", tr("pasta .fenkra do projeto aberto").to_string(), tem_projeto);
        linha(ui, "Usuário", cc::caminho_usuario().map(|p| p.display().to_string()).unwrap_or_default(), true);
    });
    let travadas = cc::travadas();
    if travadas.is_empty() {
        ui.label(RichText::new(tr("Nenhuma opção travada.")).weak().small());
    } else {
        ui.label(RichText::new(tr("Opções travadas")).small().strong());
        for (k, quem) in &travadas {
            ui.label(RichText::new(format!("🔒 {k} — {} {}", tr("travada pela camada"), tr(&quem.clone()))).small());
        }
    }

    ui.separator();
    ui.label(RichText::new(tr("Opções do documento")).strong());
    ui.label(RichText::new(tr("Cotagem, grade do sketch, anotações 3D e qualidade da malha podem ficar gravadas no arquivo da peça: quem abrir vê do mesmo jeito.")).weak().small());
    let mut so_doc = cc::documento_sobrepoe();
    if ui
        .checkbox(&mut so_doc, tr("Estas opções valem só para este documento"))
        .on_hover_text(tr("Ligado: os ajustes de cotagem, grade, anotações 3D e qualidade vão para o arquivo da peça. Desligado: ficam nas suas configurações."))
        .changed()
    {
        cc::somente_documento(so_doc, s, doc);
    }
    ui.label(RichText::new(cc::CHAVES_DOC.join(", ")).small().weak());

    ui.separator();
    ui.label(RichText::new(tr("Perfil de interface")).strong());
    ui.label(RichText::new(tr("Tema, atalhos, barras, painéis e navegação num arquivo para levar a outro computador. Não leva gráficos, recentes nem o que depende da máquina.")).weak().small());
    ui.horizontal(|ui| {
        if ui.button(tr("📤 Exportar perfil…")).clicked() {
            if let Some(p) = rfd::FileDialog::new().set_file_name("perfil-fenkra.json").add_filter("Perfil Fenkra", &["json"]).save_file() {
                let m = match std::fs::write(&p, cc::exportar_perfil(s)) {
                    Ok(()) => format!("{} {}", tr("Perfil gravado em"), p.display()),
                    Err(e) => format!("{}: {e}", tr("Não consegui gravar o perfil")),
                };
                ui.ctx().data_mut(|d| d.insert_temp(msg_key, m));
            }
        }
        if ui.button(tr("📥 Importar perfil…")).clicked() {
            if let Some(p) = rfd::FileDialog::new().add_filter("Perfil Fenkra", &["json"]).pick_file() {
                let m = match std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|t| cc::importar_perfil(&t, s)) {
                    Ok(r) => {
                        let mut m = format!("{} {}", r.aplicadas, tr("opções importadas"));
                        if !r.travadas.is_empty() {
                            m += &format!(" · {} {}", r.travadas.len(), tr("travadas pela empresa ou projeto ficaram de fora"));
                        }
                        if !r.ignoradas.is_empty() {
                            m += &format!(" · {} {}", r.ignoradas.len(), tr("ignoradas (não são de interface)"));
                        }
                        if let Some(a) = r.aviso {
                            m += &format!(" · {a}");
                        }
                        m
                    }
                    Err(e) => e,
                };
                ui.ctx().data_mut(|d| d.insert_temp(msg_key, m));
            }
        }
    });
    if let Some(m) = ui.ctx().data(|d| d.get_temp::<String>(msg_key)) {
        ui.label(RichText::new(m).small());
    }

    ui.separator();
    ui.label(RichText::new(tr("Arquivo de configurações")).strong());
    ui.label(RichText::new(tr("Cada gravação guarda a anterior como backup (config.json.bak). Se o arquivo estragar, o programa restaura o backup e avisa.")).weak().small());
}
