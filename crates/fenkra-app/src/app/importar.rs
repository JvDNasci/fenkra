// SPDX-License-Identifier: LGPL-2.1-or-later
//! Importar STEP, IGES e BREP (B88, B229, B104). O arquivo é lido no processo filho e o painel mostra
//! miniatura, caixa, contagens, defeitos e aviso de unidade antes de a pessoa confirmar; depois fica o
//! relatório, que realça cada defeito na viewport e grava em HTML ou JSON.
//! Com projeto aberto, o original vai para o cofre e reimportar o mesmo nome compara o hash; sem
//! projeto, o documento só abre, já com o selo de origem.
//!
//! A decisão sobre o arquivo mudado vive em `fenkra_project::importacao`; aqui fica só a pergunta.

use super::*;
use crate::examples::{paint_preview, preview_na_tela, Preview};
use crate::ui_ext::dv;
use fenkra_conversor::processo::Filho;
use fenkra_conversor::{unidade_suspeita, Destino, ErroConv, Importado, OpcoesImportar, Previa, Registro, Relatorio, Reparo, Suspeita, UnidadeArquivo};
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
    let raio = |q: &[f32; 3]| (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt();
    // o DXF só tem arestas, e o tamanho da miniatura vem delas
    let radius = p.pos.iter().chain(p.arestas.iter().flatten()).map(raio).fold(0., f32::max).max(1e-3);
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

fn dica_destino(d: Destino) -> &'static str {
    match d {
        Destino::Montagem => "Cada peça vira um documento, e cada ocorrência um componente. A peça repetida existe uma vez só.",
        Destino::UmaPeca => "Tudo vira um documento só, com um corpo por ocorrência.",
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

    fn e_arvore(&self) -> bool {
        self.caminho.extension().is_some_and(|e| e.eq_ignore_ascii_case("fkx"))
    }

    fn e_dxf(&self) -> bool {
        self.caminho.extension().is_some_and(|e| e.eq_ignore_ascii_case("dxf"))
    }

    /// Vale perguntar o destino: o arquivo tem mais de uma ocorrência, lido em qualquer um dos dois.
    fn tem_varias_pecas(&self) -> bool {
        matches!(&self.leitura, Leitura::Pronta(i, _) if i.conjunto.is_some() || i.relatorio.instancias > 1 || i.relatorio.produtos > 1)
    }

    fn pode_confirmar(&self) -> bool {
        self.mudou_em.is_none() && matches!(self.leitura, Leitura::Pronta(..))
    }

    fn opcoes_ui(&mut self, ui: &mut egui::Ui) {
        let (dxf, varias) = (self.e_dxf(), self.tem_varias_pecas());
        let o = &mut self.opcoes;
        let antes = *o;
        ui.horizontal_wrapped(|ui| {
            if dxf {
                ui.checkbox(&mut o.camadas, tr("Camadas")).on_hover_text(tr("Um sketch por camada, com o nome dela. Desligado, tudo entra em um sketch só."));
                return;
            }
            ui.checkbox(&mut o.cores, tr("Cores")).on_hover_text(tr("Traz a cor de cada peça. A cor por face fica de fora: o documento guarda uma por corpo."));
            ui.checkbox(&mut o.nomes, tr("Nomes")).on_hover_text(tr("Usa os nomes do arquivo nos corpos. Desligado, eles ficam numerados."));
            ui.checkbox(&mut o.camadas, tr("Camadas")).on_hover_text(tr("Lê as camadas para o relatório. O documento ainda não tem camadas."));
        });
        if varias {
            ui.label(tr("Destino"));
            egui::ComboBox::from_id_salt("importar_destino").selected_text(tr(o.destino.nome())).width(LARGURA - 40.).show_ui(ui, |ui| {
                for d in Destino::TODOS {
                    ui.selectable_value(&mut o.destino, d, tr(d.nome())).on_hover_text(tr(dica_destino(d)));
                }
            }).response.on_hover_text(tr(dica_destino(o.destino)));
        }
        ui.label(tr("Unidade do arquivo"));
        egui::ComboBox::from_id_salt("importar_unidade").selected_text(tr(o.unidade.nome())).width(LARGURA - 40.).show_ui(ui, |ui| {
            for u in UnidadeArquivo::TODAS {
                ui.selectable_value(&mut o.unidade, u, tr(u.nome()));
            }
        }).response.on_hover_text(tr("Como ler os números do arquivo. Troque quando o tamanho da peça não fizer sentido."));
        if !dxf {
            ui.label(tr("Reparo"));
            egui::ComboBox::from_id_salt("importar_reparo").selected_text(tr(o.reparo.nome())).width(LARGURA - 40.).show_ui(ui, |ui| {
                for r in Reparo::TODOS {
                    ui.selectable_value(&mut o.reparo, r, tr(r.nome())).on_hover_text(tr(nome_reparo_dica(r)));
                }
            }).response.on_hover_text(tr(nome_reparo_dica(o.reparo)));
        }
        ui.add(
            dv(&mut o.tolerancia)
                .range(0.0001..=1.0)
                .speed(0.001)
                .max_decimals(4)
                .suffix(" mm")
                .rotulo(tr("Tolerância"))
                .dica(tr(if dxf { "Folga que ainda junta duas pontas no mesmo ponto." } else { "Maior folga entre faces que o reparo e a costura aceitam fechar." })),
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
                let pintor = ui.painter().with_clip_rect(rect);
                paint_preview(&pintor, rect, mini, *giro);
                // o DXF só tem curvas, na cor do texto: a tinta das arestas some no fundo escuro
                let traco = egui::Stroke::new(1., ui.visuals().text_color());
                if mini.tris.is_empty() {
                    for e in &mini.edges {
                        pintor.line_segment([preview_na_tela(rect, mini, *giro, e[0]), preview_na_tela(rect, mini, *giro, e[1])], traco);
                    }
                }
                pintar_defeitos(&pintor, rect, mini, *giro, &i.relatorio);
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
                if !rel.camadas.is_empty() {
                    let curvas: usize = rel.camadas.iter().map(|c| c.1).sum();
                    ui.label(tr("Curvas: {0}   Camadas: {1}").replace("{0}", &curvas.to_string()).replace("{1}", &rel.camadas.len().to_string()));
                } else {
                    ui.label(format!("{}: {}   {}: {}", tr("Corpos"), rel.corpos, tr("Faces"), rel.faces));
                }
                if let Some(c) = &i.conjunto {
                    ui.label(tr("Peças: {0}   Ocorrências: {1}").replace("{0}", &c.pecas.len().to_string()).replace("{1}", &rel.instancias.to_string()));
                }
                let origem = if rel.esquema.starts_with(&rel.formato) { rel.esquema.clone() } else { format!("{} {}", rel.formato, rel.esquema) };
                ui.label(RichText::new(origem).weak().small());
                if !rel.nao_fecharam.is_empty() {
                    ui.colored_label(ui.visuals().warn_fg_color, format!("{}: {}", tr("Sólidos que não fecharam"), rel.nao_fecharam.len()));
                }
                if !rel.defeitos.is_empty() {
                    ui.colored_label(ui.visuals().warn_fg_color, format!("{}: {}", tr("Defeitos"), rel.defeitos.len()));
                }
            }
        }
    }

    /// Conteúdo do painel, sem depender da aplicação.
    fn ui(&mut self, ui: &mut egui::Ui) -> Acao {
        ui.label(RichText::new(self.nome()).strong());
        ui.add_space(4.);
        self.resultado_ui(ui);
        // a árvore não tem unidade a escolher nem forma a reparar: a geometria nasce do recálculo
        if !self.e_arvore() {
            ui.separator();
            self.opcoes_ui(ui);
        }
        if fenkra_conversor::Formato::por_extensao(&self.caminho).is_some_and(|f| f.e_malha()) && ui.checkbox(&mut self.opcoes.malha_em_solido, tr("Converter em sólido")).on_hover_text(tr("Liga: cada triângulo vira face plana, até 200 mil triângulos. Desligado: entra como corpo de malha, sem teto.")).changed() {
            self.mudou_em = Some(ui.input(|i| i.time));
        }
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

/// Traço dos defeitos na miniatura, que é centrada na caixa da peça.
fn pintar_defeitos(pintor: &egui::Painter, rect: Rect, mini: &Preview, giro: f32, rel: &Relatorio) {
    let Some((a, b)) = rel.caixa else { return };
    let centro = [0, 1, 2].map(|i| ((a[i] + b[i]) / 2.) as f32);
    let traco = egui::Stroke::new(1.6, crate::theme::st_err());
    for s in rel.defeitos.iter().flat_map(|d| &d.segmentos) {
        let ponto = |p: [f32; 3]| preview_na_tela(rect, mini, giro, [p[0] - centro[0], p[1] - centro[1], p[2] - centro[2]]);
        pintor.line_segment([ponto(s[0]), ponto(s[1])], traco);
    }
}

/// Defeitos na viewport enquanto o relatório está aberto: todos em aviso e o escolhido em erro, por cima
/// das faces, porque um defeito costuma estar atrás de alguma coisa.
pub(super) fn defeitos_na_cena(rel: Option<&Relatorio>, escolhido: Option<usize>, s: &mut crate::render::Scene) {
    let Some(rel) = rel else { return };
    let rgba = |c: egui::Color32| [c.r() as f32 / 255., c.g() as f32 / 255., c.b() as f32 / 255., 1.];
    let (aviso, erro) = (rgba(crate::theme::st_warn()), rgba(crate::theme::st_err()));
    for (i, d) in rel.defeitos.iter().enumerate() {
        let col = if escolhido == Some(i) { erro } else { aviso };
        s.overlay.extend(d.segmentos.iter().flatten().map(|p| crate::render::LineVertex { pos: *p, col }));
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SaidaRelatorio {
    fechar: bool,
    /// linha da lista de defeitos em que a pessoa clicou
    escolhido: Option<usize>,
    /// extensão que a pessoa pediu para gravar
    gravar: Option<&'static str>,
}

/// Painel do relatório: antes e depois, defeitos clicáveis e o texto inteiro para selecionar.
fn relatorio_ui(ui: &mut egui::Ui, rel: &Relatorio, realce: Option<usize>) -> SaidaRelatorio {
    let mut saida = SaidaRelatorio::default();
    ui.label(RichText::new(tr("Antes e depois do reparo")).strong());
    egui::Grid::new("relatorio_antes_depois").striped(true).show(ui, |ui| {
        ui.label("");
        ui.label(RichText::new(tr("Antes")).weak());
        ui.label(RichText::new(tr("Depois")).weak());
        ui.end_row();
        for (rotulo, antes, depois) in rel.linhas_antes_depois() {
            ui.label(tr(rotulo));
            ui.label(antes);
            ui.label(depois);
            ui.end_row();
        }
    });
    ui.add_space(6.);
    if rel.defeitos.is_empty() {
        ui.label(tr("Nenhum defeito depois do reparo."));
    } else {
        ui.label(RichText::new(format!("{} ({})", tr("Defeitos"), rel.defeitos.len())).strong());
        egui::ScrollArea::vertical().id_salt("relatorio_defeitos").max_height(130.).show(ui, |ui| {
            for (i, d) in rel.defeitos.iter().enumerate() {
                let linha = ui.selectable_label(realce == Some(i), format!("{} · {}", tr(d.tipo.nome()), d.peca));
                if linha.on_hover_text(tr("Clique para realçar o defeito na peça e levar a câmera até ele.")).clicked() {
                    saida.escolhido = Some(i);
                }
            }
        });
    }
    ui.add_space(6.);
    let texto = rel.texto(&|s| tr(s));
    egui::CollapsingHeader::new(tr("Texto completo")).show(ui, |ui| {
        egui::ScrollArea::vertical().id_salt("relatorio_texto").max_height(240.).show(ui, |ui| {
            ui.add(egui::TextEdit::multiline(&mut texto.as_str()).desired_width(f32::INFINITY).font(egui::TextStyle::Small));
        });
    });
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Copiar")).on_hover_text(tr("Copia o relatório inteiro.")).clicked() {
            ui.ctx().copy_text(texto.clone());
        }
        if ui.button(tr("Salvar HTML…")).on_hover_text(tr("Grava o relatório numa página para abrir em qualquer navegador.")).clicked() {
            saida.gravar = Some("html");
        }
        if ui.button(tr("Salvar JSON…")).on_hover_text(tr("Grava o relatório em dados, para outro programa ler.")).clicked() {
            saida.gravar = Some("json");
        }
        saida.fechar = ui.button(tr("Fechar")).clicked();
    });
    saida
}

impl App {
    pub(super) fn importar_arquivo(&mut self) {
        let malhas: Vec<&str> = fenkra_conversor::Formato::MALHA.iter().filter(|f| Registro::default().capacidades(**f).ler).flat_map(|f| f.extensoes().iter().copied()).collect();
        let dialogo = rfd::FileDialog::new().add_filter(tr("Todos os formatos"), &Registro::default().extensoes_de_leitura()).add_filter("STEP, IGES, BREP, DXF", &["step", "stp", "igs", "iges", "brep", "dxf"]).add_filter(tr("Malha"), &malhas).add_filter(tr("Árvore paramétrica"), &["fkx"]).add_filter(tr("Todos os arquivos"), &["*"]);
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
            let realce = self.importar_defeito;
            let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| relatorio_ui(ui, &rel, realce));
            if let Some(p) = saida.movido {
                self.settings.guardar_painel("relatorio_importacao", p);
            }
            let acao = saida.conteudo.unwrap_or_default();
            if let Some(i) = acao.escolhido {
                self.realcar_defeito(&rel, i);
            }
            if let Some(extensao) = acao.gravar {
                self.gravar_relatorio(&rel, extensao);
            }
            if acao.fechar {
                self.importar_defeito = None;
                self.scene_dirty = true;
            } else {
                self.importar_relatorio = Some(rel);
            }
        }
    }

    /// Realça o defeito e leva a câmera até ele, sem mexer na distância: quem clica quer ver onde está.
    fn realcar_defeito(&mut self, rel: &Relatorio, i: usize) {
        self.importar_defeito = Some(i);
        self.scene_dirty = true;
        if let (Some(d), Some(v)) = (rel.defeitos.get(i), self.viewport.as_mut()) {
            v.camera.target = glam::Vec3::from(d.centro());
        }
    }

    fn gravar_relatorio(&mut self, rel: &Relatorio, extensao: &str) {
        let nome = std::path::Path::new(&rel.arquivo).file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let Some(caminho) = rfd::FileDialog::new().add_filter(extensao.to_uppercase(), &[extensao]).set_file_name(format!("{nome}-relatorio.{extensao}")).save_file() else { return };
        let texto = if extensao == "html" { rel.html(&|s| tr(s)) } else { rel.json() };
        match std::fs::write(&caminho, texto) {
            Ok(()) => self.toast(tr("Relatório gravado: {0}").replace("{0}", &caminho.display().to_string()), false),
            Err(e) => self.toast(e.to_string(), true),
        }
    }

    /// Reabre o relatório guardado no selo do documento.
    pub(super) fn abrir_relatorio_embutido(&mut self) {
        let texto = self.open.doc.origem.as_ref().and_then(|o| o.relatorio.as_deref());
        self.importar_relatorio = texto.and_then(Relatorio::de_json);
        self.importar_defeito = None;
        self.scene_dirty = true;
    }

    fn confirmar_importacao(&mut self, d: Dialogo) {
        let Dialogo { caminho, opcoes, leitura, .. } = d;
        let Leitura::Pronta(i, _) = leitura else { return };
        self.settings.opcoes_importar = opcoes;
        let Importado { doc, relatorio, conjunto, .. } = *i;
        if let Some(c) = conjunto {
            let texto = relatorio.texto_pt();
            self.importar_relatorio = Some(relatorio);
            self.importar_defeito = None;
            self.receber_conjunto(&caminho, c, &texto);
            return;
        }
        let documento = Documento { doc, relatorio: relatorio.texto_pt() };
        self.importar_relatorio = Some(relatorio);
        self.importar_defeito = std::env::var("FENKRA_DEMO_DEFEITO").ok().and_then(|n| n.parse().ok());
        self.receber_importacao(caminho, documento);
    }

    fn receber_importacao(&mut self, caminho: PathBuf, importacao: Documento) {
        let Some(p) = &mut self.project else {
            self.load_doc(importacao.doc, None, None);
            // fora de um projeto a importação só existe na memória
            self.open.modified = true;
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
    use fenkra_conversor::{Defeito, Formato, ModeloNeutro, OpcoesExportar, TipoDefeito, NOTAS};
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
    fn dxf_nao_pergunta_reparo_e_miniatura_so_de_arestas_tem_tamanho() {
        let mut d = Dialogo::novo("perfil.dxf".into(), OpcoesImportar::default());
        let t = textos_do_painel(&mut d).0.join("\n");
        assert!(!t.contains("Reparo") && t.contains("Camadas") && t.contains("Unidade do arquivo"), "{t}");
        let p = Previa { arestas: vec![[[-3., -4., 0.], [3., 4., 0.]]], ..Default::default() };
        assert_eq!(miniatura(&p, [9, 9, 9]).radius, 5.);
    }

    #[test]
    fn destino_so_aparece_com_varias_pecas() {
        let mut d = Dialogo::novo("peça.step".into(), OpcoesImportar::default());
        d.receber(Ok(importado([10., 10., 10.], "destino")));
        assert!(!textos_do_painel(&mut d).0.join("\n").contains("Destino"));
        let Leitura::Pronta(i, _) = &mut d.leitura else { panic!() };
        i.relatorio.instancias = 3;
        assert!(textos_do_painel(&mut d).0.join("\n").contains("Destino"));
    }

    fn com_defeitos() -> Relatorio {
        let segmentos = vec![[[0., 0., 0.], [10., 0., 0.]]];
        let d = |tipo, peca: &str| Defeito { tipo, peca: peca.into(), indice: 0, segmentos: segmentos.clone() };
        Relatorio { defeitos: vec![d(TipoDefeito::ArestaLivre, "Tampa"), d(TipoDefeito::FaceInvalida, "Base")], ..Default::default() }
    }

    #[test]
    fn relatorio_mostra_antes_depois_e_a_lista_de_defeitos() {
        let rel = com_defeitos();
        let ctx = egui::Context::default();
        let entrada = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500., 800.))), ..Default::default() };
        let mut saida = ctx.run_ui(entrada, |ui| {
            let s = relatorio_ui(ui, &rel, Some(1));
            assert_eq!(s, SaidaRelatorio::default());
        });
        saida.textures_delta.clear();
        let tudo: Vec<String> = saida.shapes.iter().filter_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) => Some(t.galley.text().to_string()),
            _ => None,
        }).collect();
        let tudo = tudo.join("\n");
        assert!(tudo.contains("Antes e depois do reparo") && tudo.contains("Arestas livres"), "{tudo}");
        assert!(tudo.contains("Defeitos (2)") && tudo.contains("Aresta livre · Tampa") && tudo.contains("Face inválida · Base"), "{tudo}");
    }

    #[test]
    fn defeitos_vao_para_a_cena_e_o_escolhido_muda_de_cor() {
        let rel = com_defeitos();
        let mut cena = crate::render::Scene::default();
        defeitos_na_cena(Some(&rel), Some(1), &mut cena);
        assert_eq!(cena.overlay.len(), 4, "dois pontos por segmento");
        assert_ne!(cena.overlay[0].col, cena.overlay[2].col, "o escolhido se destaca dos outros");
        let mut vazia = crate::render::Scene::default();
        defeitos_na_cena(None, None, &mut vazia);
        assert!(vazia.overlay.is_empty());
    }

    #[test]
    fn frases_do_relatorio_tem_traducao() {
        let rotulos = [
            "Arquivo", "Tamanho", "Formato", "Esquema", "Unidades lidas", "Convertido por", "Produtos", "Instâncias", "Corpos", "Faces", "Sólidos", "Volume", "Área", "Caixa envolvente",
            "Reparo", "Faces reparadas", "Faces costuradas", "Faces ainda inválidas", "Sólidos que não fecharam", "O que o formato não carrega", "Avisos", "Antes e depois do reparo", "Antes", "Depois", "Peça",
            "Defeitos", "Tipo", "Posição", "Relatório da importação", "Nenhum defeito depois do reparo.", "Texto completo", "Salvar HTML…", "Salvar JSON…", "Relatório gravado: {0}",
            "Importar STEP, IGES, BREP, DXF ou árvore…", "Importar arquivo STEP, IGES, BREP ou DXF", "Relatório da importação…", "Exportar IGES…", "Destino", "Peças: {0}   Ocorrências: {1}", "Curvas: {0}   Camadas: {1}",
            "Camadas lidas", "Conjunto igual ao já importado.", "O arquivo mudou: entra como um conjunto novo, com peças novas.", "Folga que ainda junta duas pontas no mesmo ponto.",
            "Um sketch por camada, com o nome dela. Desligado, tudo entra em um sketch só.",
            "Grava o relatório numa página para abrir em qualquer navegador.", "Grava o relatório em dados, para outro programa ler.", "Clique para realçar o defeito na peça e levar a câmera até ele.",
            "Abre o relatório guardado neste documento, mesmo sem o arquivo de origem.",
        ];
        for (rotulo, ..) in Relatorio::default().linhas_antes_depois() {
            assert!(crate::i18n::tem_traducao(rotulo), "sem tradução: {rotulo}");
        }
        for t in [TipoDefeito::FaceInvalida, TipoDefeito::ArestaLivre, TipoDefeito::FaceSolta] {
            assert!(crate::i18n::tem_traducao(t.nome()), "{}", t.nome());
        }
        for s in NOTAS.iter().chain(rotulos.iter()) {
            assert!(crate::i18n::tem_traducao(s), "sem tradução: {s}");
        }
        for r in Reparo::TODOS {
            assert!(crate::i18n::tem_traducao(r.nome()) && crate::i18n::tem_traducao(nome_reparo_dica(r)));
        }
        for u in UnidadeArquivo::TODAS {
            assert!(crate::i18n::tem_traducao(u.nome()), "{}", u.nome());
        }
        for d in Destino::TODOS {
            assert!(crate::i18n::tem_traducao(d.nome()) && crate::i18n::tem_traducao(dica_destino(d)), "{}", d.nome());
        }
    }
}
