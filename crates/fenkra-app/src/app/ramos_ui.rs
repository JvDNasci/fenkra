// SPDX-License-Identifier: LGPL-2.1-or-later
//! Interface dos ramos e da mesclagem (B130): lista de ramos no painel do projeto, criar, trocar,
//! pedir mesclagem, aprovar, e a resolução de conflito lado a lado, feature por feature.
//! A decisão mora em `fenkra_project::ramos`; aqui ficam a tela e o gancho `verificar`, que roda o
//! recálculo real do documento mesclado (status, volume e referências) antes de qualquer aprovação
//! ou mesclagem. Cores só por tema; o estado das janelas fica na memória temporária do egui.

use super::*;
use fenkra_project::ramos::{EstadoPedido, EstadoRamo, Lado, Mescla, Ramo, PRINCIPAL};
use std::collections::HashMap;

/// Conferência do documento mesclado com o recálculo real. Vazio = pode entrar.
/// - status: nenhum comando ativo com erro (a causa vem do classificador, não do texto);
/// - referências: nenhuma referência perdida;
/// - volume: todo corpo sólido com volume finito e positivo.
pub(crate) fn verificar_recalculo(doc: &Document) -> Vec<String> {
    conferir(doc, None)
}

/// A mesma conferência e, com `perfil`, o checador de qualidade (B305) no momento "ao criar revisão":
/// regra que bloqueia barra a aprovação e a mesclagem; aviso e informação não barram.
pub(crate) fn conferir(doc: &Document, perfil: Option<&fenkra_project::qualidade_modelo::Perfil>) -> Vec<String> {
    use fenkra_solidos::erro::ErrorKind;
    use fenkra_solidos::Status;
    let r = fenkra_solidos::recompute(doc);
    let mut p = vec![];
    if let Some(e) = &r.param_error {
        p.push(format!("parâmetro com erro: {e}"));
    }
    let tipos = r.tipos_de_erro(doc);
    for f in doc.features.iter().filter(|f| !f.suppressed) {
        if let Some(Status::Error(msg)) = r.status.get(&f.id) {
            let causa = match tipos.get(&f.id) {
                Some(ErrorKind::ReferenciaPerdida { .. }) | Some(ErrorKind::GeometriaPerdida) => "referência perdida",
                Some(ErrorKind::PaiFalhou { .. }) => "depende de um comando que falhou",
                _ => "erro",
            };
            p.push(format!("{} ({causa}): {msg}", f.name));
        }
    }
    for b in &r.bodies {
        if !b.mass.volume.is_finite() || (b.faces > 0 && b.mass.volume <= 0.) {
            p.push(format!("corpo sem volume válido ({:.3})", b.mass.volume));
        }
    }
    if let Some(perfil) = perfil {
        let m = super::plm::medidas_do_resultado(&r, doc);
        let rel = fenkra_project::qualidade_modelo::avaliar(perfil, &m, fenkra_project::qualidade_modelo::Momento::Revisar, &[]);
        if rel.bloqueia() {
            p.push(format!("checador de qualidade: {}", rel.resumo_bloqueios()));
        }
    }
    p
}

#[derive(Clone, Default)]
struct RamosUi {
    novo: String,
    mensagem: String,
    motivo: String,
}

#[derive(Clone)]
struct MesclaUi {
    id: u64,
    nome: String,
    escolhas: HashMap<String, Lado>,
}

fn id_mescla() -> egui::Id {
    egui::Id::new("fenkra_mescla_ramo")
}

fn cor_pedido(ui: &egui::Ui, e: EstadoPedido) -> egui::Color32 {
    match e {
        EstadoPedido::Aprovado => ui.visuals().hyperlink_color,
        EstadoPedido::Recusado => ui.visuals().error_fg_color,
        EstadoPedido::Aberto => ui.visuals().warn_fg_color,
    }
}

fn rotulo_pedido(e: EstadoPedido) -> &'static str {
    match e {
        EstadoPedido::Aberto => "pedido aberto",
        EstadoPedido::Aprovado => "aprovado",
        EstadoPedido::Recusado => "recusado",
    }
}

enum AcaoRamo {
    Criar(String),
    Trocar(String),
    Principal,
    Pedir(String),
    Aprovar(String),
    Recusar(String),
    Mesclar(String),
    Descartar(String),
    /// traz a principal de hoje para dentro do ramo
    Trazer(String),
    /// cópia derivada (fork) do documento
    Derivar(String),
    /// leva a cópia derivada de volta ao original como ramo
    Levar(String),
}

impl App {
    /// Grava no ramo ativo em vez de na principal. Devolve `true` se tratou o salvamento.
    pub(super) fn salvar_no_ramo_ativo(&mut self) -> bool {
        let (Some(id), Some(nome)) = (self.open.entry, self.open.ramo.clone()) else { return false };
        let Some(p) = self.project.as_mut() else { return false };
        match p.salvar_no_ramo(id, &nome, &self.open.doc, "salvo no ramo") {
            Ok(()) => {
                self.open.modified = false;
                self.toast(format!("{} '{nome}'", tr("Trabalho salvo no ramo")), false);
            }
            Err(e) => self.toast(e.to_string(), true),
        }
        true
    }

    /// Desenvolvimento (`FENKRA_DEMO_JANELA=ramos` com `FENKRA_DEMO_PROJETO=1`): monta um ramo que
    /// conflita com a principal, aprova o pedido e abre a janela de mesclagem.
    pub(super) fn demo_ramos(&mut self) {
        let (Some(id), Some(p)) = (self.project_sel, self.project.as_mut()) else { return };
        let mut montar = || -> fenkra_project::PResult<()> {
            p.criar_ramo(id, "variante")?;
            let mut d = p.carregar_ramo(id, "variante")?;
            if let Some(q) = d.params.iter_mut().find(|q| q.name == "esp") {
                q.expr = "20".into();
            }
            p.salvar_no_ramo(id, "variante", &d, "mais grossa")?;
            let mut dp = p.load(id)?;
            if let Some(q) = dp.params.iter_mut().find(|q| q.name == "esp") {
                q.expr = "16".into();
            }
            p.save(id, &dp, "principal mudou a espessura")?;
            p.pedir_mesclagem(id, "variante", "Espessura maior para a versão reforçada")?;
            p.revisar_pedido(id, "variante", true, "", &verificar_recalculo)
        };
        if let Err(e) = montar() {
            self.toast(e.to_string(), true);
        }
    }

    /// Abre a janela de mesclagem do primeiro ramo aberto (desenvolvimento).
    pub(super) fn demo_abrir_mescla(&mut self, ctx: &egui::Context) {
        let (Some(id), Some(p)) = (self.project_sel, self.project.as_ref()) else { return };
        if let Some(r) = p.ramos(id).into_iter().find(|r| r.estado == EstadoRamo::Aberto) {
            ctx.data_mut(|d| d.insert_temp(id_mescla(), Some(MesclaUi { id, nome: r.nome, escolhas: HashMap::new() })));
        }
    }

    /// Seção "Ramos" do painel do projeto: lista, criar, trocar, pedir mesclagem e aprovar (B130).
    pub(super) fn secao_ramos(&mut self, ui: &mut egui::Ui, sel: u64) {
        let Some(p) = self.project.as_ref() else { return };
        let ramos: Vec<Ramo> = p.ramos(sel).into_iter().filter(|r| r.estado == EstadoRamo::Aberto).collect();
        let avisos = p.avisos_de_sobreposicao(sel).unwrap_or_default();
        let aprovador = p.papeis.iter().any(|x| x == "aprovador");
        // quanto a principal andou desde cada ramo: quem está no ramo continua vendo a linha principal
        let andou: HashMap<String, usize> = ramos.iter().map(|r| (r.nome.clone(), p.principal_desde_o_ramo(sel, &r.nome).map(|c| c.total()).unwrap_or(0))).collect();
        let origem = p.origem_do_derivado(sel);
        let ativo = if self.open.entry == Some(sel) { self.open.ramo.clone() } else { None };
        let mut st: RamosUi = ui.ctx().data(|d| d.get_temp(egui::Id::new(("ramos_ui", sel)))).unwrap_or_default();
        let mut acao: Option<AcaoRamo> = None;

        let cab = ui.horizontal(|ui| {
            ui.add(egui::Image::new(crate::icons::get("comparar_versoes")).fit_to_exact_size(egui::vec2(18., 18.)));
            ui.label(RichText::new(format!("{} ({})", tr("Ramos"), ramos.len())).strong());
        });
        cab.response.on_hover_text(tr("Em vez de travar a peça, cada pessoa trabalha num ramo. A linha principal só recebe um ramo depois de um pedido de mesclagem aprovado."));
        egui::CollapsingHeader::new(tr("Ramos e mesclagem")).id_salt(("ramos", sel)).default_open(!ramos.is_empty() || std::env::var("FENKRA_DEMO_SECOES").is_ok()).show(ui, |ui| {
            // linha principal, sempre presente
            ui.horizontal(|ui| {
                let na_principal = ativo.is_none();
                ui.label(RichText::new(format!("{} {PRINCIPAL}", if na_principal { "●" } else { "○" })).strong());
                ui.label(RichText::new(tr("protegida")).weak().small()).on_hover_text(tr("Só recebe ramos com pedido de mesclagem aprovado por quem tem o papel de aprovador."));
                if !na_principal && ui.small_button(tr("Voltar à principal")).on_hover_text(tr("Salva o ramo e abre a versão de trabalho da principal")).clicked() {
                    acao = Some(AcaoRamo::Principal);
                }
            });
            for r in &ramos {
                ui.horizontal_wrapped(|ui| {
                    let aqui = ativo.as_deref() == Some(r.nome.as_str());
                    ui.label(RichText::new(format!("{} {}", if aqui { "●" } else { "○" }, r.nome)).strong());
                    ui.label(RichText::new(format!("{} · {} {}", r.autor, r.versoes.len(), tr("versões"))).weak().small());
                    if let Some(pd) = &r.pedido {
                        ui.label(RichText::new(tr(rotulo_pedido(pd.estado))).small().color(cor_pedido(ui, pd.estado))).on_hover_text(if pd.motivo.is_empty() { pd.mensagem.clone() } else { format!("{}\n{}", pd.mensagem, pd.motivo) });
                    }
                    if !aqui && ui.small_button(tr("Trocar")).on_hover_text(tr("Abre o ramo para editar; a principal não muda")).clicked() {
                        acao = Some(AcaoRamo::Trocar(r.nome.clone()));
                    }
                    let n = andou.get(&r.nome).copied().unwrap_or(0);
                    if n > 0 {
                        ui.label(RichText::new(format!("{} {n}", tr("principal andou:"))).small().color(ui.visuals().warn_fg_color));
                        if ui.small_button(tr("Trazer a principal")).on_hover_text(tr("Traz para o ramo o que a principal mudou. Onde os dois mexeram no mesmo valor, fica o do ramo; a principal não muda.")).clicked() {
                            acao = Some(AcaoRamo::Trazer(r.nome.clone()));
                        }
                    }
                    let estado = r.pedido.as_ref().map(|x| x.estado);
                    if estado != Some(EstadoPedido::Aberto) && estado != Some(EstadoPedido::Aprovado) && ui.small_button(tr("Pedir mesclagem")).on_hover_text(tr("Pede a entrada deste ramo na linha principal, com a mensagem abaixo")).clicked() {
                        acao = Some(AcaoRamo::Pedir(r.nome.clone()));
                    }
                    if estado == Some(EstadoPedido::Aberto) {
                        let dica = if aprovador { tr("Confere o resultado da mesclagem com o recálculo e aprova") } else { tr("Falta o papel de aprovador") };
                        if ui.add_enabled(aprovador, egui::Button::new(tr("Aprovar")).small()).on_hover_text(dica).clicked() {
                            acao = Some(AcaoRamo::Aprovar(r.nome.clone()));
                        }
                        if ui.add_enabled(aprovador, egui::Button::new(tr("Recusar")).small()).on_hover_text(tr("Recusa o pedido; o motivo abaixo fica no diário")).clicked() {
                            acao = Some(AcaoRamo::Recusar(r.nome.clone()));
                        }
                    }
                    if estado == Some(EstadoPedido::Aprovado) && ui.add_enabled(aprovador, egui::Button::new(tr("Mesclar…")).small()).on_hover_text(tr("Mostra os conflitos lado a lado e entra na principal")).clicked() {
                        acao = Some(AcaoRamo::Mesclar(r.nome.clone()));
                    }
                    if ui.small_button("✖").on_hover_text(tr("Descartar o ramo (as versões continuam guardadas)")).clicked() {
                        acao = Some(AcaoRamo::Descartar(r.nome.clone()));
                    }
                });
            }
            for a in &avisos {
                let contra = if a.contra == PRINCIPAL { tr("a principal").to_string() } else { format!("{} '{}'", tr("o ramo"), a.contra) };
                ui.label(RichText::new(format!("⚠ {} '{}' {} {} ({} {})", tr("O ramo"), a.ramo, tr("mexeu nas mesmas features que"), contra, a.features.len(), tr("feature(s)"))).small().color(ui.visuals().warn_fg_color));
            }
            ui.add(egui::TextEdit::singleline(&mut st.mensagem).hint_text(tr("Mensagem do pedido de mesclagem")).desired_width(260.));
            ui.add(egui::TextEdit::singleline(&mut st.motivo).hint_text(tr("Motivo da revisão (opcional)")).desired_width(260.));
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::TextEdit::singleline(&mut st.novo).hint_text(tr("Nome do novo ramo")).desired_width(160.));
                let ok = !st.novo.trim().is_empty();
                if ui.add_enabled(ok, egui::Button::new(tr("Criar ramo"))).on_hover_text(tr("Abre um ramo a partir da versão atual da principal")).clicked() {
                    acao = Some(AcaoRamo::Criar(st.novo.trim().to_string()));
                }
                if ui.add_enabled(ok, egui::Button::new(tr("Cópia derivada"))).on_hover_text(tr("Cria um documento novo a partir da principal, com vida própria; depois ele pode voltar ao original como ramo")).clicked() {
                    acao = Some(AcaoRamo::Derivar(st.novo.trim().to_string()));
                }
            });
            if let Some(o) = origem {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("{} #{o}", tr("Cópia derivada de"))).small());
                    let ok = !st.novo.trim().is_empty();
                    if ui.add_enabled(ok, egui::Button::new(tr("Levar ao original como ramo")).small()).on_hover_text(tr("A principal desta cópia vira um ramo do original, com o nome acima; daí segue o pedido de mesclagem")).clicked() {
                        acao = Some(AcaoRamo::Levar(st.novo.trim().to_string()));
                    }
                });
            }
        });

        if let Some(a) = acao {
            self.executar_acao_ramo(ui.ctx(), sel, a, &mut st);
        }
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("ramos_ui", sel)), st));
    }

    fn executar_acao_ramo(&mut self, ctx: &egui::Context, sel: u64, acao: AcaoRamo, st: &mut RamosUi) {
        // trabalho em andamento não se perde ao trocar de linha
        let precisa_salvar = matches!(acao, AcaoRamo::Trocar(_) | AcaoRamo::Principal | AcaoRamo::Mesclar(_)) && self.open.modified && self.open.entry == Some(sel) && self.open.frozen.is_none();
        if precisa_salvar {
            if self.open.ramo.is_some() {
                self.salvar_no_ramo_ativo();
            } else {
                self.save();
            }
        }
        let Some(p) = self.project.as_mut() else { return };
        let perfil = p.perfil_qualidade();
        let r: Result<String, String> = match acao {
            AcaoRamo::Criar(nome) => {
                let r = p.criar_ramo(sel, &nome).map(|_| format!("{} '{nome}'", tr("Ramo criado"))).map_err(|e| e.to_string());
                if r.is_ok() {
                    st.novo.clear();
                }
                r
            }
            AcaoRamo::Trocar(nome) => match p.carregar_ramo(sel, &nome) {
                Ok(d) => {
                    self.load_doc(d, Some(sel), None);
                    self.open.ramo = Some(nome.clone());
                    Ok(format!("{} '{nome}'", tr("Editando o ramo")))
                }
                Err(e) => Err(e.to_string()),
            },
            AcaoRamo::Principal => {
                self.open_entry(sel, None);
                Ok(tr("De volta à linha principal").to_string())
            }
            AcaoRamo::Pedir(nome) => {
                let msg = if st.mensagem.trim().is_empty() { format!("{} {nome}", tr("Mesclar")) } else { st.mensagem.trim().to_string() };
                p.pedir_mesclagem(sel, &nome, &msg).map(|_| tr("Pedido de mesclagem enviado").to_string()).map_err(|e| e.to_string())
            }
            AcaoRamo::Aprovar(nome) => p.revisar_pedido(sel, &nome, true, st.motivo.trim(), &|d: &Document| conferir(d, Some(&perfil))).map(|_| tr("Pedido aprovado: o resultado recalcula sem erro").to_string()).map_err(|e| e.to_string()),
            AcaoRamo::Recusar(nome) => p.revisar_pedido(sel, &nome, false, st.motivo.trim(), &verificar_recalculo).map(|_| tr("Pedido recusado").to_string()).map_err(|e| e.to_string()),
            AcaoRamo::Mesclar(nome) => {
                ctx.data_mut(|d| d.insert_temp(id_mescla(), Some(MesclaUi { id: sel, nome, escolhas: HashMap::new() })));
                return;
            }
            AcaoRamo::Trazer(nome) => {
                // onde os dois mexeram no mesmo valor, quem está no ramo fica com o próprio trabalho
                let escolhas: HashMap<String, Lado> = p.previa_atualizar_ramo(sel, &nome, &HashMap::new()).map(|m| m.conflitos.into_iter().map(|c| (c.chave, Lado::Nossa)).collect()).unwrap_or_default();
                let r = p.atualizar_ramo_com_principal(sel, &nome, &escolhas, &|d: &Document| conferir(d, None));
                match r {
                    Ok(m) => {
                        if self.open.entry == Some(sel) && self.open.ramo.as_deref() == Some(nome.as_str()) {
                            if let Ok(d) = self.project.as_ref().unwrap().carregar_ramo(sel, &nome) {
                                self.load_doc(d, Some(sel), None);
                                self.open.ramo = Some(nome.clone());
                            }
                        }
                        Ok(format!("{} '{nome}' ({} {})", tr("Principal trazida para o ramo"), m.conflitos.len(), tr("conflito(s) ficaram com o ramo")))
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            AcaoRamo::Derivar(nome) => {
                let r = p.derivar_documento(sel, &nome).map(|_| format!("{} '{nome}'", tr("Cópia derivada criada"))).map_err(|e| e.to_string());
                if r.is_ok() {
                    st.novo.clear();
                }
                r
            }
            AcaoRamo::Levar(nome) => {
                let r = p.trazer_derivado_como_ramo(sel, &nome).map(|_| format!("{} '{nome}'", tr("Levada ao original como ramo"))).map_err(|e| e.to_string());
                if r.is_ok() {
                    st.novo.clear();
                }
                r
            }
            AcaoRamo::Descartar(nome) => {
                let r = p.descartar_ramo(sel, &nome).map(|_| format!("{} '{nome}'", tr("Ramo descartado"))).map_err(|e| e.to_string());
                if r.is_ok() && self.open.ramo.as_deref() == Some(nome.as_str()) {
                    self.open_entry(sel, None);
                }
                r
            }
        };
        match r {
            Ok(m) => self.toast(m, false),
            Err(e) => self.toast(e, true),
        }
    }

    /// Prévia lado a lado no 3D de um conflito (B130): o documento mesclado ficando com a principal e
    /// ficando com o ramo, as duas escolhas só neste conflito e as demais como já estão na janela.
    fn previa_lado_a_lado(&mut self, st: &MesclaUi, feature: Option<u64>, chave: &str, rotulo: &str) {
        let Some(p) = self.project.as_ref() else { return };
        let com = |l: Lado| {
            let mut e = st.escolhas.clone();
            e.insert(chave.to_string(), l);
            p.previa_mesclagem(st.id, &st.nome, &e).map(|m| m.documento)
        };
        match (com(Lado::Nossa), com(Lado::Deles)) {
            (Ok(nossa), Ok(deles)) => {
                crate::camada3d::calcular_lado_a_lado(nossa, deles, feature, rotulo.to_string());
                self.toast(tr("Calculando a prévia lado a lado…"), false);
            }
            (Err(e), _) | (_, Err(e)) => self.toast(e.to_string(), true),
        }
    }

    /// Janela de mesclagem: o que o ramo mudou e, para cada conflito, os dois valores lado a lado.
    pub(super) fn janela_mescla(&mut self, ctx: &egui::Context) {
        let Some(mut st) = ctx.data(|d| d.get_temp::<Option<MesclaUi>>(id_mescla())).flatten() else { return };
        let Some(p) = self.project.as_ref() else { return };
        let previa: Result<Mescla, String> = p.previa_mesclagem(st.id, &st.nome, &st.escolhas).map_err(|e| e.to_string());
        let mudou = p.comparar_ramo(st.id, &st.nome).ok();
        let (mut aberta, mut confirmar) = (true, false);
        let mut ver_no_3d: Option<(Option<u64>, String, String)> = None;
        egui::Window::new(format!("{} '{}'", tr("Mesclar o ramo"), st.nome)).id(id_mescla()).open(&mut aberta).default_pos([420., 120.]).default_width(560.).show(ctx, |ui| {
            if let Some(c) = &mudou {
                ui.label(RichText::new(format!("{}: {}", tr("O ramo mudou"), c.total())).strong());
                for f in c.features.iter().take(12) {
                    ui.label(RichText::new(format!("{} {} ({})", f.frase(), f.nome, f.tipo)).small());
                }
            }
            ui.separator();
            match &previa {
                Err(e) => {
                    ui.label(RichText::new(e).color(ui.visuals().error_fg_color));
                }
                Ok(m) if m.limpa() => {
                    ui.label(RichText::new(tr("Sem conflitos: as mudanças entram sozinhas.")).color(ui.visuals().hyperlink_color));
                }
                Ok(m) => {
                    ui.label(RichText::new(format!("{} ({})", tr("Conflitos"), m.conflitos.len())).strong());
                    egui::Grid::new("grade_conflitos").striped(true).num_columns(5).show(ui, |ui| {
                        ui.label(RichText::new(tr("Onde")).weak());
                        ui.label(RichText::new(tr("Principal")).weak());
                        ui.label(RichText::new(tr("Ramo")).weak());
                        ui.label(RichText::new(tr("Fica com")).weak());
                        ui.label(RichText::new(tr("No 3D")).weak());
                        ui.end_row();
                        for c in &m.conflitos {
                            ui.label(&c.rotulo).on_hover_text(format!("{}: {}", tr("Valor de partida"), c.base));
                            let atual = st.escolhas.get(&c.chave).copied();
                            let marca = |l: Lado| if atual == Some(l) { "● " } else { "" };
                            ui.label(format!("{}{}", marca(Lado::Nossa), c.nossa));
                            ui.label(format!("{}{}", marca(Lado::Deles), c.deles));
                            ui.horizontal(|ui| {
                                if ui.selectable_label(atual == Some(Lado::Nossa), tr("Principal")).clicked() {
                                    st.escolhas.insert(c.chave.clone(), Lado::Nossa);
                                }
                                if ui.selectable_label(atual == Some(Lado::Deles), tr("Ramo")).clicked() {
                                    st.escolhas.insert(c.chave.clone(), Lado::Deles);
                                }
                            });
                            let r = ui.small_button(tr("Ver lado a lado")).on_hover_text(tr("Mostra no 3D o resultado ficando com a principal e ficando com o ramo, um ao lado do outro, com as faces deste conflito em destaque"));
                            if r.clicked() {
                                ver_no_3d = Some((c.feature, c.chave.clone(), c.rotulo.clone()));
                            }
                            ui.end_row();
                        }
                    });
                }
            }
            ui.separator();
            let resolvido = previa.as_ref().is_ok_and(|m| m.conflitos.iter().all(|c| st.escolhas.contains_key(&c.chave)));
            ui.horizontal(|ui| {
                let r = ui.add_enabled(resolvido, egui::Button::new(tr("Mesclar na principal")));
                r.clone().on_hover_text(if resolvido { tr("Recalcula o resultado e só entra se status, volume e referências estiverem em ordem") } else { tr("Escolha um lado em cada conflito") });
                confirmar = r.clicked();
                ui.label(RichText::new(tr("O resultado fica como versão nova; o histórico não é reescrito.")).weak().small());
            });
        });
        if let Some((feature, chave, rotulo)) = ver_no_3d {
            self.previa_lado_a_lado(&st, feature, &chave, &rotulo);
        }
        if confirmar {
            let (id, nome, escolhas) = (st.id, st.nome.clone(), st.escolhas.clone());
            let perfil = self.project.as_ref().unwrap().perfil_qualidade();
            let r = self.project.as_mut().unwrap().mesclar_ramo(id, &nome, &escolhas, &|d: &Document| conferir(d, Some(&perfil)));
            match r {
                Ok(_) => {
                    self.toast(format!("{} '{nome}'", tr("Ramo mesclado na principal")), false);
                    self.open_entry(id, None);
                    aberta = false;
                }
                Err(e) => self.toast(e.to_string(), true),
            }
        }
        ctx.data_mut(|d| d.insert_temp(id_mescla(), if aberta { Some(st) } else { None }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{BoolOp, FeatureKind, OriginPlane, PlaneRef, PrimKind, Primitive};

    fn caixa(d: &mut Document, a: &str) -> fenkra_core::Id {
        let s = |x: &str| x.to_string();
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s(a), b: s("20"), c: s("5"), d: s("0"), op: BoolOp::Add }))
    }

    /// Fluxo inteiro com o gancho real: o ramo que quebra o modelo não é aprovado; o ramo são
    /// é aprovado, mescla e a principal passa a ter o volume novo.
    #[test]
    fn gancho_real_barra_ramo_quebrado_e_deixa_entrar_o_sao() {
        let raiz = std::env::temp_dir().join(format!("plm6_ramos_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        let mut p = fenkra_project::Project::create(&raiz, "Teste").unwrap();
        p.autor = "ana".into();
        let mut d = Document::new("Placa");
        caixa(&mut d, "40");
        let id = p.add_document(d).unwrap();

        let editar = |p: &mut fenkra_project::Project, ramo: &str, largura: &str| {
            let mut doc = p.carregar_ramo(id, ramo).unwrap();
            if let FeatureKind::Primitive(pr) = &mut doc.features[0].kind {
                pr.a = largura.to_string();
            }
            p.salvar_no_ramo(id, ramo, &doc, "largura").unwrap();
        };
        p.criar_ramo(id, "quebrado").unwrap();
        editar(&mut p, "quebrado", "0");
        p.pedir_mesclagem(id, "quebrado", "teste").unwrap();
        let e = p.revisar_pedido(id, "quebrado", true, "", &verificar_recalculo).unwrap_err();
        assert!(e.to_string().contains("volume") || e.to_string().contains("erro"), "{e}");

        p.criar_ramo(id, "largo").unwrap();
        editar(&mut p, "largo", "60");
        p.pedir_mesclagem(id, "largo", "mais largo").unwrap();
        p.revisar_pedido(id, "largo", true, "", &verificar_recalculo).unwrap();
        p.mesclar_ramo(id, "largo", &HashMap::new(), &verificar_recalculo).unwrap();
        let principal = p.load(id).unwrap();
        let vol = fenkra_solidos::recompute(&principal).bodies[0].mass.volume;
        assert!((vol - 60. * 20. * 5.).abs() < 1e-6, "volume da principal depois da mescla: {vol}");
        let _ = std::fs::remove_dir_all(raiz);
    }

    #[test]
    fn verificacao_aceita_documento_sao_e_barra_o_quebrado() {
        let mut d = Document::new("Placa");
        caixa(&mut d, "40");
        assert!(verificar_recalculo(&d).is_empty(), "{:?}", verificar_recalculo(&d));
        // dimensão zero: o kernel não gera sólido válido
        let mut ruim = Document::new("Ruim");
        caixa(&mut ruim, "0");
        let p = verificar_recalculo(&ruim);
        assert!(!p.is_empty(), "caixa de largura zero não pode passar");
    }

    #[test]
    fn checador_de_qualidade_barra_a_aprovacao_no_perfil_rigido() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use fenkra_project::qualidade_modelo::{Gravidade, Perfil, Regra};
        let mut d = Document::new("q");
        caixa(&mut d, "40");
        assert!(conferir(&d, Some(&Perfil::padrao())).is_empty(), "{:?}", conferir(&d, Some(&Perfil::padrao())));
        // perfil que só aceita 2 faces e bloqueia: a caixa (6 faces) é barrada
        let mut duro = Perfil::padrao();
        if let Some(l) = duro.regras.iter_mut().find(|l| l.regra == Regra::Faces) {
            l.limite = 2.;
            l.gravidade = Gravidade::Bloqueia;
        }
        let p = conferir(&d, Some(&duro));
        assert!(p.iter().any(|x| x.contains("checador de qualidade")), "{p:?}");
    }
}
