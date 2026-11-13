// SPDX-License-Identifier: LGPL-2.1-or-later
//! Uso por teclado (B321): o que o mouse faz na vista e na árvore também sai das teclas, com o foco
//! sempre visível.
//!
//! - Tab e Shift+Tab passam o foco por botões, campos e listas (o egui faz isso); Enter ou Espaço
//!   acionam o botão em foco, e todo botão desenhado à mão pinta o anel de foco (`anel_de_foco`).
//! - Sem nada em foco: setas giram a vista 15°, Shift+setas a movem, Page Up e Page Down aproximam e
//!   afastam. Esc solta o foco e volta para a vista.
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
pub const RESUMO: [(&str, &str); 6] = [
    ("Tab / Shift+Tab", "passa o foco por botões e campos; Enter ou Espaço aciona"),
    ("Setas", "giram a vista; com Shift, movem"),
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
    fn resumo_traduzido() {
        for (_, d) in RESUMO {
            assert!(crate::i18n::tem_traducao(d), "sem tradução: {d}");
        }
        assert!(crate::i18n::tem_traducao("Uso por teclado"));
    }
}
