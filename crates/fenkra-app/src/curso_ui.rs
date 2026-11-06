// SPDX-License-Identifier: LGPL-2.1-or-later
//! Interface do curso (B323), do mapa de macroprocessos (B325) e da pergunta "de onde você vem?"
//! (B324). Um painel livre e encaixável com três abas, mais uma janela única na primeira abertura.
//!
//! Esta camada só desenha e devolve pedidos (`Pedido`): abrir a peça de exemplo, mostrar um comando.
//! Quem muda o documento é o aplicativo, com a mesma confirmação de alterações do resto do programa.
//! Cores só pelo tema; o texto de cada passo está sempre na tela, com ou sem áudio.

use crate::compat::{self, Perfil};
use crate::curso::{self, Aula, Contexto, Trilha};
use crate::i18n::tr;
use crate::mapa_macro::{self, BANCADAS};
use crate::paineis::{self, Painel};
use crate::settings::{Bench, Settings};
use crate::theme;
use eframe::egui::{self, Align2, RichText, Sense, Stroke, Vec2};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Aba {
    #[default]
    Mapa,
    Trilhas,
    Perfil,
}

/// Estado só da tela (nada disso vai para as preferências).
#[derive(Default)]
pub struct CursoUi {
    pub aberto: bool,
    pub aba: Aba,
    /// bancada do mapa; sem escolha, a bancada ativa
    pub bancada: Option<Bench>,
    pub trilha: Option<&'static str>,
    pub aula: Option<&'static str>,
    pub perfil_sel: Option<usize>,
    /// a pergunta "de onde você vem?" está na tela (primeira abertura ou menu Ajuda)
    pub pergunta: bool,
    /// instante em que a aula aberta apareceu, para a mídia animada
    t_aula: f64,
    /// capturas de desenvolvimento (`FENKRA_DEMO*`): a pergunta inicial não aparece sozinha
    pub sem_auto: bool,
    /// ícones dos comandos acionados nesta sessão (passos que não deixam rastro no modelo)
    pub usados: Vec<&'static str>,
}

impl CursoUi {
    /// Anota que o comando foi acionado; o passo "Comando" da aula em andamento passa a valer.
    pub fn usou(&mut self, icone: &'static str) {
        if !self.usados.contains(&icone) {
            self.usados.push(icone);
        }
    }
}

// ------------------------------------------------------------------ destaque na barra (B297)

/// O comando que o passo atual pede: a barra desenha um anel pulsante sobre o botão dele.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Destaque {
    pub icone: &'static str,
    /// pulsar (desligado com animações paradas ou "reduzir movimento")
    pub animar: bool,
}

fn id_destaque() -> egui::Id {
    egui::Id::new("curso_destaque_do_passo")
}

/// Destaque do quadro anterior, se a aula aberta pede um comando.
pub fn destaque(ctx: &egui::Context) -> Option<Destaque> {
    ctx.data(|d| d.get_temp::<Destaque>(id_destaque()))
}

/// Anel de destaque ao redor do botão. A cor vem do tema; a pulsação some com animações paradas.
pub fn pintar_destaque(ui: &egui::Ui, rect: egui::Rect, d: Destaque) {
    let t = ui.ctx().input(|i| i.time) as f32;
    let pulso = if d.animar { 0.5 + 0.5 * (t * 3.).sin() } else { 1. };
    let cor = theme::accent(ui.visuals());
    let cor = cor.gamma_multiply((110. + 145. * pulso) / 255.);
    ui.painter().rect_stroke(rect.expand(1. + 1.5 * pulso), 7., Stroke::new(2.5, cor), egui::StrokeKind::Outside);
    if d.animar {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
    }
}

/// Para botões de família: o comando pedido pode estar numa variação que não é a mostrada.
pub fn destaque_em_familia(ui: &egui::Ui, rect: egui::Rect, icones: &[&str], mostrado: &str) {
    if let Some(d) = destaque(ui.ctx()) {
        if d.icone != mostrado && icones.contains(&d.icone) {
            pintar_destaque(ui, rect, d);
        }
    }
}

/// O que a tela pede ao aplicativo.
#[derive(Debug, PartialEq)]
pub enum Pedido {
    /// abrir a peça de exemplo (com a pergunta de alterações não salvas)
    Exemplo(&'static str),
    /// mostrar o comando da barra pelo ícone
    Comando(&'static str),
    /// mudar a bancada ativa
    Bancada(Bench),
}

#[derive(Default)]
pub struct Saida {
    pub pedidos: Vec<Pedido>,
    /// as preferências mudaram e precisam ser gravadas
    pub mudou: bool,
}

impl CursoUi {
    /// Estado inicial. Com variáveis `FENKRA_DEMO*` a pergunta da primeira abertura não aparece
    /// sozinha; `FENKRA_DEMO_CURSO` abre o que se quer ver: `mapa`, `aulas`, `perfil`, `pergunta` ou
    /// `aula:<id>`, e `FENKRA_DEMO_CURSO_BANCADA` (solidos, chapa, ...) escolhe a bancada do mapa.
    pub fn do_ambiente() -> Self {
        let mut st = CursoUi { sem_auto: std::env::vars().any(|(k, _)| k.starts_with("FENKRA_DEMO")), ..Default::default() };
        if let Ok(v) = std::env::var("FENKRA_DEMO_CURSO") {
            st.aberto = true;
            match v.as_str() {
                "aulas" => st.aba = Aba::Trilhas,
                "perfil" => st.aba = Aba::Perfil,
                "pergunta" => st.pergunta = true,
                outro => {
                    if let Some(a) = outro.strip_prefix("aula:").and_then(curso::aula) {
                        st.aba = Aba::Trilhas;
                        st.aula = Some(a.id);
                    }
                }
            }
            st.bancada = std::env::var("FENKRA_DEMO_CURSO_BANCADA").ok().and_then(|n| {
                BANCADAS.into_iter().find(|b| format!("{b:?}").to_lowercase() == n.to_lowercase())
            });
        }
        st
    }

    /// Abre o painel já numa aula (vindo do mapa, da pergunta inicial ou do menu).
    pub fn abrir_aula(&mut self, a: &'static Aula, s: &mut Settings) {
        self.aberto = true;
        self.aba = Aba::Trilhas;
        self.aula = Some(a.id);
        self.trilha = None;
        s.curso.atual = Some(a.id.to_string());
    }

    pub fn abrir_mapa(&mut self, b: Bench) {
        self.aberto = true;
        self.aba = Aba::Mapa;
        self.bancada = Some(b);
    }

    pub fn abrir_perfil(&mut self) {
        self.aberto = true;
        self.aba = Aba::Perfil;
    }
}

// ------------------------------------------------------------------ peças pequenas

fn imagem(nome: &str, lado: f32) -> egui::Image<'static> {
    egui::Image::new(crate::icons::get(nome)).fit_to_exact_size(Vec2::splat(lado))
}

/// Botão de ícone de um comando, com a dica completa (nome, frase e onde fica). Clicar o mostra.
fn botao_comando(ui: &mut egui::Ui, icone: &'static str, out: &mut Saida) {
    let Some(c) = compat::comando(icone) else { return };
    let r = ui.add(egui::Button::image(imagem(icone, 22.)).min_size(Vec2::splat(30.)));
    let onde = compat::onde_fica(icone).unwrap_or_default();
    let r = r.on_hover_ui(|ui| {
        ui.label(RichText::new(tr(c.name)).strong());
        ui.label(tr(c.brief));
        if !onde.is_empty() {
            ui.label(RichText::new(format!("{}: {onde}", tr("Fica em"))).weak().small());
        }
        ui.label(RichText::new(tr("Clique para usar o comando")).weak().small());
    });
    if r.clicked() {
        out.pedidos.push(Pedido::Comando(icone));
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Marca {
    Feito,
    Pulado,
    Atual,
    Pendente,
}

/// Marca de estado do passo, desenhada com formas para não depender de glifos da fonte.
fn marca(ui: &mut egui::Ui, m: Marca) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.), Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    let acento = theme::accent(ui.visuals());
    let fraco = ui.visuals().weak_text_color();
    match m {
        Marca::Feito => {
            p.circle_filled(c, 7., acento);
            let fundo = ui.visuals().window_fill;
            p.line(vec![c + egui::vec2(-3.5, 0.), c + egui::vec2(-1., 3.), c + egui::vec2(3.5, -2.5)], Stroke::new(1.8, fundo));
        }
        Marca::Pulado => {
            p.circle_stroke(c, 7., Stroke::new(1.2, fraco));
            p.line(vec![c + egui::vec2(-3.5, 0.), c + egui::vec2(3.5, 0.)], Stroke::new(1.2, fraco));
        }
        Marca::Atual => {
            p.circle_stroke(c, 7., Stroke::new(2., acento));
            p.circle_filled(c, 3., acento);
        }
        Marca::Pendente => {
            p.circle_stroke(c, 7., Stroke::new(1.2, fraco));
        }
    }
}

fn barra(ui: &mut egui::Ui, feitos: usize, total: usize) {
    let f = if total == 0 { 0. } else { feitos as f32 / total as f32 };
    ui.add(egui::ProgressBar::new(f).desired_width(120.).text(format!("{feitos}/{total}")));
}

// ------------------------------------------------------------------ painel

/// Desenha o painel do curso, quando aberto. Chame uma vez por quadro, depois da viewport.
pub fn painel(ctx: &egui::Context, area: egui::Rect, st: &mut CursoUi, s: &mut Settings, cx: &Contexto) -> Saida {
    let mut out = Saida::default();
    // o destaque do passo vale só enquanto a aula está à vista; `aula_aberta` o põe de novo
    ctx.data_mut(|d| d.remove::<Destaque>(id_destaque()));
    if !st.aberto {
        return out;
    }
    // o documento diz quais passos da aula em andamento já foram cumpridos
    // (a aula aberta na tela e a que ficou em andamento; nunca mais que duas por quadro)
    let em_andamento = s.curso.atual.clone();
    for id in [em_andamento.as_deref(), st.aula].into_iter().flatten() {
        if let Some(a) = curso::aula(id) {
            if s.curso.sincronizar(a, cx) {
                out.mudou = true;
            }
        }
    }
    const LARGURA: f32 = 380.;
    let posicao = s.painel("curso", paineis::Encaixe::Livre);
    let cfg = Painel { nome: "curso".into(), titulo: tr("Curso").to_string(), padrao: paineis::ao_lado_da_arvore(area), largura: LARGURA, redimensiona: true };
    let mut fechar = false;
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        ui.horizontal(|ui| {
            ui.add(imagem("curso", 20.));
            for (aba, rotulo) in [(Aba::Mapa, "Mapa"), (Aba::Trilhas, "Aulas"), (Aba::Perfil, "De onde você vem?")] {
                if ui.selectable_label(st.aba == aba, tr(rotulo)).clicked() {
                    st.aba = aba;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✕").on_hover_text(tr("Fechar o curso; o progresso fica salvo")).clicked() {
                    fechar = true;
                }
            });
        });
        ui.separator();
        egui::ScrollArea::vertical().auto_shrink([false, true]).max_height(520.).show(ui, |ui| match st.aba {
            Aba::Mapa => aba_mapa(ui, st, s, &mut out),
            Aba::Trilhas => aba_trilhas(ui, st, s, &mut out),
            Aba::Perfil => aba_perfil(ui, st, s, &mut out, false),
        });
    });
    if let Some(p) = saida.movido {
        s.guardar_painel("curso", p);
    }
    if fechar {
        st.aberto = false;
    }
    out
}

// ------------------------------------------------------------------ mapa (B325)

fn aba_mapa(ui: &mut egui::Ui, st: &mut CursoUi, s: &mut Settings, out: &mut Saida) {
    let atual = st.bancada.unwrap_or(s.bench);
    ui.horizontal_wrapped(|ui| {
        for b in BANCADAS {
            if ui.selectable_label(atual == b, tr(mapa_macro::nome_da_bancada(b))).clicked() {
                st.bancada = Some(b);
            }
        }
    });
    let m = mapa_macro::mapa(atual);
    ui.add_space(4.);
    ui.label(RichText::new(tr(m.resumo)).weak());
    ui.add_space(6.);
    for (n, et) in m.etapas.iter().enumerate() {
        let a = et.aula();
        let (feitos, total) = s.curso.andamento(a);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{}", n + 1)).strong().color(theme::accent(ui.visuals())).size(18.));
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr(et.titulo())).strong());
                    ui.label(tr(et.pode));
                });
            });
            ui.horizontal_wrapped(|ui| {
                for c in et.comandos {
                    botao_comando(ui, c, out);
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr("Abrir a aula")).on_hover_text(tr("Passo a passo no modelo real; dá para pular a qualquer momento")).clicked() {
                    st.abrir_aula(a, s);
                    out.mudou = true;
                }
                if let Some(ex) = et.exemplo() {
                    if ui.button(format!("{}: {}", tr("Abrir o exemplo"), tr(ex))).on_hover_text(tr("Abre uma peça pronta para você olhar e mexer")).clicked() {
                        out.pedidos.push(Pedido::Exemplo(ex));
                    }
                }
                if total > 0 && (feitos > 0 || s.curso.prog(a.id).aula_pulada) {
                    barra(ui, feitos, total);
                }
            });
        });
    }
    if atual != s.bench && ui.button(tr("Ir para esta bancada")).clicked() {
        out.pedidos.push(Pedido::Bancada(atual));
    }
}

// ------------------------------------------------------------------ aulas (B323)

fn aba_trilhas(ui: &mut egui::Ui, st: &mut CursoUi, s: &mut Settings, out: &mut Saida) {
    if let Some(a) = st.aula.and_then(curso::aula) {
        aula_aberta(ui, a, st, s, out);
        return;
    }
    if let Some(t) = st.trilha.and_then(curso::trilha) {
        lista_de_aulas(ui, t, st, s, out);
        return;
    }
    if let Some(a) = s.curso.atual.as_deref().and_then(curso::aula).filter(|a| !s.curso.resolvida(a)) {
        if ui.button(format!("{}: {}", tr("Continuar"), tr(a.titulo))).on_hover_text(tr("Volta ao ponto em que você parou")).clicked() {
            st.abrir_aula(a, s);
        }
        ui.add_space(6.);
    }
    let sugerida = compat::escolhido(s).map(|p| p.trilha);
    ui.label(RichText::new(tr("Trilhas")).strong());
    for t in curso::TRILHAS {
        let (feitas, total) = s.curso.andamento_trilha(t);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr(t.titulo)).strong());
                ui.label(RichText::new(tr(t.nivel.texto())).weak().small());
                if sugerida == Some(t.id) {
                    ui.label(RichText::new(tr("seu perfil")).small().color(theme::accent(ui.visuals())));
                }
            });
            ui.label(tr(t.resumo));
            ui.horizontal(|ui| {
                if ui.button(tr("Ver as aulas")).clicked() {
                    st.trilha = Some(t.id);
                }
                barra(ui, feitas, total);
            });
        });
    }
}

fn lista_de_aulas(ui: &mut egui::Ui, t: &'static Trilha, st: &mut CursoUi, s: &mut Settings, _out: &mut Saida) {
    if ui.button(format!("← {}", tr("Trilhas"))).clicked() {
        st.trilha = None;
    }
    ui.label(RichText::new(tr(t.titulo)).heading());
    ui.label(RichText::new(tr(t.resumo)).weak());
    ui.add_space(4.);
    for a in curso::aulas_da_trilha(t) {
        let (feitos, total) = s.curso.andamento(a);
        let estado = if s.curso.concluida(a) {
            Marca::Feito
        } else if s.curso.prog(a.id).aula_pulada || (s.curso.resolvida(a) && feitos < total) {
            Marca::Pulado
        } else if s.curso.atual.as_deref() == Some(a.id) {
            Marca::Atual
        } else {
            Marca::Pendente
        };
        ui.horizontal(|ui| {
            marca(ui, estado);
            if ui.link(tr(a.titulo)).clicked() {
                st.abrir_aula(a, s);
            }
            ui.label(RichText::new(format!("{feitos}/{total}")).weak().small());
        });
    }
}

fn aula_aberta(ui: &mut egui::Ui, a: &'static Aula, st: &mut CursoUi, s: &mut Settings, out: &mut Saida) {
    let agora = ui.ctx().input(|i| i.time);
    if st.t_aula == 0. {
        st.t_aula = agora;
    }
    ui.horizontal(|ui| {
        if ui.button(format!("← {}", tr("Aulas"))).clicked() {
            st.aula = None;
            st.t_aula = 0.;
        }
        ui.label(RichText::new(tr(a.nivel.texto())).weak().small());
    });
    ui.label(RichText::new(tr(a.titulo)).heading());
    ui.label(tr(a.resumo));
    ui.add_space(4.);
    // o GIF do comando (B257), quando o pacote de mídia o tem; sem ele a aula segue só com texto
    if let Some(m) = a.midia.filter(|m| crate::midia::exibir::tem(m)) {
        let parado = !s.animacoes || s.acessibilidade.reduzir_movimento;
        let t = if parado { 0. } else { agora - st.t_aula };
        crate::midia::exibir::desenhar(ui, m, t, 300.);
        if !parado {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(80));
        }
        ui.add_space(4.);
    }
    if let Some(ex) = a.exemplo {
        if ui.button(format!("{}: {}", tr("Abrir a peça de exemplo"), tr(ex))).on_hover_text(tr("Abre uma peça pronta; se houver alterações, o programa pergunta antes")).clicked() {
            out.pedidos.push(Pedido::Exemplo(ex));
        }
        ui.add_space(4.);
    }
    let pr = s.curso.prog(a.id);
    let proximo = s.curso.proximo_passo(a);
    let pulada = pr.aula_pulada;
    ui.separator();
    for (i, p) in a.passos.iter().enumerate() {
        let m = if pr.passo_feito(i) {
            Marca::Feito
        } else if pr.passo_pulado(i) || pulada {
            Marca::Pulado
        } else if proximo == Some(i) {
            Marca::Atual
        } else {
            Marca::Pendente
        };
        ui.horizontal_top(|ui| {
            marca(ui, m);
            let mut t = RichText::new(tr(p.texto));
            if m == Marca::Atual {
                t = t.strong();
            } else if m != Marca::Pendente {
                t = t.weak();
            }
            ui.label(t);
        });
        if m == Marca::Atual {
            if !p.icone.is_empty() && p.icone != "exemplo" {
                let animar = s.animacoes && !s.acessibilidade.reduzir_movimento;
                ui.ctx().data_mut(|d| d.insert_temp(id_destaque(), Destaque { icone: p.icone, animar }));
            }
            ui.horizontal_wrapped(|ui| {
                ui.add_space(22.);
                if !p.icone.is_empty() && p.icone != "exemplo" {
                    botao_comando(ui, p.icone, out);
                    ui.label(RichText::new(tr("Mostre-me")).weak().small());
                    ui.label(RichText::new(tr("O botão pulsa na barra")).weak().small());
                }
                if p.cond == curso::Cond::Manual && ui.button(tr("Já fiz")).on_hover_text(tr("Este passo não deixa rastro no modelo; confirme quando terminar")).clicked() {
                    s.curso.confirmar(a, i);
                    out.mudou = true;
                }
                if ui.small_button(tr("Pular o passo")).clicked() {
                    s.curso.pular_passo(a, i);
                    out.mudou = true;
                }
            });
        }
    }
    ui.separator();
    let (feitos, total) = s.curso.andamento(a);
    ui.horizontal(|ui| {
        barra(ui, feitos, total);
        if s.curso.concluida(a) {
            ui.label(RichText::new(tr("Aula concluída")).strong().color(theme::accent(ui.visuals())));
        } else if pulada {
            ui.label(RichText::new(tr("Aula pulada")).weak());
        }
    });
    ui.horizontal_wrapped(|ui| {
        if !pulada && !s.curso.resolvida(a) && ui.button(tr("Pular a aula")).on_hover_text(tr("Nada se perde: você pode retomar quando quiser")).clicked() {
            s.curso.pular_aula(a);
            out.mudou = true;
        }
        if (pulada || pr.puladas != 0) && ui.button(tr("Retomar")).on_hover_text(tr("Volta aos passos pulados e mantém o que já foi feito")).clicked() {
            s.curso.retomar(a);
            out.mudou = true;
        }
        if feitos > 0 && ui.button(tr("Refazer")).clicked() {
            s.curso.refazer(a);
            out.mudou = true;
        }
        if s.curso.resolvida(a) {
            let t = st.trilha.and_then(curso::trilha).or_else(|| curso::TRILHAS.iter().find(|t| t.aulas.contains(&a.id)));
            if let Some(prox) = t.and_then(|t| s.curso.proxima_depois(t, a.id)) {
                if ui.button(format!("{} →", tr(prox.titulo))).on_hover_text(tr("Próxima aula da trilha")).clicked() {
                    st.abrir_aula(prox, s);
                    st.t_aula = 0.;
                    out.mudou = true;
                }
            }
        }
    });
    // narração: opcional e só quando existe voz instalada; o texto acima acompanha sempre
    let voz = crate::midia::exibir::tem(&curso::id_voz(a.id));
    ui.add_space(4.);
    let mut audio = s.curso.audio && voz;
    let r = ui.add_enabled(voz, egui::Checkbox::new(&mut audio, tr("Narração em áudio")));
    r.on_disabled_hover_text(tr("Nenhuma voz instalada. O curso funciona igual, só com o texto de cada passo."));
    if voz && audio != s.curso.audio {
        s.curso.audio = audio;
        out.mudou = true;
    }
}

// ------------------------------------------------------------------ de onde você vem? (B324)

fn aba_perfil(ui: &mut egui::Ui, st: &mut CursoUi, s: &mut Settings, out: &mut Saida, modal: bool) {
    let atual = compat::escolhido(s);
    let sel = st.perfil_sel.or_else(|| atual.and_then(|p| compat::PERFIS.iter().position(|x| x.id == p.id))).unwrap_or(0);
    ui.label(tr("Como você trabalha hoje? Mostramos o fluxo lado a lado e ajustamos atalhos e navegação. Dá para trocar depois em Ajuda."));
    if let Some(p) = atual {
        ui.label(RichText::new(format!("{}: {}", tr("Perfil atual"), tr(p.nome))).weak());
    }
    ui.add_space(4.);
    egui::ScrollArea::vertical().id_salt("perfis").max_height(if modal { 140. } else { 170. }).show(ui, |ui| {
        for (i, p) in compat::PERFIS.iter().enumerate() {
            let r = ui.selectable_label(sel == i, RichText::new(tr(p.nome)).strong());
            if r.clicked() {
                st.perfil_sel = Some(i);
            }
            r.on_hover_text(tr(p.para_quem));
        }
    });
    let p = &compat::PERFIS[sel];
    ui.label(RichText::new(tr(p.para_quem)).weak());
    ui.add_space(6.);
    comparacao(ui, p, s);
    ui.add_space(6.);
    ui.label(RichText::new(format!("{}: {}", tr("Atalhos"), p.preset.label())).small());
    ui.label(RichText::new(p.preset.nav().help()).small().weak());
    ui.add_space(6.);
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Aplicar e começar o curso")).on_hover_text(tr("Ajusta atalhos e navegação e abre a trilha indicada para este perfil")).clicked() {
            compat::aplicar(s, p);
            out.pedidos.push(Pedido::Bancada(p.bancada()));
            st.aberto = true;
            st.aba = Aba::Trilhas;
            st.trilha = Some(p.trilha);
            st.aula = None;
            st.pergunta = false;
            out.mudou = true;
        }
        if ui.button(tr("Só aplicar")).on_hover_text(tr("Ajusta atalhos e navegação, sem abrir o curso")).clicked() {
            compat::aplicar(s, p);
            st.pergunta = false;
            out.mudou = true;
        }
        if modal && ui.button(tr("Agora não")).on_hover_text(tr("Nada muda; a pergunta fica em Ajuda")).clicked() {
            compat::dispensar(s);
            st.pergunta = false;
            out.mudou = true;
        }
    });
}

/// Dois fluxos lado a lado: o que você faz e o comando daqui, com onde fica e o que muda. Uma
/// linha de cada vez recebe o destaque; com animações desligadas, todas ficam paradas.
fn comparacao(ui: &mut egui::Ui, p: &Perfil, s: &Settings) {
    let animar = s.animacoes && !s.acessibilidade.reduzir_movimento;
    let t = ui.ctx().input(|i| i.time);
    let ativa = if animar { Some(((t / 1.6) as usize) % p.equivalentes.len()) } else { None };
    if animar {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
    }
    let acento = theme::accent(ui.visuals());
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Seu fluxo")).strong());
        ui.add_space(ui.available_width() * 0.42);
        ui.label(RichText::new(tr("No Fenkra")).strong());
    });
    for (i, e) in p.equivalentes.iter().enumerate() {
        let destaque = ativa == Some(i);
        let borda = if destaque { Stroke::new(2., acento) } else { Stroke::new(1., ui.visuals().widgets.noninteractive.bg_stroke.color) };
        egui::Frame::new().stroke(borda).corner_radius(6.).inner_margin(6.).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                let metade = (ui.available_width() - 24.) / 2.;
                ui.allocate_ui_with_layout(Vec2::new(metade, 0.), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(metade);
                    ui.label(RichText::new(tr(e.seu)).strong());
                });
                // seta entre os dois lados
                let (r, _) = ui.allocate_exact_size(Vec2::new(16., 16.), Sense::hover());
                let c = r.center();
                let cor = if destaque { acento } else { ui.visuals().weak_text_color() };
                ui.painter().line(vec![c + egui::vec2(-6., 0.), c + egui::vec2(6., 0.)], Stroke::new(1.6, cor));
                ui.painter().line(vec![c + egui::vec2(2., -4.), c + egui::vec2(6., 0.), c + egui::vec2(2., 4.)], Stroke::new(1.6, cor));
                ui.vertical(|ui| {
                    if let Some(c) = compat::comando(e.comando) {
                        ui.horizontal(|ui| {
                            ui.add(imagem(e.comando, 20.));
                            ui.label(RichText::new(tr(c.name)).strong());
                        });
                    }
                    if let Some(onde) = compat::onde_fica(e.comando) {
                        ui.label(RichText::new(onde).weak().small());
                    }
                    ui.label(tr(e.muda));
                });
            });
        });
        ui.add_space(2.);
    }
}

/// Janela única da primeira abertura (e de Ajuda > "De onde você vem?"). Responder ou dispensar
/// encerra a pergunta; nada é aplicado sem um clique.
pub fn pergunta_inicial(ctx: &egui::Context, st: &mut CursoUi, s: &mut Settings) -> Saida {
    let mut out = Saida::default();
    if !st.pergunta {
        return out;
    }
    egui::Window::new(RichText::new(tr("De onde você vem?")).strong()).id(egui::Id::unique("curso_pergunta")).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0., 0.]).default_width(520.).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.add(imagem("perfil_fluxo", 28.));
            ui.vertical(|ui| {
                ui.label(RichText::new(tr("Escolha o jeito de trabalhar mais parecido com o seu")).strong());
            });
        });
        ui.add_space(4.);
        aba_perfil(ui, st, s, &mut out, true);
    });
    out
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Todo texto que a tela passa por `tr("…")` tem versão em inglês.
    #[test]
    fn todo_texto_da_tela_tem_traducao() {
        let fonte = include_str!("curso_ui.rs").split("#[cfg(test)]").next().unwrap();
        let mut faltam = vec![];
        for trecho in fonte.split("tr(\"").skip(1) {
            if let Some(fim) = trecho.find("\")") {
                let t = &trecho[..fim];
                if !t.contains("\\") && !crate::i18n::tem_traducao(t) {
                    faltam.push(t.to_string());
                }
            }
        }
        for r in ["Mapa", "Aulas", "De onde você vem?", "Curso…", "Mapa de macroprocessos…", "De onde você vem?…"] {
            if !crate::i18n::tem_traducao(r) {
                faltam.push(r.into());
            }
        }
        assert!(faltam.is_empty(), "sem tradução: {faltam:#?}");
    }

    #[test]
    fn abrir_aula_marca_a_aula_atual_e_a_aba() {
        let mut st = CursoUi::default();
        let mut s = Settings::default();
        let a = curso::aula("ch_abas").unwrap();
        st.abrir_aula(a, &mut s);
        assert!(st.aberto && st.aba == Aba::Trilhas && st.aula == Some("ch_abas"));
        assert_eq!(s.curso.atual.as_deref(), Some("ch_abas"));
    }

    #[test]
    fn o_painel_e_a_pergunta_desenham_sem_travar_e_sem_pedir_nada_sozinhos() {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let mut st = CursoUi { aberto: true, pergunta: true, ..Default::default() };
        let mut s = Settings::default();
        let doc = fenkra_core::Document::new("Peça");
        let cx = Contexto::so_documento(&doc);
        let area = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 800.));
        for aba in [Aba::Mapa, Aba::Trilhas, Aba::Perfil] {
            st.aba = aba;
            for b in BANCADAS {
                st.bancada = Some(b);
                let mut pedidos = 0;
                let mut saida = ctx.run_ui(egui::RawInput { screen_rect: Some(area), ..Default::default() }, |ui| {
                    let o = painel(ui.ctx(), area, &mut st, &mut s, &cx);
                    pedidos += o.pedidos.len();
                    let o = pergunta_inicial(ui.ctx(), &mut st, &mut s);
                    pedidos += o.pedidos.len();
                });
                saida.textures_delta.clear();
                assert_eq!(pedidos, 0, "{aba:?}/{b:?}: o desenho não pode pedir nada sozinho");
            }
        }
        // aula aberta
        st.abrir_aula(curso::aula("sol_volume").unwrap(), &mut s);
        let mut saida = ctx.run_ui(egui::RawInput { screen_rect: Some(area), ..Default::default() }, |ui| {
            painel(ui.ctx(), area, &mut st, &mut s, &cx);
        });
        saida.textures_delta.clear();
        assert!(!s.curso.perfil_visto, "desenhar não responde a pergunta");
    }
}
