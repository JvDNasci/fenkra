// SPDX-License-Identifier: LGPL-2.1-or-later
//! Formatos suportados e conversão em lote (B213). A tela de formatos só lê a matriz do conversor. O
//! painel do lote monta o plano, mostra a simulação e roda a lista simulada numa linha à parte, com cada
//! item num processo filho: um arquivo ruim custa um item, e o programa segue aberto.

use super::exportar::opcoes_ui;
use super::*;
use fenkra_conversor::lote::{self, EntradaLote, ItemLote, PerfilSaida, PlanoLote, ResultadoItem, SeExistir, Situacao};
use fenkra_conversor::processo::Filho;
use fenkra_conversor::{Formato, OpcoesExportar, Registro};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

const LARGURA: f32 = 400.;
const LARGURA_MATRIZ: f32 = 900.;

fn marca(ui: &mut egui::Ui, sim: bool) {
    if sim {
        ui.label(tr("Sim"));
    } else {
        ui.weak("–");
    }
}

/// Tabela da matriz. Devolve se a pessoa pediu para fechar.
fn matriz_ui(ui: &mut egui::Ui) -> bool {
    let matriz = Registro::default().matriz();
    ui.small(tr("Gerada do registro de provedores e conferida por teste, gravando e lendo um modelo de prova em cada formato."));
    ui.add_space(4.);
    egui::ScrollArea::both().max_height(560.).show(ui, |ui| {
        egui::Grid::new("matriz_formatos").striped(true).spacing([10., 4.]).show(ui, |ui| {
            ui.strong(tr("Formato"));
            ui.strong(tr("Ler"));
            ui.strong(tr("Gravar"));
            if let Some(l) = matriz.first() {
                for (nome, _) in l.capacidades.colunas() {
                    ui.strong(tr(nome));
                }
            }
            ui.strong(tr("Provedor"));
            ui.end_row();
            for l in &matriz {
                ui.label(l.formato.nome()).on_hover_text(l.formato.extensoes().join(", "));
                marca(ui, l.capacidades.ler);
                marca(ui, l.capacidades.gravar);
                for (_, valor) in l.capacidades.colunas() {
                    marca(ui, valor);
                }
                ui.label(&l.provedor);
                ui.end_row();
            }
        });
    });
    ui.add_space(6.);
    ui.button(tr("Fechar")).clicked()
}

/// O que a linha de execução manda para a tela.
enum Noticia {
    Item(ResultadoItem),
    Fim,
}

struct Execucao {
    recebe: mpsc::Receiver<Noticia>,
    cancelar: Arc<AtomicBool>,
    total: usize,
    terminou: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Acao {
    Nada,
    Arquivos,
    Pasta,
    Projeto,
    Saida,
    Simular,
    Executar,
    Fechar,
}

/// Painel aberto durante o comando.
pub struct Painel {
    entradas: Vec<EntradaLote>,
    /// um perfil por formato ligado
    perfis: Vec<PerfilSaida>,
    pasta: PathBuf,
    regra: String,
    se_existir: SeExistir,
    /// limite por item, em minutos
    minutos: u32,
    simulacao: Option<Vec<ItemLote>>,
    execucao: Option<Execucao>,
    resultados: Vec<ResultadoItem>,
    tem_projeto: bool,
}

impl Painel {
    fn novo(tem_projeto: bool) -> Painel {
        Painel { entradas: vec![], perfis: vec![], pasta: PathBuf::new(), regra: lote::REGRA_PADRAO.into(), se_existir: SeExistir::default(), minutos: 10, simulacao: None, execucao: None, resultados: vec![], tem_projeto }
    }

    fn plano(&self) -> PlanoLote {
        PlanoLote { pasta: self.pasta.clone(), regra: self.regra.clone(), perfis: self.perfis.clone(), se_existir: self.se_existir, limite: u64::from(self.minutos) * 60, ..Default::default() }
    }

    /// Qualquer mudança no plano tira a validade da simulação.
    fn invalidar(&mut self) {
        self.simulacao = None;
    }

    fn acrescentar(&mut self, novas: Vec<EntradaLote>) {
        for e in novas {
            if !self.entradas.iter().any(|q| q.caminho == e.caminho) {
                self.entradas.push(e);
            }
        }
        self.invalidar();
    }

    fn simular(&mut self) {
        self.simulacao = Some(lote::simular(&self.entradas, &self.plano(), &Registro::default()));
        self.resultados.clear();
    }

    fn executar(&mut self) -> Result<(), String> {
        let Some(itens) = self.simulacao.clone() else { return Ok(()) };
        let plano = self.plano();
        let mut filho = Filho::ao_lado_do_programa().map_err(|e| e.to_string())?;
        filho.limite = Duration::from_secs(plano.limite.max(1));
        let (envia, recebe) = mpsc::channel();
        let cancelar = Arc::new(AtomicBool::new(false));
        let parar = cancelar.clone();
        let total = itens.iter().filter(|i| i.situacao.converte()).count();
        std::thread::spawn(move || {
            lote::executar(&itens, &plano, |t| filho.executar(t, &parar), |_, r| {
                let _ = envia.send(Noticia::Item(r.clone()));
            });
            let _ = envia.send(Noticia::Fim);
        });
        self.resultados.clear();
        self.execucao = Some(Execucao { recebe, cancelar, total, terminou: false });
        Ok(())
    }

    fn avancar(&mut self, ctx: &egui::Context) {
        let Some(x) = &mut self.execucao else { return };
        while let Ok(n) = x.recebe.try_recv() {
            match n {
                Noticia::Item(r) => self.resultados.push(r),
                Noticia::Fim => x.terminou = true,
            }
        }
        if !x.terminou {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn rodando(&self) -> bool {
        self.execucao.as_ref().is_some_and(|x| !x.terminou)
    }

    fn entradas_ui(&mut self, ui: &mut egui::Ui, acao: &mut Acao) {
        ui.strong(tr("Entradas"));
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("Arquivos…")).on_hover_text(tr("Escolhe arquivos soltos em qualquer formato que o Fenkra lê.")).clicked() {
                *acao = Acao::Arquivos;
            }
            if ui.button(tr("Pasta…")).on_hover_text(tr("Acrescenta tudo o que o Fenkra lê nesta pasta e nas de dentro dela.")).clicked() {
                *acao = Acao::Pasta;
            }
            if ui.add_enabled(self.tem_projeto, egui::Button::new(tr("Documentos do projeto"))).on_hover_text(tr("Acrescenta as peças do projeto aberto, com número e revisão para a regra de nome.")).clicked() {
                *acao = Acao::Projeto;
            }
        });
        let mut tirar = None;
        egui::ScrollArea::vertical().id_salt("lote_entradas").max_height(110.).show(ui, |ui| {
            for (i, e) in self.entradas.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui.small_button("x").on_hover_text(tr("Tira esta entrada do lote.")).clicked() {
                        tirar = Some(i);
                    }
                    ui.label(&e.nome).on_hover_text(e.caminho.display().to_string());
                    if !e.pn.is_empty() {
                        ui.weak(format!("{} {}", e.pn, e.rev));
                    }
                });
            }
        });
        if let Some(i) = tirar {
            self.entradas.remove(i);
            self.invalidar();
        }
        if self.entradas.is_empty() {
            ui.weak(tr("Nenhuma entrada ainda."));
        }
    }

    fn saidas_ui(&mut self, ui: &mut egui::Ui, acao: &mut Acao) {
        ui.strong(tr("Saída do lote"));
        let registro = Registro::default();
        let mut mudou = false;
        ui.horizontal_wrapped(|ui| {
            for f in registro.formatos_de_gravacao() {
                let mut ligado = self.perfis.iter().any(|p| p.formato == f);
                if ui.checkbox(&mut ligado, f.nome()).on_hover_text(tr("Liga este formato de saída. Cada entrada sai em todos os formatos ligados.")).changed() {
                    self.perfis.retain(|p| p.formato != f);
                    if ligado {
                        self.perfis.push(PerfilSaida { formato: f, opcoes: OpcoesExportar::default() });
                    }
                    mudou = true;
                }
            }
        });
        for p in &mut self.perfis {
            let antes = p.opcoes;
            egui::CollapsingHeader::new(format!("{} {}", tr("Perfil"), p.formato.nome())).id_salt(("perfil_lote", p.formato.nome())).show(ui, |ui| {
                ui.push_id(p.formato.nome(), |ui| opcoes_ui(ui, p.formato, &mut p.opcoes, true));
            });
            mudou |= antes != p.opcoes;
        }
        ui.horizontal(|ui| {
            if ui.button(tr("Pasta de saída…")).on_hover_text(tr("Onde os arquivos convertidos vão ficar.")).clicked() {
                *acao = Acao::Saida;
            }
            let pasta = self.pasta.display().to_string();
            ui.add(egui::Label::new(egui::RichText::new(&pasta).weak()).truncate()).on_hover_text(pasta);
        });
        ui.label(tr("Regra de nome"));
        mudou |= ui.text_edit_singleline(&mut self.regra).on_hover_text(tr("Use {PN}, {REV}, {NOME}, {FORMATO} e {ext}. Sem número, {PN} vira o nome do arquivo.")).changed();
        ui.label(tr("Se o arquivo já existir"));
        let atual = self.se_existir;
        egui::ComboBox::from_id_salt("lote_existente").selected_text(tr(atual.nome())).width(LARGURA - 40.).show_ui(ui, |ui| {
            for s in SeExistir::TODOS {
                ui.selectable_value(&mut self.se_existir, s, tr(s.nome()));
            }
        }).response.on_hover_text(tr("Pular mantém o que está lá. Substituir troca. Numerar grava ao lado, com um número no nome."));
        mudou |= atual != self.se_existir;
        ui.label(tr("Limite por item"));
        mudou |= ui.add(crate::ui_ext::dv(&mut self.minutos).speed(1).range(1..=240).suffix(" min").dica(tr("Item que passar deste tempo é encerrado e marcado com erro, sem deixar arquivo pela metade."))).changed();
        if mudou {
            self.invalidar();
        }
    }

    fn simulacao_ui(&self, ui: &mut egui::Ui) {
        let Some(itens) = &self.simulacao else { return };
        let converte = itens.iter().filter(|i| i.situacao.converte()).count();
        ui.strong(tr("{0} de {1} itens vão ser convertidos").replace("{0}", &converte.to_string()).replace("{1}", &itens.len().to_string()));
        egui::ScrollArea::vertical().id_salt("lote_simulacao").max_height(150.).show(ui, |ui| {
            for i in itens {
                let nome = i.destino.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let (rotulo, detalhe) = match &i.situacao {
                    Situacao::Converter => (tr("Grava"), None),
                    Situacao::Substituir => (tr("Substitui"), None),
                    Situacao::Numerado => (tr("Grava ao lado"), None),
                    Situacao::Pular(m) => (tr("Pula"), Some(m.as_str())),
                    Situacao::Recusado(m) => (tr("Recusa"), Some(m.as_str())),
                };
                let origem = i.origem.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let linha = ui.add(egui::Label::new(format!("{rotulo}: {origem} → {nome}")).truncate());
                if let Some(d) = detalhe {
                    linha.on_hover_text(tr(d));
                    ui.weak(tr(d));
                }
            }
        });
    }

    fn resultados_ui(&self, ui: &mut egui::Ui) {
        let Some(x) = &self.execucao else { return };
        let certos = self.resultados.iter().filter(|r| r.saida.is_ok()).count();
        ui.add(egui::ProgressBar::new(self.resultados.len() as f32 / x.total.max(1) as f32).text(format!("{} / {}", self.resultados.len(), x.total)));
        if x.terminou {
            ui.strong(tr("Lote terminado: {0} certos e {1} com erro").replace("{0}", &certos.to_string()).replace("{1}", &(self.resultados.len() - certos).to_string()));
        }
        egui::ScrollArea::vertical().id_salt("lote_resultados").max_height(100.).show(ui, |ui| {
            for r in self.resultados.iter().filter(|r| r.saida.is_err()) {
                let nome = r.origem.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if let Err(e) = &r.saida {
                    ui.label(format!("{nome}: {e}"));
                }
            }
        });
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Acao {
        ui.set_max_width(LARGURA - 16.);
        let mut acao = Acao::Nada;
        ui.add_enabled_ui(!self.rodando(), |ui| {
            egui::ScrollArea::vertical().id_salt("lote_plano").max_height(420.).show(ui, |ui| {
                self.entradas_ui(ui, &mut acao);
                ui.add_space(6.);
                self.saidas_ui(ui, &mut acao);
            });
        });
        ui.add_space(6.);
        self.simulacao_ui(ui);
        self.resultados_ui(ui);
        ui.add_space(6.);
        ui.horizontal(|ui| {
            let pronto = !self.entradas.is_empty() && !self.perfis.is_empty() && !self.pasta.as_os_str().is_empty();
            if ui.add_enabled(pronto && !self.rodando(), egui::Button::new(tr("Simular"))).on_hover_text(tr("Mostra o que cada item vai virar, sem gravar nada.")).clicked() {
                acao = Acao::Simular;
            }
            let tem_o_que = self.simulacao.as_ref().is_some_and(|s| s.iter().any(|i| i.situacao.converte()));
            if ui.add_enabled(tem_o_que && !self.rodando(), egui::Button::new(tr("Executar"))).on_hover_text(tr("Converte a lista simulada. Só depois de simular.")).clicked() {
                acao = Acao::Executar;
            }
            if self.rodando() {
                if ui.button(tr("Cancelar")).on_hover_text(tr("Encerra o item em andamento e não começa os outros.")).clicked() {
                    if let Some(x) = &self.execucao {
                        x.cancelar.store(true, Ordering::Relaxed);
                    }
                }
            } else if ui.button(tr("Fechar")).clicked() {
                acao = Acao::Fechar;
            }
        });
        acao
    }
}

impl App {
    pub(super) fn abrir_formatos(&mut self) {
        self.formatos_aberto = true;
        self.started = true;
    }

    pub(super) fn abrir_painel_lote(&mut self) {
        self.lote_painel = Some(Painel::novo(self.project.is_some()));
        self.started = true;
    }

    /// Lote de exemplo para a captura de tela: um STEP de verdade e um arquivo que não existe.
    pub(super) fn demo_lote(&mut self, pasta: PathBuf) {
        let mut p = Painel::novo(false);
        p.entradas.push(EntradaLote { caminho: pasta.join("placa.step"), nome: "Placa".into(), pn: "FK-0002".into(), rev: "B".into() });
        p.entradas.push(EntradaLote { caminho: pasta.join("sumiu.step"), nome: "Sumiu".into(), pn: String::new(), rev: String::new() });
        p.perfis.push(PerfilSaida { formato: Formato::Tmf, opcoes: OpcoesExportar::default() });
        p.perfis.push(PerfilSaida { formato: Formato::Glb, opcoes: OpcoesExportar::default() });
        p.pasta = pasta;
        p.simular();
        self.lote_painel = Some(p);
        self.started = true;
    }

    fn entradas_do_projeto(&self) -> Vec<EntradaLote> {
        let Some(p) = &self.project else { return vec![] };
        p.index.documents.iter().map(|d| EntradaLote { caminho: p.root.join(&d.file), nome: d.name.clone(), pn: d.pn.clone(), rev: d.revisions.last().map(|r| r.label.clone()).unwrap_or_default() }).collect()
    }

    pub(super) fn painel_lote(&mut self, ctx: &egui::Context, area: Rect) {
        use crate::paineis::{self, Encaixe, Painel as Quadro};
        if self.formatos_aberto {
            let padrao = paineis::ao_lado_da_arvore(area);
            let cfg = Quadro { nome: "formatos".into(), titulo: tr("Formatos suportados").to_string(), padrao, largura: LARGURA_MATRIZ, redimensiona: true };
            let posicao = self.settings.painel("formatos", Encaixe::Livre);
            let saida = paineis::mostrar(ctx, area, &cfg, posicao, matriz_ui);
            if let Some(p) = saida.movido {
                self.settings.guardar_painel("formatos", p);
            }
            self.formatos_aberto = !saida.conteudo.unwrap_or(false);
        }
        let Some(mut d) = self.lote_painel.take() else { return };
        d.avancar(ctx);
        let padrao = paineis::abaixo_do_cubo(area, LARGURA);
        let cfg = Quadro { nome: "lote".into(), titulo: tr("Conversão em lote").to_string(), padrao, largura: LARGURA, redimensiona: false };
        let posicao = self.settings.painel("lote", Encaixe::Direita);
        let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| d.ui(ui));
        if let Some(p) = saida.movido {
            self.settings.guardar_painel("lote", p);
        }
        let registro = Registro::default();
        match saida.conteudo.unwrap_or(Acao::Nada) {
            Acao::Nada => {}
            Acao::Arquivos => {
                let extensoes = registro.extensoes_de_leitura();
                if let Some(l) = rfd::FileDialog::new().add_filter(tr("Todos os formatos"), &extensoes).add_filter("Fenkra", &["fkp", "fka"]).pick_files() {
                    d.acrescentar(lote::listar_entradas(&l, &registro));
                }
            }
            Acao::Pasta => {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    d.acrescentar(lote::listar_entradas(&[p], &registro));
                }
            }
            Acao::Projeto => d.acrescentar(self.entradas_do_projeto()),
            Acao::Saida => {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    d.pasta = p;
                    d.invalidar();
                }
            }
            Acao::Simular => d.simular(),
            Acao::Executar => {
                if let Err(e) = d.executar() {
                    self.toast(e, true);
                }
            }
            Acao::Fechar => return,
        }
        self.lote_painel = Some(d);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn textos_novos_tem_traducao() {
        for t in lote::MOTIVOS {
            assert!(crate::i18n::tem_traducao(t), "sem tradução: {t}");
        }
        for s in SeExistir::TODOS {
            assert!(crate::i18n::tem_traducao(s.nome()), "sem tradução: {}", s.nome());
        }
    }

    #[test]
    fn mudar_o_plano_tira_a_simulacao_e_sem_simular_nao_executa() {
        let mut p = Painel::novo(false);
        p.pasta = std::env::temp_dir();
        p.entradas.push(EntradaLote::de_arquivo(std::path::Path::new("nao-existe.step")));
        p.perfis.push(PerfilSaida { formato: Formato::Stl, opcoes: OpcoesExportar::default() });
        p.simular();
        assert!(p.simulacao.is_some());
        assert!(matches!(p.simulacao.as_ref().unwrap()[0].situacao, Situacao::Recusado(_)));
        p.acrescentar(vec![EntradaLote::de_arquivo(std::path::Path::new("outro.step"))]);
        assert!(p.simulacao.is_none());
        assert!(p.executar().is_ok());
        assert!(p.execucao.is_none());
    }

    #[test]
    fn matriz_aparece_com_as_colunas_do_conversor() {
        let ctx = egui::Context::default();
        let mut textos = vec![];
        for _ in 0..2 {
            let entrada = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900., 800.))), ..Default::default() };
            let mut saida = ctx.run_ui(entrada, |ui| {
                matriz_ui(ui);
            });
            saida.textures_delta.clear();
            textos = saida.shapes.iter().filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) => Some(t.galley.text().to_string()),
                _ => None,
            }).collect();
        }
        let t = textos.join("\n");
        for esperado in ["Geometria exata", "Montagem", "Parâmetros", "STEP", "3MF", "USDZ", "Provedor"] {
            assert!(t.contains(esperado), "{esperado}: {t}");
        }
    }
}
