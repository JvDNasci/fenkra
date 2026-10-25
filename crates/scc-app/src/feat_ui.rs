//! Painéis das features de forma, acabamento e transformação: nervura/rasgo, multi-seções,
//! primitivas, casca, ângulo de saída, espessura, padrões, espelho, escala, reforço e rosca,
//! além das opções extras de extrusão, revolução, filete e chanfro.

use crate::editor::{expr, icon_choice, plane_combo, section};
use crate::geo_ui::{self, Seed};
use eframe::egui::{self, Color32, RichText};
use scc_core::*;
use std::collections::HashMap;
use crate::i18n::tr;

type Vars = HashMap<String, f64>;

/// Comandos que geram forma (podem ser repetidos por padrões e espelho).
pub fn is_shape(k: &FeatureKind) -> bool {
    matches!(
        k,
        FeatureKind::Extrude(_) | FeatureKind::Revolve(_) | FeatureKind::Hole(_) | FeatureKind::Sweep(_) | FeatureKind::Loft(_) | FeatureKind::Primitive(_) | FeatureKind::Stiffener(_) | FeatureKind::Thickness(_)
    )
}

/// Comandos em que clicar numa face na viewport (com o comando aberto) acrescenta a face.
pub fn wants_faces(k: &FeatureKind) -> bool {
    matches!(k, FeatureKind::Shell(_) | FeatureKind::Draft(_) | FeatureKind::Thickness(_) | FeatureKind::Thread(_) | FeatureKind::SmCornerRelief(_) | FeatureKind::SmArcSlot(_) | FeatureKind::SmSurfFlange(_))
        || matches!(k, FeatureKind::SmFlangedHole(h) if crate::slots::is_unset(h.sketch) || !h.points.is_empty())
        || matches!(k, FeatureKind::Fillet(o) if matches!(o.mode, EdgeMode::FaceFace(..) | EdgeMode::FullRound { .. }))
}

/// Acrescenta (ou, na rosca, troca) a face clicada no comando.
pub fn add_face(k: &mut FeatureKind, p: [f64; 3]) {
    match k {
        FeatureKind::Shell(s) => s.faces.push(p),
        FeatureKind::Draft(d) => d.faces.push(p),
        FeatureKind::Thickness(t) => t.faces.push(p),
        FeatureKind::Thread(t) => t.face = p,
        FeatureKind::SmFlangedHole(h) => h.points.push(p),
        FeatureKind::SmCornerRelief(c) => c.at = p,
        FeatureKind::SmArcSlot(c) => c.center = p,
        FeatureKind::SmSurfFlange(c) => c.face = p,
        // face-face: as duas últimas faces clicadas; completo: meio, lado 1, lado 2 (na ordem do clique)
        FeatureKind::Fillet(o) => match &mut o.mode {
            EdgeMode::FaceFace(a, b) => {
                *a = *b;
                *b = p;
            }
            EdgeMode::FullRound { mid, a, b } => {
                *mid = *a;
                *a = *b;
                *b = p;
            }
            _ => {}
        },
        _ => {}
    }
}

/// Alterna a face (para listas de faces): se já está escolhida (mesma face do corpo), tira.
pub fn toggle_face(k: &mut FeatureKind, p: [f64; 3], same: impl Fn([f64; 3], [f64; 3]) -> bool) {
    let list = match k {
        FeatureKind::Shell(s) => Some(&mut s.faces),
        FeatureKind::Draft(d) => Some(&mut d.faces),
        FeatureKind::Thickness(t) => Some(&mut t.faces),
        _ => None,
    };
    match list {
        Some(l) => match l.iter().position(|q| same(*q, p)) {
            Some(i) => {
                l.remove(i);
            }
            None => l.push(p),
        },
        None => add_face(k, p),
    }
}

/// Pontos das faces escolhidas (para marcar na viewport).
pub fn face_marks(k: &FeatureKind) -> Vec<[f64; 3]> {
    match k {
        FeatureKind::Shell(s) => s.faces.clone(),
        FeatureKind::Draft(d) => d.faces.clone(),
        FeatureKind::Thickness(t) => t.faces.clone(),
        FeatureKind::Thread(t) => vec![t.face],
        FeatureKind::Fillet(o) => match o.mode {
            EdgeMode::FaceFace(a, b) => vec![a, b],
            EdgeMode::FullRound { mid, a, b } => vec![mid, a, b],
            _ => vec![],
        },
        _ => vec![],
    }
}

fn faces_ui(ui: &mut egui::Ui, faces: &mut Vec<[f64; 3]>, picked: &[[f64; 3]]) {
    section(ui, &format!("Faces ({})", faces.len()));
    let mut del = None;
    for (i, f) in faces.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Face {} · {}", i + 1, crate::app::p3(f, 1))).small());
            if ui.small_button(tr("✖")).clicked() {
                del = Some(i);
            }
        });
    }
    if let Some(i) = del {
        faces.remove(i);
    }
    ui.horizontal(|ui| {
        if ui.add_enabled(!picked.is_empty(), egui::Button::new(format!("+ seleção ({})", picked.len()))).clicked() {
            faces.extend_from_slice(picked);
        }
        if !faces.is_empty() && ui.button(tr("Limpar")).clicked() {
            faces.clear();
        }
    });
    ui.label(RichText::new(tr("Com o comando aberto, clique nas faces na viewport para acrescentá-las.")).weak().small());
}

fn op_choice(ui: &mut egui::Ui, op: &mut BoolOp, add: (&str, &str), cut: (&str, &str)) {
    let opts = [(add.0, add.1, "acrescenta material", *op != BoolOp::Cut), (cut.0, cut.1, "remove material", *op == BoolOp::Cut)];
    if let Some(i) = icon_choice(ui, &opts) {
        *op = if i == 0 { BoolOp::Add } else { BoolOp::Cut };
    }
}

fn thin_ui(ui: &mut egui::Ui, thin: &mut Option<String>, vars: &Vars) {
    let mut on = thin.is_some();
    if crate::editor::check(ui, &mut on, "parede fina (thin)").changed() {
        *thin = on.then(|| "2".into());
    }
    if let Some(t) = thin {
        expr(ui, "Espessura", t, vars, "mm");
    }
}

/// Parede fina com o lado (num perfil aberto: um lado, o outro ou simétrica).
fn thin_side_ui(ui: &mut egui::Ui, thin: &mut Option<String>, side: &mut u8, vars: &Vars) {
    thin_ui(ui, thin, vars);
    if thin.is_some() {
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Lado"));
            ui.radio_value(side, 0, tr("um lado"));
            ui.radio_value(side, 1, tr("outro lado"));
            ui.radio_value(side, 2, tr("simétrica"));
        });
        ui.label(RichText::new(tr("Num perfil aberto, a parede fecha o contorno com essa espessura (vale para recorte também). Perfil aberto sem parede fina usa 1 mm.")).weak().small());
    }
}

/// Lista de comandos de forma (antes deste) a repetir; vazia = corpo inteiro.
fn sources_ui(ui: &mut egui::Ui, doc: &Document, idx: usize, sources: &mut Vec<Id>) {
    section(ui, "O que repetir");
    ui.label(RichText::new(if sources.is_empty() { "Corpo inteiro (nenhum comando marcado)" } else { "Comandos marcados" }).weak().small());
    egui::ScrollArea::vertical().max_height(140.).id_salt("fontes").show(ui, |ui| {
        for f in doc.features[..idx.min(doc.features.len())].iter().filter(|f| is_shape(&f.kind)) {
            let mut on = sources.contains(&f.id);
            if ui.checkbox(&mut on, format!("{} {}", f.kind.icon(), f.name)).changed() {
                if on {
                    sources.push(f.id);
                } else {
                    sources.retain(|x| *x != f.id);
                }
            }
        }
    });
}

fn sketch_pick(ui: &mut egui::Ui, salt: &str, label: &str, doc: &Document, before: usize, s: &mut Id) {
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(label));
        let text = crate::editor::profile_text(doc, *s);
        egui::ComboBox::from_id_salt(salt).selected_text(text).width(170.).show_ui(ui, |ui| crate::editor::profile_options(ui, doc, before, s));
    });
}

/// Painel das features novas. Devolve false se `kind` não é uma delas.
#[allow(clippy::too_many_arguments)]
pub fn panel(ui: &mut egui::Ui, doc: &Document, idx: usize, me: Id, kind: &mut FeatureKind, vars: &Vars, picked_faces: &[[f64; 3]], seed: &Seed) -> bool {
    match kind {
        FeatureKind::Sweep(w) => {
            section(ui, "Tipo");
            op_choice(ui, &mut w.op, ("nervura", "Nervura"), ("rasgo_varrido", "Rasgo"));
            section(ui, "Perfil e guia");
            sketch_pick(ui, "sw_sk", "Perfil", doc, idx, &mut w.sketch);
            geo_ui::curve_slot(ui, "sw_spine", "Curva guia", doc, me, &mut w.spine, seed);
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Orientação")));
                ui.radio_value(&mut w.mode, SweepMode::Corrigido, tr("acompanha"));
                ui.radio_value(&mut w.mode, SweepMode::Fixo, tr("fixa"));
                ui.radio_value(&mut w.mode, SweepMode::Frenet, tr("Frenet"));
            });
            thin_ui(ui, &mut w.thin, vars);
            ui.label(RichText::new(tr("O perfil deve estar no início da curva guia, de preferência num plano normal a ela (Plano → Normal à curva).")).weak().small());
        }
        FeatureKind::Loft(l) => {
            section(ui, "Tipo");
            op_choice(ui, &mut l.op, ("multisecoes", "Multi-seções"), ("multisecoes_remover", "Remover"));
            section(ui, &format!("Perfis em ordem ({})", l.sections.len()));
            let mut mv: Option<(usize, isize)> = None;
            let mut del = None;
            for (i, s) in l.sections.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    sketch_pick(ui, &format!("lf{i}"), &format!("Perfil {}", i + 1), doc, idx, s);
                    if ui.small_button(tr("▲")).clicked() {
                        mv = Some((i, -1));
                    }
                    if ui.small_button(tr("▼")).clicked() {
                        mv = Some((i, 1));
                    }
                    if ui.small_button(tr("✖")).clicked() {
                        del = Some(i);
                    }
                });
            }
            if let Some((i, d)) = mv {
                let j = i as isize + d;
                if j >= 0 && (j as usize) < l.sections.len() {
                    l.sections.swap(i, j as usize);
                }
            }
            if let Some(i) = del {
                if l.sections.len() > 2 {
                    l.sections.remove(i);
                }
            }
            if ui.button(tr("+ perfil")).clicked() {
                if let Some(f) = doc.features[..idx].iter().rev().find(|f| matches!(f.kind, FeatureKind::Sketch(_)) && !l.sections.contains(&f.id)) {
                    l.sections.push(f.id);
                }
            }
            crate::editor::check(ui, &mut l.ruled, "faces retas entre perfis (regrada)");
            crate::editor::check(ui, &mut l.closed, "fechada (volta ao primeiro perfil)");
        }
        FeatureKind::Primitive(p) => {
            section(ui, "Forma");
            let opts: Vec<(&str, &str, &str, bool)> = PrimKind::ALL.iter().map(|k| (prim_icon(*k), k.label(), "", *k == p.kind)).collect();
            if let Some(i) = icon_choice(ui, &opts) {
                p.kind = PrimKind::ALL[i];
            }
            op_choice(ui, &mut p.op, ("op_adicionar", "Adicionar"), ("op_remover", "Remover"));
            section(ui, "Posição");
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Plano")));
                plane_combo(ui, "pr_pl", doc, &mut p.plane, None);
            });
            section(ui, "Medidas");
            let names = p.kind.params();
            for (e, n) in [&mut p.a, &mut p.b, &mut p.c, &mut p.d].into_iter().zip(names) {
                if !n.is_empty() {
                    expr(ui, n, e, vars, if n == "Lados" { "" } else { "mm" });
                }
            }
        }
        FeatureKind::Shell(s) => {
            expr(ui, "Espessura", &mut s.thickness, vars, "mm");
            crate::editor::check(ui, &mut s.outward, "parede para fora (mantém o volume interno)");
            faces_ui(ui, &mut s.faces, picked_faces);
            ui.label(RichText::new(tr("As faces escolhidas ficam abertas; o resto vira parede com a espessura.")).weak().small());
        }
        FeatureKind::Draft(d) => {
            expr(ui, "Ângulo", &mut d.angle, vars, "°");
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Plano neutro")));
                plane_combo(ui, "dr_pl", doc, &mut d.neutral, None);
            });
            crate::editor::check(ui, &mut d.reverse, "inverter a direção de extração");
            faces_ui(ui, &mut d.faces, picked_faces);
            ui.label(RichText::new(tr("Inclina as faces a partir do plano neutro; a extração segue a normal dele.")).weak().small());
        }
        FeatureKind::Thickness(t) => {
            expr(ui, "Espessura", &mut t.value, vars, "mm");
            ui.label(RichText::new(tr("Positivo acrescenta material, negativo remove.")).weak().small());
            faces_ui(ui, &mut t.faces, picked_faces);
        }
        FeatureKind::Pattern(p) => {
            section(ui, "Tipo");
            let cur = match p.kind {
                PatternKind::Linear { .. } => 0,
                PatternKind::Circular { .. } => 1,
                PatternKind::Points { .. } => 2,
            };
            let opts = [
                ("padrao_linear", "Linear", "Uma ou duas direções", cur == 0),
                ("padrao_circular", "Circular", "Em volta de um eixo", cur == 1),
                ("padrao_pontos", "Por pontos", "Nos pontos escolhidos", cur == 2),
            ];
            if let Some(i) = icon_choice(ui, &opts) {
                if i != cur {
                    p.kind = default_pattern(i, seed);
                }
            }
            section(ui, "Definição");
            match &mut p.kind {
                PatternKind::Linear { dir, count, spacing, dir2, count2, spacing2, total } => {
                    geo_ui::axis_slot(ui, "pt_d1", "Direção 1", doc, me, dir, seed);
                    expr(ui, "Quantidade", count, vars, "");
                    ui.horizontal_wrapped(|ui| {
                        ui.radio_value(total, false, tr("espaçamento"));
                        ui.radio_value(total, true, tr("comprimento total"));
                    });
                    expr(ui, if *total { "Comprimento total" } else { "Espaçamento" }, spacing, vars, "mm");
                    let mut two = dir2.is_some();
                    if crate::editor::check(ui, &mut two, "segunda direção").changed() {
                        *dir2 = two.then_some(AxisRef::Origin(OriginAxis::Y));
                    }
                    if let Some(d2) = dir2 {
                        geo_ui::axis_slot(ui, "pt_d2", "Direção 2", doc, me, d2, seed);
                        expr(ui, "Quantidade 2", count2, vars, "");
                        expr(ui, "Espaçamento 2", spacing2, vars, "mm");
                    }
                }
                PatternKind::Circular { axis, count, angle } => {
                    geo_ui::axis_slot(ui, "pt_ax", "Eixo", doc, me, axis, seed);
                    expr(ui, "Quantidade", count, vars, "");
                    expr(ui, "Ângulo total", angle, vars, "°");
                    ui.label(RichText::new(tr("360° distribui por igual; menos que isso põe a primeira e a última nas pontas do arco.")).weak().small());
                }
                PatternKind::Points { points, base } => {
                    geo_ui::point_slot(ui, "pt_base", "Ponto base", doc, me, base, seed);
                    geo_ui::points_list(ui, "pt_pts", doc, me, points, seed, 1);
                }
            }
            sources_ui(ui, doc, idx, &mut p.sources);
        }
        FeatureKind::Mirror(m) => {
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Plano de simetria")));
                plane_combo(ui, "mi_pl", doc, &mut m.plane, None);
            });
            sources_ui(ui, doc, idx, &mut m.sources);
        }
        FeatureKind::Scale(s) => {
            expr(ui, "Fator", &mut s.factor, vars, "×");
            geo_ui::point_slot(ui, "sc_c", "Centro", doc, me, &mut s.center, seed);
            ui.label(RichText::new(tr("Escala o corpo inteiro a partir do centro (1 = sem mudança).")).weak().small());
        }
        FeatureKind::Stiffener(st) => {
            sketch_pick(ui, "st_sk", "Linhas", doc, idx, &mut st.sketch);
            expr(ui, "Espessura", &mut st.thickness, vars, "mm");
            crate::editor::check(ui, &mut st.to_body, "até encontrar o corpo");
            if !st.to_body {
                expr(ui, "Profundidade", &mut st.depth, vars, "mm");
            }
            crate::editor::check(ui, &mut st.reverse, "para o lado da normal do plano");
            ui.label(RichText::new(tr("Cada linha do sketch vira uma parede com a espessura, extrudada a partir do plano.")).weak().small());
        }
        FeatureKind::Thread(t) => {
            section(ui, "Rosca");
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Norma e tamanho")));
                egui::ComboBox::from_id_salt("th_spec").selected_text(&t.spec).width(170.).show_ui(ui, |ui| {
                    for std in scc_solidos::hardware::STANDARDS {
                        ui.label(RichText::new(std).weak().small());
                        for s in scc_solidos::hardware::threads_of(std) {
                            ui.selectable_value(&mut t.spec, s.name.to_string(), s.name);
                        }
                    }
                });
            });
            let mut end = t.end.clone().unwrap_or(Extent::Blind(t.length.clone()));
            crate::editor::extent_ui(ui, "th_end", &mut end, vars, &["Cego", "Passante", "Até plano", "Até a superfície"], doc, seed);
            if matches!(end, Extent::ThroughAll) {
                ui.label(RichText::new(tr("rosca em toda a face cilíndrica")).weak().small());
            }
            match end {
                Extent::Blind(l) => {
                    t.length = l;
                    t.end = None;
                }
                other => t.end = Some(other),
            }
            let opts = [
                ("furo_simples", "Cosmética", "Só a indicação (desenho e BOM)", !t.modeled),
                ("furo_roscado", "Modelada", "Sulco helicoidal real (mais pesado)", t.modeled),
            ];
            if let Some(i) = icon_choice(ui, &opts) {
                t.modeled = i == 1;
            }
            section(ui, "Face cilíndrica");
            ui.label(RichText::new(format!("{}", crate::app::p3(t.face, 1))).small());
            if let Some(p) = picked_faces.last() {
                if ui.button(tr("Usar a face selecionada")).clicked() {
                    t.face = *p;
                }
            }
            ui.label(RichText::new(tr("Clique na face do eixo (rosca externa) ou do furo (interna). O diâmetro da face deve ser o da rosca.")).weak().small());
        }
        FeatureKind::Combine(c) => {
            section(ui, "Operação");
            let opts = [
                ("op_adicionar", "Unir", "Junta os dois corpos", c.op == BoolOp::Add),
                ("op_remover", "Subtrair", "Tira o corpo ferramenta deste", c.op == BoolOp::Cut),
                ("op_intersecao", "Interseção", "Fica só o volume comum", c.op == BoolOp::Intersect),
            ];
            if let Some(i) = icon_choice(ui, &opts) {
                c.op = [BoolOp::Add, BoolOp::Cut, BoolOp::Intersect][i];
            }
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Corpo ferramenta")));
                let name = doc.collectors.iter().find(|x| x.id == c.tool).map(|x| x.name.clone()).unwrap_or_else(|| "⚠ perdido".into());
                egui::ComboBox::from_id_salt("cb_tool").selected_text(name).width(170.).show_ui(ui, |ui| {
                    for col in &doc.collectors {
                        ui.selectable_value(&mut c.tool, col.id, &col.name);
                    }
                });
            });
            crate::editor::check(ui, &mut c.keep_tool, "manter o corpo ferramenta visível");
            ui.label(RichText::new(tr("O resultado fica no coletor deste comando; o corpo ferramenta é o estado do outro coletor neste ponto da linha do tempo.")).weak().small());
        }
        FeatureKind::StructMember(m) => {
            crate::st_ui::member_panel(ui, doc, me, m, vars, seed);
        }
        FeatureKind::StructPlate(p) => {
            section(ui, "Placa de extremidade");
            ui.label(RichText::new(format!("ponta perto de {}", crate::app::p3(p.at, 1))).small());
            if let Some(q) = seed.points.iter().find_map(|x| if let PointRef::Fixed(q) = x { Some(*q) } else { None }) {
                if ui.button(tr("⊙ Usar o vértice selecionado")).clicked() {
                    p.at = q;
                }
            }
            expr(ui, "Espessura", &mut p.thickness, vars, "mm");
            expr(ui, "Sobra", &mut p.margin, vars, "mm");
        }
        FeatureKind::StructGusset(g) => {
            section(ui, "Reforço");
            ui.label(RichText::new(format!("encontro perto de {}", crate::app::p3(g.at, 1))).small());
            if let Some(q) = seed.points.iter().find_map(|x| if let PointRef::Fixed(q) = x { Some(*q) } else { None }) {
                if ui.button(tr("⊙ Usar o vértice selecionado")).clicked() {
                    g.at = q;
                }
            }
            expr(ui, "Aba", &mut g.leg, vars, "mm");
            expr(ui, "Espessura", &mut g.thickness, vars, "mm");
            expr(ui, "Chanfro de alívio", &mut g.chamfer, vars, "mm");
            ui.label(RichText::new(tr("Chanfro 0 = triangular; maior que 0 corta o canto (passagem do cordão de solda).")).weak().small());
        }
        FeatureKind::SmWall(w) => {
            section(ui, "Parede base");
            let mut from_surf = w.surface.is_some();
            ui.horizontal_wrapped(|ui| {
                if ui.selectable_label(!from_surf, tr("de sketch")).clicked() {
                    from_surf = false;
                }
                if ui.selectable_label(from_surf, tr("de superfície")).clicked() {
                    from_surf = true;
                }
            });
            if from_surf != w.surface.is_some() {
                w.surface = from_surf.then(|| seed.surface.unwrap_or(SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY))));
            }
            match &mut w.surface {
                Some(sf) => crate::geo_ui::surf_slot(ui, "smw_sup", "Superfície", doc, me, sf, seed),
                None => {
                    sketch_pick(ui, "smw_sk", "Perfil", doc, idx, &mut w.sketch);
                    expr(ui, "Afastamento do plano", &mut w.offset, vars, "mm");
                }
            }
            crate::editor::check(ui, &mut w.reverse, "espessura para o outro lado");
            crate::editor::check(ui, &mut w.mid, "espessura dividida pelos dois lados");
            ui.label(RichText::new(tr("A espessura vem dos parâmetros da chapa (▭ Parâmetros).")).weak().small());
        }
        FeatureKind::SmFlange(fl) => {
            section(ui, "Aba de aresta");
            edge_line(ui, &fl.edge);
            expr(ui, "Comprimento reto", &mut fl.length, vars, "mm");
            expr(ui, "Ângulo", &mut fl.angle, vars, "°");
            let mut own = fl.radius.is_some();
            if crate::editor::check(ui, &mut own, "raio próprio (senão o da chapa)").changed() {
                fl.radius = own.then(|| "2".to_string());
            }
            if let Some(r) = &mut fl.radius {
                expr(ui, "Raio interno", r, vars, "mm");
            }
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Lado do material"));
                for (k, n) in [(0u8, "dobra por fora"), (1, "material por fora"), (2, "material por dentro")] {
                    ui.selectable_value(&mut fl.side, k, tr(n));
                }
            });
            crate::editor::check(ui, &mut fl.flip, "dobrar para o outro lado");
            section(ui, "Limites laterais");
            expr(ui, "Recuo início", &mut fl.off1, vars, "mm");
            expr(ui, "Recuo fim", &mut fl.off2, vars, "mm");
            crate::editor::check(ui, &mut fl.miter, "pontas em meia-esquadria (45°) para a aba vizinha");
            crate::editor::check(ui, &mut fl.min_gap, "recuo mínimo para aba vizinha (0,0001)").on_hover_text(tr("abas em arestas vizinhas sem recuo: planifica sem sobrepor"));
            let mut rel = fl.relief.is_some();
            if crate::editor::check(ui, &mut rel, "alívio de canto nas pontas recuadas").changed() {
                fl.relief = rel.then(|| "2".to_string());
            }
            if let Some(w) = &mut fl.relief {
                expr(ui, "Largura do alívio", w, vars, "mm");
                ui.horizontal_wrapped(|ui| {
                    ui.label(tr("Tipo"));
                    for (k, n) in [(0u8, "retangular"), (1, "redondo"), (2, "rasgo")] {
                        ui.selectable_value(&mut fl.relief_kind, k, tr(n));
                    }
                });
            }
        }
        FeatureKind::SmHem(h) => {
            section(ui, "Bainha");
            edge_line(ui, &h.edge);
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Tipo"));
                for (k, n) in [(0u8, "fechada/aberta"), (1, "gota"), (2, "enrolada")] {
                    ui.selectable_value(&mut h.kind, k, tr(n));
                }
            });
            expr(ui, "Comprimento", &mut h.length, vars, "mm");
            expr(ui, "Folga interna", &mut h.gap, vars, "mm");
            ui.label(RichText::new(tr(match h.kind {
                1 => "Gota: passa de 180° e fecha a ponta contra a chapa (raio = folga ÷ 2).",
                2 => "Enrolada: três quartos de volta (raio = folga ÷ 2).",
                _ => "Folga 0 = bainha esmagada; maior que 0 = aberta/circular.",
            })).weak().small());
            crate::editor::check(ui, &mut h.flip, "dobrar para o outro lado");
        }
        FeatureKind::SmJoggle(j) => {
            section(ui, "Joggle");
            let pts: Vec<[f64; 3]> = seed.points.iter().filter_map(|p| match p {
                PointRef::Fixed(q) => Some(*q),
                _ => None,
            }).collect();
            let mut by_plane = j.plane.is_some();
            ui.horizontal_wrapped(|ui| {
                if ui.selectable_label(!by_plane, tr("por dois pontos")).clicked() {
                    by_plane = false;
                }
                if ui.selectable_label(by_plane, tr("por plano")).clicked() {
                    by_plane = true;
                }
            });
            if by_plane != j.plane.is_some() {
                j.plane = by_plane.then(|| seed.planes.last().copied().unwrap_or(PlaneRef::Origin(OriginPlane::YZ)));
            }
            ui.label(RichText::new(format!("linha de {} a {}", crate::app::p3(j.a, 1), crate::app::p3(j.b, 1))).small());
            if ui.add_enabled(pts.len() >= 2, egui::Button::new(tr("⊙ Usar os dois vértices selecionados"))).clicked() {
                j.a = pts[0];
                j.b = pts[1];
            }
            if let Some(p) = &mut j.plane {
                crate::editor::plane_combo(ui, "jog_plano", doc, p, Some(me));
                ui.label(RichText::new(tr("A linha é a interseção do plano com a face (o primeiro ponto marca a face).")).weak().small());
            }
            expr(ui, "Desvio", &mut j.depth, vars, "mm");
            let mut by_angle = j.angle.is_some();
            if crate::editor::check(ui, &mut by_angle, "rampa pelo ângulo").changed() {
                j.angle = by_angle.then(|| "45".to_string());
            }
            match &mut j.angle {
                Some(a) => expr(ui, "Ângulo da rampa", a, vars, "°"),
                None => expr(ui, "Rampa", &mut j.run, vars, "mm"),
            }
            crate::editor::check(ui, &mut j.keep_length, "compensação: manter o comprimento desenvolvido");
            crate::editor::check(ui, &mut j.flip, "desviar o outro lado da linha");
        }
        FeatureKind::SmBend(b) => {
            section(ui, "Dobra por linha");
            let pts: Vec<[f64; 3]> = seed.points.iter().filter_map(|p| match p {
                PointRef::Fixed(q) => Some(*q),
                _ => None,
            }).collect();
            ui.label(RichText::new(format!("linha de {} a {}", crate::app::p3(b.a, 1), crate::app::p3(b.b, 1))).small());
            if ui.add_enabled(pts.len() >= 2, egui::Button::new(tr("⊙ Usar os dois vértices selecionados"))).clicked() {
                b.a = pts[0];
                b.b = pts[1];
            }
            expr(ui, "Ângulo", &mut b.angle, vars, "°");
            let mut own = b.radius.is_some();
            if crate::editor::check(ui, &mut own, "raio próprio (senão o da chapa)").changed() {
                b.radius = own.then(|| "2".to_string());
            }
            if let Some(r) = &mut b.radius {
                expr(ui, "Raio interno", r, vars, "mm");
            }
            crate::editor::check(ui, &mut b.flip, "mover o outro lado da linha");
            crate::editor::check(ui, &mut b.down, "dobrar para o outro lado");
            ui.label(RichText::new(tr("A linha é o centro da dobra; o comprimento desenvolvido vem do fator K. Na planificada a dobra é desfeita.")).weak().small());
        }
        FeatureKind::SmLouver(l) => {
            section(ui, "Veneziana");
            let pts: Vec<[f64; 3]> = seed.points.iter().filter_map(|p| match p {
                PointRef::Fixed(q) => Some(*q),
                _ => None,
            }).collect();
            ui.label(RichText::new(format!("abertura de {} a {}", crate::app::p3(l.a, 1), crate::app::p3(l.b, 1))).small());
            if ui.add_enabled(pts.len() >= 2, egui::Button::new(tr("⊙ Usar os dois vértices selecionados"))).clicked() {
                l.a = pts[0];
                l.b = pts[1];
            }
            expr(ui, "Largura da rampa", &mut l.width, vars, "mm");
            expr(ui, "Altura da abertura", &mut l.height, vars, "mm");
            crate::editor::check(ui, &mut l.flip, "rampa do outro lado da linha");
            crate::editor::check(ui, &mut l.down, "levantar para o outro lado da chapa");
        }
        FeatureKind::SmStamp(s) => {
            section(ui, "Estampo");
            egui::ComboBox::from_id_salt("sms_tipo").selected_text(tr(scc_core::SM_STAMP_KINDS[s.kind.min(4) as usize])).width(220.).show_ui(ui, |ui| {
                for (k, n) in scc_core::SM_STAMP_KINDS.iter().enumerate() {
                    ui.selectable_value(&mut s.kind, k as u8, tr(n));
                }
            });
            if s.kind == 4 {
                let cur = s.punch.map(|c| doc.name_of(c)).unwrap_or_else(|| tr("(escolha)").to_string());
                egui::ComboBox::from_id_salt("sms_puncao").selected_text(cur).show_ui(ui, |ui| {
                    for c in &doc.collectors {
                        if c.id != me {
                            ui.selectable_value(&mut s.punch, Some(c.id), &c.name);
                        }
                    }
                });
                ui.label(RichText::new(tr("O corpo do punção (em outro coletor) encostado na face: a chapa toma a forma dele, com a espessura.")).weak().small());
            } else {
                let what = match s.kind {
                    1 => "Curvas",
                    0 => "Perfil",
                    _ => "Pontos",
                };
                sketch_pick(ui, "sms_sk", what, doc, idx, &mut s.sketch);
            }
            expr(ui, if s.kind == 3 { "Aba do reforço" } else { "Profundidade" }, &mut s.depth, vars, "mm");
            if matches!(s.kind, 1..=3) {
                if s.width.trim().is_empty() {
                    s.width = "0".into();
                }
                expr(ui, if s.kind == 2 { "Diâmetro" } else { "Largura" }, &mut s.width, vars, "mm");
                ui.label(RichText::new(tr("0 = proporcional à espessura")).weak().small());
            }
            if s.kind != 3 {
                crate::editor::check(ui, &mut s.flip, "estampar para o outro lado");
            }
            ui.label(RichText::new(tr(match s.kind {
                1 => "Nervura: seção trapezoidal varrida ao longo das linhas/curvas do sketch na face.",
                2 => "Circular: tronco de cone em cada ponto do sketch.",
                3 => "Reforço: nervura triangular no canto interno da dobra mais perto de cada ponto.",
                4 => "Punção e matriz: a forma do corpo escolhido.",
                _ => "Desenhe o perfil num sketch no plano da face da chapa.",
            })).weak().small());
        }
        FeatureKind::SmFlangedHole(h) => {
            section(ui, "Furo flangeado");
            let by_points = crate::slots::is_unset(h.sketch) || !h.points.is_empty();
            if by_points {
                ui.label(format!("{} {}", h.points.len(), tr("centro(s) escolhido(s) nas faces — clique mais faces para somar")));
                if !h.points.is_empty() && ui.small_button(tr("Limpar pontos")).clicked() {
                    h.points.clear();
                }
            } else {
                sketch_pick(ui, "smf_sk", "Centros", doc, idx, &mut h.sketch);
            }
            // tabela de série + linhas cadastradas pelo usuário
            let user = user_holes();
            let cur = if h.standard.is_empty() { tr("medidas à mão").to_string() } else { h.standard.clone() };
            egui::ComboBox::from_id_salt("smf_tab").selected_text(cur).width(160.).show_ui(ui, |ui| {
                ui.selectable_value(&mut h.standard, String::new(), tr("medidas à mão"));
                for r in scc_core::FLANGED_HOLES {
                    ui.selectable_value(&mut h.standard, r.0.to_string(), format!("{} — Ø{} × {}", r.0, crate::app::fmt(r.1), crate::app::fmt(r.2)));
                }
                for (n, d, t) in &user {
                    if ui.selectable_label(false, format!("★ {n} — Ø{} × {}", crate::app::fmt(*d), crate::app::fmt(*t))).clicked() {
                        h.standard = String::new();
                        h.diameter = crate::app::fmt(*d);
                        h.height = crate::app::fmt(*t);
                    }
                }
            });
            if h.standard.is_empty() {
                expr(ui, "Diâmetro", &mut h.diameter, vars, "mm");
                expr(ui, "Altura do colar", &mut h.height, vars, "mm");
                ui.horizontal(|ui| {
                    let key = egui::Id::unique("smf_nome_novo");
                    let mut name: String = ui.ctx().data(|m| m.get_temp(key)).unwrap_or_default();
                    ui.add(egui::TextEdit::singleline(&mut name).desired_width(90.).hint_text(tr("nome")));
                    if ui.add_enabled(!name.trim().is_empty(), egui::Button::new(tr("＋ Cadastrar na tabela"))).clicked() {
                        let d = scc_core::expr::eval_str(&h.diameter, vars).unwrap_or(0.);
                        let t = scc_core::expr::eval_str(&h.height, vars).unwrap_or(0.);
                        add_user_hole(name.trim(), d, t);
                        name.clear();
                    }
                    ui.ctx().data_mut(|m| m.insert_temp(key, name));
                });
            }
            crate::editor::check(ui, &mut h.flip, "colar para o outro lado");
        }
        FeatureKind::SmCut(c) => {
            section(ui, "Recorte na chapa");
            sketch_pick(ui, "smc_sk", "Perfil", doc, idx, &mut c.sketch);
            ui.horizontal_wrapped(|ui| {
                for (k, n) in [(0u8, "normal"), (1, "com saída"), (2, "flangeado")] {
                    ui.selectable_value(&mut c.mode, k, tr(n));
                }
            });
            expr(ui, "Folga (offset)", &mut c.offset, vars, "mm");
            match c.mode {
                1 => expr(ui, "Ângulo de saída", &mut c.draft, vars, "°"),
                2 => expr(ui, "Altura do colar", &mut c.height, vars, "mm"),
                _ => {}
            }
            crate::editor::check(ui, &mut c.flip, "cortar para o outro lado");
            ui.label(RichText::new(tr("Desenhe o perfil num sketch na face da chapa. Vale no dobrado e no planificado.")).weak().small());
        }
        FeatureKind::SmProfileFlange(p) => {
            section(ui, "Aba por perfil");
            sketch_pick(ui, "smp_sk", "Perfil (aberto)", doc, idx, &mut p.sketch);
            expr(ui, "Comprimento lado 1", &mut p.l1, vars, "mm");
            expr(ui, "Comprimento lado 2", &mut p.l2, vars, "mm");
            let mut own = p.radius.is_some();
            if crate::editor::check(ui, &mut own, "raio próprio nos cantos (senão o da chapa)").changed() {
                p.radius = own.then(|| "2".to_string());
            }
            if let Some(r) = &mut p.radius {
                expr(ui, "Raio interno", r, vars, "mm");
            }
            crate::editor::check(ui, &mut p.flip, "material do outro lado da cadeia");
            ui.label(RichText::new(tr("Desenhe a cadeia aberta (linhas e arcos) num plano que corta a aresta; os cantos ganham o raio de dobra.")).weak().small());
        }
        FeatureKind::SmCurveFlange(cf) => {
            section(ui, "Aba em aresta curva");
            ui.label(RichText::new(if cf.edge.len() >= 2 { format!("{} {}", cf.edge.len(), tr("pontos na aresta")) } else { tr("⚠ clique numa aresta curva da chapa").to_string() }).small());
            let picked = seed.curves.iter().find_map(|c| match c {
                CurveRef::Edge(p) => Some(p.clone()),
                _ => None,
            });
            if ui.add_enabled(picked.is_some(), egui::Button::new(tr("⊙ Usar a aresta selecionada"))).clicked() {
                cf.edge = picked.unwrap_or_default();
            }
            expr(ui, "Comprimento", &mut cf.length, vars, "mm");
            if cf.angle.trim().is_empty() {
                cf.angle = "90".into();
            }
            expr(ui, "Ângulo", &mut cf.angle, vars, "°");
            let mut own = cf.radius.is_some();
            if crate::editor::check(ui, &mut own, "raio próprio (senão o da chapa)").changed() {
                cf.radius = own.then(|| "2".to_string());
            }
            if let Some(r) = &mut cf.radius {
                expr(ui, "Raio interno", r, vars, "mm");
            }
            crate::editor::check(ui, &mut cf.flip, "para o outro lado da chapa");
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Lado do material"));
                for (k, n, tip) in [
                    (0u8, "dobra por fora", "a dobra começa na aresta (a aba acrescenta material)"),
                    (1, "material por fora", "a face interna da aba fica alinhada à aresta"),
                    (2, "material por dentro", "a face externa da aba fica alinhada à aresta (o tamanho da peça não muda)"),
                ] {
                    ui.selectable_value(&mut cf.side, k, tr(n)).on_hover_text(tr(tip));
                }
            });
            ui.label(RichText::new(tr("A aba segue a aresta e as que continuam tangentes (reta e curva juntas), com o raio de dobra; planifica pelo comprimento desenvolvido.")).weak().small());
        }
        FeatureKind::SmSurfFlange(sf) => {
            section(ui, "Base (web)");
            ui.label(RichText::new(if sf.face == [0.; 3] { tr("⚠ clique na face da web (a parede base)").to_string() } else { format!("{} {}", tr("face da web em"), crate::app::p3(sf.face, 1)) }).small());
            crate::editor::check(ui, &mut sf.keep_base, "não relimitar a web pelo suporte");
            let mut own = sf.radius.is_some();
            if crate::editor::check(ui, &mut own, "raio de dobra próprio (senão o da chapa)").changed() {
                sf.radius = own.then(|| "2".to_string());
            }
            if let Some(r) = &mut sf.radius {
                expr(ui, "Raio de dobra", r, vars, "mm");
            }
            section(ui, "Suporte");
            crate::geo_ui::surf_slot(ui, "smsf_sup", "Superfície", doc, me, &mut sf.support, seed);
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Tipo"));
                for (k, n, tip) in [
                    (0u8, "Exato", "a aba se apoia no suporte escolhido; a linha de dobra (OML) é a interseção do suporte com o plano da web"),
                    (1, "Aproximação", "o suporte é trocado pelo plano tangente no meio da OML: linha de dobra reta, aba plana (bom para suportes curvos)"),
                    (2, "Ângulo", "a OML vem do suporte, mas a aba sobe com um ângulo constante dado a partir da web"),
                ] {
                    ui.selectable_value(&mut sf.mode, k, tr(n)).on_hover_text(tr(tip));
                }
            });
            if sf.mode == 2 {
                if sf.angle.trim().is_empty() {
                    sf.angle = "90".into();
                }
                expr(ui, "Ângulo", &mut sf.angle, vars, "°");
            }
            let help = match sf.mode {
                1 => "Aproximação: suporte trocado pelo plano tangente no meio da OML (dobra reta).",
                2 => "Ângulo: linha de dobra no suporte, aba com o ângulo dado a partir da web.",
                _ => "Exato: a aba se apoia no suporte escolhido. A linha de dobra (OML) é a interseção do suporte com o plano da web.",
            };
            ui.label(RichText::new(tr(help)).weak().small());
            section(ui, "EOP e direção");
            expr(ui, "Comprimento (EOP)", &mut sf.length, vars, "mm");
            crate::editor::check(ui, &mut sf.flip, "inverter a direção (para o outro lado da web)");
        }
        FeatureKind::SmArcSlot(sl) => {
            section(ui, "Rasgo circular");
            ui.label(RichText::new(format!("{} {} — {}", tr("centro em"), crate::app::p3(sl.center, 1), tr("clique na face para trocar"))).small());
            expr(ui, "Raio do arco", &mut sl.radius, vars, "mm");
            expr(ui, "Largura", &mut sl.width, vars, "mm");
            expr(ui, "Ângulo inicial", &mut sl.start, vars, "°");
            expr(ui, "Varredura", &mut sl.sweep, vars, "°");
            ui.label(RichText::new(tr("Rasgo em arco com pontas redondas, atravessando a chapa; vale dobrado e planificado.")).weak().small());
        }
        FeatureKind::SmCornerRelief(cr) => {
            section(ui, "Alívio de canto");
            ui.label(RichText::new(format!("{} {} — {}", tr("canto em"), crate::app::p3(cr.at, 1), tr("clique outro vértice/face para trocar"))).small());
            ui.horizontal_wrapped(|ui| {
                for (k, n) in [(0u8, "redondo"), (1, "quadrado"), (2, "gota")] {
                    ui.selectable_value(&mut cr.kind, k, tr(n));
                }
            });
            expr(ui, "Tamanho", &mut cr.size, vars, "mm");
        }
        FeatureKind::MoveBody(m) => {
            section(ui, "Mover o corpo");
            let cur = match m.xf {
                BodyXf::Translate { .. } => 0,
                BodyXf::Rotate { .. } => 1,
                BodyXf::Symmetry { .. } => 2,
                BodyXf::Affinity { .. } => 3,
            };
            let opts = [
                ("xf_mover", "Translação", "Move o corpo ao longo de uma direção", cur == 0),
                ("xf_girar", "Rotação", "Gira o corpo em torno de um eixo", cur == 1),
                ("xf_simetria", "Simetria", "Espelha o corpo (o original deixa a posição antiga)", cur == 2),
                ("xf_afinidade", "Afinidade", "Escala diferente em X, Y e Z", cur == 3),
            ];
            if let Some(i) = icon_choice(ui, &opts) {
                if i != cur {
                    m.xf = default_move(i);
                }
            }
            match &mut m.xf {
                BodyXf::Translate { dir, dist } => {
                    geo_ui::axis_slot(ui, "mv_d", "Direção", doc, me, dir, seed);
                    expr(ui, "Distância", dist, vars, "mm");
                }
                BodyXf::Rotate { axis, angle } => {
                    geo_ui::axis_slot(ui, "mv_a", "Eixo", doc, me, axis, seed);
                    expr(ui, "Ângulo", angle, vars, "°");
                }
                BodyXf::Symmetry { plane } => {
                    plane_combo(ui, "mv_pl", doc, plane, None);
                    ui.label(RichText::new(tr("Para manter o original e criar a cópia, use Espelho.")).weak().small());
                }
                BodyXf::Affinity { plane, fx, fy, fz } => {
                    plane_combo(ui, "mv_af", doc, plane, None);
                    expr(ui, "Fator X", fx, vars, "×");
                    expr(ui, "Fator Y", fy, vars, "×");
                    expr(ui, "Fator Z (normal)", fz, vars, "×");
                    ui.label(RichText::new(tr("Eixos do plano escolhido: X do plano, Y e a normal. As faces viram superfícies livres.")).weak().small());
                }
            }
        }
        FeatureKind::SplitBody(sp) => {
            section(ui, "Dividir o corpo");
            geo_ui::surf_slot(ui, "hb_s", "Superfície", doc, me, &mut sp.surface, seed);
            crate::editor::check(ui, &mut sp.flip, "manter o outro lado");
            ui.label(RichText::new(tr("Face do corpo é estendida para cortar a peça inteira; superfícies de referência cortam como estão.")).weak().small());
        }
        FeatureKind::ThickSurface(t) => {
            section(ui, "Engrossar superfície");
            geo_ui::surf_slot(ui, "hb_s", "Superfície", doc, me, &mut t.surface, seed);
            expr(ui, "Espessura 1", &mut t.t1, vars, "mm");
            expr(ui, "Espessura 2", &mut t.t2, vars, "mm");
            ui.label(RichText::new(tr("1 = lado da normal; 2 = lado oposto (0 = só um lado).")).weak().small());
            hyb_op(ui, &mut t.op);
        }
        FeatureKind::CloseSurface(c) => {
            section(ui, "Fechar superfície");
            geo_ui::surf_slot(ui, "hb_s", "Superfície", doc, me, &mut c.surface, seed);
            expr(ui, "Tolerância", &mut c.tol, vars, "mm");
            ui.label(RichText::new(tr("Costura (Stitch) as faces e tampa as bordas fechadas; o casco vira sólido.")).weak().small());
            hyb_op(ui, &mut c.op);
        }
        FeatureKind::SewSurface(sw) => {
            section(ui, "Costurar no corpo");
            geo_ui::surf_slot(ui, "hb_s", "Superfície", doc, me, &mut sw.surface, seed);
            let opts = [
                ("op_unir", "Acrescentar", "O volume entre a superfície e o corpo entra no corpo", !sw.remove),
                ("op_remover", "Remover", "Corta o corpo pela superfície e tira o lado escolhido", sw.remove),
            ];
            if let Some(i) = icon_choice(ui, &opts) {
                sw.remove = i == 1;
            }
            if sw.remove {
                crate::editor::check(ui, &mut sw.flip, "tirar o outro lado");
            }
        }
        FeatureKind::BoundaryFill(bf) => {
            section(ui, "Preencher fronteiras");
            geo_ui::surfs_list(ui, "hb_bf", doc, me, &mut bf.tools, seed);
            section(ui, &format!("Células excluídas ({})", bf.exclude.len()));
            let mut del = None;
            for (i, p) in bf.exclude.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("ponto {}", crate::app::p3(p, 1))).small());
                    if ui.small_button(tr("✖")).clicked() {
                        del = Some(i);
                    }
                });
            }
            if let Some(i) = del {
                bf.exclude.remove(i);
            }
            let pick = seed.points.iter().find_map(|p| match p {
                PointRef::Fixed(q) => Some(*q),
                _ => None,
            });
            if ui.add_enabled(pick.is_some(), egui::Button::new(tr("+ excluir célula no ponto selecionado"))).clicked() {
                bf.exclude.push(pick.unwrap());
            }
            ui.label(RichText::new(tr("Todas as células fechadas entram; para tirar uma, selecione um vértice/ponto dentro dela.")).weak().small());
            hyb_op(ui, &mut bf.op);
        }
        FeatureKind::MeshImport(m) => {
            section(ui, "Malha → B-rep");
            ui.label(RichText::new(&m.path).small());
            if ui.button(tr("📂 Trocar arquivo…")).clicked() {
                if let Some(p) = rfd::FileDialog::new().add_filter("Malha", &["stl", "obj"]).pick_file() {
                    m.path = p.to_string_lossy().into();
                }
            }
            crate::editor::check(ui, &mut m.solid, "fechar em sólido quando a malha for fechada");
            ui.label(RichText::new(tr("Cada triângulo vira uma face plana; malhas grandes ficam pesadas (máx. 200 mil triângulos).")).weak().small());
        }
        _ => return false,
    }
    true
}

/// Operação do sólido gerado pela superfície.
fn hyb_op(ui: &mut egui::Ui, op: &mut BoolOp) {
    let opts = [
        ("op_unir", "Acrescentar", "Une ao corpo (ou cria o corpo)", *op == BoolOp::Add),
        ("op_remover", "Remover", "Tira do corpo", *op == BoolOp::Cut),
        ("op_intersecao", "Interseção", "Fica só a parte comum", *op == BoolOp::Intersect),
    ];
    if let Some(i) = icon_choice(ui, &opts) {
        *op = [BoolOp::Add, BoolOp::Cut, BoolOp::Intersect][i];
    }
}

fn prim_icon(k: PrimKind) -> &'static str {
    match k {
        PrimKind::Caixa => "prim_caixa",
        PrimKind::Cilindro => "prim_cilindro",
        PrimKind::Esfera => "prim_esfera",
        PrimKind::Cone => "prim_cone",
        PrimKind::Toro => "prim_toro",
        PrimKind::Cunha => "prim_cunha",
        PrimKind::Prisma => "prim_prisma",
    }
}

pub fn default_pattern(i: usize, seed: &Seed) -> PatternKind {
    let axis = seed.curves.first().map(geo_ui::axis_of);
    let s = |x: &str| x.to_string();
    match i {
        0 => PatternKind::Linear { dir: axis.unwrap_or(AxisRef::Origin(OriginAxis::X)), count: s("3"), spacing: s("20"), dir2: None, count2: s("2"), spacing2: s("20"), total: false },
        1 => PatternKind::Circular { axis: axis.unwrap_or(AxisRef::Origin(OriginAxis::Z)), count: s("6"), angle: s("360") },
        _ => PatternKind::Points { points: seed.points.clone(), base: PointRef::Origin },
    }
}

/// Opções extras da extrusão: ângulo de saída e parede fina.
pub fn extrude_extras(ui: &mut egui::Ui, e: &mut Extrude, vars: &Vars) {
    egui::CollapsingHeader::new(tr("Ângulo de saída e parede fina")).default_open(true).show(ui, |ui| {
        expr(ui, "Ângulo de saída", &mut e.taper, vars, "°");
        if e.taper.trim() != "0" && !matches!(e.extent, Extent::Blind(_)) {
            ui.label(RichText::new(tr("o ângulo vale só com término 'cego'")).color(Color32::from_rgb(255, 170, 60)).small());
        }
        thin_side_ui(ui, &mut e.thin, &mut e.thin_side, vars);
    });
}

pub fn revolve_extras(ui: &mut egui::Ui, r: &mut Revolve, vars: &Vars) {
    let opts = [("revolucao", "Revolução", "acrescenta material", r.op != BoolOp::Cut), ("ranhura", "Ranhura", "remove material", r.op == BoolOp::Cut)];
    if let Some(i) = icon_choice(ui, &opts) {
        r.op = if i == 0 { BoolOp::Add } else { BoolOp::Cut };
    }
    let mut two = r.angle2.is_some();
    if crate::editor::check(ui, &mut two, "segundo ângulo (para o outro lado)").changed() {
        r.angle2 = two.then(|| "0".to_string());
    }
    if let Some(a2) = &mut r.angle2 {
        expr(ui, "Ângulo 2", a2, vars, "°");
        ui.label(RichText::new(tr("Primeiro e segundo ângulo somam no máximo 360°.")).weak().small());
    }
    thin_side_ui(ui, &mut r.thin, &mut r.thin_side, vars);
}

/// Variações de filete (constante/variável) e chanfro (distância, duas distâncias, distância+ângulo).
pub fn edge_mode_ui(ui: &mut egui::Ui, o: &mut EdgeOp, vars: &Vars, fillet: bool, picked_faces: &[[f64; 3]]) {
    let cur = match o.mode {
        EdgeMode::Constant => 0,
        EdgeMode::Variable(_) | EdgeMode::TwoDist(_) => 1,
        EdgeMode::DistAngle(_) | EdgeMode::FaceFace(..) => 2,
        EdgeMode::FullRound { .. } => 3,
    };
    let opts: Vec<(&str, &str, &str, bool)> = if fillet {
        vec![
            ("filete", "Constante", "Mesmo raio ao longo da aresta", cur == 0),
            ("filete_variavel", "Variável", "Raio muda do início ao fim da aresta", cur == 1),
            ("filete_face_face", "Face-face", "Entre duas faces (arredonda a aresta comum)", cur == 2),
            ("filete_completo", "Completo", "Tritangente: consome a face do meio entre duas laterais", cur == 3),
        ]
    } else {
        vec![
            ("chanfro_dist", "Distância", "Mesma distância nas duas faces", cur == 0),
            ("chanfro_dist_dist", "2 distâncias", "Uma distância em cada face", cur == 1),
            ("chanfro_dist_angulo", "Dist. + ângulo", "Distância numa face e ângulo", cur == 2),
        ]
    };
    if let Some(i) = icon_choice(ui, &opts) {
        let f = |k: usize| picked_faces.get(k).copied().unwrap_or([0.; 3]);
        o.mode = match (fillet, i) {
            (_, 0) => EdgeMode::Constant,
            (true, 1) => EdgeMode::Variable("3".into()),
            (true, 2) => EdgeMode::FaceFace(f(0), f(1)),
            (true, _) => EdgeMode::FullRound { mid: f(0), a: f(1), b: f(2) },
            (false, 1) => EdgeMode::TwoDist("2".into()),
            _ => EdgeMode::DistAngle("45".into()),
        };
        if matches!(o.mode, EdgeMode::FaceFace(..) | EdgeMode::FullRound { .. }) {
            o.edges = EdgeSel::AllOfBody;
        }
    }
    match &mut o.mode {
        EdgeMode::Constant => {}
        EdgeMode::Variable(v) => expr(ui, "Raio final", v, vars, "mm"),
        EdgeMode::TwoDist(v) => expr(ui, "Distância 2", v, vars, "mm"),
        EdgeMode::DistAngle(v) => expr(ui, "Ângulo", v, vars, "°"),
        EdgeMode::FaceFace(..) => {
            ui.label(RichText::new(tr("Clique nas duas faces na viewport (com o comando aberto).")).weak().small());
        }
        EdgeMode::FullRound { .. } => {
            ui.label(RichText::new(tr("Clique na face do meio e depois nas duas laterais. O raio sai da largura (o valor abaixo é ignorado).")).weak().small());
        }
    }
    if matches!(o.mode, EdgeMode::Variable(_) | EdgeMode::TwoDist(_) | EdgeMode::DistAngle(_)) && !matches!(o.edges, EdgeSel::Edges(_)) {
        ui.label(RichText::new(tr("esta variação usa as arestas escolhidas (não 'Todas')")).color(Color32::from_rgb(255, 170, 60)).small());
    }
}

/// Transformação padrão do corpo (0 translação, 1 rotação, 2 simetria, 3 afinidade).
pub fn default_move(i: usize) -> BodyXf {
    let s = |x: &str| x.to_string();
    match i {
        0 => BodyXf::Translate { dir: AxisRef::Origin(OriginAxis::X), dist: s("20") },
        1 => BodyXf::Rotate { axis: AxisRef::Origin(OriginAxis::Z), angle: s("90") },
        2 => BodyXf::Symmetry { plane: PlaneRef::Origin(OriginPlane::YZ) },
        _ => BodyXf::Affinity { plane: PlaneRef::Origin(OriginPlane::XY), fx: s("1"), fy: s("1"), fz: s("2") },
    }
}

/// Linha informativa da aresta escolhida (aba/bainha).
/// Tabela de furos flangeados do usuário: fica nas configurações; o painel lê e acrescenta aqui
/// e o app grava de volta (ver `sync_user_holes`).
static USER_HOLES: std::sync::Mutex<(Vec<(String, f64, f64)>, bool)> = std::sync::Mutex::new((Vec::new(), false));

fn user_holes() -> Vec<(String, f64, f64)> {
    USER_HOLES.lock().map(|g| g.0.clone()).unwrap_or_default()
}

fn add_user_hole(name: &str, d: f64, t: f64) {
    if let Ok(mut g) = USER_HOLES.lock() {
        g.0.retain(|r| r.0 != name);
        g.0.push((name.to_string(), d, t));
        g.1 = true;
    }
}

/// Copia a tabela das configurações para o painel e grava o que o painel cadastrou.
pub fn sync_user_holes(s: &mut crate::settings::Settings) {
    let Ok(mut g) = USER_HOLES.lock() else { return };
    if g.1 {
        s.flanged_holes = g.0.clone();
        g.1 = false;
        s.save();
    } else if g.0 != s.flanged_holes {
        g.0 = s.flanged_holes.clone();
    }
}

/// Planificada: linhas de dobra (centro traço-ponto, limites do arco finos) e o rótulo de cada uma.
pub fn bend_overlay(app: &crate::app::App, ui: &mut egui::Ui, rect: egui::Rect, cam: &crate::render::Camera) {
    if !app.open.doc.sheet.as_ref().is_some_and(|s| s.flat) || app.in_asm() {
        return;
    }
    let Some(r) = app.result.as_ref() else { return };
    if r.bend_lines.is_empty() {
        return;
    }
    let painter = ui.painter_at(rect);
    let v = |p: [f64; 3]| glam::Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
    let col = Color32::from_rgb(230, 120, 40);
    for b in &r.bend_lines {
        let seg = |q: [[f64; 3]; 2]| match (cam.project(rect, v(q[0])), cam.project(rect, v(q[1]))) {
            (Some(a), Some(c)) => Some([a, c]),
            _ => None,
        };
        if let Some([a, c]) = seg(b.center) {
            painter.extend(egui::Shape::dashed_line(&[a, c], egui::Stroke::new(1.6, col), 10., 4.));
            let m = a + (c - a) * 0.5;
            let g = painter.layout_no_wrap(b.label(), egui::FontId::proportional(12.), col);
            let rr = egui::Rect::from_min_size(m + egui::vec2(6., -g.size().y - 6.), g.size() + egui::vec2(8., 4.));
            painter.rect_filled(rr, 3., Color32::from_rgba_unmultiplied(20, 20, 24, 200));
            painter.galley(rr.min + egui::vec2(4., 2.), g, col);
        }
        for q in [b.start, b.end] {
            if let Some([a, c]) = seg(q) {
                painter.line_segment([a, c], egui::Stroke::new(1., col.gamma_multiply(0.6)));
            }
        }
    }
}

fn edge_line(ui: &mut egui::Ui, e: &[f64; 3]) {
    let txt = if *e == [0.; 3] { tr("⚠ clique numa aresta reta da chapa na viewport").to_string() } else { format!("aresta em {} — clique outra aresta para trocar", crate::app::p3(e, 1)) };
    ui.label(RichText::new(txt).small());
}
