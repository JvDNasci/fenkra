// SPDX-License-Identifier: LGPL-2.1-or-later
//! Bancada Forma livre (B153): criar a primitiva arrastando no plano e editar a gaiola no 3D. Aqui só
//! se desenha e se lê o mouse; as regras estão em `forma_edit.rs`. Toda tecla tem botão na barra do 3D,
//! e as cores vêm do tema.

use crate::app::{fmt, App};
use crate::editor::section;
use crate::forma_edit::{self as fe, Elem, Espaco, Ferramenta, ModoVista, Tela};
use crate::i18n::tr;
use crate::render::{Camera, MeshVertex, Scene};
use crate::settings::Bench;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use fenkra_core::subd::{Gaiola, V3};
use fenkra_core::subd_ops::{PrimitivaForma, TipoNo};
use fenkra_core::*;
use fenkra_solidos::RecomputeResult;
use glam::Vec3;

fn v3(p: V3) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}
fn d3(p: Vec3) -> V3 {
    [p.x as f64, p.y as f64, p.z as f64]
}

/// Arraste em andamento: como a gaiola estava quando começou.
#[derive(Clone, Debug)]
struct Arraste {
    base: Gaiola,
    nos: Vec<u32>,
    centro: V3,
    eixos: [V3; 3],
}

/// Rótulo do manipulador aberto para digitar: eixo (0 a 2 mover, 3 a 5 girar) e o texto.
#[derive(Clone, Debug)]
struct Digitando {
    alca: usize,
    texto: String,
}

/// Criação da primitiva: o plano e o primeiro canto.
#[derive(Clone, Debug)]
pub struct Criacao {
    pub tipo: PrimitivaForma,
    /// origem, x, y e normal do plano
    plano: [V3; 4],
    inicio: Option<V3>,
}

#[derive(Clone, Debug, Default)]
pub struct Estado {
    pub criando: Option<Criacao>,
    pub modo: ModoVista,
    pub espaco: Espaco,
    pub ferramenta: Ferramenta,
    pub sel: Vec<Elem>,
    pub hover: Option<Elem>,
    /// peso usado pelo chip Vinco
    pub peso: f64,
    arraste: Option<Arraste>,
    /// valores do manipulador no arraste atual
    off: [f64; 3],
    giro: [f64; 3],
    digitando: Option<Digitando>,
    /// aresta do corte em laço e deslize
    laco: Option<(u32, u32, f64)>,
    /// Alt pressionado no botão Extrudar: o próximo arraste abre a face
    extrudar: bool,
    tangente_base: Option<(V3, f64)>,
}

/// Feature de forma aberta.
fn aberta(app: &App) -> Option<Id> {
    let id = app.session.as_ref()?.id;
    matches!(app.open.doc.feature(id)?.kind, FeatureKind::Forma(_)).then_some(id)
}

fn gaiola(app: &App, id: Id) -> Option<&Gaiola> {
    match &app.open.doc.feature(id)?.kind {
        FeatureKind::Forma(f) => Some(&f.gaiola),
        _ => None,
    }
}

fn gaiola_mut(app: &mut App, id: Id) -> Option<&mut Gaiola> {
    match &mut app.open.doc.feature_mut(id)?.kind {
        FeatureKind::Forma(f) => Some(&mut f.gaiola),
        _ => None,
    }
}

fn congelada(app: &App, id: Id) -> bool {
    matches!(app.open.doc.feature(id).map(|f| &f.kind), Some(FeatureKind::Forma(f)) if f.congelada)
}

/// Aplica a mudança e avisa o erro sem perder o que havia.
fn editar(app: &mut App, id: Id, f: impl FnOnce(&mut Gaiola) -> Result<(), String>) {
    let Some(g) = gaiola_mut(app, id) else { return };
    let antes = g.clone();
    if let Err(e) = f(g) {
        *g = antes;
        app.toast(e, true);
        return;
    }
    let g2 = gaiola(app, id).cloned();
    app.forma.sel.retain(|e| {
        let Some(g) = &g2 else { return false };
        match *e {
            Elem::No(n) => g.no(n).is_some(),
            Elem::Aresta(a, b) => g.arestas().contains(&(a, b)),
            Elem::Face(f) => g.face(f).is_some(),
        }
    });
}

/// A bancada está sem arraste: fim do estado temporário.
fn soltar(st: &mut Estado) {
    st.arraste = None;
    st.off = [0.; 3];
    st.giro = [0.; 3];
    st.tangente_base = None;
}

/// Botão da barra: espera o arraste no plano.
pub fn comecar(app: &mut App, tipo: PrimitivaForma) {
    app.conclude_session();
    let plano = plano_atual(app);
    app.forma.criando = Some(Criacao { tipo, plano, inicio: None });
    app.forma.sel.clear();
}

/// Plano escolhido, ou o XY.
fn plano_atual(app: &App) -> [V3; 4] {
    let pega = |f: &fenkra_kernel::Frame| [f.origin, f.x, f.y(), f.normal];
    let r = app.result.as_ref();
    let do_pick = app.picks.iter().find_map(|p| match p {
        crate::picking::Pick::Plane(PlaneRef::Origin(o)) => Some(fenkra_solidos::origin_frame(*o)),
        crate::picking::Pick::Plane(PlaneRef::Datum(d)) => r?.planes.iter().find(|x| x.id == Some(*d)).map(|x| x.frame.clone()),
        _ => None,
    });
    do_pick.as_ref().map_or([[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]], pega)
}

/// Ponto do raio do mouse no plano.
fn no_plano(cam: &Camera, rect: Rect, m: Pos2, plano: &[V3; 4]) -> Option<V3> {
    let (o, d) = cam.ray(rect, m);
    let (po, n) = (v3(plano[0]), v3(plano[3]));
    let den = d.dot(n);
    if den.abs() < 1e-6 {
        return None;
    }
    Some(d3(o + d * ((po - o).dot(n) / den)))
}

fn coord(plano: &[V3; 4], p: V3) -> (f64, f64) {
    let d = [p[0] - plano[0][0], p[1] - plano[0][1], p[2] - plano[0][2]];
    (d[0] * plano[1][0] + d[1] * plano[1][1] + d[2] * plano[1][2], d[0] * plano[2][0] + d[1] * plano[2][1] + d[2] * plano[2][2])
}

fn no_quadro(plano: &[V3; 4], u: f64, v: f64, w: f64) -> V3 {
    let mut q = plano[0];
    for k in 0..3 {
        q[k] += plano[1][k] * u + plano[2][k] * v + plano[3][k] * w;
    }
    q
}

/// Gaiola criada entre dois cantos: a altura é a menor largura.
pub fn gaiola_do_arraste(tipo: PrimitivaForma, plano: &[V3; 4], a: V3, b: V3) -> Gaiola {
    let ((u0, v0), (u1, v1)) = (coord(plano, a), coord(plano, b));
    let (mut lu, mut lv) = ((u1 - u0).abs(), (v1 - v0).abs());
    if lu < 1. && lv < 1. {
        // clique sem arraste
        (lu, lv) = (40., 40.);
    }
    let (lu, lv) = (lu.max(1.), lv.max(1.));
    let alt = lu.min(lv);
    let (cu, cv) = ((u0 + u1) / 2., (v0 + v1) / 2.);
    let mut g = Gaiola::primitiva(tipo, [0., 0., if tipo == PrimitivaForma::Plano { 0. } else { alt / 2. }], [lu, lv, alt]);
    for n in &mut g.nos {
        n.p = no_quadro(plano, cu + n.p[0], cv + n.p[1], n.p[2]);
    }
    g
}

/// Superfície na cena: cor do corpo, salvo no modo Gaiola.
pub fn na_cena(doc: &Document, r: &RecomputeResult, oculta: Option<Id>, s: &mut Scene, cresce: &mut dyn FnMut(Vec3)) {
    for f in &r.formas {
        if oculta == Some(f.id) {
            continue;
        }
        let Some(feat) = doc.feature(f.id) else { continue };
        let cor = doc.collectors.iter().find(|c| c.id == feat.target).map_or([200, 200, 205], |c| c.color).map(|x| x as f32 / 255.);
        let base = s.mesh.len() as u32;
        for (p, n) in f.malha.pos.iter().zip(&f.malha.nrm) {
            let pos = p.map(|x| x as f32);
            cresce(Vec3::from(pos));
            s.mesh.push(MeshVertex { pos, nrm: n.map(|x| x as f32), col: [cor[0], cor[1], cor[2], 1.], mat: [0.45, 0., 1., 0.] });
        }
        s.indices.extend(f.malha.tris.iter().flat_map(|t| t.iter().map(|i| base + i)));
    }
}

/// Forma cuja superfície fica escondida na cena.
pub fn oculta(app: &App) -> Option<Id> {
    aberta(app).filter(|_| app.forma.modo == ModoVista::Gaiola)
}

fn visao(cam: &Camera) -> [V3; 3] {
    let (r, u) = cam.basis();
    [d3(r), d3(u), d3((cam.eye() - cam.target).normalize_or_zero())]
}

fn com_tela<R>(cam: &Camera, rect: Rect, f: impl FnOnce(&Tela) -> R) -> R {
    let olho = cam.eye();
    let ortho = cam.ortho;
    let frente = (cam.eye() - cam.target).normalize_or_zero();
    let projetar = move |p: V3| cam.project(rect, v3(p)).map(|s| [s.x, s.y]);
    let de_frente = move |c: V3, n: V3| {
        let para = if ortho { frente } else { olho - v3(c) };
        para.dot(v3(n)) > 0.
    };
    let profundidade = move |p: V3| (v3(p) - olho).length() as f64;
    f(&Tela { projetar: &projetar, de_frente: &de_frente, profundidade: &profundidade })
}

/// Chamado pela viewport: criação ou edição da forma.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, fundo: &egui::Response) {
    if app.forma.criando.is_some() {
        criar(app, ui, rect, cam);
        return;
    }
    let Some(id) = aberta(app) else { return };
    if app.settings.bench != Bench::FormaLivre {
        app.settings.bench = Bench::FormaLivre;
    }
    // Tab alterna gaiola, liso e ambos; a barra do 3D faz o mesmo
    let tab = !ui.ctx().egui_wants_keyboard_input() && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
    if tab {
        trocar_modo(app, app.forma.modo.proximo());
    }
    if !ui.input(|i| i.pointer.any_down()) {
        soltar(&mut app.forma);
    }
    let Some(g) = gaiola(app, id).cloned() else { return };
    let inteira = g.inteira();
    let so_leitura = congelada(app, id);
    let mouse = ui.input(|i| i.pointer.latest_pos()).filter(|p| rect.contains(*p));
    desenhar_gaiola(app, ui, rect, cam, &g, &inteira);

    // o que está sob o mouse, salvo durante um arraste de alça
    let ocupado = ui.ctx().dragged_id().is_some_and(|d| d != fundo.id) || app.forma.arraste.is_some();
    if !ocupado {
        app.forma.hover = mouse.filter(|_| !sobre_a_barra(ui)).and_then(|m| com_tela(cam, rect, |t| fe::escolher(&g, t, [m.x, m.y])));
    }
    if app.forma.modo == ModoVista::Liso && app.forma.sel.is_empty() {
        app.forma.hover = None;
    }

    if let (Some(alvo), Some(m), false) = (app.forma.hover, mouse, so_leitura) {
        alvo_do_mouse(app, ui, rect, cam, id, alvo, m, &g);
    } else if let (Some(alvo), Some(m)) = (app.forma.hover, mouse) {
        // congelada ainda deixa ver o que é cada coisa
        let r = ui.interact(Rect::from_center_size(m, Vec2::splat(24.)), egui::Id::unique("forma_alvo"), egui::Sense::click());
        if r.clicked() {
            app.forma.sel = vec![alvo];
        }
    }
    if fundo.clicked_by(egui::PointerButton::Primary) && app.forma.hover.is_none() {
        app.forma.sel.clear();
        app.forma.ferramenta = match app.forma.ferramenta {
            Ferramenta::InserirAresta(_) => Ferramenta::InserirAresta(None),
            f => f,
        };
    }
    if !so_leitura {
        if app.forma.ferramenta == Ferramenta::Tangencia {
            disco_de_tangencia(app, ui, rect, cam, id);
        } else if app.forma.ferramenta == Ferramenta::Selecionar {
            manipulador(app, ui, rect, cam, id);
        }
    }
    barra(app, ui, rect, id, so_leitura);
}

fn sobre_a_barra(ui: &egui::Ui) -> bool {
    ui.ctx().is_pointer_over_egui() && ui.ctx().layer_id_at(ui.input(|i| i.pointer.latest_pos()).unwrap_or_default()).is_some_and(|l| l.order != egui::Order::Background)
}

fn trocar_modo(app: &mut App, m: ModoVista) {
    app.forma.modo = m;
    app.scene_dirty = true;
}

/// Linhas da gaiola: finas na frente, apagadas atrás; o reflexo da simetria mais apagado ainda.
fn desenhar_gaiola(app: &App, ui: &egui::Ui, rect: Rect, cam: &Camera, g: &Gaiola, inteira: &Gaiola) {
    if app.forma.modo == ModoVista::Liso {
        return;
    }
    let p = ui.painter_at(rect);
    let vis = ui.visuals();
    let (cor, quente, sel) = (crate::theme::accent(vis), crate::theme::st_warn(), crate::theme::st_ok());
    let frente = com_tela(cam, rect, |t| fe::faces_de_frente(inteira, t));
    let de_frente = |a: u32, b: u32| inteira.faces.iter().any(|f| frente.contains(&f.id) && f.nos.contains(&a) && f.nos.contains(&b));
    let tela = |id: u32| inteira.pos(id).and_then(|q| cam.project(rect, v3(q)));
    let marcada = |a: u32, b: u32, e: &Elem| matches!(*e, Elem::Aresta(x, y) if (x, y) == (a, b));
    for (a, b) in inteira.arestas() {
        let (Some(pa), Some(pb)) = (tela(a), tela(b)) else { continue };
        let espelho = (a | b) & fenkra_core::subd::ESPELHO != 0;
        let frontal = de_frente(a, b);
        let peso = inteira.peso(a, b);
        let mut s = Stroke::new(if peso > 0. { 1.2 + 1.3 * peso as f32 } else { 1. }, cor.gamma_multiply(if espelho { 0.3 } else if frontal { 0.9 } else { 0.25 }));
        if peso > 0. && !espelho {
            s.color = quente;
        }
        if app.forma.sel.iter().any(|e| marcada(a, b, e)) {
            s = Stroke::new(3., sel);
        } else if app.forma.hover.as_ref().is_some_and(|e| marcada(a, b, e)) {
            s = Stroke::new(3., quente);
        }
        if !frontal && espelho {
            continue;
        }
        p.line_segment([pa, pb], s);
    }
    // face acesa ou selecionada
    for (e, c) in app.forma.hover.iter().map(|e| (e, quente)).chain(app.forma.sel.iter().map(|e| (e, sel))) {
        if let Elem::Face(f) = e {
            let Some(f) = g.face(*f) else { continue };
            let poli: Vec<Pos2> = f.nos.iter().filter_map(|n| tela(*n)).collect();
            if poli.len() == f.nos.len() {
                p.add(egui::Shape::closed_line(poli.clone(), Stroke::new(2.5, c)));
                // polígono côncavo na tela não pode usar preenchimento convexo: basta o leque do centro
                let c0 = poli.iter().fold(Vec2::ZERO, |a, q| a + q.to_vec2()) / poli.len() as f32;
                for k in 0..poli.len() {
                    p.add(egui::Shape::convex_polygon(vec![c0.to_pos2(), poli[k], poli[(k + 1) % poli.len()]], c.gamma_multiply(0.18), Stroke::NONE));
                }
            }
        }
    }
    let irregulares = inteira.nos_irregulares();
    for n in &g.nos {
        let Some(q) = cam.project(rect, v3(n.p)) else { continue };
        let visto = inteira.faces.iter().any(|f| frente.contains(&f.id) && f.nos.contains(&n.id));
        if !visto {
            continue;
        }
        let e = Elem::No(n.id);
        let (raio, c) = if app.forma.sel.contains(&e) {
            (5.5, sel)
        } else if app.forma.hover == Some(e) {
            (5.5, quente)
        } else {
            (3., cor)
        };
        if n.canto {
            p.rect_filled(Rect::from_center_size(q, Vec2::splat(raio * 2.)), 0., c);
        } else {
            p.circle_filled(q, raio, c);
        }
        if irregulares.contains(&n.id) {
            p.circle_stroke(q, raio + 2.5, Stroke::new(1., c.gamma_multiply(0.7)));
        }
    }
}

/// Clique seleciona, arrastar move no plano da vista, e cada ferramenta usa o elemento a seu modo.
#[allow(clippy::too_many_arguments)]
fn alvo_do_mouse(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id, alvo: Elem, m: Pos2, g: &Gaiola) {
    let r = ui.interact(Rect::from_center_size(m, Vec2::splat(26.)), egui::Id::unique("forma_alvo"), egui::Sense::click_and_drag());
    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    match app.forma.ferramenta {
        Ferramenta::Laco => {
            let Elem::Aresta(a, b) = alvo else { return };
            let t = app.forma.laco.filter(|x| (x.0, x.1) == (a, b)).map_or(0.5, |x| x.2);
            let t = if r.dragged() {
                let par = (g.pos(a).and_then(|p| cam.project(rect, v3(p))), g.pos(b).and_then(|p| cam.project(rect, v3(p))));
                if let (Some(sa), Some(sb)) = par { fe::fracao_na_aresta([sa.x, sa.y], [sb.x, sb.y], [m.x, m.y]) } else { t }
            } else {
                t
            };
            app.forma.laco = Some((a, b, t));
            let pinta = ui.painter_at(rect);
            let cor = crate::theme::st_ok();
            for [p, q] in fe::previa_do_laco(g, a, b, t) {
                if let (Some(p), Some(q)) = (cam.project(rect, v3(p)), cam.project(rect, v3(q))) {
                    pinta.line_segment([p, q], Stroke::new(2., cor));
                }
            }
            if r.clicked() || r.drag_stopped() {
                editar(app, id, |g| g.corte_em_laco(a, b, t).map(|_| ()));
                app.forma.laco = None;
            }
        }
        Ferramenta::InserirAresta(primeiro) => {
            let Elem::No(n) = alvo else { return };
            if r.clicked() {
                match primeiro {
                    None => app.forma.ferramenta = Ferramenta::InserirAresta(Some(n)),
                    Some(p) => {
                        editar(app, id, |g| g.inserir_aresta(p, n).map(|_| ()));
                        app.forma.ferramenta = Ferramenta::InserirAresta(None);
                    }
                }
            }
        }
        Ferramenta::Suavizar { raio, segmentos } => {
            if let (Elem::Aresta(a, b), true) = (alvo, r.clicked()) {
                editar(app, id, |g| g.suavizar_borda(a, b, raio, segmentos));
            }
        }
        Ferramenta::Tangencia => {
            if let (Elem::No(_), true) = (alvo, r.clicked()) {
                app.forma.sel = vec![alvo];
            }
        }
        Ferramenta::Selecionar => {
            if r.clicked() {
                let somar = ui.input(|i| i.modifiers.shift || i.modifiers.command);
                let sel = &mut app.forma.sel;
                if somar {
                    if let Some(k) = sel.iter().position(|e| *e == alvo) {
                        sel.remove(k);
                    } else {
                        sel.push(alvo);
                    }
                } else {
                    *sel = vec![alvo];
                }
            }
            if r.drag_started_by(egui::PointerButton::Primary) {
                if !app.forma.sel.contains(&alvo) {
                    app.forma.sel = vec![alvo];
                }
                let extruda = matches!(alvo, Elem::Face(_)) && (ui.input(|i| i.modifiers.alt) || app.forma.extrudar);
                if extruda {
                    let sel = app.forma.sel.clone();
                    editar(app, id, |g| fe::comecar_extrusao(g, &sel));
                    app.forma.extrudar = false;
                }
                let Some(g) = gaiola(app, id).cloned() else { return };
                let sel = app.forma.sel.clone();
                let c = fe::centro(&g, &sel).unwrap_or([0.; 3]);
                let n = fe::normal(&g, &sel).unwrap_or([0., 0., 1.]);
                let e = if extruda { [[1., 0., 0.], [0., 1., 0.], n] } else { visao(cam) };
                app.forma.arraste = Some(Arraste { nos: fe::nos_de(&g, &sel), centro: c, eixos: e, base: g });
                app.forma.off = [0.; 3];
            }
            if r.dragged_by(egui::PointerButton::Primary) {
                let Some(a) = app.forma.arraste.clone() else { return };
                let d = ui.input(|i| i.pointer.press_origin()).map_or(Vec2::ZERO, |o| m - o);
                let ppm = crate::gizmo::screen_axis(cam, rect, v3(a.centro), cam.basis().0).map_or(1., |t| t.2.max(1e-3)) as f64;
                let vis = visao(cam);
                // Alt: só ao longo da normal; senão, no plano da tela
                let mv = if a.eixos[2] != vis[2] {
                    let s = crate::gizmo::screen_axis(cam, rect, v3(a.centro), v3(a.eixos[2]));
                    let t = s.map_or(0., |s| d.dot(s.1) as f64 / s.2.max(1e-3) as f64);
                    fe::deslocamento(a.eixos, [0., 0., t])
                } else {
                    fe::deslocamento(vis, [d.x as f64 / ppm, -d.y as f64 / ppm, 0.])
                };
                if let Some(g) = gaiola_mut(app, id) {
                    fe::mover(g, &a.base, &a.nos, mv);
                }
            }
        }
    }
}

/// Setas, quadrados e arcos do manipulador, com rótulo que aceita digitar.
fn manipulador(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let sel = app.forma.sel.clone();
    let Some(g) = gaiola(app, id).cloned() else { return };
    let Some(c) = fe::centro(&g, &sel) else { return };
    let eixos = app.forma.arraste.as_ref().map_or_else(|| fe::eixos(&g, &sel, app.forma.espaco, visao(cam)), |a| a.eixos);
    let ax = eixos.map(v3);
    let salt = id.0 ^ 0xf0a;
    let off = app.forma.off;
    let inicio = |app: &mut App, g: &Gaiola, ui: &egui::Ui| {
        if app.forma.arraste.is_none() {
            let mut base = g.clone();
            if ui.input(|i| i.modifiers.alt) || app.forma.extrudar {
                if !fe::faces_de(&sel).is_empty() && fe::comecar_extrusao(&mut base, &sel).is_ok() {
                    if let Some(x) = gaiola_mut(app, id) {
                        *x = base.clone();
                    }
                    app.forma.extrudar = false;
                }
            }
            app.forma.arraste = Some(Arraste { nos: fe::nos_de(&base, &sel), centro: c, eixos, base });
        }
    };
    if let Some(v) = crate::gizmo::translate_alcas(ui, rect, cam, salt, v3(c), ax, off, [false; 3], [true; 3]) {
        inicio(app, &g, ui);
        app.forma.off = v;
        if let (Some(a), Some(x)) = (app.forma.arraste.clone(), gaiola_mut(app, id)) {
            fe::mover(x, &a.base, &a.nos, fe::deslocamento(eixos, v));
        }
    }
    let ppm = crate::gizmo::screen_axis(cam, rect, v3(c), cam.basis().0).map_or(1., |t| t.2.max(1e-3));
    let raio = 95. / ppm;
    for k in 0..3 {
        let fonte = v3(c) + ax[(k + 1) % 3] * raio;
        let cor = crate::gizmo::AXIS_COL[k];
        if let Some(a) = crate::gizmo::rotate_arc_col(ui, rect, cam, salt.wrapping_mul(31).wrapping_add(k as u64 + 3), v3(c), ax[k], fonte, app.forma.giro[k], false, cor) {
            inicio(app, &g, ui);
            app.forma.giro[k] = a;
            if let (Some(b), Some(x)) = (app.forma.arraste.clone(), gaiola_mut(app, id)) {
                fe::girar(x, &b.base, &b.nos, b.centro, eixos[k], a);
            }
        }
    }
    // rótulos: clicar abre a caixa de digitar
    for k in 0..3 {
        let Some((cs, tela, _)) = crate::gizmo::screen_axis(cam, rect, v3(c), ax[k]) else { continue };
        let at = cs + tela * (crate::gizmo::ARM + 30.);
        if pilula(ui, rect, at, &format!("{} {}", ["X", "Y", "Z"][k], fmt(app.forma.off[k])), crate::gizmo::AXIS_COL[k], k, "Clique e digite quanto mover neste eixo") {
            app.forma.digitando = Some(Digitando { alca: k, texto: String::new() });
        }
    }
    if let Some((alca, v)) = digitar(app, ui, rect, cam, c) {
        app.forma.digitando = None;
        inicio(app, &g, ui);
        if let (Some(a), Some(x)) = (app.forma.arraste.take(), gaiola_mut(app, id)) {
            if alca < 3 {
                let mut vals = [0.; 3];
                vals[alca] = v;
                fe::mover(x, &a.base, &a.nos, fe::deslocamento(eixos, vals));
            }
        }
    }
}

fn pilula(ui: &mut egui::Ui, rect: Rect, at: Pos2, txt: &str, cor: Color32, k: usize, dica: &str) -> bool {
    let v = ui.visuals().clone();
    let g = ui.painter().layout_no_wrap(txt.to_string(), egui::FontId::proportional(11.), v.text_color());
    let caixa = Rect::from_center_size(at, g.size() + egui::vec2(8., 4.));
    let r = ui.interact(caixa, egui::Id::unique(("forma_rotulo", k)), egui::Sense::click()).on_hover_text(tr(dica));
    let p = ui.painter_at(rect);
    p.rect_filled(caixa, 3., v.window_fill.gamma_multiply(if r.hovered() { 1. } else { 0.8 }));
    p.rect_stroke(caixa, 3., Stroke::new(1., cor), egui::StrokeKind::Inside);
    p.galley(caixa.min + egui::vec2(4., 2.), g, v.text_color());
    r.clicked()
}

fn digitar(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, c: V3) -> Option<(usize, f64)> {
    let mut d = app.forma.digitando.clone()?;
    let em = cam.project(rect, v3(c)).unwrap_or(rect.center()) + egui::vec2(30., 30.);
    let mut fim = None;
    egui::Area::new(egui::Id::unique("forma_digitar")).order(egui::Order::Foreground).fixed_pos(em).show(ui.ctx(), |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut d.texto).desired_width(70.).hint_text("mm"));
            if !r.has_focus() && !r.lost_focus() {
                r.request_focus();
            }
            let (enter, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
            if esc || (r.lost_focus() && !enter) {
                fim = Some(None);
            } else if enter {
                fim = Some(crate::ui_ext::parse_com_unidade(&d.texto, false));
            }
        });
    });
    match fim {
        None => {
            app.forma.digitando = Some(d);
            None
        }
        Some(None) => {
            app.forma.digitando = None;
            None
        }
        Some(Some(v)) => Some((d.alca, v)),
    }
}

/// Alça de disco no nó: arrastar a borda inclina o plano tangente; a seta vale a tensão.
fn disco_de_tangencia(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let Some(Elem::No(no)) = app.forma.sel.first().copied() else { return };
    let Some(g) = gaiola(app, id).cloned() else { return };
    let Some(p) = g.pos(no) else { return };
    let atual = g.no(no).and_then(|n| n.tangente);
    let n = atual.map(|t| t.normal).or_else(|| g.normal_limite(no).ok()).or_else(|| g.normal_no(no)).unwrap_or([0., 0., 1.]);
    let Some(c) = cam.project(rect, v3(p)) else { return };
    let ppm = crate::gizmo::screen_axis(cam, rect, v3(p), cam.basis().0).map_or(1., |t| t.2.max(1e-3));
    let rad = 46. / ppm;
    let (nv, cor) = (v3(n), crate::theme::accent(ui.visuals()));
    let (a, b) = nv.any_orthonormal_pair();
    let anel: Vec<Pos2> = (0..=40).filter_map(|i| {
        let t = i as f32 / 40. * std::f32::consts::TAU;
        cam.project(rect, v3(p) + (a * t.cos() + b * t.sin()) * rad)
    }).collect();
    let pinta = ui.painter_at(rect);
    if anel.len() > 3 {
        pinta.add(egui::Shape::closed_line(anel.clone(), Stroke::new(2., cor)));
    }
    let borda = anel.first().copied().unwrap_or(c);
    let r = ui.interact(Rect::from_center_size(borda, Vec2::splat(crate::handles::PEGA)), egui::Id::unique("forma_disco"), egui::Sense::drag()).on_hover_text(tr("Arraste para inclinar o plano tangente do nó"));
    pinta.circle_filled(borda, 6., if r.hovered() || r.dragged() { crate::theme::st_warn() } else { cor });
    if r.drag_started() {
        app.forma.tangente_base = Some((n, atual.map_or(1., |t| t.tensao)));
        app.forma.arraste = Some(Arraste { base: g.clone(), nos: vec![], centro: p, eixos: visao(cam) });
    }
    if r.dragged() {
        if let (Some((n0, _)), Some(o)) = (app.forma.tangente_base, ui.input(|i| i.pointer.press_origin())) {
            let d = ui.input(|i| i.pointer.latest_pos()).unwrap_or(o) - o;
            let vis = visao(cam);
            let novo = fe::inclinar(n0, vis[0], vis[1], d.x as f64 * 0.01, d.y as f64 * 0.01);
            let base = app.forma.arraste.as_ref().map(|a| a.base.clone());
            if let Some(x) = gaiola_mut(app, id) {
                if let Some(b) = base {
                    *x = b;
                }
            }
            editar(app, id, |g| g.colocar_tangencia(no, novo));
        }
    }
    // seta da tensão
    let Some((_, tela, _)) = crate::gizmo::screen_axis(cam, rect, v3(p), nv) else { return };
    let tensao = atual.map_or(1., |t| t.tensao);
    let ponta = c + tela * (40. + 30. * tensao.ln().clamp(-2., 2.) as f32 + 30.);
    crate::gizmo::arrow(&pinta, c, ponta, cor);
    let r = ui.interact(Rect::from_center_size(ponta, Vec2::splat(crate::handles::PEGA)), egui::Id::unique("forma_tensao"), egui::Sense::drag()).on_hover_text(tr("Arraste para mudar a tensão: o anel abre ou fecha sem sair do plano tangente"));
    if atual.is_some() {
        if r.drag_started() {
            app.forma.tangente_base = Some((n, tensao));
        }
        if r.dragged() {
            if let (Some((_, t0)), Some(o)) = (app.forma.tangente_base, ui.input(|i| i.pointer.press_origin())) {
                let d = (ui.input(|i| i.pointer.latest_pos()).unwrap_or(o) - o).dot(tela);
                editar(app, id, |g| g.definir_tensao(no, t0 * (d as f64 / 60.).exp()));
            }
        }
    }
    pinta.text(ponta + egui::vec2(10., -6.), egui::Align2::LEFT_BOTTOM, format!("{} {}", tr("Tensão"), fmt(tensao)), egui::FontId::proportional(11.), ui.visuals().text_color());
}

fn chip(ui: &mut egui::Ui, ligado: bool, icone: Option<&str>, texto: &str, dica: &str) -> bool {
    let r = match icone {
        Some(i) => ui.add(egui::Button::image_and_text(egui::Image::new(crate::icons::get(i)).fit_to_exact_size(Vec2::splat(16.)), tr(texto)).selected(ligado)),
        None => ui.add(egui::Button::new(tr(texto)).selected(ligado)),
    };
    r.on_hover_text(tr(dica)).clicked()
}

/// Barra do 3D: toda tecla e todo gesto têm um botão aqui.
fn barra(app: &mut App, ui: &mut egui::Ui, rect: Rect, id: Id, so_leitura: bool) {
    let mut acao: Option<Box<dyn FnOnce(&mut App)>> = None;
    let st = &app.forma;
    let (modo, espaco, ferramenta, peso) = (st.modo, st.espaco, st.ferramenta, st.peso);
    let sel = st.sel.clone();
    let Some(g) = gaiola(app, id) else { return };
    let tipo = fe::tipo_da_selecao(g, &sel);
    let simetria = g.simetria;
    let tem_face = !fe::faces_de(&sel).is_empty();
    let extrudar = st.extrudar;
    let area = egui::Area::new(egui::Id::unique("forma_barra")).order(egui::Order::Middle).fixed_pos(rect.center_bottom() - egui::vec2(0., 34.)).pivot(egui::Align2::CENTER_BOTTOM);
    area.show(ui.ctx(), |ui| {
        crate::paineis::quadro(ui.style()).show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.;
            ui.horizontal(|ui| {
                for m in ModoVista::TODOS {
                    if chip(ui, modo == m, None, m.rotulo(), "O que aparece no 3D (tecla Tab)") {
                        acao = Some(Box::new(move |a| trocar_modo(a, m)));
                    }
                }
                ui.separator();
                for e in Espaco::TODOS {
                    if chip(ui, espaco == e, None, e.rotulo(), e.dica()) {
                        acao = Some(Box::new(move |a| a.forma.espaco = e));
                    }
                }
                ui.separator();
                if !so_leitura {
                    let mut pega = |ui: &mut egui::Ui, f: Ferramenta, ligado: bool, ic: &str, t: &str, d: &str| {
                        if chip(ui, ligado, Some(ic), t, d) {
                            let novo = if ligado { Ferramenta::Selecionar } else { f };
                            acao = Some(Box::new(move |a: &mut App| a.forma.ferramenta = novo));
                        }
                    };
                    pega(ui, Ferramenta::Laco, ferramenta == Ferramenta::Laco, "forma_laco", "Corte em laço", "Passe sobre uma aresta para ver o laço; arraste para deslizar e solte para cortar");
                    pega(ui, Ferramenta::InserirAresta(None), matches!(ferramenta, Ferramenta::InserirAresta(_)), "forma_aresta", "Inserir aresta", "Clique em dois nós da mesma face");
                    pega(ui, Ferramenta::Suavizar { raio: 2., segmentos: 2 }, matches!(ferramenta, Ferramenta::Suavizar { .. }), "forma_borda", "Suavizar borda", "Clique numa aresta: ela vira uma faixa com o raio e os segmentos abaixo");
                    pega(ui, Ferramenta::Tangencia, ferramenta == Ferramenta::Tangencia, "forma_tangencia", "Tangência", "Clique num nó: o disco define o plano tangente e a seta, a tensão");
                    ui.separator();
                    if chip(ui, extrudar, Some("forma_extrudar"), "Extrudar", "O próximo arraste de face cria paredes (o mesmo que segurar Alt)") {
                        acao = Some(Box::new(move |a: &mut App| a.forma.extrudar = !extrudar));
                    }
                    if chip(ui, simetria.is_some(), Some("forma_simetria"), "Simetria", "Corta no plano do centro em X e espelha: só a metade é editada") {
                        acao = Some(Box::new(move |a: &mut App| {
                            editar(a, id, |g| fe::alternar_simetria(g, 0));
                            a.forma.sel.clear();
                        }));
                    }
                }
            });
            if let (Some((t, w)), false) = (tipo, so_leitura) {
                ui.horizontal(|ui| {
                    for (tp, nome, dica) in [
                        (TipoNo::Suave, "Suave", "Superfície contínua aqui"),
                        (TipoNo::Vinco, "Vinco", "Tira a tangência ao longo da aresta, com o peso ao lado"),
                        (TipoNo::Canto, "Canto", "A superfície passa exatamente aqui, sem tangência"),
                    ] {
                        if chip(ui, t == tp, None, nome, dica) {
                            let s = sel.clone();
                            let peso_usado = if tp == TipoNo::Vinco && peso <= 0. { 1. } else { peso };
                            acao = Some(Box::new(move |a: &mut App| {
                                a.forma.peso = peso_usado;
                                editar(a, id, |g| fe::aplicar_tipo(g, &s, tp, peso_usado));
                            }));
                        }
                    }
                    if t == TipoNo::Vinco {
                        let mut v = w;
                        let r = ui.add(crate::ui_ext::slider(&mut v, 0.0..=1.0)).on_hover_text(tr("Peso do vinco: 0 é liso, 1 é aresta viva"));
                        if r.changed() {
                            let s = sel.clone();
                            acao = Some(Box::new(move |a: &mut App| {
                                a.forma.peso = v;
                                editar(a, id, |g| fe::aplicar_tipo(g, &s, TipoNo::Vinco, v));
                            }));
                        }
                    }
                });
            }
            if let Ferramenta::Suavizar { raio, segmentos } = ferramenta {
                ui.horizontal(|ui| {
                    let (mut r, mut s) = (raio, segmentos);
                    ui.label(tr("Raio"));
                    let a = ui.add(crate::ui_ext::dv(&mut r).speed(0.1).range(0.01..=1000.).dica("Quanto a faixa recua de cada lado da aresta")).changed();
                    ui.label(tr("Segmentos"));
                    let b = ui.add(crate::ui_ext::dv(&mut s).range(1..=8).dica("Faces da faixa")).changed();
                    if a || b {
                        acao = Some(Box::new(move |x: &mut App| x.forma.ferramenta = Ferramenta::Suavizar { raio: r, segmentos: s }));
                    }
                });
            }
            if !sel.is_empty() && tem_face && !so_leitura {
                ui.label(RichText::new(tr("Alt ao arrastar a face extruda")).weak().small());
            }
        });
    });
    if let Some(f) = acao {
        f(app);
    }
}

/// Painel do comando: só o que não tem gesto no 3D.
pub fn painel(ui: &mut egui::Ui, f: &mut Forma) {
    section(ui, "Superfície");
    let mut n = f.niveis as u32;
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Suavidade")));
        if ui.add(crate::ui_ext::dv(&mut n).range(1..=5).dica("Passos de subdivisão da superfície mostrada")).changed() {
            f.niveis = n as u8;
        }
    });
    section(ui, "Gaiola");
    let irr = f.gaiola.inteira().nos_irregulares().len();
    ui.label(RichText::new(format!("{} {} · {} {} · {} {}", f.gaiola.nos.len(), tr("nós"), f.gaiola.faces.len(), tr("faces"), irr, tr("nós irregulares"))).weak());
    if f.gaiola.faces.iter().any(|x| x.nos.len() != 4) {
        ui.label(RichText::new(tr("Há faces que não são quadriláteros: a superfície funciona, mas com mais nós irregulares.")).weak().small());
    }
    if ui.checkbox(&mut f.congelada, tr("Congelar")).on_hover_text(tr("Guarda a gaiola e trava a edição, para não mexer sem querer")).changed() {}
}

/// Criação por arraste no plano.
fn criar(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let Some(mut c) = app.forma.criando.clone() else { return };
    let r = ui.interact(rect, egui::Id::unique("forma_criar"), egui::Sense::click_and_drag());
    let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
    if esc {
        app.forma.criando = None;
        return;
    }
    let pinta = ui.painter_at(rect);
    let (cor, texto) = (crate::theme::accent(ui.visuals()), ui.visuals().text_color());
    let ponteiro = r.hover_pos().or(ui.input(|i| i.pointer.latest_pos()));
    let ponto = ponteiro.and_then(|m| no_plano(cam, rect, m, &c.plano));
    if r.drag_started_by(egui::PointerButton::Primary) {
        let origem = ui.input(|i| i.pointer.press_origin()).and_then(|m| no_plano(cam, rect, m, &c.plano));
        c.inicio = origem.or(ponto);
    }
    pinta.text(rect.center_top() + egui::vec2(0., 34.), egui::Align2::CENTER_TOP, tr("Arraste no plano para dar o tamanho. Esc desiste."), egui::FontId::proportional(13.), texto);
    if let (Some(a), Some(b)) = (c.inicio, ponto) {
        let ((u0, v0), (u1, v1)) = (coord(&c.plano, a), coord(&c.plano, b));
        let cantos: Vec<Pos2> = [(u0, v0), (u1, v0), (u1, v1), (u0, v1)].iter().filter_map(|(u, v)| cam.project(rect, v3(no_quadro(&c.plano, *u, *v, 0.)))).collect();
        if cantos.len() == 4 {
            pinta.add(egui::Shape::closed_line(cantos.clone(), Stroke::new(2., cor)));
            pinta.text(cantos[2] + egui::vec2(8., 0.), egui::Align2::LEFT_CENTER, format!("{} × {} mm", fmt((u1 - u0).abs()), fmt((v1 - v0).abs())), egui::FontId::proportional(12.), texto);
        }
    }
    let solto = r.drag_stopped_by(egui::PointerButton::Primary) || r.clicked_by(egui::PointerButton::Primary);
    if solto {
        let b = ponto.unwrap_or(c.plano[0]);
        let a = c.inicio.unwrap_or(b);
        let g = gaiola_do_arraste(c.tipo, &c.plano, a, b);
        app.forma.criando = None;
        criar_feature(app, g);
        return;
    }
    app.forma.criando = Some(c);
}

/// Põe a forma na árvore e abre a edição.
pub fn criar_feature(app: &mut App, gaiola: Gaiola) -> Id {
    let snapshot = app.open.doc.clone();
    let id = app.open.doc.add_feature(FeatureKind::Forma(Forma { gaiola, niveis: 3, congelada: false }));
    app.start_session(id, true, Some(snapshot));
    app.forma.sel.clear();
    app.forma.ferramenta = Ferramenta::Selecionar;
    app.request_recompute();
    app.scene_dirty = true;
    id
}

/// Desenvolvimento: `FENKRA_DEMO_FORMA=1` abre a cúpula em edição, com o nó do topo selecionado.
pub fn demo(app: &mut App, modo: &str) {
    let mut g = Gaiola::caixa([0., 0., 15.], [60., 60., 30.], [2, 2, 2]);
    fe::alternar_simetria(&mut g, 0).ok();
    let topo = g.nos.iter().find(|n| n.p[0].abs() < 1e-9 && n.p[1].abs() < 1e-9 && n.p[2] > 29.).map(|n| n.id);
    if let Some(n) = topo.and_then(|t| g.no_mut(t)) {
        n.p[2] += 35.;
    }
    if let Some((a, b)) = g.arestas().into_iter().find(|(a, b)| [a, b].iter().all(|x| g.pos(**x).is_some_and(|p| p[2] < 1e-6 && p[1] > 29.))) {
        g.definir_vinco(a, b, 1.);
    }
    app.settings.bench = Bench::FormaLivre;
    criar_feature(app, g);
    if let Some(t) = topo {
        app.forma.sel = vec![Elem::No(t)];
    }
    match modo {
        "gaiola" => app.forma.modo = ModoVista::Gaiola,
        "liso" => app.forma.modo = ModoVista::Liso,
        "laco" => app.forma.ferramenta = Ferramenta::Laco,
        "tangencia" => app.forma.ferramenta = Ferramenta::Tangencia,
        _ => {}
    }
}
