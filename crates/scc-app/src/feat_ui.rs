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
    matches!(k, FeatureKind::Shell(_) | FeatureKind::Draft(_) | FeatureKind::Thickness(_) | FeatureKind::Thread(_))
        || matches!(k, FeatureKind::Fillet(o) if matches!(o.mode, EdgeMode::FaceFace(..) | EdgeMode::FullRound { .. }))
}

/// Acrescenta (ou, na rosca, troca) a face clicada no comando.
pub fn add_face(k: &mut FeatureKind, p: [f64; 3]) {
    match k {
        FeatureKind::Shell(s) => s.faces.push(p),
        FeatureKind::Draft(d) => d.faces.push(p),
        FeatureKind::Thickness(t) => t.faces.push(p),
        FeatureKind::Thread(t) => t.face = p,
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
            ui.label(RichText::new(format!("Face {} · ({:.1}, {:.1}, {:.1})", i + 1, f[0], f[1], f[2])).small());
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
    if ui.checkbox(&mut on, tr("parede fina (thin)")).changed() {
        *thin = on.then(|| "2".into());
    }
    if let Some(t) = thin {
        expr(ui, "Espessura", t, vars, "mm");
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
        let name = doc.feature(*s).map(|f| f.name.clone()).unwrap_or_else(|| format!("⚠ perdido #{}", s.0));
        egui::ComboBox::from_id_salt(salt).selected_text(name).width(170.).show_ui(ui, |ui| {
            for f in doc.features[..before.min(doc.features.len())].iter().filter(|f| matches!(f.kind, FeatureKind::Sketch(_))) {
                ui.selectable_value(s, f.id, &f.name);
            }
        });
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
            ui.checkbox(&mut l.ruled, tr("faces retas entre perfis (regrada)"));
            ui.checkbox(&mut l.closed, tr("fechada (volta ao primeiro perfil)"));
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
            ui.checkbox(&mut s.outward, tr("parede para fora (mantém o volume interno)"));
            faces_ui(ui, &mut s.faces, picked_faces);
            ui.label(RichText::new(tr("As faces escolhidas ficam abertas; o resto vira parede com a espessura.")).weak().small());
        }
        FeatureKind::Draft(d) => {
            expr(ui, "Ângulo", &mut d.angle, vars, "°");
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Plano neutro")));
                plane_combo(ui, "dr_pl", doc, &mut d.neutral, None);
            });
            ui.checkbox(&mut d.reverse, tr("inverter a direção de extração"));
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
                    ui.horizontal(|ui| {
                        ui.radio_value(total, false, tr("espaçamento"));
                        ui.radio_value(total, true, tr("comprimento total"));
                    });
                    expr(ui, if *total { "Comprimento total" } else { "Espaçamento" }, spacing, vars, "mm");
                    let mut two = dir2.is_some();
                    if ui.checkbox(&mut two, tr("segunda direção")).changed() {
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
            ui.checkbox(&mut st.to_body, tr("até encontrar o corpo"));
            if !st.to_body {
                expr(ui, "Profundidade", &mut st.depth, vars, "mm");
            }
            ui.checkbox(&mut st.reverse, tr("para o lado da normal do plano"));
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
            ui.label(RichText::new(format!("({:.1}, {:.1}, {:.1})", t.face[0], t.face[1], t.face[2])).small());
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
            ui.checkbox(&mut c.keep_tool, tr("manter o corpo ferramenta visível"));
            ui.label(RichText::new(tr("O resultado fica no coletor deste comando; o corpo ferramenta é o estado do outro coletor neste ponto da linha do tempo.")).weak().small());
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
            ui.checkbox(&mut sp.flip, tr("manter o outro lado"));
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
                ui.checkbox(&mut sw.flip, tr("tirar o outro lado"));
            }
        }
        FeatureKind::BoundaryFill(bf) => {
            section(ui, "Preencher fronteiras");
            geo_ui::surfs_list(ui, "hb_bf", doc, me, &mut bf.tools, seed);
            section(ui, &format!("Células excluídas ({})", bf.exclude.len()));
            let mut del = None;
            for (i, p) in bf.exclude.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("ponto ({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).small());
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
            ui.checkbox(&mut m.solid, tr("fechar em sólido quando a malha for fechada"));
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
    egui::CollapsingHeader::new(tr("Ângulo de saída e parede fina")).default_open(e.thin.is_some() || e.taper.trim() != "0").show(ui, |ui| {
        expr(ui, "Ângulo de saída", &mut e.taper, vars, "°");
        if e.taper.trim() != "0" && !matches!(e.extent, Extent::Blind(_)) {
            ui.label(RichText::new(tr("o ângulo vale só com término 'cego'")).color(Color32::from_rgb(255, 170, 60)).small());
        }
        thin_ui(ui, &mut e.thin, vars);
    });
}

pub fn revolve_extras(ui: &mut egui::Ui, r: &mut Revolve, vars: &Vars) {
    let opts = [("revolucao", "Revolução", "acrescenta material", r.op != BoolOp::Cut), ("ranhura", "Ranhura", "remove material", r.op == BoolOp::Cut)];
    if let Some(i) = icon_choice(ui, &opts) {
        r.op = if i == 0 { BoolOp::Add } else { BoolOp::Cut };
    }
    let mut two = r.angle2.is_some();
    if ui.checkbox(&mut two, tr("segundo ângulo (para o outro lado)")).changed() {
        r.angle2 = two.then(|| "0".to_string());
    }
    if let Some(a2) = &mut r.angle2 {
        expr(ui, "Ângulo 2", a2, vars, "°");
        ui.label(RichText::new(tr("Primeiro e segundo ângulo somam no máximo 360°.")).weak().small());
    }
    thin_ui(ui, &mut r.thin, vars);
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
