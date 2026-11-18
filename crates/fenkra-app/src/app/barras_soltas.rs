// SPDX-License-Identifier: LGPL-2.1-or-later
//! Barras soltas (B502, ADR 0104): arrastar pela alça, flutuar, encaixar em qualquer borda com prévia,
//! fechar no "x" e reabrir pelo menu Janela.
//!
//! O arraste começa na alça de qualquer barra, encaixada ou flutuante (`Act::DockDrag`). Enquanto o
//! botão está apertado, um fantasma da barra segue o cursor; perto de uma borda, a faixa daquela borda
//! acende (a prévia de onde a barra vai encaixar) e o fantasma desliza até ela. Soltar numa borda
//! encaixa; soltar longe delas deixa a barra flutuando onde foi solta. Tudo vai para
//! `Settings::barras`, por bancada (`barras_layout`).

use super::*;
use crate::barras_layout::{self, Barra};
use crate::toolbar::Forma;

/// Mais ícones que isto por fila, a barra flutuante quebra em outra fila.
const ICONES_POR_FILA: usize = 8;

impl App {
    /// Barras flutuantes da bancada: janelinhas com alça, nome e "x", acima da vista.
    pub(super) fn barras_flutuantes(&mut self, ctx: &egui::Context) {
        let sketching = self.sketch.is_some();
        let tela = ctx.content_rect();
        let flutuantes = self.settings.barras_em(Dock::Flutuante, sketching);
        if flutuantes.is_empty() {
            return;
        }
        let ts = self.tool_state();
        let celula = toolbar::celula(toolbar::tamanho_na_coluna(&self.settings));
        let mut acts = vec![];
        for (k, b) in flutuantes.into_iter().enumerate() {
            let n = match b {
                Barra::Grupo(g) => toolbar::entradas_na_barra(g, &self.settings).len(),
                Barra::Minha(i) => self.settings.custom.groups.get(i).map_or(0, |c| c.items.len()),
            };
            if n == 0 && matches!(b, Barra::Minha(_)) {
                continue;
            }
            let nome = self.settings.nome_da_barra(b);
            let largura = (n.clamp(3, ICONES_POR_FILA) as f32) * celula.x + 4.;
            let id = egui::Id::unique(("barra_flutuante", format!("{b:?}")));
            // tamanho do quadro anterior, para manter a barra inteira dentro da tela
            let tam = ctx.memory(|m| m.area_rect(id)).map_or(egui::vec2(largura + 16., celula.y * 2. + 30.), |r| r.size());
            let padrao = egui::pos2(tela.center().x - 120. + 24. * k as f32, tela.top() + 120. + 24. * k as f32);
            let canto = barras_layout::dentro_da_tela(self.settings.canto_da_barra(b).unwrap_or(padrao), tam, tela);
            let mut comecou = false;
            let mut fechar = false;
            let resp = egui::Area::new(id).order(egui::Order::Middle).fixed_pos(canto).movable(false).constrain(false).show(ctx, |ui| {
                let mut quadro = egui::Frame::window(ui.style()).inner_margin(egui::Margin::same(4));
                quadro.shadow = egui::Shadow::NONE;
                quadro.show(ui, |ui| {
                    ui.set_width(largura);
                    ui.horizontal(|ui| {
                        comecou = toolbar::alca(ui, egui::vec2(toolbar::ALCA, 16.), true, &nome);
                        ui.add(egui::Label::new(RichText::new(&nome).small().strong()).truncate().selectable(false));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if crate::paineis::botao_fechar(ui, tr("Fechar a barra. Ela volta pelo menu Janela, em Barras de ferramentas.")).clicked() {
                                fechar = true;
                            }
                        });
                    });
                    match b {
                        Barra::Grupo(g) => acts.extend(toolbar::show_group_com(ui, g, &mut self.settings, &ts, Forma::Coluna, false)),
                        Barra::Minha(i) => acts.extend(toolbar::show_custom(ui, i, &mut self.settings, &ts, Forma::Coluna, false)),
                    }
                });
            });
            if comecou {
                acts.push(Act::DockDrag(b, resp.response.rect));
            }
            if fechar {
                acts.push(Act::Barra(b, Dock::Oculta));
            }
        }
        for a in acts {
            self.act(a);
        }
    }

    /// Começa a arrastar a barra: o fantasma nasce no retângulo dela, preso onde o cursor pegou.
    pub(super) fn comecar_arraste_de_barra(&mut self, b: Barra, r: Rect) {
        let p = self.ctx_pointer.unwrap_or(r.center());
        self.dock_drag = Some(DockDrag { barra: b, grab: p - r.min, size: r.size(), ghost: r });
    }

    /// Arraste em curso: zonas das quatro bordas, a da vez acesa como prévia, e o fantasma seguindo o
    /// cursor. Ao soltar, encaixa na borda ou flutua ali.
    pub(super) fn arraste_de_barra(&mut self, ctx: &egui::Context) {
        let Some(mut dd) = self.dock_drag else { return };
        let tela = ctx.content_rect();
        let p = ctx.input(|i| i.pointer.interact_pos().or(i.pointer.latest_pos())).unwrap_or(dd.ghost.center());
        let alvo = barras_layout::zona(p, tela);
        let destino = match alvo.and_then(|d| barras_layout::previa(d, tela).map(|z| (d, z))) {
            Some((d, z)) => {
                let e = barras_layout::ESPESSURA_PREVIA;
                let comprido = dd.size.x.max(dd.size.y);
                match d {
                    Dock::Topo | Dock::Baixo => Rect::from_center_size(egui::pos2(p.x.clamp(z.left() + comprido / 2., (z.right() - comprido / 2.).max(z.left() + comprido / 2.)), z.center().y), egui::vec2(comprido, e)),
                    _ => Rect::from_center_size(egui::pos2(z.center().x, p.y.clamp(z.top() + comprido / 2., (z.bottom() - comprido / 2.).max(z.top() + comprido / 2.))), egui::vec2(e, comprido.min(z.height()))),
                }
            }
            None => Rect::from_min_size(p - dd.grab, dd.size),
        };
        // o fantasma se aproxima um pouco a cada quadro; sem animações, vai direto
        let k = if ctx.global_style().animation_time > 0. { 0.35 } else { 1. };
        dd.ghost = Rect::from_min_max(dd.ghost.min + (destino.min - dd.ghost.min) * k, dd.ghost.max + (destino.max - dd.ghost.max) * k);
        let v = ctx.global_style().visuals.clone();
        let destaque = crate::theme::accent(&v);
        let pt = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("fantasma_da_barra")));
        for d in [Dock::Topo, Dock::Lateral, Dock::Direita, Dock::Baixo] {
            let Some(z) = barras_layout::previa(d, tela) else { continue };
            let acesa = alvo == Some(d);
            pt.rect_filled(z, 6., destaque.gamma_multiply(if acesa { 0.28 } else { 0.06 }));
            pt.rect_stroke(z, 6., egui::Stroke::new(if acesa { 2. } else { 1. }, destaque.gamma_multiply(if acesa { 1. } else { 0.35 })), egui::StrokeKind::Inside);
        }
        pt.rect_filled(dd.ghost, 6., v.panel_fill.gamma_multiply(0.85));
        pt.rect_stroke(dd.ghost, 6., egui::Stroke::new(2., destaque), egui::StrokeKind::Inside);
        let nome = self.settings.nome_da_barra(dd.barra);
        pt.text(dd.ghost.center(), egui::Align2::CENTER_CENTER, nome, egui::FontId::proportional(12.), v.text_color());
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        ctx.request_repaint();
        if ctx.input(|i| i.pointer.any_released() || !i.pointer.primary_down()) {
            let canto = barras_layout::dentro_da_tela(p - dd.grab, dd.size, tela);
            self.settings.soltar_barra(dd.barra, alvo, canto);
            self.dock_drag = None;
        } else {
            self.dock_drag = Some(dd);
        }
    }

    /// Menu Janela (e Exibir) > Barras de ferramentas: uma caixa por barra da bancada, marcada se está
    /// aberta, com o lugar no submenu; nova caixa de comandos; restaurar.
    pub(super) fn menu_barras(&mut self, ui: &mut egui::Ui) {
        let sketching = self.sketch.is_some();
        ui.label(RichText::new(tr("Arraste a alça de uma barra para soltá-la ou encaixá-la noutra borda.")).small().weak());
        for b in self.settings.barras_da_bancada(sketching) {
            let doca = self.settings.doca_da_barra(b);
            let mut aberta = doca != Dock::Oculta;
            let nome = self.settings.nome_da_barra(b);
            ui.horizontal(|ui| {
                if ui.checkbox(&mut aberta, &nome).changed() {
                    self.settings.abrir_barra(b, aberta);
                }
                ui.menu_button(RichText::new("⋯").weak(), |ui| {
                    for d in Dock::ALL {
                        if ui.radio(doca == d, d.label()).clicked() {
                            self.settings.mover_barra(b, d);
                            ui.close();
                        }
                    }
                });
            });
        }
        ui.separator();
        if ui.button(tr("Nova barra de comandos…")).on_hover_text(tr("Cria uma caixa de comandos sua. Arraste os ícones da lista para dentro dela.")).clicked() {
            let n = self.settings.custom.groups.len();
            self.settings.custom.groups.push(crate::settings::CustomGroup { name: format!("{} {}", tr("Minha barra"), n + 1), items: vec![], sketch: sketching });
            self.customize_sel = crate::settings::Group::ALL.len() - 1 + n;
            self.customize_open = true;
            ui.close();
        }
        if ui.button(tr("⚙ Personalizar comandos…")).clicked() {
            self.customize_open = true;
            ui.close();
        }
        ui.separator();
        if ui.button(tr("Restaurar o layout desta bancada")).on_hover_text(tr("Volta as barras e o tamanho delas ao padrão desta bancada. As barras de comandos suas continuam.")).clicked() {
            self.settings.restaurar_barras();
            barras_layout::esquecer_tamanhos(ui.ctx());
            ui.close();
        }
        if ui.button(tr("Restaurar o layout de todas as bancadas")).on_hover_text(tr("Volta as barras de todas as bancadas ao padrão. As barras de comandos suas continuam.")).clicked() {
            self.settings.restaurar_todas_as_barras();
            barras_layout::esquecer_tamanhos(ui.ctx());
            ui.close();
        }
    }
}
