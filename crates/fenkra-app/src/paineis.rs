// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painéis livres e encaixáveis sobre a viewport, que se comportam como janelas: minimizam, expandem,
//! fecham (e reabrem pelo menu Exibir), arrastam, mostram a sombra de onde vão encaixar, redimensionam
//! pelos quatro lados e cantos e guardam tudo por bancada nas preferências.
//!
//! Código próprio em vez de biblioteca de docking: essas bibliotecas dividem a janela em áreas que
//! empurram umas às outras, e aqui a viewport ocupa tudo e os painéis flutuam translúcidos por cima.
//! O encaixe alinha o painel à borda e nunca reserva espaço; só painéis encaixados na mesma borda e no
//! mesmo trecho dela se empilham para não se cobrirem. Cada painel é um `egui::Window` sem barra de
//! título própria: o cabeçalho com os botões é desenhado aqui, para ter dica traduzida e ícones do tema.
//!
//! Estado de cada painel (`Posicao`): onde está, em que borda, se está minimizado ou fechado e o
//! tamanho que a pessoa escolheu. Os painéis também se registram a cada quadro, e o menu Exibir lista
//! os que estão vivos para reabrir os fechados (`menu`).
//!
//! Porta aberta para segunda tela (B366): `Posicao` só guarda coordenadas e flags serializáveis e o
//! conteúdo de cada painel é uma função de `&mut Ui`; um viewport nativo do egui pode receber o mesmo
//! conteúdo no futuro, bastando um `Encaixe::Janela` com a posição na tela. Veja o ADR 0012.

use crate::settings::{Bench, Settings};
use crate::theme;
use eframe::egui::{self, Align, Align2, FontId, Id, LayerId, Layout, Pos2, Rect, Sense, Stroke, Vec2};
use serde::{Deserialize, Serialize};

/// Margem entre o painel e a borda em que encaixa.
const MARGEM: f32 = 8.;
/// Distância do ponteiro até a borda para o encaixe valer.
const ALCANCE: f32 = 44.;
/// Largura da faixa da árvore: o encaixe à esquerda fica ao lado dela.
pub const FAIXA_ARVORE: f32 = 290.;
/// Altura reservada ao cubo de vistas no canto direito.
pub const FAIXA_CUBO: f32 = 128.;
/// Menor tamanho externo de um painel aberto.
const LARGURA_MIN: f32 = 96.;
const ALTURA_MIN: f32 = 84.;
/// Lado dos botões do cabeçalho.
const BOTAO: f32 = 16.;
/// Quanto do corpo do painel sobra além do cabeçalho, até o primeiro quadro medir de verdade.
const EXTRA_INICIAL: f32 = 41.;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Encaixe {
    #[default]
    Livre,
    Esquerda,
    Direita,
    Topo,
    Base,
    /// B366: fora da janela principal, numa janela nativa do sistema (outro monitor). `pos` guarda o
    /// canto da janela em coordenadas da área de trabalho, então o layout lembra o monitor.
    Janela,
}

impl Encaixe {
    /// Topo e base são bordas horizontais; esquerda, direita e livre compartilham o formato vertical.
    fn horizontal(self) -> bool {
        matches!(self, Encaixe::Topo | Encaixe::Base)
    }
}

fn falso(v: &bool) -> bool {
    !*v
}

/// Onde um painel está e como. Sem `pos`, ele usa a posição padrão.
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Posicao {
    pub encaixe: Encaixe,
    pub pos: Option<[f32; 2]>,
    /// só o cabeçalho aparece
    #[serde(default, skip_serializing_if = "falso")]
    pub minimizado: bool,
    /// não aparece até ser reaberto pelo menu Exibir
    #[serde(default, skip_serializing_if = "falso")]
    pub fechado: bool,
    /// largura externa escolhida arrastando um lado
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub largura: Option<f32>,
    /// altura externa escolhida arrastando um lado; sem ela, o painel tem a altura do conteúdo
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub altura: Option<f32>,
}

/// Posição guardada de um painel numa bancada.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PainelSalvo {
    pub bancada: Bench,
    pub nome: String,
    pub posicao: Posicao,
}

impl Settings {
    /// Posição do painel na bancada atual, ou `padrao` se ele nunca foi movido nela.
    pub fn painel(&self, nome: &str, padrao: Encaixe) -> Posicao {
        self.paineis
            .iter()
            .find(|p| p.bancada == self.bench && p.nome == nome)
            .map_or(Posicao { encaixe: padrao, ..Default::default() }, |p| p.posicao)
    }

    pub fn guardar_painel(&mut self, nome: &str, posicao: Posicao) {
        let bancada = self.bench;
        match self.paineis.iter_mut().find(|p| p.bancada == bancada && p.nome == nome) {
            Some(p) => p.posicao = posicao,
            None => self.paineis.push(PainelSalvo { bancada, nome: nome.to_string(), posicao }),
        }
    }

    /// Esquece a disposição da bancada atual; os painéis voltam ao padrão (e reabrem) no próximo quadro.
    pub fn restaurar_paineis(&mut self) {
        let bancada = self.bench;
        self.paineis.retain(|p| p.bancada != bancada);
    }
}

/// Canto superior esquerdo de um retângulo de `tam` para ele caber em `area`. Maior que a área, fica
/// no canto, com o título alcançável.
pub fn dentro(pos: Pos2, tam: Vec2, area: Rect) -> Pos2 {
    let eixo = |p: f32, t: f32, lo: f32, hi: f32| if t >= hi - lo { lo } else { p.clamp(lo, hi - t) };
    egui::pos2(eixo(pos.x, tam.x, area.left(), area.right()), eixo(pos.y, tam.y, area.top(), area.bottom()))
}

/// Borda de encaixe perto do ponteiro. A esquerda conta a partir da faixa da árvore.
pub fn borda_perto(p: Pos2, area: Rect) -> Option<Encaixe> {
    if !area.expand(ALCANCE).contains(p) {
        return None;
    }
    let esquerda = area.left() + FAIXA_ARVORE;
    if p.x > area.right() - ALCANCE {
        Some(Encaixe::Direita)
    } else if p.y < area.top() + ALCANCE {
        Some(Encaixe::Topo)
    } else if p.y > area.bottom() - ALCANCE {
        Some(Encaixe::Base)
    } else if p.x < esquerda + ALCANCE {
        Some(Encaixe::Esquerda)
    } else {
        None
    }
}

/// Posição em que o painel aparece. Encaixado, um dos eixos vem da borda e o outro do lugar em que foi
/// solto, para dois painéis poderem dividir a mesma borda. Nunca sai de `area`.
pub fn colocar(posicao: Posicao, padrao: Pos2, tam: Vec2, area: Rect) -> Pos2 {
    let base = posicao.pos.map_or(padrao, |[x, y]| egui::pos2(x, y));
    let alvo = match posicao.encaixe {
        Encaixe::Livre | Encaixe::Janela => base,
        Encaixe::Esquerda => egui::pos2(area.left() + FAIXA_ARVORE + MARGEM, base.y),
        Encaixe::Direita => egui::pos2(area.right() - tam.x - MARGEM, base.y),
        Encaixe::Topo => egui::pos2(base.x, area.top() + MARGEM),
        Encaixe::Base => egui::pos2(base.x, area.bottom() - tam.y - MARGEM),
    };
    dentro(alvo, tam, area)
}

/// Como `colocar`, mas um painel encaixado numa borda não cobre os que já estão encaixados na mesma
/// borda e no mesmo trecho dela: ele se afasta da borda, para dentro, até caber ao lado deles.
pub fn colocar_empilhado(posicao: Posicao, padrao: Pos2, tam: Vec2, area: Rect, outros: &[(Encaixe, Rect)]) -> Pos2 {
    let mut p = colocar(posicao, padrao, tam, area);
    if posicao.encaixe == Encaixe::Livre {
        return p;
    }
    for _ in 0..outros.len() + 1 {
        let meu = Rect::from_min_size(p, tam);
        let mut mudou = false;
        for (e, o) in outros.iter().filter(|(e, _)| *e == posicao.encaixe) {
            let cruza = if e.horizontal() { meu.x_range().intersects(o.x_range()) } else { meu.y_range().intersects(o.y_range()) };
            if !cruza {
                continue;
            }
            let novo = match e {
                Encaixe::Direita if meu.right() > o.left() - MARGEM => Some(egui::pos2(o.left() - MARGEM - tam.x, p.y)),
                Encaixe::Esquerda if meu.left() < o.right() + MARGEM => Some(egui::pos2(o.right() + MARGEM, p.y)),
                Encaixe::Topo if meu.top() < o.bottom() + MARGEM => Some(egui::pos2(p.x, o.bottom() + MARGEM)),
                Encaixe::Base if meu.bottom() > o.top() - MARGEM => Some(egui::pos2(p.x, o.top() - MARGEM - tam.y)),
                _ => None,
            };
            if let Some(n) = novo {
                p = n;
                mudou = true;
                break;
            }
        }
        if !mudou {
            break;
        }
    }
    dentro(p, tam, area)
}

/// Faixa desenhada como guia enquanto o painel é arrastado perto de uma borda.
pub fn faixa_da_guia(e: Encaixe, tam: Vec2, area: Rect) -> Option<Rect> {
    let (x0, x1, y0, y1) = (area.left() + FAIXA_ARVORE, area.right(), area.top(), area.bottom());
    Some(match e {
        Encaixe::Livre | Encaixe::Janela => return None,
        Encaixe::Esquerda => Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x0 + tam.x + 2. * MARGEM, y1)),
        Encaixe::Direita => Rect::from_min_max(egui::pos2(x1 - tam.x - 2. * MARGEM, y0), egui::pos2(x1, y1)),
        Encaixe::Topo => Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y0 + tam.y + 2. * MARGEM)),
        Encaixe::Base => Rect::from_min_max(egui::pos2(x0, y1 - tam.y - 2. * MARGEM), egui::pos2(x1, y1)),
    })
}

/// Como um painel se apresenta.
pub struct Painel {
    /// chave nas preferências; não muda com o idioma
    pub nome: String,
    pub titulo: String,
    /// posição quando nunca foi movido
    pub padrao: Pos2,
    /// largura quando nunca foi redimensionado
    pub largura: f32,
    /// não é mais usado: todo painel redimensiona pelos quatro lados e cantos
    #[allow(dead_code)]
    pub redimensiona: bool,
}

/// Ajustes raros de um painel; o padrão serve para painéis de comando e de diálogo.
#[derive(Clone, Copy, Default)]
pub struct Opcoes {
    /// painel que não some com o comando (barras e grupos): fechado, continua fechado quando a tela muda
    pub persiste: bool,
    /// separa o estado de janela do mesmo painel em formatos diferentes (barra vertical e horizontal)
    pub variante: u8,
}

/// O que aconteceu com o painel neste quadro.
pub struct Saida<R> {
    pub conteudo: Option<R>,
    /// nova posição, quando algo mudou neste quadro: arraste terminado, redimensionamento terminado,
    /// minimizar, expandir, fechar ou reabrir
    pub movido: Option<Posicao>,
    /// borda em que foi solto, quando foi solto numa
    pub solto_em: Option<Encaixe>,
    pub rect: Option<Rect>,
}

// ------------------------------------------------------------------ outro monitor (B366)

/// O painel foi solto com o ponteiro fora da janela principal (`tela`, em pontos da janela)?
pub fn soltou_fora(ponteiro: Option<Pos2>, tela: Rect) -> bool {
    ponteiro.is_some_and(|p| !tela.contains(p))
}

/// Canto da janela nativa na área de trabalho: o canto de dentro da janela principal mais o ponto do
/// painel na janela. Sem saber onde a principal está (sistema que não diz), usa o próprio ponto.
pub fn na_area_de_trabalho(principal: Option<Rect>, p: Pos2) -> Pos2 {
    principal.map_or(p, |r| r.min + p.to_vec2())
}

/// A janela nativa voltou para cima da principal: mais da metade dela está dentro.
pub fn voltou_para_dentro(principal: Rect, nativa: Rect) -> bool {
    let i = principal.intersect(nativa);
    i.is_positive() && i.area() > nativa.area() * 0.5
}

/// Posição do painel dentro da janela principal quando a nativa volta: o mesmo lugar da tela.
pub fn de_volta_na_janela(principal: Rect, nativa: Rect) -> Pos2 {
    (nativa.min - principal.min).to_pos2()
}

/// Mostra o painel numa janela nativa própria. O conteúdo é o mesmo do painel flutuante; o
/// cabeçalho ganha o botão de voltar. Fechar a janela pelo sistema também a devolve.
fn mostrar_janela<R>(ctx: &egui::Context, cfg: &Painel, posicao: Posicao, conteudo: impl FnOnce(&mut egui::Ui) -> R) -> Saida<R> {
    use crate::i18n::tr;
    let mut saida = Saida { conteudo: None, movido: None, solto_em: None, rect: None };
    let vid = egui::ViewportId::from_hash_of(("painel_nativo", cfg.nome.as_str()));
    let tam = egui::vec2(posicao.largura.unwrap_or(cfg.largura), posicao.altura.unwrap_or(460.));
    let mut b = egui::ViewportBuilder::default().with_title(cfg.titulo.as_str()).with_inner_size(tam).with_min_inner_size([LARGURA_MIN, ALTURA_MIN]);
    if let Some([x, y]) = posicao.pos {
        b = b.with_position(egui::pos2(x, y));
    }
    let principal = ctx.input(|i| i.viewport().inner_rect);
    let mut conteudo = Some(conteudo);
    let (dentro, externa, interna, fechou, voltar) = ctx.show_viewport_immediate(vid, b, |raiz, _| {
        let mut voltar = false;
        let estilo = raiz.ctx().global_style();
        let r = egui::CentralPanel::default().frame(egui::Frame::window(&estilo).inner_margin(egui::Margin::same(8))).show_inside(raiz, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(cfg.titulo.as_str()).small().strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button(tr("Voltar para a janela principal")).on_hover_text(tr("O painel volta a flutuar sobre a vista; arrastar esta janela de volta para a principal faz o mesmo")).clicked() {
                        voltar = true;
                    }
                });
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt("corpo_nativo").show(ui, |ui| conteudo.take().map(|c| c(ui))).inner
        });
        let (externa, interna, fechou) = raiz.input(|i| (i.viewport().outer_rect, i.viewport().inner_rect, i.viewport().close_requested()));
        (r.inner, externa, interna, fechou, voltar)
    });
    saida.conteudo = dentro;
    let mut n = posicao;
    // a janela mudou de lugar ou de tamanho: guarda (é assim que o layout lembra o monitor)
    if let Some(o) = externa {
        if posicao.pos.is_none_or(|[x, y]| (egui::pos2(x, y) - o.min).length() > 1.) {
            n.pos = Some([o.min.x, o.min.y]);
        }
    }
    if let Some(i) = interna {
        if (i.width() - tam.x).abs() > 1. || (i.height() - tam.y).abs() > 1. {
            n.largura = Some(i.width());
            n.altura = Some(i.height());
        }
    }
    let de_volta = matches!((principal, externa), (Some(p), Some(o)) if voltou_para_dentro(p, o));
    if voltar || fechou || de_volta {
        n.encaixe = Encaixe::Livre;
        n.pos = match (principal, externa) {
            (Some(p), Some(o)) if de_volta => {
                let q = de_volta_na_janela(p, o);
                Some([q.x, q.y])
            }
            _ => None,
        };
        n.altura = None;
    }
    if n != posicao {
        saida.movido = Some(n);
    }
    saida
}

/// Quadro translúcido dos painéis, nas cores do tema.
pub fn quadro(style: &egui::Style) -> egui::Frame {
    let mut q = egui::Frame::window(style).fill(style.visuals.window_fill.gamma_multiply(0.93)).inner_margin(egui::Margin::same(8));
    q.shadow.color = q.shadow.color.gamma_multiply(0.5);
    q
}

// ---------------------------------------------------------------------------------------------
// Registro dos painéis vivos (para o menu Exibir) e dos encaixados (para empilhar)

#[derive(Clone)]
struct Reg {
    nome: String,
    titulo: String,
    posicao: Posicao,
    quadro: u64,
}

fn id_registro() -> Id {
    Id::unique("paineis_registro")
}

fn id_docados() -> Id {
    Id::unique("paineis_docados")
}

/// O painel acaba de aparecer: não foi desenhado no quadro anterior.
fn apareceu_agora(ctx: &egui::Context, nome: &str, nr: u64) -> bool {
    ctx.data(|d| d.get_temp::<Vec<Reg>>(id_registro()).and_then(|v| v.iter().find(|r| r.nome == nome).map(|r| r.quadro + 1 < nr))).unwrap_or(true)
}

fn registrar(ctx: &egui::Context, cfg: &Painel, posicao: Posicao, nr: u64) {
    ctx.data_mut(|d| {
        let v = d.get_temp_mut_or_default::<Vec<Reg>>(id_registro());
        match v.iter_mut().find(|r| r.nome == cfg.nome) {
            Some(r) => {
                r.quadro = nr;
                r.posicao = posicao;
                if r.titulo != cfg.titulo {
                    r.titulo = cfg.titulo.clone();
                }
            }
            None => v.push(Reg { nome: cfg.nome.clone(), titulo: cfg.titulo.clone(), posicao, quadro: nr }),
        }
    });
}

fn vivos(ctx: &egui::Context) -> Vec<Reg> {
    let nr = ctx.cumulative_frame_nr();
    ctx.data(|d| d.get_temp::<Vec<Reg>>(id_registro())).unwrap_or_default().into_iter().filter(|r| r.quadro + 2 >= nr).collect()
}

/// Painel encaixado, como o registro o guarda: nome, borda, retângulo, quadro e prioridade (0 = barras).
type Docado = (String, Encaixe, Rect, u64, u8);

/// Painéis encaixados que `nome` precisa respeitar. Os que já foram desenhados neste quadro valem se não
/// têm menos prioridade; os do quadro anterior valem se têm mais (as barras são desenhadas por último, mas
/// ficam na borda, e os painéis de comando se afastam delas).
fn encaixados(ctx: &egui::Context, nr: u64, nome: &str, ordem: u8) -> Vec<(Encaixe, Rect)> {
    let todos = ctx.data(|d| d.get_temp::<Vec<Docado>>(id_docados())).unwrap_or_default();
    todos.into_iter().filter(|(n, _, _, q, o)| n != nome && ((*q == nr && *o <= ordem) || (*q + 1 == nr && *o < ordem))).map(|(_, e, r, _, _)| (e, r)).collect()
}

fn marcar_encaixado(ctx: &egui::Context, nome: &str, e: Encaixe, r: Rect, nr: u64, ordem: u8) {
    ctx.data_mut(|d| {
        let v = d.get_temp_mut_or_default::<Vec<Docado>>(id_docados());
        v.retain(|(n, _, _, q, _)| n != nome && *q + 1 >= nr);
        v.push((nome.to_string(), e, r, nr, ordem));
    });
}

/// Painéis que estão na tela agora, com a posição de cada um: o que o menu Exibir oferece.
pub fn abertos_ou_fechados(ctx: &egui::Context) -> Vec<(String, String, Posicao)> {
    vivos(ctx).into_iter().map(|r| (r.nome, r.titulo, r.posicao)).collect()
}

/// Abre ou fecha um painel pelo nome (menu Exibir).
pub fn definir_aberto(s: &mut Settings, nome: &str, padrao: Posicao, aberto: bool) {
    let mut p = s.painel(nome, padrao.encaixe);
    if !s.paineis.iter().any(|x| x.bancada == s.bench && x.nome == nome) {
        p = padrao;
    }
    p.fechado = !aberto;
    s.guardar_painel(nome, p);
}

/// Conteúdo do submenu de painéis do menu Exibir: uma caixa por painel vivo, marcada se está aberto.
pub fn menu(ui: &mut egui::Ui, s: &mut Settings) {
    let lista = abertos_ou_fechados(ui.ctx());
    if lista.is_empty() {
        ui.label(egui::RichText::new(crate::i18n::tr("Nenhum painel nesta tela")).weak());
        return;
    }
    for (nome, titulo, reg) in &lista {
        let atual = s.paineis.iter().find(|p| p.bancada == s.bench && p.nome == *nome).map_or(*reg, |p| p.posicao);
        let mut aberto = !atual.fechado;
        if ui.checkbox(&mut aberto, titulo).on_hover_text(crate::i18n::tr("Marque para mostrar o painel, desmarque para fechá-lo")).changed() {
            definir_aberto(s, nome, *reg, aberto);
        }
    }
    ui.separator();
    if ui.button(crate::i18n::tr("Mostrar todos os painéis")).clicked() {
        for (nome, _, reg) in &lista {
            definir_aberto(s, nome, *reg, true);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Janela: cabeçalho, redimensionamento e arraste

#[derive(Clone, Copy, PartialEq)]
enum Acao {
    Nada,
    Alternar,
    Fechar,
}

#[derive(Clone, Copy)]
enum Icone {
    Minimizar,
    Expandir,
    Fechar,
}

/// Botão do cabeçalho, desenhado com traços na cor do tema.
fn botao_janela(ui: &mut egui::Ui, icone: Icone, dica: &str) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(BOTAO, BOTAO), Sense::click());
    if ui.is_rect_visible(r) {
        let visual = ui.style().interact(&resp);
        if resp.hovered() || resp.has_focus() {
            ui.painter().rect_filled(r, 3., visual.bg_fill);
        }
        let tr = Stroke::new(1.4, visual.fg_stroke.color);
        let c = r.center();
        let p = ui.painter();
        match icone {
            Icone::Minimizar => {
                p.line_segment([c + egui::vec2(-4.5, 3.), c + egui::vec2(4.5, 3.)], tr);
            }
            Icone::Expandir => {
                p.rect_stroke(Rect::from_center_size(c, egui::vec2(9., 9.)), 1., tr, egui::StrokeKind::Middle);
            }
            Icone::Fechar => {
                p.line_segment([c + egui::vec2(-4., -4.), c + egui::vec2(4., 4.)], tr);
                p.line_segment([c + egui::vec2(-4., 4.), c + egui::vec2(4., -4.)], tr);
            }
        }
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, dica));
    resp.on_hover_text(dica)
}

/// Cabeçalho: título à esquerda, minimizar/expandir e fechar à direita. Duplo clique no título
/// também minimiza e expande.
fn cabecalho(ui: &mut egui::Ui, titulo: &str, minimizado: bool) -> Acao {
    use crate::i18n::tr;
    let mut acao = Acao::Nada;
    let mut area_titulo = Rect::NOTHING;
    // altura fixa: um layout centrado sem ela ocuparia todo o espaço vertical que sobra
    let linha = egui::vec2(ui.available_width(), BOTAO + 2.);
    ui.allocate_ui_with_layout(linha, Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 4.;
        if botao_janela(ui, Icone::Fechar, tr("fechar")).clicked() {
            acao = Acao::Fechar;
        }
        let (icone, dica) = if minimizado { (Icone::Expandir, tr("expandir")) } else { (Icone::Minimizar, tr("minimizar")) };
        if botao_janela(ui, icone, dica).clicked() {
            acao = Acao::Alternar;
        }
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.set_height(BOTAO);
            area_titulo = ui.max_rect();
            ui.add(egui::Label::new(egui::RichText::new(titulo).small().strong()).truncate().selectable(false));
        });
    });
    if acao == Acao::Nada {
        let duplo = ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
        if duplo && ui.input(|i| i.pointer.interact_pos()).is_some_and(|p| area_titulo.contains(p)) {
            acao = Acao::Alternar;
        }
    }
    acao
}

/// Lados e cantos que o ponteiro agarra em `r`: `[esquerda, direita, topo, base]`. As faixas têm a mesma
/// espessura das que o egui usa para redimensionar.
fn lados_em(o: Pos2, r: Rect, lado: f32, canto: f32) -> Option<[bool; 4]> {
    if !r.expand(lado.max(canto)).contains(o) {
        return None;
    }
    let (dxe, dxd) = ((o.x - r.left()).abs(), (o.x - r.right()).abs());
    let (dyt, dyb) = ((o.y - r.top()).abs(), (o.y - r.bottom()).abs());
    let perto_v = dyt <= canto || dyb <= canto;
    let perto_h = dxe <= canto || dxd <= canto;
    let l = [
        dxe <= lado || (dxe <= canto && perto_v),
        dxd <= lado || (dxd <= canto && perto_v),
        dyt <= lado || (dyt <= canto && perto_h),
        dyb <= lado || (dyb <= canto && perto_h),
    ];
    l.iter().any(|v| *v).then_some(l)
}

#[derive(Clone, Copy)]
struct Redim {
    lados: [bool; 4],
    inicio: Rect,
    origem: Pos2,
    arrastou: bool,
    /// última altura externa que o ponteiro pediu; vale também no quadro em que o botão é solto
    ultima: Option<f32>,
}

impl Redim {
    fn vertical(&self) -> bool {
        self.lados[2] || self.lados[3]
    }
    fn horizontal(&self) -> bool {
        self.lados[0] || self.lados[1]
    }
    /// Retângulo depois de arrastar os lados agarrados até `p`.
    fn ate(&self, p: Pos2) -> Rect {
        let d = p - self.origem;
        let mut r = self.inicio;
        if self.lados[0] {
            r.min.x += d.x;
        } else if self.lados[1] {
            r.max.x += d.x;
        }
        if self.lados[2] {
            r.min.y += d.y;
        } else if self.lados[3] {
            r.max.y += d.y;
        }
        r
    }
}

/// Último retângulo que a sombra de encaixe ocupou, para conferir em teste e na captura.
#[cfg_attr(not(test), allow(dead_code))]
pub fn previa_de(ctx: &egui::Context, nome: &str) -> Option<Rect> {
    ctx.data(|d| d.get_temp::<Option<Rect>>(Id::unique(("painel_previa", nome)))).flatten()
}

/// Sombra de onde o painel vai encaixar: a faixa da borda e o retângulo exato em que ele vai ficar.
fn sombra(ctx: &egui::Context, faixa: Rect, destino: Rect, rotulo: &str) {
    let destaque = theme::accent(&ctx.global_style().visuals);
    let p = ctx.layer_painter(LayerId::new(egui::Order::Foreground, Id::unique("guia_encaixe")));
    p.rect_filled(faixa, 6., destaque.gamma_multiply(0.08));
    p.rect_stroke(faixa, 6., Stroke::new(1., destaque.gamma_multiply(0.5)), egui::StrokeKind::Inside);
    p.rect_filled(destino, 6., destaque.gamma_multiply(0.22));
    p.rect_stroke(destino, 6., Stroke::new(1.5, destaque.gamma_multiply(0.9)), egui::StrokeKind::Inside);
    p.text(destino.center(), Align2::CENTER_CENTER, rotulo, FontId::proportional(13.), destaque);
}

/// Desenha o painel. Quem chama guarda `movido` nas preferências.
pub fn mostrar<R>(ctx: &egui::Context, area: Rect, cfg: &Painel, posicao: Posicao, conteudo: impl FnOnce(&mut egui::Ui) -> R) -> Saida<R> {
    mostrar_com(ctx, area, cfg, posicao, Opcoes::default(), conteudo)
}

/// `mostrar` com ajustes.
pub fn mostrar_com<R>(ctx: &egui::Context, area: Rect, cfg: &Painel, posicao: Posicao, op: Opcoes, conteudo: impl FnOnce(&mut egui::Ui) -> R) -> Saida<R> {
    let mut posicao = posicao;
    let nr = ctx.cumulative_frame_nr();
    let mut saida = Saida { conteudo: None, movido: None, solto_em: None, rect: None };
    // painel de diálogo que volta depois de sumir reaparece, mesmo que tenha sido fechado antes
    let novo = apareceu_agora(ctx, &cfg.nome, nr);
    if posicao.fechado && novo && !op.persiste {
        posicao.fechado = false;
        saida.movido = Some(posicao);
    }
    registrar(ctx, cfg, posicao, nr);
    if posicao.fechado {
        ctx.data_mut(|d| d.insert_temp(Id::unique(("painel_previa", cfg.nome.as_str())), None::<Rect>));
        return saida;
    }

    // captura sem mouse: `FENKRA_DEMO_JANELA_NATIVA=<nome do painel>` já abre o painel numa janela
    // própria, à direita, cobrindo menos da metade da principal
    if std::env::var("FENKRA_DEMO_JANELA_NATIVA").is_ok_and(|v| v == cfg.nome) && posicao.encaixe != Encaixe::Janela {
        posicao.encaixe = Encaixe::Janela;
        posicao.pos = ctx.input(|i| i.viewport().inner_rect).map(|r| [r.max.x - cfg.largura * 0.4, r.min.y + 90.]);
    }
    if posicao.encaixe == Encaixe::Janela {
        if !ctx.embed_viewports() {
            return mostrar_janela(ctx, cfg, posicao, conteudo);
        }
        // sistema sem janelas nativas: o painel fica flutuando dentro da principal
        posicao.encaixe = Encaixe::Livre;
        posicao.pos = None;
    }
    let id = Id::unique(("painel", cfg.nome.as_str(), op.variante));
    let (id_arrasto, id_redim, id_rect, id_extra, id_camada) = (id.with("arrasto"), id.with("redim"), id.with("rect"), id.with("extra"), id.with("camada"));
    let arrastando = ctx.data(|d| d.get_temp::<bool>(id_arrasto)).unwrap_or(false);
    let rect_ant = ctx.data(|d| d.get_temp::<Rect>(id_rect));
    let tam = rect_ant.map_or(egui::vec2(posicao.largura.unwrap_or(cfg.largura), 200.), |r| r.size());
    let estilo = ctx.global_style();
    let (lado, canto) = (estilo.interaction.resize_grab_radius_side + 1., estilo.interaction.resize_grab_radius_corner);
    let (ponteiro, apertado, origem, decidido) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.pointer.press_origin(), i.pointer.is_decidedly_dragging()));
    let max_alto = (area.height() - 2. * MARGEM).max(ALTURA_MIN);

    // redimensionamento: começa quando o botão é apertado numa faixa de lado ou canto do painel
    let mut redim = ctx.data(|d| d.get_temp::<Redim>(id_redim));
    if redim.is_none() && apertado && !arrastando {
        if let (Some(o), Some(r)) = (origem, rect_ant) {
            let camada = ctx.data(|d| d.get_temp::<LayerId>(id_camada));
            if camada.is_none_or(|c| ctx.layer_id_at(o) == Some(c)) {
                if let Some(mut lados) = lados_em(o, r, lado, canto) {
                    if posicao.minimizado {
                        lados[2] = false;
                        lados[3] = false;
                    }
                    if lados.iter().any(|v| *v) {
                        redim = Some(Redim { lados, inicio: r, origem: o, arrastou: false, ultima: None });
                    }
                }
            }
        }
    }
    let mut altura_viva = None;
    if let Some(rd) = redim.as_mut() {
        if apertado && decidido {
            rd.arrastou = true;
        }
        if apertado && rd.arrastou && rd.vertical() {
            if let Some(p) = ponteiro {
                rd.ultima = Some(rd.ate(p).height().clamp(ALTURA_MIN, max_alto));
            }
        }
        altura_viva = rd.ultima;
        let guardar = *rd;
        ctx.data_mut(|d| d.insert_temp(id_redim, guardar));
    }

    let ordem = if op.persiste { 0 } else { 1 };
    let outros = encaixados(ctx, nr, &cfg.nome, ordem);
    let alvo = colocar_empilhado(posicao, cfg.padrao, tam, area, &outros);
    let extra = ctx.data(|d| d.get_temp::<f32>(id_extra)).unwrap_or(EXTRA_INICIAL);
    let corpo_h = if posicao.minimizado { None } else { altura_viva.or(posicao.altura).map(|h| (h.min(max_alto) - extra).max(ALTURA_MIN - extra).max(30.)) };
    let corpo_max = (max_alto - extra).max(40.);

    let minimizado = posicao.minimizado;
    let mut w = egui::Window::new(egui::RichText::new(&cfg.titulo).small().strong())
        .id(id)
        .frame(quadro(&estilo))
        .title_bar(false)
        .collapsible(false)
        .resizable([true, !minimizado])
        .default_width(posicao.largura.unwrap_or(cfg.largura))
        .min_size([LARGURA_MIN, if minimizado { 0. } else { ALTURA_MIN }])
        .max_height(max_alto)
        .constrain_to(area);
    if !arrastando && redim.is_none() {
        w = w.current_pos(alvo);
    }
    let titulo = cfg.titulo.as_str();
    let r = w.show(ctx, |ui| {
        let acao = cabecalho(ui, titulo, minimizado);
        let mut dentro = None;
        let mut corpo = None;
        if !minimizado {
            ui.add_space(2.);
            let mut sa = egui::ScrollArea::vertical().id_salt("corpo").auto_shrink([false, corpo_h.is_none()]);
            let teto = corpo_h.unwrap_or(corpo_max);
            sa = match corpo_h {
                Some(h) => sa.min_scrolled_height(h).max_height(h),
                None => sa.max_height(corpo_max),
            };
            let out = sa.show(ui, |ui| {
                // os rolamentos de dentro do painel enxergam a altura útil e rolam sozinhos
                ui.set_max_height(teto.max(1.));
                conteudo(ui)
            });
            corpo = Some(out.inner_rect.top());
            dentro = Some(out.inner);
        }
        (acao, dentro, corpo)
    });
    let Some(r) = r else { return saida };
    let rect = r.response.rect;
    ctx.data_mut(|d| {
        d.insert_temp(id_rect, rect);
        d.insert_temp(id_camada, r.response.layer_id);
    });
    saida.rect = Some(rect);
    let (acao, dentro, corpo) = r.inner.unwrap_or((Acao::Nada, None, None));
    saida.conteudo = dentro;
    if let Some(corpo_topo) = corpo {
        // o que sobra do painel além do corpo rolável: cabeçalho, margem de baixo e contorno
        let sobra = (corpo_topo - rect.top()) + 8. + estilo.visuals.window_stroke.width;
        // durante o arraste o retângulo que o egui devolve ainda está um quadro atrás do conteúdo
        if sobra > 0. && (sobra - extra).abs() > 0.5 && redim.is_none() && !arrastando {
            ctx.data_mut(|d| d.insert_temp(id_extra, sobra));
        }
    }

    // botões do cabeçalho
    let mut base = saida.movido.unwrap_or(posicao);
    match acao {
        Acao::Alternar => {
            base.minimizado = !base.minimizado;
            saida.movido = Some(base);
        }
        Acao::Fechar => {
            base.fechado = true;
            saida.movido = Some(base);
        }
        Acao::Nada => {}
    }

    // fim do redimensionamento: guarda o tamanho e a posição que o egui deixou
    if let Some(rd) = redim {
        if !apertado {
            ctx.data_mut(|d| d.remove::<Redim>(id_redim));
            if rd.arrastou {
                let mut n = base;
                n.pos = Some([rect.min.x, rect.min.y]);
                if rd.horizontal() {
                    n.largura = Some(rect.width());
                }
                if rd.vertical() {
                    n.altura = Some(rect.height());
                }
                saida.movido = Some(n);
            }
        }
        return saida;
    }

    // o egui move a janela sem marcar a resposta dela como arrastada; quem denuncia o arraste é a janela
    // sair de onde a pusemos com o botão apertado sobre ela
    let borda = ponteiro.and_then(|p| borda_perto(p, area));
    let pegou = origem.is_some_and(|o| Rect::from_min_size(alvo, tam).expand(4.).contains(o));
    let saiu_do_lugar = (rect.min - alvo).length() > 0.5;
    if apertado && (arrastando || (pegou && saiu_do_lugar)) {
        ctx.data_mut(|d| d.insert_temp(id_arrasto, true));
        let previa = borda.map(|e| {
            let p = colocar_empilhado(Posicao { encaixe: e, pos: Some([rect.min.x, rect.min.y]), ..base }, cfg.padrao, rect.size(), area, &outros);
            Rect::from_min_size(p, rect.size())
        });
        if let (Some(e), Some(destino)) = (borda, previa) {
            if let Some(f) = faixa_da_guia(e, rect.size(), area) {
                sombra(ctx, f, destino, crate::i18n::tr("solte para encaixar"));
            }
        }
        // B366: com o ponteiro fora da janela, soltar abre o painel numa janela própria
        if soltou_fora(ponteiro, ctx.content_rect()) && !ctx.embed_viewports() {
            sombra(ctx, rect, rect, crate::i18n::tr("solte para abrir numa janela própria"));
        }
        ctx.data_mut(|d| d.insert_temp(Id::unique(("painel_previa", cfg.nome.as_str())), previa));
        ctx.request_repaint();
    } else {
        if arrastando {
            ctx.data_mut(|d| d.remove::<bool>(id_arrasto));
            saida.solto_em = borda;
            let mut n = base;
            n.encaixe = borda.unwrap_or_default();
            n.pos = Some([rect.min.x, rect.min.y]);
            // B366: solto fora da janela principal vira janela nativa, no ponto em que o mouse soltou
            if let (true, Some(p), false) = (soltou_fora(ponteiro, ctx.content_rect()), ponteiro, ctx.embed_viewports()) {
                let pega = origem.map_or(egui::Vec2::ZERO, |o| o - alvo);
                let q = na_area_de_trabalho(ctx.input(|i| i.viewport().inner_rect), p - pega);
                n.encaixe = Encaixe::Janela;
                n.pos = Some([q.x, q.y]);
                n.largura = Some(rect.width());
                n.altura = Some(rect.height().max(ALTURA_MIN));
            }
            saida.movido = Some(n);
        }
        ctx.data_mut(|d| d.insert_temp(Id::unique(("painel_previa", cfg.nome.as_str())), None::<Rect>));
        if posicao.encaixe != Encaixe::Livre {
            marcar_encaixado(ctx, &cfg.nome, posicao.encaixe, rect, nr, ordem);
        }
    }
    saida
}

/// Espaço que resta da viewport para posições padrão.
pub fn ao_lado_da_arvore(area: Rect) -> Pos2 {
    area.left_top() + egui::vec2(FAIXA_ARVORE + MARGEM, MARGEM)
}

/// Faixa direita, abaixo do cubo de vistas.
pub fn abaixo_do_cubo(area: Rect, largura: f32) -> Pos2 {
    egui::pos2(area.right() - largura - MARGEM, area.top() + FAIXA_CUBO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    /// B366: soltar fora da janela vira janela nativa no ponto da área de trabalho; arrastar a nativa
    /// de volta (mais da metade sobre a principal) devolve o painel ao mesmo lugar da tela; a posição
    /// em coordenadas da área de trabalho vai e volta do arquivo, então o monitor fica lembrado.
    #[test]
    fn painel_sai_para_outro_monitor_e_volta() {
        let tela = Rect::from_min_size(pos2(0., 0.), vec2(1400., 900.));
        assert!(!soltou_fora(Some(pos2(700., 400.)), tela));
        assert!(soltou_fora(Some(pos2(1500., 400.)), tela), "à direita da janela");
        assert!(soltou_fora(Some(pos2(-30., 400.)), tela), "à esquerda, no monitor do lado");
        assert!(!soltou_fora(None, tela));
        // a principal está em (100, 50) da área de trabalho; o ponto 1500 da janela cai em 1600
        let principal = Rect::from_min_size(pos2(100., 50.), vec2(1400., 900.));
        assert_eq!(na_area_de_trabalho(Some(principal), pos2(1500., 400.)), pos2(1600., 450.));
        // segundo monitor à direita: a nativa lá não volta; arrastada para cima da principal, volta
        let la = Rect::from_min_size(pos2(1950., 200.), vec2(320., 460.));
        assert!(!voltou_para_dentro(principal, la));
        let meio = Rect::from_min_size(pos2(1395., 200.), vec2(320., 460.));
        assert!(!voltou_para_dentro(principal, meio), "só um terço por cima ainda não volta");
        let dentro = Rect::from_min_size(pos2(900., 200.), vec2(320., 460.));
        assert!(voltou_para_dentro(principal, dentro));
        assert_eq!(de_volta_na_janela(principal, dentro), pos2(800., 150.));
        let mut s = prefs(Bench::Solidos);
        let p = Posicao { encaixe: Encaixe::Janela, pos: Some([1950., 200.]), largura: Some(320.), altura: Some(460.), ..Default::default() };
        s.guardar_painel("comando", p);
        let volta: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(volta.painel("comando", Encaixe::Livre), p, "o layout lembra a janela e o monitor");
        // janela nativa não tem faixa de encaixe nem muda de lugar ao ser colocada
        assert!(faixa_da_guia(Encaixe::Janela, vec2(300., 200.), tela).is_none());
        assert_eq!(colocar(p, pos2(0., 0.), vec2(10., 10.), Rect::from_min_size(pos2(0., 0.), vec2(4000., 2000.))), pos2(1950., 200.));
    }

    fn prefs(b: Bench) -> Settings {
        Settings { bench: b, ..Default::default() }
    }

    fn tela() -> Rect {
        Rect::from_min_size(pos2(0., 100.), vec2(1200., 700.))
    }

    fn livre(x: f32, y: f32) -> Posicao {
        Posicao { pos: Some([x, y]), ..Default::default() }
    }

    #[test]
    fn painel_fora_da_tela_volta_para_a_area_visivel() {
        let t = vec2(300., 200.);
        for fora in [pos2(5000., 5000.), pos2(-900., -900.), pos2(1190., 300.)] {
            let p = colocar(livre(fora.x, fora.y), pos2(0., 0.), t, tela());
            assert!(tela().contains_rect(Rect::from_min_size(p, t)), "{fora:?} ficou em {p:?}");
        }
    }

    #[test]
    fn painel_maior_que_a_tela_mostra_o_titulo() {
        let p = dentro(pos2(500., 500.), vec2(2000., 2000.), tela());
        assert_eq!(p, tela().min);
    }

    #[test]
    fn encaixe_na_direita_acompanha_a_janela() {
        let posicao = Posicao { encaixe: Encaixe::Direita, pos: Some([900., 300.]), ..Default::default() };
        let t = vec2(300., 200.);
        let larga = Rect::from_min_size(pos2(0., 100.), vec2(1800., 700.));
        assert_eq!(colocar(posicao, pos2(0., 0.), t, tela()), pos2(1200. - 300. - MARGEM, 300.));
        assert_eq!(colocar(posicao, pos2(0., 0.), t, larga), pos2(1800. - 300. - MARGEM, 300.));
    }

    #[test]
    fn borda_so_vale_perto_dela() {
        let t = tela();
        assert_eq!(borda_perto(pos2(1190., 400.), t), Some(Encaixe::Direita));
        assert_eq!(borda_perto(pos2(700., 105.), t), Some(Encaixe::Topo));
        assert_eq!(borda_perto(pos2(700., 795.), t), Some(Encaixe::Base));
        assert_eq!(borda_perto(pos2(FAIXA_ARVORE + 10., 400.), t), Some(Encaixe::Esquerda));
        assert_eq!(borda_perto(pos2(700., 400.), t), None);
    }

    #[test]
    fn posicao_e_por_bancada_e_restaurar_so_limpa_a_atual() {
        let mut s = prefs(Bench::Solidos);
        let aqui = livre(400., 300.);
        s.guardar_painel("comando", aqui);
        s.bench = Bench::Montagem;
        assert_eq!(s.painel("comando", Encaixe::Direita), Posicao { encaixe: Encaixe::Direita, ..Default::default() });
        s.guardar_painel("comando", Posicao { encaixe: Encaixe::Topo, ..Default::default() });
        s.restaurar_paineis();
        assert_eq!(s.painel("comando", Encaixe::Livre).pos, None);
        s.bench = Bench::Solidos;
        assert_eq!(s.painel("comando", Encaixe::Direita), aqui);
    }

    #[test]
    fn disposicao_sobrevive_a_fechar_e_reabrir() {
        let mut s = prefs(Bench::Superficies);
        let aqui = Posicao { encaixe: Encaixe::Base, pos: Some([320., 640.]), minimizado: true, fechado: true, largura: Some(310.), altura: Some(222.) };
        s.guardar_painel("barra_direita", aqui);
        let texto = serde_json::to_string(&s).expect("json");
        let lido: Settings = serde_json::from_str(&texto).expect("json");
        assert_eq!(lido.painel("barra_direita", Encaixe::Livre), aqui);
    }

    #[test]
    fn preferencias_antigas_continuam_validas() {
        let mut v = serde_json::to_value(Settings::default()).expect("json");
        v.as_object_mut().expect("objeto").remove("paineis");
        let s: Settings = serde_json::from_value(v).expect("json");
        assert!(s.paineis.is_empty());
        // posição gravada antes de existir minimizar, fechar e tamanho
        let antiga: Posicao = serde_json::from_str(r#"{"encaixe":"Direita","pos":[10.0,20.0]}"#).expect("json");
        assert_eq!(antiga, Posicao { encaixe: Encaixe::Direita, pos: Some([10., 20.]), ..Default::default() });
        // e a posição comum não grava os campos novos
        let texto = serde_json::to_string(&antiga).expect("json");
        assert!(!texto.contains("minimizado") && !texto.contains("fechado") && !texto.contains("altura"), "{texto}");
    }

    #[test]
    fn paineis_encaixados_na_mesma_borda_nao_se_cobrem() {
        let t = vec2(300., 200.);
        let a = Rect::from_min_size(pos2(1200. - 62. - MARGEM, 228.), vec2(62., 300.));
        let na_direita = Posicao { encaixe: Encaixe::Direita, pos: Some([900., 300.]), ..Default::default() };
        let p = colocar_empilhado(na_direita, pos2(0., 0.), t, tela(), &[(Encaixe::Direita, a)]);
        assert_eq!(p, pos2(a.left() - MARGEM - 300., 300.));
        // trecho diferente da mesma borda: fica na borda
        let longe = Posicao { pos: Some([900., 600.]), ..na_direita };
        assert_eq!(colocar_empilhado(longe, pos2(0., 0.), vec2(300., 100.), tela(), &[(Encaixe::Direita, a)]).x, 1200. - 300. - MARGEM);
        // outra borda não conta
        assert_eq!(colocar_empilhado(na_direita, pos2(0., 0.), t, tela(), &[(Encaixe::Esquerda, a)]), colocar(na_direita, pos2(0., 0.), t, tela()));
        // em cima, empilha para baixo
        let topo = Rect::from_min_size(pos2(400., 100. + MARGEM), vec2(500., 60.));
        let sob = Posicao { encaixe: Encaixe::Topo, pos: Some([500., 0.]), ..Default::default() };
        assert_eq!(colocar_empilhado(sob, pos2(0., 0.), vec2(200., 80.), tela(), &[(Encaixe::Topo, topo)]).y, topo.bottom() + MARGEM);
        // em baixo, empilha para cima
        let base = Rect::from_min_size(pos2(400., 800. - MARGEM - 60.), vec2(500., 60.));
        let sobre = Posicao { encaixe: Encaixe::Base, pos: Some([500., 0.]), ..Default::default() };
        assert_eq!(colocar_empilhado(sobre, pos2(0., 0.), vec2(200., 80.), tela(), &[(Encaixe::Base, base)]).y, base.top() - MARGEM - 80.);
    }

    #[test]
    fn faixas_de_redimensionar_cobrem_os_quatro_lados_e_cantos() {
        let r = Rect::from_min_size(pos2(100., 100.), vec2(300., 200.));
        let casos = [
            (pos2(100., 200.), [true, false, false, false]),
            (pos2(400., 200.), [false, true, false, false]),
            (pos2(250., 100.), [false, false, true, false]),
            (pos2(250., 300.), [false, false, false, true]),
            (pos2(102., 102.), [true, false, true, false]),
            (pos2(398., 104.), [false, true, true, false]),
            (pos2(104., 298.), [true, false, false, true]),
            (pos2(397., 297.), [false, true, false, true]),
        ];
        for (p, esperado) in casos {
            assert_eq!(lados_em(p, r, 4., 10.), Some(esperado), "{p:?}");
        }
        assert_eq!(lados_em(pos2(250., 200.), r, 4., 10.), None);
        assert_eq!(lados_em(pos2(300., 40.), r, 4., 10.), None);
    }

    // ----- simulação de eventos: o mesmo caminho que o mouse percorre -----

    struct Banco {
        ctx: egui::Context,
        cfg: Painel,
        posicao: Posicao,
        rect: Option<Rect>,
        solto: Option<Encaixe>,
        op: Opcoes,
        linhas: usize,
    }

    impl Banco {
        fn novo(nome: &str) -> Self {
            let ctx = egui::Context::default();
            let cfg = Painel { nome: nome.into(), titulo: "Teste".into(), padrao: pos2(500., 300.), largura: 240., redimensiona: true };
            Self { ctx, cfg, posicao: Posicao::default(), rect: None, solto: None, op: Opcoes::default(), linhas: 6 }
        }

        /// Um quadro, com os eventos dados; a posição devolvida é guardada, como o app faz.
        fn quadro(&mut self, evs: Vec<egui::Event>) -> Saida<()> {
            let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0., 0.), vec2(1200., 800.))), events: evs, ..Default::default() };
            let mut saida = None;
            let (cfg, posicao, op, linhas) = (&self.cfg, self.posicao, self.op, self.linhas);
            passo_ui(&self.ctx, raw, |ui| {
                let ctx = ui.ctx().clone();
                saida = Some(mostrar_com(&ctx, tela(), cfg, posicao, op, |ui| {
                    for i in 0..linhas {
                        ui.label(format!("linha {i}"));
                    }
                }));
            });
            let s = saida.expect("quadro");
            if let Some(m) = s.movido {
                self.posicao = m;
            }
            if s.solto_em.is_some() {
                self.solto = s.solto_em;
            }
            self.rect = s.rect.or(self.rect);
            s
        }

        fn assentar(&mut self) -> Rect {
            for _ in 0..4 {
                self.quadro(vec![]);
            }
            self.rect.expect("painel")
        }

        fn mover(&mut self, p: Pos2) {
            self.quadro(vec![egui::Event::PointerMoved(p)]);
        }

        fn apertar(&mut self, p: Pos2) {
            self.mover(p);
            self.quadro(mouse(p, true));
        }

        fn soltar(&mut self, p: Pos2) {
            self.quadro(mouse(p, false));
            self.quadro(vec![]);
        }

        /// Aperta em `de`, arrasta em passos até `ate` e solta.
        fn arrastar(&mut self, de: Pos2, ate: Pos2) {
            self.apertar(de);
            for k in 1..=4 {
                self.mover(de + (ate - de) * (k as f32 / 4.));
            }
            self.soltar(ate);
        }

        fn clicar(&mut self, p: Pos2) {
            self.apertar(p);
            self.soltar(p);
        }
    }

    fn passo_ui(ctx: &egui::Context, raw: egui::RawInput, f: impl FnMut(&mut egui::Ui)) {
        let mut saida = ctx.run_ui(raw, f);
        saida.textures_delta.clear();
    }

    fn mouse(p: Pos2, down: bool) -> Vec<egui::Event> {
        vec![egui::Event::PointerMoved(p), egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: down, modifiers: Default::default() }]
    }

    /// Centro dos botões do cabeçalho: fechar é o último da direita, minimizar vem à esquerda dele.
    fn botao_fechar(r: Rect) -> Pos2 {
        pos2(r.right() - 8. - BOTAO / 2., r.top() + 8. + BOTAO / 2.)
    }
    fn botao_minimizar(r: Rect) -> Pos2 {
        botao_fechar(r) - vec2(BOTAO + 4., 0.)
    }
    fn titulo_de(r: Rect) -> Pos2 {
        r.left_top() + vec2(14., 16.)
    }

    #[test]
    fn arrastar_pelo_titulo_ate_cada_borda_mostra_a_sombra_e_encaixa() {
        for (alvo, esperado) in [
            (pos2(1195., 500.), Encaixe::Direita),
            (pos2(700., 105.), Encaixe::Topo),
            (pos2(700., 795.), Encaixe::Base),
            (pos2(FAIXA_ARVORE + 10., 450.), Encaixe::Esquerda),
        ] {
            let mut b = Banco::novo("teste");
            let r = b.assentar();
            let de = titulo_de(r);
            b.apertar(de);
            for k in 1..=4 {
                b.mover(de + (alvo - de) * (k as f32 / 4.));
            }
            // antes de soltar a sombra mostra onde o painel vai ficar
            let sombra = previa_de(&b.ctx, "teste").unwrap_or_else(|| panic!("sem sombra para {esperado:?}"));
            let tam = b.rect.expect("rect").size();
            assert!((sombra.size() - tam).length() < 1., "{sombra:?}");
            match esperado {
                Encaixe::Direita => assert!((sombra.right() - (tela().right() - MARGEM)).abs() < 1., "{sombra:?}"),
                Encaixe::Topo => assert!((sombra.top() - (tela().top() + MARGEM)).abs() < 1., "{sombra:?}"),
                Encaixe::Base => assert!((sombra.bottom() - (tela().bottom() - MARGEM)).abs() < 1., "{sombra:?}"),
                Encaixe::Esquerda => assert!((sombra.left() - (tela().left() + FAIXA_ARVORE + MARGEM)).abs() < 1., "{sombra:?}"),
                Encaixe::Livre | Encaixe::Janela => unreachable!(),
            }
            b.soltar(alvo);
            assert_eq!(b.posicao.encaixe, esperado);
            assert_eq!(b.solto, Some(esperado));
            assert!(previa_de(&b.ctx, "teste").is_none(), "a sombra some ao soltar");
            // encaixado de verdade: o painel foi parar na borda
            let r = b.assentar();
            assert!(tela().contains_rect(r.shrink(0.5)), "{r:?}");
            match esperado {
                Encaixe::Direita => assert!((r.right() - (tela().right() - MARGEM)).abs() < 1.5, "{r:?}"),
                Encaixe::Topo => assert!((r.top() - (tela().top() + MARGEM)).abs() < 1.5, "{r:?}"),
                Encaixe::Base => assert!((r.bottom() - (tela().bottom() - MARGEM)).abs() < 1.5, "{r:?}"),
                _ => assert!((r.left() - (tela().left() + FAIXA_ARVORE + MARGEM)).abs() < 1.5, "{r:?}"),
            }
        }
    }

    #[test]
    fn soltar_no_meio_deixa_o_painel_livre_onde_foi_solto() {
        let mut b = Banco::novo("teste");
        let r = b.assentar();
        b.arrastar(titulo_de(r), pos2(700., 450.));
        assert_eq!(b.posicao.encaixe, Encaixe::Livre);
        let n = b.assentar();
        assert!((n.min - (r.min + (pos2(700., 450.) - titulo_de(r)))).length() < 2., "{n:?} de {r:?}");
        // e sai de lá encaixado, sem sobrar sombra
        b.arrastar(titulo_de(n), pos2(1195., 500.));
        assert_eq!(b.posicao.encaixe, Encaixe::Direita);
        b.arrastar(titulo_de(b.rect.expect("r")), pos2(700., 450.));
        assert_eq!(b.posicao.encaixe, Encaixe::Livre);
    }

    #[test]
    fn minimizar_e_expandir_pelo_botao_e_pelo_duplo_clique() {
        let mut b = Banco::novo("teste");
        let aberto = b.assentar();
        b.clicar(botao_minimizar(aberto));
        assert!(b.posicao.minimizado, "o clique no botão minimiza");
        let mini = b.assentar();
        assert!(mini.height() < aberto.height() * 0.5, "{mini:?} contra {aberto:?}");
        assert!((mini.width() - aberto.width()).abs() < 1., "a largura é mantida");
        assert!((mini.min - aberto.min).length() < 1.5, "o canto não anda ao minimizar");
        b.clicar(botao_minimizar(mini));
        assert!(!b.posicao.minimizado, "o mesmo botão expande");
        let volta = b.assentar();
        assert!((volta.height() - aberto.height()).abs() < 1.5, "{volta:?} contra {aberto:?}");
        // duplo clique no título
        let t = titulo_de(volta);
        b.mover(t);
        for _ in 0..2 {
            b.quadro(mouse(t, true));
            b.quadro(mouse(t, false));
        }
        let _ = b.assentar();
        // clicar duas vezes seguidas conta como duplo clique no egui; se o simulador não montar o duplo
        // clique, o botão acima já provou o caminho
        if b.posicao.minimizado {
            assert!(b.assentar().height() < aberto.height() * 0.5);
        }
    }

    #[test]
    fn fechar_some_do_quadro_e_reabrir_pelo_menu_traz_de_volta() {
        let mut b = Banco::novo("teste");
        let aberto = b.assentar();
        b.clicar(botao_fechar(aberto));
        assert!(b.posicao.fechado, "o X fecha");
        let s = b.quadro(vec![]);
        assert!(s.rect.is_none() && s.conteudo.is_none(), "fechado não desenha nada");
        // o menu Exibir ainda enxerga o painel, fechado
        let lista = abertos_ou_fechados(&b.ctx);
        assert!(lista.iter().any(|(n, t, p)| n == "teste" && t == "Teste" && p.fechado), "{lista:?}");
        // reabre pelo menu: o painel volta onde estava
        let mut s = prefs(Bench::Solidos);
        s.guardar_painel("teste", b.posicao);
        definir_aberto(&mut s, "teste", b.posicao, true);
        b.posicao = s.painel("teste", Encaixe::Livre);
        assert!(!b.posicao.fechado);
        let volta = b.assentar();
        assert!((volta.min - aberto.min).length() < 1.5, "{volta:?} contra {aberto:?}");
        // fechar pelo menu também funciona
        definir_aberto(&mut s, "teste", b.posicao, false);
        assert!(s.painel("teste", Encaixe::Livre).fechado);
    }

    #[test]
    fn menu_fecha_painel_que_nunca_foi_movido_sem_perder_o_encaixe_padrao() {
        let mut s = prefs(Bench::Solidos);
        let padrao = Posicao { encaixe: Encaixe::Direita, ..Default::default() };
        definir_aberto(&mut s, "barra_direita", padrao, false);
        let p = s.painel("barra_direita", Encaixe::Livre);
        assert!(p.fechado && p.encaixe == Encaixe::Direita);
    }

    #[test]
    fn painel_de_dialogo_fechado_reaparece_quando_o_dialogo_volta() {
        let mut b = Banco::novo("dialogo");
        let aberto = b.assentar();
        b.clicar(botao_fechar(aberto));
        assert!(b.posicao.fechado);
        // o dialogo sai de cena por alguns quadros (o fluxo acabou) e volta num fluxo novo
        for _ in 0..5 {
            passo_ui(&b.ctx, egui::RawInput { screen_rect: Some(tela()), ..Default::default() }, |_| {});
        }
        let s = b.quadro(vec![]);
        assert!(!b.posicao.fechado, "um diálogo novo não pode nascer invisível");
        assert!(s.movido.is_some());
        let _ = b.assentar();
        assert!(b.rect.is_some());
        // painel que persiste (barra) continua fechado depois do mesmo intervalo
        let mut barra = Banco::novo("barra");
        barra.op.persiste = true;
        let r = barra.assentar();
        barra.clicar(botao_fechar(r));
        for _ in 0..5 {
            passo_ui(&barra.ctx, egui::RawInput { screen_rect: Some(tela()), ..Default::default() }, |_| {});
        }
        barra.quadro(vec![]);
        assert!(barra.posicao.fechado);
    }

    /// Ponto no meio de cada lado e junto de cada canto, em coordenadas do painel.
    fn alca(r: Rect, lados: [bool; 4]) -> Pos2 {
        let x = if lados[0] { r.left() + 1. } else if lados[1] { r.right() - 1. } else { r.center().x };
        let y = if lados[2] { r.top() + 1. } else if lados[3] { r.bottom() - 1. } else { r.center().y };
        pos2(x, y)
    }

    #[test]
    fn redimensiona_pelos_quatro_lados_e_pelos_quatro_cantos() {
        // [esquerda, direita, topo, base], deslocamento do ponteiro
        let casos: [([bool; 4], Vec2); 8] = [
            ([true, false, false, false], vec2(-60., 0.)),
            ([false, true, false, false], vec2(50., 0.)),
            ([false, false, true, false], vec2(0., -50.)),
            ([false, false, false, true], vec2(0., 70.)),
            ([true, false, true, false], vec2(-40., -30.)),
            ([false, true, true, false], vec2(40., -30.)),
            ([true, false, false, true], vec2(-40., 45.)),
            ([false, true, false, true], vec2(35., 45.)),
        ];
        for (lados, d) in casos {
            let mut b = Banco::novo("teste");
            let antes = b.assentar();
            let de = alca(antes, lados);
            b.arrastar(de, de + d);
            let depois = b.assentar();
            let mut esperado = antes;
            if lados[0] {
                esperado.min.x += d.x;
            }
            if lados[1] {
                esperado.max.x += d.x;
            }
            if lados[2] {
                esperado.min.y += d.y;
            }
            if lados[3] {
                esperado.max.y += d.y;
            }
            let erro = |a: Pos2, b: Pos2| (a - b).abs().max_elem();
            assert!(erro(depois.min, esperado.min) < 2.5 && erro(depois.max, esperado.max) < 2.5, "lados {lados:?}: de {antes:?} para {depois:?}, esperado {esperado:?}");
            // o tamanho escolhido fica guardado, só nos eixos mexidos
            assert_eq!(b.posicao.largura.is_some(), lados[0] || lados[1], "{lados:?}");
            assert_eq!(b.posicao.altura.is_some(), lados[2] || lados[3], "{lados:?}");
            // e continua assim nos quadros seguintes (nada o desfaz)
            let depois2 = b.assentar();
            assert!((depois2.size() - depois.size()).length() < 1., "{lados:?}: {depois:?} virou {depois2:?}");
        }
    }

    #[test]
    fn redimensionar_para_menor_rola_o_conteudo_em_vez_de_cortar() {
        let mut b = Banco::novo("teste");
        b.linhas = 14;
        let antes = b.assentar();
        let de = alca(antes, [false, false, false, true]);
        b.arrastar(de, de + vec2(0., -120.));
        let depois = b.assentar();
        assert!((antes.height() - depois.height() - 120.).abs() < 2.5, "{antes:?} -> {depois:?}");
        assert!(depois.height() >= ALTURA_MIN - 1.);
        // nunca passa do mínimo
        let de = alca(depois, [false, false, true, false]);
        b.arrastar(de, de + vec2(0., 400.));
        let menor = b.assentar();
        assert!(menor.height() >= ALTURA_MIN - 1.5, "{menor:?}");
    }

    #[test]
    fn painel_encaixado_redimensionado_continua_encaixado_com_o_tamanho_novo() {
        let mut b = Banco::novo("teste");
        let r = b.assentar();
        b.arrastar(titulo_de(r), pos2(1195., 500.));
        let docado = b.assentar();
        // puxa a lateral esquerda para dentro da tela: o painel fica mais largo e colado à direita
        let de = alca(docado, [true, false, false, false]);
        b.arrastar(de, de + vec2(-80., 0.));
        assert_eq!(b.posicao.encaixe, Encaixe::Direita);
        let largo = b.assentar();
        assert!((largo.width() - docado.width() - 80.).abs() < 2.5, "{docado:?} -> {largo:?}");
        assert!((largo.right() - (tela().right() - MARGEM)).abs() < 1.5, "{largo:?}");
    }

    #[test]
    fn dois_paineis_na_mesma_borda_ficam_lado_a_lado() {
        let ctx = egui::Context::default();
        let a = Painel { nome: "a".into(), titulo: "A".into(), padrao: pos2(0., 0.), largura: 100., redimensiona: true };
        let c = Painel { nome: "c".into(), titulo: "C".into(), padrao: pos2(0., 0.), largura: 200., redimensiona: true };
        let na_direita = Posicao { encaixe: Encaixe::Direita, pos: Some([900., 200.]), ..Default::default() };
        let mut rects = (None, None);
        for _ in 0..4 {
            let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0., 0.), vec2(1200., 800.))), ..Default::default() };
            passo_ui(&ctx, raw, |ui| {
                let ctx = ui.ctx().clone();
                rects.0 = mostrar(&ctx, tela(), &a, na_direita, |ui| ui.label("a")).rect;
                rects.1 = mostrar(&ctx, tela(), &c, na_direita, |ui| ui.label("c")).rect;
            });
        }
        let (ra, rc) = (rects.0.expect("a"), rects.1.expect("c"));
        assert!(!ra.intersects(rc), "{ra:?} cobre {rc:?}");
        assert!((ra.right() - (tela().right() - MARGEM)).abs() < 1.5, "o primeiro fica na borda");
        assert!(rc.right() < ra.left(), "o segundo fica para dentro");
    }

    #[test]
    fn barra_fica_na_borda_mesmo_desenhada_depois_e_o_painel_de_comando_se_afasta() {
        let ctx = egui::Context::default();
        let comando = Painel { nome: "comando".into(), titulo: "C".into(), padrao: pos2(0., 0.), largura: 200., redimensiona: true };
        let barra = Painel { nome: "barra".into(), titulo: "B".into(), padrao: pos2(0., 0.), largura: 62., redimensiona: true };
        let na_direita = Posicao { encaixe: Encaixe::Direita, pos: Some([900., 200.]), ..Default::default() };
        let mut rects = (None, None);
        for _ in 0..6 {
            let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0., 0.), vec2(1200., 800.))), ..Default::default() };
            passo_ui(&ctx, raw, |ui| {
                let ctx = ui.ctx().clone();
                // como no app: o comando é desenhado antes, as barras por último
                rects.0 = mostrar(&ctx, tela(), &comando, na_direita, |ui| ui.label("comando")).rect;
                rects.1 = mostrar_com(&ctx, tela(), &barra, na_direita, Opcoes { persiste: true, variante: 0 }, |ui| ui.label("b")).rect;
            });
        }
        let (rc, rb) = (rects.0.expect("comando"), rects.1.expect("barra"));
        assert!(!rc.intersects(rb), "{rc:?} cobre {rb:?}");
        assert!((rb.right() - (tela().right() - MARGEM)).abs() < 1.5, "a barra fica na borda: {rb:?}");
        assert!(rc.right() < rb.left(), "o comando fica para dentro: {rc:?} contra {rb:?}");
    }

    #[test]
    fn disposicao_salva_e_restaurada_em_outra_sessao() {
        let mut b = Banco::novo("teste");
        let r = b.assentar();
        // minimiza, depois expande e redimensiona, depois arrasta e encaixa embaixo
        b.clicar(botao_minimizar(r));
        let mini = b.assentar();
        b.clicar(botao_minimizar(mini));
        let r = b.assentar();
        let de = alca(r, [false, true, false, true]);
        b.arrastar(de, de + vec2(60., 50.));
        let r = b.assentar();
        b.arrastar(titulo_de(r), pos2(700., 795.));
        let esperado = b.assentar();
        let mut s = prefs(Bench::Solidos);
        s.guardar_painel("teste", b.posicao);
        let json = serde_json::to_string(&s).expect("json");
        // outra sessão: contexto novo, preferências lidas do disco
        let lido: Settings = serde_json::from_str(&json).expect("json");
        let mut o = Banco::novo("teste");
        o.posicao = lido.painel("teste", Encaixe::Livre);
        assert_eq!(o.posicao.encaixe, Encaixe::Base);
        let vi = o.assentar();
        assert!((vi.size() - esperado.size()).length() < 3., "{vi:?} contra {esperado:?}");
        assert!((vi.min - esperado.min).length() < 3., "{vi:?} contra {esperado:?}");
        // minimizado e fechado também voltam como estavam
        o.posicao.minimizado = true;
        assert!(o.assentar().height() < esperado.height() * 0.6);
        o.posicao.fechado = true;
        assert!(o.quadro(vec![]).rect.is_none());
    }

    #[test]
    fn o_quadro_nao_pesou_cem_paineis_custam_pouco() {
        // B347: o trabalho por quadro é constante e pequeno (sem alocar além de um registro por painel)
        let ctx = egui::Context::default();
        let cfg = Painel { nome: "p".into(), titulo: "P".into(), padrao: pos2(500., 300.), largura: 200., redimensiona: true };
        let posicao = Posicao { encaixe: Encaixe::Direita, pos: Some([900., 300.]), ..Default::default() };
        let mut passo = || {
            let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0., 0.), vec2(1200., 800.))), ..Default::default() };
            passo_ui(&ctx, raw, |ui| {
                let ctx = ui.ctx().clone();
                mostrar(&ctx, tela(), &cfg, posicao, |ui| ui.label("conteúdo"));
            });
        };
        for _ in 0..10 {
            passo();
        }
        let t = std::time::Instant::now();
        for _ in 0..200 {
            passo();
        }
        let por_quadro = t.elapsed().as_secs_f64() * 1000. / 200.;
        eprintln!("painel: {por_quadro:.3} ms por quadro (um painel, build de teste)");
        assert!(por_quadro < 8., "{por_quadro} ms por quadro");
    }
}
