//! Barras de comandos em grupos (RF-UI-001): cada grupo pode ficar no topo, na lateral,
//! flutuar pela tela ou ser ocultado. Clique direito no grupo muda a posição.

use crate::app::Cmd;
use crate::icons::tool_button;
use crate::render::DisplayMode;
use crate::settings::{Dock, Group, Settings};
use eframe::egui::{self, RichText};

/// Ação pedida por um botão de ferramenta.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Act {
    Cmd(Cmd),
    Fit,
    View(f32, f32),
    Normal,
    Ortho,
    HiddenEnv,
    Settings,
    Dock(Group, Dock),
}

pub fn items(g: Group) -> Vec<(&'static str, &'static str, Act)> {
    use Act::*;
    match g {
        Group::Sketch => vec![("sketch", "Sketch", Cmd(crate::app::Cmd::Sketch))],
        Group::Basicos => vec![
            ("extrusao", "Extrusão", Cmd(crate::app::Cmd::Extrude)),
            ("recorte", "Recorte", Cmd(crate::app::Cmd::Cut)),
            ("revolucao", "Revolução", Cmd(crate::app::Cmd::Revolve)),
            ("furo", "Furo", Cmd(crate::app::Cmd::Hole)),
        ],
        Group::Acabamento => vec![("filete", "Filete", Cmd(crate::app::Cmd::Fillet)), ("chanfro", "Chanfro", Cmd(crate::app::Cmd::Chamfer))],
        Group::Referencias => vec![("plano", "Plano", Cmd(crate::app::Cmd::Plane))],
        Group::Organizacao => vec![
            ("parametro", "Parâmetro", Cmd(crate::app::Cmd::Param)),
            ("pasta", "Pasta", Cmd(crate::app::Cmd::Folder)),
            ("coletor", "Coletor", Cmd(crate::app::Cmd::Collector)),
        ],
        Group::Vista => vec![
            ("ajustar", "Ajustar  F", Fit),
            ("vista_iso", "Iso  Ctrl+7", View(-0.785, 0.6)),
            ("vista_frente", "Frente  Ctrl+1", View(-1.5708, 0.0)),
            ("vista_topo", "Topo  Ctrl+5", View(-1.5708, 1.5706)),
            ("vista_direita", "Direita  Ctrl+3", View(0.0, 0.0)),
            ("ortho", "Ortogonal  O", Ortho),
        ],
    }
}

/// Mostra os botões de um grupo; devolve as ações clicadas.
pub fn show_group(ui: &mut egui::Ui, g: Group, s: &mut Settings, hidden_env: bool, vertical: bool) -> Vec<Act> {
    let mut acts = vec![];
    let size = s.icon_size;
    let labels = s.show_labels && !vertical;
    let mut body = |ui: &mut egui::Ui| {
        for (icon, label, act) in items(g) {
            let selected = matches!(act, Act::Ortho) && s.ortho;
            let short = label.split("  ").next().unwrap_or(label);
            let r = tool_button(ui, icon, short, size, labels, selected).on_hover_text(label);
            if r.clicked() {
                acts.push(act);
            }
        }
        if g == Group::Vista {
            // estilo de exibição
            let r = tool_button(ui, "estilo", "Estilo", size, labels, false);
            egui::Popup::menu(&r).show(|ui| {
                for m in DisplayMode::ALL {
                    ui.radio_value(&mut s.display, m, m.label());
                }
                ui.separator();
                ui.checkbox(&mut s.edges_in_render, "arestas no semi-renderizado");
            });
            // filtro de exibição
            let r = tool_button(ui, "filtro", "Filtro", size, labels, false);
            egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                let f = &mut s.filter;
                ui.label(RichText::new("Mostrar na viewport").strong());
                ui.checkbox(&mut f.corpos, "Corpos");
                ui.checkbox(&mut f.sketches, "Sketches");
                ui.checkbox(&mut f.pontos, "Pontos de sketch");
                ui.checkbox(&mut f.planos_origem, "Planos da Origem");
                ui.checkbox(&mut f.planos_ref, "Planos de referência");
                ui.checkbox(&mut f.eixos, "Eixos da Origem");
                ui.checkbox(&mut f.rotulos, "Rótulos dos planos");
            });
            let r = tool_button(ui, "ocultos", "Ocultos", size, labels, hidden_env).on_hover_text("Ambiente de ocultos  Shift+H");
            if r.clicked() {
                acts.push(Act::HiddenEnv);
            }
            if tool_button(ui, "config", "Ajustes", size, labels, false).on_hover_text("Configurações").clicked() {
                acts.push(Act::Settings);
            }
        }
    };
    let inner = if vertical {
        ui.vertical_centered(|ui| body(ui)).response
    } else {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| body(ui)).response
    };
    // clique direito no grupo: escolher onde ele fica
    inner.context_menu(|ui| {
        ui.label(RichText::new(g.label()).strong());
        for d in Dock::ALL {
            if ui.radio(s.dock(g) == d, d.label()).clicked() {
                acts.push(Act::Dock(g, d));
                ui.close();
            }
        }
    });
    acts
}

/// Grupos com determinada posição, em ordem.
pub fn groups_at(s: &Settings, d: Dock) -> Vec<Group> {
    Group::ALL.iter().copied().filter(|g| s.dock(*g) == d).collect()
}
