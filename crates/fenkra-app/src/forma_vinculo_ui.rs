// SPDX-License-Identifier: LGPL-2.1-or-later
//! Vínculos da Forma livre com a peça (B303): pedir no 3D, desenhar e listar. A conta é do cálculo em
//! `fenkra_solidos::forma_vinculo`; aqui a gaiola da tela soma o que ele resolveu, e o nó preso desliza
//! na referência durante o arraste.

use crate::app::{fmt, App};
use crate::editor::section;
use crate::forma_edit::{self as fe, Elem};
use crate::forma_ui::{forma, forma_mut, gaiola};
use crate::i18n::tr;
use crate::picking::Pick;
use crate::render::Camera;
use eframe::egui::{self, Rect, Stroke, Vec2};
use fenkra_core::subd::{Gaiola, V3};
use fenkra_core::subd_vinculo::*;
use fenkra_core::*;
use fenkra_solidos::forma_vinculo::FormaResolvida;
use glam::Vec3;

fn v3(p: V3) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// O que a bancada espera o clique no modelo para fazer.
#[derive(Clone, Debug, PartialEq)]
pub enum Pedido {
    /// prender os nós selecionados ao que for clicado
    No,
    /// cota dos nós selecionados até o plano clicado
    Cota,
    /// casar o caminho selecionado com a aresta; G1 e G2 esperam a face em seguida
    Borda { nos: Vec<u32>, continuidade: u8, aresta: Option<CurveRef> },
}

pub fn resolvida(app: &App, id: Id) -> Option<&FormaResolvida> {
    app.result.as_ref()?.formas_resolvidas.iter().find(|x| x.id == id)
}

/// A gaiola da tela: a do documento, com os nós onde o cálculo os pôs. Só muda quem o vínculo mexeu,
/// para o arraste de um nó solto não esperar o recálculo.
pub fn gaiola_vista(app: &App, id: Id) -> Option<Gaiola> {
    let mut g = gaiola(app, id)?.clone();
    let Some(r) = resolvida(app, id) else { return Some(g) };
    if r.entrada.faces != g.faces {
        return Some(g);
    }
    for n in &r.gaiola.nos {
        if r.entrada.pos(n.id) != Some(n.p) {
            if let Some(x) = g.no_mut(n.id) {
                x.p = n.p;
            }
        }
    }
    Some(g)
}

/// Fora do arraste, leva para o documento onde o cálculo pôs os nós presos e ancora quem ainda não
/// tinha coordenada. Só vale para uma conta feita sobre esta mesma gaiola: uma conta atrasada
/// devolveria o nó ao lugar de antes do último gesto.
pub fn sincronizar(app: &mut App, id: Id) {
    let Some(r) = resolvida(app, id).cloned() else { return };
    let Some(f) = forma_mut(app, id) else { return };
    if f.gaiola != r.entrada {
        return;
    }
    let presos: Vec<u32> = f.vinculos.nos.iter().map(|v| v.no).chain(f.vinculos.cotas.iter().map(|c| c.no)).collect();
    for no in presos {
        let Some(q) = r.gaiola.pos(no) else { continue };
        if f.gaiola.pos(no).is_some_and(|p| (0..3).any(|k| (p[k] - q[k]).abs() > 1e-7)) {
            if let Some(n) = f.gaiola.no_mut(no) {
                n.p = q;
            }
        }
    }
    for v in &mut f.vinculos.nos {
        if v.alvo.sem_ancora() {
            if let (Some((_, geo)), Some(q)) = (r.geos.iter().find(|x| x.0 == v.no), r.gaiola.pos(v.no)) {
                v.alvo.ancorar(geo, q);
            }
        }
    }
}

/// Depois de um passo do arraste: nó preso desliza na referência dele.
pub fn deslizar(app: &mut App, id: Id) {
    let Some(geos) = resolvida(app, id).map(|r| r.geos.clone()) else { return };
    if let Some(f) = forma_mut(app, id) {
        fe::deslizar(f, &geos);
    }
}

/// Curva do modelo a partir da aresta clicada: com chave, ela segue a aresta.
fn curva_da_aresta(app: &App, body: usize, edge: usize) -> Option<CurveRef> {
    let r = app.result.as_ref()?;
    let pts = crate::picking::edge_polyline(r, body, edge);
    if pts.len() < 2 {
        return None;
    }
    Some(match r.bodies.get(body).and_then(|b| b.edge_key(edge)) {
        Some(key) => CurveRef::BodyEdge { pts, key },
        None => CurveRef::Edge(pts),
    })
}

fn face_do_pick(app: &App, p: Pick) -> Option<SurfRef> {
    let Pick::Face { body, face, owner, point, normal } = p else { return None };
    let key = app.result.as_ref()?.bodies.get(body)?.face_key(face);
    Some(SurfRef::Face { point: point.map(|v| v as f64), normal: normal.map(|v| v as f64), owner: Some(owner), key })
}

/// Tradução do que foi clicado no alvo do nó.
fn alvo_do_pick(app: &App, p: Pick) -> Option<AlvoNo> {
    Some(match p {
        Pick::Vertex { body, pos } => AlvoNo::Ponto(app.vertex_ref(body, pos.map(|v| v as f64))),
        Pick::SkPoint { sketch, point, .. } => AlvoNo::Ponto(PointRef::SketchPoint { sketch, point }),
        Pick::CurveVtx { curve, index, .. } => AlvoNo::Ponto(PointRef::CurveVertex { curve, index }),
        Pick::Edge { body, edge, .. } => AlvoNo::Curva { curva: curva_da_aresta(app, body, edge)?, t: None },
        Pick::SkCurve { sketch, curve, .. } => AlvoNo::Curva { curva: CurveRef::SketchCurve { sketch, curve }, t: None },
        Pick::Plane(plano) => AlvoNo::Plano { plano, uv: None },
        Pick::Face { .. } => AlvoNo::Face { face: face_do_pick(app, p)? },
        Pick::Geo(id) => match app.open.doc.reference(id)?.kind.class() {
            RefClass::Point => AlvoNo::Ponto(PointRef::Ref(id)),
            RefClass::Curve => AlvoNo::Curva { curva: CurveRef::Ref(id), t: None },
            RefClass::Surface => AlvoNo::Face { face: SurfRef::Ref(id) },
            RefClass::Plane => AlvoNo::Plano { plano: PlaneRef::Datum(id), uv: None },
        },
    })
}

/// Espera o clique no modelo. Enquanto há pedido, a gaiola não pega o mouse.
pub fn atender(app: &mut App, ui: &egui::Ui, rect: Rect, id: Id) {
    let Some(pedido) = app.forma.pedido.clone() else { return };
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.forma.pedido = None;
        return;
    }
    let dica = match &pedido {
        Pedido::No => "Clique no ponto, curva, plano ou face em que os nós ficam presos. Esc desiste.",
        Pedido::Cota => "Clique no plano de onde a distância é medida. Esc desiste.",
        Pedido::Borda { aresta: None, .. } => "Clique na aresta do modelo que a borda da forma vai seguir. Esc desiste.",
        Pedido::Borda { .. } => "Clique na face vizinha, a que a forma continua. Esc desiste.",
    };
    ui.painter_at(rect).text(rect.center_top() + egui::vec2(0., 34.), egui::Align2::CENTER_TOP, tr(dica), egui::FontId::proportional(13.), ui.visuals().text_color());
    let Some(p) = app.picks.last().copied() else { return };
    app.picks.clear();
    let sel = gaiola(app, id).map_or(vec![], |g| fe::nos_de(g, &app.forma.sel));
    let mut erro = None;
    match pedido {
        Pedido::No => match alvo_do_pick(app, p) {
            Some(alvo) => {
                if let Some(f) = forma_mut(app, id) {
                    for no in sel {
                        f.vinculos.definir_no(VinculoNo { no, alvo: alvo.clone() });
                    }
                }
                app.forma.pedido = None;
            }
            None => erro = Some("esse elemento não serve de referência"),
        },
        Pedido::Cota => match p {
            Pick::Plane(plano) => {
                let medida = app.result.as_ref().and_then(|r| crate::handles::plane_of(r, &plano));
                let vista = gaiola_vista(app, id);
                if let (Some((o, n)), Some(g), Some(f)) = (medida, vista, forma_mut(app, id)) {
                    for no in sel {
                        let d = g.pos(no).map_or(0., |q| (v3(q) - o).dot(n) as f64);
                        f.vinculos.definir_cota(CotaNo { no, plano, expr: format!("{d:.3}") });
                    }
                }
                app.forma.pedido = None;
            }
            _ => erro = Some("clique num plano"),
        },
        Pedido::Borda { nos, continuidade, aresta: None } => match p {
            Pick::Edge { body, edge, .. } => match curva_da_aresta(app, body, edge) {
                Some(c) if continuidade == 0 => fechar_borda(app, id, nos, c, None, 0),
                Some(c) => app.forma.pedido = Some(Pedido::Borda { nos, continuidade, aresta: Some(c) }),
                None => erro = Some("essa aresta não tem forma para seguir"),
            },
            _ => erro = Some("clique numa aresta do modelo"),
        },
        Pedido::Borda { nos, continuidade, aresta: Some(c) } => match face_do_pick(app, p) {
            Some(s) => fechar_borda(app, id, nos, c, Some(s), continuidade),
            None => erro = Some("clique na face vizinha"),
        },
    }
    if let Some(e) = erro {
        app.toast(tr(e).to_string(), true);
    }
    app.scene_dirty = true;
}

fn fechar_borda(app: &mut App, id: Id, nos: Vec<u32>, aresta: CurveRef, face: Option<SurfRef>, continuidade: u8) {
    if let Some(f) = forma_mut(app, id) {
        f.vinculos.bordas.retain(|b| !b.nos.iter().any(|n| nos.contains(n)));
        f.vinculos.bordas.push(VinculoBorda { nos, aresta, face, continuidade });
    }
    app.forma.pedido = None;
}

/// Corrente nos nós presos e etiqueta nas bordas casadas. Com vínculo perdido, tudo em cor de erro.
pub fn desenhar(app: &App, ui: &egui::Ui, rect: Rect, cam: &Camera, id: Id, g: &Gaiola) {
    let Some(f) = forma(app, id) else { return };
    if f.vinculos.vazio() {
        return;
    }
    let erro = matches!(app.result.as_ref().and_then(|r| r.status.get(&id)), Some(fenkra_solidos::Status::Error(_)));
    let cor = if erro { crate::theme::st_err() } else { crate::theme::st_ok() };
    let p = ui.painter_at(rect);
    let tela = |n: u32| g.pos(n).and_then(|q| cam.project(rect, v3(q)));
    for no in f.vinculos.nos.iter().map(|v| v.no).chain(f.vinculos.cotas.iter().map(|c| c.no)) {
        let Some(s) = tela(no) else { continue };
        // dois elos
        p.circle_stroke(s + egui::vec2(-3., 0.), 4.5, Stroke::new(1.6, cor));
        p.circle_stroke(s + egui::vec2(3., 0.), 4.5, Stroke::new(1.6, cor));
    }
    if let Some(r) = resolvida(app, id) {
        for (no, _, d) in &r.cotas {
            let Some(s) = tela(*no) else { continue };
            p.text(s + egui::vec2(10., -8.), egui::Align2::LEFT_BOTTOM, format!("{} mm", fmt(*d)), egui::FontId::proportional(11.), ui.visuals().text_color());
        }
    }
    for (k, b) in f.vinculos.bordas.iter().enumerate() {
        for w in b.nos.windows(2) {
            if let (Some(a), Some(c)) = (tela(w[0]), tela(w[1])) {
                p.line_segment([a, c], Stroke::new(3.5, cor));
            }
        }
        let Some(s) = tela(b.nos[b.nos.len() / 2]) else { continue };
        let rel = resolvida(app, id).and_then(|r| r.bordas.get(k));
        let texto = match rel {
            Some(x) if x.obtida < x.pedida => format!("G{} ({} G{})", x.obtida, tr("pedido"), x.pedida),
            Some(x) => format!("G{} · {} mm", x.obtida, fmt(x.desvio)),
            None => format!("G{}", b.continuidade),
        };
        let v = ui.visuals();
        let gal = p.layout_no_wrap(texto, egui::FontId::proportional(11.), v.text_color());
        let caixa = Rect::from_center_size(s + egui::vec2(0., 16.), gal.size() + Vec2::new(8., 4.));
        p.rect_filled(caixa, 3., v.window_fill);
        p.rect_stroke(caixa, 3., Stroke::new(1., cor), egui::StrokeKind::Inside);
        p.galley(caixa.min + egui::vec2(4., 2.), gal, v.text_color());
    }
}

fn nome_do_alvo(doc: &Document, a: &AlvoNo) -> String {
    match a {
        AlvoNo::Ponto(PointRef::Ref(id)) => doc.name_of(*id),
        AlvoNo::Ponto(_) => tr("Ponto do modelo").to_string(),
        AlvoNo::Curva { curva: CurveRef::Ref(id), .. } => doc.name_of(*id),
        AlvoNo::Curva { .. } => tr("Curva do modelo").to_string(),
        AlvoNo::Plano { plano, .. } => crate::scene::plane_ref_name(doc, plano),
        AlvoNo::Face { face: SurfRef::Face { owner: Some(o), .. } } => format!("{} {}", tr("Face de"), doc.name_of(*o)),
        AlvoNo::Face { .. } => tr("Face do modelo").to_string(),
    }
}

/// Lista dos vínculos no painel do comando: tirar um, e a expressão de cada cota.
pub fn painel(ui: &mut egui::Ui, doc: &Document, f: &mut Forma, vars: &std::collections::HashMap<String, f64>) {
    if f.vinculos.vazio() {
        return;
    }
    section(ui, "Vínculos");
    let mut tirar = None;
    for (k, v) in f.vinculos.nos.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("{} {} → {}", tr("Nó"), v.no, nome_do_alvo(doc, &v.alvo)));
            if ui.small_button("×").on_hover_text(tr("Solta o nó, que fica onde está")).clicked() {
                tirar = Some((0, k));
            }
        });
    }
    for (k, c) in f.vinculos.cotas.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let rotulo = format!("{} {} ⟂ {}", tr("Nó"), c.no, crate::scene::plane_ref_name(doc, &c.plano));
            crate::editor::expr(ui, &rotulo, &mut c.expr, vars, "mm");
            if ui.small_button("×").on_hover_text(tr("Solta o nó, que fica onde está")).clicked() {
                tirar = Some((1, k));
            }
        });
    }
    for (k, b) in f.vinculos.bordas.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("{} · {} {}", tr("Borda casada"), b.nos.len(), tr("nós")));
            for c in 0..3u8 {
                if c > 0 && b.face.is_none() {
                    continue;
                }
                if ui.selectable_label(b.continuidade == c, format!("G{c}")).on_hover_text(tr("Continuidade com a peça nessa borda")).clicked() {
                    b.continuidade = c;
                }
            }
            if ui.small_button("×").on_hover_text(tr("Solta a borda, que fica onde está")).clicked() {
                tirar = Some((2, k));
            }
        });
    }
    match tirar {
        Some((0, k)) => {
            f.vinculos.nos.remove(k);
        }
        Some((1, k)) => {
            f.vinculos.cotas.remove(k);
        }
        Some((_, k)) => {
            f.vinculos.bordas.remove(k);
        }
        None => {}
    }
}

/// Nós selecionados que têm alguma regra.
pub fn presos_na_selecao(f: &Forma, sel: &[Elem]) -> Vec<u32> {
    fe::nos_de(&f.gaiola, sel).into_iter().filter(|n| f.vinculos.prende(*n)).collect()
}

/// Solta os nós: vínculo, cota e as bordas que passam por eles.
pub fn soltar(f: &mut Forma, nos: &[u32]) {
    for n in nos {
        f.vinculos.soltar_no(*n);
    }
    f.vinculos.bordas.retain(|b| !b.nos.iter().any(|n| nos.contains(n)));
}
