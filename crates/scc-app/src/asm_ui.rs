//! Bancada de Montagem: árvore de componentes, painéis, restrições escolhidas na viewport,
//! movimentação com manipulador e verificação de interferências.
//!
//! A viewport e a seleção da peça são reaproveitadas: a montagem vira um documento "sombra" com
//! um coletor por componente e um resultado com as malhas já posicionadas (`scc_montagem::view`).

use crate::app::{fmt, App, Cmd};
use crate::i18n::tr;
use crate::picking::Pick;
use eframe::egui::{self, Color32, RichText};
use scc_core::{Document, Id};
use scc_montagem::load::{full_path, geom_at, PartCache};
use scc_montagem::solve::SolveReport;
use scc_montagem::*;
use scc_solidos::RecomputeResult;
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
}

impl MateCmd {
    fn kind(self) -> Option<MateKind> {
        Some(match self {
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
    /// posição antes do arrasto do manipulador
    drag_start: Option<Placement>,
    snapshot: Option<Assembly>,
}

impl AsmState {
    pub fn new(asm: Assembly, file: Option<PathBuf>) -> Self {
        let report_dof = scc_montagem::solve::dof(&asm);
        AsmState {
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
            drag_start: None,
            snapshot: None,
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
        let mut shown = self.asm.clone();
        if self.isolate {
            if let Some(s) = self.sel {
                for n in &mut shown.nodes {
                    if n.id != s && !matches!(n.kind, NodeKind::Group) {
                        n.visible = false;
                    }
                }
            }
        }
        let (d, r) = scc_montagem::view::compose(&shown, dir.as_deref(), &mut self.cache, &mut self.errors);
        self.view_doc = d;
        self.view = Some(r);
        self.dirty = false;
        true
    }

    /// Resolve as restrições e marca a vista para recompor.
    pub fn solve(&mut self) {
        let dir = self.dir();
        scc_montagem::load::refresh_refs(&mut self.asm, &mut self.cache, dir.as_deref());
        self.report = scc_montagem::solve::solve(&mut self.asm);
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
                let c = scc_solidos::geo3d::Curve3::poly(pts.clone(), false)?;
                if c.is_straight() {
                    Geom::Axis { p: pts[0], d: unit(sub(pts[pts.len() - 1], pts[0])) }
                } else {
                    // aresta circular: eixo pelo centro, normal ao círculo
                    let (ctr, n, _) = scc_solidos::geo3d::fit_circle(&c)?;
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
            let Some(p) = rfd::FileDialog::new().add_filter("Montagem SolidCadCraft", &["sccasm"]).pick_file() else { return true };
            match std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|s| Assembly::from_json(&s)) {
                Ok(a) => {
                    app.asm = Some(AsmState::new(a, Some(p)));
                    app.fit_now();
                }
                Err(e) => app.toast(format!("Não foi possível abrir: {e}"), true),
            }
        }
        Cmd::AsmSave => save(app),
        _ => {
            let Some(st) = app.asm.as_mut() else {
                app.toast(tr("Crie ou abra uma montagem primeiro"), true);
                return matches!(
                    c,
                    Cmd::AsmInsert | Cmd::AsmNewPart | Cmd::AsmGroup | Cmd::AsmReplace | Cmd::AsmMate(_) | Cmd::AsmFix | Cmd::AsmRigid | Cmd::AsmSolve | Cmd::AsmNumber | Cmd::AsmInterf | Cmd::AsmSubAsm | Cmd::AsmPattern
                );
            };
            match c {
                Cmd::AsmInsert | Cmd::AsmSubAsm => {
                    let sub = c == Cmd::AsmSubAsm;
                    let (filter, ext) = if sub { ("Montagem", "sccasm") } else { ("Peça", "sccpart") };
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
                        let id = st.asm.add_component(&name, kind, Placement { origin: at, ..Default::default() }, parent);
                        st.sel = Some(id);
                        st.dirty = true;
                        st.refresh();
                        x = st.view.as_ref().map(|r| r.bodies.iter().flat_map(|b| b.mesh.positions.chunks_exact(3).map(|p| p[0] as f64)).fold(0f64, f64::max)).unwrap_or(x);
                    }
                    app.fit_now();
                }
                Cmd::AsmNewPart => {
                    // peça nova no contexto: cria o arquivo ao lado da montagem e abre para modelar
                    let mut dlg = rfd::FileDialog::new().add_filter("Peça", &["sccpart"]).set_file_name("Peça nova.sccpart");
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
                    let Some(f) = rfd::FileDialog::new().add_filter("Peça ou montagem", &["sccpart", "sccasm"]).pick_file() else { return true };
                    st.push_undo();
                    let path = st.rel_path(&f);
                    let kind = if f.extension().is_some_and(|e| e == "sccasm") { NodeKind::SubAssembly { path } } else { NodeKind::Part { path } };
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
                Cmd::AsmInterf => {
                    let dir = st.dir();
                    st.interf = scc_montagem::view::interferences(&st.asm, dir.as_deref(), &mut st.cache, 0.01);
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

/// Seleções feitas na viewport com uma restrição pendente: cria quando houver duas.
pub fn after_viewport(app: &mut App) {
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
        None => rfd::FileDialog::new().add_filter("Montagem SolidCadCraft", &["sccasm"]).set_file_name(format!("{}.sccasm", st.asm.name)).save_file(),
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
    match std::fs::write(&path, st.asm.to_json()) {
        Ok(()) => {
            st.file = Some(path);
            st.modified = false;
            app.toast(tr("Montagem salva"), false);
        }
        Err(e) => app.toast(format!("Erro ao salvar: {e}"), true),
    }
}

// ============================================================== árvore

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
    let title = format!("⚙ {}{}", st.asm.name, if st.modified { " •" } else { "" });
    ui.label(RichText::new(title).strong());
    if let Some(f) = &st.file {
        ui.label(RichText::new(f.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default()).weak().small());
    }
    ui.separator();
    // nós, com grupos aninhados (a ordem da árvore só muda a visualização)
    fn rows(ui: &mut egui::Ui, st: &AsmState, parent: Option<Id>, depth: usize, new_sel: &mut Option<Id>, act: &mut Option<(Id, u8)>, hover: &mut Option<Id>) {
        let kids: Vec<&Node> = st.asm.nodes.iter().filter(|n| n.parent == parent || (depth == 0 && n.parent.is_some_and(|p| st.asm.node(p).is_none()))).collect();
        for n in kids {
            let err = st.errors.iter().find(|(i, _)| *i == n.id).map(|(_, e)| e.clone());
            let icon = match n.kind {
                NodeKind::Group => "📁",
                NodeKind::Part { .. } => "🧩",
                NodeKind::SubAssembly { .. } => "🗂",
            };
            let num = if n.number > 0 { format!("{} · ", n.number) } else { String::new() };
            let mode = if n.mode == LoadMode::Visualizacao { " 👓" } else { "" };
            let txt = format!("{icon} {num}{}{}{mode}{}", n.name, if n.fixed { " 📌" } else { "" }, if err.is_some() { " ✖" } else { "" });
            let r = ui
                .horizontal(|ui| {
                    ui.add_space(depth as f32 * 14.);
                    if crate::app::eye_button(ui, !n.visible) {
                        *act = Some((n.id, 1));
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
                        let t = if err.is_some() { RichText::new(txt).color(Color32::from_rgb(255, 120, 100)) } else if n.visible { RichText::new(txt) } else { RichText::new(txt).weak() };
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
            if matches!(n.kind, NodeKind::Group) {
                rows(ui, st, Some(n.id), depth + 1, new_sel, act, hover);
            }
        }
    }
    rows(ui, st, None, 0, &mut new_sel, &mut act, &mut hover);
    if st.asm.nodes.is_empty() {
        ui.label(RichText::new(tr("vazia — 🧩 Inserir peça ou ✚ Peça nova no contexto")).weak().italics().small());
    }
    // restrições
    ui.add_space(6.);
    egui::CollapsingHeader::new(format!("🔗 {} ({})", tr("Restrições"), st.asm.mates.len() + st.asm.rigid.len())).default_open(true).show(ui, |ui| {
        for m in &st.asm.mates {
            let failed = st.report.failed.contains(&m.id);
            let txt = format!("{} {}  ({} ↔ {}){}", m.kind.icon(), m.name, st.asm.name_of(m.a.comp), st.asm.name_of(m.b.comp), if failed { " ✖" } else { "" });
            let mut t = RichText::new(txt).small();
            if failed {
                t = t.color(Color32::from_rgb(255, 120, 100));
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
        st.sel = Some(s);
        if st.isolate {
            st.dirty = true;
        }
    }
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
            st.solve();
        }
        _ => {}
    }
    st.dirty = true;
}

// ============================================================== painel

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
    let before = st.asm.clone();
    let mut resolve = false;
    match sel.and_then(|s| st.asm.nodes.iter().position(|n| n.id == s)) {
        Some(i) => {
            let n = &mut st.asm.nodes[i];
            let kind_txt = match &n.kind {
                NodeKind::Group => tr("Grupo (organização)").to_string(),
                NodeKind::Part { path } => format!("🧩 {}", path),
                NodeKind::SubAssembly { path } => format!("🗂 {}", path),
            };
            ui.heading(&n.name);
            ui.label(RichText::new(kind_txt).weak().small());
            ui.horizontal(|ui| {
                ui.label(tr("Nome"));
                ui.text_edit_singleline(&mut n.name);
            });
            if !matches!(n.kind, NodeKind::Group) {
                ui.horizontal(|ui| {
                    ui.label(tr("Número"));
                    ui.add(egui::DragValue::new(&mut n.number).range(0..=9999));
                });
                ui.checkbox(&mut n.fixed, tr("📌 fixo (referência, sem graus de liberdade)"));
                ui.horizontal(|ui| {
                    ui.radio_value(&mut n.mode, LoadMode::Editavel, tr("editável"));
                    ui.radio_value(&mut n.mode, LoadMode::Visualizacao, tr("só visualização"));
                });
                let mut own = n.color.is_some();
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut own, tr("cor própria")).changed() {
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
                        ui.add(egui::DragValue::new(&mut n.placement.origin[k]).speed(0.5).max_decimals(3));
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
                ui.checkbox(&mut st.respect, tr("mover respeitando as restrições"));
                ui.checkbox(&mut st.stop_collision, tr("parar ao encostar (colisão)"));
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
            if let Some(i) = pi {
                let src = st.asm.name_of(st.asm.patterns[i].source);
                let p = &mut st.asm.patterns[i];
                ui.heading(format!("⁂ {}", p.name));
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
                            ui.add(egui::DragValue::new(x).speed(0.1).max_decimals(3));
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
                    ui.heading(format!("{} {}", m.kind.icon(), m.name));
                    ui.label(RichText::new(format!("{na}  ↔  {nb}")).weak());
                    if failed {
                        ui.label(RichText::new(tr("✖ não satisfeita: conflita com outras restrições ou é impossível")).color(Color32::from_rgb(255, 130, 110)));
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
                        ] {
                            if ui.selectable_label(k.label() == cur, tr(k.label())).clicked() && k.label() != cur {
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
                        _ => {}
                    }
                    ui.checkbox(&mut m.suppressed, tr("suprimida"));
                }
                None => {
                    ui.heading(&st.asm.name);
                    ui.horizontal(|ui| {
                        ui.label(tr("Nome"));
                        ui.text_edit_singleline(&mut st.asm.name);
                    });
                    let parts = st.asm.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).count();
                    ui.label(format!("{parts} componente(s) · {} restrição(ões) · {} GDL livres", st.asm.mates.len(), st.report.dof));
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
    // interferências
    if !st.interf.is_empty() {
        crate::editor::section(ui, "Interferências");
        for (a, b, v) in &st.interf {
            ui.label(RichText::new(format!("✖ {} × {} — {} mm³", st.asm.name_of(*a), st.asm.name_of(*b), fmt(*v))).color(Color32::from_rgb(255, 130, 110)));
        }
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

/// Setas e quadrados no componente selecionado; ao soltar, resolve (se respeitar restrições) e
/// para no primeiro contato (se a parada por colisão estiver ligada).
pub fn gizmo(app: &mut App, ui: &mut egui::Ui, rect: egui::Rect, cam: &crate::render::Camera) {
    let Some(st) = app.asm.as_mut() else { return };
    let Some(id) = st.sel else { return };
    let Some(n) = st.asm.node(id) else { return };
    if matches!(n.kind, NodeKind::Group) || n.fixed {
        return;
    }
    let p = n.placement;
    // centro do componente na tela: centro da malha posicionada
    let c = st
        .view
        .as_ref()
        .and_then(|r| r.bodies.iter().find(|b| b.collector == id))
        .map(|b| {
            let k = (b.mesh.positions.len() / 3).max(1) as f32;
            b.mesh.positions.chunks_exact(3).fold(glam::Vec3::ZERO, |a, q| a + glam::Vec3::new(q[0], q[1], q[2])) / k
        })
        .unwrap_or(glam::Vec3::new(p.origin[0] as f32, p.origin[1] as f32, p.origin[2] as f32));
    let vals = p.origin;
    let res = crate::gizmo::translate_gizmo(ui, rect, cam, id.0, c, [glam::Vec3::X, glam::Vec3::Y, glam::Vec3::Z], vals, [false; 3]);
    let released = ui.input(|i| i.pointer.any_released());
    if let Some(v) = res {
        if st.drag_start.is_none() {
            st.drag_start = Some(p);
            st.snapshot = Some(st.asm.clone());
        }
        if let Some(n) = st.asm.node_mut(id) {
            n.placement.origin = v;
        }
        st.dirty = true;
    } else if released {
        if let (Some(start), Some(snap)) = (st.drag_start.take(), st.snapshot.take()) {
            st.undo.push(snap);
            st.modified = true;
            if st.stop_collision {
                // volta pelo caminho até o último ponto sem contato
                let end = st.asm.node(id).unwrap().placement;
                let dir = st.dir();
                let mut ok = start;
                for k in 1..=20 {
                    let t = k as f64 / 20.;
                    let q = Placement { origin: [0, 1, 2].map(|i| start.origin[i] + (end.origin[i] - start.origin[i]) * t), rot: end.rot };
                    st.asm.node_mut(id).unwrap().placement = q;
                    let hit = scc_montagem::view::interferences(&st.asm, dir.as_deref(), &mut st.cache, 0.01).iter().any(|(a, b, _)| *a == id || *b == id);
                    if hit {
                        break;
                    }
                    ok = q;
                }
                st.asm.node_mut(id).unwrap().placement = ok;
            }
            if st.respect && !st.asm.mates.is_empty() {
                st.solve();
            }
            st.dirty = true;
        }
    }
}
