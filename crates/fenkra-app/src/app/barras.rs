// SPDX-License-Identifier: LGPL-2.1-or-later
//! Barras da tela (B434, ADR 0094): topo fino só com o auxiliar, recursos à direita só com ícones e
//! navegação embaixo.
//!
//! - **Topo** (cerca de 26 px): menu num ícone, abas de bancada em ícones e, à direita, Propriedades e o
//!   estado de versões/PLM do documento. O nome do documento vai para o título da janela do sistema.
//! - **Direita**: os grupos de comandos da bancada (ou do sketch), só ícones. Cabem numa coluna; quando
//!   não cabem (Superfícies), viram duas; passando disso, a barra rola.
//! - **Embaixo**: indicador de recalculado, "auto", os avisos que pedem ação e a navegação (vista,
//!   projeção, modo de exibição e filtro de seleção).
//!
//! O que saiu da tela continua a um clique: reconstruir, Scan e o passo a passo do histórico estão em
//! Editar > Histórico da peça; versão do kernel, tempo de recálculo e objeto em trabalho estão em
//! Propriedades da peça (`info_do_modelo`).

use super::*;
use crate::settings::{Bench, Group};
use crate::toolbar::Forma;

/// Lado dos ícones do topo.
const ICONE_TOPO: f32 = 18.;

/// Bancadas na ordem das abas, com o ícone e o nome de cada uma.
pub const BANCADAS: [(Bench, &str, &str); 7] = [
    (Bench::Solidos, "banc_solidos", "Modelagem Sólida"),
    (Bench::Superficies, "banc_superficies", "Superfícies"),
    (Bench::Montagem, "banc_montagem", "Montagem"),
    (Bench::Chapa, "banc_chapa", "Chapa metálica"),
    (Bench::Estrutural, "banc_estrutural", "Estrutural"),
    (Bench::FormaLivre, "forma", "Forma livre"),
    (Bench::Desenho, "banc_desenho", "Desenho"),
];

/// Botão só com ícone, do tamanho do topo. `ativo` pinta o fundo e o traço de baixo na cor de destaque.
fn botao_icone(ui: &mut egui::Ui, icone: &str, nome: &str, ativo: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(ICONE_TOPO + 10., ICONE_TOPO + 4.), egui::Sense::click());
    let resp = crate::teclado::anel(ui, resp);
    if ui.is_rect_visible(r) {
        let destaque = crate::theme::accent(ui.visuals());
        if ativo {
            ui.painter().rect_filled(r, 4., destaque.gamma_multiply(0.22));
            ui.painter().rect_filled(Rect::from_min_max(egui::pos2(r.left() + 3., r.bottom() - 2.), egui::pos2(r.right() - 3., r.bottom())), 1., destaque);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, 4., ui.visuals().widgets.hovered.weak_bg_fill);
        }
        egui::Image::new(crate::icons::get(icone)).paint_at(ui, Rect::from_center_size(r.center(), egui::vec2(ICONE_TOPO, ICONE_TOPO)));
    }
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, ativo, nome));
    resp
}

/// Uma linha de `info_do_modelo`: rótulo e valor.
pub type Info = (String, String);

impl App {
    /// Menu num ícone só: abre os menus de sempre como submenus (B434).
    pub(super) fn icone_do_menu() -> egui::Image<'static> {
        egui::Image::new(crate::icons::get("menu")).fit_to_exact_size(egui::vec2(ICONE_TOPO, ICONE_TOPO))
    }

    /// Abas de bancada, só ícones; o nome fica na dica.
    pub(super) fn abas_de_bancada(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.x = 2.;
        for (b, icone, nome) in BANCADAS {
            let r = botao_icone(ui, icone, tr(nome), self.settings.bench == b);
            if r.on_hover_text(format!("{}\n{}", tr(nome), tr("As bancadas compartilham o mesmo modelo: sólidos e superfícies se conversam."))).clicked() {
                self.settings.bench = b;
            }
        }
    }

    /// Lado direito do topo: Propriedades e o estado de versões/PLM. Chamado num layout da direita
    /// para a esquerda.
    pub(super) fn auxiliares_do_topo(&mut self, ui: &mut egui::Ui) {
        // captura sem mouse (B435): FENKRA_DEMO_PROPRIEDADES=1 abre Propriedades do comando aberto ou da seleção
        let pronto = self.session.is_some() || std::env::var("FENKRA_ABRIR").is_err();
        if pronto && self.props_target.is_none() && std::env::var("FENKRA_DEMO_PROPRIEDADES").is_ok() && ui.ctx().cumulative_frame_nr() > 30 && !ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::unique("demo_propriedades")).unwrap_or(false)) {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique("demo_propriedades"), true));
            self.abrir_propriedades();
        }
        self.estado_na_barra(ui);
        let dica = tr("Propriedades do que está selecionado: nome, estado, suprimir e aparência. Sem seleção, as propriedades da peça e as informações do modelo.");
        if botao_icone(ui, "aparencia", tr("Propriedades"), self.props_target.is_some()).on_hover_text(format!("{}\n{dica}", tr("Propriedades"))).clicked() {
            self.abrir_propriedades();
        }
    }

    /// Propriedades do comando aberto ou do item selecionado; sem nenhum, as da peça.
    pub(super) fn abrir_propriedades(&mut self) {
        let alvo = match (self.session.as_ref().map(|s| s.id), self.sel) {
            (Some(id), _) | (None, Sel::Feature(id)) | (None, Sel::Ref(id)) => Some(if self.open.doc.reference(id).is_some() { AppTarget::Ref(id) } else { AppTarget::Feature(id) }),
            _ => None,
        };
        match alvo {
            Some(a) => self.props_target = Some(a),
            None if !self.in_asm() && self.settings.bench != Bench::Desenho && self.sketch.is_none() => self.sel = Sel::Part,
            None => {}
        }
    }

    /// O nome do documento sai do topo e vai para o título da janela do sistema.
    pub(super) fn titulo_da_janela(&mut self, ctx: &egui::Context) {
        let asm = self.asm.as_ref().filter(|_| self.settings.bench == Bench::Montagem).map(|s| format!("{}{}", s.asm.name, if s.modified { " •" } else { "" }));
        let dwg = self.dwg.as_ref().filter(|_| self.settings.bench == Bench::Desenho).map(|s| format!("{}{}", s.d.name, if s.modified { " •" } else { "" }));
        let nome = asm.or(dwg).unwrap_or_else(|| match (&self.open.frozen, self.open.modified) {
            (Some(r), _) => format!("{} — {} {r} ({})", self.open.doc.name, tr("revisão"), tr("congelada")),
            (None, true) => format!("{} •", self.open.doc.name),
            (None, false) => self.open.doc.name.clone(),
        });
        let titulo = format!("{nome} — Fenkra");
        let id = egui::Id::unique("titulo_da_janela");
        if ctx.data(|d| d.get_temp::<String>(id)).as_deref() != Some(titulo.as_str()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(titulo.clone()));
            ctx.data_mut(|d| d.insert_temp(id, titulo));
        }
    }

    /// Barra de recursos: os grupos de comandos da bancada, só ícones, à direita (B434).
    pub(super) fn barra_direita(&mut self, root: &mut egui::Ui) {
        let sketching = self.sketch.is_some();
        let ts = self.tool_state();
        let (grupos, mais) = toolbar::contexto(&self.settings, toolbar::groups_at(&self.settings, Dock::Direita, sketching), sketching);
        let meus: Vec<crate::settings::CustomGroup> = self.settings.custom.groups.iter().filter(|c| c.sketch == sketching && !c.items.is_empty()).cloned().collect();
        if grupos.is_empty() && mais.is_empty() && meus.is_empty() {
            return;
        }
        let celula = toolbar::celula(toolbar::tamanho_na_coluna(&self.settings));
        let comandos: usize = grupos.iter().map(|g| toolbar::custom_entries(*g, &self.settings).len()).sum::<usize>() + meus.iter().map(|c| c.items.len()).sum::<usize>() + usize::from(!mais.is_empty());
        let altura = root.available_rect_before_wrap().height() - 8.;
        let colunas = toolbar::colunas_para(comandos, grupos.len() + meus.len(), altura, celula.y);
        let largura = colunas as f32 * celula.x + 4.;
        let mut acts = vec![];
        let fundo = root.visuals().panel_fill;
        egui::Panel::right("barra_de_recursos").resizable(false).exact_size(largura + 8.).frame(egui::Frame::new().fill(fundo).inner_margin(egui::Margin::symmetric(4, 4))).show(root, |ui| {
            egui::ScrollArea::vertical().id_salt("rolagem_recursos").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_width(largura);
                for (i, g) in grupos.iter().enumerate() {
                    if i > 0 {
                        ui.separator();
                    }
                    acts.extend(toolbar::show_group(ui, *g, &mut self.settings, &ts, Forma::Coluna));
                }
                // grupos montados pelo usuário
                for cg in &meus {
                    ui.separator();
                    acts.extend(toolbar::show_custom(ui, cg, &mut self.settings, &ts, Forma::Coluna));
                }
                // o que não é do contexto continua a um clique
                if !mais.is_empty() {
                    ui.separator();
                    acts.extend(toolbar::mais(ui, &mais, &mut self.settings, &ts));
                }
            });
        });
        for a in acts {
            self.act(a);
        }
    }

    /// Texto e cor do indicador de recálculo.
    fn indicador_de_recalculo(&self, ui: &egui::Ui) -> (String, Color32) {
        let ms = self.result.as_ref().map(|r| r.elapsed_ms).unwrap_or_default();
        let busy = self.gen_done < self.gen_sent;
        let now = ui.input(|i| i.time);
        let n = self.open.doc.features.len();
        let swept = (((now - self.sweep_t0) / 0.035) as usize).min(n);
        let stale = self.stale_from().is_some() && !busy;
        let compondo = self.asm.as_ref().filter(|s| self.settings.bench == Bench::Montagem && s.compondo);
        if let Some(s) = compondo {
            let t = match s.progresso_composicao() {
                Some((f, t)) => format!("⏳ {} {}/{t}", tr("compondo a montagem…"), f.min(t)),
                None => format!("⏳ {}", tr("compondo a montagem…")),
            };
            (t, crate::theme::st_warn())
        } else if busy {
            let t = match self.worker.progresso() {
                Some((f, t)) => format!("⏳ recalculando… {}/{t}", f.min(t)),
                None => "⏳ recalculando…".to_string(),
            };
            (t, crate::theme::st_warn())
        } else if stale {
            (tr("⟳ desatualizado — F5 atualiza").to_string(), crate::theme::st_err())
        } else if swept < n {
            (format!("▶ atualizando {swept}/{n}"), crate::theme::accent(ui.visuals()))
        } else {
            (format!("✔ recalculado em {ms:.0} ms"), ui.visuals().text_color())
        }
    }

    /// O objeto em trabalho. No Scan é o de antes dele, não o comando que se está olhando.
    fn em_trabalho(&self) -> Option<Id> {
        match self.scan_volta {
            Some(v) => {
                let d = &self.open.doc;
                d.features[..v.min(d.features.len())].iter().rev().find(|f| !d.is_free(f)).map(|f| f.id)
            }
            None => self.open.doc.objeto_em_trabalho(),
        }
    }

    /// Informações do modelo que saíram da barra de baixo (B434): aparecem em Propriedades da peça e na
    /// dica do indicador de recálculo.
    pub fn info_do_modelo(&self) -> Vec<Info> {
        let mut v: Vec<Info> = vec![];
        if let Some(r) = &self.result {
            v.push((tr("Recálculo").to_string(), format!("{:.0} ms", r.elapsed_ms)));
            v.push((tr("Kernel geométrico").to_string(), r.kernel.clone()));
        }
        let (atual, total) = self.open.doc.posicao_em_trabalho();
        v.push((tr("Comandos").to_string(), format!("{atual}/{total}")));
        if let Some(nome) = self.em_trabalho().map(|id| self.open.doc.name_of(id)).filter(|_| crate::barra_estado::mostra_historico(self.sem_peca_em_edicao())) {
            v.push((tr("Em trabalho").to_string(), nome));
        }
        if let (true, Some(st)) = (self.settings.bench == Bench::Montagem, self.asm.as_ref()) {
            v.push((tr("Graus de liberdade livres").to_string(), st.report.dof.to_string()));
            v.push((tr("Restrições com erro").to_string(), st.report.failed.len().to_string()));
        }
        v.push((tr("Navegação").to_string(), self.settings.nav.help().to_string()));
        v
    }

    /// Editar > Histórico da peça: o que saiu da barra de baixo e continua a um clique (B434).
    pub(super) fn menu_historico(&mut self, ui: &mut egui::Ui) {
        use fenkra_core::PassoTrabalho as P;
        if ui.button(tr("⟳ Atualizar   F5")).on_hover_text(tr("recalcula o que mudou (F5)")).clicked() {
            self.send_recompute(false);
            ui.close();
        }
        if ui.button(tr("⟲ Reconstruir   Ctrl+F5")).on_hover_text(tr("reconstrói tudo do zero, sem cache (Ctrl+F5)")).clicked() {
            self.send_recompute(true);
            ui.close();
        }
        if !crate::barra_estado::mostra_historico(self.sem_peca_em_edicao()) {
            return;
        }
        ui.separator();
        let ligado = self.scan_volta.is_some();
        if ui.add(egui::Button::new(tr("🔍 Scan")).selected(ligado)).on_hover_text(tr("Scan: escolha comandos na árvore (clique ou setas) para ver a peça como estava em cada um. O objeto em trabalho não muda; ao desligar, a peça volta.")).clicked() {
            crate::scan::alternar(self);
            ui.close();
        }
        if self.scan_volta.is_some() {
            return;
        }
        // passo a passo (B59): anda o objeto em trabalho comando a comando, a posição que a árvore sublinha
        let (atual, total) = self.open.doc.posicao_em_trabalho();
        ui.label(RichText::new(format!("{}: {atual}/{total}", tr("Objeto em trabalho"))).weak().small());
        let mut passo = None;
        for (rotulo, dica, p) in [
            ("⏮", "voltar ao início (reconstruir passo a passo)", P::Inicio),
            ("◀", "um comando para trás", P::Anterior),
            ("▶", "reconstruir o próximo comando", P::Proximo),
            ("⏭", "até o fim", P::Fim),
        ] {
            if ui.button(format!("{rotulo}  {}", tr(dica))).clicked() {
                passo = Some(p);
            }
        }
        if let Some(p) = passo {
            self.open.doc.passo_em_trabalho(p);
        }
    }

    /// Barra de baixo (B434): recalculado, auto, avisos que pedem ação e a navegação.
    pub(super) fn barra_de_baixo(&mut self, root: &mut egui::Ui) {
        let fundo = root.visuals().panel_fill;
        let mut acts = vec![];
        egui::Panel::bottom("barra_de_baixo").resizable(false).frame(egui::Frame::new().fill(fundo).inner_margin(egui::Margin::symmetric(8, 2))).show(root, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.set_min_height(toolbar::TAMANHO_NA_LINHA + 6.);
                let busy = self.gen_done < self.gen_sent;
                let (txt, cor) = self.indicador_de_recalculo(ui);
                let info = self.info_do_modelo();
                ui.label(RichText::new(txt).small().color(cor)).on_hover_ui(|ui| {
                    egui::Grid::new("info_do_modelo_dica").num_columns(2).show(ui, |ui| {
                        for (k, v) in &info {
                            ui.label(RichText::new(k).weak().small());
                            ui.label(RichText::new(v).small());
                            ui.end_row();
                        }
                    });
                });
                if busy && ui.small_button(tr("✖ Cancelar")).on_hover_text(tr("para o cálculo; o que já terminou fica guardado")).clicked() {
                    self.worker.cancelar();
                }
                // o modo fica no documento: cada peça lembra o seu
                let mut auto = self.open.doc.atualizacao == ModoAtualizacao::Automatica;
                if ui.checkbox(&mut auto, RichText::new(tr("auto")).small()).on_hover_text(tr("atualizar automaticamente a cada mudança")).changed() {
                    self.open.doc.atualizacao = if auto { ModoAtualizacao::Automatica } else { ModoAtualizacao::Manual };
                }
                // com o automático ligado e nada pendente, o botão de atualizar não tem o que fazer
                let pendente = self.stale_from().is_some() && !busy;
                if (!auto || pendente) && ui.small_button(tr("⟳ Atualizar")).on_hover_text(tr("recalcula o que mudou (F5)")).clicked() {
                    self.send_recompute(false);
                }
                // avisos que pedem ação: só aparecem quando existem
                if self.scan_volta.is_some() {
                    let nome = self.open.doc.rollback.checked_sub(1).and_then(|i| self.open.doc.features.get(i)).map(|f| f.name.clone()).unwrap_or_default();
                    if ui.add(egui::Button::new(RichText::new(format!("🔍 {} {nome}", tr("Scan ligado: vendo a peça até"))).small().color(ui.visuals().warn_fg_color)).selected(true)).on_hover_text(tr("Clique para desligar o Scan; a peça volta")).clicked() {
                        crate::scan::alternar(self);
                    }
                } else if self.open.doc.retrocedida() && crate::barra_estado::mostra_historico(self.sem_peca_em_edicao()) && ui.small_button(tr("Voltar ao fim da peça")).clicked() {
                    self.open.doc.voltar_ao_fim_da_peca();
                }
                if let (true, Some(st)) = (self.settings.bench == Bench::Montagem, self.asm.as_ref()) {
                    let n = st.report.failed.len();
                    if n > 0 {
                        ui.label(RichText::new(format!("⚙ {n} {}", tr("restrição(ões) com erro"))).small().color(crate::theme::st_err()));
                    }
                }
                crate::pmi_ui::status(self, ui);
                let errs = self.errors_prev;
                if errs > 0 && ui.small_button(RichText::new(format!("✖ {errs} com erro — reparar")).color(crate::theme::st_err())).clicked() {
                    self.wizard_open = true;
                }
                if let Some(e) = self.result.as_ref().and_then(|r| r.param_error.clone()) {
                    ui.label(RichText::new(format!("Parâmetros: {e}")).small().color(crate::theme::st_err()));
                }
                ui.separator();
                // navegação: vista e seleção, só ícones
                let ts = self.tool_state();
                for (i, g) in toolbar::grupos_de_baixo(&self.settings, self.sketch.is_some()).into_iter().enumerate() {
                    if i > 0 {
                        ui.separator();
                    }
                    acts.extend(toolbar::show_group(ui, g, &mut self.settings, &ts, Forma::Linha));
                }
                // projeção e modo de exibição, por extenso, à direita
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // a folha de desenho não tem projeção nem modo de exibição
                    if self.settings.bench == Bench::Desenho {
                        return;
                    }
                    let ortho = self.viewport.as_ref().is_some_and(|v| v.camera.ortho);
                    let modo = format!("{} · {}", tr(if ortho { "Ortogonal" } else { "Perspectiva" }), self.settings.display.label());
                    let r = ui.add(egui::Button::new(RichText::new(modo).small()).frame(false)).on_hover_text(tr("Projeção e modo de exibição. Clique para escolher."));
                    egui::Popup::menu(&r).show(|ui| {
                        if ui.selectable_label(!ortho, tr("Perspectiva")).clicked() && ortho {
                            acts.push(Act::Ortho);
                        }
                        if ui.selectable_label(ortho, tr("Ortogonal")).clicked() && !ortho {
                            acts.push(Act::Ortho);
                        }
                        ui.separator();
                        for m in crate::render::DisplayMode::ALL {
                            ui.radio_value(&mut self.settings.display, m, m.label());
                        }
                    });
                });
            });
        });
        for a in acts {
            self.act(a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B434: toda bancada tem aba com ícone da família nova e nome traduzido; nenhuma se repete.
    #[test]
    fn toda_bancada_tem_aba_com_icone_e_nome() {
        let mut vistos = vec![];
        for (b, icone, nome) in BANCADAS {
            assert!(!vistos.contains(&b), "{b:?} repetida");
            vistos.push(b);
            assert!(crate::icons::tem_modelo(icone), "sem ícone novo: {icone}");
            assert!(crate::i18n::tem_traducao(nome), "sem tradução: {nome}");
        }
        assert_eq!(vistos.len(), 7);
        assert!(crate::icons::tem_modelo("menu"));
    }

    /// B434: a barra da direita usa uma coluna quando os comandos cabem e duas quando não cabem; os
    /// grupos de navegação nunca entram nela, ficam na barra de baixo; grupo oculto não aparece em
    /// lugar nenhum.
    #[test]
    fn recursos_a_direita_e_navegacao_embaixo() {
        let mut s = Settings::default();
        let celula = toolbar::celula(toolbar::tamanho_na_coluna(&s));
        assert_eq!(toolbar::colunas_para(20, 3, 900., celula.y), 1);
        assert_eq!(toolbar::colunas_para(40, 4, 900., celula.y), 2);
        for b in [Bench::Solidos, Bench::Superficies, Bench::Montagem, Bench::Chapa, Bench::Estrutural, Bench::FormaLivre] {
            s.bench = b;
            for sketching in [false, true] {
                let direita = toolbar::groups_at(&s, Dock::Direita, sketching);
                assert!(!direita.is_empty(), "{b:?} sem comandos à direita");
                assert!(direita.iter().all(|g| !g.de_navegacao()), "{b:?}: navegação na barra de recursos");
                assert!(!direita.contains(&Group::SkRestricoes), "restrições ficam no botão direito");
                let baixo = toolbar::grupos_de_baixo(&s, sketching);
                assert!(baixo.contains(&Group::Vista), "{b:?}: vista embaixo");
                assert!(baixo.iter().all(|g| g.de_navegacao()));
            }
        }
        // nenhuma doca antiga sobra: tudo é "na barra" ou oculto
        s.bench = Bench::Solidos;
        for d in [Dock::Topo, Dock::Lateral, Dock::Flutuante, Dock::Direita] {
            s.set_dock(Group::Solidos, d);
            assert_eq!(s.dock(Group::Solidos), Dock::Direita, "{d:?}");
            assert!(toolbar::groups_at(&s, Dock::Direita, false).contains(&Group::Solidos));
        }
        s.set_dock(Group::Solidos, Dock::Oculta);
        assert!(!toolbar::groups_at(&s, Dock::Direita, false).contains(&Group::Solidos));
        s.set_dock(Group::Vista, Dock::Oculta);
        assert!(!toolbar::grupos_de_baixo(&s, false).contains(&Group::Vista));
        // a superfície, a bancada com mais comandos, cabe em duas colunas numa tela de 1080 px
        s.bench = Bench::Superficies;
        s.docks.clear();
        let (grupos, _) = toolbar::contexto(&s, toolbar::groups_at(&s, Dock::Direita, false), false);
        let n: usize = grupos.iter().map(|g| toolbar::custom_entries(*g, &s).len()).sum();
        assert!(n as f32 * celula.y / 2. < 950., "{n} comandos não cabem em duas colunas");
    }
}
