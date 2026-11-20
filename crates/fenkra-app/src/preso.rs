// SPDX-License-Identifier: LGPL-2.1-or-later
//! B554 e B552: comando "preso" por duplo clique no ícone.
//!
//! Regra única, igual em todas as bancadas: clique no ícone roda o comando uma vez e, ao concluir
//! (ou ao terminar o elemento no sketch), volta à seleção. Duplo clique deixa o comando preso: ao
//! concluir ele reabre do mesmo jeito que um comando novo abriria, até Esc, novo clique no ícone ou
//! outro comando. O estado é um `Preso` (uma vez para os comandos do documento, no `App`; outra
//! para as ferramentas do sketch aberto, em `SketchEdit`) e a decisão é a função pura
//! [`Preso::ao_concluir`], sem tocar em egui nem no documento.

use crate::app::Cmd;
use crate::sketch_ui::Tool;

/// O que pode ficar preso.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Alvo {
    Cmd(Cmd),
    Sk(Tool),
}

/// O que fazer quando o comando ou o elemento termina.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Depois {
    Reabrir(Alvo),
    Selecao,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Preso(pub Option<Alvo>);

impl Preso {
    /// Duplo clique no ícone: prende.
    pub fn prender(&mut self, a: Alvo) {
        self.0 = Some(a);
    }
    /// Esc, clique de novo no ícone, outro comando.
    pub fn soltar(&mut self) {
        self.0 = None;
    }
    /// Escolheu um comando: solta, a menos que seja o próprio preso (reabertura).
    pub fn escolher(&mut self, a: Alvo) {
        if self.0 != Some(a) {
            self.0 = None;
        }
    }
    pub fn esta(&self, a: Alvo) -> bool {
        self.0 == Some(a)
    }
    /// Terminou `concluido`: reabre se estava preso, senão volta à seleção.
    pub fn ao_concluir(&self, concluido: Alvo) -> Depois {
        match self.0 {
            Some(p) if p == concluido => Depois::Reabrir(p),
            _ => Depois::Selecao,
        }
    }
}

/// Ferramentas do sketch que criam um elemento e, sem estar presas, terminam nele.
pub fn de_um_so(t: Tool) -> bool {
    matches!(
        t,
        Tool::Line
            | Tool::Rect
            | Tool::RectCenter
            | Tool::Rect3
            | Tool::Parallelogram
            | Tool::Circle
            | Tool::Circle3
            | Tool::CircleTan3
            | Tool::Arc3
            | Tool::ArcCenter
            | Tool::Slot
            | Tool::Polygon
            | Tool::Point
            | Tool::Spline
            | Tool::Dimension
            | Tool::DimH
            | Tool::DimV
            | Tool::DimAngle
            | Tool::DimRadius
            | Tool::DimDiameter
            | Tool::Constr(_)
    )
}

/// O que um botão da barra prenderia.
pub fn alvo_de(act: crate::toolbar::Act) -> Option<Alvo> {
    match act {
        crate::toolbar::Act::Cmd(c) => Some(Alvo::Cmd(c)),
        crate::toolbar::Act::SkTool(t) if t != Tool::Select => Some(Alvo::Sk(t)),
        _ => None,
    }
}

pub const DICA: &str = "preso: duplo clique repete; Esc solta";

pub const TEXTOS: &[(&str, &str)] = &[(DICA, "pinned: double-click repeats; Esc releases")];

/// Realce do ícone preso, pela cor do tema, com a dica.
pub fn marcar(ui: &eframe::egui::Ui, r: eframe::egui::Response, preso: bool) -> eframe::egui::Response {
    use eframe::egui;
    if !preso {
        return r;
    }
    if ui.is_rect_visible(r.rect) {
        let cor = crate::theme::accent(ui.visuals());
        ui.painter().rect_stroke(r.rect.shrink(1.), 6., egui::Stroke::new(2., cor), egui::StrokeKind::Inside);
        ui.painter().circle_filled(r.rect.right_top() + egui::vec2(-6., 6.), 3., cor);
    }
    r.on_hover_text(crate::i18n::tr(DICA))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn clique_unico_nao_repete() {
        let p = Preso::default();
        assert_eq!(p.ao_concluir(Alvo::Cmd(Cmd::Cut)), Depois::Selecao);
        assert_eq!(p.ao_concluir(Alvo::Sk(Tool::Line)), Depois::Selecao);
    }

    #[test]
    fn duplo_clique_repete_o_mesmo() {
        let mut p = Preso::default();
        p.prender(Alvo::Cmd(Cmd::Cut));
        assert_eq!(p.ao_concluir(Alvo::Cmd(Cmd::Cut)), Depois::Reabrir(Alvo::Cmd(Cmd::Cut)));
        // concluir outra coisa não reabre
        assert_eq!(p.ao_concluir(Alvo::Cmd(Cmd::Extrude)), Depois::Selecao);
    }

    #[test]
    fn esc_e_outro_comando_soltam() {
        let mut p = Preso::default();
        p.prender(Alvo::Sk(Tool::Circle));
        p.soltar();
        assert_eq!(p.ao_concluir(Alvo::Sk(Tool::Circle)), Depois::Selecao);
        p.prender(Alvo::Sk(Tool::Circle));
        p.escolher(Alvo::Sk(Tool::Rect));
        assert_eq!(p.0, None);
        // reabrir o próprio preso não solta
        p.prender(Alvo::Cmd(Cmd::Cut));
        p.escolher(Alvo::Cmd(Cmd::Cut));
        assert!(p.esta(Alvo::Cmd(Cmd::Cut)));
    }

    #[test]
    fn so_o_que_cria_elemento_termina_sozinho() {
        assert!(de_um_so(Tool::Line) && de_um_so(Tool::Dimension) && de_um_so(Tool::Constr(crate::sketch_ui::CKind::Parallel)));
        assert!(!de_um_so(Tool::Select) && !de_um_so(Tool::Trim) && !de_um_so(Tool::Move));
    }

    /// Sketch: simples faz um elemento e volta à seleção; preso faz um atrás do outro até o Esc.
    #[test]
    fn sketch_simples_faz_um_e_preso_faz_varios() {
        use fenkra_core::Id;
        for t in [Tool::Line, Tool::Circle, Tool::Rect, Tool::Point, Tool::Dimension] {
            let mut e = crate::sketch_ui::SketchEdit::new(Id(1), false);
            e.set_tool(t);
            e.fim_do_elemento(t, true);
            assert_eq!(e.tool, Tool::Select, "{t:?}: clique único = um elemento");

            let mut e = crate::sketch_ui::SketchEdit::new(Id(1), false);
            e.set_tool(t);
            e.preso.prender(Alvo::Sk(t));
            for _ in 0..3 {
                e.fim_do_elemento(t, true);
                assert_eq!(e.tool, t, "{t:?}: preso segue na ferramenta");
            }
            assert!(e.cancel());
            assert_eq!(e.tool, Tool::Select, "{t:?}: Esc solta e volta à seleção");
            assert_eq!(e.preso.0, None);
        }
    }

    #[test]
    fn sketch_outra_ferramenta_solta_e_modo_irmao_mantem() {
        use fenkra_core::Id;
        let mut e = crate::sketch_ui::SketchEdit::new(Id(1), false);
        e.set_tool(Tool::Rect);
        e.preso.prender(Alvo::Sk(Tool::Rect));
        e.escolher(Tool::RectCenter);
        assert!(e.preso.esta(Alvo::Sk(Tool::RectCenter)), "outro modo do mesmo comando mantém o preso");
        e.escolher(Tool::Circle);
        assert_eq!(e.preso.0, None, "outro comando solta");
    }

    #[test]
    fn textos_traduzidos() {
        assert!(crate::i18n::tem_traducao(DICA));
    }
}
