//! Painel à direita: edição no lugar do objeto selecionado (RF-UI-002, RF-NUC-020).
//! Toda alteração modifica o objeto existente; o recompute é disparado automaticamente.

use crate::app::{fmt, App, Sel, MATERIALS};
use crate::scene::{plane_name, plane_ref_name};
use eframe::egui::{self, Color32, RichText};
use scc_core::*;
use scc_solidos::hardware;
use scc_solidos::Status;
use std::collections::HashMap;

type Vars = HashMap<String, f64>;

/// Campo de expressão: mostra o valor avaliado ou o erro.
pub(crate) fn expr(ui: &mut egui::Ui, label: &str, s: &mut String, vars: &Vars, unit: &str) {
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(label).truncate());
        let r = scc_core::expr::eval_str(s, vars);
        let bad = r.is_err();
        let mut te = egui::TextEdit::singleline(s).desired_width(110.);
        if bad {
            te = te.text_color(Color32::from_rgb(255, 120, 100));
        }
        let resp = ui.add(te);
        match r {
            Ok(v) if s.trim().parse::<f64>().is_err() => {
                ui.label(RichText::new(format!("= {} {unit}", fmt(v))).weak());
            }
            Ok(_) => {
                ui.label(RichText::new(unit).weak());
            }
            Err(e) => {
                resp.on_hover_text(e.to_string());
                ui.label(RichText::new("⚠").color(Color32::from_rgb(255, 120, 100)));
            }
        }
    });
}

pub(crate) fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.);
    ui.label(RichText::new(title).strong());
}

pub(crate) fn plane_combo(ui: &mut egui::Ui, id: &str, doc: &Document, p: &mut PlaneRef, exclude: Option<Id>) {
    // sem o próprio elemento e sem quem depende dele (evita ciclos)
    let deps = exclude.map(|x| doc.dependents(x)).unwrap_or_default();
    egui::ComboBox::from_id_salt(id).selected_text(plane_ref_name(doc, p)).width(150.).show_ui(ui, |ui| {
        for o in [OriginPlane::XY, OriginPlane::XZ, OriginPlane::YZ] {
            ui.selectable_value(p, PlaneRef::Origin(o), format!("Origem · {}", plane_name(o)));
        }
        for r in doc.references.iter().filter(|r| Some(r.id) != exclude && r.kind.class() == RefClass::Plane && !deps.contains(&r.id)) {
            ui.selectable_value(p, PlaneRef::Datum(r.id), &r.name);
        }
    });
}

fn sketch_combo(ui: &mut egui::Ui, id: &str, doc: &Document, before: usize, s: &mut Id) {
    let name = doc.feature(*s).map(|f| f.name.clone()).unwrap_or_else(|| format!("⚠ perdido #{}", s.0));
    egui::ComboBox::from_id_salt(id).selected_text(name).width(200.).show_ui(ui, |ui| {
        for f in doc.features[..before].iter().filter(|f| matches!(f.kind, FeatureKind::Sketch(_))) {
            ui.selectable_value(s, f.id, &f.name);
        }
    });
}

/// Linha de opções com ícone + nome (uma variação = um ícone). Devolve o índice clicado.
pub fn icon_choice(ui: &mut egui::Ui, opts: &[(&str, &str, &str, bool)]) -> Option<usize> {
    let mut hit = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.;
        for (i, (icon, name, tip, sel)) in opts.iter().enumerate() {
            if crate::icons::tool_button(ui, icon, name, 26., true, *sel, tip, "").clicked() {
                hit = Some(i);
            }
        }
    });
    hit
}

fn op_combo(ui: &mut egui::Ui, _id: &str, op: &mut BoolOp) {
    ui.label(RichText::new("Operação").weak());
    let o = *op;
    let opts = [
        ("op_adicionar", "Adicionar", "Une ao corpo", o == BoolOp::Add),
        ("op_remover", "Remover", "Subtrai do corpo", o == BoolOp::Cut),
        ("op_intersecao", "Interseção", "Mantém só a parte comum", o == BoolOp::Intersect),
    ];
    if let Some(i) = icon_choice(ui, &opts) {
        *op = [BoolOp::Add, BoolOp::Cut, BoolOp::Intersect][i];
    }
}

fn extent_ui(ui: &mut egui::Ui, _id: &str, e: &mut Extent, vars: &Vars, allow: &[&str], doc: &Document) {
    let cur = match e {
        Extent::Blind(_) => "Cego",
        Extent::Symmetric(_) => "Plano médio",
        Extent::TwoSides(..) => "Dois sentidos",
        Extent::ThroughAll => "Passante",
        Extent::UpTo(_) => "Até plano",
    };
    let keep = match e {
        Extent::Blind(d) | Extent::Symmetric(d) | Extent::TwoSides(d, _) => d.clone(),
        _ => "20".into(),
    };
    ui.label(RichText::new("Término").weak());
    let info = |n: &str| match n {
        "Cego" => ("ext_cego", "Distância fixa a partir do plano do sketch"),
        "Plano médio" => ("ext_simetrico", "Metade para cada lado do plano do sketch"),
        "Dois sentidos" => ("ext_dois_sentidos", "Distâncias diferentes para cada lado"),
        "Passante" => ("ext_passante", "Atravessa todo o corpo"),
        _ => ("ext_ate_plano", "Termina exatamente num plano (inclusive inclinado)"),
    };
    let opts: Vec<(&str, &str, &str, bool)> = allow.iter().map(|n| (info(n).0, *n, info(n).1, cur == *n)).collect();
    if let Some(i) = icon_choice(ui, &opts) {
        let n = allow[i];
        if n != cur {
            *e = match n {
                "Cego" => Extent::Blind(keep.clone()),
                "Plano médio" => Extent::Symmetric(keep.clone()),
                "Dois sentidos" => Extent::TwoSides(keep.clone(), keep.clone()),
                "Passante" => Extent::ThroughAll,
                _ => Extent::UpTo(doc.references.first().map(|r| PlaneRef::Datum(r.id)).unwrap_or(PlaneRef::Origin(OriginPlane::XY))),
            };
        }
    }
    match e {
        Extent::Blind(d) => expr(ui, "Distância", d, vars, "mm"),
        Extent::Symmetric(d) => expr(ui, "Total", d, vars, "mm"),
        Extent::TwoSides(a, b) => {
            expr(ui, "Sentido 1", a, vars, "mm");
            expr(ui, "Sentido 2", b, vars, "mm");
        }
        Extent::ThroughAll => {
            ui.label(RichText::new("atravessa todo o corpo de destino").weak().small());
        }
        Extent::UpTo(p) => {
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new("Até o plano"));
                plane_combo(ui, "ate_plano", doc, p, None);
            });
            ui.label(RichText::new("Crie planos inclinados em Referências › Plano por ângulo.").weak().small());
        }
    }
}

fn target_combo(ui: &mut egui::Ui, doc: &Document, t: &mut Id) {
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new("Coletor"));
        let name = doc.collectors.iter().find(|c| c.id == *t).map(|c| c.name.clone()).unwrap_or("?".into());
        egui::ComboBox::from_id_salt("alvo").selected_text(name).show_ui(ui, |ui| {
            for c in &doc.collectors {
                ui.selectable_value(t, c.id, &c.name);
            }
        });
    });
}

/// Lista "onde é usado" (RF-LNK-022).
fn used_by(ui: &mut egui::Ui, doc: &Document, id: Id, sel: &mut Option<Sel>) {
    let deps = doc.dependents(id);
    if deps.is_empty() {
        return;
    }
    section(ui, &format!("Usado por ({})", deps.len()));
    for d in deps {
        if ui.link(doc.name_of(d)).clicked() {
            *sel = Some(if doc.reference(d).is_some() { Sel::Ref(d) } else { Sel::Feature(d) });
        }
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let vars = app.vars();
    let mut new_sel = None;
    let mut delete = None;
    let status = match app.sel {
        Sel::Feature(id) => app.status_of(id).cloned(),
        _ => None,
    };
    let sel = app.sel;
    let doc_ro = app.open.doc.clone();
    let frozen = app.open.frozen.clone();
    let bodies: Vec<(Id, f64, f64)> = app.result.as_ref().map(|r| r.bodies.iter().map(|b| (b.collector, b.mass.volume, b.mass.area)).collect()).unwrap_or_default();
    let picked = app.picked_edges();
    let picked_faces = app.picked_faces();
    let session = app.session.as_ref().map(|s| (s.id, s.is_new));
    let seed = app.seed(false);
    let ref_err = match sel {
        Sel::Ref(id) => match app.status_of(id) {
            Some(Status::Error(e)) => Some(e.clone()),
            _ => None,
        },
        _ => None,
    };
    let mut sess_act: Option<u8> = None; // 0 = concluir, 1 = cancelar, 2 = abrir
    let doc = &mut app.open.doc;

    if let Some(rev) = &frozen {
        egui::Frame::NONE.fill(Color32::from_rgb(70, 55, 25)).inner_margin(6.).corner_radius(4.).show(ui, |ui| {
            ui.label(RichText::new(format!("🔒 Revisão {rev} congelada — somente leitura")).color(Color32::from_rgb(255, 210, 120)));
        });
    }

    match sel {
        Sel::None => {
            ui.heading("Modelagem Sólida");
            ui.label(RichText::new("Escolha um comando na barra ou clique num plano/face na viewport.").weak());
            ui.add_space(6.);
            ui.label(RichText::new("Atalhos").strong());
            ui.label(RichText::new("Espaço: vistas · F: ajustar · N: alinhar ao plano · H: ocultar · Esc: limpar seleção").weak().small());
        }

        Sel::Part => {
            ui.heading("Peça");
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new("Nome"));
                ui.text_edit_singleline(&mut doc.name);
            });
            section(ui, "Propriedades");
            let p = &mut doc.props;
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new("PN"));
                ui.text_edit_singleline(&mut p.pn);
            });
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new("Descrição"));
                ui.text_edit_singleline(&mut p.description);
            });
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new("Material"));
                egui::ComboBox::from_id_salt("mat").selected_text(p.material.clone()).show_ui(ui, |ui| {
                    for (m, _) in MATERIALS {
                        ui.selectable_value(&mut p.material, m.to_string(), m);
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new("Tipo"));
                egui::ComboBox::from_id_salt("tipo").selected_text(p.part_type.label()).show_ui(ui, |ui| {
                    for t in PartType::ALL {
                        ui.selectable_value(&mut p.part_type, t, t.label());
                    }
                });
            });
            section(ui, "Propriedades personalizadas");
            let mut remove = None;
            for (i, (k, v)) in p.custom.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(k).desired_width(96.));
                    ui.add(egui::TextEdit::singleline(v).desired_width(140.));
                    if ui.small_button("✖").clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                p.custom.remove(i);
            }
            if ui.button("➕ Propriedade").clicked() {
                p.custom.push((format!("Campo{}", p.custom.len() + 1), String::new()));
            }
            section(ui, "Propriedades de massa");
            let dens = MATERIALS.iter().find(|(m, _)| *m == p.material).map(|(_, d)| *d).unwrap_or(7.85);
            for (cid, vol, area) in &bodies {
                let name = doc.collectors.iter().find(|c| c.id == *cid).map(|c| c.name.clone()).unwrap_or_default();
                ui.label(format!("{name}: {:.1} cm³ · {:.1} cm² · {:.3} kg", vol / 1000., area / 100., vol * dens / 1e6));
            }
            if bodies.is_empty() {
                ui.label(RichText::new("sem corpos").weak());
            }
            section(ui, "Aparência da peça");
            ui.label(RichText::new("Sobrepõe a cor dos coletores; comandos com aparência própria têm prioridade.").weak().small());
            appearance_opt(ui, "ap_peca", &mut doc.appearance);
        }

        Sel::Origin(o) => {
            ui.heading(format!("Plano {} da Origem", plane_name(o)));
            ui.label(RichText::new("Os elementos da Origem são fixos: não podem ser apagados nem movidos.").weak());
            ui.add_space(6.);
            ui.label("Use os botões ✏ Sketch ou ▱ Plano para criar a partir deste plano.");
            used_by_origin(ui, &doc_ro, o, &mut new_sel);
        }

        Sel::Ref(id) => {
            let names: Vec<String> = doc.params.iter().map(|p| p.name.clone()).chain(doc.features.iter().map(|f| f.name.clone())).collect();
            let Some(r) = doc.references.iter_mut().find(|r| r.id == id) else {
                ui.label("Referência não existe mais.");
                return;
            };
            crate::geo_ui::panel(ui, &doc_ro, r, &vars, &seed, ref_err.as_deref());
            section(ui, "Nome e exibição");
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new("Nome (variável)"));
                let mut n = r.name.clone();
                if ui.text_edit_singleline(&mut n).changed() && !n.is_empty() && !names.contains(&n) {
                    r.name = n;
                }
            });
            ui.label(RichText::new("O nome é a variável referenciável; os comandos apontam para o elemento, então renomear não quebra nada.").weak().small());
            ui.checkbox(&mut r.visible, "visível");
            ui.add_space(4.);
            ui.label(RichText::new("Trocar uma entrada redireciona a variável: tudo que usa este elemento se atualiza.").weak().small());
            used_by(ui, &doc_ro, id, &mut new_sel);
            ui.add_space(8.);
            if ui.button("🗑 Apagar…").clicked() {
                delete = Some(id);
            }
        }

        Sel::Params => params_table(ui, doc, &vars, &mut app_toast_sink()),

        Sel::Collector(id) => {
            let active = doc.active_collector == id;
            let users: Vec<String> = doc.features.iter().filter(|f| f.target == id && !matches!(f.kind, FeatureKind::Sketch(_))).map(|f| f.name.clone()).collect();
            let Some(c) = doc.collectors.iter_mut().find(|c| c.id == id) else { return };
            ui.heading("Coletor de corpos");
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new("Nome"));
                ui.text_edit_singleline(&mut c.name);
            });
            let mut a = Appearance { color: c.color, finish: c.finish };
            appearance_fields(ui, "ap_col", &mut a);
            c.color = a.color;
            c.finish = a.finish;
            ui.checkbox(&mut c.visible, "visível");
            if active {
                ui.label(RichText::new("✔ coletor ativo: novos comandos atuam nele").color(Color32::from_rgb(140, 210, 140)));
            } else if ui.button("Tornar ativo").clicked() {
                doc.active_collector = id;
            }
            section(ui, &format!("Comandos neste coletor ({})", users.len()));
            for u in users {
                ui.label(format!("• {u}"));
            }
        }

        Sel::Folder(id) => {
            let Some(f) = doc.folders.iter_mut().find(|f| f.id == id) else { return };
            ui.heading("Pasta");
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new("Nome"));
                ui.text_edit_singleline(&mut f.name);
            });
            ui.label(RichText::new("Pastas só organizam a árvore: não mudam a ordem de execução.").weak());
            let items: Vec<String> = doc.features.iter().filter(|g| g.folder == Some(id)).map(|g| g.name.clone()).collect();
            section(ui, &format!("Conteúdo ({})", items.len()));
            for i in items {
                ui.label(format!("• {i}"));
            }
            ui.add_space(8.);
            if ui.button("Remover pasta (mantém os comandos)").clicked() {
                doc.remove_folder(id);
                new_sel = Some(Sel::None);
            }
        }

        Sel::Feature(id) if session.map(|s| s.0) != Some(id) && !matches!(doc.feature(id).map(|f| &f.kind), Some(FeatureKind::Sketch(_))) => {
            // resumo limpo: o comando só abre para edição com duplo clique / Editar
            let Some(f) = doc.feature_mut(id) else { return };
            ui.heading(format!("{} {}", f.kind.icon(), f.name));
            ui.label(RichText::new(f.kind.type_name()).weak());
            match &status {
                Some(Status::Error(m)) => {
                    ui.label(RichText::new(format!("✖ {m}")).color(Color32::from_rgb(255, 120, 100)));
                }
                Some(Status::Warning(m)) => {
                    ui.label(RichText::new(format!("⚠ {m}")).color(Color32::from_rgb(240, 200, 90)));
                }
                _ => {}
            }
            ui.add_space(6.);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("✏ Editar").strong()).min_size(egui::vec2(110., 28.))).clicked() {
                    sess_act = Some(2);
                }
                let mut vis = !f.hidden;
                if ui.checkbox(&mut vis, "visível").changed() {
                    f.hidden = !vis;
                }
                ui.checkbox(&mut f.suppressed, "suprimido");
            });
            ui.label(RichText::new("Duplo clique na árvore também abre a edição.").weak().small());
            used_by(ui, &doc_ro, id, &mut new_sel);
        }

        Sel::Feature(id) => {
            let Some(idx) = doc.feature_index(id) else {
                ui.label("Comando não existe mais.");
                return;
            };
            if session.map(|s| s.0) == Some(id) {
                egui::Frame::NONE.fill(ui.visuals().faint_bg_color).inner_margin(6.).corner_radius(6.).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(RichText::new("✔ Concluir").strong()).min_size(egui::vec2(110., 28.))).on_hover_text("Enter").clicked() {
                            sess_act = Some(0);
                        }
                        if ui.add(egui::Button::new("✖ Cancelar").min_size(egui::vec2(100., 28.))).on_hover_text("Esc").clicked() {
                            sess_act = Some(1);
                        }
                    });
                });
            }
            let names: Vec<String> = doc.params.iter().map(|p| p.name.clone()).chain(doc.references.iter().map(|r| r.name.clone())).collect();
            {
                let f = &mut doc.features[idx];
                ui.heading(format!("{} {}", f.kind.icon(), f.kind.type_name()));
                ui.horizontal(|ui| {
                    ui.add_sized([96., 18.], egui::Label::new("Nome"));
                    let mut n = f.name.clone();
                    if ui.text_edit_singleline(&mut n).changed() && !n.is_empty() && !names.contains(&n) {
                        f.name = n;
                    }
                });
                match &status {
                    Some(Status::Error(m)) => {
                        ui.label(RichText::new(format!("✖ {m}")).color(Color32::from_rgb(255, 120, 100)));
                    }
                    Some(Status::Warning(m)) => {
                        ui.label(RichText::new(format!("⚠ {m}")).color(Color32::from_rgb(240, 200, 90)));
                    }
                    Some(Status::Inactive) => {
                        ui.label(RichText::new("inativo (suprimido ou depois do marcador)").weak());
                    }
                    _ => {}
                }
                ui.horizontal(|ui| {
                    ui.checkbox(&mut f.suppressed, "suprimido");
                    let mut vis = !f.hidden;
                    if ui.checkbox(&mut vis, "visível").changed() {
                        f.hidden = !vis;
                    }
                });
                if !matches!(f.kind, FeatureKind::Sketch(_)) {
                    egui::CollapsingHeader::new("🎨 Aparência das faces deste comando").id_salt("ap_cmd").show(ui, |ui| {
                        appearance_opt(ui, "ap_feat", &mut f.appearance);
                    });
                }
            }
            let mut target = doc.features[idx].target;
            let mut kind = doc.features[idx].kind.clone();
            feature_ui(ui, &doc_ro, idx, id, &mut kind, &mut target, &vars, doc, &picked, &picked_faces, &seed);
            let f = &mut doc.features[idx];
            if f.kind != kind {
                f.kind = kind;
            }
            f.target = target;

            used_by(ui, &doc_ro, id, &mut new_sel);
            if matches!(doc.features[idx].kind, FeatureKind::Sketch(_)) {
                replace_all(ui, doc, id);
            }
            ui.add_space(10.);
            if ui.button("🗑 Apagar…").clicked() {
                delete = Some(id);
            }
        }
    }

    if let Some(s) = new_sel {
        app.sel = s;
    }
    match (sess_act, sel) {
        (Some(0), _) => app.conclude_session(),
        (Some(1), _) => app.cancel_session(),
        (Some(2), Sel::Feature(id) | Sel::Ref(id)) => app.start_session(id, false, None),
        _ => {}
    }
    if let Some(id) = ui.ctx().data_mut(|m| {
        let k = egui::Id::unique("scc_editar_sketch");
        let v = m.get_temp::<Id>(k);
        m.remove::<Id>(k);
        v
    }) {
        app.begin_sketch(id);
    }
    if let Some(id) = delete {
        app.ask_delete(id);
    }
}

/// Cor + acabamento + predefinições.
fn appearance_fields(ui: &mut egui::Ui, id: &str, a: &mut Appearance) {
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new("Cor"));
        ui.color_edit_button_srgb(&mut a.color);
        egui::ComboBox::from_id_salt((id, "acab")).selected_text(a.finish.label()).show_ui(ui, |ui| {
            for f in Finish::ALL {
                ui.selectable_value(&mut a.finish, f, f.label());
            }
        });
    });
    ui.horizontal_wrapped(|ui| {
        for (name, p) in APPEARANCE_PRESETS {
            let c = Color32::from_rgb(p.color[0], p.color[1], p.color[2]);
            if ui.add(egui::Button::new(RichText::new("■").color(c)).small()).on_hover_text(name).clicked() {
                *a = p;
            }
        }
    });
}

/// Aparência opcional (com botão para aplicar/remover).
fn appearance_opt(ui: &mut egui::Ui, id: &str, a: &mut Option<Appearance>) {
    match a {
        Some(ap) => {
            appearance_fields(ui, id, ap);
            if ui.small_button("remover aparência").clicked() {
                *a = None;
            }
        }
        None => {
            if ui.button("🎨 Definir aparência").clicked() {
                *a = Some(APPEARANCE_PRESETS[0].1);
            }
        }
    }
}

/// Ninguém consome as mensagens aqui; erros de nome aparecem no próprio campo.
fn app_toast_sink() -> Vec<String> {
    vec![]
}

fn used_by_origin(ui: &mut egui::Ui, doc: &Document, o: OriginPlane, sel: &mut Option<Sel>) {
    let users: Vec<&Feature> = doc.features.iter().filter(|f| matches!(&f.kind, FeatureKind::Sketch(s) if s.plane == PlaneRef::Origin(o))).collect();
    let refs: Vec<&RefElement> = doc.references.iter().filter(|r| matches!(&r.kind, RefKind::OffsetPlane { base, .. } if *base == PlaneRef::Origin(o))).collect();
    if users.is_empty() && refs.is_empty() {
        return;
    }
    section(ui, "Usado por");
    for f in users {
        if ui.link(&f.name).clicked() {
            *sel = Some(Sel::Feature(f.id));
        }
    }
    for r in refs {
        if ui.link(&r.name).clicked() {
            *sel = Some(Sel::Ref(r.id));
        }
    }
}

/// Substituir referência em todos os usuários (RF-NUC-021).
fn replace_all(ui: &mut egui::Ui, doc: &mut Document, id: Id) {
    let users = doc.dependents(id);
    if users.is_empty() {
        return;
    }
    let others: Vec<(Id, String)> = doc.features.iter().filter(|f| f.id != id && matches!(f.kind, FeatureKind::Sketch(_))).map(|f| (f.id, f.name.clone())).collect();
    if others.is_empty() {
        return;
    }
    ui.horizontal(|ui| {
        ui.label("Substituir em todos os usuários por");
        egui::ComboBox::from_id_salt("subst_all").selected_text("…").show_ui(ui, |ui| {
            for (o, n) in others {
                if ui.selectable_label(false, n).clicked() {
                    let _ = doc.replace_reference(id, o);
                }
            }
        });
    });
}

fn params_table(ui: &mut egui::Ui, doc: &mut Document, vars: &Vars, _sink: &mut Vec<String>) {
    ui.heading("Parâmetros");
    ui.label(RichText::new("Expressões: + − × ÷ ^, sin cos tan sqrt abs min max round if(cond; a; b), pi. Ângulos em graus.").weak().small());
    ui.add_space(4.);
    let mut rename: Option<(Id, String)> = None;
    let mut remove = None;
    egui::Grid::new("params").striped(true).num_columns(5).show(ui, |ui| {
        ui.label(RichText::new("Nome").strong());
        ui.label(RichText::new("Expressão").strong());
        ui.label(RichText::new("Unid.").strong());
        ui.label(RichText::new("Valor").strong());
        ui.label("");
        ui.end_row();
        for p in &mut doc.params {
            let mut n = p.name.clone();
            let r = ui.add(egui::TextEdit::singleline(&mut n).desired_width(70.));
            if r.changed() {
                rename = Some((p.id, n));
            }
            ui.add(egui::TextEdit::singleline(&mut p.expr).desired_width(100.));
            egui::ComboBox::from_id_salt(("un", p.id.0)).width(40.).selected_text(match p.unit {
                Unit::Comprimento => "mm",
                Unit::Angulo => "°",
                Unit::Numero => "—",
            }).show_ui(ui, |ui| {
                ui.selectable_value(&mut p.unit, Unit::Comprimento, "mm");
                ui.selectable_value(&mut p.unit, Unit::Angulo, "°");
                ui.selectable_value(&mut p.unit, Unit::Numero, "—");
            });
            match vars.get(&p.name) {
                Some(v) => ui.label(fmt(*v)),
                None => ui.label(RichText::new("⚠").color(Color32::from_rgb(255, 120, 100))),
            };
            if ui.small_button("✖").clicked() {
                remove = Some(p.id);
            }
            ui.end_row();
        }
    });
    if let Some((id, n)) = rename {
        // renomeia reescrevendo todas as expressões que usam o parâmetro
        let _ = doc.rename_param(id, &n);
    }
    if let Some(id) = remove {
        let name = doc.params.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default();
        let mut used = false;
        let mut probe = doc.clone();
        probe.for_each_expr(|e| {
            if rename_ident(e, &name, "\u{1}") != *e {
                used = true
            }
        });
        if used {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique("param_used"), name));
        } else {
            doc.params.retain(|p| p.id != id);
        }
    }
    if let Some(n) = ui.ctx().data(|d| d.get_temp::<String>(egui::Id::unique("param_used"))) {
        ui.label(RichText::new(format!("'{n}' está em uso em expressões e não pode ser removido.")).color(Color32::from_rgb(240, 200, 90)));
    }
    ui.add_space(6.);
    if ui.button("➕ Parâmetro").clicked() {
        let n = (1..).map(|i| format!("P{i}")).find(|n| !doc.params.iter().any(|p| &p.name == n)).unwrap();
        let _ = doc.add_param(&n, "10", Unit::Comprimento);
    }
}

#[allow(clippy::too_many_arguments)]
fn feature_ui(
    ui: &mut egui::Ui,
    doc: &Document,
    idx: usize,
    id: Id,
    kind: &mut FeatureKind,
    target: &mut Id,
    vars: &Vars,
    live: &mut Document,
    picked: &[[f64; 3]],
    picked_faces: &[[f64; 3]],
    seed: &crate::geo_ui::Seed,
) {
    if crate::feat_ui::panel(ui, doc, idx, id, kind, vars, picked_faces, seed) {
        target_combo(ui, doc, target);
        return;
    }
    match kind {
        FeatureKind::Sketch(s) => {
            section(ui, "Plano do sketch");
            plane_combo(ui, "plano_sk", doc, &mut s.plane, None);
            let d = &mut s.data;
            ui.label(format!("{} curvas · {} pontos · {} restrições", d.curves.len(), d.points.len(), d.constraints.len()));
            if ui.add(egui::Button::new(RichText::new("✏ Editar sketch").strong()).min_size(egui::vec2(160., 28.))).clicked() {
                ui.ctx().data_mut(|m| m.insert_temp(egui::Id::unique("scc_editar_sketch"), id));
            }
            ui.label(RichText::new("Duplo clique no sketch na árvore também abre a edição.").weak().small());
            let dims: Vec<usize> = (0..d.constraints.len()).filter(|i| d.constraints[*i].kind.is_dimension()).collect();
            if !dims.is_empty() {
                section(ui, &format!("Cotas ({})", dims.len()));
                for i in dims {
                    let c = &mut d.constraints[i];
                    let label = c.kind.label();
                    if let Some(v) = c.kind.value_mut() {
                        expr(ui, label, v, vars, "");
                    }
                }
            }
            let _ = live;
        }

        FeatureKind::Extrude(e) => {
            section(ui, "Perfil");
            sketch_combo(ui, "sk_ex", doc, idx, &mut e.sketch);
            op_combo(ui, "op_ex", &mut e.op);
            section(ui, "Extensão");
            extent_ui(ui, "ext_ex", &mut e.extent, vars, &["Cego", "Plano médio", "Dois sentidos", "Passante", "Até plano"], doc);
            ui.checkbox(&mut e.reverse, "inverter sentido");
            crate::feat_ui::extrude_extras(ui, e, vars);
            if e.op == BoolOp::Cut {
                ui.label(RichText::new("O recorte entra no material (contra a normal do plano). Use 'inverter' se necessário.").weak().small());
            }
            target_combo(ui, doc, target);
        }

        FeatureKind::Revolve(r) => {
            section(ui, "Perfil");
            sketch_combo(ui, "sk_rv", doc, idx, &mut r.sketch);
            op_combo(ui, "op_rv", &mut r.op);
            section(ui, "Eixo");
            let axis_name = |a: &AxisRef| match a {
                AxisRef::Origin(OriginAxis::X) => "Origem · eixo X".to_string(),
                AxisRef::Origin(OriginAxis::Y) => "Origem · eixo Y".to_string(),
                AxisRef::Origin(OriginAxis::Z) => "Origem · eixo Z".to_string(),
                AxisRef::SketchLine { sketch, entity } => format!("{} · linha #{}", doc.name_of(*sketch), entity.0),
                AxisRef::Ref(id) => doc.name_of(*id),
                AxisRef::Fixed { .. } => "Aresta (direção fixa)".to_string(),
            };
            egui::ComboBox::from_id_salt("eixo").selected_text(axis_name(&r.axis)).width(200.).show_ui(ui, |ui| {
                for a in [OriginAxis::X, OriginAxis::Y, OriginAxis::Z] {
                    ui.selectable_value(&mut r.axis, AxisRef::Origin(a), axis_name(&AxisRef::Origin(a)));
                }
                for f in &doc.features[..idx] {
                    if let FeatureKind::Sketch(s) = &f.kind {
                        for e in s.data.curves.iter().filter(|e| matches!(e.geom, scc_core::sketch::Curve::Line { .. })) {
                            let a = AxisRef::SketchLine { sketch: f.id, entity: e.id };
                            ui.selectable_value(&mut r.axis, a, axis_name(&a));
                        }
                    }
                }
                for g in doc.references.iter().filter(|g| g.kind.class() == RefClass::Curve) {
                    ui.selectable_value(&mut r.axis, AxisRef::Ref(g.id), &g.name);
                }
            });
            expr(ui, "Ângulo", &mut r.angle, vars, "°");
            ui.checkbox(&mut r.symmetric, "simétrico ao plano do sketch");
            crate::feat_ui::revolve_extras(ui, r, vars);
            target_combo(ui, doc, target);
        }

        FeatureKind::Hole(h) => hole_ui(ui, doc, idx, h, vars, target),

        FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => {
            let fillet = matches!(kind_tag(id, doc), Some(true));
            let lbl = if fillet { "Raio" } else { "Distância" };
            crate::feat_ui::edge_mode_ui(ui, o, vars, fillet);
            expr(ui, lbl, &mut o.value, vars, "mm");
            ui.label(RichText::new("Arestas").weak());
            let all = matches!(o.edges, EdgeSel::AllOfBody | EdgeSel::CreatedBy(_));
            let opts = [
                ("arestas_selecionar", "Selecionadas", "Só as arestas escolhidas na viewport", !all),
                ("arestas_todas", "Todas", "Coleta todas as arestas do corpo", all),
            ];
            match icon_choice(ui, &opts) {
                Some(0) if all => o.edges = EdgeSel::Edges(picked.to_vec()),
                Some(1) => o.edges = EdgeSel::AllOfBody,
                _ => {}
            }
            if let EdgeSel::Edges(m) = &mut o.edges {
                ui.label(format!("{} aresta(s) escolhida(s)", m.len()));
                ui.horizontal(|ui| {
                    if ui.add_enabled(!picked.is_empty(), egui::Button::new(format!("Usar seleção atual ({})", picked.len()))).clicked() {
                        *m = picked.to_vec();
                    }
                    if !picked.is_empty() && ui.button("Somar à lista").clicked() {
                        m.extend_from_slice(picked);
                    }
                    if !m.is_empty() && ui.button("Limpar").clicked() {
                        m.clear();
                    }
                });
                ui.label(RichText::new("Clique nas arestas na viewport (Shift soma) e use 'Usar seleção atual'.").weak().small());
            }
            target_combo(ui, doc, target);
        }
        // demais comandos: painel em feat_ui
        _ => {}
    }
}


/// true = filete, false = chanfro
fn kind_tag(id: Id, doc: &Document) -> Option<bool> {
    doc.feature(id).map(|f| matches!(f.kind, FeatureKind::Fillet(_)))
}

/// Furo inteligente (RF-SOL-007/008): escolher norma e uso preenche os diâmetros.
fn hole_ui(ui: &mut egui::Ui, doc: &Document, idx: usize, h: &mut Hole, vars: &Vars, target: &mut Id) {
    section(ui, "Centros");
    sketch_combo(ui, "sk_furo", doc, idx, &mut h.sketch);
    ui.label(RichText::new("Os pontos do sketch são os centros; o furo entra contra a normal do plano.").weak().small());

    // estado atual: "norma|designação|folga" (formato antigo "M6 passagem normal" também é lido)
    let st = h.standard.clone().unwrap_or_default();
    let parts: Vec<&str> = st.split('|').collect();
    let (mut norma, mut size, mut folga) = if parts.len() == 3 {
        (parts[0].to_string(), parts[1].to_string(), parts[2].to_string())
    } else {
        let m = st.split_whitespace().next().unwrap_or("M6");
        let name = hardware::THREADS.iter().find(|t| t.name.starts_with(&format!("{m}x"))).map(|t| t.name).unwrap_or("M6x1");
        let f = if st.contains("fino") { "fino" } else if st.contains("folgado") { "folgado" } else { "normal" };
        ("ISO métrica grossa".to_string(), name.to_string(), f.to_string())
    };
    let tapped = h.thread.is_some();
    let kind_idx = match (&h.kind, tapped) {
        (_, true) => 3,
        (HoleKind::Rebaixado { .. }, _) => 1,
        (HoleKind::Escareado { .. }, _) => 2,
        _ => 0,
    };
    let before = (norma.clone(), size.clone(), folga.clone(), kind_idx);

    section(ui, "Tipo");
    let opts = [
        ("furo_simples", "Simples", "Furo de passagem simples", kind_idx == 0),
        ("furo_rebaixado", "Rebaixado", "Passagem + rebaixo para cabeça de parafuso Allen", kind_idx == 1),
        ("furo_escareado", "Escareado", "Passagem + escareado para cabeça chata", kind_idx == 2),
        ("furo_roscado", "Roscado", "Broca de rosca + rosca (cosmética ou modelada)", kind_idx == 3),
    ];
    let new_kind = icon_choice(ui, &opts).unwrap_or(kind_idx);

    section(ui, "Rosca / fixador");
    ui.horizontal(|ui| {
        ui.add_sized([60., 18.], egui::Label::new("Norma"));
        egui::ComboBox::from_id_salt("norma_rosca").selected_text(norma.clone()).width(190.).show_ui(ui, |ui| {
            for s in hardware::STANDARDS {
                if ui.selectable_label(norma == s, s).clicked() && norma != s {
                    norma = s.to_string();
                    size = hardware::threads_of(s).next().map(|t| t.name.to_string()).unwrap_or_default();
                }
            }
        });
    });
    ui.horizontal(|ui| {
        ui.add_sized([60., 18.], egui::Label::new("Tamanho"));
        egui::ComboBox::from_id_salt("tam_rosca").selected_text(size.clone()).width(190.).show_ui(ui, |ui| {
            for t in hardware::threads_of(&norma) {
                ui.selectable_value(&mut size, t.name.to_string(), t.name);
            }
        });
    });
    if new_kind != 3 {
        ui.horizontal(|ui| {
            ui.add_sized([60., 18.], egui::Label::new("Folga"));
            for f in ["fino", "normal", "folgado"] {
                ui.radio_value(&mut folga, f.to_string(), f);
            }
        });
    }

    // aplica a escolha (só quando algo mudou, para não sobrescrever ajustes finos)
    if (norma.clone(), size.clone(), folga.clone(), new_kind) != before {
        let spec = hardware::thread(&size).copied();
        let metric = spec.and_then(|t| hardware::METRIC.iter().find(|m| (t.major - m.name[1..].parse::<f64>().unwrap_or(0.)).abs() < 1e-6));
        let fi = match folga.as_str() {
            "fino" => 0,
            "folgado" => 2,
            _ => 1,
        };
        if let Some(t) = spec {
            let clearance = metric.map(|m| m.clearance[fi]).unwrap_or(t.major * [1.05, 1.1, 1.2][fi]);
            h.diameter = format!("{}", if new_kind == 3 { t.tap_drill } else { (clearance * 100.).round() / 100. });
            h.thread = (new_kind == 3).then(|| t.name.to_string());
            h.kind = match new_kind {
                1 => {
                    let (d, p) = metric.map(|m| (m.cbore_diameter, m.cbore_depth)).unwrap_or((t.major * 1.8, t.major));
                    HoleKind::Rebaixado { diameter: d.to_string(), depth: p.to_string() }
                }
                2 => HoleKind::Escareado { diameter: metric.map(|m| m.csink_diameter).unwrap_or(t.major * 2.).to_string(), angle: "90".into() },
                _ => HoleKind::Simples,
            };
        }
        h.standard = Some(format!("{norma}|{size}|{folga}"));
    }

    extent_ui(ui, "ext_furo", &mut h.depth, vars, &["Cego", "Passante"], doc);

    if h.thread.is_some() {
        section(ui, "Representação da rosca");
        let opts = [
            ("furo_simples", "Cosmética", "Leve: só a indicação da rosca (desenho e BOM)", !h.thread_modeled),
            ("furo_roscado", "Modelada", "Sulco helicoidal real recortado por booleana (mais pesado)", h.thread_modeled),
        ];
        if let Some(i) = icon_choice(ui, &opts) {
            h.thread_modeled = i == 1;
        }
    }

    egui::CollapsingHeader::new("Ajuste fino das dimensões").default_open(false).show(ui, |ui| {
        expr(ui, "Diâmetro", &mut h.diameter, vars, "mm");
        match &mut h.kind {
            HoleKind::Simples => {}
            HoleKind::Rebaixado { diameter, depth } => {
                expr(ui, "Ø rebaixo", diameter, vars, "mm");
                expr(ui, "Prof. rebaixo", depth, vars, "mm");
            }
            HoleKind::Escareado { diameter, angle } => {
                expr(ui, "Ø escareado", diameter, vars, "mm");
                expr(ui, "Ângulo", angle, vars, "°");
            }
        }
        if let Some(t) = &mut h.thread {
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new("Designação"));
                ui.text_edit_singleline(t);
            });
        }
    });
    target_combo(ui, doc, target);
}
