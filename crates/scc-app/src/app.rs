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
    /// anotação 3D (PMI) da peça
    Pmi(Id),
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
    /// (geração, documento, reconstruir do zero)
    tx: Sender<(u64, Document, bool)>,
    rx: Receiver<(u64, RecomputeResult)>,
}

impl Worker {
    /// Recompute fora da thread da interface (RNF-001); só o pedido mais recente é processado.
    fn spawn(ctx: egui::Context) -> Self {
        let (tx, jobs) = channel::<(u64, Document, bool)>();
        let (out, rx) = channel();
        std::thread::Builder::new()
            .name("recompute".into())
            .spawn(move || {
                // cache entre cálculos: só refaz a partir do primeiro comando que mudou
                let mut cache = scc_solidos::Cache::default();
                while let Ok(mut job) = jobs.recv() {
                    let mut fresh = job.2;
                    while let Ok(newer) = jobs.try_recv() {
                        fresh |= newer.2;
                        job = newer;
                    }
                    if fresh {
                        cache = scc_solidos::Cache::default();
                    }
                    let r = scc_solidos::recompute_cached(&job.1, &mut cache);
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
    /// janela de publicações (o que outras peças podem importar)
    pub pub_open: bool,
    /// repetição de pontos (e planos normais) ao longo de uma curva, em edição
    pub repeat: Option<Repeat>,
    /// janela de extração (simples/múltipla, propagação); `None` = fechada
    pub extract: Option<ExtractOpts>,
    /// últimas opções de extração usadas
    extract_last: ExtractOpts,
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
    /// novo sketch esperando o clique no plano/face
    pub sketch_wait_plane: bool,
    /// árvore aberta ao lado do painel do comando
    pub tree_peek: bool,
    /// início da seleção em caixa (Ctrl + arrastar) na viewport
    box_start: Option<egui::Pos2>,
    /// elemento sob o mouse no clique direito da viewport (menu de contexto do 3D)
    ctx_pick: Option<crate::picking::Pick>,
    /// ações escolhidas no menu de contexto do 3D (aplicadas depois da viewport)
    ctx_actions: Vec<TreeAction>,
    /// comandos desatualizados (calculado a cada quadro para a árvore)
    stale_set: std::cell::RefCell<(bool, std::collections::HashSet<Id>)>,
    /// seletor de vistas (tecla Espaço)
    view_menu: bool,
    new_view_name: String,
    /// direção escolhida no cubo, aplicada à câmera no fim do quadro
    view_choice: Option<Vec3>,
    /// item sob o mouse na árvore (realce dinâmico na viewport)
    hover_item: Option<Id>,
    hover_now: std::cell::Cell<Option<Id>>,
    /// multisseleção na árvore (Ctrl soma/tira, Shift faixa) e a ordem das linhas desenhadas
    pub multi: std::cell::RefCell<Vec<Id>>,
    tree_order: std::cell::RefCell<Vec<Id>>,
    multi_anchor: std::cell::Cell<Option<Id>>,
    last_order: std::cell::RefCell<Vec<Id>>,
    /// item sendo renomeado na própria árvore (F2)
    renaming: Option<Id>,
    /// janela de propriedades gráficas aberta para este alvo
    pub props_target: Option<AppTarget>,
    /// página da janela de configurações
    settings_page: usize,
    /// documento do último pedido de cálculo e do último resultado (para marcar o que está desatualizado)
    sent_doc: Option<Document>,
    result_doc: Option<Document>,
    /// início da varredura visual da árvore após um cálculo (comando a comando)
    sweep_t0: f64,
    /// assistente de referências quebradas
    wizard_open: bool,
    errors_prev: usize,
    /// link sendo religado pelo assistente (a janela de importação vira "religar")
    relink_ext: Option<Id>,
    /// montagem aberta (bancada Montagem)
    pub asm: Option<crate::asm_ui::AsmState>,
    /// janela de parâmetros de chapa
    sheet_open: bool,
    /// anotação 3D esperando seleções
    pub pmi_pending: Option<u8>,
    /// desenho 2D aberto (bancada Desenho)
    pub dwg: Option<crate::dwg_ui::DwgState>,
    /// janelas do estrutural: biblioteca de perfis e lista de corte
    pub st_lib_open: bool,
    pub st_cut_open: bool,
    pub st_lib_sel: usize,
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
            pub_open: false,
            repeat: None,
            extract: None,
            extract_last: ExtractOpts::default(),
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
            hover_item: None,
            hover_now: std::cell::Cell::new(None),
            multi: Default::default(),
            tree_order: Default::default(),
            multi_anchor: std::cell::Cell::new(None),
            last_order: Default::default(),
            renaming: None,
            props_target: None,
            settings_page: 0,
            sent_doc: None,
            result_doc: None,
            sweep_t0: -10.,
            wizard_open: false,
            errors_prev: 0,
            relink_ext: None,
            asm: None,
            sheet_open: false,
            pmi_pending: None,
            sketch_wait_plane: false,
            tree_peek: false,
            box_start: None,
            ctx_pick: None,
            ctx_actions: vec![],
            stale_set: Default::default(),
            dwg: None,
            st_lib_open: false,
            st_cut_open: false,
            st_lib_sel: 0,
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
        // desenvolvimento: SCC_BANCADA=sup abre na bancada de Superfícies; SCC_EXIBIR=zebra|curvatura
        // muda o modo de exibição; SCC_DEMO_SUP=1 acrescenta superfícies de exemplo
        if std::env::var("SCC_BANCADA").is_ok_and(|v| v.starts_with("sup")) {
            app.settings.bench = crate::settings::Bench::Superficies;
            app.started = true;
        }
        match std::env::var("SCC_EXIBIR").as_deref() {
            Ok("zebra") => app.settings.display = crate::render::DisplayMode::Zebra,
            Ok("curvatura") => app.settings.display = crate::render::DisplayMode::Curvatura,
            _ => {}
        }
        if std::env::var("SCC_DEMO_SUP").is_ok() {
            let d = &mut app.open.doc;
            let s = |x: &str| x.to_string();
            d.add_geometry(RefKind::SurfSphere { center: PointRef::Fixed([0., 0., 40.]), radius: s("15") });
            let la = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([40., 0., 0.]), b: PointRef::Fixed([40., 40., 0.]), start: s("0"), end: s("0") });
            let lb = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([70., 0., 20.]), b: PointRef::Fixed([70., 40., 20.]), start: s("0"), end: s("0") });
            let fa = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(la), dir: AxisRef::Origin(OriginAxis::X), length: s("-20"), length2: "0".into() });
            let fb = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(lb), dir: AxisRef::Origin(OriginAxis::X), length: s("20"), length2: "0".into() });
            d.add_geometry(RefKind::SurfBlend { a: CurveRef::Ref(la), sa: Some(SurfRef::Ref(fa)), b: CurveRef::Ref(lb), sb: Some(SurfRef::Ref(fb)), cont: Continuity::G2, tension: s("1") });
            d.add_geometry(RefKind::SurfCylinder { point: PointRef::Fixed([-40., 0., 0.]), dir: AxisRef::Origin(OriginAxis::Z), radius: s("12"), start: s("0"), end: s("50") });
            app.started = true;
        }
        // desenvolvimento: SCC_DEMO_ASM=1 monta duas placas de exemplo (contato face a face) e abre a Montagem
        if std::env::var("SCC_DEMO_ASM").is_ok() {
            let dir = std::env::temp_dir().join("scc_demo_montagem");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("placa.sccpart"), scc_solidos::example_document().to_json());
            let mut a = scc_montagem::Assembly::new("Conjunto de exemplo");
            use scc_montagem::{NodeKind, Placement};
            let g = a.add_group("Placas", None);
            a.add_component("Placa base", NodeKind::Part { path: "placa.sccpart".into() }, Placement::default(), Some(g));
            a.add_component("Placa de cima", NodeKind::Part { path: "placa.sccpart".into() }, Placement { origin: [140., 20., 30.], ..Default::default() }, Some(g));
            if std::env::var("SCC_DEMO_ASM").is_ok_and(|v| v == "mec") {
                // mecanismo: a placa de cima gira em torno do furo (48, 24) da base
                use scc_montagem::kin::{JFrame, JRef, JointKind};
                let ids: Vec<scc_core::Id> = a.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).map(|n| n.id).collect();
                a.node_mut(ids[1]).unwrap().placement.origin = [0., 0., 12.];
                let fr = JFrame::from_axis([48., 24., 0.], [0., 0., 1.]);
                let j = a.add_joint(JointKind::Revolute, JRef { comp: ids[0], frame: fr }, JRef { comp: ids[1], frame: fr });
                a.joints.iter_mut().find(|x| x.id == j).unwrap().drive_angle = Some("90*t".into());
            }
            let f = dir.join("conjunto.sccasm");
            let _ = std::fs::write(&f, a.to_json());
            let mut stt = crate::asm_ui::AsmState::new(a, Some(f));
            stt.sim_open = stt.asm.joints.len() > 0;
            app.asm = Some(stt);
            app.settings.bench = crate::settings::Bench::Montagem;
            app.started = true;
        }
        // desenvolvimento: SCC_DEMO_CHAPA=1 abre uma peça de chapa de exemplo na bancada Chapa
        if std::env::var("SCC_DEMO_CHAPA").is_ok() {
            let s = |x: &str| x.to_string();
            let mut d = Document::new("Suporte de chapa");
            d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: std::env::var("SCC_DEMO_CHAPA").is_ok_and(|v| v == "plana") });
            let mut data = SketchData::default();
            {
                let mut n = 9_000_000u64;
                let mut al = || {
                    n += 1;
                    Id(n)
                };
                data.add_rect(&mut al, [0., 0.], [100., 60.], false);
            }
            let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
            d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false }));
            d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 30., 2.], length: s("25"), angle: s("90"), radius: None, off1: s("8"), off2: s("8"), flip: false, relief: Some(s("3")) }));
            d.add_feature(FeatureKind::SmHem(SmHem { edge: [0., 30., 2.], length: s("8"), gap: s("0"), flip: false }));
            if std::env::var("SCC_DEMO_CHAPA").is_ok_and(|v| v == "linha") {
                d.add_feature(FeatureKind::SmLouver(SmLouver { a: [60., 15., 2.], b: [60., 45., 2.], width: s("12"), height: s("5"), flip: false, down: false }));
                d.add_feature(FeatureKind::SmBend(SmBend { a: [20., 0., 2.], b: [20., 60., 2.], angle: s("45"), radius: None, flip: false, down: false }));
            }
            d.hide_consumed(d.features[1].id);
            app.load_doc(d, None, None);
            app.settings.bench = crate::settings::Bench::Chapa;
            app.started = true;
        }
        // desenvolvimento: SCC_DEMO_EST=1 abre um quadro de mesa estrutural de exemplo
        if std::env::var("SCC_DEMO_EST").is_ok() {
            let s = |x: &str| x.to_string();
            let mut d = Document::new("Quadro de mesa");
            let top = [[0., 0., 700.], [800., 0., 700.], [800., 500., 700.], [0., 500., 700.]];
            let mut paths = vec![];
            for i in 0..4 {
                let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(top[i]), b: PointRef::Fixed(top[(i + 1) % 4]), start: s("0"), end: s("0") });
                paths.push(CurveRef::Ref(l));
            }
            let mut legs = vec![];
            for p in top {
                let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([p[0], p[1], 0.]), b: PointRef::Fixed([p[0], p[1], 680.]), start: s("0"), end: s("0") });
                legs.push(CurveRef::Ref(l));
            }
            let mk = |paths: Vec<CurveRef>, corner, anchor| StructMember {
                paths,
                family: s("Tubo quadrado"),
                designation: s("40x3"),
                kind: SectionKind::TuboQuadrado,
                dims: vec![s("40"), s("3")],
                anchor,
                angle: s("0"),
                off_x: s("0"),
                off_y: s("0"),
                ext1: s("0"),
                ext2: s("0"),
                corner,
                gap: s("0"),
            };
            d.add_feature(FeatureKind::StructMember(mk(paths, CornerMode::MeiaEsquadria, 0)));
            d.add_feature(FeatureKind::StructMember(mk(legs, CornerMode::Nenhum, 0)));
            d.add_feature(FeatureKind::StructPlate(StructPlate { at: [0., 0., 0.], thickness: s("8"), margin: s("15") }));
            for r in &mut d.references {
                r.visible = false;
            }
            app.load_doc(d, None, None);
            app.settings.bench = crate::settings::Bench::Estrutural;
            app.st_cut_open = true;
            app.started = true;
        }
        // desenvolvimento: SCC_DEMO_PMI=1 anota a placa de exemplo (cota, nota, tolerância, rugosidade)
        if std::env::var("SCC_DEMO_PMI").is_ok() {
            use scc_core::pmi::*;
            let s = |x: &str| x.to_string();
            let d = &mut app.open.doc;
            let mk = |d: &mut Document, kind, a: Vec<[f64; 3]>, off, text: &str| {
                let id = d.alloc();
                d.pmi.push(Pmi { id, name: format!("PMI {}", id.0), kind, anchors: a, offset: off, text: text.into(), tol_plus: s(""), tol_minus: s(""), visible: true, plane: None });
            };
            mk(d, PmiKind::Linear, vec![[-60., -35., 8.], [60., -35., 8.]], [0., -25., 0.], "");
            mk(d, PmiKind::Note, vec![[0., 0., 8.]], [40., 30., 30.], "Rebarbar arestas vivas");
            if let Some(p) = d.pmi.last_mut() {
                // nota deitada no plano do topo (exemplo de texto fixo num plano)
                p.plane = Some(([0., 0., 1.], [1., 0., 0.]));
            }
            mk(d, PmiKind::Gdt { ch: GdtChar::Planeza, tol: s("0,05"), datums: s("") }, vec![[-40., 20., 8.]], [-30., 30., 30.], "");
            mk(d, PmiKind::Roughness { ra: s("1,6"), process: s("fresado") }, vec![[30., 20., 8.]], [20., 25., 25.], "");
            app.started = true;
        }
        // desenvolvimento: SCC_DEMO_DWG=1 abre um desenho da placa de exemplo (3 vistas, corte, detalhe, cotas, tabela)
        if std::env::var("SCC_DEMO_DWG").is_ok() {
            use scc_desenho::*;
            let dir = std::env::temp_dir().join("scc_demo_desenho");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("placa.sccpart"), scc_solidos::example_document().to_json());
            let src = Source { path: "placa.sccpart".into(), display_state: String::new() };
            let mut d = Drawing::new("Placa de exemplo", Format::A3);
            let sh = d.sheets[0].id;
            let ids = d.add_standard_set(sh, src.clone(), [95., 235.], 1., 100.);
            let mut m = scc_desenho::source::Models::new(Some(dir.clone()));
            let f = m.proj(&d, ids[0]);
            let t = m.proj(&d, ids[1]);
            let ym = (t.lo[1] + t.hi[1]) / 2.;
            d.add_view(sh, src.clone(), ViewKind::Section { parent: ids[1], a: [t.lo[0] - 6., ym], b: [t.hi[0] + 6., ym], label: "A".into() }, [95., 60.], 1.);
            d.add_view(sh, src.clone(), ViewKind::Detail { parent: ids[0], center: [f.hi[0] - 8., f.hi[1] - 2.], radius: 12., label: "B".into() }, [200., 125.], 2.);
            if let Some(ax) = f.axes {
                let id = d.alloc();
                d.dims.push(Dim { id, view: ids[0], kind: DimKind::Horizontal, anchors: vec![ax.unproject([f.lo[0], f.hi[1]]), ax.unproject([f.hi[0], f.hi[1]])], offset: 8., text: String::new(), tol: String::new() });
                let id = d.alloc();
                d.dims.push(Dim { id, view: ids[0], kind: DimKind::Vertical, anchors: vec![ax.unproject([f.lo[0], f.lo[1]]), ax.unproject([f.lo[0], f.hi[1]])], offset: 8., text: String::new(), tol: "±0,1".into() });
            }
            let id = d.alloc();
            d.tables.push(Table { id, sheet: sh, kind: TableKind::Holes, source: src.clone(), pos: [270., 150.] });
            let id = d.alloc();
            d.notes.push(Note { id, sheet: sh, kind: NoteKind::Text, pos: [25., 25.], leader: None, text: "Quebrar cantos vivos 0,5x45°
Tolerâncias gerais ISO 2768-m".into(), size: 3.5 });
            match std::env::var("SCC_DEMO_DWG").as_deref() {
                Ok("abnt") => d.title.style = TitleStyle::Abnt,
                Ok("ansi") => d.title.style = TitleStyle::Ansi,
                _ => {}
            }
            let f = dir.join("placa.sccdwg");
            let _ = std::fs::write(&f, d.to_json());
            app.dwg = Some(crate::dwg_ui::DwgState::new(d, Some(f)));
            app.settings.bench = crate::settings::Bench::Desenho;
            app.started = true;
        }
        // desenvolvimento: SCC_VISTAS=1 abre o seletor de vistas ao iniciar
        if std::env::var("SCC_VISTAS").is_ok() {
            app.view_menu = true;
            app.started = true;
        }
        // desenvolvimento: SCC_ABRIR=<nome> abre esse comando para edição ao iniciar
        if let Ok(n) = std::env::var("SCC_ABRIR") {
            app.open_pending = app.open.doc.features.iter().find(|f| f.name == n).map(|f| f.id);
            if let Some(p) = app.open.doc.pmi.iter().find(|p| p.name == n) {
                app.sel = Sel::Pmi(p.id);
            }
        }
        app.request_recompute();
        app
    }

    pub fn doc(&self) -> &Document {
        &self.open.doc
    }

    /// Bancada de montagem ativa (com montagem aberta)?
    pub fn in_asm(&self) -> bool {
        self.settings.bench == crate::settings::Bench::Montagem && self.asm.is_some()
    }

    pub fn fit_now(&mut self) {
        self.fit_pending = true;
        self.scene_dirty = true;
    }

    pub fn load_doc_pub(&mut self, d: Document, file: Option<PathBuf>) {
        self.load_doc(d, None, None);
        self.open.file = file;
    }

    pub fn save_pub(&mut self) {
        self.save();
    }

    /// Câmera atual (para estados de exibição e animações).
    /// Id novo para uma anotação (não colide com os ids do documento nem da montagem).
    pub fn next_pmi_id(&mut self) -> u64 {
        if self.in_asm() {
            self.asm.as_mut().unwrap().asm.alloc().0
        } else {
            self.open.doc.alloc().0
        }
    }

    pub fn camera_get(&self) -> Option<scc_montagem::display::Cam> {
        self.viewport.as_ref().map(|v| {
            let c = v.camera;
            scc_montagem::display::Cam { target: c.target.to_array().map(|x| x as f64), yaw: c.yaw as f64, pitch: c.pitch as f64, dist: c.dist as f64, ortho: c.ortho }
        })
    }

    pub fn camera_set(&mut self, c: scc_montagem::display::Cam) {
        if let Some(v) = self.viewport.as_mut() {
            v.camera.target = Vec3::from(c.target.map(|x| x as f32));
            v.camera.yaw = c.yaw as f32;
            v.camera.pitch = c.pitch as f32;
            v.camera.dist = c.dist as f32;
            v.camera.ortho = c.ortho;
        }
    }

    pub fn take_open_pending(&mut self) -> Option<Id> {
        self.open_pending.take()
    }

    pub fn set_tree_hover(&self, id: Option<Id>) {
        if id.is_some() {
            self.hover_now.set(id);
        }
    }

    pub fn request_recompute(&mut self) {
        self.send_recompute(false);
    }

    /// Pede o cálculo; `fresh` descarta o cache (reconstruir tudo do zero).
    fn send_recompute(&mut self, fresh: bool) {
        self.gen_sent += 1;
        self.sent_doc = Some(self.open.doc.clone());
        let _ = self.worker.tx.send((self.gen_sent, self.open.doc.clone(), fresh));
    }

    /// Índice do primeiro comando desatualizado (a geometria mudou depois do último cálculo).
    /// Mudanças só de exibição (nome, ocultar, cor, pasta de organização) não desatualizam nada.
    pub fn stale_from(&self) -> Option<usize> {
        let Some(rd) = &self.result_doc else { return Some(0) };
        geo_stale_from(&self.open.doc, rd)
    }

    /// Comandos desatualizados: os que mudaram e os que dependem deles (mesmo corpo depois deles,
    /// ou que leem o que eles produzem). `true` = mudou algo global (tudo desatualizado).
    fn compute_stale(&self) -> (bool, std::collections::HashSet<Id>) {
        let mut set = std::collections::HashSet::new();
        let Some(rd) = &self.result_doc else { return (true, set) };
        let d = &self.open.doc;
        if geo_stale_from(d, rd).is_none() {
            return (false, set);
        }
        if geo_global_changed(d, rd) {
            return (true, set);
        }
        // parâmetros mudados (pelo nome) e referências mudadas: marcam só quem os usa
        let mut names: Vec<String> = d.params.iter().filter(|p| rd.params.iter().find(|q| q.name == p.name).is_none_or(|q| q.expr != p.expr || q.unit != p.unit)).map(|p| p.name.clone()).collect();
        names.extend(rd.params.iter().filter(|q| !d.params.iter().any(|p| p.name == q.name)).map(|q| q.name.clone()));
        let cites = |js: &str| names.iter().any(|n| js.match_indices(n.as_str()).any(|(i, _)| {
            let w = |c: char| c.is_alphanumeric() || c == '_';
            !js[..i].chars().next_back().is_some_and(w) && !js[i + n.len()..].chars().next().is_some_and(w)
        }));
        for _ in 0..3 {
            for r in &d.references {
                let js = serde_json::to_string(&r.kind).unwrap_or_default();
                let mut ids = vec![];
                r.kind.clone().walk(&mut |x| ids.push(*x), &mut |_| {});
                if rd.references.iter().find(|q| q.id == r.id).is_none_or(|q| q.kind != r.kind) || cites(&js) || ids.iter().any(|x| set.contains(x)) {
                    set.insert(r.id);
                }
            }
        }
        let same = |a: &Feature, b: &Feature| a.kind == b.kind && a.suppressed == b.suppressed && a.target == b.target && a.folder == b.folder;
        for f in &d.features {
            if cites(&serde_json::to_string(&f.kind).unwrap_or_default()) {
                set.insert(f.id);
            }
            if rd.features.iter().find(|g| g.id == f.id).is_none_or(|g| !same(g, f)) {
                set.insert(f.id);
            }
        }
        for _ in 0..2 {
            let mut dirty_bodies: Vec<Id> = vec![];
            for f in &d.features {
                let mut ids = vec![];
                f.kind.clone().walk(&mut |x| ids.push(*x), &mut |_| {});
                let sketch = matches!(f.kind, FeatureKind::Sketch(_));
                if set.contains(&f.id) || ids.iter().any(|x| set.contains(x)) || (!sketch && dirty_bodies.contains(&f.target)) {
                    set.insert(f.id);
                    if !sketch {
                        dirty_bodies.push(f.target);
                    }
                }
            }
        }
        (false, set)
    }

    /// A geometria mudou desde o último pedido de cálculo?
    fn needs_recompute(&self) -> bool {
        match &self.sent_doc {
            Some(sd) => geo_stale_from(&self.open.doc, sd).is_some(),
            None => true,
        }
    }

    /// Mostra que o comando `i` ainda não foi atualizado: desatualizado (mudou e não recalculou)
    /// ou ainda não alcançado pela varredura visual depois do cálculo.
    fn row_pending(&self, i: usize, now: f64) -> (bool, bool) {
        let (all, set) = &*self.stale_set.borrow();
        let stale = *all || self.open.doc.features.get(i).is_some_and(|f| set.contains(&f.id));
        let swept = (now - self.sweep_t0) / 0.035 < i as f64 + 1.;
        (stale, swept && !stale)
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
                Pick::Face { body, .. } if body >= crate::picking::SURF_BASE => {
                    if let Some(id) = self.result.as_ref().and_then(|r| crate::picking::surface_of(r, body)) {
                        s.surface = Some(SurfRef::Ref(id));
                    }
                }
                Pick::Face { point, normal, owner, .. } => {
                    s.surface = Some(SurfRef::Face { point: point.map(|v| v as f64), normal: normal.map(|v| v as f64), owner: Some(owner) })
                }
                Pick::Plane(pr) => s.planes.push(pr),
                Pick::Geo(id) => add_ref(&mut s, id),
                Pick::SkCurve { sketch, curve, .. } => s.curves.push(CurveRef::SketchCurve { sketch, curve }),
                Pick::SkPoint { sketch, point, .. } => s.points.push(PointRef::SketchPoint { sketch, point }),
                Pick::CurveVtx { curve, index, .. } => s.points.push(PointRef::CurveVertex { curve, index }),
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
        // curva do wireframe pré-selecionada também serve de perfil
        if let Sel::Ref(id) = self.sel {
            if !need_points && d.reference(id).is_some_and(|r| r.kind.class() == RefClass::Curve) {
                return Some(id);
            }
        }
        // só a pré-seleção: sem escolher sozinho o último sketch (o campo fica para preencher)
        None
    }

    fn command(&mut self, c: Cmd) {
        if let Cmd::Pmi(i) = c {
            crate::pmi_ui::command(self, i);
            return;
        }
        if let Cmd::Dwg(i) = c {
            crate::dwg_ui::command(self, i);
            return;
        }
        if crate::asm_ui::command(self, c) {
            return;
        }
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
            Cmd::StMember | Cmd::StPlate | Cmd::StGusset => self.seed(true),
            Cmd::SplitBody | Cmd::ThickSurface | Cmd::CloseSurface | Cmd::SewSurface | Cmd::BoundaryFill => self.seed(true),
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
        // plano pré-selecionado (árvore ou viewport) — sem ele, os campos de plano ficam a escolher
        let pre_plane = matches!(self.sel, Sel::Origin(_))
            || matches!(self.sel, Sel::Ref(id) if self.doc().reference(id).is_some_and(|r| r.kind.class() == RefClass::Plane))
            || self.picks.iter().any(|p| matches!(p, crate::picking::Pick::Plane(_) | crate::picking::Pick::Face { .. }));
        let opt_plane = if pre_plane { plane } else { crate::slots::plane() };
        let up_to = opt_plane;
        let sk_or_unset = sketch.unwrap_or(crate::slots::UNSET_SKETCH);
        if c == Cmd::Sketch && !pre_plane {
            // sketch precisa de um plano: espera o clique num plano/face (ou na árvore)
            self.sketch_wait_plane = true;
            self.toast("Escolha o plano ou a face do sketch (na viewport ou na árvore) — Esc cancela", false);
            return;
        }
        let d = &mut self.open.doc;
        let new_sel = match c {
            Cmd::Sketch => {
                let on_face = matches!(plane, PlaneRef::Datum(id) if d.reference(id).is_some_and(|r| matches!(r.kind, RefKind::FacePlane { .. })));
                let geo = match sel_now {
                    Sel::Folder(fo) if d.folder(fo).is_some_and(|x| x.kind == FolderKind::Geometria) => Some(fo),
                    _ if self.settings.sketch_in_geometry && !on_face => Some(
                        d.folders.iter().find(|f| f.kind == FolderKind::Geometria).map(|f| f.id).unwrap_or_else(|| d.add_geometry_folder("Geometria")),
                    ),
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
                let sk = sk_or_unset;
                let cut = matches!(c, Cmd::Cut | Cmd::CutBlind | Cmd::CutUpTo);
                let extent = match c {
                    Cmd::Extrude => Extent::Blind(s("20")),
                    Cmd::ExtrudeMid => Extent::Symmetric(s("20")),
                    Cmd::CutBlind => Extent::Blind(s("5")),
                    Cmd::ExtrudeUpTo | Cmd::CutUpTo => Extent::UpTo(up_to),
                    _ => Extent::ThroughAll,
                };
                Sel::Feature(d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: if cut { BoolOp::Cut } else { BoolOp::Add }, extent, reverse: false, taper: s("0"), thin: None, dir: None })))
            }
            Cmd::Revolve | Cmd::Groove => {
                let sk = sk_or_unset;
                let axis = self.seed(true).curves.first().map(crate::geo_ui::axis_of).unwrap_or_else(crate::slots::axis);
                let d = &mut self.open.doc;
                Sel::Feature(d.add_feature(FeatureKind::Revolve(Revolve {
                    sketch: sk,
                    axis,
                    angle: s("360"),
                    symmetric: false,
                    op: if c == Cmd::Groove { BoolOp::Cut } else { BoolOp::Add },
                    thin: None,
                    angle2: None,
                })))
            }
            Cmd::Rib | Cmd::Slot => {
                let sk = sk_or_unset;
                let spine = seed.curves.first().cloned().unwrap_or_else(crate::slots::curve);
                Sel::Feature(d.add_feature(FeatureKind::Sweep(Sweep {
                    sketch: sk,
                    spine,
                    mode: SweepMode::Corrigido,
                    op: if c == Cmd::Slot { BoolOp::Cut } else { BoolOp::Add },
                    thin: None,
                })))
            }
            Cmd::Loft | Cmd::LoftCut => {
                // só o sketch pré-selecionado; os outros entram clicando na árvore
                let sections: Vec<Id> = sketch.into_iter().collect();
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
            Cmd::Pmi(_) | Cmd::Dwg(_) => return,
            Cmd::StLibrary => {
                self.st_lib_open = true;
                return;
            }
            Cmd::StCutList => {
                self.st_cut_open = true;
                return;
            }
            Cmd::StMember => {
                let paths: Vec<CurveRef> = seed.curves.clone();
                let fam = crate::st_ui::families(&self.settings).into_iter().next().unwrap();
                let (des, dims) = fam.sizes[3.min(fam.sizes.len() - 1)].clone();
                let rule = self.settings.user_profiles.iter().find(|u| u.name == fam.name).map(|u| (u.anchor, u.angle)).unwrap_or((0, 0.));
                if paths.is_empty() {
                    self.toast("Selecione as linhas do esqueleto (arestas, linhas ou curvas de referência) e depois Membro", true);
                }
                let d = &mut self.open.doc;
                Sel::Feature(d.add_feature(FeatureKind::StructMember(StructMember {
                    paths,
                    family: fam.name.clone(),
                    designation: des,
                    kind: fam.kind,
                    dims: dims.iter().map(|v| fmt(*v)).collect(),
                    anchor: rule.0,
                    angle: fmt(rule.1),
                    off_x: s("0"),
                    off_y: s("0"),
                    ext1: s("0"),
                    ext2: s("0"),
                    corner: CornerMode::MeiaEsquadria,
                    gap: s("0"),
                })))
            }
            Cmd::StPlate | Cmd::StGusset => {
                let at = seed.points.iter().find_map(|p| match p {
                    PointRef::Fixed(q) => Some(*q),
                    _ => None,
                });
                let Some(at) = at else {
                    self.toast("Selecione o vértice da ponta (placa) ou do encontro dos membros (reforço)", true);
                    return;
                };
                Sel::Feature(d.add_feature(if c == Cmd::StPlate {
                    FeatureKind::StructPlate(StructPlate { at, thickness: s("8"), margin: s("10") })
                } else {
                    FeatureKind::StructGusset(StructGusset { at, leg: s("80"), thickness: s("6"), chamfer: s("10") })
                }))
            }
            Cmd::SmParams => {
                if d.sheet.is_none() {
                    d.sheet = Some(SheetParams::default());
                }
                self.sheet_open = true;
                return;
            }
            Cmd::SmUnfold => {
                let sp = d.sheet.get_or_insert_with(SheetParams::default);
                sp.flat = !sp.flat;
                let msg = if sp.flat { "Planificada (desdobrada) — o mesmo comando dobra de novo" } else { "Dobrada" };
                self.toast(msg, false);
                return;
            }
            Cmd::SmWall | Cmd::SmStamp | Cmd::SmFlangedHole => {
                if d.sheet.is_none() {
                    d.sheet = Some(SheetParams::default());
                }
                let sk = sk_or_unset;
                Sel::Feature(d.add_feature(match c {
                    Cmd::SmWall => FeatureKind::SmWall(SmWall { sketch: sk, reverse: false }),
                    Cmd::SmStamp => FeatureKind::SmStamp(SmStamp { sketch: sk, depth: s("3"), flip: false }),
                    _ => FeatureKind::SmFlangedHole(SmFlangedHole { sketch: sk, diameter: s("8"), height: s("4"), flip: false }),
                }))
            }
            Cmd::SmFlange | Cmd::SmHem | Cmd::SmJoggle | Cmd::SmBend | Cmd::SmLouver => {
                if d.sheet.is_none() {
                    d.sheet = Some(SheetParams::default());
                }
                use crate::picking::Pick;
                let edge = self.picks.iter().find_map(|p| match p {
                    Pick::Edge { mid, .. } => Some(mid.map(|v| v as f64)),
                    _ => None,
                });
                let verts: Vec<[f64; 3]> = self.picks.iter().filter_map(|p| match p {
                    Pick::Vertex { pos, .. } => Some(pos.map(|v| v as f64)),
                    _ => None,
                }).collect();
                let d = &mut self.open.doc;
                Sel::Feature(d.add_feature(match c {
                    Cmd::SmFlange => FeatureKind::SmFlange(SmFlange {
                        edge: edge.unwrap_or([0.; 3]),
                        length: s("20"),
                        angle: s("90"),
                        radius: None,
                        off1: s("0"),
                        off2: s("0"),
                        flip: false,
                        relief: None,
                    }),
                    Cmd::SmHem => FeatureKind::SmHem(SmHem { edge: edge.unwrap_or([0.; 3]), length: s("8"), gap: s("0"), flip: false }),
                    Cmd::SmBend => FeatureKind::SmBend(SmBend {
                        a: verts.first().copied().unwrap_or([0.; 3]),
                        b: verts.get(1).copied().unwrap_or([0., 10., 0.]),
                        angle: s("90"),
                        radius: None,
                        flip: false,
                        down: false,
                    }),
                    Cmd::SmLouver => FeatureKind::SmLouver(SmLouver {
                        a: verts.first().copied().unwrap_or([0.; 3]),
                        b: verts.get(1).copied().unwrap_or([0., 10., 0.]),
                        width: s("12"),
                        height: s("5"),
                        flip: false,
                        down: false,
                    }),
                    _ => FeatureKind::SmJoggle(SmJoggle {
                        a: verts.first().copied().unwrap_or([0.; 3]),
                        b: verts.get(1).copied().unwrap_or([0., 10., 0.]),
                        depth: s("3"),
                        run: s("6"),
                        flip: false,
                    }),
                }))
            }
            Cmd::Stiffener => {
                let sk = sk_or_unset;
                Sel::Feature(d.add_feature(FeatureKind::Stiffener(Stiffener { sketch: sk, thickness: s("3"), depth: s("20"), reverse: false, to_body: false })))
            }
            Cmd::Shell => Sel::Feature(d.add_feature(FeatureKind::Shell(Shell { faces: faces.clone(), thickness: s("2"), outward: false }))),
            Cmd::Draft => Sel::Feature(d.add_feature(FeatureKind::Draft(Draft { faces: faces.clone(), neutral: opt_plane, angle: s("3"), reverse: false }))),
            Cmd::Thickness => Sel::Feature(d.add_feature(FeatureKind::Thickness(Thickness { faces: faces.clone(), value: s("2") }))),
            Cmd::Thread => Sel::Feature(d.add_feature(FeatureKind::Thread(Thread { face: faces.last().copied().unwrap_or([0.; 3]), spec: s("M10x1.5"), length: s("15"), modeled: false, end: None }))),
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
                    None => FeatureKind::Mirror(Mirror { plane: opt_plane, sources }),
                }))
            }
            Cmd::Scale => Sel::Feature(d.add_feature(FeatureKind::Scale(Scale { center: PointRef::Origin, factor: s("2") }))),
            Cmd::MoveBody(i) => Sel::Feature(d.add_feature(FeatureKind::MoveBody(MoveBody { xf: crate::feat_ui::default_move(i as usize) }))),
            Cmd::ExtrudeDraftFillet => {
                // atalho de manufatura: extrusão com saída + filete nas arestas criadas por ela
                let sk = sk_or_unset;
                let ex = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("20")), reverse: false, taper: s("3"), thin: None, dir: None }));
                d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("1"), edges: EdgeSel::CreatedBy(ex), mode: EdgeMode::Constant }));
                Sel::Feature(ex)
            }
            Cmd::SplitBody | Cmd::ThickSurface | Cmd::CloseSurface | Cmd::SewSurface | Cmd::BoundaryFill => {
                // superfície da seleção (ou a última superfície de referência; senão, o plano)
                let surface = seed.surface.or_else(|| seed.planes.first().map(|p| SurfRef::Plane(*p))).unwrap_or_else(crate::slots::surf);
                Sel::Feature(d.add_feature(match c {
                    Cmd::SplitBody => FeatureKind::SplitBody(SplitBody { surface, flip: false }),
                    Cmd::ThickSurface => FeatureKind::ThickSurface(ThickSurface { surface, t1: s("2"), t2: s("0"), op: BoolOp::Add }),
                    Cmd::CloseSurface => FeatureKind::CloseSurface(CloseSurface { surface, tol: s("0.01"), op: BoolOp::Add }),
                    Cmd::SewSurface => FeatureKind::SewSurface(SewSurface { surface, remove: false, flip: false }),
                    _ => {
                        let mut tools = vec![surface];
                        tools.extend(seed.planes.iter().map(|p| SurfRef::Plane(*p)));
                        FeatureKind::BoundaryFill(BoundaryFill { tools, exclude: vec![], op: BoolOp::Add })
                    }
                }))
            }
            Cmd::PointRepeat => {
                let Some(curve) = self.seed(true).curves.first().cloned() else {
                    self.toast("Selecione uma curva (aresta, linha, curva do wireframe ou do sketch) e use Repetir pontos", true);
                    return;
                };
                self.repeat = Some(Repeat { curve, count: "5".into(), spacing: "10".into(), by_spacing: false, ends: true, planes: false });
                return;
            }
            Cmd::Extract => {
                self.extract = Some(self.extract_last);
                return;
            }
            Cmd::MeshImport => {
                let Some(p) = rfd::FileDialog::new().add_filter("Malha", &["stl", "obj"]).pick_file() else { return };
                Sel::Feature(d.add_feature(FeatureKind::MeshImport(MeshImport { path: p.to_string_lossy().into(), solid: true })))
            }
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
                // vértices / pontos selecionados: um furo em cada, normal à superfície (sem sketch)
                // centros: sketch com pontos pré-selecionado, ou vértices/pontos clicados; senão a escolher
                let pts3: Vec<PointRef> = self.seed(true).points;
                let sk = match self.selected_sketch(true) {
                    Some(sk) => sk,
                    None if !pts3.is_empty() => Id(0),
                    None => crate::slots::UNSET_SKETCH,
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
                    points: if sk == Id(0) { pts3 } else { vec![] }, dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false,
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
            // comandos de montagem são tratados em asm_ui::command
            Cmd::AsmNew | Cmd::AsmOpen | Cmd::AsmSave | Cmd::AsmInsert | Cmd::AsmSubAsm | Cmd::AsmNewPart | Cmd::AsmGroup | Cmd::AsmReplace | Cmd::AsmMate(_) | Cmd::AsmFix | Cmd::AsmRigid
            | Cmd::AsmSolve | Cmd::AsmNumber | Cmd::AsmInterf | Cmd::AsmPattern | Cmd::AsmJoint(_) | Cmd::AsmAutoJoints | Cmd::AsmSimulate | Cmd::AsmComposer => return,
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
        if let Sel::Feature(id) | Sel::Ref(id) = new_sel {
            // entradas consumidas (sketch do comando, superfícies juntadas/divididas…) ficam ocultas;
            // voltam a aparecer se a operação for apagada
            self.open.doc.hide_consumed(id);
            if scene::is_sketch_feature(&self.open.doc, id) {
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
            Act::SelMode(m) => {
                self.settings.box_mode = m;
                let msg = ["Clique; Ctrl + arrastar faz caixa", "Arrastar faz caixa janela (inteiro dentro)", "Arrastar faz caixa cruzada (toca)", "Arrastar faz caixa: direita = janela, esquerda = cruzada"][m.min(3) as usize];
                self.toast(msg, false);
            }
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
            Act::Display(m) => {
                self.settings.display = m;
                self.toast(m.label(), false);
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
        self.tree_peek = false;
        self.reframe_pending = true;
        self.sel = if self.open.doc.reference(id).is_some() { Sel::Ref(id) } else { Sel::Feature(id) };
    }

    /// Concluir: mantém as alterações.
    pub fn conclude_session(&mut self) {
        if let Some(s) = self.session.take() {
            self.open.doc.hide_consumed(s.id);
            // concluir sempre atualiza (também no modo manual)
            if self.needs_recompute() {
                self.request_recompute();
            }
            self.scene_dirty = true;
        }
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
        let relink = self.relink_ext;
        let title = if relink.is_some() { tr("🔗 Assistente de relink") } else { tr("🔗 Importar publicação") };
        let fechar = egui::Id::unique(("concluir_janela", "app.rs2"));
        if ctx.data_mut(|m| m.remove_temp::<bool>(fechar)).unwrap_or(false) {
            open = false;
        }
        egui::Window::new(title).id(egui::Id::unique("janela_links")).open(&mut open).default_width(420.).show(ctx, |ui| {
            if crate::app::done_bar(ui) {
                ui.ctx().data_mut(|m| m.insert_temp(fechar, true));
            }
            if let Some(e) = relink.and_then(|x| self.open.doc.external.iter().find(|e| e.id == x)) {
                ui.label(RichText::new(format!("Religando {} — hoje: {}.{} ({})", self.open.doc.name_of(e.target.unwrap_or(e.id)), e.source, e.publication, e.path)).strong());
                ui.label(RichText::new(tr("Escolha a peça e a publicação nova (mesmo tipo de geometria). Os comandos que usam o elemento continuam ligados a ele.")).weak().small());
                ui.separator();
            }
            ui.label(RichText::new(tr("Escolha a peça de origem; aparecem os planos, pontos, curvas, superfícies e parâmetros que ela publica (📤).")).weak().small());
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
                            LinkedGeo::Surface(_) => "◫ superfície".into(),
                        };
                        ui.horizontal(|ui| {
                            ui.label(format!("{n}  ({kind})"));
                            if ui.small_button(if relink.is_some() { tr("Religar a este") } else { tr("Importar") }).clicked() {
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
            match relink {
                Some(ext) => {
                    let ok = self.open.doc.external.iter().find(|e| e.id == ext).and_then(|e| e.snapshot.as_ref()).is_none_or(|s| std::mem::discriminant(s) == std::mem::discriminant(&g));
                    if ok {
                        self.open.doc.relink(ext, src, &path, publ, g);
                        self.toast("Link religado", false);
                        open = false;
                    } else {
                        self.toast("Tipo diferente: escolha uma publicação do mesmo tipo (plano, ponto, curva, superfície ou valor)", true);
                    }
                }
                None => {
                    let id = self.open.doc.add_link(src, &path, publ, g);
                    self.toast(format!("Link criado: {}", self.open.doc.name_of(id)), false);
                }
            }
        }
        if !open {
            self.link_open = false;
            self.link_src = None;
            self.relink_ext = None;
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
        let many: Vec<Id> = self.multi.borrow().clone();
        if many.len() > 1 {
            for id in many.iter().rev() {
                if self.open.doc.folder(*id).is_some() {
                    self.open.doc.remove_folder(*id);
                } else {
                    self.open.doc.delete(*id, false);
                }
            }
            self.multi.borrow_mut().clear();
            self.sel = Sel::None;
            self.toast(format!("{} itens apagados (Ctrl+Z desfaz)", many.len()), false);
            return;
        }
        match self.sel {
            Sel::Pmi(id) => {
                crate::pmi_ui::list_mut(self).retain(|p| p.id != id);
                self.sel = Sel::None;
            }
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
        let dark = root.ctx().global_style().visuals.dark_mode;
        egui::Panel::top("menu").frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 4))).show(root, |ui| {
            // faixa do topo em degradê suave
            let r = ui.max_rect().expand2(egui::vec2(8., 4.));
            let (a, b) = if dark { (Color32::from_rgb(30, 40, 58), Color32::from_rgb(20, 24, 32)) } else { (Color32::from_rgb(236, 242, 250), Color32::from_rgb(214, 224, 238)) };
            let mut m = egui::Mesh::default();
            m.colored_vertex(r.left_top(), a);
            m.colored_vertex(r.right_top(), a);
            m.colored_vertex(r.right_bottom(), b);
            m.colored_vertex(r.left_bottom(), b);
            m.add_triangle(0, 1, 2);
            m.add_triangle(0, 2, 3);
            ui.painter().add(egui::Shape::mesh(m));
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
                use crate::settings::Bench;
                ui.add_space(10.);
                for (b, n, icon) in [(Bench::Solidos, "Modelagem Sólida", "▣"), (Bench::Superficies, "Superfícies", "◫"), (Bench::Montagem, "Montagem", "⚙"), (Bench::Chapa, "Chapa metálica", "▭"), (Bench::Estrutural, "Estrutural", "⌶"), (Bench::Desenho, "Desenho", "📐")] {
                    if bench_tab(ui, self.settings.bench == b, &format!("{icon}  {}", tr(n)), true).on_hover_text(tr("As bancadas compartilham o mesmo modelo: sólidos e superfícies se conversam.")).clicked() {
                        self.settings.bench = b;
                    }
                }
                if self.asm.as_ref().is_some_and(|s| s.editing_part.is_some()) && self.settings.bench != Bench::Montagem {
                    ui.add_space(8.);
                    if ui.button(RichText::new(tr("↩ Voltar à montagem")).strong().color(Color32::from_rgb(120, 190, 255))).on_hover_text(tr("salva a peça e volta para a montagem")).clicked() {
                        crate::asm_ui::back_to_assembly(self);
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let asm_title = self.asm.as_ref().filter(|_| self.settings.bench == Bench::Montagem).map(|s| format!("⚙ {}{}", s.asm.name, if s.modified { " •" } else { "" }));
                    let asm_title = asm_title.or_else(|| self.dwg.as_ref().filter(|_| self.settings.bench == Bench::Desenho).map(|s| format!("📐 {}{}", s.d.name, if s.modified { " •" } else { "" })));
                    let title = match (&self.open.frozen, self.open.modified) {
                        _ if asm_title.is_some() => asm_title.clone().unwrap(),
                        (Some(r), _) => format!("{} — revisão {r} (congelada)", self.open.doc.name),
                        (None, true) => format!("{} •", self.open.doc.name),
                        (None, false) => self.open.doc.name.clone(),
                    };
                    // tema dos menus (faixas de cor por grupo)
                    let themes = ["Padrão", "Grafite", "Carmim", "Azul"];
                    let cur = self.settings.menu_theme.min(3) as usize;
                    ui.menu_button(RichText::new(format!("🎨 {}", tr(themes[cur]))).small(), |ui| {
                        ui.label(RichText::new(tr("Tema dos menus")).strong());
                        for (i, n) in themes.iter().enumerate() {
                            if ui.radio(cur == i, tr(n)).clicked() {
                                self.settings.menu_theme = i as u8;
                                ui.close();
                            }
                        }
                    });
                    ui.label(RichText::new(title).weak());
                });
            });
        });

        let sketching = self.sketch.is_some();
        let ts = self.tool_state();
        let top = toolbar::groups_at(&self.settings, Dock::Topo, sketching);
        if !top.is_empty() {
            let mut acts = vec![];
            let fill = if dark { Color32::from_rgb(24, 27, 34) } else { Color32::from_rgb(244, 247, 251) };
            egui::Panel::top("comandos").frame(egui::Frame::new().fill(fill).inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 2 })).show(root, |ui| {
                // uma linha só, com barra de rolagem horizontal fixa embaixo (não quebra linha);
                // a roda do mouse (vertical) rola a barra para os lados
                if ui.rect_contains_pointer(ui.max_rect()) {
                    ui.ctx().input_mut(|i| {
                        if i.smooth_scroll_delta.x == 0. {
                            i.smooth_scroll_delta.x = i.smooth_scroll_delta.y;
                            i.smooth_scroll_delta.y = 0.;
                        }
                    });
                }
                egui::ScrollArea::horizontal()
                    .id_salt("rolagem_comandos")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let gap = |ui: &mut egui::Ui| {
                                ui.add_space(8.);
                                ui.separator();
                                ui.add_space(8.);
                            };
                            for (i, g) in top.iter().enumerate() {
                                if i > 0 {
                                    gap(ui);
                                }
                                acts.extend(toolbar::show_group(ui, *g, &mut self.settings, &ts, false));
                            }
                            // grupos montados pelo usuário
                            let mine: Vec<crate::settings::CustomGroup> = self.settings.custom.groups.iter().filter(|c| c.sketch == sketching && !c.items.is_empty()).cloned().collect();
                            for cg in &mine {
                                gap(ui);
                                acts.extend(toolbar::show_custom(ui, cg, &mut self.settings, &ts, false));
                            }
                        });
                        ui.add_space(4.);
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
        let right = toolbar::groups_at(&self.settings, Dock::Direita, sketching);
        if !right.is_empty() {
            egui::Panel::right("barra_direita").resizable(false).show(root, |ui| {
                egui::ScrollArea::vertical().id_salt("barra_dir").show(ui, |ui| {
                    for (i, g) in right.iter().enumerate() {
                        if i > 0 {
                            ui.separator();
                        }
                        acts.extend(toolbar::show_group(ui, *g, &mut self.settings, &ts, true));
                    }
                });
            });
        }
        let ctx = root.ctx().clone();
        let area = ctx.content_rect();
        let dragging = ctx.input(|i| i.pointer.primary_down());
        for (i, g) in toolbar::groups_at(&self.settings, Dock::Flutuante, sketching).into_iter().enumerate() {
            let w = egui::Window::new(g.label())
                .id(egui::Id::unique(("flutuante", g.label())))
                // empilhados na borda esquerda da viewport, sem cobrir uns aos outros
                .default_pos(egui::pos2(290., 140. + 120. * i as f32))
                .resizable(false)
                .collapsible(true)
                .show(&ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tr("📌 fixar em:")).small().weak());
                        for (d, n) in [(Dock::Topo, "topo"), (Dock::Lateral, "lateral"), (Dock::Direita, "direita")] {
                            if ui.small_button(tr(n)).clicked() {
                                acts.push(Act::Dock(g, d));
                            }
                        }
                    });
                    acts.extend(toolbar::show_group(ui, g, &mut self.settings, &ts, false));
                });
            // encaixe por proximidade: soltar perto do topo ou de uma lateral fixa o grupo ali
            if let Some(w) = w {
                let r = w.response.rect;
                let snap = 36.;
                if w.response.dragged() {
                    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("docas_possiveis")));
                    for (zone, name) in [
                        (Rect::from_min_size(area.left_top() + egui::vec2(0., 50.), egui::vec2(area.width(), 70.)), "⬆ topo"),
                        (Rect::from_min_size(area.left_top() + egui::vec2(0., 130.), egui::vec2(60., area.height() - 160.)), "⬅ lateral"),
                        (Rect::from_min_size(area.right_top() + egui::vec2(-60., 130.), egui::vec2(60., area.height() - 160.)), "➡ direita"),
                    ] {
                        p.rect_filled(zone, 6., Color32::from_rgba_unmultiplied(90, 170, 255, 28));
                        p.rect_stroke(zone, 6., egui::Stroke::new(1.5, Color32::from_rgba_unmultiplied(90, 170, 255, 120)), egui::StrokeKind::Inside);
                        p.text(zone.center(), egui::Align2::CENTER_CENTER, tr(name), egui::FontId::proportional(13.), Color32::from_rgba_unmultiplied(160, 205, 255, 220));
                    }
                }
                let to = if r.top() < area.top() + 60. + snap {
                    Some(Dock::Topo)
                } else if r.left() < area.left() + snap {
                    Some(Dock::Lateral)
                } else if r.right() > area.right() - snap {
                    Some(Dock::Direita)
                } else {
                    None
                };
                if let Some(d) = to {
                    if dragging && w.response.dragged() {
                        // guia visual enquanto arrasta
                        let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("guia_encaixe")));
                        let guide = match d {
                            Dock::Topo => Rect::from_min_size(area.left_top() + egui::vec2(0., 56.), egui::vec2(area.width(), 6.)),
                            Dock::Lateral => Rect::from_min_size(area.left_top(), egui::vec2(6., area.height())),
                            _ => Rect::from_min_size(area.right_top() - egui::vec2(6., 0.), egui::vec2(6., area.height())),
                        };
                        p.rect_filled(guide, 2., Color32::from_rgba_unmultiplied(90, 170, 255, 180));
                    }
                    // só encaixa ao terminar de arrastar a janela (clique em botão não conta)
                    let key = egui::Id::unique(("arrastando_grupo", g.label()));
                    if w.response.dragged() {
                        ctx.data_mut(|m| m.insert_temp(key, true));
                    }
                    if ctx.input(|i| i.pointer.any_released()) && ctx.data(|m| m.get_temp::<bool>(key)).unwrap_or(false) {
                        ctx.data_mut(|m| m.remove::<bool>(key));
                        acts.push(Act::Dock(g, d));
                    }
                }
            }
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

    /// Janela de propriedades gráficas do objeto (cor, acabamento, textura, visibilidade), separada
    /// do comando: aberta pelo botão direito na árvore. Posição e tamanho ficam salvos.
    fn props_window(&mut self, ctx: &egui::Context) {
        let Some(t) = self.props_target else { return };
        let tex: Vec<String> = self.open.doc.textures.iter().map(|x| x.name.clone()).collect();
        let d = &mut self.open.doc;
        let name = match t {
            AppTarget::Part => d.name.clone(),
            AppTarget::Collector(id) | AppTarget::Feature(id) | AppTarget::Ref(id) => d.name_of(id),
            AppTarget::Elem(id) => format!("{} #{}", tr("elemento"), id.0),
        };
        let mut open = true;
        let fechar = egui::Id::unique(("concluir_janela", "app.rs4"));
        if ctx.data_mut(|m| m.remove_temp::<bool>(fechar)).unwrap_or(false) {
            open = false;
        }
        egui::Window::new(format!("🎨 {} — {name}", tr("Propriedades gráficas")))
            .id(egui::Id::unique("janela_props"))
            .open(&mut open)
            .default_width(300.)
            .resizable(true)
            .show(ctx, |ui| {
            if crate::app::done_bar(ui) {
                ui.ctx().data_mut(|m| m.insert_temp(fechar, true));
            }
                match t {
                    AppTarget::Elem(id) => {
                        // curva/ponto de sketch: espessura, tipo de linha e símbolo próprios
                        let cur = d.line_style(id);
                        let mut own = cur.is_some();
                        ui.checkbox(&mut own, tr("estilo próprio de linha/ponto"));
                        if own {
                            let mut st = cur.unwrap_or(scc_core::LineStyle { width: 2.5, dash: 0, point: 1, size: 8. });
                            line_style_ui(ui, &mut st, true, true);
                            d.set_line_style(id, Some(st));
                        } else if cur.is_some() {
                            d.set_line_style(id, None);
                        }
                    }
                    AppTarget::Part => {
                        ui.label(RichText::new(tr("Aparência da peça inteira (vale sobre os corpos)")).weak().small());
                        editor::appearance_opt(ui, "pg_peca", &mut d.appearance, &tex);
                    }
                    AppTarget::Collector(id) => {
                        if let Some(c) = d.collectors.iter_mut().find(|c| c.id == id) {
                            ui.checkbox(&mut c.visible, tr("visível"));
                            let mut a = Appearance { color: c.color, finish: c.finish, texture: c.texture };
                            editor::appearance_fields(ui, "pg_col", &mut a, &tex);
                            (c.color, c.finish, c.texture) = (a.color, a.finish, a.texture);
                        }
                    }
                    AppTarget::Feature(id) => {
                        ui.label(RichText::new(tr("Cor das faces criadas por este comando (o corpo continua sendo um só).")).weak().small());
                        if let Some(f) = d.feature_mut(id) {
                            editor::appearance_opt(ui, "pg_feat", &mut f.appearance, &tex);
                        }
                    }
                    AppTarget::Ref(id) => {
                        let class = d.reference(id).map(|r| r.kind.class());
                        if let Some(r) = d.references.iter_mut().find(|r| r.id == id) {
                            ui.checkbox(&mut r.visible, tr("visível"));
                            editor::appearance_opt(ui, "pg_ref", &mut r.appearance, &tex);
                        }
                        // espessura, tipo de linha e símbolo próprios (pontos, linhas e curvas)
                        if matches!(class, Some(RefClass::Curve | RefClass::Point)) {
                            ui.separator();
                            let cur = d.line_style(id);
                            let mut own = cur.is_some();
                            ui.checkbox(&mut own, tr("estilo próprio de linha/ponto"));
                            if own {
                                let mut st = cur.unwrap_or(scc_core::LineStyle { width: 2., dash: 0, point: 1, size: 8. });
                                line_style_ui(ui, &mut st, class == Some(RefClass::Curve), class == Some(RefClass::Point));
                                d.set_line_style(id, Some(st));
                            } else if cur.is_some() {
                                d.set_line_style(id, None);
                            }
                        }
                    }
                }
            });
        if !open {
            self.props_target = None;
        }
    }

    /// Parâmetros de chapa (espessura, raio, fator K), estado dobrado/planificado e tabela de dobras.
    fn sheet_window(&mut self, ctx: &egui::Context) {
        if !self.sheet_open {
            return;
        }
        let vars = self.vars();
        let mut open = true;
        let table = scc_solidos::bend_table(&self.open.doc);
        let d = &mut self.open.doc;
        let sp = d.sheet.get_or_insert_with(SheetParams::default);
        let fechar = egui::Id::unique(("concluir_janela", "app.rs1"));
        if ctx.data_mut(|m| m.remove_temp::<bool>(fechar)).unwrap_or(false) {
            open = false;
        }
        egui::Window::new(tr("▭ Chapa metálica"))
            .id(egui::Id::unique("janela_chapa"))
            .open(&mut open)
            .default_width(380.)
            .resizable(true)
            .show(ctx, |ui| {
            if crate::app::done_bar(ui) {
                ui.ctx().data_mut(|m| m.insert_temp(fechar, true));
            }
                editor::expr(ui, "Espessura", &mut sp.thickness, &vars, "mm");
                editor::expr(ui, "Raio de dobra", &mut sp.radius, &vars, "mm");
                editor::expr(ui, "Fator K", &mut sp.k, &vars, "");
                ui.label(RichText::new(tr("Fator K: posição da linha neutra (0 = face interna, 0,5 = meio). Comprimento desenvolvido de cada dobra = ângulo × (R + K·t).")).weak().small());
                ui.horizontal(|ui| {
                    ui.radio_value(&mut sp.flat, false, tr("dobrada"));
                    ui.radio_value(&mut sp.flat, true, tr("planificada"));
                });
                editor::section(ui, "Tabela de dobras");
                if table.is_empty() {
                    ui.label(RichText::new(tr("nenhuma dobra ainda")).weak());
                }
                egui::Grid::new("tab_dobras").striped(true).num_columns(5).show(ui, |ui| {
                    for h in ["Dobra", "Ângulo", "Raio", "Desenvolvido", "Dedução"] {
                        ui.label(RichText::new(tr(h)).strong().small());
                    }
                    ui.end_row();
                    for r in &table {
                        ui.label(RichText::new(&r.name).small());
                        ui.label(RichText::new(format!("{}°", fmt(r.angle))).small());
                        ui.label(RichText::new(fmt(r.radius)).small());
                        ui.label(RichText::new(fmt(r.allowance)).small());
                        ui.label(RichText::new(fmt(r.deduction)).small());
                        ui.end_row();
                    }
                });
            });
        if !open {
            self.sheet_open = false;
        }
    }

    /// Assistente de referências quebradas: lista o que falhou e oferece reeditar, suprimir ou apagar.
    fn wizard_window(&mut self, ctx: &egui::Context) {
        if !self.wizard_open {
            return;
        }
        let Some(r) = &self.result else { return };
        let doc = &self.open.doc;
        let mut errs: Vec<(Id, String, String, bool)> = vec![];
        for f in &doc.features {
            if let Some(Status::Error(m)) = r.status.get(&f.id) {
                errs.push((f.id, f.name.clone(), m.clone(), true));
            }
        }
        for x in &doc.references {
            if let Some(Status::Error(m)) = r.status.get(&x.id) {
                errs.push((x.id, x.name.clone(), m.clone(), false));
            }
        }
        let mut open = true;
        let mut act: Option<(Id, u8)> = None;
        egui::Window::new(tr("🩹 Assistente de reparo"))
            .id(egui::Id::unique("assistente_reparo"))
            .open(&mut open)
            .default_width(460.)
            .resizable(true)
            .show(ctx, |ui| {
                if errs.is_empty() {
                    ui.label(RichText::new(tr("✔ Nada quebrado: todos os comandos e elementos foram calculados.")).color(Color32::from_rgb(120, 210, 130)));
                    return;
                }
                ui.label(RichText::new(tr("Estes itens não puderam ser calculados (referência apagada, face que mudou, entrada inválida). Escolha refazer a referência ou apagar.")).weak());
                ui.add_space(6.);
                egui::ScrollArea::vertical().max_height(380.).show(ui, |ui| {
                    for (id, name, msg, is_feat) in &errs {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(RichText::new(format!("✖ {name}")).strong().color(Color32::from_rgb(255, 130, 110)));
                            ui.label(RichText::new(msg).small());
                            ui.horizontal(|ui| {
                                if ui.button(tr("✏ Refazer referência")).on_hover_text(tr("abre o comando para escolher a nova entrada")).clicked() {
                                    act = Some((*id, 0));
                                }
                                if *is_feat && ui.button(tr("⏸ Suprimir")).clicked() {
                                    act = Some((*id, 1));
                                }
                                if ui.button(tr("🗑 Apagar")).clicked() {
                                    act = Some((*id, 2));
                                }
                            });
                        });
                    }
                });
            });
        match act {
            Some((id, 0)) => self.start_session(id, false, None),
            Some((id, 1)) => {
                if let Some(f) = self.open.doc.feature_mut(id) {
                    f.suppressed = true;
                }
            }
            Some((id, _)) => self.ask_delete(id),
            None => {}
        }
        if !open {
            self.wizard_open = false;
        }
    }

    /// Configurações em páginas (lista com ícones à esquerda, conteúdo com rolagem à direita).
    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        const PAGES: [(&str, &str); 9] = [
            ("🖥", "Interface"),
            ("🎨", "Fundo da viewport"),
            ("⌨", "Atalhos"),
            ("🖱", "Navegação e seleção"),
            ("👁", "Exibição"),
            ("🧰", "Barras e painéis"),
            ("⟳", "Recálculo"),
            ("📐", "Cotagem"),
            ("✏", "Sketch"),
        ];
        let mut open = true;
        let page = &mut self.settings_page;
        let s = &mut self.settings;
        egui::Window::new(tr("⚙ Configurações"))
            .id(egui::Id::unique("janela_config"))
            .open(&mut open)
            .default_size([760., 520.])
            .min_size([560., 360.])
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    // lista de páginas
                    ui.vertical(|ui| {
                        ui.set_width(190.);
                        for (k, (icon, name)) in PAGES.iter().enumerate() {
                            let b = egui::Button::selectable(*page == k, RichText::new(format!("{icon}  {}", tr(name))).size(14.)).min_size(egui::vec2(186., 30.));
                            if ui.add(b).clicked() {
                                *page = k;
                            }
                        }
                        ui.add_space(12.);
                        if ui.button(tr("Restaurar padrões")).clicked() {
                            *s = Settings::default();
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.heading(format!("{}  {}", PAGES[*page].0, tr(PAGES[*page].1)));
                        ui.separator();
                        egui::ScrollArea::vertical().id_salt("pagina_config").auto_shrink([false, false]).show(ui, |ui| settings_page(ui, *page, s));
                    });
                });
            });
        self.settings_open = open;
    }

    fn tree(&mut self, ui: &mut egui::Ui) {
        *self.stale_set.borrow_mut() = self.compute_stale();
        // ordem das linhas do quadro anterior (para o Shift+clique)
        let prev = std::mem::take(&mut *self.tree_order.borrow_mut());
        if !prev.is_empty() {
            *self.last_order.borrow_mut() = prev;
        }
        let doc = self.open.doc.clone();
        let sel = self.sel;
        let mut new_sel = None;
        let mut actions: Vec<TreeAction> = vec![];

        let pick = |ui: &mut egui::Ui, s: Sel, text: RichText| -> egui::Response { ui.selectable_label(sel == s, text) };
        // seções que o usuário ocultou (botão direito na peça → Seções da árvore)
        let hidden_secs = self.settings.tree_hidden.clone();
        let hide = |k: &str| hidden_secs.iter().any(|x| x == k);

        // ---------- peça
        let r = pick(ui, Sel::Part, RichText::new(format!("🧊 {}", doc.name)).strong());
        if r.clicked() {
            new_sel = Some(Sel::Part);
        }
        r.context_menu(|ui| {
            if ui.button(tr("🎨 Propriedades gráficas…")).clicked() {
                actions.push(TreeAction::Props(AppTarget::Part));
                ui.close();
            }
            appearance_menu(ui, doc.appearance, AppTarget::Part, &mut actions);
            if ui.button(tr("📁 Nova pasta de geometria")).clicked() {
                actions.push(TreeAction::NewGeometryFolder);
                ui.close();
            }
            if ui.button(tr("▣ Novo coletor de corpos")).clicked() {
                actions.push(TreeAction::NewCollector);
                ui.close();
            }
            ui.menu_button(tr("👁 Seções da árvore"), |ui| {
                for (k, n) in TREE_SECTIONS {
                    let mut on = !hidden_secs.iter().any(|x| x == k);
                    if ui.checkbox(&mut on, tr(n)).changed() {
                        actions.push(TreeAction::ToggleSection(k.to_string()));
                    }
                }
            });
        });
        if !doc.props.pn.is_empty() {
            ui.label(RichText::new(format!("   PN {} · {}", doc.props.pn, doc.props.part_type.label())).weak().small());
        }

        if !hide("origem") {
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
        }

        if !hide("externas") {
        // ---------- referências externas (sempre separadas)
        let h = egui::CollapsingHeader::new(format!("🔗 {} ({})", tr("Referências externas"), doc.external.len())).default_open(false).show(ui, |ui| {
            if doc.external.is_empty() {
                ui.label(RichText::new(tr("Geometria importada de outras peças por link (planos, curvas, pontos e parâmetros publicados). Fica sempre separada da geometria própria.")).weak().small());
            }
            for e in &doc.external {
                let st = match e.state {
                    LinkState::Ativo => "●",
                    LinkState::Congelado => "❄",
                    LinkState::Quebrado => "⚠",
                    LinkState::Desativado => "⏸",
                };
                let txt = format!("🔗 {st} {} ← {}.{}", e.target.map(|t| doc.name_of(t)).unwrap_or_default(), e.source, e.publication);
                let r = ui.selectable_label(e.target.is_some_and(|t| self.sel == Sel::Ref(t)), txt).on_hover_text(format!("caminho: {}", e.path));
                if r.clicked() {
                    if let Some(t) = e.target {
                        new_sel = Some(if doc.reference(t).is_some() { Sel::Ref(t) } else { Sel::Params });
                    }
                }
                let (ext, state, path) = (e.id, e.state, e.path.clone());
                r.context_menu(|ui| {
                    ui.label(RichText::new(crate::geo_ui::link_state_label(state).0).weak());
                    ui.label(RichText::new(format!("📂 {path}")).weak().small());
                    ui.separator();
                    crate::geo_ui::link_actions(ui, ext, state);
                });
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
        h.header_response.context_menu(|ui| {
            if ui.button(tr("➕ Importar publicação de outra peça…")).clicked() {
                actions.push(TreeAction::ImportLink);
                ui.close();
            }
            if ui.add_enabled(!doc.external.is_empty(), egui::Button::new(tr("⟳ Atualizar todos os links"))).clicked() {
                actions.push(TreeAction::RefreshLinks);
                ui.close();
            }
            ui.separator();
            if ui.button(tr("📤 Publicações desta peça…")).clicked() {
                actions.push(TreeAction::OpenPublications);
                ui.close();
            }
        });
        }

        if !hide("publicacoes") {
        // ---------- publicações desta peça (o que outras peças podem importar)
        let n_pub = doc.references.iter().filter(|r| r.published).count() + doc.params.iter().filter(|p| p.published).count() + doc.collectors.iter().filter(|c| c.published).count();
        let h = egui::CollapsingHeader::new(format!("📤 {} ({n_pub})", tr("Publicações"))).id_salt("arvore_pub").default_open(false).show(ui, |ui| {
            if n_pub == 0 {
                ui.label(RichText::new(tr("Nada publicado. Clique direito → Nova publicação para escolher planos, pontos, curvas, superfícies, corpos e parâmetros que outras peças podem usar por link.")).weak().small());
            }
            for r in doc.references.iter().filter(|r| r.published) {
                let resp = ui.selectable_label(self.sel == Sel::Ref(r.id), format!("📤 {}", r.name));
                if resp.clicked() {
                    new_sel = Some(Sel::Ref(r.id));
                }
                resp.context_menu(|ui| {
                    if ui.button(tr("✎ Renomear (o nome é a publicação)")).clicked() {
                        actions.push(TreeAction::StartRename(r.id));
                        ui.close();
                    }
                    if ui.button(tr("Despublicar")).clicked() {
                        actions.push(TreeAction::Publish(r.id, false));
                        ui.close();
                    }
                });
            }
            for c in doc.collectors.iter().filter(|c| c.published) {
                let resp = ui.selectable_label(self.sel == Sel::Collector(c.id), format!("📤 ▣ {}", c.name));
                if resp.clicked() {
                    new_sel = Some(Sel::Collector(c.id));
                }
                resp.context_menu(|ui| {
                    if ui.button(tr("Despublicar")).clicked() {
                        actions.push(TreeAction::CollectorPublish(c.id, false));
                        ui.close();
                    }
                });
            }
            for p in doc.params.iter().filter(|p| p.published) {
                let resp = ui.selectable_label(false, format!("📤 ƒ {}", p.name));
                if resp.clicked() {
                    new_sel = Some(Sel::Params);
                }
                resp.context_menu(|ui| {
                    if ui.button(tr("Despublicar")).clicked() {
                        actions.push(TreeAction::ParamPublish(p.id, false));
                        ui.close();
                    }
                });
            }
        });
        h.header_response.context_menu(|ui| {
            if ui.button(tr("➕ Nova publicação…")).clicked() {
                actions.push(TreeAction::OpenPublications);
                ui.close();
            }
        });
        }

        if !hide("pmi") {
        // ---------- PMI (detalhamento 3D)
        crate::pmi_ui::tree_section(self, ui);
        }

        if !hide("parametros") {
        // ---------- parâmetros
        let h = egui::CollapsingHeader::new(format!("ƒ {} ({})", tr("Parâmetros"), doc.params.len())).default_open(false).show(ui, |ui| {
            let vars = self.vars();
            for p in &doc.params {
                let v = vars.get(&p.name).map(|v| format!("{} {}", fmt(*v), p.unit.symbol())).unwrap_or("⚠".into());
                let r = pick(ui, Sel::Params, RichText::new(format!("{}{} = {v}", if p.published { "📤 " } else { "" }, p.name)));
                if r.clicked() {
                    new_sel = Some(Sel::Params);
                }
                r.context_menu(|ui| {
                    ui.label(RichText::new(&p.name).strong());
                    if ui.button(tr("✏ Editar na tabela")).clicked() {
                        new_sel = Some(Sel::Params);
                        ui.close();
                    }
                    let lbl = if p.published { tr("Despublicar") } else { tr("📤 Publicar para outras peças") };
                    if ui.button(lbl).clicked() {
                        actions.push(TreeAction::ParamPublish(p.id, !p.published));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(tr("🗑 Apagar parâmetro")).clicked() {
                        actions.push(TreeAction::ParamDelete(p.id));
                        ui.close();
                    }
                });
            }
            if doc.params.is_empty() && pick(ui, Sel::Params, RichText::new(tr("vazio — clique para editar")).weak().italics()).clicked() {
                new_sel = Some(Sel::Params);
            }
        });
        h.header_response.context_menu(|ui| {
            if ui.button(tr("➕ Novo parâmetro")).clicked() {
                actions.push(TreeAction::Run(Cmd::Param));
                ui.close();
            }
            if ui.button(tr("✏ Editar tabela de parâmetros")).clicked() {
                new_sel = Some(Sel::Params);
                ui.close();
            }
        });
        }

        if !hide("vistas") {
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
        }

        if !hide("geometria") {
        // ---------- pastas de geometria (sem linha do tempo), com subpastas
        let first_geo = doc.folders.iter().find(|f| f.kind == FolderKind::Geometria).map(|f| f.id);
        for fo in doc.folders.iter().filter(|f| f.kind == FolderKind::Geometria && f.parent.is_none_or(|p| doc.folder(p).is_none())) {
            self.geo_folder(ui, &doc, fo, first_geo, &mut new_sel, &mut actions);
        }
        // planos antigos sem pasta, quando ainda não existe pasta de geometria
        if first_geo.is_none() && !doc.references.is_empty() {
            let h = egui::CollapsingHeader::new(tr("📁 Geometria")).default_open(true).show(ui, |ui| {
                for r in &doc.references {
                    self.ref_row(ui, &doc, r, &mut new_sel, &mut actions);
                }
            });
            h.header_response.context_menu(|ui| {
                if ui.button(tr("📁 Nova pasta de geometria")).clicked() {
                    actions.push(TreeAction::NewGeometryFolder);
                    ui.close();
                }
            });
        }
        if ui.small_button(tr("➕ Pasta de geometria")).on_hover_text(tr("Planos, pontos, curvas e sketches livres, organizados como você quiser (sem linha do tempo)")).clicked() {
            actions.push(TreeAction::NewGeometryFolder);
        }
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
                    let resp = self.row_label(ui, c.id, &c.name, sel == Sel::Collector(c.id), txt, &mut actions);
                    if resp.clicked() {
                        new_sel = Some(Sel::Collector(c.id));
                    }
                    if resp.double_clicked() {
                        actions.push(TreeAction::Activate(c.id));
                    }
                    resp.context_menu(|ui| {
                        ui.label(RichText::new(&c.name).strong());
                        if ui.button(tr("✎ Renomear   F2")).clicked() {
                            actions.push(TreeAction::StartRename(c.id));
                            ui.close();
                        }
                        ui.menu_button(tr("👁 Exibição"), |ui| {
                            if ui.button(if c.visible { tr("Ocultar corpo   H") } else { tr("Mostrar corpo   H") }).clicked() {
                                actions.push(TreeAction::CollectorVisible(c.id, !c.visible));
                                ui.close();
                            }
                            if ui.button(tr("🎨 Propriedades gráficas…")).clicked() {
                                actions.push(TreeAction::Props(AppTarget::Collector(c.id)));
                                ui.close();
                            }
                        });
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
                        let lbl = if c.published { tr("Despublicar corpo") } else { tr("📤 Publicar corpo para outras peças") };
                        if ui.button(lbl).clicked() {
                            actions.push(TreeAction::CollectorPublish(c.id, !c.published));
                            ui.close();
                        }
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
                                h.header_response.context_menu(|ui| {
                                    ui.label(RichText::new(&name).strong());
                                    if ui.button(tr("✎ Renomear   F2")).clicked() {
                                        actions.push(TreeAction::StartRename(fid));
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui.button(tr("Remover pasta (os comandos ficam)")).clicked() {
                                        actions.push(TreeAction::RemoveFolder(fid));
                                        ui.close();
                                    }
                                });
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
            let r = pick(ui, Sel::Folder(fo.id), RichText::new(format!("🗀 {} (vazia)", fo.name)).weak());
            if r.clicked() {
                new_sel = Some(Sel::Folder(fo.id));
            }
            r.context_menu(|ui| {
                if ui.button(tr("✎ Renomear   F2")).clicked() {
                    actions.push(TreeAction::StartRename(fo.id));
                    ui.close();
                }
                if ui.button(tr("🗑 Apagar pasta")).clicked() {
                    actions.push(TreeAction::RemoveFolder(fo.id));
                    ui.close();
                }
            });
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
                let all_hidden = n > 0 && refs.iter().all(|r| !r.visible) && sketches.iter().all(|(_, f)| f.hidden);
                if eye(ui, all_hidden) {
                    actions.push(TreeAction::FolderShown(fo.id, all_hidden));
                }
                let r = self.row_label(ui, fo.id, &fo.name, self.sel == Sel::Folder(fo.id), RichText::new(format!("📁 {}  ({n})", fo.name)).strong(), actions);
                let r = drag_item(ui, r, fo.id, self.renaming != Some(fo.id));
                if r.double_clicked() {
                    actions.push(TreeAction::StartRename(fo.id));
                }
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
                    ui.label(RichText::new(&fo.name).strong());
                    if ui.button(tr("✎ Renomear   F2")).clicked() {
                        actions.push(TreeAction::StartRename(fo.id));
                        ui.close();
                    }
                    if ui.button(if all_hidden { tr("👁 Mostrar o conteúdo") } else { tr("👁 Ocultar o conteúdo") }).clicked() {
                        actions.push(TreeAction::FolderShown(fo.id, all_hidden));
                        ui.close();
                    }
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
                    if ui.button(tr("🗑 Apagar pasta e todo o conteúdo")).clicked() {
                        actions.push(TreeAction::DeleteFolderAll(fo.id));
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

    /// Rótulo da linha da árvore, ou o campo de renomear (F2) quando este item está sendo renomeado.
    fn row_label(&self, ui: &mut egui::Ui, id: Id, name: &str, selected: bool, text: RichText, actions: &mut Vec<TreeAction>) -> egui::Response {
        if self.renaming == Some(id) {
            let key = egui::Id::unique(("renomear", id.0));
            let mut buf: String = ui.ctx().data(|m| m.get_temp(key)).unwrap_or_else(|| name.to_string());
            let r = ui.add(egui::TextEdit::singleline(&mut buf).desired_width(150.));
            if !r.has_focus() && !r.lost_focus() {
                r.request_focus();
            }
            let (enter, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
            if enter || (r.lost_focus() && !esc) {
                actions.push(TreeAction::Rename(id, buf.clone()));
                ui.ctx().data_mut(|m| m.remove::<String>(key));
            } else if esc {
                actions.push(TreeAction::RenameCancel);
                ui.ctx().data_mut(|m| m.remove::<String>(key));
            } else {
                ui.ctx().data_mut(|m| m.insert_temp(key, buf));
            }
            return r;
        }
        self.tree_order.borrow_mut().push(id);
        let in_multi = self.multi.borrow().contains(&id) && self.multi.borrow().len() > 1;
        let r = ui.selectable_label(selected || in_multi, text);
        if r.hovered() {
            self.hover_now.set(Some(id));
        }
        if r.clicked() {
            // Ctrl soma/tira, Shift seleciona a faixa desde o último clicado, sem tecla = só este
            let (ctrl, shift) = ui.input(|i| (i.modifiers.command, i.modifiers.shift));
            let mut m = self.multi.borrow_mut();
            if ctrl {
                if let Some(k) = m.iter().position(|x| *x == id) {
                    m.remove(k);
                } else {
                    m.push(id);
                }
                self.multi_anchor.set(Some(id));
            } else if shift {
                let order = self.tree_order.borrow();
                // a ordem desta passada ainda está sendo montada: usa a do quadro anterior junto
                let last = self.last_order.borrow();
                let all: Vec<Id> = if last.contains(&id) { last.clone() } else { order.clone() };
                let a = self.multi_anchor.get().and_then(|x| all.iter().position(|y| *y == x));
                let b = all.iter().position(|y| *y == id);
                if let (Some(a), Some(b)) = (a, b) {
                    let (lo, hi) = (a.min(b), a.max(b));
                    *m = all[lo..=hi].to_vec();
                } else {
                    *m = vec![id];
                }
            } else {
                *m = vec![id];
                self.multi_anchor.set(Some(id));
            }
        }
        r
    }

    fn ref_row(&self, ui: &mut egui::Ui, doc: &Document, r: &RefElement, new_sel: &mut Option<Sel>, actions: &mut Vec<TreeAction>) {
        let users = doc.dependents(r.id).len();
        let class = r.kind.class();
        let icon = match (&r.kind, class) {
            (RefKind::AnglePlane { .. }, _) => "◩",
            (RefKind::FacePlane { .. }, _) => "▣",
            (_, RefClass::Point) => "•",
            (_, RefClass::Surface) => "◫",
            (RefKind::LinePoints { .. } | RefKind::LineDir { .. } | RefKind::LineAngle { .. } | RefKind::LineNormalSurface { .. }, _) => "╱",
            (_, RefClass::Curve) => "∿",
            _ => "▱",
        };
        let err = matches!(self.status_of(r.id), Some(Status::Error(_)));
        let stale = {
            let (all, set) = &*self.stale_set.borrow();
            *all || set.contains(&r.id)
        };
        let txt = format!(
            "{icon} {}{}{}{}",
            r.name,
            if r.published { " 📤" } else { "" },
            if users > 0 { format!("  ·{users}") } else { String::new() },
            if err { " ✖" } else if stale { " ⟳" } else { "" }
        );
        let resp = ui
            .horizontal(|ui| {
                if eye(ui, !r.visible) {
                    actions.push(TreeAction::RefVisible(r.id, !r.visible));
                }
                if let Some(a) = r.appearance {
                    let (rr, _) = ui.allocate_exact_size(egui::vec2(8., 8.), egui::Sense::hover());
                    ui.painter().circle_filled(rr.center(), 4., Color32::from_rgb(a.color[0], a.color[1], a.color[2]));
                }
                let t = if err || stale {
                    RichText::new(txt).color(Color32::from_rgb(255, 110, 100))
                } else if r.visible {
                    RichText::new(txt)
                } else {
                    RichText::new(txt).weak()
                };
                self.row_label(ui, r.id, &r.name, self.sel == Sel::Ref(r.id), t, actions)
            })
            .inner;
        let resp = drag_item(ui, resp, r.id, self.renaming != Some(r.id));
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
            ui.label(RichText::new(&r.name).strong());
            if ui.button(tr("✏ Editar   (duplo clique)")).clicked() {
                actions.push(TreeAction::EditSketch(r.id));
                ui.close();
            }
            if ui.button(tr("✎ Renomear   F2")).clicked() {
                actions.push(TreeAction::StartRename(r.id));
                ui.close();
            }
            ui.menu_button(tr("👁 Exibição"), |ui| {
                if ui.button(if r.visible { tr("Ocultar   H") } else { tr("Mostrar   H") }).clicked() {
                    actions.push(TreeAction::RefVisible(r.id, !r.visible));
                    ui.close();
                }
                appearance_menu(ui, r.appearance, AppTarget::Ref(r.id), actions);
                if ui.button(tr("🎨 Propriedades gráficas…")).clicked() {
                    actions.push(TreeAction::Props(AppTarget::Ref(r.id)));
                    ui.close();
                }
            });
            ui.menu_button(tr("🔗 Publicação"), |ui| {
                let lbl = if r.published { tr("Despublicar") } else { tr("📤 Publicar para outras peças") };
                if ui.button(lbl).clicked() {
                    actions.push(TreeAction::Publish(r.id, !r.published));
                    ui.close();
                }
                ui.label(RichText::new(tr("Publicado: outras peças importam este elemento por link (Referências externas).")).weak().small());
            });
            if class == RefClass::Plane && ui.button(tr("✏ Novo sketch neste plano")).clicked() {
                actions.push(TreeAction::SketchOn(PlaneRef::Datum(r.id)));
                ui.close();
            }
            ui.menu_button(tr("📁 Organizar"), |ui| clip_menu(ui, r.id, doc, actions));
            ui.separator();
            if ui.button(tr("🗑 Apagar   Del")).clicked() {
                actions.push(TreeAction::Delete(r.id));
                ui.close();
            }
        });
    }

    fn feature_row(&self, ui: &mut egui::Ui, doc: &Document, i: usize, f: &Feature, new_sel: &mut Option<Sel>, actions: &mut Vec<TreeAction>) {
        let st = self.status_of(f.id);
        let now = ui.input(|x| x.time);
        let (stale, sweeping) = self.row_pending(i, now);
        let (badge, color) = match st {
            _ if stale => (" ⟳", Color32::from_rgb(255, 110, 100)),
            Some(Status::Error(_)) => (" ✖", Color32::from_rgb(255, 110, 100)),
            _ if sweeping => (" …", Color32::from_rgb(255, 150, 120)),
            Some(Status::Warning(_)) => (" ⚠", Color32::from_rgb(240, 200, 90)),
            Some(Status::Inactive) => ("", Color32::GRAY),
            _ => ("", ui.visuals().text_color()),
        };
        let is_sketch = matches!(f.kind, FeatureKind::Sketch(_));
        let mut text = RichText::new(format!("{} {}{badge}", f.kind.icon(), f.name)).color(color);
        if f.suppressed {
            text = text.strikethrough();
        }
        if is_sketch && f.hidden {
            text = text.italics();
        }
        // entradas do comando (sketch, pontos, curvas, superfícies) como subitens
        let inputs: Vec<Id> = if is_sketch { vec![] } else { Document::refs_of(&f.kind).into_iter().filter(|x| doc.reference(*x).is_some() || doc.feature(*x).is_some_and(|g| matches!(g.kind, FeatureKind::Sketch(_)))).collect() };
        let open_key = ui.make_persistent_id(("entradas", f.id.0));
        let mut open: bool = ui.ctx().data(|m| m.get_temp(open_key)).unwrap_or(false);
        let resp = ui
            .horizontal(|ui| {
                if !inputs.is_empty() {
                    if ui.add(egui::Button::new(RichText::new(if open { "⏷" } else { "⏵" }).size(10.)).frame(false)).on_hover_text(tr("entradas do comando")).clicked() {
                        open = !open;
                        ui.ctx().data_mut(|m| m.insert_temp(open_key, open));
                    }
                } else {
                    ui.add_space(12.);
                }
                // só sketches têm olho: os sólidos aparecem pelo corpo (a última feature junta tudo)
                if is_sketch && eye(ui, f.hidden) {
                    actions.push(TreeAction::Hide(f.id, !f.hidden));
                }
                if let Some(a) = f.appearance {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(8., 8.), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 4., Color32::from_rgb(a.color[0], a.color[1], a.color[2]));
                }
                self.row_label(ui, f.id, &f.name, self.sel == Sel::Feature(f.id), text, actions)
            })
            .inner;
        if resp.double_clicked() {
            actions.push(TreeAction::EditSketch(f.id));
        }
        let free = doc.is_free(f);
        let resp = drag_item(ui, resp, f.id, free && self.renaming != Some(f.id));
        let resp = match st.and_then(|s| s.message()) {
            Some(m) => resp.on_hover_text(m),
            None => resp,
        };
        if resp.clicked() {
            *new_sel = Some(Sel::Feature(f.id));
        }
        resp.context_menu(|ui| {
            ui.label(RichText::new(&f.name).strong());
            if ui.button(tr("✏ Editar   (duplo clique)")).clicked() {
                actions.push(TreeAction::EditSketch(f.id));
                ui.close();
            }
            if ui.button(tr("✎ Renomear   F2")).clicked() {
                actions.push(TreeAction::StartRename(f.id));
                ui.close();
            }
            ui.menu_button(tr("👁 Exibição"), |ui| {
                if is_sketch {
                    if ui.button(if f.hidden { tr("Mostrar   H") } else { tr("Ocultar   H") }).clicked() {
                        actions.push(TreeAction::Hide(f.id, !f.hidden));
                        ui.close();
                    }
                    if ui.button(tr("Alinhar vista ao sketch   N")).clicked() {
                        actions.push(TreeAction::Align(f.id));
                        ui.close();
                    }
                } else {
                    let vis = doc.collectors.iter().find(|c| c.id == f.target).is_some_and(|c| c.visible);
                    if ui.button(if vis { tr("Ocultar o corpo   H") } else { tr("Mostrar o corpo   H") }).clicked() {
                        actions.push(TreeAction::CollectorVisible(f.target, !vis));
                        ui.close();
                    }
                    if ui.button(tr("🎨 Propriedades gráficas…")).clicked() {
                        actions.push(TreeAction::Props(AppTarget::Feature(f.id)));
                        ui.close();
                    }
                    appearance_menu(ui, f.appearance, AppTarget::Feature(f.id), actions);
                }
            });
            ui.menu_button(tr("⚙ Linha do tempo"), |ui| {
                if ui.button(if f.suppressed { tr("Reativar") } else { tr("Suprimir") }).clicked() {
                    actions.push(TreeAction::Suppress(f.id));
                    ui.close();
                }
                if !free && ui.button(tr("Marcador até aqui")).clicked() {
                    let k = doc.features[..=i].iter().filter(|g| g.target == f.target && !doc.is_free(g)).count();
                    actions.push(TreeAction::CollectorMarker(f.target, Some(k)));
                    ui.close();
                }
                ui.horizontal(|ui| {
                    if ui.button(tr("◀ antes")).clicked() && i > 0 {
                        actions.push(TreeAction::Move(f.id, i - 1));
                    }
                    if ui.button(tr("depois ▶")).clicked() {
                        actions.push(TreeAction::Move(f.id, i + 1));
                    }
                });
            });
            ui.menu_button(tr("📁 Organizar"), |ui| {
                if free {
                    clip_menu(ui, f.id, doc, actions);
                    ui.separator();
                }
                ui.label(RichText::new(tr("Coletor (linha do tempo)")).weak().small());
                for c in &doc.collectors {
                    if ui.button(format!("▣ {}", c.name)).clicked() {
                        actions.push(TreeAction::ToCollector(f.id, c.id));
                        ui.close();
                    }
                }
                ui.label(RichText::new(if is_sketch { tr("Pasta (geometria livre / comandos)") } else { tr("Pasta de comandos") }).weak().small());
                for fo in doc.folders.iter().filter(|fo| is_sketch || fo.kind == FolderKind::Comandos) {
                    let icon = if fo.kind == FolderKind::Geometria { "📁" } else { "🗀" };
                    if ui.button(format!("{icon} {}", fo.name)).clicked() {
                        actions.push(TreeAction::ToFolder(f.id, Some(fo.id)));
                        ui.close();
                    }
                }
            });
            if is_sketch && ui.button(tr("⬆ Extrudar este sketch")).clicked() {
                actions.push(TreeAction::Select(f.id, Cmd::Extrude));
                ui.close();
            }
            ui.separator();
            if ui.button(tr("🗑 Apagar   Del")).clicked() {
                actions.push(TreeAction::Delete(f.id));
                ui.close();
            }
        });
        if open {
            ui.indent(("entradas_ind", f.id.0), |ui| {
                for x in &inputs {
                    let name = doc.name_of(*x);
                    let r = ui.selectable_label(false, RichText::new(format!("↳ {name}")).small().weak());
                    if r.hovered() {
                        self.hover_now.set(Some(*x));
                    }
                    if r.clicked() {
                        *new_sel = Some(if doc.reference(*x).is_some() { Sel::Ref(*x) } else { Sel::Feature(*x) });
                    }
                    if r.double_clicked() {
                        actions.push(TreeAction::EditSketch(*x));
                    }
                }
            });
        }
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
            TreeAction::SetAppearance(t, a) if {
                let id = match t {
                    AppTarget::Ref(x) | AppTarget::Feature(x) | AppTarget::Collector(x) => Some(x),
                    AppTarget::Part | AppTarget::Elem(_) => None,
                };
                let m = self.multi.borrow();
                m.len() > 1 && id.is_some_and(|x| m.contains(&x))
            } => {
                // vários selecionados na árvore: a mesma aparência em todos
                let many: Vec<Id> = self.multi.borrow().clone();
                for x in many {
                    if let Some(r) = d.references.iter_mut().find(|r| r.id == x) {
                        r.appearance = a;
                    } else if let (Some(c), Some(a)) = (d.collectors.iter_mut().find(|c| c.id == x), a) {
                        c.color = a.color;
                        c.finish = a.finish;
                    } else if let Some(f) = d.feature_mut(x) {
                        f.appearance = a;
                    }
                }
            }
            TreeAction::SetAppearance(t, a) => match t {
                AppTarget::Part => d.appearance = a,
                AppTarget::Elem(_) => {}
                AppTarget::Ref(id) => {
                    if let Some(r) = d.references.iter_mut().find(|r| r.id == id) {
                        r.appearance = a
                    }
                }
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
                    AppTarget::Part | AppTarget::Elem(_) => Sel::Part,
                    AppTarget::Feature(id) => Sel::Feature(id),
                    AppTarget::Collector(id) => Sel::Collector(id),
                    AppTarget::Ref(id) => Sel::Ref(id),
                };
                self.props_target = Some(t);
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
            TreeAction::StartRename(id) => self.renaming = Some(id),
            TreeAction::RenameCancel => self.renaming = None,
            TreeAction::Rename(id, n) => {
                self.renaming = None;
                if let Err(e) = d.rename_item(id, &n) {
                    self.toast(e.to_string(), true);
                }
            }
            TreeAction::Props(t) => self.props_target = Some(t),
            TreeAction::Publish(id, on) => {
                if let Some(r) = d.references.iter_mut().find(|r| r.id == id) {
                    r.published = on;
                }
            }
            TreeAction::DeleteFolderAll(fo) => {
                let inside: Vec<Id> = d.folders.iter().filter(|f| f.id == fo || d.folder_within(f.id, fo)).map(|f| f.id).collect();
                let items: Vec<Id> = d
                    .references
                    .iter()
                    .filter(|r| r.folder.is_some_and(|x| inside.contains(&x)))
                    .map(|r| r.id)
                    .chain(d.features.iter().filter(|f| f.folder.is_some_and(|x| inside.contains(&x)) && d.is_free(f)).map(|f| f.id))
                    .collect();
                for id in items.into_iter().rev() {
                    d.delete(id, false);
                }
                for f in inside.into_iter().rev() {
                    d.remove_folder(f);
                }
                self.sel = Sel::None;
            }
            TreeAction::Run(c) => self.command(c),
            TreeAction::ParamDelete(id) => {
                let n = d.params.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default();
                let in_feats = serde_json::to_string(&d.features).unwrap_or_default();
                let used = d.params.iter().any(|p| p.id != id && p.expr.contains(&n)) || in_feats.contains(&n);
                if used {
                    self.toast(format!("'{n}' é usado em outras expressões — troque-as antes de apagar"), true);
                } else {
                    d.params.retain(|p| p.id != id);
                }
            }
            TreeAction::ParamPublish(id, on) => {
                if let Some(p) = d.params.iter_mut().find(|p| p.id == id) {
                    p.published = on;
                }
            }
            TreeAction::CollectorPublish(id, on) => {
                if let Some(c) = d.collectors.iter_mut().find(|c| c.id == id) {
                    c.published = on;
                }
            }
            TreeAction::OpenPublications => self.pub_open = true,
            TreeAction::WithPicks(p, c) => {
                self.picks = p;
                self.sel = Sel::None;
                self.command(c);
            }
            TreeAction::Act(x) => self.act(x),
            TreeAction::OrbitCenter(p) => {
                if let Some(vp) = self.viewport.as_mut() {
                    let before = vp.camera;
                    let eye = vp.camera.eye();
                    vp.camera.target = Vec3::from(p);
                    vp.camera.dist = (eye - vp.camera.target).length().max(1.);
                    vp.animate_from(before);
                }
                self.toast("Órbita centrada no ponto escolhido", false);
            }
            TreeAction::ToggleSection(k) => {
                if let Some(i) = self.settings.tree_hidden.iter().position(|x| *x == k) {
                    self.settings.tree_hidden.remove(i);
                } else {
                    self.settings.tree_hidden.push(k);
                }
            }
            TreeAction::SelectSel(s) => self.sel = s,

            TreeAction::FolderShown(fo, on) => {
                let inside: Vec<Id> = d.folders.iter().filter(|f| d.folder_within(f.id, fo)).map(|f| f.id).collect();
                for r in d.references.iter_mut().filter(|r| r.folder.is_some_and(|x| x == fo || inside.contains(&x))) {
                    r.visible = on;
                }
                for f in d.features.iter_mut().filter(|f| f.folder.is_some_and(|x| x == fo || inside.contains(&x))) {
                    f.hidden = !on;
                }
            }
        }
    }

    /// Menu do botão direito na viewport: opções do elemento clicado (face/aresta/vértice do corpo,
    /// referência, plano) ou, no vazio, da vista.
    fn viewport_menu(&self, ui: &mut egui::Ui, r: &RecomputeResult, target: Option<crate::picking::Pick>, acts: &mut Vec<TreeAction>) {
        use crate::picking::Pick;
        let doc = &self.open.doc;
        let item = |ui: &mut egui::Ui, txt: &str, a: TreeAction, acts: &mut Vec<TreeAction>| {
            if ui.button(tr(txt)).clicked() {
                acts.push(a);
                ui.close();
            }
        };
        // ponto 3D do clique (para o centro de órbita)
        let at: Option<[f32; 3]> = match target {
            Some(Pick::Face { point, .. }) => Some(point),
            Some(Pick::Vertex { pos, .. }) => Some(pos),
            Some(Pick::Edge { mid, .. }) => Some(mid),
            Some(Pick::Geo(id)) => r.points.iter().find(|p| p.id == id).map(|p| p.p.map(|v| v as f32)),
            _ => None,
        };
        // subelemento de superfície: menu da superfície (mais as opções de face/aresta)
        if let Some(sid) = match target {
            Some(Pick::Face { body, .. } | Pick::Edge { body, .. } | Pick::Vertex { body, .. }) => crate::picking::surface_of(r, body),
            _ => None,
        } {
            if let Some(p) = target {
                ui.label(RichText::new(p.label(doc)).weak().small());
                if matches!(p, Pick::Face { .. }) {
                    item(ui, "✏ Novo sketch nesta face", TreeAction::WithPicks(vec![p], Cmd::Sketch), acts);
                }
                item(ui, "◫ Extrair", TreeAction::WithPicks(vec![p], Cmd::Extract), acts);
                if let Some(q) = at {
                    item(ui, "⊙ Centro de órbita aqui", TreeAction::OrbitCenter(q), acts);
                }
                ui.separator();
            }
            return self.viewport_menu(ui, r, Some(Pick::Geo(sid)), acts);
        }
        if let Some(q) = at {
            item(ui, "⊙ Centro de órbita aqui", TreeAction::OrbitCenter(q), acts);
        }
        match target {
            Some(p @ (Pick::Face { .. } | Pick::Edge { .. } | Pick::Vertex { .. })) => {
                let body = match p {
                    Pick::Face { body, .. } | Pick::Edge { body, .. } | Pick::Vertex { body, .. } => body,
                    _ => 0,
                };
                let col = r.bodies.get(body).map(|b| b.collector);
                let owner = match p {
                    Pick::Face { owner, .. } => Some(owner),
                    _ => None,
                }
                .filter(|o| doc.feature(*o).is_some());
                ui.label(RichText::new(p.label(doc)).strong());
                if let Some(o) = owner {
                    ui.label(RichText::new(format!("{} {}", tr("criada por"), doc.name_of(o))).weak().small());
                    item(ui, "✏ Editar o comando que criou", TreeAction::EditSketch(o), acts);
                    item(ui, "Selecionar na árvore", TreeAction::SelectSel(Sel::Feature(o)), acts);
                }
                ui.separator();
                if matches!(p, Pick::Face { .. }) {
                    item(ui, "✏ Novo sketch nesta face", TreeAction::WithPicks(vec![p], Cmd::Sketch), acts);
                    item(ui, "▱ Plano nesta face", TreeAction::WithPicks(vec![p], Cmd::Geo(crate::geo_ui::GeoKind::PlaneOffset)), acts);
                    item(ui, "◫ Extrair face", TreeAction::WithPicks(vec![p], Cmd::Extract), acts);
                    item(ui, "▣ Casca a partir desta face", TreeAction::WithPicks(vec![p], Cmd::Shell), acts);
                    item(ui, "⦿ Furo nesta face", TreeAction::WithPicks(vec![p], Cmd::Hole), acts);
                }
                if matches!(p, Pick::Edge { .. }) {
                    item(ui, "◜ Filete nesta aresta", TreeAction::WithPicks(vec![p], Cmd::Fillet), acts);
                    item(ui, "◺ Chanfro nesta aresta", TreeAction::WithPicks(vec![p], Cmd::Chamfer), acts);
                    item(ui, "∿ Extrair aresta (curva)", TreeAction::WithPicks(vec![p], Cmd::Extract), acts);
                }
                if matches!(p, Pick::Vertex { .. }) {
                    item(ui, "• Extrair vértice (ponto)", TreeAction::WithPicks(vec![p], Cmd::Extract), acts);
                    item(ui, "⦿ Furo neste vértice", TreeAction::WithPicks(vec![p], Cmd::Hole), acts);
                }
                if let Some(c) = col {
                    ui.separator();
                    let vis = doc.collectors.iter().find(|x| x.id == c).is_some_and(|x| x.visible);
                    item(ui, if vis { "👁 Ocultar o corpo" } else { "👁 Mostrar o corpo" }, TreeAction::CollectorVisible(c, !vis), acts);
                    item(ui, "🎨 Propriedades gráficas do corpo…", TreeAction::Props(AppTarget::Collector(c)), acts);
                    if let Some(o) = owner {
                        item(ui, "🎨 Cor das faces deste comando…", TreeAction::Props(AppTarget::Feature(o)), acts);
                    }
                }
                if let Some(o) = owner {
                    ui.separator();
                    item(ui, "🗑 Apagar o comando…", TreeAction::Delete(o), acts);
                }
            }
            Some(Pick::Geo(id) | Pick::Plane(PlaneRef::Datum(id))) if doc.reference(id).is_some() => {
                let rf = doc.reference(id).unwrap();
                ui.label(RichText::new(&rf.name).strong());
                item(ui, "✏ Editar", TreeAction::EditSketch(id), acts);
                item(ui, "Selecionar na árvore", TreeAction::SelectSel(Sel::Ref(id)), acts);
                if rf.kind.class() == RefClass::Plane {
                    item(ui, "✏ Novo sketch neste plano", TreeAction::SketchOn(PlaneRef::Datum(id)), acts);
                }
                ui.separator();
                item(ui, if rf.visible { "👁 Ocultar" } else { "👁 Mostrar" }, TreeAction::RefVisible(id, !rf.visible), acts);
                item(ui, "🎨 Propriedades gráficas…", TreeAction::Props(AppTarget::Ref(id)), acts);
                item(ui, if rf.published { "Despublicar" } else { "📤 Publicar para outras peças" }, TreeAction::Publish(id, !rf.published), acts);
                ui.separator();
                item(ui, "🗑 Apagar", TreeAction::Delete(id), acts);
            }
            Some(Pick::SkCurve { sketch, curve, .. }) => {
                ui.label(RichText::new(format!("{} · {}", doc.name_of(sketch), tr("curva"))).strong());
                item(ui, "✏ Editar o sketch", TreeAction::EditSketch(sketch), acts);
                item(ui, "🎨 Estilo desta curva (espessura, tipo de linha)…", TreeAction::Props(AppTarget::Elem(curve)), acts);
                if let Some(p) = target {
                    item(ui, "∿ Superfície extrudada desta curva", TreeAction::WithPicks(vec![p], Cmd::Geo(crate::geo_ui::GeoKind::SurfExtrude)), acts);
                    item(ui, "▱ Plano normal a esta curva", TreeAction::WithPicks(vec![p], Cmd::Geo(crate::geo_ui::GeoKind::PlaneNormalCurve)), acts);
                    item(ui, "• Ponto nesta curva", TreeAction::WithPicks(vec![p], Cmd::Geo(crate::geo_ui::GeoKind::PtOnCurve)), acts);
                }
            }
            Some(p @ (Pick::SkPoint { .. } | Pick::CurveVtx { .. })) => {
                ui.label(RichText::new(p.label(doc)).strong());
                let owner = match p {
                    Pick::SkPoint { sketch, .. } => sketch,
                    Pick::CurveVtx { curve, .. } => curve,
                    _ => Id(0),
                };
                item(ui, "✏ Editar o dono", TreeAction::EditSketch(owner), acts);
                item(ui, "╱ Linha a partir deste ponto", TreeAction::WithPicks(vec![p], Cmd::Geo(crate::geo_ui::GeoKind::LineDir)), acts);
                item(ui, "▱ Plano normal por este ponto", TreeAction::WithPicks(vec![p], Cmd::Geo(crate::geo_ui::GeoKind::PlanePointNormal)), acts);
                item(ui, "⦿ Furo neste ponto", TreeAction::WithPicks(vec![p], Cmd::Hole), acts);
            }
            Some(Pick::Plane(PlaneRef::Origin(o))) => {
                ui.label(RichText::new(format!("Plano {}", crate::scene::plane_name(o))).strong());
                item(ui, "✏ Novo sketch neste plano", TreeAction::SketchOn(PlaneRef::Origin(o)), acts);
                item(ui, "▱ Plano deslocado deste", TreeAction::WithPicks(vec![Pick::Plane(PlaneRef::Origin(o))], Cmd::Geo(crate::geo_ui::GeoKind::PlaneOffset)), acts);
            }
            _ => {
                ui.label(RichText::new(tr("Vista")).strong());
                item(ui, "⤢ Ajustar tudo   F", TreeAction::Act(Act::Fit), acts);
                item(ui, "⟂ Normal à seleção   N", TreeAction::Act(Act::Normal), acts);
                item(ui, if self.settings.ortho { "Perspectiva" } else { "Ortogonal" }, TreeAction::Act(Act::Ortho), acts);
                item(ui, if self.hidden_env { "Sair do ambiente de ocultos" } else { "Ambiente de ocultos" }, TreeAction::Act(Act::HiddenEnv), acts);
                ui.separator();
                item(ui, "✏ Novo sketch…", TreeAction::Run(Cmd::Sketch), acts);
                item(ui, "📁 Nova pasta de geometria", TreeAction::NewGeometryFolder, acts);
                if !self.picks.is_empty() {
                    item(ui, "Limpar seleção   Esc", TreeAction::Act(Act::ClearPicks), acts);
                }
            }
        }
    }

    /// Barra Concluir/Cancelar para o que está aberto fora dos comandos (anotação 3D, parâmetros,
    /// montagem, desenho, espera do plano do sketch). Os comandos e o sketch têm a barra deles.
    fn conclude_bar(&mut self, ui: &mut egui::Ui) {
        use crate::settings::Bench;
        let big = |ui: &mut egui::Ui, t: &str| ui.add(egui::Button::new(RichText::new(tr(t)).strong()).min_size(egui::vec2(110., 28.)));
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter)) && !ui.ctx().egui_wants_keyboard_input();
        match self.settings.bench {
            Bench::Desenho => {
                let Some(st) = self.dwg.as_mut() else { return };
                if !st.active() {
                    return;
                }
                ui.horizontal(|ui| {
                    if big(ui, "✔ Concluir").on_hover_text("Enter").clicked() || enter {
                        st.conclude();
                    }
                    if let Some(id) = st.sel.filter(|_| st.tool.is_none()) {
                        if big(ui, "🗑 Apagar").clicked() {
                            st.d.delete(id);
                            st.conclude();
                        }
                    } else if big(ui, "✖ Cancelar").clicked() {
                        st.conclude();
                    }
                });
            }
            Bench::Montagem => {
                let Some(st) = self.asm.as_mut() else { return };
                if st.sel.is_none() && st.pending.is_none() && st.pending_joint.is_none() {
                    return;
                }
                ui.horizontal(|ui| {
                    if big(ui, "✔ Concluir").on_hover_text("Enter").clicked() || enter {
                        st.sel = None;
                        st.pending = None;
                        st.pending_joint = None;
                    }
                    if (st.pending.is_some() || st.pending_joint.is_some()) && big(ui, "✖ Cancelar").clicked() {
                        st.pending = None;
                        st.pending_joint = None;
                    }
                    if let Some(id) = st.sel.filter(|_| st.pending.is_none() && st.pending_joint.is_none()) {
                        if big(ui, "🗑 Excluir").on_hover_text("Del").clicked() {
                            ui.ctx().data_mut(|m| m.insert_temp(egui::Id::unique("asm_excluir"), id.0));
                        }
                    }
                });
                if let Some(id) = ui.ctx().data_mut(|m| m.remove_temp::<u64>(egui::Id::unique("asm_excluir"))).map(Id) {
                    crate::asm_ui::node_action(self, id, 0);
                }
            }
            _ => {
                if self.session.is_some() || self.sketch.is_some() {
                    return;
                }
                if self.sketch_wait_plane || self.pmi_pending.is_some() {
                    ui.label(RichText::new(tr(if self.sketch_wait_plane { "Escolha o plano ou a face do sketch…" } else { "Escolha os elementos da anotação no 3D…" })).color(Color32::from_rgb(255, 190, 80)));
                    if big(ui, "✖ Cancelar").clicked() {
                        self.sketch_wait_plane = false;
                        self.pmi_pending = None;
                    }
                    ui.separator();
                    return;
                }
                match self.sel {
                    Sel::Pmi(id) => {
                        ui.horizontal(|ui| {
                            if big(ui, "✔ Concluir").on_hover_text("Enter").clicked() || enter {
                                self.sel = Sel::None;
                            }
                            if big(ui, "🗑 Apagar").on_hover_text("Del").clicked() {
                                crate::pmi_ui::list_mut(self).retain(|p| p.id != id);
                                self.sel = Sel::None;
                            }
                        });
                    }
                    Sel::Params => {
                        if big(ui, "✔ Concluir").on_hover_text("Enter").clicked() || enter {
                            self.sel = Sel::None;
                        }
                    }
                    _ => return,
                }
            }
        }
        ui.separator();
    }

    /// Preenche os campos vazios do comando/elemento em edição com a seleção da viewport (`tree`
    /// = None) ou com o item clicado na árvore. Devolve se algo foi preenchido.
    fn fill_from_selection(&mut self, id: Id, tree: Option<Sel>) -> bool {
        use crate::picking::Pick;
        use crate::slots::*;
        let incomplete = match (self.open.doc.feature(id), self.open.doc.reference(id)) {
            (Some(f), _) => feature_incomplete(&f.kind),
            (_, Some(r)) => count_unset(&r.kind) > 0,
            _ => false,
        };
        if !incomplete {
            return false;
        }
        let mut seed = crate::geo_ui::Seed::default();
        let (mut sketch, mut shape) = (None, None);
        match tree {
            None => {
                seed = self.seed(false);
                // face clicada onde se espera um plano: vira plano da face
                let wants_plane = serde_json::to_string(&self.open.doc.feature(id).map(|f| f.kind.clone())).unwrap_or_default().contains(&UNSET_PLANE.0.to_string())
                    || serde_json::to_string(&self.open.doc.reference(id).map(|r| r.kind.clone())).unwrap_or_default().contains(&UNSET_PLANE.0.to_string());
                if wants_plane && seed.planes.is_empty() && self.picks.iter().any(|p| matches!(p, Pick::Face { .. })) {
                    let pl = self.plane_for_new();
                    seed.planes.push(pl);
                } else if wants_plane && seed.planes.is_empty() {
                    // curva plana (aresta, curva do wireframe) onde se espera um plano: o plano da curva,
                    // se nenhum campo de curva/eixo/perfil estiver esperando por ela
                    let js = serde_json::to_string(&self.open.doc.feature(id).map(|f| f.kind.clone())).unwrap_or_default()
                        + &serde_json::to_string(&self.open.doc.reference(id).map(|r| r.kind.clone())).unwrap_or_default();
                    let wants_curve = [UNSET_CURVE, UNSET_AXIS, UNSET_SKETCH].iter().any(|u| js.contains(&u.0.to_string()));
                    let straight = |p: &[[f64; 3]]| {
                        let (a, b) = (p[0], *p.last().unwrap());
                        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                        let l = (ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2]).sqrt();
                        l > 1e-9
                            && p.iter().all(|q| {
                                let c = scc_kernel::cross(ab, [q[0] - a[0], q[1] - a[1], q[2] - a[2]]);
                                (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt() / l < 1e-4 * l + 1e-6
                            })
                    };
                    let usable = |c: &CurveRef| match c {
                        CurveRef::Edge(p) => p.len() >= 3 && !straight(p),
                        CurveRef::Ref(_) | CurveRef::SketchCurve { .. } => true,
                        _ => false,
                    };
                    if !wants_curve {
                        if let Some(i) = seed.curves.iter().position(usable) {
                            let c = seed.curves.remove(i);
                            let pid = self.open.doc.add_geometry(RefKind::PlaneCurve { curve: c });
                            seed.planes.push(PlaneRef::Datum(pid));
                        }
                    }
                }
            }
            Some(Sel::Origin(o)) => seed.planes.push(PlaneRef::Origin(o)),
            Some(Sel::Ref(r)) if r != id => match self.doc().reference(r).map(|x| x.kind.class()) {
                Some(RefClass::Plane) => seed.planes.push(PlaneRef::Datum(r)),
                Some(RefClass::Point) => seed.points.push(PointRef::Ref(r)),
                Some(RefClass::Curve) => seed.curves.push(CurveRef::Ref(r)),
                Some(RefClass::Surface) => seed.surface = Some(SurfRef::Ref(r)),
                None => {}
            },
            Some(Sel::Feature(f)) if f != id => match self.doc().feature(f).map(|x| &x.kind) {
                Some(FeatureKind::Sketch(_)) => sketch = Some(f),
                Some(k) if crate::feat_ui::is_shape(k) => shape = Some(f),
                _ => {}
            },
            _ => return false,
        }
        let edge_mid = self.picks.iter().find_map(|p| if let Pick::Edge { mid, .. } = p { Some(mid.map(|v| v as f64)) } else { None });
        let face_pt = self.picks.iter().find_map(|p| if let Pick::Face { point, .. } = p { Some(point.map(|v| v as f64)) } else { None });
        let verts: Vec<[f64; 3]> = self.picks.iter().filter_map(|p| if let Pick::Vertex { pos, .. } = p { Some(pos.map(|v| v as f64)) } else { None }).collect();
        let mut c = Cands::from_seed(&seed, sketch);
        self.geo_alternatives(&mut c, sketch, tree.is_some());
        let tree_pick = tree.is_some();
        let d = &mut self.open.doc;
        let filled = if let Some(f) = d.feature_mut(id) {
            let a = fill(&mut f.kind, &mut c);
            let b = fill_special(&mut f.kind, &seed, if tree_pick { None } else { edge_mid }, if tree_pick { None } else { face_pt }, &verts, sketch, shape);
            a || b
        } else if let Some(r) = d.references.iter_mut().find(|r| r.id == id) {
            fill(&mut r.kind, &mut c)
        } else {
            false
        };
        if filled {
            d.hide_consumed(id);
            self.scene_dirty = true;
        }
        filled
    }

    /// Alternativas geométricas para os campos que não acharam o tipo exato na seleção: aresta
    /// circular → centro e eixo; face plana → normal; cilindro/cone → eixo; face → ponto clicado;
    /// sketch (ou curva de sketch) → o plano dele.
    fn geo_alternatives(&self, c: &mut crate::slots::Cands, sketch: Option<Id>, tree: bool) {
        use crate::geom_fit::{self, FaceKind};
        use crate::picking::Pick;
        let plane_of = |id: Id| match self.doc().feature(id).map(|f| &f.kind) {
            Some(FeatureKind::Sketch(s)) => Some(s.plane),
            _ => None,
        };
        if let Some(p) = sketch.and_then(plane_of) {
            c.alt_planes.push_back(p);
        }
        if tree {
            return;
        }
        let Some(r) = &self.result else { return };
        for p in &self.picks {
            match *p {
                Pick::Edge { body, edge, .. } => {
                    let pts = crate::picking::edge_polyline(r, body, edge);
                    if let Some((ctr, n, _)) = geom_fit::circle(&pts) {
                        c.alt_points.push_back(PointRef::Fixed(ctr));
                        c.alt_axes.push_back(AxisRef::Fixed { origin: ctr, dir: n });
                    }
                }
                Pick::Face { body, face, point, .. } => {
                    let pt = point.map(|v| v as f64);
                    if let Some(m) = crate::picking::mesh_of(r, body) {
                        let s1 = m.face_ranges.get(face + 1).map(|x| *x as usize).unwrap_or(m.indices.len()).min(m.indices.len());
                        let s0 = (m.face_ranges.get(face).copied().unwrap_or(0) as usize).min(s1);
                        match geom_fit::face(&m.positions, &m.normals, &m.indices[s0..s1]) {
                            FaceKind::Plane(n) => c.alt_axes.push_back(AxisRef::Fixed { origin: pt, dir: n }),
                            FaceKind::Axis(o, d) => c.alt_axes.push_back(AxisRef::Fixed { origin: o, dir: d }),
                            FaceKind::Other => {}
                        }
                    }
                    c.alt_points.push_back(PointRef::Fixed(pt));
                }
                Pick::SkCurve { sketch, .. } | Pick::SkPoint { sketch, .. } => {
                    if let Some(p) = plane_of(sketch) {
                        c.alt_planes.push_back(p);
                    }
                }
                _ => {}
            }
        }
    }

    /// Janela de extração: simples = um elemento por item clicado (com o que a propagação juntar);
    /// múltipla = tudo num elemento só. Propagação por ponto, tangente ou curvatura.
    fn extract_window(&mut self, ctx: &egui::Context) {
        use crate::extract::Prop;
        let Some(mut o) = self.extract else { return };
        let (mut open, mut done) = (true, false);
        let n = self.picks.len();
        egui::Window::new(tr("◫ Extrair")).open(&mut open).default_width(340.).resizable(false).show(ctx, |ui| {
            done = crate::app::done_bar(ui);
            ui.label(RichText::new(format!("{} {}", n, tr("item(ns) selecionado(s) — clique faces, arestas ou vértices no 3D (Shift soma)"))).weak().small());
            ui.horizontal(|ui| {
                ui.radio_value(&mut o.multi, false, tr("simples"))
                    .on_hover_text(tr("Cada item clicado vira um elemento (com o que a propagação juntar a ele)."));
                ui.radio_value(&mut o.multi, true, tr("múltipla"))
                    .on_hover_text(tr("Tudo o que foi clicado vira um elemento só (arestas juntadas numa curva; faces numa superfície)."));
            });
            ui.horizontal(|ui| {
                ui.label(tr("Propagação"));
                for (p, t) in [(Prop::Nenhuma, "nenhuma"), (Prop::Ponto, "ponto"), (Prop::Tangente, "tangente"), (Prop::Curvatura, "curvatura")] {
                    ui.radio_value(&mut o.prop, p, tr(t));
                }
            });
            ui.label(RichText::new(tr("Ponto: tudo o que está ligado. Tangente: continua enquanto não houver canto vivo. Curvatura: também para onde a curvatura muda (nas faces vale como tangente).")).weak().small());
            ui.checkbox(&mut o.publish, tr("publicar o resultado (outras peças podem usar)"));
        });
        self.extract_last = o;
        if !open {
            self.extract = None;
            return;
        }
        self.extract = Some(o);
        if done {
            self.extract = None;
            self.run_extract(o);
        }
    }

    fn run_extract(&mut self, o: ExtractOpts) {
        use crate::extract::{edge_pts, edge_set, face_point, face_set};
        use crate::picking::Pick;
        // o resultado sai emprestado durante a coleta e volta logo depois
        let Some(r) = self.result.take() else { return };
        let (mut edges, mut faces, mut made): (Vec<Vec<Vec<[f64; 3]>>>, Vec<Vec<[f64; 3]>>, Vec<Id>) = (vec![], vec![], vec![]);
        let mut seen_e: Vec<(usize, usize)> = vec![];
        let mut seen_f: Vec<(usize, usize)> = vec![];
        for p in self.picks.clone() {
            match p {
                Pick::Face { body, face, .. } => {
                    let Some(m) = crate::picking::mesh_of(&r, body) else { continue };
                    let set: Vec<[f64; 3]> = face_set(m, face, o.prop)
                        .into_iter()
                        .filter(|f| {
                            let new = !seen_f.contains(&(body, *f));
                            if new {
                                seen_f.push((body, *f));
                            }
                            new
                        })
                        .filter_map(|f| face_point(m, f))
                        .collect();
                    if !set.is_empty() {
                        faces.push(set);
                    }
                }
                Pick::Edge { body, edge, .. } => {
                    let Some(m) = crate::picking::mesh_of(&r, body) else { continue };
                    let set: Vec<Vec<[f64; 3]>> = edge_set(m, edge, o.prop)
                        .into_iter()
                        .filter(|e| {
                            let new = !seen_e.contains(&(body, *e));
                            if new {
                                seen_e.push((body, *e));
                            }
                            new
                        })
                        .map(|e| edge_pts(m, e))
                        .filter(|p| p.len() >= 2)
                        .collect();
                    if !set.is_empty() {
                        edges.push(set);
                    }
                }
                Pick::Vertex { pos, .. } => {
                    let q = pos.map(|v| crate::app::fmt(v as f64));
                    made.push(self.open.doc.add_geometry(RefKind::PointCoords { x: q[0].clone(), y: q[1].clone(), z: q[2].clone(), frame: None }));
                }
                // curva de sketch: cópia associativa
                Pick::SkCurve { sketch, curve, .. } => made.push(self.open.doc.add_geometry(RefKind::CurveCopy { curve: CurveRef::SketchCurve { sketch, curve } })),
                _ => {}
            }
        }
        self.result = Some(r);
        if o.multi {
            let all_e: Vec<Vec<[f64; 3]>> = edges.drain(..).flatten().collect();
            if !all_e.is_empty() {
                edges.push(all_e);
            }
            let all_f: Vec<[f64; 3]> = faces.drain(..).flatten().collect();
            if !all_f.is_empty() {
                faces.push(all_f);
            }
        }
        let d = &mut self.open.doc;
        for set in edges {
            made.push(if set.len() == 1 {
                d.add_geometry(RefKind::CurveCopy { curve: CurveRef::Edge(set.into_iter().next().unwrap()) })
            } else {
                d.add_geometry(RefKind::CurveJoin { curves: set.into_iter().map(CurveRef::Edge).collect() })
            });
        }
        for set in faces {
            made.push(if set.len() == 1 { d.add_geometry(RefKind::SurfExtract { face: set[0], owner: None }) } else { d.add_geometry(RefKind::SurfMulti { faces: set }) });
        }
        if made.is_empty() {
            self.toast("Selecione faces, arestas ou vértices na viewport e conclua a extração", true);
            return;
        }
        if o.publish {
            for r in d.references.iter_mut().filter(|r| made.contains(&r.id)) {
                r.published = true;
            }
        }
        self.toast(format!("{} elemento(s) extraído(s) para a pasta de geometria", made.len()), false);
        self.sel = Sel::Ref(*made.last().unwrap());
        self.picks.clear();
        self.scene_dirty = true;
    }

    /// Repetição de pontos ao longo de uma curva: cada ponto é um "ponto na curva" associativo
    /// (razão ou distância), numa pasta própria; opcionalmente um plano normal à curva em cada um.
    fn repeat_window(&mut self, ctx: &egui::Context) {
        if self.repeat.is_none() {
            return;
        }
        let vars = self.vars();
        let Some(rp) = self.repeat.as_mut() else { return };
        let (mut open, mut done) = (true, false);
        egui::Window::new(tr("⋯ Repetir pontos na curva")).open(&mut open).default_width(340.).resizable(false).show(ctx, |ui| {
            done = crate::app::done_bar(ui);
            ui.label(RichText::new(tr("Pontos associativos: se a curva mudar, eles acompanham.")).weak().small());
            ui.horizontal(|ui| {
                ui.radio_value(&mut rp.by_spacing, false, tr("por quantidade"));
                ui.radio_value(&mut rp.by_spacing, true, tr("por espaçamento"));
            });
            ui.horizontal(|ui| {
                ui.label(tr("Quantidade"));
                let r = ui.add(egui::TextEdit::singleline(&mut rp.count).desired_width(70.));
                crate::ui_ext::wheel_text(ui, &r, &mut rp.count);
            });
            if rp.by_spacing {
                ui.horizontal(|ui| {
                    ui.label(tr("Espaçamento"));
                    let r = ui.add(egui::TextEdit::singleline(&mut rp.spacing).desired_width(70.));
                    crate::ui_ext::wheel_text(ui, &r, &mut rp.spacing);
                    ui.label("mm");
                });
            } else {
                ui.checkbox(&mut rp.ends, tr("incluir as pontas da curva"));
            }
            ui.checkbox(&mut rp.planes, tr("criar também um plano normal à curva em cada ponto"));
        });
        if !open {
            self.repeat = None;
            return;
        }
        if !done {
            return;
        }
        let rp = self.repeat.take().unwrap();
        let n = scc_core::expr::eval_str(&rp.count, &vars).map(|v| v.round() as i64).unwrap_or(0);
        if !(1..=500).contains(&n) {
            self.toast("Quantidade entre 1 e 500", true);
            self.repeat = Some(rp);
            return;
        }
        let n = n as usize;
        let d = &mut self.open.doc;
        let folder = d.add_geometry_folder(&format!("Repetição ({n})"));
        let mut last = None;
        for i in 0..n {
            let at = if rp.by_spacing {
                OnCurve::Distance(format!("({})*{i}", rp.spacing))
            } else if rp.ends {
                OnCurve::Ratio(if n == 1 { "0.5".into() } else { format!("{i}/{}", n - 1) })
            } else {
                OnCurve::Ratio(format!("{}/{}", i + 1, n + 1))
            };
            let p = d.add_geometry(RefKind::PointOnCurve { curve: rp.curve.clone(), at: at.clone(), reverse: false });
            if let Some(r) = d.references.iter_mut().find(|r| r.id == p) {
                r.folder = Some(folder);
            }
            if rp.planes {
                let pl = d.add_geometry(RefKind::PlaneNormalCurve { curve: rp.curve.clone(), at, reverse: false });
                if let Some(r) = d.references.iter_mut().find(|r| r.id == pl) {
                    r.folder = Some(folder);
                }
            }
            last = Some(p);
        }
        if let Some(p) = last {
            self.sel = Sel::Ref(p);
        }
        self.scene_dirty = true;
        self.toast(format!("{n} ponto(s) criados na pasta de geometria"), false);
    }

    /// Publicações: marca planos, pontos, curvas, superfícies, corpos e parâmetros que outras peças
    /// podem importar por link (o nome do elemento é o nome da publicação).
    fn pub_window(&mut self, ctx: &egui::Context) {
        if !self.pub_open {
            return;
        }
        let mut open = true;
        let d = &mut self.open.doc;
        let fechar = egui::Id::unique(("concluir_janela", "app.rs3"));
        if ctx.data_mut(|m| m.remove_temp::<bool>(fechar)).unwrap_or(false) {
            open = false;
        }
        egui::Window::new(tr("📤 Publicações desta peça")).open(&mut open).default_width(420.).resizable(true).show(ctx, |ui| {
            if crate::app::done_bar(ui) {
                ui.ctx().data_mut(|m| m.insert_temp(fechar, true));
            }
            ui.label(RichText::new(tr("Marque o que outras peças podem usar. Na outra peça: Referências externas → Importar publicação. O nome do elemento é o nome da publicação.")).weak().small());
            let mut filter: String = ctx.data(|m| m.get_temp(egui::Id::unique("pub_filtro"))).unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.text_edit_singleline(&mut filter);
            });
            ctx.data_mut(|m| m.insert_temp(egui::Id::unique("pub_filtro"), filter.clone()));
            let ok = |n: &str| filter.is_empty() || n.to_lowercase().contains(&filter.to_lowercase());
            egui::ScrollArea::vertical().max_height(420.).show(ui, |ui| {
                for (class, title) in [(RefClass::Plane, "Planos"), (RefClass::Point, "Pontos"), (RefClass::Curve, "Linhas e curvas"), (RefClass::Surface, "Superfícies")] {
                    let items: Vec<usize> = (0..d.references.len()).filter(|i| d.references[*i].kind.class() == class && ok(&d.references[*i].name)).collect();
                    if items.is_empty() {
                        continue;
                    }
                    ui.label(RichText::new(tr(title)).strong());
                    for i in items {
                        let r = &mut d.references[i];
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut r.published, "");
                            ui.add(egui::TextEdit::singleline(&mut r.name).desired_width(240.));
                        });
                    }
                }
                if !d.collectors.is_empty() {
                    ui.label(RichText::new(tr("Corpos (forma exata)")).strong());
                    for c in d.collectors.iter_mut().filter(|c| ok(&c.name)) {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut c.published, "");
                            ui.add(egui::TextEdit::singleline(&mut c.name).desired_width(240.));
                        });
                    }
                }
                if !d.params.is_empty() {
                    ui.label(RichText::new(tr("Parâmetros (valor)")).strong());
                    for p in d.params.iter_mut().filter(|p| ok(&p.name)) {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut p.published, "");
                            ui.label(&p.name);
                        });
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(tr("Publicar todos os visíveis")).clicked() {
                    for r in d.references.iter_mut().filter(|r| r.visible) {
                        r.published = true;
                    }
                }
                if ui.button(tr("Despublicar tudo")).clicked() {
                    d.references.iter_mut().for_each(|r| r.published = false);
                    d.collectors.iter_mut().for_each(|c| c.published = false);
                    d.params.iter_mut().for_each(|p| p.published = false);
                }
            });
            ui.label(RichText::new(tr("Para valer nas outras peças, salve esta peça.")).weak().small());
        });
        self.pub_open = open;
    }

    /// Alterna ocultar/mostrar o item selecionado (tecla H).
    fn toggle_hide_selected(&mut self) {
        let many: Vec<Id> = self.multi.borrow().clone();
        if many.len() > 1 {
            let d = &mut self.open.doc;
            for id in many {
                if let Some(r) = d.references.iter_mut().find(|r| r.id == id) {
                    r.visible = !r.visible;
                } else if let Some(c) = d.collectors.iter_mut().find(|c| c.id == id) {
                    c.visible = !c.visible;
                } else if let Some(f) = d.feature_mut(id) {
                    f.hidden = !f.hidden;
                }
            }
            return;
        }
        let d = &mut self.open.doc;
        match self.sel {
            Sel::Feature(id) => {
                let target = d.feature(id).filter(|f| !matches!(f.kind, FeatureKind::Sketch(_))).map(|f| f.target);
                match target {
                    // sólidos não se ocultam um a um: oculta o corpo inteiro
                    Some(c) => {
                        if let Some(c) = d.collectors.iter_mut().find(|x| x.id == c) {
                            c.visible = !c.visible
                        }
                    }
                    None => {
                        if let Some(f) = d.feature_mut(id) {
                            f.hidden = !f.hidden
                        }
                    }
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
                let now = ui.input(|i| i.time);
                let n = self.open.doc.features.len();
                let swept = (((now - self.sweep_t0) / 0.035) as usize).min(n);
                let stale = self.stale_from().is_some() && !busy;
                let (txt, col) = if busy {
                    ("⏳ recalculando…".to_string(), Color32::from_rgb(240, 200, 90))
                } else if stale {
                    (tr("⟳ desatualizado — F5 atualiza").to_string(), Color32::from_rgb(255, 120, 100))
                } else if swept < n {
                    (format!("▶ atualizando {swept}/{n}"), Color32::from_rgb(140, 200, 255))
                } else {
                    (format!("✔ recalculado em {ms:.0} ms"), ui.visuals().text_color())
                };
                ui.label(RichText::new(txt).small().color(col));
                ui.separator();
                ui.checkbox(&mut self.settings.auto_update, RichText::new(tr("auto")).small()).on_hover_text(tr("atualizar automaticamente a cada mudança"));
                if ui.small_button(tr("⟳ Atualizar")).on_hover_text(tr("recalcula o que mudou (F5)")).clicked() {
                    self.send_recompute(false);
                }
                if ui.small_button(tr("⟲ Reconstruir")).on_hover_text(tr("reconstrói tudo do zero, sem cache (Ctrl+F5)")).clicked() {
                    self.send_recompute(true);
                }
                // passo a passo: move o marcador do corpo ativo comando a comando
                let act = self.open.doc.active_collector;
                let total = self.open.doc.features.iter().filter(|f| f.target == act && !self.open.doc.is_free(f)).count();
                let cur = self.open.doc.collectors.iter().find(|c| c.id == act).and_then(|c| c.marker).unwrap_or(total).min(total);
                let mut set: Option<Option<usize>> = None;
                if ui.small_button("⏮").on_hover_text(tr("voltar ao início (reconstruir passo a passo)")).clicked() {
                    set = Some(Some(0));
                }
                if ui.small_button("◀").on_hover_text(tr("um comando para trás")).clicked() {
                    set = Some(Some(cur.saturating_sub(1)));
                }
                ui.label(RichText::new(format!("{cur}/{total}")).small().weak());
                if ui.small_button("▶").on_hover_text(tr("reconstruir o próximo comando")).clicked() {
                    set = Some(if cur + 1 >= total { None } else { Some(cur + 1) });
                }
                if ui.small_button("⏭").on_hover_text(tr("até o fim")).clicked() {
                    set = Some(None);
                }
                if let Some(m) = set {
                    if let Some(c) = self.open.doc.collectors.iter_mut().find(|c| c.id == act) {
                        c.marker = m;
                    }
                }
                if let (true, Some(st)) = (self.settings.bench == crate::settings::Bench::Montagem, self.asm.as_ref()) {
                    let r = &st.report;
                    let col = if r.failed.is_empty() { ui.visuals().text_color() } else { Color32::from_rgb(255, 120, 100) };
                    ui.separator();
                    ui.label(RichText::new(format!("⚙ {} GDL livres · {} restrição(ões) com erro", r.dof, r.failed.len())).small().color(col));
                }
                let errs = self.errors_prev;
                if errs > 0 && ui.small_button(RichText::new(format!("✖ {errs} com erro — reparar")).color(Color32::from_rgb(255, 120, 100))).clicked() {
                    self.wizard_open = true;
                }
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
            // Ctrl + arrastar com o esquerdo = seleção em caixa (fora do sketch)
            if resp.drag_started_by(egui::PointerButton::Primary) && (ctrl || self.settings.box_mode > 0) && !sketching {
                self.box_start = ctx.input(|i| i.pointer.press_origin());
            }
            let boxing = self.box_start.is_some();
            let left = left && !boxing;
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
                // só os planos visíveis agora (o resultado traz também os ocultos)
                let d = &self.open.doc;
                let flt = &self.settings.filter;
                let env = self.hidden_env;
                let shown = |hidden: bool| hidden == env;
                for p in &r.planes {
                    let (vis, name) = match p.id {
                        Some(id) => match d.reference(id) {
                            Some(rf) => (flt.planos_ref && shown(!rf.visible), rf.name.clone()),
                            None => (false, String::new()),
                        },
                        None => (flt.planos_origem && d.origin_visible && !env, p.name.clone()),
                    };
                    if !vis {
                        continue;
                    }
                    let f = &p.frame;
                    let at = Vec3::from(f.origin.map(|v| v as f32));
                    let size = vp.bounds.map(|(a, b)| (b - a).max_element().max(20.) * 0.6).unwrap_or(60.);
                    let corner = at + (Vec3::from(f.x.map(|v| v as f32)) + Vec3::from(f.y().map(|v| v as f32))) * size * 0.45;
                    if let Some(pos) = vp.project(rect, corner) {
                        ui.painter().text(pos, egui::Align2::LEFT_BOTTOM, &name, egui::FontId::proportional(11.), ink);
                    }
                }
            }

            // rótulos de pontos e curvas de referência
            if let (Some(r), true) = (&self.result, self.settings.filter.rotulos) {
                let d = &self.open.doc;
                let vis = |id: Id| d.reference(id).filter(|rf| rf.visible != self.hidden_env).map(|rf| rf.name.clone());
                for p in &r.points {
                    let Some(name) = vis(p.id).filter(|_| self.settings.filter.pontos) else { continue };
                    if let Some(pos) = vp.project(rect, Vec3::from(p.p.map(|v| v as f32))) {
                        ui.painter().text(pos + egui::vec2(6., -4.), egui::Align2::LEFT_BOTTOM, &name, egui::FontId::proportional(11.), ink);
                    }
                }
                for c in &r.curves {
                    let Some(name) = vis(c.id).filter(|_| self.settings.filter.planos_ref) else { continue };
                    if c.pts.is_empty() {
                        continue;
                    }
                    let m = c.pts[c.pts.len() / 2];
                    if let Some(pos) = vp.project(rect, Vec3::from(m.map(|v| v as f32))) {
                        ui.painter().text(pos + egui::vec2(6., -4.), egui::Align2::LEFT_BOTTOM, &name, egui::FontId::proportional(11.), ink);
                    }
                }
            }

            let cam = vp.camera;
            // comando em edição: entradas realçadas com o papel de cada uma
            if let (Some(r), Some(sid)) = (&self.result, self.session.as_ref().map(|x| x.id)) {
                crate::roles::draw(&ui.painter_at(rect), &self.open.doc, r, &cam, rect, sid);
            }
            // símbolos dos pontos em tamanho fixo na tela (estilo das configurações / do elemento)
            if let Some(r) = &self.result {
                scene::draw_points(
                    &ui.painter_at(rect),
                    &self.open.doc,
                    r,
                    &cam,
                    rect,
                    self.settings.point_style,
                    self.settings.point_size,
                    &self.settings.filter,
                    self.hidden_env,
                    self.sketch.as_ref().map(|e| e.feature),
                    self.hover_item,
                    &self.sel,
                );
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
                    hidden_env: self.hidden_env,
                    exclude_sketch: None,
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
                } else if let (Some(Some(sid)), true, Some(crate::picking::Pick::Edge { mid, .. })) = (
                    self.session.as_ref().map(|x| x.id).map(|id| self.open.doc.feature(id).filter(|f| matches!(f.kind, FeatureKind::SmFlange(_) | FeatureKind::SmHem(_))).map(|f| f.id)),
                    resp.clicked_by(egui::PointerButton::Primary),
                    self.hover_pick,
                ) {
                    // aba/bainha aberta: clicar numa aresta troca a aresta da dobra
                    let m = mid.map(|v| v as f64);
                    if let Some(f) = self.open.doc.feature_mut(sid) {
                        match &mut f.kind {
                            FeatureKind::SmFlange(fl) => fl.edge = m,
                            FeatureKind::SmHem(h) => h.edge = m,
                            _ => {}
                        }
                    }
                } else if resp.double_clicked_by(egui::PointerButton::Primary) {
                    // duplo clique numa face abre o comando que a criou
                    let target = match self.hover_pick {
                        Some(crate::picking::Pick::Face { owner, .. }) => Some(owner),
                        Some(crate::picking::Pick::Geo(id) | crate::picking::Pick::Plane(PlaneRef::Datum(id))) => Some(id),
                        Some(crate::picking::Pick::Edge { body, .. } | crate::picking::Pick::Vertex { body, .. }) => crate::picking::surface_of(r, body),
                        Some(crate::picking::Pick::SkCurve { sketch, .. } | crate::picking::Pick::SkPoint { sketch, .. }) => Some(sketch),
                        Some(crate::picking::Pick::CurveVtx { curve, .. }) => Some(curve),
                        _ => None,
                    };
                    if let Some(id) = target.filter(|id| self.open.doc.feature(*id).is_some() || self.open.doc.reference(*id).is_some()) {
                        if edge_cmd.is_none() {
                            self.open_pending = Some(id);
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
                // seleção em caixa: janela (da esquerda p/ a direita) ou cruzada (da direita p/ a esquerda)
                if let (Some(a0), Some(now)) = (self.box_start, ctx.input(|i| i.pointer.latest_pos())) {
                    let b = Rect::from_two_pos(a0, now);
                    let crossing = match self.settings.box_mode {
                        1 => false,
                        2 => true,
                        _ => now.x < a0.x,
                    };
                    let (fill, stroke) = if crossing {
                        (Color32::from_rgba_unmultiplied(80, 200, 120, 40), egui::Stroke::new(1.2, Color32::from_rgb(80, 200, 120)))
                    } else {
                        (Color32::from_rgba_unmultiplied(80, 150, 255, 40), egui::Stroke::new(1.2, Color32::from_rgb(80, 150, 255)))
                    };
                    ui.painter().rect_filled(b, 0., fill);
                    if crossing {
                        let pts = [b.left_top(), b.right_top(), b.right_bottom(), b.left_bottom(), b.left_top()];
                        for w in pts.windows(2) {
                            ui.painter().extend(egui::Shape::dashed_line(&[w[0], w[1]], stroke, 6., 4.));
                        }
                    } else {
                        ui.painter().rect_stroke(b, 0., stroke, egui::StrokeKind::Middle);
                    }
                    ui.painter().text(now + egui::vec2(10., 10.), egui::Align2::LEFT_TOP, if crossing { "cruzada (toca)" } else { "janela (inteiro dentro)" }, egui::FontId::proportional(11.), stroke.color);
                    if !ctx.input(|i| i.pointer.primary_down()) {
                        let mut got = crate::picking::box_select(r, &self.open.doc, &cam, rect, b, crossing, &filt);
                        if !ctx.input(|i| i.modifiers.shift) {
                            self.picks.clear();
                        }
                        got.retain(|p| !self.picks.contains(p));
                        let n = got.len();
                        self.picks.extend(got);
                        self.box_start = None;
                        self.toasts.push((format!("{n} elemento(s) na seleção"), 0., false));
                    }
                }
                // botão direito (sem arrastar) no 3D: menu do elemento sob o mouse
                if resp.secondary_clicked() {
                    self.ctx_pick = self.hover_pick;
                }
                // (na montagem a cena é a composição das peças: o menu do 3D é o da montagem)
                if !self.in_asm() {
                    let mut acts: Vec<TreeAction> = vec![];
                    let target = self.ctx_pick;
                    resp.context_menu(|ui| self.viewport_menu(ui, r, target, &mut acts));
                    self.ctx_actions.extend(acts);
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
            if self.sketch.is_none() {
                crate::pmi_ui::overlay(self, ui, rect, &cam);
            }
            // seta e cota 3D do comando aberto (fast 3D)
            if self.sketch.is_none() {
                if self.settings.bench == crate::settings::Bench::Montagem && self.asm.is_some() {
                    crate::asm_ui::gizmo(self, ui, rect, &cam);
                } else {
                    crate::handles::overlay(self, ui, rect, cam);
                }
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
    /// extrair a seleção da viewport como geometria de referência
    Extract,
    /// repetir pontos (e planos normais) ao longo de uma curva
    PointRepeat,
    // ---- montagem
    AsmNew,
    AsmOpen,
    AsmSave,
    AsmInsert,
    AsmSubAsm,
    AsmNewPart,
    AsmGroup,
    AsmReplace,
    AsmMate(crate::asm_ui::MateCmd),
    AsmFix,
    AsmRigid,
    AsmSolve,
    AsmNumber,
    AsmInterf,
    AsmPattern,
    /// junta: 0 revolução … 7 ponto em curva
    AsmJoint(u8),
    AsmAutoJoints,
    AsmSimulate,
    AsmComposer,
    // ---- chapa metálica
    SmParams,
    SmWall,
    SmFlange,
    SmHem,
    SmJoggle,
    SmBend,
    SmLouver,
    SmStamp,
    SmFlangedHole,
    SmUnfold,
    // ---- estrutural
    StMember,
    StPlate,
    StGusset,
    StLibrary,
    StCutList,
    /// anotação 3D: 0 linear … 8 tolerância geométrica
    Pmi(u8),
    /// detalhamento 2D (ver `dwg_ui::c`)
    Dwg(u8),
    /// mover o corpo: 0 translação, 1 rotação, 2 simetria, 3 afinidade
    MoveBody(u8),
    /// extrusão com ângulo de saída e filetes num clique
    ExtrudeDraftFillet,
    SplitBody,
    ThickSurface,
    CloseSurface,
    SewSurface,
    BoundaryFill,
    MeshImport,
    Param,
    Folder,
    Collector,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AppTarget {
    Part,
    Collector(Id),
    Feature(Id),
    Ref(Id),
    /// subelemento (curva/ponto de sketch): só o estilo de linha/ponto
    Elem(Id),
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
        // cores salvas pelo usuário e copiar/colar
        let user = crate::palette::USER.lock().unwrap().clone();
        if !user.is_empty() {
            ui.menu_button(tr("Minhas cores"), |ui| {
                for (n, c) in user {
                    if ui.button(RichText::new(format!("■ {n}")).color(Color32::from_rgb(c[0], c[1], c[2]))).clicked() {
                        let base = current.unwrap_or(APPEARANCE_PRESETS[0].1);
                        actions.push(TreeAction::SetAppearance(t, Some(Appearance { color: c, ..base })));
                        ui.close();
                    }
                }
            });
        }
        if ui.add_enabled(current.is_some(), egui::Button::new(tr("📋 Copiar aparência"))).clicked() {
            *crate::palette::CLIP.lock().unwrap() = current;
            ui.close();
        }
        let clip = *crate::palette::CLIP.lock().unwrap();
        if ui.add_enabled(clip.is_some(), egui::Button::new(tr("📋 Colar aparência (vale para todos os selecionados)"))).clicked() {
            actions.push(TreeAction::SetAppearance(t, clip));
            ui.close();
        }
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
pub fn eye_button(ui: &mut egui::Ui, hidden: bool) -> bool {
    eye(ui, hidden)
}

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
    StartRename(Id),
    Rename(Id, String),
    RenameCancel,
    Props(AppTarget),
    Publish(Id, bool),
    /// mostra/oculta todo o conteúdo de uma pasta de geometria
    FolderShown(Id, bool),
    /// apaga a pasta e tudo o que está nela (subpastas inclusive)
    DeleteFolderAll(Id),
    /// executa um comando da barra
    Run(Cmd),
    ParamDelete(Id),
    ParamPublish(Id, bool),
    CollectorPublish(Id, bool),
    OpenPublications,
    /// mostra/oculta uma seção da árvore (Origem, Parâmetros…)
    ToggleSection(String),
    /// usa estes elementos do 3D como seleção e executa o comando
    WithPicks(Vec<crate::picking::Pick>, Cmd),
    /// ação da barra (vistas, ajustar…)
    Act(Act),
    /// a câmera passa a orbitar em volta deste ponto
    OrbitCenter([f32; 3]),
    /// seleciona na árvore
    SelectSel(Sel),
}

/// Item da árvore de geometria sendo arrastado.
#[derive(Clone, Copy)]
struct DragItem(Id);

/// Botão "Concluir" no topo das janelas de comando: devolve true quando clicado.
/// Opções da extração.
#[derive(Clone, Copy, Default)]
pub struct ExtractOpts {
    pub multi: bool,
    pub prop: crate::extract::Prop,
    pub publish: bool,
}

/// Repetição de pontos ao longo de uma curva (janela em edição).
pub struct Repeat {
    pub curve: CurveRef,
    pub count: String,
    pub spacing: String,
    pub by_spacing: bool,
    pub ends: bool,
    pub planes: bool,
}

pub fn done_bar(ui: &mut egui::Ui) -> bool {
    let r = ui.add(egui::Button::new(RichText::new(tr("✔ Concluir")).strong()).min_size(egui::vec2(100., 26.))).clicked();
    ui.separator();
    r
}

/// Seções da árvore da peça que podem ser ocultadas: (chave, nome).
pub const TREE_SECTIONS: [(&str, &str); 7] = [
    ("origem", "📍 Origem"),
    ("externas", "🔗 Referências externas"),
    ("publicacoes", "📤 Publicações"),
    ("pmi", "📐 PMI"),
    ("parametros", "ƒ Parâmetros"),
    ("vistas", "📷 Vistas salvas"),
    ("geometria", "📁 Pastas de geometria"),
];

/// Primeiro comando cuja geometria difere entre `d` e `rd` (0 = algo global mudou: parâmetros,
/// referências, links, chapa, marcadores); `None` = mesma geometria.
fn geo_stale_from(d: &Document, rd: &Document) -> Option<usize> {
    let refs_eq = d.references.len() == rd.references.len() && d.references.iter().zip(&rd.references).all(|(a, b)| a.id == b.id && a.kind == b.kind);
    let params_eq = d.params.len() == rd.params.len() && d.params.iter().zip(&rd.params).all(|(a, b)| a.name == b.name && a.expr == b.expr && a.unit == b.unit);
    if geo_global_changed(d, rd) || !refs_eq || !params_eq {
        return Some(0);
    }
    let same = |a: &Feature, b: &Feature| a.id == b.id && a.kind == b.kind && a.suppressed == b.suppressed && a.target == b.target && a.folder == b.folder;
    (0..d.features.len())
        .find(|i| rd.features.get(*i).is_none_or(|g| !same(g, &d.features[*i])))
        .or_else(|| (rd.features.len() != d.features.len()).then_some(d.features.len()))
}

/// Mudou algo que vale para o modelo inteiro (parâmetros, referências, links, chapa, marcadores)?
fn geo_global_changed(d: &Document, rd: &Document) -> bool {
    let cols_eq = d.collectors.len() == rd.collectors.len() && d.collectors.iter().zip(&rd.collectors).all(|(a, b)| a.id == b.id && a.marker == b.marker);
    !cols_eq || d.external != rd.external || d.sheet != rd.sheet || d.rollback != rd.rollback
}

/// Torna o próprio rótulo arrastável (clique, duplo clique e botão direito continuam nele — uma
/// área de arraste por cima engolia o menu de contexto).
fn drag_item(ui: &mut egui::Ui, resp: egui::Response, id: Id, enabled: bool) -> egui::Response {
    if !enabled {
        return resp;
    }
    let r = resp.interact(egui::Sense::click_and_drag());
    r.dnd_set_drag_payload(DragItem(id));
    if r.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    r
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
        crate::toolbar::BLOCK_IDX.store(0, std::sync::atomic::Ordering::Relaxed);
        // cores salvas: preferências ↔ paleta usada pelos painéis
        {
            let mut dirty = crate::palette::DIRTY.lock().unwrap();
            let mut user = crate::palette::USER.lock().unwrap();
            if *dirty {
                self.settings.user_colors = user.clone();
                *dirty = false;
            } else if *user != self.settings.user_colors {
                *user = self.settings.user_colors.clone();
            }
        }
        // resultados do recompute
        while let Ok((g, r)) = self.worker.rx.try_recv() {
            if g >= self.gen_done {
                self.gen_done = g;
                let editing = self.session.as_ref().map(|s| s.id);
                let errs = r.status.iter().filter(|(id, s)| Some(**id) != editing && matches!(s, Status::Error(_))).count();
                if errs > self.errors_prev && self.started {
                    // algo quebrou (referência apagada, entrada inválida): abre o assistente
                    self.wizard_open = true;
                }
                self.errors_prev = errs;
                self.result = Some(r);
                if g == self.gen_sent {
                    self.result_doc = self.sent_doc.clone();
                }
                self.sweep_t0 = ctx.input(|i| i.time);
                self.scene_dirty = true;
            }
        }
        if ctx.input(|i| i.time) - self.sweep_t0 < 0.035 * (self.open.doc.features.len() as f64 + 1.) {
            ctx.request_repaint();
        }

        // atalhos (mapa configurável em Configurações → Atalhos e navegação)
        use crate::keymap::KeyAction as K;
        let km = self.settings.keys.clone();
        let typing = ctx.egui_wants_keyboard_input();
        let (save, undo, redo) = (km.pressed(ctx, K::Save), !typing && km.pressed(ctx, K::Undo), !typing && km.pressed(ctx, K::Redo));
        let del = km.pressed(ctx, K::Delete);
        let asm_mode = self.in_asm();
        let dwg_mode = self.settings.bench == crate::settings::Bench::Desenho;
        if dwg_mode {
            crate::dwg_ui::keys(self, ctx, save, undo, redo, del);
        } else if asm_mode {
            if save {
                crate::asm_ui::save(self);
            }
            if let Some(st) = self.asm.as_mut() {
                if undo {
                    if let Some(prev) = st.undo.pop() {
                        st.redo.push(std::mem::replace(&mut st.asm, prev));
                        st.dirty = true;
                    }
                }
                if redo {
                    if let Some(next) = st.redo.pop() {
                        st.undo.push(std::mem::replace(&mut st.asm, next));
                        st.dirty = true;
                    }
                }
                if !typing && ctx.input(|i| i.key_pressed(egui::Key::F2)) {
                    st.renaming = st.sel;
                }
            }
            if del && !ctx.egui_wants_keyboard_input() {
                if let Some(id) = self.asm.as_ref().and_then(|s| s.sel) {
                    crate::asm_ui::node_action(self, id, 0);
                }
            }
            if !typing && km.pressed(ctx, K::Hide) {
                if let Some(id) = self.asm.as_ref().and_then(|s| s.sel) {
                    crate::asm_ui::node_action(self, id, 1);
                }
            }
        } else {
            if save {
                self.save();
            }
            if undo {
                self.undo();
            }
            if redo {
                self.redo();
            }
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
        if del && self.sketch.is_none() && !asm_mode && !dwg_mode && !ctx.egui_wants_keyboard_input() {
            // (fora da janela de comparação do quadro: registra o desfazer aqui)
            let before_del = self.open.doc.clone();
            self.delete_selected();
            if self.open.doc != before_del {
                self.undo.push(before_del);
                self.redo.clear();
                self.open.modified = true;
                self.scene_dirty = true;
            }
        }
        // atalhos de vista e de comandos (só quando não há campo de texto em edição)
        if !typing && !dwg_mode {
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
            // F2 renomeia na árvore; F5 atualiza (no modo manual); Ctrl+F5 reconstrói tudo
            if ctx.input(|i| i.key_pressed(egui::Key::F2)) {
                if let Sel::Feature(id) | Sel::Ref(id) | Sel::Folder(id) | Sel::Collector(id) = self.sel {
                    self.renaming = Some(id);
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
                let fresh = ctx.input(|i| i.modifiers.command);
                self.send_recompute(fresh);
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
        self.hover_now.set(None);
        // comando aberto: o painel de edição fica no lugar da árvore (a seta ⯈ abre a árvore ao lado
        // quando for preciso escolher algo nela; escolhendo no 3D, ela nem precisa aparecer)
        // sem comando aberto, o painel de edição some e a árvore fica sozinha à esquerda
        // (montagem continua com árvore e painel lado a lado)
        let part_bench = self.settings.bench != crate::settings::Bench::Montagem;
        let active = if self.settings.bench == crate::settings::Bench::Desenho {
            self.dwg.as_ref().is_some_and(|s| s.active())
        } else {
            self.session.is_some() || self.sketch.is_some() || self.sketch_wait_plane || self.pmi_pending.is_some() || matches!(self.sel, Sel::Params | Sel::Pmi(_))
        };
        let overlay = active && self.settings.editor_left && part_bench;
        let hide_editor = !active && self.settings.editor_left && part_bench;
        let tree_panel = |app: &mut App, root: &mut egui::Ui| egui::Panel::left("arvore").default_size(270.).resizable(true).show(root, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.left, LeftTab::Peca, tr("Peça"));
                ui.selectable_value(&mut app.left, LeftTab::Projeto, tr("Projeto"));
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| match app.left {
                LeftTab::Peca if app.settings.bench == crate::settings::Bench::Desenho => crate::dwg_ui::tree(app, ui),
                LeftTab::Peca if app.settings.bench == crate::settings::Bench::Montagem => crate::asm_ui::tree(app, ui),
                LeftTab::Peca => app.tree(ui),
                LeftTab::Projeto => app.project_panel(ui),
            });
        });
        if !overlay {
            tree_panel(self, root);
        }
        // painel de edição: junto da árvore (padrão) ou à direita
        let editor_panel = |app: &mut App, ui: &mut egui::Ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Edição")).weak().small());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {

                    let (txt, tip) = if app.settings.editor_left { ("⇥", "levar o painel de edição para a direita") } else { ("⇤", "trazer o painel de edição para a esquerda") };
                    if ui.small_button(txt).on_hover_text(tr(tip)).clicked() {
                        app.settings.editor_left = !app.settings.editor_left;
                    }
                });
            });
            app.conclude_bar(ui);
            egui::ScrollArea::vertical().show(ui, |ui| {
                if app.settings.bench == crate::settings::Bench::Desenho {
                    crate::dwg_ui::panel(app, ui)
                } else if app.settings.bench == crate::settings::Bench::Montagem {
                    crate::asm_ui::panel(app, ui)
                } else if app.sketch.is_some() {
                    crate::sketch_ui::panel(app, ui)
                } else {
                    editor::show(app, ui);
                    crate::measure::panel(app, ui);
                }
            });
        };
        if self.settings.editor_left {
            if !hide_editor {
                egui::Panel::left("editor_esq").default_size(300.).size_range(220.0..=440.0).resizable(true).show(root, |ui| editor_panel(self, ui));
            }
            if overlay {
                // seta na lateral do painel: abre/fecha a árvore à direita dele
                egui::Panel::left("seta_arvore").exact_size(26.).resizable(false).show(root, |ui| {
                    let r = ui.max_rect();
                    let (txt, tip) = if self.tree_peek { ("⯇
🌳", "fechar a árvore") } else { ("⯈
🌳", "abrir a árvore ao lado para escolher itens nela") };
                    let b = egui::Rect::from_center_size(r.center(), egui::vec2(22., 96.));
                    let btn = egui::Button::new(RichText::new(txt).size(15.).color(Color32::WHITE)).fill(Color32::from_rgb(40, 96, 170));
                    if ui.put(b, btn).on_hover_text(tr(tip)).clicked() {
                        self.tree_peek = !self.tree_peek;
                    }
                });
                if self.tree_peek {
                    tree_panel(self, root);
                }
            }
        } else {
            egui::Panel::right("editor").default_size(300.).size_range(220.0..=440.0).resizable(true).show(root, |ui| editor_panel(self, ui));
        }
        // realce dinâmico do item sob o mouse
        if self.hover_now.get() != self.hover_item {
            self.hover_item = self.hover_now.get();
            self.scene_dirty = true;
        }
        if let Some(id) = ctx.data(|m| m.get_temp::<Id>(egui::Id::unique("scc_props"))) {
            ctx.data_mut(|m| m.remove::<Id>(egui::Id::unique("scc_props")));
            self.props_target = Some(if self.open.doc.reference(id).is_some() { AppTarget::Ref(id) } else { AppTarget::Feature(id) });
        }
        self.props_window(ctx);
        self.wizard_window(ctx);
        crate::asm_ui::sim_window(self, ctx);
        crate::asm_ui::composer_window(self, ctx);
        self.sheet_window(ctx);
        crate::st_ui::windows(self, ctx);
        self.side_toolbars(root);
        self.settings_window(ctx);
        self.view_menu_window(ctx);
        self.dialogs(ctx);
        if self.settings.bench == crate::settings::Bench::Desenho {
            crate::dwg_ui::canvas(self, root);
        } else if self.in_asm() {
            let st = self.asm.as_mut().unwrap();
            if st.refresh() {
                self.scene_dirty = true;
            }
            let st = self.asm.as_mut().unwrap();
            std::mem::swap(&mut self.result, &mut st.view);
            std::mem::swap(&mut self.open.doc, &mut st.view_doc);
            self.viewport(root, frame);
            let st = self.asm.as_mut().unwrap();
            std::mem::swap(&mut self.result, &mut st.view);
            std::mem::swap(&mut self.open.doc, &mut st.view_doc);
            crate::asm_ui::after_viewport(self);
            crate::pmi_ui::after_viewport(self);
            if self.asm.as_ref().is_some_and(|s| s.dirty) {
                self.scene_dirty = true;
            }
        } else {
            self.viewport(root, frame);
            crate::pmi_ui::after_viewport(self);
        }
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
                self.scene_dirty = true;
            }
        }
        // atualização automática: compara com o último documento enviado (pega também mudanças
        // feitas fora deste trecho, como Del, desfazer, assistentes e janelas)
        if self.settings.auto_update && self.open.frozen.is_none() && self.needs_recompute() {
            self.request_recompute();
        }
        let link_key = egui::Id::unique("scc_link_acao");
        if let Some((ext, act)) = ctx.data(|m| m.get_temp::<(Id, u8)>(link_key)) {
            ctx.data_mut(|m| m.remove::<(Id, u8)>(link_key));
            match act {
                0 => self.refresh_links(),
                1 | 2 => {
                    let other = if act == 1 { LinkState::Congelado } else { LinkState::Desativado };
                    if act == 2 {
                        // desativar de propósito não deve abrir o assistente de reparo
                        self.errors_prev = usize::MAX;
                    }
                    if let Some(e) = self.open.doc.external.iter_mut().find(|e| e.id == ext) {
                        e.state = if e.state == other { LinkState::Ativo } else { other };
                    }
                }
                3 => {
                    self.open.doc.isolate_link(ext);
                    self.toast("Link isolado: a geometria ficou fixa nesta peça", false);
                }
                _ => {
                    // assistente de relink: a janela de importação passa a religar este link
                    self.relink_ext = Some(ext);
                    self.link_open = true;
                }
            }
        }
        self.link_window(ctx);
        self.pub_window(ctx);
        self.repeat_window(ctx);
        self.extract_window(ctx);
        for x in std::mem::take(&mut self.ctx_actions) {
            self.tree_action(x);
        }
        self.customize_window(ctx);
        self.macro_window(ctx);
        if let Some(id) = self.open_pending.take() {
            self.start_session(id, false, None);
        }
        if let Some(id) = self.begin_sketch_pending.take() {
            self.begin_sketch(id);
        }
        // sketch esperando o plano: escolhido na árvore ou clicado na viewport
        if self.sketch_wait_plane {
            let tree_plane = self.sel != sel_before
                && (matches!(self.sel, Sel::Origin(_)) || matches!(self.sel, Sel::Ref(id) if self.doc().reference(id).is_some_and(|r| r.kind.class() == RefClass::Plane)));
            let pick_plane = self.picks.iter().any(|p| matches!(p, crate::picking::Pick::Plane(_) | crate::picking::Pick::Face { .. }));
            if tree_plane || pick_plane {
                self.sketch_wait_plane = false;
                self.command(Cmd::Sketch);
            } else if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.sketch_wait_plane = false;
            }
        }
        if let Some(sid) = self.session.as_ref().map(|s| s.id) {
            // campos vazios do comando em edição: a seleção (viewport ou árvore) preenche
            if !self.picks.is_empty() && self.fill_from_selection(sid, None) {
                self.picks.clear();
            }
            let still = matches!(self.sel, Sel::Feature(x) | Sel::Ref(x) if x == sid);
            if !still && self.sel != sel_before {
                let tree_sel = self.sel;
                if self.fill_from_selection(sid, Some(tree_sel)) {
                    self.sel = if self.open.doc.reference(sid).is_some() { Sel::Ref(sid) } else { Sel::Feature(sid) };
                } else {
                    self.conclude_session();
                }
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
            let asm_view = if self.in_asm() { self.asm.as_ref().and_then(|s| s.view.as_ref().map(|v| (&s.view_doc, v))) } else { None };
            let (scene_doc, scene_res) = match asm_view {
                Some((d, v)) => (d, Some(v)),
                None => (&self.open.doc, self.result.as_ref()),
            };
            if let (Some(vp), Some(r), Some(rs)) = (self.viewport.as_mut(), scene_res, frame.wgpu_render_state()) {
                let lum = |c: [u8; 3]| 0.3 * c[0] as f32 + 0.59 * c[1] as f32 + 0.11 * c[2] as f32;
                let opts = scene::SceneOpts {
                    editing: self.sketch.as_ref().map(|e| e.feature),
                    sel: &self.sel,
                    filter: &self.settings.filter,
                    hidden_env: self.hidden_env,
                    dark_edges: !matches!(self.settings.display, crate::render::DisplayMode::Arame) || lum(self.settings.bg_bottom) > 140.,
                    hilite: self.hover_item.or(self.session.as_ref().map(|s| s.id)),
                    line_w: self.settings.ref_line_width,
                    line_dash: self.settings.ref_line_dash,
                    point_style: self.settings.point_style,
                };
                let s = scene::build(scene_doc, r, &opts);
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

/// Conteúdo de cada página da janela de configurações.
/// Estilo de linha/ponto (como os tipos de linha do AutoCAD).
pub fn line_style_ui(ui: &mut egui::Ui, st: &mut scc_core::LineStyle, lines: bool, points: bool) {
    if lines {
        ui.horizontal(|ui| {
            ui.label(tr("Espessura"));
            ui.add(egui::Slider::new(&mut st.width, 0.5..=8.0).suffix(" px"));
        });
        ui.horizontal(|ui| {
            ui.label(tr("Tipo de linha"));
            for (i, n) in ["━━━ contínua", "─ ─ tracejada", "─ · ─ traço-ponto", "··· pontilhada"].iter().enumerate() {
                ui.selectable_value(&mut st.dash, i as u8, tr(n));
            }
        });
    }
    if points {
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Símbolo do ponto"));
            for (i, n) in ["✛ cruz 3D", "✳ asterisco", "✕ X", "○ círculo", "□ quadrado", "• ponto"].iter().enumerate() {
                ui.selectable_value(&mut st.point, i as u8, tr(n));
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("Tamanho do símbolo"));
            ui.add(egui::Slider::new(&mut st.size, 3.0..=24.0).suffix(" px"));
        });
    }
}

fn settings_page(ui: &mut egui::Ui, page: usize, s: &mut Settings) {
    match page {
        0 => {
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
                ui.separator();
                ui.label(RichText::new(tr("Seções da árvore")).strong());
                ui.horizontal_wrapped(|ui| {
                    for (k, n) in TREE_SECTIONS {
                        let mut on = !s.tree_hidden.iter().any(|x| x == k);
                        if ui.checkbox(&mut on, tr(n)).changed() {
                            if on {
                                s.tree_hidden.retain(|x| x != k);
                            } else {
                                s.tree_hidden.push(k.to_string());
                            }
                        }
                    }
                });
            }
        1 => {
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
                ui.separator();
                ui.label(RichText::new(tr("Pontos, linhas e curvas de referência")).strong());
                let mut st = scc_core::LineStyle { width: s.ref_line_width, dash: s.ref_line_dash, point: s.point_style, size: s.point_size };
                line_style_ui(ui, &mut st, true, true);
                (s.ref_line_width, s.ref_line_dash, s.point_style, s.point_size) = (st.width, st.dash, st.point, st.size);
                ui.horizontal(|ui| {
                    ui.label(tr("Centro de furo no sketch"));
                    for (i, n) in ["✳ asterisco", "✛ cruz", "⊕ círculo com cruz"].iter().enumerate() {
                        ui.selectable_value(&mut s.hole_mark, i as u8, tr(n));
                    }
                });
                ui.label(RichText::new(tr("Cada elemento pode ter estilo próprio: botão direito → Exibição → Propriedades gráficas.")).weak().small());
                ui.separator();
                ui.checkbox(&mut s.sketch_in_geometry, tr("sketch novo num plano vai para a pasta de geometria (numa face do corpo fica na linha do tempo)"));
            }
        2 => {
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
            }
        3 => {
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
            }
        4 => {
                for m in crate::render::DisplayMode::ALL {
                    ui.radio_value(&mut s.display, m, m.label());
                }
                ui.checkbox(&mut s.edges_in_render, tr("arestas no semi-renderizado"));
                ui.checkbox(&mut s.ortho, tr("projeção ortogonal"));
            }
        5 => {
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
            
            ui.add_space(8.);
            ui.label(RichText::new(tr("Painéis")).strong());
            ui.checkbox(&mut s.editor_left, tr("painel de edição à esquerda, junto da árvore"));
            ui.label(RichText::new(tr("Grupos flutuantes encaixam sozinhos quando soltos perto do topo ou das laterais; arraste o nome de um grupo encaixado para soltá-lo. Posições de janelas e painéis ficam salvas.")).weak().small());
        }
        6 => {
            ui.checkbox(&mut s.auto_update, tr("atualizar automaticamente a cada mudança"));
            ui.label(RichText::new(tr("Desligado: o que mudou fica em vermelho (⟳) na árvore até você atualizar (botão ⟳ ou F5). Ctrl+F5 reconstrói tudo do zero. Os botões ⏮ ◀ ▶ ⏭ da barra de status reconstroem passo a passo, comando a comando.")).weak().small());
        }
        7 => {
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
            }
        _ => {
                ui.checkbox(&mut s.grid, tr("grade"));
                ui.add(egui::Slider::new(&mut s.grid_step, 1.0..=50.0).text("passo da grade (mm)"));
                ui.checkbox(&mut s.snaps, tr("snaps"));
                ui.checkbox(&mut s.auto_constraints, tr("restrições automáticas"));
                ui.checkbox(&mut s.auto_dims, tr("cotas automáticas"));
                ui.label(RichText::new(tr("Aplicado pelo editor de sketch interativo (próxima alpha).")).weak().small());
            }
    }
}

/// Aba de bancada no topo: destacada (fundo e sublinhado) quando ativa; apagada quando indisponível.
fn bench_tab(ui: &mut egui::Ui, active: bool, text: &str, enabled: bool) -> egui::Response {
    let font = egui::FontId::proportional(13.5);
    let col = if !enabled {
        ui.visuals().weak_text_color().gamma_multiply(0.7)
    } else if active {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().text_color()
    };
    let g = ui.painter().layout_no_wrap(text.to_string(), font, col);
    let size = g.size() + egui::vec2(22., 10.);
    let (r, resp) = ui.allocate_exact_size(size, if enabled { egui::Sense::click() } else { egui::Sense::hover() });
    let p = ui.painter();
    if active {
        p.rect_filled(r, egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 }, Color32::from_rgba_unmultiplied(70, 140, 240, 50));
        p.rect_filled(Rect::from_min_max(egui::pos2(r.left() + 4., r.bottom() - 3.), egui::pos2(r.right() - 4., r.bottom())), 1.5, Color32::from_rgb(80, 160, 255));
    } else if resp.hovered() && enabled {
        p.rect_filled(r, 6., Color32::from_rgba_unmultiplied(120, 150, 200, 30));
    }
    p.galley(r.center() - g.size() / 2., g, col);
    resp
}
