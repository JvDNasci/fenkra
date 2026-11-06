// SPDX-License-Identifier: LGPL-2.1-or-later
//! "Fast 3D": alças na viewport para o comando aberto — seta arrastável e cota editável
//! (extrusão/recorte, plano deslocado, altura das primitivas, profundidade do reforço).
//! Arrastar muda o valor em passos de 1 mm (Shift: 0,1 mm); duplo clique na cota abre a edição,
//! que aceita expressões. Valores ligados a parâmetros ficam travados (🔒) para não quebrar o vínculo.

use crate::app::{fmt, App};
use crate::render::Camera;
use eframe::egui::{self, Color32, Rect, RichText, Stroke};
use glam::Vec3;
use fenkra_core::*;
use crate::i18n::tr;

mod aresta;
mod furo;
mod limites;
mod padrao;
pub(crate) mod solidos;

/// O comando Furo espera o clique numa face para posicionar o furo (B90)?
pub(crate) fn furo_espera_clique(h: &Hole) -> bool {
    furo::espera_clique(h)
}

/// Tamanho da pega das alças na tela, igual em toda seta (B228): um alvo que o mouse acerta sem mira.
pub(crate) const PEGA: f32 = 28.;

/// Alça linear: valor = distância ao longo de `dir` a partir de `base` (vezes `k`).
struct Linear {
    base: Vec3,
    dir: Vec3,
    /// valor exibido = comprimento da seta × k (plano médio: k = 2)
    k: f32,
    /// expressão editada
    expr: String,
    label: &'static str,
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Origem e normal de um plano já calculado.
pub(crate) fn plane_of(r: &fenkra_solidos::RecomputeResult, p: &PlaneRef) -> Option<(Vec3, Vec3)> {
    let f = match p {
        PlaneRef::Origin(o) => fenkra_solidos::origin_frame(*o),
        PlaneRef::Datum(id) => r.planes.iter().find(|x| x.id == Some(*id))?.frame,
    };
    Some((v3(f.origin), v3(f.normal)))
}

/// Centro do perfil de um sketch e a normal do plano dele.
fn sketch_center(r: &fenkra_solidos::RecomputeResult, id: Id) -> Option<(Vec3, Vec3)> {
    let s = r.sketches.iter().find(|s| s.feature == id)?;
    let n = v3(s.frame.normal);
    let c = if s.lines.is_empty() {
        v3(s.frame.origin)
    } else {
        s.lines.iter().fold(Vec3::ZERO, |a, p| a + Vec3::from(*p)) / s.lines.len() as f32
    };
    Some((c, n))
}

fn handle_for(app: &App, id: Id) -> Option<Linear> {
    let r = app.result.as_ref()?;
    let doc = &app.open.doc;
    if let Some(f) = doc.feature(id) {
        return match &f.kind {
            FeatureKind::Extrude(e) => {
                let (c, n) = sketch_center(r, e.sketch)?;
                let into = (e.op == BoolOp::Cut) != e.reverse;
                let dir = if into { -n } else { n };
                match &e.extent {
                    Extent::Blind(d) => Some(Linear { base: c, dir, k: 1., expr: d.clone(), label: "Distância" }),
                    Extent::Symmetric(d) => Some(Linear { base: c, dir, k: 2., expr: d.clone(), label: "Distância total" }),
                    Extent::TwoSides(a, _) => Some(Linear { base: c, dir, k: 1., expr: a.clone(), label: "Distância 1" }),
                    _ => None,
                }
            }
            FeatureKind::Primitive(p) => {
                let (o, n) = plane_of(r, &p.plane)?;
                let h = match p.kind {
                    PrimKind::Caixa | PrimKind::Cone | PrimKind::Prisma | PrimKind::Cunha => p.c.clone(),
                    PrimKind::Cilindro => p.b.clone(),
                    _ => return None,
                };
                Some(Linear { base: o, dir: n, k: 1., expr: h, label: "Altura" })
            }
            FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => {
                let base = match (&o.edges, &o.mode) {
                    (_, EdgeMode::FaceFace(a, _)) => *a,
                    (EdgeSel::Edges(m), _) if !m.is_empty() => m[0],
                    _ => return None,
                };
                let b = r.bodies.iter().find(|b| b.collector == f.target)?;
                let c = mesh_center(&b.mesh);
                let base = v3(base);
                let dir = (base - c).try_normalize()?;
                Some(Linear { base, dir, k: 1., expr: o.value.clone(), label: if matches!(f.kind, FeatureKind::Fillet(_)) { "Raio" } else { "Distância" } })
            }
            FeatureKind::Shell(sh) => {
                let p = v3(*sh.faces.first()?);
                let b = r.bodies.iter().find(|b| b.collector == f.target)?;
                let dir = (mesh_center(&b.mesh) - p).try_normalize()?;
                Some(Linear { base: p, dir: if sh.outward { -dir } else { dir }, k: 1., expr: sh.thickness.clone(), label: "Espessura" })
            }
            FeatureKind::ThickSurface(t) => {
                let (base, n) = surface_anchor(r, &t.surface)?;
                Some(Linear { base, dir: n, k: 1., expr: t.t1.clone(), label: "Espessura" })
            }
            // B07: a seta sai da face escolhida, pela normal dela na peça de antes do comando
            FeatureKind::Thickness(t) => {
                let p = *t.faces.first()?;
                let antes = app.session.as_ref()?.corpos_antes.iter().find(|c| c.collector == f.target)?;
                let info = antes.mesh.face_info.get(crate::picking::face_da_malha(&antes.mesh, p)?)?;
                let n = v3(info.dir);
                (info.kind == 0).then_some(Linear { base: v3(p), dir: if t.remove { -n } else { n }, k: 1., expr: t.value.clone(), label: "Espessura" })
            }
            FeatureKind::Stiffener(s) => {
                let (c, n) = sketch_center(r, s.sketch)?;
                Some(Linear { base: c, dir: if s.reverse { n } else { -n }, k: 1., expr: s.depth.clone(), label: "Profundidade" })
            }
            // chapa metálica: aba/bainha pelo centro das faces que ela criou; estampo e furo flangeado pelo sketch
            FeatureKind::SmFlange(fl) => {
                let base = v3(fl.edge);
                let c = owned_center(r, f.target, f.id)?;
                Some(Linear { base, dir: (c - base).try_normalize()?, k: 1., expr: fl.length.clone(), label: "Comprimento da aba" })
            }
            FeatureKind::SmHem(hm) => {
                let base = v3(hm.edge);
                let c = owned_center(r, f.target, f.id)?;
                Some(Linear { base, dir: (c - base).try_normalize()?, k: 1., expr: hm.length.clone(), label: "Comprimento da bainha" })
            }
            FeatureKind::SmStamp(st) => {
                let (c, n) = sketch_center(r, st.sketch)?;
                Some(Linear { base: c, dir: if st.flip { n } else { -n }, k: 1., expr: st.depth.clone(), label: "Profundidade" })
            }
            FeatureKind::SmFlangedHole(fh) => {
                let (c, n) = sketch_center(r, fh.sketch)?;
                Some(Linear { base: c, dir: if fh.flip { n } else { -n }, k: 1., expr: fh.height.clone(), label: "Altura do colar" })
            }
            FeatureKind::SmLouver(lv) => {
                let base = (v3(lv.a) + v3(lv.b)) * 0.5;
                let c = owned_center(r, f.target, f.id)?;
                let n = (c - base).try_normalize()?;
                Some(Linear { base, dir: n, k: 1., expr: lv.height.clone(), label: "Altura da veneziana" })
            }
            _ => None,
        };
    }
    let rf = doc.reference(id)?;
    match &rf.kind {
        RefKind::OffsetPlane { base, offset } => {
            let (o, n) = plane_of(r, base)?;
            // a seta sai do centro do plano deslocado projetado no plano base
            let me = r.planes.iter().find(|x| x.id == Some(id)).map(|x| v3(x.frame.origin)).unwrap_or(o);
            let b = me - n * (me - o).dot(n);
            Some(Linear { base: b, dir: n, k: 1., expr: offset.clone(), label: "Deslocamento" })
        }
        RefKind::LineDir { point, dir, end, .. } => {
            let base = point_of(r, point)?;
            let (_, d) = revolve_axis(app, dir)?;
            Some(Linear { base, dir: d, k: 1., expr: end.clone(), label: "Comprimento" })
        }
        RefKind::PointOffset { point, dir, dist } => {
            let base = point_of(r, point)?;
            let (_, d) = revolve_axis(app, dir)?;
            Some(Linear { base, dir: d, k: 1., expr: dist.clone(), label: "Distância" })
        }
        _ => None,
    }
}

/// Ponto e normal de uma superfície de entrada, pela malha.
fn surface_anchor(r: &fenkra_solidos::RecomputeResult, s: &SurfRef) -> Option<(Vec3, Vec3)> {
    match s {
        SurfRef::Plane(p) => plane_of(r, p),
        SurfRef::Face { point, normal, .. } => Some((v3(*point), v3(*normal).try_normalize()?)),
        SurfRef::Ref(id) => {
            let m = &r.surfaces.iter().find(|x| x.id == *id)?.mesh;
            let c = mesh_center(m);
            let at = |i: usize, a: &[f32]| Vec3::new(a[3 * i], a[3 * i + 1], a[3 * i + 2]);
            let i = (0..m.positions.len() / 3).min_by(|a, b| (at(*a, &m.positions) - c).length().total_cmp(&(at(*b, &m.positions) - c).length()))?;
            Some((at(i, &m.positions), at(i, &m.normals).try_normalize()?))
        }
    }
}

/// Setas da superfície deslocada: saem do ponto da superfície pela normal real, e o sinal da
/// distância escolhe o lado sozinho. Com os dois lados, uma seta para cada lado.
fn offset_arrows(base: Vec3, n: Vec3, expr: &str, both: bool) -> Vec<(u8, Linear)> {
    let one = |dir: Vec3| (0, Linear { base, dir, k: 1., expr: expr.to_string(), label: "Distância" });
    if both {
        vec![one(n), one(-n)]
    } else {
        vec![one(n)]
    }
}

/// Alças dos valores principais da bancada de superfícies e wireframe: (campo, alça).
fn surface_handles(r: &fenkra_solidos::RecomputeResult, doc: &Document, id: Id) -> Vec<(u8, Linear)> {
    let Some(rf) = doc.reference(id) else { return vec![] };
    let axis = |a: &AxisRef| axis_pos(r, doc, a).map(|x| x.1);
    let center_of = |s: &SurfRef| match s {
        SurfRef::Ref(x) => r.surfaces.iter().find(|m| m.id == *x).map(|m| mesh_center(&m.mesh)),
        SurfRef::Face { point, .. } => Some(v3(*point)),
        SurfRef::Plane(p) => plane_of(r, p).map(|x| x.0),
    };
    let me = || r.surfaces.iter().find(|s| s.id == id).map(|s| mesh_center(&s.mesh));
    let lin = |base, dir, expr: &String, label| Linear { base, dir, k: 1., expr: expr.clone(), label };
    let mut out = vec![];
    match &rf.kind {
        RefKind::SurfOffset { dist, both, .. } => {
            if let Some((q, n)) = r.notes.get(&id).and_then(|x| x.arrow) {
                return offset_arrows(v3(q), v3(n), dist, *both);
            }
        }
        RefKind::SurfExtrude { curve, dir, length, length2, until, until2 } => {
            if let (Some(base), Some(d)) = (curve_mid(r, curve), axis(dir)) {
                if until.is_none() {
                    out.push((0, lin(base, d, length, "Limite 1")));
                }
                if until2.is_none() {
                    out.push((1, lin(base, -d, length2, "Limite 2")));
                }
            }
        }
        RefKind::SurfCylinder { point, dir, end, radius, .. } => {
            if let (Some(base), Some(d)) = (point_of(r, point), axis(dir)) {
                out.push((0, lin(base, d, end, "Altura")));
                out.push((1, lin(base, d.any_orthonormal_vector(), radius, "Raio")));
            }
        }
        RefKind::SurfSphere { center, radius } => {
            if let Some(base) = point_of(r, center) {
                out.push((0, lin(base, Vec3::X, radius, "Raio")));
            }
        }
        RefKind::SurfSweep { profile: SweepProfile::Circle(radius), spine, .. } => {
            if let Some(p) = crate::picking::curve_pts(r, spine) {
                let (a, b) = (v3(p[0]), v3(p[1.min(p.len() - 1)]));
                if let Some(t) = (b - a).try_normalize() {
                    out.push((0, lin(a, t.any_orthonormal_vector(), radius, "Raio")));
                }
            }
        }
        RefKind::SurfExtrapolate { surface, near, length, edges, until, .. } if until.is_none() => {
            let base = edges.first().and_then(|c| curve_mid(r, c)).or_else(|| point_of(r, near));
            if let (Some(base), Some(c)) = (base, center_of(surface)) {
                if let Some(d) = (base - c).try_normalize() {
                    out.push((0, lin(base, d, length, "Comprimento")));
                }
            }
        }
        RefKind::SurfFillet { a, radius, .. } => {
            if let (Some(base), Some(c)) = (me(), center_of(a)) {
                if let Some(d) = (base - c).try_normalize() {
                    out.push((0, lin(base, d, radius, "Raio")));
                }
            }
        }
        RefKind::SurfEdgeFillet { surface, edges, radius, .. } => {
            if let (Some(p), Some(c)) = (edges.first(), center_of(surface)) {
                if let Some(d) = (v3(*p) - c).try_normalize() {
                    out.push((0, lin(v3(*p), d, radius, "Raio")));
                }
            }
        }
        RefKind::PointOnCurve { curve, at, .. } => {
            if let (Some(c), Some(p)) = (crate::picking::curve_pts(r, curve), r.points.iter().find(|x| x.id == id)) {
                let total: f32 = c.windows(2).map(|w| (v3(w[1]) - v3(w[0])).length()).sum();
                let (start, q) = (v3(c[0]), v3(p.p));
                // o valor corre ao longo da curva; a seta mostra a corda até o ponto
                if let Some(d) = (q - start).try_normalize() {
                    let (expr, k) = match at {
                        OnCurve::Distance(e) => (e, 1.),
                        OnCurve::Ratio(e) => (e, 1. / total.max(1e-6)),
                    };
                    out.push((0, Linear { base: start, dir: d, k, expr: expr.clone(), label: "Posição" }));
                }
            }
        }
        _ => {}
    }
    out
}

/// Posição de um ponto de entrada no resultado atual.
fn point_of(r: &fenkra_solidos::RecomputeResult, p: &PointRef) -> Option<Vec3> {
    crate::picking::point_pos(r, p).map(v3)
}

/// Ponto do meio de uma curva de entrada.
fn curve_mid(r: &fenkra_solidos::RecomputeResult, c: &CurveRef) -> Option<Vec3> {
    let p = crate::picking::curve_pts(r, c)?;
    p.get(p.len() / 2).map(|q| v3(*q))
}

/// Centro das faces que o comando criou no corpo (direção das abas/bainhas).
fn owned_center(r: &fenkra_solidos::RecomputeResult, target: Id, me: Id) -> Option<Vec3> {
    let b = r.bodies.iter().find(|b| b.collector == target)?;
    let m = &b.mesh;
    let mut acc = Vec3::ZERO;
    let mut n = 0;
    for (fi, owner) in b.face_owner.iter().enumerate() {
        if *owner != me {
            continue;
        }
        let i0 = *m.face_ranges.get(fi)? as usize;
        let i1 = m.face_ranges.get(fi + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
        for i in &m.indices[i0..i1] {
            let k = *i as usize * 3;
            acc += Vec3::new(m.positions[k], m.positions[k + 1], m.positions[k + 2]);
            n += 1;
        }
    }
    (n > 0).then(|| acc / n as f32)
}

/// Grava o novo valor na expressão da alça.
fn write_value(app: &mut App, id: Id, value: String) {
    let doc = &mut app.open.doc;
    if let Some(f) = doc.feature_mut(id) {
        match &mut f.kind {
            FeatureKind::Extrude(e) => match &mut e.extent {
                Extent::Blind(d) | Extent::Symmetric(d) | Extent::TwoSides(d, _) => *d = value,
                _ => {}
            },
            FeatureKind::Primitive(p) => match p.kind {
                PrimKind::Caixa | PrimKind::Cone | PrimKind::Prisma | PrimKind::Cunha => p.c = value,
                PrimKind::Cilindro => p.b = value,
                _ => {}
            },
            FeatureKind::Stiffener(s) => s.depth = value,
            FeatureKind::Thickness(t) => t.value = value,
            FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => o.value = value,
            FeatureKind::Shell(sh) => sh.thickness = value,
            FeatureKind::ThickSurface(t) => t.t1 = value,
            FeatureKind::Revolve(r) => r.angle = value,
            FeatureKind::SmFlange(x) => x.length = value,
            FeatureKind::SmHem(x) => x.length = value,
            FeatureKind::SmStamp(x) => x.depth = value,
            FeatureKind::SmFlangedHole(x) => x.height = value,
            FeatureKind::SmLouver(x) => x.height = value,
            _ => {}
        }
        return;
    }
    if let Some(rf) = doc.references.iter_mut().find(|r| r.id == id) {
        match &mut rf.kind {
            RefKind::OffsetPlane { offset, .. } => *offset = value,
            RefKind::SurfExtrude { length, .. } => *length = value,
            RefKind::SurfCylinder { end, .. } => *end = value,
            RefKind::SurfSphere { radius, .. } => *radius = value,
            RefKind::SurfOffset { dist, .. } => *dist = value,
            RefKind::SurfRevolve { angle, .. } => *angle = value,
            RefKind::SurfExtrapolate { length, .. } => *length = value,
            RefKind::SurfFillet { radius, .. } | RefKind::SurfEdgeFillet { radius, .. } => *radius = value,
            RefKind::SurfSweep { profile: SweepProfile::Circle(r), .. } => *r = value,
            RefKind::PointOnCurve { at, .. } => *at.expr_mut() = value,
            RefKind::LineDir { end, .. } => *end = value,
            RefKind::PointOffset { dist, .. } => *dist = value,
            _ => {}
        }
    }
}

/// Valor da segunda alça de um elemento (limite 2, raio do cilindro); a primeira usa `write_value`.
fn write_slot(app: &mut App, id: Id, slot: u8, value: String) {
    if crate::sup_loft::escrever(&mut app.open.doc, id, slot, &value) {
        return;
    }
    if slot == 0 {
        return write_value(app, id, value);
    }
    if let Some(FeatureKind::Loft(l)) = app.open.doc.feature_mut(id).map(|f| &mut f.kind) {
        match slot {
            crate::loft_3d::CAMPO_INICIO => l.inicio.intensidade = value,
            crate::loft_3d::CAMPO_FIM => l.fim.intensidade = value,
            _ => {}
        }
        return;
    }
    // B288: alças extras da bancada Sólidos têm os campos em handles/solidos.rs
    if slot >= solidos::PRIMEIRO {
        solidos::aplicar(&mut app.open.doc, id, slot, value);
        return;
    }
    if let Some(rf) = app.open.doc.references.iter_mut().find(|r| r.id == id) {
        match &mut rf.kind {
            RefKind::SurfExtrude { length2, .. } => *length2 = value,
            RefKind::SurfCylinder { radius, .. } => *radius = value,
            _ => {}
        }
    }
}

pub(crate) fn mesh_center(m: &fenkra_kernel::Mesh) -> Vec3 {
    let n = (m.positions.len() / 3).max(1) as f32;
    m.positions.chunks_exact(3).fold(Vec3::ZERO, |a, p| a + Vec3::new(p[0], p[1], p[2])) / n
}

/// Eixo da revolução no mundo (origem, direção).
fn revolve_axis(app: &App, a: &AxisRef) -> Option<(Vec3, Vec3)> {
    axis_pos(app.result.as_ref()?, &app.open.doc, a)
}

fn axis_pos(r: &fenkra_solidos::RecomputeResult, doc: &Document, a: &AxisRef) -> Option<(Vec3, Vec3)> {
    match a {
        AxisRef::Origin(x) => Some((Vec3::ZERO, v3(fenkra_solidos::origin_axis(*x)))),
        AxisRef::Fixed { origin, dir } => Some((v3(*origin), v3(*dir).try_normalize()?)),
        AxisRef::Face { .. } | AxisRef::Edge { .. } | AxisRef::Planes { .. } => {
            let (o, d) = crate::picking::live_axis(r, a)?;
            Some((v3(o), v3(d).try_normalize()?))
        }
        AxisRef::Ref(id) => {
            let c = r.curves.iter().find(|c| c.id == *id)?;
            let (p, q) = (v3(c.pts[0]), v3(*c.pts.last()?));
            Some((p, (q - p).try_normalize()?))
        }
        AxisRef::SketchLine { sketch, entity } => {
            let fr = r.sketches.iter().find(|s| s.feature == *sketch)?.frame;
            let FeatureKind::Sketch(sk) = &doc.feature(*sketch)?.kind else { return None };
            let fenkra_core::sketch::Curve::Line { a, b } = sk.data.curve(*entity)?.geom else { return None };
            let (pa, pb) = (sk.data.pos(a), sk.data.pos(b));
            let (p, q) = (v3(fr.point(pa[0], pa[1])), v3(fr.point(pb[0], pb[1])));
            Some((p, (q - p).try_normalize()?))
        }
    }
}

/// Alça de ângulo da revolução: arco em volta do eixo; arrastar a ponta muda o ângulo (passo 1°, Shift 5°).
fn revolve_handle(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let Some(FeatureKind::Revolve(rv)) = app.open.doc.feature(id).map(|f| f.kind.clone()) else { return };
    let Some(r) = app.result.as_ref() else { return };
    let Some(sk) = r.sketches.iter().find(|s| s.feature == rv.sketch) else { return };
    let Some((o, d)) = revolve_axis(app, &rv.axis) else { return };
    let c = if sk.lines.is_empty() { v3(sk.frame.origin) } else { sk.lines.iter().fold(Vec3::ZERO, |a, p| a + Vec3::from(*p)) / sk.lines.len() as f32 };
    arc_handle(app, ui, rect, cam, id, c, (o, d), &rv.angle, rv.symmetric, 0, (1., 360.));
}

/// Alça em arco do ângulo de revolução, em volta de `(o, d)`, a partir do ponto `c`.
#[allow(clippy::too_many_arguments)]
fn arc_handle(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id, c: Vec3, (o, d): (Vec3, Vec3), expr: &str, symmetric: bool, slot: u8, lim: (f64, f64)) {
    let foot = o + d * (c - o).dot(d);
    let r0 = c - foot;
    if r0.length() < 1e-4 {
        return;
    }
    let vars = app.vars();
    let Ok(ang) = fenkra_core::expr::eval_str(expr, &vars) else { return };
    let numeric = expr.trim().replace(',', ".").parse::<f64>().is_ok();
    let start = if symmetric { -ang / 2. } else { 0. };
    let at = |a: f64| foot + glam::Quat::from_axis_angle(d, a.to_radians() as f32) * r0;
    let pts: Vec<egui::Pos2> = (0..=48).filter_map(|i| cam.project(rect, at(start + ang * i as f64 / 48.))).collect();
    let Some(tip) = cam.project(rect, at(start + ang)) else { return };
    let painter = ui.painter_at(rect);
    let hid = egui::Id::unique(("alca_rev", id.0));
    let hr = ui.interact(Rect::from_center_size(tip, egui::vec2(PEGA, PEGA)), hid, if numeric { egui::Sense::drag() } else { egui::Sense::hover() });
    let erro = matches!(app.result.as_ref().and_then(|r| r.status.get(&id)), Some(fenkra_solidos::Status::Error(_)));
    let estado = crate::alca_estado::EstadoAlca::de(hr.hovered(), hr.dragged(), !numeric, erro);
    let col = estado.cor(ui.visuals());
    if let Some(d) = estado.dica_traduzida() {
        let _ = hr.clone().on_hover_text(d);
    }
    painter.add(egui::Shape::line(pts, Stroke::new(estado.espessura() + 1., col)));
    painter.circle_stroke(tip, 9., Stroke::new(estado.espessura(), col));
    painter.circle_filled(tip, 4., col);
    if hr.dragged() {
        if let Some(m) = ui.input(|i| i.pointer.interact_pos()) {
            // ângulo cujo ponto projetado fica mais perto do mouse
            let mut best = (f32::MAX, ang);
            for k in (lim.0 as i32)..=(lim.1 as i32) {
                if let Some(p) = cam.project(rect, at(start + k as f64)) {
                    let dd = (p - m).length();
                    if dd < best.0 {
                        best = (dd, k as f64);
                    }
                }
            }
            let step = if ui.input(|i| i.modifiers.shift) { 5. } else { 1. };
            let v = ((best.1 / step).round() * step).clamp(lim.0, lim.1);
            write_slot(app, id, slot, fmt(v));
        }
    }
    let txt = if numeric { format!("{}°", fmt(ang)) } else { format!("🔒 {} = {}°", expr.trim(), fmt(ang)) };
    let galley = painter.layout_no_wrap(txt, egui::FontId::proportional(crate::acessibilidade::fonte_3d(estado.fonte())), Color32::WHITE);
    let lr = Rect::from_min_size(tip + egui::vec2(14., -10.), galley.size() + egui::vec2(10., 6.));
    painter.rect_filled(lr, 4., Color32::from_rgba_unmultiplied(20, 60, 110, 220));
    painter.galley(lr.min + egui::vec2(5., 3.), galley, Color32::WHITE);
}

/// Desenha e trata a alça do comando aberto. Chamado pela viewport fora do modo sketch.
/// Etiqueta pequena na tela; a resposta traz o clique.
pub(crate) fn tag(ui: &mut egui::Ui, rect: Rect, at: egui::Pos2, text: String, fg: Color32, salt: impl std::hash::Hash + std::fmt::Debug) -> egui::Response {
    let painter = ui.painter_at(rect);
    let g = painter.layout_no_wrap(text, egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), fg);
    let r = Rect::from_center_size(at, g.size() + egui::vec2(8., 4.));
    painter.rect_filled(r, 3., ui.visuals().extreme_bg_color);
    painter.rect_stroke(r, 3., Stroke::new(1., fg), egui::StrokeKind::Inside);
    painter.galley(r.min + egui::vec2(4., 2.), g, fg);
    ui.interact(r, egui::Id::unique(salt), egui::Sense::click())
}

/// Contorno do preenchimento no 3D: bola em cada ponta aberta, folga em mm (fica como "ok" quando a
/// tolerância a fecha), etiqueta clicável de continuidade por curva e pontos de passagem arrastáveis.
fn contour_overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let Some(r) = app.result.as_ref() else { return };
    let (bad, good) = (crate::theme::st_err(), crate::theme::st_ok());
    let mut tags = vec![];
    if let Some(c) = r.notes.get(&id).and_then(|n| n.contour.as_ref()) {
        let painter = ui.painter_at(rect);
        for (i, p) in c.open_ends.iter().enumerate() {
            if let Some(s) = cam.project(rect, v3(*p)) {
                painter.circle_filled(s, 6., bad);
                if i == 0 {
                    tags.push((s + egui::vec2(0., -16.), format!("{} {}", c.open_ends.len(), tr("ponta(s) aberta(s)")), bad));
                }
            }
        }
        for g in &c.gaps {
            let m = fenkra_solidos::geo3d::lerp(g.a, g.b, 0.5);
            if let Some(s) = cam.project(rect, v3(m)) {
                tags.push((s + egui::vec2(0., 16.), format!("{} mm", fmt(g.d)), if g.closed { good } else { bad }));
            }
        }
    }
    for (i, (at, t, c)) in tags.into_iter().enumerate() {
        tag(ui, rect, at, t, c, ("folga_3d", id.0, i));
    }
    let Some(RefKind::SurfFill { curves, conds, points, .. }) = app.open.doc.reference(id).map(|x| x.kind.clone()) else { return };
    let mut cycle = None;
    for (i, c) in curves.iter().enumerate() {
        let Some(s) = curve_mid(r, c).and_then(|m| cam.project(rect, m)) else { continue };
        let cont = conds.get(i).map_or(Continuity::G0, |c| c.cont);
        let name = match cont {
            Continuity::G0 => "G0",
            Continuity::G1 => "G1",
            _ => "G2",
        };
        let fg = ui.visuals().strong_text_color();
        if tag(ui, rect, s, name.to_string(), fg, ("cont_3d", id.0, i)).on_hover_text(tr("Clique para trocar a continuidade desta curva.")).clicked() {
            cycle = Some(i);
        }
    }
    let mut moved = None;
    for (i, p) in points.iter().enumerate() {
        let PointRef::Fixed(q) = *p else { continue };
        let Some(s) = cam.project(rect, v3(q)) else { continue };
        let hr = ui.interact(Rect::from_center_size(s, egui::vec2(18., 18.)), egui::Id::unique(("passagem_3d", id.0, i)), egui::Sense::drag());
        ui.painter_at(rect).circle_stroke(s, 6., Stroke::new(2., ui.visuals().selection.stroke.color));
        if hr.dragged() {
            if let Some(m) = ui.input(|x| x.pointer.interact_pos()) {
                // o ponto corre no plano que passa por ele de frente para a câmera
                let (o, d) = cam.ray(rect, m);
                let (_, d0) = cam.ray(rect, s);
                let t = (v3(q) - o).dot(d0) / d.dot(d0).max(1e-6);
                let n = o + d * t;
                moved = Some((i, [n.x as f64, n.y as f64, n.z as f64]));
            }
        }
    }
    if cycle.is_none() && moved.is_none() {
        return;
    }
    if let Some(RefKind::SurfFill { curves, conds, points, .. }) = app.open.doc.references.iter_mut().find(|x| x.id == id).map(|x| &mut x.kind) {
        conds.resize(curves.len(), FillCond::default());
        if let Some(i) = cycle {
            conds[i].cont = match conds[i].cont {
                Continuity::G0 => Continuity::G1,
                Continuity::G1 => Continuity::G2,
                _ => Continuity::G0,
            };
        }
        if let Some((i, p)) = moved {
            points[i] = PointRef::Fixed(p);
        }
    }
}

pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: Camera) {
    let Some(id) = app.session.as_ref().map(|s| s.id) else { return };
    crate::sup3d::overlay(app, ui, rect, &cam, id);
    if crate::gizmo::overlay(app, ui, rect, &cam, id) {
        return;
    }
    // chapa: linha de dobra na face e alças de ângulo, raio, largura e recuos (B243 e B249)
    crate::chapa_manip_ui::overlay(app, ui, rect, &cam, id);
    // B253: o Membro tem as próprias alças (deslocamento, rotação com ímã de 45°, prolongar, espelhar)
    if let Some(FeatureKind::StructMember(m)) = app.open.doc.feature(id).map(|f| f.kind.clone()) {
        crate::st_esqueleto::overlay(app, ui, rect, &cam, &m);
        crate::st_alcas::overlay(app, ui, rect, &cam, id);
        return;
    }
    if matches!(app.open.doc.reference(id).map(|r| &r.kind), Some(RefKind::SurfFill { .. })) {
        contour_overlay(app, ui, rect, &cam, id);
        return;
    }
    if let Some(FeatureKind::Loft(l)) = app.open.doc.feature(id).map(|f| f.kind.clone()) {
        crate::loft_3d::overlay(app, ui, rect, &cam, id);
        let setas = app.result.as_ref().map(|r| crate::loft_3d::setas(r, &l)).unwrap_or_default();
        for (i, (campo, base, dir, mm, expr)) in setas.into_iter().enumerate() {
            linear_handle(app, ui, rect, &cam, id, campo, i, Linear { base, dir, k: 1. / mm, expr, label: "Intensidade" });
        }
        return;
    }
    if matches!(app.open.doc.feature(id).map(|f| &f.kind), Some(FeatureKind::Revolve(_))) {
        revolve_handle(app, ui, rect, &cam, id);
        return;
    }
    if let Some(RefKind::SurfRevolve { curve, axis, angle, .. }) = app.open.doc.reference(id).map(|r| r.kind.clone()) {
        let c = app.result.as_ref().and_then(|r| curve_mid(r, &curve));
        if let (Some(c), Some(ax)) = (c, revolve_axis(app, &axis)) {
            arc_handle(app, ui, rect, &cam, id, c, ax, &angle, false, 0, (1., 360.));
        }
        return;
    }
    let mut list = app.result.as_ref().map(|r| surface_handles(r, &app.open.doc, id)).unwrap_or_default();
    // B290: setas de tangência e de tensão
    if let Some(r) = app.result.as_ref() {
        list.extend(crate::sup_loft::setas(&app.open.doc, r, id).into_iter().map(|(campo, base, dir, mm, expr, label)| (campo, Linear { base, dir, k: 1. / mm, expr, label })));
    }
    if let (true, Some(h)) = (list.is_empty(), handle_for(app, id)) {
        list.push((0, h));
    }
    // B288: saída, dois lados, base das primitivas, mover e escala
    if let Some(r) = app.result.as_ref() {
        list.extend(solidos::alcas(r, &app.open.doc, id));
    }
    // B33: edição direta (mover tem uma seta por eixo; deslocar e raio, uma pela normal da face)
    list.extend(solidos::alcas_edicao(app, id));
    // B49: uma seta por ponto de controle do filete variável
    list.extend(solidos::alcas_filete(app, id));
    // B91: setas de espaçamento do padrão linear
    list.extend(padrao::alcas(app, id));
    // B289: início e fim do trecho parcial do filete e do chanfro
    list.extend(aresta::alcas_trecho(app, id));
    for (i, (slot, h)) in list.into_iter().enumerate() {
        linear_handle(app, ui, rect, &cam, id, slot, i, h);
    }
    solidos::arcos(app, ui, rect, &cam, id);
    // B91: ponto clicável por instância do padrão e arco do ângulo do circular
    padrao::overlay(app, ui, rect, &cam, id);
    // B289: seta que troca a face da primeira medida
    aresta::inverter(app, ui, rect, &cam, id);
    // B90: furo-guia que segue o cursor sobre a face
    furo::guia(app, ui, rect, &cam, id);
    // B89: o término "até" encaixa na face sob o cursor
    solidos::seta_ate_face(app, ui, rect, &cam, id);
    // B275: o plano de referência do comando (espelho, saída, base) tem a seta do próprio deslocamento
    if let Some(rid) = plano_deslocavel(&app.open.doc, id) {
        if let Some(h) = handle_for(app, rid) {
            linear_handle(app, ui, rect, &cam, rid, 0, 7, h);
        }
    }
}

/// O comando usa um plano deslocado (referência numérica)? Então arrastar a seta dele muda o valor.
fn plano_deslocavel(doc: &Document, cmd: Id) -> Option<Id> {
    let (p, _) = crate::previa::plano_do_comando(&doc.feature(cmd)?.kind)?;
    match p {
        PlaneRef::Datum(rid) if matches!(doc.reference(*rid).map(|r| &r.kind), Some(RefKind::OffsetPlane { .. })) => Some(*rid),
        _ => None,
    }
}

/// Uma alça linear: seta arrastável com a cota editável na ponta.
#[allow(clippy::too_many_arguments)]
fn linear_handle(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id, slot: u8, nth: usize, h: Linear) {
    let vars = app.vars();
    let Ok(val) = fenkra_core::expr::eval_str(&h.expr, &vars) else { return };
    let numeric = h.expr.trim().replace(',', ".").parse::<f64>().is_ok();
    let len = val as f32 / h.k;
    let tip = h.base + h.dir * len;
    let (Some(sb), Some(st)) = (cam.project(rect, h.base), cam.project(rect, tip)) else { return };
    // escala da tela: pixels por mm ao longo do eixo
    let Some(s1) = cam.project(rect, h.base + h.dir) else { return };
    let ppm = (s1 - sb).length().max(1e-3);
    let axis = (s1 - sb) / ppm;

    let painter = ui.painter_at(rect);
    let hid = egui::Id::unique(("alca_3d", id.0, nth));
    let hr = ui.interact(Rect::from_center_size(st, egui::vec2(PEGA, PEGA)), hid, if numeric { egui::Sense::click_and_drag() } else { egui::Sense::hover() });
    // B20: cinco estados (repouso, pairando, ativa, travada, erro) com a cor do tema
    let erro = matches!(app.result.as_ref().and_then(|r| r.status.get(&id)), Some(fenkra_solidos::Status::Error(_)));
    let estado = crate::alca_estado::EstadoAlca::de(hr.hovered(), hr.dragged(), !numeric, erro);
    let c = estado.cor(ui.visuals());
    if let Some(d) = estado.dica_traduzida() {
        let _ = hr.clone().on_hover_text(d);
    }
    let esp = estado.espessura();

    // haste e ponta
    painter.line_segment([sb, st], Stroke::new(esp + 1., c));
    let d = if (st - sb).length() > 1. { (st - sb).normalized() } else { axis };
    let n = egui::vec2(-d.y, d.x);
    painter.add(egui::Shape::convex_polygon(vec![st + d * 14., st + n * 7., st - n * 7.], c, Stroke::new(1., Color32::from_black_alpha(120))));
    painter.circle_filled(sb, 3.5, c);
    painter.circle_stroke(st, PEGA / 2., Stroke::new(esp, c));

    // captura do B17: o salto real desta seta, com o zoom real desta vista
    if std::env::var_os("FENKRA_DEMO_PASSO").is_some() {
        let p = crate::ui_ext::passo_3d(ui, ppm);
        painter.text(st + egui::vec2(16., 20.), egui::Align2::LEFT_TOP, format!("{} mm = {:.1} px", fmt(p), p * ppm as f64), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), ui.visuals().strong_text_color());
    }

    // arrastar ao longo do eixo
    if hr.dragged() {
        if let Some(m) = ui.input(|i| i.pointer.interact_pos()) {
            let t = (m - sb).dot(axis) / ppm;
            // passo segue o zoom (série 1-2-5, até 8 px) e as teclas do painel
            let step = (crate::ui_ext::passo_3d(ui, ppm) * h.k as f64) as f32;
            let mut v = ((t * h.k) / step).round() * step;
            if v.abs() < step {
                v = step * v.signum().max(1.);
            }
            write_slot(app, id, slot, fmt(v as f64));
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if hr.hovered() && numeric {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    // B14: a seta que passa sobre um elemento o propõe como limite
    if hr.dragged() || hr.drag_stopped() {
        limites::propor(app, ui, rect, id, slot, st, hr.drag_stopped());
    }

    // cota editável junto da ponta
    let key = egui::Id::unique(("alca_edit", id.0, nth));
    let mut editing: Option<String> = ui.ctx().data(|m| m.get_temp(key));
    let txt = if numeric && crate::sup_loft::sem_unidade(h.label) { format!("{} {}", tr(h.label), fmt(val)) } else if numeric { format!("{} {} mm", tr(h.label), fmt(val)) } else { format!("🔒 {} = {}", h.expr.trim(), fmt(val)) };
    let lp = st + n * 22. + d * 10.;
    let galley = painter.layout_no_wrap(txt, egui::FontId::proportional(crate::acessibilidade::fonte_3d(estado.fonte())), Color32::WHITE);
    let lr = Rect::from_min_size(lp, galley.size() + egui::vec2(10., 6.));
    painter.rect_filled(lr, 4., Color32::from_rgba_unmultiplied(20, 60, 110, 220));
    painter.rect_stroke(lr, 4., Stroke::new(1., c), egui::StrokeKind::Inside);
    painter.galley(lr.min + egui::vec2(5., 3.), galley, Color32::WHITE);
    // B246: com o valor que deixa a geometria inválida, a causa aparece junto da seta que o arrasta,
    // e a prévia anterior continua na tela
    if nth == 0 {
        if let Some(fenkra_solidos::Status::Error(msg)) = app.result.as_ref().and_then(|r| r.status.get(&id)) {
            crate::previa::etiqueta_de_erro(ui, rect, lr.left_bottom() + egui::vec2(0., 4.), msg);
        }
    }
    let lresp = ui.interact(lr, egui::Id::unique(("alca_cota", id.0, nth)), if numeric { egui::Sense::click_and_drag() } else { egui::Sense::click() });
    let lresp = lresp.on_hover_text(format!("{} · {}", h.label, tr("arraste a seta ou este rótulo, gire a roda, ou clique para digitar (aceita expressão). Shift ×10, Ctrl ×0,1, Alt ×0,01")));
    if numeric {
        // rótulo arrastável (horizontal, na escala da tela) e com roda; mesmo passo adaptativo da seta
        let passo = crate::ui_ext::passo_3d(ui, ppm) * h.k as f64;
        let dx = if lresp.dragged() { lresp.drag_delta().x as f64 } else { 0. };
        let dentes = crate::ui_ext::dentes_3d(ui, &lresp);
        if dx != 0. || dentes != 0. {
            let bruto = val * h.k as f64 + dx / ppm as f64 * h.k as f64 + dentes * passo;
            let mut v = (bruto / passo).round() * passo;
            if v.abs() < passo {
                v = passo * v.signum().max(1.);
            }
            write_slot(app, id, slot, fmt(v));
        }
        if lresp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
    }
    if lresp.double_clicked() || (lresp.clicked() && !lresp.dragged()) {
        editing = Some(h.expr.clone());
    }
    if let Some(mut text) = editing.take() {
        let mut done: Option<bool> = None;
        let mut desvincular = false;
        egui::Area::new(egui::Id::unique(("alca_area", id.0, nth))).fixed_pos(lr.left_bottom() + egui::vec2(0., 4.)).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(RichText::new(h.label).small());
                let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(110.));
                r.request_focus();
                let ok = fenkra_core::expr::eval_str(&text, &vars).is_ok();
                if !ok {
                    ui.label(RichText::new(tr("expressão inválida")).small().color(crate::theme::st_err()));
                }
                // valor ligado a parâmetro: o rótulo diz qual e deixa soltar o vínculo
                if !numeric && ui.small_button(tr("Desvincular")).on_hover_text(tr("Troca a expressão pelo valor de agora; a seta volta a arrastar.")).clicked() {
                    desvincular = true;
                }
                let (enter, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
                if enter && ok {
                    done = Some(true);
                } else if esc {
                    done = Some(false);
                }
            });
        });
        if desvincular {
            write_slot(app, id, slot, fmt(val));
            done = Some(false);
        }
        match done {
            Some(true) => {
                // a cota guarda o comprimento total; a seta do plano médio mostra metade
                write_slot(app, id, slot, text.trim().to_string());
                ui.ctx().data_mut(|m| m.remove::<String>(key));
            }
            Some(false) => {
                ui.ctx().data_mut(|m| m.remove::<String>(key));
            }
            None => {
                ui.ctx().data_mut(|m| m.insert_temp(key, text));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_kernel::Shape;
    use fenkra_solidos::{recompute, surface_brep};

    fn t(x: &str) -> String {
        x.into()
    }

    /// Superfície plana (linha extrudada) ou curva (arco extrudado), deslocada: confere se a ponta de
    /// cada seta cai exatamente sobre a superfície deslocada.
    fn confere(curva: RefKind, dist: &str, both: bool) {
        let mut d = Document::new("t");
        let c = d.add_geometry(curva);
        let f = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(c), dir: AxisRef::Origin(OriginAxis::Z), length: t("20"), length2: t("0"), until: None, until2: None });
        let o = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(f), dist: t(dist), both, join: OffsetJoin::Automatico });
        let r = recompute(&d);
        assert!(matches!(r.status[&o], fenkra_solidos::Status::Ok), "{:?}", r.status[&o]);
        let hs = surface_handles(&r, &d, o);
        assert_eq!(hs.len(), if both { 2 } else { 1 });
        let sh = Shape::from_brep(&surface_brep(&d, o).unwrap()).unwrap();
        let base = Shape::from_brep(&surface_brep(&d, f).unwrap()).unwrap();
        let v: f32 = dist.parse().unwrap();
        for (i, (slot, h)) in hs.iter().enumerate() {
            assert_eq!(*slot, 0);
            let tip = h.base + h.dir * v;
            let q = [tip.x as f64, tip.y as f64, tip.z as f64];
            assert!(fenkra_solidos::geo3d::dist(sh.surface_near(q).unwrap().0, q) < 1e-3, "seta {i} fora da superfície deslocada: {q:?}");
            // a base está na superfície original, não num centro de malha
            let b = [h.base.x as f64, h.base.y as f64, h.base.z as f64];
            assert!(fenkra_solidos::geo3d::dist(base.surface_near(b).unwrap().0, b) < 1e-3);
        }
        if both {
            assert!((hs[0].1.dir + hs[1].1.dir).length() < 1e-5);
        }
    }

    fn plana() -> RefKind {
        RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([40., 0., 0.]), start: t("0"), end: t("0") }
    }

    fn curva() -> RefKind {
        RefKind::Circle { center: PointRef::Origin, plane: PlaneRef::Origin(OriginPlane::XY), radius: t("30"), start: t("10"), end: t("120") }
    }

    #[test]
    fn seta_do_offset_em_superficie_plana() {
        confere(plana(), "5", false);
        confere(plana(), "-5", false);
        confere(plana(), "5", true);
    }

    #[test]
    fn seta_do_offset_em_superficie_curva() {
        confere(curva(), "5", false);
        confere(curva(), "-5", false);
        confere(curva(), "5", true);
    }

    #[test]
    fn valores_principais_ganham_alca() {
        let mut d = Document::new("t");
        let l = d.add_geometry(plana());
        let z = AxisRef::Origin(OriginAxis::Z);
        let ext = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(l), dir: z, length: t("20"), length2: t("5"), until: None, until2: None });
        let cil = d.add_geometry(RefKind::SurfCylinder { point: PointRef::Origin, dir: z, radius: t("9"), start: t("0"), end: t("20") });
        let tubo = d.add_geometry(RefKind::SurfSweep { profile: SweepProfile::Circle(t("2")), spine: CurveRef::Ref(l), mode: SweepMode::Corrigido });
        let pc = d.add_geometry(RefKind::PointOnCurve { curve: CurveRef::Ref(l), at: OnCurve::Ratio(t("0.5")), reverse: false });
        let ep = d.add_geometry(RefKind::SurfExtrapolate { surface: SurfRef::Ref(ext), near: PointRef::Origin, length: t("7"), curvature: false, more: vec![], edges: vec![CurveRef::Edge(vec![[0., 0., 20.], [40., 0., 20.]])], until: None, normal_edge: false, tangent_prop: false });
        let r = recompute(&d);
        let n = |x: Id| surface_handles(&r, &d, x);
        let e = n(ext);
        assert_eq!((e[0].0, e[1].0), (0, 1));
        assert!((e[0].1.dir + e[1].1.dir).length() < 1e-6, "limite 2 para o outro lado");
        assert_eq!(n(cil).iter().map(|h| h.1.label).collect::<Vec<_>>(), ["Altura", "Raio"]);
        assert_eq!(n(tubo)[0].1.label, "Raio");
        assert_eq!(n(ep)[0].1.expr, "7");
        // proporção: a seta de meia curva tem 20 mm, e 20 × k volta a 0,5
        let p = &n(pc)[0].1;
        assert!((p.k * 20. - 0.5).abs() < 1e-5);
    }
}
