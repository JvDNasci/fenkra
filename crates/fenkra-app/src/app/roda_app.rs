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
            self.curso_ui.usou(i);
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
        let contexto = self.roda_contexto();
        let Some((saida, paginas)) = passo_do_quadro(&mut self.roda, &self.settings.roda, &contexto, ctx, digitando, &b, tecla) else { return };
        let (t, pos) = ctx.input(|i| (i.time, i.pointer.latest_pos().map_or([0., 0.], |p| [p.x, p.y])));
        let np = paginas.len();
        let oc = paginas.get(self.roda.pagina.min(np.saturating_sub(1))).map_or(0, Vec::len);
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

/// O que a roda decide num quadro: avança a máquina com a tecla e o mouse e devolve a saída com as
/// páginas do contexto.
///
/// Com a roda fechada e a tecla solta não há o que decidir, e a função volta `None` sem montar o
/// conteúdo. Esse conteúdo consulta o catálogo de comandos; montá-lo todo quadro, mesmo parado, custava
/// uns 29 ms e deixava o app em 30 quadros por segundo (B347).
pub(super) fn passo_do_quadro(roda: &mut roda::Roda, cfg: &roda::Config, contexto: &Contexto, ctx: &egui::Context, digitando: bool, b: &crate::keymap::Binding, tecla: egui::Key) -> Option<(Saida, Vec<Vec<String>>)> {
    let apertou = !digitando && ctx.input(|i| b.pressed(i));
    if !roda.aberta() && !apertou {
        return None;
    }
    let (t, pos) = ctx.input(|i| (i.time, i.pointer.latest_pos().map_or([0., 0.], |p| [p.x, p.y])));
    let icones: Vec<String> = cfg.icones(contexto).into_iter().filter(|i| roda::item(i).is_some()).collect();
    let provaveis = cfg.uso.provaveis(2);
    let paginas = roda::paginas(&icones, &provaveis);
    let pagina_atual = roda.pagina.min(paginas.len().saturating_sub(1));
    let oc = paginas.get(pagina_atual).map_or(0, Vec::len);
    let np = paginas.len();
    let mut saida = Saida::Nada;
    if !roda.aberta() {
        if oc > 0 {
            saida = roda.passo(Evento::Tecla { t, pos }, np, oc);
        }
    } else {
        let (solta, clique, esc, rolagem, numero) = ctx.input(|i| {
            let numero = [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3, egui::Key::Num4, egui::Key::Num5, egui::Key::Num6, egui::Key::Num7, egui::Key::Num8].iter().position(|k| i.key_pressed(*k)).map(|n| n + 1);
            (i.key_released(tecla), i.pointer.primary_clicked(), i.key_pressed(egui::Key::Escape), i.smooth_scroll_delta.y, numero)
        });
        if esc {
            saida = roda.passo(Evento::Esc, np, oc);
        } else if let Some(n) = numero {
            saida = roda.passo(Evento::Numero(n), np, oc);
        } else if rolagem != 0. {
            saida = roda.passo(Evento::Roda { passos: if rolagem > 0. { -1 } else { 1 } }, np, oc);
        } else if matches!(roda.fase, roda::Fase::Segurando { .. }) && solta {
            saida = roda.passo(Evento::Solta { t, pos }, np, oc);
        } else if matches!(roda.fase, roda::Fase::Fixa { .. }) && clique {
            saida = roda.passo(Evento::Clique { pos }, np, oc);
        }
    }
    Some((saida, paginas))
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::settings::Bench;

    fn contexto() -> Contexto {
        Contexto { bancada: Bench::Solidos, modo: Modo::Peca, selecao: Selecao::Nada }
    }

    fn tecla_roda() -> crate::keymap::Binding {
        crate::keymap::Keymap::default().binding(KeyAction::RodaComandos).expect("a roda tem tecla padrão")
    }

    /// Roda `n` quadros vazios do egui chamando o passo da roda a cada um. Devolve em quantos o
    /// passo teve algo a decidir.
    fn quadros_parados(n: usize, roda: &mut roda::Roda) -> usize {
        let cfg = roda::Config::default();
        let b = tecla_roda();
        let tecla = egui::Key::from_name(&b.key).expect("tecla conhecida");
        let ctx = egui::Context::default();
        let mut decididos = 0;
        for i in 0..n {
            let entrada = egui::RawInput { time: Some(i as f64 / 60.), ..Default::default() };
            let mut saida = ctx.run_ui(entrada, |ui| {
                if passo_do_quadro(roda, &cfg, &contexto(), ui.ctx(), false, &b, tecla).is_some() {
                    decididos += 1;
                }
            });
            saida.textures_delta.clear();
        }
        decididos
    }

    /// B347: a roda fechada, sem a tecla, não monta o catálogo de comandos em quadro nenhum. Antes
    /// ela o montava uma vez por ícone da roda a cada quadro (cerca de 29 ms no total).
    #[test]
    fn roda_fechada_nao_monta_o_catalogo_a_cada_quadro() {
        // aquece o catálogo para que a contagem abaixo só veja montagens feitas pelos quadros
        let _ = crate::ajuda_cmd::catalogo();
        let antes = crate::ajuda_cmd::montagens_do_catalogo();
        let mut roda = roda::Roda::default();
        let decididos = quadros_parados(120, &mut roda);
        assert_eq!(decididos, 0, "sem tecla e com a roda fechada não há o que decidir");
        assert_eq!(crate::ajuda_cmd::montagens_do_catalogo(), antes, "o catálogo foi remontado");
        assert!(!roda.aberta());
    }

    /// Com a roda aberta o passo roda de verdade, e mesmo assim consulta o catálogo sem remontá-lo.
    #[test]
    fn roda_aberta_decide_e_reaproveita_o_catalogo() {
        let _ = crate::ajuda_cmd::catalogo();
        let antes = crate::ajuda_cmd::montagens_do_catalogo();
        let mut roda = roda::Roda { fase: roda::Fase::Fixa { p0: [100., 100.] }, pagina: 0 };
        let decididos = quadros_parados(30, &mut roda);
        assert_eq!(decididos, 30);
        assert_eq!(crate::ajuda_cmd::montagens_do_catalogo(), antes, "o catálogo é montado uma vez só");
    }
}
