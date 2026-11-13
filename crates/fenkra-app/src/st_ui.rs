// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: painel do membro (família, medida, ancoragem, rotação, cantos), biblioteca de
//! famílias de perfis (cadastro do usuário) e lista de corte.

use crate::app::{fmt, App, Sel};
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
    let mut v: Vec<Family> = s.user_profiles.iter().map(|u| Family { name: u.name.clone(), norm: crate::st_painel::norma_do_usuario().into(), kind: u.kind, sizes: u.sizes.clone() }).collect();
    v.extend(structural::builtin());
    v.extend(fenkra_solidos::perfis_catalogo::familias());
    v
}

pub fn member_panel(ui: &mut egui::Ui, doc: &Document, me: Id, m: &mut StructMember, vars: &HashMap<String, f64>, seed: &Seed) {
    let fams: Vec<Family> = ui.ctx().data(|d| d.get_temp::<Vec<FamCache>>(egui::Id::unique("st_fams"))).map(|v| v.into_iter().map(|f| f.0).collect()).unwrap_or_else(structural::builtin);
    section(ui, "Perfil");
    crate::st_painel::seletor(ui, &fams, m);
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
    crate::st_painel::ancoragem(ui, m, vars);
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
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Material")));
        ui.add(egui::TextEdit::singleline(&mut m.material).desired_width(180.).hint_text(doc_material(doc))).on_hover_text(tr("Material do membro (vazio = o da peça)"));
    });
    joints_ui(ui, doc, me, m, vars);
    crate::st_dobra_ui::ficha_do_tubo(ui, me);
    ui.label(RichText::new(tr("Tudo é associativo: mexer no esqueleto (pontos, linhas) atualiza as barras.")).weak().small());
}

fn doc_material(doc: &Document) -> String {
    doc.props.material.clone()
}

/// Cantos encontrados no último cálculo: ((comando, linha), (comando, linha), ponto).
#[derive(Clone)]
pub struct JointCache(pub Vec<((Id, usize), (Id, usize), [f64; 3])>);

/// Operações de canto por par: para cada encontro deste membro com outro (ou entre linhas dele),
/// o tratamento, quem é o membro 1 (continua) e a folga.
fn joints_ui(ui: &mut egui::Ui, doc: &Document, me: Id, m: &mut StructMember, vars: &HashMap<String, f64>) {
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
    // B326: a dobra de tubo só vale em tubo, entre duas linhas do mesmo comando
    let tubo = crate::st_dobra_ui::e_tubo(m.kind);
    let sugestao = crate::st_dobra_ui::raio_inicial(m, vars);
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
            let com_dobra = tubo && other == me;
            let opcoes: Vec<(Option<CornerMode>, &str)> = modes.iter().copied().chain(com_dobra.then_some((Some(CornerMode::Dobra), "Dobra de tubo"))).collect();
            let label = opcoes.iter().find(|(x, _)| *x == cur).map(|(_, n)| tr(n).to_string()).unwrap_or_default();
            let mut pick: Option<Option<CornerMode>> = None;
            egui::ComboBox::from_id_salt(("st_jm", k)).selected_text(label).width(110.).show_ui(ui, |ui| {
                for (x, n) in opcoes.iter().copied() {
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
                Some(Some(mode)) => {
                    let bend = (mode == CornerMode::Dobra).then(|| TubeBend { radius: sugestao.clone(), ..Default::default() });
                    match at {
                        Some(a) => {
                            m.joints[a].mode = mode;
                            // sair da dobra apaga os dados dela; entrar nela ganha um raio inicial
                            if mode != CornerMode::Dobra {
                                m.joints[a].bend = None;
                            } else if m.joints[a].bend.is_none() {
                                m.joints[a].bend = bend;
                            }
                        }
                        // padrão: o anterior é o membro 1
                        None => m.joints.push(StructJoint { mine: i, other, other_path: j, mode, first: (pos(me), i) < (pos(other), j), gap: None, bend }),
                    }
                }
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
    // B326: editor de cada nó de dobra (modo, raio, regra, resultado)
    let dobras: Vec<usize> = (0..m.joints.len()).filter(|k| m.joints[*k].mode == CornerMode::Dobra && m.joints[*k].other == me).collect();
    for k in dobras {
        crate::st_dobra_ui::editor_do_no(ui, doc, me, m, k, vars);
    }
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
        crate::st_dobra_ui::guardar(ctx, app.doc());
    }
    crate::st_dobra_ui::janela_regras(app, ctx);
    library(app, ctx);
    cut_list(app, ctx);
    crate::st_cantos_ui::window(app, ctx);
    crate::st_montagem_ui::window(app, ctx);
    crate::st_acesso_ui::window(app, ctx);
    crate::st_corte_tubo_ui::window(app, ctx);
}

/// Gerar montagem: uma peça por grupo de peças iguais e a montagem, em `<nome>_montagem` ao lado
/// do arquivo da peça (a pasta existente é reaproveitada: gerar de novo atualiza). Abre o resultado.
pub fn generate_assembly(app: &mut App) {
    if app.open.frozen.is_some() {
        app.toast(tr("Revisão congelada: abra a versão de trabalho para gerar a montagem"), true);
        return;
    }
    // no projeto, o arquivo da peça é o de trabalho dentro da pasta do projeto
    let pai = match (&app.project, app.open.entry) {
        (Some(p), Some(id)) => p.entry(id).ok().map(|e| p.root.join(&e.file)),
        _ => app.open.file.clone(),
    };
    let Some(pai) = pai.filter(|p| p.is_file()) else {
        app.toast(tr("Salve a peça antes de gerar a montagem"), true);
        return;
    };
    if app.open.modified {
        app.toast(tr("A peça tem alterações não salvas: salve antes de gerar a montagem"), true);
        return;
    }
    let nome: String = app.open.doc.name.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' }).collect();
    let nome = if nome.is_empty() { "estrutura".to_string() } else { nome };
    let saida = pai.parent().unwrap_or(std::path::Path::new(".")).join(format!("{nome}_montagem"));
    match fenkra_montagem::estrutura::estrutura_para_montagem(&pai, &saida) {
        Ok(g) => {
            let inst: usize = g.grupos.iter().map(|x| x.quantidade).sum();
            app.toast(format!("{}: {} {}, {} {}", tr("Montagem gerada"), g.grupos.len(), tr("peças"), inst, tr("instâncias")), false);
            app.open_path(&g.montagem);
        }
        Err(e) => app.toast(tr(&e).to_string(), true),
    }
}

fn library(app: &mut App, ctx: &egui::Context) {
    if !app.st_lib_open {
        return;
    }
    let mut open = true;
    let s = &mut app.settings;
    let sel = &mut app.st_lib_sel;
    let medida_id = egui::Id::unique("st_lib_medida");
    let mut medida_v: usize = ctx.data(|d| d.get_temp(medida_id)).unwrap_or(0);
    let medida = &mut medida_v;
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
                            let tipo = f.kind;
                            for (i, (n, d)) in f.sizes.iter_mut().enumerate() {
                                d.resize(names.len(), 1.);
                                // B254: a linha da medida inválida fica marcada, com o motivo na dica
                                let erro = structural::section(tipo, d).err();
                                let escolhida = *medida == i;
                                let nome = ui.add(egui::TextEdit::singleline(n).desired_width(80.).text_color_opt(erro.as_ref().map(|_| crate::theme::st_err())));
                                if nome.clicked() || nome.gained_focus() {
                                    *medida = i;
                                }
                                for v in d.iter_mut() {
                                    let r = ui.add(crate::ui_ext::dv(v).speed(0.1).max_decimals(3));
                                    if r.dragged() || r.gained_focus() {
                                        *medida = i;
                                    }
                                }
                                if let Some(e) = &erro {
                                    ui.label(RichText::new("⚠").color(crate::theme::st_err())).on_hover_text(e.clone());
                                } else {
                                    ui.label(if escolhida { "◀" } else { "" });
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
                        ui.label(RichText::new(format!("{} {}", tr("⚠ medidas inválidas:"), bad.join(", "))).color(crate::theme::st_warn()));
                    }
                    // o mesmo desenho do painel do membro, com o nome de cada cota, para a medida escolhida
                    let ix = (*medida).min(f.sizes.len().saturating_sub(1));
                    if let Some((n, d)) = f.sizes.get(ix) {
                        ui.separator();
                        ui.label(RichText::new(format!("{} {}", tr("Desenho da medida"), n)).small());
                        crate::st_painel::desenho_da_medida(ui, f.kind, d);
                    }
                });
            });
        });
    ctx.data_mut(|d| d.insert_temp(medida_id, medida_v));
    if !open {
        app.st_lib_open = false;
    }
}

/// Estado da janela da lista de corte (ordem, filtro e linha clicada), guardado na memória da interface.
#[derive(Clone, Default)]
struct CutUi {
    /// coluna de ordenação: 0 perfil, 1 material, 2 comprimento, 3 ângulo 1, 4 ângulo 2, 5 quantidade
    sort: usize,
    desc: bool,
    material: String,
    /// primeiro membro da linha clicada (comando, trecho)
    clicada: Option<(Id, usize)>,
    /// seleção do 3D já mostrada na lista (para rolar até a linha só quando ela muda)
    vista_sel: Option<Id>,
    /// B332: opções e conferência dos dados de dobra para máquina
    maq: crate::st_maquina_ui::Estado,
}

fn ang_txt(a: Option<f64>) -> String {
    a.map(|v| format!("{}°", fmt(v))).unwrap_or_else(|| "—".into())
}

/// Texto da ponta na lista de corte: a ponta de encaixe (B387) diz que é encaixe em vez de "—".
fn ponta_txt(a: Option<f64>, encaixe: bool) -> String {
    if encaixe && a.is_none() {
        tr("encaixe").to_string()
    } else {
        ang_txt(a)
    }
}

/// Ordena as linhas pela coluna escolhida (estável; empate cai na ordem do item).
fn ordenar(rows: &mut [fenkra_solidos::CutRow], col: usize, desc: bool) {
    let num = |a: Option<f64>| a.unwrap_or(f64::MAX);
    rows.sort_by(|a, b| {
        let o = match col {
            0 => a.designation.cmp(&b.designation),
            1 => a.material.cmp(&b.material),
            2 => a.length.total_cmp(&b.length),
            3 => num(a.angle1).total_cmp(&num(b.angle1)),
            4 => num(a.angle2).total_cmp(&num(b.angle2)),
            _ => a.count.cmp(&b.count),
        };
        let o = if desc { o.reverse() } else { o };
        o.then(a.item.cmp(&b.item))
    });
}

fn cut_list(app: &mut App, ctx: &egui::Context) {
    if !app.st_cut_open {
        if !app.st_cut_hi.is_empty() {
            app.st_cut_hi.clear();
            app.scene_dirty = true;
        }
        return;
    }
    let mut rows = fenkra_solidos::cut_list(app.doc());
    let tubos_dobrados = fenkra_solidos::tubos_dobrados(app.doc());
    let estado_id = egui::Id::new("st_cut_ui");
    let mut ui_state: CutUi = ctx.data(|d| d.get_temp(estado_id)).unwrap_or_default();
    let materiais: Vec<String> = {
        let mut m: Vec<String> = rows.iter().map(|r| r.material.clone()).collect();
        m.sort();
        m.dedup();
        m
    };
    if !ui_state.material.is_empty() && !materiais.contains(&ui_state.material) {
        ui_state.material.clear();
    }
    rows.retain(|r| ui_state.material.is_empty() || r.material == ui_state.material);
    ordenar(&mut rows, ui_state.sort, ui_state.desc);
    // desenvolvimento: FENKRA_DEMO_CORTE=<n> deixa a linha n (de 0) clicada, para a captura do realce
    if ui_state.clicada.is_none() {
        if let Some(k) = std::env::var("FENKRA_DEMO_CORTE").ok().and_then(|v| v.parse::<usize>().ok()) {
            ui_state.clicada = rows.get(k).and_then(|r| r.members.first().copied());
        }
    }
    // clicar o corpo no 3D seleciona a linha que o contém
    let sel_feature = match app.sel {
        Sel::Feature(id) => Some(id),
        _ => None,
    };
    let mut open = true;
    let mut export = false;
    let mut export_ifc = false;
    let mut export_nc = false;
    let mut gerar = false;
    let mut acao_maq: Option<crate::st_maquina_ui::Acao> = None;
    let mut hover_row: Option<usize> = None;
    let fechar = egui::Id::unique(("concluir_janela", "st2"));
    if ctx.data_mut(|m| m.remove_temp::<bool>(fechar)).unwrap_or(false) {
        open = false;
    }
    egui::Window::new(tr("📋 Lista de corte"))
        .id(egui::Id::unique("janela_lista_corte"))
        .open(&mut open)
        .default_width(620.)
        .show(ctx, |ui| {
            if crate::app::done_bar(ui) {
                ui.ctx().data_mut(|m| m.insert_temp(fechar, true));
            }
            if rows.is_empty() && ui_state.material.is_empty() {
                ui.label(RichText::new(tr("Nenhum membro estrutural na peça.")).weak());
                return;
            }
            if materiais.len() > 1 || !ui_state.material.is_empty() {
                ui.horizontal(|ui| {
                    ui.label(tr("Material"));
                    egui::ComboBox::from_id_salt("st_cut_mat")
                        .selected_text(if ui_state.material.is_empty() { tr("Todos").to_string() } else { ui_state.material.clone() })
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(ui_state.material.is_empty(), tr("Todos")).clicked() {
                                ui_state.material.clear();
                            }
                            for m in &materiais {
                                if ui.selectable_label(ui_state.material == *m, m).clicked() {
                                    ui_state.material = m.clone();
                                }
                            }
                        });
                });
            }
            let stock = app.open.doc.props.stock_length.trim().to_string();
            egui::Grid::new("lista_corte").striped(true).num_columns(8).show(ui, |ui| {
                let cab = ["Perfil", "Material", "Comprimento", "Ângulo 1", "Ângulo 2", "Qtd."];
                for (i, h) in cab.iter().enumerate() {
                    let seta = if ui_state.sort == i {
                        if ui_state.desc {
                            " ▼"
                        } else {
                            " ▲"
                        }
                    } else {
                        ""
                    };
                    let r = ui.add(egui::Label::new(RichText::new(format!("{}{}", tr(h), seta)).strong()).sense(egui::Sense::click()));
                    let r = crate::teclado::anel(ui, r).on_hover_text(tr("Clique para ordenar"));
                    if r.clicked() {
                        if ui_state.sort == i {
                            ui_state.desc = !ui_state.desc;
                        } else {
                            ui_state.sort = i;
                            ui_state.desc = false;
                        }
                    }
                }
                ui.label(RichText::new(tr("Barras")).strong());
                ui.label(RichText::new(tr("Dobras")).strong());
                ui.end_row();
                for (k, r) in rows.iter().enumerate() {
                    let primeiro = r.members.first().copied();
                    let marcada = crate::st_corte_liga::marcada(r, ui_state.clicada, sel_feature);
                    let resp = ui.selectable_label(marcada, &r.designation);
                    if resp.clicked() {
                        ui_state.clicada = primeiro;
                    }
                    // B296: o corpo clicado no 3D leva a lista até a linha dele (uma vez por mudança de seleção)
                    if sel_feature.is_some() && sel_feature != ui_state.vista_sel && marcada {
                        resp.scroll_to_me(Some(egui::Align::Center));
                    }
                    if resp.hovered() {
                        hover_row = Some(k);
                    }
                    ui.label(&r.material);
                    let comp = ui.label(format!("{} mm", fmt(r.length)));
                    if r.tubo.is_some() {
                        comp.on_hover_text(tr("tubo dobrado: o comprimento é o desenvolvido"));
                    } else if (r.long - r.length).abs() > 0.05 && r.item > 0 {
                        comp.on_hover_text(format!("{}: {} mm", tr("ponta longa"), fmt(r.long)));
                    }
                    for (a, e) in [(r.angle1, r.encaixe1), (r.angle2, r.encaixe2)] {
                        let l = ui.label(ponta_txt(a, e));
                        if e {
                            l.on_hover_text(tr("Ponta recortada no contorno do outro membro (encaixe)"));
                        }
                    }
                    ui.label(format!("{}", r.count));
                    if r.bars > 1 {
                        ui.label(RichText::new(format!("⚠ {} ({})", r.bars * r.count, tr("emenda"))).color(egui::Color32::from_rgb(240, 170, 80)))
                            .on_hover_text(format!("{} {} mm", tr("cada peça precisa de emenda: matéria-prima de"), stock));
                    } else {
                        ui.label(format!("{}", r.count));
                    }
                    match &r.tubo {
                        Some(t) => {
                            let modo = t.modo().map(|m| tr(fenkra_solidos::st_dobra::nome_modo(m)).to_string()).unwrap_or_default();
                            ui.label(t.resumo()).on_hover_text(format!("{modo} · {}", tr("Comprimento de corte da barra reta, já com os ajustes de cada dobra. Alongamento e afinamento não entram aqui.")));
                        }
                        None => {
                            ui.label("—");
                        }
                    }
                    ui.end_row();
                }
            });
            if !tubos_dobrados.is_empty() {
                ui.separator();
                ui.label(RichText::new(tr("Tubos dobrados")).strong());
                for (n, t) in tubos_dobrados.iter().enumerate() {
                    let modo = t.modo().map(|m| tr(fenkra_solidos::st_dobra::nome_modo(m)).to_string()).unwrap_or_default();
                    let titulo = if t.so_corte_simples() { format!("{} {} · {} · {} mm {}", tr("Tubo"), n + 1, modo, fmt(t.soma_pernas), tr("em pernas")) } else { format!("{} {} · {} · {} {} mm", tr("Tubo"), n + 1, modo, tr("desenvolvido"), fmt(t.desenvolvido)) };
                    ui.label(RichText::new(titulo).strong()).on_hover_text(tr("Comprimento de corte da barra reta, já com os ajustes de cada dobra. Alongamento e afinamento não entram aqui."));
                    ui.label(RichText::new(format!("{} {} mm · {} {}", tr("pernas"), t.pernas.iter().map(|p| fmt(*p)).collect::<Vec<_>>().join(" + "), tr("dobras"), t.resumo())).weak().small());
                    for (k, d) in t.dobras.iter().enumerate() {
                        ui.label(RichText::new(format!("  {} {}: {} → {} mm · {}°", tr("dobra"), k + 1, fmt(d.inicio), fmt(d.fim), fmt(d.resultado.theta))).weak().small());
                    }
                    for e in t.erros.iter().chain(&t.avisos) {
                        ui.label(RichText::new(format!("⚠ {e}")).color(crate::theme::st_warn()).small());
                    }
                    if let Some(a) = crate::st_maquina_ui::bloco(ui, t, n, &mut ui_state.maq) {
                        acao_maq = Some(a);
                    }
                }
                ui.label(RichText::new(tr("Alongamento e afinamento são estimativas de um modelo simples, sem fonte confirmada: não são valores de engenharia.")).weak().small());
            }
            ui.label(RichText::new(tr("Comprimento e ângulos são lidos do sólido (somente leitura). Ângulo 1: início da linha guia; ângulo 2: fim. 0° = corte reto.")).weak().small());
            if stock.is_empty() {
                ui.label(RichText::new(tr("Defina o comprimento máximo da barra nas propriedades da peça para ver as emendas.")).weak().small());
            }
            let total: f64 = rows.iter().map(|r| r.length * r.count as f64).sum();
            ui.label(RichText::new(format!("total de barras: {} mm", fmt(total))).weak());
            ui.horizontal(|ui| {
                if ui.button(tr("💾 Exportar CSV…")).clicked() {
                    export = true;
                }
                if ui.button(tr("Exportar IFC…")).on_hover_text(tr("Estrutura de aço em IFC4: membros, placas e soldas com o corpo já cortado, o perfil paramétrico, o material e o comprimento de corte.")).clicked() {
                    export_ifc = true;
                }
                if ui.button(tr("Exportar NC1 e STEP…")).on_hover_text(tr("Para as máquinas de perfil e de tubo: um arquivo DSTV NC1 (cabeçalho, cortes inclinados e furos) e um STEP do sólido já cortado por item da lista, numa pasta.")).clicked() {
                    export_nc = true;
                }
                if ui.button(tr("Caminho de corte…")).on_hover_text(tr("Caminho da máquina de tubo rotativo para os encaixes: vista desenvolvida, tabela por ponto, CSV e animação do cabeçote.")).clicked() {
                    crate::st_corte_tubo_ui::abrir(ui.ctx());
                }
                if ui.button(tr("Gerar montagem…")).on_hover_text(tr("Abre o resumo com peças, instâncias, pasta e nomes dos arquivos; nada é gravado antes de Confirmar.")).clicked() {
                    gerar = true;
                }
            });
        });
    // realce no 3D: a linha sob o mouse, senão a clicada
    let alvo: Vec<Id> = crate::st_corte_liga::alvo_do_realce(&rows, hover_row, ui_state.clicada);
    ui_state.vista_sel = sel_feature;
    if alvo != app.st_cut_hi {
        app.st_cut_hi = alvo;
        app.scene_dirty = true;
    }
    if let Some(a) = acao_maq {
        crate::st_maquina_ui::executar(app, &tubos_dobrados, a, &mut ui_state.maq);
    }
    ctx.data_mut(|d| d.insert_temp(estado_id, ui_state));
    if gerar {
        crate::st_montagem_ui::abrir(app);
    }
    if export_ifc {
        exportar_ifc(app);
    }
    if export_nc {
        exportar_nc(app);
    }
    if export {
        if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name("lista_de_corte.csv").save_file() {
            let mut t = String::from("perfil;material;comprimento_mm;angulo1_graus;angulo2_graus;quantidade;barras_materia_prima;dobras;modo_da_dobra;corte_encaixe_mm\n");
            let num = |a: Option<f64>| a.map(fmt).unwrap_or_default();
            let ponta = |a: Option<f64>, e: bool| if e && a.is_none() { "encaixe".to_string() } else { num(a) };
            // B155: comprimento de corte do encaixe das pontas (laser ou plasma em tubo), por peça
            let encaixes = fenkra_solidos::st_encaixe_doc::encaixes_da_estrutura(app.doc(), &Default::default());
            for r in &rows {
                let encaixe = r.members.first().and_then(|m| encaixes.iter().find(|e| (e.feature, e.linha) == *m)).map(|e| fmt(e.comprimento_de_corte())).unwrap_or_default();
                let dobras = r.tubo.as_ref().map(|t| t.resumo()).unwrap_or_default();
                let modo = r.tubo.as_ref().and_then(|t| t.modo()).map(|m| fenkra_solidos::st_dobra::nome_modo(m)).unwrap_or_default();
                t += &format!("{};{};{};{};{};{};{};{};{};{}\n", r.designation, r.material, fmt(r.length), ponta(r.angle1, r.encaixe1), ponta(r.angle2, r.encaixe2), r.count, r.bars * r.count, dobras, modo, encaixe);
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

/// B176: grava, numa pasta escolhida, o NC1 e o STEP de cada item da lista de corte.
fn exportar_nc(app: &mut App) {
    let Some(pasta) = rfd::FileDialog::new().pick_folder() else { return };
    match fenkra_solidos::st_dstv::gravar_saidas(app.doc(), &pasta) {
        Ok(v) => app.toast(format!("{}: {} {}", tr("NC1 e STEP gravados"), v.len(), tr("arquivos")), false),
        Err(e) => app.toast(format!("{}: {}", tr("Erro"), tr(&e)), true),
    }
}

/// B134: grava a estrutura da peça em IFC4.
fn exportar_ifc(app: &mut App) {
    let nome = format!("{}.ifc", if app.doc().name.trim().is_empty() { "estrutura" } else { app.doc().name.trim() });
    let Some(p) = rfd::FileDialog::new().add_filter("IFC", &["ifc"]).set_file_name(nome).save_file() else { return };
    match fenkra_solidos::st_ifc::ifc_da_estrutura(app.doc()).and_then(|r| std::fs::write(&p, &r.texto).map(|_| r).map_err(|e| e.to_string())) {
        Ok(r) => app.toast(format!("{}: {} {}, {} {}, {} {}", tr("IFC exportado"), r.membros, tr("membros"), r.placas, tr("placas"), r.soldas, tr("soldas")), false),
        Err(e) => app.toast(format!("{}: {}", tr("Erro"), tr(&e)), true),
    }
}

/// B79: algum membro avisou que perdeu linhas (a interface então pede o contorno vermelho).
pub fn tem_falha_de_membro(status: &HashMap<Id, fenkra_solidos::Status>) -> bool {
    status.values().any(|s| matches!(s, fenkra_solidos::Status::Warning(m) if m.starts_with(fenkra_solidos::MEMBRO_FALHOU)))
}

/// B79: contorno vermelho da linha que não gerou sólido e o motivo ao lado, sobre o 3D.
pub fn draw_falhas(ui: &egui::Ui, rect: egui::Rect, vp: &crate::render::Viewport, falhas: &[fenkra_solidos::MemberFailure]) {
    let cor = crate::theme::st_err();
    for f in falhas {
        let pts: Vec<egui::Pos2> = f.pts.iter().filter_map(|p| vp.project(rect, glam::Vec3::from(p.map(|v| v as f32)))).collect();
        if pts.len() < 2 {
            continue;
        }
        ui.painter().add(egui::Shape::line(pts.clone(), egui::Stroke::new(5., cor)));
        let meio = pts[pts.len() / 2];
        let texto = format!("{} {}: {}", tr("linha"), f.line + 1, f.reason);
        let galley = ui.painter().layout_no_wrap(texto, egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), egui::Color32::WHITE);
        let caixa = egui::Rect::from_min_size(meio + egui::vec2(8., -8.), galley.size()).expand2(egui::vec2(4., 2.));
        ui.painter().rect_filled(caixa, 3., cor);
        ui.painter().galley(caixa.min + egui::vec2(4., 2.), galley, egui::Color32::WHITE);
    }
}

/// B79: o comando Membro nunca deixa um recurso vazio: ao concluir, um membro sem nenhuma linha
/// guia (ainda a escolher) é descartado e o documento volta ao estado de antes do comando.
pub fn membro_sem_linhas(doc: &Document, id: Id) -> bool {
    matches!(doc.feature(id).map(|f| &f.kind), Some(FeatureKind::StructMember(m)) if m.paths.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn membro_sem_linhas_e_detectado() {
        let s = |x: &str| x.to_string();
        let mut d = Document::new("t");
        let id = d.add_feature(FeatureKind::StructMember(StructMember {
            paths: vec![],
            family: s("Tubo quadrado"),
            designation: s("40x3"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("40"), s("3")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::MeiaEsquadria,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
        }));
        assert!(membro_sem_linhas(&d, id));
        if let Some(FeatureKind::StructMember(m)) = d.feature_mut(id).map(|f| &mut f.kind) {
            m.paths.push(crate::slots::curve());
        }
        assert!(!membro_sem_linhas(&d, id));
    }
}
