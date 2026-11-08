// SPDX-License-Identifier: LGPL-2.1-or-later
//! Exportar visualização (B181, B338, B365). Painel aberto só durante o comando.
//!
//! A saída padrão é uma página HTML autocontida: o pacote vai dentro dela, junto com o visualizador, e
//! ela abre com duplo clique em qualquer navegador, sem rede e sem o Fenkra. O `.fkv` puro é a opção
//! avançada (arquivo menor, para quem hospeda a página ou usa o visualizador solto). O navegador não
//! roda o kernel, então é este arquivo, gerado aqui, que vai para a web. O documento só é lido.

use super::*;
use crate::paineis::{self, Encaixe, Painel};
use fenkra_core::trabalho::Controle;
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

/// Geração do pacote numa thread (B427): o painel mostra o progresso e deixa cancelar, e a janela
/// continua respondendo. Montagem grande leva minutos.
struct Trabalho {
    rx: std::sync::mpsc::Receiver<Result<Pacote, String>>,
    controle: Controle,
    caminho: PathBuf,
    saida: Saida,
    nome: String,
}

pub struct Dialogo {
    saida: Saida,
    simplificada: bool,
    avancado_aberto: bool,
    gravado: Option<Gravado>,
    trabalho: Option<Trabalho>,
    /// o que está aberto é uma montagem: só nela há análise de interferências e folgas
    montagem: bool,
    /// incluir a análise no pacote (B400); `None` = acompanha a versão simplificada, que sai sem ela
    /// porque é o que leva mais de um minuto numa montagem grande
    analise: Option<bool>,
}

/// Quem gera o pacote do que está aberto, pronto para ir para a thread.
type Gerador = Box<dyn FnOnce(&Opcoes) -> Result<Pacote, String> + Send>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Acao {
    Nada,
    Gravar,
    Abrir,
    Fechar,
    /// parar a geração em andamento
    Parar,
}

impl Dialogo {
    fn novo(simplificada: bool) -> Dialogo {
        Dialogo { saida: Saida::Html, simplificada, avancado_aberto: false, gravado: None, trabalho: None, montagem: false, analise: None }
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Acao {
        let mut acao = Acao::Nada;
        ui.set_max_width(LARGURA - 20.);
        if let Some(t) = &self.trabalho {
            let (feitos, total) = t.controle.progresso();
            let frac = if total > 0 { (feitos as f32 / total as f32).clamp(0., 1.) } else { 0. };
            ui.label(format!("{}: {}", tr("Gerando"), t.nome));
            let texto = if total > 0 { format!("{} {} {}", feitos.min(total), tr("de"), total) } else { tr("Preparando…").to_string() };
            ui.add(egui::ProgressBar::new(frac).text(texto).animate(total == 0)).on_hover_text(tr("Montagem grande leva minutos. A janela continua respondendo; dá para cancelar."));
            if ui.button(tr("Cancelar")).on_hover_text(tr("Para a geração no próximo componente e não grava nada.")).clicked() {
                acao = Acao::Parar;
            }
            return acao;
        }
        ui.label(tr("O que sai"));
        ui.radio_value(&mut self.saida, Saida::Html, tr("Página web (.html)")).on_hover_text(tr("Um arquivo só, com o visualizador e o modelo dentro. Abre com duplo clique no navegador, sem internet e sem o Fenkra."));
        ui.checkbox(&mut self.simplificada, tr("Versão simplificada")).on_hover_text(tr("Reduz a malha e tira os pontos de medição exatos: protege a geometria e o arquivo fica menor."));
        if self.montagem {
            let mut on = self.analise.unwrap_or(!self.simplificada);
            if ui.checkbox(&mut on, tr("Incluir análise de interferências e folgas")).on_hover_text(tr("Calcula as interferências e as folgas entre as peças e as grava no pacote. Numa montagem grande leva mais de um minuto; a versão simplificada sai sem ela, a menos que você marque.")).changed() {
                self.analise = Some(on);
            }
        }
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
    /// O que está aberto, pronto para ser gerado numa thread: o nome e a função que gera. Copia o
    /// documento (só é lido) para a thread não depender da interface.
    fn gerador_do_que_esta_aberto(&self) -> Result<(String, Gerador), String> {
        if self.settings.bench == crate::settings::Bench::Desenho {
            if let Some(d) = &self.dwg {
                let pasta = d.path.as_ref().and_then(|p| p.parent()).ok_or_else(|| tr("Salve o desenho antes de exportar: as peças das vistas são lidas dos arquivos dele.").to_string())?.to_path_buf();
                let desenho = d.d.clone();
                return Ok((desenho.name.clone(), Box::new(move |o| de_desenho(&desenho, &pasta, o))));
            }
        }
        if self.in_asm() {
            let s = self.asm.as_ref().ok_or("sem montagem")?;
            let pasta = s.file.as_ref().and_then(|p| p.parent()).ok_or_else(|| tr("Salve a montagem antes de exportar: as peças são lidas dos arquivos dela.").to_string())?.to_path_buf();
            let asm = s.asm.clone();
            return Ok((asm.name.clone(), Box::new(move |o| de_montagem(&asm, &pasta, o))));
        }
        let doc = self.open.doc.clone();
        Ok((doc.name.clone(), Box::new(move |o| de_peca(&doc, o))))
    }

    /// Abre o painel do comando. `simplificada` já liga a opção (atalho do menu).
    pub(super) fn exportar_visualizacao(&mut self, simplificada: bool) {
        let mut d = Dialogo::novo(simplificada);
        d.montagem = self.in_asm();
        self.visualizacao_dialogo = Some(d);
    }

    pub(super) fn painel_visualizacao(&mut self, ctx: &egui::Context, area: Rect) {
        let Some(mut d) = self.visualizacao_dialogo.take() else { return };
        self.concluir_visualizacao(&mut d, ctx);
        let padrao = paineis::abaixo_do_cubo(area, LARGURA);
        let cfg = Painel { nome: "visualizacao".into(), titulo: tr("Exportar visualização").to_string(), padrao, largura: LARGURA, redimensiona: false };
        let posicao = self.settings.painel("visualizacao", Encaixe::Direita);
        let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| d.ui(ui));
        if let Some(p) = saida.movido {
            self.settings.guardar_painel("visualizacao", p);
        }
        match saida.conteudo.unwrap_or(Acao::Nada) {
            Acao::Nada => self.visualizacao_dialogo = Some(d),
            Acao::Fechar => {
                if let Some(t) = &d.trabalho {
                    t.controle.cancelar();
                }
            }
            Acao::Gravar => {
                self.gravar_visualizacao(&mut d, ctx);
                self.visualizacao_dialogo = Some(d);
            }
            Acao::Abrir => {
                self.abrir_gravado(&d);
                self.visualizacao_dialogo = Some(d);
            }
            Acao::Parar => {
                if let Some(t) = &d.trabalho {
                    t.controle.cancelar();
                }
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

    /// Escolhe o arquivo e começa a gerar numa thread; `concluir_visualizacao` grava quando acabar.
    fn gravar_visualizacao(&mut self, d: &mut Dialogo, ctx: &egui::Context) {
        let analise = d.analise.unwrap_or(!d.simplificada);
        let o = if d.simplificada { Opcoes { decimar: Some(SIMPLIFICADA), capturas: false, analise, ..Default::default() } } else { Opcoes { analise, ..Default::default() } };
        let (nome, gerar) = match self.gerador_do_que_esta_aberto() {
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
        let controle = Controle::novo();
        let o = Opcoes { controle: Some(controle.clone()), ..o };
        let (tx, rx) = std::sync::mpsc::channel();
        let acordar = ctx.clone();
        let nasceu = std::thread::Builder::new().name("gerar-visualizacao".into()).spawn(move || {
            fenkra_kernel::reduzir_prioridade_da_thread();
            let _ = tx.send(gerar(&o));
            acordar.request_repaint();
        });
        if let Err(e) = nasceu {
            return self.toast(format!("{}: {e}", tr("Falha ao exportar")), true);
        }
        d.trabalho = Some(Trabalho { rx, controle, caminho, saida: d.saida, nome });
    }

    /// Recolhe o pacote pronto da thread, grava o arquivo e avisa.
    fn concluir_visualizacao(&mut self, d: &mut Dialogo, ctx: &egui::Context) {
        let Some(t) = &d.trabalho else { return };
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
        let Ok(resultado) = t.rx.try_recv() else { return };
        let Some(t) = d.trabalho.take() else { return };
        let caminho = t.caminho;
        let saida = t.saida;
        let pacote = match resultado {
            Ok(p) => p,
            Err(e) if e == fenkra_pacote::CANCELADO => return self.toast(tr("Exportação cancelada: nada foi gravado.").to_string(), false),
            Err(e) => return self.toast(format!("{}: {e}", tr("Falha ao exportar")), true),
        };
        let bytes = match pacote.bytes() {
            Ok(b) => b,
            Err(e) => return self.toast(format!("{}: {e}", tr("Falha ao exportar")), true),
        };
        let r = match saida {
            Saida::Html => std::fs::write(&caminho, pagina_html(Some(&bytes))),
            Saida::Fkv => std::fs::write(&caminho, &bytes),
        };
        match r {
            Ok(()) => {
                let kb = std::fs::metadata(&caminho).map_or(bytes.len(), |m| m.len() as usize) / 1024;
                self.toast(format!("{}: {} ({} KB)", tr("Exportado"), caminho.display(), kb), false);
                let avisos = pacote.avisos();
                if !avisos.is_empty() {
                    self.toast(format!("{}: {}", tr("Faltou no pacote"), avisos.join("; ")), true);
                }
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
    fn analise_acompanha_a_versao_simplificada_e_so_aparece_em_montagem() {
        // B400: a análise é o que passa de um minuto numa montagem grande; a simplificada sai sem ela
        let mut d = Dialogo::novo(false);
        assert!(!textos(&mut d).contains("Incluir análise"));
        d.montagem = true;
        assert!(textos(&mut d).contains("Incluir análise de interferências e folgas"));
        assert!(d.analise.unwrap_or(!d.simplificada));
        d.simplificada = true;
        assert!(!d.analise.unwrap_or(!d.simplificada));
        d.analise = Some(true);
        assert!(d.analise.unwrap_or(!d.simplificada), "a pessoa pode ligar de volta");
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
