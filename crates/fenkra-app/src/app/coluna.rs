// SPDX-License-Identifier: LGPL-2.1-or-later
//! Coluna da esquerda (B433, ADR 0094): a árvore e o painel do comando dividem uma coluna que reserva
//! espaço. A área 3D começa depois dela, então nada cobre a peça.
//!
//! Sem comando aberto, a coluna mostra a árvore. Ao abrir um comando, o painel dele toma a coluna e a
//! árvore recolhe numa seta, na primeira linha. Clicar na seta mostra a árvore no lugar dos campos, para
//! escolher entradas; a barra de Concluir e Cancelar continua à vista e o comando segue aberto. Clicar de
//! novo volta aos campos. Fechar o comando devolve a coluna à árvore.
//!
//! O painel do comando só sai da coluna para uma janela nativa do sistema (outro monitor, B366).

use super::*;
use crate::paineis::{self, Encaixe, Painel};

/// Largura da coluna quando a pessoa nunca a arrastou.
pub const LARGURA: f32 = paineis::LARGURA_COLUNA;
const LARGURA_MIN: f32 = 240.;
const LARGURA_MAX: f32 = 560.;
/// Largura do trilho que sobra quando a árvore está minimizada.
const TRILHO: f32 = 26.;
/// Altura da linha da seta e da linha do título.
const LINHA: f32 = 22.;

/// O que a coluna mostra neste quadro.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Conteudo {
    /// árvore fechada pelo menu Exibir e nenhum comando: a coluna some e a área 3D ocupa o lugar
    Nada,
    /// árvore minimizada: só o trilho com a seta
    Trilho,
    Arvore,
    /// campos do comando; a árvore está recolhida na seta
    Comando,
    /// árvore aberta pela seta, com o comando ainda aberto
    ArvoreNoComando,
    /// propriedades de um item escolhido (peça, parâmetros, anotação, vista, restrição): a árvore
    /// fica em cima, para continuar escolhendo, e as propriedades embaixo
    ArvoreEPropriedades,
}

/// Regra da coluna. `painel`: há painel para mostrar na coluna (comando ou propriedades), e ele não
/// está numa janela nativa. `comando`: é um comando de verdade, e não só as propriedades de um item
/// escolhido. `quer_arvore`: a pessoa clicou na seta.
pub fn conteudo(painel: bool, comando: bool, quer_arvore: bool, fechada: bool, minimizada: bool) -> Conteudo {
    match (painel, quer_arvore) {
        // propriedades de uma seleção: com a árvore à vista, dividem a coluna com ela
        (true, false) if !comando && !fechada && !minimizada => Conteudo::ArvoreEPropriedades,
        (true, false) => Conteudo::Comando,
        (true, true) => Conteudo::ArvoreNoComando,
        (false, _) if fechada => Conteudo::Nada,
        (false, _) if minimizada => Conteudo::Trilho,
        (false, _) => Conteudo::Arvore,
    }
}

fn id_quer_arvore() -> egui::Id {
    egui::Id::unique("coluna_arvore_no_comando")
}

/// A árvore está à vista porque a pessoa clicou na seta, com um comando aberto: os cliques nela são
/// para escolher entradas do comando.
pub(super) fn arvore_pela_seta(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(id_quer_arvore())).unwrap_or(false)
}

/// Linha da seta: triângulo, rótulo e, à direita, um lembrete do que o clique faz. Toda a linha clica.
fn seta(ui: &mut egui::Ui, aberta: bool, rotulo: &str, lembrete: &str, dica: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), LINHA), egui::Sense::click());
    let resp = crate::teclado::anel(ui, resp);
    if ui.is_rect_visible(r) {
        let v = ui.style().interact(&resp);
        if resp.hovered() || resp.has_focus() {
            ui.painter().rect_filled(r, 3., v.weak_bg_fill);
        }
        let cor = if aberta { crate::theme::accent(ui.visuals()) } else { v.fg_stroke.color };
        let c = egui::pos2(r.left() + 12., r.center().y);
        let pts = if aberta {
            vec![c + egui::vec2(-4.5, -3.), c + egui::vec2(4.5, -3.), c + egui::vec2(0., 4.)]
        } else {
            vec![c + egui::vec2(-3., -4.5), c + egui::vec2(-3., 4.5), c + egui::vec2(4., 0.)]
        };
        ui.painter().add(egui::Shape::convex_polygon(pts, cor, egui::Stroke::NONE));
        let fonte = egui::TextStyle::Body.resolve(ui.style());
        let fim = ui.painter().with_clip_rect(r).text(egui::pos2(r.left() + 24., r.center().y), egui::Align2::LEFT_CENTER, rotulo, fonte.clone(), cor);
        if !lembrete.is_empty() && r.width() > fim.width() + 150. {
            let pequena = egui::TextStyle::Small.resolve(ui.style());
            ui.painter().with_clip_rect(r).text(egui::pos2(r.right() - 8., r.center().y), egui::Align2::RIGHT_CENTER, lembrete, pequena, ui.visuals().weak_text_color());
        }
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, rotulo));
    resp.on_hover_text(dica).clicked()
}

impl App {
    /// B438: o compasso preso tem o painel dele na coluna, quando nenhum comando de verdade está aberto.
    pub(super) fn compasso_no_painel(&self) -> bool {
        let comando_asm = self.in_asm() && self.asm.as_ref().is_some_and(|s| s.cmd.is_some());
        crate::compasso::painel_aberto(self) && self.session.is_none() && self.sketch.is_none() && !self.sketch_wait_plane && self.pmi_pending.is_none() && !comando_asm
    }

    /// B182: o painel da cena está na coluna (na peça, sem comando aberto: um comando o encobre).
    pub(super) fn cena_no_painel(&self) -> bool {
        crate::cena_ui::aberto() && self.settings.bench != crate::settings::Bench::Desenho && !self.in_asm() && self.session.is_none() && self.sketch.is_none() && !self.sketch_wait_plane && self.pmi_pending.is_none() && !self.compasso_no_painel()
    }

    /// Título do painel do comando aberto.
    fn titulo_do_comando(&self) -> String {
        if self.cena_no_painel() {
            tr("Cena").to_string()
        } else if self.compasso_no_painel() {
            tr("Compasso").to_string()
        } else if self.sketch.is_some() {
            tr("Sketch").to_string()
        } else if let Some(s) = &self.session {
            // B435: o tipo do recurso saiu do corpo do painel e fica no título, junto do nome
            let nome = self.open.doc.name_of(s.id);
            match self.open.doc.feature(s.id) {
                Some(f) => format!("{} · {nome}", tr(crate::arvore_nomes::tipo_legivel(f.kind.type_name()))),
                None => nome,
            }
        } else if let Some(p) = self.asm.as_ref().and_then(|s| s.cmd.as_ref()).filter(|_| self.in_asm()) {
            tr(p.c.tipo.label()).to_string()
        } else if let Some(i) = self.pmi_pending.filter(|_| !self.in_asm()) {
            // B448: anotação pendente: o painel é do comando, não da seleção anterior
            tr(crate::pmi_ui::kind_of(i).label()).to_string()
        } else if self.sel == Sel::Part {
            tr("Propriedades da peça").to_string()
        } else {
            tr("Edição").to_string()
        }
    }

    /// Concluir, Cancelar e companhia: ficam à vista também com a árvore aberta pela seta.
    fn barras_do_comando(&mut self, ui: &mut egui::Ui) {
        if self.compasso_no_painel() || self.cena_no_painel() {
            return;
        }
        self.conclude_bar(ui);
        if !matches!(self.settings.bench, crate::settings::Bench::Desenho | crate::settings::Bench::Montagem) && self.sketch.is_none() {
            editor::barra_fixa(self, ui);
        }
    }

    /// Campos do comando, rolando dentro do espaço que sobra.
    fn campos_do_comando(&mut self, ui: &mut egui::Ui) {
        // desenvolvimento: FENKRA_DEMO_ROLAR=<px> abre o painel já rolado, para capturar o que fica abaixo da dobra
        let mut rolagem = egui::ScrollArea::vertical().id_salt("campos_do_comando").auto_shrink([false, false]);
        if let Some(px) = std::env::var("FENKRA_DEMO_ROLAR").ok().and_then(|v| v.parse::<f32>().ok()) {
            rolagem = rolagem.vertical_scroll_offset(px);
        }
        rolagem.show(ui, |ui| {
            if self.cena_no_painel() {
                crate::cena_ui::painel(self, ui)
            } else if self.compasso_no_painel() {
                crate::compasso::painel(self, ui)
            } else if self.settings.bench == crate::settings::Bench::Desenho {
                crate::dwg_ui::panel(self, ui)
            } else if self.settings.bench == crate::settings::Bench::Montagem {
                crate::medir_quadros::fase("painel", || crate::asm_ui::panel(self, ui))
            } else if self.sketch.is_some() {
                crate::sketch_ui::panel(self, ui)
            } else if self.pmi_pending.is_some() {
                crate::pmi_nota_cmd::painel(self, ui)
            } else {
                editor::show(self, ui);
                crate::measure::panel(self, ui);
            }
        });
    }

    /// Painel do comando numa janela nativa do sistema (outro monitor): o único jeito de ele sair da coluna.
    fn comando_em_janela(&mut self, ctx: &egui::Context, area: Rect) {
        let posicao = paineis::Posicao { fechado: false, ..self.settings.painel("comando", Encaixe::Janela) };
        let cfg = Painel { nome: "comando".into(), titulo: self.titulo_do_comando(), padrao: area.left_top(), largura: LARGURA, redimensiona: true };
        let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
            self.barras_do_comando(ui);
            self.campos_do_comando(ui);
        });
        if let Some(mut p) = saida.movido {
            // o X do painel de comando encerra o comando: não existe comando aberto sem painel
            let fechou = std::mem::take(&mut p.fechado);
            self.settings.guardar_painel("comando", p);
            if fechou {
                self.fechar_comando();
            }
        }
    }

    /// A coluna inteira. `painel`: há painel de comando ou de propriedades neste quadro. `comando`: é
    /// um comando de verdade.
    pub(super) fn coluna_esquerda(&mut self, root: &mut egui::Ui, painel: bool, comando: bool) {
        let ctx = root.ctx().clone();
        let nativa = painel && !ctx.embed_viewports() && self.settings.painel("comando", Encaixe::Esquerda).encaixe == Encaixe::Janela;
        let na_coluna = painel && !nativa;
        if !na_coluna {
            ctx.data_mut(|d| d.remove::<bool>(id_quer_arvore()));
        }
        // captura sem mouse: FENKRA_DEMO_SETA=1 abre a árvore pela seta com o comando aberto
        let quer = na_coluna && (ctx.data(|d| d.get_temp::<bool>(id_quer_arvore())).unwrap_or(false) || std::env::var("FENKRA_DEMO_SETA").is_ok());
        let o_que = conteudo(na_coluna, comando, quer, self.settings.arvore_fechada, self.settings.arvore_minimizada);
        let quadro = egui::Frame::new().fill(root.visuals().panel_fill);
        match o_que {
            Conteudo::Nada => {
                self.arvore.regiao = Rect::NOTHING;
                self.arvore.conteudo = Rect::NOTHING;
            }
            Conteudo::Trilho => {
                egui::Panel::left("coluna_trilho").resizable(false).exact_size(TRILHO).frame(quadro).show(root, |ui| {
                    ui.add_space(4.);
                    if seta(ui, false, "", "", tr("Mostrar a árvore")) {
                        self.settings.arvore_minimizada = false;
                    }
                });
                self.arvore.regiao = Rect::NOTHING;
                self.arvore.conteudo = Rect::NOTHING;
            }
            _ => {
                let largura = self.settings.coluna_largura.clamp(LARGURA_MIN, LARGURA_MAX);
                let resp = egui::Panel::left("coluna_esquerda").resizable(true).default_size(largura).size_range(LARGURA_MIN..=LARGURA_MAX).frame(quadro).show(root, |ui| {
                    // filho com retângulo próprio: conteúdo largo é cortado, não alarga a coluna
                    let tudo = ui.max_rect();
                    let mut dentro = ui.new_child(egui::UiBuilder::new().id_salt("coluna").max_rect(tudo.shrink2(egui::vec2(6., 3.))));
                    dentro.set_clip_rect(tudo);
                    self.conteudo_da_coluna(&mut dentro, o_que);
                    ui.allocate_rect(tudo, egui::Sense::hover());
                });
                // a largura escolhida fica nas preferências; só grava com o botão solto, para não
                // escrever o arquivo a cada quadro do arraste
                let agora = resp.response.rect.width();
                if (agora - self.settings.coluna_largura).abs() > 0.5 && !ctx.input(|i| i.pointer.any_down()) {
                    self.settings.coluna_largura = agora;
                }
            }
        }
        if nativa {
            let area = root.available_rect_before_wrap();
            self.comando_em_janela(&ctx, area);
        }
    }

    /// Linha do título do painel, com o X. Devolve se a pessoa pediu para fechar. O botão direito no
    /// título leva o painel para uma janela do sistema (segunda tela).
    fn titulo_do_painel(&mut self, ui: &mut egui::Ui, titulo: &str) -> bool {
        let ctx = ui.ctx().clone();
        let mut fechar = false;
        let linha = ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), LINHA), egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if paineis::botao_fechar(ui, tr("Cancelar o comando")).clicked() {
                fechar = true;
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(egui::Label::new(RichText::new(titulo).strong()).truncate().selectable(false));
            });
        });
        // segunda tela: o painel só deixa a coluna para uma janela do sistema
        if !ctx.embed_viewports() {
            linha.response.interact(egui::Sense::click()).context_menu(|ui| {
                if ui.button(tr("Abrir numa janela própria")).on_hover_text(tr("Leva o painel para uma janela do sistema, que pode ir para outro monitor")).clicked() {
                    let mut p = self.settings.painel("comando", Encaixe::Esquerda);
                    p.encaixe = Encaixe::Janela;
                    p.pos = None;
                    self.settings.guardar_painel("comando", p);
                    ui.close();
                }
            });
        }
        fechar
    }

    fn conteudo_da_coluna(&mut self, ui: &mut egui::Ui, o_que: Conteudo) {
        let ctx = ui.ctx().clone();
        let titulo = self.titulo_do_comando();
        match o_que {
            Conteudo::Comando => {
                if seta(ui, false, tr("Árvore"), tr("escolher entradas"), tr("Mostra a árvore para escolher entradas. O comando continua aberto.")) {
                    ctx.data_mut(|d| d.insert_temp(id_quer_arvore(), true));
                }
                ui.separator();
                let fechar = self.titulo_do_painel(ui, &titulo);
                self.barras_do_comando(ui);
                self.campos_do_comando(ui);
                if fechar {
                    self.fechar_comando();
                }
            }
            Conteudo::ArvoreEPropriedades => {
                // árvore em cima, propriedades embaixo: escolher outro item troca as propriedades
                let tudo = ui.available_rect_before_wrap();
                let meio = tudo.top() + (tudo.height() * 0.5).round();
                self.arvore_na_coluna(ui, Rect::from_min_max(tudo.min, egui::pos2(tudo.right(), meio - 4.)));
                ui.painter().hline(tudo.x_range(), meio, ui.visuals().widgets.noninteractive.bg_stroke);
                let baixo = Rect::from_min_max(egui::pos2(tudo.left(), meio + 4.), tudo.max);
                let mut dentro = ui.new_child(egui::UiBuilder::new().id_salt("propriedades_da_selecao").max_rect(baixo));
                dentro.set_clip_rect(baixo.intersect(ui.clip_rect()));
                let fechar = self.titulo_do_painel(&mut dentro, &titulo);
                self.barras_do_comando(&mut dentro);
                self.campos_do_comando(&mut dentro);
                ui.allocate_rect(tudo, egui::Sense::hover());
                if fechar {
                    self.fechar_comando();
                }
            }
            Conteudo::ArvoreNoComando => {
                let lembrete = format!("{} {titulo}", tr("voltar a"));
                if seta(ui, true, tr("Árvore"), &lembrete, tr("Volta aos campos do comando")) {
                    ctx.data_mut(|d| d.remove::<bool>(id_quer_arvore()));
                }
                ui.separator();
                self.barras_do_comando(ui);
                let resto = ui.available_rect_before_wrap();
                self.arvore_na_coluna(ui, resto);
            }
            _ => {
                let resto = ui.available_rect_before_wrap();
                self.arvore_na_coluna(ui, resto);
            }
        }
    }

    /// Árvore dentro da coluna, com zoom e deslocamento próprios.
    fn arvore_na_coluna(&mut self, root: &mut egui::Ui, area: Rect) {
        use crate::tree_ui::{self, Localizar};
        let ctx = root.ctx().clone();
        let largura = area.width();
        let topo = Rect::from_min_size(area.min + egui::vec2(0., 1.), egui::vec2(largura, 22.));
        let regiao = Rect::from_min_max(topo.left_bottom() + egui::vec2(0., 2.), egui::pos2(topo.right(), area.bottom() - 2.));
        self.arvore.regiao = regiao;
        let (ponteiro, roda, pinca) = ctx.input(|i| (i.pointer.latest_pos(), i.smooth_scroll_delta, i.zoom_delta()));
        let zona = self.arvore.conteudo.intersect(regiao);
        let sobre = ponteiro.is_some_and(|p| regiao.contains(p));
        let centro = ponteiro.filter(|_| sobre).unwrap_or(regiao.min);
        if sobre && pinca != 1. {
            self.arvore.zoom_em(centro, pinca);
        }
        // a coluna é da árvore: com o ponteiro nela, a roda rola a lista
        if sobre && self.arvore.transborda() {
            self.arvore.pan += roda;
        }
        // captura sem mouse (B447): FENKRA_DEMO_ARVORE=solta mostra a árvore solta e deslocada, cortada pela coluna
        if std::env::var("FENKRA_DEMO_ARVORE").is_ok_and(|v| v == "solta") {
            self.settings.arvore_solta = true;
            self.arvore.pan = egui::vec2(170., 60.);
        }
        // atalhos: Alt + mais, menos, 0 (enquadrar) e L (localizar)
        let tecla = |k: egui::Key| ctx.input_mut(|i| i.consume_key(egui::Modifiers::ALT, k));
        if tecla(egui::Key::Plus) || tecla(egui::Key::Equals) {
            self.arvore.zoom_em(centro, 1.15);
        }
        if tecla(egui::Key::Minus) {
            self.arvore.zoom_em(centro, 1. / 1.15);
        }
        if tecla(egui::Key::Num0) {
            self.arvore.enquadrar();
        }
        if tecla(egui::Key::L) {
            self.arvore.localizar = Localizar::Abrir;
        }

        let mut barra = root.new_child(egui::UiBuilder::new().id_salt("arvore_controles").max_rect(topo).layout(egui::Layout::left_to_right(egui::Align::Center)));
        barra.set_clip_rect(topo);
        tree_ui::estilo(&mut barra, 1.);
        barra.selectable_value(&mut self.left, LeftTab::Peca, tr("Peça"));
        barra.selectable_value(&mut self.left, LeftTab::Projeto, tr("Projeto"));
        barra.add_space(6.);
        if tree_ui::controle(&mut barra, "arvore_zoom_menos", tr("Diminuir a árvore (Alt + menos, ou Ctrl + roda)")) {
            self.arvore.zoom_em(regiao.min, 1. / 1.15);
        }
        if tree_ui::controle(&mut barra, "arvore_zoom_mais", tr("Aumentar a árvore (Alt + mais, ou Ctrl + roda)")) {
            self.arvore.zoom_em(regiao.min, 1.15);
        }
        if tree_ui::controle(&mut barra, "arvore_localizar", tr("Localizar o item ativo (Alt + L)")) {
            self.arvore.localizar = Localizar::Abrir;
        }
        if tree_ui::controle(&mut barra, "arvore_enquadrar", tr("Voltar ao enquadramento (Alt + 0)")) {
            self.arvore.enquadrar();
        }
        if tree_ui::controle(&mut barra, "arvore_recolher", tr("Recolher todos os ramos")) {
            self.arvore.recolher = true;
        }
        let opcoes = barra.menu_button(RichText::new("⋯").weak(), |ui| {
            let solta = self.settings.arvore_solta;
            if ui.checkbox(&mut self.settings.arvore_solta, tr("Soltar a árvore")).on_hover_text(tr("Solta: arrastar o fundo desloca a árvore. Travada: ela fica no lugar e a roda do mouse rola a lista.")).changed() && solta {
                self.arvore.pan = egui::Vec2::ZERO;
            }
            if ui.button(tr("Minimizar a árvore")).on_hover_text(tr("A coluna vira um trilho com a seta que traz a árvore de volta")).clicked() {
                self.settings.arvore_minimizada = true;
                ui.close();
            }
            if ui.button(tr("Fechar a árvore")).on_hover_text(tr("Volta pelo menu Exibir, em Árvore")).clicked() {
                self.settings.arvore_fechada = true;
                ui.close();
            }
        });
        opcoes.response.on_hover_text(tr("Opções da árvore"));

        tree_ui::comecar(&self.arvore);
        let zoom = self.arvore.zoom;
        let caixa = Rect::from_min_size(regiao.min + self.arvore.pan, egui::vec2(largura * zoom, 1e5));
        let mut ui = root.new_child(egui::UiBuilder::new().id_salt("arvore").max_rect(caixa));
        // B447: a árvore solta (deslocada) é cortada pela área útil da coluna, nunca desenha fora dela
        ui.set_clip_rect(regiao.intersect(root.clip_rect()));
        tree_ui::estilo(&mut ui, zoom);
        // travada por padrão, para o arraste ser do recurso; solta, o fundo desloca a árvore
        if zona.is_positive() {
            let arrasto = ui.interact(zona, egui::Id::unique("arvore_mover"), egui::Sense::drag());
            if self.settings.arvore_solta {
                self.arvore.pan += arrasto.drag_delta();
            }
        }
        match self.left {
            LeftTab::Peca if self.settings.bench == crate::settings::Bench::Desenho => crate::dwg_ui::tree(self, &mut ui),
            LeftTab::Peca if self.settings.bench == crate::settings::Bench::Montagem => crate::medir_quadros::fase("arvore", || crate::asm_ui::tree(self, &mut ui)),
            LeftTab::Peca => self.tree(&mut ui),
            LeftTab::Projeto => self.project_panel(&mut ui),
        }
        self.arvore.conteudo = ui.min_rect();
        self.arvore.pan = tree_ui::limitar_pan(self.arvore.pan, ui.min_rect().size(), regiao.size());
        if !tree_ui::terminar(&ctx, &mut self.arvore) {
            self.toast(tr("Nenhum item ativo para localizar").to_string(), false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B433: comando aberto toma a coluna e a árvore recolhe na seta; a seta troca para a árvore sem
    /// fechar o comando; sem comando vale o estado da árvore (aberta, minimizada no trilho, fechada).
    #[test]
    fn comando_toma_a_coluna_e_a_seta_alterna_com_a_arvore() {
        assert_eq!(conteudo(false, false, false, false, false), Conteudo::Arvore);
        assert_eq!(conteudo(true, true, false, false, false), Conteudo::Comando);
        assert_eq!(conteudo(true, true, true, false, false), Conteudo::ArvoreNoComando);
        // a seta não sobrevive ao comando: sem comando, o pedido não muda nada
        assert_eq!(conteudo(false, false, true, false, false), Conteudo::Arvore);
        // árvore minimizada ou fechada não esconde o comando nem a árvore pedida pela seta
        for (fechada, minimizada) in [(true, false), (false, true), (true, true)] {
            assert_eq!(conteudo(true, true, false, fechada, minimizada), Conteudo::Comando);
            assert_eq!(conteudo(true, true, true, fechada, minimizada), Conteudo::ArvoreNoComando);
        }
        assert_eq!(conteudo(false, false, false, true, false), Conteudo::Nada);
        assert_eq!(conteudo(false, false, false, false, true), Conteudo::Trilho);
        assert_eq!(conteudo(false, false, false, true, true), Conteudo::Nada, "fechada vence minimizada");
    }

    /// Propriedades de um item escolhido (peça, parâmetros, vista do desenho, restrição) não são um
    /// comando: a árvore continua à vista, em cima, para escolher outro item. Com a árvore fechada ou
    /// minimizada, as propriedades ficam com a coluna e a seta ainda traz a árvore.
    #[test]
    fn propriedades_da_selecao_dividem_a_coluna_com_a_arvore() {
        assert_eq!(conteudo(true, false, false, false, false), Conteudo::ArvoreEPropriedades);
        assert_eq!(conteudo(true, false, false, true, false), Conteudo::Comando);
        assert_eq!(conteudo(true, false, false, false, true), Conteudo::Comando);
        assert_eq!(conteudo(true, false, true, true, false), Conteudo::ArvoreNoComando);
        // um comando de verdade nunca divide: ele toma a coluna
        assert_ne!(conteudo(true, true, false, false, false), Conteudo::ArvoreEPropriedades);
    }

    /// A coluna reserva espaço de verdade: a área que sobra para a vista começa depois dela, e o
    /// conteúdo mais largo que a coluna não a alarga.
    #[test]
    fn a_coluna_reserva_espaco_e_conteudo_largo_nao_a_alarga() {
        let ctx = egui::Context::default();
        let tela = Rect::from_min_size(egui::pos2(0., 0.), egui::vec2(1200., 800.));
        let mut resto = Rect::NOTHING;
        let mut coluna = Rect::NOTHING;
        for _ in 0..3 {
            let raw = egui::RawInput { screen_rect: Some(tela), ..Default::default() };
            let mut saida = ctx.run_ui(raw, |root| {
                let r = egui::Panel::left("coluna_esquerda").resizable(true).default_size(LARGURA).size_range(LARGURA_MIN..=LARGURA_MAX).show(root, |ui| {
                    let tudo = ui.max_rect();
                    let mut dentro = ui.new_child(egui::UiBuilder::new().id_salt("coluna").max_rect(tudo));
                    dentro.set_clip_rect(tudo);
                    dentro.add_sized([900., 30.], egui::Label::new("campo muito mais largo que a coluna"));
                    ui.allocate_rect(tudo, egui::Sense::hover());
                });
                coluna = r.response.rect;
                resto = root.available_rect_before_wrap();
            });
            saida.textures_delta.clear();
        }
        assert!(coluna.width() <= LARGURA + 20., "{coluna:?}");
        assert!(resto.left() >= coluna.right() - 0.5, "a vista começa depois da coluna: {resto:?} contra {coluna:?}");
        assert!(resto.width() > 800. && resto.width() < 1200. - LARGURA + 1., "{resto:?}");
    }
}
