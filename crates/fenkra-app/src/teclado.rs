// SPDX-License-Identifier: LGPL-2.1-or-later
//! Uso por teclado (B321): o que o mouse faz na vista e na árvore também sai das teclas, com o foco
//! sempre visível.
//!
//! - Tab e Shift+Tab passam o foco por botões, campos e listas (o egui faz isso); Enter ou Espaço
//!   acionam o botão em foco, e todo botão desenhado à mão pinta o anel de foco (`anel_de_foco`).
//! - Sem nada em foco: setas giram a vista 15°, Shift+setas a movem, Page Up e Page Down aproximam e
//!   afastam. Esc solta o foco e volta para a vista.
//! - Com um item da árvore em foco (clicado, ou com o foco do Tab), ↑ e ↓ andam pela árvore (B449);
//!   um clique fora da árvore ou o Esc devolvem as setas à vista.
//! - Alt+↑ e Alt+↓ escolhem o comando anterior ou o seguinte da árvore; Enter abre o escolhido para
//!   edição; com o comando aberto, Enter conclui e Esc cancela.
//! - Ctrl+K abre a busca de comandos, que se percorre com setas e Enter.
//!
//! As decisões ficam em funções puras (`movimento`, `vizinho_na_arvore`), testadas sem janela.

use eframe::egui;

/// Passo de giro por toque de seta, em radianos (15°).
pub const GIRO: f32 = std::f32::consts::PI / 12.;
/// Fração da distância da câmera que Shift+seta move o alvo.
pub const PASSO_MOVER: f32 = 0.08;
/// Fator de aproximação por Page Up (Page Down usa o inverso).
pub const ZOOM: f32 = 0.8;

/// Teclas lidas no quadro, sem o egui (para teste).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Teclas {
    pub esquerda: bool,
    pub direita: bool,
    pub cima: bool,
    pub baixo: bool,
    pub pg_cima: bool,
    pub pg_baixo: bool,
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}

impl Teclas {
    pub fn ler(i: &egui::InputState) -> Self {
        use egui::Key as K;
        Teclas {
            esquerda: i.key_pressed(K::ArrowLeft),
            direita: i.key_pressed(K::ArrowRight),
            cima: i.key_pressed(K::ArrowUp),
            baixo: i.key_pressed(K::ArrowDown),
            pg_cima: i.key_pressed(K::PageUp),
            pg_baixo: i.key_pressed(K::PageDown),
            shift: i.modifiers.shift,
            alt: i.modifiers.alt,
            ctrl: i.modifiers.ctrl || i.modifiers.command,
        }
    }
}

/// Movimento da câmera pedido pelas teclas: giro (yaw, pitch), deslocamento em frações da distância
/// no plano da tela (direita, cima) e fator de distância.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Movimento {
    pub giro: [f32; 2],
    pub mover: [f32; 2],
    pub zoom: f32,
}

/// O que as teclas pedem à vista; `None` quando nada. Alt e Ctrl ficam para a árvore e os atalhos.
pub fn movimento(t: Teclas) -> Option<Movimento> {
    if t.alt || t.ctrl {
        return None;
    }
    let h = t.direita as i32 - t.esquerda as i32;
    let v = t.cima as i32 - t.baixo as i32;
    let zoom = match (t.pg_cima, t.pg_baixo) {
        (true, false) => ZOOM,
        (false, true) => 1. / ZOOM,
        _ => 1.,
    };
    if h == 0 && v == 0 && zoom == 1. {
        return None;
    }
    let (giro, mover) = if t.shift { ([0., 0.], [h as f32 * PASSO_MOVER, v as f32 * PASSO_MOVER]) } else { ([-(h as f32) * GIRO, v as f32 * GIRO], [0., 0.]) };
    Some(Movimento { giro, mover, zoom })
}

/// Aplica o movimento à câmera, com os mesmos limites do arrasto do mouse.
pub fn aplicar(cam: &mut crate::render::Camera, m: Movimento) {
    cam.yaw += m.giro[0];
    cam.pitch = (cam.pitch + m.giro[1]).clamp(-1.5706, 1.5706);
    if m.mover != [0., 0.] {
        let (r, u) = cam.basis();
        cam.target += (r * m.mover[0] + u * m.mover[1]) * cam.dist;
    }
    cam.dist = (cam.dist * m.zoom).clamp(0.5, 1e6);
}

/// Alt+↑ (-1) ou Alt+↓ (+1): o comando vizinho do escolhido entre os ativos da árvore. Sem nada
/// escolhido, ↓ vai ao primeiro e ↑ ao último. Comando suprimido também conta: a árvore o mostra.
pub fn vizinho_na_arvore(doc: &fenkra_core::Document, atual: Option<fenkra_core::Id>, passo: i32) -> Option<fenkra_core::Id> {
    // `rollback` é a quantidade de comandos ativos; os de depois do marcador ficam fora
    let ativos: Vec<fenkra_core::Id> = doc.features.iter().take(doc.rollback.min(doc.features.len())).map(|f| f.id).collect();
    if ativos.is_empty() {
        return None;
    }
    let n = ativos.len() as i32;
    let i = match atual.and_then(|a| ativos.iter().position(|x| *x == a)) {
        Some(i) => (i as i32 + passo).clamp(0, n - 1),
        None if passo > 0 => 0,
        None => n - 1,
    };
    Some(ativos[i as usize])
}

/// Passo da árvore pedido no quadro (Alt+↑ ou Alt+↓).
pub fn passo_da_arvore(t: Teclas) -> Option<i32> {
    if !t.alt || t.ctrl {
        return None;
    }
    match (t.cima, t.baixo) {
        (true, false) => Some(-1),
        (false, true) => Some(1),
        _ => None,
    }
}

/// ↑ ou ↓ sem modificador (B449): o passo das setas quando a árvore tem o foco.
pub fn passo_na_arvore(t: Teclas) -> Option<i32> {
    if t.alt || t.ctrl || t.shift {
        return None;
    }
    match (t.cima, t.baixo) {
        (true, false) => Some(-1),
        (false, true) => Some(1),
        _ => None,
    }
}

/// A linha vizinha na ordem em que a árvore foi desenhada (B449). Sem linha atual, ↓ vai à primeira
/// e ↑ à última; nas pontas, fica onde está.
pub fn vizinho_na_lista<T: PartialEq + Copy>(ordem: &[T], atual: Option<T>, passo: i32) -> Option<T> {
    if ordem.is_empty() {
        return None;
    }
    let n = ordem.len() as i32;
    let i = match atual.and_then(|a| ordem.iter().position(|x| *x == a)) {
        Some(i) => (i as i32 + passo).clamp(0, n - 1),
        None if passo > 0 => 0,
        None => n - 1,
    };
    Some(ordem[i as usize])
}

/// Quem recebe ↑ e ↓ (B449): a árvore, quando o foco do teclado está numa linha dela ou quando o
/// último clique foi nela e nenhum outro widget pegou o foco; senão, a vista (B321). Antes, o
/// clique na árvore deixava as setas sem dono até um Esc.
pub fn setas_na_arvore(arvore_ativa: bool, foco_em_linha: bool, foco_em_outro: bool) -> bool {
    foco_em_linha || (arvore_ativa && !foco_em_outro)
}

const CHAVE_ATIVA: &str = "fk_arvore_teclado";
const CHAVE_LINHAS: &str = "fk_arvore_linhas";
const CHAVE_LINHAS_NOVAS: &str = "fk_arvore_linhas_novas";

/// A árvore está com as setas (último clique nela, sem Esc nem clique fora depois)?
pub fn arvore_ativa(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp(egui::Id::new(CHAVE_ATIVA))).unwrap_or(false)
}

pub fn marcar_arvore_ativa(ctx: &egui::Context, ativa: bool) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(CHAVE_ATIVA), ativa));
}

/// Linhas da árvore do quadro anterior: o id do objeto e o do widget, na ordem desenhada.
pub fn linhas_da_arvore(ctx: &egui::Context) -> Vec<(fenkra_core::Id, egui::Id)> {
    ctx.data(|d| d.get_temp(egui::Id::new(CHAVE_LINHAS))).unwrap_or_default()
}

/// Começo do quadro da árvore: as linhas anotadas no quadro anterior passam a valer.
pub fn comecar_linhas_da_arvore(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        let novas: Vec<(fenkra_core::Id, egui::Id)> = d.get_temp(egui::Id::new(CHAVE_LINHAS_NOVAS)).unwrap_or_default();
        if !novas.is_empty() {
            d.insert_temp(egui::Id::new(CHAVE_LINHAS), novas);
        }
        d.insert_temp(egui::Id::new(CHAVE_LINHAS_NOVAS), Vec::<(fenkra_core::Id, egui::Id)>::new());
    });
}

pub fn anotar_linha_da_arvore(ctx: &egui::Context, id: fenkra_core::Id, widget: egui::Id) {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<(fenkra_core::Id, egui::Id)>>(egui::Id::new(CHAVE_LINHAS_NOVAS)).push((id, widget)));
}

/// Solta um foco de teclado que não seja de campo de texto. Um widget com foco faz o egui achar
/// que o teclado é dele: as setas e os atalhos param até um Esc (B449).
pub fn soltar_foco_escondido(ctx: &egui::Context) {
    if !ctx.text_edit_focused() {
        ctx.memory_mut(|m| {
            if let Some(f) = m.focused() {
                m.surrender_focus(f);
            }
        });
    }
}

/// Algum evento deste quadro pode mudar o estado da interface: botão do mouse, tecla, texto ou roda.
///
/// O egui desenha um quadro por evento e o app muda o estado no meio dele (os cliques da árvore e
/// dos painéis viram ações aplicadas depois de desenhar). Sem pedir mais um quadro, o resultado só
/// aparecia no próximo movimento do mouse (B449). Movimento puro do ponteiro não conta: ele já traz
/// o próprio quadro e não muda estado sozinho.
pub fn evento_muda_estado(eventos: &[egui::Event]) -> bool {
    eventos.iter().any(|e| {
        matches!(
            e,
            egui::Event::PointerButton { .. } | egui::Event::Key { .. } | egui::Event::Text(_) | egui::Event::MouseWheel { .. } | egui::Event::Paste(_) | egui::Event::Copy | egui::Event::Cut | egui::Event::PointerGone
        )
    })
}

/// Anel de foco do teclado em widget desenhado à mão: o mesmo traço de 2 px que o tema dá ao foco
/// dos widgets do egui (contraste de componente de 3:1).
pub fn anel_de_foco(ui: &egui::Ui, resp: &egui::Response, rect: egui::Rect) {
    if resp.has_focus() {
        let s = ui.visuals().selection.stroke;
        ui.painter().rect_stroke(rect.expand(1.), 5., egui::Stroke::new(s.width.max(2.), s.color), egui::StrokeKind::Outside);
    }
}

/// Anel de foco no retângulo da própria resposta e a resposta de volta, para encadear depois de
/// `allocate_exact_size` ou `ui.interact` em botões, abas, miniaturas e alças desenhados à mão.
pub fn anel(ui: &egui::Ui, resp: egui::Response) -> egui::Response {
    anel_de_foco(ui, &resp, resp.rect);
    resp
}

/// Linhas do resumo das teclas, para as Preferências e o mapa de atalhos.
pub const RESUMO: [(&str, &str); 7] = [
    ("Tab / Shift+Tab", "passa o foco por botões e campos; Enter ou Espaço aciona"),
    ("Setas", "giram a vista; com Shift, movem"),
    ("↑ / ↓", "com um item da árvore em foco, andam pela árvore"),
    ("Page Up / Page Down", "aproximam e afastam a vista"),
    ("Alt+↑ / Alt+↓", "escolhem o comando anterior ou o seguinte da árvore"),
    ("Enter", "abre o comando escolhido; com ele aberto, conclui"),
    ("Esc", "cancela o comando ou solta o foco"),
];

/// Resumo das teclas nas Preferências de aparência e acessibilidade.
pub fn resumo_ui(ui: &mut egui::Ui) {
    ui.label(egui::RichText::new(crate::i18n::tr("Uso por teclado")).strong());
    egui::Grid::new("resumo_teclado").num_columns(2).spacing([10., 2.]).show(ui, |ui| {
        for (k, d) in RESUMO {
            ui.label(egui::RichText::new(k).monospace());
            ui.label(egui::RichText::new(crate::i18n::tr(d)).small());
            ui.end_row();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{Document, FeatureKind, OriginPlane, PlaneRef, Sketch};

    #[test]
    fn setas_giram_shift_move_page_aproxima() {
        let m = movimento(Teclas { direita: true, ..Default::default() }).unwrap();
        assert_eq!(m.giro, [-GIRO, 0.]);
        assert_eq!(m.zoom, 1.);
        let m = movimento(Teclas { cima: true, shift: true, ..Default::default() }).unwrap();
        assert_eq!((m.giro, m.mover), ([0., 0.], [0., PASSO_MOVER]));
        assert_eq!(movimento(Teclas { pg_cima: true, ..Default::default() }).unwrap().zoom, ZOOM);
        assert!(movimento(Teclas::default()).is_none());
        assert!(movimento(Teclas { cima: true, alt: true, ..Default::default() }).is_none(), "Alt+seta é da árvore");
    }

    #[test]
    fn aplicar_respeita_os_limites_do_mouse() {
        let mut c = crate::render::Camera::default();
        for _ in 0..20 {
            aplicar(&mut c, movimento(Teclas { cima: true, ..Default::default() }).unwrap());
        }
        assert!(c.pitch <= 1.5706);
        let d = c.dist;
        aplicar(&mut c, movimento(Teclas { pg_cima: true, ..Default::default() }).unwrap());
        assert!((c.dist - d * ZOOM).abs() < 1e-3);
        let alvo = c.target;
        aplicar(&mut c, movimento(Teclas { direita: true, shift: true, ..Default::default() }).unwrap());
        assert!((c.target - alvo).length() > 0.01 * c.dist, "Shift+seta move o alvo");
    }

    #[test]
    fn alt_setas_percorrem_a_arvore() {
        let mut d = Document::new("t");
        let a = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: Default::default() }));
        let b = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XZ), data: Default::default() }));
        assert_eq!(vizinho_na_arvore(&d, None, 1), Some(a));
        assert_eq!(vizinho_na_arvore(&d, None, -1), Some(b));
        assert_eq!(vizinho_na_arvore(&d, Some(a), 1), Some(b));
        assert_eq!(vizinho_na_arvore(&d, Some(b), 1), Some(b), "para no fim");
        assert_eq!(vizinho_na_arvore(&d, Some(b), -1), Some(a));
        assert_eq!(passo_da_arvore(Teclas { alt: true, baixo: true, ..Default::default() }), Some(1));
        assert_eq!(passo_da_arvore(Teclas { baixo: true, ..Default::default() }), None);
    }

    /// Roda um quadro com a caixinha da árvore e um botão desenhado à mão; devolve as formas pintadas
    /// e quem ficou com o foco.
    fn quadro(ctx: &egui::Context, eventos: Vec<egui::Event>) -> (Vec<egui::epaint::ClippedShape>, [bool; 2]) {
        let raw = egui::RawInput { events: eventos, screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400., 300.))), ..Default::default() };
        let mut foco = [false; 2];
        let mut out = ctx.run_ui(raw, |ui| {
            egui::CentralPanel::default().show_inside(ui, |ui| {
                let a = crate::tree_ui::caixa(ui, true);
                let (_, b) = ui.allocate_exact_size(egui::vec2(30., 20.), egui::Sense::click());
                let b = anel(ui, b);
                foco = [a.has_focus(), b.has_focus()];
            });
        });
        // sem placa de vídeo no teste: as texturas do quadro são descartadas
        out.textures_delta.clear();
        (out.shapes, foco)
    }

    fn tab() -> Vec<egui::Event> {
        let e = |pressed| egui::Event::Key { key: egui::Key::Tab, physical_key: None, pressed, repeat: false, modifiers: Default::default() };
        vec![e(true), e(false)]
    }

    /// Traços com a cor e a largura do anel (a cor do traço de seleção do tema, 2 px).
    fn aneis(ctx: &egui::Context, formas: &[egui::epaint::ClippedShape]) -> usize {
        let cor = ctx.global_style().visuals.selection.stroke.color;
        formas.iter().filter(|c| matches!(&c.shape, egui::Shape::Rect(r) if r.stroke.color == cor && r.stroke.width >= 2.)).count()
    }

    #[test]
    fn tab_leva_o_foco_aos_botoes_desenhados_e_pinta_o_anel() {
        let ctx = egui::Context::default();
        let (f, foco) = quadro(&ctx, vec![]);
        assert_eq!(foco, [false, false]);
        assert_eq!(aneis(&ctx, &f), 0, "sem foco não há anel");
        // o Tab vale no quadro seguinte ao que o lê
        quadro(&ctx, tab());
        let (f, foco) = quadro(&ctx, vec![]);
        assert_eq!(foco, [true, false], "primeiro Tab: a caixinha da árvore");
        assert_eq!(aneis(&ctx, &f), 1);
        quadro(&ctx, tab());
        let (f, foco) = quadro(&ctx, vec![]);
        assert_eq!(foco, [false, true], "segundo Tab: o botão desenhado à mão");
        assert_eq!(aneis(&ctx, &f), 1);
    }

    #[test]
    fn setas_andam_na_arvore_com_ela_em_foco_e_voltam_para_a_vista_sem() {
        let ordem = [10, 20, 30];
        assert_eq!(vizinho_na_lista(&ordem, Some(20), 1), Some(30));
        assert_eq!(vizinho_na_lista(&ordem, Some(20), -1), Some(10));
        assert_eq!(vizinho_na_lista(&ordem, Some(30), 1), Some(30), "para no fim");
        assert_eq!(vizinho_na_lista(&ordem, None, 1), Some(10));
        assert_eq!(vizinho_na_lista(&ordem, None, -1), Some(30));
        assert_eq!(vizinho_na_lista::<i32>(&[], None, 1), None);
        assert_eq!(passo_na_arvore(Teclas { baixo: true, ..Default::default() }), Some(1));
        assert_eq!(passo_na_arvore(Teclas { cima: true, ..Default::default() }), Some(-1));
        assert_eq!(passo_na_arvore(Teclas { baixo: true, alt: true, ..Default::default() }), None, "Alt+↓ é o comando seguinte");
        assert_eq!(passo_na_arvore(Teclas { direita: true, ..Default::default() }), None, "← e → ficam com a vista");
        // o clique na árvore dá as setas a ela; o foco do Tab numa linha também; outro widget com
        // foco (um campo) fica com elas; sem nada, a vista
        assert!(setas_na_arvore(true, false, false));
        assert!(setas_na_arvore(false, true, false));
        assert!(!setas_na_arvore(true, false, true));
        assert!(!setas_na_arvore(false, false, false));
    }

    #[test]
    fn clique_tecla_e_roda_pedem_quadro_movimento_nao() {
        let mv = egui::Event::PointerMoved(egui::pos2(1., 1.));
        assert!(!evento_muda_estado(&[mv.clone()]));
        assert!(!evento_muda_estado(&[]));
        let clique = egui::Event::PointerButton { pos: egui::pos2(1., 1.), button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() };
        assert!(evento_muda_estado(&[mv, clique]));
        assert!(evento_muda_estado(&tab()));
    }

    #[test]
    fn foco_escondido_em_botao_sai_e_o_de_campo_de_texto_fica() {
        let ctx = egui::Context::default();
        quadro(&ctx, vec![]);
        quadro(&ctx, tab());
        let (_, foco) = quadro(&ctx, vec![]);
        assert_eq!(foco, [true, false]);
        assert!(ctx.egui_wants_keyboard_input(), "com o foco num botão, o egui segura as setas e os atalhos");
        soltar_foco_escondido(&ctx);
        assert!(!ctx.egui_wants_keyboard_input());
        // campo de texto em edição não perde o foco
        let campo = |ctx: &egui::Context, focar: bool| {
            let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400., 300.))), ..Default::default() };
            let mut out = ctx.run_ui(raw, |ui| {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    let mut t = String::new();
                    let r = ui.text_edit_singleline(&mut t);
                    if focar {
                        r.request_focus();
                    }
                });
            });
            out.textures_delta.clear();
        };
        campo(&ctx, true);
        campo(&ctx, false);
        assert!(ctx.text_edit_focused());
        soltar_foco_escondido(&ctx);
        assert!(ctx.text_edit_focused());
    }

    /// Diagnóstico do B449: o clique numa linha da árvore (rótulo escolhível com arrastar, como
    /// `drag_item`) dá ou não o foco do teclado ao rótulo?
    #[test]
    fn clique_numa_linha_e_o_foco_do_teclado() {
        let ctx = egui::Context::default();
        let rodar = |ctx: &egui::Context, eventos: Vec<egui::Event>| {
            let raw = egui::RawInput { events: eventos, screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400., 300.))), ..Default::default() };
            let mut rect = egui::Rect::NOTHING;
            let mut out = ctx.run_ui(raw, |ui| {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    let r = ui.selectable_label(false, "Placa");
                    let r = r.interact(egui::Sense::click_and_drag());
                    rect = r.rect;
                });
            });
            out.textures_delta.clear();
            rect
        };
        let rect = rodar(&ctx, vec![]);
        let p = rect.center();
        let botao = |pressed| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        rodar(&ctx, vec![egui::Event::PointerMoved(p)]);
        rodar(&ctx, vec![botao(true)]);
        rodar(&ctx, vec![botao(false)]);
        rodar(&ctx, vec![]);
        assert!(!ctx.egui_wants_keyboard_input(), "o clique não deve prender o foco: {:?}", ctx.memory(|m| m.focused()));
    }

    #[test]
    fn resumo_traduzido() {
        for (_, d) in RESUMO {
            assert!(crate::i18n::tem_traducao(d), "sem tradução: {d}");
        }
        assert!(crate::i18n::tem_traducao("Uso por teclado"));
    }
}
