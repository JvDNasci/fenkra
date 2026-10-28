//! Estrutural: painel do membro (família, medida, ancoragem, rotação, cantos), biblioteca de
//! famílias de perfis (cadastro do usuário) e lista de corte.

use crate::app::{fmt, App};
use crate::editor::{expr, icon_choice, section};
use crate::geo_ui::Seed;
use crate::i18n::tr;
use crate::settings::{Settings, UserFamily};
use eframe::egui::{self, RichText};
use fenkra_core::*;
use fenkra_solidos::structural::{self, Family};
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
    section(ui, "Cantos (padrão)");
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
    joints_ui(ui, doc, me, m);
    ui.label(RichText::new(tr("Tudo é associativo: mexer no esqueleto (pontos, linhas) atualiza as barras.")).weak().small());
}

/// Cantos encontrados no último cálculo: ((comando, linha), (comando, linha), ponto).
#[derive(Clone)]
pub struct JointCache(pub Vec<((Id, usize), (Id, usize), [f64; 3])>);

/// Operações de canto por par: para cada encontro deste membro com outro (ou entre linhas dele),
/// o tratamento, quem é o membro 1 (continua) e a folga.
fn joints_ui(ui: &mut egui::Ui, doc: &Document, me: Id, m: &mut StructMember) {
    let all = ui.ctx().data(|d| d.get_temp::<JointCache>(egui::Id::unique("st_joints"))).map(|c| c.0).unwrap_or_default();
    let mine: Vec<(usize, Id, usize, [f64; 3])> = all
        .iter()
        .filter_map(|(a, b, p)| {
            if a.0 == me {
                Some((a.1, b.0, b.1, *p))
            } else if b.0 == me {
                Some((b.1, a.0, a.1, *p))
            } else {
                None
            }
        })
        .collect();
    if mine.is_empty() {
        return;
    }
    section(ui, &format!("Cantos com outros membros ({})", mine.len()));
    let pos = |id: Id| doc.features.iter().position(|f| f.id == id).unwrap_or(0);
    let modes = [(None, "padrão"), (Some(CornerMode::MeiaEsquadria), "Meia-esquadria"), (Some(CornerMode::Aparar), "Aparar"), (Some(CornerMode::Encaixe), "Encaixe"), (Some(CornerMode::Nenhum), "Nenhum")];
    egui::Grid::new("st_joints_grid").striped(true).num_columns(4).show(ui, |ui| {
        for (k, (i, other, j, p)) in mine.into_iter().enumerate() {
            let them = if other == me { format!("{} {}", tr("linha"), j + 1) } else { format!("{} · {} {}", doc.name_of(other), tr("linha"), j + 1) };
            ui.label(format!("{} {} ↔ {}", tr("linha"), i + 1, them)).on_hover_text(format!("{}", crate::app::p3(p, 0)));
            // regra guardada no outro membro (posterior) manda: só mostra
            let other_rule = (other != me && pos(other) > pos(me))
                .then(|| match doc.feature(other).map(|f| &f.kind) {
                    Some(FeatureKind::StructMember(o)) => o.joints.iter().find(|x| x.mine == j && x.other == me && x.other_path == i).cloned(),
                    _ => None,
                })
                .flatten();
            if let Some(r) = other_rule {
                ui.label(RichText::new(format!("{:?} — {} {}", r.mode, tr("definido em"), doc.name_of(other))).weak());
                ui.label("");
                ui.label("");
                ui.end_row();
                continue;
            }
            let at = m.joints.iter().position(|x| (x.mine == i && x.other == other && x.other_path == j) || (other == me && x.mine == j && x.other == me && x.other_path == i));
            let cur = at.map(|a| m.joints[a].mode);
            let label = modes.iter().find(|(x, _)| *x == cur).map(|(_, n)| tr(n).to_string()).unwrap_or_default();
            let mut pick: Option<Option<CornerMode>> = None;
            egui::ComboBox::from_id_salt(("st_jm", k)).selected_text(label).width(110.).show_ui(ui, |ui| {
                for (x, n) in modes {
                    if ui.selectable_label(cur == x, tr(n)).clicked() {
                        pick = Some(x);
                    }
                }
            });
            match pick {
                Some(None) => {
                    if let Some(a) = at {
                        m.joints.remove(a);
                    }
                }
                Some(Some(mode)) => match at {
                    Some(a) => m.joints[a].mode = mode,
                    // padrão: o anterior é o membro 1
                    None => m.joints.push(StructJoint { mine: i, other, other_path: j, mode, first: (pos(me), i) < (pos(other), j), gap: None }),
                },
                None => {}
            }
            let at = m.joints.iter().position(|x| x.mine == i && x.other == other && x.other_path == j);
            if let Some(a) = at.filter(|a| matches!(m.joints[*a].mode, CornerMode::Aparar | CornerMode::Encaixe)) {
                let jt = &mut m.joints[a];
                ui.horizontal(|ui| {
                    ui.label(tr("membro 1:"));
                    ui.selectable_value(&mut jt.first, true, tr("este")).on_hover_text(tr("esta linha continua; a outra para nela"));
                    ui.selectable_value(&mut jt.first, false, tr("o outro"));
                });
                let mut g = jt.gap.clone().unwrap_or_default();
                if ui.add(egui::TextEdit::singleline(&mut g).desired_width(50.).hint_text(tr("folga"))).changed() {
                    jt.gap = (!g.trim().is_empty()).then_some(g);
                }
            } else {
                ui.label("");
                ui.label("");
            }
            ui.end_row();
        }
    });
}

/// Cópia das famílias guardada na memória da interface (o painel não tem acesso às configurações).
#[derive(Clone)]
pub struct FamCache(pub Family);

pub fn windows(app: &mut App, ctx: &egui::Context) {
    let fams: Vec<FamCache> = families(&app.settings).into_iter().map(FamCache).collect();
    ctx.data_mut(|d| d.insert_temp(egui::Id::unique("st_fams"), fams));
    // cantos para o painel do membro em edição
    if app.session.as_ref().is_some_and(|s| matches!(app.doc().feature(s.id).map(|f| &f.kind), Some(FeatureKind::StructMember(_)))) {
        let j = JointCache(fenkra_solidos::member_joints(app.doc()));
        ctx.data_mut(|d| d.insert_temp(egui::Id::unique("st_joints"), j));
    }
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
                        { let __r = ui.add(crate::ui_ext::dv(&mut f.anchor).range(0..=9)); crate::ui_ext::wheel(ui, &__r, &mut f.anchor); __r };
                        ui.label(tr("rotação"));
                        { let __r = ui.add(crate::ui_ext::dv(&mut f.angle).suffix("°")); crate::ui_ext::wheel(ui, &__r, &mut f.angle); __r };
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
                                    ui.add(crate::ui_ext::dv(v).speed(0.1).max_decimals(3));
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
    let rows = fenkra_solidos::cut_list(app.doc());
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
            let stock = app.open.doc.props.stock_length.trim().to_string();
            egui::Grid::new("lista_corte").striped(true).num_columns(4).show(ui, |ui| {
                for h in ["Perfil", "Comprimento", "Qtd.", "Barras"] {
                    ui.label(RichText::new(tr(h)).strong());
                }
                ui.end_row();
                for r in &rows {
                    ui.label(&r.designation);
                    ui.label(format!("{} mm", fmt(r.length)));
                    ui.label(format!("{}", r.count));
                    if r.bars > 1 {
                        ui.label(RichText::new(format!("⚠ {} ({})", r.bars * r.count, tr("emenda"))).color(egui::Color32::from_rgb(240, 170, 80)))
                            .on_hover_text(format!("{} {} mm", tr("cada peça precisa de emenda: matéria-prima de"), stock));
                    } else {
                        ui.label(format!("{}", r.count));
                    }
                    ui.end_row();
                }
            });
            if stock.is_empty() {
                ui.label(RichText::new(tr("Defina o comprimento máximo da barra nas propriedades da peça para ver as emendas.")).weak().small());
            }
            let total: f64 = rows.iter().map(|r| r.length * r.count as f64).sum();
            ui.label(RichText::new(format!("total de barras: {} mm", fmt(total))).weak());
            if ui.button(tr("💾 Exportar CSV…")).clicked() {
                export = true;
            }
        });
    if export {
        if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name("lista_de_corte.csv").save_file() {
            let mut t = String::from("perfil;comprimento_mm;quantidade;barras_materia_prima\n");
            for r in &rows {
                t += &format!("{};{};{};{}\n", r.designation, fmt(r.length), r.count, r.bars * r.count);
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
