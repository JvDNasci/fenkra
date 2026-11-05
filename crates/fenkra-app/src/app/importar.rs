// SPDX-License-Identifier: LGPL-2.1-or-later
//! Importar STEP e BREP (B88, B229). O arquivo é lido no processo filho e o painel mostra miniatura,
//! caixa, contagens e aviso de unidade antes de a pessoa confirmar; depois fica o relatório.
//! Com projeto aberto, o original vai para o cofre e reimportar o mesmo nome compara o hash; sem
//! projeto, o documento só abre, já com o selo de origem.
//!
//! A decisão sobre o arquivo mudado vive em `fenkra_project::importacao`; aqui fica só a pergunta.

use super::*;
use crate::examples::{paint_preview, Preview};
use crate::ui_ext::dv;
use fenkra_conversor::processo::Filho;
use fenkra_conversor::{unidade_suspeita, ErroConv, Importado, OpcoesImportar, Previa, Relatorio, Reparo, Suspeita, UnidadeArquivo};
use fenkra_project::importacao::Reimportacao;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

/// Pausa depois de mudar uma opção antes de reler: arrastar a tolerância não dispara um processo por quadro.
const ESPERA_RELEITURA: f64 = 0.4;
const LARGURA: f32 = 300.;

/// Documento lido, com o relatório como vai para o cofre.
struct Documento {
    doc: Document,
    relatorio: String,
}

/// Arquivo que mudou desde a importação, esperando a pessoa escolher o que fazer.
pub struct Pendente {
    id: u64,
    caminho: PathBuf,
    importacao: Documento,
}

enum Leitura {
    /// ainda não começou: o fio precisa do contexto, que só o desenho tem
    Parada,
    Lendo { recebe: mpsc::Receiver<Result<Importado, ErroConv>>, cancelar: Arc<AtomicBool> },
    Pronta(Box<Importado>, Preview),
    Falhou(String),
}

/// Painel aberto durante o comando.
pub struct Dialogo {
    caminho: PathBuf,
    opcoes: OpcoesImportar,
    leitura: Leitura,
    giro: f32,
    /// quando a última opção mudou, para reler depois da pausa
    mudou_em: Option<f64>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Acao {
    Nada,
    Confirmar,
    Cancelar,
}

/// Miniatura pronta para desenhar. Peça sem cor no arquivo usa `cor_padrao`, a mesma do documento.
fn miniatura(p: &Previa, cor_padrao: [u8; 3]) -> Preview {
    let radius = p.pos.iter().map(|q| (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt()).fold(0., f32::max).max(1e-3);
    Preview {
        pos: p.pos.clone(),
        nrm: p.nrm.clone(),
        tris: p.tris.clone(),
        edges: p.arestas.clone(),
        color: cor_padrao,
        cores: p.cor.iter().map(|c| c.unwrap_or(cor_padrao)).collect(),
        radius,
    }
}

/// Aviso para quem vai confirmar, com o limite que a peça passou.
fn aviso_de_unidade(medidas: [f64; 3]) -> Option<&'static str> {
    match unidade_suspeita(medidas)? {
        Suspeita::Grande => Some("Peça com mais de 10 000 mm: confira a unidade do arquivo."),
        Suspeita::Pequena => Some("Peça com menos de 0,1 mm: confira a unidade do arquivo."),
    }
}

fn nome_reparo_dica(r: Reparo) -> &'static str {
    match r {
        Reparo::Nenhum => "Só analisa: a geometria entra como veio e os defeitos vão para o relatório.",
        Reparo::Padrao => "Corrige o que está dentro da tolerância e costura as faces soltas. Peça sã não é tocada.",
        Reparo::Agressivo => "Como o padrão, mas deixa a tolerância subir até 100 vezes. Pode mudar a forma.",
    }
}

impl Dialogo {
    fn novo(caminho: PathBuf, opcoes: OpcoesImportar) -> Dialogo {
        Dialogo { caminho, opcoes, leitura: Leitura::Parada, giro: 0.6, mudou_em: None }
    }

    fn nome(&self) -> String {
        self.caminho.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
    }

    fn parar(&mut self) {
        if let Leitura::Lendo { cancelar, .. } = &self.leitura {
            cancelar.store(true, Ordering::Relaxed);
        }
    }

    fn ler(&mut self, ctx: &egui::Context) {
        self.parar();
        let (envia, recebe) = mpsc::channel();
        let cancelar = Arc::new(AtomicBool::new(false));
        let (caminho, opcoes, sinal, ctx) = (self.caminho.clone(), self.opcoes, cancelar.clone(), ctx.clone());
        std::thread::spawn(move || {
            let r = Filho::ao_lado_do_programa().and_then(|f| f.importar(&caminho, opcoes, &sinal));
            let _ = envia.send(r);
            ctx.request_repaint();
        });
        self.leitura = Leitura::Lendo { recebe, cancelar };
    }

    fn receber(&mut self, r: Result<Importado, ErroConv>) {
        self.leitura = match r {
            Ok(i) => {
                let cor = i.doc.collectors.first().map(|c| c.color).unwrap_or_default();
                let mini = miniatura(&i.previa, cor);
                Leitura::Pronta(Box::new(i), mini)
            }
            Err(e) => Leitura::Falhou(e.to_string()),
        };
    }

    /// Cuida da leitura em andamento e da releitura depois de mudar uma opção.
    fn avancar(&mut self, ctx: &egui::Context) {
        let agora = ctx.input(|i| i.time);
        if self.mudou_em.is_some_and(|t| agora - t >= ESPERA_RELEITURA) {
            self.mudou_em = None;
            self.leitura = Leitura::Parada;
        }
        if self.mudou_em.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if matches!(self.leitura, Leitura::Parada) && self.mudou_em.is_none() {
            self.ler(ctx);
        }
        if let Leitura::Lendo { recebe, .. } = &self.leitura {
            match recebe.try_recv() {
                Ok(r) => self.receber(r),
                Err(mpsc::TryRecvError::Disconnected) => self.leitura = Leitura::Falhou(tr("A leitura terminou sem resposta.").into()),
                Err(mpsc::TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
            }
        }
    }

    fn pode_confirmar(&self) -> bool {
        self.mudou_em.is_none() && matches!(self.leitura, Leitura::Pronta(..))
    }

    fn opcoes_ui(&mut self, ui: &mut egui::Ui) {
        let o = &mut self.opcoes;
        let antes = *o;
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut o.cores, tr("Cores")).on_hover_text(tr("Traz a cor de cada peça. A cor por face fica de fora: o documento guarda uma por corpo."));
            ui.checkbox(&mut o.nomes, tr("Nomes")).on_hover_text(tr("Usa os nomes do arquivo nos corpos. Desligado, eles ficam numerados."));
            ui.checkbox(&mut o.camadas, tr("Camadas")).on_hover_text(tr("Lê as camadas para o relatório. O documento ainda não tem camadas."));
        });
        ui.label(tr("Unidade do arquivo"));
        egui::ComboBox::from_id_salt("importar_unidade").selected_text(tr(o.unidade.nome())).width(LARGURA - 40.).show_ui(ui, |ui| {
            for u in UnidadeArquivo::TODAS {
                ui.selectable_value(&mut o.unidade, u, tr(u.nome()));
            }
        }).response.on_hover_text(tr("Como ler os números do arquivo. Troque quando o tamanho da peça não fizer sentido."));
        ui.label(tr("Reparo"));
        egui::ComboBox::from_id_salt("importar_reparo").selected_text(tr(o.reparo.nome())).width(LARGURA - 40.).show_ui(ui, |ui| {
            for r in Reparo::TODOS {
                ui.selectable_value(&mut o.reparo, r, tr(r.nome())).on_hover_text(tr(nome_reparo_dica(r)));
            }
        }).response.on_hover_text(tr(nome_reparo_dica(o.reparo)));
        ui.add(
            dv(&mut o.tolerancia)
                .range(0.0001..=1.0)
                .speed(0.001)
                .max_decimals(4)
                .suffix(" mm")
                .rotulo(tr("Tolerância"))
                .dica(tr("Maior folga entre faces que o reparo e a costura aceitam fechar.")),
        );
        if *o != antes {
            let agora = ui.input(|i| i.time);
            self.parar();
            self.mudou_em = Some(agora);
        }
    }

    fn resultado_ui(&mut self, ui: &mut egui::Ui) {
        let (giro, leitura) = (&mut self.giro, &self.leitura);
        match leitura {
            Leitura::Parada | Leitura::Lendo { .. } => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr("Lendo o arquivo…"));
                });
            }
            Leitura::Falhou(e) => {
                ui.colored_label(ui.visuals().error_fg_color, e.as_str());
            }
            Leitura::Pronta(i, mini) => {
                let (rect, r) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 190.), egui::Sense::drag());
                ui.painter().rect_filled(rect, 4., ui.visuals().extreme_bg_color);
                paint_preview(&ui.painter().with_clip_rect(rect), rect, mini, *giro);
                *giro += r.drag_delta().x * 0.012;
                r.on_hover_text(tr("Arraste para girar a miniatura."));
                if i.previa.incompleta {
                    ui.label(RichText::new(tr("Miniatura sem todas as peças: o arquivo é grande demais para ela.")).weak().small());
                }
                let rel = &i.relatorio;
                if let Some(m) = rel.medidas() {
                    ui.label(format!("{}: {:.2} x {:.2} x {:.2} mm", tr("Caixa envolvente"), m[0], m[1], m[2]));
                    if let Some(aviso) = aviso_de_unidade(m) {
                        ui.colored_label(ui.visuals().warn_fg_color, tr(aviso));
                    }
                }
                ui.label(format!("{}: {}   {}: {}", tr("Corpos"), rel.corpos, tr("Faces"), rel.faces));
                ui.label(RichText::new(format!("{} {}", rel.formato, rel.esquema)).weak().small());
                if !rel.nao_fecharam.is_empty() {
                    ui.colored_label(ui.visuals().warn_fg_color, format!("{}: {}", tr("Sólidos que não fecharam"), rel.nao_fecharam.len()));
                }
            }
        }
    }

    /// Conteúdo do painel, sem depender da aplicação.
    fn ui(&mut self, ui: &mut egui::Ui) -> Acao {
        ui.label(RichText::new(self.nome()).strong());
        ui.add_space(4.);
        self.resultado_ui(ui);
        ui.separator();
        self.opcoes_ui(ui);
        ui.add_space(6.);
        let mut acao = Acao::Nada;
        ui.horizontal(|ui| {
            let confirmar = ui.add_enabled(self.pode_confirmar(), egui::Button::new(tr("Importar")));
            if confirmar.on_hover_text(tr("Cria o documento com o que o painel mostra.")).clicked() {
                acao = Acao::Confirmar;
            }
            if ui.button(tr("Cancelar")).clicked() {
                acao = Acao::Cancelar;
            }
        });
        acao
    }
}

/// Janela do relatório: o texto pode ser selecionado, e o botão copia tudo.
fn relatorio_ui(ui: &mut egui::Ui, rel: &Relatorio) -> bool {
    let texto = rel.texto(&|s| tr(s));
    egui::ScrollArea::vertical().max_height(320.).show(ui, |ui| {
        ui.add(egui::TextEdit::multiline(&mut texto.as_str()).desired_width(f32::INFINITY).font(egui::TextStyle::Small));
    });
    let mut fechar = false;
    ui.horizontal(|ui| {
        if ui.button(tr("Copiar")).on_hover_text(tr("Copia o relatório inteiro.")).clicked() {
            ui.ctx().copy_text(texto.clone());
        }
        fechar = ui.button(tr("Fechar")).clicked();
    });
    fechar
}

impl App {
    pub(super) fn importar_arquivo(&mut self) {
        let dialogo = rfd::FileDialog::new().add_filter("STEP, BREP", &["step", "stp", "brep"]).add_filter(tr("Todos os arquivos"), &["*"]);
        let Some(caminho) = dialogo.pick_file() else { return };
        self.abrir_painel_importar(caminho);
    }

    pub(super) fn abrir_painel_importar(&mut self, caminho: PathBuf) {
        self.importar_dialogo = Some(Dialogo::novo(caminho, self.settings.opcoes_importar));
        self.started = true;
    }

    pub(super) fn painel_importar(&mut self, ctx: &egui::Context, area: Rect) {
        use crate::paineis::{self, Encaixe, Painel};
        if let Some(mut d) = self.importar_dialogo.take() {
            d.avancar(ctx);
            let padrao = paineis::abaixo_do_cubo(area, LARGURA);
            let cfg = Painel { nome: "importar".into(), titulo: tr("Importar arquivo").to_string(), padrao, largura: LARGURA, redimensiona: false };
            let posicao = self.settings.painel("importar", Encaixe::Direita);
            let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| d.ui(ui));
            if let Some(p) = saida.movido {
                self.settings.guardar_painel("importar", p);
            }
            // desenvolvimento: confirma sozinho, para capturar o relatório sem mouse
            let sozinho = d.pode_confirmar() && std::env::var("FENKRA_DEMO_IMPORTAR_OK").is_ok();
            match saida.conteudo.unwrap_or(Acao::Nada) {
                Acao::Nada if sozinho => self.confirmar_importacao(d),
                Acao::Nada => self.importar_dialogo = Some(d),
                Acao::Cancelar => d.parar(),
                Acao::Confirmar => self.confirmar_importacao(d),
            }
        }
        if let Some(rel) = self.importar_relatorio.take() {
            let padrao = paineis::ao_lado_da_arvore(area);
            let cfg = Painel { nome: "relatorio_importacao".into(), titulo: tr("Relatório da importação").to_string(), padrao, largura: 420., redimensiona: true };
            let posicao = self.settings.painel("relatorio_importacao", Encaixe::Livre);
            let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| relatorio_ui(ui, &rel));
            if let Some(p) = saida.movido {
                self.settings.guardar_painel("relatorio_importacao", p);
            }
            if saida.conteudo != Some(true) {
                self.importar_relatorio = Some(rel);
            }
        }
    }

    fn confirmar_importacao(&mut self, d: Dialogo) {
        let Dialogo { caminho, opcoes, leitura, .. } = d;
        let Leitura::Pronta(i, _) = leitura else { return };
        self.settings.opcoes_importar = opcoes;
        let Importado { doc, relatorio, .. } = *i;
        let documento = Documento { doc, relatorio: relatorio.texto_pt() };
        self.importar_relatorio = Some(relatorio);
        self.receber_importacao(caminho, documento);
    }

    fn receber_importacao(&mut self, caminho: PathBuf, importacao: Documento) {
        let Some(p) = &mut self.project else {
            self.load_doc(importacao.doc, None, None);
            self.started = true;
            return;
        };
        let Some(selo) = importacao.doc.origem.clone() else { return };
        match p.reimportacao(&selo.nome, &selo.hash) {
            Reimportacao::Nova => {
                let r = p.adicionar_importado(&caminho, importacao.doc, &importacao.relatorio);
                self.abrir_importado(r);
            }
            Reimportacao::Igual { id } => {
                self.toast(tr("Arquivo igual ao já importado.").to_string(), false);
                self.open_entry(id, None);
            }
            Reimportacao::Mudou { id, .. } => self.importacao_pendente = Some(Pendente { id, caminho, importacao }),
        }
        self.started = true;
    }

    fn abrir_importado(&mut self, r: fenkra_project::PResult<u64>) {
        match r {
            Ok(id) => {
                self.left = LeftTab::Projeto;
                self.open_entry(id, None);
            }
            Err(e) => self.toast(e.to_string(), true),
        }
    }

    pub(super) fn importacao_window(&mut self, ctx: &egui::Context) {
        let Some(pendente) = &self.importacao_pendente else { return };
        let nome = pendente.importacao.doc.origem.as_ref().map(|o| o.nome.clone()).unwrap_or_default();
        let mut escolha = None;
        egui::Window::new(tr("Arquivo importado mudou")).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.label(RichText::new(nome).strong());
            ui.label(tr("O conteúdo não é o mesmo da última importação."));
            ui.add_space(6.);
            ui.horizontal(|ui| {
                if ui.button(tr("Atualizar")).on_hover_text(tr("Troca a geometria e mantém os comandos feitos depois dela.")).clicked() {
                    escolha = Some(true);
                }
                if ui.button(tr("Criar derivado")).on_hover_text(tr("Cria outro documento e deixa o atual como está.")).clicked() {
                    escolha = Some(false);
                }
                if ui.button(tr("Cancelar")).clicked() {
                    self.importacao_pendente = None;
                }
            });
        });
        let Some(atualizar) = escolha else { return };
        let (Some(Pendente { id, caminho, importacao }), Some(p)) = (self.importacao_pendente.take(), self.project.as_mut()) else { return };
        if atualizar {
            let r = p.atualizar_importado(id, &caminho, &importacao.doc, &importacao.relatorio).map(|()| id);
            self.abrir_importado(r);
        } else {
            let r = p.adicionar_derivado(id, &caminho, importacao.doc, &importacao.relatorio);
            self.abrir_importado(r);
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_conversor::{Formato, ModeloNeutro, OpcoesExportar, Registro, NOTAS};
    use fenkra_kernel::Shape;

    fn importado(l: [f64; 3], teste: &str) -> Importado {
        let dir = std::env::temp_dir().join(format!("Fenkra painel açaí {} {teste}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let arquivo = dir.join("peça.step");
        let caixa = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], l).unwrap();
        Registro::default().gravar(&ModeloNeutro::de_forma(Formato::Step, "Peça", caixa), &arquivo, &OpcoesExportar::default()).unwrap();
        Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap()
    }

    /// Desenha o painel fora de uma janela de verdade e devolve os textos pintados.
    fn textos_do_painel(d: &mut Dialogo) -> (Vec<String>, Acao) {
        let ctx = egui::Context::default();
        let mut acao = Acao::Nada;
        let mut textos = vec![];
        for _ in 0..2 {
            let entrada = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400., 800.))), ..Default::default() };
            let mut saida = ctx.run_ui(entrada, |ui| {
                acao = d.ui(ui);
            });
            saida.textures_delta.clear();
            textos = saida.shapes.iter().filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) => Some(t.galley.text().to_string()),
                _ => None,
            }).collect();
        }
        (textos, acao)
    }

    #[test]
    fn painel_mostra_caixa_contagem_e_nao_avisa_peca_normal() {
        let mut d = Dialogo::novo("peça.step".into(), OpcoesImportar::default());
        assert!(!d.pode_confirmar());
        d.receber(Ok(importado([120., 60., 40.], "normal")));
        assert!(d.pode_confirmar());
        let (textos, acao) = textos_do_painel(&mut d);
        assert_eq!(acao, Acao::Nada);
        let tudo = textos.join("\n");
        assert!(tudo.contains("120.00 x 60.00 x 40.00 mm"), "{tudo}");
        assert!(tudo.contains("Corpos: 1") && tudo.contains("Faces: 6"), "{tudo}");
        assert!(!tudo.contains("confira a unidade"), "{tudo}");
    }

    #[test]
    fn painel_avisa_unidade_suspeita_nos_dois_extremos() {
        for (l, trecho, nome) in [([20_000., 10., 10.], "mais de 10 000 mm", "grande"), ([0.05, 0.05, 0.05], "menos de 0,1 mm", "pequena")] {
            let mut d = Dialogo::novo("peça.step".into(), OpcoesImportar::default());
            d.receber(Ok(importado(l, nome)));
            let tudo = textos_do_painel(&mut d).0.join("\n");
            assert!(tudo.contains(trecho), "{tudo}");
        }
    }

    #[test]
    fn falha_de_leitura_aparece_e_impede_confirmar() {
        let mut d = Dialogo::novo("peça.step".into(), OpcoesImportar::default());
        d.receber(Err(ErroConv::Queda(3)));
        assert!(!d.pode_confirmar());
        let tudo = textos_do_painel(&mut d).0.join("\n");
        assert!(tudo.contains("segue aberto"), "{tudo}");
    }

    #[test]
    fn opcao_mudada_bloqueia_confirmar_ate_reler() {
        let mut d = Dialogo::novo("peça.step".into(), OpcoesImportar::default());
        d.receber(Ok(importado([10., 10., 10.], "releitura")));
        d.mudou_em = Some(0.);
        assert!(!d.pode_confirmar(), "o painel mostraria o resultado das opções antigas");
    }

    #[test]
    fn miniatura_usa_a_cor_do_arquivo_e_a_do_documento_na_falta() {
        let p = Previa { pos: vec![[1., 0., 0.], [0., 2., 0.]], cor: vec![Some([1, 2, 3]), None], ..Default::default() };
        let m = miniatura(&p, [9, 9, 9]);
        assert_eq!(m.cores, vec![[1, 2, 3], [9, 9, 9]]);
        assert_eq!(m.radius, 2.);
    }

    #[test]
    fn frases_do_relatorio_tem_traducao() {
        let rotulos = [
            "Arquivo", "Tamanho", "Formato", "Esquema", "Unidades lidas", "Convertido por", "Produtos", "Instâncias", "Corpos", "Faces", "Sólidos", "Volume", "Área", "Caixa envolvente",
            "Reparo", "Faces reparadas", "Faces costuradas", "Faces ainda inválidas", "Sólidos que não fecharam", "O que o formato não carrega", "Avisos",
        ];
        for s in NOTAS.iter().chain(rotulos.iter()) {
            assert!(crate::i18n::tem_traducao(s), "sem tradução: {s}");
        }
        for r in Reparo::TODOS {
            assert!(crate::i18n::tem_traducao(r.nome()) && crate::i18n::tem_traducao(nome_reparo_dica(r)));
        }
        for u in UnidadeArquivo::TODAS {
            assert!(crate::i18n::tem_traducao(u.nome()), "{}", u.nome());
        }
    }
}
