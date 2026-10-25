//! Bancada Desenho (detalhamento 2D): folha com zoom/pan, vistas associativas (projeção HLR da
//! peça ou montagem salva), cortes, detalhes, cotas com snap nos vértices, notas, símbolos,
//! tabelas, legenda e exportação PDF/SVG/DXF.

use crate::app::App;
use crate::i18n::tr;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use scc_core::Id;
use scc_desenho::layout::{self, Align, Item, Prim, Style};
use scc_desenho::source::Models;
use scc_desenho::*;
use std::path::PathBuf;

/// Comandos (índice do `Cmd::Dwg`).
pub mod c {
    pub const NEW: u8 = 0;
    pub const OPEN: u8 = 1;
    pub const SAVE: u8 = 2;
    pub const PDF: u8 = 3;
    pub const SVG: u8 = 4;
    pub const DXF: u8 = 5;
    pub const SET: u8 = 10;
    pub const VIEW: u8 = 11;
    pub const PROJ: u8 = 12;
    pub const SECTION: u8 = 13;
    pub const DETAIL: u8 = 14;
    pub const FLAT: u8 = 15;
    pub const ISO: u8 = 16;
    pub const SCHEMA: u8 = 17;
    pub const VIEW_CAM: u8 = 18;
    pub const DIM_H: u8 = 20;
    pub const DIM_V: u8 = 21;
    pub const DIM_A: u8 = 22;
    pub const DIM_R: u8 = 23;
    pub const DIM_D: u8 = 24;
    pub const DIM_ANG: u8 = 25;
    pub const NOTE: u8 = 30;
    pub const BALLOON: u8 = 31;
    pub const ROUGH: u8 = 32;
    pub const WELD: u8 = 33;
    pub const CENTER: u8 = 34;
    pub const AUTO_BALLOONS: u8 = 35;
    pub const GDT: u8 = 36;
    pub const DATUM: u8 = 37;
    pub const T_BOM: u8 = 40;
    pub const T_HOLES: u8 = 41;
    pub const T_BENDS: u8 = 42;
    pub const T_CUT: u8 = 43;
    pub const SHEET: u8 = 50;
}

pub struct DwgState {
    pub d: Drawing,
    pub path: Option<PathBuf>,
    pub models: Models,
    pub sheet: Id,
    pub sel: Option<Id>,
    /// comando esperando cliques na folha
    pub tool: Option<u8>,
    /// cliques já dados (mm na folha) e a vista do primeiro
    clicks: Vec<V2>,
    tool_view: Option<Id>,
    /// origem da próxima vista/tabela
    pub source: Option<Source>,
    pub new_dir: Dir,
    pub new_scale: f64,
    /// canto superior esquerdo da folha na tela e px por mm (None = enquadrar)
    origin: Option<Vec2>,
    zoom: f32,
    undo: Vec<Drawing>,
    redo: Vec<Drawing>,
    last: Drawing,
    pub modified: bool,
    items: Option<Vec<Item>>,
    boxes: Vec<(Id, V2, V2)>,
    last_check: f64,
    dragging: bool,
    pub renaming: Option<Id>,
    /// painel com as propriedades do desenho (folha, origem, legenda) aberto
    pub props_open: bool,
    /// realçado com 1 clique (2 cliques = `sel`, em edição)
    pub focus: Option<Id>,
    /// item do menu do botão direito
    ctx_item: Option<Id>,
    /// o que o arraste move: 0 o item, 1 o texto da cota, 2 a ponta da chamada
    drag_kind: u8,
    /// estilo copiado (colar em outra cota/nota/vista)
    style_clip: Option<scc_core::pmi::PmiStyle>,
    /// direção da câmera do 3D para a próxima vista (para o observador, eixo x)
    cam: Option<(V3, V3)>,
}

impl DwgState {
    /// Concluir: fecha a ferramenta em andamento e solta a seleção.
    pub fn conclude(&mut self) {
        self.tool = None;
        self.clicks.clear();
        self.tool_view = None;
        self.sel = None;
        self.focus = None;
        self.props_open = false;
    }

    /// Item em edição ou, se nenhum, o realçado.
    pub fn cur(&self) -> Option<Id> {
        self.sel.or(self.focus)
    }

    /// Algo aberto no painel (ferramenta, item selecionado ou propriedades)?
    pub fn active(&self) -> bool {
        self.tool.is_some() || self.sel.is_some() || self.props_open
    }

    pub fn new(d: Drawing, path: Option<PathBuf>) -> Self {
        let dir = path.as_ref().and_then(|p| p.parent()).map(|p| p.to_path_buf());
        let sheet = d.sheets[0].id;
        let source = d.views.first().map(|v| v.source.clone());
        DwgState {
            last: d.clone(),
            d,
            path,
            models: Models::new(dir),
            sheet,
            sel: None,
            tool: None,
            clicks: vec![],
            tool_view: None,
            source,
            new_dir: Dir::Frente,
            new_scale: 1.,
            origin: None,
            zoom: 2.,
            undo: vec![],
            redo: vec![],
            modified: false,
            items: None,
            boxes: vec![],
            last_check: 0.,
            dragging: false,
            renaming: None,
            props_open: false,
            focus: None,
            ctx_item: None,
            drag_kind: 0,
            style_clip: None,
            cam: None,
        }
    }

    fn invalidate(&mut self) {
        self.items = None;
    }

    /// Caminho da origem relativo à pasta do desenho (se der).
    fn rel(&self, p: &std::path::Path) -> String {
        if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
            if let Ok(r) = p.strip_prefix(dir) {
                return r.to_string_lossy().replace('\\', "/");
            }
        }
        p.to_string_lossy().to_string()
    }

    fn next_label(&self) -> String {
        let used: Vec<String> = self
            .d
            .views
            .iter()
            .filter_map(|v| match &v.kind {
                ViewKind::Section { label, .. } | ViewKind::Detail { label, .. } => Some(label.clone()),
                _ => None,
            })
            .collect();
        ('A'..='Z').map(|c| c.to_string()).find(|l| !used.contains(l)).unwrap_or_else(|| "Z".into())
    }
}

/// Estado do desenho (cria um vazio na primeira vez que a bancada abre).
pub fn ensure(app: &mut App) {
    if app.dwg.is_none() {
        let mut st = DwgState::new(Drawing::new(&tr("Desenho"), Format::A3), None);
        st.source = current_source(app);
        app.dwg = Some(st);
    }
}

/// Peça ou montagem aberta como origem (precisa estar salva).
fn current_source(app: &App) -> Option<Source> {
    let f = app.asm.as_ref().and_then(|s| s.file.clone()).filter(|_| app.settings.bench == crate::settings::Bench::Montagem).or_else(|| app.open.file.clone())?;
    Some(Source { path: f.to_string_lossy().to_string(), display_state: String::new(), hidden: vec![] })
}

fn hint(tool: u8, n: usize) -> &'static str {
    match (tool, n) {
        (c::SET | c::VIEW | c::ISO | c::FLAT | c::SCHEMA, _) => "Clique na folha para posicionar a vista",
        (c::VIEW_CAM, _) => "Clique na folha para posicionar a vista (direção da câmera do 3D)",
        (c::GDT | c::DATUM, 0) => "Clique o elemento indicado (ponta da chamada; presa à vista)",
        (c::GDT | c::DATUM, _) => "Clique onde o símbolo vai ficar",
        (c::PROJ, _) => "Clique ao lado da vista selecionada: abaixo = superior, à direita = lateral esquerda (1º diedro)",
        (c::SECTION, 0) => "Clique o primeiro ponto da linha de corte na vista",
        (c::SECTION, 1) => "Clique o segundo ponto da linha de corte",
        (c::SECTION, _) => "Clique onde a vista em corte vai ficar",
        (c::DETAIL, 0) => "Clique o centro do detalhe na vista",
        (c::DETAIL, 1) => "Clique para definir o raio do detalhe",
        (c::DETAIL, _) => "Clique onde a vista de detalhe vai ficar",
        (c::DIM_R | c::DIM_D, _) => "Clique num arco ou círculo da vista",
        (c::DIM_ANG, 0) => "Clique o vértice do ângulo",
        (c::DIM_ANG, _) => "Clique um ponto de cada lado do ângulo",
        (c::DIM_H | c::DIM_V | c::DIM_A, 0) => "Clique o primeiro vértice (snap nas arestas da vista)",
        (c::DIM_H | c::DIM_V | c::DIM_A, _) => "Clique o segundo vértice",
        (c::BALLOON | c::ROUGH | c::WELD, 0) => "Clique o ponto indicado (ponta da chamada)",
        (c::BALLOON | c::ROUGH | c::WELD, _) => "Clique onde o símbolo vai ficar",
        (c::NOTE | c::CENTER, _) => "Clique onde a nota vai ficar",
        (c::T_BOM | c::T_HOLES | c::T_BENDS | c::T_CUT, _) => "Clique o canto superior esquerdo da tabela",
        _ => "",
    }
}

/// Executa um comando da barra.
pub fn command(app: &mut App, i: u8) {
    ensure(app);
    match i {
        c::NEW => {
            if app.dwg.as_ref().is_some_and(|s| s.modified) {
                app.toast(tr("O desenho atual tinha alterações não salvas — começando um novo."), false);
            }
            let mut st = DwgState::new(Drawing::new(&tr("Desenho"), Format::A3), None);
            st.source = current_source(app);
            app.dwg = Some(st);
        }
        c::OPEN => {
            let Some(p) = rfd::FileDialog::new().add_filter(tr("Desenho SolidCadCraft"), &["sccdwg"]).pick_file() else { return };
            match std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|t| Drawing::from_json(&t)) {
                Ok(d) => app.dwg = Some(DwgState::new(d, Some(p))),
                Err(e) => app.toast(format!("{}: {e}", tr("Não abriu o desenho")), true),
            }
        }
        c::SAVE => save(app, false),
        c::PDF | c::SVG | c::DXF => {
            let ext = match i {
                c::PDF => "pdf",
                c::SVG => "svg",
                _ => "dxf",
            };
            let st = app.dwg.as_mut().unwrap();
            let name = format!("{}.{ext}", st.d.name);
            let Some(p) = rfd::FileDialog::new().add_filter(ext.to_uppercase(), &[ext]).set_file_name(name).save_file() else { return };
            let sheet = st.sheet;
            match export::save(&st.d, sheet, &mut st.models, &p.to_string_lossy()) {
                Ok(()) => app.toast(format!("{} {}", tr("Exportado:"), p.display()), false),
                Err(e) => app.toast(e, true),
            }
        }
        c::SHEET => {
            let st = app.dwg.as_mut().unwrap();
            let f = st.d.sheet(st.sheet).map(|s| s.format).unwrap_or(Format::A3);
            st.sheet = st.d.add_sheet(f);
        }
        c::AUTO_BALLOONS => auto_balloons(app),
        _ => {
            // vista pela câmera: direção atual do 3D (peça ou montagem)
            let cam = app.viewport.as_ref().map(|v| {
                let c = v.camera;
                let t = (c.eye() - c.target).normalize_or_zero();
                let (r, _) = c.basis();
                ([t.x as f64, t.y as f64, t.z as f64], [r.x as f64, r.y as f64, r.z as f64])
            });
            if i == c::VIEW_CAM && cam.is_none() {
                app.toast(tr("Abra a peça ou a montagem no 3D e escolha a vista com a câmera"), true);
                return;
            }
            let st = app.dwg.as_mut().unwrap();
            st.cam = cam;
            let needs_parent = matches!(i, c::PROJ | c::SECTION | c::DETAIL);
            let needs_view = (c::DIM_H..=c::DIM_ANG).contains(&i);
            let needs_source = matches!(i, c::SET | c::VIEW | c::ISO | c::FLAT | c::SCHEMA | c::VIEW_CAM | c::T_BOM | c::T_HOLES | c::T_BENDS | c::T_CUT);
            if needs_parent && !st.cur().is_some_and(|s| st.d.view(s).is_some_and(|v| !matches!(v.kind, ViewKind::Detail { .. }))) {
                app.toast(tr("Selecione antes a vista de origem (clique nela na folha)"), true);
                return;
            }
            if needs_source && st.source.is_none() {
                st.props_open = true;
                app.toast(tr("Escolha a origem (peça ou montagem salva) no painel do desenho"), true);
                return;
            }
            if needs_view && st.d.views.iter().all(|v| v.sheet != st.sheet) {
                app.toast(tr("A folha não tem vistas para cotar"), true);
                return;
            }
            if needs_parent {
                st.sel = st.cur();
            }
            st.tool = Some(i);
            st.clicks.clear();
            st.tool_view = None;
        }
    }
}

fn save(app: &mut App, ask: bool) {
    let Some(st) = app.dwg.as_mut() else { return };
    let path = match (&st.path, ask) {
        (Some(p), false) => p.clone(),
        _ => {
            let Some(p) = rfd::FileDialog::new().add_filter(tr("Desenho SolidCadCraft"), &["sccdwg"]).set_file_name(format!("{}.sccdwg", st.d.name)).save_file() else { return };
            p
        }
    };
    // caminhos das origens ficam relativos à pasta do desenho quando possível
    let dir = path.parent().map(|p| p.to_path_buf());
    if st.path.as_ref().and_then(|p| p.parent()) != dir.as_deref() {
        let old = st.models.dir.clone();
        let fix = |s: &mut Source| {
            let full = scc_montagem::load::full_path(old.as_deref(), &s.path);
            if let Some(r) = dir.as_ref().and_then(|d| full.strip_prefix(d).ok()) {
                s.path = r.to_string_lossy().replace('\\', "/");
            } else {
                s.path = full.to_string_lossy().to_string();
            }
        };
        for v in &mut st.d.views {
            fix(&mut v.source);
        }
        for t in &mut st.d.tables {
            fix(&mut t.source);
        }
        if let Some(s) = st.source.as_mut() {
            fix(s);
        }
        st.models = Models::new(dir);
        st.last = st.d.clone();
        st.invalidate();
    }
    match std::fs::write(&path, st.d.to_json()) {
        Ok(()) => {
            st.path = Some(path);
            st.modified = false;
            app.toast(tr("Desenho salvo"), false);
        }
        Err(e) => app.toast(e.to_string(), true),
    }
}

/// Balões numerados para cada item da lista de materiais, apontando para o centro da peça na
/// primeira vista da montagem.
fn auto_balloons(app: &mut App) {
    let st = app.dwg.as_mut().unwrap();
    let Some(v) = st.d.views.iter().find(|v| v.sheet == st.sheet && v.source.path.to_lowercase().ends_with(".sccasm")).cloned() else {
        app.toast(tr("Coloque antes uma vista da montagem na folha"), true);
        return;
    };
    let full = st.models.full(&v.source.path);
    let Ok(a) = std::fs::read_to_string(&full).map_err(|e| e.to_string()).and_then(|t| scc_montagem::Assembly::from_json(&t)) else {
        app.toast(tr("Não consegui ler a montagem"), true);
        return;
    };
    let p = st.models.proj(&st.d, v.id);
    let Some(ax) = p.axes else { return };
    let dir = full.parent().map(|p| p.to_path_buf());
    let mut cache = scc_montagem::load::PartCache::default();
    let mut paths: Vec<String> = vec![];
    let (lo, hi) = layout::view_rect(&v, &p);
    let mut k = 0;
    let items = scc_montagem::bom::bom(&a, dir.as_deref(), scc_montagem::bom::BomLevel::Topo);
    for n in &a.nodes {
        let (scc_montagem::NodeKind::Part { path } | scc_montagem::NodeKind::SubAssembly { path }) = &n.kind else { continue };
        if paths.contains(path) || !scc_montagem::view::shown(&a, n.id) {
            continue;
        }
        paths.push(path.clone());
        let fp = scc_montagem::load::full_path(dir.as_deref(), path);
        let item = items.iter().find(|r| r.path == fp).map(|r| r.item.clone()).unwrap_or_else(|| paths.len().to_string());
        let Some(sh) = scc_montagem::view::placed_shape(&a, n.id, dir.as_deref(), &mut cache) else { continue };
        let Some((l, h)) = sh.bbox() else { continue };
        let c3 = [0, 1, 2].map(|i| (l[i] + h[i]) / 2.);
        let tip = layout::to_sheet(&v, &p, ax.project(c3));
        // balões em volta da vista, do lado mais próximo
        let right = tip[0] > (lo[0] + hi[0]) / 2.;
        let pos = [if right { hi[0] + 12. } else { lo[0] - 12. }, (tip[1] + 6. * (k % 3) as f64).clamp(lo[1], hi[1] + 10.)];
        let id = st.d.alloc();
        st.d.notes.push(Note { id, sheet: st.sheet, kind: NoteKind::Balloon, pos, leader: Some(tip), text: item, size: st.d.text_height, weld: None, view: Some(v.id), anchor3: Some(c3), ..Default::default() });
        k += 1;
    }
    if k == 0 {
        app.toast(tr("A montagem não tem peças visíveis"), true);
    }
}

/// Atalhos da bancada.
pub fn keys(app: &mut App, ctx: &egui::Context, save_k: bool, undo: bool, redo: bool, del: bool) {
    ensure(app);
    if save_k {
        save(app, false);
    }
    let typing = ctx.egui_wants_keyboard_input();
    let st = app.dwg.as_mut().unwrap();
    if undo {
        if let Some(prev) = st.undo.pop() {
            st.redo.push(std::mem::replace(&mut st.d, prev.clone()));
            st.last = prev;
            st.invalidate();
        }
    }
    if redo {
        if let Some(next) = st.redo.pop() {
            st.undo.push(std::mem::replace(&mut st.d, next.clone()));
            st.last = next;
            st.invalidate();
        }
    }
    if !typing && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        st.tool = None;
        st.clicks.clear();
    }
    if del && !typing {
        if let Some(id) = st.sel.take().or(st.focus.take()) {
            st.d.delete(id);
        }
    }
    if !typing && ctx.input(|i| i.key_pressed(egui::Key::F2)) {
        st.renaming = st.sel;
    }
    if !typing && ctx.input(|i| i.key_pressed(egui::Key::F5)) {
        st.models.clear();
        st.invalidate();
    }
}

fn style_px(s: Style, z: f32) -> f32 {
    (s.width() as f32 * z).clamp(0.6, 6.)
}

/// Folha na tela.
pub fn canvas(app: &mut App, root: &mut egui::Ui) {
    ensure(app);
    let dark = app.settings.theme == crate::settings::Theme::Escuro;
    egui::CentralPanel::no_frame().show(root, |ui| {
        let rect = ui.max_rect();
        let resp = ui.allocate_rect(rect, egui::Sense::click_and_drag());
        let st = app.dwg.as_mut().unwrap();
        let now = ui.ctx().input(|i| i.time);
        // o 3D mudou no disco? (de tempos em tempos)
        if now - st.last_check > 1.5 {
            st.last_check = now;
            if st.models.refresh() {
                st.invalidate();
            }
        }
        // mudanças no desenho: desfazer e recalcular a folha
        layout::align_views(&mut st.d);
        if st.d != st.last {
            if !st.dragging || st.undo.is_empty() {
                st.undo.push(st.last.clone());
                if st.undo.len() > 100 {
                    st.undo.remove(0);
                }
            }
            st.redo.clear();
            st.last = st.d.clone();
            st.modified = true;
            st.invalidate();
        }
        if !st.d.sheets.iter().any(|s| s.id == st.sheet) {
            st.sheet = st.d.sheets[0].id;
        }
        let size = st.d.sheet(st.sheet).map(|s| s.size()).unwrap_or([420., 297.]);
        let (w, h) = (size[0] as f32, size[1] as f32);
        if st.origin.is_none() {
            st.zoom = ((rect.width() - 40.) / w).min((rect.height() - 40.) / h).max(0.2);
            st.origin = Some(rect.center().to_vec2() - Vec2::new(w, h) * st.zoom / 2.);
        }
        // navegação: roda = zoom no cursor; botão do meio/direito = arrastar
        let hover = resp.hover_pos();
        let scroll = ui.ctx().input(|i| i.smooth_scroll_delta.y);
        if let (Some(m), true) = (hover, scroll != 0.) {
            let f = (scroll * 0.0025).exp();
            let o = st.origin.unwrap();
            let nz = (st.zoom * f).clamp(0.1, 60.);
            let k = nz / st.zoom;
            st.origin = Some(m.to_vec2() - (m.to_vec2() - o) * k);
            st.zoom = nz;
        }
        if resp.dragged_by(egui::PointerButton::Middle) || resp.dragged_by(egui::PointerButton::Secondary) {
            st.origin = Some(st.origin.unwrap() + resp.drag_delta());
        }
        let (o, z) = (st.origin.unwrap(), st.zoom);
        let to_scr = |p: V2| Pos2::new(o.x + p[0] as f32 * z, o.y + (h - p[1] as f32) * z);
        let to_mm = |s: Pos2| [((s.x - o.x) / z) as f64, (h - (s.y - o.y) / z) as f64];
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0., if dark { Color32::from_rgb(52, 56, 64) } else { Color32::from_rgb(170, 176, 186) });
        let sheet_r = Rect::from_min_max(to_scr([0., size[1]]), to_scr([size[0], 0.]));
        painter.rect_filled(sheet_r.translate(Vec2::new(4., 4.)), 0., Color32::from_black_alpha(70));
        painter.rect_filled(sheet_r, 0., Color32::WHITE);
        // primitivas (refeitas só quando algo muda)
        if st.items.is_none() {
            let items = layout::sheet(&st.d, st.sheet, &mut st.models);
            st.boxes = layout::owner_boxes(&items);
            st.items = Some(items);
        }
        // seleção: menor caixa que contém o ponto
        let mouse_mm = hover.map(to_mm);
        let pick = |st: &DwgState, p: V2| -> Option<Id> {
            st.boxes
                .iter()
                .filter(|(_, lo, hi)| p[0] >= lo[0] - 1. && p[0] <= hi[0] + 1. && p[1] >= lo[1] - 1. && p[1] <= hi[1] + 1.)
                .min_by(|a, b| ((a.2[0] - a.1[0]) * (a.2[1] - a.1[1])).total_cmp(&((b.2[0] - b.1[0]) * (b.2[1] - b.1[1]))))
                .map(|x| x.0)
        };
        let hot = if st.tool.is_none() { mouse_mm.and_then(|p| pick(st, p)) } else { None };
        let bold_fam = if crate::pmi_draw::fonts().iter().any(|f| f == "Arial negrito") { egui::FontFamily::Name("Arial negrito".into()) } else { egui::FontFamily::Proportional };
        let active = st.cur();
        let is_dim = |id: Option<Id>| id.is_some_and(|i| st.d.dims.iter().any(|d| d.id == i));
        // alça do texto da cota em edição/realçada (centro do texto, mm) e ponta da chamada
        let mut text_handle: Option<(Id, V2, f64)> = None;
        let tip_handle: Option<(Id, V2)> = active.and_then(|a| st.d.notes.iter().find(|n| n.id == a && n.anchor3.is_none()).and_then(|n| n.leader.map(|l| (n.id, l))));
        let items = st.items.as_ref().unwrap();
        for it in items {
            let mine = it.owner.is_some();
            let own = it.color.map(|c| Color32::from_rgb(c[0], c[1], c[2])).unwrap_or(Color32::BLACK);
            let col = if mine && it.owner == st.sel {
                Color32::from_rgb(20, 110, 230)
            } else if mine && (it.owner == hot || it.owner == st.focus) {
                Color32::from_rgb(70, 150, 255)
            } else {
                own
            };
            match &it.prim {
                Prim::Line { pts, style } => {
                    let pts: Vec<Pos2> = pts.iter().map(|p| to_scr(*p)).collect();
                    let w = it.width.map(|w| (w as f32 * z).clamp(0.6, 14.)).unwrap_or_else(|| style_px(*style, z));
                    let stroke = Stroke::new(w, col);
                    let dash = style.dash();
                    if dash.is_empty() {
                        painter.add(egui::Shape::line(pts, stroke));
                    } else {
                        let (d, g) = ((dash[0] as f32 * z).max(3.), (dash[1] as f32 * z).max(2.));
                        painter.extend(egui::Shape::dashed_line(&pts, stroke, d, g));
                    }
                }
                Prim::Fill { pts } => {
                    // fundos e sombreado (cor própria) não mudam com o realce; pintados como malha
                    // (sem anti-serrilhado: triângulos finos não viram espinhos)
                    if it.color.is_some() {
                        let mut m = egui::Mesh::default();
                        for p in pts {
                            m.colored_vertex(to_scr(*p), own);
                        }
                        for k in 1..pts.len().saturating_sub(1) {
                            m.add_triangle(0, k as u32, k as u32 + 1);
                        }
                        painter.add(egui::Shape::mesh(m));
                    } else {
                        painter.add(egui::Shape::convex_polygon(pts.iter().map(|p| to_scr(*p)).collect(), col, Stroke::NONE));
                    }
                }
                Prim::Text { pos, text, size, angle, align } => {
                    if mine && it.owner == active && is_dim(it.owner) {
                        let a = angle.to_radians();
                        text_handle = Some((it.owner.unwrap(), [pos[0] - a.sin() * size / 2., pos[1] + a.cos() * size / 2.], *size));
                    }
                    let px = (*size as f32 * z / 0.72).max(1.);
                    if px < 2.5 {
                        continue;
                    }
                    let fam = if it.bold { bold_fam.clone() } else { egui::FontFamily::Proportional };
                    let g = painter.layout_no_wrap(text.clone(), egui::FontId::new(px, fam), col);
                    let fx = match align {
                        Align::Left => 0.,
                        Align::Center => -g.size().x / 2.,
                        Align::Right => -g.size().x,
                    };
                    // ângulo da folha (anti-horário) → tela (y para baixo)
                    let th = -(*angle as f32).to_radians();
                    let off = Vec2::new(fx, -px * 0.95);
                    let rot = Vec2::new(off.x * th.cos() - off.y * th.sin(), off.x * th.sin() + off.y * th.cos());
                    let base = to_scr(*pos);
                    painter.add(egui::epaint::TextShape::new(base + rot, g, col).with_angle(th));
                }
            }
        }
        // alças: texto da cota (arrastar solta o texto) e ponta da chamada
        let orange = Color32::from_rgb(255, 130, 20);
        if st.tool.is_none() {
            if let Some((_, c, _)) = text_handle {
                painter.rect_stroke(Rect::from_center_size(to_scr(c), Vec2::splat(11.)), 2., Stroke::new(1.8, orange), egui::StrokeKind::Middle);
            }
            if let Some((_, t)) = tip_handle {
                painter.circle_stroke(to_scr(t), 5.5, Stroke::new(1.8, orange));
            }
        }
        // ---------- ferramentas
        let snap = |st: &mut DwgState, p: V2, view_only: Option<Id>| -> Option<(Id, V2, V2)> {
            // vértice mais próximo (extremos das polilinhas) das vistas da folha, em mm da folha
            let tol = (10. / z) as f64;
            let mut best: Option<(f64, Id, V2, V2)> = None;
            let views: Vec<View> = st.d.views.iter().filter(|v| v.sheet == st.sheet && view_only.is_none_or(|x| x == v.id)).cloned().collect();
            for v in &views {
                let pr = st.models.proj(&st.d, v.id);
                for (pl, _) in &pr.lines {
                    let n = pl.len();
                    let mut cands = vec![pl[0], pl[n - 1]];
                    if n > 2 {
                        // ponto médio das retas (para cotar até o meio) não; centro dos arcos sim
                        if let Some((c, _)) = arc_center(pl) {
                            cands.push(c);
                        }
                    }
                    for q in cands {
                        let s = layout::to_sheet(v, &pr, q);
                        let d = ((s[0] - p[0]).powi(2) + (s[1] - p[1]).powi(2)).sqrt();
                        if d < tol && best.as_ref().is_none_or(|b| d < b.0) {
                            best = Some((d, v.id, s, q));
                        }
                    }
                }
            }
            best.map(|b| (b.1, b.2, b.3))
        };
        let view_at = |st: &DwgState, p: V2| -> Option<Id> {
            st.boxes.iter().filter(|(id, lo, hi)| st.d.view(*id).is_some() && p[0] >= lo[0] && p[0] <= hi[0] && p[1] >= lo[1] && p[1] <= hi[1]).map(|x| x.0).next()
        };
        if let Some(tool) = st.tool {
            painter.text(rect.left_top() + Vec2::new(12., 10.), egui::Align2::LEFT_TOP, format!("{}   (Esc {})", tr(hint(tool, st.clicks.len())), tr("cancela")), egui::FontId::proportional(14.), if dark { Color32::from_rgb(255, 210, 120) } else { Color32::from_rgb(120, 60, 0) });
            let dim_tool = (c::DIM_H..=c::DIM_ANG).contains(&tool);
            // marcador do snap sob o mouse
            if let Some(m) = mouse_mm {
                if dim_tool && !matches!(tool, c::DIM_R | c::DIM_D) {
                    if let Some((_, s, _)) = snap(st, m, st.tool_view) {
                        painter.circle_stroke(to_scr(s), 5., Stroke::new(2., Color32::from_rgb(255, 120, 0)));
                    }
                }
                for c in &st.clicks {
                    painter.circle_filled(to_scr(*c), 3.5, Color32::from_rgb(255, 120, 0));
                }
                if let Some(last) = st.clicks.last() {
                    painter.line_segment([to_scr(*last), to_scr(m)], Stroke::new(1., Color32::from_rgb(255, 120, 0)));
                }
            }
            if resp.clicked_by(egui::PointerButton::Primary) {
                if let Some(m) = mouse_mm {
                    click(app, m, &snap, &view_at);
                }
            }
        } else {
            // 1 clique realça, 2 cliques editam, clicar e arrastar move, botão direito = opções
            let near = |a: V2, b: V2, r: f64| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt() < r;
            if resp.double_clicked() {
                match mouse_mm.and_then(|p| pick(st, p)) {
                    Some(id) => {
                        st.sel = Some(id);
                        st.focus = Some(id);
                        st.props_open = false;
                    }
                    None => st.origin = None,
                }
            } else if resp.clicked_by(egui::PointerButton::Primary) {
                let p = mouse_mm.and_then(|p| pick(st, p));
                st.focus = p;
                if p != st.sel {
                    st.sel = None;
                }
            } else if resp.drag_started_by(egui::PointerButton::Primary) {
                let origin = ui.ctx().input(|i| i.pointer.press_origin()).map(to_mm).or(mouse_mm);
                st.drag_kind = 0;
                if let Some(o) = origin {
                    let tol = (8. / z) as f64;
                    if let Some((id, c, s)) = text_handle.filter(|(_, c, s)| near(o, *c, (s * 1.2).max(tol))) {
                        let _ = (c, s);
                        st.focus = Some(id);
                        st.drag_kind = 1;
                    } else if let Some((id, _)) = tip_handle.filter(|(_, t)| near(o, *t, tol)) {
                        st.focus = Some(id);
                        st.drag_kind = 2;
                    } else {
                        let id = pick(st, o);
                        if id != st.sel {
                            st.sel = None;
                        }
                        st.focus = id;
                    }
                }
                st.dragging = true;
            } else if resp.dragged_by(egui::PointerButton::Primary) && st.dragging {
                let dv = resp.drag_delta();
                let dm = [(dv.x / z) as f64, (-dv.y / z) as f64];
                if let Some(id) = st.focus {
                    match st.drag_kind {
                        1 => {
                            if let Some(d) = st.d.dims.iter_mut().find(|d| d.id == id) {
                                let t = d.text_pos.get_or_insert([0., 0.]);
                                *t = [t[0] + dm[0], t[1] + dm[1]];
                            }
                        }
                        2 => {
                            if let Some(l) = st.d.notes.iter_mut().find(|n| n.id == id).and_then(|n| n.leader.as_mut()) {
                                *l = [l[0] + dm[0], l[1] + dm[1]];
                            }
                        }
                        _ => move_item(st, id, dm),
                    }
                }
            }
            if resp.secondary_clicked() {
                st.ctx_item = mouse_mm.and_then(|p| pick(st, p));
                if st.ctx_item.is_some() {
                    st.focus = st.ctx_item;
                }
            }
        }
        let st = app.dwg.as_mut().unwrap();
        if resp.drag_stopped() {
            // ponta da chamada solta sobre uma aresta: fica presa à vista (associativa)
            if st.dragging && st.drag_kind == 2 {
                if let Some(id) = st.focus {
                    let tip = st.d.notes.iter().find(|n| n.id == id).and_then(|n| n.leader);
                    if let Some(t) = tip {
                        let hit = snap(st, t, None);
                        let assoc = hit.and_then(|(vid, s, q)| {
                            let p = st.models.proj(&st.d, vid);
                            p.axes.map(|ax| (vid, s, ax.unproject(q)))
                        });
                        if let Some(n) = st.d.notes.iter_mut().find(|n| n.id == id) {
                            if let Some((vid, s, a3)) = assoc {
                                n.view = Some(vid);
                                n.anchor3 = Some(a3);
                                n.leader = Some(s);
                            }
                        }
                    }
                }
            }
            st.dragging = false;
            st.drag_kind = 0;
        }
        if st.tool.is_none() {
            let item = st.ctx_item;
            resp.context_menu(|ui| ctx_menu(ui, st, item));
        }
        // folha e escala no rodapé
        let info = format!("{} — {} {}   ·   {}: {:.0}%", st.d.name, st.d.sheet(st.sheet).map(|s| s.name.clone()).unwrap_or_default(), st.d.sheet(st.sheet).map(|s| s.format.label()).unwrap_or(""), tr("zoom"), z / 3.78 * 100.);
        painter.text(rect.left_bottom() + Vec2::new(12., -10.), egui::Align2::LEFT_BOTTOM, info, egui::FontId::proportional(12.), if dark { Color32::from_gray(200) } else { Color32::from_gray(40) });
        // folhas: abas no rodapé à direita
        let sheets: Vec<(Id, String)> = st.d.sheets.iter().map(|s| (s.id, s.name.clone())).collect();
        let mut x = rect.right() - 12.;
        for (id, name) in sheets.iter().rev() {
            let g = painter.layout_no_wrap(name.clone(), egui::FontId::proportional(12.), Color32::BLACK);
            let r = Rect::from_min_size(Pos2::new(x - g.size().x - 14., rect.bottom() - 26.), Vec2::new(g.size().x + 14., 20.));
            let active = *id == st.sheet;
            painter.rect_filled(r, 4., if active { Color32::WHITE } else { Color32::from_gray(200) });
            painter.galley(r.min + Vec2::new(7., 3.), g, Color32::BLACK);
            if ui.interact(r, ui.scope_id().with(("aba_folha", id.0)), egui::Sense::click()).clicked() {
                st.sheet = *id;
                st.sel = None;
                st.invalidate();
                st.origin = None;
            }
            x = r.left() - 4.;
        }
    });
}

/// Centro e raio de uma polilinha que é um arco (três pontos e conferência dos outros).
fn arc_center(pl: &[V2]) -> Option<(V2, f64)> {
    let n = pl.len();
    if n < 5 {
        return None;
    }
    let (a, b, c) = (pl[0], pl[n / 2], pl[n - 1]);
    let (a, b, c) = if (a[0] - c[0]).abs() + (a[1] - c[1]).abs() < 1e-6 { (pl[0], pl[n / 3], pl[2 * n / 3]) } else { (a, b, c) };
    let d = 2. * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() < 1e-9 {
        return None;
    }
    let sq = |p: V2| p[0] * p[0] + p[1] * p[1];
    let ux = (sq(a) * (b[1] - c[1]) + sq(b) * (c[1] - a[1]) + sq(c) * (a[1] - b[1])) / d;
    let uy = (sq(a) * (c[0] - b[0]) + sq(b) * (a[0] - c[0]) + sq(c) * (b[0] - a[0])) / d;
    let r = ((a[0] - ux).powi(2) + (a[1] - uy).powi(2)).sqrt();
    let ok = pl.iter().all(|p| (((p[0] - ux).powi(2) + (p[1] - uy).powi(2)).sqrt() - r).abs() < r * 0.02 + 1e-3);
    (ok && r < 1e5).then_some(([ux, uy], r))
}

fn move_item(st: &mut DwgState, id: Id, dm: V2) {
    if let Some(v) = st.d.views.iter_mut().find(|v| v.id == id) {
        v.pos = [v.pos[0] + dm[0], v.pos[1] + dm[1]];
        // a vista mãe leva junto as projetadas (e o alinhamento mantém o resto)
        let mut kids: Vec<Id> = st.d.views.iter().filter(|c| matches!(&c.kind, ViewKind::Projected { parent, .. } if *parent == id)).map(|c| c.id).collect();
        for k in &kids {
            if let Some(c) = st.d.views.iter_mut().find(|c| c.id == *k) {
                c.pos = [c.pos[0] + dm[0], c.pos[1] + dm[1]];
            }
        }
        // as notas da vista andam junto (a ponta presa ao modelo se acerta sozinha)
        kids.push(id);
        for n in st.d.notes.iter_mut().filter(|n| n.view.is_some_and(|v| kids.contains(&v))) {
            n.pos = [n.pos[0] + dm[0], n.pos[1] + dm[1]];
            if n.anchor3.is_none() {
                if let Some(l) = n.leader.as_mut() {
                    *l = [l[0] + dm[0], l[1] + dm[1]];
                }
            }
        }
        return;
    }
    if let Some(n) = st.d.notes.iter_mut().find(|n| n.id == id) {
        n.pos = [n.pos[0] + dm[0], n.pos[1] + dm[1]];
        return;
    }
    if let Some(t) = st.d.tables.iter_mut().find(|t| t.id == id) {
        t.pos = [t.pos[0] + dm[0], t.pos[1] + dm[1]];
        return;
    }
    if let Some(dim) = st.d.dims.iter_mut().find(|d| d.id == id) {
        match dim.kind {
            DimKind::Horizontal | DimKind::Aligned => dim.offset += dm[1],
            DimKind::Vertical => dim.offset -= dm[0],
            _ => dim.offset += dm[0],
        }
    }
}

type SnapFn<'a> = dyn Fn(&mut DwgState, V2, Option<Id>) -> Option<(Id, V2, V2)> + 'a;
type ViewAt<'a> = dyn Fn(&DwgState, V2) -> Option<Id> + 'a;

fn click(app: &mut App, m: V2, snap: &SnapFn, view_at: &ViewAt) {
    let st = app.dwg.as_mut().unwrap();
    let Some(tool) = st.tool else { return };
    let sheet = st.sheet;
    let th = st.d.text_height;
    let done = |st: &mut DwgState, id: Id| {
        st.tool = None;
        st.clicks.clear();
        st.tool_view = None;
        st.sel = Some(id);
    };
    match tool {
        c::SET | c::VIEW | c::ISO | c::FLAT | c::SCHEMA | c::VIEW_CAM => {
            let src = st.source.clone().unwrap();
            let sc = st.new_scale;
            let id = match tool {
                c::SET => {
                    // espaçamento pelo tamanho da peça (a superior e a lateral não se encostam)
                    let f = st.d.add_view(sheet, src.clone(), ViewKind::Standard(Dir::Frente), m, sc);
                    let p = st.models.proj(&st.d, f);
                    let gap = ((p.hi[0] - p.lo[0]).max(p.hi[1] - p.lo[1]) * sc + 25.).max(40.);
                    st.d.delete(f);
                    st.d.add_standard_set(sheet, src, m, sc, gap)[0]
                }
                c::ISO => st.d.add_view(sheet, src, ViewKind::Standard(Dir::Iso), m, sc),
                c::FLAT => st.d.add_view(sheet, src, ViewKind::Flat, m, sc),
                c::SCHEMA => st.d.add_view(sheet, src, ViewKind::Schematic(st.new_dir), m, sc),
                c::VIEW_CAM => {
                    let (toward, x) = st.cam.unwrap_or(([0., -1., 0.], [1., 0., 0.]));
                    st.d.add_view(sheet, src, ViewKind::Camera { toward, x }, m, sc)
                }
                _ => st.d.add_view(sheet, src, ViewKind::Standard(st.new_dir), m, sc),
            };
            done(st, id);
        }
        c::PROJ => {
            let Some(parent) = st.cur().and_then(|s| st.d.view(s).cloned()) else { return };
            let (dx, dy) = (m[0] - parent.pos[0], m[1] - parent.pos[1]);
            let base = match &parent.kind {
                ViewKind::Standard(d) | ViewKind::Projected { dir: d, .. } => *d,
                _ => Dir::Frente,
            };
            // 1º diedro, a partir da frente: abaixo vê-se de cima, à direita vê-se da esquerda
            let dir = if dx.abs() > dy.abs() {
                match (base, dx > 0.) {
                    (Dir::Frente, true) => Dir::Esquerda,
                    (Dir::Frente, false) => Dir::Direita,
                    (Dir::Esquerda, true) => Dir::Tras,
                    (Dir::Direita, false) => Dir::Tras,
                    (_, true) => Dir::Esquerda,
                    (_, false) => Dir::Direita,
                }
            } else if dy < 0. {
                Dir::Topo
            } else {
                Dir::Base
            };
            let id = st.d.add_view(sheet, parent.source.clone(), ViewKind::Projected { parent: parent.id, dir }, m, parent.scale);
            done(st, id);
        }
        c::SECTION | c::DETAIL => {
            let Some(parent) = st.cur().and_then(|s| st.d.view(s).cloned()) else { return };
            st.clicks.push(m);
            if st.clicks.len() < 3 {
                return;
            }
            let p = st.models.proj(&st.d, parent.id);
            let a = layout::from_sheet(&parent, &p, st.clicks[0]);
            let b = layout::from_sheet(&parent, &p, st.clicks[1]);
            let label = st.next_label();
            let kind = if tool == c::SECTION {
                ViewKind::Section { parent: parent.id, a, b, label }
            } else {
                let r = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt().max(1.);
                ViewKind::Detail { parent: parent.id, center: a, radius: r, label }
            };
            let sc = if tool == c::DETAIL { parent.scale * 2. } else { parent.scale };
            let id = st.d.add_view(sheet, parent.source.clone(), kind, st.clicks[2], sc);
            done(st, id);
        }
        c::DIM_H | c::DIM_V | c::DIM_A | c::DIM_ANG => {
            let Some((vid, s, _)) = snap(st, m, st.tool_view).or_else(|| view_at(st, m).filter(|v| st.tool_view.is_none_or(|x| x == *v)).map(|v| (v, m, m))) else { return };
            st.tool_view = Some(vid);
            st.clicks.push(s);
            let need = if tool == c::DIM_ANG { 3 } else { 2 };
            if st.clicks.len() < need {
                return;
            }
            let Some(v) = st.d.view(vid).cloned() else { return };
            let p = st.models.proj(&st.d, vid);
            let Some(ax) = p.axes else { return };
            let anchors = st.clicks.iter().map(|c| ax.unproject(layout::from_sheet(&v, &p, *c))).collect();
            let kind = match tool {
                c::DIM_H => DimKind::Horizontal,
                c::DIM_V => DimKind::Vertical,
                c::DIM_A => DimKind::Aligned,
                _ => DimKind::Angle,
            };
            let id = st.d.alloc();
            st.d.dims.push(Dim { id, view: vid, kind, anchors, offset: if kind == DimKind::Angle { 0. } else { 8. }, text: String::new(), tol: String::new(), ..Default::default() });
            done(st, id);
        }
        c::DIM_R | c::DIM_D => {
            // arco mais próximo do clique
            let tol = 4.;
            let mut best: Option<(f64, Id, V2, V2)> = None;
            let views: Vec<View> = st.d.views.iter().filter(|v| v.sheet == sheet).cloned().collect();
            for v in &views {
                let p = st.models.proj(&st.d, v.id);
                let q = layout::from_sheet(v, &p, m);
                for (pl, _) in &p.lines {
                    let Some((c, r)) = arc_center(pl) else { continue };
                    let d = ((((q[0] - c[0]).powi(2) + (q[1] - c[1]).powi(2)).sqrt() - r) * v.scale).abs();
                    if d < tol && best.as_ref().is_none_or(|b| d < b.0) {
                        // ponto do arco na direção do clique
                        let u = [q[0] - c[0], q[1] - c[1]];
                        let l = (u[0] * u[0] + u[1] * u[1]).sqrt().max(1e-9);
                        best = Some((d, v.id, c, [c[0] + u[0] / l * r, c[1] + u[1] / l * r]));
                    }
                }
            }
            let Some((_, vid, c, on)) = best else {
                app.toast(tr("Nenhum arco ou círculo perto do clique"), true);
                return;
            };
            let p = st.models.proj(&st.d, vid);
            let Some(ax) = p.axes else { return };
            let id = st.d.alloc();
            st.d.dims.push(Dim { id, view: vid, kind: if tool == c::DIM_R { DimKind::Radius } else { DimKind::Diameter }, anchors: vec![ax.unproject(c), ax.unproject(on)], offset: 6., text: String::new(), tol: String::new(), ..Default::default() });
            done(st, id);
        }
        c::NOTE | c::CENTER => {
            let id = st.d.alloc();
            let (kind, text) = if tool == c::NOTE { (NoteKind::Text, tr("Nota").to_string()) } else { (NoteKind::CenterMark, "5".to_string()) };
            // dentro de uma vista: pertence a ela (anda junto)
            let view = view_at(st, m);
            st.d.notes.push(Note { id, sheet, kind, pos: m, leader: None, text, size: th, weld: None, view, ..Default::default() });
            done(st, id);
        }
        c::BALLOON | c::ROUGH | c::WELD | c::GDT | c::DATUM => {
            // 1º clique: ponta da chamada (snap nas arestas; presa à vista); 2º: posição do símbolo
            if st.clicks.is_empty() {
                match snap(st, m, None) {
                    Some((vid, s, _)) => {
                        st.tool_view = Some(vid);
                        st.clicks.push(s);
                    }
                    None => {
                        st.tool_view = view_at(st, m);
                        st.clicks.push(m);
                    }
                }
                return;
            }
            st.clicks.push(m);
            let (kind, text) = match tool {
                c::BALLOON => (NoteKind::Balloon, (st.d.notes.iter().filter(|n| n.kind == NoteKind::Balloon).count() + 1).to_string()),
                c::ROUGH => (NoteKind::Roughness, "Ra 1,6".to_string()),
                c::GDT => (NoteKind::Gdt, String::new()),
                c::DATUM => {
                    let used: Vec<&str> = st.d.notes.iter().filter(|n| n.kind == NoteKind::Datum).map(|n| n.text.as_str()).collect();
                    (NoteKind::Datum, ('A'..='Z').map(|c| c.to_string()).find(|l| !used.contains(&l.as_str())).unwrap_or_else(|| "A".into()))
                }
                _ => (NoteKind::Weld, "a4".to_string()),
            };
            let (view, anchor3) = match st.tool_view.and_then(|vid| st.d.view(vid).cloned()) {
                Some(v) => {
                    let p = st.models.proj(&st.d, v.id);
                    (Some(v.id), p.axes.map(|ax| ax.unproject(layout::from_sheet(&v, &p, st.clicks[0]))))
                }
                None => (None, None),
            };
            let gdt = (kind == NoteKind::Gdt).then(DwgGdt::default);
            let id = st.d.alloc();
            st.d.notes.push(Note { id, sheet, kind, pos: st.clicks[1], leader: Some(st.clicks[0]), text, size: th, weld: None, gdt, view, anchor3, ..Default::default() });
            done(st, id);
        }
        c::T_BOM | c::T_HOLES | c::T_BENDS | c::T_CUT => {
            let kind = match tool {
                c::T_BOM => TableKind::Bom,
                c::T_HOLES => TableKind::Holes,
                c::T_BENDS => TableKind::Bends,
                _ => TableKind::CutList,
            };
            let id = st.d.alloc();
            let source = st.source.clone().unwrap();
            st.d.tables.push(Table { id, sheet, kind, source, pos: m, level: Default::default() });
            done(st, id);
        }
        _ => st.tool = None,
    }
}

// ---------- árvore e painel

/// Cor própria (ou padrão) e espessura (mm) de um elemento 2D, para menus e painéis.
fn color_width(ui: &mut egui::Ui, color: &mut Option<[u8; 3]>, width: &mut f64) {
    ui.horizontal(|ui| {
        let mut own = color.is_some();
        if ui.checkbox(&mut own, tr("Cor")).changed() {
            *color = own.then_some([200, 30, 30]);
        }
        if let Some(c) = color {
            ui.color_edit_button_srgb(c);
        }
        ui.label(tr("Espessura"));
        let r = ui.add(egui::DragValue::new(width).range(0.0..=3.).speed(0.01).custom_formatter(|v, _| if v <= 0. { tr("padrão").to_string() } else { format!("{v:.2} mm") }));
        crate::ui_ext::wheel(ui, &r, width);
    });
}

/// O mesmo, para o estilo de anotação (espessura em f32).
fn style_color_width(ui: &mut egui::Ui, st: &mut scc_core::pmi::PmiStyle) {
    let mut w = st.line_width as f64;
    color_width(ui, &mut st.color, &mut w);
    st.line_width = w as f32;
}

/// Menu do botão direito de um item (na folha e na árvore); `None` = a folha.
fn ctx_menu(ui: &mut egui::Ui, st: &mut DwgState, id: Option<Id>) {
    let Some(id) = id else {
        if ui.button(tr("Ajustar à janela")).clicked() {
            st.origin = None;
            ui.close();
        }
        if ui.button(tr("Propriedades do desenho")).clicked() {
            st.props_open = true;
            st.sel = None;
            ui.close();
        }
        return;
    };
    if ui.button(tr("✏ Editar")).clicked() {
        st.sel = Some(id);
        st.focus = Some(id);
        st.props_open = false;
        ui.close();
    }
    let mut tool = None;
    let mut copy = None;
    let paste = st.style_clip.clone();
    if let Some(v) = st.d.views.iter_mut().find(|v| v.id == id) {
        if ui.button(tr("Renomear   F2")).clicked() {
            st.renaming = Some(id);
            ui.close();
        }
        ui.separator();
        ui.checkbox(&mut v.hidden_lines, tr("Linhas ocultas (tracejadas)"));
        ui.checkbox(&mut v.tangent_edges, tr("Arestas tangentes"));
        ui.checkbox(&mut v.label, tr("Mostrar rótulo e escala"));
        ui.menu_button(tr("Estilo de renderização"), |ui| {
            for r in Render::ALL {
                if ui.selectable_label(v.render == r, tr(r.label())).clicked() {
                    v.render = r;
                    ui.close();
                }
            }
        });
        color_width(ui, &mut v.color, &mut v.line_width);
        ui.separator();
        for (t, cmd) in [("Vista projetada", c::PROJ), ("Vista em corte", c::SECTION), ("Vista de detalhe", c::DETAIL)] {
            if ui.button(tr(t)).clicked() {
                tool = Some(cmd);
                ui.close();
            }
        }
        if ui.button(tr("Copiar estilo das anotações")).clicked() {
            copy = Some(v.ann.clone());
            ui.close();
        }
        if let Some(s) = &paste {
            if ui.button(tr("Colar estilo nas anotações")).clicked() {
                v.ann = s.clone();
                ui.close();
            }
        }
    } else if let Some(d) = st.d.dims.iter_mut().find(|d| d.id == id) {
        ui.menu_button(tr("Setas"), |ui| {
            for (k, n) in DIM_ARROWS.iter().enumerate() {
                if ui.selectable_label(d.arrows == k as u8, tr(n)).clicked() {
                    d.arrows = k as u8;
                    ui.close();
                }
            }
        });
        ui.menu_button(tr("Ponta"), |ui| {
            use scc_core::pmi::PmiArrow;
            for (a, n) in [(PmiArrow::Padrao, "padrão"), (PmiArrow::Cheia, "▶ cheia"), (PmiArrow::Aberta, "▷ aberta"), (PmiArrow::Ponto, "● ponto"), (PmiArrow::Barra, "⁄ barra"), (PmiArrow::Nenhuma, "nenhuma")] {
                if ui.selectable_label(d.style.arrow == a, tr(n)).clicked() {
                    d.style.arrow = a;
                    ui.close();
                }
            }
        });
        if d.text_pos.is_some() && ui.button(tr("Texto no lugar automático")).clicked() {
            d.text_pos = None;
            ui.close();
        }
        style_color_width(ui, &mut d.style);
        if ui.button(tr("Copiar estilo")).clicked() {
            copy = Some(d.style.clone());
            ui.close();
        }
        if let Some(s) = &paste {
            if ui.button(tr("Colar estilo")).clicked() {
                d.style = s.clone();
                ui.close();
            }
        }
    } else if let Some(n) = st.d.notes.iter_mut().find(|n| n.id == id) {
        let mut lead = n.leader.is_some();
        if ui.checkbox(&mut lead, tr("Chamada (seta)")).changed() {
            n.leader = lead.then(|| [n.pos[0] - 15., n.pos[1] - 10.]);
            if !lead {
                n.anchor3 = None;
            }
        }
        if n.anchor3.is_some() && ui.button(tr("Soltar da vista (ponta livre)")).clicked() {
            n.anchor3 = None;
            ui.close();
        }
        style_color_width(ui, &mut n.style);
        if ui.button(tr("Copiar estilo")).clicked() {
            copy = Some(n.style.clone());
            ui.close();
        }
        if let Some(s) = &paste {
            if ui.button(tr("Colar estilo")).clicked() {
                n.style = s.clone();
                ui.close();
            }
        }
    }
    ui.separator();
    if ui.button(tr("🗑 Apagar   Del")).clicked() {
        st.d.delete(id);
        if st.sel == Some(id) {
            st.sel = None;
        }
        st.focus = None;
        ui.close();
    }
    if let Some(s) = copy {
        st.style_clip = Some(s);
    }
    if let Some(t) = tool {
        st.sel = Some(id);
        st.tool = Some(t);
        st.clicks.clear();
        st.tool_view = None;
    }
}

/// Linha da árvore: 1 clique realça, 2 cliques editam, botão direito = opções.
fn tree_row(ui: &mut egui::Ui, st: &mut DwgState, sid: Id, id: Id, text: String, depth: usize) {
    ui.horizontal(|ui| {
        ui.add_space(depth as f32 * 14.);
        if st.renaming == Some(id) {
            if let Some(v) = st.d.views.iter_mut().find(|v| v.id == id) {
                let r = ui.text_edit_singleline(&mut v.name);
                r.request_focus();
                if r.lost_focus() {
                    st.renaming = None;
                }
                return;
            }
            st.renaming = None;
        }
        let on = st.sel == Some(id) || st.focus == Some(id);
        let txt = if st.sel == Some(id) { RichText::new(text).strong() } else { RichText::new(text) };
        let r = ui.selectable_label(on, txt);
        if r.double_clicked() {
            st.sel = Some(id);
            st.focus = Some(id);
            st.props_open = false;
            st.sheet = sid;
        } else if r.clicked() {
            st.focus = Some(id);
            if st.sel != Some(id) {
                st.sel = None;
            }
            st.sheet = sid;
        }
        r.context_menu(|ui| ctx_menu(ui, st, Some(id)));
    });
}

/// A que vista pertence cada nota da folha atual: a presa a ela ou a de caixa menor que contém a
/// nota (a caixa 2D da vista detecta o que está dentro dela).
fn note_owners(st: &DwgState, sheet: Id) -> std::collections::HashMap<Id, Id> {
    let mut out = std::collections::HashMap::new();
    let views: Vec<&(Id, V2, V2)> = st.boxes.iter().filter(|b| st.d.view(b.0).is_some_and(|v| v.sheet == sheet)).collect();
    for n in st.d.notes.iter().filter(|n| n.sheet == sheet) {
        if let Some(v) = n.view.filter(|v| st.d.view(*v).is_some()) {
            out.insert(n.id, v);
            continue;
        }
        if sheet != st.sheet {
            continue;
        }
        let Some(b) = st.boxes.iter().find(|b| b.0 == n.id) else { continue };
        let c = [(b.1[0] + b.2[0]) / 2., (b.1[1] + b.2[1]) / 2.];
        let inside = views
            .iter()
            .filter(|v| c[0] >= v.1[0] && c[0] <= v.2[0] && c[1] >= v.1[1] && c[1] <= v.2[1])
            .min_by(|a, b| ((a.2[0] - a.1[0]) * (a.2[1] - a.1[1])).total_cmp(&((b.2[0] - b.1[0]) * (b.2[1] - b.1[1]))));
        if let Some(v) = inside {
            out.insert(n.id, v.0);
        }
    }
    out
}

/// Texto da nota na árvore (GD&T: símbolo e tolerância).
fn note_title(n: &Note) -> String {
    match (&n.kind, &n.gdt) {
        (NoteKind::Gdt, Some(g)) => format!("{} {} {}", g.ch.symbol(), g.tol, g.datums),
        _ => n.text.lines().next().unwrap_or("").to_string(),
    }
}

fn note_icon(k: &NoteKind) -> &'static str {
    match k {
        NoteKind::Text => "✎",
        NoteKind::Balloon => "①",
        NoteKind::Roughness => "√",
        NoteKind::Weld => "⌒",
        NoteKind::CenterMark => "✛",
        NoteKind::Gdt => "⌖",
        NoteKind::Datum => "Ⓐ",
    }
}

pub fn tree(app: &mut App, ui: &mut egui::Ui) {
    ensure(app);
    let st = app.dwg.as_mut().unwrap();
    ui.horizontal(|ui| {
        // clicar no nome abre as propriedades do desenho (folha, origem, legenda)
        if ui.selectable_label(st.props_open, RichText::new(format!("📐 {}{}", st.d.name, if st.modified { " •" } else { "" })).strong()).on_hover_text(tr("propriedades do desenho: folha, origem das vistas e legenda")).clicked() {
            st.props_open = true;
            st.sel = None;
        }
    });
    if let Some(p) = &st.path {
        ui.label(RichText::new(p.display().to_string()).weak().small());
    }
    ui.add_space(4.);
    let sheets: Vec<(Id, String, &'static str)> = st.d.sheets.iter().map(|s| (s.id, s.name.clone(), s.format.label())).collect();
    let mut kill = None;
    for (sid, name, fmt) in sheets {
        let open = sid == st.sheet;
        let r = egui::CollapsingHeader::new(RichText::new(format!("🗋 {name} ({fmt})")).strong()).id_salt(("folha", sid.0)).default_open(open).show(ui, |ui| {
            // vistas em árvore (filhas abaixo da mãe), cada uma com as cotas e anotações dela
            fn kids(d: &Drawing, sheet: Id, parent: Option<Id>) -> Vec<Id> {
                d.views
                    .iter()
                    .filter(|v| v.sheet == sheet)
                    .filter(|v| {
                        let p = match &v.kind {
                            ViewKind::Projected { parent, .. } | ViewKind::Section { parent, .. } | ViewKind::Detail { parent, .. } => Some(*parent),
                            _ => None,
                        };
                        p == parent || (parent.is_none() && p.is_some_and(|p| d.view(p).is_none_or(|pv| pv.sheet != sheet)))
                    })
                    .map(|v| v.id)
                    .collect()
            }
            let owners = note_owners(st, sid);
            let mut stack: Vec<(Id, usize)> = kids(&st.d, sid, None).into_iter().rev().map(|i| (i, 0)).collect();
            while let Some((id, depth)) = stack.pop() {
                let Some(v) = st.d.view(id) else { continue };
                let icon = match v.kind {
                    ViewKind::Section { .. } => "✂",
                    ViewKind::Detail { .. } => "🔍",
                    ViewKind::Flat => "▭",
                    ViewKind::Schematic(_) => "⚙",
                    ViewKind::Camera { .. } => "🎥",
                    _ => "▣",
                };
                let text = format!("{icon} {}  ({})", v.name, scale_text(v.scale));
                tree_row(ui, st, sid, id, text, depth);
                let mut items: Vec<(Id, String)> = st.d.dims.iter().filter(|d| d.view == id).map(|d| (d.id, format!("↔ {}{}", tr(d.kind.label()), if d.text.is_empty() { String::new() } else { format!(" — {}", d.text) }))).collect();
                items.extend(st.d.notes.iter().filter(|n| owners.get(&n.id) == Some(&id)).map(|n| (n.id, format!("{} {}", note_icon(&n.kind), note_title(n)))));
                for (cid, t) in items {
                    tree_row(ui, st, sid, cid, t, depth + 1);
                }
                for k in kids(&st.d, sid, Some(id)).into_iter().rev() {
                    stack.push((k, depth + 1));
                }
            }
            // anotações soltas na folha e tabelas
            let notes: Vec<(Id, String)> = st.d.notes.iter().filter(|n| n.sheet == sid && !owners.contains_key(&n.id)).map(|n| (n.id, format!("{} {}", note_icon(&n.kind), note_title(n)))).collect();
            for (id, t) in notes {
                tree_row(ui, st, sid, id, t, 0);
            }
            let tables: Vec<(Id, String)> = st.d.tables.iter().filter(|t| t.sheet == sid).map(|t| (t.id, format!("▦ {}", tr(t.kind.label())))).collect();
            for (id, t) in tables {
                tree_row(ui, st, sid, id, t, 0);
            }
        });
        if r.header_response.clicked() {
            st.sheet = sid;
            st.origin = None;
            st.invalidate();
            st.props_open = true;
            st.sel = None;
        }
        r.header_response.context_menu(|ui| {
            if st.d.sheets.len() > 1 && ui.button(tr("Apagar folha")).clicked() {
                kill = Some(sid);
                ui.close();
            }
        });
    }
    if let Some(s) = kill {
        st.d.delete(s);
    }
}

pub fn panel(app: &mut App, ui: &mut egui::Ui) {
    ensure(app);
    let cur = current_source(app);
    let cam_now = app.viewport.as_ref().map(|v| {
        let c = v.camera;
        let t = (c.eye() - c.target).normalize_or_zero();
        let (r, _) = c.basis();
        ([t.x as f64, t.y as f64, t.z as f64], [r.x as f64, r.y as f64, r.z as f64])
    });
    let st = app.dwg.as_mut().unwrap();
    let mut changed_src = false;
    if let Some(id) = st.sel {
        if let Some(v) = st.d.views.iter_mut().find(|v| v.id == id) {
            ui.heading(tr("Vista"));
            ui.horizontal(|ui| {
                ui.label(tr("Nome"));
                ui.text_edit_singleline(&mut v.name);
            });
            scale_combo(ui, &mut v.scale, ("escala_vista", id.0));
            ui.checkbox(&mut v.hidden_lines, tr("Linhas ocultas (tracejadas)"));
            ui.checkbox(&mut v.tangent_edges, tr("Arestas tangentes"));
            ui.checkbox(&mut v.label, tr("Mostrar rótulo e escala"));
            ui.horizontal(|ui| {
                ui.label(tr("Renderização"));
                egui::ComboBox::from_id_salt(("render_vista", id.0)).selected_text(tr(v.render.label())).show_ui(ui, |ui| {
                    for r in Render::ALL {
                        ui.selectable_value(&mut v.render, r, tr(r.label()));
                    }
                });
            });
            ui.label(RichText::new(tr("Arestas visíveis")).weak().small());
            color_width(ui, &mut v.color, &mut v.line_width);
            match &mut v.kind {
                ViewKind::Camera { toward, x } => {
                    ui.label(RichText::new(format!("{} ({:.2}, {:.2}, {:.2})", tr("Direção"), toward[0], toward[1], toward[2])).weak().small());
                    if ui.add_enabled(cam_now.is_some(), egui::Button::new(tr("🎥 Usar a câmera atual do 3D"))).clicked() {
                        if let Some((t, xx)) = cam_now {
                            *toward = t;
                            *x = xx;
                        }
                    }
                }
                ViewKind::Standard(d) | ViewKind::Schematic(d) => {
                    egui::ComboBox::from_label(tr("Direção")).selected_text(tr(d.label())).show_ui(ui, |ui| {
                        for x in Dir::ALL {
                            ui.selectable_value(d, x, tr(x.label()));
                        }
                    });
                }
                ViewKind::Section { label, .. } => {
                    ui.horizontal(|ui| {
                        ui.label(tr("Letra do corte"));
                        ui.add(egui::TextEdit::singleline(label).desired_width(40.));
                    });
                    ui.label(RichText::new(tr("Para inverter o lado visto, apague e trace a linha no sentido oposto.")).weak().small());
                }
                ViewKind::Detail { label, radius, .. } => {
                    ui.horizontal(|ui| {
                        ui.label(tr("Letra do detalhe"));
                        ui.add(egui::TextEdit::singleline(label).desired_width(40.));
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("Raio (mm do modelo)"));
                        ui.add(egui::DragValue::new(radius).range(0.5..=1e4).speed(0.2));
                    });
                }
                _ => {}
            }
            ui.separator();
            ui.label(RichText::new(tr("Origem")).strong());
            ui.label(RichText::new(&v.source.path).small());
            if v.source.path.to_lowercase().ends_with(".sccasm") {
                ui.horizontal(|ui| {
                    ui.label(tr("Estado de exibição"));
                    changed_src |= ui.text_edit_singleline(&mut v.source.display_state).changed();
                });
                ui.label(RichText::new(tr("Nome de um estado salvo no compositor (vazio = como está; ex.: vista explodida).")).weak().small());
                // componentes visíveis só nesta vista
                let src = v.source.clone();
                let comps: Vec<(u64, String)> = match &st.models.load(&src).model {
                    scc_desenho::source::Model::Asm(a) => a.nodes.iter().filter(|n| !matches!(n.kind, scc_montagem::NodeKind::Group)).map(|n| (n.id.0, n.name.clone())).collect(),
                    _ => vec![],
                };
                egui::CollapsingHeader::new(format!("{} ({}/{})", tr("Componentes visíveis nesta vista"), comps.len() - v.source.hidden.len().min(comps.len()), comps.len())).id_salt(("comps_vista", id.0)).show(ui, |ui| {
                    for (nid, name) in comps {
                        let mut on = !v.source.hidden.contains(&nid);
                        if ui.checkbox(&mut on, name).changed() {
                            if on {
                                v.source.hidden.retain(|x| *x != nid);
                            } else {
                                v.source.hidden.push(nid);
                            }
                            changed_src = true;
                        }
                    }
                    if !v.source.hidden.is_empty() && ui.small_button(tr("Mostrar todos")).clicked() {
                        v.source.hidden.clear();
                        changed_src = true;
                    }
                });
            }
            egui::CollapsingHeader::new(tr("Estilo das anotações desta vista")).id_salt(("ann_vista", id.0)).show(ui, |ui| {
                ui.label(RichText::new(tr("Cotas e notas da vista herdam daqui o que estiver em \"padrão\".")).weak().small());
                crate::pmi_draw::style_ui(ui, &mut v.ann, false);
            });
            let p = st.models.proj(&st.d, id);
            if let Some(e) = &p.error {
                ui.colored_label(Color32::from_rgb(230, 90, 70), e);
            }
            ui.label(RichText::new(format!("{} {}", p.lines.len(), tr("arestas projetadas"))).weak().small());
            ui.add_space(6.);
            if ui.button(tr("🗑 Apagar vista (com as filhas e cotas)")).clicked() {
                st.d.delete(id);
                st.sel = None;
            }
            return;
        }
        if let Some(dm) = st.d.dims.iter_mut().find(|d| d.id == id) {
            ui.heading(format!("{} — {}", tr("Cota"), tr(dm.kind.label())));
            ui.horizontal(|ui| {
                ui.label(tr("Texto"));
                ui.text_edit_singleline(&mut dm.text);
            });
            ui.label(RichText::new(tr("Vazio = valor medido. Use <> para incluir o valor (ex.: 4x <> passante).")).weak().small());
            ui.horizontal(|ui| {
                ui.label(tr("Tolerância"));
                ui.text_edit_singleline(&mut dm.tol);
            });
            ui.horizontal(|ui| {
                for t in ["±0,1", "+0,1/0", "H7", "h6", "g6"] {
                    if ui.small_button(t).clicked() {
                        dm.tol = t.into();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("Afastamento (mm)"));
                { let __r = ui.add(egui::DragValue::new(&mut dm.offset).speed(0.2)); crate::ui_ext::wheel(ui, &__r, &mut dm.offset); __r };
            });
            ui.horizontal(|ui| {
                ui.label(tr("Setas"));
                egui::ComboBox::from_id_salt(("setas_cota", id.0)).selected_text(tr(DIM_ARROWS[dm.arrows.min(5) as usize])).show_ui(ui, |ui| {
                    for (k, n) in DIM_ARROWS.iter().enumerate() {
                        ui.selectable_value(&mut dm.arrows, k as u8, tr(n));
                    }
                });
            });
            if dm.text_pos.is_some() && ui.button(tr("Texto no lugar automático")).clicked() {
                dm.text_pos = None;
            }
            ui.label(RichText::new(tr("Arraste a cota para afastá-la; arraste a alça do texto para soltá-lo da linha de cota.")).weak().small());
            egui::CollapsingHeader::new(tr("Estilo")).id_salt(("estilo_cota", id.0)).show(ui, |ui| {
                style_color_width(ui, &mut dm.style);
                crate::pmi_draw::style_ui(ui, &mut dm.style, true);
            });
            if ui.button(tr("🗑 Apagar cota")).clicked() {
                st.d.delete(id);
                st.sel = None;
            }
            return;
        }
        if let Some(n) = st.d.notes.iter_mut().find(|n| n.id == id) {
            ui.heading(tr("Nota / símbolo"));
            egui::ComboBox::from_label(tr("Tipo"))
                .selected_text(note_label(&n.kind))
                .show_ui(ui, |ui| {
                    for k in [NoteKind::Text, NoteKind::Balloon, NoteKind::Roughness, NoteKind::Weld, NoteKind::CenterMark, NoteKind::Gdt, NoteKind::Datum] {
                        let l = note_label(&k);
                        ui.selectable_value(&mut n.kind, k, l);
                    }
                });
            ui.label(tr(match n.kind {
                NoteKind::Text => "Texto (várias linhas)",
                NoteKind::Balloon => "Número do item",
                NoteKind::Roughness => "Rugosidade (ex.: Ra 1,6)",
                NoteKind::Weld => "Medida da solda (ex.: a4)",
                NoteKind::CenterMark => "Raio da marca (mm)",
                NoteKind::Gdt => "Quadro de controle geométrico",
                NoteKind::Datum => "Letra da referência",
            }));
            if n.kind == NoteKind::Gdt {
                let g = n.gdt.get_or_insert_with(Default::default);
                crate::pmi_ui::gdt_ui(ui, "dwg_gdt", &mut g.ch, &mut g.tol, &mut g.datums, &mut g.extra);
            } else {
                ui.text_edit_multiline(&mut n.text);
            }
            if n.kind == NoteKind::Weld {
                let w = n.weld.get_or_insert(scc_desenho::DwgWeld { kind: scc_core::pmi::WeldKind::Filete, all_around: false, extra: Default::default() });
                let mut size = n.text.clone();
                crate::pmi_ui::weld_ui(ui, "dwg_solda", &mut w.kind, &mut size, &mut w.all_around, &mut w.extra);
                n.text = size;
            }
            ui.horizontal(|ui| {
                ui.label(tr("Altura do texto"));
                { let __r = ui.add(egui::DragValue::new(&mut n.size).range(1.0..=20.).speed(0.1)); crate::ui_ext::wheel(ui, &__r, &mut n.size); __r };
            });
            let mut lead = n.leader.is_some();
            if ui.checkbox(&mut lead, tr("Chamada (seta)")).changed() {
                n.leader = lead.then(|| [n.pos[0] - 15., n.pos[1] - 10.]);
            }
            if let Some(l) = n.leader.as_mut() {
                ui.horizontal(|ui| {
                    ui.label(tr("Ponta (x, y)"));
                    { let __r = ui.add(egui::DragValue::new(&mut l[0]).speed(0.5)); crate::ui_ext::wheel(ui, &__r, &mut l[0]); __r };
                    { let __r = ui.add(egui::DragValue::new(&mut l[1]).speed(0.5)); crate::ui_ext::wheel(ui, &__r, &mut l[1]); __r };
                });
            }
            if let Some(v) = n.view {
                let vname = st.d.views.iter().find(|x| x.id == v).map(|x| x.name.clone()).unwrap_or_default();
                ui.label(RichText::new(format!("{} {vname}{}", tr("Pertence à vista"), if n.anchor3.is_some() { tr(" — ponta presa ao modelo (acompanha a peça)") } else { "" })).weak().small());
            }
            egui::CollapsingHeader::new(tr("Estilo")).id_salt(("estilo_nota", id.0)).default_open(true).show(ui, |ui| {
                style_color_width(ui, &mut n.style);
                crate::pmi_draw::style_ui(ui, &mut n.style, true);
            });
            if ui.button(tr("🗑 Apagar")).clicked() {
                st.d.delete(id);
                st.sel = None;
            }
            return;
        }
        if let Some(t) = st.d.tables.iter_mut().find(|t| t.id == id) {
            ui.heading(tr(t.kind.label()));
            ui.label(RichText::new(&t.source.path).small());
            if matches!(t.kind, TableKind::Bom) && t.source.path.to_lowercase().ends_with(".sccasm") {
                ui.horizontal(|ui| {
                    ui.label(tr("Nível"));
                    egui::ComboBox::from_id_salt(("bom_nivel", id.0)).selected_text(tr(t.level.label())).show_ui(ui, |ui| {
                        for l in scc_montagem::bom::BomLevel::ALL {
                            ui.selectable_value(&mut t.level, l, tr(l.label()));
                        }
                    });
                });
                ui.label(RichText::new(tr("Os números de item vêm da montagem e ficam fixos entre revisões (salve a montagem para gravar os novos).")).weak().small());
            }
            let src = st.source.clone();
            if let Some(s) = src.filter(|s| *s != t.source) {
                if ui.button(tr("Usar a origem atual do desenho")).clicked() {
                    t.source = s;
                }
            }
            let mut csv = None;
            if ui.button(tr("Exportar CSV…")).clicked() {
                let t = t.clone();
                let data = st.models.table(&t, &st.d);
                csv = Some(data);
            }
            if let Some(data) = csv {
                if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).save_file() {
                    let mut s = data.headers.join(";") + "\n";
                    for r in &data.rows {
                        s += &(r.join(";") + "\n");
                    }
                    let _ = std::fs::write(p, s);
                }
            }
            if ui.button(tr("🗑 Apagar tabela")).clicked() {
                st.d.delete(id);
                st.sel = None;
            }
            return;
        }
    }
    // ---------- sem seleção: desenho, folha, origem e legenda
    ui.heading(tr("Desenho"));
    ui.horizontal(|ui| {
        ui.label(tr("Nome"));
        ui.text_edit_singleline(&mut st.d.name);
    });
    if let Some(sh) = st.d.sheets.iter_mut().find(|s| s.id == st.sheet) {
        ui.horizontal(|ui| {
            ui.label(tr("Folha"));
            ui.text_edit_singleline(&mut sh.name);
        });
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("formato").selected_text(sh.format.label()).show_ui(ui, |ui| {
                for f in Format::ALL {
                    ui.selectable_value(&mut sh.format, f, f.label());
                }
            });
            ui.checkbox(&mut sh.portrait, tr("Retrato"));
            ui.checkbox(&mut sh.zones, tr("Zoneamento"));
        });
    }
    // modelos prontos: formato + orientação + legenda
    let mut tpl = None;
    egui::ComboBox::from_id_salt("modelo_folha").selected_text(tr("📄 Modelo de folha…")).show_ui(ui, |ui| {
        for (i, (n, ..)) in TEMPLATES.iter().enumerate() {
            if ui.selectable_label(false, tr(n)).clicked() {
                tpl = Some(i);
            }
        }
    });
    if let Some(i) = tpl {
        let (_, f, portrait, style) = TEMPLATES[i];
        if let Some(sh) = st.d.sheets.iter_mut().find(|s| s.id == st.sheet) {
            sh.format = f;
            sh.portrait = portrait;
        }
        st.d.title.style = style;
        st.origin = None;
    }
    ui.horizontal(|ui| {
        ui.label(tr("Texto das cotas (mm)"));
        { let __r = ui.add(egui::DragValue::new(&mut st.d.text_height).range(1.5..=10.).speed(0.1)); crate::ui_ext::wheel(ui, &__r, &mut st.d.text_height); __r };
    });
    ui.horizontal(|ui| {
        ui.label(tr("Casas decimais"));
        { let __r = ui.add(egui::DragValue::new(&mut st.d.decimals).range(0..=4)); crate::ui_ext::wheel(ui, &__r, &mut st.d.decimals); __r };
    });
    ui.separator();
    ui.label(RichText::new(tr("Origem das novas vistas e tabelas")).strong());
    match &st.source {
        Some(s) => ui.label(RichText::new(&s.path).small()),
        None => ui.colored_label(Color32::from_rgb(230, 150, 60), tr("nenhuma — salve a peça/montagem e use o botão abaixo")),
    };
    ui.horizontal(|ui| {
        if ui.add_enabled(cur.is_some(), egui::Button::new(tr("Peça/montagem aberta"))).on_hover_text(tr("usa o arquivo salvo da peça (ou da montagem) aberta")).clicked() {
            st.source = cur.clone().map(|s| Source { path: st.rel(std::path::Path::new(&s.path)), ..s });
        }
        if ui.button(tr("Escolher arquivo…")).clicked() {
            if let Some(p) = rfd::FileDialog::new().add_filter(tr("Peça ou montagem"), &["sccpart", "sccasm"]).pick_file() {
                st.source = Some(Source { path: st.rel(&p), display_state: String::new(), hidden: vec![] });
            }
        }
    });
    ui.horizontal(|ui| {
        egui::ComboBox::from_label(tr("Vista única")).selected_text(tr(st.new_dir.label())).show_ui(ui, |ui| {
            for x in Dir::ALL {
                ui.selectable_value(&mut st.new_dir, x, tr(x.label()));
            }
        });
    });
    scale_combo(ui, &mut st.new_scale, ("escala_nova", 0));
    ui.separator();
    ui.label(RichText::new(tr("Legenda")).strong());
    let t = &mut st.d.title;
    egui::ComboBox::from_label(tr("Modelo da legenda")).selected_text(t.style.label()).show_ui(ui, |ui| {
        for s in TitleStyle::ALL {
            ui.selectable_value(&mut t.style, s, s.label());
        }
    });
    egui::Grid::new("legenda").num_columns(2).show(ui, |ui| {
        for (l, v) in [
            ("Empresa", &mut t.company),
            ("Título", &mut t.title),
            ("Código", &mut t.pn),
            ("Material", &mut t.material),
            ("Desenhado por", &mut t.drawn_by),
            ("Verificado", &mut t.checked_by),
            ("Aprovado", &mut t.approved_by),
            ("Data", &mut t.date),
            ("Revisão", &mut t.revision),
            ("Observações", &mut t.notes),
        ] {
            ui.label(tr(l));
            ui.text_edit_singleline(v);
            ui.end_row();
        }
        ui.label(tr("Massa específica"));
        { let __r = ui.add(egui::DragValue::new(&mut t.density).range(0.1..=25.).speed(0.01).suffix(" g/cm³")); crate::ui_ext::wheel(ui, &__r, &mut t.density); __r };
        ui.end_row();
    });
    egui::CollapsingHeader::new(tr("Campos automáticos")).default_open(false).show(ui, |ui| {
        ui.label(RichText::new(tr("Use nos campos da legenda e nas notas; o valor vem da peça/montagem da primeira vista da folha e se atualiza sozinho.")).weak().small());
        for (k, d) in FIELDS {
            ui.horizontal(|ui| {
                ui.monospace(k);
                ui.label(RichText::new(tr(d)).weak().small());
            });
        }
    });
    if ui.button(tr("Usar campos automáticos")).on_hover_text(tr("título, código, material, autor e data passam a vir da origem")).clicked() {
        let keep = (t.company.clone(), t.revision.clone(), t.notes.clone(), t.style, t.density);
        *t = TitleBlock::default();
        (t.company, t.revision, t.notes, t.style, t.density) = keep;
    }
    ui.separator();
    ui.label(RichText::new(tr("Vistas associativas: quando a peça ou a montagem é salva, o desenho se atualiza sozinho (F5 força). As cotas ficam presas às posições projetadas: se a geometria mudar muito, confira as cotas.")).weak().small());
    if changed_src {
        st.invalidate();
    }
}

fn note_label(k: &NoteKind) -> String {
    tr(match k {
        NoteKind::Text => "Nota",
        NoteKind::Balloon => "Balão",
        NoteKind::Roughness => "Rugosidade",
        NoteKind::Weld => "Solda",
        NoteKind::CenterMark => "Marca de centro",
        NoteKind::Gdt => "Tolerância geométrica",
        NoteKind::Datum => "Referência (datum)",
    })
    .to_string()
}

fn scale_combo(ui: &mut egui::Ui, s: &mut f64, salt: (&str, u64)) {
    ui.horizontal(|ui| {
        ui.label(tr("Escala"));
        egui::ComboBox::from_id_salt(salt).selected_text(scale_text(*s)).show_ui(ui, |ui| {
            for v in [10., 5., 2., 1., 0.5, 0.2, 0.1, 0.05, 0.02, 0.01] {
                ui.selectable_value(s, v, scale_text(v));
            }
        });
        ui.add(egui::DragValue::new(s).range(0.001..=100.).speed(0.01));
    });
}

#[allow(dead_code)]
fn today() -> String {
    // dias desde 1970 → data civil (sem dependência de calendário)
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let z = secs / 86400 + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{d:02}/{m:02}/{y}")
}
