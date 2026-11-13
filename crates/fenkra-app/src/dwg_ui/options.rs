// SPDX-License-Identifier: LGPL-2.1-or-later
//! Barra de opções da ferramenta (B225): ao armar uma ferramenta de vista, uma barra ancorada na
//! folha traz direção (menu com miniaturas), escala (Auto, presets e livre), estilo e rótulo,
//! editáveis enquanto o fantasma segue o cursor. O que falta (vista mãe, origem) aparece ali com o
//! motivo e um atalho para resolver, no lugar do aviso depois do clique.

use super::*;

/// Escalas de prateleira para o Auto e para o menu.
const PRESETS: [f64; 10] = [10., 5., 2., 1., 0.5, 0.2, 0.1, 0.05, 0.02, 0.01];

/// Ferramentas que mostram a barra.
pub fn has_bar(tool: u8) -> bool {
    matches!(tool, c::SET | c::VIEW | c::PROJ | c::SECTION | c::DETAIL | c::ISO | c::FLAT | c::SCHEMA | c::VIEW_CAM | c::CENTER | c::T_BOM | c::T_HOLES | c::T_BENDS | c::T_CUT)
}

fn tool_name(tool: u8) -> &'static str {
    match tool {
        c::SET => "Três vistas e isométrica",
        c::VIEW => "Vista única",
        c::PROJ => "Vista projetada",
        c::SECTION => "Vista em corte",
        c::DETAIL => "Vista de detalhe",
        c::ISO => "Vista isométrica",
        c::FLAT => "Vista planificada",
        c::SCHEMA => "Esquema cinemático",
        c::VIEW_CAM => "Vista pela câmera do 3D",
        c::CENTER => "Marca de centro",
        c::T_BOM => "Lista de peças",
        c::T_HOLES => "Tabela de furos",
        c::T_BENDS => "Tabela de dobras",
        _ => "Lista de corte",
    }
}

/// A vista herda a escala da mãe (a projetada e o corte sempre; o detalhe, ampliada).
fn inherits_scale(tool: u8) -> bool {
    matches!(tool, c::PROJ | c::SECTION)
}

/// Escala que a ferramenta usa: a escolhida na barra ou, no Auto, a da mãe (detalhe: o dobro) ou a
/// que cabe na folha.
pub fn scale_for(st: &mut DwgState, tool: u8) -> f64 {
    if inherits_scale(tool) || tool == c::DETAIL {
        let p = parent_of(st).and_then(|id| st.d.view(id)).map(|v| v.scale);
        if inherits_scale(tool) {
            return p.unwrap_or(1.);
        }
        if st.scale_auto {
            return p.unwrap_or(1.) * 2.;
        }
    }
    if st.scale_auto {
        fit_scale(st, tool)
    } else {
        st.new_scale
    }
}

/// Maior escala de prateleira em que a vista (ou o conjunto) cabe na folha. Se a folha já tem
/// vistas, mantém a escala delas: o desenho todo na mesma escala.
fn fit_scale(st: &mut DwgState, tool: u8) -> f64 {
    if let Some(v) = st.d.views.iter().find(|v| v.sheet == st.sheet && matches!(v.kind, ViewKind::Standard(_) | ViewKind::Projected { .. } | ViewKind::Camera { .. } | ViewKind::Flat)) {
        return v.scale;
    }
    let Some(src) = st.source.clone() else { return 1. };
    // B415: a área que sobra dentro da moldura, descontada a legenda (largura e altura)
    let size = st.d.sheet(st.sheet).map(|s| s.size()).unwrap_or([420., 297.]);
    let (m0, m1) = st.d.sheet(st.sheet).map(fenkra_desenho::ocupacao::area_moldura).unwrap_or(([20., 10.], [size[0] - 10., size[1] - 10.]));
    let legenda = fenkra_desenho::ocupacao::legenda_caixa(st.d.title.style, size);
    let livre = [m1[0] - m0[0], m1[1] - legenda.1[1]];
    let mut t = ghost::scratch(st, 2);
    let sheet = st.sheet;
    let nd = st.new_dir;
    let mut ext = |st: &mut DwgState, kind: ViewKind| -> (f64, f64) {
        let id = t.add_view(sheet, src.clone(), kind, [0., 0.], 1.);
        let p = st.models.proj(&t, id);
        ((p.hi[0] - p.lo[0]).max(0.), (p.hi[1] - p.lo[1]).max(0.))
    };
    let ((w, h), margin) = match tool {
        c::SET => {
            let (f, l, tp) = (ext(st, ViewKind::Standard(Dir::Frente)), ext(st, ViewKind::Standard(Dir::Esquerda)), ext(st, ViewKind::Standard(Dir::Topo)));
            ((f.0 + l.0, f.1 + tp.1), 60.)
        }
        c::ISO => (ext(st, ViewKind::Standard(Dir::Iso)), 30.),
        c::FLAT => (ext(st, ViewKind::Flat), 30.),
        c::SCHEMA => (ext(st, ViewKind::Schematic(nd)), 30.),
        _ => (ext(st, ViewKind::Standard(nd)), 30.),
    };
    if w <= 0. || h <= 0. {
        return 1.;
    }
    PRESETS.iter().copied().find(|s| w * s + margin <= livre[0] && h * s + margin <= livre[1]).unwrap_or(PRESETS[PRESETS.len() - 1])
}

/// Miniatura de uma direção: um cubo em perspectiva cavaleira com a(s) face(s) vista(s) pintadas.
/// Faces escondidas (trás, inferior, lateral esquerda) ficam pintadas com o contorno do cubo atrás.
fn dir_thumb(painter: &egui::Painter, ui: &egui::Ui, r: Rect, dir: Dir) {
    let (s, d) = (r.width().min(r.height()) * 0.5, r.width().min(r.height()) * 0.25);
    let f0 = Pos2::new(r.min.x + d * 0.6, r.min.y + d * 1.4);
    let off = Vec2::new(d, -d);
    let front = [f0, f0 + Vec2::new(s, 0.), f0 + Vec2::new(s, s), f0 + Vec2::new(0., s)];
    let back = front.map(|p| p + off);
    let face = |ids: [usize; 4], from_back: [bool; 4]| -> Vec<Pos2> { ids.iter().zip(from_back).map(|(i, b)| if b { back[*i] } else { front[*i] }).collect() };
    let faces: Vec<(Vec<Pos2>, bool)> = match dir {
        Dir::Frente => vec![(front.to_vec(), true)],
        Dir::Tras => vec![(back.to_vec(), false)],
        Dir::Topo => vec![(face([0, 1, 1, 0], [false, false, true, true]), true)],
        Dir::Base => vec![(face([3, 2, 2, 3], [false, false, true, true]), false)],
        Dir::Direita => vec![(face([1, 1, 2, 2], [false, true, true, false]), true)],
        Dir::Esquerda => vec![(face([0, 0, 3, 3], [false, true, true, false]), false)],
        Dir::Iso => vec![(front.to_vec(), true), (face([0, 1, 1, 0], [false, false, true, true]), true), (face([1, 1, 2, 2], [false, true, true, false]), true)],
    };
    let line = ui.visuals().widgets.noninteractive.fg_stroke;
    let weak = Stroke::new(1., ui.visuals().weak_text_color());
    for (a, b) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
        painter.line_segment([back[a], back[b]], weak);
        painter.line_segment([front[a], front[b]], line);
    }
    for i in 0..4 {
        painter.line_segment([front[i], back[i]], weak);
    }
    let fill = ui.visuals().selection.bg_fill;
    let edge = Stroke::new(1.6, ui.visuals().selection.stroke.color);
    for (pts, visible) in faces {
        painter.add(egui::Shape::convex_polygon(pts, if visible { fill } else { fill.gamma_multiply(0.45) }, edge));
    }
}

/// Menu de direção com miniaturas.
fn dir_menu(ui: &mut egui::Ui, dir: &mut Dir) {
    ui.menu_button(format!("{}: {}", tr("Direção"), tr(dir.label())), |ui| {
        egui::Grid::new("dwg_dir_menu").spacing(Vec2::new(4., 4.)).show(ui, |ui| {
            for (i, d) in Dir::ALL.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(Vec2::new(104., 62.), egui::Sense::click());
                // B321: anel de foco quando o Tab chega aqui
                let resp = crate::teclado::anel(ui, resp);
                if *d == *dir {
                    ui.painter().rect_filled(r, 4., ui.visuals().selection.bg_fill.gamma_multiply(0.5));
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, 4., ui.visuals().widgets.hovered.bg_fill);
                }
                dir_thumb(ui.painter(), ui, Rect::from_min_size(r.min + Vec2::new(36., 2.), Vec2::splat(34.)), *d);
                ui.painter().text(r.center_bottom() - Vec2::new(0., 3.), egui::Align2::CENTER_BOTTOM, tr(d.label()), egui::FontId::proportional(11.), ui.visuals().text_color());
                if resp.clicked() {
                    *dir = *d;
                    ui.close();
                }
                if (i + 1) % 4 == 0 {
                    ui.end_row();
                }
            }
        });
    });
}

/// A barra. `cur` é a peça ou montagem aberta no 3D (atalho para a origem que falta).
pub fn bar(ctx: &egui::Context, st: &mut DwgState, rect: Rect, cur: Option<Source>) {
    let Some(tool) = st.tool else { return };
    if !has_bar(tool) {
        return;
    }
    egui::Area::new(egui::Id::unique("dwg_barra_opcoes")).order(egui::Order::Foreground).fixed_pos(rect.left_top() + Vec2::new(12., 44.)).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_max_width((rect.width() - 40.).max(240.));
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr(tool_name(tool))).strong());
                ui.separator();
                if matches!(tool, c::VIEW | c::SCHEMA) {
                    dir_menu(ui, &mut st.new_dir);
                }
                if tool == c::CENTER {
                    centros_ui::barra(ui, st);
                } else if !(c::T_BOM..=c::T_CUT).contains(&tool) {
                    scale_ui(ui, st, tool);
                    ui.separator();
                    ui.label(tr("Estilo"));
                    egui::ComboBox::from_id_salt("dwg_barra_render").selected_text(tr(st.new_render.label())).show_ui(ui, |ui| {
                        for r in Render::ALL {
                            ui.selectable_value(&mut st.new_render, r, tr(r.label()));
                        }
                    });
                    crate::editor::check(ui, &mut st.new_hidden, "Com ocultas");
                    ui.separator();
                    ui.label(tr("Rótulo"));
                    ui.add(egui::TextEdit::singleline(&mut st.new_label).desired_width(110.).hint_text(tr("automático")));
                }
                if tool == c::SECTION {
                    corte_ui::barra(ui, st);
                }
                if tool == c::DETAIL {
                    ui.separator();
                    ui.label(tr("Contorno"));
                    let antes = st.detalhe_perfil;
                    ui.selectable_value(&mut st.detalhe_perfil, false, tr("Círculo"));
                    ui.selectable_value(&mut st.detalhe_perfil, true, tr("Perfil livre"));
                    if antes != st.detalhe_perfil {
                        st.clicks.clear();
                        st.corte_pronto = false;
                    }
                }
            });
            if let Some(reason) = block_reason(st) {
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(ui.visuals().warn_fg_color, format!("▲ {}", tr(reason)));
                    resolve_ui(ui, st, tool, cur);
                });
            }
        });
    });
}

fn scale_ui(ui: &mut egui::Ui, st: &mut DwgState, tool: u8) {
    ui.label(tr("Escala"));
    if inherits_scale(tool) {
        let p = parent_of(st).and_then(|id| st.d.view(id)).map(|v| v.scale);
        ui.label(RichText::new(match p {
            Some(s) => format!("{} ({})", tr("igual à vista mãe"), scale_text(s)),
            None => tr("igual à vista mãe").to_string(),
        })
        .weak());
        return;
    }
    let shown = if st.scale_auto { tr("Auto").to_string() } else { scale_text(st.new_scale) };
    egui::ComboBox::from_id_salt("dwg_barra_escala").selected_text(shown).show_ui(ui, |ui| {
        if ui.selectable_label(st.scale_auto, tr("Auto")).clicked() {
            st.scale_auto = true;
        }
        for v in PRESETS {
            if ui.selectable_label(!st.scale_auto && st.new_scale == v, scale_text(v)).clicked() {
                st.scale_auto = false;
                st.new_scale = v;
            }
        }
    });
    if st.scale_auto {
        // mostra o valor que o Auto escolheu agora
        let now = scale_for(st, tool);
        ui.label(RichText::new(scale_text(now)).weak());
    } else if ui.add(crate::ui_ext::dv(&mut st.new_scale).range(0.001..=100.).speed(0.01)).changed() {
        st.scale_auto = false;
    }
}

/// Atalhos para resolver o que bloqueia a ferramenta.
fn resolve_ui(ui: &mut egui::Ui, st: &mut DwgState, tool: u8, cur: Option<Source>) {
    if needs_parent(tool) {
        let views: Vec<(Id, String)> = st.d.views.iter().filter(|v| v.sheet == st.sheet && !matches!(v.kind, ViewKind::Detail { .. })).map(|v| (v.id, v.name.clone())).take(5).collect();
        for (id, name) in views {
            if ui.small_button(format!("{} {name}", tr("Usar"))).clicked() {
                st.sel = Some(id);
            }
        }
    } else if needs_source(tool) && st.source.is_none() {
        if let Some(s) = cur {
            if ui.button(tr("Peça/montagem aberta")).on_hover_text(tr("usa o arquivo salvo da peça (ou da montagem) aberta")).clicked() {
                let path = st.rel(std::path::Path::new(&s.path));
                st.source = Some(Source { path, ..s });
            }
        }
        if ui.button(tr("Escolher arquivo…")).clicked() {
            if let Some(p) = rfd::FileDialog::new().add_filter(tr("Peça ou montagem"), &["fkp", "fka"]).pick_file() {
                st.source = Some(Source { path: st.rel(&p), display_state: String::new(), hidden: vec![] });
            }
        }
    }
}
