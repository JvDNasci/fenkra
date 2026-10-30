//! Bancada de Montagem: árvore de componentes, painéis, restrições escolhidas na viewport,
//! movimentação com manipulador e verificação de interferências.
//!
//! A viewport e a seleção da peça são reaproveitadas: a montagem vira um documento "sombra" com
//! um coletor por componente e um resultado com as malhas já posicionadas (`fenkra_montagem::view`).

use crate::app::{fmt, App, Cmd};
use crate::i18n::tr;
use crate::picking::Pick;
use eframe::egui::{self, Color32, RichText};
use fenkra_core::{Document, Id};
use fenkra_montagem::load::{full_path, geom_at, PartCache};
use fenkra_montagem::kin::{self, CouplingKind, JFrame, JRef, JointKind};
use fenkra_montagem::solve::SolveReport;
use fenkra_montagem::*;
use fenkra_solidos::RecomputeResult;
use std::path::PathBuf;

/// Tipo de restrição pedido na barra (espera duas seleções).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MateCmd {
    Auto,
    Coincident,
    Contact,
    Distance,
    Angle,
    Parallel,
    Perpendicular,
    /// alinhamento instantâneo: posiciona e não guarda a restrição
    AlignOnce,
    Gear,
    Pulley,
    RackPinion,
}

impl MateCmd {
    pub fn label(self) -> &'static str {
        match self {
            MateCmd::Auto => "Automática",
            MateCmd::Coincident => "Coincidência",
            MateCmd::Contact => "Contato",
            MateCmd::Distance => "Distância",
            MateCmd::Angle => "Ângulo",
            MateCmd::Parallel => "Paralelismo",
            MateCmd::Perpendicular => "Perpendicularidade",
            MateCmd::AlignOnce => "Alinhar",
            MateCmd::Gear => "Engrenagem",
            MateCmd::Pulley => "Polia (correia)",
            MateCmd::RackPinion => "Pinhão e cremalheira",
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            MateCmd::Auto => "✨",
            MateCmd::Coincident => "◎",
            MateCmd::Contact => "⫴",
            MateCmd::Distance => "↔",
            MateCmd::Angle => "∠",
            MateCmd::Parallel => "∥",
            MateCmd::Perpendicular => "⟂",
            MateCmd::AlignOnce => "⇥",
            MateCmd::Gear => "⚙",
            MateCmd::Pulley => "◎",
            MateCmd::RackPinion => "⛭",
        }
    }
    fn kind(self) -> Option<MateKind> {
        Some(match self {
            MateCmd::Gear => MateKind::Gear { ratio: "-1".into(), belt: false, refs: [[0.; 3]; 4] },
            MateCmd::Pulley => MateKind::Gear { ratio: "1".into(), belt: true, refs: [[0.; 3]; 4] },
            MateCmd::RackPinion => MateKind::RackPinion { mm_per_rev: "31.4159".into(), refs: [[0.; 3]; 4] },
            MateCmd::Coincident => MateKind::Coincident { flip: false },
            MateCmd::Contact => MateKind::Contact,
            MateCmd::Distance => MateKind::Distance("0".into()),
            MateCmd::Angle => MateKind::Angle("90".into()),
            MateCmd::Parallel => MateKind::Parallel,
            MateCmd::Perpendicular => MateKind::Perpendicular,
            MateCmd::Auto | MateCmd::AlignOnce => return None,
        })
    }
}

/// Ação escolhida no menu do botão direito da viewport.
#[derive(Clone, Copy, Debug)]
pub enum AsmCtx {
    Mate(MateCmd),
    Fix(Id),
    Hide(Id),
    Isolate(Id),
    Edit(Id),
    Select(Id),
    Delete(Id),
    SubFromSel,
}

/// Restrições que fazem sentido entre as duas geometrias.
fn quick_options(a: &Geom, b: &Geom) -> Vec<MateCmd> {
    use MateCmd::*;
    match (a, b) {
        (Geom::Plane { .. }, Geom::Plane { .. }) => vec![Contact, Coincident, Distance, Angle, Parallel, Perpendicular, AlignOnce],
        (Geom::Axis { .. }, Geom::Axis { .. }) => vec![Coincident, Distance, Angle, Parallel, Perpendicular, Gear, Pulley, RackPinion, AlignOnce],
        (Geom::Axis { .. }, Geom::Plane { .. }) | (Geom::Plane { .. }, Geom::Axis { .. }) => vec![Coincident, Distance, Parallel, Perpendicular, Angle],
        _ => vec![Coincident, Distance],
    }
}

/// Opções da janela "Preencher furos com fixadores" e os avisos da última aplicação.
#[derive(Clone, Default)]
pub struct FastenersUi {
    pub op: OpcoesFixadores,
    /// só nos componentes escolhidos (árvore ou viewport)
    pub only_selected: bool,
    pub warnings: Vec<String>,
}

/// Janela "Engrenagens": parâmetros do par (o `tipo` é o do par), dentes da coroa e a última mensagem.
#[derive(Clone)]
pub struct GearsUi {
    pub p: fenkra_solidos::ParamEngrenagem,
    pub dentes_coroa: u32,
    pub msg: Vec<String>,
}

impl Default for GearsUi {
    fn default() -> Self {
        GearsUi { p: fenkra_solidos::ParamEngrenagem { furo: 8., ..Default::default() }, dentes_coroa: 40, msg: vec![] }
    }
}

pub struct AsmState {
    pub asm: Assembly,
    pub file: Option<PathBuf>,
    pub cache: PartCache,
    pub view_doc: Document,
    pub view: Option<RecomputeResult>,
    /// recompor a vista (peças ou posições mudaram)
    pub dirty: bool,
    pub sel: Option<Id>,
    /// restrição esperando as duas seleções
    pub pending: Option<MateCmd>,
    /// propriedades do componente escolhido (ou da montagem, sem escolha) abertas no painel
    pub props: bool,
    pub report: SolveReport,
    pub errors: Vec<(Id, String)>,
    pub undo: Vec<Assembly>,
    pub redo: Vec<Assembly>,
    pub modified: bool,
    pub interf: Vec<(Id, Id, f64)>,
    /// mostrar só o componente selecionado
    pub isolate: bool,
    /// mover respeitando as restrições (resolve ao soltar)
    pub respect: bool,
    /// parar o movimento no primeiro contato entre sólidos
    pub stop_collision: bool,
    /// peça aberta na Modelagem Sólida a partir da montagem (componente, arquivo)
    pub editing_part: Option<(Id, PathBuf)>,
    pub renaming: Option<Id>,
    /// item em edição e o estado de antes (Cancelar volta para ele; item novo some)
    pub edit_snap: Option<(Id, Assembly)>,
    /// posição antes do arrasto do manipulador
    drag_start: Option<Placement>,
    snapshot: Option<Assembly>,
    /// junta esperando as duas seleções
    pub pending_joint: Option<u8>,
    // ---- simulação
    pub sim_open: bool,
    pub sim_frames: Vec<kin::Frame>,
    pub sim_t: f64,
    pub sim_t1: f64,
    pub sim_steps: usize,
    pub sim_play: bool,
    /// componente cuja velocidade/aceleração é mostrada
    pub sim_track: Option<Id>,
    /// posições antes de tocar a simulação (voltam ao fechar)
    sim_rest: Option<Vec<(Id, Placement)>>,
    // ---- exibição (composer)
    /// estado de exibição em uso (visibilidade, cores, explosão)
    pub live: fenkra_montagem::display::DisplayState,
    pub explode_factor: f64,
    /// o manipulador mexe na explosão (não na posição real)
    pub explode_mode: bool,
    pub composer_open: bool,
    /// janela da lista de peças (nível escolhido)
    pub bom_open: Option<fenkra_montagem::bom::BomLevel>,
    /// janela de preenchimento de furos com fixadores (opções e avisos da última aplicação)
    pub fasteners_open: Option<FastenersUi>,
    /// janela do gerador de engrenagens (B28)
    pub gears_open: Option<GearsUi>,
    pub anim_sel: usize,
    pub anim_t: f64,
    pub anim_play: bool,
    /// modo visualização: só escolhe e mostra (sem manipulador nem restrição rápida)
    pub view_only: bool,
    /// graus de liberdade de cada componente (depois da última resolução)
    pub comp_dof: std::collections::HashMap<Id, usize>,
    /// anel de rotação em arraste (eixo, ângulo) e o centro do giro
    rot_drag: Option<(usize, f64)>,
    drag_center: Option<[f64; 3]>,
    /// o arraste parou num contato
    pub contact: bool,
    /// restrições possíveis para as duas seleções atuais
    quick: Option<(Vec<Pick>, Vec<MateCmd>)>,
    /// ação do menu do botão direito da viewport (aplicada no próximo quadro)
    pub ctx_act: Option<AsmCtx>,
    /// subconjuntos lidos para a árvore (caminho → linhas)
    sub_rows: std::collections::HashMap<PathBuf, Vec<(usize, String, bool)>>,
}

impl AsmState {
    pub fn new(asm: Assembly, file: Option<PathBuf>) -> Self {
        let report_dof = fenkra_montagem::solve::dof(&asm);
        AsmState {
            props: false,
            asm,
            file,
            cache: PartCache::default(),
            view_doc: Document::new("montagem"),
            view: None,
            dirty: true,
            sel: None,
            pending: None,
            report: SolveReport { converged: true, dof: report_dof, ..Default::default() },
            errors: vec![],
            undo: vec![],
            redo: vec![],
            modified: false,
            interf: vec![],
            isolate: false,
            respect: true,
            stop_collision: false,
            editing_part: None,
            renaming: None,
            edit_snap: None,
            drag_start: None,
            snapshot: None,
            pending_joint: None,
            sim_open: false,
            sim_frames: vec![],
            sim_t: 0.,
            sim_t1: 2.,
            sim_steps: 60,
            sim_play: false,
            sim_track: None,
            sim_rest: None,
            live: Default::default(),
            explode_factor: 0.,
            explode_mode: false,
            composer_open: false,
            bom_open: None,
            fasteners_open: None,
            gears_open: None,
            anim_sel: 0,
            anim_t: 0.,
            anim_play: false,
            view_only: false,
            comp_dof: Default::default(),
            rot_drag: None,
            drag_center: None,
            contact: false,
            quick: None,
            ctx_act: None,
            sub_rows: Default::default(),
        }
    }

    pub fn dir(&self) -> Option<PathBuf> {
        self.file.as_ref().and_then(|f| f.parent().map(|p| p.to_path_buf()))
    }

    /// Guarda o estado para desfazer (antes de uma mudança).
    pub fn push_undo(&mut self) {
        self.undo.push(self.asm.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.modified = true;
    }

    /// Recompõe a vista se preciso.
    pub fn refresh(&mut self) -> bool {
        if !self.dirty {
            return false;
        }
        let dir = self.dir();
        self.errors.clear();
        let mut shown = self.asm.with_state(&self.live);
        if self.isolate {
            if let Some(s) = self.sel {
                for n in &mut shown.nodes {
                    if n.id != s && !matches!(n.kind, NodeKind::Group) {
                        n.visible = false;
                    }
                }
            }
        }
        let (d, r) = fenkra_montagem::view::compose(&shown, dir.as_deref(), &mut self.cache, &mut self.errors);
        self.view_doc = d;
        self.view = Some(r);
        self.dirty = false;
        true
    }

    /// Resolve as restrições e marca a vista para recompor.
    pub fn solve(&mut self) {
        let dir = self.dir();
        fenkra_montagem::load::refresh_refs(&mut self.asm, &mut self.cache, dir.as_deref());
        self.report = fenkra_montagem::solve::solve(&mut self.asm);
        self.comp_dof = fenkra_montagem::solve::dof_per_comp(&self.asm);
        self.sub_rows.clear();
        self.dirty = true;
    }

    /// Caminho guardado para uma peça: relativo à pasta da montagem quando possível.
    fn rel_path(&self, p: &std::path::Path) -> String {
        if let Some(d) = self.dir() {
            if let Ok(r) = p.strip_prefix(&d) {
                return r.to_string_lossy().replace('\\', "/");
            }
        }
        p.to_string_lossy().to_string()
    }

    fn comp_of_body(&self, body: usize) -> Option<Id> {
        self.view.as_ref()?.bodies.get(body).map(|b| b.collector)
    }

    /// Elemento da restrição a partir de uma seleção da viewport.
    fn mate_ref(&mut self, p: Pick) -> Result<MateRef, String> {
        let (body, world) = match p {
            Pick::Face { body, point, .. } => (body, point.map(|v| v as f64)),
            Pick::Vertex { body, pos } => (body, pos.map(|v| v as f64)),
            Pick::Edge { body, mid, .. } => (body, mid.map(|v| v as f64)),
            _ => return Err("escolha faces, arestas ou vértices das peças".into()),
        };
        let comp = self.comp_of_body(body).ok_or("componente não encontrado")?;
        let node = self.asm.node(comp).ok_or("componente não encontrado")?;
        if !matches!(node.kind, NodeKind::Part { .. }) {
            return Err("restrições em peças de subconjunto: abra o subconjunto".into());
        }
        let pl = node.placement;
        let local = pl.to_local(world);
        let geom = match p {
            Pick::Vertex { .. } => Geom::Point(local),
            Pick::Edge { edge, .. } => {
                let r = self.view.as_ref().ok_or("sem vista")?;
                let pts: Vec<[f64; 3]> = crate::picking::edge_polyline(r, body, edge).into_iter().map(|q| pl.to_local(q)).collect();
                if pts.len() < 2 {
                    return Err("aresta sem pontos".into());
                }
                let c = fenkra_solidos::geo3d::Curve3::poly(pts.clone(), false)?;
                if c.is_straight() {
                    Geom::Axis { p: pts[0], d: unit(sub(pts[pts.len() - 1], pts[0])) }
                } else {
                    // aresta circular: eixo pelo centro, normal ao círculo
                    let (ctr, n, _) = fenkra_solidos::geo3d::fit_circle(&c)?;
                    Geom::Axis { p: ctr, d: unit(n) }
                }
            }
            _ => {
                let NodeKind::Part { path } = &node.kind else { unreachable!() };
                let full = full_path(self.dir().as_deref(), path);
                let lp = self.cache.get(&full, true)?;
                let sh = lp.shape.as_ref().ok_or("peça em modo visualização: passe para editável para restringir")?;
                geom_at(sh, local)?
            }
        };
        Ok(MateRef { comp, geom, pick: matches!(p, Pick::Face { .. }).then_some(local) })
    }
}

/// Restrição automática: o tipo mais lógico para o par escolhido.
fn auto_kind(a: &Geom, b: &Geom, wa: &Placement, wb: &Placement) -> MateKind {
    match (a, b) {
        (Geom::Plane { n: na, .. }, Geom::Plane { n: nb, .. }) => {
            // faces de frente uma para a outra → contato; mesmo sentido → coincidência
            if dot(wa.dir(*na), wb.dir(*nb)) < 0. {
                MateKind::Contact
            } else {
                MateKind::Coincident { flip: false }
            }
        }
        _ => MateKind::Coincident { flip: false },
    }
}

// ============================================================== comandos

/// Trata os comandos da bancada de montagem. Devolve `true` se o comando era dela.
pub fn command(app: &mut App, c: Cmd) -> bool {
    match c {
        Cmd::AsmNew => {
            app.asm = Some(AsmState::new(Assembly::new("Montagem"), None));
            app.toast(tr("Montagem nova: insira peças (🧩) e posicione com restrições"), false);
        }
        Cmd::AsmOpen => {
            let Some(p) = rfd::FileDialog::new().add_filter("Montagem Fenkra", &["fka"]).pick_file() else { return true };
            match std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|s| Assembly::from_json(&s)) {
                Ok(a) => {
                    app.settings.add_recent_file(&p);
                    app.asm = Some(AsmState::new(a, Some(p)));
                    app.fit_now();
                }
                Err(e) => app.toast(format!("Não foi possível abrir: {e}"), true),
            }
        }
        Cmd::AsmSave => save(app),
        _ => {
            let is_asm = matches!(
                c,
                Cmd::AsmInsert | Cmd::AsmNewPart | Cmd::AsmGroup | Cmd::AsmReplace | Cmd::AsmMate(_) | Cmd::AsmFix | Cmd::AsmRigid | Cmd::AsmSolve | Cmd::AsmNumber | Cmd::AsmInterf | Cmd::AsmSubAsm | Cmd::AsmPattern
                    | Cmd::AsmJoint(_) | Cmd::AsmAutoJoints | Cmd::AsmSimulate | Cmd::AsmComposer | Cmd::AsmBom | Cmd::AsmSubFromSel | Cmd::AsmFeat(_) | Cmd::AsmLink | Cmd::AsmFasteners | Cmd::AsmGears
            );
            let Some(st) = app.asm.as_mut() else {
                // só os comandos de montagem avisam; os da peça seguem normalmente
                if is_asm {
                    app.toast(tr("Crie ou abra uma montagem primeiro"), true);
                }
                return is_asm;
            };
            match c {
                Cmd::AsmInsert | Cmd::AsmSubAsm => {
                    let sub = c == Cmd::AsmSubAsm;
                    let (filter, ext) = if sub { ("Montagem", "fka") } else { ("Peça", "fkp") };
                    let Some(files) = rfd::FileDialog::new().add_filter(filter, &[ext]).pick_files() else { return true };
                    st.push_undo();
                    let parent = st.sel.filter(|s| st.asm.node(*s).is_some_and(|n| matches!(n.kind, NodeKind::Group)));
                    // peças novas entram um pouco ao lado da última, para não ficarem sobrepostas
                    let mut x = st.view.as_ref().map(|r| r.bodies.iter().flat_map(|b| b.mesh.positions.chunks_exact(3).map(|p| p[0] as f64)).fold(0f64, f64::max)).unwrap_or(0.);
                    for f in files {
                        let name = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Peça".into());
                        let path = st.rel_path(&f);
                        let kind = if sub { NodeKind::SubAssembly { path } } else { NodeKind::Part { path } };
                        let first = st.asm.nodes.iter().all(|n| matches!(n.kind, NodeKind::Group));
                        let at = if first { [0.; 3] } else { [x + 20., 0., 0.] };
                        let id = st.asm.add_component(&name, kind, Placement { origin: if st.asm.relational { [0.; 3] } else { at }, ..Default::default() }, parent);
                        if st.asm.relational {
                            // projeto relacional: a peça fica na origem da montagem, a forma vem dos links
                            st.asm.node_mut(id).unwrap().fixed = true;
                        }
                        st.sel = Some(id);
                        st.dirty = true;
                        st.refresh();
                        x = st.view.as_ref().map(|r| r.bodies.iter().flat_map(|b| b.mesh.positions.chunks_exact(3).map(|p| p[0] as f64)).fold(0f64, f64::max)).unwrap_or(x);
                    }
                    app.fit_now();
                }
                Cmd::AsmNewPart => {
                    // peça nova no contexto: cria o arquivo ao lado da montagem e abre para modelar
                    let mut dlg = rfd::FileDialog::new().add_filter("Peça", &["fkp"]).set_file_name("Peça nova.fkp");
                    if let Some(d) = st.dir() {
                        dlg = dlg.set_directory(d);
                    }
                    let Some(f) = dlg.save_file() else { return true };
                    let name = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Peça".into());
                    if let Err(e) = std::fs::write(&f, Document::new(&name).to_json()) {
                        app.toast(format!("Erro ao criar a peça: {e}"), true);
                        return true;
                    }
                    st.push_undo();
                    let path = st.rel_path(&f);
                    let id = st.asm.add_component(&name, NodeKind::Part { path }, Placement::default(), None);
                    if st.asm.relational {
                        st.asm.node_mut(id).unwrap().fixed = true;
                    }
                    st.sel = Some(id);
                    st.dirty = true;
                    edit_part(app, id);
                }
                Cmd::AsmGroup => {
                    st.push_undo();
                    let parent = st.sel.filter(|s| st.asm.node(*s).is_some_and(|n| matches!(n.kind, NodeKind::Group)));
                    let id = st.asm.add_group("Grupo", parent);
                    st.sel = Some(id);
                    st.renaming = Some(id);
                }
                Cmd::AsmReplace => {
                    let Some(id) = st.sel.filter(|s| st.asm.node(*s).is_some_and(|n| !matches!(n.kind, NodeKind::Group))) else {
                        app.toast(tr("Selecione o componente a substituir"), true);
                        return true;
                    };
                    let Some(f) = rfd::FileDialog::new().add_filter("Peça ou montagem", &["fkp", "fka"]).pick_file() else { return true };
                    st.push_undo();
                    let path = st.rel_path(&f);
                    let kind = if f.extension().is_some_and(|e| e == "fka") { NodeKind::SubAssembly { path } } else { NodeKind::Part { path } };
                    st.asm.replace(id, kind);
                    st.solve();
                    app.toast(tr("Componente substituído; as restrições reencontram as faces pelo ponto clicado"), false);
                }
                Cmd::AsmMate(m) => {
                    if app.picks.len() >= 2 {
                        let picks = app.picks.clone();
                        app.picks.clear();
                        create_mate(app, m, picks[0], picks[1]);
                    } else {
                        let st = app.asm.as_mut().unwrap();
                        st.pending = Some(m);
                        app.toast(tr("Escolha dois elementos (faces, arestas ou vértices) de peças diferentes"), false);
                    }
                }
                Cmd::AsmFix => {
                    let Some(id) = st.sel else {
                        app.toast(tr("Selecione o componente"), true);
                        return true;
                    };
                    st.push_undo();
                    if let Some(n) = st.asm.node_mut(id) {
                        n.fixed = !n.fixed;
                    }
                    st.solve();
                }
                Cmd::AsmRigid => {
                    let comps: Vec<Id> = app.picks.iter().filter_map(|p| match p {
                        Pick::Face { body, .. } | Pick::Edge { body, .. } | Pick::Vertex { body, .. } => st.comp_of_body(*body),
                        _ => None,
                    }).collect::<std::collections::BTreeSet<Id>>().into_iter().collect();
                    if comps.len() < 2 {
                        app.toast(tr("Selecione na viewport peças de pelo menos dois componentes (Ctrl+clique)"), true);
                        return true;
                    }
                    st.push_undo();
                    st.asm.add_rigid(comps);
                    app.picks.clear();
                    st.solve();
                }
                Cmd::AsmSolve => {
                    st.push_undo();
                    st.solve();
                }
                Cmd::AsmNumber => {
                    st.push_undo();
                    st.asm.renumber();
                    st.dirty = true;
                }
                Cmd::AsmPattern => {
                    let Some(src) = st.sel.filter(|s| st.asm.node(*s).is_some_and(|n| !matches!(n.kind, NodeKind::Group))) else {
                        app.toast(tr("Selecione o componente a repetir"), true);
                        return true;
                    };
                    st.push_undo();
                    let id = st.asm.add_pattern(src, PatternKind::Linear { dir: [1., 0., 0.], count: "3".into(), spacing: "20".into() });
                    st.sel = Some(id);
                    st.dirty = true;
                }
                Cmd::AsmJoint(k) => {
                    if app.picks.len() >= 2 {
                        let picks = app.picks.clone();
                        app.picks.clear();
                        create_joint(app, k, picks[0], picks[1]);
                    } else {
                        let st = app.asm.as_mut().unwrap();
                        st.pending_joint = Some(k);
                        st.pending = None;
                        app.toast(tr("Escolha os elementos das duas peças (eixo/face cilíndrica, face plana, vértice ou aresta)"), false);
                    }
                }
                Cmd::AsmAutoJoints => {
                    st.push_undo();
                    let made = st.asm.joints_from_mates();
                    st.solve();
                    app.toast(format!("{} junta(s) criadas a partir das restrições", made.len()), made.is_empty());
                }
                Cmd::AsmSimulate => {
                    st.sim_open = true;
                }
                Cmd::AsmComposer => {
                    st.composer_open = true;
                }
                Cmd::AsmBom => {
                    st.bom_open = Some(st.bom_open.unwrap_or_default());
                }
                Cmd::AsmFasteners => {
                    if st.fasteners_open.is_none() {
                        st.fasteners_open = Some(FastenersUi::default());
                    }
                }
                Cmd::AsmGears => {
                    if st.gears_open.is_none() {
                        st.gears_open = Some(GearsUi::default());
                    }
                }
                Cmd::AsmSubFromSel => {
                    // subconjunto novo com os componentes escolhidos (na viewport ou na árvore)
                    let mut comps = st.comps_of(&app.picks);
                    if let Some(s) = st.sel.filter(|s| st.asm.node(*s).is_some_and(|n| !matches!(n.kind, NodeKind::Group))) {
                        if !comps.contains(&s) {
                            comps.push(s);
                        }
                    }
                    if comps.is_empty() {
                        app.toast(tr("Selecione os componentes (Ctrl+clique nas peças na viewport ou escolha na árvore)"), true);
                        return true;
                    }
                    let mut dlg = rfd::FileDialog::new().add_filter("Montagem", &["fka"]).set_file_name("Subconjunto.fka");
                    if let Some(d) = st.dir() {
                        dlg = dlg.set_directory(d);
                    }
                    let Some(f) = dlg.save_file() else { return true };
                    let name = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Subconjunto".into());
                    st.push_undo();
                    let old_dir = st.dir();
                    let (mut sub, dropped) = st.asm.extract_subassembly(&comps, &name);
                    // caminhos das peças passam a ser relativos ao arquivo novo
                    let new_dir = f.parent().map(|p| p.to_path_buf());
                    for n in &mut sub.nodes {
                        if let NodeKind::Part { path } | NodeKind::SubAssembly { path } = &mut n.kind {
                            let full = full_path(old_dir.as_deref(), path);
                            *path = match new_dir.as_ref().and_then(|d| full.strip_prefix(d).ok()) {
                                Some(r) => r.to_string_lossy().replace('\\', "/"),
                                None => full.to_string_lossy().to_string(),
                            };
                        }
                    }
                    if let Err(e) = std::fs::write(&f, sub.to_json()) {
                        app.toast(format!("{}: {e}", tr("Erro ao gravar o subconjunto")), true);
                        return true;
                    }
                    let path = st.rel_path(&f);
                    let id = st.asm.add_component(&name, NodeKind::SubAssembly { path }, Placement::default(), None);
                    st.sel = Some(id);
                    app.picks.clear();
                    st.solve();
                    let extra = if dropped > 0 { format!(" · {dropped} {}", tr("restrição(ões) com peças de fora apagada(s)")) } else { String::new() };
                    app.toast(format!("{} {name}: {} {}{extra}", tr("Subconjunto"), comps.len(), tr("componente(s)")), dropped > 0);
                }
                Cmd::AsmFeat(k) => {
                    let comps = st.comps_of(&app.picks);
                    let fid = if k == 0 {
                        let Some(tool) = st.sel.filter(|s| matches!(st.asm.node(*s).map(|n| &n.kind), Some(NodeKind::Part { .. }))) else {
                            app.toast(tr("Recorte pela peça: escolha na árvore a peça que corta (pino, aba…) e clique nas peças a recortar"), true);
                            return true;
                        };
                        let targets: Vec<Id> = comps.into_iter().filter(|c| *c != tool).collect();
                        if targets.is_empty() {
                            app.toast(tr("Clique na viewport nas peças a recortar (Ctrl+clique para várias)"), true);
                            return true;
                        }
                        st.push_undo();
                        st.asm.add_feature(AsmFeatKind::Cut { tool, clearance: String::new() }, targets)
                    } else {
                        let face = app.picks.iter().find_map(|p| match p {
                            Pick::Face { point, normal, .. } => Some((point.map(|v| v as f64), normal.map(|v| v as f64))),
                            _ => None,
                        });
                        let Some((p, n)) = face else {
                            app.toast(tr("Furo entre peças: clique na face onde o furo começa (Ctrl+clique nas outras peças que ele atravessa)"), true);
                            return true;
                        };
                        st.push_undo();
                        st.asm.add_feature(AsmFeatKind::Hole { p, d: [-n[0], -n[1], -n[2]], dia: "6".into(), depth: String::new() }, comps)
                    };
                    app.picks.clear();
                    st.sel = Some(fid);
                    st.dirty = true;
                }
                Cmd::AsmLink => {
                    let Some(target) = st.sel.filter(|s| matches!(st.asm.node(*s).map(|n| &n.kind), Some(NodeKind::Part { .. }))) else {
                        app.toast(tr("Copiar geometria: escolha na árvore a peça que recebe e clique numa face plana de outra peça"), true);
                        return true;
                    };
                    let face = app.picks.iter().find_map(|p| match p {
                        Pick::Face { body, .. } => st.comp_of_body(*body).filter(|c| *c != target).map(|c| (c, *p)),
                        _ => None,
                    });
                    let Some((src, pick)) = face else {
                        app.toast(tr("Clique numa face plana de outra peça (a origem do link)"), true);
                        return true;
                    };
                    match link_in_context(st, src, target, pick) {
                        Ok(msg) => {
                            app.picks.clear();
                            app.toast(msg, false);
                        }
                        Err(e) => app.toast(e, true),
                    }
                }
                Cmd::AsmInterf => {
                    let dir = st.dir();
                    st.interf = fenkra_montagem::view::interferences(&st.asm, dir.as_deref(), &mut st.cache, 0.01);
                    let n = st.interf.len();
                    app.toast(if n == 0 { tr("Nenhuma interferência entre os componentes").to_string() } else { format!("{n} interferência(s) — veja no painel") }, n > 0);
                }
                _ => return false,
            }
        }
    }
    true
}

/// Cria a restrição entre duas seleções e resolve.
fn create_mate(app: &mut App, m: MateCmd, a: Pick, b: Pick) {
    let Some(st) = app.asm.as_mut() else { return };
    let (ra, rb) = match (st.mate_ref(a), st.mate_ref(b)) {
        (Ok(x), Ok(y)) => (x, y),
        (Err(e), _) | (_, Err(e)) => {
            app.toast(e, true);
            return;
        }
    };
    if ra.comp == rb.comp {
        app.toast(tr("Os dois elementos são do mesmo componente"), true);
        return;
    }
    let (pa, pb) = (st.asm.node(ra.comp).unwrap().placement, st.asm.node(rb.comp).unwrap().placement);
    let kind = m.kind().unwrap_or_else(|| auto_kind(&ra.geom, &rb.geom, &pa, &pb));
    st.push_undo();
    let id = st.asm.add_mate(kind, ra, rb);
    // engrenagem/cremalheira: o giro de agora é o zero
    st.asm.init_motion(id);
    st.solve();
    if m == MateCmd::AlignOnce {
        // alinhamento instantâneo: posiciona e não deixa histórico
        st.asm.mates.retain(|x| x.id != id);
        app.toast(tr("Alinhado (sem restrição guardada)"), false);
    } else {
        st.sel = Some(id);
        let ok = !st.report.failed.contains(&id);
        let msg = if ok { format!("{} criada · {} GDL livres", st.asm.name_of(id), st.report.dof) } else { format!("{}: não foi possível satisfazer (conflito com outras restrições)", st.asm.name_of(id)) };
        app.toast(msg, !ok);
    }
}

/// Cria a junta entre duas seleções (o sistema de B começa alinhado ao de A: ângulo e curso 0).
fn create_joint(app: &mut App, k: u8, a: Pick, b: Pick) {
    let Some(st) = app.asm.as_mut() else { return };
    let kind = match k {
        0 => JointKind::Revolute,
        1 => JointKind::Prismatic,
        2 => JointKind::Cylindrical,
        3 => JointKind::Screw { pitch: "2".into() },
        4 => JointKind::Spherical,
        5 => JointKind::Planar,
        6 => JointKind::Rigid,
        7 => JointKind::PointCurve { curve: vec![] },
        _ => JointKind::Cam { profile: vec![], radius: "0".into() },
    };
    // came: perfil = contorno da seção da peça pelo plano da aresta escolhida (vários arcos
    // viram um perfil só); seguidor = vértice (ponta) ou aresta circular (rolete)
    let cam = if let (JointKind::Cam { .. }, Pick::Edge { body, edge, mid }) = (&kind, a) {
        let comp = st.comp_of_body(body);
        let node = comp.and_then(|c| st.asm.node(c)).cloned();
        let poly: Vec<[f64; 3]> = st.view.as_ref().map(|r| crate::picking::edge_polyline(r, body, edge)).unwrap_or_default();
        match (node, poly.len() >= 3) {
            (Some(n), true) => {
                let pl = n.placement;
                let loc: Vec<[f64; 3]> = poly.iter().map(|q| pl.to_local(*q)).collect();
                // normal do plano da aresta (Newell; aresta reta: cai no eixo z da peça)
                let mut nn = [0.; 3];
                for i in 0..loc.len() {
                    let (p, q) = (loc[i], loc[(i + 1) % loc.len()]);
                    nn[0] += (p[1] - q[1]) * (p[2] + q[2]);
                    nn[1] += (p[2] - q[2]) * (p[0] + q[0]);
                    nn[2] += (p[0] - q[0]) * (p[1] + q[1]);
                }
                let nn = if norm(nn) > 1e-9 { unit(nn) } else { [0., 0., 1.] };
                let m = pl.to_local(mid.map(|v| v as f64));
                let dir = st.file.as_ref().and_then(|f| f.parent()).map(|p| p.to_path_buf());
                let shape = match &n.kind {
                    NodeKind::Part { path } => st.cache.get(&fenkra_montagem::load::full_path(dir.as_deref(), path), true).ok().and_then(|p| p.shape.clone()),
                    _ => None,
                };
                let lp = shape.and_then(|s| s.section_loops(m, nn, 1e4).ok()).and_then(|loops| {
                    loops.into_iter().min_by(|x, y| {
                        let dmin = |l: &Vec<[f64; 3]>| l.iter().map(|p| norm(sub(*p, m))).fold(f64::MAX, f64::min);
                        dmin(x).total_cmp(&dmin(y))
                    })
                });
                Some(lp.unwrap_or(loc))
            }
            _ => None,
        }
    } else {
        None
    };
    let roller = match (&kind, b) {
        (JointKind::Cam { .. }, Pick::Edge { body, edge, .. }) => st
            .view
            .as_ref()
            .map(|r| crate::picking::edge_polyline(r, body, edge))
            .and_then(|pts| fenkra_solidos::geo3d::Curve3::poly(pts, false).ok())
            .filter(|c| !c.is_straight())
            .and_then(|c| fenkra_solidos::geo3d::fit_circle(&c).ok())
            .map(|(_, _, r)| r),
        _ => None,
    };
    // ponto em curva: a aresta (trajetória) é da peça A, o vértice da peça B
    let curve = if let (JointKind::PointCurve { .. }, Pick::Edge { body, edge, .. }) = (&kind, a) {
        let comp = st.comp_of_body(body);
        let pl = comp.and_then(|c| st.asm.node(c)).map(|n| n.placement).unwrap_or_default();
        st.view.as_ref().map(|r| crate::picking::edge_polyline(r, body, edge).into_iter().map(|q| pl.to_local(q)).collect::<Vec<_>>())
    } else {
        None
    };
    let (ra, rb) = match (st.mate_ref(a), st.mate_ref(b)) {
        (Ok(x), Ok(y)) => (x, y),
        (Err(e), _) | (_, Err(e)) => {
            app.toast(e, true);
            return;
        }
    };
    if ra.comp == rb.comp {
        app.toast(tr("Os dois elementos são da mesma peça"), true);
        return;
    }
    let frame = |g: &Geom| match *g {
        Geom::Axis { p, d } => JFrame::from_axis(p, d),
        Geom::Plane { p, n } => JFrame::from_axis(p, n),
        Geom::Point(p) => JFrame::from_axis(p, [0., 0., 1.]),
    };
    let (pa, pb) = (st.asm.node(ra.comp).unwrap().placement, st.asm.node(rb.comp).unwrap().placement);
    let fa = frame(&ra.geom);
    let mut fb = frame(&rb.geom);
    // x de B = x de A projetado no plano de B (zero comum do ângulo)
    let xa = pb.dir_to_local(pa.dir(fa.x));
    let k2 = dot(xa, fb.z);
    let px = [xa[0] - fb.z[0] * k2, xa[1] - fb.z[1] * k2, xa[2] - fb.z[2] * k2];
    if norm(px) > 1e-6 {
        fb.x = unit(px);
    }
    let kind = match (kind, curve) {
        (JointKind::PointCurve { .. }, Some(c)) => JointKind::PointCurve { curve: c },
        (JointKind::PointCurve { .. }, None) => {
            app.toast(tr("Ponto em curva: escolha primeiro a aresta (trajetória) e depois o vértice"), true);
            return;
        }
        (JointKind::Cam { .. }, _) => match cam {
            Some(p) => JointKind::Cam { profile: p, radius: roller.map(|r| format!("{r:.3}")).unwrap_or_else(|| "0".into()) },
            None => {
                app.toast(tr("Came: escolha primeiro uma aresta do perfil do came e depois o seguidor (vértice ou aresta circular do rolete)"), true);
                return;
            }
        },
        (k, _) => k,
    };
    st.push_undo();
    let id = st.asm.add_joint(kind, JRef { comp: ra.comp, frame: fa }, JRef { comp: rb.comp, frame: fb });
    st.solve();
    st.sel = Some(id);
    let (sem, com) = kin::analyze(&st.asm);
    let msg = format!("{} criada · mecanismo com {sem} GDL ({com} com os comandos)", st.asm.name_of(id));
    app.toast(msg, false);
}

/// Seleções feitas na viewport com uma restrição pendente: cria quando houver duas.
pub fn after_viewport(app: &mut App) {
    after_view_quick(app);
    let pj = app.asm.as_ref().and_then(|s| s.pending_joint);
    if let (Some(k), true) = (pj, app.picks.len() >= 2) {
        let picks = app.picks.clone();
        app.picks.clear();
        if let Some(st) = app.asm.as_mut() {
            st.pending_joint = None;
        }
        create_joint(app, k, picks[0], picks[1]);
        return;
    }
    let pending = app.asm.as_ref().and_then(|s| s.pending);
    if let Some(m) = pending {
        if app.picks.len() >= 2 {
            let picks = app.picks.clone();
            app.picks.clear();
            if let Some(st) = app.asm.as_mut() {
                st.pending = None;
            }
            create_mate(app, m, picks[0], picks[1]);
        }
    } else if let Some(p) = app.picks.last().copied() {
        // um clique numa peça seleciona o componente na árvore
        if let Pick::Face { body, .. } | Pick::Edge { body, .. } | Pick::Vertex { body, .. } = p {
            if let Some(st) = app.asm.as_mut() {
                if let Some(c) = st.comp_of_body(body) {
                    if st.sel != Some(c) {
                        st.sel = Some(c);
                    }
                }
            }
        }
    }
    // duplo clique numa peça: abre para editar
    if let Some(id) = app.take_open_pending() {
        if app.asm.as_ref().is_some_and(|s| s.asm.node(id).is_some()) {
            edit_part(app, id);
        }
    }
}

/// Abre a peça do componente na Modelagem Sólida; "Voltar à montagem" recarrega.
pub fn edit_part(app: &mut App, id: Id) {
    let Some(st) = app.asm.as_mut() else { return };
    let Some(NodeKind::Part { path }) = st.asm.node(id).map(|n| n.kind.clone()) else {
        app.toast(tr("Só peças abrem para edição (subconjunto: abra a montagem dele)"), true);
        return;
    };
    let full = full_path(st.dir().as_deref(), &path);
    match std::fs::read_to_string(&full).map_err(|e| e.to_string()).and_then(|s| Document::from_json(&s).map_err(|e| e.to_string())) {
        Ok(d) => {
            st.editing_part = Some((id, full.clone()));
            app.load_doc_pub(d, Some(full));
            app.settings.bench = crate::settings::Bench::Solidos;
            app.toast(tr("Editando a peça no contexto da montagem — salve e use ↩ Voltar à montagem"), false);
        }
        Err(e) => app.toast(format!("Não foi possível abrir a peça: {e}"), true),
    }
}

/// Volta da edição de uma peça: salva, recarrega e reposiciona.
pub fn back_to_assembly(app: &mut App) {
    app.save_pub();
    let Some(st) = app.asm.as_mut() else { return };
    if let Some((_, f)) = st.editing_part.take() {
        st.cache.forget(&f);
    }
    st.solve();
    app.settings.bench = crate::settings::Bench::Montagem;
    app.fit_now();
}

pub fn save(app: &mut App) {
    let Some(st) = app.asm.as_mut() else { return };
    let path = match &st.file {
        Some(f) => Some(f.clone()),
        None => rfd::FileDialog::new().add_filter("Montagem Fenkra", &["fka"]).set_file_name(format!("{}.fka", st.asm.name)).save_file(),
    };
    let Some(path) = path else { return };
    // caminhos absolutos dentro da pasta nova viram relativos
    let dir = path.parent().map(|p| p.to_path_buf());
    for n in &mut st.asm.nodes {
        if let NodeKind::Part { path: p } | NodeKind::SubAssembly { path: p } = &mut n.kind {
            if let (Some(d), true) = (&dir, std::path::Path::new(p.as_str()).is_absolute()) {
                if let Ok(r) = std::path::Path::new(p.as_str()).strip_prefix(d) {
                    *p = r.to_string_lossy().replace('\\', "/");
                }
            }
        }
    }
    // números de item fixos para os componentes novos
    st.asm.assign_items(dir.as_deref());
    match std::fs::write(&path, st.asm.to_json()) {
        Ok(()) => {
            st.file = Some(path.clone());
            st.modified = false;
            app.settings.add_recent_file(&path);
            app.toast(tr("Montagem salva"), false);
        }
        Err(e) => app.toast(format!("Erro ao salvar: {e}"), true),
    }
}

// ============================================================== árvore

/// O que a árvore mostra além dos nós: graus de liberdade, recursos de cada peça, conteúdo dos
/// subconjuntos e quantas instâncias de cada arquivo.
/// Resumo da árvore de uma peça (como aparece no Sólidos): corpos com a linha do tempo,
/// geometria e parâmetros.
#[derive(Clone, Default)]
struct PartOutline {
    /// (coletor, nome, cor, comandos: (id, ícone, nome, suprimido))
    bodies: Vec<(Id, String, [u8; 3], Vec<(Id, &'static str, String, bool)>)>,
    /// geometria (ícone, nome)
    geo: Vec<(&'static str, String)>,
    params: Vec<String>,
}

impl PartOutline {
    fn of(doc: &fenkra_core::Document) -> Self {
        let bodies = doc
            .collectors
            .iter()
            .map(|c| {
                let feats = doc.features.iter().filter(|f| f.target == c.id && !doc.is_free(f)).map(|f| (f.id, f.kind.icon(), f.name.clone(), f.suppressed)).collect();
                (c.id, c.name.clone(), c.color, feats)
            })
            .collect();
        let mut geo: Vec<(&'static str, String)> = doc.references.iter().filter(|r| !matches!(r.kind, fenkra_core::RefKind::Linked { .. })).map(|r| ("◇", r.name.clone())).collect();
        geo.extend(doc.features.iter().filter(|f| doc.is_free(f)).map(|f| (f.kind.icon(), f.name.clone())));
        let params = doc.params.iter().map(|p| format!("{} = {}", p.name, p.expr)).collect();
        PartOutline { bodies, geo, params }
    }
    fn is_empty(&self) -> bool {
        self.bodies.iter().all(|b| b.3.is_empty()) && self.geo.is_empty()
    }
}

struct TreeInfo {
    feats: std::collections::HashMap<Id, PartOutline>,
    subs: std::collections::HashMap<Id, Vec<(usize, String, bool)>>,
    inst: std::collections::HashMap<String, usize>,
}

/// Componentes de um subconjunto (e dos subconjuntos dentro dele), com a profundidade.
fn sub_listing(full: &std::path::Path, depth: usize) -> Vec<(usize, String, bool)> {
    let mut out = vec![];
    if depth > 6 {
        return out;
    }
    let Some(a) = std::fs::read_to_string(full).ok().and_then(|t| Assembly::from_json(&t).ok()) else { return out };
    let dir = full.parent().map(|p| p.to_path_buf());
    for n in a.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)) {
        let sub = matches!(n.kind, NodeKind::SubAssembly { .. });
        out.push((depth, n.name.clone(), sub));
        if let NodeKind::SubAssembly { path } = &n.kind {
            out.extend(sub_listing(&full_path(dir.as_deref(), path), depth + 1));
        }
    }
    out
}

pub fn tree(app: &mut App, ui: &mut egui::Ui) {
    let Some(st) = app.asm.as_mut() else {
        ui.add_space(8.);
        ui.label(RichText::new(tr("Nenhuma montagem aberta.")).strong());
        ui.label(RichText::new(tr("Uma montagem posiciona peças e subconjuntos com restrições. As peças continuam nos arquivos delas.")).weak());
        ui.add_space(6.);
        let mut c = None;
        if ui.button(tr("➕ Nova montagem")).clicked() {
            c = Some(Cmd::AsmNew);
        }
        if ui.button(tr("📂 Abrir montagem…")).clicked() {
            c = Some(Cmd::AsmOpen);
        }
        if let Some(c) = c {
            command(app, c);
        }
        return;
    };
    let mut act: Option<(Id, u8)> = None;
    let mut new_sel = None;
    let mut hover = None;
    let title = format!("{}{}", st.asm.name, if st.modified { " •" } else { "" });
    let tr_resp = ui
        .horizontal(|ui| {
            ui.add(egui::Image::new(crate::icons::get("asm_nova")).fit_to_exact_size(egui::vec2(18., 18.)));
            ui.selectable_label(st.props && st.sel.is_none(), RichText::new(title).strong())
        })
        .inner
        .on_hover_text(tr("clique: propriedades e parâmetros da montagem"));
    if tr_resp.clicked() {
        st.sel = None;
        st.props = true;
    }
    if let Some(f) = &st.file {
        ui.label(RichText::new(f.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default()).weak().small());
    }
    // modo edição × visualização
    ui.horizontal_wrapped(|ui| {
        if ui.selectable_label(!st.view_only, tr("✏ Edição")).on_hover_text(tr("mover, restringir e editar")).clicked() {
            st.view_only = false;
        }
        if ui.selectable_label(st.view_only, tr("👁 Visualização")).on_hover_text(tr("só olhar e escolher: sem manipulador nem restrição rápida")).clicked() {
            st.view_only = true;
        }
        if st.asm.relational {
            ui.label(RichText::new(tr("relacional")).weak().small()).on_hover_text(tr("projeto relacional: peças novas fixas na origem, forma por links entre peças"));
        }
    });
    // resumo: definidos × soltos
    if st.comp_dof.is_empty() && !st.asm.nodes.is_empty() {
        st.comp_dof = fenkra_montagem::solve::dof_per_comp(&st.asm);
    }
    {
        let comps: Vec<&Node> = st.asm.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).collect();
        let full = comps.iter().filter(|n| n.fixed || st.comp_dof.get(&n.id) == Some(&0)).count();
        let free = comps.iter().filter(|n| !n.fixed && st.comp_dof.get(&n.id) == Some(&6)).count();
        let part = comps.len() - full - free;
        ui.label(RichText::new(format!("■ {full} {}  ◐ {part} {}  ○ {free} {}", tr("definidos"), tr("parciais"), tr("soltos"))).small().weak());
    }
    // recursos das peças, subconjuntos e instâncias (para a árvore)
    let info = {
        let dir = st.dir();
        let mut feats = std::collections::HashMap::new();
        let mut subs = std::collections::HashMap::new();
        let mut inst: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for n in st.asm.nodes.clone() {
            match &n.kind {
                NodeKind::Part { path } => {
                    *inst.entry(path.clone()).or_insert(0) += 1;
                    if let Ok(p) = st.cache.get(&full_path(dir.as_deref(), path), false) {
                        feats.insert(n.id, PartOutline::of(&p.doc));
                    }
                }
                NodeKind::SubAssembly { path } => {
                    *inst.entry(path.clone()).or_insert(0) += 1;
                    let full = full_path(dir.as_deref(), path);
                    let rows = st.sub_rows.entry(full.clone()).or_insert_with(|| sub_listing(&full, 0)).clone();
                    subs.insert(n.id, rows);
                }
                NodeKind::Group => {}
            }
        }
        TreeInfo { feats, subs, inst }
    };
    ui.separator();
    // nós, com grupos aninhados (a ordem da árvore só muda a visualização)
    #[allow(clippy::too_many_arguments)]
    fn rows(ui: &mut egui::Ui, st: &AsmState, info: &TreeInfo, parent: Option<Id>, depth: usize, new_sel: &mut Option<Id>, act: &mut Option<(Id, u8)>, hover: &mut Option<Id>) {
        let kids: Vec<&Node> = st.asm.nodes.iter().filter(|n| n.parent == parent || (depth == 0 && n.parent.is_some_and(|p| st.asm.node(p).is_none()))).collect();
        for n in kids {
            let err = st.errors.iter().find(|(i, _)| *i == n.id).map(|(_, e)| e.clone());
            let icon = match n.kind {
                NodeKind::Group => "asm_grupo",
                NodeKind::Part { .. } => "nova_peca",
                NodeKind::SubAssembly { .. } => "asm_subconjunto",
            };
            let num = if n.number > 0 { format!("{} · ", n.number) } else { String::new() };
            let mode = if n.mode == LoadMode::Visualizacao { " 👓" } else { "" };
            // instâncias do mesmo arquivo
            let copies = match &n.kind {
                NodeKind::Part { path } | NodeKind::SubAssembly { path } => info.inst.get(path).copied().unwrap_or(1),
                _ => 1,
            };
            let inst = if copies > 1 { format!("  ×{copies}") } else { String::new() };
            let txt = format!("{num}{}{}{mode}{}{inst}", n.name, if n.fixed { " 📌" } else { "" }, if err.is_some() { " ✖" } else { "" });
            let open_key = egui::Id::unique(("asm_expandir", n.id.0));
            let has_kids = info.feats.get(&n.id).is_some_and(|v| !v.is_empty()) || info.subs.get(&n.id).is_some_and(|v| !v.is_empty()) || matches!(n.kind, NodeKind::Group);
            let mut open = ui.ctx().data(|m| m.get_temp::<bool>(open_key)).unwrap_or(false);
            let r = ui
                .horizontal(|ui| {
                    ui.add_space(depth as f32 * 14.);
                    // subárvore da peça / do subconjunto
                    // grupos começam abertos (o valor guardado é "fechado"); peças começam fechadas
                    let shown = if matches!(n.kind, NodeKind::Group) { !open } else { open };
                    if has_kids {
                        if ui.add(egui::Button::new(RichText::new(if shown { "⏷" } else { "⏵" }).size(10.)).frame(false)).on_hover_text(tr("recursos da peça / componentes do subconjunto")).clicked() {
                            open = !open;
                            ui.ctx().data_mut(|m| m.insert_temp(open_key, open));
                        }
                    } else {
                        ui.add_space(12.);
                    }
                    if crate::app::eye_button(ui, !n.visible) {
                        *act = Some((n.id, 1));
                    }
                    let img = egui::Image::new(crate::icons::get(icon)).fit_to_exact_size(egui::vec2(16., 16.));
                    ui.add(if n.visible { img } else { img.tint(Color32::from_white_alpha(110)) });
                    // totalmente definido × parcial × solto
                    if !matches!(n.kind, NodeKind::Group) && !n.fixed {
                        let (t, c, tip) = match st.comp_dof.get(&n.id).copied() {
                            Some(0) => ("■".to_string(), crate::theme::st_ok(), tr("totalmente definido").to_string()),
                            Some(6) => ("○".to_string(), Color32::from_gray(150), tr("solto: 6 graus de liberdade").to_string()),
                            Some(k) => (format!("◐{k}"), crate::theme::st_warn(), format!("{k} {}", tr("grau(s) de liberdade"))),
                            None => ("·".to_string(), Color32::from_gray(150), String::new()),
                        };
                        ui.label(RichText::new(t).color(c).small()).on_hover_text(tip);
                    }
                    if st.renaming == Some(n.id) {
                        let key = egui::Id::unique(("asm_ren", n.id.0));
                        let mut buf: String = ui.ctx().data(|m| m.get_temp(key)).unwrap_or_else(|| n.name.clone());
                        let r = ui.add(egui::TextEdit::singleline(&mut buf).desired_width(140.));
                        if !r.has_focus() && !r.lost_focus() {
                            r.request_focus();
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) || r.lost_focus() {
                            ui.ctx().data_mut(|m| m.insert_temp(egui::Id::unique("asm_ren_ok"), (n.id, buf.clone())));
                            ui.ctx().data_mut(|m| m.remove::<String>(key));
                        } else {
                            ui.ctx().data_mut(|m| m.insert_temp(key, buf));
                        }
                        r
                    } else {
                        let t = if err.is_some() { RichText::new(txt).color(crate::theme::st_err()) } else if n.visible { RichText::new(txt) } else { RichText::new(txt).weak() };
                        ui.selectable_label(st.sel == Some(n.id), t)
                    }
                })
                .inner;
            if r.hovered() {
                *hover = Some(n.id);
            }
            if r.clicked() {
                *new_sel = Some(n.id);
            }
            if r.double_clicked() {
                *act = Some((n.id, if matches!(n.kind, NodeKind::Group) { 9 } else { 2 }));
            }
            let r = match &err {
                Some(e) => r.on_hover_text(e),
                None => r,
            };
            r.context_menu(|ui| {
                ui.label(RichText::new(&n.name).strong());
                if matches!(n.kind, NodeKind::Part { .. }) && ui.button(tr("✏ Editar a peça (Modelagem Sólida)")).clicked() {
                    *act = Some((n.id, 2));
                    ui.close();
                }
                if ui.button(tr("⚙ Propriedades do componente")).clicked() {
                    *act = Some((n.id, 12));
                    ui.close();
                }
                if ui.button(tr("✎ Renomear   F2")).clicked() {
                    *act = Some((n.id, 9));
                    ui.close();
                }
                if !matches!(n.kind, NodeKind::Group) {
                    ui.menu_button(tr("👁 Modo e exibição"), |ui| {
                        if ui.radio(n.mode == LoadMode::Editavel, tr("Editável (geometria completa)")).clicked() {
                            *act = Some((n.id, 3));
                            ui.close();
                        }
                        if ui.radio(n.mode == LoadMode::Visualizacao, tr("Só visualização (leve, carrega só se visível)")).clicked() {
                            *act = Some((n.id, 4));
                            ui.close();
                        }
                        ui.separator();
                        if ui.button(if n.visible { tr("Ocultar   H") } else { tr("Mostrar   H") }).clicked() {
                            *act = Some((n.id, 1));
                            ui.close();
                        }
                        if ui.button(tr("🔍 Isolar (mostrar só este)")).clicked() {
                            *act = Some((n.id, 10));
                            ui.close();
                        }
                    });
                    if ui.button(if n.fixed { tr("📌 Liberar") } else { tr("📌 Fixar (sem graus de liberdade)") }).clicked() {
                        *act = Some((n.id, 5));
                        ui.close();
                    }
                    if ui.button(tr("⇄ Substituir…")).clicked() {
                        *act = Some((n.id, 6));
                        ui.close();
                    }
                }
                ui.menu_button(tr("📁 Organizar"), |ui| {
                    if ui.button(tr("▲ Subir")).clicked() {
                        *act = Some((n.id, 7));
                    }
                    if ui.button(tr("▼ Descer")).clicked() {
                        *act = Some((n.id, 8));
                    }
                    ui.separator();
                    ui.label(RichText::new(tr("Mover para o grupo")).weak().small());
                    if ui.button(tr("(raiz)")).clicked() {
                        ui.ctx().data_mut(|m| m.insert_temp(egui::Id::unique("asm_mover"), (n.id, Id(0))));
                        ui.close();
                    }
                    for g in st.asm.nodes.iter().filter(|g| matches!(g.kind, NodeKind::Group) && g.id != n.id) {
                        if ui.button(format!("📁 {}", g.name)).clicked() {
                            ui.ctx().data_mut(|m| m.insert_temp(egui::Id::unique("asm_mover"), (n.id, g.id)));
                            ui.close();
                        }
                    }
                });
                ui.separator();
                if ui.button(tr("🗑 Apagar   Del")).clicked() {
                    *act = Some((n.id, 0));
                    ui.close();
                }
            });
            let group_open = matches!(n.kind, NodeKind::Group) && !open;
            if open && !matches!(n.kind, NodeKind::Group) {
                // a peça por dentro, como no Sólidos: corpos (cada um com a linha do tempo), geometria
                // e parâmetros; 2 cliques num comando abre a peça (objeto de trabalho) nele
                if let Some(o) = info.feats.get(&n.id) {
                    let ind = (depth + 1) as f32 * 14. + 6.;
                    let line = |ui: &mut egui::Ui, extra: f32| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(ind + extra, 18.), egui::Sense::hover());
                        let x = r.left() + (depth as f32) * 14. + 12.;
                        ui.painter().line_segment([egui::pos2(x, r.top()), egui::pos2(x, r.bottom())], egui::Stroke::new(1., ui.visuals().weak_text_color().gamma_multiply(0.35)));
                    };
                    for (cid, cname, color, feats) in &o.bodies {
                        let key = egui::Id::unique(("asm_corpo", n.id.0, cid.0));
                        let mut copen = ui.ctx().data(|m| m.get_temp::<bool>(key)).unwrap_or(true);
                        ui.horizontal(|ui| {
                            line(ui, 0.);
                            if ui.add(egui::Button::new(RichText::new(if copen { "⏷" } else { "⏵" }).size(10.)).frame(false)).clicked() {
                                copen = !copen;
                                ui.ctx().data_mut(|m| m.insert_temp(key, copen));
                            }
                            let (r, _) = ui.allocate_exact_size(egui::vec2(10., 10.), egui::Sense::hover());
                            ui.painter().rect_filled(r, 2., Color32::from_rgb(color[0], color[1], color[2]));
                            ui.label(RichText::new(cname).strong());
                        });
                        if !copen {
                            continue;
                        }
                        if feats.is_empty() {
                            ui.horizontal(|ui| {
                                line(ui, 26.);
                                ui.label(RichText::new(tr("sem comandos")).weak().italics().small());
                            });
                        }
                        for (fid, ficon, fname, sup) in feats {
                            ui.horizontal_wrapped(|ui| {
                                line(ui, 26.);
                                let mut t = RichText::new(format!("{ficon} {fname}"));
                                if *sup {
                                    t = t.strikethrough().weak();
                                }
                                let r = ui.selectable_label(false, t).on_hover_text(tr("2 cliques: abre a peça (objeto de trabalho) neste recurso"));
                                if r.double_clicked() {
                                    ui.ctx().data_mut(|m| m.insert_temp(egui::Id::unique("asm_recurso_editar"), (n.id, *fid)));
                                }
                            });
                        }
                    }
                    for (title, items, k) in [(tr("📁 Geometria").to_string(), o.geo.iter().map(|(i, s)| format!("{i} {s}")).collect::<Vec<_>>(), "g"), (format!("ƒ {}", tr("Parâmetros")), o.params.clone(), "p")] {
                        if items.is_empty() {
                            continue;
                        }
                        let key = egui::Id::unique(("asm_sec", n.id.0, k));
                        let mut sopen = ui.ctx().data(|m| m.get_temp::<bool>(key)).unwrap_or(false);
                        ui.horizontal(|ui| {
                            line(ui, 0.);
                            if ui.add(egui::Button::new(RichText::new(if sopen { "⏷" } else { "⏵" }).size(10.)).frame(false)).clicked() {
                                sopen = !sopen;
                                ui.ctx().data_mut(|m| m.insert_temp(key, sopen));
                            }
                            ui.label(format!("{title} ({})", items.len()));
                        });
                        if sopen {
                            for s in items {
                                ui.horizontal(|ui| {
                                    line(ui, 26.);
                                    ui.label(RichText::new(s).small());
                                });
                            }
                        }
                    }
                }
                for (d, name, sub) in info.subs.get(&n.id).map(|v| v.as_slice()).unwrap_or(&[]) {
                    ui.horizontal(|ui| {
                        ui.add_space((depth + 2 + d) as f32 * 14.);
                        ui.add(egui::Image::new(crate::icons::get(if *sub { "asm_subconjunto" } else { "nova_peca" })).fit_to_exact_size(egui::vec2(14., 14.)));
                        ui.label(RichText::new(name).small());
                    });
                }
            }
            if group_open {
                rows(ui, st, info, Some(n.id), depth + 1, new_sel, act, hover);
            }
        }
    }
    rows(ui, st, &info, None, 0, &mut new_sel, &mut act, &mut hover);
    // 2 cliques num recurso da peça: abre a peça e o recurso
    if let Some((comp, fid)) = ui.ctx().data(|m| m.get_temp::<(Id, Id)>(egui::Id::unique("asm_recurso_editar"))) {
        ui.ctx().data_mut(|m| m.remove::<(Id, Id)>(egui::Id::unique("asm_recurso_editar")));
        edit_part(app, comp);
        if app.open.doc.feature(fid).is_some() {
            app.start_session(fid, false, None);
        }
        return;
    }
    let st = app.asm.as_mut().unwrap();
    if st.asm.nodes.is_empty() {
        ui.label(RichText::new(tr("vazia — 🧩 Inserir peça ou ✚ Peça nova no contexto")).weak().italics().small());
    }
    crate::pmi_ui::tree_section(app, ui);
    let st = app.asm.as_mut().unwrap();
    // restrições
    ui.add_space(6.);
    egui::CollapsingHeader::new(format!("🔗 {} ({})", tr("Restrições e juntas"), st.asm.mates.len() + st.asm.rigid.len() + st.asm.joints.len() + st.asm.couplings.len() + st.asm.patterns.len())).default_open(true).show(ui, |ui| {
        for m in &st.asm.mates {
            let failed = st.report.failed.contains(&m.id);
            let txt = format!("{} {}  ({} ↔ {}){}", m.kind.icon(), m.name, st.asm.name_of(m.a.comp), st.asm.name_of(m.b.comp), if failed { " ✖" } else { "" });
            let mut t = RichText::new(txt).small();
            if failed {
                t = t.color(crate::theme::st_err());
            }
            if m.suppressed {
                t = t.strikethrough().weak();
            }
            let r = ui.selectable_label(st.sel == Some(m.id), t);
            if r.clicked() {
                new_sel = Some(m.id);
            }
            if r.hovered() {
                hover = Some(m.a.comp);
            }
            r.context_menu(|ui| {
                if ui.button(if m.suppressed { tr("Reativar") } else { tr("Suprimir") }).clicked() {
                    act = Some((m.id, 11));
                    ui.close();
                }
                if ui.button(tr("🗑 Apagar   Del")).clicked() {
                    act = Some((m.id, 0));
                    ui.close();
                }
            });
        }
        for j in &st.asm.joints {
            let failed = st.report.failed.contains(&j.id);
            let drive = match (&j.drive_angle, &j.drive_len) {
                (Some(e), _) => format!("  ⟳ {e}°"),
                (_, Some(e)) => format!("  ↕ {e} mm"),
                _ => String::new(),
            };
            let mut t = RichText::new(format!("⚙ {}  ({} ↔ {}){drive}{}", j.name, st.asm.name_of(j.a.comp), st.asm.name_of(j.b.comp), if failed { " ✖" } else { "" })).small();
            if failed {
                t = t.color(crate::theme::st_err());
            }
            if j.suppressed {
                t = t.strikethrough().weak();
            }
            let r = ui.selectable_label(st.sel == Some(j.id), t);
            if r.clicked() {
                new_sel = Some(j.id);
            }
            if r.hovered() {
                hover = Some(j.b.comp);
            }
            r.context_menu(|ui| {
                if ui.button(tr("🗑 Apagar   Del")).clicked() {
                    act = Some((j.id, 0));
                    ui.close();
                }
            });
        }
        for c in &st.asm.couplings {
            let r = ui.selectable_label(st.sel == Some(c.id), RichText::new(format!("⚭ {}  ({} → {}, {})", c.name, st.asm.name_of(c.a), st.asm.name_of(c.b), c.ratio)).small());
            if r.clicked() {
                new_sel = Some(c.id);
            }
            r.context_menu(|ui| {
                if ui.button(tr("🗑 Apagar   Del")).clicked() {
                    act = Some((c.id, 0));
                    ui.close();
                }
            });
        }
        for p in &st.asm.patterns {
            let r = ui.selectable_label(st.sel == Some(p.id), RichText::new(format!("⁂ {}  ({} × {})", p.name, st.asm.name_of(p.source), p.instances.len() + 1)).small());
            if r.clicked() {
                new_sel = Some(p.id);
            }
            if r.hovered() {
                hover = Some(p.source);
            }
            r.context_menu(|ui| {
                if ui.button(tr("🗑 Apagar   Del")).clicked() {
                    act = Some((p.id, 0));
                    ui.close();
                }
            });
        }
        for f in &st.asm.features {
            let names: Vec<String> = f.targets.iter().map(|c| st.asm.name_of(*c)).collect();
            let mut t = RichText::new(format!("🛠 {}  ({})", f.name, names.join(", "))).small();
            if f.suppressed {
                t = t.strikethrough().weak();
            }
            let r = ui.selectable_label(st.sel == Some(f.id), t);
            if r.clicked() {
                new_sel = Some(f.id);
            }
            if r.hovered() {
                hover = f.targets.first().copied();
            }
            r.context_menu(|ui| {
                if ui.button(if f.suppressed { tr("Reativar") } else { tr("Suprimir") }).clicked() {
                    act = Some((f.id, 11));
                    ui.close();
                }
                if ui.button(tr("🗑 Apagar   Del")).clicked() {
                    act = Some((f.id, 0));
                    ui.close();
                }
            });
        }
        for g in &st.asm.rigid {
            let names: Vec<String> = g.comps.iter().map(|c| st.asm.name_of(*c)).collect();
            let r = ui.selectable_label(st.sel == Some(g.id), RichText::new(format!("🔒 {}  ({})", g.name, names.join(", "))).small());
            if r.clicked() {
                new_sel = Some(g.id);
            }
            r.context_menu(|ui| {
                if ui.button(tr("🗑 Apagar   Del")).clicked() {
                    act = Some((g.id, 0));
                    ui.close();
                }
            });
        }
    });
    // ---- aplica
    if let Some(s) = new_sel {
        if st.sel != Some(s) {
            st.props = false;
        }
        st.sel = Some(s);
        if st.isolate {
            st.dirty = true;
        }
    }
    // item escolhido: as peças envolvidas ficam realçadas (sem passar o mouse)
    let st_ro = app.asm.as_ref().unwrap();
    let hover = hover.or_else(|| {
        let s = st_ro.sel?;
        st_ro.asm.mates.iter().find(|m| m.id == s).map(|m| m.b.comp)
            .or_else(|| st_ro.asm.joints.iter().find(|j| j.id == s).map(|j| j.b.comp))
            .or_else(|| st_ro.asm.patterns.iter().find(|p| p.id == s).map(|p| p.source))
    });
    app.set_tree_hover(hover);
    let st = app.asm.as_mut().unwrap();
    if let Some((id, k)) = ui.ctx().data(|m| m.get_temp::<(Id, Id)>(egui::Id::unique("asm_mover"))) {
        ui.ctx().data_mut(|m| m.remove::<(Id, Id)>(egui::Id::unique("asm_mover")));
        st.push_undo();
        if let Some(n) = st.asm.node_mut(id) {
            n.parent = (k != Id(0)).then_some(k);
        }
        st.dirty = true;
    }
    if let Some((id, name)) = ui.ctx().data(|m| m.get_temp::<(Id, String)>(egui::Id::unique("asm_ren_ok"))) {
        ui.ctx().data_mut(|m| m.remove::<(Id, String)>(egui::Id::unique("asm_ren_ok")));
        st.renaming = None;
        st.push_undo();
        if let Some(n) = st.asm.node_mut(id) {
            n.name = name.trim().to_string();
        }
        st.dirty = true;
    }
    if let Some((id, k)) = act {
        node_action(app, id, k);
    }
}

/// Ações da árvore: 0 apagar, 1 ocultar, 2 editar peça, 3 editável, 4 visualização, 5 fixar,
/// 6 substituir, 7 subir, 8 descer, 9 renomear, 10 isolar, 11 suprimir restrição.
pub fn node_action(app: &mut App, id: Id, k: u8) {
    let Some(st) = app.asm.as_mut() else { return };
    match k {
        2 => return edit_part(app, id),
        12 => {
            st.sel = Some(id);
            st.props = true;
            return;
        }
        6 => {
            st.sel = Some(id);
            command(app, Cmd::AsmReplace);
            return;
        }
        9 => {
            st.renaming = Some(id);
            return;
        }
        10 => {
            st.sel = Some(id);
            st.isolate = !st.isolate;
            st.dirty = true;
            return;
        }
        _ => {}
    }
    st.push_undo();
    match k {
        0 => {
            st.asm.delete(id);
            if st.sel == Some(id) {
                st.sel = None;
            }
        }
        1 => {
            if let Some(n) = st.asm.node_mut(id) {
                n.visible = !n.visible;
            }
        }
        3 | 4 => {
            if let Some(n) = st.asm.node_mut(id) {
                n.mode = if k == 3 { LoadMode::Editavel } else { LoadMode::Visualizacao };
            }
        }
        5 => {
            if let Some(n) = st.asm.node_mut(id) {
                n.fixed = !n.fixed;
            }
            st.solve();
        }
        7 | 8 => {
            let i = st.asm.nodes.iter().position(|n| n.id == id).unwrap_or(0);
            let parent = st.asm.node(id).and_then(|n| n.parent);
            let sibs: Vec<usize> = st.asm.nodes.iter().enumerate().filter(|(_, n)| n.parent == parent).map(|(j, _)| j).collect();
            let pos = sibs.iter().position(|j| *j == i).unwrap_or(0);
            let to = if k == 7 { pos.checked_sub(1) } else { (pos + 1 < sibs.len()).then_some(pos + 1) };
            if let Some(t) = to {
                st.asm.nodes.swap(i, sibs[t]);
            }
        }
        11 => {
            if let Some(m) = st.asm.mates.iter_mut().find(|m| m.id == id) {
                m.suppressed = !m.suppressed;
            }
            if let Some(f) = st.asm.features.iter_mut().find(|f| f.id == id) {
                f.suppressed = !f.suppressed;
            }
            st.solve();
        }
        _ => {}
    }
    st.dirty = true;
}

// ============================================================== painel

/// Ícone, tipo, nome e selo do item escolhido (componente, restrição, junta, padrão…).
fn item_info(st: &AsmState, sel: Option<Id>) -> Option<(String, String, String, (&'static str, Color32))> {
    let s = sel?;
    let ok = ("OK", crate::theme::st_ok());
    let bad = ("ERRO", Color32::from_rgb(255, 110, 95));
    let off = ("SUPRIMIDA", Color32::from_gray(150));
    if let Some(n) = st.asm.node(s) {
        let (icon, kind) = match n.kind {
            NodeKind::Group => ("📁", tr("Grupo")),
            NodeKind::Part { .. } => ("🧩", tr("Peça")),
            NodeKind::SubAssembly { .. } => ("🗂", tr("Subconjunto")),
        };
        let chip = if st.errors.iter().any(|(i, _)| *i == s) { bad } else if n.fixed { ("FIXO", Color32::from_rgb(120, 170, 255)) } else { ok };
        return Some((icon.into(), kind.to_string(), n.name.clone(), chip));
    }
    if let Some(m) = st.asm.mates.iter().find(|m| m.id == s) {
        let chip = if m.suppressed { off } else if st.report.failed.contains(&s) { bad } else { ok };
        return Some((m.kind.icon().to_string(), tr(m.kind.label()).to_string(), m.name.clone(), chip));
    }
    if let Some(j) = st.asm.joints.iter().find(|j| j.id == s) {
        let chip = if j.suppressed { off } else if st.report.failed.contains(&s) { bad } else { ok };
        return Some(("⚙".into(), tr(j.kind.label()).to_string(), j.name.clone(), chip));
    }
    if let Some(c) = st.asm.couplings.iter().find(|c| c.id == s) {
        return Some(("⚭".into(), tr("Acoplamento").to_string(), c.name.clone(), ok));
    }
    if let Some(p) = st.asm.patterns.iter().find(|p| p.id == s) {
        return Some(("⁂".into(), tr("Padrão de componentes").to_string(), p.name.clone(), ok));
    }
    if let Some(f) = st.asm.features.iter().find(|f| f.id == s) {
        return Some(("🛠".into(), tr(f.kind.label()).to_string(), f.name.clone(), if f.suppressed { off } else { ok }));
    }
    st.asm.rigid.iter().find(|g| g.id == s).map(|g| ("🔒".to_string(), tr("Fixação conjunta").to_string(), g.name.clone(), ok))
}

pub fn panel(app: &mut App, ui: &mut egui::Ui) {
    let Some(st) = app.asm.as_mut() else {
        ui.heading(tr("Montagem"));
        ui.label(RichText::new(tr("Crie uma montagem nova ou abra uma existente pela árvore à esquerda.")).weak());
        return;
    };
    if let Some(m) = st.pending {
        egui::Frame::NONE.fill(Color32::from_rgba_unmultiplied(40, 90, 160, 60)).inner_margin(6.).corner_radius(4.).show(ui, |ui| {
            ui.label(RichText::new(format!("{} — {}", tr("Restrição"), format!("{m:?}"))).strong());
            ui.label(format!("{} {}/2", tr("Clique nos dois elementos na viewport:"), app.picks.len()));
            if ui.button(tr("Cancelar")).clicked() {
                st.pending = None;
            }
        });
        ui.add_space(6.);
    }
    let sel = st.sel;
    if let Some(s) = sel {
        if st.asm.pmi.iter().any(|p| p.id == s) {
            crate::pmi_ui::panel(app, ui, s);
            return;
        }
    }
    let st = app.asm.as_mut().unwrap();
    // edição do item escolhido: guarda o estado de antes (item recém-criado: o de antes de criar)
    if st.edit_snap.as_ref().map(|s| s.0) != sel {
        st.edit_snap = sel.map(|s| {
            let prev = st.undo.last().filter(|u| !u.has_item(s)).cloned();
            (s, prev.unwrap_or_else(|| st.asm.clone()))
        });
    }
    let mut bar: Option<u8> = None;
    if let (Some(s), Some((icon, kind, name, chip))) = (sel, item_info(st, sel)) {
        // barra do comando, como nos outros módulos
        egui::Frame::NONE.fill(ui.visuals().faint_bg_color).inner_margin(6.).corner_radius(6.).show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new(tr("✔ Concluir")).strong()).min_size(egui::vec2(100., 28.))).on_hover_text(tr("Enter")).clicked() {
                    bar = Some(0);
                }
                if ui.add(egui::Button::new(tr("✖ Cancelar")).min_size(egui::vec2(90., 28.))).on_hover_text(tr("Esc")).clicked() {
                    bar = Some(1);
                }
                if ui.add(egui::Button::new(tr("🗑 Excluir")).min_size(egui::vec2(80., 28.))).on_hover_text(tr("Del")).clicked() {
                    bar = Some(2);
                }
            });
        });
        if !ui.ctx().text_edit_focused() {
            ui.input(|i| {
                if i.key_pressed(egui::Key::Enter) {
                    bar = Some(0);
                } else if i.key_pressed(egui::Key::Escape) {
                    bar = Some(1);
                }
            });
        }
        ui.add_space(4.);
        egui::Frame::NONE.fill(ui.visuals().faint_bg_color).corner_radius(12.).inner_margin(10.).show(ui, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(38., 38.), egui::Sense::hover());
                let acc = crate::theme::accent(ui.visuals());
                ui.painter().rect_filled(r, 9., acc.gamma_multiply(0.18));
                ui.painter().rect_stroke(r, 9., egui::Stroke::new(1., acc.gamma_multiply(0.5)), egui::StrokeKind::Inside);
                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, &icon, egui::FontId::proportional(22.), acc);
                ui.vertical(|ui| {
                    ui.label(RichText::new(&kind).size(17.).strong());
                    ui.label(RichText::new(&name).weak());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    let (t, c) = chip;
                    crate::editor::status_chip(ui, tr(t), c);
                });
            });
        });
        let _ = s;
        crate::editor::tabs_begin(ui, ("montagem", kind.clone()));
    } else {
        if st.props {
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new(tr("✔ Concluir")).strong()).min_size(egui::vec2(100., 28.))).on_hover_text(tr("Enter")).clicked() {
                    bar = Some(0);
                }
            });
            ui.add_space(4.);
        }
        crate::editor::tabs_begin(ui, "montagem_raiz");
    }
    let before = st.asm.clone();
    let mut resolve = false;
    let mut reinit: Option<Id> = None;
    let picked_comps = st.comps_of(&app.picks);
    match sel.and_then(|s| st.asm.nodes.iter().position(|n| n.id == s)) {
        Some(i) => {
            let n = &mut st.asm.nodes[i];
            let kind_txt = match &n.kind {
                NodeKind::Group => tr("Grupo (organização)").to_string(),
                NodeKind::Part { path } => format!("🧩 {}", path),
                NodeKind::SubAssembly { path } => format!("🗂 {}", path),
            };
            crate::editor::section(ui, "Componente");
            ui.label(RichText::new(kind_txt).weak().small());
            ui.horizontal(|ui| {
                ui.label(tr("Nome"));
                ui.text_edit_singleline(&mut n.name);
            });
            if !matches!(n.kind, NodeKind::Group) {
                ui.horizontal(|ui| {
                    ui.label(tr("Número"));
                    { let __r = ui.add(crate::ui_ext::dv(&mut n.number).range(0..=9999)); crate::ui_ext::wheel(ui, &__r, &mut n.number); __r };
                });
                crate::editor::check(ui, &mut n.fixed, "📌 fixo (referência, sem graus de liberdade)");
                ui.horizontal_wrapped(|ui| {
                    ui.radio_value(&mut n.mode, LoadMode::Editavel, tr("editável"));
                    ui.radio_value(&mut n.mode, LoadMode::Visualizacao, tr("só visualização"));
                });
                let mut own = n.color.is_some();
                ui.horizontal(|ui| {
                    if crate::editor::check(ui, &mut own, "cor própria").changed() {
                        n.color = own.then_some([210, 160, 90]);
                    }
                    if let Some(c) = &mut n.color {
                        ui.color_edit_button_srgb(c);
                    }
                });
                crate::editor::section(ui, "Posição");
                ui.horizontal(|ui| {
                    for (k, l) in ["X", "Y", "Z"].iter().enumerate() {
                        ui.label(*l);
                        { let __r = ui.add(crate::ui_ext::dv(&mut n.placement.origin[k]).speed(0.5).max_decimals(3)); crate::ui_ext::wheel(ui, &__r, &mut n.placement.origin[k]); __r };
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Girar 90°:"));
                    for (l, w) in [("X", [1., 0., 0.]), ("Y", [0., 1., 0.]), ("Z", [0., 0., 1.])] {
                        if ui.small_button(format!("⟳ {l}")).clicked() {
                            let o = n.placement.origin;
                            n.placement = n.placement.moved([0.; 3], w.map(|v: f64| v * std::f64::consts::FRAC_PI_2), o);
                        }
                    }
                });
                crate::editor::section(ui, "Movimento");
                crate::editor::check(ui, &mut st.respect, "mover respeitando as restrições");
                crate::editor::check(ui, &mut st.stop_collision, "parar ao encostar (colisão)");
                ui.label(RichText::new(tr("Arraste as setas e quadrados na viewport para mover o componente.")).weak().small());
                if matches!(st.asm.nodes[i].kind, NodeKind::Part { .. }) && ui.button(tr("✏ Editar a peça")).clicked() {
                    let id = st.asm.nodes[i].id;
                    edit_part(app, id);
                    return;
                }
            }
        }
        None => {
            let mi = sel.and_then(|s| st.asm.mates.iter().position(|m| m.id == s));
            let pi = sel.and_then(|s| st.asm.patterns.iter().position(|p| p.id == s));
            let ji = sel.and_then(|s| st.asm.joints.iter().position(|j| j.id == s));
            let ci = sel.and_then(|s| st.asm.couplings.iter().position(|c| c.id == s));
            let fi = sel.and_then(|s| st.asm.features.iter().position(|f| f.id == s));
            if let Some(i) = fi {
                let names: std::collections::HashMap<Id, String> = st.asm.nodes.iter().map(|n| (n.id, n.name.clone())).collect();
                let f = &mut st.asm.features[i];
                crate::editor::section(ui, "Recurso entre peças");
                ui.horizontal(|ui| {
                    ui.label(tr("Nome"));
                    ui.text_edit_singleline(&mut f.name);
                });
                match &mut f.kind {
                    AsmFeatKind::Cut { tool, .. } => {
                        ui.label(format!("{} {}", tr("Peça que corta:"), names.get(tool).cloned().unwrap_or_default()));
                        ui.label(RichText::new(tr("A forma dela é subtraída das peças afetadas, na posição atual (move a peça e o recorte acompanha).")).weak().small());
                    }
                    AsmFeatKind::Hole { p, dia, depth, .. } => {
                        crate::editor::expr(ui, "Diâmetro", dia, &Default::default(), "mm");
                        crate::editor::expr(ui, "Profundidade", depth, &Default::default(), "mm");
                        ui.label(RichText::new(format!("{} {} · {}", tr("início"), crate::app::p3(p, 1), tr("profundidade vazia = passante"))).weak().small());
                    }
                }
                crate::editor::section(ui, "Peças afetadas");
                let mut del = None;
                for (k, t) in f.targets.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("🧩 {}", names.get(t).cloned().unwrap_or_default()));
                        if ui.small_button("✖").clicked() {
                            del = Some(k);
                        }
                    });
                }
                if let Some(k) = del {
                    f.targets.remove(k);
                }
                let add: Vec<Id> = picked_comps.iter().copied().filter(|c| !f.targets.contains(c) && !matches!(f.kind, AsmFeatKind::Cut { tool, .. } if tool == *c)).collect();
                if ui.add_enabled(!add.is_empty(), egui::Button::new(format!("{} ({})", tr("+ peças clicadas na viewport"), add.len()))).clicked() {
                    f.targets.extend(add);
                }
                crate::editor::section(ui, "Estado");
                crate::editor::check(ui, &mut f.suppressed, "suprimido");
            } else if let Some(i) = ji {
                let others: Vec<(Id, String)> = st.asm.joints.iter().filter(|o| o.id != st.asm.joints[i].id).map(|o| (o.id, o.name.clone())).collect();
                let (na, nb) = (st.asm.name_of(st.asm.joints[i].a.comp), st.asm.name_of(st.asm.joints[i].b.comp));
                let pl: std::collections::HashMap<Id, Placement> = st.asm.nodes.iter().map(|n| (n.id, n.placement)).collect();
                let now = kin::measure(&st.asm.joints[i], &pl);
                let j = &mut st.asm.joints[i];
                crate::editor::section(ui, "Junta");
                ui.horizontal(|ui| {
                    ui.label(tr("Nome"));
                    ui.text_edit_singleline(&mut j.name);
                });
                ui.label(RichText::new(format!("{} · {na} ↔ {nb}", tr(j.kind.label()))).weak());
                if let Some((ang, len)) = now {
                    ui.label(RichText::new(format!("agora: {}° · {} mm", fmt(ang), fmt(len))).small());
                }
                if let JointKind::Screw { pitch } = &mut j.kind {
                    crate::editor::expr(ui, "Passo (mm/volta)", pitch, &Default::default(), "mm");
                }
                if let JointKind::Cam { profile, radius } = &mut j.kind {
                    crate::editor::expr(ui, "Raio do rolete", radius, &Default::default(), "mm");
                    ui.label(RichText::new(format!("{} {}", profile.len(), tr("pontos no perfil do came"))).weak().small());
                }
                let (can_a, can_l) = j.kind.drives();
                crate::editor::section(ui, "Comando");
                if can_a {
                    let mut on = j.drive_angle.is_some();
                    if crate::editor::check(ui, &mut on, "comando de ângulo (graus; aceita lei em t)").changed() {
                        j.drive_angle = on.then(|| "0".to_string());
                    }
                    if let Some(e) = &mut j.drive_angle {
                        ui.text_edit_singleline(e);
                        let mut v: f64 = e.trim().replace(',', ".").parse().unwrap_or(0.);
                        if ui.add(crate::ui_ext::slider(&mut v, -180.0..=180.0).text("°")).changed() {
                            *e = fmt(v);
                        }
                    }
                }
                if can_l {
                    let mut on = j.drive_len.is_some();
                    if crate::editor::check(ui, &mut on, "comando de curso (mm; aceita lei em t)").changed() {
                        j.drive_len = on.then(|| "0".to_string());
                    }
                    if let Some(e) = &mut j.drive_len {
                        ui.text_edit_singleline(e);
                        let mut v: f64 = e.trim().replace(',', ".").parse().unwrap_or(0.);
                        if ui.add(crate::ui_ext::slider(&mut v, -200.0..=200.0).text("mm")).changed() {
                            *e = fmt(v);
                        }
                    }
                }
                crate::editor::check(ui, &mut j.suppressed, "suprimida");
                let jid = j.id;
                if !others.is_empty() {
                    crate::editor::section(ui, "Acoplar a outra junta");
                    for (oid, on) in &others {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(on).small());
                            for (k, l) in [(CouplingKind::Gear, "engrenagem"), (CouplingKind::RackPinion, "pinhão-cremalheira"), (CouplingKind::Cable, "cabo")] {
                                if ui.small_button(tr(l)).clicked() {
                                    ui.ctx().data_mut(|m| m.insert_temp(egui::Id::unique("asm_acoplar"), (jid, *oid, k as u8)));
                                }
                            }
                        });
                    }
                }
            } else if let Some(i) = ci {
                let c = &mut st.asm.couplings[i];
                crate::editor::section(ui, "Acoplamento");
                ui.horizontal(|ui| {
                    ui.label(tr("Nome"));
                    ui.text_edit_singleline(&mut c.name);
                });
                let hint = match c.kind {
                    CouplingKind::Gear => "razão = ângulo da 2ª ÷ ângulo da 1ª (externas: −z1/z2)",
                    CouplingKind::RackPinion => "razão = mm de curso por volta do pinhão (π × diâmetro primitivo)",
                    CouplingKind::Cable => "razão = curso da 2ª ÷ curso da 1ª",
                };
                crate::editor::expr(ui, "Razão", &mut c.ratio, &Default::default(), "");
                ui.label(RichText::new(tr(hint)).weak().small());
            } else if let Some(i) = pi {
                let src = st.asm.name_of(st.asm.patterns[i].source);
                let p = &mut st.asm.patterns[i];
                crate::editor::section(ui, "Padrão");
                ui.horizontal(|ui| {
                    ui.label(tr("Nome"));
                    ui.text_edit_singleline(&mut p.name);
                });
                ui.label(RichText::new(format!("{} {src}", tr("Original:"))).weak());
                let circ = matches!(p.kind, PatternKind::Circular { .. });
                ui.horizontal(|ui| {
                    if ui.radio(!circ, tr("linear")).clicked() && circ {
                        p.kind = PatternKind::Linear { dir: [1., 0., 0.], count: "3".into(), spacing: "20".into() };
                    }
                    if ui.radio(circ, tr("circular")).clicked() && !circ {
                        p.kind = PatternKind::Circular { origin: [0.; 3], axis: [0., 0., 1.], count: "4".into(), angle: "360".into() };
                    }
                });
                let v3 = |ui: &mut egui::Ui, l: &str, v: &mut [f64; 3]| {
                    ui.horizontal(|ui| {
                        ui.label(tr(l));
                        for x in v.iter_mut() {
                            ui.add(crate::ui_ext::dv(x).speed(0.1).max_decimals(3));
                        }
                    });
                };
                match &mut p.kind {
                    PatternKind::Linear { dir, count, spacing } => {
                        v3(ui, "Direção", dir);
                        crate::editor::expr(ui, "Quantidade", count, &Default::default(), "");
                        crate::editor::expr(ui, "Espaçamento", spacing, &Default::default(), "mm");
                    }
                    PatternKind::Circular { origin, axis, count, angle } => {
                        v3(ui, "Centro", origin);
                        v3(ui, "Eixo", axis);
                        crate::editor::expr(ui, "Quantidade", count, &Default::default(), "");
                        crate::editor::expr(ui, "Ângulo total", angle, &Default::default(), "°");
                    }
                }
                ui.label(RichText::new(tr("A quantidade inclui o original; as cópias se refazem quando ela muda.")).weak().small());
            } else {
            match mi {
                Some(i) => {
                    let failed = st.report.failed.contains(&st.asm.mates[i].id);
                    let (na, nb) = (st.asm.name_of(st.asm.mates[i].a.comp), st.asm.name_of(st.asm.mates[i].b.comp));
                    let m = &mut st.asm.mates[i];
                    crate::editor::section(ui, "Restrição");
                    ui.horizontal(|ui| {
                        ui.label(tr("Nome"));
                        ui.text_edit_singleline(&mut m.name);
                    });
                    ui.label(RichText::new(format!("{na}  ↔  {nb}")).weak());
                    if failed {
                        ui.label(RichText::new(tr("✖ não satisfeita: conflita com outras restrições ou é impossível")).color(crate::theme::st_err()));
                    }
                    // converter o tipo sem apagar o vínculo
                    let cur = m.kind.label();
                    egui::ComboBox::from_id_salt("tipo_rest").selected_text(tr(cur)).show_ui(ui, |ui| {
                        for k in [
                            MateKind::Coincident { flip: false },
                            MateKind::Contact,
                            MateKind::Distance("0".into()),
                            MateKind::Angle("90".into()),
                            MateKind::Parallel,
                            MateKind::Perpendicular,
                            MateKind::Gear { ratio: "-1".into(), belt: false, refs: [[0.; 3]; 4] },
                            MateKind::Gear { ratio: "1".into(), belt: true, refs: [[0.; 3]; 4] },
                            MateKind::RackPinion { mm_per_rev: "31.4159".into(), refs: [[0.; 3]; 4] },
                        ] {
                            if ui.selectable_label(k.label() == cur, tr(k.label())).clicked() && k.label() != cur {
                                if k.is_motion() {
                                    reinit = Some(m.id);
                                }
                                m.kind = k;
                            }
                        }
                    });
                    match &mut m.kind {
                        MateKind::Coincident { flip } => {
                            ui.checkbox(flip, tr("invertida"));
                        }
                        MateKind::Distance(e) | MateKind::Angle(e) => {
                            ui.horizontal(|ui| {
                                ui.label(tr("Valor"));
                                ui.text_edit_singleline(e);
                            });
                            ui.label(RichText::new(tr("Aceita fórmulas com os parâmetros da montagem; distância negativa troca o lado.")).weak().small());
                        }
                        MateKind::Gear { ratio, belt, .. } => {
                            crate::editor::expr(ui, "Razão", ratio, &Default::default(), "");
                            let hint = if *belt { "giro da 2ª ÷ giro da 1ª = d1/d2 (mesmo sentido)" } else { "giro da 2ª ÷ giro da 1ª = −z1/z2 (engrenagens externas)" };
                            ui.label(RichText::new(tr(hint)).weak().small());
                        }
                        MateKind::RackPinion { mm_per_rev, .. } => {
                            crate::editor::expr(ui, "mm por volta", mm_per_rev, &Default::default(), "mm");
                            ui.label(RichText::new(tr("π × diâmetro primitivo do pinhão")).weak().small());
                        }
                        _ => {}
                    }
                    if m.kind.is_motion() && ui.button(tr("⟲ Posição atual = giro zero")).clicked() {
                        reinit = Some(m.id);
                    }
                    crate::editor::section(ui, "Estado");
                    crate::editor::check(ui, &mut m.suppressed, "suprimida");
                }
                None => {
                    crate::editor::section(ui, "Montagem");
                    ui.horizontal(|ui| {
                        ui.label(tr("Nome"));
                        ui.text_edit_singleline(&mut st.asm.name);
                    });
                    let parts = st.asm.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).count();
                    ui.label(format!("{parts} componente(s) · {} restrição(ões) · {} GDL livres", st.asm.mates.len(), st.report.dof));
                    crate::editor::check(ui, &mut st.asm.relational, "projeto relacional (peças novas fixas na origem; forma por links entre peças)");
                    crate::editor::check(ui, &mut st.view_only, "modo visualização (sem mover nem restringir)");
                    crate::editor::section(ui, "Parâmetros da montagem");
                    let mut del = None;
                    for (k, p) in st.asm.params.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut p.name).desired_width(80.));
                            ui.label("=");
                            ui.add(egui::TextEdit::singleline(&mut p.expr).desired_width(100.));
                            if ui.small_button("✖").clicked() {
                                del = Some(k);
                            }
                        });
                    }
                    if let Some(k) = del {
                        st.asm.params.remove(k);
                    }
                    if ui.button(tr("+ parâmetro")).clicked() {
                        let n = st.asm.params.len() + 1;
                        st.asm.params.push(AsmParam { name: format!("P{n}"), expr: "10".into() });
                    }
                }
            }
        }
    }
    }
    if let Some(id) = reinit {
        st.asm.init_motion(id);
    }
    // interferências
    if !st.interf.is_empty() {
        crate::editor::section(ui, "Interferências");
        for (a, b, v) in &st.interf {
            ui.label(RichText::new(format!("✖ {} × {} — {} mm³", st.asm.name_of(*a), st.asm.name_of(*b), fmt(*v))).color(crate::theme::st_err()));
        }
    }
    crate::editor::tabs_end(ui);
    // barra: concluir (fica), cancelar (volta ao de antes), excluir
    match (bar, sel) {
        (Some(0), _) => {
            st.sel = None;
            st.props = false;
            st.edit_snap = None;
        }
        (Some(1), _) => {
            if let Some((_, snap)) = st.edit_snap.take() {
                if st.asm != snap {
                    st.undo.push(st.asm.clone());
                    st.asm = snap;
                    st.solve();
                    st.dirty = true;
                }
            }
            st.sel = None;
            st.props = false;
            return;
        }
        (Some(2), Some(id)) => {
            st.edit_snap = None;
            node_action(app, id, 0);
            return;
        }
        _ => {}
    }
    if let Some((ja, jb, k)) = ui.ctx().data(|m| m.get_temp::<(Id, Id, u8)>(egui::Id::unique("asm_acoplar"))) {
        ui.ctx().data_mut(|m| m.remove::<(Id, Id, u8)>(egui::Id::unique("asm_acoplar")));
        let kind = match k {
            0 => CouplingKind::Gear,
            1 => CouplingKind::RackPinion,
            _ => CouplingKind::Cable,
        };
        let ratio = match kind {
            CouplingKind::Gear => "-1",
            CouplingKind::RackPinion => "31.4159",
            CouplingKind::Cable => "1",
        };
        let id = st.asm.add_coupling(kind, ja, jb, ratio);
        st.sel = Some(id);
    }
    if st.asm != before {
        if st.asm.patterns != before.patterns {
            st.asm.update_patterns();
        }
        let positions_only = st.asm.mates == before.mates && st.asm.rigid == before.rigid;
        st.undo.push(before);
        st.modified = true;
        if !positions_only || st.asm.params != st.undo.last().unwrap().params {
            resolve = true;
        }
        st.dirty = true;
    }
    if resolve {
        st.solve();
    }
}

// ============================================================== manipulador

impl AsmState {
    /// Há algo em edição (o painel de edição toma o lugar da árvore, como nos outros módulos)?
    /// Escolher um componente só o seleciona; restrições, juntas e recursos abrem a edição.
    pub fn editing(&self) -> bool {
        self.pending.is_some() || self.pending_joint.is_some() || self.props || self.sel.is_some_and(|s| !self.asm.nodes.iter().any(|n| n.id == s))
    }

    /// Componentes das seleções da viewport (sem repetir, na ordem do clique).
    fn comps_of(&self, picks: &[Pick]) -> Vec<Id> {
        let mut out = vec![];
        for p in picks {
            if let Pick::Face { body, .. } | Pick::Edge { body, .. } | Pick::Vertex { body, .. } = p {
                if let Some(c) = self.comp_of_body(*body) {
                    if !out.contains(&c) {
                        out.push(c);
                    }
                }
            }
        }
        out
    }
}

/// A peça `src` depende (por links, direta ou indiretamente) do arquivo `target`?
fn depends_on(src: &std::path::Path, target: &std::path::Path, depth: u32) -> bool {
    if depth > 8 {
        return false;
    }
    let Some(d) = std::fs::read_to_string(src).ok().and_then(|t| Document::from_json(&t).ok()) else { return false };
    d.external.iter().any(|e| {
        let p = std::path::Path::new(&e.path);
        let full = if p.is_absolute() { p.to_path_buf() } else { src.parent().map(|x| x.join(p)).unwrap_or_else(|| p.to_path_buf()) };
        full == target || depends_on(&full, target, depth + 1)
    })
}

/// Copia por link a face plana `pick` da peça `src` para a peça `target` (no contexto da
/// montagem): a face vira publicação na origem (automática) e a cópia entra na outra peça já no
/// sistema dela. Recusa referência circular.
fn link_in_context(st: &mut AsmState, src: Id, target: Id, pick: Pick) -> Result<String, String> {
    let r = st.mate_ref(pick)?;
    let Geom::Plane { p, n } = r.geom else { return Err(tr("escolha uma face plana (o link copia o plano)").to_string()) };
    let dir = st.dir();
    let (Some(ns), Some(nt)) = (st.asm.node(src).cloned(), st.asm.node(target).cloned()) else { return Err("componente não encontrado".into()) };
    let (NodeKind::Part { path: ps }, NodeKind::Part { path: pt }) = (&ns.kind, &nt.kind) else { return Err(tr("o link liga duas peças").to_string()) };
    let (fs, ft) = (full_path(dir.as_deref(), ps), full_path(dir.as_deref(), pt));
    if fs == ft {
        return Err(tr("as duas são instâncias do mesmo arquivo").to_string());
    }
    if depends_on(&fs, &ft, 0) {
        return Err(tr("referência circular: a peça de origem já usa geometria desta").to_string());
    }
    let read = |f: &std::path::Path| std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display())).and_then(|t| Document::from_json(&t).map_err(|e| e.to_string()));
    let mut ds = read(&fs)?;
    let x = any_perp(n);
    // publicação automática na origem (reaproveita o mesmo plano, se já publicado)
    let existing = ds
        .references
        .iter()
        .find(|e| e.published && matches!(&e.kind, fenkra_core::RefKind::FacePlane { origin, normal, .. } if norm(sub(*origin, p)) < 1e-6 && norm(sub(*normal, n)) < 1e-6))
        .map(|e| e.name.clone());
    let pub_name = match existing {
        Some(nm) => nm,
        None => {
            let id = ds.add_face_plane(p, x, n, None);
            let nm = (1..).map(|i| format!("Publicação{i}")).find(|c| !ds.name_taken(c)).unwrap();
            if let Some(e) = ds.references.iter_mut().find(|e| e.id == id) {
                e.name = nm.clone();
                e.published = true;
            }
            std::fs::write(&fs, ds.to_json()).map_err(|e| e.to_string())?;
            nm
        }
    };
    let geo = fenkra_core::LinkedGeo::Plane { origin: p, x, normal: n };
    // sistema da origem → mundo → sistema da peça que recebe
    let rel = nt.placement.inverse().then(&ns.placement);
    let xf: fenkra_core::LinkXf = [rel.origin, rel.rot[0], rel.rot[1], rel.rot[2]];
    let mut dt = read(&ft)?;
    let tid = dt.add_link_xf(&ns.name, &fs.to_string_lossy(), &pub_name, geo, xf);
    std::fs::write(&ft, dt.to_json()).map_err(|e| e.to_string())?;
    st.cache.forget(&fs);
    st.cache.forget(&ft);
    st.dirty = true;
    Ok(format!("{} ← {}.{} ({})", dt.name_of(tid), ns.name, pub_name, tr("publicação criada automaticamente na origem")))
}

/// Menu do botão direito na viewport da montagem: restrições possíveis entre os dois elementos
/// escolhidos e as ações do componente sob o mouse.
pub fn viewport_menu(app: &App, ui: &mut egui::Ui, target: Option<Pick>) -> Option<AsmCtx> {
    let st = app.asm.as_ref()?;
    let mut out = None;
    if let Some((_, opts)) = st.quick.as_ref().filter(|q| app.picks.len() == 2 && q.0 == app.picks && !q.1.is_empty()) {
        ui.label(RichText::new(tr("Restrição entre os dois elementos")).strong());
        for o in opts {
            if ui.button(format!("{} {}", o.icon(), tr(o.label()))).clicked() {
                out = Some(AsmCtx::Mate(*o));
                ui.close();
            }
        }
        ui.separator();
    }
    // durante a viewport a vista da montagem está em `app.result`
    let comp = target.and_then(|p| match p {
        Pick::Face { body, .. } | Pick::Edge { body, .. } | Pick::Vertex { body, .. } => app.result.as_ref().and_then(|r| r.bodies.get(body)).map(|b| b.collector),
        _ => None,
    });
    if let Some(c) = comp {
        let Some(n) = st.asm.node(c) else { return out };
        ui.label(RichText::new(&n.name).strong());
        if matches!(n.kind, NodeKind::Part { .. }) && ui.button(tr("✏ Editar a peça (objeto de trabalho)")).clicked() {
            out = Some(AsmCtx::Edit(c));
            ui.close();
        }
        if ui.button(tr("☑ Selecionar na árvore")).clicked() {
            out = Some(AsmCtx::Select(c));
            ui.close();
        }
        if ui.button(if n.fixed { tr("📌 Liberar") } else { tr("📌 Fixar") }).clicked() {
            out = Some(AsmCtx::Fix(c));
            ui.close();
        }
        if ui.button(tr("Ocultar   H")).clicked() {
            out = Some(AsmCtx::Hide(c));
            ui.close();
        }
        if ui.button(tr("🔍 Isolar")).clicked() {
            out = Some(AsmCtx::Isolate(c));
            ui.close();
        }
        ui.separator();
        if ui.button(tr("🗑 Apagar   Del")).clicked() {
            out = Some(AsmCtx::Delete(c));
            ui.close();
        }
    }
    if app.picks.len() >= 2 && ui.button(tr("🗂 Subconjunto dos selecionados")).clicked() {
        out = Some(AsmCtx::SubFromSel);
        ui.close();
    }
    if comp.is_none() && app.picks.len() < 2 {
        ui.label(RichText::new(tr("Clique numa peça (ou escolha dois elementos para restringir)")).weak());
    }
    out
}

fn apply_ctx(app: &mut App, act: AsmCtx) {
    match act {
        AsmCtx::Mate(m) => {
            if app.picks.len() >= 2 {
                let picks = app.picks.clone();
                app.picks.clear();
                create_mate(app, m, picks[0], picks[1]);
            }
        }
        AsmCtx::Fix(id) => node_action(app, id, 5),
        AsmCtx::Hide(id) => node_action(app, id, 1),
        AsmCtx::Isolate(id) => node_action(app, id, 10),
        AsmCtx::Edit(id) => edit_part(app, id),
        AsmCtx::Delete(id) => node_action(app, id, 0),
        AsmCtx::Select(id) => {
            if let Some(st) = app.asm.as_mut() {
                st.sel = Some(id);
            }
        }
        AsmCtx::SubFromSel => {
            command(app, Cmd::AsmSubFromSel);
        }
    }
}

/// Peça aberta no contexto da montagem: as outras peças aparecem em linhas claras (o conjunto
/// "pai" em volta do objeto de trabalho), no sistema da peça.
pub fn context_overlay(app: &App, ui: &mut egui::Ui, rect: egui::Rect, cam: &crate::render::Camera) {
    let Some(st) = app.asm.as_ref() else { return };
    let Some((cid, _)) = &st.editing_part else { return };
    if app.settings.bench == crate::settings::Bench::Montagem {
        return;
    }
    let (Some(pl), Some(v)) = (st.asm.node(*cid).map(|n| n.placement), st.view.as_ref()) else { return };
    let painter = ui.painter_at(rect);
    let col = Color32::from_rgba_unmultiplied(120, 170, 235, 90);
    let mut n = 0;
    for b in v.bodies.iter().filter(|b| b.collector != *cid) {
        for s in b.mesh.edges.chunks_exact(6) {
            n += 1;
            if n > 40000 {
                break;
            }
            let a = pl.to_local([s[0] as f64, s[1] as f64, s[2] as f64]);
            let c = pl.to_local([s[3] as f64, s[4] as f64, s[5] as f64]);
            let (Some(pa), Some(pc)) = (cam.project(rect, glam::Vec3::new(a[0] as f32, a[1] as f32, a[2] as f32)), cam.project(rect, glam::Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32))) else { continue };
            painter.line_segment([pa, pc], egui::Stroke::new(1., col));
        }
    }
    painter.text(rect.right_bottom() + egui::vec2(-12., -30.), egui::Align2::RIGHT_BOTTOM, format!("{} — {}", tr("contexto da montagem"), st.asm.name), egui::FontId::proportional(12.), col.gamma_multiply(2.));
}

/// Manipulador do componente selecionado: setas e quadrados (mover), anéis (girar). Com
/// "respeitar restrições" o componente segue o mouse só no que as restrições deixam; com a parada
/// por colisão ele não entra em outra peça. Também mostra a restrição rápida para duas seleções.
/// Depois da viewport (a vista da montagem já voltou para o estado): ações do menu e opções da
/// restrição rápida para as duas seleções.
fn after_view_quick(app: &mut App) {
    if let Some(act) = app.asm.as_mut().and_then(|s| s.ctx_act.take()) {
        apply_ctx(app, act);
    }
    let Some(st) = app.asm.as_mut() else { return };
    if app.picks.len() == 2 && st.quick.as_ref().map(|q| &q.0) != Some(&app.picks) {
        let picks = app.picks.clone();
        let opts = match (st.mate_ref(picks[0]), st.mate_ref(picks[1])) {
            (Ok(x), Ok(y)) if x.comp != y.comp => quick_options(&x.geom, &y.geom),
            _ => vec![],
        };
        st.quick = Some((picks, opts));
    }
}

pub fn gizmo(app: &mut App, ui: &mut egui::Ui, rect: egui::Rect, cam: &crate::render::Camera) {
    let Some(st) = app.asm.as_mut() else { return };
    // restrição rápida: dois elementos de peças diferentes escolhidos (opções calculadas depois
    // da viewport; o clique vira ação aplicada lá)
    if !st.view_only && st.pending.is_none() && st.pending_joint.is_none() && app.picks.len() == 2 && st.quick.as_ref().is_some_and(|q| q.0 == app.picks) {
        let picks = app.picks.clone();
        let opts = st.quick.as_ref().map(|q| q.1.clone()).unwrap_or_default();
        if !opts.is_empty() {
            let mut chosen = None;
            egui::Area::new(egui::Id::unique("asm_restricao_rapida")).order(egui::Order::Foreground).fixed_pos(rect.center_top() + egui::vec2(-300., 40.)).show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(tr("Restrição rápida:")).strong());
                        for o in &opts {
                            if ui.button(format!("{} {}", o.icon(), tr(o.label()))).on_hover_text(tr("cria entre os dois elementos escolhidos")).clicked() {
                                chosen = Some(*o);
                            }
                        }
                    });
                });
            });
            if let Some(m) = chosen {
                let _ = picks;
                st.ctx_act = Some(AsmCtx::Mate(m));
                return;
            }
        }
    }
    let Some(st) = app.asm.as_mut() else { return };
    if st.view_only {
        return;
    }
    let Some(id) = st.sel else { return };
    let Some(n) = st.asm.node(id) else { return };
    if matches!(n.kind, NodeKind::Group) || n.fixed {
        return;
    }
    let p = n.placement;
    let view = app.result.as_ref();
    let center = |_: &AsmState| {
        view
            .and_then(|r| r.bodies.iter().find(|b| b.collector == id))
            .map(|b| {
                let k = (b.mesh.positions.len() / 3).max(1) as f32;
                b.mesh.positions.chunks_exact(3).fold(glam::Vec3::ZERO, |a, q| a + glam::Vec3::new(q[0], q[1], q[2])) / k
            })
    };
    if st.explode_mode {
        let off = st.live.offset(id);
        let c = center(st).unwrap_or(glam::Vec3::ZERO);
        if let Some(v) = crate::gizmo::translate_gizmo(ui, rect, cam, id.0 ^ 0x5eed, c, [glam::Vec3::X, glam::Vec3::Y, glam::Vec3::Z], off, [false; 3]) {
            st.live.set_offset(id, v);
            st.dirty = true;
        }
        return;
    }
    // centro do componente na tela: centro da malha posicionada
    let c = center(st).unwrap_or(glam::Vec3::new(p.origin[0] as f32, p.origin[1] as f32, p.origin[2] as f32));
    let mut cand: Option<Placement> = None;
    if let Some(v) = crate::gizmo::translate_gizmo(ui, rect, cam, id.0, c, [glam::Vec3::X, glam::Vec3::Y, glam::Vec3::Z], p.origin, [false; 3]) {
        let base = st.drag_start.unwrap_or(p);
        cand = Some(Placement { origin: v, rot: base.rot });
    }
    // anéis de rotação em volta dos eixos do mundo, pelo centro do componente
    let ppm = {
        let (r, _) = cam.basis();
        match (cam.project(rect, c), cam.project(rect, c + r)) {
            (Some(a), Some(b)) => (b - a).length().max(1e-3),
            _ => 1.,
        }
    };
    let rad = 95. / ppm;
    let axes = [glam::Vec3::X, glam::Vec3::Y, glam::Vec3::Z];
    // alças em diagonais diferentes (não caem umas sobre as outras nem sobre as setas)
    let starts = [(glam::Vec3::Y + glam::Vec3::Z).normalize(), (glam::Vec3::Z + glam::Vec3::X).normalize(), (glam::Vec3::X + glam::Vec3::Y).normalize()];
    for (k, ax) in axes.iter().enumerate() {
        let src = c + starts[k] * rad;
        let cur = st.rot_drag.filter(|r| r.0 == k).map(|r| r.1).unwrap_or(0.);
        if let Some(v) = crate::gizmo::rotate_arc_col(ui, rect, cam, id.0.wrapping_mul(31).wrapping_add(k as u64 + 7), c, *ax, src, cur, false, crate::gizmo::AXIS_COL[k]) {
            let start = st.drag_start.unwrap_or(p);
            let about = *st.drag_center.get_or_insert([c.x as f64, c.y as f64, c.z as f64]);
            st.rot_drag = Some((k, v));
            let w = [0, 1, 2].map(|i| ax[i] as f64 * v.to_radians());
            cand = Some(start.moved([0.; 3], w, about));
        }
    }
    let released = ui.input(|i| i.pointer.any_released());
    if let Some(q) = cand {
        if st.drag_start.is_none() {
            st.drag_start = Some(p);
            st.snapshot = Some(st.asm.clone());
        }
        // tentativa: com restrições, o componente vai até onde elas deixam
        let mut trial = st.asm.clone();
        let constrained = st.respect && (!trial.mates.is_empty() || !trial.joints.is_empty());
        if constrained {
            fenkra_montagem::solve::solve_toward(&mut trial, id, q, 0.05);
        } else if let Some(n) = trial.node_mut(id) {
            n.placement = q;
        }
        let dir = st.dir();
        st.contact = st.stop_collision && fenkra_montagem::view::collides(&trial, id, dir.as_deref(), &mut st.cache, 0.01);
        if !st.contact {
            st.asm = trial;
            st.dirty = true;
        }
    } else if released {
        st.rot_drag = None;
        st.drag_center = None;
        if let (Some(_), Some(snap)) = (st.drag_start.take(), st.snapshot.take()) {
            st.undo.push(snap);
            st.modified = true;
            if st.respect && !st.asm.mates.is_empty() {
                st.solve();
            }
            st.dirty = true;
        }
        st.contact = false;
    }
    if st.contact {
        if let Some(s) = cam.project(rect, c) {
            ui.painter_at(rect).text(s + egui::vec2(18., 18.), egui::Align2::LEFT_TOP, tr("⛔ contato — o movimento parou na colisão"), egui::FontId::proportional(13.), Color32::from_rgb(255, 120, 90));
        }
    }
}

// ============================================================== simulação

/// Janela de simulação: leis no tempo, quadros, reprodução, análise de GDL e gráfico de
/// velocidade/aceleração do componente acompanhado.
pub fn sim_window(app: &mut App, ctx: &egui::Context) {
    let Some(st) = app.asm.as_mut() else { return };
    if !st.sim_open {
        return;
    }
    let mut open = true;
    let (sem, com) = kin::analyze(&st.asm);
    let mut compute = false;
    let mut restore = false;
    let fechar = egui::Id::unique(("concluir_janela", "asm_ui.rs5"));
    if ctx.data_mut(|m| m.remove_temp::<bool>(fechar)).unwrap_or(false) {
        open = false;
    }
    egui::Window::new(tr("▶ Simulação cinemática"))
        .id(egui::Id::unique("janela_simulacao"))
        .open(&mut open)
        .default_width(460.)
        .resizable(true)
        .show(ctx, |ui| {
            if crate::app::done_bar(ui) {
                ui.ctx().data_mut(|m| m.insert_temp(fechar, true));
            }
            let ok = com == 0;
            ui.label(RichText::new(format!("Mecanismo: {sem} GDL sem comandos · {com} com os comandos")).strong());
            ui.label(
                RichText::new(if ok {
                    tr("✔ cada grau de liberdade tem comando: pode simular")
                } else if com > 0 {
                    tr("⚠ faltam comandos: o que sobra fica parado na posição atual")
                } else {
                    tr("")
                })
                .color(if ok { Color32::from_rgb(120, 210, 130) } else { crate::theme::st_warn() }),
            );
            crate::editor::section(ui, "Comandos (leis no tempo t, em segundos)");
            for j in st.asm.joints.iter_mut().filter(|j| !j.suppressed) {
                let (ca, cl) = j.kind.drives();
                if !(ca || cl) {
                    continue;
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&j.name).small());
                    if ca {
                        let mut on = j.drive_angle.is_some();
                        if ui.checkbox(&mut on, "°").changed() {
                            j.drive_angle = on.then(|| "90*t".to_string());
                        }
                        if let Some(e) = &mut j.drive_angle {
                            ui.add(egui::TextEdit::singleline(e).desired_width(110.));
                        }
                    }
                    if cl {
                        let mut on = j.drive_len.is_some();
                        if ui.checkbox(&mut on, "mm").changed() {
                            j.drive_len = on.then(|| "10*t".to_string());
                        }
                        if let Some(e) = &mut j.drive_len {
                            ui.add(egui::TextEdit::singleline(e).desired_width(110.));
                        }
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.label(tr("Duração"));
                { let __r = ui.add(crate::ui_ext::dv(&mut st.sim_t1).range(0.01..=600.0).suffix(" s").speed(0.1)); crate::ui_ext::wheel(ui, &__r, &mut st.sim_t1); __r };
                ui.label(tr("Quadros"));
                { let __r = ui.add(crate::ui_ext::dv(&mut st.sim_steps).range(2..=2000)); crate::ui_ext::wheel(ui, &__r, &mut st.sim_steps); __r };
                if ui.button(tr("⏺ Calcular quadros")).clicked() {
                    compute = true;
                }
            });
            if !st.sim_frames.is_empty() {
                let fails = st.sim_frames.iter().filter(|f| !f.ok).count();
                ui.label(RichText::new(format!("{} quadros{}", st.sim_frames.len(), if fails > 0 { format!(" · {fails} sem solução (trava ou fim de curso)") } else { String::new() })).small());
                ui.horizontal(|ui| {
                    if ui.button(if st.sim_play { "⏸" } else { "▶" }).clicked() {
                        st.sim_play = !st.sim_play;
                    }
                    if ui.button("⏮").clicked() {
                        st.sim_t = 0.;
                    }
                    let t1 = st.sim_t1;
                    ui.add(crate::ui_ext::slider(&mut st.sim_t, 0.0..=t1).text("t (s)"));
                    if ui.button(tr("Repor")).on_hover_text(tr("volta às posições de antes da simulação")).clicked() {
                        restore = true;
                    }
                });
                // gráfico de velocidade e aceleração do componente acompanhado
                let comps: Vec<(Id, String)> = st.asm.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).map(|n| (n.id, n.name.clone())).collect();
                egui::ComboBox::from_id_salt("sim_track").selected_text(st.sim_track.map(|c| st.asm.name_of(c)).unwrap_or_else(|| tr("acompanhar componente…").into())).show_ui(ui, |ui| {
                    for (id, n) in &comps {
                        ui.selectable_value(&mut st.sim_track, Some(*id), n);
                    }
                });
                if let Some(c) = st.sim_track {
                    let k = kin::kinematics_of(&st.sim_frames, c, [0.; 3]);
                    let vmax = k.iter().map(|x| x.2).fold(1e-9, f64::max);
                    let amax = k.iter().map(|x| x.3).fold(1e-9, f64::max);
                    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 120.), egui::Sense::hover());
                    let p = ui.painter_at(r);
                    p.rect_filled(r, 4., ui.visuals().extreme_bg_color);
                    let to = |t: f64, v: f64, m: f64| egui::pos2(r.left() + (t / st.sim_t1.max(1e-9)) as f32 * r.width(), r.bottom() - 6. - (v / m) as f32 * (r.height() - 12.));
                    p.add(egui::Shape::line(k.iter().map(|x| to(x.0, x.2, vmax)).collect(), egui::Stroke::new(2., Color32::from_rgb(90, 180, 255))));
                    p.add(egui::Shape::line(k.iter().map(|x| to(x.0, x.3, amax)).collect(), egui::Stroke::new(1.5, Color32::from_rgb(255, 160, 80))));
                    let xt = r.left() + (st.sim_t / st.sim_t1.max(1e-9)) as f32 * r.width();
                    p.line_segment([egui::pos2(xt, r.top()), egui::pos2(xt, r.bottom())], egui::Stroke::new(1., Color32::GRAY));
                    let now = k.iter().min_by(|a, b| (a.0 - st.sim_t).abs().total_cmp(&(b.0 - st.sim_t).abs()));
                    if let Some(n) = now {
                        ui.label(RichText::new(format!("v = {} mm/s (azul, máx {}) · a = {} mm/s² (laranja, máx {})", fmt(n.2), fmt(vmax), fmt(n.3), fmt(amax))).small());
                    }
                }
            } else {
                ui.label(RichText::new(tr("Defina os comandos e calcule os quadros; depois toque ou arraste o tempo.")).weak().small());
            }
        });
    if compute {
        if st.sim_rest.is_none() {
            st.sim_rest = Some(st.asm.nodes.iter().map(|n| (n.id, n.placement)).collect());
        }
        let (t1, n) = (st.sim_t1, st.sim_steps);
        st.sim_frames = kin::simulate(&mut st.asm, 0., t1, n);
        st.sim_t = 0.;
        st.sim_play = true;
    }
    // reprodução: põe as posições do quadro do tempo atual
    if !st.sim_frames.is_empty() {
        if st.sim_play {
            st.sim_t += ctx.input(|i| i.stable_dt) as f64;
            if st.sim_t > st.sim_t1 {
                st.sim_t = 0.;
            }
            ctx.request_repaint();
        }
        let f = st.sim_frames.iter().min_by(|a, b| (a.t - st.sim_t).abs().total_cmp(&(b.t - st.sim_t).abs())).cloned();
        if let Some(f) = f {
            let mut changed = false;
            for n in &mut st.asm.nodes {
                if let Some(p) = f.placements.get(&n.id) {
                    if n.placement != *p {
                        n.placement = *p;
                        changed = true;
                    }
                }
            }
            if changed {
                st.dirty = true;
            }
        }
    }
    if restore || !open {
        if let Some(rest) = st.sim_rest.take() {
            for (id, p) in rest {
                if let Some(n) = st.asm.node_mut(id) {
                    n.placement = p;
                }
            }
            st.dirty = true;
        }
        st.sim_frames.clear();
        st.sim_play = false;
    }
    if !open {
        st.sim_open = false;
    }
}

// ============================================================== composer

/// Centro de cada componente sem explosão (para a explosão automática).
fn centers(st: &AsmState) -> std::collections::HashMap<Id, [f64; 3]> {
    let mut out = std::collections::HashMap::new();
    if let Some(r) = &st.view {
        for b in &r.bodies {
            let k = (b.mesh.positions.len() / 3).max(1) as f64;
            let c = b.mesh.positions.chunks_exact(3).fold([0f64; 3], |a, q| [a[0] + q[0] as f64, a[1] + q[1] as f64, a[2] + q[2] as f64]);
            let o = st.live.offset(b.collector);
            out.insert(b.collector, [c[0] / k - o[0], c[1] / k - o[1], c[2] / k - o[2]]);
        }
    }
    out
}

/// Lista de peças da montagem no nível escolhido, com os números de item fixos.
pub fn bom_window(app: &mut App, ctx: &egui::Context) {
    use fenkra_montagem::bom::{bom, BomLevel};
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut level) = st.bom_open else { return };
    let dir = st.dir();
    let rows = bom(&st.asm, dir.as_deref(), level);
    let mut open = true;
    let (mut fix, mut export) = (false, false);
    egui::Window::new(tr("▦ Lista de peças")).open(&mut open).default_width(460.).show(ctx, |ui| {
        if crate::app::done_bar(ui) {
            st.bom_open = None;
        }
        ui.horizontal_wrapped(|ui| {
            for l in BomLevel::ALL {
                ui.selectable_value(&mut level, l, tr(l.label()));
            }
        });
        egui::ScrollArea::vertical().max_height(360.).show(ui, |ui| {
            egui::Grid::new("asm_bom").striped(true).num_columns(3).show(ui, |ui| {
                for h in ["Item", "Componente", "Qtd."] {
                    ui.label(RichText::new(tr(h)).strong());
                }
                ui.end_row();
                for r in &rows {
                    let fixed = st.asm.item_of(&r.key).is_some() || r.depth > 0;
                    let item = RichText::new(&r.item);
                    ui.label(if fixed { item } else { item.italics().weak() }).on_hover_text(if fixed { tr("número fixo") } else { tr("número provisório: fica fixo ao salvar a montagem") });
                    ui.label(format!("{}{}{}", "    ".repeat(r.depth), if r.sub { "▣ " } else { "" }, r.name));
                    ui.label(r.qty.to_string());
                    ui.end_row();
                }
            });
        });
        ui.horizontal(|ui| {
            fix = ui.button(tr("📌 Fixar números")).on_hover_text(tr("Grava os números dos itens novos na montagem (também acontece ao salvar).")).clicked();
            export = ui.button(tr("💾 Exportar CSV…")).clicked();
        });
        ui.label(RichText::new(tr("Um item mantém o número entre revisões, mesmo que saia e volte; os novos recebem o próximo número livre.")).weak().small());
    });
    if let Some(l) = st.bom_open.as_mut() {
        *l = level;
    }
    if !open {
        st.bom_open = None;
    }
    if fix {
        st.asm.assign_items(dir.as_deref());
        st.modified = true;
    }
    if export {
        if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name("lista_de_pecas.csv").save_file() {
            let mut t = String::from("item;componente;quantidade\n");
            for r in &rows {
                t += &format!("{};{};{}\n", r.item, r.name, r.qty);
            }
            if let Err(e) = std::fs::write(&p, t) {
                app.toast(format!("Erro: {e}"), true);
            }
        }
    }
}

/// Resumo do preenchimento para o aviso na tela: "12 fixadores: 8× M6x35, 4× M8x40".
fn fasteners_summary(r: &RelatorioFixadores) -> String {
    let lista = r.por_tamanho.iter().map(|(t, n)| format!("{n}× {t}")).collect::<Vec<_>>().join(", ");
    let mut s = format!("{} {}: {lista}", r.fixadores, tr(if r.fixadores == 1 { "fixador" } else { "fixadores" }));
    if r.pulados > 0 {
        s += &format!(" · {} {}", r.pulados, tr("furo(s) já preenchido(s)"));
    }
    if !r.avisos.is_empty() {
        s += &format!(" · {} {}", r.avisos.len(), tr("aviso(s) na janela"));
    }
    s
}

/// Preenche os furos da montagem com parafuso, arruela e porca da biblioteca (B29).
pub fn fasteners_window(app: &mut App, ctx: &egui::Context) {
    use fenkra_solidos::biblioteca::{self, TipoFixador};
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut f) = st.fasteners_open.clone() else { return };
    let mut open = true;
    let (mut apply, mut close) = (false, false);
    let familias = |pred: fn(TipoFixador) -> bool| biblioteca::familias().iter().filter(|x| pred(x.tipo)).map(|x| (x.id.clone(), x.norma.clone())).collect::<Vec<_>>();
    let combo = |ui: &mut egui::Ui, id: &str, atual: &mut String, itens: &[(String, String)]| {
        let rotulo = itens.iter().find(|(i, _)| i == atual).map(|(_, n)| n.clone()).unwrap_or_else(|| atual.clone());
        egui::ComboBox::from_id_salt(id).selected_text(rotulo).show_ui(ui, |ui| {
            for (i, n) in itens {
                ui.selectable_value(atual, i.clone(), n);
            }
        });
    };
    egui::Window::new(tr("🔩 Preencher furos com fixadores")).id(egui::Id::unique("janela_fixadores")).open(&mut open).default_width(400.).show(ctx, |ui| {
        if crate::app::done_bar(ui) {
            close = true;
        }
        ui.label(RichText::new(tr("Os furos passantes com a folga de um tamanho métrico (e os roscados) recebem parafuso, arruela e porca da biblioteca. Furos de peças diferentes no mesmo eixo formam uma pilha só, e o comprimento sai da espessura atravessada.")).weak().small());
        crate::editor::section(ui, "Parafuso");
        ui.horizontal(|ui| {
            ui.label(tr("Família"));
            combo(ui, "fix_parafuso", &mut f.op.familia_parafuso, &familias(TipoFixador::e_parafuso));
        });
        ui.horizontal(|ui| {
            ui.label(tr("Folga do furo"));
            for (i, n) in ["Fina", "Normal", "Folgada"].into_iter().enumerate() {
                ui.selectable_value(&mut f.op.folga, i as u8, tr(n));
            }
        });
        ui.label(RichText::new(tr("M6: fina 6,4 · normal 6,6 · folgada 7,0 mm. Furos roscados não dependem da folga.")).weak().small());
        crate::editor::section(ui, "Arruela e porca");
        crate::editor::check(ui, &mut f.op.arruela, "arruela sob a cabeça (e sob a porca)");
        if f.op.arruela {
            ui.horizontal(|ui| {
                ui.label(tr("Família da arruela"));
                combo(ui, "fix_arruela", &mut f.op.familia_arruela, &familias(|t| t == TipoFixador::Arruela));
            });
        }
        crate::editor::check(ui, &mut f.op.porca, "porca na saída (furos roscados não levam porca)");
        if f.op.porca {
            ui.horizontal(|ui| {
                ui.label(tr("Família da porca"));
                combo(ui, "fix_porca", &mut f.op.familia_porca, &familias(|t| t == TipoFixador::Porca));
            });
        }
        crate::editor::section(ui, "Onde");
        crate::editor::check(ui, &mut f.only_selected, "só nos componentes selecionados (árvore ou viewport)");
        ui.add_space(4.);
        apply = ui.add(egui::Button::new(RichText::new(tr("Preencher furos")).strong()).min_size(egui::vec2(140., 28.))).on_hover_text(tr("Insere os fixadores já na posição, com restrições de eixo e de contato, e resolve a montagem. Rodar de novo não duplica.")).clicked();
        if !f.warnings.is_empty() {
            crate::editor::section(ui, "Avisos");
            egui::ScrollArea::vertical().max_height(140.).show(ui, |ui| {
                for w in &f.warnings {
                    ui.label(RichText::new(format!("⚠ {w}")).small());
                }
            });
        }
    });
    if !open || close {
        st.fasteners_open = None;
        return;
    }
    // aviso na tela, dado só depois do último uso de `st` (o aviso empresta o app inteiro)
    let mut aviso: Option<(String, bool)> = None;
    if apply {
        let dir = st.dir();
        let mut op = f.op.clone();
        if dir.is_none() {
            aviso = Some((tr("Salve a montagem antes: as peças dos fixadores são gravadas na pasta dela").to_string(), true));
        } else if f.only_selected {
            let mut comps = st.comps_of(&app.picks);
            if let Some(s) = st.sel.filter(|s| st.asm.node(*s).is_some_and(|n| matches!(n.kind, NodeKind::Part { .. }))) {
                if !comps.contains(&s) {
                    comps.push(s);
                }
            }
            if comps.is_empty() {
                aviso = Some((tr("Escolha os componentes na árvore ou na viewport (ou desmarque “só nos componentes selecionados”)").to_string(), true));
            }
            op.componentes = comps;
        }
        if aviso.is_none() {
            let modificada = st.modified;
            st.push_undo();
            match preencher_furos(&mut st.asm, dir.as_deref(), &mut st.cache, &op) {
                Ok(rel) => {
                    if rel.fixadores > 0 {
                        st.solve();
                        aviso = Some((fasteners_summary(&rel), false));
                    } else {
                        // nada mudou: o passo de desfazer não vale
                        st.undo.pop();
                        st.modified = modificada;
                        let msg = if rel.pulados > 0 { tr("Os furos já têm fixador") } else { tr("Nenhum furo compatível com um tamanho métrico encontrado") };
                        aviso = Some((msg.to_string(), true));
                    }
                    f.warnings = rel.avisos;
                }
                Err(e) => {
                    if let Some(antes) = st.undo.pop() {
                        st.asm = antes;
                    }
                    st.modified = modificada;
                    st.dirty = true;
                    f.warnings = vec![e.clone()];
                    aviso = Some((e, true));
                }
            }
        }
    }
    st.fasteners_open = Some(f);
    if let Some((msg, erro)) = aviso {
        app.toast(msg, erro);
    }
}

/// Grava o `.fkp` de uma engrenagem escolhendo o destino (pasta da montagem como sugestão).
fn save_gear(app: &mut App, p: &fenkra_solidos::ParamEngrenagem, dir: Option<PathBuf>) {
    let doc = match fenkra_solidos::engrenagem_documento(p) {
        Ok(d) => d,
        Err(e) => return app.toast(e, true),
    };
    let mut dlg = rfd::FileDialog::new().add_filter("Peça", &["fkp"]).set_file_name(format!("{}.fkp", p.chave()));
    if let Some(d) = dir {
        dlg = dlg.set_directory(d);
    }
    let Some(path) = dlg.save_file() else { return };
    match std::fs::write(&path, doc.to_json()) {
        Ok(()) => app.toast(format!("{}: {}", tr("Peça salva"), path.display()), false),
        Err(e) => app.toast(format!("Erro: {e}"), true),
    }
}

/// Gerador de engrenagens (B28): cálculo ao vivo, par engrenado na montagem e peça avulsa.
pub fn gears_window(app: &mut App, ctx: &egui::Context) {
    use fenkra_solidos::TipoEngrenagem;
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut g) = st.gears_open.clone() else { return };
    let mut open = true;
    let (mut inserir, mut close) = (false, false);
    let mut salvar: Option<bool> = None;
    let tipo = g.p.tipo;
    let par = calculo_par(&g.p, g.dentes_coroa);
    let campo = |ui: &mut egui::Ui, rotulo: &str, v: &mut f64, r: std::ops::RangeInclusive<f64>, suf: &str, vel: f64| {
        ui.horizontal(|ui| {
            ui.label(tr(rotulo));
            let resp = ui.add(crate::ui_ext::dv(v).range(r).speed(vel).max_decimals(3).suffix(suf));
            crate::ui_ext::wheel(ui, &resp, v);
        });
    };
    egui::Window::new(tr("⚙ Engrenagens")).id(egui::Id::unique("janela_engrenagens")).open(&mut open).default_width(430.).show(ctx, |ui| {
        if crate::app::done_bar(ui) {
            close = true;
        }
        ui.label(RichText::new(tr("Gera engrenagens com perfil em evolvente e insere o par já engrenado: distância entre centros, dentes encaixados e restrição de engrenagem. O pinhão é a peça de z1 dentes.")).weak().small());
        crate::editor::section(ui, "Tipo");
        ui.horizontal_wrapped(|ui| {
            for t in TipoEngrenagem::ALL {
                ui.selectable_value(&mut g.p.tipo, t, tr(t.rotulo()));
            }
        });
        crate::editor::section(ui, "Medidas");
        campo(ui, "Módulo", &mut g.p.modulo, 0.1..=50., " mm", 0.05);
        if tipo != TipoEngrenagem::Cremalheira {
            ui.horizontal(|ui| {
                ui.label(tr("Dentes do pinhão (z1)"));
                let resp = ui.add(crate::ui_ext::dv(&mut g.p.dentes).range(6..=400));
                crate::ui_ext::wheel(ui, &resp, &mut g.p.dentes);
            });
            ui.horizontal(|ui| {
                ui.label(tr(if tipo == TipoEngrenagem::Interna { "Dentes da coroa interna (z2)" } else { "Dentes da coroa (z2)" }));
                let resp = ui.add(crate::ui_ext::dv(&mut g.dentes_coroa).range(6..=400));
                crate::ui_ext::wheel(ui, &resp, &mut g.dentes_coroa);
            });
        } else {
            campo(ui, "Comprimento da cremalheira", &mut g.p.comprimento_cremalheira, 1.0..=10000., " mm", 1.);
            ui.horizontal(|ui| {
                ui.label(tr("Dentes do pinhão (z1)"));
                let resp = ui.add(crate::ui_ext::dv(&mut g.p.dentes).range(6..=400));
                crate::ui_ext::wheel(ui, &resp, &mut g.p.dentes);
            });
        }
        campo(ui, "Ângulo de pressão", &mut g.p.angulo_pressao, 14.5..=30., "°", 0.1);
        if tipo == TipoEngrenagem::Helicoidal {
            campo(ui, "Ângulo de hélice (sinal = sentido)", &mut g.p.angulo_helice, -45.0..=45., "°", 0.1);
        }
        campo(ui, "Largura", &mut g.p.largura, 0.1..=1000., " mm", 0.5);
        campo(ui, "Folga entre dentes (par)", &mut g.p.folga_dentes, 0.0..=10., " mm", 0.01);
        if matches!(tipo, TipoEngrenagem::Reta | TipoEngrenagem::Helicoidal) {
            campo(ui, "Furo central", &mut g.p.furo, 0.0..=1000., " mm", 0.5);
            campo(ui, "Correção do perfil (x)", &mut g.p.coef_correcao, -1.0..=1., "", 0.01);
        }

        crate::editor::section(ui, "Cálculo");
        match &par {
            Err(e) => {
                ui.label(RichText::new(format!("⚠ {e}")).color(Color32::from_rgb(220, 90, 60)));
            }
            Ok(c) => {
                let v = |x: f64| if x == 0. { "—".to_string() } else { fmt(x) };
                let rack = tipo == TipoEngrenagem::Cremalheira;
                egui::Grid::new("grade_engrenagens").num_columns(3).spacing([14., 3.]).show(ui, |ui| {
                    ui.label("");
                    ui.label(RichText::new(tr("Pinhão")).strong());
                    ui.label(RichText::new(tr(if rack { "Cremalheira" } else { "Coroa" })).strong());
                    ui.end_row();
                    for (rotulo, a, b) in [
                        ("Diâmetro primitivo d", c.calc_pinhao.diametro_primitivo, c.calc_coroa.diametro_primitivo),
                        ("Diâmetro base db", c.calc_pinhao.diametro_base, c.calc_coroa.diametro_base),
                        ("Diâmetro de topo da", c.calc_pinhao.diametro_externo, c.calc_coroa.diametro_externo),
                        ("Diâmetro de raiz df", c.calc_pinhao.diametro_raiz, c.calc_coroa.diametro_raiz),
                        ("Passo circular", c.calc_pinhao.passo, c.calc_coroa.passo),
                        ("Espessura do dente", c.calc_pinhao.espessura_dente, c.calc_coroa.espessura_dente),
                        ("Altura do dente", c.calc_pinhao.altura_dente, c.calc_coroa.altura_dente),
                    ] {
                        ui.label(tr(rotulo));
                        ui.label(v(a));
                        ui.label(v(b));
                        ui.end_row();
                    }
                    if rack {
                        ui.label(tr("Dentes da cremalheira"));
                        ui.label("");
                        ui.label(c.calc_coroa.dentes_cremalheira.to_string());
                        ui.end_row();
                    }
                });
                ui.add_space(4.);
                if rack {
                    ui.label(format!("{}: {} mm", tr("Centro do pinhão sobre a linha primitiva"), fmt(c.distancia_centros)));
                    ui.label(format!("{}: {} mm", tr("Avanço por volta do pinhão"), fmt(c.mm_por_volta)));
                } else {
                    ui.label(format!("{}: {} mm", tr("Distância entre centros a"), fmt(c.distancia_centros)));
                    ui.label(format!("{}: {} ({})", tr("Razão (giro da coroa por giro do pinhão)"), fmt(c.razao), crate::i18n::tr(if c.razao < 0. { "sentido contrário" } else { "mesmo sentido" })));
                }
                for a in &c.avisos {
                    ui.label(RichText::new(format!("⚠ {a}")).small());
                }
            }
        }
        ui.add_space(4.);
        ui.horizontal_wrapped(|ui| {
            let ok = par.is_ok();
            let b = ui.add_enabled(ok, egui::Button::new(RichText::new(tr("Inserir par na montagem")).strong()).min_size(egui::vec2(160., 28.)));
            inserir = b.on_hover_text(tr("Grava as duas peças em “engrenagens/” ao lado da montagem, insere engrenadas (centros, dentes encaixados) e cria a restrição de movimento. Rodar de novo insere outro par.")).clicked();
            let rotulo_b = if tipo == TipoEngrenagem::Cremalheira { "Salvar cremalheira..." } else { "Salvar coroa..." };
            if ui.add_enabled(ok, egui::Button::new(tr("Salvar pinhão..."))).on_hover_text(tr("Salva só a peça do pinhão (.fkp), para usar em qualquer montagem.")).clicked() {
                salvar = Some(false);
            }
            if ui.add_enabled(ok, egui::Button::new(tr(rotulo_b))).on_hover_text(tr("Salva só a outra peça do par (.fkp). A folga entre dentes é dividida entre as duas peças.")).clicked() {
                salvar = Some(true);
            }
        });
        if !g.msg.is_empty() {
            for m in &g.msg {
                ui.label(RichText::new(format!("⚠ {m}")).small());
            }
        }
    });
    if !open || close {
        st.gears_open = None;
        return;
    }
    let mut aviso: Option<(String, bool)> = None;
    let mut gravar: Option<(fenkra_solidos::ParamEngrenagem, Option<PathBuf>)> = None;
    if let (Some(coroa), Ok(c)) = (salvar, &par) {
        gravar = Some((if coroa { c.coroa.clone() } else { c.pinhao.clone() }, st.dir()));
    }
    if inserir {
        let dir = st.dir();
        if dir.is_none() {
            aviso = Some((tr("Salve a montagem antes: as peças das engrenagens são gravadas na pasta dela").to_string(), true));
        } else {
            let modificada = st.modified;
            st.push_undo();
            match inserir_par_engrenagens(&mut st.asm, dir.as_deref(), &g.p, g.dentes_coroa) {
                Ok(rel) => {
                    st.solve();
                    aviso = Some((format!("{} · {} {} mm", tr("Par de engrenagens inserido"), tr("centros a"), fmt(rel.distancia_centros)), false));
                    g.msg = rel.avisos;
                }
                Err(e) => {
                    if let Some(antes) = st.undo.pop() {
                        st.asm = antes;
                    }
                    st.modified = modificada;
                    st.dirty = true;
                    g.msg = vec![e.clone()];
                    aviso = Some((e, true));
                }
            }
        }
    }
    st.gears_open = Some(g);
    if let Some((msg, erro)) = aviso {
        app.toast(msg, erro);
    }
    if let Some((p, dir)) = gravar {
        save_gear(app, &p, dir);
    }
}

/// Janela do composer: estados de exibição, explosão e animação por quadros-chave.
pub fn composer_window(app: &mut App, ctx: &egui::Context) {
    let cam_now = app.camera_get();
    let Some(st) = app.asm.as_mut() else { return };
    if !st.composer_open {
        return;
    }
    use fenkra_montagem::display::{auto_explode, Animation, Key};
    let mut open = true;
    let mut set_cam: Option<fenkra_montagem::display::Cam> = None;
    let before = st.live.clone();
    let fechar = egui::Id::unique(("concluir_janela", "asm_ui.rs6"));
    if ctx.data_mut(|m| m.remove_temp::<bool>(fechar)).unwrap_or(false) {
        open = false;
    }
    egui::Window::new(tr("🎬 Exibição e animação"))
        .id(egui::Id::unique("janela_composer"))
        .open(&mut open)
        .default_width(470.)
        .resizable(true)
        .show(ctx, |ui| {
            if crate::app::done_bar(ui) {
                ui.ctx().data_mut(|m| m.insert_temp(fechar, true));
            }
            crate::editor::section(ui, "Estados de exibição");
            ui.label(RichText::new(tr("Guardam o que aparece, cores, explosão e câmera. O detalhamento usa estes estados como coletores de exibição.")).weak().small());
            let mut apply = None;
            let mut del = None;
            for (i, s) in st.asm.display_states.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut s.name).desired_width(150.));
                    if ui.small_button(tr("▶ aplicar")).clicked() {
                        apply = Some(i);
                    }
                    if ui.small_button("✖").clicked() {
                        del = Some(i);
                    }
                });
            }
            if let Some(i) = apply {
                st.live = st.asm.display_states[i].clone();
                // o estado salvo guarda quem fica oculto: aplica na árvore também
                for n in &mut st.asm.nodes {
                    n.visible = !st.live.hidden.contains(&n.id);
                }
                set_cam = st.live.camera;
                st.dirty = true;
            }
            if let Some(i) = del {
                st.asm.display_states.remove(i);
            }
            if ui.button(tr("💾 Salvar estado atual")).clicked() {
                let mut s = st.asm.snapshot_state(&format!("Estado {}", st.asm.display_states.len() + 1));
                s.offsets = st.live.offsets.clone();
                s.camera = cam_now;
                st.asm.display_states.push(s);
                st.modified = true;
            }
            crate::editor::section(ui, "Vista explodida");
            if ui.add(crate::ui_ext::slider(&mut st.explode_factor, 0.0..=3.0).text(tr("afastamento"))).changed() {
                let c = centers(st);
                st.live.offsets = auto_explode(&st.asm, &c, st.explode_factor);
                st.dirty = true;
            }
            crate::editor::check(ui, &mut st.explode_mode, "ajustar à mão (o manipulador mexe só na explosão)");
            if ui.button(tr("Juntar tudo (sem explosão)")).clicked() {
                st.live.offsets.clear();
                st.explode_factor = 0.;
                st.dirty = true;
            }
            crate::editor::section(ui, "Animação");
            if st.asm.animations.is_empty() {
                if ui.button(tr("+ Nova animação")).clicked() {
                    st.asm.animations.push(Animation { name: "Animação 1".into(), keys: vec![] });
                }
                return;
            }
            st.anim_sel = st.anim_sel.min(st.asm.animations.len() - 1);
            ui.horizontal_wrapped(|ui| {
                let names: Vec<String> = st.asm.animations.iter().map(|a| a.name.clone()).collect();
                egui::ComboBox::from_id_salt("anim_sel").selected_text(&names[st.anim_sel]).show_ui(ui, |ui| {
                    for (i, n) in names.iter().enumerate() {
                        ui.selectable_value(&mut st.anim_sel, i, n);
                    }
                });
                if ui.small_button(tr("+ nova")).clicked() {
                    let n = st.asm.animations.len() + 1;
                    st.asm.animations.push(Animation { name: format!("Animação {n}"), keys: vec![] });
                    st.anim_sel = n - 1;
                }
            });
            let a = &mut st.asm.animations[st.anim_sel];
            ui.horizontal(|ui| {
                ui.label(tr("Nome"));
                ui.text_edit_singleline(&mut a.name);
            });
            let dur = a.duration().max(0.1);
            ui.horizontal(|ui| {
                if ui.button(if st.anim_play { "⏸" } else { "▶" }).clicked() {
                    st.anim_play = !st.anim_play;
                }
                if ui.button("⏮").clicked() {
                    st.anim_t = 0.;
                }
                ui.add(crate::ui_ext::slider(&mut st.anim_t, 0.0..=dur).text("t (s)"));
            });
            // linha do tempo com os quadros-chave
            let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.), egui::Sense::hover());
            let p = ui.painter_at(r);
            p.rect_filled(r, 4., ui.visuals().extreme_bg_color);
            for k in &a.keys {
                let x = r.left() + (k.t / dur) as f32 * r.width();
                p.add(egui::Shape::convex_polygon(vec![egui::pos2(x, r.top() + 4.), egui::pos2(x + 6., r.center().y), egui::pos2(x, r.bottom() - 4.), egui::pos2(x - 6., r.center().y)], Color32::from_rgb(255, 190, 40), egui::Stroke::NONE));
            }
            let xt = r.left() + (st.anim_t / dur) as f32 * r.width();
            p.line_segment([egui::pos2(xt, r.top()), egui::pos2(xt, r.bottom())], egui::Stroke::new(2., Color32::from_rgb(90, 180, 255)));
            let mut kdel = None;
            for (i, k) in a.keys.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("◆ {}", k.state.name)).small());
                    { let __r = ui.add(crate::ui_ext::dv(&mut k.t).range(0.0..=600.0).speed(0.05).suffix(" s")); crate::ui_ext::wheel(ui, &__r, &mut k.t); __r };
                    if ui.small_button("✖").clicked() {
                        kdel = Some(i);
                    }
                });
            }
            if let Some(i) = kdel {
                a.keys.remove(i);
            }
            if ui.button(tr("◆ Quadro-chave aqui (estado atual + câmera)")).clicked() {
                let mut s = st.asm.snapshot_state(&format!("quadro {}", st.asm.animations[st.anim_sel].keys.len() + 1));
                s.offsets = st.live.offsets.clone();
                s.camera = cam_now;
                let t = if st.asm.animations[st.anim_sel].keys.is_empty() { 0. } else { st.anim_t.max(st.asm.animations[st.anim_sel].duration() + 1.) };
                st.asm.animations[st.anim_sel].keys.push(Key { t, state: s });
                st.modified = true;
            }
            ui.label(RichText::new(tr("Monte a cena (oculte peças, exploda, gire a câmera) e grave um quadro; repita em outro tempo. A animação interpola a explosão e a câmera.")).weak().small());
        });
    // reprodução
    if st.anim_play {
        if let Some(a) = st.asm.animations.get(st.anim_sel) {
            st.anim_t += ctx.input(|i| i.stable_dt) as f64;
            let d = a.duration();
            if st.anim_t > d {
                st.anim_t = 0.;
            }
            if let Some(s) = a.at(st.anim_t) {
                for n in &mut st.asm.nodes {
                    n.visible = !s.hidden.contains(&n.id);
                }
                set_cam = s.camera;
                st.live = s;
                st.dirty = true;
            }
            ctx.request_repaint();
        }
    }
    if st.live != before {
        st.dirty = true;
    }
    if !open {
        st.composer_open = false;
        st.anim_play = false;
    }
    if let Some(c) = set_cam {
        app.camera_set(c);
    }
}
