//! Estado da chapa no 3D (B428): quando a chapa está planificada, um selo no alto da vista diz isso,
//! com a cor de destaque do tema, e o botão Planificar da barra fica aceso. Antes a troca era só um
//! aviso que sumia, e nenhuma captura mostrava a chapa plana.

use crate::i18n::tr;
use eframe::egui::{self, Align2, Color32, FontId, Rect, Stroke, StrokeKind};
use fenkra_core::Document;

/// Texto do selo para o estado da chapa; `None` se o documento não é de chapa ou está dobrado.
pub fn texto_do_selo(d: &Document) -> Option<&'static str> {
    d.sheet.as_ref().filter(|s| s.flat).map(|_| "Chapa planificada")
}

fn rgb(c: Color32) -> [f32; 3] {
    [c.r() as f32 / 255., c.g() as f32 / 255., c.b() as f32 / 255.]
}

/// Tinta do texto do selo: a de seleção do tema, ou a forte se não lê sobre o fundo do selo.
fn tinta_legivel(fundo: Color32, tinta: Color32, forte: Color32) -> Color32 {
    let d = (crate::contraste::luminancia(rgb(tinta)) - crate::contraste::luminancia(rgb(fundo))).abs();
    if d < 0.35 {
        forte
    } else {
        tinta
    }
}

/// Desenha o selo no alto e ao centro da vista.
pub fn selo(ui: &egui::Ui, rect: Rect, d: &Document) {
    let Some(t) = texto_do_selo(d) else { return };
    let v = ui.visuals();
    let fundo = v.selection.bg_fill;
    let tinta = tinta_legivel(fundo, v.selection.stroke.color, v.strong_text_color());
    let painter = ui.painter_at(rect);
    let fonte = FontId::proportional(crate::acessibilidade::fonte_3d(13.));
    let galeria = painter.layout_no_wrap(tr(t).to_string(), fonte.clone(), tinta);
    let tam = galeria.size() + egui::vec2(20., 10.);
    let caixa = Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 46.), tam);
    painter.rect_filled(caixa, 8., fundo);
    painter.rect_stroke(caixa, 8., Stroke::new(1., v.widgets.noninteractive.fg_stroke.color), StrokeKind::Inside);
    painter.text(caixa.center(), Align2::CENTER_CENTER, tr(t), fonte, tinta);
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn selo_so_aparece_com_a_chapa_planificada() {
        let mut d = Document::new("Chapa");
        assert_eq!(texto_do_selo(&d), None, "sem chapa");
        d.sheet = Some(Default::default());
        assert_eq!(texto_do_selo(&d), None, "dobrada");
        d.sheet.as_mut().unwrap().flat = true;
        assert_eq!(texto_do_selo(&d), Some("Chapa planificada"));
    }

    #[test]
    fn tinta_ilegivel_troca_pela_forte() {
        let fundo = Color32::BLUE;
        let forte = Color32::WHITE;
        assert_eq!(tinta_legivel(fundo, Color32::DARK_BLUE, forte), forte);
        assert_eq!(tinta_legivel(fundo, Color32::WHITE, Color32::BLACK), Color32::WHITE);
    }
}
