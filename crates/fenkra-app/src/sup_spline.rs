// SPDX-License-Identifier: LGPL-2.1-or-later
//! B291: Spline, Spline controle e Polilinha editadas na viewport. Com o comando aberto, cada clique
//! acrescenta um ponto; os pontos têm gizmo, a spline tem alça de tangente por ponto, o polígono fica
//! visível e o pente é ligado no painel. Ponto que veio de outra referência fica preso, com o elo.

use crate::app::App;
use crate::i18n::tr;
use crate::picking::Pick;
use crate::render::Camera;
use eframe::egui::{self, Pos2, Rect, RichText, Stroke};
use fenkra_core::*;
use fenkra_solidos::geo3d::{self, Curve3};
use fenkra_solidos::RecomputeResult;
use glam::Vec3;

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

fn d3(v: Vec3) -> [f64; 3] {
    [v.x as f64, v.y as f64, v.z as f64]
}

/// Estado da edição, que não vai para o documento.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Edicao {
    /// ponto com o gizmo
    pub ponto: Option<usize>,
    pub pente: bool,
    /// multiplica o dente maior, que por padrão mede um quinto da curva
    pub escala: f32,
}

impl Default for Edicao {
    fn default() -> Self {
        // FENKRA_DEMO_SPLINE: captura com o segundo ponto escolhido e o pente ligado
        let demo = std::env::var_os("FENKRA_DEMO_SPLINE").is_some();
        Edicao { ponto: demo.then_some(1), pente: demo, escala: 1. }
    }
}

fn chave(id: Id) -> egui::Id {
    egui::Id::unique(("edicao_spline", id.0))
}

pub fn edicao(ctx: &egui::Context, id: Id) -> Edicao {
    ctx.data(|d| d.get_temp(chave(id))).unwrap_or_default()
}

fn guardar(ctx: &egui::Context, id: Id, e: Edicao) {
    ctx.data_mut(|d| d.insert_temp(chave(id), e));
}

/// Pontos da curva editável.
pub fn pontos_de(k: &RefKind) -> Option<&Vec<PointRef>> {
    match k {
        RefKind::Spline { points, .. } | RefKind::SplineControl { points, .. } | RefKind::Polyline { points, .. } => Some(points),
        _ => None,
    }
}

fn pontos_mut(k: &mut RefKind) -> Option<&mut Vec<PointRef>> {
    match k {
        RefKind::Spline { points, .. } | RefKind::SplineControl { points, .. } | RefKind::Polyline { points, .. } => Some(points),
        _ => None,
    }
}

/// O comando aberto é uma curva por pontos?
pub fn editavel(doc: &Document, id: Id) -> bool {
    doc.reference(id).is_some_and(|r| pontos_de(&r.kind).is_some())
}

/// Ponto novo: ocupa o primeiro campo vazio ou entra no fim.
pub fn acrescentar(k: &mut RefKind, p: PointRef) {
    let Some(v) = pontos_mut(k) else { return };
    match v.iter().position(|x| *x == crate::slots::point()) {
        Some(i) => v[i] = p,
        None => v.push(p),
    }
}

/// Só ponto fixo anda. O que vem de outra referência segue a referência.
pub fn preso(p: &PointRef) -> bool {
    !matches!(p, PointRef::Fixed(_))
}

/// Traduz o que está sob o ponteiro no ponto que entra na curva. Vértice, ponto e vértice de curva
/// viram vínculo; face vira coordenada, porque o ponto clicado não tem nome na peça.
pub fn ponto_do_alvo(alvo: Option<Pick>, vertice: impl Fn(usize, [f64; 3]) -> PointRef, ref_ponto: impl Fn(Id) -> bool) -> Option<PointRef> {
    let f = |p: [f32; 3]| p.map(|x| x as f64);
    match alvo? {
        Pick::Vertex { body, pos } => Some(vertice(body, f(pos))),
        Pick::Geo(id) if ref_ponto(id) => Some(PointRef::Ref(id)),
        Pick::CurveVtx { curve, index, .. } => Some(PointRef::CurveVertex { curve, index }),
        Pick::SkPoint { sketch, point, .. } => Some(PointRef::SketchPoint { sketch, point }),
        Pick::Face { point, .. } => Some(PointRef::Fixed(f(point))),
        Pick::Edge { mid, .. } => Some(PointRef::Fixed(f(mid))),
        _ => None,
    }
}

/// Clique no vazio: o ponto cai no plano de frente para a tela que passa pelo último ponto. O primeiro
/// ponto da curva cai no plano de frente pela origem.
pub fn no_vazio(o: Vec3, d: Vec3, ancora: Vec3, frente: Vec3) -> Vec3 {
    let den = d.dot(frente);
    if den.abs() < 1e-6 {
        return ancora;
    }
    o + d * ((ancora - o).dot(frente) / den)
}

/// Ponto que o clique na viewport acrescenta à curva aberta.
pub fn ponto_do_clique(app: &App, cam: &Camera, rect: Rect, pos: Pos2, id: Id, alvo: Option<Pick>) -> Option<PointRef> {
    let r = app.result.as_ref()?;
    let doc = &app.open.doc;
    let pts = pontos_de(&doc.reference(id)?.kind)?;
    // a curva não pode apontar para ela mesma
    let alvo = alvo.filter(|a| !matches!(a, Pick::CurveVtx { curve, .. } if *curve == id));
    let eh_ponto = |x: Id| x != id && doc.reference(x).is_some_and(|q| q.kind.class() == RefClass::Point);
    Some(ponto_do_alvo(alvo, |b, q| app.vertex_ref(b, q), eh_ponto).unwrap_or_else(|| {
        let ancora = pts.iter().rev().find_map(|q| if *q == crate::slots::point() { None } else { crate::picking::point_pos(r, q) }).map_or(Vec3::ZERO, v3);
        let (o, d) = cam.ray(rect, pos);
        let (_, centro) = cam.ray(rect, rect.center());
        PointRef::Fixed(d3(no_vazio(o, d, ancora, centro)))
    }))
}

/// Grava na curva o ponto clicado.
pub fn gravar_ponto(doc: &mut Document, id: Id, p: PointRef) {
    if let Some(rf) = doc.references.iter_mut().find(|x| x.id == id) {
        acrescentar(&mut rf.kind, p);
    }
}

/// Pente com o dente maior em um quinto do comprimento da curva, vezes `escala`. Uma escala fixa em mm
/// não serve: a curvatura muda de ordem de grandeza com a peça, e o pente sairia da tela ou sumiria.
pub fn dentes_do_pente(c: &Curve3, escala: f64) -> Vec<fenkra_solidos::analise::Dente> {
    use fenkra_solidos::analise::{pente, pico};
    let medida = pente(c, 96, 1.);
    match pico(&medida).map(|d| d.k).filter(|k| *k > 1e-9) {
        Some(k) => pente(c, 96, escala * c.length() / 5. / k),
        None => medida,
    }
}

/// Curva calculada como `Curve3`, para o pente.
fn curva(r: &RecomputeResult, id: Id, fechada: bool) -> Option<Curve3> {
    let c = r.curves.iter().find(|c| c.id == id)?;
    Curve3::poly(c.pts.clone(), fechada).ok()
}

/// Posições de todos os pontos, com `None` nos que ainda faltam.
fn posicoes(r: &RecomputeResult, pts: &[PointRef]) -> Vec<Option<[f64; 3]>> {
    pts.iter().map(|p| if *p == crate::slots::point() { None } else { crate::picking::point_pos(r, p) }).collect()
}

/// Elo de corrente: o sinal de "este ponto vem de outra referência".
fn elo(p: &egui::Painter, q: Pos2, cor: egui::Color32) {
    for dx in [-3.5, 3.5] {
        p.circle_stroke(q + egui::vec2(dx + 13., -13.), 4.5, Stroke::new(1.6, cor));
    }
}

/// Tangente natural do ponto `i` em tamanho real de tela: serve de base para a alça.
fn natural(pos: &[[f64; 3]], fechada: bool, i: usize) -> Option<Vec3> {
    v3(geo3d::tangente_natural(pos, fechada, i)).try_normalize()
}

/// Comprimento da alça com tensão 1, em mm: um quarto da distância ao vizinho mais perto.
fn base_da_alca(pos: &[[f64; 3]], i: usize) -> f32 {
    let p = v3(pos[i]);
    let d = |j: usize| (v3(pos[j]) - p).length();
    let mut m = f32::MAX;
    if i > 0 {
        m = m.min(d(i - 1));
    }
    if i + 1 < pos.len() {
        m = m.min(d(i + 1));
    }
    (m / 2.).max(1.)
}

/// Pontos, polígono, pente, gizmo e alça de tangente.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let Some(r) = app.result.as_ref() else { return };
    let Some(k) = app.open.doc.reference(id).map(|x| x.kind.clone()) else { return };
    let Some(pts) = pontos_de(&k) else { return };
    let (fechada, tangentes) = match &k {
        RefKind::Spline { closed, tangents, .. } => (*closed, Some(tangents.clone())),
        RefKind::SplineControl { closed, .. } | RefKind::Polyline { closed, .. } => (*closed, None),
        _ => (false, None),
    };
    let mut e = edicao(ui.ctx(), id);
    let pintor = ui.painter_at(rect);
    let (destaque, fundo, forte) = (crate::theme::accent(ui.visuals()), ui.visuals().extreme_bg_color, ui.visuals().strong_text_color());
    let pos = posicoes(r, pts);
    let tela: Vec<Option<Pos2>> = pos.iter().map(|p| p.and_then(|q| cam.project(rect, v3(q)))).collect();
    // polígono: o caminho dos pontos, tracejado para não ser lido como a curva
    if !matches!(k, RefKind::Polyline { .. }) {
        let mut seq: Vec<Pos2> = tela.iter().flatten().copied().collect();
        if fechada {
            seq.extend(seq.first().copied());
        }
        for w in seq.windows(2) {
            pintor.extend(egui::Shape::dashed_line(&[w[0], w[1]], Stroke::new(1., destaque.gamma_multiply(0.7)), 6., 4.));
        }
    }
    if e.pente {
        if let Some(c) = curva(r, id, fechada) {
            let dentes = dentes_do_pente(&c, e.escala as f64);
            let topo: Vec<Pos2> = dentes.iter().filter_map(|d| cam.project(rect, v3(d.ponta))).collect();
            for d in &dentes {
                if let (Some(a), Some(b)) = (cam.project(rect, v3(d.base)), cam.project(rect, v3(d.ponta))) {
                    pintor.line_segment([a, b], Stroke::new(1., crate::theme::st_warn()));
                }
            }
            pintor.add(egui::Shape::line(topo, Stroke::new(1.5, crate::theme::st_warn())));
        }
    }
    let mut novo_ponto = None;
    for (i, q) in tela.iter().enumerate() {
        let Some(q) = *q else { continue };
        let hr = ui.interact(Rect::from_center_size(q, egui::vec2(18., 18.)), egui::Id::unique(("ponto_spline", id.0, i)), egui::Sense::click());
        let sel = e.ponto == Some(i);
        let tem_tan = tangentes.as_ref().is_some_and(|t| t.get(i).is_some_and(Option::is_some));
        let cor = if preso(&pts[i]) { crate::theme::st_warn() } else if sel || hr.hovered() { forte } else { destaque };
        pintor.circle_filled(q, if sel { 7. } else { 5.5 }, cor);
        pintor.circle_stroke(q, if sel { 7. } else { 5.5 }, Stroke::new(1.5, fundo));
        if preso(&pts[i]) {
            elo(&pintor, q, cor);
        }
        if tem_tan {
            pintor.circle_stroke(q, 9.5, Stroke::new(1.5, crate::theme::st_ok()));
        }
        let dica = if preso(&pts[i]) { tr("Ponto preso a outra referência: ele segue a origem.") } else { tr("Clique para mover este ponto com o gizmo.") };
        if hr.on_hover_text(dica).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            novo_ponto = Some(i);
        }
    }
    if let Some(i) = novo_ponto {
        e.ponto = if e.ponto == Some(i) { None } else { Some(i) };
        guardar(ui.ctx(), id, e);
    }
    let Some(i) = e.ponto.filter(|i| *i < pts.len()) else { return };
    let conhecidos: Option<Vec<[f64; 3]>> = pos.iter().copied().collect();
    let (mut mover, mut alca) = (None, None);
    if let (Some(PointRef::Fixed(p)), Some(_)) = (pts.get(i), tela[i]) {
        let eixos = [Vec3::X, Vec3::Y, Vec3::Z];
        mover = crate::gizmo::translate_gizmo(ui, rect, cam, id.0 as u64 * 4099 + i as u64, v3(*p), eixos, *p, [false; 3]);
    }
    // solta, a alça mostra a tangente que a curva escolheu; puxar a ponta é o que a fixa
    let tangente = |lista: &[[f64; 3]]| tangentes.as_ref()?.get(i).copied().flatten().or_else(|| Some(SplineTan { dir: d3(natural(lista, fechada, i)?), tension: 1. }));
    if let Some((t, lista)) = conhecidos.as_ref().filter(|_| tangentes.is_some()).and_then(|l| Some((tangente(l)?, l))) {
        let (p, l) = (v3(lista[i]), base_da_alca(lista, i));
        let ponta = p + v3(t.dir) * l * t.tension as f32;
        if let (Some(a), Some(b)) = (cam.project(rect, p), cam.project(rect, ponta)) {
            let hr = ui.interact(Rect::from_center_size(b, egui::vec2(crate::handles::PEGA, crate::handles::PEGA)), egui::Id::unique(("tangente_spline", id.0, i)), egui::Sense::drag());
            let cor = if hr.hovered() || hr.dragged() { forte } else { crate::theme::st_ok() };
            pintor.line_segment([a, b], Stroke::new(2., cor));
            pintor.circle_filled(b, 6., cor);
            if hr.dragged() {
                if let Some(m) = ui.input(|x| x.pointer.interact_pos()) {
                    // a ponta corre no plano que passa por ela de frente para a câmera
                    let (o, d) = cam.ray(rect, m);
                    let (_, frente) = cam.ray(rect, b);
                    let alvo = no_vazio(o, d, ponta, frente);
                    alca = tangente_da_ponta(p, alvo, l);
                }
            }
            let _ = hr.on_hover_text(tr("Gire para mudar a tangente; o tamanho é a tensão.")).on_hover_cursor(egui::CursorIcon::Grab);
        }
    }
    if mover.is_some() || alca.is_some() {
        if let Some(rf) = app.open.doc.references.iter_mut().find(|x| x.id == id) {
            if let (Some(v), Some(l)) = (mover, pontos_mut(&mut rf.kind)) {
                l[i] = PointRef::Fixed(v);
            }
            if let (Some(t), RefKind::Spline { points, tangents, .. }) = (alca, &mut rf.kind) {
                tangents.resize(points.len(), None);
                tangents[i] = Some(t);
            }
        }
    }
}

/// Tangente que a ponta da alça representa.
pub fn tangente_da_ponta(p: Vec3, ponta: Vec3, base: f32) -> Option<SplineTan> {
    let v = ponta - p;
    let d = v.try_normalize()?;
    Some(SplineTan { dir: d3(d), tension: ((v.length() / base) as f64).clamp(0.05, 20.) })
}

/// Fixa a tangente do ponto `i` na direção que a curva já tem: a forma não muda.
pub fn fixar(k: &mut RefKind, i: usize, pos: &[[f64; 3]]) {
    let RefKind::Spline { points, closed, tangents, .. } = k else { return };
    tangents.resize(points.len(), None);
    tangents[i] = natural(pos, *closed, i).map(|d| SplineTan { dir: d3(d), tension: 1. });
}

/// Solta a tangente: a curva volta a escolher sozinha.
pub fn soltar(k: &mut RefKind, i: usize) {
    if let RefKind::Spline { tangents, .. } = k {
        if let Some(t) = tangents.get_mut(i) {
            *t = None;
        }
        while tangents.last().is_some_and(Option::is_none) {
            tangents.pop();
        }
    }
}

/// Bloco do painel: ponto escolhido, tangente dele e pente. `pos` são os pontos que o cálculo resolveu.
pub fn painel(ui: &mut egui::Ui, id: Id, k: &mut RefKind, pos: &[[f64; 3]]) {
    let mut e = edicao(ui.ctx(), id);
    ui.label(RichText::new(tr("Clique no 3D para acrescentar pontos; clique num ponto para movê-lo.")).weak().small());
    ui.checkbox(&mut e.pente, tr("pente de curvatura"));
    if e.pente {
        ui.add(crate::ui_ext::slider(&mut e.escala, 0.1..=5.).logarithmic(true).text(tr("Escala"))).on_hover_text(tr("Tamanho dos dentes do pente: não muda a curva."));
    }
    guardar(ui.ctx(), id, e);
    let Some(i) = e.ponto else { return };
    let Some(pts) = pontos_de(k).cloned() else { return };
    if i >= pts.len() || !matches!(k, RefKind::Spline { .. }) {
        return;
    }
    let tem = matches!(k, RefKind::Spline { tangents, .. } if tangents.get(i).is_some_and(Option::is_some));
    let mut liga = tem;
    if ui.checkbox(&mut liga, format!("{} {}", tr("fixar a tangente do ponto"), i + 1)).on_hover_text(tr("Guarda a direção de agora; depois gire a alça no 3D.")).changed() {
        if liga {
            if pos.len() == pts.len() {
                fixar(k, i, pos);
            }
        } else {
            soltar(k, i);
        }
    }
    if let RefKind::Spline { tangents, .. } = k {
        if let Some(Some(t)) = tangents.get_mut(i) {
            ui.add(crate::ui_ext::slider(&mut t.tension, 0.05..=20.).logarithmic(true).text(tr("Tensão")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_solidos::{recompute, Status};

    fn spline(pts: &[[f64; 3]]) -> RefKind {
        RefKind::Spline { points: pts.iter().map(|p| PointRef::Fixed(*p)).collect(), closed: false, tangents: vec![] }
    }

    fn curva_de(d: &Document, id: Id) -> Vec<[f64; 3]> {
        let r = recompute(d);
        assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{:?}", r.status.get(&id));
        r.curves.iter().find(|c| c.id == id).unwrap().pts.clone()
    }

    fn tipo(d: &mut Document, id: Id) -> &mut RefKind {
        &mut d.references.iter_mut().find(|r| r.id == id).unwrap().kind
    }

    fn igual(a: &[[f64; 3]], b: &[[f64; 3]], tol: f64) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(p, q)| geo3d::dist(*p, *q) < tol)
    }

    const P: [[f64; 3]; 4] = [[0., 0., 0.], [30., 20., 5.], [60., -10., 10.], [90., 15., 0.]];

    /// Aceite, na ordem inversa: fixar a tangência, arrastar e voltar o ponto, sem soltar nada.
    #[test]
    fn fixar_arrastar_e_voltar_reproduz_a_original_com_a_tangente_presa() {
        let mut d = Document::new("spline");
        let id = d.add_geometry(spline(&P));
        let original = curva_de(&d, id);
        fixar(tipo(&mut d, id), 1, &P);
        assert!(igual(&original, &curva_de(&d, id), 1e-6), "fixar onde ela está não muda a forma");
        if let Some(v) = pontos_mut(tipo(&mut d, id)) {
            v[2] = PointRef::Fixed([60., 30., 10.]);
        }
        let movida = curva_de(&d, id);
        assert!(!igual(&original, &movida, 1.));
        // a direção fixada aguenta o arraste do vizinho
        let antes = geo3d::unit(geo3d::tangente_natural(&P, false, 1)).unwrap();
        let i = 24;
        let saida = geo3d::unit(geo3d::sub(movida[i + 1], movida[i])).unwrap();
        assert!(geo3d::dot(antes, saida) > 0.99, "{antes:?} {saida:?}");
        if let Some(v) = pontos_mut(tipo(&mut d, id)) {
            v[2] = PointRef::Fixed(P[2]);
        }
        assert!(igual(&original, &curva_de(&d, id), 1e-6));
    }

    /// Aceite: arrastar um ponto, fixar a tangência e voltar reproduz a curva original.
    #[test]
    fn arrastar_fixar_e_voltar_reproduz_a_original() {
        let mut d = Document::new("spline");
        let id = d.add_geometry(spline(&P));
        let original = curva_de(&d, id);
        // arrasta o ponto 2
        if let Some(v) = pontos_mut(tipo(&mut d, id)) {
            v[1] = PointRef::Fixed([30., 40., 5.]);
        }
        let movida = curva_de(&d, id);
        assert!(!igual(&original, &movida, 1.), "o arraste muda a curva");
        // fixa a tangência onde ela está e volta o ponto
        let pos = [P[0], [30., 40., 5.], P[2], P[3]];
        fixar(tipo(&mut d, id), 1, &pos);
        assert!(igual(&movida, &curva_de(&d, id), 1e-6), "fixar a tangente onde ela está não muda a forma");
        if let Some(v) = pontos_mut(tipo(&mut d, id)) {
            v[1] = PointRef::Fixed(P[1]);
        }
        soltar(tipo(&mut d, id), 1);
        assert!(igual(&original, &curva_de(&d, id), 1e-9));
        let RefKind::Spline { tangents, .. } = tipo(&mut d, id) else { panic!() };
        assert!(tangents.is_empty(), "soltar limpa a lista");
    }

    #[test]
    fn tangente_fixada_vira_a_saida_da_curva() {
        let mut d = Document::new("tan");
        let id = d.add_geometry(spline(&P));
        let k = &mut d.references.iter_mut().find(|r| r.id == id).unwrap().kind;
        let RefKind::Spline { tangents, .. } = k else { panic!() };
        *tangents = vec![Some(SplineTan { dir: [0., 0., 1.], tension: 2. })];
        let c = curva_de(&d, id);
        assert!(geo3d::dist(c[0], P[0]) < 1e-9, "a curva segue passando pelo ponto");
        let t = geo3d::unit(geo3d::sub(c[1], c[0])).unwrap();
        assert!(t[2] > 0.95, "{t:?}");
    }

    /// Aceite: salvar e abrir preserva as tangentes.
    #[test]
    fn salvar_e_abrir_preserva_tangentes() {
        let mut d = Document::new("salvo");
        let id = d.add_geometry(RefKind::Spline { points: P.iter().map(|p| PointRef::Fixed(*p)).collect(), closed: false, tangents: vec![None, Some(SplineTan { dir: [0., 1., 0.], tension: 1.5 })] });
        let antes = curva_de(&d, id);
        let aberto = Document::from_json(&d.to_json()).unwrap();
        assert_eq!(aberto.reference(id).unwrap().kind, d.reference(id).unwrap().kind);
        assert!(igual(&antes, &curva_de(&aberto, id), 1e-12));
    }

    #[test]
    fn arquivo_antigo_abre_sem_tangentes() {
        let mut d = Document::new("velho");
        let id = d.add_geometry(spline(&P));
        let texto = serde_json::to_string(&d).unwrap().replace(",\"tangents\":[]", "");
        assert!(!texto.contains("tangents"));
        let aberto = Document::from_json(&texto).unwrap();
        assert!(matches!(&aberto.reference(id).unwrap().kind, RefKind::Spline { tangents, .. } if tangents.is_empty()));
    }

    #[test]
    fn pente_tem_o_mesmo_tamanho_em_qualquer_escala_da_peca() {
        let dente = |k: f64| {
            let p: Vec<[f64; 3]> = P.iter().map(|q| q.map(|x| x * k)).collect();
            let c = geo3d::spline(&p, false).unwrap();
            let maior = dentes_do_pente(&c, 1.).iter().map(|d| geo3d::dist(d.base, d.ponta)).fold(0., f64::max);
            maior / c.length()
        };
        assert!((dente(1.) - 0.2).abs() < 1e-6 && (dente(1000.) - 0.2).abs() < 1e-6, "{} {}", dente(1.), dente(1000.));
        // reta não tem curvatura: o pente fica rente
        let reta = geo3d::spline(&[[0.; 3], [10., 0., 0.], [20., 0., 0.]], false).unwrap();
        assert!(dentes_do_pente(&reta, 1.).iter().all(|d| geo3d::dist(d.base, d.ponta) < 1e-6));
    }

    #[test]
    fn cliques_ocupam_os_campos_vazios_e_depois_crescem() {
        let u = crate::slots::point();
        let mut k = RefKind::Polyline { points: vec![u, u], closed: false };
        for x in 0..3 {
            acrescentar(&mut k, PointRef::Fixed([x as f64, 0., 0.]));
        }
        assert_eq!(pontos_de(&k).unwrap(), &vec![PointRef::Fixed([0.; 3]), PointRef::Fixed([1., 0., 0.]), PointRef::Fixed([2., 0., 0.])]);
    }

    #[test]
    fn alvo_de_referencia_fica_preso_e_face_nao() {
        let v = |_: usize, q: [f64; 3]| PointRef::Fixed(q);
        let geo = ponto_do_alvo(Some(Pick::Geo(Id(5))), v, |_| true).unwrap();
        assert_eq!(geo, PointRef::Ref(Id(5)));
        assert!(preso(&geo));
        let c = ponto_do_alvo(Some(Pick::CurveVtx { curve: Id(5), index: 1, pos: [0.; 3] }), v, |_| true).unwrap();
        assert!(preso(&c));
        let f = ponto_do_alvo(Some(Pick::Face { body: 0, face: 0, owner: Id(1), point: [1., 2., 3.], normal: [0., 0., 1.] }), v, |_| true).unwrap();
        assert_eq!(f, PointRef::Fixed([1., 2., 3.]));
        assert!(!preso(&f));
        assert_eq!(ponto_do_alvo(None, v, |_| true), None);
    }

    #[test]
    fn clique_no_vazio_cai_no_plano_do_ultimo_ponto() {
        let p = no_vazio(Vec3::new(5., 5., 100.), Vec3::NEG_Z, Vec3::new(0., 0., 7.), Vec3::NEG_Z);
        assert!((p - Vec3::new(5., 5., 7.)).length() < 1e-4);
    }

    #[test]
    fn alca_vira_direcao_e_tensao() {
        let t = tangente_da_ponta(Vec3::ZERO, Vec3::new(0., 20., 0.), 10.).unwrap();
        assert_eq!(t.dir, [0., 1., 0.]);
        assert!((t.tension - 2.).abs() < 1e-6);
        assert!(tangente_da_ponta(Vec3::ONE, Vec3::ONE, 10.).is_none());
    }
}
