// SPDX-License-Identifier: LGPL-2.1-or-later
//! Árvore desenhada direto sobre o modelo: sem painel, com nós ligados por linhas, caixinha de abrir e
//! fechar, ícone por tipo, zoom e deslocamento próprios.
//!
//! A árvore mora na mesma camada da viewport, depois dela. Por isso só as linhas capturam o mouse: o
//! vazio entre elas continua sendo o modelo, para girar e selecionar. Uma `Area` do egui bloquearia o
//! retângulo inteiro. O zoom escala as fontes e os espaçamentos da própria `Ui`, em vez de escalar a
//! camada, para o texto continuar nítido.

use crate::theme;
use eframe::egui::{self, epaint::TextShape, Color32, Id, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, WidgetText};
use fenkra_core::{BoolOp, FeatureKind};
use std::cell::RefCell;
use std::hash::Hash;

pub const ZOOM_MIN: f32 = 0.6;
pub const ZOOM_MAX: f32 = 2.0;
/// Largura máxima da área da árvore; o que passar disso se alcança arrastando.
pub const LARGURA: f32 = 320.;
/// Sobra mínima da árvore dentro da área, para ela nunca sumir de vez ao ser arrastada.
const SOBRA: f32 = 48.;

/// Etapas de "localizar item ativo": um quadro abre tudo para achar o caminho, o seguinte centraliza.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Localizar {
    #[default]
    Nada,
    Abrir,
    Centrar,
}

/// Estado de exibição da árvore. Nada aqui pertence ao documento nem à seleção.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct VistaArvore {
    pub zoom: f32,
    pub pan: Vec2,
    pub localizar: Localizar,
    /// recolhe todos os ramos no próximo quadro
    pub recolher: bool,
    /// retângulo das linhas no quadro anterior
    pub conteudo: Rect,
    /// área da árvore no quadro anterior
    pub regiao: Rect,
}

impl Default for VistaArvore {
    fn default() -> Self {
        Self { zoom: 1., pan: Vec2::ZERO, localizar: Localizar::Nada, recolher: false, conteudo: Rect::NOTHING, regiao: Rect::NOTHING }
    }
}

impl VistaArvore {
    /// Muda o zoom mantendo parado o ponto da árvore sob `ponteiro`.
    pub fn zoom_em(&mut self, ponteiro: Pos2, fator: f32) {
        let novo = (self.zoom * fator).clamp(ZOOM_MIN, ZOOM_MAX);
        self.pan = pan_com_zoom(self.pan, self.regiao.min, ponteiro, self.zoom, novo);
        self.zoom = novo;
    }

    /// A árvore passa da altura da área: só então a roda rola a árvore em vez do modelo.
    pub fn transborda(&self) -> bool {
        self.conteudo.is_positive() && self.conteudo.height() > self.regiao.height()
    }

    /// Volta ao enquadramento: início da árvore no canto, com zoom que a faz caber quando possível.
    pub fn enquadrar(&mut self) {
        self.pan = Vec2::ZERO;
        if self.conteudo.is_positive() && self.regiao.is_positive() {
            let altura = self.conteudo.height() / self.zoom;
            self.zoom = (self.regiao.height() / altura).clamp(ZOOM_MIN, 1.);
        } else {
            self.zoom = 1.;
        }
    }
}

/// Deslocamento que mantém o conteúdo sob o ponteiro ao trocar de zoom.
pub fn pan_com_zoom(pan: Vec2, origem: Pos2, ponteiro: Pos2, z0: f32, z1: f32) -> Vec2 {
    let ponto = (ponteiro - origem - pan) / z0;
    ponteiro - origem - ponto * z1
}

/// Deslocamento que traz `alvo` para dentro de `regiao`; se já está visível, não mexe.
pub fn trazer_a_vista(pan: Vec2, alvo: Rect, regiao: Rect) -> Vec2 {
    if regiao.shrink(4.).contains_rect(alvo) {
        return pan;
    }
    let dy = regiao.center().y - alvo.center().y;
    let dx = if alvo.left() < regiao.left() || alvo.right() > regiao.right() { regiao.left() + 8. - alvo.left() } else { 0. };
    pan + egui::vec2(dx, dy)
}

/// Mantém parte da árvore na área, por mais que se arraste.
pub fn limitar_pan(pan: Vec2, conteudo: Vec2, regiao: Vec2) -> Vec2 {
    let eixo = |p: f32, c: f32, r: f32| {
        let (lo, hi) = (SOBRA - c, r - SOBRA);
        if lo < hi { p.clamp(lo, hi) } else { p }
    };
    egui::vec2(eixo(pan.x, conteudo.x, regiao.x), eixo(pan.y, conteudo.y, regiao.y))
}

#[derive(Default)]
struct Nivel {
    x: f32,
    ultimo_y: f32,
}

/// O que se acumula enquanto a árvore é desenhada num quadro.
#[derive(Default)]
struct Quadro {
    niveis: Vec<Nivel>,
    caminho: Vec<Id>,
    abrir_tudo: bool,
    recolher: bool,
    achado: Option<(Vec<Id>, Rect)>,
}

thread_local! {
    // A interface desenha numa thread só, e cada linha da árvore fala com o nó em que está.
    static QUADRO: RefCell<Quadro> = RefCell::default();
}

fn quadro<R>(f: impl FnOnce(&mut Quadro) -> R) -> R {
    QUADRO.with(|q| f(&mut q.borrow_mut()))
}

pub fn comecar(v: &VistaArvore) {
    quadro(|q| *q = Quadro { abrir_tudo: v.localizar == Localizar::Abrir, recolher: v.recolher, ..Default::default() });
}

/// Fecha o quadro. Devolve `false` quando "localizar" não achou item ativo.
pub fn terminar(ctx: &egui::Context, v: &mut VistaArvore) -> bool {
    let q = quadro(std::mem::take);
    v.recolher = false;
    let mut achou = true;
    match v.localizar {
        Localizar::Nada => {}
        Localizar::Abrir => match q.achado {
            Some((caminho, _)) => {
                for id in caminho {
                    ctx.data_mut(|d| d.insert_persisted(id, true));
                }
                v.localizar = Localizar::Centrar;
                ctx.request_repaint();
            }
            None => {
                v.localizar = Localizar::Nada;
                achou = false;
            }
        },
        Localizar::Centrar => {
            if let Some((_, r)) = q.achado {
                v.pan = trazer_a_vista(v.pan, r, v.regiao);
            }
            v.localizar = Localizar::Nada;
        }
    }
    achou
}

/// Avisa que a linha em `r` é o item ativo.
pub fn achar(r: Rect) {
    quadro(|q| {
        if q.achado.is_none() {
            q.achado = Some((q.caminho.clone(), r));
        }
    });
}

/// Estilo da `Ui` da árvore: zoom e fundos transparentes.
pub fn estilo(ui: &mut Ui, zoom: f32) {
    let st = ui.style_mut();
    for f in st.text_styles.values_mut() {
        f.size *= zoom;
    }
    let sp = &mut st.spacing;
    sp.item_spacing = egui::vec2(4., 1.) * zoom;
    sp.interact_size *= zoom;
    sp.interact_size.y = 18. * zoom;
    sp.button_padding = egui::vec2(3., 1.) * zoom;
    sp.indent = 17. * zoom;
    sp.icon_width *= zoom;
    sp.icon_width_inner *= zoom;
    sp.icon_spacing *= zoom;
    let v = &mut st.visuals;
    let destaque = theme::accent(v);
    v.indent_has_left_vline = false;
    v.selection.bg_fill = destaque.gamma_multiply(0.45);
    let w = &mut v.widgets;
    w.inactive.weak_bg_fill = Color32::TRANSPARENT;
    w.inactive.bg_stroke = Stroke::NONE;
    w.hovered.weak_bg_fill = destaque.gamma_multiply(0.22);
    w.hovered.bg_stroke = Stroke::NONE;
    w.active.weak_bg_fill = destaque.gamma_multiply(0.32);
    w.open.weak_bg_fill = destaque.gamma_multiply(0.16);
}

/// Fundo de campo de texto dentro da árvore: um véu discreto do tema em vez do `extreme_bg_color`
/// (branco opaco no tema claro), que quebrava a árvore transparente.
pub fn campo_discreto(ui: &mut Ui, f: impl FnOnce(&mut Ui)) {
    ui.scope(|ui| {
        let v = ui.visuals_mut();
        let texto = v.text_color();
        v.extreme_bg_color = texto.gamma_multiply(0.07);
        v.widgets.inactive.bg_stroke = Stroke::new(1., texto.gamma_multiply(0.25));
        v.widgets.hovered.bg_stroke = Stroke::new(1., texto.gamma_multiply(0.45));
        f(ui);
    });
}

fn traco(ui: &Ui) -> Stroke {
    Stroke::new(1., ui.visuals().text_color().gamma_multiply(0.45))
}

fn altura_linha(ui: &Ui) -> f32 {
    ui.spacing().interact_size.y
}

/// Ramo horizontal da próxima linha até o tronco do nó pai. Chame antes de desenhar a linha.
pub fn marca(ui: &Ui) {
    marca_em(ui, 0.)
}

/// Como `marca`, para linhas que recuam `recuo` por conta própria.
pub fn marca_em(ui: &Ui, recuo: f32) {
    let y = ui.cursor().top() + altura_linha(ui) / 2.;
    let fim = ui.cursor().left() + recuo - 1.;
    let x = quadro(|q| {
        q.niveis.last_mut().map(|n| {
            n.ultimo_y = y;
            n.x
        })
    });
    if let Some(x) = x {
        ui.painter().hline(x..=fim, y, traco(ui));
    }
}

/// Caixinha com sinal de mais ou menos. Devolve a resposta do clique.
pub fn caixa(ui: &mut Ui, aberto: bool) -> Response {
    let h = altura_linha(ui);
    let (r, resp) = ui.allocate_exact_size(egui::vec2(h * 0.72, h), Sense::click());
    let lado = (h * 0.56).round();
    let q = Rect::from_center_size(r.center(), egui::vec2(lado, lado));
    let cor = if resp.hovered() { theme::accent(ui.visuals()) } else { ui.visuals().text_color().gamma_multiply(0.7) };
    let p = ui.painter();
    p.rect_filled(q, 1.5, ui.visuals().panel_fill.gamma_multiply(0.7));
    p.rect_stroke(q, 1.5, Stroke::new(1., cor), egui::StrokeKind::Inside);
    let m = lado * 0.27;
    p.hline(q.center().x - m..=q.center().x + m, q.center().y, Stroke::new(1.2, cor));
    if !aberto {
        p.vline(q.center().x, q.center().y - m..=q.center().y + m, Stroke::new(1.2, cor));
    }
    resp
}

/// Ícone de tipo na linha da árvore, no tamanho do zoom.
pub fn icone(ui: &mut Ui, nome: &str) {
    let h = (altura_linha(ui) * 0.86).round();
    ui.add(egui::Image::new(crate::icons::get(nome)).fit_to_exact_size(egui::vec2(h, h)));
}

/// Nó aberto ou fechado, à espera do corpo.
#[must_use = "chame .corpo() para desenhar os filhos"]
pub struct No<R> {
    id: Id,
    aberto: bool,
    x: f32,
    y: f32,
    pub cabeca: R,
}

impl<R> No<R> {
    /// Desenha os filhos, se o nó está aberto, e devolve o resultado da cabeça.
    pub fn corpo(self, ui: &mut Ui, f: impl FnOnce(&mut Ui)) -> R {
        if self.aberto {
            let ultimo = ramo(ui, self.x, self.y, Some(self.id), f);
            ui.painter().vline(self.x, self.y..=ultimo, traco(ui));
        }
        self.cabeca
    }
}

fn ramo(ui: &mut Ui, x: f32, y: f32, id: Option<Id>, f: impl FnOnce(&mut Ui)) -> f32 {
    quadro(|q| {
        q.niveis.push(Nivel { x, ultimo_y: y });
        q.caminho.extend(id);
    });
    let salt = id.unwrap_or(Id::unique("raiz_arvore")).with("corpo");
    ui.indent(salt, f);
    quadro(|q| {
        if id.is_some() {
            q.caminho.pop();
        }
        q.niveis.pop().map_or(y, |n| n.ultimo_y)
    })
}

/// Filhos diretos da raiz, que não se fecha: use logo depois da linha da raiz.
pub fn filhos(ui: &mut Ui, f: impl FnOnce(&mut Ui)) {
    let h = altura_linha(ui);
    let x = ui.cursor().left() + h * 0.36;
    let y = ui.cursor().top();
    let ultimo = ramo(ui, x, y, None, f);
    ui.painter().vline(x, y..=ultimo, traco(ui));
}

/// Tronco e marcas para uma sublista sem nó próprio, como as entradas de um comando.
pub fn sublista(ui: &mut Ui, x: f32, f: impl FnOnce(&mut Ui)) {
    let y = ui.cursor().top();
    let ultimo = ramo(ui, x, y, None, f);
    ui.painter().vline(x, y..=ultimo, traco(ui));
}

fn carregar(ui: &Ui, id: Id, padrao: bool) -> bool {
    let mut aberto = ui.ctx().data_mut(|d| *d.get_persisted_mut_or(id, padrao));
    if quadro(|q| q.recolher) && aberto {
        aberto = false;
        ui.ctx().data_mut(|d| d.insert_persisted(id, false));
    }
    aberto
}

/// Nó com cabeça própria. `cabeca` desenha o que vem depois da caixinha, numa fila horizontal.
pub fn no<R>(ui: &mut Ui, salt: impl Hash + std::fmt::Debug, padrao: bool, cabeca: impl FnOnce(&mut Ui) -> R) -> No<R> {
    let id = ui.make_persistent_id(salt);
    let aberto = carregar(ui, id, padrao);
    marca(ui);
    let mut x = 0.;
    let mut y = 0.;
    let r = ui
        .horizontal(|ui| {
            let c = caixa(ui, aberto);
            x = c.rect.center().x;
            y = c.rect.center().y + altura_linha(ui) * 0.28;
            if c.clicked() {
                ui.ctx().data_mut(|d| d.insert_persisted(id, !aberto));
            }
            cabeca(ui)
        })
        .inner;
    No { id, aberto: aberto || quadro(|q| q.abrir_tudo), x, y, cabeca: r }
}

/// Grupo simples: clicar no nome também abre e fecha.
pub fn grupo(ui: &mut Ui, salt: impl Hash + std::fmt::Debug, icone_nome: Option<&str>, texto: impl Into<WidgetText>, padrao: bool) -> No<Response> {
    let id = ui.make_persistent_id(&salt);
    no(ui, salt, padrao, |ui| {
        if let Some(n) = icone_nome {
            icone(ui, n);
        }
        let r = ui.selectable_label(false, texto);
        if r.clicked() {
            let aberto: bool = ui.ctx().data_mut(|d| *d.get_persisted_mut_or(id, padrao));
            ui.ctx().data_mut(|d| d.insert_persisted(id, !aberto));
        }
        r
    })
}

/// Linha simples com ícone de tipo.
pub fn linha(ui: &mut Ui, icone_nome: &str, selecionado: bool, texto: impl Into<WidgetText>) -> Response {
    marca(ui);
    let r = ui
        .horizontal(|ui| {
            ui.add_space(altura_linha(ui) * 0.2);
            icone(ui, icone_nome);
            ui.selectable_label(selecionado, texto)
        })
        .inner;
    if selecionado {
        achar(r.rect);
    }
    r
}

/// Contorno leve no texto, no lugar de um painel atrás dele.
///
/// O egui não sabe desenhar texto com contorno, então reservamos um lugar antes da árvore e, depois de
/// pronta, preenchemos esse lugar com cópias de cada texto em cor de contraste. Assim vale para qualquer
/// componente dentro da árvore, sem mudar um por um.
pub struct Contorno {
    pintor: egui::Painter,
    lugar: egui::layers::ShapeIdx,
}

impl Contorno {
    pub fn reservar(ui: &Ui, clip: Rect) -> Self {
        let pintor = ui.painter().with_clip_rect(clip);
        let lugar = pintor.add(Shape::Noop);
        Self { pintor, lugar }
    }

    pub fn aplicar(self, ui: &Ui) {
        let cor = contraste(ui.visuals().text_color());
        let mut copias = vec![];
        let mut i = 0;
        self.pintor.for_each_shape(|s| {
            if let (true, Shape::Text(t)) = (i > self.lugar.0, &s.shape) {
                copias.extend(contornar(t, cor));
            }
            i += 1;
        });
        self.pintor.set(self.lugar, Shape::Vec(copias));
    }
}

fn contornar(t: &TextShape, cor: Color32) -> impl Iterator<Item = Shape> + use<'_> {
    [(-1., 0.), (1., 0.), (0., -1.), (0., 1.)].into_iter().map(move |(x, y)| {
        let mut c = t.clone();
        c.pos += egui::vec2(x, y);
        c.override_text_color = Some(cor);
        c.underline = Stroke::NONE;
        Shape::Text(c)
    })
}

/// Preto translúcido sob texto claro; branco translúcido sob texto escuro.
pub fn contraste(texto: Color32) -> Color32 {
    let luz = 0.3 * texto.r() as f32 + 0.59 * texto.g() as f32 + 0.11 * texto.b() as f32;
    if luz > 128. { Color32::from_black_alpha(130) } else { Color32::from_white_alpha(170) }
}

/// Botãozinho da fila de controles da árvore.
pub fn controle(ui: &mut Ui, nome: &str, dica: &str) -> bool {
    let img = egui::Image::new(crate::icons::get(nome)).fit_to_exact_size(egui::vec2(15., 15.));
    ui.add(egui::Button::image(img).frame(false)).on_hover_text(dica).clicked()
}

/// Desenho do tipo do comando. O `match` sem curinga obriga todo comando novo a escolher um ícone.
pub fn icone_comando(k: &FeatureKind) -> &'static str {
    use FeatureKind::*;
    match k {
        Sketch(_) => "sketch",
        Extrude(e) if e.op == BoolOp::Cut => "recorte",
        Extrude(_) => "extrusao",
        Revolve(_) => "revolucao",
        Hole(_) => "furo",
        Fillet(_) => "filete",
        Chamfer(_) => "chanfro",
        Sweep(_) => "sweep_fixo",
        Loft(_) => "multisecoes",
        Primitive(_) => "prim_caixa",
        Shell(_) => "casca",
        Draft(_) => "saida",
        Thickness(_) => "espessura",
        Pattern(_) => "padrao_linear",
        Mirror(_) => "espelho",
        Scale(_) => "escala",
        Stiffener(_) => "reforco",
        Thread(_) => "rosca",
        Combine(_) => "bool_unir",
        SplitBody(_) => "hib_dividir",
        ThickSurface(_) => "hib_engrossar",
        CloseSurface(_) => "hib_fechar",
        SewSurface(_) => "hib_costurar",
        BoundaryFill(_) => "hib_preencher",
        MeshImport(_) => "hib_malha",
        LinkedBody(_) => "asm_link",
        MoveBody(_) => "xf_mover",
        SmWall(_) => "sm_parede",
        SmFlange(_) => "sm_aba",
        SmHem(_) => "sm_bainha",
        SmJoggle(_) => "sm_joggle",
        SmBend(_) => "sm_dobra_linha",
        SmLouver(_) => "sm_veneziana",
        SmStamp(_) => "sm_estampo",
        SmFlangedHole(_) => "sm_furo_flangeado",
        SmCut(_) => "sm_recorte",
        SmProfileFlange(_) => "sm_aba_perfil",
        SmCurveFlange(_) => "sm_aba_curva",
        SmFlangeLoop(_) => "sm_aba",
        SmCornerRelief(_) => "sm_alivio_canto",
        SmArcSlot(_) => "sm_rasgo_circular",
        SmSurfFlange(_) => "sm_aba_superficie",
        StructMember(_) => "st_membro",
        StructPlate(_) => "st_placa",
        StructGusset(_) => "st_reforco",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    fn regiao() -> Rect {
        Rect::from_min_size(pos2(10., 100.), vec2(300., 400.))
    }

    #[test]
    fn zoom_mantem_parado_o_ponto_sob_o_ponteiro() {
        let mut v = VistaArvore { regiao: regiao(), pan: vec2(5., -30.), ..Default::default() };
        let p = pos2(120., 260.);
        let antes = (p - v.regiao.min - v.pan) / v.zoom;
        v.zoom_em(p, 1.5);
        let depois = (p - v.regiao.min - v.pan) / v.zoom;
        assert!((antes - depois).length() < 1e-3);
        assert!((v.zoom - 1.5).abs() < 1e-6);
    }

    #[test]
    fn zoom_fica_na_faixa() {
        let mut v = VistaArvore { regiao: regiao(), ..Default::default() };
        v.zoom_em(pos2(0., 0.), 100.);
        assert_eq!(v.zoom, ZOOM_MAX);
        v.zoom_em(pos2(0., 0.), 0.001);
        assert_eq!(v.zoom, ZOOM_MIN);
    }

    #[test]
    fn localizar_traz_item_escondido_e_nao_mexe_em_item_visivel() {
        let r = regiao();
        let abaixo = Rect::from_min_size(pos2(30., 900.), vec2(80., 18.));
        let pan = trazer_a_vista(Vec2::ZERO, abaixo, r);
        assert!(r.contains_rect(abaixo.translate(pan)));
        let visivel = Rect::from_min_size(pos2(30., 200.), vec2(80., 18.));
        assert_eq!(trazer_a_vista(vec2(3., 4.), visivel, r), vec2(3., 4.));
    }

    #[test]
    fn arrastar_nao_perde_a_arvore_de_vista() {
        let conteudo = vec2(200., 800.);
        let reg = vec2(300., 400.);
        let p = limitar_pan(vec2(-9000., 9000.), conteudo, reg);
        assert!(p.x + conteudo.x >= SOBRA && p.y <= reg.y - SOBRA);
    }

    #[test]
    fn enquadrar_faz_caber_e_zera_o_deslocamento() {
        let mut v = VistaArvore { zoom: 1.6, pan: vec2(40., -300.), regiao: regiao(), conteudo: Rect::from_min_size(pos2(0., 0.), vec2(200., 1600.)), ..Default::default() };
        v.enquadrar();
        assert_eq!(v.pan, Vec2::ZERO);
        assert!((v.zoom - 0.6).abs() < 1e-6 || v.conteudo.height() / 1.6 * v.zoom <= 400.);
    }

    /// Uma árvore de dois ramos com o item ativo no segundo, desenhada fora de um quadro real.
    fn desenhar(ctx: &egui::Context, v: &mut VistaArvore, ordem: &mut Vec<&'static str>) {
        let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0., 0.), vec2(800., 600.))), ..Default::default() };
        let mut saida = ctx.run_ui(raw, |root| {
            comecar(v);
            let mut ui = root.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(10., 10.) + v.pan, vec2(300., 900.))));
            estilo(&mut ui, v.zoom);
            for (nome, filhos) in [("a", ["a1", "a2"]), ("b", ["b1", "alvo"])] {
                ordem.push(nome);
                let _ = grupo(&mut ui, nome, None, nome, false).corpo(&mut ui, |ui| {
                    for f in filhos {
                        ordem.push(f);
                        let r = ui.selectable_label(f == "alvo", f);
                        if f == "alvo" {
                            achar(r.rect);
                        }
                    }
                });
            }
            terminar(root.ctx(), v);
        });
        saida.textures_delta.clear();
    }

    #[test]
    fn localizar_abre_so_o_caminho_do_item_ativo() {
        let ctx = egui::Context::default();
        let mut v = VistaArvore { regiao: regiao(), localizar: Localizar::Abrir, ..Default::default() };
        let mut ordem = vec![];
        desenhar(&ctx, &mut v, &mut ordem);
        assert_eq!(v.localizar, Localizar::Centrar);
        ordem.clear();
        desenhar(&ctx, &mut v, &mut ordem);
        assert_eq!(v.localizar, Localizar::Nada);
        assert_eq!(ordem, ["a", "b", "b1", "alvo"]);
    }

    #[test]
    fn zoom_e_deslocamento_nao_mudam_a_ordem_das_linhas() {
        let ctx = egui::Context::default();
        let mut v = VistaArvore { regiao: regiao(), ..Default::default() };
        let (mut a, mut b) = (vec![], vec![]);
        desenhar(&ctx, &mut v, &mut a);
        v.zoom_em(pos2(50., 50.), 1.7);
        v.pan += vec2(0., -120.);
        desenhar(&ctx, &mut v, &mut b);
        assert_eq!(a, b);
    }

    #[test]
    fn todo_comando_da_arvore_tem_desenho() {
        let fonte = include_str!("tree_ui.rs");
        let inicio = fonte.find("pub fn icone_comando").expect("função");
        let fim = fonte[inicio..].find("\n}").expect("fim") + inicio;
        let nomes: Vec<&str> = fonte[inicio..fim].lines().filter_map(|l| l.split("=> \"").nth(1)).filter_map(|l| l.split('"').next()).collect();
        assert!(nomes.len() > 40);
        for n in nomes {
            assert!(crate::icons::existe(n), "{n} sem desenho");
        }
    }

    #[test]
    fn contorno_contrasta_com_o_texto() {
        assert_eq!(contraste(Color32::WHITE), Color32::from_black_alpha(130));
        assert_eq!(contraste(Color32::BLACK), Color32::from_white_alpha(170));
    }
}
