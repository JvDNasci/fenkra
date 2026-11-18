// SPDX-License-Identifier: LGPL-2.1-or-later
//! Coluna da esquerda (B433, ADR 0094) e árvore transparente sobre a vista (B503, ADR 0104).
//!
//! Sem comando aberto, não há coluna: a árvore fica transparente sobre a vista, no canto da área 3D,
//! só com texto e ícones (contorno pela regra de contraste do tema), e se move pela alça do cabeçalho,
//! muda de largura pela beira e amplia com Ctrl + roda. Ao abrir um comando, o painel dele toma a coluna,
//! que reserva espaço, e a árvore recolhe numa seta, na primeira linha. Clicar na seta mostra a árvore
//! no lugar dos campos, para escolher entradas; a barra de Concluir e Cancelar continua à vista e o
//! comando segue aberto. Clicar de novo volta aos campos. Fechar o comando devolve a árvore à vista.
//!
//! Propriedades de um item escolhido (peça, parâmetros, anotação) ocupam a coluna, e a árvore continua
//! transparente sobre a vista, ao lado, para escolher outro item.
//!
//! O painel do comando sai da coluna de dois jeitos: flutuando sobre a vista (B507), pelo botão do
//! título ou arrastado, e numa janela nativa do sistema (outro monitor, B366). Flutuando, ele lembra a
//! posição, conferida contra a área a cada quadro, e volta para a coluna pelo botão de encaixar ou
//! arrastado até uma borda.

use super::*;
use crate::paineis::{self, Encaixe, Painel};

/// Largura da coluna quando a pessoa nunca a arrastou.
pub const LARGURA: f32 = paineis::LARGURA_COLUNA;
const LARGURA_MIN: f32 = 240.;
const LARGURA_MAX: f32 = 560.;
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

/// A árvore fica transparente sobre a vista (B503): sem comando, e também ao lado das propriedades de
/// um item escolhido. Com comando, ela mora na coluna, pela seta.
pub fn arvore_sobre_a_vista(o_que: Conteudo) -> bool {
    matches!(o_que, Conteudo::Arvore | Conteudo::Trilho | Conteudo::ArvoreEPropriedades)
}

/// Menor largura da árvore transparente.
const ARVORE_MIN: f32 = 160.;

/// Árvore transparente inteira dentro da área 3D: a largura cabe na área e o cabeçalho nunca sai dela.
pub fn arvore_na_area(desloc: [f32; 2], largura: f32, area: Rect) -> (egui::Vec2, f32) {
    let largura = largura.clamp(ARVORE_MIN, (area.width() - 12.).max(ARVORE_MIN));
    let x = desloc[0].clamp(0., (area.width() - largura).max(0.));
    let y = desloc[1].clamp(0., (area.height() - LINHA * 2.).max(0.));
    (egui::vec2(x, y), largura)
}

fn id_arvore_viva() -> egui::Id {
    egui::Id::unique("arvore_sobre_viva")
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
        } else if self.in_asm() && self.asm.as_ref().is_some_and(|s| s.sim_open) {
            tr("Estudo de movimento").to_string()
        } else if let Some(t) = crate::asm_inserir::titulo(self).filter(|_| self.in_asm()) {
            tr(t).to_string()
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
            } else if crate::asm_inserir::na_coluna(self) {
                crate::asm_inserir::painel(self, ui)
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

    /// Painel do comando fora da coluna: flutuando sobre a vista (B507) ou numa janela nativa do sistema
    /// (outro monitor, B366). Encaixar (botão ou arraste até uma borda) devolve o painel à coluna.
    fn comando_solto(&mut self, ctx: &egui::Context, area: Rect) {
        let posicao = paineis::Posicao { fechado: false, ..self.settings.painel("comando", Encaixe::Janela) };
        // primeira vez flutuando: perto da borda direita, abaixo do cubo, longe da seleção em curso
        let cfg = Painel { nome: "comando".into(), titulo: self.titulo_do_comando(), padrao: paineis::abaixo_do_cubo(area, LARGURA), largura: LARGURA, redimensiona: true };
        let op = paineis::Opcoes { flutua: true, encaixe_ao_fixar: Some(Encaixe::Esquerda), ..Default::default() };
        let saida = paineis::mostrar_com(ctx, area, &cfg, posicao, op, |ui| {
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
    /// um comando de verdade. Devolve se a árvore vai transparente sobre a vista neste quadro.
    pub(super) fn coluna_esquerda(&mut self, root: &mut egui::Ui, painel: bool, comando: bool) -> bool {
        let ctx = root.ctx().clone();
        let lugar = self.settings.painel("comando", Encaixe::Esquerda).encaixe;
        let nativa = painel && !ctx.embed_viewports() && lugar == Encaixe::Janela;
        // B507: o painel do comando flutua sobre a vista quando a pessoa o soltou
        let flutuando = painel && lugar == Encaixe::Livre;
        let na_coluna = painel && !nativa && !flutuando;
        if !na_coluna {
            ctx.data_mut(|d| d.remove::<bool>(id_quer_arvore()));
        }
        // captura sem mouse: FENKRA_DEMO_SETA=1 abre a árvore pela seta com o comando aberto
        let quer = na_coluna && (ctx.data(|d| d.get_temp::<bool>(id_quer_arvore())).unwrap_or(false) || std::env::var("FENKRA_DEMO_SETA").is_ok());
        let o_que = conteudo(na_coluna, comando, quer, self.settings.arvore_fechada, self.settings.arvore_minimizada);
        let quadro = egui::Frame::new().fill(root.visuals().panel_fill);
        match o_que {
            // B503: a árvore vai sobre a vista, desenhada depois dela (`arvore_sobre_modelo`)
            Conteudo::Nada | Conteudo::Arvore | Conteudo::Trilho => {
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
        if nativa || flutuando {
            let area = root.available_rect_before_wrap();
            self.comando_solto(&ctx, area);
        }
        arvore_sobre_a_vista(o_que) && !self.settings.arvore_fechada
    }

    /// Linha do título do painel, com o X. Devolve se a pessoa pediu para fechar. O botão direito no
    /// título leva o painel para uma janela do sistema (segunda tela).
    fn titulo_do_painel(&mut self, ui: &mut egui::Ui, titulo: &str) -> bool {
        let ctx = ui.ctx().clone();
        let mut fechar = false;
        let mut flutuar = false;
        let linha = ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), LINHA), egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if paineis::botao_fechar(ui, tr("Cancelar o comando")).clicked() {
                fechar = true;
            }
            // B507: o painel do comando também flutua sobre a vista, à escolha
            if paineis::botao_flutuar(ui, tr(paineis::DICA_FLUTUAR)).clicked() {
                flutuar = true;
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(egui::Label::new(RichText::new(titulo).strong()).truncate().selectable(false));
            });
        });
        let mut janela = false;
        linha.response.interact(egui::Sense::click()).context_menu(|ui| {
            if ui.button(tr("Flutuar sobre a vista")).on_hover_text(tr(paineis::DICA_FLUTUAR)).clicked() {
                flutuar = true;
                ui.close();
            }
            // segunda tela: o painel também deixa a coluna para uma janela do sistema
            if !ctx.embed_viewports() && ui.button(tr("Abrir numa janela própria")).on_hover_text(tr("Leva o painel para uma janela do sistema, que pode ir para outro monitor")).clicked() {
                janela = true;
                ui.close();
            }
        });
        if flutuar {
            self.soltar_comando();
        }
        if janela {
            let mut p = self.settings.painel("comando", Encaixe::Esquerda);
            p.encaixe = Encaixe::Janela;
            p.pos = None;
            self.settings.guardar_painel("comando", p);
        }
        fechar
    }

    /// B507: o painel do comando sai da coluna e flutua sobre a vista, na última posição guardada (ou,
    /// na primeira vez, perto da borda direita).
    pub(super) fn soltar_comando(&mut self) {
        let mut p = self.settings.painel("comando", Encaixe::Esquerda);
        p.encaixe = Encaixe::Livre;
        p.fechado = false;
        self.settings.guardar_painel("comando", p);
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
                // B503: as propriedades ficam com a coluna; a árvore segue transparente sobre a vista,
                // ao lado, e escolher outro item nela troca as propriedades
                let fechar = self.titulo_do_painel(ui, &titulo);
                self.barras_do_comando(ui);
                self.campos_do_comando(ui);
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

    /// B503: árvore transparente sobre a vista, no canto da área 3D. Só texto e ícones, com o contorno
    /// da regra de contraste do tema; nada de placa. Move pela alça do cabeçalho, muda de largura pela
    /// beira da direita e amplia com Ctrl + roda (ou Alt + mais e menos), sem mexer na câmera. Posição e
    /// largura ficam nas preferências quando o botão solta.
    pub(super) fn arvore_sobre_modelo(&mut self, root: &mut egui::Ui, area: Rect) {
        use crate::tree_ui::{self, Contorno, Localizar};
        if !area.is_positive() {
            return;
        }
        let ctx = root.ctx().clone();
        // enquanto arrasta, a posição e a largura vivem na memória; o arquivo só grava ao soltar
        let viva = ctx.data(|d| d.get_temp::<([f32; 2], f32)>(id_arvore_viva()));
        let (desloc, largura) = viva.unwrap_or((self.settings.arvore_desloc, self.settings.arvore_largura));
        let (desloc, largura) = arvore_na_area(desloc, largura, area);
        let topo = Rect::from_min_size(area.min + desloc, egui::vec2(largura, LINHA));
        let regiao = Rect::from_min_max(topo.left_bottom() + egui::vec2(0., 2.), egui::pos2(topo.right(), area.bottom() - 6.));
        self.arvore.regiao = regiao;
        let (ponteiro, pinca) = ctx.input(|i| (i.pointer.latest_pos(), i.zoom_delta()));
        let zona = self.arvore.conteudo.intersect(regiao);
        let sobre = ponteiro.is_some_and(|p| zona.contains(p));
        let centro = ponteiro.filter(|_| sobre).unwrap_or(regiao.min);
        if sobre && pinca != 1. {
            self.arvore.zoom_em(centro, pinca);
        }
        if self.arvore_com_roda {
            self.arvore.pan += ctx.input(|i| i.smooth_scroll_delta);
        }
        // captura sem mouse (B447): FENKRA_DEMO_ARVORE=solta mostra a árvore solta e deslocada
        if std::env::var("FENKRA_DEMO_ARVORE").is_ok_and(|v| v == "solta") {
            self.settings.arvore_solta = true;
            self.arvore.pan = egui::vec2(170., 60.);
        }
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

        // cabeçalho: fraco até o mouse chegar perto, para não pesar sobre a peça
        let mut barra = root.new_child(egui::UiBuilder::new().id_salt("arvore_sobre_controles").max_rect(topo).layout(egui::Layout::left_to_right(egui::Align::Center)));
        let perto = ponteiro.is_some_and(|p| topo.expand(6.).contains(p) || zona.contains(p));
        barra.set_opacity(if perto { 1. } else { 0.6 });
        let contorno = Contorno::reservar(&barra, topo);
        tree_ui::estilo(&mut barra, 1.);
        // alça: arrastar move a árvore pela vista
        let (r_alca, alca) = barra.allocate_exact_size(egui::vec2(12., LINHA), egui::Sense::drag());
        let cor_alca = if alca.hovered() || alca.dragged() { crate::theme::accent(barra.visuals()) } else { barra.visuals().weak_text_color() };
        for k in [-1., 0., 1.] {
            barra.painter().circle_filled(r_alca.center() + egui::vec2(0., k * 4.5), 1.5, cor_alca);
        }
        if alca.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::Grab);
        }
        let alca = alca.on_hover_text(tr("Arraste para mover a árvore sobre a vista"));
        let mut novo = (desloc, largura);
        if alca.dragged() {
            novo.0 += alca.drag_delta();
        }
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
            let rotulo = if self.settings.arvore_minimizada { tr("Expandir a árvore") } else { tr("Minimizar a árvore") };
            if ui.button(rotulo).clicked() {
                self.settings.arvore_minimizada = !self.settings.arvore_minimizada;
                ui.close();
            }
            if ui.button(tr("Voltar a árvore ao canto")).on_hover_text(tr("Devolve a árvore ao canto da vista, com a largura padrão")).clicked() {
                self.settings.arvore_desloc = crate::settings::ARVORE_DESLOC;
                self.settings.arvore_largura = tree_ui::LARGURA;
                ui.close();
            }
            if ui.button(tr("Fechar a árvore")).on_hover_text(tr("Volta pelo menu Exibir, em Árvore")).clicked() {
                self.settings.arvore_fechada = true;
                ui.close();
            }
        });
        opcoes.response.on_hover_text(tr("Opções da árvore"));
        contorno.aplicar(&barra);
        let mut arrastando = alca.dragged();
        if !self.settings.arvore_minimizada {
            tree_ui::comecar(&self.arvore);
            let zoom = self.arvore.zoom;
            let caixa = Rect::from_min_size(regiao.min + self.arvore.pan, egui::vec2(largura * zoom, 1e5));
            let mut ui = root.new_child(egui::UiBuilder::new().id_salt("arvore_sobre").max_rect(caixa));
            ui.set_clip_rect(regiao.intersect(root.clip_rect()));
            tree_ui::estilo(&mut ui, zoom);
            // a área das linhas fica mesmo travada: senão o arraste sobre a árvore vaza para o 3D e gira a vista
            if zona.is_positive() {
                let arrasto = ui.interact(zona, egui::Id::unique("arvore_sobre_mover"), egui::Sense::drag());
                if self.settings.arvore_solta {
                    self.arvore.pan += arrasto.drag_delta();
                }
            }
            let contorno = Contorno::reservar(&ui, regiao);
            match self.left {
                LeftTab::Peca if self.settings.bench == crate::settings::Bench::Desenho => crate::dwg_ui::tree(self, &mut ui),
                LeftTab::Peca if self.settings.bench == crate::settings::Bench::Montagem => crate::medir_quadros::fase("arvore", || crate::asm_ui::tree(self, &mut ui)),
                LeftTab::Peca => self.tree(&mut ui),
                LeftTab::Projeto => self.project_panel(&mut ui),
            }
            contorno.aplicar(&ui);
            self.arvore.conteudo = ui.min_rect();
            self.arvore.pan = tree_ui::limitar_pan(self.arvore.pan, ui.min_rect().size(), regiao.size());
            if !tree_ui::terminar(&ctx, &mut self.arvore) {
                self.toast(tr("Nenhum item ativo para localizar").to_string(), false);
            }
            // beira da direita: arrastar muda a largura
            let baixo = self.arvore.conteudo.bottom().clamp(regiao.top() + LINHA, regiao.bottom());
            let beira = Rect::from_min_max(egui::pos2(regiao.right() - 3., topo.top()), egui::pos2(regiao.right() + 3., baixo));
            let r = root.interact(beira, egui::Id::unique("arvore_sobre_largura"), egui::Sense::drag());
            if r.hovered() || r.dragged() {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                root.painter().vline(beira.center().x, beira.y_range(), egui::Stroke::new(1., crate::theme::accent(root.visuals()).gamma_multiply(0.6)));
            }
            let r = r.on_hover_text(tr("Arraste para mudar a largura da árvore"));
            if r.dragged() {
                novo.1 += r.drag_delta().x;
                arrastando = true;
            }
        } else {
            self.arvore.conteudo = Rect::NOTHING;
        }
        if arrastando {
            ctx.data_mut(|d| d.insert_temp(id_arvore_viva(), ([novo.0.x, novo.0.y], novo.1)));
        } else if viva.is_some() {
            // soltou: grava a posição e a largura
            ctx.data_mut(|d| d.remove::<([f32; 2], f32)>(id_arvore_viva()));
            self.settings.arvore_desloc = [desloc.x, desloc.y];
            self.settings.arvore_largura = largura;
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

    /// B503: sem comando, e ao lado das propriedades de um item, a árvore fica transparente sobre a
    /// vista; com comando (campos ou árvore pela seta), ela mora na coluna. A árvore movida ou alargada
    /// além da área volta para dentro dela.
    #[test]
    fn arvore_transparente_sobre_a_vista_sem_comando() {
        assert!(arvore_sobre_a_vista(conteudo(false, false, false, false, false)));
        assert!(arvore_sobre_a_vista(conteudo(false, false, false, false, true)), "minimizada: só o cabeçalho, sobre a vista");
        assert!(arvore_sobre_a_vista(conteudo(true, false, false, false, false)), "propriedades na coluna, árvore ao lado");
        assert!(!arvore_sobre_a_vista(conteudo(true, true, false, false, false)));
        assert!(!arvore_sobre_a_vista(conteudo(true, true, true, false, false)));
        assert!(!arvore_sobre_a_vista(conteudo(false, false, false, true, false)), "fechada");
        let area = Rect::from_min_size(egui::pos2(100., 60.), egui::vec2(900., 600.));
        let (d, l) = arvore_na_area(crate::settings::ARVORE_DESLOC, crate::tree_ui::LARGURA, area);
        assert_eq!((d, l), (egui::vec2(6., 4.), crate::tree_ui::LARGURA));
        let (d, l) = arvore_na_area([5000., -40.], 5000., area);
        assert!(l <= area.width() - 12. && d.x + l <= area.width() + 0.5 && d.y == 0., "{d:?} {l}");
        let (_, l) = arvore_na_area([0., 0.], 10., area);
        assert_eq!(l, ARVORE_MIN);
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
