//! Medição da seleção da viewport (vértices, arestas, faces) e extração: cada medida pode virar
//! parâmetro global; vértice, aresta e face podem virar ponto, curva e superfície de referência.

use crate::app::{fmt, App, Sel};
use crate::picking::Pick;
use eframe::egui::{self, RichText};
use glam::Vec3;
use scc_core::*;
use scc_solidos::geo3d::{self, Curve3};

struct Row {
    label: String,
    value: f64,
    unit: &'static str,
}

enum Act {
    Param(String, f64, Unit),
    Point([f64; 3]),
    Curve(Vec<[f64; 3]>),
    Surface([f64; 3], Option<Id>),
}

fn v3(p: &[f32]) -> Vec3 {
    Vec3::new(p[0], p[1], p[2])
}

/// Dados geométricos de uma seleção, tirados das malhas do último cálculo.
enum G {
    Vertex([f64; 3]),
    Edge(Curve3),
    /// ponto clicado, normal, área, plana?
    Face([f64; 3], [f64; 3], f64, bool, Option<Id>),
    Other,
}

fn geom(app: &App, p: &Pick) -> G {
    let Some(r) = &app.result else { return G::Other };
    match *p {
        Pick::Vertex { pos, .. } => G::Vertex(pos.map(|v| v as f64)),
        Pick::Edge { body, edge, .. } => {
            let pts = crate::picking::edge_polyline(r, body, edge);
            match Curve3::poly(pts.clone(), pts.len() > 2 && geo3d::dist(pts[0], *pts.last().unwrap()) < 1e-6) {
                Ok(c) => G::Edge(c),
                Err(_) => G::Other,
            }
        }
        Pick::Face { body, face, point, normal, owner } => {
            let Some(b) = r.bodies.get(body) else { return G::Other };
            let m = &b.mesh;
            let s = m.face_ranges.get(face).copied().unwrap_or(0) as usize;
            let e = m.face_ranges.get(face + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
            let mut area = 0.;
            let mut planar = true;
            let n0 = Vec3::from(normal);
            for t in m.indices[s..e].chunks_exact(3) {
                let q = |i: u32| v3(&m.positions[i as usize * 3..i as usize * 3 + 3]);
                let c = (q(t[1]) - q(t[0])).cross(q(t[2]) - q(t[0]));
                area += c.length() as f64 / 2.;
                if c.length() > 1e-9 && c.normalize().dot(n0).abs() < 0.9999 {
                    planar = false;
                }
            }
            G::Face(point.map(|v| v as f64), normal.map(|v| v as f64), area, planar, Some(owner))
        }
        _ => G::Other,
    }
}

fn rows(app: &App) -> (Vec<Row>, Vec<(String, Act)>) {
    let gs: Vec<G> = app.picks.iter().map(|p| geom(app, p)).collect();
    let mut r = vec![];
    let mut ex = vec![];
    let row = |r: &mut Vec<Row>, l: &str, v: f64, u: &'static str| r.push(Row { label: l.into(), value: v, unit: u });
    match gs.as_slice() {
        [G::Vertex(p)] => {
            row(&mut r, "X", p[0], "mm");
            row(&mut r, "Y", p[1], "mm");
            row(&mut r, "Z", p[2], "mm");
            ex.push(("• Extrair como ponto".into(), Act::Point(*p)));
        }
        [G::Edge(c)] => {
            row(&mut r, "Comprimento", c.length(), "mm");
            if let Ok((_, _, rad)) = geo3d::fit_circle(c) {
                if !c.is_straight() {
                    row(&mut r, "Raio", rad, "mm");
                    row(&mut r, "Diâmetro", 2. * rad, "mm");
                }
            }
            ex.push(("∿ Extrair como curva".into(), Act::Curve(c.pts.clone())));
        }
        [G::Face(p, n, area, planar, owner)] => {
            row(&mut r, "Área", *area, "mm²");
            if *planar {
                ui_normal(&mut r, *n);
            }
            ex.push(("◫ Extrair como superfície".into(), Act::Surface(*p, *owner)));
        }
        [G::Vertex(a), G::Vertex(b)] => {
            row(&mut r, "Distância", geo3d::dist(*a, *b), "mm");
            row(&mut r, "ΔX", (b[0] - a[0]).abs(), "mm");
            row(&mut r, "ΔY", (b[1] - a[1]).abs(), "mm");
            row(&mut r, "ΔZ", (b[2] - a[2]).abs(), "mm");
        }
        [G::Vertex(p), G::Face(q, n, _, true, _)] | [G::Face(q, n, _, true, _), G::Vertex(p)] => {
            row(&mut r, "Distância ao plano da face", geo3d::dot(geo3d::sub(*p, *q), *n).abs(), "mm");
        }
        [G::Vertex(p), G::Edge(c)] | [G::Edge(c), G::Vertex(p)] => {
            row(&mut r, "Distância à aresta", geo3d::dist(c.nearest(*p).1, *p), "mm");
        }
        [G::Face(qa, na, _, true, _), G::Face(qb, nb, _, true, _)] => {
            let cos = geo3d::dot(*na, *nb).clamp(-1., 1.);
            let ang = cos.acos().to_degrees();
            row(&mut r, "Ângulo entre faces", ang.min(180. - ang), "°");
            if cos.abs() > 0.9999 {
                row(&mut r, "Distância entre faces", geo3d::dot(geo3d::sub(*qa, *qb), *na).abs(), "mm");
            }
        }
        [G::Edge(a), G::Edge(b)] => {
            if a.is_straight() && b.is_straight() {
                let da = geo3d::unit(geo3d::sub(a.end(), a.start())).unwrap_or([1., 0., 0.]);
                let db = geo3d::unit(geo3d::sub(b.end(), b.start())).unwrap_or([1., 0., 0.]);
                let ang = geo3d::dot(da, db).clamp(-1., 1.).acos().to_degrees();
                row(&mut r, "Ângulo entre arestas", ang.min(180. - ang), "°");
            }
            let mut best = f64::MAX;
            for q in &a.pts {
                best = best.min(geo3d::dist(b.nearest(*q).1, *q));
            }
            row(&mut r, "Distância mínima", best, "mm");
        }
        _ => {}
    }
    (r, ex)
}

fn ui_normal(r: &mut Vec<Row>, n: [f64; 3]) {
    for (l, v) in [("Normal X", n[0]), ("Normal Y", n[1]), ("Normal Z", n[2])] {
        r.push(Row { label: l.into(), value: v, unit: "" });
    }
}

/// Painel de medição (aparece com seleção na viewport e nenhum comando aberto).
pub fn panel(app: &mut App, ui: &mut egui::Ui) {
    if app.picks.is_empty() || app.session.is_some() {
        return;
    }
    let (rs, ex) = rows(app);
    ui.add_space(8.);
    ui.separator();
    ui.label(RichText::new("📏 Medição da seleção").strong());
    if rs.is_empty() {
        ui.label(RichText::new("Selecione 1 ou 2 itens: vértice, aresta, face (Shift soma).").weak().small());
    }
    let mut act: Option<Act> = None;
    egui::Grid::new("medidas").num_columns(3).striped(true).show(ui, |ui| {
        for r in &rs {
            ui.label(&r.label);
            ui.label(RichText::new(format!("{} {}", fmt(r.value), r.unit)).strong());
            if ui.small_button("→ ƒ").on_hover_text("criar parâmetro global com este valor").clicked() {
                let unit = if r.unit == "°" { Unit::Angulo } else if r.unit == "mm" { Unit::Comprimento } else { Unit::Numero };
                act = Some(Act::Param(r.label.clone(), r.value, unit));
            }
            ui.end_row();
        }
    });
    for (l, a) in ex {
        if ui.button(l).clicked() {
            act = Some(a);
        }
    }
    let Some(a) = act else { return };
    let d = &mut app.open.doc;
    let new = match a {
        Act::Param(label, v, unit) => {
            let base: String = label.chars().filter(|c| c.is_alphanumeric()).collect();
            let n = d.unique_name(if base.is_empty() { "Medida" } else { &base });
            let _ = d.add_param(&n, &fmt(v), unit);
            app.toast(format!("Parâmetro {n} = {}", fmt(v)), false);
            Sel::Params
        }
        Act::Point(p) => Sel::Ref(d.add_geometry(RefKind::PointCoords { x: fmt(p[0]), y: fmt(p[1]), z: fmt(p[2]), frame: None })),
        Act::Curve(pts) => Sel::Ref(d.add_geometry(RefKind::CurveCopy { curve: CurveRef::Edge(pts) })),
        Act::Surface(p, owner) => Sel::Ref(d.add_geometry(RefKind::SurfExtract { face: p, owner })),
    };
    app.sel = new;
}
