// SPDX-License-Identifier: LGPL-2.1-or-later
//! Tela inicial (B455): visual limpo, hierarquia por tipografia e espaço, movimento leve.
//!
//! Como está montada:
//! - Duas colunas. À esquerda, a marca pequena, a lista "Começar" e os recentes, em linhas de texto
//!   com ícone; à direita, os exemplos em miniaturas. Sem cartão com sombra nem selo colorido: o que
//!   separa as partes é o espaço e o rótulo pequeno de cada seção.
//! - Cores só do tema, pela `Paleta` de `inicio_tema.rs`. Este arquivo não tem nenhuma cor fixa
//!   (`padrao.rs` confere).
//! - Movimento: ao passar o mouse a linha acende, o texto anda 4 px e um traço de destaque se
//!   desenha; na miniatura, o contorno se traça em volta e a peça gira. Ao apertar, o item afunda
//!   1 px e o traço fica cheio. No fundo, uma grade de prancheta bem fraca desliza devagar.
//! - Barato: tudo é linha e retângulo do pintor do egui, sem textura nem shader. A grade são cerca
//!   de cem segmentos. A tela só pede quadro enquanto anima: transição em curso pede o próximo na
//!   hora; a grade pede a cada `QUADRO_MS`; desligada, não pede nenhum.
//! - Desliga sozinha com a opção "Animações da tela inicial" desligada, no modo leve, com
//!   "reduzir movimento" (do Fenkra ou do sistema) e no adaptador gráfico por software.

use std::time::Duration;

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke};

use crate::i18n::tr;
use crate::inicio_tema::Paleta;
use crate::theme::com_alfa;

/// Duração de cada realce (segundos): curta, para a resposta parecer imediata.
pub const TRANSICAO: f32 = 0.16;
/// Intervalo entre quadros da grade do fundo: 30 por segundo bastam para uma deriva tão lenta.
pub const QUADRO_MS: u64 = crate::inicio_tema::QUADRO_MS;
/// Passo da grade fina do fundo (px) e de quantas em quantas linhas vem uma mais marcada.
pub const PASSO_GRADE: f32 = 24.;
pub const GRADE_MAIOR: usize = 5;
/// Velocidade da deriva da grade (px por segundo).
pub const DERIVA: f32 = 3.;

/// O que decide se a tela inicial anima.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Contexto {
    /// opção "Animações da tela inicial"
    pub opcao: bool,
    pub modo_leve: bool,
    /// "reduzir movimento" das configurações do Fenkra
    pub reduzir_movimento: bool,
    /// o sistema pediu menos animação (Windows: efeitos de animação desligados)
    pub sistema_reduz: bool,
    /// adaptador gráfico por software
    pub software: bool,
}

/// A tela inicial anima só com a opção ligada e nenhum motivo para poupar movimento.
pub fn animacao_ligada(c: Contexto) -> bool {
    c.opcao && !c.modo_leve && !c.reduzir_movimento && !c.sistema_reduz && !c.software
}

/// A miniatura gira com o mouse em cima (B508)? É resposta direta ao gesto, não movimento de enfeite:
/// vale sempre que a pessoa não desligou as animações da tela nem pediu "reduzir movimento" no Fenkra.
/// O pedido do sistema, o modo leve e o adaptador por software param só a grade do fundo e as
/// transições; antes paravam o giro também, e numa máquina com os efeitos de animação do Windows
/// desligados as miniaturas viravam foto parada (regressão da alpha.37, B455).
pub fn gira_com_mouse(c: Contexto) -> bool {
    c.opcao && !c.reduzir_movimento
}

/// Contexto lido das configurações, do hardware medido na partida e do sistema.
pub fn contexto_de(s: &crate::settings::Settings) -> Contexto {
    Contexto {
        opcao: s.animar_inicio,
        modo_leve: s.modo_leve,
        reduzir_movimento: s.acessibilidade.reduzir_movimento,
        // desenvolvimento: FENKRA_DEMO_ANIMAR=1 ignora o pedido do sistema, para conferir o movimento
        // numa máquina com os efeitos de animação do Windows desligados
        sistema_reduz: sistema_reduz_movimento() && std::env::var_os("FENKRA_DEMO_ANIMAR").is_none(),
        software: crate::perfil::medido().is_some_and(|h| h.software),
    }
}

/// Opção "Animações da tela inicial" na página Geral das configurações, com a dica e o aviso de
/// quando ela está marcada mas desligada por outro motivo.
pub fn opcao_ui(ui: &mut egui::Ui, s: &mut crate::settings::Settings) {
    ui.checkbox(&mut s.animar_inicio, tr("Animações da tela inicial")).on_hover_text(tr(DICA_OPCAO));
    if s.animar_inicio && !animacao_ligada(contexto_de(s)) {
        ui.label(egui::RichText::new(tr("desligada agora: modo leve, adaptador por software ou reduzir movimento")).weak().small());
    }
}

/// Dica da opção (também o texto de apoio).
pub const DICA_OPCAO: &str = "Realce suave ao passar o mouse e grade do fundo deslizando devagar. Fica desligada no modo leve, no adaptador gráfico por software e com \"reduzir movimento\" (do Fenkra ou do sistema).";

/// Próximo quadro que a tela inicial pede. `None`: nenhum (a tela fica parada até um evento).
/// Transição de realce em curso pede o próximo já; a deriva do fundo, a cada `QUADRO_MS`. Janela
/// sem foco ou minimizada não pede nada.
pub fn proximo_quadro(ligada: bool, visivel: bool, transicao: bool) -> Option<Duration> {
    if !ligada || !visivel {
        None
    } else if transicao {
        Some(Duration::ZERO)
    } else {
        Some(Duration::from_millis(QUADRO_MS))
    }
}

/// Duração das transições: zero com a animação desligada (o realce troca de uma vez, sem quadro extra).
pub fn tempo_transicao(ligada: bool) -> f32 {
    if ligada {
        TRANSICAO
    } else {
        0.
    }
}

/// O sistema pediu menos movimento? No Windows, lê "efeitos de animação" (SPI_GETCLIENTAREAANIMATION)
/// e guarda por 5 s, para mudar no sistema valer sem reiniciar e sem consultar a cada quadro.
pub fn sistema_reduz_movimento() -> bool {
    use std::sync::Mutex;
    use std::time::Instant;
    static CACHE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((quando, v)) = *c {
        if quando.elapsed() < Duration::from_secs(5) {
            return v;
        }
    }
    let v = consultar_sistema();
    *c = Some((Instant::now(), v));
    v
}

#[cfg(windows)]
fn consultar_sistema() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION};
    let mut ligado: windows_sys::core::BOOL = 1;
    let ok = unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut ligado as *mut _ as *mut _, 0) };
    ok != 0 && ligado == 0
}

#[cfg(not(windows))]
fn consultar_sistema() -> bool {
    false
}

/// O que a pessoa escolheu na tela inicial.
#[derive(Clone, Debug, PartialEq)]
pub enum Escolha {
    Voltar,
    /// código da ação de começar (0 novo projeto, 1 abrir projeto, 2 nova peça, 3 abrir peça,
    /// 5 nova montagem, 6 novo desenho)
    Comecar(u8),
    Projeto(String),
    Arquivo(String),
    Exemplo(usize),
}

/// Ações da coluna "Começar": código, ícone, nome e dica.
pub const COMECAR: [(u8, &str, &str, &str); 6] = [
    (2, "nova_peca", "Nova peça", "Peça avulsa, fora de projeto"),
    (3, "abrir_projeto", "Abrir peça", "Abre um arquivo .fkp"),
    (5, "asm_nova", "Nova montagem", "Começa uma montagem vazia (arquivo .fka)."),
    (6, "des_novo", "Novo desenho", "Folha de detalhamento 2D (arquivo .fkd)"),
    (0, "novo_projeto", "Novo projeto", "Cria uma pasta de projeto com versões e revisões"),
    (1, "abrir_projeto", "Abrir projeto", "Abre uma pasta de projeto existente"),
];

/// Estado e dados que a tela inicial lê do aplicativo.
pub struct Entrada<'a> {
    pub pal: Paleta,
    pub ligada: bool,
    /// a miniatura gira com o mouse em cima ([`gira_com_mouse`])
    pub gira: bool,
    /// segundos desde que a tela abriu
    pub t: f32,
    pub dt: f32,
    pub voltar: bool,
    pub arquivos: &'a [String],
    pub projetos: &'a [String],
    pub mostrar_ao_iniciar: &'a mut bool,
    pub previews: &'a crate::examples::PreviewCache,
    pub giro: &'a std::cell::RefCell<std::collections::HashMap<String, f32>>,
    pub exemplos: &'a [(&'static str, &'static str, crate::settings::Bench)],
}

/// Desenha a tela inicial inteira e devolve a escolha da pessoa, se houver.
pub fn mostrar(root: &mut egui::Ui, e: Entrada) -> Option<Escolha> {
    let mut escolha = None;
    let mut transicao = false;
    // B508: alguma miniatura girando neste quadro (pede o próximo mesmo com a animação desligada)
    let girando = std::cell::Cell::new(false);
    let pal = e.pal;
    let tt = tempo_transicao(e.ligada);
    egui::CentralPanel::default().frame(egui::Frame::NONE).show(root, |ui| {
        let tela = ui.max_rect();
        fundo(ui.painter(), tela, &pal, if e.ligada { e.t } else { 0. });
        // entrada: só um esmaecer curto, e só com animação
        let k_in = if e.ligada { (e.t / 0.35).clamp(0., 1.) } else { 1. };
        if k_in < 1. {
            transicao = true;
        }
        ui.set_opacity(k_in);
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            let largura = 1120f32.min(ui.available_width() - 48.);
            let margem = ((ui.available_width() - largura) / 2.).max(24.);
            ui.add_space(40.);
            ui.horizontal_top(|ui| {
                ui.add_space(margem);
                ui.vertical(|ui| {
                    ui.set_width(largura);
                    marca(ui, &pal);
                    ui.add_space(14.);
                    // B314: fio de arabesco só no tema Fênix; nos outros não desenha nada
                    crate::arabesco::separador(ui);
                    ui.add_space(14.);
                    let estreita = largura < 760.;
                    let esq = if estreita { largura } else { 280. };
                    let mut coluna_esq = |ui: &mut egui::Ui, escolha: &mut Option<Escolha>, transicao: &mut bool| {
                        ui.set_width(esq);
                        if e.voltar {
                            let r = linha(ui, ("voltar", 0), "", &format!("↩  {}", tr("Voltar ao trabalho")), None, &pal, tt, transicao);
                            if r.on_hover_text(tr("Volta ao documento aberto sem mudar nada")).clicked() {
                                *escolha = Some(Escolha::Voltar);
                            }
                            ui.add_space(14.);
                        }
                        rotulo(ui, "Começar", &pal);
                        for (cod, icone, nome, dica) in COMECAR {
                            if linha(ui, ("comecar", cod as usize), icone, tr(nome), None, &pal, tt, transicao).on_hover_text(tr(dica)).clicked() {
                                *escolha = Some(Escolha::Comecar(cod));
                            }
                        }
                        ui.add_space(24.);
                        rotulo(ui, "Recentes", &pal);
                        let existentes: Vec<&String> = e.arquivos.iter().filter(|f| std::path::Path::new(f.as_str()).exists()).collect();
                        if existentes.is_empty() && e.projetos.is_empty() {
                            // B314: no tema Fênix a pluma do estado vazio continua; nos outros, só o texto
                            crate::arabesco::vazio_com_texto(ui, egui::RichText::new(tr("nenhum ainda — abra um exemplo ao lado")).color(pal_fraco(&pal)));
                        }
                        for (i, f) in existentes.iter().enumerate() {
                            let p = std::path::Path::new(f.as_str());
                            let icone = match p.extension().map(|x| x.to_string_lossy().to_lowercase()).as_deref() {
                                Some("fka") => "asm_nova",
                                Some("fkd") => "des_novo",
                                _ => "nova_peca",
                            };
                            let nome = p.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| f.to_string());
                            let pasta = p.parent().map(|d| d.to_string_lossy().to_string());
                            // B508: peça recente com a prévia 3D dela no lugar do ícone, girando com o mouse em cima
                            let chave = chave_recente(f);
                            let pv = (icone == "nova_peca")
                                .then(|| {
                                    let ff = f.to_string();
                                    e.previews.get(&chave, move || std::fs::read_to_string(&ff).ok().and_then(|s| fenkra_core::Document::from_json(&s).ok()))
                                })
                                .flatten();
                            let mut giros = e.giro.borrow_mut();
                            let giro = giros.entry(chave).or_insert(i as f32 * 0.7);
                            let (r, quente) = linha_com_previa(ui, ("rec_arq", i), icone, &nome, pasta.as_deref(), pv.as_deref().map(|pv| (pv, *giro)), &pal, tt, transicao);
                            if e.gira && pv.is_some() && quente > 0.001 {
                                *giro += e.dt * 1.4 * quente;
                                girando.set(true);
                            }
                            if r.on_hover_text(f.as_str()).clicked() {
                                *escolha = Some(Escolha::Arquivo(f.to_string()));
                            }
                        }
                        for (i, d) in e.projetos.iter().enumerate() {
                            let p = std::path::Path::new(d.as_str());
                            let nome = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| d.clone());
                            let pasta = p.parent().map(|x| x.to_string_lossy().to_string());
                            if linha(ui, ("rec_proj", i), "pasta", &nome, pasta.as_deref(), &pal, tt, transicao).on_hover_text(d.as_str()).clicked() {
                                *escolha = Some(Escolha::Projeto(d.clone()));
                            }
                        }
                        ui.add_space(16.);
                        ui.checkbox(e.mostrar_ao_iniciar, tr("mostrar esta tela ao iniciar"));
                    };
                    let galeria = |ui: &mut egui::Ui, escolha: &mut Option<Escolha>, transicao: &mut bool| {
                        rotulo(ui, "Exemplos", &pal);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(20., 22.);
                            for (i, (nome, desc, bench)) in e.exemplos.iter().enumerate() {
                                let chave = format!("ex:{i}");
                                let pv = e.previews.get(&chave, move || crate::examples::all().into_iter().nth(i).map(|x| x.doc));
                                let mut giros = e.giro.borrow_mut();
                                let giro = giros.entry(chave.clone()).or_insert(i as f32 * 0.9);
                                let (r, quente) = miniatura(ui, &chave, tr(nome), tr(desc), bancada(*bench), pv.as_deref(), *giro, &pal, tt);
                                if quente > 0.001 && quente < 0.999 {
                                    *transicao = true;
                                }
                                // a peça só gira com o mouse em cima (B222: parada, não gasta quadros); o giro
                                // não depende do pedido do sistema nem do modo leve (B508, `gira_com_mouse`)
                                if e.gira && quente > 0.001 {
                                    *giro += e.dt * 1.4 * quente;
                                    girando.set(true);
                                }
                                if r.clicked() {
                                    *escolha = Some(Escolha::Exemplo(i));
                                }
                            }
                        });
                    };
                    if estreita {
                        galeria(ui, &mut escolha, &mut transicao);
                        ui.add_space(28.);
                        coluna_esq(ui, &mut escolha, &mut transicao);
                    } else {
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| coluna_esq(ui, &mut escolha, &mut transicao));
                            ui.add_space(48.);
                            ui.vertical(|ui| galeria(ui, &mut escolha, &mut transicao));
                        });
                    }
                    ui.add_space(40.);
                });
            });
        });
    });
    let visivel = root.ctx().input(|i| i.viewport().focused.unwrap_or(true) && !i.viewport().minimized.unwrap_or(false));
    let girando = girando.get();
    match proximo_quadro(e.ligada, visivel, transicao || girando) {
        Some(d) if d.is_zero() => root.ctx().request_repaint(),
        Some(d) => root.ctx().request_repaint_after(d),
        None if girando && visivel => root.ctx().request_repaint(),
        None => {}
    }
    escolha
}

fn pal_fraco(pal: &Paleta) -> Color32 {
    pal.texto.gamma_multiply(0.55)
}

fn bancada(b: crate::settings::Bench) -> &'static str {
    use crate::settings::Bench as B;
    tr(match b {
        B::Chapa => "Chapa metálica",
        B::Estrutural => "Estrutural",
        B::FormaLivre => "Forma livre",
        B::Superficies => "Superfícies",
        B::Montagem => "Montagem",
        B::Desenho => "Desenho",
        B::Solidos => "Sólidos",
    })
}

/// Marca: fênix pequena, nome e uma linha de apoio. A fênix fica no tamanho de um ícone.
fn marca(ui: &mut egui::Ui, pal: &Paleta) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(34., 34.), Sense::hover());
        egui::Image::new(egui::include_image!("../../../assets/app/fenix-256.png")).corner_radius(7.).paint_at(ui, r);
        ui.add_space(6.);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.;
            ui.label(egui::RichText::new("Fenkra").size(24.).strong().color(pal.texto));
            ui.label(egui::RichText::new(format!("{} · v{}", tr("CAD paramétrico open source"), env!("CARGO_PKG_VERSION"))).size(12.).color(pal_fraco(pal)));
        });
    });
}

/// Rótulo de seção: texto pequeno e espaçado, na cor fraca. A hierarquia vem do tamanho, não de enfeite.
fn rotulo(ui: &mut egui::Ui, t: &str, pal: &Paleta) {
    let s: String = tr(t).to_uppercase().chars().flat_map(|c| [c, '\u{2009}']).collect();
    ui.label(egui::RichText::new(s.trim_end()).size(11.).color(pal_fraco(pal)));
    ui.add_space(6.);
}

/// Uma linha clicável (ação ou recente): ícone, nome e, se houver, a pasta em texto fraco.
/// Ao passar o mouse: fundo acende de leve, o texto anda 4 px e um traço se desenha à esquerda.
/// Ao apertar: afunda 1 px e o traço fica cheio.
#[allow(clippy::too_many_arguments)]
fn linha(ui: &mut egui::Ui, id: (&str, usize), icone: &str, nome: &str, pasta: Option<&str>, pal: &Paleta, tt: f32, transicao: &mut bool) -> egui::Response {
    linha_com_previa(ui, id, icone, nome, pasta, None, pal, tt, transicao).0
}

/// Chave da prévia de uma peça recente no cache.
pub fn chave_recente(arquivo: &str) -> String {
    format!("arq:{arquivo}")
}

/// Como [`linha`], com a prévia 3D da peça (e o ângulo dela) no lugar do ícone quando há prévia.
/// Devolve também o realce, para quem chama girar a peça com o mouse em cima.
#[allow(clippy::too_many_arguments)]
fn linha_com_previa(ui: &mut egui::Ui, id: (&str, usize), icone: &str, nome: &str, pasta: Option<&str>, previa: Option<(&crate::examples::Preview, f32)>, pal: &Paleta, tt: f32, transicao: &mut bool) -> (egui::Response, f32) {
    let alt = if pasta.is_some() { 42. } else { 32. };
    let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), alt), Sense::click());
    let resp = crate::teclado::anel(ui, resp);
    let ctx = ui.ctx().clone();
    let quente = ctx.animate_bool_with_time(egui::Id::unique(("ini_linha", id)), resp.hovered() || resp.has_focus(), tt);
    let aperto = ctx.animate_bool_with_time(egui::Id::unique(("ini_aperto", id)), resp.is_pointer_button_down_on(), tt * 0.5);
    if (quente > 0.001 && quente < 0.999) || (aperto > 0.001 && aperto < 0.999) {
        *transicao = true;
    }
    let p = ui.painter();
    let r = r.translate(egui::vec2(0., aperto));
    if quente > 0.001 {
        p.rect_filled(r, 5., com_alfa(pal.destaque, (18. * quente + 14. * aperto) as u8));
        traco_vertical(p, r, (quente + aperto).min(1.), pal.destaque);
    }
    let x = r.left() + 12. + 4. * quente;
    let mut tx = x;
    if let Some((pv, giro)) = previa {
        let ir = Rect::from_center_size(egui::pos2(x + 13., r.center().y), egui::vec2(30., 30.));
        crate::examples::paint_preview(&ui.painter().with_clip_rect(ir.expand(2.)), ir, pv, giro);
        marcar_previa(ui.ctx(), id.0, id.1, r);
        tx += 36.;
    } else if !icone.is_empty() {
        let ir = Rect::from_center_size(egui::pos2(x + 9., r.top() + 16.), egui::vec2(18., 18.));
        egui::Image::new(crate::icons::get(icone)).paint_at(ui, ir);
        tx += 30.;
    }
    let p = ui.painter();
    p.text(egui::pos2(tx, r.top() + 16.), egui::Align2::LEFT_CENTER, nome, egui::FontId::proportional(14.), pal.texto);
    if let Some(d) = pasta {
        let largura = (r.right() - tx - 8.).max(20.);
        let max = (largura / 6.2) as usize;
        let d = if d.chars().count() > max { format!("…{}", d.chars().rev().take(max.saturating_sub(1)).collect::<Vec<_>>().into_iter().rev().collect::<String>()) } else { d.to_string() };
        p.text(egui::pos2(tx, r.top() + 31.), egui::Align2::LEFT_CENTER, d, egui::FontId::proportional(11.), pal_fraco(pal));
    }
    (resp.on_hover_cursor(egui::CursorIcon::PointingHand), quente)
}

/// Guarda onde o item com prévia foi pintado neste quadro (para o teste achar e passar o mouse).
fn marcar_previa(ctx: &egui::Context, grupo: &str, i: usize, r: Rect) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(("ini_previa_rect", grupo, i)), r));
}

/// Retângulo do item com prévia pintada no último quadro: grupo `ex` (exemplos) ou `rec_arq` (recentes).
#[cfg_attr(not(test), allow(dead_code))]
pub fn previa_pintada(ctx: &egui::Context, grupo: &str, i: usize) -> Option<Rect> {
    ctx.data(|d| d.get_temp(egui::Id::new(("ini_previa_rect", grupo, i))))
}

/// Traço vertical à esquerda da linha, crescendo do centro para as pontas com `k` de 0 a 1.
fn traco_vertical(p: &egui::Painter, r: Rect, k: f32, cor: Color32) {
    let meia = (r.height() * 0.5 - 7.) * k;
    let c = r.left_center() + egui::vec2(3., 0.);
    p.line_segment([c - egui::vec2(0., meia), c + egui::vec2(0., meia)], Stroke::new(2., cor));
}

/// Pontos do contorno de `r` percorrido a partir do canto de cima à esquerda, no sentido horário,
/// até a fração `k` do perímetro (0 a 1). É o traço que se desenha em volta da miniatura.
pub fn contorno_parcial(r: Rect, k: f32) -> Vec<Pos2> {
    let k = k.clamp(0., 1.);
    let cantos = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    let perimetro = 2. * (r.width() + r.height());
    let mut falta = perimetro * k;
    let mut pts = vec![cantos[0]];
    for w in cantos.windows(2) {
        let lado = (w[1] - w[0]).length();
        if falta <= 0. {
            break;
        }
        if falta >= lado {
            pts.push(w[1]);
            falta -= lado;
        } else {
            pts.push(w[0] + (w[1] - w[0]) * (falta / lado));
            falta = 0.;
        }
    }
    pts
}

/// Miniatura de exemplo: prévia, bancada em texto pequeno, nome e descrição. Sem sombra nem borda
/// em repouso; com o mouse, sobe 2 px e o contorno se traça em volta. Devolve a resposta e o realce.
#[allow(clippy::too_many_arguments)]
fn miniatura(ui: &mut egui::Ui, chave: &str, nome: &str, desc: &str, bancada: &str, pv: Option<&crate::examples::Preview>, giro: f32, pal: &Paleta, tt: f32) -> (egui::Response, f32) {
    let tamanho = egui::vec2(208., 214.);
    let (r0, resp) = ui.allocate_exact_size(tamanho, Sense::click());
    let resp = crate::teclado::anel(ui, resp);
    let ctx = ui.ctx().clone();
    let quente = ctx.animate_bool_with_time(egui::Id::unique(("ini_mini", chave)), resp.hovered() || resp.has_focus(), tt);
    let aperto = ctx.animate_bool_with_time(egui::Id::unique(("ini_mini_ap", chave)), resp.is_pointer_button_down_on(), tt * 0.5);
    let r = r0.translate(egui::vec2(0., -2. * quente + 1. * aperto));
    let p = ui.painter().with_clip_rect(ui.clip_rect());
    let pr = Rect::from_min_size(r.min, egui::vec2(r.width(), 132.));
    let mut m = egui::Mesh::default();
    for (pos, cor) in [(pr.left_top(), pal.mini_topo), (pr.right_top(), pal.mini_topo), (pr.right_bottom(), pal.mini_base), (pr.left_bottom(), pal.mini_base)] {
        m.colored_vertex(pos, cor);
    }
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(m));
    match pv {
        Some(pv) => {
            crate::examples::paint_preview(&p.with_clip_rect(pr), pr.shrink(10.), pv, giro);
            if let Some(i) = chave.strip_prefix("ex:").and_then(|i| i.parse().ok()) {
                marcar_previa(&ctx, "ex", i, r0);
            }
        }
        None => {
            // carregando: um traço curto girando, sem pedir quadro (o cache avisa quando termina)
            p.circle_stroke(pr.center(), 10., Stroke::new(1.5, com_alfa(pal.texto, 60)));
        }
    }
    // linha fina de base sempre; o contorno de destaque se traça com o realce
    p.line_segment([pr.left_bottom(), pr.right_bottom()], Stroke::new(1., com_alfa(pal.texto, 40)));
    if quente > 0.001 {
        let pts = contorno_parcial(pr, (quente + aperto * 0.2).min(1.));
        p.add(egui::Shape::line(pts, Stroke::new(1.5, pal.destaque)));
    }
    let y = pr.bottom() + 12.;
    p.text(egui::pos2(r.left(), y), egui::Align2::LEFT_TOP, bancada.to_uppercase(), egui::FontId::proportional(10.), pal.destaque);
    p.text(egui::pos2(r.left() + 3. * quente, y + 15.), egui::Align2::LEFT_TOP, nome, egui::FontId::proportional(15.), pal.texto);
    let g = p.layout(desc.to_string(), egui::FontId::proportional(11.5), pal_fraco(pal), r.width());
    p.galley(egui::pos2(r.left(), y + 36.), g, pal_fraco(pal));
    (resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tr("abrir este exemplo")), quente.max(aperto))
}

/// Fundo: degradê leve do tema e grade de prancheta muito fraca, que some para a esquerda e desliza
/// devagar na diagonal. Com `t` fixo, fica parada.
pub fn fundo(p: &egui::Painter, rect: Rect, pal: &Paleta, t: f32) {
    let mut m = egui::Mesh::default();
    for (pos, cor) in [(rect.left_top(), pal.topo), (rect.right_top(), pal.topo), (rect.right_bottom(), pal.base), (rect.left_bottom(), pal.base)] {
        m.colored_vertex(pos, cor);
    }
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(m));
    for (a, b, cor) in grade(rect, t) {
        let alfa = if pal.escuro { cor } else { cor + cor / 2 };
        p.line_segment([a, b], Stroke::new(1., com_alfa(pal.texto, alfa)));
    }
}

/// Segmentos da grade do fundo com a opacidade de cada um (0 a 255, antes do ajuste do tema).
/// A grade cobre os 60% da direita da tela e esmaece para a esquerda em faixas; desliza `DERIVA`
/// px por segundo e volta ao início a cada linha maior, sem salto visível.
pub fn grade(rect: Rect, t: f32) -> Vec<(Pos2, Pos2, u8)> {
    let ciclo = PASSO_GRADE * GRADE_MAIOR as f32;
    let d = (t * DERIVA).rem_euclid(ciclo);
    let x0 = rect.left() + rect.width() * 0.4;
    let mut v = Vec::new();
    let opac = |maior: bool| if maior { 16u8 } else { 8u8 };
    // linhas verticais, mais fracas quanto mais para a esquerda
    let mut i = 0usize;
    let mut x = x0 - ciclo + d;
    while x <= rect.right() {
        if x >= x0 {
            let f = ((x - x0) / (rect.right() - x0).max(1.)).clamp(0., 1.);
            let a = (opac(i % GRADE_MAIOR == 0) as f32 * f) as u8;
            if a > 0 {
                v.push((egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom()), a));
            }
        }
        x += PASSO_GRADE;
        i += 1;
    }
    // linhas horizontais em quatro faixas, cada uma mais forte à direita
    let mut j = 0usize;
    let mut y = rect.top() - ciclo + d;
    while y <= rect.bottom() {
        if y >= rect.top() {
            let base = opac(j % GRADE_MAIOR == 0) as f32;
            for k in 0..4 {
                let xa = x0 + (rect.right() - x0) * k as f32 / 4.;
                let xb = x0 + (rect.right() - x0) * (k + 1) as f32 / 4.;
                let a = (base * (k as f32 + 0.5) / 4.) as u8;
                if a > 0 {
                    v.push((egui::pos2(xa, y), egui::pos2(xb, y), a));
                }
            }
        }
        y += PASSO_GRADE;
        j += 1;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_ligado() -> Contexto {
        Contexto { opcao: true, ..Default::default() }
    }

    #[test]
    fn opcao_liga_e_desliga_a_animacao() {
        assert!(animacao_ligada(ctx_ligado()));
        assert!(!animacao_ligada(Contexto { opcao: false, ..ctx_ligado() }));
    }

    #[test]
    fn modo_leve_software_e_reduzir_movimento_desligam() {
        assert!(!animacao_ligada(Contexto { modo_leve: true, ..ctx_ligado() }));
        assert!(!animacao_ligada(Contexto { software: true, ..ctx_ligado() }));
        assert!(!animacao_ligada(Contexto { reduzir_movimento: true, ..ctx_ligado() }));
        assert!(!animacao_ligada(Contexto { sistema_reduz: true, ..ctx_ligado() }));
    }

    #[test]
    fn desligada_nao_pede_quadro() {
        // nem com transição em curso, nem com a janela visível
        assert_eq!(proximo_quadro(false, true, true), None);
        assert_eq!(proximo_quadro(false, true, false), None);
        assert_eq!(tempo_transicao(false), 0.);
        // ligada: transição pede já, deriva pede a cada QUADRO_MS, janela escondida não pede
        assert_eq!(proximo_quadro(true, true, true), Some(Duration::ZERO));
        assert_eq!(proximo_quadro(true, true, false), Some(Duration::from_millis(QUADRO_MS)));
        assert_eq!(proximo_quadro(true, false, true), None);
    }

    /// Roda a tela inicial num contexto do egui sem janela e devolve os pedidos de redesenho.
    fn quadros(ligada: bool) -> (bool, Option<Duration>) {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let cache = crate::examples::PreviewCache::default();
        let giro: std::cell::RefCell<std::collections::HashMap<String, f32>> = Default::default();
        let mut mostra = true;
        let mut pedidos = (false, None);
        // quadros de aquecimento: o carregador de imagens termina em outra thread e pede um quadro ao
        // terminar; sob carga isso caía dentro da medição. Mede-se só depois de as imagens assentarem.
        for _ in 0..40 {
            let mut s = ctx.run_ui(egui::RawInput { time: Some(0.), screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280., 800.))), ..Default::default() }, |ui| {
                let e = Entrada { pal: Paleta::atual(), ligada: false, t: 0., dt: 0.016, voltar: false, arquivos: &[], projetos: &[], mostrar_ao_iniciar: &mut mostra, previews: &cache, giro: &giro, gira: false, exemplos: &[] };
                mostrar(ui, e);
            });
            s.textures_delta.clear();
            if !s.viewport_output[&egui::ViewportId::ROOT].repaint_delay.is_zero() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        for quadro in 0..3 {
            let mut saida = ctx.run_ui(egui::RawInput { time: Some(quadro as f64 * 0.5), screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280., 800.))), ..Default::default() }, |ui| {
                let e = Entrada { pal: Paleta::atual(), ligada, t: quadro as f32 * 0.5 + 1., dt: 0.016, voltar: false, arquivos: &[], projetos: &[], mostrar_ao_iniciar: &mut mostra, previews: &cache, giro: &giro, gira: false, exemplos: &[] };
                mostrar(ui, e);
            });
            let vp = &saida.viewport_output[&egui::ViewportId::ROOT];
            pedidos = (vp.repaint_delay.is_zero(), (vp.repaint_delay < Duration::from_secs(1000)).then_some(vp.repaint_delay));
            saida.textures_delta.clear();
        }
        pedidos
    }

    #[test]
    fn tela_desligada_nao_pede_redesenho_e_ligada_pede() {
        let (_, desligada) = quadros(false);
        assert_eq!(desligada, None, "desligada, a tela parada não pode pedir quadro");
        let (_, ligada) = quadros(true);
        assert!(ligada.is_some_and(|d| d <= Duration::from_millis(QUADRO_MS)), "ligada, a grade pede quadro: {ligada:?}");
    }

    #[test]
    fn giro_com_mouse_nao_depende_do_sistema_nem_do_modo_leve() {
        // B508: com os efeitos de animação do Windows desligados a tela fica sem grade e sem transição,
        // mas a miniatura ainda gira com o mouse em cima
        let sistema = Contexto { sistema_reduz: true, ..ctx_ligado() };
        assert!(!animacao_ligada(sistema) && gira_com_mouse(sistema));
        assert!(gira_com_mouse(Contexto { modo_leve: true, software: true, ..ctx_ligado() }));
        // quem desliga de propósito, desliga também o giro
        assert!(!gira_com_mouse(Contexto { reduzir_movimento: true, ..ctx_ligado() }));
        assert!(!gira_com_mouse(Contexto { opcao: false, ..ctx_ligado() }));
    }

    /// B508: cada exemplo e cada peça recente tem prévia, e ela gira com o mouse em cima, mesmo com a
    /// animação da tela desligada (o caso do sistema que pede menos movimento).
    #[test]
    fn exemplos_e_recentes_tem_previa_que_gira_com_o_mouse_em_cima() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let pasta = std::env::temp_dir().join(format!("fenkra_b508_{}", std::process::id()));
        std::fs::create_dir_all(&pasta).unwrap();
        let arq = pasta.join("Placa.fkp");
        std::fs::write(&arq, fenkra_solidos::example_document().to_json()).unwrap();
        let arquivos = vec![arq.to_string_lossy().to_string()];
        let exemplos: Vec<(&'static str, &'static str, crate::settings::Bench)> = crate::examples::all().into_iter().map(|e| (e.name, e.desc, e.bench)).collect();
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let cache = crate::examples::PreviewCache::default();
        let giro: std::cell::RefCell<std::collections::HashMap<String, f32>> = Default::default();
        let mut mostra = true;
        let mut t = 0f64;
        let tela = Rect::from_min_size(Pos2::ZERO, egui::vec2(1280., 1600.));
        let mut quadro = |ponteiro: Option<Pos2>| -> Duration {
            t += 0.05;
            // sem ponteiro, nenhum evento: evento de entrada já faz o egui pedir mais um quadro
            let eventos = ponteiro.map_or_else(Vec::new, |p| vec![egui::Event::PointerMoved(p)]);
            let mut s = ctx.run_ui(egui::RawInput { time: Some(t), screen_rect: Some(tela), events: eventos, ..Default::default() }, |ui| {
                let e = Entrada { pal: Paleta::atual(), ligada: false, gira: true, t: 1., dt: 0.05, voltar: false, arquivos: &arquivos, projetos: &[], mostrar_ao_iniciar: &mut mostra, previews: &cache, giro: &giro, exemplos: &exemplos };
                mostrar(ui, e);
            });
            s.textures_delta.clear();
            s.viewport_output[&egui::ViewportId::ROOT].repaint_delay
        };
        // as prévias são calculadas em outra thread: espera todas ficarem prontas
        let chave = chave_recente(&arquivos[0]);
        let prontas = |cache: &crate::examples::PreviewCache| (0..exemplos.len()).all(|i| cache.get(&format!("ex:{i}"), || None).is_some()) && cache.get(&chave, || None).is_some();
        for _ in 0..600 {
            quadro(None);
            if prontas(&cache) {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        quadro(None);
        for (i, (nome, _, _)) in exemplos.iter().enumerate() {
            assert!(cache.get(&format!("ex:{i}"), || None).is_some(), "exemplo sem prévia: {nome}");
            assert!(previa_pintada(&ctx, "ex", i).is_some(), "prévia não pintada: {nome}");
        }
        assert!(cache.get(&chave, || None).is_some(), "peça recente sem prévia");
        let rec = previa_pintada(&ctx, "rec_arq", 0).expect("prévia da peça recente pintada");
        // parada sem o mouse: não gira nem pede quadro
        // (o carregador de ícones pede quadros ao terminar em outra thread: espera assentar)
        let antes = giro.borrow().clone();
        let mut espera = Duration::ZERO;
        for _ in 0..60 {
            espera = quadro(None);
            if espera > Duration::from_secs(1) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(*giro.borrow(), antes);
        assert!(espera > Duration::from_secs(1), "parada, não pede quadro: {espera:?}");
        // mouse sobre cada miniatura: gira e pede o próximo quadro
        for (i, (nome, _, _)) in exemplos.iter().enumerate() {
            let c = previa_pintada(&ctx, "ex", i).unwrap().center();
            let k = format!("ex:{i}");
            let a0 = giro.borrow()[&k];
            let mut espera = Duration::MAX;
            for _ in 0..4 {
                espera = quadro(Some(c));
            }
            assert!(giro.borrow()[&k] > a0 + 0.01, "{nome} não girou com o mouse em cima");
            assert!(espera.is_zero(), "girando, pede o próximo quadro");
        }
        let a0 = giro.borrow()[&chave];
        for _ in 0..4 {
            quadro(Some(rec.center()));
        }
        assert!(giro.borrow()[&chave] > a0 + 0.01, "a peça recente não girou com o mouse em cima");
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn contorno_se_desenha_ate_a_fracao() {
        let r = Rect::from_min_size(Pos2::ZERO, egui::vec2(100., 50.));
        assert_eq!(contorno_parcial(r, 0.), vec![Pos2::ZERO]);
        let meio = contorno_parcial(r, 0.5);
        assert_eq!(*meio.last().unwrap(), r.right_bottom() + egui::vec2(0., 0.));
        let cheio = contorno_parcial(r, 1.);
        assert_eq!(cheio.len(), 5);
        assert_eq!(*cheio.last().unwrap(), r.left_top());
    }

    #[test]
    fn grade_e_leve_e_desliza_sem_salto() {
        let r = Rect::from_min_size(Pos2::ZERO, egui::vec2(1920., 1080.));
        let g = grade(r, 0.);
        assert!(g.len() < 400, "grade pesada: {} segmentos", g.len());
        assert!(g.iter().all(|(a, b, _)| a.x >= r.left() + r.width() * 0.4 - 0.5 && b.x <= r.right() + 0.5));
        // depois de um ciclo inteiro a grade volta ao mesmo desenho
        let ciclo = PASSO_GRADE * GRADE_MAIOR as f32 / DERIVA;
        assert_eq!(grade(r, 0.), grade(r, ciclo));
        assert_ne!(grade(r, 0.), grade(r, ciclo / 3.));
    }

    #[test]
    fn sem_cor_fixa_neste_arquivo() {
        let fonte = include_str!("tela_inicial.rs");
        let codigo = fonte.split("#[cfg(test)]").next().unwrap();
        for proibido in ["Color32::from_rgb(", "Color32::from_rgba", "Color32::WHITE", "Color32::BLACK", "from_black_alpha", "from_white_alpha", "from_gray("] {
            assert!(!codigo.contains(proibido), "cor fixa na tela inicial: {proibido}");
        }
    }

    #[test]
    fn textos_da_tela_tem_traducao() {
        for (_, _, nome, dica) in COMECAR {
            assert!(crate::i18n::tem_traducao(nome) && crate::i18n::tem_traducao(dica), "{nome}");
        }
        assert!(crate::i18n::tem_traducao(DICA_OPCAO));
        for t in ["Começar", "Recentes", "Exemplos", "Animações da tela inicial", "CAD paramétrico open source", "Voltar ao trabalho", "nenhum ainda — abra um exemplo ao lado"] {
            assert!(crate::i18n::tem_traducao(t), "{t}");
        }
    }
}
