// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural, B332: bloco "Dados para a dobradeira" de cada tubo dobrado na lista de corte. Exporta
//! a rota em LRA, YBC ou XYZ, e importa um arquivo para conferir se ele ainda bate com o tubo.

use crate::app::App;
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_solidos::st_dobra::TuboDobrado;
use fenkra_solidos::st_dobra_maquina::{self as maq, Formato, Modo, Opcoes};

/// Escolhas do bloco, guardadas junto da janela da lista de corte.
#[derive(Clone, Default)]
pub struct Estado {
    pub absoluto: bool,
    pub angulo_maquina: bool,
    pub inverter: bool,
    /// resultado da última conferência de arquivo
    pub conferencia: String,
}

impl Estado {
    pub fn opcoes(&self) -> Opcoes {
        Opcoes { modo: if self.absoluto { Modo::Absoluto } else { Modo::Incremental }, angulo_maquina: self.angulo_maquina, inverter_rotacao: self.inverter }
    }
}

/// O que o clique pediu (o diálogo de arquivo abre depois de fechar a janela).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acao {
    Exportar(usize, Formato),
    Importar(usize),
}

/// A tabela do tubo em texto simples, para mostrar na janela sem abrir arquivo.
pub fn tabela(t: &TuboDobrado, est: &Estado) -> Result<String, String> {
    let rota = maq::rota_do_tubo(t)?;
    maq::exportar(&rota, Formato::Lra, est.opcoes(), &t.designacao)
}

/// Desenha o bloco de um tubo. Devolve a ação pedida, se houver.
pub fn bloco(ui: &mut egui::Ui, t: &TuboDobrado, n: usize, est: &mut Estado) -> Option<Acao> {
    let mut acao = None;
    match maq::rota_do_tubo(t) {
        Err(e) => {
            ui.label(RichText::new(format!("{}: {}", tr("Dados para a dobradeira indisponíveis"), tr(&e))).weak().small());
        }
        Ok(rota) => {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Dados para a dobradeira")).small().strong());
                for f in [Formato::Lra, Formato::Ybc, Formato::Xyz] {
                    if ui.small_button(format!("{} {}…", tr("Exportar"), f.nome())).on_hover_text(tr(match f {
                        Formato::Lra => "L: reto entre tangentes; R: rotação entre planos de dobra; A: ângulo. Texto com ponto e vírgula.",
                        Formato::Ybc => "Os mesmos dados de LRA com as letras Y (avanço), B (rotação) e C (ângulo).",
                        Formato::Xyz => "Pontos do eixo do tubo e o raio de cada dobra, no sistema da rota (início na origem).",
                    })).clicked()
                    {
                        acao = Some(Acao::Exportar(n, f));
                    }
                }
                if ui.small_button(tr("Conferir arquivo…")).on_hover_text(tr("Lê um arquivo LRA, YBC ou XYZ e diz se ele bate com este tubo.")).clicked() {
                    acao = Some(Acao::Importar(n));
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut est.absoluto, tr("Absoluto")).on_hover_text(tr("Avanço e rotação acumulados desde o começo do tubo, no lugar de cada valor por dobra."));
                ui.checkbox(&mut est.angulo_maquina, tr("Ângulo com retorno")).on_hover_text(tr("Escreve o ângulo que a máquina dobra (com o retorno elástico da regra), no lugar do geométrico."));
                ui.checkbox(&mut est.inverter, tr("Inverter rotação")).on_hover_text(tr("Para máquinas que giram o tubo no sentido contrário. Valide a convenção no software da máquina antes de produzir."));
            });
            ui.label(RichText::new(format!("{} {} mm · {}", tr("desenvolvido"), crate::app::fmt(rota.desenvolvido()), rota.retos.iter().map(|l| crate::app::fmt(*l)).collect::<Vec<_>>().join(" / "))).weak().small());
        }
    }
    if !est.conferencia.is_empty() {
        ui.label(RichText::new(est.conferencia.clone()).small());
    }
    acao
}

/// Executa a ação (abre o diálogo de arquivo e grava ou lê).
pub fn executar(app: &mut App, tubos: &[TuboDobrado], acao: Acao, est: &mut Estado) {
    match acao {
        Acao::Exportar(n, f) => {
            let Some(t) = tubos.get(n) else { return };
            let texto = maq::rota_do_tubo(t).and_then(|r| maq::exportar(&r, f, est.opcoes(), &t.designacao));
            match texto {
                Err(e) => app.toast(tr(&e).to_string(), true),
                Ok(txt) => {
                    let nome = format!("tubo_{}.{}", n + 1, f.extensao());
                    if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name(&nome).save_file() {
                        match std::fs::write(&p, txt) {
                            Ok(()) => app.toast(tr("Dados de dobra exportados"), false),
                            Err(e) => app.toast(format!("Erro: {e}"), true),
                        }
                    }
                }
            }
        }
        Acao::Importar(n) => {
            let Some(t) = tubos.get(n) else { return };
            let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv", "txt"]).pick_file() else { return };
            est.conferencia = match std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|s| maq::importar(&s)) {
                Err(e) => format!("⚠ {e}"),
                Ok(lido) => match maq::rota_do_tubo(t).ok().and_then(|r| maq::diferenca(&r, &lido.rota)) {
                    Some(d) if d < 0.01 => format!("{} {} ({})", tr("O arquivo bate com o tubo"), n + 1, lido.formato.nome()),
                    Some(d) => format!("⚠ {} {}: {} {:.3}", tr("O arquivo difere do tubo"), n + 1, tr("maior diferença"), d),
                    None => format!("⚠ {} {}", tr("O arquivo tem outro número de dobras que o tubo"), n + 1),
                },
            };
        }
    }
}
