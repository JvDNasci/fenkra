// SPDX-License-Identifier: LGPL-2.1-or-later
//! Exportar visualização (B181, B338, B365). Painel aberto só durante o comando.
//!
//! A saída padrão é uma página HTML autocontida: o pacote vai dentro dela, junto com o visualizador, e
//! ela abre com duplo clique em qualquer navegador, sem rede e sem o Fenkra. O `.fkv` puro é a opção
//! avançada (arquivo menor, para quem hospeda a página ou usa o visualizador solto). O navegador não
//! roda o kernel, então é este arquivo, gerado aqui, que vai para a web. O documento só é lido.

use super::*;
use crate::paineis::{self, Encaixe, Painel};
use fenkra_pacote::{de_desenho, de_montagem, de_peca, pagina_html, Opcoes, Pacote};
use std::path::Path;

/// Fração de triângulos que a visualização simplificada mantém.
const SIMPLIFICADA: f64 = 0.3;
const LARGURA: f32 = 320.;

/// Entrega o arquivo ao navegador padrão (para `.html`).
pub(crate) fn abrir_no_navegador(caminho: &Path) -> std::io::Result<()> {
    super::visualizador_plm::abrir_no_sistema(caminho)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Saida {
    Html,
    Fkv,
}

impl Saida {
    fn extensao(self) -> &'static str {
        match self {
            Saida::Html => "html",
            Saida::Fkv => "fkv",
        }
    }
}

/// O que foi gravado, para o painel oferecer abrir.
struct Gravado {
    caminho: PathBuf,
    kb: usize,
}

pub struct Dialogo {
    saida: Saida,
    simplificada: bool,
    avancado_aberto: bool,
    gravado: Option<Gravado>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Acao {
    Nada,
    Gravar,
    Abrir,
    Fechar,
}

impl Dialogo {
    fn novo(simplificada: bool) -> Dialogo {
        Dialogo { saida: Saida::Html, simplificada, avancado_aberto: false, gravado: None }
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Acao {
        let mut acao = Acao::Nada;
        ui.set_max_width(LARGURA - 20.);
        ui.label(tr("O que sai"));
        ui.radio_value(&mut self.saida, Saida::Html, tr("Página web (.html)")).on_hover_text(tr("Um arquivo só, com o visualizador e o modelo dentro. Abre com duplo clique no navegador, sem internet e sem o Fenkra."));
        ui.checkbox(&mut self.simplificada, tr("Versão simplificada")).on_hover_text(tr("Reduz a malha e tira os pontos de medição exatos: protege a geometria e o arquivo fica menor."));
        egui::CollapsingHeader::new(tr("Avançado")).id_salt("visualizacao_avancado").default_open(self.avancado_aberto || self.saida == Saida::Fkv).show(ui, |ui| {
            ui.radio_value(&mut self.saida, Saida::Fkv, tr("Pacote (.fkv)")).on_hover_text(tr("Só o modelo, sem a página. Abre pela página Fenkra Visualizador.html (arrastar ou escolher o arquivo), por um link ou pela associação do .fkv em Configurações."));
        });
        ui.add_space(4.);
        egui::CollapsingHeader::new(tr("Como abrir")).id_salt("visualizacao_como").default_open(true).show(ui, |ui| {
            ui.label(RichText::new(tr("No computador: dê duplo clique no arquivo .html. Ele abre no navegador, sem internet. Um .fkv abre arrastando-o para a página Fenkra Visualizador.html, ou por duplo clique depois de associá-lo em Configurações.")).small());
            ui.add_space(2.);
            ui.label(RichText::new(tr("No celular: envie o arquivo para o aparelho e toque nele. Nem todo celular abre arquivo local no navegador, e isso depende do sistema. O caminho garantido é colocar o .html num endereço seu e abrir o link.")).small());
        });
        ui.add_space(6.);
        if let Some(g) = &self.gravado {
            ui.label(RichText::new(format!("{}: {} ({} KB)", tr("Exportado"), g.caminho.display(), g.kb)).small());
            ui.horizontal(|ui| {
                if ui.button(tr("Abrir no navegador")).on_hover_text(tr("Abre o arquivo gravado no navegador padrão, para conferir como ele fica.")).clicked() {
                    acao = Acao::Abrir;
                }
                if ui.button(tr("Fechar")).clicked() {
                    acao = Acao::Fechar;
                }
            });
            ui.add_space(4.);
        }
        ui.horizontal(|ui| {
            if ui.button(tr("Exportar…")).on_hover_text(tr("Escolhe o arquivo e grava.")).clicked() {
                acao = Acao::Gravar;
            }
            if self.gravado.is_none() && ui.button(tr("Cancelar")).clicked() {
                acao = Acao::Fechar;
            }
        });
        acao
    }
}

impl App {
    fn pacote_do_que_esta_aberto(&self, o: &Opcoes) -> Result<(Pacote, String), String> {
        if self.settings.bench == crate::settings::Bench::Desenho {
            if let Some(d) = &self.dwg {
                let pasta = d.path.as_ref().and_then(|p| p.parent()).ok_or_else(|| tr("Salve o desenho antes de exportar: as peças das vistas são lidas dos arquivos dele.").to_string())?;
                return de_desenho(&d.d, pasta, o).map(|p| (p, d.d.name.clone()));
            }
        }
        if self.in_asm() {
            let s = self.asm.as_ref().ok_or("sem montagem")?;
            let pasta = s.file.as_ref().and_then(|p| p.parent()).ok_or_else(|| tr("Salve a montagem antes de exportar: as peças são lidas dos arquivos dela.").to_string())?;
            return de_montagem(&s.asm, pasta, o).map(|p| (p, s.asm.name.clone()));
        }
        de_peca(&self.open.doc, o).map(|p| (p, self.open.doc.name.clone()))
    }

    /// Abre o painel do comando. `simplificada` já liga a opção (atalho do menu).
    pub(super) fn exportar_visualizacao(&mut self, simplificada: bool) {
        self.visualizacao_dialogo = Some(Dialogo::novo(simplificada));
    }

    pub(super) fn painel_visualizacao(&mut self, ctx: &egui::Context, area: Rect) {
        let Some(mut d) = self.visualizacao_dialogo.take() else { return };
        let padrao = paineis::abaixo_do_cubo(area, LARGURA);
        let cfg = Painel { nome: "visualizacao".into(), titulo: tr("Exportar visualização").to_string(), padrao, largura: LARGURA, redimensiona: false };
        let posicao = self.settings.painel("visualizacao", Encaixe::Direita);
        let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| d.ui(ui));
        if let Some(p) = saida.movido {
            self.settings.guardar_painel("visualizacao", p);
        }
        match saida.conteudo.unwrap_or(Acao::Nada) {
            Acao::Nada => self.visualizacao_dialogo = Some(d),
            Acao::Fechar => {}
            Acao::Gravar => {
                self.gravar_visualizacao(&mut d);
                self.visualizacao_dialogo = Some(d);
            }
            Acao::Abrir => {
                self.abrir_gravado(&d);
                self.visualizacao_dialogo = Some(d);
            }
        }
    }

    /// Abre o que foi gravado: a página direto; o `.fkv` por uma página temporária com ele dentro.
    fn abrir_gravado(&mut self, d: &Dialogo) {
        let Some(g) = &d.gravado else { return };
        let r = match d.saida {
            Saida::Html => abrir_no_navegador(&g.caminho).map_err(|e| e.to_string()),
            Saida::Fkv => crate::assoc_fkv::pagina_do_fkv(&g.caminho).and_then(|p| abrir_no_navegador(&p).map_err(|e| e.to_string())),
        };
        if let Err(e) = r {
            self.toast(format!("{}: {e}", tr("Falha ao abrir a visualização")), true);
        }
    }

    fn gravar_visualizacao(&mut self, d: &mut Dialogo) {
        let o = if d.simplificada { Opcoes { decimar: Some(SIMPLIFICADA), capturas: false, ..Default::default() } } else { Opcoes::default() };
        let (pacote, nome) = match self.pacote_do_que_esta_aberto(&o) {
            Ok(x) => x,
            Err(e) => return self.toast(format!("{}: {e}", tr("Falha ao exportar")), true),
        };
        // desenvolvimento: FENKRA_DEMO_VISUALIZACAO_ARQUIVO=<arquivo> grava sem perguntar
        let demo = std::env::var("FENKRA_DEMO_VISUALIZACAO_ARQUIVO").ok().map(PathBuf::from);
        let escolhido = demo.or_else(|| match d.saida {
            Saida::Html => rfd::FileDialog::new().add_filter(tr("Página web"), &["html"]).set_file_name(format!("{nome}.html")).save_file(),
            Saida::Fkv => rfd::FileDialog::new().add_filter(tr("Pacote de visualização"), &["fkv"]).set_file_name(format!("{nome}.fkv")).save_file(),
        });
        let Some(caminho) = escolhido else { return };
        // a extensão acompanha a escolha: arquivo sem extensão ou com a errada não abre com duplo clique
        let caminho = caminho.with_extension(d.saida.extensao());
        let bytes = match pacote.bytes() {
            Ok(b) => b,
            Err(e) => return self.toast(format!("{}: {e}", tr("Falha ao exportar")), true),
        };
        let r = match d.saida {
            Saida::Html => std::fs::write(&caminho, pagina_html(Some(&bytes))),
            Saida::Fkv => std::fs::write(&caminho, &bytes),
        };
        match r {
            Ok(()) => {
                let kb = std::fs::metadata(&caminho).map_or(bytes.len(), |m| m.len() as usize) / 1024;
                self.toast(format!("{}: {} ({} KB)", tr("Exportado"), caminho.display(), kb), false);
                d.gravado = Some(Gravado { caminho, kb });
            }
            Err(e) => self.toast(format!("{}: {e}", tr("Falha ao exportar")), true),
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn textos(d: &mut Dialogo) -> String {
        let ctx = egui::Context::default();
        let mut textos = vec![];
        for _ in 0..2 {
            let entrada = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400., 900.))), ..Default::default() };
            let mut saida = ctx.run_ui(entrada, |ui| {
                d.ui(ui);
            });
            saida.textures_delta.clear();
            textos = saida.shapes.iter().filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) => Some(t.galley.text().to_string()),
                _ => None,
            }).collect();
        }
        textos.join("\n")
    }

    #[test]
    fn a_saida_padrao_e_a_pagina_html() {
        let d = Dialogo::novo(false);
        assert_eq!(d.saida, Saida::Html);
        assert_eq!(d.saida.extensao(), "html");
        assert!(Dialogo::novo(true).simplificada);
    }

    #[test]
    fn painel_explica_computador_e_celular_e_deixa_o_fkv_no_avancado() {
        let t = textos(&mut Dialogo::novo(false));
        assert!(t.contains("Página web (.html)") && t.contains("Avançado"), "{t}");
        assert!(t.contains("No computador") && t.contains("No celular"), "{t}");
        assert!(t.contains("caminho garantido"), "{t}");
        // o .fkv só aparece com o avançado aberto, que abre sozinho quando ele está escolhido
        assert!(!t.contains("Pacote (.fkv)"), "{t}");
        let mut f = Dialogo::novo(false);
        f.saida = Saida::Fkv;
        assert!(textos(&mut f).contains("Pacote (.fkv)"));
    }

    #[test]
    fn depois_de_gravar_oferece_abrir_no_navegador() {
        let mut d = Dialogo::novo(false);
        assert!(!textos(&mut d).contains("Abrir no navegador"));
        d.gravado = Some(Gravado { caminho: PathBuf::from("peca.html"), kb: 12 });
        let t = textos(&mut d);
        assert!(t.contains("Abrir no navegador") && t.contains("peca.html") && t.contains("12 KB"), "{t}");
    }

    #[test]
    fn textos_do_painel_tem_traducao() {
        for s in [
            "O que sai",
            "Página web (.html)",
            "Pacote (.fkv)",
            "Como abrir",
            "Abrir no navegador",
            "Exportar visualização",
            "Falha ao abrir a visualização",
            "Página web",
        ] {
            assert!(crate::i18n::tem_traducao(s), "sem tradução: {s}");
        }
    }
}
