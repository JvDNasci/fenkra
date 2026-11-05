// SPDX-License-Identifier: LGPL-2.1-or-later
//! Diálogos de gravação (B65 e B56): Salvar / Descartar / Cancelar, Sobrescrever / Salvar como /
//! Recarregar diante de mudança externa, e a janela do plano de salvamento (Ctrl+Shift+S).
//!
//! Este módulo é filho de `app` para enxergar o estado do `App` sem abrir os campos. A lógica de
//! decisão (quem entra no plano, em que ordem, o que bloqueia) vive em
//! `fenkra_project::plano`; a detecção de mudança externa em `fenkra_project::seguro`.

use super::*;
use fenkra_project::plano::{Candidato, Motivo, Plano};
use fenkra_project::seguro::{self, Assinatura, MudancaExterna};
use std::collections::HashMap;

/// Documentos que o plano conhece: a peça aberta, a montagem e o desenho.
const PECA: u64 = 0;
const MONTAGEM: u64 = 1;
const DESENHO: u64 = 2;

/// O que o usuário queria fazer quando o programa perguntou sobre as alterações.
#[derive(Clone, Debug, PartialEq)]
pub enum Acao {
    NovaPeca,
    Exemplo,
    AbrirDialogo,
    Importar,
    AbrirCaminho(PathBuf),
    Sair,
}

#[derive(Default)]
pub struct Estado {
    /// pergunta Salvar / Descartar / Cancelar em andamento
    confirmar: Option<Acao>,
    /// mudança externa detectada ao salvar o documento (id do plano)
    conflito: Option<u64>,
    /// o arquivo do documento ativo mudou por fora, visto na última checagem
    externo: bool,
    /// plano de salvamento aberto (Ctrl+Shift+S)
    plano: Option<Plano>,
    /// assinatura do arquivo como estava quando abrimos ou salvamos
    assin: HashMap<PathBuf, Assinatura>,
    /// caminho de cada documento no quadro anterior e se estava modificado (para reassinar após salvar)
    visto: HashMap<u64, (Option<PathBuf>, bool)>,
    /// última checagem de mudança externa
    checado: f64,
    /// o fechamento já foi decidido pelo usuário
    pub pode_fechar: bool,
}

impl Estado {
    /// Esquece as assinaturas (documento recarregado ou aberto de novo do disco).
    pub fn esquecer(&mut self) {
        self.assin.clear();
        self.visto.clear();
    }
}

impl App {
    fn caminho_de(&self, id: u64) -> Option<PathBuf> {
        match id {
            PECA => self.open.file.clone(),
            MONTAGEM => self.asm.as_ref().and_then(|s| s.file.clone()),
            _ => self.dwg.as_ref().and_then(|s| s.path.clone()),
        }
    }

    fn modificado(&self, id: u64) -> bool {
        match id {
            PECA => self.open.modified && self.open.frozen.is_none(),
            MONTAGEM => self.asm.as_ref().is_some_and(|s| s.modified),
            _ => self.dwg.as_ref().is_some_and(|s| s.modified),
        }
    }

    fn mudanca_externa(&self, id: u64) -> MudancaExterna {
        match self.caminho_de(id) {
            Some(p) => match self.salvar.assin.get(&p) {
                Some(a) => seguro::verificar(&p, a),
                None => MudancaExterna::Igual,
            },
            None => MudancaExterna::Igual,
        }
    }

    /// Qualquer documento com alteração por salvar.
    pub fn algo_modificado(&self) -> bool {
        [PECA, MONTAGEM, DESENHO].iter().any(|&i| self.modificado(i))
    }

    /// Mantém as assinaturas em dia: assina o que acabou de ser aberto ou salvo.
    fn atualizar_assinaturas(&mut self, agora: f64) {
        for id in [PECA, MONTAGEM, DESENHO] {
            let (p, m) = (self.caminho_de(id), self.modificado(id));
            let (p_ant, m_ant) = self.salvar.visto.get(&id).cloned().unwrap_or((None, false));
            let acabou_de_salvar = m_ant && !m;
            if let Some(path) = &p {
                if p_ant.as_ref() != Some(path) || acabou_de_salvar || !self.salvar.assin.contains_key(path) {
                    if let Ok(a) = seguro::assinar(path) {
                        self.salvar.assin.insert(path.clone(), a);
                    }
                }
            }
            self.salvar.visto.insert(id, (p, m));
        }
        let _ = agora;
    }

    // ------------------------------------------------------------ pedir confirmação

    /// Executa `acao` já, ou pergunta antes quando há alterações que seriam perdidas.
    pub fn com_confirmacao(&mut self, acao: Acao) {
        let perde = match acao {
            Acao::Sair => self.algo_modificado(),
            _ => self.modificado(PECA),
        };
        if perde {
            self.salvar.confirmar = Some(acao);
        } else {
            self.rodar_acao(acao);
        }
    }

    fn rodar_acao(&mut self, acao: Acao) {
        match acao {
            Acao::NovaPeca => {
                self.load_doc(Document::new("Peça"), None, None);
                self.started = true;
            }
            Acao::Exemplo => self.load_doc(fenkra_solidos::example_document(), None, None),
            Acao::Importar => {
                self.importar_arquivo();
                self.started = true;
            }
            Acao::AbrirDialogo => {
                self.open_file();
                self.started = true;
            }
            Acao::AbrirCaminho(p) => self.open_path(&p),
            Acao::Sair => self.salvar.pode_fechar = true,
        }
    }

    // ------------------------------------------------------------ salvar

    /// Id do documento que o Ctrl+S salva, conforme a bancada.
    fn documento_ativo(&self) -> u64 {
        use crate::settings::Bench;
        if self.settings.bench == Bench::Desenho && self.dwg.is_some() {
            DESENHO
        } else if self.in_asm() && self.asm.is_some() {
            MONTAGEM
        } else {
            PECA
        }
    }

    pub(super) fn documento_ativo_e_peca(&self) -> bool {
        self.documento_ativo() == PECA
    }

    pub(super) fn ativo_modificado(&self) -> bool {
        self.modificado(self.documento_ativo())
    }

    /// Sem ler o disco: a barra pergunta a cada quadro, e a checagem periódica já olhou o arquivo.
    pub(super) fn ativo_em_conflito(&self) -> bool {
        self.salvar.externo || self.salvar.conflito.is_some()
    }

    /// Salvar do documento ativo (menu e Ctrl+S): antes de gravar confere se o arquivo mudou por fora.
    pub fn salvar_ativo(&mut self) {
        let id = self.documento_ativo();
        self.salvar_documento(id);
    }

    fn salvar_documento(&mut self, id: u64) {
        if self.mudanca_externa(id) != MudancaExterna::Igual {
            self.salvar.conflito = Some(id);
            return;
        }
        self.gravar_sem_checar(id);
    }

    fn gravar_sem_checar(&mut self, id: u64) {
        match id {
            PECA => self.save(),
            MONTAGEM => crate::asm_ui::save(self),
            _ => crate::dwg_ui::salvar(self, false),
        }
    }

    /// Salva tudo o que estiver modificado e puder ser salvo; devolve se nada ficou pendente.
    fn salvar_pendentes(&mut self) -> bool {
        for id in [PECA, MONTAGEM, DESENHO] {
            if self.modificado(id) {
                self.salvar_documento(id);
            }
        }
        self.salvar.conflito.is_none() && !self.algo_modificado()
    }

    // ------------------------------------------------------------ plano

    fn candidatos(&self) -> Vec<Candidato> {
        let mut v = vec![];
        let base = |id: u64, nome: String, file: Option<PathBuf>, editado: bool, so_leitura: bool, tem_projeto: bool| Candidato {
            id,
            nome,
            pasta: file.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf())),
            novo: file.is_none() && !tem_projeto,
            editado,
            so_leitura,
            conflito: self.mudanca_externa(id) != MudancaExterna::Igual,
            ..Default::default()
        };
        v.push(base(PECA, self.open.doc.name.clone(), self.open.file.clone(), self.open.modified, self.open.frozen.is_some(), self.open.entry.is_some()));
        if let Some(s) = &self.asm {
            v.push(base(MONTAGEM, s.asm.name.clone(), s.file.clone(), s.modified, false, false));
        }
        if let Some(s) = &self.dwg {
            v.push(base(DESENHO, s.d.name.clone(), s.path.clone(), s.modified, false, false));
        }
        v
    }

    /// Abre a janela do plano (Ctrl+Shift+S).
    pub fn abrir_plano(&mut self) {
        let ativo = self.documento_ativo();
        self.salvar.plano = Some(Plano::montar(&self.candidatos(), Some(ativo)));
        if self.salvar.plano.as_ref().is_some_and(|p| p.itens.is_empty()) {
            self.salvar.plano = None;
            self.toast(tr("Nada a salvar"), false);
        }
    }

    fn gravar_item(&mut self, item: &fenkra_project::plano::ItemPlano) -> Result<(), String> {
        if item.motivo == Motivo::Novo {
            let (Some(pasta), nome) = (item.pasta.clone(), item.nome.trim()) else { return Err(tr("Escolha a pasta do arquivo novo").to_string()) };
            let ext = match item.id {
                PECA => "fkp",
                MONTAGEM => "fka",
                _ => "fkd",
            };
            let caminho = pasta.join(format!("{nome}.{ext}"));
            match item.id {
                PECA => self.open.file = Some(caminho),
                MONTAGEM => {
                    if let Some(s) = self.asm.as_mut() {
                        s.file = Some(caminho);
                    }
                }
                _ => {
                    if let Some(s) = self.dwg.as_mut() {
                        s.path = Some(caminho);
                    }
                }
            }
        }
        self.gravar_sem_checar(item.id);
        if self.modificado(item.id) {
            Err(tr("Não foi gravado").to_string())
        } else {
            Ok(())
        }
    }

    // ------------------------------------------------------------ janelas

    /// Chamado a cada quadro, junto dos demais diálogos.
    pub fn dialogos_de_gravacao(&mut self, ctx: &egui::Context) {
        let agora = ctx.input(|i| i.time);
        if agora - self.salvar.checado > 0.5 {
            self.salvar.checado = agora;
            self.atualizar_assinaturas(agora);
            self.salvar.externo = self.mudanca_externa(self.documento_ativo()) != MudancaExterna::Igual;
        }
        // pedido de fechar a janela com alterações por salvar
        if ctx.input(|i| i.viewport().close_requested()) && !self.salvar.pode_fechar && self.algo_modificado() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.salvar.confirmar = Some(Acao::Sair);
        }
        if self.salvar.pode_fechar {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.janela_confirmar(ctx);
        self.janela_conflito(ctx);
        self.janela_plano(ctx);
    }

    fn janela_confirmar(&mut self, ctx: &egui::Context) {
        let Some(acao) = self.salvar.confirmar.clone() else { return };
        let (mut salvar, mut descartar, mut cancelar) = (false, false, false);
        let nomes: Vec<String> = self.candidatos().into_iter().filter(|c| c.editado && (acao == Acao::Sair || c.id == PECA)).map(|c| c.nome).collect();
        egui::Modal::new(egui::Id::new("fenkra_confirmar_descarte")).show(ctx, |ui| {
            ui.set_max_width(420.);
            ui.heading(tr("Salvar as alterações?"));
            ui.label(tr("Estas alterações serão perdidas se você não salvar:"));
            for n in &nomes {
                ui.label(format!("  • {n}"));
            }
            ui.add_space(6.);
            ui.horizontal(|ui| {
                salvar = ui.button(tr("Salvar")).clicked();
                descartar = ui.button(tr("Descartar")).clicked();
                cancelar = ui.button(tr("Cancelar")).clicked();
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            cancelar = true;
        }
        if cancelar {
            self.salvar.confirmar = None;
        } else if descartar {
            self.salvar.confirmar = None;
            self.open.modified = false;
            if acao == Acao::Sair {
                if let Some(s) = self.asm.as_mut() {
                    s.modified = false;
                }
                if let Some(s) = self.dwg.as_mut() {
                    s.modified = false;
                }
            }
            self.rodar_acao(acao);
        } else if salvar {
            self.salvar.confirmar = None;
            let ok = if acao == Acao::Sair { self.salvar_pendentes() } else {
                self.salvar_documento(PECA);
                self.salvar.conflito.is_none() && !self.modificado(PECA)
            };
            if ok {
                self.rodar_acao(acao);
            } else if self.salvar.conflito.is_none() {
                // o usuário desistiu do diálogo de arquivo: nada acontece
                self.toast(tr("Nada foi gravado; a ação foi cancelada"), false);
            }
        }
    }

    fn janela_conflito(&mut self, ctx: &egui::Context) {
        let Some(id) = self.salvar.conflito else { return };
        let Some(path) = self.caminho_de(id) else {
            self.salvar.conflito = None;
            return;
        };
        let sumiu = self.mudanca_externa(id) == MudancaExterna::Sumiu;
        let (mut sobrescrever, mut como, mut recarregar, mut cancelar) = (false, false, false, false);
        egui::Modal::new(egui::Id::new("fenkra_conflito_externo")).show(ctx, |ui| {
            ui.set_max_width(440.);
            ui.heading(tr("O arquivo mudou fora do Fenkra"));
            ui.label(path.display().to_string());
            ui.label(if sumiu { tr("O arquivo foi apagado ou movido desde que você o abriu.") } else { tr("Outro programa gravou conteúdo diferente desde que você o abriu.") });
            ui.add_space(6.);
            ui.horizontal(|ui| {
                if !sumiu {
                    sobrescrever = ui.button(tr("Sobrescrever")).on_hover_text(tr("Grava a sua versão por cima da que está no disco (a anterior fica em .bak)")).clicked();
                } else {
                    sobrescrever = ui.button(tr("Gravar de novo")).clicked();
                }
                como = ui.button(tr("Salvar como…")).clicked();
                if !sumiu {
                    recarregar = ui.button(tr("Recarregar")).on_hover_text(tr("Descarta as suas alterações e abre a versão do disco")).clicked();
                }
                cancelar = ui.button(tr("Cancelar")).clicked();
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            cancelar = true;
        }
        if cancelar {
            self.salvar.conflito = None;
        } else if sobrescrever {
            self.salvar.conflito = None;
            self.salvar.assin.remove(&path);
            self.gravar_sem_checar(id);
        } else if como {
            self.salvar.conflito = None;
            self.salvar.assin.remove(&path);
            let antigo = path.clone();
            self.definir_caminho(id, None);
            self.gravar_sem_checar(id);
            if self.modificado(id) {
                // desistiu do diálogo de arquivo: volta ao caminho de antes e ao conflito
                self.definir_caminho(id, Some(antigo));
            }
        } else if recarregar {
            self.salvar.conflito = None;
            self.salvar.esquecer();
            match id {
                PECA => self.open.modified = false,
                MONTAGEM => {
                    if let Some(s) = self.asm.as_mut() {
                        s.modified = false;
                    }
                }
                _ => {
                    if let Some(s) = self.dwg.as_mut() {
                        s.modified = false;
                    }
                }
            }
            self.open_path(&path);
        }
    }

    fn definir_caminho(&mut self, id: u64, p: Option<PathBuf>) {
        match id {
            PECA => self.open.file = p,
            MONTAGEM => {
                if let Some(s) = self.asm.as_mut() {
                    s.file = p;
                }
            }
            _ => {
                if let Some(s) = self.dwg.as_mut() {
                    s.path = p;
                }
            }
        }
    }

    fn janela_plano(&mut self, ctx: &egui::Context) {
        let Some(mut plano) = self.salvar.plano.take() else { return };
        let (mut confirmar, mut cancelar) = (false, false);
        let problemas = plano.problemas();
        egui::Window::new(tr("Salvar tudo")).collapsible(false).resizable(true).default_width(520.).show(ctx, |ui| {
            ui.label(RichText::new(tr("Cada arquivo aparece com o motivo de ser gravado. Cancelar não grava nada.")).weak().small());
            ui.add_space(4.);
            egui::Grid::new("fenkra_plano_grade").num_columns(4).spacing([10., 6.]).show(ui, |ui| {
                for item in &mut plano.itens {
                    let bloqueado = item.motivo.bloqueia();
                    let mut m = item.marcado;
                    ui.add_enabled(!bloqueado, egui::Checkbox::without_text(&mut m));
                    item.marcado = m && !bloqueado;
                    if item.motivo == Motivo::Novo {
                        ui.add(egui::TextEdit::singleline(&mut item.nome).desired_width(150.).hint_text(tr("nome do arquivo")));
                    } else {
                        ui.label(&item.nome);
                    }
                    let cor = if bloqueado { crate::theme::st_err() } else { ui.visuals().text_color() };
                    ui.label(RichText::new(tr(item.motivo.rotulo())).color(cor));
                    if item.motivo == Motivo::Novo {
                        let texto = item.pasta.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| tr("Escolher pasta…").to_string());
                        if ui.button(texto).clicked() {
                            if let Some(d) = rfd::FileDialog::new().pick_folder() {
                                item.pasta = Some(d);
                            }
                        }
                    } else {
                        ui.label("");
                    }
                    ui.end_row();
                }
            });
            for p in &problemas {
                ui.colored_label(crate::theme::st_err(), tr(p.mensagem));
            }
            ui.add_space(6.);
            ui.horizontal(|ui| {
                confirmar = ui.add_enabled(problemas.is_empty() && plano.a_gravar().next().is_some(), egui::Button::new(tr("Salvar"))).clicked();
                cancelar = ui.button(tr("Cancelar")).clicked();
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            cancelar = true;
        }
        if cancelar {
            return;
        }
        if confirmar {
            match plano.executar(|item| self.gravar_item(item)) {
                Ok(r) => {
                    if r.falhas.is_empty() {
                        self.toast(format!("{} {}", r.gravados.len(), tr("arquivo(s) salvo(s)")), false);
                    } else {
                        let msg = r.falhas.iter().map(|(_, e)| e.as_str()).collect::<Vec<_>>().join("; ");
                        self.toast(msg, true);
                    }
                }
                Err(_) => self.salvar.plano = Some(plano),
            }
            return;
        }
        self.salvar.plano = Some(plano);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn sem_alteracao_nao_pergunta() {
        let mut e = Estado::default();
        e.esquecer();
        assert!(e.confirmar.is_none() && e.plano.is_none());
    }

    #[test]
    fn plano_de_peca_nova_exige_nome_e_pasta() {
        let c = Candidato { id: PECA, nome: String::new(), novo: true, editado: true, ..Default::default() };
        let plano = Plano::montar(&[c], Some(PECA));
        assert_eq!(plano.itens[0].motivo, Motivo::Novo);
        assert_eq!(plano.problemas().len(), 2, "nome e pasta");
    }
}
