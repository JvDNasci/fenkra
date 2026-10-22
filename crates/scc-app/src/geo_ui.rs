//! Geometria de referência: catálogo dos tipos de planos, pontos e curvas, criação a partir da
//! seleção da viewport e painel de edição (cada entrada aceita a seleção atual ou a lista).

use crate::editor::{expr, icon_choice, plane_combo, section};
use crate::scene::plane_ref_name;
use eframe::egui::{self, Color32, RichText};
use scc_core::*;
use std::collections::HashMap;

/// Entradas tiradas da seleção atual (viewport + árvore).
#[derive(Clone, Default, Debug)]
pub struct Seed {
    pub planes: Vec<PlaneRef>,
    pub points: Vec<PointRef>,
    pub curves: Vec<CurveRef>,
    pub surface: Option<SurfRef>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GeoKind {
    PlaneOffset,
    PlaneParallelPoint,
    PlaneAngle,
    Plane3P,
    Plane2L,
    PlanePL,
    PlaneCurve,
    PlaneNormalCurve,
    PlaneTangent,
    PlaneEquation,
    PlaneMean,
    PtCoords,
    PtOnCurve,
    PtOnPlane,
    PtOnSurface,
    PtCenter,
    PtTangent,
    PtBetween,
    LinePts,
    LineDir,
    LineAngle,
    LineTangent,
    LineNormal,
    Polyline,
    Circle,
    Circle3,
    Spline,
    Helix,
    Spiral,
    Conic,
    Connect,
    Corner,
    Spine,
}

use GeoKind::*;

pub const PLANES: [GeoKind; 11] = [PlaneOffset, PlaneParallelPoint, PlaneAngle, Plane3P, Plane2L, PlanePL, PlaneCurve, PlaneNormalCurve, PlaneTangent, PlaneEquation, PlaneMean];
pub const POINTS: [GeoKind; 7] = [PtCoords, PtOnCurve, PtOnPlane, PtOnSurface, PtCenter, PtTangent, PtBetween];
pub const LINES: [GeoKind; 6] = [LinePts, LineDir, LineAngle, LineTangent, LineNormal, Polyline];
pub const CURVES: [GeoKind; 9] = [Circle, Circle3, Spline, Helix, Spiral, Conic, Connect, Corner, Spine];

impl GeoKind {
    /// (ícone, nome curto, descrição breve, descrição avançada)
    pub fn info(self) -> (&'static str, &'static str, &'static str, &'static str) {
        match self {
            PlaneOffset => ("plano_deslocado", "Deslocado", "Plano deslocado de um plano ou face", "Paralelo ao plano (ou face) selecionado, a uma distância que aceita expressão."),
            PlaneParallelPoint => ("plano_paralelo_ponto", "Paralelo/ponto", "Paralelo passando por um ponto", "Paralelo a um plano ou face, passando exatamente pelo ponto escolhido (vértice ou ponto de referência)."),
            PlaneAngle => ("plano_angulo", "Ângulo/normal", "Plano por ângulo ou normal", "Gira o plano base em torno de um eixo ou linha. 90° deixa o plano normal ao base."),
            Plane3P => ("plano_3pontos", "3 pontos", "Plano por três pontos", "Passa por três pontos não colineares (vértices ou pontos de referência)."),
            Plane2L => ("plano_2linhas", "2 linhas", "Plano por duas linhas", "Definido por duas linhas coplanares (concorrentes ou paralelas)."),
            PlanePL => ("plano_ponto_linha", "Ponto e linha", "Plano por ponto e linha", "Passa por um ponto e contém uma linha de referência ou aresta."),
            PlaneCurve => ("plano_curva", "Curva planar", "Plano de uma curva planar", "Contém uma curva aberta ou fechada que esteja num único plano (aresta, círculo, spline…)."),
            PlaneNormalCurve => ("plano_normal_curva", "Normal à curva", "Plano normal a uma curva", "Perpendicular à curva num ponto dela, por distância ou proporção do comprimento."),
            PlaneTangent => ("plano_tangente", "Tangente", "Plano tangente a uma superfície", "Tangente à face (ou plano) no ponto escolhido, projetado sobre ela."),
            PlaneEquation => ("plano_equacao", "Equação", "Plano pela equação Ax+By+Cz=D", "Coeficientes aceitam expressões e parâmetros."),
            PlaneMean => ("plano_medio", "Médio", "Plano médio por pontos", "Plano de melhor ajuste (mínimos quadrados) por três ou mais pontos."),
            PtCoords => ("ponto_coord", "Coordenadas", "Ponto por coordenadas", "X, Y, Z na Origem, ou u, v e altura num plano de referência."),
            PtOnCurve => ("ponto_curva", "Na curva", "Ponto sobre a curva", "Por distância ou proporção a partir do início (ou do fim): início, meio e fim com um clique."),
            PtOnPlane => ("ponto_plano", "No plano", "Ponto sobre o plano", "Nas coordenadas (u, v) do plano."),
            PtOnSurface => ("ponto_superficie", "Na superfície", "Ponto sobre a superfície", "Deslocado (u, v) no plano tangente a partir do ponto clicado na face."),
            PtCenter => ("ponto_centro", "Centro", "Centro de círculo/arco", "Centro de um círculo, arco ou aresta circular."),
            PtTangent => ("ponto_tangente", "Tangente", "Ponto tangente na curva", "Ponto de uma curva planar onde a tangente é paralela à direção escolhida (dois extremos)."),
            PtBetween => ("ponto_entre", "Entre", "Ponto entre dois pontos", "Por proporção: 0.5 é o ponto médio."),
            LinePts => ("linha_pontos", "Ponto-ponto", "Linha por dois pontos", "Entre dois pontos, com prolongamento opcional nas pontas."),
            LineDir => ("linha_direcao", "Ponto-direção", "Linha por ponto e direção", "Parte do ponto ao longo de um eixo, linha ou aresta, do início ao fim."),
            LineAngle => ("linha_angulo", "Ângulo/normal", "Linha em ângulo com a curva", "No ponto da curva, girada do ângulo no plano de apoio (90° = normal à curva)."),
            LineTangent => ("linha_tangente", "Tangente", "Linha tangente à curva", "Tangente à curva no ponto mais próximo do ponto escolhido."),
            LineNormal => ("linha_normal", "Normal à sup.", "Linha normal à superfície", "Perpendicular à face ou plano no ponto escolhido."),
            Polyline => ("polilinha3d", "Polilinha", "Polilinha 3D", "Segmentos em série pelos pontos, aberta ou fechada."),
            Circle => ("circulo3d", "Círculo", "Círculo/arco por centro e raio", "No plano escolhido; ângulos de início e fim fazem um arco."),
            Circle3 => ("circulo3p", "Círculo 3P", "Círculo ou arco por 3 pontos", "Passa pelos três pontos; desmarque 'completo' para o arco do 1º ao 3º."),
            Spline => ("spline", "Spline", "Spline por pontos", "Curva livre interpolada pelos pontos (aberta ou fechada)."),
            Helix => ("helice", "Hélice", "Hélice", "Cilíndrica ou cônica: raio, passo, altura, conicidade e sentido."),
            Spiral => ("espiral", "Espiral", "Espiral plana", "Do raio inicial ao final em N voltas, no plano."),
            Conic => ("conica", "Cônica", "Elipse, parábola ou hipérbole", "Cônica no plano, pelo centro e parâmetros a e b."),
            Connect => ("conexao", "Conexão", "Curva de conexão", "Liga as pontas de duas curvas com continuidade G0, G1 (tangente) ou G2 (curvatura)."),
            Corner => ("canto", "Canto", "Canto (concordância)", "Arco tangente entre duas linhas, com raio."),
            Spine => ("spine", "Spine", "Spine (curva guia)", "Curva normal a uma sequência de planos, para varreduras e multi-seções."),
        }
    }

    /// Tipos irmãos (mesma família) para a troca de tipo no painel.
    pub fn family(self) -> &'static [GeoKind] {
        if PLANES.contains(&self) {
            &PLANES
        } else if POINTS.contains(&self) {
            &POINTS
        } else if LINES.contains(&self) {
            &LINES
        } else {
            &CURVES
        }
    }

    /// Usa um plano como entrada (o plano/face selecionado vira o plano base).
    pub fn uses_plane(self) -> bool {
        matches!(self, PlaneOffset | PlaneParallelPoint | PlaneAngle | PtOnPlane | LineAngle | LineTangent | Circle | Spiral | Conic | Spine)
    }

    pub fn of(k: &RefKind) -> Option<GeoKind> {
        use RefKind as R;
        Some(match k {
            R::OffsetPlane { .. } => PlaneOffset,
            R::AnglePlane { .. } => PlaneAngle,
            R::FacePlane { .. } => return None,
            R::PlaneParallelPoint { .. } => PlaneParallelPoint,
            R::PlaneThreePoints { .. } => Plane3P,
            R::PlaneTwoLines { .. } => Plane2L,
            R::PlanePointLine { .. } => PlanePL,
            R::PlaneCurve { .. } => PlaneCurve,
            R::PlaneNormalCurve { .. } => PlaneNormalCurve,
            R::PlaneTangentSurface { .. } => PlaneTangent,
            R::PlaneEquation { .. } => PlaneEquation,
            R::PlaneMeanPoints { .. } => PlaneMean,
            R::PointCoords { .. } => PtCoords,
            R::PointOnCurve { .. } => PtOnCurve,
            R::PointOnPlane { .. } => PtOnPlane,
            R::PointOnSurface { .. } => PtOnSurface,
            R::PointCenter { .. } => PtCenter,
            R::PointTangent { .. } => PtTangent,
            R::PointBetween { .. } => PtBetween,
            R::LinePoints { .. } => LinePts,
            R::LineDir { .. } => LineDir,
            R::LineAngle { angle, .. } if angle.trim() == "0" => LineTangent,
            R::LineAngle { .. } => LineAngle,
            R::LineNormalSurface { .. } => LineNormal,
            R::Polyline { .. } => Polyline,
            R::Circle { .. } => Circle,
            R::Circle3 { .. } => Circle3,
            R::Spline { .. } => Spline,
            R::Helix { .. } => Helix,
            R::Spiral { .. } => Spiral,
            R::Conic { .. } => Conic,
            R::Connect { .. } => Connect,
            R::Corner { .. } => Corner,
            R::Spine { .. } => Spine,
        })
    }

    /// Cria o elemento com as entradas da seleção (o que faltar recebe um valor padrão editável).
    pub fn make(self, s: &Seed) -> RefKind {
        use RefKind as R;
        const DEF_PTS: [PointRef; 4] = [PointRef::Origin, PointRef::Fixed([20., 0., 0.]), PointRef::Fixed([0., 20., 0.]), PointRef::Fixed([0., 0., 20.])];
        let p = |i: usize| s.points.get(i).copied().unwrap_or(DEF_PTS[i.min(3)]);
        let c = |i: usize| s.curves.get(i).cloned().unwrap_or(CurveRef::Axis(if i == 0 { OriginAxis::X } else { OriginAxis::Y }));
        let pl = s.planes.first().copied().unwrap_or_default();
        let sf = s.surface.unwrap_or(SurfRef::Plane(pl));
        let ax = |def: OriginAxis| s.curves.first().map(axis_of).unwrap_or(AxisRef::Origin(def));
        let pts = |min: usize| -> Vec<PointRef> {
            let mut v = s.points.clone();
            while v.len() < min {
                v.push(DEF_PTS[v.len().min(3)]);
            }
            v
        };
        let t = |x: &str| x.to_string();
        match self {
            PlaneOffset => R::OffsetPlane { base: pl, offset: t("20") },
            PlaneParallelPoint => R::PlaneParallelPoint { base: pl, point: p(0) },
            PlaneAngle => R::AnglePlane { base: pl, axis: ax(OriginAxis::X), angle: t("30") },
            Plane3P => R::PlaneThreePoints { a: p(0), b: p(1), c: p(2) },
            Plane2L => R::PlaneTwoLines { a: c(0), b: c(1) },
            PlanePL => R::PlanePointLine { point: s.points.first().copied().unwrap_or(DEF_PTS[2]), line: c(0) },
            PlaneCurve => R::PlaneCurve { curve: c(0) },
            PlaneNormalCurve => R::PlaneNormalCurve { curve: c(0), at: OnCurve::Ratio(t("0.5")), reverse: false },
            PlaneTangent => R::PlaneTangentSurface { surface: sf, point: p(0) },
            PlaneEquation => R::PlaneEquation { a: t("0"), b: t("0"), c: t("1"), d: t("10") },
            PlaneMean => R::PlaneMeanPoints { points: pts(3) },
            PtCoords => R::PointCoords { x: t("0"), y: t("0"), z: t("0"), frame: s.planes.first().copied() },
            PtOnCurve => R::PointOnCurve { curve: c(0), at: OnCurve::Ratio(t("0.5")), reverse: false },
            PtOnPlane => R::PointOnPlane { plane: pl, u: t("0"), v: t("0") },
            PtOnSurface => R::PointOnSurface { surface: sf, u: t("0"), v: t("0") },
            PtCenter => R::PointCenter { curve: c(0) },
            PtTangent => R::PointTangent { curve: c(0), dir: AxisRef::Origin(OriginAxis::X), which: 0 },
            PtBetween => R::PointBetween { a: p(0), b: p(1), ratio: t("0.5") },
            LinePts => R::LinePoints { a: p(0), b: p(1), start: t("0"), end: t("0") },
            LineDir => R::LineDir { point: p(0), dir: ax(OriginAxis::Z), start: t("0"), end: t("50") },
            LineAngle => R::LineAngle { curve: c(0), point: p(0), support: pl, angle: t("90"), start: t("0"), end: t("30") },
            LineTangent => R::LineAngle { curve: c(0), point: p(0), support: pl, angle: t("0"), start: t("0"), end: t("30") },
            LineNormal => R::LineNormalSurface { surface: sf, point: p(0), start: t("0"), end: t("30") },
            Polyline => R::Polyline { points: pts(2), closed: false },
            Circle => R::Circle { center: p(0), plane: pl, radius: t("20"), start: t("0"), end: t("360") },
            Circle3 => R::Circle3 { a: p(0), b: p(1), c: p(2), full: true },
            Spline => R::Spline { points: pts(3), closed: false },
            Helix => R::Helix { origin: p(0), axis: ax(OriginAxis::Z), radius: t("10"), pitch: t("5"), height: t("50"), taper: t("0"), left: false },
            Spiral => R::Spiral { center: p(0), plane: pl, r0: t("5"), r1: t("30"), turns: t("3") },
            Conic => R::Conic { center: p(0), plane: pl, kind: ConicKind::Elipse, a: t("30"), b: t("15"), extent: t("40") },
            Connect => R::Connect { a: c(0), a_end: true, b: c(1), b_end: false, cont: Continuity::G1, tension: t("1") },
            Corner => R::Corner { a: c(0), b: c(1), radius: t("5") },
            Spine => R::Spine { planes: if s.planes.len() >= 2 { s.planes.clone() } else { vec![pl, PlaneRef::Origin(OriginPlane::YZ)] } },
        }
    }
}

/// Curva usada como eixo.
pub fn axis_of(c: &CurveRef) -> AxisRef {
    match c {
        CurveRef::Axis(a) => AxisRef::Origin(*a),
        CurveRef::Ref(id) => AxisRef::Ref(*id),
        CurveRef::SketchLine { sketch, entity } => AxisRef::SketchLine { sketch: *sketch, entity: *entity },
        CurveRef::Edge(p) => {
            let (a, b) = (p[0], *p.last().unwrap_or(&p[0]));
            AxisRef::Fixed { origin: a, dir: [b[0] - a[0], b[1] - a[1], b[2] - a[2]] }
        }
    }
}

/// Entradas atuais de um elemento (para preservá-las ao trocar o tipo).
pub fn inputs_of(k: &RefKind) -> Seed {
    let mut s = Seed::default();
    let mut k = k.clone();
    use RefKind as R;
    match &mut k {
        R::OffsetPlane { base, .. } | R::AnglePlane { base, .. } => s.planes.push(*base),
        R::PlaneParallelPoint { base, point } => {
            s.planes.push(*base);
            s.points.push(*point)
        }
        R::PlaneThreePoints { a, b, c } | R::Circle3 { a, b, c, .. } => s.points.extend([*a, *b, *c]),
        R::PlaneTwoLines { a, b } | R::Corner { a, b, .. } | R::Connect { a, b, .. } => s.curves.extend([a.clone(), b.clone()]),
        R::PlanePointLine { point, line } => {
            s.points.push(*point);
            s.curves.push(line.clone())
        }
        R::PlaneCurve { curve } | R::PlaneNormalCurve { curve, .. } | R::PointOnCurve { curve, .. } | R::PointCenter { curve } | R::PointTangent { curve, .. } => {
            s.curves.push(curve.clone())
        }
        R::PlaneTangentSurface { surface, point } | R::LineNormalSurface { surface, point, .. } => {
            s.surface = Some(*surface);
            s.points.push(*point)
        }
        R::PlaneMeanPoints { points } | R::Polyline { points, .. } | R::Spline { points, .. } => s.points.extend(points.iter().copied()),
        R::PointCoords { frame, .. } => s.planes.extend(*frame),
        R::PointOnPlane { plane, .. } => s.planes.push(*plane),
        R::PointOnSurface { surface, .. } => s.surface = Some(*surface),
        R::PointBetween { a, b, .. } | R::LinePoints { a, b, .. } => s.points.extend([*a, *b]),
        R::LineDir { point, .. } => s.points.push(*point),
        R::LineAngle { curve, point, support, .. } => {
            s.curves.push(curve.clone());
            s.points.push(*point);
            s.planes.push(*support)
        }
        R::Circle { center, plane, .. } | R::Spiral { center, plane, .. } | R::Conic { center, plane, .. } => {
            s.points.push(*center);
            s.planes.push(*plane)
        }
        R::Helix { origin, .. } => s.points.push(*origin),
        R::Spine { planes } => s.planes.extend(planes.iter().copied()),
        R::FacePlane { .. } | R::PlaneEquation { .. } => {}
    }
    s
}

// ---------------------------------------------------------------- nomes

pub fn point_name(doc: &Document, p: &PointRef) -> String {
    match p {
        PointRef::Origin => "Origem".into(),
        PointRef::Ref(id) => doc.reference(*id).map(|r| r.name.clone()).unwrap_or_else(|| format!("⚠ perdido #{}", id.0)),
        PointRef::Fixed(q) => format!("({}, {}, {})", fmt(q[0]), fmt(q[1]), fmt(q[2])),
    }
}

pub fn curve_name(doc: &Document, c: &CurveRef) -> String {
    match c {
        CurveRef::Axis(a) => format!("Origem · eixo {a:?}"),
        CurveRef::Ref(id) => doc.reference(*id).map(|r| r.name.clone()).unwrap_or_else(|| format!("⚠ perdida #{}", id.0)),
        CurveRef::SketchLine { sketch, entity } => format!("{} · linha #{}", doc.name_of(*sketch), entity.0),
        CurveRef::Edge(p) => format!("Aresta ({} pontos)", p.len()),
    }
}

fn surf_name(doc: &Document, s: &SurfRef) -> String {
    match s {
        SurfRef::Plane(p) => plane_ref_name(doc, p),
        SurfRef::Face { owner, .. } => format!("Face de {}", owner.map(|o| doc.name_of(o)).unwrap_or_default()),
    }
}

fn fmt(v: f64) -> String {
    crate::app::fmt(v)
}

/// Elementos de uma classe que podem ser usados por `me` (sem ele e sem quem depende dele).
fn candidates(doc: &Document, class: RefClass, me: Id) -> Vec<(Id, String)> {
    let deps = doc.dependents(me);
    doc.references.iter().filter(|r| r.kind.class() == class && r.id != me && !deps.contains(&r.id)).map(|r| (r.id, r.name.clone())).collect()
}

// ---------------------------------------------------------------- entradas

fn row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(label).truncate());
        add(ui);
    });
}

fn pick_button(ui: &mut egui::Ui, enabled: bool, what: &str) -> bool {
    ui.add_enabled(enabled, egui::Button::new("⊙").small())
        .on_hover_text(format!("usar {what} selecionado(a) na viewport"))
        .on_disabled_hover_text(format!("selecione {what} na viewport (Shift soma) para usar aqui"))
        .clicked()
}

pub(crate) fn point_slot(ui: &mut egui::Ui, salt: &str, label: &str, doc: &Document, me: Id, p: &mut PointRef, seed: &Seed) {
    row(ui, label, |ui| {
        egui::ComboBox::from_id_salt(salt).selected_text(point_name(doc, p)).width(150.).show_ui(ui, |ui| {
            ui.selectable_value(p, PointRef::Origin, "Origem");
            for (id, n) in candidates(doc, RefClass::Point, me) {
                ui.selectable_value(p, PointRef::Ref(id), n);
            }
            if !matches!(p, PointRef::Fixed(_)) && ui.selectable_label(false, "Coordenadas fixas…").clicked() {
                *p = PointRef::Fixed([0.; 3]);
            }
        });
        if pick_button(ui, !seed.points.is_empty(), "um vértice ou ponto") {
            *p = *seed.points.last().unwrap();
        }
    });
    if let PointRef::Fixed(q) = p {
        ui.horizontal(|ui| {
            ui.add_space(100.);
            for (i, n) in ["x ", "y ", "z "].iter().enumerate() {
                ui.add(egui::DragValue::new(&mut q[i]).speed(0.5).prefix(*n));
            }
        });
    }
}

pub(crate) fn curve_slot(ui: &mut egui::Ui, salt: &str, label: &str, doc: &Document, me: Id, c: &mut CurveRef, seed: &Seed) {
    row(ui, label, |ui| {
        egui::ComboBox::from_id_salt(salt).selected_text(curve_name(doc, c)).width(150.).show_ui(ui, |ui| {
            for a in [OriginAxis::X, OriginAxis::Y, OriginAxis::Z] {
                ui.selectable_value(c, CurveRef::Axis(a), format!("Origem · eixo {a:?}"));
            }
            for (id, n) in candidates(doc, RefClass::Curve, me) {
                ui.selectable_value(c, CurveRef::Ref(id), n);
            }
        });
        if pick_button(ui, !seed.curves.is_empty(), "uma aresta ou curva") {
            *c = seed.curves.last().unwrap().clone();
        }
    });
}

pub(crate) fn axis_slot(ui: &mut egui::Ui, salt: &str, label: &str, doc: &Document, me: Id, a: &mut AxisRef, seed: &Seed) {
    let name = match a {
        AxisRef::Origin(x) => format!("Origem · eixo {x:?}"),
        AxisRef::SketchLine { sketch, entity } => format!("{} · linha #{}", doc.name_of(*sketch), entity.0),
        AxisRef::Ref(id) => doc.name_of(*id),
        AxisRef::Fixed { .. } => "Aresta (direção fixa)".into(),
    };
    row(ui, label, |ui| {
        egui::ComboBox::from_id_salt(salt).selected_text(name).width(150.).show_ui(ui, |ui| {
            for x in [OriginAxis::X, OriginAxis::Y, OriginAxis::Z] {
                ui.selectable_value(a, AxisRef::Origin(x), format!("Origem · eixo {x:?}"));
            }
            for (id, n) in candidates(doc, RefClass::Curve, me) {
                ui.selectable_value(a, AxisRef::Ref(id), n);
            }
            for f in &doc.features {
                if let FeatureKind::Sketch(s) = &f.kind {
                    for e in s.data.curves.iter().filter(|e| matches!(e.geom, scc_core::sketch::Curve::Line { .. })) {
                        ui.selectable_value(a, AxisRef::SketchLine { sketch: f.id, entity: e.id }, format!("{} · linha #{}", f.name, e.id.0));
                    }
                }
            }
        });
        if pick_button(ui, !seed.curves.is_empty(), "uma aresta ou linha") {
            *a = axis_of(seed.curves.last().unwrap());
        }
    });
}

fn plane_slot(ui: &mut egui::Ui, salt: &str, label: &str, doc: &Document, me: Id, p: &mut PlaneRef, seed: &Seed) {
    row(ui, label, |ui| {
        plane_combo(ui, salt, doc, p, Some(me));
        if pick_button(ui, !seed.planes.is_empty(), "um plano") {
            *p = *seed.planes.last().unwrap();
        }
    });
}

fn surf_slot(ui: &mut egui::Ui, salt: &str, label: &str, doc: &Document, me: Id, s: &mut SurfRef, seed: &Seed) {
    row(ui, label, |ui| {
        let mut pl = match s {
            SurfRef::Plane(p) => *p,
            SurfRef::Face { .. } => PlaneRef::Datum(Id(u64::MAX)),
        };
        egui::ComboBox::from_id_salt(salt).selected_text(surf_name(doc, s)).width(150.).show_ui(ui, |ui| {
            for o in [OriginPlane::XY, OriginPlane::XZ, OriginPlane::YZ] {
                ui.selectable_value(&mut pl, PlaneRef::Origin(o), plane_ref_name(doc, &PlaneRef::Origin(o)));
            }
            for (id, n) in candidates(doc, RefClass::Plane, me) {
                ui.selectable_value(&mut pl, PlaneRef::Datum(id), n);
            }
        });
        if pl != PlaneRef::Datum(Id(u64::MAX)) {
            *s = SurfRef::Plane(pl);
        }
        if pick_button(ui, seed.surface.is_some() || !seed.planes.is_empty(), "uma face ou plano") {
            *s = seed.surface.unwrap_or_else(|| SurfRef::Plane(*seed.planes.last().unwrap()));
        }
    });
}

pub(crate) fn points_list(ui: &mut egui::Ui, salt: &str, doc: &Document, me: Id, pts: &mut Vec<PointRef>, seed: &Seed, min: usize) {
    let mut del = None;
    for (i, p) in pts.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.vertical(|ui| point_slot(ui, &format!("{salt}{i}"), &format!("Ponto {}", i + 1), doc, me, p, seed));
            if ui.small_button("✖").on_hover_text("remover ponto").clicked() {
                del = Some(i);
            }
        });
    }
    if let Some(i) = del {
        if pts.len() > min {
            pts.remove(i);
        }
    }
    ui.horizontal(|ui| {
        if ui.button("+ ponto").clicked() {
            pts.push(PointRef::Fixed([0.; 3]));
        }
        if ui.add_enabled(!seed.points.is_empty(), egui::Button::new(format!("+ seleção ({})", seed.points.len()))).on_hover_text("acrescenta os vértices/pontos selecionados na ordem do clique").clicked() {
            pts.extend(seed.points.iter().copied());
        }
    });
}

fn on_curve(ui: &mut egui::Ui, at: &mut OnCurve, reverse: &mut bool, vars: &HashMap<String, f64>) {
    row(ui, "Posição", |ui| {
        let is_ratio = matches!(at, OnCurve::Ratio(_));
        if ui.selectable_label(is_ratio, "proporção").clicked() && !is_ratio {
            *at = OnCurve::Ratio("0.5".into());
        }
        if ui.selectable_label(!is_ratio, "distância").clicked() && is_ratio {
            *at = OnCurve::Distance("10".into());
        }
    });
    match at {
        OnCurve::Ratio(e) => {
            expr(ui, "Proporção", e, vars, "0..1");
            ui.horizontal(|ui| {
                ui.add_space(100.);
                for (n, v) in [("Início", "0"), ("Meio", "0.5"), ("Fim", "1")] {
                    if ui.small_button(n).clicked() {
                        *e = v.into();
                    }
                }
            });
        }
        OnCurve::Distance(e) => expr(ui, "Distância", e, vars, "mm"),
    }
    ui.horizontal(|ui| {
        ui.add_space(100.);
        ui.checkbox(reverse, "medir a partir do fim");
    });
}

/// Painel de um elemento de geometria de referência.
pub fn panel(ui: &mut egui::Ui, doc: &Document, r: &mut RefElement, vars: &HashMap<String, f64>, seed: &Seed, error: Option<&str>) {
    let me = r.id;
    let class = r.kind.class();
    let cur = GeoKind::of(&r.kind);
    ui.heading(match class {
        RefClass::Plane => "Plano de referência",
        RefClass::Point => "Ponto de referência",
        RefClass::Curve => "Linha / curva de referência",
    });
    if let Some(e) = error {
        egui::Frame::NONE.fill(Color32::from_rgb(80, 30, 28)).inner_margin(6.).corner_radius(4.).show(ui, |ui| {
            ui.label(RichText::new(format!("✖ {e}")).color(Color32::from_rgb(255, 150, 140)));
        });
    }
    if let Some(k) = cur {
        section(ui, "Tipo");
        let fam = k.family();
        let opts: Vec<(&str, &str, &str, bool)> = fam.iter().map(|g| {
            let (i, n, b, _) = g.info();
            (i, n, b, *g == k)
        }).collect();
        if let Some(i) = icon_choice(ui, &opts) {
            if fam[i] != k {
                // mantém as entradas atuais; o que faltar vem da seleção
                let mut s = inputs_of(&r.kind);
                s.points.extend(seed.points.iter().copied());
                s.curves.extend(seed.curves.iter().cloned());
                s.planes.extend(seed.planes.iter().copied());
                s.surface = s.surface.or(seed.surface);
                r.kind = fam[i].make(&s);
            }
        }
        ui.label(RichText::new(k.info().3).weak().small());
    }
    section(ui, "Definição");
    use RefKind as R;
    match &mut r.kind {
        R::FacePlane { owner, .. } => {
            let o = owner.map(|o| doc.name_of(o)).unwrap_or_default();
            ui.label(RichText::new(format!("Plano da face clicada em {o} (fixo nesta versão)")).weak());
        }
        R::OffsetPlane { base, offset } => {
            plane_slot(ui, "g_base", "Plano base", doc, me, base, seed);
            expr(ui, "Deslocamento", offset, vars, "mm");
        }
        R::AnglePlane { base, axis, angle } => {
            plane_slot(ui, "g_base", "Plano base", doc, me, base, seed);
            axis_slot(ui, "g_eixo", "Eixo", doc, me, axis, seed);
            expr(ui, "Ângulo", angle, vars, "°");
            ui.horizontal(|ui| {
                ui.add_space(100.);
                if ui.small_button("Normal (90°)").clicked() {
                    *angle = "90".into();
                }
            });
        }
        R::PlaneParallelPoint { base, point } => {
            plane_slot(ui, "g_base", "Paralelo a", doc, me, base, seed);
            point_slot(ui, "g_p", "Passa por", doc, me, point, seed);
        }
        R::PlaneThreePoints { a, b, c } => {
            point_slot(ui, "g_a", "Ponto 1", doc, me, a, seed);
            point_slot(ui, "g_b", "Ponto 2", doc, me, b, seed);
            point_slot(ui, "g_c", "Ponto 3", doc, me, c, seed);
        }
        R::PlaneTwoLines { a, b } => {
            curve_slot(ui, "g_a", "Linha 1", doc, me, a, seed);
            curve_slot(ui, "g_b", "Linha 2", doc, me, b, seed);
        }
        R::PlanePointLine { point, line } => {
            point_slot(ui, "g_p", "Ponto", doc, me, point, seed);
            curve_slot(ui, "g_l", "Linha", doc, me, line, seed);
        }
        R::PlaneCurve { curve } => curve_slot(ui, "g_c", "Curva planar", doc, me, curve, seed),
        R::PlaneNormalCurve { curve, at, reverse } => {
            curve_slot(ui, "g_c", "Curva", doc, me, curve, seed);
            on_curve(ui, at, reverse, vars);
        }
        R::PlaneTangentSurface { surface, point } => {
            surf_slot(ui, "g_s", "Superfície", doc, me, surface, seed);
            point_slot(ui, "g_p", "Ponto", doc, me, point, seed);
        }
        R::PlaneEquation { a, b, c, d } => {
            expr(ui, "A", a, vars, "");
            expr(ui, "B", b, vars, "");
            expr(ui, "C", c, vars, "");
            expr(ui, "D", d, vars, "mm");
            ui.label(RichText::new("Ax + By + Cz = D (normal = A, B, C)").weak().small());
        }
        R::PlaneMeanPoints { points } => points_list(ui, "g_pts", doc, me, points, seed, 3),
        R::PointCoords { x, y, z, frame } => {
            let mut local = frame.is_some();
            row(ui, "Sistema", |ui| {
                ui.radio_value(&mut local, false, "Origem");
                ui.radio_value(&mut local, true, "Plano");
            });
            if local != frame.is_some() {
                *frame = if local { Some(seed.planes.first().copied().unwrap_or_default()) } else { None };
            }
            if let Some(f) = frame {
                plane_slot(ui, "g_f", "Plano", doc, me, f, seed);
            }
            let (lx, ly, lz) = if frame.is_some() { ("u", "v", "Altura") } else { ("X", "Y", "Z") };
            expr(ui, lx, x, vars, "mm");
            expr(ui, ly, y, vars, "mm");
            expr(ui, lz, z, vars, "mm");
        }
        R::PointOnCurve { curve, at, reverse } => {
            curve_slot(ui, "g_c", "Curva", doc, me, curve, seed);
            on_curve(ui, at, reverse, vars);
        }
        R::PointOnPlane { plane, u, v } => {
            plane_slot(ui, "g_pl", "Plano", doc, me, plane, seed);
            expr(ui, "u", u, vars, "mm");
            expr(ui, "v", v, vars, "mm");
        }
        R::PointOnSurface { surface, u, v } => {
            surf_slot(ui, "g_s", "Superfície", doc, me, surface, seed);
            expr(ui, "u", u, vars, "mm");
            expr(ui, "v", v, vars, "mm");
        }
        R::PointCenter { curve } => curve_slot(ui, "g_c", "Círculo/arco", doc, me, curve, seed),
        R::PointTangent { curve, dir, which } => {
            curve_slot(ui, "g_c", "Curva planar", doc, me, curve, seed);
            axis_slot(ui, "g_d", "Direção", doc, me, dir, seed);
            row(ui, "Solução", |ui| {
                ui.radio_value(which, 0, "1");
                ui.radio_value(which, 1, "2");
            });
        }
        R::PointBetween { a, b, ratio } => {
            point_slot(ui, "g_a", "Ponto 1", doc, me, a, seed);
            point_slot(ui, "g_b", "Ponto 2", doc, me, b, seed);
            expr(ui, "Proporção", ratio, vars, "0..1");
        }
        R::LinePoints { a, b, start, end } => {
            point_slot(ui, "g_a", "Ponto 1", doc, me, a, seed);
            point_slot(ui, "g_b", "Ponto 2", doc, me, b, seed);
            expr(ui, "Prolongar início", start, vars, "mm");
            expr(ui, "Prolongar fim", end, vars, "mm");
        }
        R::LineDir { point, dir, start, end } => {
            point_slot(ui, "g_p", "Ponto", doc, me, point, seed);
            axis_slot(ui, "g_d", "Direção", doc, me, dir, seed);
            expr(ui, "Início", start, vars, "mm");
            expr(ui, "Fim", end, vars, "mm");
        }
        R::LineAngle { curve, point, support, angle, start, end } => {
            curve_slot(ui, "g_c", "Curva", doc, me, curve, seed);
            point_slot(ui, "g_p", "Ponto", doc, me, point, seed);
            plane_slot(ui, "g_s", "Plano de apoio", doc, me, support, seed);
            expr(ui, "Ângulo", angle, vars, "°");
            ui.horizontal(|ui| {
                ui.add_space(100.);
                if ui.small_button("Tangente (0°)").clicked() {
                    *angle = "0".into();
                }
                if ui.small_button("Normal (90°)").clicked() {
                    *angle = "90".into();
                }
            });
            expr(ui, "Início", start, vars, "mm");
            expr(ui, "Fim", end, vars, "mm");
        }
        R::LineNormalSurface { surface, point, start, end } => {
            surf_slot(ui, "g_s", "Superfície", doc, me, surface, seed);
            point_slot(ui, "g_p", "Ponto", doc, me, point, seed);
            expr(ui, "Início", start, vars, "mm");
            expr(ui, "Fim", end, vars, "mm");
        }
        R::Polyline { points, closed } | R::Spline { points, closed } => {
            points_list(ui, "g_pts", doc, me, points, seed, 2);
            ui.checkbox(closed, "fechada");
        }
        R::Circle { center, plane, radius, start, end } => {
            point_slot(ui, "g_c", "Centro", doc, me, center, seed);
            plane_slot(ui, "g_pl", "Plano", doc, me, plane, seed);
            expr(ui, "Raio", radius, vars, "mm");
            expr(ui, "Ângulo inicial", start, vars, "°");
            expr(ui, "Ângulo final", end, vars, "°");
        }
        R::Circle3 { a, b, c, full } => {
            point_slot(ui, "g_a", "Ponto 1", doc, me, a, seed);
            point_slot(ui, "g_b", "Ponto 2", doc, me, b, seed);
            point_slot(ui, "g_c", "Ponto 3", doc, me, c, seed);
            ui.checkbox(full, "círculo completo (senão, arco do 1º ao 3º)");
        }
        R::Helix { origin, axis, radius, pitch, height, taper, left } => {
            point_slot(ui, "g_o", "Origem", doc, me, origin, seed);
            axis_slot(ui, "g_ax", "Eixo", doc, me, axis, seed);
            expr(ui, "Raio", radius, vars, "mm");
            expr(ui, "Passo", pitch, vars, "mm");
            expr(ui, "Altura", height, vars, "mm");
            expr(ui, "Conicidade", taper, vars, "°");
            ui.checkbox(left, "sentido anti-horário (à esquerda)");
        }
        R::Spiral { center, plane, r0, r1, turns } => {
            point_slot(ui, "g_c", "Centro", doc, me, center, seed);
            plane_slot(ui, "g_pl", "Plano", doc, me, plane, seed);
            expr(ui, "Raio inicial", r0, vars, "mm");
            expr(ui, "Raio final", r1, vars, "mm");
            expr(ui, "Voltas", turns, vars, "");
        }
        R::Conic { center, plane, kind, a, b, extent } => {
            row(ui, "Cônica", |ui| {
                ui.radio_value(kind, ConicKind::Elipse, "Elipse");
                ui.radio_value(kind, ConicKind::Parabola, "Parábola");
                ui.radio_value(kind, ConicKind::Hiperbole, "Hipérbole");
            });
            point_slot(ui, "g_c", "Centro", doc, me, center, seed);
            plane_slot(ui, "g_pl", "Plano", doc, me, plane, seed);
            let k = *kind;
            expr(ui, if k == ConicKind::Parabola { "Foco (a)" } else { "a" }, a, vars, "mm");
            if k != ConicKind::Parabola {
                expr(ui, "b", b, vars, "mm");
            }
            if k != ConicKind::Elipse {
                expr(ui, "Extensão", extent, vars, "mm");
            }
        }
        R::Connect { a, a_end, b, b_end, cont, tension } => {
            curve_slot(ui, "g_a", "Curva 1", doc, me, a, seed);
            row(ui, "Ponta", |ui| {
                ui.radio_value(a_end, false, "início");
                ui.radio_value(a_end, true, "fim");
            });
            curve_slot(ui, "g_b", "Curva 2", doc, me, b, seed);
            row(ui, "Ponta", |ui| {
                ui.radio_value(b_end, false, "início");
                ui.radio_value(b_end, true, "fim");
            });
            row(ui, "Continuidade", |ui| {
                ui.radio_value(cont, Continuity::G0, "G0 ponto");
                ui.radio_value(cont, Continuity::G1, "G1 tangente");
                ui.radio_value(cont, Continuity::G2, "G2 curvatura");
            });
            expr(ui, "Tensão", tension, vars, "");
        }
        R::Corner { a, b, radius } => {
            curve_slot(ui, "g_a", "Linha 1", doc, me, a, seed);
            curve_slot(ui, "g_b", "Linha 2", doc, me, b, seed);
            expr(ui, "Raio", radius, vars, "mm");
        }
        R::Spine { planes } => {
            let mut del = None;
            for (i, p) in planes.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| plane_slot(ui, &format!("g_sp{i}"), &format!("Plano {}", i + 1), doc, me, p, seed));
                    if ui.small_button("✖").clicked() {
                        del = Some(i);
                    }
                });
            }
            if let Some(i) = del {
                if planes.len() > 2 {
                    planes.remove(i);
                }
            }
            ui.horizontal(|ui| {
                if ui.button("+ plano").clicked() {
                    planes.push(PlaneRef::default());
                }
                if ui.add_enabled(!seed.planes.is_empty(), egui::Button::new(format!("+ seleção ({})", seed.planes.len()))).clicked() {
                    planes.extend(seed.planes.iter().copied());
                }
            });
        }
    }
    ui.add_space(4.);
    ui.label(RichText::new("⊙ usa o que está selecionado na viewport (Shift soma vértices, arestas, faces, planos, pontos e curvas).").weak().small());
}
