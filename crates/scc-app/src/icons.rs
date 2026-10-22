//! Ícones SVG (assets/icons, gerados por scripts/gerar_icones.py e editáveis no VectorCraft/Inkscape).

use eframe::egui::{self, ImageSource};

// Lista gerada a partir de assets/icons (scripts/gerar_icones.py).
pub fn get(name: &str) -> ImageSource<'static> {
    match name {
        "ajustar" => egui::include_image!("../../../assets/icons/ajustar.svg"),
        "aparencia" => egui::include_image!("../../../assets/icons/aparencia.svg"),
        "chanfro" => egui::include_image!("../../../assets/icons/chanfro.svg"),
        "coletor" => egui::include_image!("../../../assets/icons/coletor.svg"),
        "config" => egui::include_image!("../../../assets/icons/config.svg"),
        "estilo" => egui::include_image!("../../../assets/icons/estilo.svg"),
        "extrusao" => egui::include_image!("../../../assets/icons/extrusao.svg"),
        "filete" => egui::include_image!("../../../assets/icons/filete.svg"),
        "filtro" => egui::include_image!("../../../assets/icons/filtro.svg"),
        "furo" => egui::include_image!("../../../assets/icons/furo.svg"),
        "idioma" => egui::include_image!("../../../assets/icons/idioma.svg"),
        "mostrar" => egui::include_image!("../../../assets/icons/mostrar.svg"),
        "ocultar" => egui::include_image!("../../../assets/icons/ocultar.svg"),
        "ocultos" => egui::include_image!("../../../assets/icons/ocultos.svg"),
        "ortho" => egui::include_image!("../../../assets/icons/ortho.svg"),
        "parametro" => egui::include_image!("../../../assets/icons/parametro.svg"),
        "pasta" => egui::include_image!("../../../assets/icons/pasta.svg"),
        "personalizar" => egui::include_image!("../../../assets/icons/personalizar.svg"),
        "plano" => egui::include_image!("../../../assets/icons/plano.svg"),
        "r_coincidente" => egui::include_image!("../../../assets/icons/r_coincidente.svg"),
        "r_concentrica" => egui::include_image!("../../../assets/icons/r_concentrica.svg"),
        "r_fixa" => egui::include_image!("../../../assets/icons/r_fixa.svg"),
        "r_horizontal" => egui::include_image!("../../../assets/icons/r_horizontal.svg"),
        "r_igual" => egui::include_image!("../../../assets/icons/r_igual.svg"),
        "r_paralela" => egui::include_image!("../../../assets/icons/r_paralela.svg"),
        "r_perpendicular" => egui::include_image!("../../../assets/icons/r_perpendicular.svg"),
        "r_ponto_medio" => egui::include_image!("../../../assets/icons/r_ponto_medio.svg"),
        "r_simetrica" => egui::include_image!("../../../assets/icons/r_simetrica.svg"),
        "r_tangente" => egui::include_image!("../../../assets/icons/r_tangente.svg"),
        "r_vertical" => egui::include_image!("../../../assets/icons/r_vertical.svg"),
        "recorte" => egui::include_image!("../../../assets/icons/recorte.svg"),
        "revolucao" => egui::include_image!("../../../assets/icons/revolucao.svg"),
        "sk_arco3" => egui::include_image!("../../../assets/icons/sk_arco3.svg"),
        "sk_arco_centro" => egui::include_image!("../../../assets/icons/sk_arco_centro.svg"),
        "sk_auto_cota" => egui::include_image!("../../../assets/icons/sk_auto_cota.svg"),
        "sk_auto_restricao" => egui::include_image!("../../../assets/icons/sk_auto_restricao.svg"),
        "sk_circulo" => egui::include_image!("../../../assets/icons/sk_circulo.svg"),
        "sk_circulo3" => egui::include_image!("../../../assets/icons/sk_circulo3.svg"),
        "sk_concluir" => egui::include_image!("../../../assets/icons/sk_concluir.svg"),
        "sk_construcao" => egui::include_image!("../../../assets/icons/sk_construcao.svg"),
        "sk_cota" => egui::include_image!("../../../assets/icons/sk_cota.svg"),
        "sk_grade" => egui::include_image!("../../../assets/icons/sk_grade.svg"),
        "sk_linha" => egui::include_image!("../../../assets/icons/sk_linha.svg"),
        "sk_poligono" => egui::include_image!("../../../assets/icons/sk_poligono.svg"),
        "sk_polilinha" => egui::include_image!("../../../assets/icons/sk_polilinha.svg"),
        "sk_ponto" => egui::include_image!("../../../assets/icons/sk_ponto.svg"),
        "sk_rasgo" => egui::include_image!("../../../assets/icons/sk_rasgo.svg"),
        "sk_retangulo" => egui::include_image!("../../../assets/icons/sk_retangulo.svg"),
        "sk_retangulo_centro" => egui::include_image!("../../../assets/icons/sk_retangulo_centro.svg"),
        "sk_selecionar" => egui::include_image!("../../../assets/icons/sk_selecionar.svg"),
        "sk_snap" => egui::include_image!("../../../assets/icons/sk_snap.svg"),
        "sketch" => egui::include_image!("../../../assets/icons/sketch.svg"),
        "vista_direita" => egui::include_image!("../../../assets/icons/vista_direita.svg"),
        "vista_frente" => egui::include_image!("../../../assets/icons/vista_frente.svg"),
        "vista_iso" => egui::include_image!("../../../assets/icons/vista_iso.svg"),
        "vista_topo" => egui::include_image!("../../../assets/icons/vista_topo.svg"),
        _ => egui::include_image!("../../../assets/icons/config.svg"),
    }
}

/// Botão de ferramenta: ícone grande com o nome embaixo (ou só o ícone).
/// Dica: descrição breve na hora; mantendo o mouse 5 s, aparece a descrição avançada.
#[allow(clippy::too_many_arguments)]
pub fn tool_button(ui: &mut egui::Ui, icon: &str, label: &str, size: f32, show_label: bool, selected: bool, brief: &str, advanced: &str) -> egui::Response {
    let font = egui::FontId::proportional(11.5);
    let galley = show_label.then(|| ui.painter().layout_no_wrap(label.to_owned(), font, ui.visuals().text_color()));
    let text_w = galley.as_ref().map(|g| g.size().x).unwrap_or(0.);
    let text_h = galley.as_ref().map(|g| g.size().y + 2.).unwrap_or(0.);
    let w = (size + 10.).max(text_w + 10.);
    let h = size + 8. + text_h;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    let resp = hover_help(ui, resp, label, brief, advanced);
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

/// Dica em dois níveis: breve imediatamente; avançada depois de 5 s com o mouse parado em cima.
pub fn hover_help(ui: &egui::Ui, resp: egui::Response, title: &str, brief: &str, advanced: &str) -> egui::Response {
    let key = resp.id.with("hover_inicio");
    let now = ui.input(|i| i.time);
    if !resp.hovered() {
        ui.ctx().data_mut(|d| d.remove::<f64>(key));
        return resp;
    }
    let start = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(key, || now));
    let long = now - start >= 5.0 && !advanced.is_empty();
    if !advanced.is_empty() && !long {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
    }
    resp.on_hover_ui(|ui| {
        ui.set_max_width(320.);
        ui.label(egui::RichText::new(title).strong());
        if !brief.is_empty() && brief != title {
            ui.label(brief);
        }
        if long {
            ui.separator();
            ui.label(egui::RichText::new(advanced).small());
        } else if !advanced.is_empty() {
            ui.label(egui::RichText::new("(mantenha o mouse para mais detalhes)").small().weak());
        }
    })
}
