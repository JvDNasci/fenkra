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

    secao_fkv(ui);
    secao_documentos(ui);
}

/// Associar o `.fkv` ao Fenkra no Windows (B365): só com clique, em HKCU, e com desfazer.
fn secao_fkv(ui: &mut egui::Ui) {
    let msg_key = egui::Id::new("config_fkv_msg");
    ui.separator();
    ui.label(RichText::new(tr("Abrir o .fkv com duplo clique")).strong());
    ui.label(RichText::new(tr("O .fkv é um pacote: o navegador não o abre sozinho. Associá-lo faz o duplo clique abrir o modelo no navegador, pelo Fenkra, sem abrir a janela dele. Vale só para o seu usuário do Windows e não pede administrador.")).weak().small());
    let ligado = crate::assoc_fkv::registrado();
    ui.label(RichText::new(tr(if ligado { "O .fkv abre pelo Fenkra com duplo clique." } else { "O .fkv não está associado ao Fenkra." })).small());
    ui.horizontal(|ui| {
        if ui.add_enabled(cfg!(windows), egui::Button::new(tr("Associar o .fkv ao Fenkra"))).on_hover_text(tr("Grava duas chaves em HKCU\\Software\\Classes. Só acontece com este clique.")).clicked() {
            let m = match crate::assoc_fkv::registrar() {
                Ok(()) => tr("Associação feita.").to_string(),
                Err(e) => format!("{}: {e}", tr("Não consegui mudar a associação")),
            };
            ui.ctx().data_mut(|d| d.insert_temp(msg_key, m));
        }
        if ui.add_enabled(cfg!(windows) && ligado, egui::Button::new(tr("Desfazer a associação"))).on_hover_text(tr("Apaga as duas chaves que o botão ao lado criou.")).clicked() {
            let m = match crate::assoc_fkv::desfazer() {
                Ok(()) => tr("Associação desfeita.").to_string(),
                Err(e) => format!("{}: {e}", tr("Não consegui mudar a associação")),
            };
            ui.ctx().data_mut(|d| d.insert_temp(msg_key, m));
        }
    });
    if let Some(m) = ui.ctx().data(|d| d.get_temp::<String>(msg_key)) {
        ui.label(RichText::new(m).small());
    }
}

/// Associar `.fkp`, `.fka` e `.fkd` ao Fenkra (B389): só com clique, em HKCU, e com desfazer.
fn secao_documentos(ui: &mut egui::Ui) {
    let msg_key = egui::Id::new("config_docs_msg");
    ui.separator();
    ui.label(RichText::new(tr("Abrir peças, montagens e desenhos com duplo clique")).strong());
    ui.label(RichText::new(tr("Associa .fkp, .fka e .fkd ao Fenkra: o duplo clique no arquivo abre o programa já com ele. Vale só para o seu usuário do Windows e não pede administrador.")).weak().small());
    let ligado = crate::assoc_docs::registrado();
    ui.label(RichText::new(tr(if ligado { "Os arquivos do Fenkra abrem pelo programa com duplo clique." } else { "Os arquivos do Fenkra não estão associados ao programa." })).small());
    ui.horizontal(|ui| {
        if ui.add_enabled(cfg!(windows), egui::Button::new(tr("Associar arquivos do Fenkra"))).on_hover_text(tr("Grava seis chaves em HKCU\\Software\\Classes (três extensões e três classes). Só acontece com este clique.")).clicked() {
            let m = match crate::assoc_docs::registrar() {
                Ok(()) => tr("Associação feita.").to_string(),
                Err(e) => format!("{}: {e}", tr("Não consegui mudar a associação")),
            };
            ui.ctx().data_mut(|d| d.insert_temp(msg_key, m));
        }
        if ui.add_enabled(cfg!(windows) && ligado, egui::Button::new(tr("Desfazer a associação"))).on_hover_text(tr("Apaga as seis chaves que o botão ao lado criou.")).clicked() {
            let m = match crate::assoc_docs::desfazer() {
                Ok(()) => tr("Associação desfeita.").to_string(),
                Err(e) => format!("{}: {e}", tr("Não consegui mudar a associação")),
            };
            ui.ctx().data_mut(|d| d.insert_temp(msg_key, m));
        }
    });
    if let Some(m) = ui.ctx().data(|d| d.get_temp::<String>(msg_key)) {
        ui.label(RichText::new(m).small());
    }
}
