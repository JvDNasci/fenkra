// SPDX-License-Identifier: LGPL-2.1-or-later
//! Compasso (B438): o cubo de vistas do canto superior direito sai do canto. Arrastado para fora do
//! cubo e solto na peça, ele gruda numa face (Z na normal), numa aresta reta (Z ao longo dela) ou num
//! eixo (face cilíndrica ou aresta circular: Z no eixo, origem sobre ele) e se orienta por ela.
//!
//! Preso, o compasso é a régua do manipulador da montagem: as setas e os anéis nascem na origem dele,
//! nos eixos dele, e giram em volta dela. O painel do compasso, na coluna da esquerda, aceita posição e
//! ângulo digitados, em coordenada incremental ou absoluta, com referência local (os eixos do compasso)
//! ou global (os da montagem), e tem os botões de incremento por eixo.
//!
//! O cubo continua girando a vista: no canto e também preso, ao lado da origem. Arrastá-lo de volta ao
//! canto, o botão do painel ou o atalho devolvem o compasso ao canto.
//!
//! Como a conta funciona: o quadro do compasso é um `Placement` (origem e três eixos). Preso numa peça
//! da montagem, ele fica guardado no sistema da peça, então acompanha a peça quando ela anda. Mover
//! pelo painel monta a transformação rígida T pedida (incremental: translação nos eixos escolhidos e
//! giros em volta da origem do compasso; absoluta: T leva o quadro atual ao quadro digitado) e cada
//! peça escolhida vai para T aplicada depois da posição dela.

use crate::app::App;
use crate::i18n::tr;
use crate::picking::Pick;
use crate::render::Camera;
use eframe::egui::{self, Pos2, Rect, RichText, Stroke};
use fenkra_core::Id;
use fenkra_montagem::{cross, dot, norm, sub, unit, Assembly, Geom, NodeKind, Placement, V3, ORIGEM};
use glam::Vec3;

const IDENT: [V3; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
/// lado do cubo preso, ao lado da origem do compasso (px)
const MINI: f32 = 46.;

/// No que o compasso está preso.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tipo {
    Face,
    Aresta,
    Eixo,
}

impl Tipo {
    /// Rótulo do compasso já preso.
    pub fn preso(self) -> &'static str {
        match self {
            Tipo::Face => "Preso na face",
            Tipo::Aresta => "Preso na aresta",
            Tipo::Eixo => "Preso no eixo",
        }
    }

    /// Prévia sob o mouse, antes de soltar.
    pub fn prender(self) -> &'static str {
        match self {
            Tipo::Face => "Prender na face",
            Tipo::Aresta => "Prender na aresta",
            Tipo::Eixo => "Prender no eixo",
        }
    }
}

/// Coordenada digitada no painel.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Modo {
    #[default]
    Incremental,
    Absoluta,
}

/// Eixos em que a coordenada é medida.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Base {
    /// eixos do compasso
    #[default]
    Local,
    /// eixos da montagem
    Global,
}

/// Onde o compasso está preso.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ancora {
    pub tipo: Tipo,
    /// componente que leva o compasso junto; `None`: preso no mundo (peça aberta ou origem)
    pub dono: Option<Id>,
    /// quadro no sistema do dono (ou no mundo, sem dono)
    pub local: Placement,
    /// quadro no mundo no momento em que foi preso: a referência local da coordenada absoluta
    pub base: Placement,
}

impl Ancora {
    /// Quadro do compasso no mundo agora.
    pub fn mundo(&self, asm: Option<&Assembly>) -> Placement {
        match (self.dono, asm) {
            (Some(id), Some(a)) => a.node(id).map_or(self.base, |n| n.placement.then(&self.local)),
            _ => self.base,
        }
    }
}

/// Estado do compasso.
#[derive(Clone, Default)]
pub struct Compasso {
    pub ancora: Option<Ancora>,
    pub modo: Modo,
    pub base: Base,
    pub pos: [f64; 3],
    pub ang: [f64; 3],
    pub passo: f64,
    pub passo_giro: f64,
    /// o cubo está sendo levado (do canto ou de onde estava preso)
    levando: bool,
    /// câmera no aperto do botão sobre o cubo: levar o cubo não pode girar a vista
    cam0: Option<(f32, f32)>,
    /// última conta do alvo sob o mouse
    ultimo: Option<(Pick, Option<Ancora>)>,
    /// vista pedida pelo painel ("olhar pela normal")
    olhar: Option<Vec3>,
}

impl Compasso {
    pub fn novo() -> Self {
        Compasso { passo: 10., passo_giro: 15., ..Default::default() }
    }

    /// Devolve o compasso ao canto.
    pub fn devolver(&mut self) {
        self.ancora = None;
        self.levando = false;
        self.cam0 = None;
        self.ultimo = None;
    }

    pub fn preso(&self) -> bool {
        self.ancora.is_some()
    }
}

fn v3(p: V3) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn esc(a: V3, k: f64) -> V3 {
    a.map(|v| v * k)
}

/// Quadro com Z em `z`. X é o eixo do mundo menos alinhado com Z, tirada a parte em Z: numa face do
/// topo o compasso fica com X e Y do mundo, sem giro gratuito.
pub fn quadro(origem: V3, z: V3) -> Placement {
    let z = unit(z);
    let a = *IDENT.iter().min_by(|a, b| dot(**a, z).abs().total_cmp(&dot(**b, z).abs())).unwrap();
    let x = unit(sub(a, esc(z, dot(a, z))));
    Placement { origin: origem, rot: [x, cross(z, x), z] }
}

/// Âncora a partir da geometria do mundo: `ponto` é onde o mouse soltou; `p0` e `d` são o plano (ponto e
/// normal) ou a reta (ponto e direção). A origem é o ponto solto levado para o plano ou para a reta.
pub fn ancorar(tipo: Tipo, ponto: V3, p0: V3, d: V3, dono: Option<(Id, Placement)>) -> Ancora {
    let d = unit(d);
    let origem = match tipo {
        Tipo::Face => sub(ponto, esc(d, dot(sub(ponto, p0), d))),
        Tipo::Aresta | Tipo::Eixo => add(p0, esc(d, dot(sub(ponto, p0), d))),
    };
    let q = quadro(origem, d);
    let local = dono.map_or(q, |(_, pl)| pl.inverse().then(&q));
    Ancora { tipo, dono: dono.map(|x| x.0), local, base: q }
}

/// Reta sobre a qual `ponto` está? (aresta reta) Senão, é o eixo de um círculo ou de um cilindro.
fn sobre_a_reta(ponto: V3, p0: V3, d: V3) -> bool {
    let r = sub(ponto, p0);
    norm(cross(r, unit(d))) < 1e-3 * (1. + norm(r))
}

/// Geometria do que está sob o mouse, numa peça aberta (sem montagem).
fn geometria_peca(r: &fenkra_solidos::RecomputeResult, pick: Pick) -> Option<(Tipo, V3, V3, V3)> {
    let f = |p: [f32; 3]| p.map(|v| v as f64);
    match pick {
        Pick::Face { point, normal, .. } => Some((Tipo::Face, f(point), f(point), f(normal))),
        Pick::Edge { body, edge, mid } => {
            let pts = crate::picking::edge_polyline(r, body, edge);
            let (a, b) = (*pts.first()?, *pts.last()?);
            let c = fenkra_solidos::geo3d::Curve3::poly(pts, false).ok()?;
            if c.is_straight() {
                Some((Tipo::Aresta, f(mid), a, unit(sub(b, a))))
            } else {
                let (ctr, n, _) = fenkra_solidos::geo3d::fit_circle(&c).ok()?;
                Some((Tipo::Eixo, f(mid), ctr, unit(n)))
            }
        }
        Pick::Geo(id) => {
            let c = r.curves.iter().find(|c| c.id == id)?;
            let (a, b) = (*c.pts.first()?, *c.pts.last()?);
            let d = sub(b, a);
            (norm(d) > 1e-9).then(|| (Tipo::Eixo, a, a, unit(d)))
        }
        _ => None,
    }
}

/// Âncora para o que está sob o mouse. Na montagem, a geometria vem da mesma leitura das restrições
/// (face plana, cilindro, aresta reta ou circular) e o compasso fica preso no componente.
pub fn ancora_do_pick(app: &mut App, pick: Pick) -> Option<Ancora> {
    ancora_do_pick_em(app, pick, false)
}

/// `na_vista`: chamado durante o desenho da viewport. Nesse trecho a vista da montagem está trocada com
/// `app.result` (o 3D desenha a montagem como se fosse um documento), e a leitura das restrições acha o
/// componente de cada corpo em `st.view`: destroca enquanto lê.
fn ancora_do_pick_em(app: &mut App, pick: Pick, na_vista: bool) -> Option<Ancora> {
    if na_vista && app.in_asm() {
        let st = app.asm.as_mut()?;
        std::mem::swap(&mut app.result, &mut st.view);
        let a = ancora_do_pick_em(app, pick, false);
        let st = app.asm.as_mut()?;
        std::mem::swap(&mut app.result, &mut st.view);
        return a;
    }
    if app.in_asm() {
        let st = app.asm.as_mut()?;
        let ponto = match pick {
            Pick::Face { point, .. } => point.map(|v| v as f64),
            Pick::Edge { mid, .. } => mid.map(|v| v as f64),
            Pick::Vertex { .. } => return None,
            _ => [0.; 3],
        };
        let r = st.mate_ref(pick).ok()?;
        let (pl, dono) = if r.comp == ORIGEM {
            (Placement::default(), None)
        } else {
            let pl = st.asm.node(r.comp)?.placement;
            (pl, Some((r.comp, pl)))
        };
        return match r.geom {
            Geom::Plane { p, n } => Some(ancorar(Tipo::Face, ponto, pl.point(p), pl.dir(n), dono)),
            Geom::Axis { p, d } if norm(d) > 1e-9 => {
                let (p0, d) = (pl.point(p), pl.dir(d));
                let tipo = if matches!(pick, Pick::Edge { .. }) && sobre_a_reta(ponto, p0, d) { Tipo::Aresta } else { Tipo::Eixo };
                Some(ancorar(tipo, ponto, p0, d, dono))
            }
            _ => None,
        };
    }
    let r = app.result.as_ref()?;
    let (tipo, ponto, p0, d) = geometria_peca(r, pick)?;
    Some(ancorar(tipo, ponto, p0, d, None))
}

/// Ângulos (graus) de giro em X, depois Y, depois Z, todos em eixos fixos, da matriz de colunas `m`.
pub fn angulos(m: [V3; 3]) -> [f64; 3] {
    let mat = glam::DMat3::from_cols(glam::DVec3::from(m[0]), glam::DVec3::from(m[1]), glam::DVec3::from(m[2]));
    let (z, y, x) = glam::DQuat::from_mat3(&mat).to_euler(glam::EulerRot::ZYX);
    let limpo = |a: f64| {
        let g = a.to_degrees();
        if g.abs() < 1e-9 {
            0.
        } else {
            g
        }
    };
    [limpo(x), limpo(y), limpo(z)]
}

fn de_angulos(a: [f64; 3]) -> [V3; 3] {
    let m = glam::DMat3::from_euler(glam::EulerRot::ZYX, a[2].to_radians(), a[1].to_radians(), a[0].to_radians());
    [m.x_axis.to_array(), m.y_axis.to_array(), m.z_axis.to_array()]
}

fn referencia(base: Base, ancora_base: &Placement) -> Placement {
    match base {
        Base::Local => *ancora_base,
        Base::Global => Placement::default(),
    }
}

/// Posição e ângulos do compasso medidos na referência escolhida: o que a coordenada absoluta mostra.
pub fn atuais(q: &Placement, ancora_base: &Placement, base: Base) -> ([f64; 3], [f64; 3]) {
    let f = referencia(base, ancora_base);
    (f.to_local(q.origin), angulos(q.rot.map(|c| f.dir_to_local(c))))
}

/// Transformação rígida (no mundo) que o painel pede, para o compasso no quadro `q`.
/// Incremental: anda `pos` nos eixos da referência e gira `ang` em volta deles, pela origem do
/// compasso já deslocada (X, depois Y, depois Z). Absoluta: leva o quadro do compasso ao quadro com
/// origem `pos` e giros `ang` na referência.
pub fn pedido(q: &Placement, ancora_base: &Placement, modo: Modo, base: Base, pos: [f64; 3], ang: [f64; 3]) -> Placement {
    match modo {
        Modo::Incremental => {
            let e = if base == Base::Local { q.rot } else { IDENT };
            let t = [0, 1, 2].map(|i| e[0][i] * pos[0] + e[1][i] * pos[1] + e[2][i] * pos[2]);
            let centro = add(q.origin, t);
            let mut tr = Placement { origin: t, rot: IDENT };
            for k in 0..3 {
                if ang[k] != 0. {
                    tr = tr.moved([0.; 3], esc(e[k], ang[k].to_radians()), centro);
                }
            }
            tr
        }
        Modo::Absoluta => {
            let f = referencia(base, ancora_base);
            let g = Placement { origin: f.point(pos), rot: de_angulos(ang).map(|c| f.dir(c)) };
            g.then(&q.inverse())
        }
    }
}

/// Componentes que o compasso move: a escolha da montagem, sem os fixos nem os grupos.
fn escolhidas(app: &App) -> Vec<Id> {
    let Some(st) = app.asm.as_ref().filter(|_| app.in_asm()) else { return vec![] };
    let mut v: Vec<Id> = st.sel.into_iter().chain(st.multi.iter().copied()).chain(st.comps_of(&app.picks)).collect();
    let mut vistos = std::collections::HashSet::new();
    v.retain(|id| vistos.insert(*id) && st.asm.node(*id).is_some_and(|n| !n.fixed && !matches!(n.kind, NodeKind::Group)));
    v
}

/// Aplica a transformação `t` às peças escolhidas. Devolve quantas foram. Se o compasso estava preso
/// em outra peça, ele passa para a primeira movida, para seguir a mão.
pub fn aplicar(app: &mut App, t: Placement) -> usize {
    let ids = escolhidas(app);
    let Some(anc) = app.compasso.ancora else { return 0 };
    let Some(st) = app.asm.as_mut() else { return 0 };
    if ids.is_empty() {
        return 0;
    }
    let q = anc.mundo(Some(&st.asm));
    let primeira = st.asm.node(ids[0]).map(|n| n.placement);
    for id in &ids {
        let Some(p) = st.asm.node(*id).map(|n| n.placement) else { continue };
        crate::asm_manip::aplicar(st, *id, t.then(&p), &ids);
    }
    if !anc.dono.is_some_and(|d| ids.contains(&d)) {
        if let Some(p) = primeira {
            app.compasso.ancora = Some(Ancora { dono: Some(ids[0]), local: p.inverse().then(&q), ..anc });
        }
    }
    ids.len()
}

/// O arrasto das alças vai começar: o compasso passa a seguir a peça que anda.
pub fn seguir(c: &mut Compasso, asm: &Assembly, id: Id) {
    let Some(anc) = c.ancora else { return };
    if anc.dono == Some(id) {
        return;
    }
    let Some(p) = asm.node(id).map(|n| n.placement) else { return };
    let q = anc.mundo(Some(asm));
    c.ancora = Some(Ancora { dono: Some(id), local: p.inverse().then(&q), ..anc });
}

/// Quadro do compasso preso, no mundo, para o manipulador da montagem.
pub fn quadro_na_montagem(c: &Compasso, asm: &Assembly) -> Option<Placement> {
    c.ancora.map(|a| a.mundo(Some(asm)))
}

fn asm_do_app(app: &App) -> Option<&Assembly> {
    app.asm.as_ref().filter(|_| app.in_asm()).map(|s| &s.asm)
}

/// Lugar do cubo no canto superior direito da área 3D.
pub fn area_do_canto(rect: Rect) -> Rect {
    Rect::from_min_size(egui::pos2(rect.right() - 86., rect.top() + 6.), egui::vec2(80., 80.))
}

/// Eixos do compasso na tela, com as letras.
fn pinta_quadro(p: &egui::Painter, cam: &Camera, rect: Rect, q: &Placement, forte: bool) {
    let Some(o) = cam.project(rect, v3(q.origin)) else { return };
    for k in 0..3 {
        let Some((_, tela, _)) = crate::gizmo::screen_axis(cam, rect, v3(q.origin), v3(q.rot[k])) else { continue };
        let cor = crate::gizmo::AXIS_COL[k].gamma_multiply(if forte { 1. } else { 0.75 });
        let ponta = o + tela * crate::gizmo::ARM * 0.62;
        p.line_segment([o, ponta], Stroke::new(if forte { 2.4 } else { 1.6 }, cor));
        p.text(ponta + tela * 9., egui::Align2::CENTER_CENTER, ["X", "Y", "Z"][k], egui::FontId::proportional(crate::acessibilidade::fonte_3d(11.)), cor);
    }
    p.circle_filled(o, 3.5, crate::gizmo::AXIS_COL[2]);
}

fn aviso(p: &egui::Painter, ui: &egui::Ui, em: Pos2, txt: &str) {
    let v = ui.visuals();
    let g = p.layout_no_wrap(txt.to_string(), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), v.text_color());
    let caixa = Rect::from_min_size(em, g.size() + egui::vec2(10., 6.));
    p.rect_filled(caixa, 4., v.window_fill.gamma_multiply(0.92));
    p.rect_stroke(caixa, 4., Stroke::new(1., crate::theme::accent(v)), egui::StrokeKind::Inside);
    p.galley(caixa.min + egui::vec2(5., 3.), g, v.text_color());
}

/// Alvo sob o mouse enquanto o cubo é levado (com memória do último, a conta da face é cara).
fn alvo(app: &mut App, cam: &Camera, rect: Rect, em: Pos2) -> Option<Ancora> {
    let r = app.result.as_ref()?;
    let filt = crate::picking::Filter { faces: true, edges: true, vertices: false, planes: true, hidden_env: app.hidden_env, exclude_sketch: None, fantasma: None, planos_origem: app.settings.filter.planos_origem, planos_ref: app.settings.filter.planos_ref };
    let caixas = app.asm.as_ref().filter(|_| app.in_asm()).map(|s| s.caixas.as_slice());
    let pick = crate::picking::pick_com_caixas(r, &app.open.doc, cam, rect, em, &filt, caixas)?;
    if let Some((p, a)) = app.compasso.ultimo {
        if p == pick {
            return a;
        }
    }
    let a = ancora_do_pick_em(app, pick, true);
    app.compasso.ultimo = Some((pick, a));
    a
}

/// Prende o compasso: escolhe o dono se nada estiver escolhido e põe o painel em ordem.
pub fn prender(app: &mut App, a: Ancora) {
    app.compasso.ancora = Some(a);
    if let (Some(d), Some(st)) = (a.dono, app.asm.as_mut()) {
        if st.sel.is_none() && st.asm.node(d).is_some_and(|n| !n.fixed) {
            st.sel = Some(d);
        }
    }
    app.compasso.pos = [0.; 3];
    app.compasso.ang = [0.; 3];
    app.compasso.modo = Modo::Incremental;
    crate::curso::anotar("compasso_preso");
}

/// Desenha o cubo (no canto ou preso) e cuida de levá-lo. Devolve a vista escolhida num clique.
pub fn canto(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) -> Option<Vec3> {
    let lugar_canto = area_do_canto(rect);
    let q = app.compasso.ancora.map(|a| a.mundo(asm_do_app(app)));
    let origem_tela = q.and_then(|q| cam.project(rect, v3(q.origin)));
    // preso: o cubo fica ao lado da origem, ligado a ela por um traço
    let area = match origem_tela {
        Some(o) => Rect::from_center_size(o + egui::vec2(-MINI, -MINI), egui::vec2(MINI, MINI)),
        None => lugar_canto,
    };
    let (ponteiro, apertou, solto, aperto, baixo) = ui.input(|i| (i.pointer.latest_pos(), i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.press_origin(), i.pointer.primary_down()));
    if app.settings.keys.pressed(ui.ctx(), crate::keymap::KeyAction::CompassoCanto) && !ui.ctx().egui_wants_keyboard_input() {
        app.compasso.devolver();
    }
    if apertou && aperto.is_some_and(|p| area.contains(p)) {
        app.compasso.cam0 = Some((cam.yaw, cam.pitch));
    }
    let mut vista = app.compasso.olhar.take();
    let pintor = ui.painter_at(rect);
    let v = ui.visuals().clone();
    if q.is_some() {
        // o lugar vazio do canto: soltar aqui devolve o compasso
        let quente = app.compasso.levando && ponteiro.is_some_and(|p| lugar_canto.contains(p));
        let cor = if quente { crate::theme::accent(&v) } else { v.weak_text_color() };
        let borda = [lugar_canto.left_top(), lugar_canto.right_top(), lugar_canto.right_bottom(), lugar_canto.left_bottom(), lugar_canto.left_top()];
        for w in borda.windows(2) {
            pintor.extend(egui::Shape::dashed_line(&[w[0], w[1]], Stroke::new(1., cor), 5., 4.));
        }
        let r = ui.interact(lugar_canto, egui::Id::unique("compasso_lugar"), egui::Sense::click()).on_hover_text(tr("Clique ou solte o compasso aqui para devolvê-lo ao canto"));
        if r.clicked() {
            app.compasso.devolver();
        }
    }
    if let (Some(q), Some(o)) = (q.filter(|_| !app.compasso.levando), origem_tela) {
        pinta_quadro(&pintor, cam, rect, &q, false);
        pintor.line_segment([o, area.center()], Stroke::new(1., v.weak_text_color()));
    }
    if !app.compasso.levando {
        let nome = if q.is_some() { "preso" } else { "canto" };
        let (dir, giro, resp) = crate::viewcube::cube_resp(ui, area, cam, nome, false);
        let fora = ponteiro.is_some_and(|p| !area.expand(6.).contains(p));
        if baixo && app.compasso.cam0.is_some() && fora {
            // saiu do cubo com o botão apertado: é o compasso que vai, não a vista que gira
            app.compasso.levando = true;
            app.compasso.ultimo = None;
            if let (Some((y, p)), Some(vp)) = (app.compasso.cam0, app.viewport.as_mut()) {
                vp.camera.yaw = y;
                vp.camera.pitch = p;
            }
        } else {
            if dir.is_some() {
                crate::curso::anotar("vista_cubo");
                vista = dir;
            }
            if let Some(vp) = app.viewport.as_mut() {
                vp.camera.yaw -= giro.x * 0.008;
                vp.camera.pitch = (vp.camera.pitch + giro.y * 0.008).clamp(-1.5706, 1.5706);
            }
        }
        resp.on_hover_text(tr("Clique numa face do cubo para a vista; arraste dentro dele para girar; arraste para fora e solte numa face, aresta ou eixo para prender o compasso"));
    }
    if app.compasso.levando {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        let no_canto = ponteiro.is_some_and(|p| lugar_canto.contains(p));
        let candidato = ponteiro.filter(|p| rect.contains(*p) && !no_canto).and_then(|p| alvo(app, cam, rect, p));
        if let Some(p) = ponteiro {
            match &candidato {
                // prévia no 3D: onde e como o compasso vai ficar
                Some(a) => {
                    let qa = a.mundo(asm_do_app(app));
                    pinta_quadro(&pintor, cam, rect, &qa, true);
                    aviso(&pintor, ui, p + egui::vec2(16., 14.), tr(a.tipo.prender()));
                }
                None if no_canto => aviso(&pintor, ui, p + egui::vec2(-200., 14.), tr("Solte para devolver ao canto")),
                None => {
                    pintor.circle_stroke(p, 9., Stroke::new(1.5, crate::theme::accent(&v)));
                    aviso(&pintor, ui, p + egui::vec2(16., 14.), tr("Solte numa face, aresta ou eixo"));
                }
            }
        }
        if solto || !baixo {
            if no_canto {
                app.compasso.devolver();
            } else if let Some(a) = candidato {
                prender(app, a);
            }
            app.compasso.levando = false;
            app.compasso.ultimo = None;
        }
    }
    if !baixo {
        app.compasso.cam0 = None;
    }
    vista
}

/// O painel do compasso vale como comando enquanto ele está preso.
pub fn painel_aberto(app: &App) -> bool {
    app.compasso.preso() && app.settings.bench != crate::settings::Bench::Desenho
}

fn campo(ui: &mut egui::Ui, v: &mut f64, sufixo: &str, dica: &str) -> bool {
    ui.add(crate::ui_ext::dv(v).speed(0.1).suffix(sufixo).dica(dica)).changed()
}

/// Painel do compasso, na coluna da esquerda.
pub fn painel(app: &mut App, ui: &mut egui::Ui) {
    let Some(anc) = app.compasso.ancora else { return };
    let q = anc.mundo(asm_do_app(app));
    let dono = anc.dono.and_then(|d| app.asm.as_ref().and_then(|s| s.asm.node(d)).map(|n| n.name.clone()));
    let onde = match dono {
        Some(n) => format!("{} · {n}", tr(anc.tipo.preso())),
        None => tr(anc.tipo.preso()).to_string(),
    };
    ui.label(onde);
    let ids = escolhidas(app);
    let montagem = app.in_asm();
    if !montagem {
        ui.label(RichText::new(tr("Na peça aberta, o compasso orienta a vista. Mover e girar valem para as peças da montagem.")).small().weak());
    } else if ids.is_empty() {
        ui.label(RichText::new(tr("Escolha as peças a mover no 3D ou na árvore")).small().color(crate::theme::st_warn()));
    } else {
        ui.label(RichText::new(format!("{} {}", ids.len(), tr("peça(s) a mover"))).small());
    }
    ui.separator();
    let c = &mut app.compasso;
    let (modo0, base0) = (c.modo, c.base);
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Coordenada")).small());
        ui.selectable_value(&mut c.modo, Modo::Incremental, tr("Incremental")).on_hover_text(tr("Quanto andar e girar a partir de onde está"));
        ui.selectable_value(&mut c.modo, Modo::Absoluta, tr("Absoluta")).on_hover_text(tr("Onde o compasso deve ficar: posição e giros na referência"));
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Referência")).small());
        ui.selectable_value(&mut c.base, Base::Local, tr("Local")).on_hover_text(tr("Eixos do compasso; na absoluta, o lugar onde ele foi preso"));
        ui.selectable_value(&mut c.base, Base::Global, tr("Global")).on_hover_text(tr("Eixos e origem da montagem"));
    });
    if (c.modo, c.base) != (modo0, base0) {
        // absoluta começa no valor de agora; incremental começa do zero
        (c.pos, c.ang) = if c.modo == Modo::Absoluta { atuais(&q, &anc.base, c.base) } else { ([0.; 3], [0.; 3]) };
    }
    let mut passo: Option<(usize, f64, bool)> = None;
    egui::Grid::new("compasso_campos").num_columns(4).spacing([6., 4.]).show(ui, |ui| {
        ui.label("");
        ui.label(RichText::new(tr("Posição")).small());
        ui.label(RichText::new(tr("Ângulo")).small());
        ui.label(RichText::new(tr("Incrementos")).small());
        ui.end_row();
        for k in 0..3 {
            ui.label(RichText::new(["X", "Y", "Z"][k]).color(crate::gizmo::AXIS_COL[k]).strong());
            campo(ui, &mut c.pos[k], " mm", "Distância neste eixo");
            campo(ui, &mut c.ang[k], "°", "Giro em volta deste eixo, pela origem do compasso");
            ui.horizontal(|ui| {
                for (s, txt, dica) in [(-1., "−", "Anda um passo para trás neste eixo"), (1., "+", "Anda um passo para a frente neste eixo")] {
                    if ui.small_button(txt).on_hover_text(tr(dica)).clicked() {
                        passo = Some((k, s, false));
                    }
                }
                for (s, txt, dica) in [(-1., "⟲", "Gira um passo para trás em volta deste eixo"), (1., "⟳", "Gira um passo para a frente em volta deste eixo")] {
                    if ui.small_button(txt).on_hover_text(tr(dica)).clicked() {
                        passo = Some((k, s, true));
                    }
                }
            });
            ui.end_row();
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Passo")).small());
        ui.add(crate::ui_ext::dv(&mut c.passo).range(0.0..=10000.0).speed(0.1).suffix(" mm").dica("Quanto anda cada clique em + e −"));
        ui.add(crate::ui_ext::dv(&mut c.passo_giro).range(0.0..=360.0).speed(0.1).suffix("°").dica("Quanto gira cada clique em ⟳ e ⟲"));
    });
    let (modo, base, pos, ang) = (c.modo, c.base, c.pos, c.ang);
    let (p_mm, p_gr) = (c.passo, c.passo_giro);
    let mut t = None;
    ui.horizontal(|ui| {
        let pode = montagem && !ids.is_empty();
        if ui.add_enabled(pode, egui::Button::new(tr("Aplicar"))).on_hover_text(tr("Move e gira as peças escolhidas como pedem os campos")).clicked() {
            t = Some(pedido(&q, &anc.base, modo, base, pos, ang));
        }
        if modo == Modo::Absoluta && ui.button(tr("Ler posição atual")).on_hover_text(tr("Põe nos campos onde o compasso está agora")).clicked() {
            (app.compasso.pos, app.compasso.ang) = atuais(&q, &anc.base, base);
        }
        if modo == Modo::Incremental && ui.button(tr("Zerar")).on_hover_text(tr("Volta os campos a zero")).clicked() {
            app.compasso.pos = [0.; 3];
            app.compasso.ang = [0.; 3];
        }
    });
    if let Some((k, s, giro)) = passo {
        let mut d = [0.; 3];
        d[k] = s * if giro { p_gr } else { p_mm };
        t = Some(if giro { pedido(&q, &anc.base, Modo::Incremental, base, [0.; 3], d) } else { pedido(&q, &anc.base, Modo::Incremental, base, d, [0.; 3]) });
    }
    if let Some(t) = t {
        if aplicar(app, t) > 0 && app.compasso.modo == Modo::Incremental && passo.is_none() {
            // incremental aplicado: os campos voltam a zero para o próximo passo
            app.compasso.pos = [0.; 3];
            app.compasso.ang = [0.; 3];
        }
    }
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button(tr("Olhar pela normal")).on_hover_text(tr("Gira a vista para olhar ao longo do Z do compasso")).clicked() {
            app.compasso.olhar = Some(v3(q.rot[2]));
        }
        if ui.button(tr("Devolver ao canto")).on_hover_text(tr("O compasso volta ao canto e volta a ser só o cubo de vistas")).clicked() {
            app.compasso.devolver();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn perto3(a: V3, b: V3) {
        assert!(norm(sub(a, b)) < 1e-6, "{a:?} != {b:?}");
    }

    #[test]
    fn quadro_da_face_do_topo_fica_com_os_eixos_do_mundo() {
        let q = quadro([1., 2., 3.], [0., 0., 1.]);
        perto3(q.rot[0], [1., 0., 0.]);
        perto3(q.rot[1], [0., 1., 0.]);
        perto3(q.rot[2], [0., 0., 1.]);
    }

    #[test]
    fn angulos_vao_e_voltam() {
        let a = [10., -20., 35.];
        let b = angulos(de_angulos(a));
        for k in 0..3 {
            assert!((a[k] - b[k]).abs() < 1e-9, "{a:?} {b:?}");
        }
    }

    #[test]
    fn absoluta_com_os_valores_de_agora_nao_mexe() {
        let q = quadro([5., 6., 7.], [1., 1., 0.]);
        for base in [Base::Local, Base::Global] {
            let (p, a) = atuais(&q, &q, base);
            let t = pedido(&q, &q, Modo::Absoluta, base, p, a);
            perto3(t.origin, [0.; 3]);
            for k in 0..3 {
                perto3(t.rot[k], IDENT[k]);
            }
        }
    }
}

#[cfg(test)]
mod testes_montagem {
    //! Aplicativo de verdade, sem janela, com uma montagem de dois blocos: o compasso gruda nas faces e
    //! arestas lidas pelo kernel e move a peça pelo painel, conferido pela caixa envolvente da vista.
    use super::tests::perto3;
    use super::*;
    use crate::asm_ui::AsmState;
    use crate::settings::Bench;
    use fenkra_core::{BoolOp, Document, Extent, Extrude, FeatureKind, OriginPlane, PlaneRef, Sketch, SketchData};
    use std::path::PathBuf;

    fn bloco(nome: &str, l: f64) -> Document {
        let mut d = Document::new(nome);
        let mut n = 7_400_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., 0.], [l, l], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(l.to_string()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        d
    }

    struct Bancada {
        app: App,
        ctx: egui::Context,
        frame: eframe::Frame,
        t: f64,
        b: Id,
        _dir: PathBuf,
    }

    impl Bancada {
        /// A (bloco de 10, fixo) na origem e B (bloco de 20) em x = 50: B ocupa 50..70 × 0..20 × 0..20.
        fn nova(nome: &str) -> Self {
            std::env::set_var("FENKRA_DEMO_TESTE", "1");
            let dir = std::env::temp_dir().join(format!("fenkra_compasso_{nome}_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("a.fkp"), bloco("A", 10.).to_json()).unwrap();
            std::fs::write(dir.join("b.fkp"), bloco("B", 20.).to_json()).unwrap();
            let mut m = Assembly::new("Conjunto");
            let a = m.add_component("A", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
            m.node_mut(a).unwrap().fixed = true;
            let b = m.add_component("B", NodeKind::Part { path: "b.fkp".into() }, Placement { origin: [50., 0., 0.], ..Default::default() }, None);
            let ctx = egui::Context::default();
            let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
            app.started = true;
            app.asm = Some(AsmState::new(m, Some(dir.join("conjunto.fka"))));
            app.settings.bench = Bench::Montagem;
            let mut bc = Bancada { app, ctx, frame: eframe::Frame::_new_kittest(), t: 0., b, _dir: dir };
            bc.assentar();
            bc
        }

        fn quadro(&mut self) {
            self.t += 0.1;
            let raw = egui::RawInput { time: Some(self.t), screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280., 800.))), ..Default::default() };
            let (app, frame) = (&mut self.app, &mut self.frame);
            let mut saida = self.ctx.run_ui(raw, |ui| eframe::App::ui(app, ui, frame));
            saida.textures_delta.clear();
        }

        fn assentar(&mut self) {
            for _ in 0..6000 {
                self.quadro();
                let compondo = self.app.asm.as_ref().is_some_and(|s| s.compondo || s.dirty || s.envio_pendente);
                if !self.app.calculando() && !compondo {
                    self.quadro();
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            panic!("não assentou");
        }

        fn st(&mut self) -> &mut AsmState {
            self.app.asm.as_mut().unwrap()
        }

        fn corpo_de_b(&mut self) -> usize {
            let b = self.b;
            self.st().view.as_ref().unwrap().bodies.iter().position(|c| c.collector == b).unwrap()
        }

        fn caixa_de_b(&mut self) -> ([f32; 3], [f32; 3]) {
            let i = self.corpo_de_b();
            self.st().caixas[i]
        }

        /// Solta o compasso numa face de B, como o mouse soltaria: o mesmo pick que a viewport dá.
        fn soltar_na_face(&mut self, ponto: [f32; 3], normal: [f32; 3]) -> Ancora {
            let body = self.corpo_de_b();
            let pick = Pick::Face { body, face: 0, owner: Id(0), point: ponto, normal };
            let a = ancora_do_pick(&mut self.app, pick).expect("a face prende o compasso");
            prender(&mut self.app, a);
            a
        }
    }

    fn caixa_perto(c: ([f32; 3], [f32; 3]), min: [f64; 3], max: [f64; 3]) {
        for k in 0..3 {
            assert!((c.0[k] as f64 - min[k]).abs() < 1e-2 && (c.1[k] as f64 - max[k]).abs() < 1e-2, "caixa {c:?} != {min:?}..{max:?}");
        }
    }

    #[test]
    fn compasso_gruda_na_face_e_na_aresta_com_a_orientacao_certa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut b = Bancada::nova("orienta");
        caixa_perto(b.caixa_de_b(), [50., 0., 0.], [70., 20., 20.]);
        // face lateral x = 70: Z do compasso na normal (+X), origem no ponto solto
        let a = b.soltar_na_face([70., 5., 12.], [1., 0., 0.]);
        assert_eq!((a.tipo, a.dono), (Tipo::Face, Some(b.b)));
        let q = a.mundo(Some(&b.st().asm));
        perto3(q.rot[2], [1., 0., 0.]);
        perto3(q.origin, [70., 5., 12.]);
        assert!(dot(q.rot[0], q.rot[2]).abs() < 1e-9 && dot(q.rot[1], q.rot[2]).abs() < 1e-9, "eixos ortogonais");
        // face do topo z = 20
        let a = b.soltar_na_face([60., 10., 20.], [0., 0., 1.]);
        perto3(a.mundo(Some(&b.st().asm)).rot[2], [0., 0., 1.]);
        // aresta do topo da frente: y = 0, z = 20, ao longo de X; a origem cai sobre a aresta
        let body = b.corpo_de_b();
        let m = &b.st().view.as_ref().unwrap().bodies[body].mesh;
        let (edge, _) = crate::entradas::aresta_perto(m, [60., 0., 20.]).expect("aresta");
        let pick = Pick::Edge { body, edge, mid: [60., 0., 20.] };
        let a = ancora_do_pick(&mut b.app, pick).expect("a aresta prende o compasso");
        assert_eq!(a.tipo, Tipo::Aresta);
        let q = a.mundo(Some(&b.st().asm));
        assert!((dot(q.rot[2], [1., 0., 0.]).abs() - 1.).abs() < 1e-6, "Z ao longo da aresta: {:?}", q.rot[2]);
        assert!(q.origin[1].abs() < 1e-6 && (q.origin[2] - 20.).abs() < 1e-6, "origem na aresta: {:?}", q.origin);
    }

    #[test]
    fn mover_10_mm_em_incremental_no_eixo_do_compasso() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut b = Bancada::nova("mover");
        // preso na face x = 70: o Z local é +X do mundo
        let a = b.soltar_na_face([70., 5., 12.], [1., 0., 0.]);
        let id_b = b.b;
        assert_eq!(b.st().sel, Some(id_b), "soltar na peça a escolhe");
        let q = a.mundo(Some(&b.st().asm));
        let t = pedido(&q, &a.base, Modo::Incremental, Base::Local, [0., 0., 10.], [0.; 3]);
        assert_eq!(aplicar(&mut b.app, t), 1);
        b.st().dirty = true;
        b.assentar();
        caixa_perto(b.caixa_de_b(), [60., 0., 0.], [80., 20., 20.]);
        // o compasso foi junto com a peça
        let q2 = b.app.compasso.ancora.unwrap().mundo(Some(&b.st().asm));
        perto3(q2.origin, [80., 5., 12.]);
        // global: 10 mm em Y do mundo
        let t = pedido(&q2, &a.base, Modo::Incremental, Base::Global, [0., 10., 0.], [0.; 3]);
        aplicar(&mut b.app, t);
        b.st().dirty = true;
        b.assentar();
        caixa_perto(b.caixa_de_b(), [60., 10., 0.], [80., 30., 20.]);
        // absoluta global: a origem do compasso vai para x = 100
        let q3 = b.app.compasso.ancora.unwrap().mundo(Some(&b.st().asm));
        let (mut p, ang) = atuais(&q3, &a.base, Base::Global);
        p[0] = 100.;
        aplicar(&mut b.app, pedido(&q3, &a.base, Modo::Absoluta, Base::Global, p, ang));
        b.st().dirty = true;
        b.assentar();
        caixa_perto(b.caixa_de_b(), [80., 10., 0.], [100., 30., 20.]);
    }

    #[test]
    fn girar_90_graus_em_volta_do_z_do_compasso() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut b = Bancada::nova("girar");
        // topo z = 20, origem em (68, 2): girar 90° em Z leva x - 68 → -(y - 2) e y - 2 → x - 68
        let a = b.soltar_na_face([68., 2., 20.], [0., 0., 1.]);
        let q = a.mundo(Some(&b.st().asm));
        let t = pedido(&q, &a.base, Modo::Incremental, Base::Local, [0.; 3], [0., 0., 90.]);
        aplicar(&mut b.app, t);
        b.st().dirty = true;
        b.assentar();
        caixa_perto(b.caixa_de_b(), [50., -16., 0.], [70., 4., 20.]);
        // a volta pela absoluta local com ângulo zero devolve a orientação de quando foi preso
        let q2 = b.app.compasso.ancora.unwrap().mundo(Some(&b.st().asm));
        let (p, _) = atuais(&q2, &a.base, Base::Local);
        aplicar(&mut b.app, pedido(&q2, &a.base, Modo::Absoluta, Base::Local, p, [0.; 3]));
        b.st().dirty = true;
        b.assentar();
        caixa_perto(b.caixa_de_b(), [50., 0., 0.], [70., 20., 20.]);
    }

    #[test]
    fn devolver_ao_canto_solta_o_compasso() {
        let mut c = Compasso::novo();
        c.ancora = Some(ancorar(Tipo::Face, [0.; 3], [0.; 3], [0., 0., 1.], None));
        c.devolver();
        assert!(!c.preso());
    }
}
