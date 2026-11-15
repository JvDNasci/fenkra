// SPDX-License-Identifier: LGPL-2.1-or-later
//! Origem e eixos do sketch no editor (B437).
//!
//! Três partes: o painel "Origem e eixos" (de onde vem a origem, o que H segue, deslocamento,
//! trocar e inverter), a escolha das referências no 3D com prévia da origem e dos eixos antes do
//! clique, e o desenho dos eixos H e V, que ficam sempre à mostra e são alvo de restrição. O que o
//! cálculo faz com a escolha está em `fenkra_solidos::sketch_posicao`.

use crate::app::App;
use crate::i18n::tr;
use crate::picking::{Filter, Pick};
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke};
use fenkra_core::sketch::{LinkSrc, SketchPos};
use fenkra_core::sketch_origem::{OrigemSk, OrientaSk};
use fenkra_core::{FeatureKind, Id, OriginAxis, OriginPlane, PlaneRef};
use fenkra_kernel::Frame;
use fenkra_solidos::sketch_posicao::{intersecao_no_plano, meio_da_polilinha};
use fenkra_solidos::RecomputeResult;
use glam::Vec3;

type V3 = [f64; 3];

/// O que o próximo clique no 3D escolhe.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pedido {
    Ponto,
    Intersecao,
    PontoMedio,
    CentroFace,
    /// H segue uma linha
    HLinha,
    /// H aponta para um ponto
    HPonto,
    /// H segue a reta de corte de um plano
    HPlano,
}

impl Pedido {
    pub fn dica(self, segunda: bool) -> &'static str {
        match self {
            Pedido::Ponto => "Clique num vértice ou ponto: ele vira a origem, projetado no plano",
            Pedido::Intersecao if !segunda => "Clique na primeira linha ou aresta",
            Pedido::Intersecao => "Clique na segunda linha ou aresta: a origem fica no cruzamento",
            Pedido::PontoMedio => "Clique numa aresta ou linha: a origem fica no meio dela",
            Pedido::CentroFace => "Clique numa face: a origem fica no centro dela",
            Pedido::HLinha => "Clique numa aresta ou linha: H segue a direção dela",
            Pedido::HPonto => "Clique num vértice ou ponto: H aponta da origem para ele",
            Pedido::HPlano => "Clique num plano: H segue a reta em que ele corta o sketch",
        }
    }
    fn filtro(self, sketch: Id) -> Filter {
        let (faces, edges, vertices, planes) = match self {
            Pedido::Ponto | Pedido::HPonto => (false, false, true, false),
            Pedido::Intersecao | Pedido::PontoMedio | Pedido::HLinha => (false, true, false, false),
            Pedido::CentroFace => (true, false, false, false),
            Pedido::HPlano => (false, false, false, true),
        };
        Filter { faces, edges, vertices, planes, hidden_env: false, exclude_sketch: Some(sketch), fantasma: None, planos_origem: true, planos_ref: true }
    }
    fn e_de_origem(self) -> bool {
        matches!(self, Pedido::Ponto | Pedido::Intersecao | Pedido::PontoMedio | Pedido::CentroFace)
    }
}

/// Estado da escolha no editor: só visão, nada disso vai para o documento.
#[derive(Clone, Debug, Default)]
pub struct Escolha {
    pub pedido: Option<Pedido>,
    /// primeira linha da interseção, com a polilinha dela para a prévia
    pub primeira: Option<(LinkSrc, Vec<V3>)>,
    /// o painel "Origem e eixos" está aberto: a tríade aparece no 3D
    pub aberto: bool,
}

/// O clique escolheu isto.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Escolhido {
    Fonte(LinkSrc),
    Plano(PlaneRef),
}

/// O que o item clicado vira para o pedido; `None` se ele não serve.
pub fn escolhido(r: &RecomputeResult, p: &Pick, pedido: Pedido) -> Option<Escolhido> {
    let ponto = matches!(pedido, Pedido::Ponto | Pedido::HPonto);
    let linha = matches!(pedido, Pedido::Intersecao | Pedido::PontoMedio | Pedido::HLinha);
    let f = |q: [f32; 3]| q.map(f64::from);
    Some(Escolhido::Fonte(match *p {
        // só vértice de corpo: o cálculo o reencontra entre os vértices dos corpos
        Pick::Vertex { pos, .. } if ponto => LinkSrc::Vertex { pos: f(pos) },
        Pick::Edge { mid, .. } if linha => LinkSrc::Edge { mid: f(mid) },
        Pick::SkCurve { sketch, curve, .. } if linha => LinkSrc::SketchCurve { sketch, curve },
        Pick::Face { point, .. } if pedido == Pedido::CentroFace => LinkSrc::Face { point: f(point) },
        Pick::Geo(id) if ponto && r.points.iter().any(|x| x.id == id) => LinkSrc::Ref(id),
        Pick::Geo(id) if linha && r.curves.iter().any(|x| x.id == id) => LinkSrc::Ref(id),
        Pick::Geo(id) if pedido == Pedido::CentroFace && r.surfaces.iter().any(|x| x.id == id) => LinkSrc::Ref(id),
        Pick::Plane(pr) if pedido == Pedido::HPlano => return Some(Escolhido::Plano(pr)),
        _ => return None,
    }))
}

/// Grava a escolha na posição do sketch. A interseção pede a primeira linha antes.
pub fn aplica(pos: &mut SketchPos, pedido: Pedido, e: Escolhido, primeira: Option<LinkSrc>) -> bool {
    match (pedido, e) {
        (Pedido::Ponto, Escolhido::Fonte(s)) => pos.modo = OrigemSk::Ponto(s),
        (Pedido::Intersecao, Escolhido::Fonte(s)) => match primeira {
            Some(a) => pos.modo = OrigemSk::Intersecao(a, s),
            None => return false,
        },
        (Pedido::PontoMedio, Escolhido::Fonte(s)) => pos.modo = OrigemSk::PontoMedio(s),
        (Pedido::CentroFace, Escolhido::Fonte(s)) => pos.modo = OrigemSk::CentroFace(s),
        (Pedido::HLinha, Escolhido::Fonte(s)) => pos.orienta = OrientaSk::Linha(s),
        (Pedido::HPonto, Escolhido::Fonte(s)) => pos.orienta = OrientaSk::Ponto(s),
        (Pedido::HPlano, Escolhido::Plano(p)) => pos.orienta = OrientaSk::Plano(p),
        _ => return false,
    }
    // a origem escolhida é o lugar: o deslocamento antigo deixaria de fazer sentido
    if pedido.e_de_origem() {
        pos.origem = [0., 0.];
    }
    true
}

/// Polilinha 3D do item clicado (aresta, curva de outro sketch ou de referência).
fn polilinha(app: &App, r: &RecomputeResult, p: &Pick) -> Option<Vec<V3>> {
    match *p {
        Pick::Edge { body, edge, .. } => Some(crate::picking::edge_polyline(r, body, edge)),
        Pick::SkCurve { sketch, curve, .. } => crate::sketch_ext::sketch_curves(&app.open.doc, r, sketch).into_iter().find(|x| x.0 == curve).map(|x| x.1),
        Pick::Geo(id) => r.curves.iter().find(|c| c.id == id).map(|c| c.pts.clone()),
        _ => None,
    }
    .filter(|v| !v.is_empty())
}

/// Centroide da face pela malha (prévia; o cálculo usa a face exata).
fn centro_da_malha(r: &RecomputeResult, body: usize, face: usize) -> Option<V3> {
    let m = &r.bodies.get(body)?.mesh;
    let s0 = *m.face_ranges.get(face)? as usize;
    let s1 = m.face_ranges.get(face + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
    let p = |i: u32| [m.positions[i as usize * 3] as f64, m.positions[i as usize * 3 + 1] as f64, m.positions[i as usize * 3 + 2] as f64];
    let (mut soma, mut area) = ([0.; 3], 0.);
    for t in m.indices.get(s0..s1)?.chunks_exact(3) {
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = fenkra_kernel::cross(u, w);
        let s = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() / 2.;
        for i in 0..3 {
            soma[i] += s * (a[i] + b[i] + c[i]) / 3.;
        }
        area += s;
    }
    (area > 1e-12).then(|| soma.map(|x| x / area))
}

/// Onde a origem vai ficar se o item sob o mouse for escolhido (no plano do sketch).
fn origem_previa(app: &App, r: &RecomputeResult, p: &Pick, pedido: Pedido, primeira: Option<&[V3]>, f: &Frame) -> Option<V3> {
    let q = match (pedido, *p) {
        (Pedido::Ponto, Pick::Vertex { pos, .. }) => pos.map(f64::from),
        (Pedido::Ponto, Pick::Geo(id)) => r.points.iter().find(|x| x.id == id)?.p,
        (Pedido::PontoMedio, _) => meio_da_polilinha(&polilinha(app, r, p)?)?,
        (Pedido::Intersecao, _) => intersecao_no_plano(f, primeira?, &polilinha(app, r, p)?).ok()?,
        (Pedido::CentroFace, Pick::Face { body, face, .. }) => centro_da_malha(r, body, face)?,
        (Pedido::CentroFace, Pick::Geo(id)) => {
            let s = r.surfaces.iter().find(|x| x.id == id)?;
            let n = s.mesh.positions.len() / 3;
            let mut c = [0.; 3];
            for v in s.mesh.positions.chunks_exact(3) {
                for i in 0..3 {
                    c[i] += v[i] as f64 / n.max(1) as f64;
                }
            }
            c
        }
        _ => return None,
    };
    let d = [q[0] - f.origin[0], q[1] - f.origin[1], q[2] - f.origin[2]];
    let h = d[0] * f.normal[0] + d[1] * f.normal[1] + d[2] * f.normal[2];
    Some([q[0] - h * f.normal[0], q[1] - h * f.normal[1], q[2] - h * f.normal[2]])
}

/// Direção que H vai seguir se o item sob o mouse for escolhido.
fn h_previa(app: &App, r: &RecomputeResult, p: &Pick, pedido: Pedido, f: &Frame) -> Option<V3> {
    let d = match (pedido, *p) {
        (Pedido::HLinha, _) => {
            let l = polilinha(app, r, p)?;
            let (a, b) = (l[0], *l.last()?);
            [b[0] - a[0], b[1] - a[1], b[2] - a[2]]
        }
        (Pedido::HPonto, Pick::Vertex { pos, .. }) => {
            let q = pos.map(f64::from);
            [q[0] - f.origin[0], q[1] - f.origin[1], q[2] - f.origin[2]]
        }
        (Pedido::HPonto, Pick::Geo(id)) => {
            let q = r.points.iter().find(|x| x.id == id)?.p;
            [q[0] - f.origin[0], q[1] - f.origin[1], q[2] - f.origin[2]]
        }
        (Pedido::HPlano, Pick::Plane(pr)) => {
            let n = match pr {
                PlaneRef::Origin(o) => fenkra_solidos::origin_frame(o).normal,
                PlaneRef::Datum(id) => r.planes.iter().find(|x| x.id == Some(id))?.frame.normal,
            };
            fenkra_kernel::cross(f.normal, n)
        }
        _ => return None,
    };
    let k = d[0] * f.normal[0] + d[1] * f.normal[1] + d[2] * f.normal[2];
    let p = [d[0] - k * f.normal[0], d[1] - k * f.normal[1], d[2] - k * f.normal[2]];
    let n = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    (n > 1e-9).then(|| p.map(|x| x / n))
}

fn tela(cam: &Camera, rect: Rect, p: V3) -> Option<Pos2> {
    cam.project(rect, Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32))
}

/// Tríade da origem: círculo na origem e setas H e V com o nome, nas cores dos eixos.
pub fn triade(painter: &egui::Painter, cam: &Camera, rect: Rect, o: V3, h: V3, v: V3, fantasma: bool) {
    let Some(so) = tela(cam, rect, o) else { return };
    let alfa = if fantasma { 150 } else { 255 };
    let fonte = egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.));
    for (dir, eixo_v, nome) in [(h, false, "H"), (v, true, "V")] {
        let Some(s1) = tela(cam, rect, [o[0] + dir[0], o[1] + dir[1], o[2] + dir[2]]) else { continue };
        let d = (s1 - so).normalized();
        if !d.x.is_finite() {
            continue;
        }
        let cor = crate::theme::com_alfa(crate::theme::eixo_sketch(eixo_v), alfa);
        let ponta = so + d * 64.;
        let st = Stroke::new(if fantasma { 2. } else { 2.6 }, cor);
        if fantasma {
            painter.extend(egui::Shape::dashed_line(&[so, ponta], st, 6., 4.));
        } else {
            painter.line_segment([so, ponta], st);
        }
        let lado = egui::vec2(-d.y, d.x) * 5.;
        painter.add(egui::Shape::convex_polygon(vec![ponta + d * 10., ponta + lado, ponta - lado], cor, Stroke::NONE));
        painter.text(ponta + d * 20., egui::Align2::CENTER_CENTER, nome, fonte.clone(), cor);
    }
    let cor = crate::theme::com_alfa(crate::theme::accent(&egui::Visuals::dark()), alfa);
    painter.circle_stroke(so, 7., Stroke::new(2., cor));
    painter.circle_filled(so, 2.5, cor);
}

/// Prévia e escolha no 3D. Devolve `true` se o clique foi da escolha (a ferramenta não age).
pub fn overlay(app: &mut App, ui: &egui::Ui, painter: &egui::Painter, cam: &Camera, rect: Rect, f: &Frame, resp: &egui::Response) -> bool {
    let Some(edit) = app.sketch.as_ref() else { return false };
    let esc = edit.posicao.clone();
    let sketch = edit.feature;
    // `FENKRA_DEMO_SKETCH`: o cursor simulado vale como o do mouse
    let demo = edit.demo_cursor.and_then(|q| tela(cam, rect, f.point(q[0], q[1])));
    if esc.aberto || esc.pedido.is_some() {
        triade(painter, cam, rect, f.origin, f.x, f.y(), false);
    }
    let Some(pedido) = esc.pedido else { return false };
    let (Some(r), Some(h)) = (app.result.as_ref(), demo.or(resp.hover_pos())) else { return false };
    let dica = tr(pedido.dica(esc.primeira.is_some()));
    painter.text(h + egui::vec2(16., 16.), egui::Align2::LEFT_TOP, format!("{dica} · {}", tr("Esc cancela")), egui::FontId::proportional(crate::acessibilidade::fonte_3d(11.)), ui.visuals().strong_text_color());
    let p = crate::picking::pick(r, &app.open.doc, cam, rect, h, &pedido.filtro(sketch));
    let mut serve = None;
    if let Some(p) = &p {
        serve = escolhido(r, p, pedido);
        if serve.is_some() {
            crate::picking::draw(painter, r, cam, rect, p, false);
            // prévia da origem nova com os eixos de agora, ou dos eixos novos na origem de agora
            if let Some(o) = origem_previa(app, r, p, pedido, esc.primeira.as_ref().map(|x| x.1.as_slice()), f) {
                triade(painter, cam, rect, o, f.x, f.y(), true);
            }
            if let Some(hx) = h_previa(app, r, p, pedido, f) {
                let vx = fenkra_kernel::cross(f.normal, hx);
                triade(painter, cam, rect, f.origin, hx, vx, true);
            }
        } else {
            ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
        }
    }
    if !resp.clicked_by(egui::PointerButton::Primary) {
        return false;
    }
    let Some(e) = serve else {
        app.toast(tr("Isso não serve para esta escolha").to_string(), true);
        return true;
    };
    // primeira linha da interseção: guarda e pede a segunda
    if pedido == Pedido::Intersecao && esc.primeira.is_none() {
        if let (Escolhido::Fonte(s), Some(pl)) = (e, p.as_ref().and_then(|p| polilinha(app, r, p))) {
            if let Some(ed) = app.sketch.as_mut() {
                ed.posicao.primeira = Some((s, pl));
            }
        }
        return true;
    }
    if let Some(FeatureKind::Sketch(s)) = app.open.doc.feature_mut(sketch).map(|f| &mut f.kind) {
        aplica(&mut s.data.pos, pedido, e, esc.primeira.map(|x| x.0));
    }
    if let Some(ed) = app.sketch.as_mut() {
        ed.posicao.pedido = None;
        ed.posicao.primeira = None;
    }
    true
}

/// Nome traduzido do modo, com a escolha que falta.
fn rotulo_origem(m: &OrigemSk) -> String {
    tr(m.nome()).to_string()
}

#[derive(Clone, Copy, PartialEq)]
enum OpOrigem {
    Fixo(OrigemSk),
    Pede(Pedido),
}

#[derive(Clone, Copy, PartialEq)]
enum OpOrienta {
    Fixo(OrientaSk),
    Pede(Pedido),
}

/// Painel "Origem e eixos" do sketch em edição.
pub fn painel(app: &mut App, ui: &mut egui::Ui) {
    let Some(id) = app.sketch.as_ref().map(|e| e.feature) else { return };
    let avisos: Vec<String> = app.result.as_ref().and_then(|r| r.status.get(&id)).and_then(|s| if let fenkra_solidos::Status::Warning(w) = s { Some(w.clone()) } else { None }).map(|w| w.split("; ").filter(|x| x.contains("do sketch")).map(str::to_string).collect()).unwrap_or_default();
    let Some(FeatureKind::Sketch(s)) = app.open.doc.feature_mut(id).map(|f| &mut f.kind) else { return };
    let pos = &mut s.data.pos;
    let Some(edit) = app.sketch.as_mut() else { return };
    let esc = &mut edit.posicao;
    let mut pedir: Option<Pedido> = None;
    let r = egui::CollapsingHeader::new(tr("Origem e eixos")).id_salt("sk_origem_eixos").default_open(crate::sketch_demo::abre_origem()).show(ui, |ui| {
        ui.label(RichText::new(tr("Origem")).strong());
        let atual = rotulo_origem(&pos.modo);
        egui::ComboBox::from_id_salt("sk_origem_modo").selected_text(atual).width(210.).show_ui(ui, |ui| {
            for (op, nome, dica) in [
                (OpOrigem::Fixo(OrigemSk::Auto), "Automática", "A origem do plano escolhido para o sketch."),
                (OpOrigem::Fixo(OrigemSk::OrigemPeca), "Origem da peça", "A origem da peça, projetada no plano."),
                (OpOrigem::Pede(Pedido::Ponto), "Ponto projetado", "Um vértice ou ponto do modelo, projetado no plano."),
                (OpOrigem::Pede(Pedido::Intersecao), "Interseção de duas linhas", "O cruzamento de duas arestas ou linhas, projetadas no plano."),
                (OpOrigem::Pede(Pedido::PontoMedio), "Ponto médio", "O meio de uma aresta ou linha."),
                (OpOrigem::Pede(Pedido::CentroFace), "Centro da face", "O centro de uma face do modelo."),
            ] {
                let marcado = match op {
                    OpOrigem::Fixo(m) => pos.modo == m,
                    OpOrigem::Pede(p) => esc.pedido == Some(p) || std::mem::discriminant(&pos.modo) == std::mem::discriminant(&pedido_para_origem(p)),
                };
                if ui.selectable_label(marcado, tr(nome)).on_hover_text(tr(dica)).clicked() {
                    match op {
                        OpOrigem::Fixo(m) => {
                            pos.modo = m;
                            esc.pedido = None;
                        }
                        OpOrigem::Pede(p) => pedir = Some(p),
                    }
                }
            }
        })
        .response
        .on_hover_text(tr("De onde vem a origem do sketch. As escolhas no modelo acompanham quando ele muda."));
        if pos.modo.escolhas() > 0 && esc.pedido.is_none() && ui.small_button(tr("Escolher de novo")).on_hover_text(tr("Clica de novo no modelo, mantendo o modo")).clicked() {
            pedir = Some(match pos.modo {
                OrigemSk::Ponto(_) => Pedido::Ponto,
                OrigemSk::Intersecao(..) => Pedido::Intersecao,
                OrigemSk::PontoMedio(_) => Pedido::PontoMedio,
                _ => Pedido::CentroFace,
            });
        }
        ui.horizontal(|ui| {
            let r = ui.add(crate::ui_ext::dv(&mut pos.origem[0]).speed(0.5).prefix(format!("{} ", tr("deslocamento H"))));
            crate::ui_ext::wheel(ui, &r, &mut pos.origem[0]);
            r.on_hover_text(tr("Anda com a origem ao longo de H, a partir do ponto escolhido."));
            let r = ui.add(crate::ui_ext::dv(&mut pos.origem[1]).speed(0.5).prefix(format!("{} ", tr("deslocamento V"))));
            crate::ui_ext::wheel(ui, &r, &mut pos.origem[1]);
            r.on_hover_text(tr("Anda com a origem ao longo de V, a partir do ponto escolhido."));
        });
        ui.add_space(4.);
        ui.label(RichText::new(tr("H segue")).strong());
        egui::ComboBox::from_id_salt("sk_origem_orienta").selected_text(tr(pos.orienta.nome())).width(210.).show_ui(ui, |ui| {
            for (op, nome, dica) in [
                (OpOrienta::Fixo(OrientaSk::Auto), "Automática", "O H do plano escolhido para o sketch."),
                (OpOrienta::Fixo(OrientaSk::Eixo(OriginAxis::X)), "Eixo X", "O eixo X da peça, projetado no plano."),
                (OpOrienta::Fixo(OrientaSk::Eixo(OriginAxis::Y)), "Eixo Y", "O eixo Y da peça, projetado no plano."),
                (OpOrienta::Fixo(OrientaSk::Eixo(OriginAxis::Z)), "Eixo Z", "O eixo Z da peça, projetado no plano."),
                (OpOrienta::Pede(Pedido::HLinha), "Linha", "A direção de uma aresta ou linha do modelo."),
                (OpOrienta::Pede(Pedido::HPonto), "Ponto", "Da origem para um vértice ou ponto do modelo."),
                (OpOrienta::Fixo(OrientaSk::Plano(PlaneRef::Origin(OriginPlane::XY))), "Plano XY", "A reta em que o plano XY corta o sketch."),
                (OpOrienta::Fixo(OrientaSk::Plano(PlaneRef::Origin(OriginPlane::XZ))), "Plano XZ", "A reta em que o plano XZ corta o sketch."),
                (OpOrienta::Fixo(OrientaSk::Plano(PlaneRef::Origin(OriginPlane::YZ))), "Plano YZ", "A reta em que o plano YZ corta o sketch."),
                (OpOrienta::Pede(Pedido::HPlano), "Plano de referência", "A reta em que um plano de referência corta o sketch."),
            ] {
                let marcado = match op {
                    OpOrienta::Fixo(o) => pos.orienta == o,
                    OpOrienta::Pede(p) => esc.pedido == Some(p),
                };
                if ui.selectable_label(marcado, tr(nome)).on_hover_text(tr(dica)).clicked() {
                    match op {
                        OpOrienta::Fixo(o) => {
                            pos.orienta = o;
                            esc.pedido = None;
                        }
                        OpOrienta::Pede(p) => pedir = Some(p),
                    }
                }
            }
        })
        .response
        .on_hover_text(tr("O que o eixo H do sketch segue. V fica a 90° de H, no plano."));
        ui.horizontal_wrapped(|ui| {
            let dica = tr("Trocar ou inverter um eixo só vira a normal: a extrusão passa para o outro lado.");
            ui.checkbox(&mut pos.trocar, tr("Trocar H e V")).on_hover_text(dica);
            ui.checkbox(&mut pos.inverter_h, tr("Inverter H")).on_hover_text(dica);
            ui.checkbox(&mut pos.inverter_v, tr("Inverter V")).on_hover_text(dica);
        });
        if let Some(p) = esc.pedido {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("▶ {}", tr(p.dica(esc.primeira.is_some())))).color(crate::theme::accent(ui.visuals())));
                if ui.small_button(tr("Cancelar")).clicked() {
                    esc.pedido = None;
                    esc.primeira = None;
                }
            });
        }
        for a in &avisos {
            ui.label(RichText::new(format!("⚠ {a}")).color(crate::theme::st_warn()));
        }
        ui.label(RichText::new(tr("A origem e os eixos H e V entram em qualquer restrição: selecione-os junto com a geometria e use o botão direito.")).weak().small());
    });
    esc.aberto = r.body_returned.is_some();
    if let Some(p) = pedir {
        esc.pedido = Some(p);
        esc.primeira = None;
    }
}

/// Modo de origem que o pedido produz (para marcar a opção atual no menu).
fn pedido_para_origem(p: Pedido) -> OrigemSk {
    let x = LinkSrc::Vertex { pos: [0.; 3] };
    match p {
        Pedido::Ponto => OrigemSk::Ponto(x),
        Pedido::Intersecao => OrigemSk::Intersecao(x, x),
        Pedido::PontoMedio => OrigemSk::PontoMedio(x),
        Pedido::CentroFace => OrigemSk::CentroFace(x),
        // os pedidos de orientação não são modo de origem
        _ => OrigemSk::Auto,
    }
}

/// Dois pontos perto do meio do eixo, para levar à tela. As pontas ficam a 10 m: na perspectiva elas
/// podem cair atrás da câmera, e a projeção delas não serviria.
pub fn perto_do_meio(pa: [f64; 2], pb: [f64; 2]) -> ([f64; 2], [f64; 2]) {
    let m = [(pa[0] + pb[0]) / 2., (pa[1] + pb[1]) / 2.];
    let (dx, dy) = (pb[0] - pa[0], pb[1] - pa[1]);
    let n = dx.hypot(dy).max(1e-12);
    (m, [m[0] + dx / n * 10., m[1] + dy / n * 10.])
}

/// Trecho da reta do eixo que cobre a vista: as pontas ficam a 10 m, então o trecho se centra no pé
/// da reta mais perto do meio da vista.
pub fn trecho_do_eixo(rect: Rect, a: Pos2, b: Pos2) -> Option<(Pos2, Pos2)> {
    let d = b - a;
    if d.length() < 1e-3 || !d.x.is_finite() || !d.y.is_finite() {
        return None;
    }
    let d = d.normalized();
    let pe = a + d * (rect.center() - a).dot(d);
    let l = rect.size().length();
    Some((pe - d * l, pe + d * l))
}

/// Desenha um eixo do sketch atravessando a vista toda, na cor do eixo.
pub fn desenha_eixo(painter: &egui::Painter, rect: Rect, a: Pos2, b: Pos2, eixo_v: bool, destaque: Option<Color32>) {
    let Some((p, q)) = trecho_do_eixo(rect, a, b) else { return };
    let cor = destaque.unwrap_or_else(|| crate::theme::com_alfa(crate::theme::eixo_sketch(eixo_v), 170));
    painter.line_segment([p, q], Stroke::new(if destaque.is_some() { 2.2 } else { 1.4 }, cor));
}

/// Distância, na tela, do cursor à reta infinita do eixo.
pub fn distancia_ao_eixo(pos: Pos2, a: Pos2, b: Pos2) -> f32 {
    let d = b - a;
    let n = d.length();
    if n < 1e-6 {
        return f32::MAX;
    }
    ((pos - a).x * d.y - (pos - a).y * d.x).abs() / n
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn escolha_vira_modo_e_interseção_pede_duas() {
        let mut p = SketchPos { origem: [3., 4.], ..Default::default() };
        let face = Escolhido::Fonte(LinkSrc::Face { point: [1., 2., 3.] });
        assert!(aplica(&mut p, Pedido::CentroFace, face, None));
        assert_eq!(p.modo, OrigemSk::CentroFace(LinkSrc::Face { point: [1., 2., 3.] }));
        assert_eq!(p.origem, [0., 0.], "a origem escolhida zera o deslocamento");
        let a = LinkSrc::Edge { mid: [0.; 3] };
        assert!(!aplica(&mut p, Pedido::Intersecao, Escolhido::Fonte(a), None), "falta a primeira linha");
        assert!(aplica(&mut p, Pedido::Intersecao, Escolhido::Fonte(a), Some(a)));
        assert!(aplica(&mut p, Pedido::HPlano, Escolhido::Plano(PlaneRef::Origin(OriginPlane::YZ)), None));
        assert_eq!(p.orienta, OrientaSk::Plano(PlaneRef::Origin(OriginPlane::YZ)));
        // plano não serve para origem
        assert!(!aplica(&mut p, Pedido::Ponto, Escolhido::Plano(PlaneRef::Origin(OriginPlane::XY)), None));
    }

    use crate::sketch_ui::{applicable, apply_constraint, delete_selection, CKind, Ent};
    use fenkra_core::sketch::{Curve, Kind};
    use fenkra_core::SketchData;

    fn sketch_com_linha() -> (SketchData, fenkra_core::sketch_origem::EixosSk, Id, Id, Id, impl FnMut() -> Id) {
        let mut n = 7000u64;
        let mut al = move || {
            n += 1;
            Id(n)
        };
        let mut sk = SketchData::default();
        let e = sk.garantir_eixos(&mut al);
        let (a, b) = (sk.add_point(&mut al, 3., 4.), sk.add_point(&mut al, 23., 9.));
        let l = sk.add_curve(&mut al, Curve::Line { a, b }, false);
        (sk, e, a, b, l, al)
    }

    #[test]
    fn botao_direito_oferece_restricoes_com_os_eixos() {
        let (sk, e, a, _, l, _) = sketch_com_linha();
        // linha + eixo H: paralela, perpendicular, horizontal e vertical; igual não (o eixo não tem comprimento)
        let ops = applicable(&sk, &[Ent::Cv(l), Ent::Cv(e.h)]);
        for k in [CKind::Parallel, CKind::Perpendicular, CKind::Horizontal, CKind::Vertical] {
            assert!(ops.contains(&k), "{k:?} em {ops:?}");
        }
        assert!(!ops.contains(&CKind::Equal));
        // ponto + eixo: coincidente (ponto no eixo), horizontal e vertical (alinhado à origem), simetria não
        let ops = applicable(&sk, &[Ent::Pt(a), Ent::Cv(e.v)]);
        for k in [CKind::Coincident, CKind::Horizontal, CKind::Vertical] {
            assert!(ops.contains(&k), "{k:?} em {ops:?}");
        }
        // ponto + origem: coincidente, horizontal, vertical
        let ops = applicable(&sk, &[Ent::Pt(a), Ent::Pt(e.origem)]);
        for k in [CKind::Coincident, CKind::Horizontal, CKind::Vertical] {
            assert!(ops.contains(&k), "{k:?} em {ops:?}");
        }
        // só o eixo: nada a fazer, ele já é fixo
        assert!(applicable(&sk, &[Ent::Cv(e.h)]).iter().all(|k| *k != CKind::Horizontal));
    }

    #[test]
    fn restricao_com_eixo_segura_a_linha_e_nao_mexe_no_eixo() {
        let (mut sk, e, a, b, l, mut al) = sketch_com_linha();
        apply_constraint(&mut sk, &mut al, CKind::Horizontal, &[Ent::Cv(l), Ent::Cv(e.h)]).unwrap();
        assert!(sk.constraints.iter().any(|c| c.kind == Kind::Horizontal(l)));
        assert!(!sk.constraints.iter().any(|c| c.kind.refs().contains(&e.h)), "o eixo não ganha restrição sobre ele mesmo");
        apply_constraint(&mut sk, &mut al, CKind::Coincident, &[Ent::Pt(a), Ent::Cv(e.v)]).unwrap();
        apply_constraint(&mut sk, &mut al, CKind::Horizontal, &[Ent::Pt(b), Ent::Cv(e.h)]).unwrap();
        let r = sk.solve(&Default::default(), &[]);
        assert!(r.converged, "{r:?}");
        // a no eixo V, b alinhado à origem: a linha cai no eixo H, com a ponta a na origem
        assert!(sk.pos(a)[0].abs() < 1e-6 && sk.pos(a)[1].abs() < 1e-6, "{:?}", sk.pos(a));
        assert!(sk.pos(b)[1].abs() < 1e-6, "{:?}", sk.pos(b));
        assert_eq!(sk.pos(e.origem), [0., 0.]);
        // coincidente com a origem funde o ponto nela
        let (mut sk, e, a, _, l, mut al) = sketch_com_linha();
        apply_constraint(&mut sk, &mut al, CKind::Coincident, &[Ent::Pt(a), Ent::Pt(e.origem)]).unwrap();
        assert!(sk.point(a).is_none());
        assert!(matches!(sk.curve(l).map(|c| &c.geom), Some(Curve::Line { a: x, .. }) if *x == e.origem));
    }

    #[test]
    fn origem_e_eixos_nao_se_apagam_nem_entram_na_janela() {
        let (mut sk, e, _, _, l, _) = sketch_com_linha();
        delete_selection(&mut sk, &[Ent::Cv(e.h), Ent::Cv(e.v), Ent::Pt(e.origem), Ent::Cv(l)]);
        assert!(sk.curve(e.h).is_some() && sk.curve(e.v).is_some() && sk.point(e.origem).is_some());
        assert!(sk.curve(l).is_none(), "a linha comum sai");
        let (sk, e, _, _, l, _) = sketch_com_linha();
        let tela = |q: [f64; 2]| egui::pos2(q[0] as f32 * 10., -q[1] as f32 * 10.);
        let tudo = egui::Rect::from_min_max(egui::pos2(-1e6, -1e6), egui::pos2(1e6, 1e6));
        let pega = crate::sketch_cadeia::na_janela(&sk, &tela, tudo, true);
        assert!(pega.contains(&Ent::Cv(l)));
        assert!(!pega.iter().any(|x| matches!(x, Ent::Cv(c) | Ent::Pt(c) if sk.e_da_origem(*c))), "{pega:?} {e:?}");
    }

    #[test]
    fn eixo_desenhado_atravessa_a_vista_mesmo_com_as_pontas_longe() {
        let rect = Rect::from_min_size(egui::pos2(0., 0.), egui::vec2(800., 600.));
        let (p, q) = trecho_do_eixo(rect, egui::pos2(-90000., 300.), egui::pos2(90000., 300.)).unwrap();
        assert!(p.x < 0. && q.x > 800., "{p:?} {q:?}");
        assert!((p.y - 300.).abs() < 1e-3 && (q.y - 300.).abs() < 1e-3);
        // reta inclinada que passa fora do centro também cruza a vista inteira
        let (p, q) = trecho_do_eixo(rect, egui::pos2(-50000., -50000.), egui::pos2(50000., 50000. + 100.)).unwrap();
        assert!(p.x < 0. && q.x > 800.);
    }

    #[test]
    fn distancia_a_reta_infinita_do_eixo() {
        let d = distancia_ao_eixo(egui::pos2(500., 13.), egui::pos2(0., 10.), egui::pos2(1., 10.));
        assert!((d - 3.).abs() < 1e-4);
    }

    #[test]
    fn textos_tem_traducao() {
        for p in [Pedido::Ponto, Pedido::Intersecao, Pedido::PontoMedio, Pedido::CentroFace, Pedido::HLinha, Pedido::HPonto, Pedido::HPlano] {
            assert!(crate::i18n::tem_traducao(p.dica(false)), "{}", p.dica(false));
            assert!(crate::i18n::tem_traducao(p.dica(true)), "{}", p.dica(true));
        }
        for m in [OrigemSk::Auto, OrigemSk::OrigemPeca, pedido_para_origem(Pedido::Ponto), pedido_para_origem(Pedido::Intersecao), pedido_para_origem(Pedido::PontoMedio), pedido_para_origem(Pedido::CentroFace)] {
            assert!(crate::i18n::tem_traducao(m.nome()), "{}", m.nome());
        }
        for o in [OrientaSk::Auto, OrientaSk::Eixo(OriginAxis::X), OrientaSk::Eixo(OriginAxis::Y), OrientaSk::Eixo(OriginAxis::Z), OrientaSk::Linha(LinkSrc::Ref(Id(1))), OrientaSk::Ponto(LinkSrc::Ref(Id(1))), OrientaSk::Plano(PlaneRef::Origin(OriginPlane::XY))] {
            assert!(crate::i18n::tem_traducao(o.nome()), "{}", o.nome());
        }
    }
}
