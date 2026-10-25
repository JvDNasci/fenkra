//! Detalhamento 3D (PMI): criação pelas seleções da viewport, desenho sobre o modelo (linhas de
//! cota com setas, chamadas, símbolos) e painel de edição. Peça e montagem têm a própria lista.

use crate::app::{App, Sel};
use crate::i18n::tr;
use crate::picking::Pick;
use crate::pmi_draw::{Pen, HL};
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke};
use glam::Vec3;
use scc_core::pmi::*;
use scc_core::Id;
use scc_solidos::RecomputeResult;


/// Tipos na ordem dos comandos da barra.
pub fn kind_of(i: u8) -> PmiKind {
    match i {
        0 => PmiKind::Linear,
        1 => PmiKind::Radius,
        2 => PmiKind::Diameter,
        3 => PmiKind::Angle,
        4 => PmiKind::Note,
        5 => PmiKind::Roughness { ra: "1,6".into(), process: String::new() },
        6 => PmiKind::Weld { kind: WeldKind::Filete, size: "a4".into(), all_around: false, extra: Default::default() },
        7 => PmiKind::Datum { letter: "A".into() },
        9 => PmiKind::TitleBlock { template: TITLE_TEMPLATE.into() },
        _ => PmiKind::Gdt { ch: GdtChar::Planeza, tol: "0,05".into(), datums: String::new(), extra: Default::default() },
    }
}

/// Lista de PMI do nível atual (montagem aberta na bancada Montagem, senão a peça).
pub fn list(app: &App) -> &Vec<Pmi> {
    match app.asm.as_ref().filter(|_| app.in_asm()) {
        Some(st) => &st.asm.pmi,
        None => &app.open.doc.pmi,
    }
}

pub fn list_mut(app: &mut App) -> &mut Vec<Pmi> {
    if app.in_asm() {
        &mut app.asm.as_mut().unwrap().asm.pmi
    } else {
        &mut app.open.doc.pmi
    }
}

/// Âncoras a partir das seleções: vértice → posição; aresta → ponto médio (raio/diâmetro: centro
/// e um ponto do arco); face → ponto clicado.
fn anchors(app: &App, kind: &PmiKind, picks: &[Pick]) -> Option<Vec<[f64; 3]>> {
    let r = app.result.as_ref();
    let pt = |p: &Pick| -> Option<[f64; 3]> {
        Some(match *p {
            Pick::Vertex { pos, .. } => pos.map(|v| v as f64),
            Pick::Edge { mid, .. } => mid.map(|v| v as f64),
            Pick::Face { point, .. } => point.map(|v| v as f64),
            _ => return None,
        })
    };
    match kind {
        PmiKind::Radius | PmiKind::Diameter => {
            let Pick::Edge { body, edge, mid } = *picks.first()? else { return None };
            let pts = crate::picking::edge_polyline(r?, body, edge);
            let c = scc_solidos::geo3d::Curve3::poly(pts, false).ok()?;
            let (ctr, _, _) = scc_solidos::geo3d::fit_circle(&c).ok()?;
            Some(vec![ctr, mid.map(|v| v as f64)])
        }
        k => {
            let v: Vec<[f64; 3]> = picks.iter().filter_map(pt).collect();
            (v.len() >= k.anchors()).then(|| v[..k.anchors()].to_vec())
        }
    }
}

/// Cria a anotação com as seleções atuais; sem seleções suficientes, espera por elas.
pub fn command(app: &mut App, i: u8) {
    if i == 10 {
        auto_balloons(app);
        return;
    }
    let kind = kind_of(i);
    // legenda 3D: não precisa de seleção (fica no canto do modelo)
    if let PmiKind::TitleBlock { .. } = kind {
        let corner = app
            .result
            .as_ref()
            .and_then(|r| {
                let mut hi: Option<[f64; 3]> = None;
                for b in &r.bodies {
                    for v in b.mesh.positions.chunks_exact(3) {
                        let h = hi.get_or_insert([v[0] as f64, v[1] as f64, v[2] as f64]);
                        for k in 0..3 {
                            h[k] = h[k].max(v[k] as f64);
                        }
                    }
                }
                hi
            })
            .unwrap_or([0.; 3]);
        create(app, kind, vec![corner]);
        return;
    }
    let picks = app.picks.clone();
    match anchors(app, &kind, &picks) {
        Some(a) => create(app, kind, a),
        None => {
            app.pmi_pending = Some(i);
            let need = match &kind {
                PmiKind::Radius | PmiKind::Diameter => tr("uma aresta circular").to_string(),
                k => format!("{} {}", k.anchors(), tr("elemento(s): vértice, aresta ou face")),
            };
            app.toast(format!("{}: {} {need}", tr(kind.label()), tr("escolha")), false);
        }
    }
}

fn create(app: &mut App, kind: PmiKind, a: Vec<[f64; 3]>) {
    let id = Id(app.next_pmi_id());
    let n = list(app).iter().filter(|p| p.kind.label() == kind.label()).count() + 1;
    let name = format!("{} {n}", tr(kind.label()));
    let off = match kind {
        PmiKind::Linear | PmiKind::Angle => [0., 0., 12.],
        PmiKind::TitleBlock { .. } => [20., 0., 20.],
        _ => [15., 0., 15.],
    };
    list_mut(app).push(Pmi { id, name, kind, anchors: a, offset: off, text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new() });
    app.picks.clear();
    app.pmi_pending = None;
    if app.in_asm() {
        let st = app.asm.as_mut().unwrap();
        st.sel = Some(id);
        st.modified = true;
    } else {
        app.sel = Sel::Pmi(id);
    }
}

/// Balões 3D automáticos na montagem: um por componente (número de item fixo da lista de peças),
/// presos no alto de cada componente e afastados para fora do conjunto. Refaz os anteriores.
fn auto_balloons(app: &mut App) {
    if !app.in_asm() {
        app.toast(tr("Balões automáticos: abra uma montagem na bancada Montagem"), true);
        return;
    }
    let st = app.asm.as_mut().unwrap();
    let dir = st.dir();
    let rows = scc_montagem::bom::bom(&st.asm, dir.as_deref(), scc_montagem::bom::BomLevel::Topo);
    let nodes: Vec<(Id, String)> = st
        .asm
        .nodes
        .iter()
        .filter(|n| n.visible)
        .filter_map(|n| match &n.kind {
            scc_montagem::NodeKind::Part { path } | scc_montagem::NodeKind::SubAssembly { path } => Some((n.id, path.clone())),
            _ => None,
        })
        .collect();
    let mut seen: Vec<String> = vec![];
    let mut found: Vec<([f64; 3], String, f64)> = vec![];
    for (id, path) in nodes {
        if seen.contains(&path) {
            continue;
        }
        seen.push(path.clone());
        let Some(sh) = scc_montagem::view::placed_shape(&st.asm, id, dir.as_deref(), &mut st.cache) else { continue };
        let Some((l, h)) = sh.bbox() else { continue };
        let fp = scc_montagem::load::full_path(dir.as_deref(), &path);
        let item = rows.iter().find(|r| r.path == fp).map(|r| r.item.clone()).unwrap_or_else(|| seen.len().to_string());
        let size = ((h[0] - l[0]).powi(2) + (h[1] - l[1]).powi(2)).sqrt();
        found.push(([(l[0] + h[0]) / 2., (l[1] + h[1]) / 2., h[2]], item, size));
    }
    if found.is_empty() {
        app.toast(tr("A montagem não tem componentes visíveis"), true);
        return;
    }
    let n = found.len() as f64;
    let gc = [0, 1, 2].map(|k| found.iter().map(|f| f.0[k]).sum::<f64>() / n);
    st.asm.pmi.retain(|p| !matches!(p.kind, PmiKind::Balloon { .. }));
    st.modified = true;
    let count = found.len();
    for (c, item, size) in found {
        let d = [c[0] - gc[0], c[1] - gc[1]];
        let l = (d[0] * d[0] + d[1] * d[1]).sqrt();
        let u = if l > 1e-6 { [d[0] / l, d[1] / l] } else { [1., 0.] };
        let k = size * 0.35 + 25.;
        let id = Id(app.next_pmi_id());
        let pm = Pmi {
            id,
            name: format!("{} {item}", tr("Balão")),
            kind: PmiKind::Balloon { item },
            anchors: vec![c],
            offset: [u[0] * k, u[1] * k, 20.],
            text: String::new(),
            tol_plus: String::new(),
            tol_minus: String::new(),
            visible: true,
            plane: None,
            style: Default::default(),
            folder: tr("Balões").to_string(),
        };
        app.asm.as_mut().unwrap().asm.pmi.push(pm);
    }
    app.toast(format!("{count} {}", tr("balão(ões) criados com os números da lista de peças")), false);
}

/// Seleções feitas com uma anotação pendente.
pub fn after_viewport(app: &mut App) {
    let Some(i) = app.pmi_pending else { return };
    let kind = kind_of(i);
    let picks = app.picks.clone();
    if let Some(a) = anchors(app, &kind, &picks) {
        create(app, kind, a);
    }
}

/// Reencaixa a âncora no vértice ou meio de aresta mais próximo (até 2 mm).
fn snap(r: &RecomputeResult, p: [f64; 3]) -> [f64; 3] {
    let mut best = (2.0f64, p);
    for b in &r.bodies {
        for q in b.mesh.vertices.chunks_exact(3).chain(b.mesh.edge_mids.chunks_exact(3)) {
            let q = [q[0] as f64, q[1] as f64, q[2] as f64];
            let d = ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) + (q[2] - p[2]).powi(2)).sqrt();
            if d < best.0 {
                best = (d, q);
            }
        }
    }
    best.1
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Símbolo de solda ISO 2553 na tela: a junção da seta fica em `at`; `h` = altura do texto (px).
fn weld_draw(p: &egui::Painter, at: Pos2, kind: WeldKind, size: &str, all: bool, x: &WeldExtra, pen: &Pen) -> Rect {
    let h = pen.size * 1.05;
    let m = |q: [f64; 2]| at + egui::vec2(q[0] as f32, -q[1] as f32);
    let st = pen.stroke();
    let tc = if pen.sel { HL } else { pen.text };
    let mut r = Rect::from_center_size(at, egui::vec2(6., 6.));
    let mut shapes: Vec<egui::Shape> = vec![];
    for pr in weld_symbol(kind, size, all, x, h as f64) {
        match pr {
            WeldPrim::Line(v) | WeldPrim::Dashed(v) if v.len() < 2 => {}
            WeldPrim::Line(v) => {
                let pts: Vec<Pos2> = v.iter().map(|q| m(*q)).collect();
                pts.iter().for_each(|q| r.extend_with(*q));
                shapes.push(egui::Shape::line(pts, st));
            }
            WeldPrim::Dashed(v) => {
                let pts: Vec<Pos2> = v.iter().map(|q| m(*q)).collect();
                pts.iter().for_each(|q| r.extend_with(*q));
                shapes.extend(egui::Shape::dashed_line(&pts, st, 5., 3.));
            }
            WeldPrim::Fill(v) => {
                let pts: Vec<Pos2> = v.iter().map(|q| m(*q)).collect();
                pts.iter().for_each(|q| r.extend_with(*q));
                shapes.push(egui::Shape::convex_polygon(pts, st.color, Stroke::NONE));
            }
            WeldPrim::Text(q, t, al) => {
                let g = p.layout_no_wrap(t, pen.font(1.), tc);
                let pos = m(q);
                let x0 = match al {
                    -1 => pos.x - g.size().x,
                    0 => pos.x - g.size().x / 2.,
                    _ => pos.x,
                };
                let tl = egui::pos2(x0, pos.y - g.size().y * 0.8);
                r = r.union(Rect::from_min_size(tl, g.size()));
                shapes.push(egui::Shape::galley(tl, g, tc));
            }
        }
    }
    let r = r.expand(4.);
    match pen.bg {
        PmiBg::Branco => {
            p.rect_filled(r, 2., Color32::from_rgba_unmultiplied(255, 255, 255, 230));
        }
        PmiBg::Escuro => {
            p.rect_filled(r, 2., Color32::from_rgba_unmultiplied(40, 30, 22, 220));
        }
        _ => {}
    }
    p.extend(shapes);
    r
}

/// Campos da legenda 3D ({nome}, {codigo}, {descricao}, {material}, {massa}, {volume}, {data},
/// {autor}, mais as propriedades personalizadas pelo nome).
fn title_fields(app: &App) -> std::collections::HashMap<String, String> {
    let mut f = std::collections::HashMap::new();
    let mut put = |k: &str, v: String| {
        f.insert(k.to_string(), v);
    };
    put("data", scc_desenho::today());
    put("autor", std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default());
    let (name, props) = match app.asm.as_ref().filter(|_| app.in_asm()) {
        Some(st) => (st.asm.name.clone(), st.asm.props.clone()),
        None => (app.open.doc.name.clone(), app.open.doc.props.clone()),
    };
    put("nome", name.clone());
    put("codigo", props.pn.clone());
    put("descricao", if props.description.is_empty() { name } else { props.description.clone() });
    if app.in_asm() {
        put("material", "—".into());
        put("massa", "—".into());
        put("volume", "—".into());
    } else {
        let vol: f64 = app.result.as_ref().map(|r| r.bodies.iter().map(|b| b.mass.volume).sum()).unwrap_or(0.);
        let dens = crate::app::MATERIALS.iter().find(|(m, _)| *m == props.material).map(|(_, d)| *d).unwrap_or(7.85);
        put("material", props.material.clone());
        put("massa", format!("{:.3} kg", vol * dens * 1e-6).replace('.', ","));
        put("volume", format!("{:.1} cm³", vol / 1000.).replace('.', ","));
    }
    for (k, v) in &props.custom {
        put(&k.to_lowercase(), v.clone());
    }
    f
}

/// Realça o que a anotação marca: vértice (anel), aresta ou face sob cada âncora.
fn highlight(p: &egui::Painter, r: &RecomputeResult, cam: &Camera, rect: Rect, pts: &[[f64; 3]]) {
    let d = |v: &[f32], q: &[f64; 3]| ((v[0] as f64 - q[0]).powi(2) + (v[1] as f64 - q[1]).powi(2) + (v[2] as f64 - q[2]).powi(2)).sqrt();
    for q in pts {
        let mut done = false;
        for (bi, b) in r.bodies.iter().enumerate() {
            let m = &b.mesh;
            if m.vertices.chunks_exact(3).any(|v| d(v, q) < 1e-3) {
                if let Some(s) = cam.project(rect, v3(*q)) {
                    p.circle_stroke(s, 8., Stroke::new(2.5, HL));
                }
                done = true;
                break;
            }
            if let Some(e) = m.edge_mids.chunks_exact(3).position(|v| d(v, q) < 1e-3) {
                crate::picking::draw(p, r, cam, rect, &Pick::Edge { body: bi, edge: e, mid: q.map(|x| x as f32) }, true);
                done = true;
                break;
            }
        }
        if !done {
            for bi in 0..r.bodies.len() {
                if let Some(f) = crate::picking::face_at(r, bi, *q) {
                    crate::picking::draw(p, r, cam, rect, &Pick::Face { body: bi, face: f, owner: Id(0), point: q.map(|x| x as f32), normal: [0., 0., 1.] }, true);
                    break;
                }
            }
        }
    }
}

/// Rótulo deitado num plano: girado conforme a direção x do plano na tela e com altura de texto
/// em mm do modelo (acompanha o zoom). Devolve o retângulo (sem giro) para clique/arraste.
fn label_on_plane(p: &egui::Painter, cam: &Camera, rect: Rect, world: Vec3, x: Vec3, txt: &str, pen: &Pen) -> Option<Rect> {
    let at = cam.project(rect, world)?;
    let ax = cam.project(rect, world + x * 10.)?;
    let d = ax - at;
    let px_per_mm = d.length() / 10.;
    // no plano, a altura do texto é em mm do modelo (padrão 5 mm)
    let mm = if pen.size != 13. { pen.size * 0.4 } else { 5. };
    let size = (mm * px_per_mm).clamp(6., 80.);
    let angle = d.y.atan2(d.x);
    let g = p.layout_no_wrap(txt.to_string(), egui::FontId::new(size, pen.fam.clone()), pen.text);
    let (w, h) = (g.size().x + 10., g.size().y + 6.);
    let (c, s) = (angle.cos(), angle.sin());
    let rot = |v: egui::Vec2| egui::vec2(v.x * c - v.y * s, v.x * s + v.y * c);
    let corners: Vec<Pos2> = [(-w / 2., -h / 2.), (w / 2., -h / 2.), (w / 2., h / 2.), (-w / 2., h / 2.)].iter().map(|(a, b)| at + rot(egui::vec2(*a, *b))).collect();
    let fill = match pen.bg {
        PmiBg::Branco => Color32::from_rgba_unmultiplied(255, 255, 255, 220),
        PmiBg::Escuro => Color32::from_rgba_unmultiplied(40, 30, 22, 220),
        _ => Color32::TRANSPARENT,
    };
    let framed = matches!(pen.frame, PmiFrame::Caixa | PmiFrame::Arredondada);
    if framed || pen.sel || fill != Color32::TRANSPARENT {
        p.add(egui::Shape::convex_polygon(corners, fill, if framed || pen.sel { pen.stroke() } else { Stroke::NONE }));
    }
    let tl = at + rot(egui::vec2(-w / 2. + 5., -h / 2. + 3.));
    p.add(egui::epaint::TextShape::new(tl, g, pen.text).with_angle(angle));
    Some(Rect::from_center_size(at, egui::vec2(w.max(h), w.max(h)) * 0.8))
}

/// Desenha as anotações sobre a viewport (o clique num rótulo seleciona a anotação).
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let mut action: Option<(Id, u8)> = None;
    let mut drag: Option<(Id, [f64; 3])> = None;
    let Some(r) = app.result.as_ref() else { return };
    let sel = if app.in_asm() { app.asm.as_ref().and_then(|s| s.sel) } else if let Sel::Pmi(i) = app.sel { Some(i) } else { None };
    let painter = ui.painter_at(rect);
    let mut clicked = None;
    let fields = title_fields(app);
    let gstyle = app.settings.pmi_style.clone();
    for pm in list(app).iter().filter(|p| p.visible) {
        let a: Vec<[f64; 3]> = pm.anchors.iter().map(|p| snap(r, *p)).collect();
        let Some(first) = a.first().copied() else { continue };
        let val = pm.measure(&a);
        let is_sel = sel == Some(pm.id);
        let pen = Pen::new(&pm.style, &gstyle, is_sel);
        let (stroke, thin) = (pen.stroke(), pen.thin());
        let wrap = |t: String| format!("{}{}{}", pm.style.prefix, t, pm.style.suffix);
        let (main, stack) = pm.display_parts(val);
        let main = wrap(main);
        let off = v3(pm.offset);
        // a geometria que a anotação marca fica realçada enquanto ela está selecionada
        if is_sel && !matches!(pm.kind, PmiKind::TitleBlock { .. }) {
            highlight(&painter, r, cam, rect, &a);
        }
        // texto deitado num plano (cotas, notas, legenda) ou de frente para a tela
        let text_at = |at: Pos2, world: Vec3, txt: &str| -> Rect {
            match pm.plane {
                Some((_, x)) => label_on_plane(&painter, cam, rect, world, v3(x).normalize_or_zero(), txt, &pen).unwrap_or_else(|| pen.dim_text(&painter, at, txt, None)),
                None => pen.dim_text(&painter, at, txt, stack.as_ref()),
            }
        };
        let rect_lbl = match &pm.kind {
            PmiKind::Linear if a.len() >= 2 => {
                // linhas de chamada e linha de cota afastadas pelo deslocamento
                let (p0, p1) = (v3(a[0]), v3(a[1]));
                let (q0, q1) = (p0 + off, p1 + off);
                let (Some(s0), Some(s1), Some(t0), Some(t1)) = (cam.project(rect, p0), cam.project(rect, p1), cam.project(rect, q0), cam.project(rect, q1)) else { continue };
                // as linhas de chamada passam um pouco da linha de cota
                let ext = |s: Pos2, t: Pos2| t + (t - s).normalized() * 4.;
                painter.line_segment([s0 + (t0 - s0).normalized() * 3., ext(s0, t0)], thin);
                painter.line_segment([s1 + (t1 - s1).normalized() * 3., ext(s1, t1)], thin);
                painter.line_segment([t0, t1], stroke);
                if (t1 - t0).length() > 4. {
                    pen.arrow(&painter, t0, t1);
                    pen.arrow(&painter, t1, t0);
                }
                let mid = t0 + (t1 - t0) * 0.5;
                let up = egui::vec2((t1 - t0).y, -(t1 - t0).x).normalized();
                let up = if up.y > 0. { -up } else { up };
                text_at(mid + up * (pen.size * 0.9), (q0 + q1) * 0.5, &main)
            }
            PmiKind::Angle if a.len() >= 3 => {
                let (Some(sv), Some(s1), Some(s2)) = (cam.project(rect, v3(a[0])), cam.project(rect, v3(a[1])), cam.project(rect, v3(a[2]))) else { continue };
                painter.line_segment([sv, s1], thin);
                painter.line_segment([sv, s2], thin);
                let (d1, d2) = ((s1 - sv).normalized(), (s2 - sv).normalized());
                let rad = 34.;
                let pts: Vec<Pos2> = (0..=20).map(|k| {
                    let t = k as f32 / 20.;
                    sv + (d1 * (1. - t) + d2 * t).normalized() * rad
                }).collect();
                pen.arrow(&painter, pts[0], pts[2]);
                pen.arrow(&painter, pts[20], pts[18]);
                painter.add(egui::Shape::line(pts, stroke));
                text_at(sv + (d1 + d2).normalized() * (rad + pen.size * 1.4), v3(a[0]), &main)
            }
            PmiKind::Radius | PmiKind::Diameter => {
                // chamada do texto até a borda (seta na borda), passando pela direção do centro
                let (Some(sc), Some(st)) = (cam.project(rect, v3(first)), cam.project(rect, v3(first) + off)) else { continue };
                let se = a.get(1).and_then(|q| cam.project(rect, v3(*q))).unwrap_or(sc);
                let lr = text_at(st, v3(first) + off, &main);
                let (att, land) = Pen::attach(lr, se);
                painter.line_segment([att, land], stroke);
                painter.line_segment([land, se], stroke);
                pen.arrow(&painter, se, land);
                if matches!(pm.kind, PmiKind::Diameter) {
                    // diâmetro: a linha atravessa o centro até o outro lado
                    let other = sc + (sc - se);
                    painter.line_segment([se, other], thin);
                    pen.arrow(&painter, other, sc);
                }
                lr
            }
            PmiKind::Weld { kind, size, all_around, extra } => {
                // seta até a junta; linha de referência e símbolo ISO 2553 no rótulo
                let (Some(s0), Some(st)) = (cam.project(rect, v3(first)), cam.project(rect, v3(first) + off)) else { continue };
                painter.line_segment([st, s0], stroke);
                pen.arrow(&painter, s0, st);
                weld_draw(&painter, st, *kind, size, *all_around, extra, &pen)
            }
            PmiKind::Balloon { item } => {
                let (Some(s0), Some(st)) = (cam.project(rect, v3(first)), cam.project(rect, v3(first) + off)) else { continue };
                let dir = (s0 - st).normalized();
                let rad = pen.size + 3.5 * item.chars().count().saturating_sub(1) as f32;
                painter.line_segment([st + dir * rad, s0], stroke);
                painter.circle_filled(s0, pen.size * 0.22, if is_sel { HL } else { pen.line });
                pen.balloon(&painter, st, item).0
            }
            PmiKind::TitleBlock { template } => {
                // legenda: tabela sem chamada
                let txt = scc_desenho::resolve(template, &fields);
                let Some(st) = cam.project(rect, v3(first) + off) else { continue };
                match pm.plane {
                    Some((_, x)) => label_on_plane(&painter, cam, rect, v3(first) + off, v3(x).normalize_or_zero(), &txt, &pen).unwrap_or_else(|| pen.table(&painter, st, &txt)),
                    None => pen.table(&painter, st, &txt),
                }
            }
            PmiKind::Gdt { ch, tol, datums, extra } => {
                let (Some(s0), Some(st)) = (cam.project(rect, v3(first)), cam.project(rect, v3(first) + off)) else { continue };
                let cell_tol = |t: &str| format!("{}{}{}", if extra.zone_dia { "Ø" } else { "" }, t.trim(), if extra.modifier.trim().is_empty() { String::new() } else { format!(" {}", extra.modifier.trim()) });
                let row = |t: &str, d: &str| -> Vec<String> {
                    let mut v = vec![cell_tol(t)];
                    v.extend(d.split(|c: char| c == '|' || c == ',' || c.is_whitespace()).filter(|x| !x.is_empty()).map(String::from));
                    v
                };
                let mut rows = vec![row(tol, datums)];
                if let Some((t2, d2)) = &extra.second {
                    rows.push(row(t2, d2));
                }
                let fr = pen.fcf(&painter, st, ch.symbol(), &rows);
                let (att, land) = Pen::attach(fr, s0);
                painter.line_segment([att, land], stroke);
                painter.line_segment([land, s0], stroke);
                pen.arrow(&painter, s0, land);
                fr
            }
            PmiKind::Datum { letter } => {
                let (Some(s0), Some(st)) = (cam.project(rect, v3(first)), cam.project(rect, v3(first) + off)) else { continue };
                pen.datum(&painter, s0, st, letter)
            }
            PmiKind::Roughness { ra, process } => {
                let (Some(s0), Some(st)) = (cam.project(rect, v3(first)), cam.project(rect, v3(first) + off)) else { continue };
                let txt = if process.trim().is_empty() { format!("Ra {ra}") } else { format!("{}\nRa {ra}", process.trim()) };
                let (rr, tip) = pen.roughness(&painter, st, &txt);
                painter.line_segment([tip, s0], stroke);
                pen.arrow(&painter, s0, tip);
                rr
            }
            _ => {
                // nota: chamada com seta até a peça e patamar no texto
                let (Some(s0), Some(st)) = (cam.project(rect, v3(first)), cam.project(rect, v3(first) + off)) else { continue };
                let txt = wrap(pm.text.clone());
                let lr = match pm.plane {
                    Some((_, x)) => label_on_plane(&painter, cam, rect, v3(first) + off, v3(x).normalize_or_zero(), &txt, &pen).unwrap_or_else(|| pen.text_block(&painter, st, &txt, PmiFrame::Nenhuma, PmiBg::Nenhum)),
                    None => pen.text_block(&painter, st, &txt, PmiFrame::Nenhuma, PmiBg::Nenhum),
                };
                let (att, land) = Pen::attach(lr, s0);
                painter.line_segment([att, land], stroke);
                painter.line_segment([land, s0], stroke);
                pen.arrow(&painter, s0, land);
                lr
            }
        };
        let resp = ui.interact(rect_lbl, egui::Id::unique(("pmi_rotulo", pm.id.0)), egui::Sense::click_and_drag());
        // arrastar o rótulo: move o texto no plano da anotação (ou de frente para a tela)
        if resp.dragged() {
            if let Some(m) = resp.interact_pointer_pos() {
                let base = v3(first) + off;
                let n = match pm.plane {
                    Some((n, _)) => v3(n).normalize_or_zero(),
                    None => -cam.ray(rect, rect.center()).1,
                };
                let (o, dir) = cam.ray(rect, m);
                let den = dir.dot(n);
                if den.abs() > 1e-6 {
                    let t = (base - o).dot(n) / den;
                    let hit = o + dir * t;
                    let new_off = hit - v3(first);
                    drag = Some((pm.id, [new_off.x as f64, new_off.y as f64, new_off.z as f64]));
                }
            }
        }
        if resp.clicked() || resp.secondary_clicked() {
            clicked = Some(pm.id);
        }
        // botão direito no rótulo: opções da anotação
        resp.context_menu(|ui| {
            ui.label(RichText::new(&pm.name).strong());
            if ui.button(tr("✏ Editar")).clicked() {
                action = Some((pm.id, 2));
                ui.close();
            }
            if ui.button(tr("Ocultar")).clicked() {
                action = Some((pm.id, 1));
                ui.close();
            }
            if ui.button(tr(if pm.plane.is_some() { "🖵 De frente para a tela" } else { "▱ Deitar no plano da vista atual" })).clicked() {
                action = Some((pm.id, 3));
                ui.close();
            }
            ui.separator();
            if ui.button(tr("🗑 Apagar   Del")).clicked() {
                action = Some((pm.id, 0));
                ui.close();
            }
        });
        resp.on_hover_text(&pm.name);
    }
    if let Some((id, off)) = drag {
        if let Some(p) = list_mut(app).iter_mut().find(|p| p.id == id) {
            p.offset = off;
        }
        clicked = Some(id);
    }
    if let Some((id, k)) = action {
        match k {
            0 => {
                list_mut(app).retain(|p| p.id != id);
                clicked = None;
                if app.sel == Sel::Pmi(id) {
                    app.sel = Sel::None;
                }
            }
            1 => {
                if let Some(p) = list_mut(app).iter_mut().find(|p| p.id == id) {
                    p.visible = false;
                }
            }
            3 => {
                // alterna: de frente ↔ deitado no plano da tela atual (fica fixo nele ao girar)
                let (o, d) = cam.ray(rect, rect.center());
                let (_, dx) = cam.ray(rect, rect.center() + egui::vec2(10., 0.));
                let n = -d;
                let x = (dx - d).normalize_or_zero();
                let _ = o;
                if let Some(p) = list_mut(app).iter_mut().find(|p| p.id == id) {
                    p.plane = match p.plane {
                        Some(_) => None,
                        None => Some(([n.x as f64, n.y as f64, n.z as f64], [x.x as f64, x.y as f64, x.z as f64])),
                    };
                }
            }
            _ => clicked = Some(id),
        }
    }
    if let Some(id) = clicked {
        if app.in_asm() {
            app.asm.as_mut().unwrap().sel = Some(id);
        } else {
            app.sel = Sel::Pmi(id);
        }
    }
}

/// Capturas do nível atual (peça ou montagem).
fn captures(app: &App) -> &Vec<PmiCapture> {
    match app.asm.as_ref().filter(|_| app.in_asm()) {
        Some(st) => &st.asm.pmi_captures,
        None => &app.open.doc.pmi_captures,
    }
}

fn captures_mut(app: &mut App) -> &mut Vec<PmiCapture> {
    if app.in_asm() {
        &mut app.asm.as_mut().unwrap().asm.pmi_captures
    } else {
        &mut app.open.doc.pmi_captures
    }
}

/// O que está oculto no modelo agora: comandos, coletores e referências (peça) ou componentes.
fn hidden_now(app: &App) -> Vec<Id> {
    match app.asm.as_ref().filter(|_| app.in_asm()) {
        Some(st) => st.asm.nodes.iter().filter(|n| !n.visible).map(|n| n.id).collect(),
        None => {
            let d = &app.open.doc;
            let mut v: Vec<Id> = d.features.iter().filter(|f| f.hidden).map(|f| f.id).collect();
            v.extend(d.collectors.iter().filter(|c| !c.visible).map(|c| c.id));
            v.extend(d.references.iter().filter(|r| !r.visible).map(|r| r.id));
            v
        }
    }
}

/// Uma linha de anotação na árvore (com o menu de pastas).
#[allow(clippy::too_many_arguments)]
fn item_row(ui: &mut egui::Ui, p: &Pmi, sel: Option<Id>, folders: &[String], select: &mut Option<Id>, action: &mut Option<(Id, u8)>, move_to: &mut Option<(Id, String)>) {
    let txt = format!("{} {}", if p.visible { "📐" } else { "·" }, p.name);
    let r = ui.selectable_label(sel == Some(p.id), if p.visible { RichText::new(txt) } else { RichText::new(txt).weak() });
    if r.clicked() {
        *select = Some(p.id);
    }
    r.context_menu(|ui| {
        ui.label(RichText::new(&p.name).strong());
        if ui.button(tr("✏ Editar")).clicked() {
            *select = Some(p.id);
            ui.close();
        }
        if ui.button(if p.visible { tr("Ocultar") } else { tr("Mostrar") }).clicked() {
            *action = Some((p.id, 1));
            ui.close();
        }
        ui.menu_button(tr("📁 Mover para pasta"), |ui| {
            if ui.button(tr("(raiz)")).clicked() {
                *move_to = Some((p.id, String::new()));
                ui.close();
            }
            for f in folders {
                if ui.button(f).clicked() {
                    *move_to = Some((p.id, f.clone()));
                    ui.close();
                }
            }
            if ui.button(tr("＋ Nova pasta")).clicked() {
                let mut n = folders.len() + 1;
                while folders.contains(&format!("{} {n}", tr("Pasta"))) {
                    n += 1;
                }
                *move_to = Some((p.id, format!("{} {n}", tr("Pasta"))));
                ui.close();
            }
        });
        ui.separator();
        if ui.button(tr("🗑 Apagar")).clicked() {
            *action = Some((p.id, 0));
            ui.close();
        }
    });
}

/// Seção "PMI" na árvore (peça ou montagem): anotações (com pastas) e, à parte, as capturas.
pub fn tree_section(app: &mut App, ui: &mut egui::Ui) {
    let n = list(app).len();
    let mut select = None;
    let mut action: Option<(Id, u8)> = None;
    let mut move_to: Option<(Id, String)> = None;
    let mut folder_act: Option<(String, u8)> = None;
    let sel = if app.in_asm() { app.asm.as_ref().and_then(|s| s.sel) } else if let Sel::Pmi(i) = app.sel { Some(i) } else { None };
    let mut create: Option<u8> = None;
    let mut folders: Vec<String> = list(app).iter().map(|p| p.folder.clone()).filter(|f| !f.is_empty()).collect();
    folders.sort();
    folders.dedup();
    let h = egui::CollapsingHeader::new(format!("📐 {} ({n})", tr("PMI — detalhamento 3D"))).id_salt("arvore_pmi").default_open(n > 0).show(ui, |ui| {
        if n == 0 {
            ui.label(RichText::new(tr("Cotas, notas, rugosidade, solda e tolerâncias no modelo 3D (grupo Detalhamento 3D).")).weak().small());
        }
        for f in &folders {
            let r = egui::CollapsingHeader::new(format!("📁 {f}")).id_salt(("pmi_pasta", f.as_str())).default_open(true).show(ui, |ui| {
                for p in list(app).iter().filter(|p| p.folder == *f) {
                    item_row(ui, p, sel, &folders, &mut select, &mut action, &mut move_to);
                }
            });
            r.header_response.context_menu(|ui| {
                if ui.button(tr("👁 Mostrar a pasta")).clicked() {
                    folder_act = Some((f.clone(), 2));
                    ui.close();
                }
                if ui.button(tr("Ocultar a pasta")).clicked() {
                    folder_act = Some((f.clone(), 3));
                    ui.close();
                }
                if ui.button(tr("Desfazer a pasta (anotações vão para a raiz)")).clicked() {
                    folder_act = Some((f.clone(), 0));
                    ui.close();
                }
            });
        }
        for p in list(app).iter().filter(|p| p.folder.is_empty()) {
            item_row(ui, p, sel, &folders, &mut select, &mut action, &mut move_to);
        }
    });
    h.header_response.context_menu(|ui| {
        ui.label(RichText::new(tr("Nova anotação 3D")).strong());
        for (i, n) in [(0u8, "↔ Cota linear"), (1, "R Raio"), (2, "Ø Diâmetro"), (3, "∠ Ângulo"), (4, "✎ Nota"), (5, "√ Rugosidade"), (6, "⌒ Solda"), (7, "▭ Referência (datum)"), (8, "⊞ Tolerância geométrica"), (9, "▤ Legenda 3D")] {
            if ui.button(tr(n)).clicked() {
                create = Some(i);
                ui.close();
            }
        }
        if ui.button(tr("🎨 Estilo geral das anotações…")).clicked() {
            app.pmi_style_open = true;
            ui.close();
        }
        if app.in_asm() && ui.button(tr("① Balões automáticos")).clicked() {
            create = Some(10);
            ui.close();
        }
        ui.separator();
        if ui.button(tr("👁 Mostrar todas")).clicked() {
            action = Some((Id(0), 2));
            ui.close();
        }
        if ui.button(tr("Ocultar todas")).clicked() {
            action = Some((Id(0), 3));
            ui.close();
        }
    });
    // capturas: vista (câmera e zoom) + anotações visíveis (+ visibilidade do modelo, se pedida)
    let mut cap_act: Option<(usize, u8)> = None;
    let caps: Vec<(String, bool)> = captures(app).iter().map(|c| (c.name.clone(), c.with_model)).collect();
    let hc = egui::CollapsingHeader::new(format!("📷 {} ({})", tr("Capturas"), caps.len())).id_salt("arvore_capturas").default_open(!caps.is_empty()).show(ui, |ui| {
        if caps.is_empty() {
            ui.label(RichText::new(tr("Botão direito aqui: capturar a vista atual com as anotações visíveis (e, se quiser, o que está oculto no modelo).")).weak().small());
        }
        for (i, (n, wm)) in caps.iter().enumerate() {
            let r = ui.selectable_label(false, format!("📷 {n}{}", if *wm { " ◐" } else { "" })).on_hover_text(if *wm { tr("aplicar: câmera, anotações e visibilidade do modelo") } else { tr("aplicar: câmera e anotações desta captura") });
            if r.clicked() {
                cap_act = Some((i, 0));
            }
            r.context_menu(|ui| {
                if ui.button(tr("📷 Atualizar com a vista atual")).clicked() {
                    cap_act = Some((i, 1));
                    ui.close();
                }
                if ui.button(if *wm { tr("Não guardar a visibilidade do modelo") } else { tr("Guardar também a visibilidade do modelo") }).clicked() {
                    cap_act = Some((i, 4));
                    ui.close();
                }
                if ui.button(tr("🗑 Apagar captura")).clicked() {
                    cap_act = Some((i, 2));
                    ui.close();
                }
            });
        }
    });
    hc.header_response.context_menu(|ui| {
        if ui.button(tr("📷 Capturar: câmera + anotações")).clicked() {
            cap_act = Some((usize::MAX, 1));
            ui.close();
        }
        if ui.button(tr("📷 Capturar: câmera + anotações + visibilidade do modelo")).clicked() {
            cap_act = Some((usize::MAX, 3));
            ui.close();
        }
    });
    if let Some(id) = select {
        if app.in_asm() {
            app.asm.as_mut().unwrap().sel = Some(id);
        } else {
            app.sel = Sel::Pmi(id);
        }
    }
    if let Some(i) = create {
        command(app, i);
    }
    if let Some((i, k)) = cap_act {
        let view = app.camera_get().map(|c| scc_core::NamedView { name: String::new(), target: c.target, yaw: c.yaw, pitch: c.pitch, dist: c.dist, ortho: c.ortho });
        let shown: Vec<Id> = list(app).iter().filter(|p| p.visible).map(|p| p.id).collect();
        let hidden = hidden_now(app);
        match k {
            0 => {
                if let Some(c) = captures(app).get(i).cloned() {
                    for p in list_mut(app) {
                        p.visible = c.shown.contains(&p.id);
                    }
                    if c.with_model {
                        if app.in_asm() {
                            let st = app.asm.as_mut().unwrap();
                            for n in &mut st.asm.nodes {
                                n.visible = !c.hidden.contains(&n.id);
                            }
                            st.modified = true;
                        } else {
                            let d = &mut app.open.doc;
                            for f in &mut d.features {
                                f.hidden = c.hidden.contains(&f.id);
                            }
                            for col in &mut d.collectors {
                                col.visible = !c.hidden.contains(&col.id);
                            }
                            for r in &mut d.references {
                                r.visible = !c.hidden.contains(&r.id);
                            }
                        }
                        app.scene_dirty = true;
                    }
                    app.apply_view(&c.view);
                }
            }
            1 | 3 => {
                if let Some(v) = view {
                    if i == usize::MAX {
                        let id = Id(app.next_pmi_id());
                        let n = captures(app).len() + 1;
                        captures_mut(app).push(PmiCapture { id, name: format!("{} {n}", tr("Captura")), view: v, shown, hidden, with_model: k == 3 });
                    } else if let Some(c) = captures_mut(app).get_mut(i) {
                        c.view = v;
                        c.shown = shown;
                        c.hidden = hidden;
                    }
                }
            }
            4 => {
                if let Some(c) = captures_mut(app).get_mut(i) {
                    c.with_model = !c.with_model;
                    c.hidden = hidden;
                }
            }
            _ => {
                if i < captures(app).len() {
                    captures_mut(app).remove(i);
                }
            }
        }
        if app.in_asm() {
            app.asm.as_mut().unwrap().modified = true;
        }
    }
    if let Some((id, f)) = move_to {
        if let Some(p) = list_mut(app).iter_mut().find(|p| p.id == id) {
            p.folder = f;
        }
    }
    if let Some((f, k)) = folder_act {
        for p in list_mut(app).iter_mut().filter(|p| p.folder == f) {
            match k {
                0 => p.folder.clear(),
                _ => p.visible = k == 2,
            }
        }
    }
    if let Some((id, k)) = action {
        let l = list_mut(app);
        match k {
            0 => l.retain(|p| p.id != id),
            2 | 3 => l.iter_mut().for_each(|p| p.visible = k == 2),
            _ => {
                if let Some(p) = l.iter_mut().find(|p| p.id == id) {
                    p.visible = !p.visible;
                }
            }
        }
    }
}

/// Campos do símbolo de solda ISO 2553 (usado no PMI e no desenho).
pub fn weld_ui(ui: &mut egui::Ui, salt: &str, kind: &mut WeldKind, size: &mut String, all_around: &mut bool, extra: &mut WeldExtra) {
    let pick = |ui: &mut egui::Ui, id: &str, k: &mut WeldKind| {
        egui::ComboBox::from_id_salt((salt, id)).selected_text(format!("{} {}", k.symbol(), tr(k.label()))).width(180.).show_ui(ui, |ui| {
            for w in WeldKind::ALL {
                ui.selectable_value(k, w, format!("{} {}", w.symbol(), tr(w.label())));
            }
        });
    };
    ui.horizontal(|ui| {
        ui.label(tr("Lado da seta"));
        pick(ui, "seta", kind);
    });
    let mut both = extra.other.is_some();
    ui.horizontal(|ui| {
        if ui.checkbox(&mut both, tr("dos dois lados")).changed() {
            extra.other = both.then_some(*kind);
        }
        if let Some(o) = &mut extra.other {
            pick(ui, "outro", o);
        } else {
            ui.checkbox(&mut extra.other_side, tr("só do outro lado (na linha tracejada)"));
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("Medida"));
        ui.add(egui::TextEdit::singleline(size).desired_width(60.).hint_text("a5"));
        ui.label(tr("Comprimento"));
        ui.add(egui::TextEdit::singleline(&mut extra.length).desired_width(90.).hint_text("3×50 (100)"));
    });
    ui.horizontal(|ui| {
        ui.label(tr("Acabamento"));
        for (c, n) in [(WeldContour::Nenhum, "nenhum"), (WeldContour::Plano, "plano"), (WeldContour::Convexo, "convexo"), (WeldContour::Concavo, "côncavo")] {
            ui.radio_value(&mut extra.contour, c, tr(n));
        }
    });
    ui.horizontal(|ui| {
        ui.checkbox(all_around, tr("todo o contorno"));
        ui.checkbox(&mut extra.field, tr("solda de campo"));
    });
    ui.horizontal(|ui| {
        ui.label(tr("Cauda (processo)"));
        ui.add(egui::TextEdit::singleline(&mut extra.process).desired_width(120.).hint_text("135 / ISO 5817-C"));
    });
    ui.label(RichText::new(tr("ISO 2553, sistema A: símbolo na linha cheia = lado da seta; na tracejada = outro lado. Medida da seção à esquerda (a = garganta, z = perna), comprimento à direita, processo ISO 4063 na cauda.")).weak().small());
}

/// Painel de edição da anotação selecionada. Devolve `true` se havia uma anotação.
pub fn panel(app: &mut App, ui: &mut egui::Ui, id: Id) -> bool {
    let mut style_req = false;
    let r = panel_inner(app, ui, id, &mut style_req);
    if style_req {
        app.pmi_style_open = true;
    }
    r
}

/// Janela do estilo geral das anotações (vale para todas as que deixam "padrão").
pub fn style_window(app: &mut App, ctx: &egui::Context) {
    if !app.pmi_style_open {
        return;
    }
    let mut open = true;
    let mut done = false;
    egui::Window::new(tr("🎨 Estilo geral das anotações 3D")).open(&mut open).default_width(380.).show(ctx, |ui| {
        done = crate::app::done_bar(ui);
        ui.label(RichText::new(tr("Vale para todas as anotações que deixam o item em 'padrão'. Cada anotação pode trocar o que quiser no próprio painel.")).weak().small());
        crate::pmi_draw::style_ui(ui, &mut app.settings.pmi_style, false);
        ui.horizontal(|ui| {
            if ui.button(tr("Técnico (azul-escuro, sem fundo)")).clicked() {
                app.settings.pmi_style = PmiStyle { font: "Arial".into(), ..Default::default() };
            }
            if ui.button(tr("Alto contraste (fundo branco)")).clicked() {
                app.settings.pmi_style = PmiStyle { font: "Arial".into(), color: Some([0, 0, 0]), text_color: Some([0, 0, 0]), bg: PmiBg::Branco, ..Default::default() };
            }
            if ui.button(tr("Laranja (antigo)")).clicked() {
                app.settings.pmi_style = PmiStyle { color: Some([255, 150, 60]), text_color: Some([255, 255, 255]), bg: PmiBg::Escuro, frame: PmiFrame::Caixa, ..Default::default() };
            }
        });
    });
    if !open || done {
        app.pmi_style_open = false;
    }
}

fn panel_inner(app: &mut App, ui: &mut egui::Ui, id: Id, style_req: &mut bool) -> bool {
    let seed_pts: Vec<[f64; 3]> = app.picks.iter().filter_map(|p| match *p {
        Pick::Vertex { pos, .. } => Some(pos.map(|v| v as f64)),
        Pick::Edge { mid, .. } => Some(mid.map(|v| v as f64)),
        Pick::Face { point, .. } => Some(point.map(|v| v as f64)),
        _ => None,
    }).collect();
    // plano da face/plano clicado (para deitar o texto nele)
    let seed_plane: Option<([f64; 3], [f64; 3])> = app.picks.iter().find_map(|p| match *p {
        Pick::Face { normal, .. } => {
            let n = Vec3::new(normal[0], normal[1], normal[2]).normalize_or_zero();
            let helper = if n.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
            let x = helper.cross(n).normalize_or_zero();
            Some(([n.x as f64, n.y as f64, n.z as f64], [x.x as f64, x.y as f64, x.z as f64]))
        }
        Pick::Plane(pr) => app.result.as_ref().and_then(|r| {
            r.planes.iter().find(|v| match pr {
                scc_core::PlaneRef::Datum(id) => v.id == Some(id),
                scc_core::PlaneRef::Origin(o) => v.id.is_none() && v.name == crate::scene::plane_name(o),
            })
        }).map(|v| (v.frame.normal, v.frame.x)),
        _ => None,
    });
    let l = list_mut(app);
    let Some(p) = l.iter_mut().find(|p| p.id == id) else { return false };
    ui.heading(format!("📐 {}", p.name));
    ui.label(RichText::new(tr(p.kind.label())).weak());
    ui.horizontal(|ui| {
        ui.label(tr("Nome"));
        ui.text_edit_singleline(&mut p.name);
    });
    match &mut p.kind {
        PmiKind::Note => {
            ui.label(tr("Texto"));
            ui.text_edit_multiline(&mut p.text);
        }
        PmiKind::Roughness { ra, process } => {
            ui.horizontal(|ui| {
                ui.label("Ra");
                ui.text_edit_singleline(ra);
            });
            ui.horizontal(|ui| {
                ui.label(tr("Processo"));
                ui.text_edit_singleline(process);
            });
        }
        PmiKind::Weld { kind, size, all_around, extra } => weld_ui(ui, "pmi", kind, size, all_around, extra),
        PmiKind::Datum { letter } => {
            ui.horizontal(|ui| {
                ui.label(tr("Letra"));
                ui.text_edit_singleline(letter);
            });
        }
        PmiKind::TitleBlock { template } => {
            ui.label(tr("Modelo (os campos entre chaves são preenchidos com os dados da peça)"));
            ui.add(egui::TextEdit::multiline(template).desired_rows(4).desired_width(f32::INFINITY));
            ui.horizontal_wrapped(|ui| {
                for f in ["{nome}", "{codigo}", "{descricao}", "{material}", "{massa}", "{volume}", "{data}", "{autor}"] {
                    if ui.small_button(f).on_hover_text(tr("inserir o campo")).clicked() {
                        template.push_str(f);
                    }
                }
            });
            if ui.small_button(tr("↺ Modelo padrão")).clicked() {
                *template = TITLE_TEMPLATE.into();
            }
            ui.label(RichText::new(tr("Propriedades personalizadas da peça também valem, pelo nome em minúsculas: {revisao}, {cliente}…")).weak().small());
        }
        PmiKind::Balloon { item } => {
            ui.horizontal(|ui| {
                ui.label(tr("Item"));
                ui.add(egui::TextEdit::singleline(item).desired_width(60.));
            });
        }
        PmiKind::Gdt { ch, tol, datums, extra } => {
            egui::ComboBox::from_id_salt("pmi_gdt").selected_text(format!("{} {}", ch.symbol(), tr(ch.label()))).show_ui(ui, |ui| {
                for c in GdtChar::ALL {
                    ui.selectable_value(ch, c, format!("{} {}", c.symbol(), tr(c.label())));
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("Tolerância"));
                ui.text_edit_singleline(tol);
            });
            ui.horizontal(|ui| {
                ui.label(tr("Referências"));
                ui.text_edit_singleline(datums);
            });
            ui.label(RichText::new(tr("Referências separadas por espaço (ex.: A B C); cada uma vira uma casa do quadro.")).weak().small());
            ui.checkbox(&mut extra.zone_dia, tr("zona cilíndrica (Ø antes do valor)"));
            ui.horizontal(|ui| {
                ui.label(tr("Modificador"));
                for m in ["", "Ⓜ", "Ⓛ", "Ⓟ", "Ⓕ", "Ⓢ", "Ⓣ", "Ⓤ"] {
                    let lbl = if m.is_empty() { tr("nenhum").to_string() } else { m.to_string() };
                    if ui.selectable_label(extra.modifier == m, lbl).clicked() {
                        extra.modifier = m.to_string();
                    }
                }
            })
            .response
            .on_hover_text(tr("Ⓜ máximo material, Ⓛ mínimo material, Ⓟ zona projetada, Ⓕ estado livre, Ⓢ independente, Ⓣ plano tangente, Ⓤ zona desigual"));
            let mut comp = extra.second.is_some();
            if ui.checkbox(&mut comp, tr("quadro composto (segunda linha)")).changed() {
                extra.second = comp.then(|| (String::new(), String::new()));
            }
            if let Some((t2, d2)) = &mut extra.second {
                ui.horizontal(|ui| {
                    ui.label(tr("Tolerância 2"));
                    ui.add(egui::TextEdit::singleline(t2).desired_width(70.));
                    ui.label(tr("Referências 2"));
                    ui.add(egui::TextEdit::singleline(d2).desired_width(70.));
                });
            }
        }
        _ => {
            ui.horizontal(|ui| {
                ui.label(tr("Texto (vazio = medido)"));
                ui.text_edit_singleline(&mut p.text);
            });
            ui.horizontal(|ui| {
                ui.label(tr("Tolerância +"));
                ui.add(egui::TextEdit::singleline(&mut p.tol_plus).desired_width(60.));
                ui.label("−");
                ui.add(egui::TextEdit::singleline(&mut p.tol_minus).desired_width(60.));
            });
        }
    }
    crate::editor::section(ui, "Aparência");
    crate::pmi_draw::style_ui(ui, &mut p.style, true);
    if ui.small_button(tr("🎨 Estilo geral das anotações…")).clicked() {
        *style_req = true;
    }
    if !matches!(p.kind, PmiKind::TitleBlock { .. } | PmiKind::Weld { .. } | PmiKind::Balloon { .. }) {
        ui.horizontal(|ui| {
            ui.label(tr("Prefixo"));
            ui.add(egui::TextEdit::singleline(&mut p.style.prefix).desired_width(60.).hint_text("4×"));
            ui.label(tr("Sufixo"));
            ui.add(egui::TextEdit::singleline(&mut p.style.suffix).desired_width(60.).hint_text(tr("típ.")));
        });
    }
    ui.horizontal(|ui| {
        ui.label(tr("Pasta"));
        ui.add(egui::TextEdit::singleline(&mut p.folder).desired_width(140.).hint_text(tr("(raiz)")));
    });
    crate::editor::section(ui, "Posição do texto");
    ui.horizontal(|ui| {
        for (k, l) in ["X", "Y", "Z"].iter().enumerate() {
            ui.label(*l);
            { let __r = ui.add(egui::DragValue::new(&mut p.offset[k]).speed(0.5)); crate::ui_ext::wheel(ui, &__r, &mut p.offset[k]); __r };
        }
    });
    crate::editor::section(ui, "Orientação do texto");
    let mut on_plane = p.plane.is_some();
    ui.horizontal(|ui| {
        if ui.radio(!on_plane, tr("de frente para a tela")).clicked() {
            on_plane = false;
        }
        if ui.radio(on_plane, tr("deitado num plano")).clicked() {
            on_plane = true;
        }
    });
    if !on_plane {
        p.plane = None;
    } else {
        if p.plane.is_none() {
            p.plane = Some(([0., 0., 1.], [1., 0., 0.]));
        }
        ui.horizontal(|ui| {
            for (n, pl) in [("XY", ([0., 0., 1.], [1., 0., 0.])), ("XZ", ([0., -1., 0.], [1., 0., 0.])), ("YZ", ([1., 0., 0.], [0., 1., 0.]))] {
                if ui.selectable_label(p.plane == Some(pl), n).clicked() {
                    p.plane = Some(pl);
                }
            }
            if ui.add_enabled(seed_plane.is_some(), egui::Button::new(tr("⊙ Plano/face selecionado"))).on_disabled_hover_text(tr("clique numa face ou plano na viewport")).clicked() {
                p.plane = seed_plane;
            }
        });
    }
    ui.label(RichText::new(tr("Arraste o rótulo na viewport para mudar a posição.")).weak().small());
    ui.checkbox(&mut p.visible, tr("visível"));
    if seed_pts.len() >= p.kind.anchors() && ui.button(tr("⊙ Reancorar na seleção atual")).clicked() {
        p.anchors = seed_pts[..p.kind.anchors()].to_vec();
    }
    true
}
