// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painéis do projeto que vivem fora do `app.rs` (B47, B93, B94, B265).
//!
//! A decisão (máscara, plano, conferência, relações) mora em `fenkra_project`; aqui fica só a
//! interface. O estado de cada diálogo fica na memória temporária do egui, para não crescer o `App`.
//! Cores só por tema (`ui.visuals()`), como manda o padrão de interface.

use super::*;
use fenkra_project::pacote::{self, Estrutura, OpcoesPacote, ResultadoPacote};
use fenkra_project::pn::{self, AmbientePn};
use fenkra_project::salvar_como::{AcaoRef, Conflito, ModoSalvarComo, PlanoSalvarComo, ResolucaoConflito};

/// Campos em edição do ambiente de numeração.
#[derive(Clone, Default)]
struct BufAmbiente {
    origem: String,
    nome: String,
    prefixo: String,
    mascara: String,
}

impl App {
    /// Seção "Numeração de PN" do painel do projeto: ambiente ativo e máscara de cada ambiente (B47).
    pub(super) fn painel_pn(&mut self, ui: &mut egui::Ui) {
        let Some(p) = self.project.as_ref() else { return };
        let ambientes: Vec<AmbientePn> = p.index.pn_ambientes.clone();
        let ativo = p.index.pn_ambiente.clone();
        let previsto = p.pn_previsto();
        let mut escolher: Option<String> = None;
        let mut salvar: Option<AmbientePn> = None;
        egui::CollapsingHeader::new(tr("Numeração de PN")).id_salt("pn_painel").show(ui, |ui| {
            ui.label(RichText::new(format!("{}: {}", tr("Próximo PN"), previsto)).weak());
            ui.horizontal(|ui| {
                ui.label(tr("Ambiente"));
                egui::ComboBox::from_id_salt("pn_amb").selected_text(if ativo.is_empty() { tr("(simples)").to_string() } else { ativo.clone() }).show_ui(ui, |ui| {
                    if ui.selectable_label(ativo.is_empty(), tr("(simples)")).clicked() {
                        escolher = Some(String::new());
                    }
                    for a in &ambientes {
                        if ui.selectable_label(a.nome == ativo, &a.nome).clicked() {
                            escolher = Some(a.nome.clone());
                        }
                    }
                });
            })
            .response
            .on_hover_text(tr("Cada ambiente (produção, protótipo...) tem máscara e contador próprios"));

            let id = ui.make_persistent_id("pn_buf");
            let mut buf: BufAmbiente = ui.data(|d| d.get_temp(id)).unwrap_or_default();
            // ao trocar o ambiente ativo, o formulário passa a mostrar o dele
            if buf.origem != ativo {
                buf = match ambientes.iter().find(|a| a.nome == ativo) {
                    Some(a) => BufAmbiente { origem: ativo.clone(), nome: a.nome.clone(), prefixo: a.prefixo.clone(), mascara: a.mascara.clone() },
                    None => BufAmbiente { origem: ativo.clone(), nome: String::new(), prefixo: "FK".into(), mascara: "{PREFIXO}-{TIPO}-{SEQ:4}".into() },
                };
            }
            ui.horizontal(|ui| {
                ui.label(tr("Nome"));
                ui.add(egui::TextEdit::singleline(&mut buf.nome).desired_width(110.).hint_text(tr("novo ambiente")));
                ui.label(tr("Prefixo"));
                ui.add(egui::TextEdit::singleline(&mut buf.prefixo).desired_width(50.));
            });
            let r = ui.add(egui::TextEdit::singleline(&mut buf.mascara).desired_width(f32::INFINITY).hint_text("{PREFIXO}-{ANO}-{SEQ:4}"));
            let valida = pn::validar(&buf.mascara);
            let exemplo = pn::exemplo(&buf.mascara);
            r.on_hover_text(format!(
                "{}\n{}: {}",
                tr("Campos: {PREFIXO} {ANO} {TIPO} {AMB} {CAMPO:nome} {SEQ:N}. Um {SEQ:N} é obrigatório."),
                tr("Exemplo"),
                exemplo.clone().unwrap_or_else(|e| e.to_string())
            ));
            match (&valida, &exemplo) {
                (Err(e), _) => {
                    ui.label(RichText::new(e.to_string()).color(ui.visuals().error_fg_color).small());
                }
                (_, Ok(ex)) => {
                    ui.label(RichText::new(format!("{}: {}", tr("Exemplo"), ex)).weak().small());
                }
                _ => {}
            }
            if ui.add_enabled(valida.is_ok() && !buf.nome.trim().is_empty(), egui::Button::new(tr("Salvar ambiente"))).clicked() {
                salvar = Some(AmbientePn::novo(buf.nome.trim(), buf.prefixo.trim(), buf.mascara.trim()));
            }
            ui.data_mut(|d| d.insert_temp(id, buf));
        });
        if let Some(a) = salvar {
            let nome = a.nome.clone();
            let p = self.project.as_mut().unwrap();
            match p.definir_ambiente_pn(a).and_then(|_| p.ativar_ambiente_pn(&nome)) {
                Ok(()) => self.toast(format!("Ambiente {nome} salvo"), false),
                Err(e) => self.toast(e.to_string(), true),
            }
        } else if let Some(n) = escolher {
            if let Err(e) = self.project.as_mut().unwrap().ativar_ambiente_pn(&n) {
                self.toast(e.to_string(), true);
            }
        }
    }
}

// ----------------------------------------------------------------------------- B93

/// Diálogo "Salvar como": o plano editável (modo, destino, tabela de referências).
#[derive(Clone)]
struct SalvarComoUi {
    plano: PlanoSalvarComo,
    destino_txt: String,
    prefixo: String,
    sufixo: String,
}

/// Diálogo "Empacotar".
#[derive(Clone)]
struct PacoteUi {
    raiz: PathBuf,
    destino_txt: String,
    estrutura: Estrutura,
    desenhos: bool,
    linkados: bool,
    derivados: bool,
    pular_bibliotecas: bool,
    pastas_biblioteca: String,
    resultado: Option<Result<ResultadoPacote, String>>,
}

fn id_salvar_como() -> egui::Id {
    egui::Id::new("fenkra_salvar_como_ref")
}
fn id_pacote() -> egui::Id {
    egui::Id::new("fenkra_empacotar")
}

impl App {
    /// Arquivo e conteúdo do documento da bancada ativa. O conteúdo é o da memória (com edições).
    fn arquivo_do_documento_ativo(&self) -> Option<(PathBuf, String)> {
        use crate::settings::Bench;
        match self.settings.bench {
            Bench::Montagem => self.asm.as_ref().and_then(|s| s.file.clone().map(|f| (f, s.asm.to_json()))),
            Bench::Desenho => self.dwg.as_ref().and_then(|s| s.path.clone().map(|f| (f, s.d.to_json()))),
            _ => self.open.file.clone().map(|f| (f, self.open.doc.to_json())),
        }
    }

    /// Abre o diálogo Salvar como com referências (B93).
    pub(super) fn abrir_salvar_como(&mut self, ctx: &egui::Context) {
        let Some((origem, texto)) = self.arquivo_do_documento_ativo() else {
            self.toast(tr("Salve o documento uma vez antes de usar Salvar como com referências"), true);
            return;
        };
        let stem = origem.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let ext = origem.extension().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "fkp".into());
        let destino = origem.with_file_name(format!("{stem}_copia.{ext}"));
        match PlanoSalvarComo::planejar(&origem, Some(texto), &destino, ModoSalvarComo::CopiaAbrir) {
            Ok(mut plano) => {
                if let Some(p) = &self.project {
                    plano.pastas_dependentes.push(p.root.clone());
                }
                let destino_txt = plano.destino.display().to_string();
                ctx.data_mut(|d| d.insert_temp(id_salvar_como(), Some(SalvarComoUi { plano, destino_txt, prefixo: String::new(), sufixo: String::new() })));
            }
            Err(e) => self.toast(e.to_string(), true),
        }
    }

    /// Abre o diálogo Empacotar para o documento ativo (B93).
    pub(super) fn abrir_empacotar(&mut self, ctx: &egui::Context) {
        let Some((origem, _)) = self.arquivo_do_documento_ativo() else {
            self.toast(tr("Salve o documento uma vez antes de empacotar"), true);
            return;
        };
        let destino = origem.with_extension("zip");
        let pastas_biblioteca = self.project.as_ref().map(|p| p.root.join("biblioteca").display().to_string()).unwrap_or_default();
        ctx.data_mut(|d| {
            d.insert_temp(
                id_pacote(),
                Some(PacoteUi {
                    raiz: origem,
                    destino_txt: destino.display().to_string(),
                    estrutura: Estrutura::Hierarquica,
                    desenhos: true,
                    linkados: true,
                    derivados: true,
                    pular_bibliotecas: false,
                    pastas_biblioteca,
                    resultado: None,
                }),
            )
        });
    }

    /// Janelas do projeto, chamadas a cada quadro.
    pub(super) fn janelas_projeto(&mut self, ctx: &egui::Context) {
        self.janela_salvar_como(ctx);
        self.janela_empacotar(ctx);
    }

    fn janela_salvar_como(&mut self, ctx: &egui::Context) {
        let Some(mut st) = ctx.data(|d| d.get_temp::<Option<SalvarComoUi>>(id_salvar_como())).flatten() else { return };
        let (mut fechar, mut executar) = (false, false);
        let mut escolher_destino = false;
        egui::Window::new(tr("Salvar como com referências")).collapsible(false).resizable(true).default_width(640.).show(ctx, |ui| {
            for m in ModoSalvarComo::TODOS {
                ui.radio_value(&mut st.plano.modo, m, tr(m.rotulo())).on_hover_text(tr(m.descricao()));
            }
            ui.label(RichText::new(tr(st.plano.modo.descricao())).weak().small());
            ui.horizontal(|ui| {
                ui.label(tr("Destino"));
                ui.add(egui::TextEdit::singleline(&mut st.destino_txt).desired_width(360.));
                if ui.button(tr("Escolher…")).clicked() {
                    escolher_destino = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("Prefixo"));
                ui.add(egui::TextEdit::singleline(&mut st.prefixo).desired_width(70.));
                ui.label(tr("Sufixo"));
                ui.add(egui::TextEdit::singleline(&mut st.sufixo).desired_width(70.));
                if ui.button(tr("Aplicar aos nomes")).on_hover_text(tr("Recalcula o nome de destino de todas as linhas; edições manuais de nome são refeitas")).clicked() {
                    let (p, s) = (st.prefixo.clone(), st.sufixo.clone());
                    st.plano.definir_prefixo_sufixo(&p, &s);
                }
            });
            ui.separator();
            if st.plano.linhas.is_empty() {
                ui.label(RichText::new(tr("Este documento não referencia outros arquivos.")).weak());
            } else {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr("Referências")).strong());
                    if ui.small_button(tr("Copiar todas")).clicked() {
                        st.plano.marcar_todas(AcaoRef::Copiar);
                    }
                    if ui.small_button(tr("Manter todas")).clicked() {
                        st.plano.marcar_todas(AcaoRef::Manter);
                    }
                });
                let conflitos = st.plano.conflitos();
                egui::ScrollArea::vertical().max_height(260.).show(ui, |ui| {
                    egui::Grid::new("sc_tabela").striped(true).num_columns(5).show(ui, |ui| {
                        for h in ["Copiar", "Arquivo", "Usos", "Nome no destino", "Conflito"] {
                            ui.label(RichText::new(tr(h)).strong());
                        }
                        ui.end_row();
                        for (l, c) in st.plano.linhas.iter_mut().zip(&conflitos) {
                            let mut copiar = l.acao == AcaoRef::Copiar;
                            ui.add_enabled_ui(l.existe, |ui| {
                                if ui.checkbox(&mut copiar, "").clicked() {
                                    l.acao = if copiar { AcaoRef::Copiar } else { AcaoRef::Manter };
                                }
                            });
                            let nome = l.original.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                            let r = ui.label(if l.existe { RichText::new(nome) } else { RichText::new(format!("{nome} ⚠")).color(ui.visuals().error_fg_color) });
                            r.on_hover_text(format!("{}\n{} · {}", l.original.display(), tr(l.tipo.rotulo()), if l.existe { "" } else { tr("arquivo não encontrado") }));
                            ui.label(format!("{}", l.usos));
                            ui.add_enabled(l.acao == AcaoRef::Copiar, egui::TextEdit::singleline(&mut l.nome_destino).desired_width(170.));
                            match c {
                                Conflito::Nenhum => {
                                    ui.label("");
                                }
                                outro => {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(if *outro == Conflito::ExisteNoDestino { tr("já existe") } else { tr("nome repetido") }).color(ui.visuals().warn_fg_color));
                                        let atual = match l.resolucao {
                                            ResolucaoConflito::Renomear => tr("renomear"),
                                            ResolucaoConflito::Substituir => tr("substituir"),
                                            ResolucaoConflito::Pular => tr("pular"),
                                        };
                                        egui::ComboBox::from_id_salt(("sc_res", l.original.clone())).selected_text(atual).show_ui(ui, |ui| {
                                            ui.selectable_value(&mut l.resolucao, ResolucaoConflito::Renomear, tr("renomear"));
                                            ui.selectable_value(&mut l.resolucao, ResolucaoConflito::Substituir, tr("substituir"));
                                            ui.selectable_value(&mut l.resolucao, ResolucaoConflito::Pular, tr("pular"));
                                        });
                                    });
                                }
                            }
                            ui.end_row();
                        }
                    });
                });
            }
            ui.checkbox(&mut st.plano.substituir_destino, tr("Substituir o arquivo de destino se existir"));
            let problemas = st.plano.problemas();
            for p in &problemas {
                ui.label(RichText::new(tr(p)).color(ui.visuals().error_fg_color).small());
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(problemas.is_empty(), egui::Button::new(tr("Salvar"))).clicked() {
                    executar = true;
                }
                if ui.button(tr("Cancelar")).clicked() {
                    fechar = true;
                }
            });
        });
        if escolher_destino {
            let sugestao = st.plano.destino.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if let Some(p) = rfd::FileDialog::new().set_file_name(sugestao).save_file() {
                st.destino_txt = p.display().to_string();
            }
        }
        // o texto do destino manda no plano; a pasta das cópias acompanha, salvo se já foi mudada
        let novo = PathBuf::from(st.destino_txt.trim());
        if !st.destino_txt.trim().is_empty() && st.plano.destino != novo {
            let antiga_pasta = st.plano.destino.parent().map(|p| p.to_path_buf());
            st.plano.destino = novo.clone();
            if Some(st.plano.pasta_refs.clone()) == antiga_pasta {
                st.plano.pasta_refs = novo.parent().map(|p| p.to_path_buf()).unwrap_or_default();
            }
        }
        if executar {
            match st.plano.executar() {
                Ok(r) => {
                    let n = r.gravados.len();
                    let extra = if r.renomeados_por_conflito.is_empty() { String::new() } else { format!(" · {} renomeado(s) por conflito", r.renomeados_por_conflito.len()) };
                    self.toast(format!("{n} arquivo(s) gravado(s){extra}"), false);
                    if r.abrir_destino {
                        self.open_path(&r.destino);
                    }
                    fechar = true;
                }
                Err(e) => self.toast(e.to_string(), true),
            }
        }
        ctx.data_mut(|d| d.insert_temp(id_salvar_como(), if fechar { None } else { Some(st) }));
    }

    fn janela_empacotar(&mut self, ctx: &egui::Context) {
        let Some(mut st) = ctx.data(|d| d.get_temp::<Option<PacoteUi>>(id_pacote())).flatten() else { return };
        let (mut fechar, mut executar) = (false, false);
        egui::Window::new(tr("Empacotar")).collapsible(false).resizable(true).default_width(560.).show(ctx, |ui| {
            ui.label(RichText::new(st.raiz.display().to_string()).weak().small());
            ui.horizontal(|ui| {
                ui.label(tr("Destino"));
                ui.add(egui::TextEdit::singleline(&mut st.destino_txt).desired_width(340.)).on_hover_text(tr("Termine em .zip para um arquivo único; senão é uma pasta nova"));
                if ui.button(tr("Escolher…")).clicked() {
                    if let Some(p) = rfd::FileDialog::new().add_filter("ZIP", &["zip"]).set_file_name("pacote.zip").save_file() {
                        st.destino_txt = p.display().to_string();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("Estrutura"));
                ui.radio_value(&mut st.estrutura, Estrutura::Hierarquica, tr("hierárquica")).on_hover_text(tr("Mantém as pastas relativas; as referências não mudam"));
                ui.radio_value(&mut st.estrutura, Estrutura::Plana, tr("plana")).on_hover_text(tr("Tudo numa pasta; nomes repetidos são renomeados e listados no manifesto"));
            });
            ui.checkbox(&mut st.desenhos, tr("Incluir desenhos que usam o conjunto"));
            ui.checkbox(&mut st.linkados, tr("Incluir arquivos linkados"));
            ui.checkbox(&mut st.derivados, tr("Incluir documentos derivados"));
            ui.checkbox(&mut st.pular_bibliotecas, tr("Pular bibliotecas"));
            let pular = st.pular_bibliotecas;
            ui.add_enabled_ui(pular, |ui| {
                ui.add(egui::TextEdit::singleline(&mut st.pastas_biblioteca).desired_width(f32::INFINITY).hint_text(tr("pastas de biblioteca, separadas por ;")));
            });
            match &st.resultado {
                Some(Ok(r)) => {
                    ui.separator();
                    let m = &r.manifesto;
                    ui.label(format!("{} {} · {} {}", m.entradas.len(), tr("arquivos"), m.colisoes.len(), tr("colisões renomeadas")));
                    for c in &m.colisoes {
                        ui.label(RichText::new(format!("{} → {}", c.pedido, c.gravado)).weak().small());
                    }
                    for x in &m.excluidas {
                        ui.label(RichText::new(format!("{}: {}", x.original, x.motivo)).weak().small());
                    }
                    for a in &m.ausentes {
                        ui.label(RichText::new(format!("{} {a}", tr("ausente na origem:"))).color(ui.visuals().warn_fg_color).small());
                    }
                    let v = &r.verificacao;
                    if v.ok() {
                        ui.label(RichText::new(tr("Pacote reaberto e verificado: tudo confere.")).color(ui.visuals().hyperlink_color));
                    } else {
                        ui.label(
                            RichText::new(format!(
                                "{} {} · {} {}",
                                v.faltando.len() + v.hash_ou_tamanho_diferente.len(),
                                tr("arquivo(s) com problema"),
                                v.refs_quebradas.len(),
                                tr("referência(s) quebrada(s)")
                            ))
                            .color(ui.visuals().error_fg_color),
                        );
                    }
                }
                Some(Err(e)) => {
                    ui.separator();
                    ui.label(RichText::new(e).color(ui.visuals().error_fg_color));
                }
                None => {}
            }
            ui.horizontal(|ui| {
                if ui.button(tr("Empacotar")).clicked() {
                    executar = true;
                }
                if ui.button(tr("Fechar")).clicked() {
                    fechar = true;
                }
            });
        });
        if executar {
            let mut pastas_busca: Vec<PathBuf> = st.raiz.parent().map(|p| vec![p.to_path_buf()]).unwrap_or_default();
            if let Some(p) = &self.project {
                pastas_busca.push(p.root.clone());
            }
            let o = OpcoesPacote {
                estrutura: st.estrutura,
                incluir_desenhos: st.desenhos,
                incluir_linkados: st.linkados,
                incluir_derivados: st.derivados,
                pular_bibliotecas: st.pular_bibliotecas,
                pastas_biblioteca: st.pastas_biblioteca.split(';').map(str::trim).filter(|s| !s.is_empty()).map(PathBuf::from).collect(),
                pastas_busca,
            };
            st.resultado = Some(pacote::empacotar(&st.raiz, &PathBuf::from(st.destino_txt.trim()), &o).map_err(|e| e.to_string()));
        }
        ctx.data_mut(|d| d.insert_temp(id_pacote(), if fechar { None } else { Some(st) }));
    }
}
