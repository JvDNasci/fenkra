//! Aplicativo: estado, layout e comandos.
//!
//! Layout (RF-UI-001): menu mínimo + abas de módulo + grupos de comandos (topo, lateral ou
//! flutuantes), árvore/projeto à esquerda com a sequência de comandos, painel do objeto selecionado
//! à direita, viewport no centro.

use crate::editor;
use crate::render::Viewport;
use crate::scene;
use crate::settings::{Dock, Settings, Theme, BACKGROUNDS};
use crate::toolbar::{self, Act};
use eframe::egui::{self, Color32, RichText};
use glam::Vec3;
use scc_core::*;
use scc_project::{DocState, Project};
use scc_solidos::{RecomputeResult, Status};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

/// Seleção na árvore / linha de tempo.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Sel {
    #[default]
    None,
    Part,
    Origin(OriginPlane),
    Ref(Id),
    Params,
    Collector(Id),
    Folder(Id),
    Feature(Id),
}

/// Documento aberto.
pub struct OpenDoc {
    pub doc: Document,
    /// id no projeto (se pertence a um)
    pub entry: Option<u64>,
    /// revisão congelada aberta somente para leitura
    pub frozen: Option<String>,
    pub file: Option<PathBuf>,
    pub modified: bool,
}

pub struct DeleteDialog {
    pub target: Id,
    pub dependents: Vec<Id>,
    pub replacement: Option<Id>,
}

#[derive(PartialEq)]
enum LeftTab {
    Peca,
    Projeto,
}

struct Worker {
    tx: Sender<(u64, Document)>,
    rx: Receiver<(u64, RecomputeResult)>,
}

impl Worker {
    /// Recompute fora da thread da interface (RNF-001); só o pedido mais recente é processado.
    fn spawn(ctx: egui::Context) -> Self {
        let (tx, jobs) = channel::<(u64, Document)>();
        let (out, rx) = channel();
        std::thread::Builder::new()
            .name("recompute".into())
            .spawn(move || {
                while let Ok(mut job) = jobs.recv() {
                    while let Ok(newer) = jobs.try_recv() {
                        job = newer;
                    }
                    let r = scc_solidos::recompute(&job.1);
                    if out.send((job.0, r)).is_err() {
                        break;
                    }
                    ctx.request_repaint();
                }
            })
            .expect("thread de recompute");
        Worker { tx, rx }
    }
}

pub struct App {
    pub open: OpenDoc,
    pub project: Option<Project>,
    pub project_sel: Option<u64>,
    pub sel: Sel,
    pub result: Option<RecomputeResult>,
    pub delete_dialog: Option<DeleteDialog>,
    release_dialog: Option<(u64, String)>,
    worker: Worker,
    gen_sent: u64,
    gen_done: u64,
    viewport: Option<Viewport>,
    scene_dirty: bool,
    fit_pending: bool,
    undo: Vec<Document>,
    redo: Vec<Document>,
    last_edit: f64,
    toasts: Vec<(String, f64, bool)>,
    left: LeftTab,
    pub settings: Settings,
    settings_open: bool,
    /// ambiente de ocultos: mostra só o que está oculto (para reexibir)
    pub hidden_env: bool,
    /// sketch cuja vista deve ser alinhada quando o resultado chegar
    align_pending: Option<Id>,
}

pub const MATERIALS: [(&str, f64); 8] = [
    ("Aço 1020", 7.87),
    ("Aço 1045", 7.85),
    ("Aço inox 304", 8.0),
    ("Alumínio 6061", 2.7),
    ("Latão", 8.5),
    ("Nylon", 1.14),
    ("ABS", 1.05),
    ("PLA", 1.24),
];

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let settings = Settings::load();
        settings.apply(&cc.egui_ctx);
        let mut viewport = cc.wgpu_render_state.as_ref().map(Viewport::new);
        if let Some(v) = viewport.as_mut() {
            v.camera.ortho = settings.ortho;
        }
        let mut app = App {
            open: OpenDoc { doc: scc_solidos::example_document(), entry: None, frozen: None, file: None, modified: false },
            project: None,
            project_sel: None,
            sel: Sel::None,
            result: None,
            delete_dialog: None,
            release_dialog: None,
            worker: Worker::spawn(cc.egui_ctx.clone()),
            gen_sent: 0,
            gen_done: 0,
            viewport,
            scene_dirty: true,
            fit_pending: true,
            undo: vec![],
            redo: vec![],
            last_edit: -10.,
            toasts: vec![],
            left: LeftTab::Peca,
            settings,
            settings_open: false,
            hidden_env: false,
            align_pending: None,
        };
        app.request_recompute();
        app
    }

    pub fn doc(&self) -> &Document {
        &self.open.doc
    }

    pub fn request_recompute(&mut self) {
        self.gen_sent += 1;
        let _ = self.worker.tx.send((self.gen_sent, self.open.doc.clone()));
    }

    pub fn toast(&mut self, msg: impl Into<String>, error: bool) {
        self.toasts.push((msg.into(), 0., error));
    }

    pub fn status_of(&self, id: Id) -> Option<&Status> {
        self.result.as_ref().and_then(|r| r.status.get(&id))
    }

    pub fn vars(&self) -> std::collections::HashMap<String, f64> {
        self.result.as_ref().map(|r| r.params.clone()).unwrap_or_default()
    }

    fn load_doc(&mut self, doc: Document, entry: Option<u64>, frozen: Option<String>) {
        self.open = OpenDoc { doc, entry, frozen, file: None, modified: false };
        self.sel = Sel::None;
        self.undo.clear();
        self.redo.clear();
        self.fit_pending = true;
        self.request_recompute();
    }

    // ------------------------------------------------------------ arquivo e projeto

    fn save(&mut self) {
        if self.open.frozen.is_some() {
            self.toast("Revisão congelada não pode ser salva. Abra a versão de trabalho.", true);
            return;
        }
        if let (Some(p), Some(id)) = (self.project.as_mut(), self.open.entry) {
            match p.save(id, &self.open.doc, "salvo") {
                Ok(()) => {
                    self.open.modified = false;
                    let n = p.entry(id).map(|e| e.versions.len()).unwrap_or(0);
                    self.toast(format!("Versão {n} salva no projeto"), false);
                }
                Err(e) => self.toast(e.to_string(), true),
            }
            return;
        }
        let path = match &self.open.file {
            Some(f) => Some(f.clone()),
            None => rfd::FileDialog::new()
                .add_filter("Peça SolidCadCraft", &["sccpart"])
                .set_file_name(format!("{}.sccpart", self.open.doc.name))
                .save_file(),
        };
        if let Some(path) = path {
            match std::fs::write(&path, self.open.doc.to_json()) {
                Ok(()) => {
                    self.open.file = Some(path);
                    self.open.modified = false;
                    self.toast("Peça salva", false);
                }
                Err(e) => self.toast(format!("Erro ao salvar: {e}"), true),
            }
        }
    }

    fn open_file(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Peça SolidCadCraft", &["sccpart"]).pick_file() else { return };
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|s| Document::from_json(&s).map_err(|e| e.to_string())) {
            Ok(d) => {
                self.load_doc(d, None, None);
                self.open.file = Some(path);
            }
            Err(e) => self.toast(format!("Não foi possível abrir: {e}"), true),
        }
    }

    fn export(&mut self, ext: &str) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(ext.to_uppercase(), &[ext])
            .set_file_name(format!("{}.{ext}", self.open.doc.name))
            .save_file()
        else {
            return;
        };
        match scc_solidos::export(&self.open.doc, None, &path.to_string_lossy()) {
            Ok(()) => self.toast(format!("Exportado: {}", path.display()), false),
            Err(e) => self.toast(format!("Falha ao exportar: {e}"), true),
        }
    }

    fn new_project(&mut self) {
        let Some(dir) = rfd::FileDialog::new().set_title("Pasta do novo projeto").pick_folder() else { return };
        let name = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or("Projeto".into());
        match Project::create(&dir, &name) {
            Ok(p) => {
                self.project = Some(p);
                self.left = LeftTab::Projeto;
                self.toast(format!("Projeto '{name}' criado"), false);
            }
            Err(e) => self.toast(e.to_string(), true),
        }
    }

    fn open_project(&mut self) {
        let Some(dir) = rfd::FileDialog::new().set_title("Pasta do projeto").pick_folder() else { return };
        match Project::open(&dir) {
            Ok(p) => {
                self.project = Some(p);
                self.left = LeftTab::Projeto;
            }
            Err(e) => self.toast(format!("Não é um projeto SolidCadCraft: {e}"), true),
        }
    }

    fn open_entry(&mut self, id: u64, version: Option<(String, String)>) {
        if self.open.modified && self.open.entry.is_some() && self.open.frozen.is_none() {
            self.save(); // salvamento automático ao trocar de documento (RNF-007)
        }
        let Some(p) = &self.project else { return };
        let r = match &version {
            Some((hash, _)) => p.load_version(hash),
            None => p.load(id),
        };
        match r {
            Ok(d) => self.load_doc(d, Some(id), version.map(|v| v.1)),
            Err(e) => self.toast(e.to_string(), true),
        }
    }

    // ------------------------------------------------------------ edição

    fn undo(&mut self) {
        if let Some(d) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.open.doc, d));
            self.open.modified = true;
            self.request_recompute();
        }
    }

    fn redo(&mut self) {
        if let Some(d) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.open.doc, d));
            self.open.modified = true;
            self.request_recompute();
        }
    }

    /// Plano a usar para um novo sketch/plano: o selecionado, ou XY.
    fn selected_plane(&self) -> PlaneRef {
        match self.sel {
            Sel::Origin(o) => PlaneRef::Origin(o),
            Sel::Ref(id) => PlaneRef::Datum(id),
            _ => PlaneRef::Origin(OriginPlane::XY),
        }
    }

    /// Sketch para um novo comando: o selecionado, ou o último antes do marcador.
    fn selected_sketch(&self, need_points: bool) -> Option<Id> {
        let d = self.doc();
        let ok = |f: &Feature| match &f.kind {
            FeatureKind::Sketch(s) => !need_points || s.entities.iter().any(|e| matches!(e.geom, SketchGeom::Point { .. })),
            _ => false,
        };
        if let Sel::Feature(id) = self.sel {
            if let Some(f) = d.feature(id) {
                if ok(f) {
                    return Some(id);
                }
            }
        }
        d.features[..d.rollback.min(d.features.len())].iter().rev().find(|f| ok(f)).map(|f| f.id)
    }

    fn command(&mut self, c: Cmd) {
        let s = |v: &str| v.to_string();
        let plane = self.selected_plane();
        let sketch = self.selected_sketch(false);
        let d = &mut self.open.doc;
        let new_sel = match c {
            Cmd::Sketch => {
                let e = SketchEntity { id: d.alloc(), construction: false, geom: SketchGeom::Rect { cx: s("0"), cy: s("0"), w: s("50"), h: s("30") } };
                Sel::Feature(d.add_feature(FeatureKind::Sketch(Sketch { plane, entities: vec![e] })))
            }
            Cmd::Extrude | Cmd::Cut => {
                let Some(sk) = sketch else {
                    self.toast("Crie ou selecione um sketch primeiro", true);
                    return;
                };
                let cut = c == Cmd::Cut;
                Sel::Feature(d.add_feature(FeatureKind::Extrude(Extrude {
                    sketch: sk,
                    op: if cut { BoolOp::Cut } else { BoolOp::Add },
                    extent: if cut { Extent::ThroughAll } else { Extent::Blind(s("20")) },
                    reverse: false,
                })))
            }
            Cmd::Revolve => {
                let Some(sk) = sketch else {
                    self.toast("Crie ou selecione um sketch primeiro", true);
                    return;
                };
                Sel::Feature(d.add_feature(FeatureKind::Revolve(Revolve {
                    sketch: sk,
                    axis: AxisRef::Origin(OriginAxis::Y),
                    angle: s("360"),
                    symmetric: false,
                    op: BoolOp::Add,
                })))
            }
            Cmd::Hole => {
                let sk = match self.selected_sketch(true) {
                    Some(sk) => sk,
                    None => {
                        let d = &mut self.open.doc;
                        let e = SketchEntity { id: d.alloc(), construction: false, geom: SketchGeom::Point { x: s("0"), y: s("0") } };
                        let id = d.add_feature(FeatureKind::Sketch(Sketch { plane, entities: vec![e] }));
                        d.feature_mut(id).unwrap().name = d.unique_name("Centros");
                        id
                    }
                };
                let d = &mut self.open.doc;
                let m = scc_solidos::hardware::metric("M6").unwrap();
                Sel::Feature(d.add_feature(FeatureKind::Hole(Hole {
                    sketch: sk,
                    kind: HoleKind::Simples,
                    diameter: m.clearance[1].to_string(),
                    depth: Extent::ThroughAll,
                    thread: None,
                    standard: Some(s("M6 passagem normal")),
                })))
            }
            Cmd::Fillet => Sel::Feature(d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("1"), edges: EdgeSel::AllOfBody }))),
            Cmd::Chamfer => Sel::Feature(d.add_feature(FeatureKind::Chamfer(EdgeOp { value: s("1"), edges: EdgeSel::AllOfBody }))),
            Cmd::Plane => Sel::Ref(d.add_offset_plane(plane, "20")),
            Cmd::Param => {
                let n = (1..).map(|i| format!("P{i}")).find(|n| !d.params.iter().any(|p| &p.name == n)).unwrap();
                let _ = d.add_param(&n, "10", Unit::Comprimento);
                Sel::Params
            }
            Cmd::Folder => {
                let n = format!("Pasta {}", d.folders.len() + 1);
                Sel::Folder(d.add_folder(&n))
            }
            Cmd::Collector => {
                let n = format!("Corpo {}", d.collectors.len() + 1);
                let id = d.add_collector(&n);
                d.active_collector = id;
                Sel::Collector(id)
            }
        };
        if let Sel::Feature(id) = new_sel {
            let d = &mut self.open.doc;
            let (consumed, is_sketch) = match d.feature(id).map(|f| &f.kind) {
                Some(FeatureKind::Extrude(e)) => (Some(e.sketch), false),
                Some(FeatureKind::Revolve(r)) => (Some(r.sketch), false),
                Some(FeatureKind::Hole(h)) => (Some(h.sketch), false),
                Some(FeatureKind::Sketch(_)) => (None, true),
                _ => (None, false),
            };
            // sketch consumido por um comando fica oculto (pode ser reexibido pelo olho na árvore)
            if let Some(f) = consumed.and_then(|s| d.feature_mut(s)) {
                f.hidden = true;
            }
            if is_sketch {
                self.align_pending = Some(id);
            }
        }
        self.sel = new_sel;
    }

    /// Executa uma ação da barra de ferramentas.
    fn act(&mut self, a: Act) {
        match a {
            Act::Cmd(c) => self.command(c),
            Act::Fit => self.fit_pending = true,
            Act::View(y, p) => {
                if let Some(v) = self.viewport.as_mut() {
                    v.camera.set_view(y, p)
                }
            }
            Act::Normal => self.align_to_selection(),
            Act::Ortho => {
                self.settings.ortho = !self.settings.ortho;
                if let Some(v) = self.viewport.as_mut() {
                    v.camera.ortho = self.settings.ortho
                }
            }
            Act::HiddenEnv => {
                self.hidden_env = !self.hidden_env;
                self.scene_dirty = true;
                let msg = if self.hidden_env { "Ambiente de ocultos: mostrando só os itens ocultos" } else { "Ambiente normal" };
                self.toast(msg, false);
            }
            Act::Settings => self.settings_open = !self.settings_open,
            Act::Dock(g, d) => self.settings.set_dock(g, d),
        }
    }

    /// Olha de frente para o plano do item selecionado (sketch, plano da Origem ou referência).
    fn align_to_selection(&mut self) {
        let Some(r) = &self.result else { return };
        let frame = match self.sel {
            Sel::Feature(id) => r.sketches.iter().find(|s| s.feature == id).map(|s| s.frame),
            Sel::Ref(id) => r.planes.iter().find(|p| p.id == Some(id)).map(|p| p.frame),
            Sel::Origin(o) => Some(scc_solidos::origin_frame(o)),
            _ => None,
        };
        match (frame, self.viewport.as_mut()) {
            (Some(f), Some(v)) => {
                let c = r
                    .sketches
                    .iter()
                    .find(|s| matches!(self.sel, Sel::Feature(id) if id == s.feature))
                    .and_then(|s| {
                        let n = s.lines.len().max(1) as f32;
                        (!s.lines.is_empty()).then(|| s.lines.iter().fold(Vec3::ZERO, |a, p| a + Vec3::from(*p)) / n)
                    })
                    .unwrap_or(Vec3::from(f.origin.map(|x| x as f32)));
                v.camera.look_along(Vec3::from(f.normal.map(|x| x as f32)), c);
            }
            _ => self.toast("Selecione um sketch ou plano para alinhar a vista", true),
        }
    }

    pub fn ask_delete(&mut self, id: Id) {
        let deps = self.doc().dependents(id);
        if deps.is_empty() {
            self.open.doc.delete(id, false);
            self.sel = Sel::None;
        } else {
            self.delete_dialog = Some(DeleteDialog { target: id, dependents: deps, replacement: None });
        }
    }

    fn delete_selected(&mut self) {
        match self.sel {
            Sel::Feature(id) | Sel::Ref(id) => self.ask_delete(id),
            Sel::Folder(id) => {
                self.open.doc.remove_folder(id);
                self.sel = Sel::None;
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ painéis

    fn top(&mut self, root: &mut egui::Ui) {
        egui::Panel::top("menu").show(root, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("◆ SolidCadCraft").strong().color(Color32::from_rgb(120, 180, 255)));
                ui.separator();
                ui.menu_button("Arquivo", |ui| {
                    if ui.button("Nova peça").clicked() {
                        self.load_doc(Document::new("Peça"), None, None);
                        ui.close();
                    }
                    if ui.button("Nova peça de exemplo").clicked() {
                        self.load_doc(scc_solidos::example_document(), None, None);
                        ui.close();
                    }
                    if ui.button("Abrir peça…").clicked() {
                        self.open_file();
                        ui.close();
                    }
                    if ui.button("Salvar   Ctrl+S").clicked() {
                        self.save();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Exportar STEP…").clicked() {
                        self.export("step");
                        ui.close();
                    }
                    if ui.button("Exportar STL…").clicked() {
                        self.export("stl");
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Novo projeto…").clicked() {
                        self.new_project();
                        ui.close();
                    }
                    if ui.button("Abrir projeto…").clicked() {
                        self.open_project();
                        ui.close();
                    }
                });
                ui.menu_button("Exibir", |ui| {
                    ui.menu_button("Estilo de exibição", |ui| {
                        for m in crate::render::DisplayMode::ALL {
                            ui.radio_value(&mut self.settings.display, m, m.label());
                        }
                    });
                    if ui.checkbox(&mut self.hidden_env, "Ambiente de ocultos   Shift+H").changed() {
                        self.scene_dirty = true;
                    }
                    ui.menu_button("Barras de ferramentas", |ui| {
                        for g in crate::settings::Group::ALL {
                            ui.menu_button(g.label(), |ui| {
                                for d in Dock::ALL {
                                    if ui.radio(self.settings.dock(g) == d, d.label()).clicked() {
                                        self.settings.set_dock(g, d);
                                        ui.close();
                                    }
                                }
                            });
                        }
                    });
                    ui.separator();
                    if ui.button("Configurações…").clicked() {
                        self.settings_open = true;
                        ui.close();
                    }
                });
                ui.menu_button("Editar", |ui| {
                    if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Desfazer   Ctrl+Z")).clicked() {
                        self.undo();
                        ui.close();
                    }
                    if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Refazer   Ctrl+Y")).clicked() {
                        self.redo();
                        ui.close();
                    }
                });
                ui.separator();
                // abas de módulo (RF-UI-006)
                let _ = ui.selectable_label(true, "Modelagem Sólida");
                for m in ["Superfícies", "Montagem", "Desenho", "Chapas", "Soldas"] {
                    ui.add_enabled(false, egui::Button::new(m).frame(false)).on_disabled_hover_text("Módulo em breve");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let title = match (&self.open.frozen, self.open.modified) {
                        (Some(r), _) => format!("{} — revisão {r} (congelada)", self.open.doc.name),
                        (None, true) => format!("{} •", self.open.doc.name),
                        (None, false) => self.open.doc.name.clone(),
                    };
                    ui.label(RichText::new(title).weak());
                });
            });
        });

        let top = toolbar::groups_at(&self.settings, Dock::Topo);
        if !top.is_empty() {
            let mut acts = vec![];
            egui::Panel::top("comandos").show(root, |ui| {
                egui::ScrollArea::horizontal().id_salt("barra_topo").show(ui, |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        for (i, g) in top.iter().enumerate() {
                            if i > 0 {
                                ui.separator();
                            }
                            acts.extend(toolbar::show_group(ui, *g, &mut self.settings, self.hidden_env, false));
                        }
                    });
                });
            });
            for a in acts {
                self.act(a);
            }
        }
    }

    /// Grupos fixos na lateral e grupos flutuantes.
    fn side_toolbars(&mut self, root: &mut egui::Ui) {
        let mut acts = vec![];
        let side = toolbar::groups_at(&self.settings, Dock::Lateral);
        if !side.is_empty() {
            egui::Panel::left("barra_lateral").resizable(false).show(root, |ui| {
                egui::ScrollArea::vertical().id_salt("barra_lat").show(ui, |ui| {
                    for (i, g) in side.iter().enumerate() {
                        if i > 0 {
                            ui.separator();
                        }
                        acts.extend(toolbar::show_group(ui, *g, &mut self.settings, self.hidden_env, true));
                    }
                });
            });
        }
        let ctx = root.ctx().clone();
        for (i, g) in toolbar::groups_at(&self.settings, Dock::Flutuante).into_iter().enumerate() {
            egui::Window::new(g.label())
                .id(egui::Id::unique(("flutuante", g.label())))
                .default_pos(egui::pos2(320. + 40. * i as f32, 140. + 40. * i as f32))
                .resizable(false)
                .collapsible(true)
                .show(&ctx, |ui| {
                    acts.extend(toolbar::show_group(ui, g, &mut self.settings, self.hidden_env, false));
                });
        }
        for a in acts {
            self.act(a);
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = true;
        let s = &mut self.settings;
        egui::Window::new("Configurações").open(&mut open).default_width(380.).resizable(true).show(ctx, |ui| {
            egui::CollapsingHeader::new("Interface").default_open(true).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Tema");
                    ui.radio_value(&mut s.theme, Theme::Escuro, "Escuro");
                    ui.radio_value(&mut s.theme, Theme::Claro, "Claro");
                });
                ui.add(egui::Slider::new(&mut s.ui_scale, 0.75..=1.75).text("escala (fontes e controles)"));
                ui.add(egui::Slider::new(&mut s.icon_size, 20.0..=48.0).text("tamanho dos ícones"));
                ui.checkbox(&mut s.show_labels, "mostrar o nome embaixo dos ícones");
            });
            egui::CollapsingHeader::new("Fundo da viewport").default_open(true).show(ui, |ui| {
                ui.checkbox(&mut s.gradient, "degradê de duas cores");
                ui.horizontal(|ui| {
                    ui.label(if s.gradient { "Topo" } else { "Cor" });
                    if s.gradient {
                        ui.color_edit_button_srgb(&mut s.bg_top);
                        ui.label("Base");
                    }
                    ui.color_edit_button_srgb(&mut s.bg_bottom);
                });
                ui.horizontal_wrapped(|ui| {
                    for (n, t, b) in BACKGROUNDS {
                        if ui.small_button(n).clicked() {
                            s.bg_top = t;
                            s.bg_bottom = b;
                            s.gradient = t != b;
                        }
                    }
                });
            });
            egui::CollapsingHeader::new("Exibição").default_open(true).show(ui, |ui| {
                for m in crate::render::DisplayMode::ALL {
                    ui.radio_value(&mut s.display, m, m.label());
                }
                ui.checkbox(&mut s.edges_in_render, "arestas no semi-renderizado");
                ui.checkbox(&mut s.ortho, "projeção ortogonal");
            });
            egui::CollapsingHeader::new("Barras de ferramentas").show(ui, |ui| {
                for g in crate::settings::Group::ALL {
                    ui.horizontal(|ui| {
                        ui.add_sized([110., 18.], egui::Label::new(g.label()));
                        let mut d = s.dock(g);
                        egui::ComboBox::from_id_salt(("dock", g.label())).selected_text(d.label()).show_ui(ui, |ui| {
                            for o in Dock::ALL {
                                ui.selectable_value(&mut d, o, o.label());
                            }
                        });
                        s.set_dock(g, d);
                    });
                }
                ui.label(RichText::new("Dica: clique direito em um grupo para mudar onde ele fica.").weak().small());
            });
            egui::CollapsingHeader::new("Cotagem").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Norma");
                    egui::ComboBox::from_id_salt("norma").selected_text(s.dim_standard.clone()).show_ui(ui, |ui| {
                        for n in ["ABNT NBR 10126", "ISO 129-1", "ASME Y14.5"] {
                            ui.selectable_value(&mut s.dim_standard, n.to_string(), n);
                        }
                    });
                });
                ui.add(egui::Slider::new(&mut s.dim_decimals, 0..=4).text("casas decimais"));
                ui.add(egui::Slider::new(&mut s.dim_text_height, 1.8..=7.0).text("altura do texto (mm)"));
                ui.label(RichText::new("Usado pela ferramenta de cotagem do sketch e pelo desenho técnico.").weak().small());
            });
            egui::CollapsingHeader::new("Sketch").show(ui, |ui| {
                ui.checkbox(&mut s.grid, "grade");
                ui.add(egui::Slider::new(&mut s.grid_step, 1.0..=50.0).text("passo da grade (mm)"));
                ui.checkbox(&mut s.snaps, "snaps");
                ui.checkbox(&mut s.auto_constraints, "restrições automáticas");
                ui.checkbox(&mut s.auto_dims, "cotas automáticas");
                ui.label(RichText::new("Aplicado pelo editor de sketch interativo (próxima alpha).").weak().small());
            });
            ui.separator();
            if ui.button("Restaurar padrões").clicked() {
                *s = Settings::default();
            }
        });
        self.settings_open = open;
    }

    fn tree(&mut self, ui: &mut egui::Ui) {
        let doc = self.open.doc.clone();
        let sel = self.sel;
        let mut new_sel = None;
        let mut actions: Vec<TreeAction> = vec![];

        let pick = |ui: &mut egui::Ui, s: Sel, text: RichText| -> egui::Response { ui.selectable_label(sel == s, text) };

        let r = pick(ui, Sel::Part, RichText::new(format!("🧊 {}", doc.name)).strong());
        if r.clicked() {
            new_sel = Some(Sel::Part);
        }
        r.context_menu(|ui| {
            appearance_menu(ui, doc.appearance, AppTarget::Part, &mut actions);
        });
        if !doc.props.pn.is_empty() {
            ui.label(RichText::new(format!("   PN {} · {}", doc.props.pn, doc.props.part_type.label())).weak().small());
        }

        egui::CollapsingHeader::new("📍 Origem").default_open(true).show(ui, |ui| {
            for o in [OriginPlane::XY, OriginPlane::XZ, OriginPlane::YZ] {
                if pick(ui, Sel::Origin(o), RichText::new(format!("▱ Plano {}", scene::plane_name(o)))).clicked() {
                    new_sel = Some(Sel::Origin(o));
                }
            }
            ui.label(RichText::new("↗ Eixos X, Y, Z · ponto de origem").weak());
            let mut v = doc.origin_visible;
            if ui.checkbox(&mut v, "mostrar origem").changed() {
                actions.push(TreeAction::OriginVisible(v));
            }
        });

        egui::CollapsingHeader::new(format!("📐 Referências ({})", doc.references.len())).default_open(true).show(ui, |ui| {
            if doc.references.is_empty() {
                ui.label(RichText::new("vazio — use ▱ Plano").weak().italics());
            }
            for r in &doc.references {
                let users = doc.dependents(r.id).len();
                let txt = format!("▱ {}{}", r.name, if users > 0 { format!("  ·{users}") } else { String::new() });
                let resp = ui
                    .horizontal(|ui| {
                        if eye(ui, !r.visible) {
                            actions.push(TreeAction::RefVisible(r.id, !r.visible));
                        }
                        pick(ui, Sel::Ref(r.id), if r.visible { RichText::new(txt) } else { RichText::new(txt).weak() })
                    })
                    .inner;
                if resp.clicked() {
                    new_sel = Some(Sel::Ref(r.id));
                }
                resp.context_menu(|ui| {
                    if ui.button("Novo sketch neste plano").clicked() {
                        actions.push(TreeAction::SketchOn(PlaneRef::Datum(r.id)));
                        ui.close();
                    }
                    if ui.button("Apagar…").clicked() {
                        actions.push(TreeAction::Delete(r.id));
                        ui.close();
                    }
                });
            }
        });

        egui::CollapsingHeader::new(format!("ƒ Parâmetros ({})", doc.params.len())).default_open(true).show(ui, |ui| {
            let vars = self.vars();
            for p in &doc.params {
                let v = vars.get(&p.name).map(|v| format!("{} {}", fmt(*v), p.unit.symbol())).unwrap_or("⚠".into());
                if pick(ui, Sel::Params, RichText::new(format!("{} = {v}", p.name))).clicked() {
                    new_sel = Some(Sel::Params);
                }
            }
            if doc.params.is_empty() && pick(ui, Sel::Params, RichText::new("vazio — clique para editar").weak().italics()).clicked() {
                new_sel = Some(Sel::Params);
            }
        });

        egui::CollapsingHeader::new("▣ Coletores de corpos").default_open(true).show(ui, |ui| {
            for c in &doc.collectors {
                ui.horizontal(|ui| {
                    let mut v = c.visible;
                    if ui.checkbox(&mut v, "").on_hover_text("visível").changed() {
                        actions.push(TreeAction::CollectorVisible(c.id, v));
                    }
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(10., 10.), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2., Color32::from_rgb(c.color[0], c.color[1], c.color[2]));
                    let active = doc.active_collector == c.id;
                    let txt = if active { RichText::new(format!("{} (ativo)", c.name)).strong() } else { RichText::new(&c.name) };
                    let resp = pick(ui, Sel::Collector(c.id), txt);
                    if resp.clicked() {
                        new_sel = Some(Sel::Collector(c.id));
                    }
                    if resp.double_clicked() {
                        actions.push(TreeAction::Activate(c.id));
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Tornar ativo").clicked() {
                            actions.push(TreeAction::Activate(c.id));
                            ui.close();
                        }
                        appearance_menu(ui, Some(Appearance { color: c.color, finish: c.finish }), AppTarget::Collector(c.id), &mut actions);
                    });
                });
            }
        });

        ui.separator();
        ui.label(RichText::new("Comandos").weak());
        let mut drawn_folders = vec![];
        for (i, f) in doc.features.iter().enumerate() {
            if i == doc.rollback {
                marker(ui, i, doc.features.len(), &mut actions);
            }
            match f.folder {
                Some(fid) if drawn_folders.contains(&fid) => continue,
                Some(fid) => {
                    drawn_folders.push(fid);
                    let name = doc.folders.iter().find(|x| x.id == fid).map(|x| x.name.clone()).unwrap_or("?".into());
                    let h = egui::CollapsingHeader::new(format!("🗀 {name}")).id_salt(("pasta", fid.0)).default_open(true).show(ui, |ui| {
                        for (j, g) in doc.features.iter().enumerate().filter(|(_, g)| g.folder == Some(fid)) {
                            self.feature_row(ui, &doc, j, g, &mut new_sel, &mut actions);
                        }
                    });
                    if h.header_response.clicked() {
                        new_sel = Some(Sel::Folder(fid));
                    }
                }
                None => self.feature_row(ui, &doc, i, f, &mut new_sel, &mut actions),
            }
        }
        if doc.rollback >= doc.features.len() {
            marker(ui, doc.features.len(), doc.features.len(), &mut actions);
        }
        for fo in doc.folders.iter().filter(|fo| !drawn_folders.contains(&fo.id)) {
            if pick(ui, Sel::Folder(fo.id), RichText::new(format!("🗀 {} (vazia)", fo.name)).weak()).clicked() {
                new_sel = Some(Sel::Folder(fo.id));
            }
        }

        if let Some(s) = new_sel {
            self.sel = s;
        }
        for a in actions {
            self.tree_action(a);
        }
    }

    fn feature_row(&self, ui: &mut egui::Ui, doc: &Document, i: usize, f: &Feature, new_sel: &mut Option<Sel>, actions: &mut Vec<TreeAction>) {
        let st = self.status_of(f.id);
        let (badge, color) = match st {
            Some(Status::Error(_)) => (" ✖", Color32::from_rgb(255, 110, 100)),
            Some(Status::Warning(_)) => (" ⚠", Color32::from_rgb(240, 200, 90)),
            Some(Status::Inactive) => ("", Color32::GRAY),
            _ => ("", ui.visuals().text_color()),
        };
        let mut text = RichText::new(format!("{} {}{badge}", f.kind.icon(), f.name)).color(color);
        if f.suppressed {
            text = text.strikethrough();
        }
        if f.hidden {
            text = text.italics();
        }
        let resp = ui
            .horizontal(|ui| {
                if eye(ui, f.hidden) {
                    actions.push(TreeAction::Hide(f.id, !f.hidden));
                }
                if let Some(a) = f.appearance {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(8., 8.), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 4., Color32::from_rgb(a.color[0], a.color[1], a.color[2]));
                }
                ui.selectable_label(self.sel == Sel::Feature(f.id), text)
            })
            .inner;
        if resp.double_clicked() && matches!(f.kind, FeatureKind::Sketch(_)) {
            actions.push(TreeAction::Align(f.id));
        }
        let resp = match st.and_then(|s| s.message()) {
            Some(m) => resp.on_hover_text(m),
            None => resp,
        };
        if resp.clicked() {
            *new_sel = Some(Sel::Feature(f.id));
        }
        resp.context_menu(|ui| {
            if ui.button(if f.suppressed { "Reativar" } else { "Suprimir" }).clicked() {
                actions.push(TreeAction::Suppress(f.id));
                ui.close();
            }
            if ui.button(if f.hidden { "Mostrar   H" } else { "Ocultar   H" }).clicked() {
                actions.push(TreeAction::Hide(f.id, !f.hidden));
                ui.close();
            }
            if matches!(f.kind, FeatureKind::Sketch(_)) && ui.button("Alinhar vista ao sketch   N").clicked() {
                actions.push(TreeAction::Align(f.id));
                ui.close();
            }
            appearance_menu(ui, f.appearance, AppTarget::Feature(f.id), actions);
            if ui.button("Marcador até aqui").clicked() {
                actions.push(TreeAction::Rollback(i + 1));
                ui.close();
            }
            ui.menu_button("Mover para pasta", |ui| {
                if ui.button("(raiz)").clicked() {
                    actions.push(TreeAction::ToFolder(f.id, None));
                    ui.close();
                }
                for fo in &doc.folders {
                    if ui.button(&fo.name).clicked() {
                        actions.push(TreeAction::ToFolder(f.id, Some(fo.id)));
                        ui.close();
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui.button("◀ antes").clicked() && i > 0 {
                    actions.push(TreeAction::Move(f.id, i - 1));
                }
                if ui.button("depois ▶").clicked() {
                    actions.push(TreeAction::Move(f.id, i + 1));
                }
            });
            if let FeatureKind::Sketch(s) = &f.kind {
                if ui.button("Extrudar este sketch").clicked() {
                    actions.push(TreeAction::Select(f.id, Cmd::Extrude));
                    ui.close();
                }
                let _ = s;
            }
            ui.separator();
            if ui.button("Apagar…").clicked() {
                actions.push(TreeAction::Delete(f.id));
                ui.close();
            }
        });
    }

    fn tree_action(&mut self, a: TreeAction) {
        let d = &mut self.open.doc;
        match a {
            TreeAction::OriginVisible(v) => d.origin_visible = v,
            TreeAction::CollectorVisible(id, v) => {
                if let Some(c) = d.collectors.iter_mut().find(|c| c.id == id) {
                    c.visible = v
                }
            }
            TreeAction::Activate(id) => d.active_collector = id,
            TreeAction::Suppress(id) => {
                if let Some(f) = d.feature_mut(id) {
                    f.suppressed = !f.suppressed
                }
            }
            TreeAction::Rollback(n) => d.rollback = n.min(d.features.len()),
            TreeAction::ToFolder(id, fo) => {
                if let Some(f) = d.feature_mut(id) {
                    f.folder = fo
                }
            }
            TreeAction::Move(id, to) => {
                if let Err(e) = d.move_feature(id, to) {
                    self.toast(e.to_string(), true);
                }
            }
            TreeAction::Delete(id) => self.ask_delete(id),
            TreeAction::SketchOn(p) => {
                self.sel = match p {
                    PlaneRef::Datum(id) => Sel::Ref(id),
                    PlaneRef::Origin(o) => Sel::Origin(o),
                };
                self.command(Cmd::Sketch);
            }
            TreeAction::Select(id, c) => {
                self.sel = Sel::Feature(id);
                self.command(c);
            }
            TreeAction::Hide(id, h) => {
                if let Some(f) = d.feature_mut(id) {
                    f.hidden = h
                }
            }
            TreeAction::RefVisible(id, v) => {
                if let Some(r) = d.references.iter_mut().find(|r| r.id == id) {
                    r.visible = v
                }
            }
            TreeAction::SetAppearance(t, a) => match t {
                AppTarget::Part => d.appearance = a,
                AppTarget::Feature(id) => {
                    if let Some(f) = d.feature_mut(id) {
                        f.appearance = a
                    }
                }
                AppTarget::Collector(id) => {
                    if let (Some(c), Some(a)) = (d.collectors.iter_mut().find(|c| c.id == id), a) {
                        c.color = a.color;
                        c.finish = a.finish;
                    }
                }
            },
            TreeAction::Customize(t) => {
                self.sel = match t {
                    AppTarget::Part => Sel::Part,
                    AppTarget::Feature(id) => Sel::Feature(id),
                    AppTarget::Collector(id) => Sel::Collector(id),
                };
            }
            TreeAction::Align(id) => {
                self.sel = Sel::Feature(id);
                self.align_to_selection();
            }
        }
    }

    /// Alterna ocultar/mostrar o item selecionado (tecla H).
    fn toggle_hide_selected(&mut self) {
        let d = &mut self.open.doc;
        match self.sel {
            Sel::Feature(id) => {
                if let Some(f) = d.feature_mut(id) {
                    f.hidden = !f.hidden
                }
            }
            Sel::Ref(id) => {
                if let Some(r) = d.references.iter_mut().find(|r| r.id == id) {
                    r.visible = !r.visible
                }
            }
            Sel::Collector(id) => {
                if let Some(c) = d.collectors.iter_mut().find(|c| c.id == id) {
                    c.visible = !c.visible
                }
            }
            Sel::Origin(_) => d.origin_visible = !d.origin_visible,
            _ => {}
        }
    }

    fn project_panel(&mut self, ui: &mut egui::Ui) {
        let Some(p) = &self.project else {
            ui.add_space(8.);
            ui.label("Nenhum projeto aberto.");
            ui.label(RichText::new("Um projeto é uma pasta com as peças, o histórico de versões e as revisões liberadas.").weak());
            ui.add_space(8.);
            if ui.button("📁 Novo projeto…").clicked() {
                self.new_project();
            }
            if ui.button("📂 Abrir projeto…").clicked() {
                self.open_project();
            }
            return;
        };
        ui.label(RichText::new(format!("📁 {}", p.index.name)).strong());
        ui.label(RichText::new(p.root.display().to_string()).weak().small());
        ui.add_space(4.);

        let mut open = None;
        let mut select = None;
        egui::Grid::new("docs").striped(true).num_columns(4).show(ui, |ui| {
            ui.label(RichText::new("PN").strong());
            ui.label(RichText::new("Nome").strong());
            ui.label(RichText::new("Estado").strong());
            ui.label(RichText::new("Rev").strong());
            ui.end_row();
            for d in &p.index.documents {
                let current = self.open.entry == Some(d.id);
                let r = ui.selectable_label(self.project_sel == Some(d.id), if current { RichText::new(&d.pn).strong() } else { RichText::new(&d.pn) });
                if r.clicked() {
                    select = Some(d.id);
                }
                if r.double_clicked() {
                    open = Some(d.id);
                }
                ui.label(&d.name);
                let c = match d.state {
                    DocState::Liberado => Color32::from_rgb(120, 200, 120),
                    DocState::Obsoleto => Color32::GRAY,
                    DocState::EmTrabalho => Color32::from_rgb(230, 190, 90),
                };
                ui.label(RichText::new(d.state.label()).color(c));
                ui.label(d.current_revision().map(|r| r.label.as_str()).unwrap_or("-"));
                ui.end_row();
            }
        });
        if p.index.documents.is_empty() {
            ui.label(RichText::new("Sem documentos. Adicione a peça aberta ou crie uma nova.").weak());
        }
        if let Some(id) = select {
            self.project_sel = Some(id);
        }
        if let Some(id) = open {
            self.open_entry(id, None);
        }

        ui.add_space(6.);
        ui.horizontal_wrapped(|ui| {
            if ui.button("➕ Nova peça").clicked() {
                let p = self.project.as_mut().unwrap();
                match p.add_document(Document::new("Peça")) {
                    Ok(id) => {
                        self.project_sel = Some(id);
                        self.open_entry(id, None);
                    }
                    Err(e) => self.toast(e.to_string(), true),
                }
            }
            if self.open.entry.is_none() && ui.button("⤵ Adicionar peça aberta").clicked() {
                let p = self.project.as_mut().unwrap();
                match p.add_document(self.open.doc.clone()) {
                    Ok(id) => {
                        self.open.entry = Some(id);
                        self.open.doc.props.pn = p.entry(id).map(|e| e.pn.clone()).unwrap_or_default();
                        self.open.modified = false;
                        self.project_sel = Some(id);
                    }
                    Err(e) => self.toast(e.to_string(), true),
                }
            }
            if ui.button("💾 Salvar versão").clicked() {
                self.save();
            }
            if ui.button("📄 Estrutura CSV").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name("estrutura.csv").save_file() {
                    if let Err(e) = self.project.as_ref().unwrap().export_csv(&path) {
                        self.toast(e.to_string(), true);
                    }
                }
            }
        });

        let Some(sel) = self.project_sel else { return };
        let Ok(e) = self.project.as_ref().unwrap().entry(sel).cloned() else { return };
        ui.separator();
        ui.label(RichText::new(format!("{} — {}", e.pn, e.name)).strong());
        if !e.description.is_empty() {
            ui.label(&e.description);
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button("Abrir").clicked() {
                self.open_entry(sel, None);
            }
            if ui.add_enabled(e.dirty_since_release(), egui::Button::new("🔒 Liberar revisão…")).clicked() {
                self.release_dialog = Some((sel, String::new()));
            }
            if ui.button("Duplicar").clicked() {
                if let Err(err) = self.project.as_mut().unwrap().duplicate(sel) {
                    self.toast(err.to_string(), true);
                }
            }
            if e.state != DocState::Obsoleto && ui.button("Tornar obsoleto").clicked() {
                let _ = self.project.as_mut().unwrap().set_state(sel, DocState::Obsoleto);
            }
        });
        egui::CollapsingHeader::new(format!("Revisões ({})", e.revisions.len())).default_open(true).show(ui, |ui| {
            for r in e.revisions.iter().rev() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Rev {}", r.label)).strong());
                    ui.label(RichText::new(&r.comment).weak());
                    if ui.small_button("abrir 🔒").on_hover_text("Abrir congelada (somente leitura)").clicked() {
                        self.open_entry(sel, Some((r.hash.clone(), r.label.clone())));
                    }
                });
            }
        });
        egui::CollapsingHeader::new(format!("Versões ({})", e.versions.len())).show(ui, |ui| {
            for (i, v) in e.versions.iter().enumerate().rev().take(30) {
                ui.horizontal(|ui| {
                    ui.label(format!("v{}", i + 1));
                    ui.label(RichText::new(format!("{} · {}", v.note, &v.hash[..8])).weak());
                    if ui.small_button("abrir").clicked() {
                        self.open_entry(sel, Some((v.hash.clone(), format!("v{}", i + 1))));
                    }
                });
            }
        });
    }

    fn timeline(&mut self, root: &mut egui::Ui) {
        egui::Panel::bottom("status").show(root, |ui| {
            ui.horizontal(|ui| {
                let (k, ms) = self.result.as_ref().map(|r| (r.kernel.clone(), r.elapsed_ms)).unwrap_or_default();
                let busy = self.gen_done < self.gen_sent;
                ui.label(RichText::new(if busy { "⏳ recalculando…".to_string() } else { format!("✔ recalculado em {ms:.0} ms") }).small());
                ui.separator();
                ui.label(RichText::new(k).small().weak());
                if let Some(e) = self.result.as_ref().and_then(|r| r.param_error.clone()) {
                    ui.separator();
                    ui.label(RichText::new(format!("Parâmetros: {e}")).small().color(Color32::from_rgb(255, 120, 100)));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Botão esq./meio: girar · direito ou Shift: mover · roda: zoom · duplo clique: ajustar").small().weak());
                });
            });
        });
    }

    fn viewport(&mut self, root: &mut egui::Ui, frame: &eframe::Frame) {
        let ctx = root.ctx().clone();
        egui::CentralPanel::no_frame().show(root, |ui| {
            let rect = ui.max_rect();
            let resp = ui.allocate_rect(rect, egui::Sense::click_and_drag());
            let Some(vp) = self.viewport.as_mut() else {
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Viewport requer wgpu", egui::FontId::proportional(16.), Color32::WHITE);
                return;
            };

            // navegação
            let (shift, scroll, ppp) = ctx.input(|i| (i.modifiers.shift, i.smooth_scroll_delta.y, i.pixels_per_point));
            let delta = resp.drag_delta();
            let pan = resp.dragged_by(egui::PointerButton::Secondary) || (shift && resp.dragged());
            if pan {
                let (r, u) = vp.camera.basis();
                let k = vp.camera.dist * 0.0012;
                vp.camera.target += (-r * delta.x + u * delta.y) * k;
            } else if resp.dragged_by(egui::PointerButton::Primary) || resp.dragged_by(egui::PointerButton::Middle) {
                vp.camera.yaw -= delta.x * 0.008;
                vp.camera.pitch = (vp.camera.pitch + delta.y * 0.008).clamp(-1.5706, 1.5706);
            }
            if resp.hovered() && scroll != 0. {
                vp.camera.dist = (vp.camera.dist * (-scroll * 0.0015).exp()).clamp(0.5, 1e6);
            }
            if resp.double_clicked() || self.fit_pending {
                if let Some(b) = vp.bounds {
                    vp.camera.fit(b);
                    self.fit_pending = false;
                }
            }

            if let Some(rs) = frame.wgpu_render_state() {
                let size = [(rect.width() * ppp) as u32, (rect.height() * ppp) as u32];
                let tex = vp.render(rs, size, self.settings.style());
                ui.painter().image(tex, rect, egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)), Color32::WHITE);
            }

            // texto legível sobre o fundo (claro ou escuro)
            let lum = |c: [u8; 3]| 0.3 * c[0] as f32 + 0.59 * c[1] as f32 + 0.11 * c[2] as f32;
            let light_bg = lum(self.settings.bg_bottom) > 140.;
            let ink = if light_bg { Color32::from_black_alpha(170) } else { Color32::from_white_alpha(150) };

            // rótulos dos planos
            if let (Some(r), true) = (&self.result, self.settings.filter.rotulos) {
                for p in &r.planes {
                    let f = &p.frame;
                    let at = Vec3::from(f.origin.map(|v| v as f32));
                    let size = vp.bounds.map(|(a, b)| (b - a).max_element().max(20.) * 0.6).unwrap_or(60.);
                    let corner = at + (Vec3::from(f.x.map(|v| v as f32)) + Vec3::from(f.y().map(|v| v as f32))) * size * 0.45;
                    if let Some(pos) = vp.project(rect, corner) {
                        ui.painter().text(pos, egui::Align2::LEFT_BOTTOM, &p.name, egui::FontId::proportional(11.), ink);
                    }
                }
            }

            // estado da vista no canto superior esquerdo
            let info = format!(
                "{}{} · {}",
                if self.hidden_env { "AMBIENTE DE OCULTOS · " } else { "" },
                if vp.camera.ortho { "Ortogonal" } else { "Perspectiva" },
                self.settings.display.label()
            );
            let color = if self.hidden_env { Color32::from_rgb(255, 190, 80) } else { ink };
            ui.painter().text(rect.left_top() + egui::vec2(10., 8.), egui::Align2::LEFT_TOP, info, egui::FontId::proportional(12.), color);
            if self.hidden_env {
                ui.painter().rect_stroke(rect.shrink(2.), 0., egui::Stroke::new(3., Color32::from_rgb(255, 170, 60)), egui::StrokeKind::Inside);
            }

            if !scc_kernel::AVAILABLE {
                ui.painter().text(
                    rect.center_top() + egui::vec2(0., 40.),
                    egui::Align2::CENTER_TOP,
                    "Kernel geométrico indisponível — compile com o OpenCASCADE instalado para ver os sólidos",
                    egui::FontId::proportional(14.),
                    Color32::from_rgb(255, 180, 90),
                );
            }
            if let Some(r) = &self.result {
                let mass: Vec<String> = r
                    .bodies
                    .iter()
                    .map(|b| {
                        let name = self.open.doc.collectors.iter().find(|c| c.id == b.collector).map(|c| c.name.as_str()).unwrap_or("");
                        format!("{name}: {:.0} mm³ · {} faces", b.mass.volume, b.faces)
                    })
                    .collect();
                ui.painter().text(rect.right_bottom() - egui::vec2(10., 8.), egui::Align2::RIGHT_BOTTOM, mass.join("\n"), egui::FontId::proportional(11.), ink);
            }
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(dd) = &mut self.delete_dialog {
            let doc = &self.open.doc;
            let mut close = false;
            let mut action = None;
            egui::Modal::new(egui::Id::unique("apagar")).show(ctx, |ui| {
                ui.set_max_width(420.);
                ui.heading(format!("Apagar {}?", doc.name_of(dd.target)));
                ui.label(format!("{} objeto(s) dependem dele:", dd.dependents.len()));
                for d in &dd.dependents {
                    ui.label(format!("  • {}", doc.name_of(*d)));
                }
                ui.add_space(6.);
                // substituir referência por um objeto compatível (RF-NUC-023)
                let is_sketch = doc.feature(dd.target).is_some_and(|f| matches!(f.kind, FeatureKind::Sketch(_)));
                let cands: Vec<(Id, String)> = if is_sketch {
                    doc.features.iter().filter(|f| f.id != dd.target && matches!(f.kind, FeatureKind::Sketch(_))).map(|f| (f.id, f.name.clone())).collect()
                } else if doc.reference(dd.target).is_some() {
                    doc.references.iter().filter(|r| r.id != dd.target).map(|r| (r.id, r.name.clone())).collect()
                } else {
                    vec![]
                };
                if !cands.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Substituir por:");
                        let cur = dd.replacement.map(|id| doc.name_of(id)).unwrap_or("escolha…".into());
                        egui::ComboBox::from_id_salt("subst").selected_text(cur).show_ui(ui, |ui| {
                            for (id, n) in &cands {
                                ui.selectable_value(&mut dd.replacement, Some(*id), n);
                            }
                        });
                        if ui.add_enabled(dd.replacement.is_some(), egui::Button::new("Substituir e apagar")).clicked() {
                            action = Some(DelAction::Replace(dd.target, dd.replacement.unwrap()));
                        }
                    });
                }
                ui.add_space(6.);
                ui.horizontal(|ui| {
                    if ui.button("Apagar junto com dependentes").clicked() {
                        action = Some(DelAction::All(dd.target));
                    }
                    if ui.button("Apagar só este").on_hover_text("Os dependentes ficam com aviso de referência perdida e podem ser reapontados").clicked() {
                        action = Some(DelAction::Only(dd.target));
                    }
                    if ui.button("Cancelar").clicked() {
                        close = true;
                    }
                });
            });
            if let Some(a) = action {
                let d = &mut self.open.doc;
                match a {
                    DelAction::Replace(old, new) => {
                        let _ = d.replace_reference(old, new);
                        d.delete(old, false);
                    }
                    DelAction::All(id) => d.delete(id, true),
                    DelAction::Only(id) => d.delete(id, false),
                }
                self.sel = Sel::None;
                close = true;
            }
            if close {
                self.delete_dialog = None;
            }
        }

        if let Some((id, comment)) = &mut self.release_dialog {
            let (id, mut done, mut cancel) = (*id, false, false);
            egui::Modal::new(egui::Id::unique("liberar")).show(ctx, |ui| {
                ui.heading("Liberar revisão");
                ui.label("A versão salva mais recente será congelada (somente leitura).");
                ui.add(egui::TextEdit::multiline(comment).hint_text("Comentário da revisão").desired_rows(3));
                ui.horizontal(|ui| {
                    done = ui.button("🔒 Liberar").clicked();
                    cancel = ui.button("Cancelar").clicked();
                });
            });
            if done {
                let c = comment.clone();
                if self.open.entry == Some(id) && self.open.modified {
                    self.save();
                }
                match self.project.as_mut().unwrap().release(id, &c) {
                    Ok(l) => self.toast(format!("Revisão {l} liberada"), false),
                    Err(e) => self.toast(e.to_string(), true),
                }
            }
            if done || cancel {
                self.release_dialog = None;
            }
        }
    }

    fn toasts(&mut self, ctx: &egui::Context) {
        let dt = ctx.input(|i| i.stable_dt).min(0.1) as f64;
        self.toasts.retain_mut(|t| {
            t.1 += dt;
            t.1 < 4.0
        });
        if self.toasts.is_empty() {
            return;
        }
        egui::Area::new(egui::Id::unique("toasts")).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0., -80.)).show(ctx, |ui| {
            for (m, _, err) in &self.toasts {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new(m).color(if *err { Color32::from_rgb(255, 140, 120) } else { Color32::from_rgb(160, 220, 160) }));
                });
            }
        });
        ctx.request_repaint();
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cmd {
    Sketch,
    Extrude,
    Cut,
    Revolve,
    Hole,
    Fillet,
    Chamfer,
    Plane,
    Param,
    Folder,
    Collector,
}

#[derive(Clone, Copy)]
pub enum AppTarget {
    Part,
    Collector(Id),
    Feature(Id),
}

/// Submenu "Aparência" com predefinições, personalizar e remover.
fn appearance_menu(ui: &mut egui::Ui, current: Option<Appearance>, t: AppTarget, actions: &mut Vec<TreeAction>) {
    ui.menu_button("🎨 Aparência", |ui| {
        for (name, a) in APPEARANCE_PRESETS {
            let resp = ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(12., 12.), egui::Sense::hover());
                ui.painter().rect_filled(r, 3., Color32::from_rgb(a.color[0], a.color[1], a.color[2]));
                ui.selectable_label(current == Some(a), name)
            });
            if resp.inner.clicked() {
                actions.push(TreeAction::SetAppearance(t, Some(a)));
                ui.close();
            }
        }
        ui.separator();
        if ui.button("Personalizar cor e acabamento…").clicked() {
            actions.push(TreeAction::Customize(t));
            ui.close();
        }
        if !matches!(t, AppTarget::Collector(_)) && current.is_some() && ui.button("Remover aparência").clicked() {
            actions.push(TreeAction::SetAppearance(t, None));
            ui.close();
        }
    });
}

/// Ícone de olho clicável; devolve true se clicado.
fn eye(ui: &mut egui::Ui, hidden: bool) -> bool {
    let img = egui::Image::new(crate::icons::get(if hidden { "ocultar" } else { "mostrar" })).fit_to_exact_size(egui::vec2(15., 15.));
    let img = if hidden { img.tint(Color32::from_white_alpha(120)) } else { img };
    ui.add(egui::Button::image(img).frame(false)).on_hover_text(if hidden { "mostrar" } else { "ocultar" }).clicked()
}

enum TreeAction {
    Hide(Id, bool),
    RefVisible(Id, bool),
    SetAppearance(AppTarget, Option<Appearance>),
    Customize(AppTarget),
    Align(Id),
    OriginVisible(bool),
    CollectorVisible(Id, bool),
    Activate(Id),
    Suppress(Id),
    Rollback(usize),
    ToFolder(Id, Option<Id>),
    Move(Id, usize),
    Delete(Id),
    SketchOn(PlaneRef),
    Select(Id, Cmd),
}

enum DelAction {
    Replace(Id, Id),
    All(Id),
    Only(Id),
}

/// Marcador de rollback na árvore: os comandos abaixo dele ficam desativados.
fn marker(ui: &mut egui::Ui, at: usize, n: usize, actions: &mut Vec<TreeAction>) {
    ui.horizontal(|ui| {
        let blue = Color32::from_rgb(90, 160, 255);
        ui.label(RichText::new("━━ marcador ━━").small().color(blue))
            .on_hover_text("Comandos abaixo do marcador ficam desativados; novos comandos entram aqui");
        if at > 0 && ui.small_button("▲").on_hover_text("subir o marcador").clicked() {
            actions.push(TreeAction::Rollback(at - 1));
        }
        if at < n && ui.small_button("▼").on_hover_text("descer o marcador").clicked() {
            actions.push(TreeAction::Rollback(at + 1));
        }
        if at < n && ui.small_button("⏬").on_hover_text("marcador no fim").clicked() {
            actions.push(TreeAction::Rollback(n));
        }
    });
}

pub fn fmt(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (name, file) in [("segoe", r"C:\Windows\Fonts\segoeui.ttf"), ("segoe_sym", r"C:\Windows\Fonts\seguisym.ttf")] {
        if let Ok(bytes) = std::fs::read(file) {
            fonts.font_data.insert(name.into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            let fam = fonts.families.entry(egui::FontFamily::Proportional).or_default();
            if name == "segoe" {
                fam.insert(0, name.into());
            } else {
                fam.push(name.into());
            }
            fonts.families.entry(egui::FontFamily::Monospace).or_default().push(name.into());
        }
    }
    ctx.set_fonts(fonts);
}

impl eframe::App for App {
    fn ui(&mut self, root: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        // resultados do recompute
        while let Ok((g, r)) = self.worker.rx.try_recv() {
            if g >= self.gen_done {
                self.gen_done = g;
                self.result = Some(r);
                self.scene_dirty = true;
            }
        }

        // atalhos
        let (save, undo, redo, del) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::S),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y),
                i.key_pressed(egui::Key::Delete),
            )
        });
        if save {
            self.save();
        }
        if undo {
            self.undo();
        }
        if redo {
            self.redo();
        }
        if del && !ctx.egui_wants_keyboard_input() {
            self.delete_selected();
        }
        // atalhos de vista (só quando não há campo de texto em edição)
        if !ctx.egui_wants_keyboard_input() {
            let keys = ctx.input(|i| {
                let c = i.modifiers.command;
                let plain = !c && !i.modifiers.alt;
                [
                    (plain && i.key_pressed(egui::Key::F), Act::Fit),
                    (plain && i.key_pressed(egui::Key::N), Act::Normal),
                    (plain && i.key_pressed(egui::Key::O), Act::Ortho),
                    (c && i.key_pressed(egui::Key::Num1), Act::View(-1.5708, 0.0)),
                    (c && i.key_pressed(egui::Key::Num3), Act::View(0.0, 0.0)),
                    (c && i.key_pressed(egui::Key::Num5), Act::View(-1.5708, 1.5706)),
                    (c && i.key_pressed(egui::Key::Num7), Act::View(-0.785, 0.6)),
                    (plain && i.modifiers.shift && i.key_pressed(egui::Key::H), Act::HiddenEnv),
                ]
            });
            for (pressed, a) in keys {
                if pressed {
                    self.act(a);
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::H) && !i.modifiers.shift && !i.modifiers.command) {
                self.toggle_hide_selected();
            }
        }

        let before = self.open.doc.clone();
        let sel_before = self.sel;
        let settings_before = self.settings.clone();
        let hidden_env_before = self.hidden_env;

        self.top(root);
        self.timeline(root);
        egui::Panel::left("arvore").default_size(270.).resizable(true).show(root, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.left, LeftTab::Peca, "Peça");
                ui.selectable_value(&mut self.left, LeftTab::Projeto, "Projeto");
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| match self.left {
                LeftTab::Peca => self.tree(ui),
                LeftTab::Projeto => self.project_panel(ui),
            });
        });
        egui::Panel::right("editor").default_size(330.).resizable(true).show(root, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| editor::show(self, ui));
        });
        self.side_toolbars(root);
        self.settings_window(ctx);
        self.dialogs(ctx);
        self.viewport(root, frame);
        self.toasts(ctx);

        // edição não destrutiva + desfazer + bloqueio de revisão congelada
        if self.open.doc != before {
            if let Some(rev) = &self.open.frozen {
                let msg = format!("Revisão {rev} está congelada. Abra a versão de trabalho para editar.");
                self.open.doc = before;
                self.toast(msg, true);
            } else {
                let now = ctx.input(|i| i.time);
                if now - self.last_edit > 0.8 {
                    self.undo.push(before);
                    if self.undo.len() > 200 {
                        self.undo.remove(0);
                    }
                }
                self.last_edit = now;
                self.redo.clear();
                self.open.modified = true;
                self.request_recompute();
            }
        }
        if self.sel != sel_before || self.hidden_env != hidden_env_before {
            self.scene_dirty = true;
        }
        if self.settings != settings_before {
            if self.settings.theme != settings_before.theme || self.settings.ui_scale != settings_before.ui_scale {
                self.settings.apply(ctx);
            }
            if let Some(v) = self.viewport.as_mut() {
                v.camera.ortho = self.settings.ortho;
            }
            self.settings.save();
            self.scene_dirty = true;
        }

        if self.scene_dirty {
            if let (Some(vp), Some(r), Some(rs)) = (self.viewport.as_mut(), &self.result, frame.wgpu_render_state()) {
                let lum = |c: [u8; 3]| 0.3 * c[0] as f32 + 0.59 * c[1] as f32 + 0.11 * c[2] as f32;
                let opts = scene::SceneOpts {
                    sel: &self.sel,
                    filter: &self.settings.filter,
                    hidden_env: self.hidden_env,
                    dark_edges: !matches!(self.settings.display, crate::render::DisplayMode::Arame) || lum(self.settings.bg_bottom) > 140.,
                };
                let s = scene::build(&self.open.doc, r, &opts);
                vp.set_scene(rs, &s);
                self.scene_dirty = false;
                ctx.request_repaint();
            }
        }
        // alinha a vista ao sketch recém-criado assim que o resultado dele chega
        if let Some(id) = self.align_pending {
            if self.result.as_ref().is_some_and(|r| r.sketches.iter().any(|s| s.feature == id)) && self.gen_done == self.gen_sent {
                self.align_pending = None;
                let prev = self.sel;
                self.sel = Sel::Feature(id);
                self.align_to_selection();
                self.sel = prev;
            }
        }
    }
}
