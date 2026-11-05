// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painéis livres e encaixáveis sobre a viewport: arrastam para qualquer ponto, encaixam nas bordas com
//! uma guia e guardam a posição por bancada nas preferências.
//!
//! Código próprio em vez de biblioteca de docking: essas bibliotecas dividem a janela em áreas que
//! empurram umas às outras, e aqui a viewport ocupa tudo e os painéis flutuam translúcidos por cima.
//! O encaixe, portanto, só alinha o painel à borda; nunca reserva espaço. Cada painel é um
//! `egui::Window`, que já traz arrastar, redimensionar e a ordem entre janelas.

use crate::settings::{Bench, Settings};
use crate::theme;
use eframe::egui::{self, Align2, FontId, Id, Pos2, Rect, Stroke, Vec2};
use serde::{Deserialize, Serialize};

/// Margem entre o painel e a borda em que encaixa.
const MARGEM: f32 = 8.;
/// Distância do ponteiro até a borda para o encaixe valer.
const ALCANCE: f32 = 44.;
/// Largura da faixa da árvore: o encaixe à esquerda fica ao lado dela.
pub const FAIXA_ARVORE: f32 = 290.;
/// Altura reservada ao cubo de vistas no canto direito.
pub const FAIXA_CUBO: f32 = 128.;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Encaixe {
    #[default]
    Livre,
    Esquerda,
    Direita,
    Topo,
    Base,
}

/// Onde um painel está. Sem `pos`, ele usa a posição padrão.
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Posicao {
    pub encaixe: Encaixe,
    pub pos: Option<[f32; 2]>,
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
            .map_or(Posicao { encaixe: padrao, pos: None }, |p| p.posicao)
    }

    pub fn guardar_painel(&mut self, nome: &str, posicao: Posicao) {
        let bancada = self.bench;
        match self.paineis.iter_mut().find(|p| p.bancada == bancada && p.nome == nome) {
            Some(p) => p.posicao = posicao,
            None => self.paineis.push(PainelSalvo { bancada, nome: nome.to_string(), posicao }),
        }
    }

    /// Esquece a disposição da bancada atual; os painéis voltam ao padrão no próximo quadro.
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
        Encaixe::Livre => base,
        Encaixe::Esquerda => egui::pos2(area.left() + FAIXA_ARVORE + MARGEM, base.y),
        Encaixe::Direita => egui::pos2(area.right() - tam.x - MARGEM, base.y),
        Encaixe::Topo => egui::pos2(base.x, area.top() + MARGEM),
        Encaixe::Base => egui::pos2(base.x, area.bottom() - tam.y - MARGEM),
    };
    dentro(alvo, tam, area)
}

/// Faixa desenhada como guia enquanto o painel é arrastado perto de uma borda.
pub fn faixa_da_guia(e: Encaixe, tam: Vec2, area: Rect) -> Option<Rect> {
    let (x0, x1, y0, y1) = (area.left() + FAIXA_ARVORE, area.right(), area.top(), area.bottom());
    Some(match e {
        Encaixe::Livre => return None,
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
    pub largura: f32,
    pub redimensiona: bool,
}

/// O que aconteceu com o painel neste quadro.
pub struct Saida<R> {
    pub conteudo: Option<R>,
    /// nova posição, quando o arraste terminou neste quadro
    pub movido: Option<Posicao>,
    /// borda em que foi solto, quando foi solto numa
    pub solto_em: Option<Encaixe>,
    pub rect: Option<Rect>,
}

/// Quadro translúcido dos painéis, nas cores do tema.
pub fn quadro(style: &egui::Style) -> egui::Frame {
    let mut q = egui::Frame::window(style).fill(style.visuals.window_fill.gamma_multiply(0.93));
    q.shadow.color = q.shadow.color.gamma_multiply(0.5);
    q
}

fn guia(ctx: &egui::Context, faixa: Rect, rotulo: &str) {
    let destaque = theme::accent(&ctx.global_style().visuals);
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, Id::unique("guia_encaixe")));
    p.rect_filled(faixa, 6., destaque.gamma_multiply(0.14));
    p.rect_stroke(faixa, 6., Stroke::new(1.5, destaque.gamma_multiply(0.8)), egui::StrokeKind::Inside);
    p.text(faixa.center(), Align2::CENTER_CENTER, rotulo, FontId::proportional(13.), destaque);
}

/// Desenha o painel. Quem chama guarda `movido` nas preferências.
pub fn mostrar<R>(ctx: &egui::Context, area: Rect, cfg: &Painel, posicao: Posicao, conteudo: impl FnOnce(&mut egui::Ui) -> R) -> Saida<R> {
    let id = Id::unique(("painel", cfg.nome.as_str()));
    let (id_arrasto, id_tam) = (id.with("arrasto"), id.with("tam"));
    let arrastando = ctx.data(|d| d.get_temp::<bool>(id_arrasto)).unwrap_or(false);
    let tam = ctx.data(|d| d.get_temp::<Vec2>(id_tam)).unwrap_or(egui::vec2(cfg.largura, 200.));
    let estilo = ctx.global_style();
    let alvo = colocar(posicao, cfg.padrao, tam, area);
    let mut w = egui::Window::new(egui::RichText::new(&cfg.titulo).small().strong())
        .id(id)
        .frame(quadro(&estilo))
        .collapsible(true)
        .resizable([cfg.redimensiona, false])
        .default_width(cfg.largura)
        .max_height((area.height() - 2. * MARGEM).max(120.))
        .constrain_to(area);
    if !arrastando {
        w = w.current_pos(alvo);
    }
    let r = w.show(ctx, conteudo);
    let mut saida = Saida { conteudo: None, movido: None, solto_em: None, rect: None };
    let Some(r) = r else { return saida };
    let rect = r.response.rect;
    saida.rect = Some(rect);
    saida.conteudo = r.inner;
    ctx.data_mut(|d| d.insert_temp(id_tam, rect.size()));
    let (ponteiro, apertado, origem) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.pointer.press_origin()));
    let borda = ponteiro.and_then(|p| borda_perto(p, area));
    // o egui move a janela pelo título sem marcar a resposta dela como arrastada; quem denuncia o
    // arraste é a janela sair de onde a pusemos com o botão apertado sobre ela
    let pegou = origem.is_some_and(|o| Rect::from_min_size(alvo, tam).expand(4.).contains(o));
    let saiu_do_lugar = (rect.min - alvo).length() > 0.5;
    if apertado && (arrastando || (pegou && saiu_do_lugar)) {
        ctx.data_mut(|d| d.insert_temp(id_arrasto, true));
        if let Some(f) = borda.and_then(|e| faixa_da_guia(e, rect.size(), area)) {
            guia(ctx, f, crate::i18n::tr("solte para encaixar"));
        }
        ctx.request_repaint();
    } else if arrastando {
        ctx.data_mut(|d| d.remove::<bool>(id_arrasto));
        saida.solto_em = borda;
        saida.movido = Some(Posicao { encaixe: borda.unwrap_or_default(), pos: Some([rect.min.x, rect.min.y]) });
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

    fn prefs(b: Bench) -> Settings {
        Settings { bench: b, ..Default::default() }
    }

    fn tela() -> Rect {
        Rect::from_min_size(pos2(0., 100.), vec2(1200., 700.))
    }

    #[test]
    fn painel_fora_da_tela_volta_para_a_area_visivel() {
        let t = vec2(300., 200.);
        for fora in [pos2(5000., 5000.), pos2(-900., -900.), pos2(1190., 300.)] {
            let p = colocar(Posicao { encaixe: Encaixe::Livre, pos: Some([fora.x, fora.y]) }, pos2(0., 0.), t, tela());
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
        let posicao = Posicao { encaixe: Encaixe::Direita, pos: Some([900., 300.]) };
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
        let aqui = Posicao { encaixe: Encaixe::Livre, pos: Some([400., 300.]) };
        s.guardar_painel("comando", aqui);
        s.bench = Bench::Montagem;
        assert_eq!(s.painel("comando", Encaixe::Direita), Posicao { encaixe: Encaixe::Direita, pos: None });
        s.guardar_painel("comando", Posicao { encaixe: Encaixe::Topo, pos: None });
        s.restaurar_paineis();
        assert_eq!(s.painel("comando", Encaixe::Livre).pos, None);
        s.bench = Bench::Solidos;
        assert_eq!(s.painel("comando", Encaixe::Direita), aqui);
    }

    #[test]
    fn disposicao_sobrevive_a_fechar_e_reabrir() {
        let mut s = prefs(Bench::Superficies);
        let aqui = Posicao { encaixe: Encaixe::Base, pos: Some([320., 640.]) };
        s.guardar_painel("barra_direita", aqui);
        let texto = serde_json::to_string(&s).expect("json");
        let lido: Settings = serde_json::from_str(&texto).expect("json");
        assert_eq!(lido.painel("barra_direita", Encaixe::Livre), aqui);
    }

    #[test]
    fn preferencias_antigas_sem_paineis_continuam_validas() {
        let mut v = serde_json::to_value(Settings::default()).expect("json");
        v.as_object_mut().expect("objeto").remove("paineis");
        let s: Settings = serde_json::from_value(v).expect("json");
        assert!(s.paineis.is_empty());
    }

    /// Passa um quadro do painel com o ponteiro em `p` e o botão no estado dado.
    fn passo(ctx: &egui::Context, cfg: &Painel, pos: Posicao, evs: Vec<egui::Event>) -> (Option<Posicao>, Option<Rect>) {
        let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0., 0.), vec2(1200., 800.))), events: evs, ..Default::default() };
        let mut out = (None, None);
        let mut saida = ctx.run_pass(raw, |ctx| {
            let s = mostrar(ctx, tela(), cfg, pos, |ui| ui.label("conteúdo"));
            out = (s.movido, s.rect);
        });
        saida.textures_delta.clear();
        out
    }

    fn mouse(p: Pos2, down: bool) -> Vec<egui::Event> {
        vec![egui::Event::PointerMoved(p), egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: down, modifiers: Default::default() }]
    }

    #[test]
    fn arrastar_pelo_titulo_ate_a_borda_encaixa_e_no_meio_fica_livre() {
        let ctx = egui::Context::default();
        let cfg = Painel { nome: "teste".into(), titulo: "Teste".into(), padrao: pos2(500., 300.), largura: 240., redimensiona: false };
        let mut pos = Posicao::default();
        let arrastar = |pos: &mut Posicao, ate: Pos2| {
            let mut r = None;
            for _ in 0..3 {
                r = passo(&ctx, &cfg, *pos, vec![]).1;
            }
            let titulo = r.expect("painel").left_top() + vec2(90., 12.);
            passo(&ctx, &cfg, *pos, vec![egui::Event::PointerMoved(titulo)]);
            passo(&ctx, &cfg, *pos, mouse(titulo, true));
            let meio = titulo + (ate - titulo) * 0.5;
            passo(&ctx, &cfg, *pos, vec![egui::Event::PointerMoved(meio)]);
            passo(&ctx, &cfg, *pos, vec![egui::Event::PointerMoved(ate)]);
            let solto = passo(&ctx, &cfg, *pos, mouse(ate, false)).0;
            let solto = solto.or_else(|| passo(&ctx, &cfg, *pos, vec![]).0);
            *pos = solto.expect("terminou o arraste");
        };
        arrastar(&mut pos, pos2(1195., 500.));
        assert_eq!(pos.encaixe, Encaixe::Direita);
        arrastar(&mut pos, pos2(700., 450.));
        assert_eq!(pos.encaixe, Encaixe::Livre);
    }
}
