//! Estrutural: painel do membro (família, medida, ancoragem, rotação, cantos), biblioteca de
//! famílias de perfis (cadastro do usuário) e lista de corte.

use crate::app::{fmt, App};
use crate::editor::{expr, icon_choice, section};
use crate::geo_ui::Seed;
use crate::i18n::tr;
use crate::settings::{Settings, UserFamily};
use eframe::egui::{self, RichText};
use scc_core::*;
use scc_solidos::structural::{self, Family};
use std::collections::HashMap;

/// Famílias disponíveis: as cadastradas pelo usuário primeiro, depois as de fábrica.
pub fn families(s: &Settings) -> Vec<Family> {
    let mut v: Vec<Family> = s.user_profiles.iter().map(|u| Family { name: u.name.clone(), kind: u.kind, sizes: u.sizes.clone() }).collect();
    v.extend(structural::builtin());
    v
}

pub fn member_panel(ui: &mut egui::Ui, doc: &Document, me: Id, m: &mut StructMember, vars: &HashMap<String, f64>, seed: &Seed) {
    let fams: Vec<Family> = ui.ctx().data(|d| d.get_temp::<Vec<FamCache>>(egui::Id::unique("st_fams"))).map(|v| v.into_iter().map(|f| f.0).collect()).unwrap_or_else(structural::builtin);
    section(ui, "Perfil");
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Família")));
        egui::ComboBox::from_id_salt("st_fam").selected_text(&m.family).width(180.).show_ui(ui, |ui| {
            for f in &fams {
                if ui.selectable_label(f.name == m.family, &f.name).clicked() && f.name != m.family {
                    m.family = f.name.clone();
                    m.kind = f.kind;
                    if let Some((n, d)) = f.sizes.first() {
                        m.designation = n.clone();
                        m.dims = d.iter().map(|v| fmt(*v)).collect();
                    }
                }
            }
        });
    });
    if let Some(f) = fams.iter().find(|f| f.name == m.family) {
        ui.horizontal(|ui| {
            ui.add_sized([96., 18.], egui::Label::new(tr("Medida")));
            egui::ComboBox::from_id_salt("st_size").selected_text(&m.designation).width(180.).show_ui(ui, |ui| {
                for (n, d) in &f.sizes {
                    if ui.selectable_label(*n == m.designation, n).clicked() {
                        m.designation = n.clone();
                        m.dims = d.iter().map(|v| fmt(*v)).collect();
                    }
                }
            });
        });
    }
    let names = structural::dim_names(m.kind);
    m.dims.resize(names.len(), "1".into());
    egui::CollapsingHeader::new(tr("Cotas da seção")).id_salt("st_cotas").show(ui, |ui| {
        for (i, n) in names.iter().enumerate() {
            expr(ui, n, &mut m.dims[i], vars, "mm");
        }
    });
    section(ui, "Linhas guia (esqueleto)");
    crate::geo_ui::curves_list(ui, "st_paths", "Linha", doc, me, &mut m.paths, seed, 1);
    section(ui, "Ancoragem e orientação");
    // grade 3×3 dos pontos de ancoragem da seção + centróide
    let grid = [[1u8, 2, 3], [8, 0, 4], [7, 6, 5]];
    ui.horizontal(|ui| {
        egui::Grid::new("st_anchor").spacing([2., 2.]).show(ui, |ui| {
            for row in grid {
                for a in row {
                    let txt = if a == 0 { "◉" } else { "•" };
                    if ui.selectable_label(m.anchor == a, txt).on_hover_text(tr("ponto da seção que fica sobre a linha guia")).clicked() {
                        m.anchor = a;
                    }
                }
                ui.end_row();
            }
        });
        if ui.selectable_label(m.anchor == 9, tr("centróide")).clicked() {
            m.anchor = 9;
        }
    });
    expr(ui, "Rotação", &mut m.angle, vars, "°");
    expr(ui, "Deslocamento X", &mut m.off_x, vars, "mm");
    expr(ui, "Deslocamento Y", &mut m.off_y, vars, "mm");
    expr(ui, "Prolongar início", &mut m.ext1, vars, "mm");
    expr(ui, "Prolongar fim", &mut m.ext2, vars, "mm");
    section(ui, "Cantos");
    let opts = [
        ("st_canto_meia", "Meia-esquadria", "Os dois membros cortados na bissetriz", m.corner == CornerMode::MeiaEsquadria),
        ("st_canto_aparar", "Aparar", "O posterior para na face do anterior", m.corner == CornerMode::Aparar),
        ("st_canto_encaixe", "Encaixe", "A ponta acompanha o contorno do outro membro", m.corner == CornerMode::Encaixe),
        ("st_canto_nenhum", "Nenhum", "Sem tratamento", m.corner == CornerMode::Nenhum),
    ];
    if let Some(i) = icon_choice(ui, &opts) {
        m.corner = [CornerMode::MeiaEsquadria, CornerMode::Aparar, CornerMode::Encaixe, CornerMode::Nenhum][i];
    }
    expr(ui, "Folga de solda", &mut m.gap, vars, "mm");
    ui.label(RichText::new(tr("Tudo é associativo: mexer no esqueleto (pontos, linhas) atualiza as barras.")).weak().small());
}

/// Cópia das famílias guardada na memória da interface (o painel não tem acesso às configurações).
#[derive(Clone)]
pub struct FamCache(pub Family);

pub fn windows(app: &mut App, ctx: &egui::Context) {
    let fams: Vec<FamCache> = families(&app.settings).into_iter().map(FamCache).collect();
    ctx.data_mut(|d| d.insert_temp(egui::Id::unique("st_fams"), fams));
    library(app, ctx);
    cut_list(app, ctx);
}

fn library(app: &mut App, ctx: &egui::Context) {
    if !app.st_lib_open {
        return;
    }
    let mut open = true;
    let s = &mut app.settings;
    let sel = &mut app.st_lib_sel;
    let fechar = egui::Id::unique(("concluir_janela", "st1"));
    if ctx.data_mut(|m| m.remove_temp::<bool>(fechar)).unwrap_or(false) {
        open = false;
    }
    egui::Window::new(tr("⌶ Biblioteca de perfis"))
        .id(egui::Id::unique("janela_perfis"))
        .open(&mut open)
        .default_size([620., 420.])
        .resizable(true)
        .show(ctx, |ui| {
            if crate::app::done_bar(ui) {
                ui.ctx().data_mut(|m| m.insert_temp(fechar, true));
            }
            ui.label(RichText::new(tr("Famílias cadastradas por você (salvas nas configurações). As de fábrica servem de modelo: copie e ajuste.")).weak().small());
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(200.);
                    for (i, f) in s.user_profiles.iter().enumerate() {
                        if ui.selectable_label(*sel == i, &f.name).clicked() {
                            *sel = i;
                        }
                    }
                    ui.separator();
                    if ui.button(tr("+ Nova família")).clicked() {
                        s.user_profiles.push(UserFamily { name: format!("Família {}", s.user_profiles.len() + 1), kind: SectionKind::TuboQuadrado, sizes: vec![("40x3".into(), vec![40., 3.])], anchor: 0, angle: 0. });
                        *sel = s.user_profiles.len() - 1;
                    }
                    ui.menu_button(tr("Copiar de fábrica…"), |ui| {
                        for f in structural::builtin() {
                            if ui.button(&f.name).clicked() {
                                s.user_profiles.push(UserFamily { name: format!("{} (minha)", f.name), kind: f.kind, sizes: f.sizes.clone(), anchor: 0, angle: 0. });
                                *sel = s.user_profiles.len() - 1;
                                ui.close();
                            }
                        }
                    });
                });
                ui.separator();
                ui.vertical(|ui| {
                    let Some(f) = s.user_profiles.get_mut(*sel) else {
                        ui.label(RichText::new(tr("Crie ou copie uma família.")).weak());
                        return;
                    };
                    ui.horizontal(|ui| {
                        ui.label(tr("Nome"));
                        ui.text_edit_singleline(&mut f.name);
                    });
                    egui::ComboBox::from_id_salt("lib_kind").selected_text(structural::kind_label(f.kind)).show_ui(ui, |ui| {
                        for k in structural::KINDS {
                            ui.selectable_value(&mut f.kind, k, structural::kind_label(k));
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("Regra: ancoragem"));
                        ui.add(egui::DragValue::new(&mut f.anchor).range(0..=9));
                        ui.label(tr("rotação"));
                        ui.add(egui::DragValue::new(&mut f.angle).suffix("°"));
                    });
                    ui.label(RichText::new(tr("Ancoragem: 0 centro, 1–8 cantos/meios da caixa, 9 centróide — aplicada quando o membro nasce do arame 3D.")).weak().small());
                    let names = structural::dim_names(f.kind);
                    egui::ScrollArea::vertical().max_height(240.).show(ui, |ui| {
                        egui::Grid::new("lib_sizes").striped(true).show(ui, |ui| {
                            ui.label(RichText::new(tr("Medida")).strong());
                            for n in names {
                                ui.label(RichText::new(*n).strong());
                            }
                            ui.end_row();
                            let mut del = None;
                            for (i, (n, d)) in f.sizes.iter_mut().enumerate() {
                                d.resize(names.len(), 1.);
                                ui.add(egui::TextEdit::singleline(n).desired_width(80.));
                                for v in d.iter_mut() {
                                    ui.add(egui::DragValue::new(v).speed(0.1).max_decimals(3));
                                }
                                if ui.small_button("✖").clicked() {
                                    del = Some(i);
                                }
                                ui.end_row();
                            }
                            if let Some(i) = del {
                                f.sizes.remove(i);
                            }
                        });
                    });
                    if ui.button(tr("+ medida")).clicked() {
                        f.sizes.push((format!("nova {}", f.sizes.len() + 1), vec![1.; names.len()]));
                    }
                    let bad: Vec<String> = f.sizes.iter().filter(|(_, d)| structural::section(f.kind, d).is_err()).map(|(n, _)| n.clone()).collect();
                    if !bad.is_empty() {
                        ui.label(RichText::new(format!("⚠ medidas inválidas: {}", bad.join(", "))).color(egui::Color32::from_rgb(240, 170, 80)));
                    }
                });
            });
        });
    if !open {
        app.st_lib_open = false;
    }
}

fn cut_list(app: &mut App, ctx: &egui::Context) {
    if !app.st_cut_open {
        return;
    }
    let rows = scc_solidos::cut_list(app.doc());
    let mut open = true;
    let mut export = false;
    let fechar = egui::Id::unique(("concluir_janela", "st2"));
    if ctx.data_mut(|m| m.remove_temp::<bool>(fechar)).unwrap_or(false) {
        open = false;
    }
    egui::Window::new(tr("📋 Lista de corte"))
        .id(egui::Id::unique("janela_lista_corte"))
        .open(&mut open)
        .default_width(420.)
        .show(ctx, |ui| {
            if crate::app::done_bar(ui) {
                ui.ctx().data_mut(|m| m.insert_temp(fechar, true));
            }
            if rows.is_empty() {
                ui.label(RichText::new(tr("Nenhum membro estrutural na peça.")).weak());
                return;
            }
            egui::Grid::new("lista_corte").striped(true).num_columns(3).show(ui, |ui| {
                for h in ["Perfil", "Comprimento", "Qtd."] {
                    ui.label(RichText::new(tr(h)).strong());
                }
                ui.end_row();
                for r in &rows {
                    ui.label(&r.designation);
                    ui.label(format!("{} mm", fmt(r.length)));
                    ui.label(format!("{}", r.count));
                    ui.end_row();
                }
            });
            let total: f64 = rows.iter().map(|r| r.length * r.count as f64).sum();
            ui.label(RichText::new(format!("total de barras: {} mm", fmt(total))).weak());
            if ui.button(tr("💾 Exportar CSV…")).clicked() {
                export = true;
            }
        });
    if export {
        if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name("lista_de_corte.csv").save_file() {
            let mut t = String::from("perfil;comprimento_mm;quantidade\n");
            for r in &rows {
                t += &format!("{};{};{}\n", r.designation, fmt(r.length), r.count);
            }
            match std::fs::write(&p, t) {
                Ok(()) => app.toast(tr("Lista de corte exportada"), false),
                Err(e) => app.toast(format!("Erro: {e}"), true),
            }
        }
    }
    if !open {
        app.st_cut_open = false;
    }
}
