// SPDX-License-Identifier: LGPL-2.1-or-later
//! Ligação da roda de comandos (B256) ao app: tecla, contexto, execução e uso aprendido.

use super::*;
use crate::keymap::KeyAction;
use crate::roda::{self, Contexto, Evento, Modo, Saida, Selecao};

impl App {
    /// Em que a roda está: bancada, modo e o tipo da última seleção no 3D.
    fn roda_contexto(&self) -> Contexto {
        let modo = if self.sketch.is_some() {
            Modo::Sketch
        } else if self.in_asm() {
            Modo::Montagem
        } else if self.settings.bench == crate::settings::Bench::Desenho {
            Modo::Desenho
        } else {
            Modo::Peca
        };
        let selecao = match self.picks.last() {
            None => Selecao::Nada,
            Some(crate::picking::Pick::Face { .. }) => Selecao::Face,
            Some(crate::picking::Pick::Edge { .. }) => Selecao::Aresta,
            Some(_) => Selecao::Outra,
        };
        Contexto { bancada: self.settings.bench, modo, selecao }
    }

    /// Anota o comando executado para aprender os próximos prováveis.
    pub(super) fn roda_registrar(&mut self, a: Act) {
        if let Some(i) = roda::icone_da_acao(a) {
            self.settings.roda.uso.registrar(i);
        }
    }

    /// Uma etapa por quadro: lê a tecla e o mouse, avança a máquina, desenha e executa.
    pub(super) fn roda_passo(&mut self, ctx: &egui::Context, digitando: bool) {
        roda::ui::editor(ctx, &mut self.settings.roda);
        let semana = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 86_400 / 7).unwrap_or(0);
        if self.settings.roda.uso.semana != semana {
            self.settings.roda.uso.decair(semana);
        }
        let Some(b) = self.settings.keys.binding(KeyAction::RodaComandos) else { return };
        let Some(tecla) = egui::Key::from_name(&b.key) else { return };
        let (t, pos) = ctx.input(|i| (i.time, i.pointer.latest_pos().map_or([0., 0.], |p| [p.x, p.y])));
        let icones: Vec<String> = self.settings.roda.icones(&self.roda_contexto()).into_iter().filter(|i| roda::item(i).is_some()).collect();
        let provaveis = self.settings.roda.uso.provaveis(2);
        let paginas = roda::paginas(&icones, &provaveis);
        let pagina_atual = self.roda.pagina.min(paginas.len().saturating_sub(1));
        let na_pagina: Vec<String> = paginas.get(pagina_atual).cloned().unwrap_or_default();
        let (np, oc) = (paginas.len(), na_pagina.len());
        let mut saida = Saida::Nada;
        if !self.roda.aberta() {
            let aperta = !digitando && ctx.input(|i| b.pressed(i));
            if aperta && oc > 0 {
                saida = self.roda.passo(Evento::Tecla { t, pos }, np, oc);
            }
        } else {
            let (solta, clique, esc, rolagem, numero) = ctx.input(|i| {
                let numero = [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3, egui::Key::Num4, egui::Key::Num5, egui::Key::Num6, egui::Key::Num7, egui::Key::Num8].iter().position(|k| i.key_pressed(*k)).map(|n| n + 1);
                (i.key_released(tecla), i.pointer.primary_clicked(), i.key_pressed(egui::Key::Escape), i.raw_scroll_delta.y, numero)
            });
            if esc {
                saida = self.roda.passo(Evento::Esc, np, oc);
            } else if let Some(n) = numero {
                saida = self.roda.passo(Evento::Numero(n), np, oc);
            } else if rolagem != 0. {
                saida = self.roda.passo(Evento::Roda { passos: if rolagem > 0. { -1 } else { 1 } }, np, oc);
            } else if matches!(self.roda.fase, roda::Fase::Segurando { .. }) && solta {
                saida = self.roda.passo(Evento::Solta { t, pos }, np, oc);
            } else if matches!(self.roda.fase, roda::Fase::Fixa { .. }) && clique {
                saida = self.roda.passo(Evento::Clique { pos }, np, oc);
            }
        }
        // a rolagem ou a tecla já foram usadas: o resto do app não as vê neste quadro
        if self.roda.aberta() {
            ctx.request_repaint();
            if self.roda.desenhada(t) {
                if let Some(c) = self.roda.centro() {
                    let alvo = roda::setor_do_deslocamento(pos[0] - c[0], pos[1] - c[1]).filter(|s| *s < oc);
                    let pagina = self.roda.pagina.min(np.saturating_sub(1));
                    let atual = paginas.get(pagina).map(Vec::as_slice).unwrap_or(&[]);
                    roda::ui::desenhar(ctx, egui::pos2(c[0], c[1]), atual, pagina, np, alvo);
                }
            }
        }
        if let Saida::Executar(i) = saida {
            // a página pode ter mudado no mesmo quadro: usa a que o usuário via
            let pagina = self.roda.pagina.min(np.saturating_sub(1));
            if let Some(a) = paginas.get(pagina).and_then(|p| p.get(i)).and_then(|ic| roda::item(ic)).map(|it| it.act) {
                self.act(a);
            }
        }
    }
}
