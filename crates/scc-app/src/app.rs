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
use eframe::egui::{self, Color32, Rect, RichText};
use glam::Vec3;
use scc_core::*;
use scc_project::{DocState, Project};
use scc_solidos::{RecomputeResult, Status};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use crate::i18n::tr;

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
    /// texturas carregadas na GPU (arquivo, escala)
    tex_key: Vec<(String, f64)>,
    /// janela de automação (macros) e o registro da última execução
    macro_open: bool,
    macro_log: Vec<String>,
    /// janela "Personalizar comandos" e o grupo escolhido nela
    customize_open: bool,
    customize_sel: usize,
    /// janela de importação de publicações e peça escolhida (nome, caminho, publicações)
    link_open: bool,
    link_src: Option<(String, String, Vec<(String, LinkedGeo)>)>,
    /// área de transferência da árvore de geometria: (item, recortado?)
    clip: Option<(Id, bool)>,
    /// reenquadrar a vista (suave) ao abrir um comando
    reframe_pending: bool,
    /// início da tela de carregamento
    splash_t0: Option<f64>,
    /// comando a abrir como ambiente (duplo clique na face)
    open_pending: Option<Id>,
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
    /// sketch em edição interativa
    pub sketch: Option<crate::sketch_ui::SketchEdit>,
    begin_sketch_pending: Option<Id>,
    /// seleção na viewport (faces, arestas, vértices, planos)
    pub picks: Vec<crate::picking::Pick>,
    hover_pick: Option<crate::picking::Pick>,
    /// já passou da tela inicial
    started: bool,
    /// comando aberto em edição (ambiente com Concluir/Cancelar)
    pub session: Option<Session>,
    /// seletor de vistas (tecla Espaço)
    view_menu: bool,
    new_view_name: String,
    /// direção escolhida no cubo, aplicada à câmera no fim do quadro
    view_choice: Option<Vec3>,
}

/// Edição de um comando como "ambiente": só termina com Concluir (mantém) ou Cancelar (desfaz).
pub struct Session {
    pub id: Id,
    pub snapshot: Document,
    pub is_new: bool,
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
            open_pending: None,
            splash_t0: None,
            reframe_pending: false,
            clip: None,
            link_open: false,
            customize_open: false,
            macro_open: false,
            tex_key: vec![],
            macro_log: vec![],
            customize_sel: 0,
            link_src: None,
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
            sketch: None,
            begin_sketch_pending: None,
            picks: vec![],
            hover_pick: None,
            started: false,
            session: None,
            view_menu: false,
            new_view_name: String::new(),
            view_choice: None,
        };
        app.started = !app.settings.show_start;
        // desenvolvimento: SCC_ABRIR_SKETCH=<nome> abre esse sketch em edição ao iniciar
        if let Ok(name) = std::env::var("SCC_ABRIR_SKETCH") {
            app.begin_sketch_pending = app.open.doc.features.iter().find(|f| f.name == name).map(|f| f.id);
            app.started = true;
        }
        // desenvolvimento: SCC_EDITAR=<nome> abre esse comando como ambiente ao iniciar
        if let Ok(name) = std::env::var("SCC_EDITAR") {
            if let Some(id) = app.open.doc.features.iter().find(|f| f.name == name).map(|f| f.id) {
                app.start_session(id, false, None);
                app.started = true;
            }
        }
        // desenvolvimento: SCC_TEXTURA=<imagem> aplica a imagem como textura na peça
        if let Ok(p) = std::env::var("SCC_TEXTURA") {
            app.open.doc.textures.push(TextureRef { name: "teste".into(), path: p, scale: 40. });
            app.open.doc.appearance = Some(Appearance { color: [255, 255, 255], finish: Finish::Acetinado, texture: Some(0) });
        }
        // desenvolvimento: SCC_EN=1 mostra a interface em inglês (sem gravar a configuração)
        if std::env::var("SCC_EN").is_ok() {
            app.settings.english = true;
        }
        // desenvolvimento: SCC_AJUSTES=1 abre as configurações ao iniciar
        if std::env::var("SCC_AJUSTES").is_ok() {
            app.settings_open = true;
            app.started = true;
        }
        // desenvolvimento: SCC_VISTAS=1 abre o seletor de vistas ao iniciar
        if std::env::var("SCC_VISTAS").is_ok() {
            app.view_menu = true;
            app.started = true;
        }
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
                self.settings.add_recent(&dir.to_string_lossy());
                self.project = Some(p);
                self.left = LeftTab::Projeto;
                self.started = true;
                self.toast(format!("Projeto '{name}' criado"), false);
            }
            Err(e) => self.toast(e.to_string(), true),
        }
    }

    fn open_project(&mut self) {
        let Some(dir) = rfd::FileDialog::new().set_title("Pasta do projeto").pick_folder() else { return };
        self.open_project_at(&dir);
    }

    fn open_project_at(&mut self, dir: &std::path::Path) {
        match Project::open(dir) {
            Ok(p) => {
                self.settings.add_recent(&dir.to_string_lossy());
                self.project = Some(p);
                self.left = LeftTab::Projeto;
                self.started = true;
            }
            Err(e) => self.toast(format!("Não é um projeto SolidCadCraft: {e}"), true),
        }
    }

    /// Tela inicial: escolher projeto/peça antes de cair no modelo.
    /// Tela de carregamento: fênix, nome e barra de progresso até o kernel e o primeiro cálculo ficarem prontos.
    fn splash(&mut self, root: &mut egui::Ui) -> bool {
        let now = root.ctx().input(|i| i.time);
        let t0 = *self.splash_t0.get_or_insert(now);
        let el = (now - t0) as f32;
        let ready = self.gen_done >= 1 || self.gen_done == self.gen_sent;
        let min_time = 1.3;
        let p = if ready { (el / min_time).min(1.) } else { (el / min_time).min(0.85) };
        if ready && el >= min_time {
            return false;
        }
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(root, |ui| {
            let rect = ui.max_rect();
            paint_backdrop(ui.painter(), rect, true);
            let c = rect.center();
            let img = egui::Image::new(egui::include_image!("../../../assets/app/fenix-256.png")).corner_radius(28.);
            img.paint_at(ui, Rect::from_center_size(c - egui::vec2(0., 70.), egui::vec2(170., 170.)));
            ui.painter().text(c + egui::vec2(0., 40.), egui::Align2::CENTER_CENTER, "SolidCadCraft", egui::FontId::proportional(32.), Color32::WHITE);
            ui.painter().text(c + egui::vec2(0., 70.), egui::Align2::CENTER_CENTER, concat!("CAD paramétrico open source · ", env!("CARGO_PKG_VERSION")), egui::FontId::proportional(13.), Color32::from_white_alpha(150));
            let bar = Rect::from_center_size(c + egui::vec2(0., 110.), egui::vec2(320., 6.));
            ui.painter().rect_filled(bar, 3., Color32::from_white_alpha(30));
            let mut fill = bar;
            fill.set_width(bar.width() * p);
            ui.painter().rect_filled(fill, 3., Color32::from_rgb(90, 170, 255));
            let stage = if p < 0.3 { "Iniciando o kernel OpenCASCADE…" } else if p < 0.6 { "Carregando ferramentas e ícones…" } else if p < 0.95 { "Calculando a peça…" } else { "Pronto" };
            ui.painter().text(bar.center_bottom() + egui::vec2(0., 16.), egui::Align2::CENTER_CENTER, stage, egui::FontId::proportional(12.), Color32::from_white_alpha(140));
        });
        root.ctx().request_repaint();
        true
    }

    fn start_screen(&mut self, root: &mut egui::Ui) {
        let mut action: Option<u8> = None;
        let mut recent_open: Option<String> = None;
        let dark = !self.light_background();
        egui::CentralPanel::default().frame(egui::Frame::NONE.inner_margin(12.)).show(root, |ui| {
            paint_backdrop(ui.painter(), ui.max_rect().expand(12.), dark);
            ui.vertical_centered(|ui| {
                ui.add_space(40.);
                ui.add(egui::Image::new(egui::include_image!("../../../assets/app/fenix-256.png")).fit_to_exact_size(egui::vec2(128., 128.)));
                ui.label(RichText::new(tr("SolidCadCraft")).size(30.).strong());
                ui.label(RichText::new(concat!("CAD paramétrico open source · ", env!("CARGO_PKG_VERSION"))).weak());
                ui.add_space(24.);
            });
            let w = 720.0f32.min(ui.available_width());
            ui.horizontal(|ui| {
                ui.add_space(((ui.available_width() - w) / 2.).max(0.));
                ui.vertical(|ui| {
                    ui.set_width(w);
                    ui.label(RichText::new(tr("Começar")).strong());
                    ui.horizontal_wrapped(|ui| {
                        let big = |ui: &mut egui::Ui, icon: &str, name: &str, brief: &str| crate::icons::tool_button(ui, icon, name, 54., true, false, brief, "");
                        if big(ui, "novo_projeto", "Novo projeto", "Cria uma pasta de projeto com versões e revisões").clicked() {
                            action = Some(0);
                        }
                        if big(ui, "abrir_projeto", "Abrir projeto", "Abre uma pasta de projeto existente").clicked() {
                            action = Some(1);
                        }
                        if big(ui, "nova_peca", "Nova peça", "Peça avulsa, fora de projeto").clicked() {
                            action = Some(2);
                        }
                        if big(ui, "abrir_projeto", "Abrir peça", "Abre um arquivo .sccpart").clicked() {
                            action = Some(3);
                        }
                        if big(ui, "exemplo", "Exemplo", "Placa paramétrica de exemplo").clicked() {
                            action = Some(4);
                        }
                    });
                    ui.add_space(16.);
                    ui.label(RichText::new(tr("Projetos recentes")).strong());
                    if self.settings.recent.is_empty() {
                        ui.label(RichText::new(tr("nenhum ainda")).weak());
                    }
                    for r in &self.settings.recent {
                        let name = std::path::Path::new(r).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(r.clone());
                        let resp = ui.horizontal(|ui| {
                            ui.add(egui::Image::new(crate::icons::get("pasta")).fit_to_exact_size(egui::vec2(20., 20.)));
                            let l = ui.link(RichText::new(name).strong());
                            ui.label(RichText::new(r).weak().small());
                            l
                        });
                        if resp.inner.clicked() {
                            recent_open = Some(r.clone());
                        }
                    }
                    ui.add_space(16.);
                    ui.checkbox(&mut self.settings.show_start, tr("mostrar esta tela ao iniciar"));
                });
            });
        });
        if let Some(r) = recent_open {
            self.open_project_at(std::path::Path::new(&r));
        }
        match action {
            Some(0) => self.new_project(),
            Some(1) => self.open_project(),
            Some(2) => {
                self.load_doc(Document::new("Peça"), None, None);
                self.started = true;
            }
            Some(3) => {
                self.open_file();
                self.started = true;
            }
            Some(4) => {
                self.load_doc(scc_solidos::example_document(), None, None);
                self.started = true;
            }
            _ => {}
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
    /// Plano para um novo sketch/plano: o plano ou a face clicada na viewport, senão o da árvore.
    fn plane_for_new(&mut self) -> PlaneRef {
        use crate::picking::Pick;
        match self.picks.first().copied() {
            Some(Pick::Plane(p)) => p,
            Some(Pick::Face { point, normal, owner, .. }) => {
                let n = normal.map(|v| v as f64);
                let helper = if n[2].abs() < 0.9 { [0., 0., 1.] } else { [1., 0., 0.] };
                // x = helper × n (no plano da face)
                let x = scc_kernel::cross(helper, n);
                let l = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt().max(1e-12);
                let x = x.map(|v| v / l);
                let id = self.open.doc.add_face_plane(point.map(|v| v as f64), x, n, Some(owner));
                PlaneRef::Datum(id)
            }
            _ => self.selected_plane(),
        }
    }

    /// Pontos clicados nas faces selecionadas na viewport.
    pub fn picked_faces(&self) -> Vec<[f64; 3]> {
        self.picks
            .iter()
            .filter_map(|p| if let crate::picking::Pick::Face { point, .. } = p { Some(point.map(|v| v as f64)) } else { None })
            .collect()
    }

    /// Pontos médios das arestas selecionadas na viewport.
    pub fn picked_edges(&self) -> Vec<[f64; 3]> {
        self.picks
            .iter()
            .filter_map(|p| if let crate::picking::Pick::Edge { mid, .. } = p { Some(mid.map(|v| v as f64)) } else { None })
            .collect()
    }

    /// Entradas para geometria de referência a partir da seleção (viewport e, se `with_sel`, árvore).
    pub fn seed(&self, with_sel: bool) -> crate::geo_ui::Seed {
        use crate::picking::Pick;
        let mut s = crate::geo_ui::Seed::default();
        let d = self.doc();
        let add_ref = |s: &mut crate::geo_ui::Seed, id: Id| match d.reference(id).map(|r| r.kind.class()) {
            Some(RefClass::Plane) => s.planes.push(PlaneRef::Datum(id)),
            Some(RefClass::Point) => s.points.push(PointRef::Ref(id)),
            Some(RefClass::Curve) => s.curves.push(CurveRef::Ref(id)),
            Some(RefClass::Surface) => s.surface = Some(SurfRef::Ref(id)),
            None => {}
        };
        if with_sel {
            match self.sel {
                Sel::Origin(o) => s.planes.push(PlaneRef::Origin(o)),
                Sel::Ref(id) => add_ref(&mut s, id),
                _ => {}
            }
        }
        for p in &self.picks {
            match *p {
                Pick::Vertex { pos, .. } => s.points.push(PointRef::Fixed(pos.map(|v| v as f64))),
                Pick::Edge { body, edge, .. } => {
                    if let Some(r) = &self.result {
                        let pts = crate::picking::edge_polyline(r, body, edge);
                        if pts.len() >= 2 {
                            s.curves.push(CurveRef::Edge(pts));
                        }
                    }
                }
                Pick::Face { point, normal, owner, .. } => {
                    s.surface = Some(SurfRef::Face { point: point.map(|v| v as f64), normal: normal.map(|v| v as f64), owner: Some(owner) })
                }
                Pick::Plane(pr) => s.planes.push(pr),
                Pick::Geo(id) => add_ref(&mut s, id),
            }
        }
        s
    }

    fn selected_plane(&self) -> PlaneRef {
        match self.sel {
            Sel::Origin(o) => PlaneRef::Origin(o),
            Sel::Ref(id) if self.doc().reference(id).is_some_and(|r| r.kind.class() == RefClass::Plane) => PlaneRef::Datum(id),
            _ => PlaneRef::Origin(OriginPlane::XY),
        }
    }

    /// Sketch para um novo comando: o selecionado, ou o último antes do marcador.
    fn selected_sketch(&self, need_points: bool) -> Option<Id> {
        let d = self.doc();
        let ok = |f: &Feature| match &f.kind {
            FeatureKind::Sketch(s) => !need_points || !s.data.hole_points().is_empty(),
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
        self.conclude_session();
        let snapshot = self.open.doc.clone();
        let sel_now = self.sel;
        let s = |v: &str| v.to_string();
        let uses_plane = matches!(c, Cmd::Sketch | Cmd::Hole | Cmd::HoleCounterbore | Cmd::HoleCountersink | Cmd::HoleTapped | Cmd::Prim(_));
        let plane = if uses_plane { self.plane_for_new() } else { self.selected_plane() };
        let sketch = self.selected_sketch(false);
        let edges = self.picked_edges();
        let faces = self.picked_faces();
        let seed = match c {
            Cmd::Rib | Cmd::Slot | Cmd::PatternLinear | Cmd::PatternCircular | Cmd::PatternPoints => self.seed(true),
            Cmd::Geo(k) => {
                let mut s = self.seed(true);
                // face clicada para um tipo que usa plano: vira um plano da face
                if k.uses_plane() && s.planes.is_empty() && s.surface.is_some() {
                    s.planes.push(self.plane_for_new());
                }
                s
            }
            _ => Default::default(),
        };
        let up_to = self.open.doc.references.last().map(|r| PlaneRef::Datum(r.id)).unwrap_or(PlaneRef::Origin(OriginPlane::XY));
        let d = &mut self.open.doc;
        let new_sel = match c {
            Cmd::Sketch => {
                let geo = match sel_now {
                    Sel::Folder(fo) if d.folder(fo).is_some_and(|x| x.kind == FolderKind::Geometria) => Some(fo),
                    _ => None,
                };
                let id = match geo {
                    Some(fo) => d.add_free_sketch(plane, fo),
                    None => d.add_feature(FeatureKind::Sketch(Sketch { plane, data: Default::default() })),
                };
                self.begin_sketch_pending = Some(id);
                Sel::Feature(id)
            }
            Cmd::Extrude | Cmd::ExtrudeMid | Cmd::ExtrudeUpTo | Cmd::Cut | Cmd::CutBlind | Cmd::CutUpTo => {
                let Some(sk) = sketch else {
                    self.toast("Crie ou selecione um sketch primeiro", true);
                    return;
                };
                let cut = matches!(c, Cmd::Cut | Cmd::CutBlind | Cmd::CutUpTo);
                let extent = match c {
                    Cmd::Extrude => Extent::Blind(s("20")),
                    Cmd::ExtrudeMid => Extent::Symmetric(s("20")),
                    Cmd::CutBlind => Extent::Blind(s("5")),
                    Cmd::ExtrudeUpTo | Cmd::CutUpTo => Extent::UpTo(up_to),
                    _ => Extent::ThroughAll,
                };
                Sel::Feature(d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: if cut { BoolOp::Cut } else { BoolOp::Add }, extent, reverse: false, taper: s("0"), thin: None })))
            }
            Cmd::Revolve | Cmd::Groove => {
                let Some(sk) = sketch else {
                    self.toast("Crie ou selecione um sketch primeiro", true);
                    return;
                };
                Sel::Feature(d.add_feature(FeatureKind::Revolve(Revolve {
                    sketch: sk,
                    axis: AxisRef::Origin(OriginAxis::Y),
                    angle: s("360"),
                    symmetric: false,
                    op: if c == Cmd::Groove { BoolOp::Cut } else { BoolOp::Add },
                    thin: None,
                })))
            }
            Cmd::Rib | Cmd::Slot => {
                let Some(sk) = sketch else {
                    self.toast("Crie o sketch do perfil primeiro (e uma curva guia em Referências → Curva)", true);
                    return;
                };
                let spine = seed
                    .curves
                    .first()
                    .cloned()
                    .or_else(|| d.references.iter().rev().find(|r| r.kind.class() == RefClass::Curve).map(|r| CurveRef::Ref(r.id)))
                    .unwrap_or(CurveRef::Axis(OriginAxis::X));
                Sel::Feature(d.add_feature(FeatureKind::Sweep(Sweep {
                    sketch: sk,
                    spine,
                    mode: SweepMode::Corrigido,
                    op: if c == Cmd::Slot { BoolOp::Cut } else { BoolOp::Add },
                    thin: None,
                })))
            }
            Cmd::Loft | Cmd::LoftCut => {
                // os dois últimos sketches antes do marcador (o selecionado entra por último)
                let mut secs: Vec<Id> = d.features[..d.rollback.min(d.features.len())].iter().filter(|f| matches!(f.kind, FeatureKind::Sketch(_))).map(|f| f.id).collect();
                if let Some(sk) = sketch {
                    secs.retain(|x| *x != sk);
                    secs.push(sk);
                }
                let n = secs.len();
                let sections = secs[n.saturating_sub(2)..].to_vec();
                if sections.len() < 2 {
                    self.toast("Multi-seções precisa de pelo menos dois sketches (em planos diferentes)", true);
                    return;
                }
                Sel::Feature(d.add_feature(FeatureKind::Loft(Loft { sections, ruled: false, closed: false, op: if c == Cmd::LoftCut { BoolOp::Cut } else { BoolOp::Add } })))
            }
            Cmd::Prim(kind) => {
                let v: [&str; 4] = match kind {
                    PrimKind::Caixa => ["40", "30", "20", "0"],
                    PrimKind::Cilindro => ["10", "30", "0", "0"],
                    PrimKind::Esfera => ["15", "0", "0", "0"],
                    PrimKind::Cone => ["15", "5", "30", "0"],
                    PrimKind::Toro => ["20", "5", "0", "0"],
                    PrimKind::Cunha => ["40", "30", "20", "10"],
                    PrimKind::Prisma => ["6", "15", "20", "0"],
                };
                Sel::Feature(d.add_feature(FeatureKind::Primitive(Primitive { kind, plane, a: s(v[0]), b: s(v[1]), c: s(v[2]), d: s(v[3]), op: BoolOp::Add })))
            }
            Cmd::Stiffener => {
                let Some(sk) = sketch else {
                    self.toast("Desenhe as linhas do reforço num sketch primeiro", true);
                    return;
                };
                Sel::Feature(d.add_feature(FeatureKind::Stiffener(Stiffener { sketch: sk, thickness: s("3"), depth: s("20"), reverse: false, to_body: false })))
            }
            Cmd::Shell => Sel::Feature(d.add_feature(FeatureKind::Shell(Shell { faces: faces.clone(), thickness: s("2"), outward: false }))),
            Cmd::Draft => Sel::Feature(d.add_feature(FeatureKind::Draft(Draft { faces: faces.clone(), neutral: plane, angle: s("3"), reverse: false }))),
            Cmd::Thickness => Sel::Feature(d.add_feature(FeatureKind::Thickness(Thickness { faces: faces.clone(), value: s("2") }))),
            Cmd::Thread => Sel::Feature(d.add_feature(FeatureKind::Thread(Thread { face: faces.last().copied().unwrap_or([0.; 3]), spec: s("M10x1.5"), length: s("15"), modeled: false }))),
            Cmd::PatternLinear | Cmd::PatternCircular | Cmd::PatternPoints | Cmd::Mirror => {
                let sources: Vec<Id> = match sel_now {
                    Sel::Feature(id) if d.feature(id).is_some_and(|f| crate::feat_ui::is_shape(&f.kind)) => vec![id],
                    _ => vec![],
                };
                let kind = match c {
                    Cmd::PatternLinear => Some(0),
                    Cmd::PatternCircular => Some(1),
                    Cmd::PatternPoints => Some(2),
                    _ => None,
                };
                Sel::Feature(d.add_feature(match kind {
                    Some(i) => FeatureKind::Pattern(Pattern { kind: crate::feat_ui::default_pattern(i, &seed), sources }),
                    None => FeatureKind::Mirror(Mirror { plane: if plane == PlaneRef::Origin(OriginPlane::XY) { PlaneRef::Origin(OriginPlane::YZ) } else { plane }, sources }),
                }))
            }
            Cmd::Scale => Sel::Feature(d.add_feature(FeatureKind::Scale(Scale { center: PointRef::Origin, factor: s("2") }))),
            Cmd::Combine(op) => {
                let active = d.active_collector;
                let Some(tool) = d.collectors.iter().find(|c| c.id != active).map(|c| c.id) else {
                    self.toast("Crie outro coletor de corpos com o corpo ferramenta primeiro", true);
                    return;
                };
                Sel::Feature(d.add_feature(FeatureKind::Combine(Combine { tool, op, keep_tool: false })))
            }
            Cmd::FilletFaceFace | Cmd::FilletFull => {
                let f = |k: usize| faces.get(k).copied().unwrap_or([0.; 3]);
                let mode = if c == Cmd::FilletFull { EdgeMode::FullRound { mid: f(0), a: f(1), b: f(2) } } else { EdgeMode::FaceFace(f(0), f(1)) };
                Sel::Feature(d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("2"), edges: EdgeSel::AllOfBody, mode })))
            }
            Cmd::Hole | Cmd::HoleCounterbore | Cmd::HoleCountersink | Cmd::HoleTapped => {
                let sk = match self.selected_sketch(true) {
                    Some(sk) => sk,
                    None => {
                        let d = &mut self.open.doc;
                        let mut data = SketchData::default();
                        {
                            let mut al = || d.alloc();
                            let p = data.add_point(&mut al, 0., 0.);
                            data.add_curve(&mut al, scc_core::sketch::Curve::Point { p }, false);
                        }
                        let id = d.add_feature(FeatureKind::Sketch(Sketch { plane, data }));
                        d.feature_mut(id).unwrap().name = d.unique_name("Centros");
                        id
                    }
                };
                let d = &mut self.open.doc;
                let m = scc_solidos::hardware::metric("M6").unwrap();
                let (kind, diameter, thread, std) = match c {
                    Cmd::HoleCounterbore => (
                        HoleKind::Rebaixado { diameter: m.cbore_diameter.to_string(), depth: m.cbore_depth.to_string() },
                        m.clearance[1],
                        None,
                        "M6 passagem normal",
                    ),
                    Cmd::HoleCountersink => (HoleKind::Escareado { diameter: m.csink_diameter.to_string(), angle: s("90") }, m.clearance[1], None, "M6 passagem normal"),
                    Cmd::HoleTapped => (HoleKind::Simples, m.tap_drill, Some(format!("M6x{}", m.pitch)), "M6 roscado"),
                    _ => (HoleKind::Simples, m.clearance[1], None, "M6 passagem normal"),
                };
                Sel::Feature(d.add_feature(FeatureKind::Hole(Hole {
                    sketch: sk,
                    kind,
                    diameter: diameter.to_string(),
                    depth: Extent::ThroughAll,
                    thread,
                    thread_modeled: false,
                    standard: Some(s(std)),
                })))
            }
            Cmd::Fillet | Cmd::FilletAll | Cmd::Chamfer | Cmd::ChamferAll | Cmd::FilletVar | Cmd::ChamferTwoDist | Cmd::ChamferDistAngle => {
                let all = matches!(c, Cmd::FilletAll | Cmd::ChamferAll);
                let mode = match c {
                    Cmd::FilletVar => EdgeMode::Variable(s("3")),
                    Cmd::ChamferTwoDist => EdgeMode::TwoDist(s("2")),
                    Cmd::ChamferDistAngle => EdgeMode::DistAngle(s("45")),
                    _ => EdgeMode::Constant,
                };
                let sel = if all { EdgeSel::AllOfBody } else { EdgeSel::Edges(edges.clone()) };
                if !all && edges.is_empty() {
                    self.toast("Clique nas arestas na viewport para pô-las no comando (clique de novo tira)", false);
                }
                let op = EdgeOp { value: s("1"), edges: sel, mode };
                let d = &mut self.open.doc;
                Sel::Feature(d.add_feature(if matches!(c, Cmd::Fillet | Cmd::FilletAll | Cmd::FilletVar) { FeatureKind::Fillet(op) } else { FeatureKind::Chamfer(op) }))
            }
            Cmd::Geo(k) => Sel::Ref(d.add_geometry(k.make(&seed))),
            Cmd::Param => {
                let n = (1..).map(|i| format!("P{i}")).find(|n| !d.params.iter().any(|p| &p.name == n)).unwrap();
                let _ = d.add_param(&n, "10", Unit::Comprimento);
                Sel::Params
            }
            Cmd::Folder => {
                let n = format!("Geometria {}", d.folders.iter().filter(|f| f.kind == FolderKind::Geometria).count() + 1);
                Sel::Folder(d.add_geometry_folder(&n))
            }
            Cmd::Collector => {
                let n = format!("Corpo {}", d.collectors.len() + 1);
                let id = d.add_collector(&n);
                d.active_collector = id;
                Sel::Collector(id)
            }
        };
        self.picks.clear();
        match new_sel {
            Sel::Feature(id) if !scene::is_sketch_feature(&self.open.doc, id) => self.start_session(id, true, Some(snapshot)),
            Sel::Ref(id) => self.start_session(id, true, Some(snapshot)),
            _ => {}
        }
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
        if self.session.is_none() {
            self.sel = new_sel;
        }
    }

    /// Executa uma ação da barra de ferramentas.
    fn act(&mut self, a: Act) {
        match a {
            Act::Cmd(c) => self.command(c),
            Act::Fit => self.fit_pending = true,
            Act::Customize => self.customize_open = true,
            Act::Macros => self.macro_open = true,
            Act::View(y, p) => {
                if let Some(v) = self.viewport.as_mut() {
                    let before = v.camera;
                    v.camera.set_view(y, p);
                    v.animate_from(before);
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
            Act::SkTool(t) => {
                if let Some(e) = self.sketch.as_mut() {
                    e.set_tool(t)
                }
            }
            Act::SkConstruction => {
                if let Some(e) = self.sketch.as_mut() {
                    e.construction = !e.construction
                }
            }
            Act::SkGrid => self.settings.grid = !self.settings.grid,
            Act::SkSnap => self.settings.snaps = !self.settings.snaps,
            Act::SkAutoConstraints => self.settings.auto_constraints = !self.settings.auto_constraints,
            Act::SkAutoDims => self.settings.auto_dims = !self.settings.auto_dims,
            Act::SkFinish => self.finish_sketch(),
            Act::ClearPicks => self.picks.clear(),
            Act::SkCheck => {
                if let Some(e) = self.sketch.as_mut() {
                    e.check = !e.check
                }
            }
            Act::SkToggleConstruction => {
                let Some(e) = self.sketch.as_ref() else { return };
                let (id, sel) = (e.feature, e.sel.clone());
                if let Some(FeatureKind::Sketch(sk)) = self.open.doc.feature_mut(id).map(|f| &mut f.kind) {
                    for c in &mut sk.data.curves {
                        if sel.contains(&crate::sketch_ui::Ent::Cv(c.id)) {
                            c.construction = !c.construction;
                        }
                    }
                }
            }
        }
    }

    /// Abre um comando (ou plano) para edição como ambiente.
    pub fn start_session(&mut self, id: Id, is_new: bool, snapshot: Option<Document>) {
        if self.session.as_ref().is_some_and(|s| s.id == id) {
            return;
        }
        self.conclude_session();
        if scene::is_sketch_feature(&self.open.doc, id) {
            self.begin_sketch(id);
            return;
        }
        let snapshot = snapshot.unwrap_or_else(|| self.open.doc.clone());
        self.session = Some(Session { id, snapshot, is_new });
        self.reframe_pending = true;
        self.sel = if self.open.doc.reference(id).is_some() { Sel::Ref(id) } else { Sel::Feature(id) };
    }

    /// Concluir: mantém as alterações.
    pub fn conclude_session(&mut self) {
        self.session = None;
    }

    /// Cancelar: volta ao estado de antes de abrir o comando (um comando novo some).
    pub fn cancel_session(&mut self) {
        if let Some(s) = self.session.take() {
            self.open.doc = s.snapshot;
            if s.is_new {
                self.sel = Sel::None;
            }
            self.request_recompute();
        }
    }

    /// Fundo claro? (para escolher cores legíveis)
    pub fn light_background(&self) -> bool {
        let lum = |c: [u8; 3]| 0.3 * c[0] as f32 + 0.59 * c[1] as f32 + 0.11 * c[2] as f32;
        lum(self.settings.bg_bottom) > 140.
    }

    /// Entra na edição interativa de um sketch: alinha a vista ao plano e liga a projeção ortogonal.
    pub fn begin_sketch(&mut self, id: Id) {
        if self.sketch.as_ref().is_some_and(|e| e.feature == id) {
            return;
        }
        self.finish_sketch();
        crate::sketch_ui::begin(self, id);
        if let Some(v) = self.viewport.as_mut() {
            v.camera.ortho = true;
        }
        self.align_pending = Some(id);
        self.scene_dirty = true;
    }

    /// Sai da edição do sketch e restaura a projeção.
    pub fn finish_sketch(&mut self) {
        if let Some(e) = self.sketch.take() {
            if let Some(v) = self.viewport.as_mut() {
                v.camera.ortho = e.prev_ortho;
            }
            self.scene_dirty = true;
        }
    }

    fn tool_state(&self) -> toolbar::ToolState {
        toolbar::ToolState {
            hidden_env: self.hidden_env,
            sketch_tool: self.sketch.as_ref().map(|e| e.tool),
            construction: self.sketch.as_ref().is_some_and(|e| e.construction),
            check: self.sketch.as_ref().is_some_and(|e| e.check),
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
                let before = v.camera;
                v.camera.look_along(Vec3::from(f.normal.map(|x| x as f32)), c);
                v.animate_from(before);
            }
            _ => self.toast("Selecione um sketch ou plano para alinhar a vista", true),
        }
    }

    /// Janela de automação: escreve e executa macros (ver `scc_core::macros`).
    fn macro_window(&mut self, ctx: &egui::Context) {
        if !self.macro_open {
            return;
        }
        let mut open = true;
        let mut run = false;
        egui::Window::new(tr("⚡ Automação (macros)")).open(&mut open).default_width(560.).default_height(520.).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(RichText::new(tr("▶ Executar")).strong()).clicked() {
                    run = true;
                }
                if ui.button(tr("Exemplo")).clicked() {
                    self.settings.macro_src = scc_core::macros::EXAMPLE.to_string();
                }
                if ui.button(tr("📂 Abrir…")).clicked() {
                    if let Some(p) = rfd::FileDialog::new().add_filter("Macro", &["sccmacro", "txt"]).pick_file() {
                        match std::fs::read_to_string(&p) {
                            Ok(s) => self.settings.macro_src = s,
                            Err(e) => self.macro_log = vec![format!("⚠ {e}")],
                        }
                    }
                }
                if ui.button(tr("💾 Salvar…")).clicked() {
                    if let Some(p) = rfd::FileDialog::new().add_filter("Macro", &["sccmacro"]).set_file_name("macro.sccmacro").save_file() {
                        if let Err(e) = std::fs::write(&p, &self.settings.macro_src) {
                            self.macro_log = vec![format!("⚠ {e}")];
                        }
                    }
                }
            });
            egui::CollapsingHeader::new(tr("Comandos da macro")).default_open(false).show(ui, |ui| {
                ui.label(
                    RichText::new(
                        "param NOME = expr · plano NOME deslocado BASE DIST · sketch NOME em PLANO … fim\n\
                         retangulo x1 y1 x2 y2 · circulo cx cy r · linha x1 y1 x2 y2 · ponto x y\n\
                         extrusao NOME SKETCH DIST · recorte NOME SKETCH DIST|passante · furo NOME SKETCH Ø PROF|passante\n\
                         filete todas R · chanfro todas D · padrao linear|circular NOME COMANDO EIXO N ESP|ÂNGULO\n\
                         espelho NOME COMANDO PLANO · para i de A a B … fim · # comentário\n\
                         Números aceitam expressões com parâmetros e variáveis de laço: (L/2), (i*20). Também em inglês: plane, rect, circle, extrude, cut, hole, for, end…",
                    )
                    .monospace()
                    .small(),
                );
            });
            egui::ScrollArea::vertical().max_height(320.).id_salt("macro_src").show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut self.settings.macro_src).code_editor().desired_rows(16).desired_width(f32::INFINITY));
            });
            ui.separator();
            for l in &self.macro_log {
                let c = if l.starts_with('⚠') { Color32::from_rgb(255, 140, 120) } else { ui.visuals().text_color() };
                ui.label(RichText::new(l).color(c).small());
            }
        });
        if run {
            let src = self.settings.macro_src.clone();
            match scc_core::macros::run(&mut self.open.doc, &src) {
                Ok(log) => {
                    self.macro_log = log;
                    self.macro_log.push(format!("✔ {}", tr("macro executada")));
                }
                Err(e) => self.macro_log = vec![format!("⚠ {e}")],
            }
        }
        if !open {
            self.macro_open = false;
            self.settings.save();
        }
    }

    /// Janela de personalização: esconder e reordenar comandos dos grupos e montar grupos próprios.
    fn customize_window(&mut self, ctx: &egui::Context) {
        use crate::settings::{CustomGroup, Group};
        use crate::toolbar::{all_entries, entries, entry_key, entry_title};
        if !self.customize_open {
            return;
        }
        let mut open = true;
        let groups: Vec<Group> = Group::ALL.iter().copied().filter(|g| *g != Group::SkRestricoes).collect();
        let n_std = groups.len();
        let c = &mut self.settings.custom;
        let sel = &mut self.customize_sel;
        egui::Window::new(tr("Personalizar comandos")).open(&mut open).default_width(520.).show(ctx, |ui| {
            ui.columns(2, |cols| {
                let ui = &mut cols[0];
                ui.label(RichText::new(tr("Grupos")).strong());
                for (i, g) in groups.iter().enumerate() {
                    ui.selectable_value(sel, i, g.label());
                }
                ui.separator();
                ui.label(RichText::new(tr("Meus grupos")).strong());
                for (i, cg) in c.groups.iter().enumerate() {
                    ui.selectable_value(sel, n_std + i, format!("★ {}", cg.name));
                }
                if ui.button(tr("+ Novo grupo")).clicked() {
                    c.groups.push(CustomGroup { name: format!("{} {}", tr("Meu grupo"), c.groups.len() + 1), items: vec![], sketch: false });
                    *sel = n_std + c.groups.len() - 1;
                }
                ui.add_space(8.);
                if ui.button(tr("Restaurar padrões")).clicked() {
                    *c = Default::default();
                    *sel = 0;
                }
                let ui = &mut cols[1];
                if *sel < n_std {
                    let g = groups[*sel];
                    ui.label(RichText::new(g.label()).strong());
                    ui.label(RichText::new(tr("Marque o que aparece na barra; ▲▼ muda a ordem.")).weak().small());
                    // ordem atual (inclui as ocultas)
                    let mut keys: Vec<String> = entries(g).iter().map(entry_key).collect();
                    if let Some((_, order)) = c.order.iter().find(|(x, _)| *x == g) {
                        keys.sort_by_key(|k| order.iter().position(|o| o == k).unwrap_or(usize::MAX));
                    }
                    let all = entries(g);
                    let mut mv: Option<(usize, isize)> = None;
                    for (i, k) in keys.iter().enumerate() {
                        let Some(e) = all.iter().find(|e| entry_key(e) == *k) else { continue };
                        let (icon, brief) = entry_title(e);
                        ui.horizontal(|ui| {
                            let mut on = !c.hidden.contains(k);
                            if ui.checkbox(&mut on, "").changed() {
                                if on {
                                    c.hidden.retain(|x| x != k);
                                } else {
                                    c.hidden.push(k.clone());
                                }
                            }
                            ui.add(egui::Image::new(crate::icons::get(icon)).fit_to_exact_size(egui::vec2(20., 20.)));
                            ui.label(tr(brief));
                            if ui.small_button("▲").clicked() {
                                mv = Some((i, -1));
                            }
                            if ui.small_button("▼").clicked() {
                                mv = Some((i, 1));
                            }
                        });
                    }
                    if let Some((i, d)) = mv {
                        let j = i as isize + d;
                        if j >= 0 && (j as usize) < keys.len() {
                            keys.swap(i, j as usize);
                            c.order.retain(|(x, _)| *x != g);
                            c.order.push((g, keys));
                        }
                    }
                } else if let Some(idx) = Some(*sel - n_std).filter(|i| *i < c.groups.len()) {
                    let all = all_entries();
                    let mut remove_group = false;
                    {
                        let cg = &mut c.groups[idx];
                        ui.horizontal(|ui| {
                            ui.label(tr("Nome"));
                            ui.text_edit_singleline(&mut cg.name);
                        });
                        ui.checkbox(&mut cg.sketch, tr("aparece no modo sketch"));
                        let mut mv: Option<(usize, isize)> = None;
                        let mut del = None;
                        for (i, k) in cg.items.iter().enumerate() {
                            let Some(e) = all.iter().find(|e| entry_key(e) == *k) else { continue };
                            let (icon, brief) = entry_title(e);
                            ui.horizontal(|ui| {
                                ui.add(egui::Image::new(crate::icons::get(icon)).fit_to_exact_size(egui::vec2(20., 20.)));
                                ui.label(tr(brief));
                                if ui.small_button("▲").clicked() {
                                    mv = Some((i, -1));
                                }
                                if ui.small_button("▼").clicked() {
                                    mv = Some((i, 1));
                                }
                                if ui.small_button("✖").clicked() {
                                    del = Some(i);
                                }
                            });
                        }
                        if let Some((i, d)) = mv {
                            let j = i as isize + d;
                            if j >= 0 && (j as usize) < cg.items.len() {
                                cg.items.swap(i, j as usize);
                            }
                        }
                        if let Some(i) = del {
                            cg.items.remove(i);
                        }
                        egui::ComboBox::from_id_salt("add_cmd").selected_text(tr("+ Adicionar comando")).width(260.).show_ui(ui, |ui| {
                            for e in &all {
                                let (icon, brief) = entry_title(e);
                                let k = entry_key(e);
                                if cg.items.contains(&k) {
                                    continue;
                                }
                                let r = ui.horizontal(|ui| {
                                    ui.add(egui::Image::new(crate::icons::get(icon)).fit_to_exact_size(egui::vec2(18., 18.)));
                                    ui.selectable_label(false, tr(brief))
                                });
                                if r.inner.clicked() {
                                    cg.items.push(k);
                                }
                            }
                        });
                        if ui.button(tr("🗑 Apagar grupo")).clicked() {
                            remove_group = true;
                        }
                    }
                    if remove_group {
                        c.groups.remove(idx);
                        *sel = 0;
                    }
                }
            });
        });
        if !open {
            self.customize_open = false;
            self.settings.save();
        }
    }

    /// Lê a peça de origem de um link (relativo à pasta do projeto ou absoluto).
    fn load_link_source(&self, path: &str) -> Result<Document, String> {
        let p = std::path::Path::new(path);
        let full = match (&self.project, p.is_relative()) {
            (Some(pr), true) => pr.root.join(p),
            _ => p.to_path_buf(),
        };
        let s = std::fs::read_to_string(&full).map_err(|e| format!("{}: {e}", full.display()))?;
        Document::from_json(&s).map_err(|e| e.to_string())
    }

    /// Relê as peças de origem e atualiza a cópia de cada link ativo.
    pub fn refresh_links(&mut self) {
        let exts: Vec<(Id, String, String)> = self.open.doc.external.iter().map(|e| (e.id, e.path.clone(), e.publication.clone())).collect();
        let mut cache: std::collections::HashMap<String, Option<Vec<(String, LinkedGeo)>>> = Default::default();
        let (mut ok, mut broken) = (0, 0);
        for (id, path, publ) in exts {
            let pubs = cache.entry(path.clone()).or_insert_with(|| self.load_link_source(&path).ok().map(|d| scc_solidos::publications(&d)));
            let geo = pubs.as_ref().and_then(|v| v.iter().find(|(n, _)| *n == publ).map(|(_, g)| g.clone()));
            if geo.is_some() {
                ok += 1;
            } else {
                broken += 1;
            }
            self.open.doc.refresh_link(id, geo);
        }
        self.toast(format!("Links: {ok} atualizado(s), {broken} quebrado(s)"), broken > 0);
    }

    /// Janela "Importar publicação": escolhe a peça de origem (do projeto ou arquivo) e as publicações.
    fn link_window(&mut self, ctx: &egui::Context) {
        if !self.link_open {
            return;
        }
        let mut open = true;
        let mut choose: Option<(String, String)> = None;
        let mut import: Option<(String, String, LinkedGeo)> = None;
        let parts: Vec<(String, String)> = self
            .project
            .as_ref()
            .map(|p| p.index.documents.iter().filter(|d| Some(d.id) != self.open.entry).map(|d| (format!("{} · {}", d.pn, d.name), d.file.clone())).collect())
            .unwrap_or_default();
        egui::Window::new(tr("🔗 Importar publicação")).open(&mut open).default_width(420.).show(ctx, |ui| {
            ui.label(RichText::new(tr("Escolha a peça de origem; aparecem os planos, pontos, curvas e parâmetros que ela publica (📤).")).weak().small());
            if !parts.is_empty() {
                ui.label(RichText::new(tr("Peças do projeto")).strong());
                for (n, f) in &parts {
                    if ui.selectable_label(self.link_src.as_ref().is_some_and(|(_, p, _)| p == f), n).clicked() {
                        choose = Some((n.clone(), f.clone()));
                    }
                }
            }
            if ui.button(tr("📂 Escolher arquivo .sccpart…")).clicked() {
                if let Some(p) = rfd::FileDialog::new().add_filter("Peça SolidCadCraft", &["sccpart"]).pick_file() {
                    let n = p.file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                    choose = Some((n, p.to_string_lossy().to_string()));
                }
            }
            ui.separator();
            match &self.link_src {
                None => {
                    ui.label(RichText::new(tr("nenhuma peça escolhida")).weak());
                }
                Some((name, path, pubs)) => {
                    ui.label(RichText::new(format!("Publicações de {name}")).strong());
                    if pubs.is_empty() {
                        ui.label(RichText::new(tr("essa peça não publica nada — na peça de origem, marque 📤 publicar no elemento ou parâmetro")).weak().small());
                    }
                    for (n, g) in pubs {
                        let kind: String = match g {
                            LinkedGeo::Plane { .. } => "▱ plano".into(),
                            LinkedGeo::Point(_) => "• ponto".into(),
                            LinkedGeo::Curve(_) => "∿ curva".into(),
                            LinkedGeo::Value(v) => format!("ƒ = {}", fmt(*v)),
                        };
                        ui.horizontal(|ui| {
                            ui.label(format!("{n}  ({kind})"));
                            if ui.small_button(tr("Importar")).clicked() {
                                import = Some((name.clone(), path.clone(), g.clone()));
                                import.as_mut().unwrap().0 = format!("{}|{}", name, n);
                            }
                        });
                    }
                }
            }
        });
        if let Some((n, path)) = choose {
            match self.load_link_source(&path) {
                Ok(d) => {
                    let short = d.name.clone();
                    let _ = n;
                    self.link_src = Some((short, path, scc_solidos::publications(&d)));
                }
                Err(e) => self.toast(format!("Não foi possível ler a peça: {e}"), true),
            }
        }
        if let Some((key, path, g)) = import {
            let (src, publ) = key.split_once('|').unwrap_or((&key, ""));
            let id = self.open.doc.add_link(src, &path, publ, g);
            self.toast(format!("Link criado: {}", self.open.doc.name_of(id)), false);
        }
        if !open {
            self.link_open = false;
            self.link_src = None;
        }
    }

    /// Pasta de geometria de um item (ou a própria pasta-mãe, para pastas).
    fn folder_of(&self, id: Id) -> Option<Id> {
        let d = self.doc();
        if let Some(r) = d.reference(id) {
            return r.folder.or_else(|| d.folders.iter().find(|f| f.kind == FolderKind::Geometria).map(|f| f.id));
        }
        if let Some(f) = d.folder(id) {
            return f.parent.or(Some(id)).filter(|_| f.kind == FolderKind::Geometria);
        }
        d.feature(id).and_then(|f| f.folder)
    }

    fn sel_for(&self, id: Id) -> Sel {
        let d = self.doc();
        if d.reference(id).is_some() {
            Sel::Ref(id)
        } else if d.folder(id).is_some() {
            Sel::Folder(id)
        } else {
            Sel::Feature(id)
        }
    }

    /// Cola o item da área de transferência na pasta (recortar move; copiar duplica).
    fn paste_into(&mut self, fo: Id) {
        let Some((id, cut)) = self.clip else {
            self.toast("Nada copiado", false);
            return;
        };
        let d = &mut self.open.doc;
        if cut {
            if d.move_to_folder(id, fo).is_err() {
                self.toast("Não dá para mover para lá", true);
                return;
            }
            self.clip = None;
        } else if let Some(n) = d.duplicate_into(id, fo) {
            self.sel = self.sel_for(n);
        } else {
            self.toast("Não dá para colar aqui", true);
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
            Sel::Feature(id) | Sel::Ref(id) | Sel::Collector(id) => self.ask_delete(id),
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
                ui.label(RichText::new(tr("◆ SolidCadCraft")).strong().color(Color32::from_rgb(120, 180, 255)));
                ui.separator();
                ui.menu_button(tr("Arquivo"), |ui| {
                    if ui.button(tr("Nova peça")).clicked() {
                        self.load_doc(Document::new("Peça"), None, None);
                        ui.close();
                    }
                    if ui.button(tr("Nova peça de exemplo")).clicked() {
                        self.load_doc(scc_solidos::example_document(), None, None);
                        ui.close();
                    }
                    if ui.button(tr("Abrir peça…")).clicked() {
                        self.open_file();
                        ui.close();
                    }
                    if ui.button(tr("Salvar   Ctrl+S")).clicked() {
                        self.save();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(tr("Exportar STEP…")).clicked() {
                        self.export("step");
                        ui.close();
                    }
                    if ui.button(tr("Exportar STL…")).clicked() {
                        self.export("stl");
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(tr("Novo projeto…")).clicked() {
                        self.new_project();
                        ui.close();
                    }
                    if ui.button(tr("Abrir projeto…")).clicked() {
                        self.open_project();
                        ui.close();
                    }
                });
                ui.menu_button(tr("Exibir"), |ui| {
                    ui.menu_button(tr("Estilo de exibição"), |ui| {
                        for m in crate::render::DisplayMode::ALL {
                            ui.radio_value(&mut self.settings.display, m, m.label());
                        }
                    });
                    if ui.checkbox(&mut self.hidden_env, tr("Ambiente de ocultos   Shift+H")).changed() {
                        self.scene_dirty = true;
                    }
                    ui.menu_button(tr("Barras de ferramentas"), |ui| {
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
                    if ui.button(tr("⚙ Personalizar comandos…")).clicked() {
                        self.customize_open = true;
                        ui.close();
                    }
                    if ui.button(tr("⚡ Automação (macros)…")).clicked() {
                        self.macro_open = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(tr("Configurações…")).clicked() {
                        self.settings_open = true;
                        ui.close();
                    }
                });
                ui.menu_button(tr("Editar"), |ui| {
                    if ui.add_enabled(!self.undo.is_empty(), egui::Button::new(tr("Desfazer   Ctrl+Z"))).clicked() {
                        self.undo();
                        ui.close();
                    }
                    if ui.add_enabled(!self.redo.is_empty(), egui::Button::new(tr("Refazer   Ctrl+Y"))).clicked() {
                        self.redo();
                        ui.close();
                    }
                });
                ui.separator();
                // abas de módulo (RF-UI-006)
                let _ = ui.selectable_label(true, tr("Modelagem Sólida"));
                for m in ["Superfícies", "Montagem", "Desenho", "Chapas", "Soldas"].map(tr) {
                    ui.add_enabled(false, egui::Button::new(m).frame(false)).on_disabled_hover_text(tr("Módulo em breve"));
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

        let sketching = self.sketch.is_some();
        let ts = self.tool_state();
        let top = toolbar::groups_at(&self.settings, Dock::Topo, sketching);
        if !top.is_empty() {
            let mut acts = vec![];
            egui::Panel::top("comandos").show(root, |ui| {
                // os blocos quebram para a linha de baixo quando não cabem na largura
                ui.horizontal_wrapped(|ui| {
                    for (i, g) in top.iter().enumerate() {
                        if i > 0 {
                            ui.separator();
                        }
                        acts.extend(toolbar::show_group(ui, *g, &mut self.settings, &ts, false));
                    }
                    // grupos montados pelo usuário
                    let mine: Vec<crate::settings::CustomGroup> = self.settings.custom.groups.iter().filter(|c| c.sketch == sketching && !c.items.is_empty()).cloned().collect();
                    for cg in &mine {
                        ui.separator();
                        acts.extend(toolbar::show_custom(ui, cg, &mut self.settings, &ts, false));
                    }
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
        let sketching = self.sketch.is_some();
        let ts = self.tool_state();
        let side = toolbar::groups_at(&self.settings, Dock::Lateral, sketching);
        if !side.is_empty() {
            egui::Panel::left("barra_lateral").resizable(false).show(root, |ui| {
                egui::ScrollArea::vertical().id_salt("barra_lat").show(ui, |ui| {
                    for (i, g) in side.iter().enumerate() {
                        if i > 0 {
                            ui.separator();
                        }
                        acts.extend(toolbar::show_group(ui, *g, &mut self.settings, &ts, true));
                    }
                });
            });
        }
        let ctx = root.ctx().clone();
        for (i, g) in toolbar::groups_at(&self.settings, Dock::Flutuante, sketching).into_iter().enumerate() {
            egui::Window::new(g.label())
                .id(egui::Id::unique(("flutuante", g.label())))
                // empilhados na borda esquerda da viewport, sem cobrir uns aos outros
                .default_pos(egui::pos2(290., 140. + 120. * i as f32))
                .resizable(false)
                .collapsible(true)
                .show(&ctx, |ui| {
                    acts.extend(toolbar::show_group(ui, g, &mut self.settings, &ts, false));
                });
        }
        for a in acts {
            self.act(a);
        }
    }

    /// Seletor de vistas (Espaço): caixa central transparente com o cubo 3D translúcido,
    /// vistas padrão, opções de câmera e câmeras salvas.
    fn view_menu_window(&mut self, ctx: &egui::Context) {
        if !self.view_menu {
            return;
        }
        let Some(cam) = self.viewport.as_ref().map(|v| v.camera) else { return };
        let mut close = false;
        let mut apply: Option<scc_core::NamedView> = None;
        let mut remove: Option<usize> = None;
        let mut save = false;
        let mut drag = egui::Vec2::ZERO;
        let mut turn = 0f32;
        let mut act: Option<Act> = None;
        let dark = ctx.global_style().visuals.dark_mode;
        let fill = if dark { Color32::from_rgba_unmultiplied(20, 24, 32, 165) } else { Color32::from_rgba_unmultiplied(246, 249, 253, 170) };
        egui::Area::new(egui::Id::unique("seletor_vistas")).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0., 0.)).order(egui::Order::Foreground).show(ctx, |ui| {
            egui::Frame::new()
                .fill(fill)
                .corner_radius(14.)
                .inner_margin(egui::Margin::same(14))
                .stroke(egui::Stroke::new(1., Color32::from_rgba_unmultiplied(80, 140, 230, 140)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tr("Vistas")).strong().size(16.));
                        ui.label(RichText::new(tr("arraste o cubo para girar · clique numa face, aresta ou canto · Espaço fecha")).weak().small());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button(tr("✖")).clicked() {
                                close = true;
                            }
                        });
                    });
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(300., 300.), egui::Sense::hover());
                        let (d, dr) = crate::viewcube::cube(ui, r, &cam, "seletor", true);
                        if d.is_some() {
                            self.view_choice = d;
                        }
                        drag = dr;
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.set_width(270.);
                            ui.label(RichText::new(tr("Vistas padrão")).strong());
                            egui::Grid::new("vistas_padrao").num_columns(3).spacing([4., 4.]).show(ui, |ui| {
                                for (i, (d, n)) in crate::viewcube::STANDARD.iter().chain(crate::viewcube::EXTRA.iter()).enumerate() {
                                    let icon = match *n {
                                        "Frente" | "Trás" => "vista_frente",
                                        "Topo" | "Base" => "vista_topo",
                                        "Esquerda" | "Direita" => "vista_direita",
                                        _ => "vista_iso",
                                    };
                                    let b = egui::Button::image_and_text(egui::Image::new(crate::icons::get(icon)).fit_to_exact_size(egui::vec2(18., 18.)), *n).min_size(egui::vec2(84., 26.));
                                    if ui.add(b).clicked() {
                                        self.view_choice = Some(Vec3::from(*d).normalize());
                                    }
                                    if i % 3 == 2 {
                                        ui.end_row();
                                    }
                                }
                            });
                            ui.add_space(8.);
                            ui.label(RichText::new(tr("Câmera")).strong());
                            ui.horizontal_wrapped(|ui| {
                                if ui.selectable_label(cam.ortho, tr("Ortogonal")).clicked() && !cam.ortho {
                                    act = Some(Act::Ortho);
                                }
                                if ui.selectable_label(!cam.ortho, tr("Perspectiva")).clicked() && cam.ortho {
                                    act = Some(Act::Ortho);
                                }
                                if ui.button(tr("⛶ Ajustar (F)")).clicked() {
                                    act = Some(Act::Fit);
                                }
                                if ui.button(tr("⊥ Normal à seleção (N)")).clicked() {
                                    act = Some(Act::Normal);
                                }
                                if ui.button(tr("⟲ 90°")).on_hover_text(tr("girar 90° em torno da vertical")).clicked() {
                                    turn = std::f32::consts::FRAC_PI_2;
                                }
                                if ui.button(tr("90° ⟳")).on_hover_text(tr("girar 90° em torno da vertical")).clicked() {
                                    turn = -std::f32::consts::FRAC_PI_2;
                                }
                            });
                            ui.add_space(8.);
                            ui.label(RichText::new(tr("Câmeras salvas")).strong());
                            if self.open.doc.views.is_empty() {
                                ui.label(RichText::new(tr("nenhuma ainda")).weak());
                            }
                            for (i, v) in self.open.doc.views.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    if ui.button(format!("📷 {}", v.name)).clicked() {
                                        apply = Some(v.clone());
                                    }
                                    if ui.small_button(tr("✖")).clicked() {
                                        remove = Some(i);
                                    }
                                });
                            }
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut self.new_view_name).hint_text(tr("nome da vista")).desired_width(130.));
                                if ui.button(tr("💾 Salvar atual")).clicked() {
                                    save = true;
                                }
                            });
                            ui.label(RichText::new(tr("As câmeras ficam na peça e servirão para as vistas 2D e o detalhamento 3D.")).weak().small());
                        });
                    });
                });
        });
        if let Some(vp) = self.viewport.as_mut() {
            vp.camera.yaw -= drag.x * 0.008 + turn;
            vp.camera.pitch = (vp.camera.pitch + drag.y * 0.008).clamp(-1.5706, 1.5706);
        }
        if let Some(a) = act {
            self.act(a);
        }
        if save {
            let n = if self.new_view_name.trim().is_empty() { format!("Vista {}", self.open.doc.views.len() + 1) } else { self.new_view_name.trim().to_string() };
            self.open.doc.views.push(scc_core::NamedView {
                name: n,
                target: cam.target.to_array().map(|v| v as f64),
                yaw: cam.yaw as f64,
                pitch: cam.pitch as f64,
                dist: cam.dist as f64,
                ortho: cam.ortho,
            });
            self.new_view_name.clear();
        }
        if let Some(i) = remove {
            self.open.doc.views.remove(i);
        }
        if let Some(v) = apply {
            self.apply_view(&v);
            close = true;
        }
        if close {
            self.view_menu = false;
        }
    }

    pub fn apply_view(&mut self, v: &scc_core::NamedView) {
        if let Some(vp) = self.viewport.as_mut() {
            let before = vp.camera;
            vp.camera.target = Vec3::from(v.target.map(|x| x as f32));
            vp.camera.yaw = v.yaw as f32;
            vp.camera.pitch = v.pitch as f32;
            vp.camera.dist = v.dist as f32;
            vp.camera.ortho = v.ortho;
            vp.animate_from(before);
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = true;
        let s = &mut self.settings;
        egui::Window::new(tr("Configurações")).open(&mut open).default_width(380.).resizable(true).show(ctx, |ui| {
            egui::CollapsingHeader::new(tr("Interface")).default_open(true).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("Tema"));
                    ui.radio_value(&mut s.theme, Theme::Escuro, tr("Escuro"));
                    ui.radio_value(&mut s.theme, Theme::Claro, tr("Claro"));
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Idioma"));
                    ui.radio_value(&mut s.english, false, "Português");
                    ui.radio_value(&mut s.english, true, "English");
                });
                ui.add(egui::Slider::new(&mut s.ui_scale, 0.75..=1.75).text("escala (fontes e controles)"));
                ui.add(egui::Slider::new(&mut s.icon_size, 20.0..=48.0).text("tamanho dos ícones"));
                ui.checkbox(&mut s.show_labels, tr("mostrar o nome embaixo dos ícones"));
            });
            egui::CollapsingHeader::new(tr("Fundo da viewport")).default_open(true).show(ui, |ui| {
                ui.checkbox(&mut s.gradient, tr("degradê de duas cores"));
                ui.horizontal(|ui| {
                    ui.label(if s.gradient { "Topo" } else { "Cor" });
                    if s.gradient {
                        ui.color_edit_button_srgb(&mut s.bg_top);
                        ui.label(tr("Base"));
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
            egui::CollapsingHeader::new(tr("Atalhos e navegação")).default_open(true).show(ui, |ui| {
                use crate::keymap::{Binding, KeyAction, Preset};
                ui.horizontal(|ui| {
                    ui.label(tr("Adaptar ao meu CAD:"));
                    let cur = s.keys.preset;
                    egui::ComboBox::from_id_salt("preset").selected_text(cur.label()).show_ui(ui, |ui| {
                        for p in Preset::ALL {
                            if ui.selectable_label(cur == p, p.label()).clicked() {
                                s.keys.preset = p;
                                s.keys.custom.clear();
                                s.nav = p.nav();
                            }
                        }
                    });
                });
                ui.label(RichText::new(tr("Troca o teclado e o mouse juntos, com base nos atalhos de fábrica de cada software. Clique num atalho para gravar outro (Esc cancela, Delete remove).")).weak().small());
                let capturing: Option<KeyAction> = ui.ctx().data(|d| d.get_temp(egui::Id::unique("captura_atalho")));
                egui::ScrollArea::vertical().max_height(260.).id_salt("atalhos").show(ui, |ui| {
                    egui::Grid::new("grade_atalhos").num_columns(3).striped(true).show(ui, |ui| {
                        let mut group = "";
                        for a in KeyAction::ALL {
                            if a.group() != group {
                                group = a.group();
                                ui.label(RichText::new(group).strong());
                                ui.end_row();
                            }
                            ui.label(a.label());
                            let txt = if capturing == Some(a) { "pressione…".to_string() } else { s.keys.binding(a).map(|b| b.text()).unwrap_or_else(|| "—".into()) };
                            if ui.add(egui::Button::new(txt).min_size(egui::vec2(110., 0.))).clicked() {
                                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique("captura_atalho"), a));
                            }
                            let conf = s.keys.conflicts(a);
                            if conf.is_empty() {
                                ui.label(tr(""));
                            } else {
                                ui.label(RichText::new(format!("⚠ igual a {}", conf[0].label())).color(Color32::from_rgb(255, 170, 60)).small());
                            }
                            ui.end_row();
                        }
                    });
                });
                if let Some(a) = capturing {
                    let got = ui.ctx().input(|i| {
                        i.events.iter().find_map(|e| match e {
                            egui::Event::Key { key, pressed: true, modifiers, .. } => Some((*key, *modifiers)),
                            _ => None,
                        })
                    });
                    if let Some((key, m)) = got {
                        match key {
                            egui::Key::Escape => {}
                            egui::Key::Delete | egui::Key::Backspace if !m.any() => s.keys.set(a, None),
                            _ => s.keys.set(a, Some(Binding { key: key.name().to_string(), ctrl: m.command, shift: m.shift, alt: m.alt })),
                        }
                        ui.ctx().data_mut(|d| d.remove::<KeyAction>(egui::Id::unique("captura_atalho")));
                    }
                }
                if ui.small_button(tr("Restaurar atalhos da predefinição")).clicked() {
                    s.keys.custom.clear();
                }
            });
            egui::CollapsingHeader::new(tr("Navegação e seleção")).default_open(true).show(ui, |ui| {
                egui::ComboBox::from_id_salt("nav").selected_text(s.nav.label()).show_ui(ui, |ui| {
                    for n in crate::settings::NavStyle::ALL {
                        ui.selectable_value(&mut s.nav, n, n.label());
                    }
                });
                ui.label(RichText::new(s.nav.help()).weak().small());
                ui.checkbox(&mut s.invert_zoom, tr("inverter o sentido do zoom"));
                ui.horizontal(|ui| {
                    ui.label(tr("Selecionar:"));
                    ui.checkbox(&mut s.sel_faces, tr("faces"));
                    ui.checkbox(&mut s.sel_edges, tr("arestas"));
                    ui.checkbox(&mut s.sel_vertices, tr("vértices"));
                });
            });
            egui::CollapsingHeader::new(tr("Exibição")).default_open(true).show(ui, |ui| {
                for m in crate::render::DisplayMode::ALL {
                    ui.radio_value(&mut s.display, m, m.label());
                }
                ui.checkbox(&mut s.edges_in_render, tr("arestas no semi-renderizado"));
                ui.checkbox(&mut s.ortho, tr("projeção ortogonal"));
            });
            egui::CollapsingHeader::new(tr("Barras de ferramentas")).show(ui, |ui| {
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
                ui.label(RichText::new(tr("Dica: clique direito em um grupo para mudar onde ele fica.")).weak().small());
            });
            egui::CollapsingHeader::new(tr("Cotagem")).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("Norma"));
                    egui::ComboBox::from_id_salt("norma").selected_text(s.dim_standard.clone()).show_ui(ui, |ui| {
                        for n in ["ABNT NBR 10126", "ISO 129-1", "ASME Y14.5"] {
                            ui.selectable_value(&mut s.dim_standard, n.to_string(), n);
                        }
                    });
                });
                ui.add(egui::Slider::new(&mut s.dim_decimals, 0..=4).text("casas decimais"));
                ui.add(egui::Slider::new(&mut s.dim_text_height, 1.8..=7.0).text("altura do texto (mm)"));
                ui.label(RichText::new(tr("Usado pela ferramenta de cotagem do sketch e pelo desenho técnico.")).weak().small());
            });
            egui::CollapsingHeader::new(tr("Sketch")).show(ui, |ui| {
                ui.checkbox(&mut s.grid, tr("grade"));
                ui.add(egui::Slider::new(&mut s.grid_step, 1.0..=50.0).text("passo da grade (mm)"));
                ui.checkbox(&mut s.snaps, tr("snaps"));
                ui.checkbox(&mut s.auto_constraints, tr("restrições automáticas"));
                ui.checkbox(&mut s.auto_dims, tr("cotas automáticas"));
                ui.label(RichText::new(tr("Aplicado pelo editor de sketch interativo (próxima alpha).")).weak().small());
            });
            ui.separator();
            if ui.button(tr("Restaurar padrões")).clicked() {
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

        // ---------- peça
        let r = pick(ui, Sel::Part, RichText::new(format!("🧊 {}", doc.name)).strong());
        if r.clicked() {
            new_sel = Some(Sel::Part);
        }
        r.context_menu(|ui| {
            appearance_menu(ui, doc.appearance, AppTarget::Part, &mut actions);
            if ui.button(tr("📁 Nova pasta de geometria")).clicked() {
                actions.push(TreeAction::NewGeometryFolder);
                ui.close();
            }
            if ui.button(tr("▣ Novo coletor de corpos")).clicked() {
                actions.push(TreeAction::NewCollector);
                ui.close();
            }
        });
        if !doc.props.pn.is_empty() {
            ui.label(RichText::new(format!("   PN {} · {}", doc.props.pn, doc.props.part_type.label())).weak().small());
        }

        // ---------- Origem (fixa)
        egui::CollapsingHeader::new(tr("📍 Origem")).default_open(false).show(ui, |ui| {
            for o in [OriginPlane::XY, OriginPlane::XZ, OriginPlane::YZ] {
                let r = pick(ui, Sel::Origin(o), RichText::new(format!("▱ Plano {}", scene::plane_name(o))));
                if r.clicked() {
                    new_sel = Some(Sel::Origin(o));
                }
                r.context_menu(|ui| {
                    if ui.button(tr("✏ Novo sketch neste plano")).clicked() {
                        actions.push(TreeAction::SketchOn(PlaneRef::Origin(o)));
                        ui.close();
                    }
                });
            }
            ui.label(RichText::new(tr("↗ Eixos X, Y, Z · ponto de origem")).weak());
            let mut v = doc.origin_visible;
            if ui.checkbox(&mut v, tr("mostrar origem")).changed() {
                actions.push(TreeAction::OriginVisible(v));
            }
        });

        // ---------- referências externas (sempre separadas)
        egui::CollapsingHeader::new(format!("🔗 {} ({})", tr("Referências externas"), doc.external.len())).default_open(false).show(ui, |ui| {
            if doc.external.is_empty() {
                ui.label(RichText::new(tr("Geometria importada de outras peças por link (planos, curvas, pontos e parâmetros publicados). Fica sempre separada da geometria própria.")).weak().small());
            }
            for e in &doc.external {
                let st = match e.state {
                    LinkState::Ativo => "●",
                    LinkState::Congelado => "❄",
                    LinkState::Quebrado => "⚠",
                };
                let txt = format!("🔗 {st} {} ← {}.{}", e.target.map(|t| doc.name_of(t)).unwrap_or_default(), e.source, e.publication);
                let r = ui.selectable_label(e.target.is_some_and(|t| self.sel == Sel::Ref(t)), txt);
                if r.clicked() {
                    if let Some(t) = e.target {
                        new_sel = Some(if doc.reference(t).is_some() { Sel::Ref(t) } else { Sel::Params });
                    }
                }
            }
            ui.horizontal(|ui| {
                if ui.small_button(tr("➕ Importar publicação…")).clicked() {
                    actions.push(TreeAction::ImportLink);
                }
                if !doc.external.is_empty() && ui.small_button(tr("⟳ Atualizar")).clicked() {
                    actions.push(TreeAction::RefreshLinks);
                }
            });
        });

        // ---------- parâmetros
        egui::CollapsingHeader::new(format!("ƒ {} ({})", tr("Parâmetros"), doc.params.len())).default_open(false).show(ui, |ui| {
            let vars = self.vars();
            for p in &doc.params {
                let v = vars.get(&p.name).map(|v| format!("{} {}", fmt(*v), p.unit.symbol())).unwrap_or("⚠".into());
                if pick(ui, Sel::Params, RichText::new(format!("{} = {v}", p.name))).clicked() {
                    new_sel = Some(Sel::Params);
                }
            }
            if doc.params.is_empty() && pick(ui, Sel::Params, RichText::new(tr("vazio — clique para editar")).weak().italics()).clicked() {
                new_sel = Some(Sel::Params);
            }
        });

        // ---------- vistas salvas (câmeras)
        if !doc.views.is_empty() {
            egui::CollapsingHeader::new(format!("📷 Vistas ({})", doc.views.len())).default_open(false).show(ui, |ui| {
                for (i, v) in doc.views.iter().enumerate() {
                    if ui.selectable_label(false, format!("📷 {}", v.name)).on_hover_text(tr("aplicar esta vista")).clicked() {
                        actions.push(TreeAction::ApplyView(i));
                    }
                }
            });
        }

        // ---------- pastas de geometria (sem linha do tempo), com subpastas
        let first_geo = doc.folders.iter().find(|f| f.kind == FolderKind::Geometria).map(|f| f.id);
        for fo in doc.folders.iter().filter(|f| f.kind == FolderKind::Geometria && f.parent.is_none_or(|p| doc.folder(p).is_none())) {
            self.geo_folder(ui, &doc, fo, first_geo, &mut new_sel, &mut actions);
        }
        // planos antigos sem pasta, quando ainda não existe pasta de geometria
        if first_geo.is_none() && !doc.references.is_empty() {
            egui::CollapsingHeader::new(tr("📁 Geometria")).default_open(true).show(ui, |ui| {
                for r in &doc.references {
                    if pick(ui, Sel::Ref(r.id), RichText::new(format!("▱ {}", r.name))).clicked() {
                        new_sel = Some(Sel::Ref(r.id));
                    }
                }
            });
        }
        if ui.small_button(tr("➕ Pasta de geometria")).on_hover_text(tr("Planos, pontos, curvas e sketches livres, organizados como você quiser (sem linha do tempo)")).clicked() {
            actions.push(TreeAction::NewGeometryFolder);
        }

        ui.separator();

        // ---------- coletores de corpos, cada um com a sua linha do tempo
        for c in &doc.collectors {
            let feats: Vec<(usize, &Feature)> = doc.features.iter().enumerate().filter(|(_, f)| f.target == c.id && !doc.is_free(f)).collect();
            let n = feats.len();
            let marker_at = c.marker.unwrap_or(n).min(n);
            let id = ui.make_persistent_id(("col", c.id.0));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
                .show_header(ui, |ui| {
                    let mut v = c.visible;
                    if ui.checkbox(&mut v, tr("")).on_hover_text(tr("visível")).changed() {
                        actions.push(TreeAction::CollectorVisible(c.id, v));
                    }
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(10., 10.), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2., Color32::from_rgb(c.color[0], c.color[1], c.color[2]));
                    let active = doc.active_collector == c.id;
                    let txt = if active { RichText::new(format!("{} {}", c.name, tr("(ativo)"))).strong() } else { RichText::new(&c.name) };
                    let resp = pick(ui, Sel::Collector(c.id), txt);
                    if resp.clicked() {
                        new_sel = Some(Sel::Collector(c.id));
                    }
                    if resp.double_clicked() {
                        actions.push(TreeAction::Activate(c.id));
                    }
                    resp.context_menu(|ui| {
                        if ui.button(tr("Tornar ativo")).clicked() {
                            actions.push(TreeAction::Activate(c.id));
                            ui.close();
                        }
                        if ui.button(tr("🗀 Nova pasta de comandos")).clicked() {
                            actions.push(TreeAction::NewCommandFolder);
                            ui.close();
                        }
                        if ui.button(tr("Marcador no fim")).clicked() {
                            actions.push(TreeAction::CollectorMarker(c.id, None));
                            ui.close();
                        }
                        appearance_menu(ui, Some(Appearance { color: c.color, finish: c.finish, texture: c.texture }), AppTarget::Collector(c.id), &mut actions);
                        ui.separator();
                        if ui.button(tr("🗑 Apagar corpo…")).clicked() {
                            actions.push(TreeAction::Delete(c.id));
                            ui.close();
                        }
                    });
                })
                .body(|ui| {
                    if feats.is_empty() {
                        ui.label(RichText::new(tr("sem comandos — crie um sketch e uma extrusão")).weak().italics().small());
                    }
                    let mut drawn_folders = vec![];
                    for (k, (i, f)) in feats.iter().enumerate() {
                        if k == marker_at {
                            marker(ui, c.id, k, n, &mut actions);
                        }
                        match f.folder.filter(|fo| doc.folder(*fo).is_some_and(|x| x.kind == FolderKind::Comandos)) {
                            Some(fid) if drawn_folders.contains(&fid) => continue,
                            Some(fid) => {
                                drawn_folders.push(fid);
                                let name = doc.folder(fid).map(|x| x.name.clone()).unwrap_or_default();
                                let h = egui::CollapsingHeader::new(format!("🗀 {name}")).id_salt(("pasta", fid.0)).default_open(true).show(ui, |ui| {
                                    for (j, g) in feats.iter().filter(|(_, g)| g.folder == Some(fid)) {
                                        self.feature_row(ui, &doc, *j, g, &mut new_sel, &mut actions);
                                    }
                                });
                                if h.header_response.clicked() {
                                    new_sel = Some(Sel::Folder(fid));
                                }
                            }
                            None => self.feature_row(ui, &doc, *i, f, &mut new_sel, &mut actions),
                        }
                    }
                    if marker_at >= n && n > 0 {
                        marker(ui, c.id, n, n, &mut actions);
                    }
                });
        }
        // pastas de comandos vazias
        for fo in doc.folders.iter().filter(|fo| fo.kind == FolderKind::Comandos && !doc.features.iter().any(|f| f.folder == Some(fo.id))) {
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

    /// Pasta de geometria (recursiva): subpastas, elementos de referência e sketches livres.
    /// Itens podem ser arrastados para outra pasta; o menu do botão direito copia, recorta, cola e cria.
    fn geo_folder(&self, ui: &mut egui::Ui, doc: &Document, fo: &Folder, first_geo: Option<Id>, new_sel: &mut Option<Sel>, actions: &mut Vec<TreeAction>) {
        let refs: Vec<&RefElement> = doc
            .references
            .iter()
            .filter(|r| !matches!(r.kind, RefKind::Linked { .. }) && (r.folder == Some(fo.id) || (r.folder.is_none() && Some(fo.id) == first_geo)))
            .collect();
        let sketches: Vec<(usize, &Feature)> = doc.features.iter().enumerate().filter(|(_, f)| f.folder == Some(fo.id) && doc.is_free(f)).collect();
        let subs: Vec<&Folder> = doc.folders.iter().filter(|f| f.parent == Some(fo.id) && f.kind == FolderKind::Geometria).collect();
        let id = ui.make_persistent_id(("geo", fo.id.0));
        let has_clip = self.clip.is_some();
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                let n = refs.len() + sketches.len() + subs.len();
                let r = ui.selectable_label(self.sel == Sel::Folder(fo.id), RichText::new(format!("📁 {}  ({n})", fo.name)).strong());
                drag_item(ui, &r, fo.id);
                // soltar um item arrastado aqui move para esta pasta
                if let Some(p) = r.dnd_release_payload::<DragItem>() {
                    actions.push(TreeAction::MoveTo(p.0, fo.id));
                }
                if r.dnd_hover_payload::<DragItem>().is_some() {
                    ui.painter().rect_stroke(r.rect.expand(2.), 3., egui::Stroke::new(1.5, Color32::from_rgb(90, 170, 255)), egui::StrokeKind::Outside);
                }
                if r.clicked() {
                    *new_sel = Some(Sel::Folder(fo.id));
                }
                r.context_menu(|ui| {
                    ui.menu_button(tr("➕ Criar aqui"), |ui| {
                        if ui.button(tr("✏ Sketch")).clicked() {
                            actions.push(TreeAction::NewSketchIn(fo.id));
                            ui.close();
                        }
                        for (lbl, k) in [
                            ("▱ Plano", crate::geo_ui::GeoKind::PlaneOffset),
                            ("• Ponto", crate::geo_ui::GeoKind::PtCoords),
                            ("╱ Linha", crate::geo_ui::GeoKind::LinePts),
                            ("◯ Círculo", crate::geo_ui::GeoKind::Circle),
                            ("∿ Spline", crate::geo_ui::GeoKind::Spline),
                            ("⌇ Hélice", crate::geo_ui::GeoKind::Helix),
                        ] {
                            if ui.button(lbl).clicked() {
                                actions.push(TreeAction::NewGeoIn(fo.id, k));
                                ui.close();
                            }
                        }
                    });
                    if ui.button(tr("📁 Nova subpasta")).clicked() {
                        actions.push(TreeAction::NewSubfolder(fo.id));
                        ui.close();
                    }
                    ui.separator();
                    if ui.add_enabled(has_clip, egui::Button::new(tr("📋 Colar aqui  (Ctrl+V)"))).clicked() {
                        actions.push(TreeAction::Paste(fo.id));
                        ui.close();
                    }
                    if ui.button(tr("Copiar pasta")).clicked() {
                        actions.push(TreeAction::Copy(fo.id));
                        ui.close();
                    }
                    if ui.button(tr("Recortar pasta")).clicked() {
                        actions.push(TreeAction::Cut(fo.id));
                        ui.close();
                    }
                    if ui.button(tr("Duplicar pasta")).clicked() {
                        actions.push(TreeAction::Duplicate(fo.id));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(tr("Remover pasta (o conteúdo sobe um nível)")).clicked() {
                        actions.push(TreeAction::RemoveFolder(fo.id));
                        ui.close();
                    }
                });
            })
            .body(|ui| {
                if refs.is_empty() && sketches.is_empty() && subs.is_empty() {
                    ui.label(RichText::new(tr("vazia — clique direito para criar, ou arraste itens para cá")).weak().italics().small());
                }
                for s in &subs {
                    self.geo_folder(ui, doc, s, first_geo, new_sel, actions);
                }
                for r in &refs {
                    self.ref_row(ui, doc, r, new_sel, actions);
                }
                for (i, f) in &sketches {
                    self.feature_row(ui, doc, *i, f, new_sel, actions);
                }
            });
    }

    fn ref_row(&self, ui: &mut egui::Ui, doc: &Document, r: &RefElement, new_sel: &mut Option<Sel>, actions: &mut Vec<TreeAction>) {
        let users = doc.dependents(r.id).len();
        let icon = match (&r.kind, r.kind.class()) {
            (RefKind::AnglePlane { .. }, _) => "◩",
            (RefKind::FacePlane { .. }, _) => "▣",
            (_, RefClass::Point) => "•",
            (_, RefClass::Surface) => "◫",
            (RefKind::LinePoints { .. } | RefKind::LineDir { .. } | RefKind::LineAngle { .. } | RefKind::LineNormalSurface { .. }, _) => "╱",
            (_, RefClass::Curve) => "∿",
            _ => "▱",
        };
        let err = matches!(self.status_of(r.id), Some(Status::Error(_)));
        let txt = format!("{icon} {}{}{}", r.name, if users > 0 { format!("  ·{users}") } else { String::new() }, if err { " ✖" } else { "" });
        let resp = ui
            .horizontal(|ui| {
                if eye(ui, !r.visible) {
                    actions.push(TreeAction::RefVisible(r.id, !r.visible));
                }
                let t = if err { RichText::new(txt).color(Color32::from_rgb(255, 110, 100)) } else if r.visible { RichText::new(txt) } else { RichText::new(txt).weak() };
                ui.selectable_label(self.sel == Sel::Ref(r.id), t)
            })
            .inner;
        drag_item(ui, &resp, r.id);
        if resp.clicked() {
            *new_sel = Some(Sel::Ref(r.id));
        }
        if resp.double_clicked() {
            actions.push(TreeAction::EditSketch(r.id));
        }
        let resp = match self.status_of(r.id).and_then(|s| s.message()) {
            Some(m) => resp.on_hover_text(m),
            None => resp,
        };
        resp.context_menu(|ui| {
            if ui.button(tr("✏ Editar")).clicked() {
                actions.push(TreeAction::EditSketch(r.id));
                ui.close();
            }
            if r.kind.class() == RefClass::Plane && ui.button(tr("Novo sketch neste plano")).clicked() {
                actions.push(TreeAction::SketchOn(PlaneRef::Datum(r.id)));
                ui.close();
            }
            clip_menu(ui, r.id, doc, actions);
            if ui.button(tr("Apagar…")).clicked() {
                actions.push(TreeAction::Delete(r.id));
                ui.close();
            }
        });
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
        if resp.double_clicked() {
            actions.push(TreeAction::EditSketch(f.id));
        }
        let free = doc.is_free(f);
        if free {
            drag_item(ui, &resp, f.id);
        }
        let resp = match st.and_then(|s| s.message()) {
            Some(m) => resp.on_hover_text(m),
            None => resp,
        };
        if resp.clicked() {
            *new_sel = Some(Sel::Feature(f.id));
        }
        resp.context_menu(|ui| {
            if free {
                clip_menu(ui, f.id, doc, actions);
                ui.separator();
            }
            if ui.button(if f.suppressed { "Reativar" } else { "Suprimir" }).clicked() {
                actions.push(TreeAction::Suppress(f.id));
                ui.close();
            }
            if ui.button(if f.hidden { "Mostrar   H" } else { "Ocultar   H" }).clicked() {
                actions.push(TreeAction::Hide(f.id, !f.hidden));
                ui.close();
            }
            if ui.button(tr("✏ Editar   (duplo clique)")).clicked() {
                actions.push(TreeAction::EditSketch(f.id));
                ui.close();
            }
            if matches!(f.kind, FeatureKind::Sketch(_)) && ui.button(tr("Alinhar vista ao sketch   N")).clicked() {
                actions.push(TreeAction::Align(f.id));
                ui.close();
            }
            appearance_menu(ui, f.appearance, AppTarget::Feature(f.id), actions);
            if !doc.is_free(f) && ui.button(tr("Marcador até aqui")).clicked() {
                let k = doc.features[..=i].iter().filter(|g| g.target == f.target && !doc.is_free(g)).count();
                actions.push(TreeAction::CollectorMarker(f.target, Some(k)));
                ui.close();
            }
            ui.menu_button(tr("Mover para…"), |ui| {
                ui.label(RichText::new(tr("Coletor (linha do tempo)")).weak().small());
                for c in &doc.collectors {
                    if ui.button(format!("▣ {}", c.name)).clicked() {
                        actions.push(TreeAction::ToCollector(f.id, c.id));
                        ui.close();
                    }
                }
                let geo = matches!(f.kind, FeatureKind::Sketch(_));
                ui.label(RichText::new(if geo { "Pasta (geometria livre / comandos)" } else { "Pasta de comandos" }).weak().small());
                for fo in doc.folders.iter().filter(|fo| geo || fo.kind == FolderKind::Comandos) {
                    let icon = if fo.kind == FolderKind::Geometria { "📁" } else { "🗀" };
                    if ui.button(format!("{icon} {}", fo.name)).clicked() {
                        actions.push(TreeAction::ToFolder(f.id, Some(fo.id)));
                        ui.close();
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui.button(tr("◀ antes")).clicked() && i > 0 {
                    actions.push(TreeAction::Move(f.id, i - 1));
                }
                if ui.button(tr("depois ▶")).clicked() {
                    actions.push(TreeAction::Move(f.id, i + 1));
                }
            });
            if let FeatureKind::Sketch(s) = &f.kind {
                if ui.button(tr("Extrudar este sketch")).clicked() {
                    actions.push(TreeAction::Select(f.id, Cmd::Extrude));
                    ui.close();
                }
                let _ = s;
            }
            ui.separator();
            if ui.button(tr("Apagar…")).clicked() {
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
            TreeAction::EditSketch(id) => self.start_session(id, false, None),
            TreeAction::ApplyView(i) => {
                if let Some(v) = d.views.get(i).cloned() {
                    self.apply_view(&v);
                }
            }
            TreeAction::NewGeometryFolder => {
                let n = format!("Geometria {}", d.folders.iter().filter(|f| f.kind == FolderKind::Geometria).count() + 1);
                self.sel = Sel::Folder(d.add_geometry_folder(&n));
            }
            TreeAction::NewCommandFolder => {
                let n = format!("Pasta {}", d.folders.len() + 1);
                self.sel = Sel::Folder(d.add_folder(&n));
            }
            TreeAction::NewCollector => {
                let n = format!("Corpo {}", d.collectors.len() + 1);
                let id = d.add_collector(&n);
                d.active_collector = id;
                self.sel = Sel::Collector(id);
            }
            TreeAction::NewSketchIn(fo) => {
                let id = d.add_free_sketch(PlaneRef::Origin(OriginPlane::XY), fo);
                self.begin_sketch(id);
            }
            TreeAction::RemoveFolder(fo) => {
                d.remove_folder(fo);
                self.sel = Sel::None;
            }
            TreeAction::NewSubfolder(parent) => {
                let n = d.unique_name("Pasta");
                let id = d.add_subfolder(&n, parent);
                self.sel = Sel::Folder(id);
            }
            TreeAction::NewGeoIn(fo, k) => {
                let id = d.add_geometry(k.make(&Default::default()));
                let _ = d.move_to_folder(id, fo);
                self.start_session(id, true, None);
            }
            TreeAction::MoveTo(item, fo) => {
                if d.move_to_folder(item, fo).is_err() {
                    self.toast("Não dá para mover para lá (pasta dentro dela mesma, ou item de linha do tempo)", true);
                }
            }
            TreeAction::ImportLink => self.link_open = true,
            TreeAction::RefreshLinks => self.refresh_links(),
            TreeAction::Copy(id) => self.clip = Some((id, false)),
            TreeAction::Cut(id) => self.clip = Some((id, true)),
            TreeAction::Paste(fo) => self.paste_into(fo),
            TreeAction::Duplicate(id) => {
                let to = self.folder_of(id);
                if let Some(to) = to {
                    if let Some(n) = self.open.doc.duplicate_into(id, to) {
                        self.sel = self.sel_for(n);
                    }
                }
            }
            TreeAction::ToCollector(f, c) => {
                if let Some(x) = d.feature_mut(f) {
                    x.target = c;
                    // sai de uma pasta de geometria ao entrar na linha do tempo
                    if x.folder.and_then(|fo| d.folder(fo)).is_some_and(|fo| fo.kind == FolderKind::Geometria) {
                        if let Some(x) = d.feature_mut(f) {
                            x.folder = None;
                        }
                    }
                }
            }
            TreeAction::CollectorMarker(c, m) => {
                if let Some(x) = d.collectors.iter_mut().find(|x| x.id == c) {
                    x.marker = m;
                }
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
            ui.label(tr("Nenhum projeto aberto."));
            ui.label(RichText::new(tr("Um projeto é uma pasta com as peças, o histórico de versões e as revisões liberadas.")).weak());
            ui.add_space(8.);
            if ui.button(tr("📁 Novo projeto…")).clicked() {
                self.new_project();
            }
            if ui.button(tr("📂 Abrir projeto…")).clicked() {
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
            ui.label(RichText::new(tr("PN")).strong());
            ui.label(RichText::new(tr("Nome")).strong());
            ui.label(RichText::new(tr("Estado")).strong());
            ui.label(RichText::new(tr("Rev")).strong());
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
            ui.label(RichText::new(tr("Sem documentos. Adicione a peça aberta ou crie uma nova.")).weak());
        }
        if let Some(id) = select {
            self.project_sel = Some(id);
        }
        if let Some(id) = open {
            self.open_entry(id, None);
        }

        ui.add_space(6.);
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("➕ Nova peça")).clicked() {
                let p = self.project.as_mut().unwrap();
                match p.add_document(Document::new("Peça")) {
                    Ok(id) => {
                        self.project_sel = Some(id);
                        self.open_entry(id, None);
                    }
                    Err(e) => self.toast(e.to_string(), true),
                }
            }
            if self.open.entry.is_none() && ui.button(tr("⤵ Adicionar peça aberta")).clicked() {
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
            if ui.button(tr("💾 Salvar versão")).clicked() {
                self.save();
            }
            if ui.button(tr("📄 Estrutura CSV")).clicked() {
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
            if ui.button(tr("Abrir")).clicked() {
                self.open_entry(sel, None);
            }
            if ui.add_enabled(e.dirty_since_release(), egui::Button::new(tr("🔒 Liberar revisão…"))).clicked() {
                self.release_dialog = Some((sel, String::new()));
            }
            if ui.button(tr("Duplicar")).clicked() {
                if let Err(err) = self.project.as_mut().unwrap().duplicate(sel) {
                    self.toast(err.to_string(), true);
                }
            }
            if e.state != DocState::Obsoleto && ui.button(tr("Tornar obsoleto")).clicked() {
                let _ = self.project.as_mut().unwrap().set_state(sel, DocState::Obsoleto);
            }
        });
        egui::CollapsingHeader::new(format!("Revisões ({})", e.revisions.len())).default_open(true).show(ui, |ui| {
            for r in e.revisions.iter().rev() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Rev {}", r.label)).strong());
                    ui.label(RichText::new(&r.comment).weak());
                    if ui.small_button(tr("abrir 🔒")).on_hover_text(tr("Abrir congelada (somente leitura)")).clicked() {
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
                    if ui.small_button(tr("abrir")).clicked() {
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
                    ui.label(RichText::new(format!("{} · duplo clique: ajustar", self.settings.nav.help())).small().weak());
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
            let (shift, ctrl, scroll, ppp, right_down, left_down) = ctx.input(|i| {
                (i.modifiers.shift, i.modifiers.command, i.smooth_scroll_delta.y, i.pixels_per_point, i.pointer.secondary_down(), i.pointer.primary_down())
            });
            let delta = resp.drag_delta();
            let (left, mid, right) = (
                resp.dragged_by(egui::PointerButton::Primary),
                resp.dragged_by(egui::PointerButton::Middle),
                resp.dragged_by(egui::PointerButton::Secondary),
            );
            let sketching = self.sketch.is_some();
            use crate::settings::NavStyle as N;
            // (girar, mover, zoom arrastando) conforme o estilo de navegação escolhido
            let (orbit, pan, zoom_drag) = match self.settings.nav {
                N::SolidCadCraft => ((left && !shift && !sketching) || mid, right || (shift && left), false),
                N::SolidWorks => (mid && !ctrl, mid && ctrl, false),
                N::Fusion => (mid && shift, mid && !shift, false),
                N::Catia => (mid && right_down, mid && !right_down, false),
                N::Blender => (mid && !shift && !ctrl, mid && shift, mid && ctrl),
                N::Onshape => (right && !ctrl, (right && ctrl) || mid, false),
                N::FreeCad => (mid && (left_down || right_down), mid && !left_down && !right_down, false),
                N::Inventor => (mid && shift, mid && !shift, false),
                N::Rhino => (right && !shift && !ctrl, right && shift, right && ctrl),
            };
            if pan || orbit || zoom_drag || (resp.hovered() && scroll != 0.) {
                vp.stop_anim();
            }
            if pan {
                let (r, u) = vp.camera.basis();
                let k = vp.camera.dist * 0.0012;
                vp.camera.target += (-r * delta.x + u * delta.y) * k;
            } else if zoom_drag {
                vp.camera.dist = (vp.camera.dist * (delta.y * 0.01).exp()).clamp(0.5, 1e6);
            } else if orbit {
                vp.camera.yaw -= delta.x * 0.008;
                vp.camera.pitch = (vp.camera.pitch + delta.y * 0.008).clamp(-1.5706, 1.5706);
            }
            if resp.hovered() && scroll != 0. {
                let inv = self.settings.invert_zoom != (self.settings.nav == N::SolidWorks);
                let sc = if inv { scroll } else { -scroll };
                vp.camera.dist = (vp.camera.dist * (sc * 0.0015).exp()).clamp(0.5, 1e6);
            }
            if (resp.double_clicked() && self.sketch.is_none() && self.hover_pick.is_none()) || self.fit_pending {
                if let Some(b) = vp.bounds {
                    let before = vp.camera;
                    vp.camera.fit(b);
                    vp.animate_from(before);
                    self.fit_pending = false;
                }
            }
            // reenquadra ao abrir um comando, se o modelo não couber na tela
            if std::mem::take(&mut self.reframe_pending) {
                if let Some((lo, hi)) = vp.bounds {
                    let inside = (0..8).all(|c| {
                        let q = Vec3::new(if c & 1 == 0 { lo.x } else { hi.x }, if c & 2 == 0 { lo.y } else { hi.y }, if c & 4 == 0 { lo.z } else { hi.z });
                        vp.camera.project(rect, q).is_some_and(|p| rect.shrink(20.).contains(p))
                    });
                    let tiny = vp.camera.project(rect, lo).zip(vp.camera.project(rect, hi)).is_some_and(|(a, b)| (a - b).length() < rect.width().min(rect.height()) * 0.15);
                    if !inside || tiny {
                        let before = vp.camera;
                        vp.camera.fit((lo, hi));
                        vp.animate_from(before);
                    }
                }
            }
            if vp.step_anim() {
                ctx.request_repaint();
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

            // rótulos de pontos e curvas de referência
            if let (Some(r), true) = (&self.result, self.settings.filter.rotulos) {
                for p in &r.points {
                    if let Some(pos) = vp.project(rect, Vec3::from(p.p.map(|v| v as f32))) {
                        ui.painter().text(pos + egui::vec2(6., -4.), egui::Align2::LEFT_BOTTOM, &p.name, egui::FontId::proportional(11.), ink);
                    }
                }
                for c in &r.curves {
                    let m = c.pts[c.pts.len() / 2];
                    if let Some(pos) = vp.project(rect, Vec3::from(m.map(|v| v as f32))) {
                        ui.painter().text(pos + egui::vec2(6., -4.), egui::Align2::LEFT_BOTTOM, &c.name, egui::FontId::proportional(11.), ink);
                    }
                }
            }

            let cam = vp.camera;
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
            // cubo de vistas no canto superior direito
            let cube_area = Rect::from_min_size(egui::pos2(rect.right() - 120., rect.top() + 8.), egui::vec2(112., 112.));
            let (dir, drag) = crate::viewcube::cube(ui, cube_area, &cam, "canto", false);
            if dir.is_some() {
                self.view_choice = dir;
            }
            if let Some(vp) = self.viewport.as_mut() {
                vp.camera.yaw -= drag.x * 0.008;
                vp.camera.pitch = (vp.camera.pitch + drag.y * 0.008).clamp(-1.5706, 1.5706);
            }
            if self.sketch.is_some() {
                crate::sketch_ui::overlay(self, ui, rect, &resp, cam);
            } else if let Some(r) = &self.result {
                let filt = crate::picking::Filter {
                    faces: self.settings.sel_faces,
                    edges: self.settings.sel_edges,
                    vertices: self.settings.sel_vertices,
                    planes: true,
                };
                let dragging = resp.dragged();
                self.hover_pick = match (resp.hover_pos(), dragging) {
                    (Some(h), false) => crate::picking::pick(r, &self.open.doc, &cam, rect, h, &filt),
                    _ => None,
                };
                // filete/chanfro aberto com arestas escolhidas: clicar numa aresta põe/tira do comando
                let edge_cmd = self.session.as_ref().map(|x| x.id).filter(|id| {
                    matches!(self.open.doc.feature(*id).map(|f| &f.kind), Some(FeatureKind::Fillet(o) | FeatureKind::Chamfer(o)) if matches!(o.edges, EdgeSel::Edges(_)))
                });
                // casca/saída/espessura/rosca abertos: clicar numa face a acrescenta ao comando
                let face_cmd = self.session.as_ref().map(|x| x.id).filter(|id| self.open.doc.feature(*id).is_some_and(|f| crate::feat_ui::wants_faces(&f.kind)));
                if let (Some(fid), true, Some(crate::picking::Pick::Face { point, body, .. })) = (face_cmd, resp.clicked_by(egui::PointerButton::Primary), self.hover_pick) {
                    // a mesma face do corpo (pela malha atual) é tirada em vez de repetida
                    let same = |a: [f64; 3], b: [f64; 3]| crate::picking::same_face(r, body, a, b);
                    if let Some(f) = self.open.doc.feature_mut(fid) {
                        crate::feat_ui::toggle_face(&mut f.kind, point.map(|v| v as f64), same);
                    }
                } else if resp.double_clicked_by(egui::PointerButton::Primary) {
                    // duplo clique numa face abre o comando que a criou
                    if let Some(crate::picking::Pick::Face { owner, .. }) = self.hover_pick {
                        if self.open.doc.feature(owner).is_some() && edge_cmd.is_none() {
                            self.open_pending = Some(owner);
                        }
                    }
                } else if resp.clicked_by(egui::PointerButton::Primary) {
                    let add = ctx.input(|i| i.modifiers.shift || i.modifiers.command);
                    match (edge_cmd, self.hover_pick) {
                        (Some(fid), Some(crate::picking::Pick::Edge { mid, .. })) => {
                            let m = mid.map(|v| v as f64);
                            if let Some(FeatureKind::Fillet(o) | FeatureKind::Chamfer(o)) = self.open.doc.feature_mut(fid).map(|f| &mut f.kind) {
                                if let EdgeSel::Edges(list) = &mut o.edges {
                                    let near = |q: &[f64; 3]| (0..3).map(|k| (q[k] - m[k]).powi(2)).sum::<f64>().sqrt() < 0.01;
                                    if let Some(k) = list.iter().position(near) {
                                        list.remove(k);
                                    } else {
                                        list.push(m);
                                    }
                                }
                            }
                        }
                        (_, Some(p)) => {
                            if let Some(i) = self.picks.iter().position(|x| *x == p) {
                                self.picks.remove(i);
                            } else {
                                if !add {
                                    self.picks.clear();
                                }
                                self.picks.push(p);
                            }
                        }
                        (_, None) => {
                            if !add {
                                self.picks.clear();
                            }
                        }
                    }
                }
                // faces escolhidas do comando em edição
                if let Some(Some(k)) = face_cmd.map(|id| self.open.doc.feature(id).map(|f| f.kind.clone())) {
                    let marks = crate::feat_ui::face_marks(&k);
                    for q in &marks {
                        if let Some(sp) = cam.project(rect, Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32)) {
                            ui.painter().circle_filled(sp, 5., Color32::from_rgb(90, 200, 255));
                            ui.painter().circle_stroke(sp, 7., egui::Stroke::new(1.5, Color32::WHITE));
                        }
                    }
                    ui.painter().text(rect.center_top() + egui::vec2(0., 30.), egui::Align2::CENTER_TOP,
                        format!("Clique nas faces para acrescentá-las · {} escolhida(s) · Enter conclui", marks.len()), egui::FontId::proportional(13.), Color32::from_rgb(90, 200, 255));
                }
                // arestas escolhidas do filete/chanfro em edição
                if let Some(Some(FeatureKind::Fillet(o) | FeatureKind::Chamfer(o))) = edge_cmd.map(|id| self.open.doc.feature(id).map(|f| &f.kind)) {
                    if let EdgeSel::Edges(list) = &o.edges {
                        for q in list {
                            if let Some(sp) = cam.project(rect, Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32)) {
                                ui.painter().circle_filled(sp, 5., Color32::from_rgb(255, 190, 40));
                                ui.painter().circle_stroke(sp, 7., egui::Stroke::new(1.5, Color32::WHITE));
                            }
                        }
                        let n = list.len();
                        ui.painter().text(rect.center_top() + egui::vec2(0., 30.), egui::Align2::CENTER_TOP,
                            format!("Clique nas arestas para pôr/tirar do comando · {n} escolhida(s) · Enter conclui"), egui::FontId::proportional(13.), Color32::from_rgb(255, 190, 40));
                    }
                }
                let painter = ui.painter_at(rect);
                for p in &self.picks {
                    crate::picking::draw(&painter, r, &cam, rect, p, true);
                }
                if let Some(h) = self.hover_pick {
                    if !self.picks.contains(&h) {
                        crate::picking::draw(&painter, r, &cam, rect, &h, false);
                    }
                }
                if !self.picks.is_empty() {
                    let n = self.picks.len();
                    let txt = if n == 1 { self.picks[0].label(&self.open.doc) } else { format!("{n} itens selecionados") };
                    ui.painter().text(rect.left_bottom() + egui::vec2(10., -10.), egui::Align2::LEFT_BOTTOM, format!("Seleção: {txt}  ·  Esc limpa"), egui::FontId::proportional(12.), Color32::from_rgb(255, 190, 40));
                }
            }
            // seta e cota 3D do comando aberto (fast 3D)
            if self.sketch.is_none() {
                crate::handles::overlay(self, ui, rect, cam);
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
                        ui.label(tr("Substituir por:"));
                        let cur = dd.replacement.map(|id| doc.name_of(id)).unwrap_or("escolha…".into());
                        egui::ComboBox::from_id_salt("subst").selected_text(cur).show_ui(ui, |ui| {
                            for (id, n) in &cands {
                                ui.selectable_value(&mut dd.replacement, Some(*id), n);
                            }
                        });
                        if ui.add_enabled(dd.replacement.is_some(), egui::Button::new(tr("Substituir e apagar"))).clicked() {
                            action = Some(DelAction::Replace(dd.target, dd.replacement.unwrap()));
                        }
                    });
                }
                ui.add_space(6.);
                ui.horizontal(|ui| {
                    if ui.button(tr("Apagar junto com dependentes")).clicked() {
                        action = Some(DelAction::All(dd.target));
                    }
                    if ui.button(tr("Apagar só este")).on_hover_text(tr("Os dependentes ficam com aviso de referência perdida e podem ser reapontados")).clicked() {
                        action = Some(DelAction::Only(dd.target));
                    }
                    if ui.button(tr("Cancelar")).clicked() {
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
                ui.heading(tr("Liberar revisão"));
                ui.label(tr("A versão salva mais recente será congelada (somente leitura)."));
                ui.add(egui::TextEdit::multiline(comment).hint_text(tr("Comentário da revisão")).desired_rows(3));
                ui.horizontal(|ui| {
                    done = ui.button(tr("🔒 Liberar")).clicked();
                    cancel = ui.button(tr("Cancelar")).clicked();
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
    ExtrudeMid,
    ExtrudeUpTo,
    Cut,
    CutBlind,
    CutUpTo,
    Revolve,
    Hole,
    HoleCounterbore,
    HoleCountersink,
    HoleTapped,
    Fillet,
    FilletAll,
    Chamfer,
    ChamferAll,
    /// plano, ponto, linha ou curva de referência
    Geo(crate::geo_ui::GeoKind),
    Groove,
    Rib,
    Slot,
    Loft,
    LoftCut,
    Prim(PrimKind),
    Stiffener,
    FilletVar,
    ChamferTwoDist,
    ChamferDistAngle,
    Shell,
    Draft,
    Thickness,
    Thread,
    PatternLinear,
    PatternCircular,
    PatternPoints,
    Mirror,
    Scale,
    Combine(BoolOp),
    FilletFaceFace,
    FilletFull,
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
    ui.menu_button(tr("🎨 Aparência"), |ui| {
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
        if ui.button(tr("Personalizar cor e acabamento…")).clicked() {
            actions.push(TreeAction::Customize(t));
            ui.close();
        }
        if !matches!(t, AppTarget::Collector(_)) && current.is_some() && ui.button(tr("Remover aparência")).clicked() {
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
    ApplyView(usize),
    NewGeometryFolder,
    NewCommandFolder,
    NewCollector,
    NewSketchIn(Id),
    RemoveFolder(Id),
    ToCollector(Id, Id),
    CollectorMarker(Id, Option<usize>),
    Hide(Id, bool),
    RefVisible(Id, bool),
    SetAppearance(AppTarget, Option<Appearance>),
    Customize(AppTarget),
    Align(Id),
    EditSketch(Id),
    OriginVisible(bool),
    CollectorVisible(Id, bool),
    Activate(Id),
    Suppress(Id),
    ToFolder(Id, Option<Id>),
    Move(Id, usize),
    Delete(Id),
    SketchOn(PlaneRef),
    Select(Id, Cmd),
    NewSubfolder(Id),
    NewGeoIn(Id, crate::geo_ui::GeoKind),
    MoveTo(Id, Id),
    Copy(Id),
    Cut(Id),
    Paste(Id),
    Duplicate(Id),
    ImportLink,
    RefreshLinks,
}

/// Item da árvore de geometria sendo arrastado.
#[derive(Clone, Copy)]
struct DragItem(Id);

/// Torna a linha da árvore arrastável (sem atrapalhar o clique).
fn drag_item(ui: &mut egui::Ui, resp: &egui::Response, id: Id) {
    let d = ui.interact(resp.rect, ui.scope_id().with(("arrastar", id.0)), egui::Sense::drag());
    d.dnd_set_drag_payload(DragItem(id));
    if d.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
}

/// Copiar / recortar / duplicar / mover para pasta.
fn clip_menu(ui: &mut egui::Ui, id: Id, doc: &Document, actions: &mut Vec<TreeAction>) {
    if ui.button(tr("Copiar  (Ctrl+C)")).clicked() {
        actions.push(TreeAction::Copy(id));
        ui.close();
    }
    if ui.button(tr("Recortar  (Ctrl+X)")).clicked() {
        actions.push(TreeAction::Cut(id));
        ui.close();
    }
    if ui.button(tr("Duplicar  (Ctrl+D)")).clicked() {
        actions.push(TreeAction::Duplicate(id));
        ui.close();
    }
    ui.menu_button(tr("Mover para pasta"), |ui| {
        fn level(ui: &mut egui::Ui, doc: &Document, parent: Option<Id>, depth: usize, id: Id, actions: &mut Vec<TreeAction>) {
            for g in doc.folders.iter().filter(|f| f.kind == FolderKind::Geometria && f.parent == parent) {
                if g.id != id && ui.button(format!("{}📁 {}", "   ".repeat(depth), g.name)).clicked() {
                    actions.push(TreeAction::MoveTo(id, g.id));
                    ui.close();
                }
                if g.id != id {
                    level(ui, doc, Some(g.id), depth + 1, id, actions);
                }
            }
        }
        level(ui, doc, None, 0, id, actions);
    });
}

enum DelAction {
    Replace(Id, Id),
    All(Id),
    Only(Id),
}

/// Marcador da linha do tempo de um coletor: os comandos abaixo dele ficam desativados.
fn marker(ui: &mut egui::Ui, collector: Id, at: usize, n: usize, actions: &mut Vec<TreeAction>) {
    ui.horizontal(|ui| {
        let blue = Color32::from_rgb(90, 160, 255);
        ui.label(RichText::new(tr("━━ marcador ━━")).small().color(blue))
            .on_hover_text(tr("Comandos abaixo do marcador ficam desativados; novos comandos deste coletor entram aqui"));
        if at > 0 && ui.small_button(tr("▲")).on_hover_text(tr("subir o marcador")).clicked() {
            actions.push(TreeAction::CollectorMarker(collector, Some(at - 1)));
        }
        if at < n && ui.small_button(tr("▼")).on_hover_text(tr("descer o marcador")).clicked() {
            actions.push(TreeAction::CollectorMarker(collector, if at + 1 >= n { None } else { Some(at + 1) }));
        }
        if at < n && ui.small_button(tr("⏬")).on_hover_text(tr("marcador no fim")).clicked() {
            actions.push(TreeAction::CollectorMarker(collector, None));
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
        crate::i18n::set_english(self.settings.english);
        // resultados do recompute
        while let Ok((g, r)) = self.worker.rx.try_recv() {
            if g >= self.gen_done {
                self.gen_done = g;
                self.result = Some(r);
                self.scene_dirty = true;
            }
        }

        // atalhos (mapa configurável em Configurações → Atalhos e navegação)
        use crate::keymap::KeyAction as K;
        let km = self.settings.keys.clone();
        let typing = ctx.egui_wants_keyboard_input();
        let (save, undo, redo) = (km.pressed(ctx, K::Save), !typing && km.pressed(ctx, K::Undo), !typing && km.pressed(ctx, K::Redo));
        let del = km.pressed(ctx, K::Delete);
        if save {
            self.save();
        }
        if undo {
            self.undo();
        }
        if redo {
            self.redo();
        }
        if !typing && self.sketch.is_none() && km.pressed(ctx, K::ViewMenu) {
            self.view_menu = !self.view_menu;
        }
        if self.session.is_some() && !ctx.egui_wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.conclude_session();
            } else if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.cancel_session();
            }
        } else if self.sketch.is_none() && !self.picks.is_empty() && !ctx.egui_wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.picks.clear();
        }
        if self.sketch.is_some() && crate::sketch_ui::keys(self, ctx) {
            self.finish_sketch();
        }
        if del && self.sketch.is_none() && !ctx.egui_wants_keyboard_input() {
            self.delete_selected();
        }
        // atalhos de vista e de comandos (só quando não há campo de texto em edição)
        if !typing {
            let dir = |x: f32, y: f32, z: f32| Some(Vec3::new(x, y, z).normalize());
            for (k, d) in [
                (K::ViewFront, dir(0., -1., 0.)),
                (K::ViewBack, dir(0., 1., 0.)),
                (K::ViewTop, dir(0., 0., 1.)),
                (K::ViewBottom, dir(0., 0., -1.)),
                (K::ViewLeft, dir(-1., 0., 0.)),
                (K::ViewRight, dir(1., 0., 0.)),
                (K::ViewIso, dir(1., -1., 1.)),
            ] {
                if km.pressed(ctx, k) {
                    self.view_choice = d;
                }
            }
            for (k, a) in [(K::Fit, Act::Fit), (K::Normal, Act::Normal), (K::Ortho, Act::Ortho), (K::HiddenEnv, Act::HiddenEnv)] {
                if km.pressed(ctx, k) {
                    self.act(a);
                }
            }
            if km.pressed(ctx, K::Hide) {
                self.toggle_hide_selected();
            }
            if self.sketch.is_none() {
                let item = match self.sel {
                    Sel::Ref(id) | Sel::Folder(id) | Sel::Feature(id) => Some(id),
                    _ => None,
                };
                let geo = item.filter(|id| self.folder_of(*id).is_some_and(|f| self.doc().folder(f).is_some_and(|x| x.kind == FolderKind::Geometria)));
                if let Some(id) = geo {
                    if km.pressed(ctx, K::ClipCopy) {
                        self.clip = Some((id, false));
                        self.toast("Copiado — Ctrl+V cola na pasta selecionada", false);
                    }
                    if km.pressed(ctx, K::ClipCut) {
                        self.clip = Some((id, true));
                        self.toast("Recortado — Ctrl+V move para a pasta selecionada", false);
                    }
                    if km.pressed(ctx, K::ClipDup) {
                        if let Some(to) = self.folder_of(id) {
                            if let Some(n) = self.open.doc.duplicate_into(id, to) {
                                self.sel = self.sel_for(n);
                            }
                        }
                    }
                }
                if km.pressed(ctx, K::ClipPaste) {
                    let to = match self.sel {
                        Sel::Folder(f) if self.doc().folder(f).is_some_and(|x| x.kind == FolderKind::Geometria) => Some(f),
                        Sel::Ref(id) | Sel::Feature(id) => self.folder_of(id),
                        _ => None,
                    };
                    match to {
                        Some(f) => self.paste_into(f),
                        None => self.toast("Selecione uma pasta de geometria para colar", true),
                    }
                }
            }
            if self.sketch.is_none() && self.session.is_none() {
                for (k, c) in [
                    (K::Sketch, Cmd::Sketch),
                    (K::Extrude, Cmd::Extrude),
                    (K::Cut, Cmd::Cut),
                    (K::Revolve, Cmd::Revolve),
                    (K::Hole, Cmd::Hole),
                    (K::Fillet, Cmd::Fillet),
                    (K::Chamfer, Cmd::Chamfer),
                    (K::Plane, Cmd::Geo(crate::geo_ui::GeoKind::PlaneOffset)),
                ] {
                    if km.pressed(ctx, k) {
                        self.command(c);
                    }
                }
            }
        }

        if self.splash(root) {
            return;
        }
        if !self.started {
            let show_before = self.settings.show_start;
            self.start_screen(root);
            if self.settings.show_start != show_before || self.started {
                self.settings.save();
            }
            return;
        }

        let before = self.open.doc.clone();
        let sel_before = self.sel;
        let settings_before = self.settings.clone();
        let hidden_env_before = self.hidden_env;

        self.top(root);
        self.timeline(root);
        egui::Panel::left("arvore").default_size(270.).resizable(true).show(root, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.left, LeftTab::Peca, tr("Peça"));
                ui.selectable_value(&mut self.left, LeftTab::Projeto, tr("Projeto"));
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| match self.left {
                LeftTab::Peca => self.tree(ui),
                LeftTab::Projeto => self.project_panel(ui),
            });
        });
        egui::Panel::right("editor").default_size(330.).resizable(true).show(root, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.sketch.is_some() {
                    crate::sketch_ui::panel(self, ui)
                } else {
                    editor::show(self, ui);
                    crate::measure::panel(self, ui);
                }
            });
        });
        self.side_toolbars(root);
        self.settings_window(ctx);
        self.view_menu_window(ctx);
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
        let link_key = egui::Id::unique("scc_link_acao");
        if let Some((ext, act)) = ctx.data(|m| m.get_temp::<(Id, u8)>(link_key)) {
            ctx.data_mut(|m| m.remove::<(Id, u8)>(link_key));
            match act {
                0 => self.refresh_links(),
                _ => {
                    if let Some(e) = self.open.doc.external.iter_mut().find(|e| e.id == ext) {
                        e.state = if e.state == LinkState::Congelado { LinkState::Ativo } else { LinkState::Congelado };
                    }
                }
            }
        }
        self.link_window(ctx);
        self.customize_window(ctx);
        self.macro_window(ctx);
        if let Some(id) = self.open_pending.take() {
            self.start_session(id, false, None);
        }
        if let Some(id) = self.begin_sketch_pending.take() {
            self.begin_sketch(id);
        }
        if let Some(s) = &self.session {
            let still = matches!(self.sel, Sel::Feature(x) | Sel::Ref(x) if x == s.id);
            if !still && self.sel != sel_before {
                self.conclude_session();
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
                    editing: self.sketch.as_ref().map(|e| e.feature),
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
        // texturas: recarrega as imagens quando a lista (arquivo, escala) muda
        let key: Vec<(String, f64)> = self.open.doc.textures.iter().map(|t| (t.path.clone(), t.scale)).collect();
        if key != self.tex_key {
            if let (Some(vp), Some(rs)) = (self.viewport.as_mut(), frame.wgpu_render_state()) {
                let root = self.project.as_ref().map(|p| p.root.clone());
                let imgs: Vec<(Vec<u8>, f32)> = self
                    .open
                    .doc
                    .textures
                    .iter()
                    .map(|t| {
                        let p = std::path::Path::new(&t.path);
                        let full = match (&root, p.is_relative()) {
                            (Some(r), true) => r.join(p),
                            _ => p.to_path_buf(),
                        };
                        (crate::render::load_texture(&full).unwrap_or_default(), t.scale as f32)
                    })
                    .collect();
                vp.set_textures(rs, &imgs);
                self.tex_key = key;
                self.scene_dirty = true;
            }
        }
        if let (Some(d), Some(vp)) = (self.view_choice.take(), self.viewport.as_mut()) {
            let before = vp.camera;
            let t = vp.camera.target;
            vp.camera.look_along(d, t);
            if let (false, Some(b)) = (self.view_menu, vp.bounds) {
                vp.camera.fit(b);
            }
            vp.animate_from(before);
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

/// Fundo com profundidade (degradê, brilho e grade sutil) para a tela inicial e o carregamento.
fn paint_backdrop(p: &egui::Painter, rect: Rect, dark: bool) {
    let (top, bot) = if dark { (Color32::from_rgb(16, 30, 58), Color32::from_rgb(8, 11, 20)) } else { (Color32::from_rgb(232, 241, 252), Color32::from_rgb(196, 214, 238)) };
    let mut m = egui::Mesh::default();
    m.colored_vertex(rect.left_top(), top);
    m.colored_vertex(rect.right_top(), top);
    m.colored_vertex(rect.right_bottom(), bot);
    m.colored_vertex(rect.left_bottom(), bot);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(m));
    // brilho azul atrás do logo
    let glow = rect.center_top() + egui::vec2(0., rect.height() * 0.3);
    // muitos círculos bem transparentes = brilho suave, sem anéis visíveis
    for k in 0..24 {
        let r = 460. - k as f32 * 17.;
        p.circle_filled(glow, r, Color32::from_rgba_unmultiplied(60, 140, 255, if dark { 2 } else { 3 }));
    }
    // grade em perspectiva leve na parte de baixo (chão de CAD)
    let line = if dark { Color32::from_rgba_unmultiplied(120, 170, 255, 18) } else { Color32::from_rgba_unmultiplied(30, 80, 160, 22) };
    let horizon = rect.top() + rect.height() * 0.62;
    let vp = egui::pos2(rect.center().x, horizon - rect.height() * 0.25);
    for i in -14..=14 {
        let x = rect.center().x + i as f32 * rect.width() / 14.;
        let b = egui::pos2(x, rect.bottom());
        let t = (horizon - vp.y) / (b.y - vp.y);
        p.line_segment([vp + (b - vp) * t, b], egui::Stroke::new(1., line));
    }
    for k in 0..10 {
        let f = (k as f32 / 10.).powf(1.8);
        let y = horizon + (rect.bottom() - horizon) * f;
        p.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], egui::Stroke::new(1., line));
    }
}
