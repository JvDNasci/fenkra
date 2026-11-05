// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: resumo de confirmação do "Gerar montagem" (B50). Antes de gravar qualquer arquivo, a
//! janela mostra o que vai acontecer (peças, instâncias, pasta de destino, nomes dos arquivos e o que
//! será sobrescrito) com Confirmar e Cancelar. Os impedimentos (peça sem arquivo, alterações não
//! salvas, revisão congelada) aparecem ali, com o botão que resolve, em vez de avisos curtos.

use crate::app::App;
use crate::i18n::tr;
use crate::theme;
use eframe::egui::{self, RichText};
use fenkra_montagem::estrutura::{plano_do_documento, Plano};
use std::path::PathBuf;

/// O que o resumo mostra.
#[derive(Clone, Debug)]
pub struct Resumo {
    pub nome_peca: String,
    pub plano: Result<Plano, String>,
    /// motivos que impedem de confirmar (cada um com a ação que resolve, se houver)
    pub problemas: Vec<Problema>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problema {
    /// revisão congelada: abrir a versão de trabalho
    Congelada,
    /// a peça nunca foi salva (sem pasta para a montagem)
    SemArquivo,
    /// há alterações não salvas
    NaoSalva,
}

/// Arquivo da peça que serve de pai: no projeto, o de trabalho dentro da pasta do projeto.
fn arquivo_pai(app: &App) -> Option<PathBuf> {
    let pai = match (&app.project, app.open.entry) {
        (Some(p), Some(id)) => p.entry(id).ok().map(|e| p.root.join(&e.file)),
        _ => app.open.file.clone(),
    };
    pai.filter(|p| p.is_file())
}

/// Pasta de destino: `<nome>_montagem` ao lado do arquivo da peça.
fn pasta_de_destino(app: &App, pai: Option<&PathBuf>) -> PathBuf {
    let nome: String = app.open.doc.name.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' }).collect();
    let nome = if nome.is_empty() { "estrutura".to_string() } else { nome };
    let base = pai.and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_default();
    base.join(format!("{nome}_montagem"))
}

/// Monta o resumo do estado atual da peça (sem gravar nada).
pub fn montar(app: &App) -> Resumo {
    let pai = arquivo_pai(app);
    let mut problemas = vec![];
    if app.open.frozen.is_some() {
        problemas.push(Problema::Congelada);
    }
    if pai.is_none() {
        problemas.push(Problema::SemArquivo);
    } else if app.open.modified {
        problemas.push(Problema::NaoSalva);
    }
    let saida = pasta_de_destino(app, pai.as_ref());
    Resumo { nome_peca: app.open.doc.name.clone(), plano: plano_do_documento(&app.open.doc, &saida), problemas }
}

/// Comando "Gerar montagem": abre o resumo (nada é gravado até Confirmar).
pub fn abrir(app: &mut App) {
    app.st_asm_resumo = Some(montar(app));
}

/// Janela do resumo. Confirmar gera a montagem; Cancelar só fecha.
pub fn window(app: &mut App, ctx: &egui::Context) {
    let Some(r) = app.st_asm_resumo.clone() else { return };
    let mut open = true;
    let (mut confirmar, mut cancelar, mut salvar) = (false, false, false);
    egui::Window::new(tr("Gerar montagem da estrutura")).id(egui::Id::unique("janela_st_resumo")).open(&mut open).collapsible(false).resizable(true).default_size([520., 360.]).show(ctx, |ui| {
        ui.label(RichText::new(format!("{}: {}", tr("Peça"), r.nome_peca)).strong());
        match &r.plano {
            Err(e) => {
                ui.label(RichText::new(tr(e)).color(theme::st_err()));
            }
            Ok(p) => {
                egui::Grid::new("st_resumo_grid").num_columns(2).spacing([12., 4.]).show(ui, |ui| {
                    ui.label(tr("Peças"));
                    ui.label(format!("{}", p.grupos.len()));
                    ui.end_row();
                    ui.label(tr("Instâncias"));
                    ui.label(format!("{}", p.instancias()));
                    ui.end_row();
                    ui.label(tr("Pasta de destino"));
                    ui.vertical(|ui| {
                        ui.label(p.pasta.to_string_lossy().to_string());
                        if p.pasta_existe {
                            ui.label(RichText::new(tr("A pasta já existe: os arquivos com o mesmo nome serão atualizados.")).color(theme::st_warn()).small());
                        }
                    });
                    ui.end_row();
                    ui.label(tr("Montagem"));
                    ui.label(p.montagem.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
                    ui.end_row();
                });
                ui.separator();
                ui.label(RichText::new(tr("Arquivos das peças")).strong());
                egui::ScrollArea::vertical().max_height(180.).show(ui, |ui| {
                    egui::Grid::new("st_resumo_pecas").striped(true).num_columns(4).spacing([12., 3.]).show(ui, |ui| {
                        ui.label(RichText::new(tr("Item")).weak());
                        ui.label(RichText::new(tr("Arquivo")).weak());
                        ui.label(RichText::new(tr("Descrição")).weak());
                        ui.label(RichText::new(tr("Instâncias")).weak());
                        ui.end_row();
                        for g in &p.grupos {
                            ui.label(format!("{:02}", g.item));
                            let t = RichText::new(&g.arquivo);
                            ui.label(if g.existe { t.color(theme::st_warn()) } else { t }).on_hover_text(if g.existe { tr("Já existe: será sobrescrito.") } else { tr("Arquivo novo.") });
                            ui.label(&g.descricao);
                            ui.label(format!("× {}", g.quantidade));
                            ui.end_row();
                        }
                    });
                });
            }
        }
        if !r.problemas.is_empty() {
            ui.separator();
            for pb in &r.problemas {
                ui.horizontal_wrapped(|ui| {
                    let (txt, acao) = match pb {
                        Problema::Congelada => ("Revisão congelada: abra a versão de trabalho para gerar a montagem", None),
                        Problema::SemArquivo => ("A peça ainda não foi salva: a montagem é gravada ao lado do arquivo dela.", Some("Salvar")),
                        Problema::NaoSalva => ("A peça tem alterações não salvas: a montagem usa o arquivo gravado.", Some("Salvar")),
                    };
                    ui.label(RichText::new(format!("⚠ {}", tr(txt))).color(theme::st_warn()));
                    if let Some(a) = acao {
                        if ui.button(tr(a)).clicked() {
                            salvar = true;
                        }
                    }
                });
            }
        }
        ui.separator();
        ui.horizontal(|ui| {
            let pode = r.problemas.is_empty() && r.plano.is_ok();
            if ui.add_enabled(pode, egui::Button::new(RichText::new(tr("Confirmar")).strong()).min_size(egui::vec2(100., 26.))).on_hover_text(tr("Grava as peças e a montagem e abre o resultado.")).clicked() {
                confirmar = true;
            }
            if ui.add(egui::Button::new(tr("Cancelar")).min_size(egui::vec2(100., 26.))).clicked() {
                cancelar = true;
            }
        });
    });
    if salvar {
        app.save_pub();
        // depois de salvar, o resumo é refeito com o estado novo
        app.st_asm_resumo = Some(montar(app));
        return;
    }
    if confirmar {
        app.st_asm_resumo = None;
        crate::st_ui::generate_assembly(app);
        return;
    }
    if cancelar || !open {
        app.st_asm_resumo = None;
    }
}
