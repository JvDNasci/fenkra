// SPDX-License-Identifier: LGPL-2.1-or-later
//! Janelas de diagnóstico e de pais e filhos (B69) e o filtro da árvore.
//!
//! A lógica (quem falhou primeiro, quem é pai de quem, o que o filtro aceita) está em
//! `crate::diagnostico`; aqui só desenhamos e ligamos os botões às ações que a árvore já tem.

use super::*;
use crate::diagnostico as dg;

#[derive(Default)]
pub struct Estado {
    /// painel Diagnóstico aberto
    pub painel: bool,
    /// objeto cuja janela Pais e filhos está aberta
    pub pais_filhos: Option<Id>,
}

/// Caixa do filtro da árvore (`:erro`, `:suprimido`, `:oculto` ou parte do nome); devolve a consulta.
pub fn caixa_de_filtro(ui: &mut egui::Ui) -> String {
    let chave = egui::Id::new("fenkra_filtro_arvore");
    let mut q: String = ui.ctx().data(|m| m.get_temp(chave)).unwrap_or_default();
    ui.horizontal(|ui| {
        let larg = ui.available_width() - 28.;
        crate::tree_ui::campo_discreto(ui, |ui| {
            ui.add(egui::TextEdit::singleline(&mut q).desired_width(larg).hint_text(tr("Filtrar: nome, :erro, :suprimido, :oculto")));
        });
        if !q.is_empty() && ui.small_button("✕").on_hover_text(tr("Limpar o filtro")).clicked() {
            q.clear();
        }
    });
    ui.ctx().data_mut(|m| m.insert_temp(chave, q.clone()));
    q
}

/// Consulta atual do filtro (para a linha decidir se aparece).
pub fn filtro_atual(ctx: &egui::Context) -> dg::Filtro {
    let q: String = ctx.data(|m| m.get_temp(egui::Id::new("fenkra_filtro_arvore"))).unwrap_or_default();
    dg::Filtro::analisar(&q)
}

impl App {
    pub fn abrir_diagnostico(&mut self) {
        self.diag.painel = true;
    }

    pub fn abrir_pais_filhos(&mut self, id: Id) {
        self.diag.pais_filhos = Some(id);
    }

    fn realcar(&mut self, id: Id) {
        self.sel = if self.open.doc.feature(id).is_some() { Sel::Feature(id) } else { Sel::Ref(id) };
        self.scene_dirty = true;
    }

    fn alternar_supressao(&mut self, id: Id) {
        if let Some(f) = self.open.doc.feature_mut(id) {
            f.suppressed = !f.suppressed;
        }
    }

    /// Chamado a cada quadro, junto dos demais diálogos.
    pub fn janelas_de_diagnostico(&mut self, ctx: &egui::Context) {
        self.janela_diagnostico(ctx);
        self.janela_pais_filhos(ctx);
    }

    fn janela_diagnostico(&mut self, ctx: &egui::Context) {
        if !self.diag.painel {
            return;
        }
        let falhas = dg::falhas(&self.open.doc, |i| self.status_of(i));
        let mut aberto = true;
        let mut acao: Option<(u8, Id)> = None;
        egui::Window::new(tr("Diagnóstico")).open(&mut aberto).default_width(460.).show(ctx, |ui| {
            if falhas.is_empty() {
                ui.label(tr("Nenhum comando com erro."));
                return;
            }
            ui.label(RichText::new(tr("Do primeiro ao último: corrija a causa e as consequências somem sozinhas.")).weak().small());
            ui.add_space(4.);
            egui::ScrollArea::vertical().max_height(360.).show(ui, |ui| {
                for f in &falhas {
                    let nome = self.open.doc.name_of(f.id);
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("✖ {nome}")).strong().color(crate::theme::st_err()));
                            if let Some(c) = f.causa {
                                ui.label(RichText::new(format!("{} {}", tr("consequência de"), self.open.doc.name_of(c))).weak().small());
                            } else {
                                ui.label(RichText::new(tr("causa")).small());
                            }
                        });
                        ui.label(RichText::new(&f.mensagem).small());
                        ui.horizontal_wrapped(|ui| {
                            for (k, rot) in [(0u8, "Realçar"), (1, "Editar"), (2, "Desativar"), (3, "Apagar"), (4, "Pais e filhos")] {
                                if ui.small_button(tr(rot)).clicked() {
                                    acao = Some((k, f.id));
                                }
                            }
                            if let Some(c) = f.causa {
                                if ui.small_button(tr("Ir à causa")).clicked() {
                                    acao = Some((0, c));
                                }
                            }
                        });
                    });
                }
            });
        });
        if !aberto {
            self.diag.painel = false;
        }
        match acao {
            Some((0, id)) => self.realcar(id),
            Some((1, id)) => self.start_session(id, false, None),
            Some((2, id)) => self.alternar_supressao(id),
            Some((3, id)) => self.ask_delete(id),
            Some((4, id)) => self.abrir_pais_filhos(id),
            _ => {}
        }
    }

    fn janela_pais_filhos(&mut self, ctx: &egui::Context) {
        let Some(id) = self.diag.pais_filhos else { return };
        if self.open.doc.feature(id).is_none() && self.open.doc.reference(id).is_none() {
            self.diag.pais_filhos = None;
            return;
        }
        let (pais, filhos) = (dg::pais(&self.open.doc, id), dg::filhos(&self.open.doc, id));
        let mut aberto = true;
        let mut ir: Option<Id> = None;
        let titulo = format!("{} - {}", tr("Pais e filhos"), self.open.doc.name_of(id));
        egui::Window::new(titulo).id(egui::Id::new("fenkra_pais_filhos")).open(&mut aberto).default_width(340.).show(ctx, |ui| {
            for (rot, lista) in [(tr("Pais (de que depende)"), &pais), (tr("Filhos (quem depende dele)"), &filhos)] {
                ui.label(RichText::new(rot).strong());
                if lista.is_empty() {
                    ui.label(RichText::new(tr("nenhum")).weak());
                }
                for &o in lista {
                    ui.horizontal(|ui| {
                        let erro = matches!(self.status_of(o), Some(Status::Error(_)));
                        let txt = format!("{} ({})", self.open.doc.name_of(o), tr(dg::tipo_de(&self.open.doc, o)));
                        ui.label(if erro { RichText::new(format!("✖ {txt}")).color(crate::theme::st_err()) } else { RichText::new(txt) });
                        if ui.small_button(tr("Realçar")).clicked() {
                            ir = Some(o);
                        }
                    });
                }
                ui.add_space(6.);
            }
            ui.label(RichText::new(tr("Apagar este objeto leva os filhos junto; edite ou substitua os pais antes de apagar.")).weak().small());
        });
        if !aberto {
            self.diag.pais_filhos = None;
        }
        if let Some(o) = ir {
            self.realcar(o);
        }
    }
}
