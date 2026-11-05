// SPDX-License-Identifier: LGPL-2.1-or-later
//! Importar STEP (B229): com projeto aberto, o original vai para o cofre e reimportar o mesmo nome
//! compara o hash; sem projeto, o documento só abre, já com o selo de origem.
//!
//! A decisão sobre o arquivo mudado vive em `fenkra_project::importacao`; aqui fica só a pergunta.

use super::*;
use fenkra_project::importacao::Reimportacao;
use fenkra_solidos::importacao::{importar_step, Importacao};

/// Arquivo que mudou desde a importação, esperando a pessoa escolher o que fazer.
pub struct Pendente {
    id: u64,
    caminho: PathBuf,
    importacao: Importacao,
}

impl App {
    pub(super) fn importar_step(&mut self) {
        let Some(caminho) = rfd::FileDialog::new().add_filter("STEP", &["step", "stp"]).pick_file() else { return };
        match importar_step(&caminho.to_string_lossy()) {
            Ok(i) => self.receber_importacao(caminho, i),
            Err(e) => self.toast(format!("Falha ao importar: {e}"), true),
        }
    }

    fn receber_importacao(&mut self, caminho: PathBuf, importacao: Importacao) {
        let Some(p) = &mut self.project else {
            self.load_doc(importacao.doc, None, None);
            self.started = true;
            return;
        };
        let Some(selo) = importacao.doc.origem.clone() else { return };
        match p.reimportacao(&selo.nome, &selo.hash) {
            Reimportacao::Nova => {
                let r = p.adicionar_importado(&caminho, importacao.doc, &importacao.relatorio);
                self.abrir_importado(r);
            }
            Reimportacao::Igual { id } => {
                self.toast(tr("Arquivo igual ao já importado.").to_string(), false);
                self.open_entry(id, None);
            }
            Reimportacao::Mudou { id, .. } => self.importacao_pendente = Some(Pendente { id, caminho, importacao }),
        }
        self.started = true;
    }

    fn abrir_importado(&mut self, r: fenkra_project::PResult<u64>) {
        match r {
            Ok(id) => {
                self.left = LeftTab::Projeto;
                self.open_entry(id, None);
            }
            Err(e) => self.toast(e.to_string(), true),
        }
    }

    pub(super) fn importacao_window(&mut self, ctx: &egui::Context) {
        let Some(pendente) = &self.importacao_pendente else { return };
        let nome = pendente.importacao.doc.origem.as_ref().map(|o| o.nome.clone()).unwrap_or_default();
        let mut escolha = None;
        egui::Window::new(tr("Arquivo importado mudou")).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.label(RichText::new(nome).strong());
            ui.label(tr("O conteúdo não é o mesmo da última importação."));
            ui.add_space(6.);
            ui.horizontal(|ui| {
                if ui.button(tr("Atualizar")).on_hover_text(tr("Troca a geometria e mantém os comandos feitos depois dela.")).clicked() {
                    escolha = Some(true);
                }
                if ui.button(tr("Criar derivado")).on_hover_text(tr("Cria outro documento e deixa o atual como está.")).clicked() {
                    escolha = Some(false);
                }
                if ui.button(tr("Cancelar")).clicked() {
                    self.importacao_pendente = None;
                }
            });
        });
        let Some(atualizar) = escolha else { return };
        let (Some(Pendente { id, caminho, importacao }), Some(p)) = (self.importacao_pendente.take(), self.project.as_mut()) else { return };
        if atualizar {
            let r = p.atualizar_importado(id, &caminho, &importacao.doc, &importacao.relatorio).map(|()| id);
            self.abrir_importado(r);
        } else {
            let r = p.adicionar_derivado(id, &caminho, importacao.doc, &importacao.relatorio);
            self.abrir_importado(r);
        }
    }
}
