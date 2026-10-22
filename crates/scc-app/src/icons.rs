//! Ícones SVG (assets/icons, gerados por scripts/gerar_icones.py e editáveis no VectorCraft/Inkscape).

use eframe::egui::{self, ImageSource};

pub fn get(name: &str) -> ImageSource<'static> {
    match name {
        "sketch" => egui::include_image!("../../../assets/icons/sketch.svg"),
        "extrusao" => egui::include_image!("../../../assets/icons/extrusao.svg"),
        "recorte" => egui::include_image!("../../../assets/icons/recorte.svg"),
        "revolucao" => egui::include_image!("../../../assets/icons/revolucao.svg"),
        "furo" => egui::include_image!("../../../assets/icons/furo.svg"),
        "filete" => egui::include_image!("../../../assets/icons/filete.svg"),
        "chanfro" => egui::include_image!("../../../assets/icons/chanfro.svg"),
        "plano" => egui::include_image!("../../../assets/icons/plano.svg"),
        "parametro" => egui::include_image!("../../../assets/icons/parametro.svg"),
        "pasta" => egui::include_image!("../../../assets/icons/pasta.svg"),
        "coletor" => egui::include_image!("../../../assets/icons/coletor.svg"),
        "aparencia" => egui::include_image!("../../../assets/icons/aparencia.svg"),
        "ajustar" => egui::include_image!("../../../assets/icons/ajustar.svg"),
        "vista_iso" => egui::include_image!("../../../assets/icons/vista_iso.svg"),
        "vista_frente" => egui::include_image!("../../../assets/icons/vista_frente.svg"),
        "vista_topo" => egui::include_image!("../../../assets/icons/vista_topo.svg"),
        "vista_direita" => egui::include_image!("../../../assets/icons/vista_direita.svg"),
        "estilo" => egui::include_image!("../../../assets/icons/estilo.svg"),
        "mostrar" => egui::include_image!("../../../assets/icons/mostrar.svg"),
        "ocultar" => egui::include_image!("../../../assets/icons/ocultar.svg"),
        "ocultos" => egui::include_image!("../../../assets/icons/ocultos.svg"),
        "filtro" => egui::include_image!("../../../assets/icons/filtro.svg"),
        "config" => egui::include_image!("../../../assets/icons/config.svg"),
        "ortho" => egui::include_image!("../../../assets/icons/ortho.svg"),
        _ => egui::include_image!("../../../assets/icons/config.svg"),
    }
}

/// Botão de ferramenta: ícone grande com o nome embaixo (ou só o ícone).
pub fn tool_button(ui: &mut egui::Ui, icon: &str, label: &str, size: f32, show_label: bool, selected: bool) -> egui::Response {
    let font = egui::FontId::proportional(11.5);
    let galley = show_label.then(|| ui.painter().layout_no_wrap(label.to_owned(), font, ui.visuals().text_color()));
    let text_w = galley.as_ref().map(|g| g.size().x).unwrap_or(0.);
    let text_h = galley.as_ref().map(|g| g.size().y + 2.).unwrap_or(0.);
    let w = (size + 10.).max(text_w + 10.);
    let h = size + 8. + text_h;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    let resp = resp.on_hover_text(label);
    if ui.is_rect_visible(rect) {
        let v = ui.visuals();
        let fill = if selected {
            v.selection.bg_fill
        } else if resp.hovered() {
            v.widgets.hovered.weak_bg_fill
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 6., fill);
        let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 4. + size / 2.), egui::vec2(size, size));
        egui::Image::new(get(icon)).paint_at(ui, icon_rect);
        if let Some(g) = galley {
            let pos = egui::pos2(rect.center().x - g.size().x / 2., icon_rect.bottom() + 2.);
            ui.painter().galley(pos, g, v.text_color());
        }
    }
    resp
}
