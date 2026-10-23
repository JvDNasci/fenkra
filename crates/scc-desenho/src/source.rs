//! Origem das vistas: lê peças/montagens do disco (com cache por data de modificação), monta a
//! forma e calcula as projeções (HLR), cortes, detalhes e os dados das tabelas.

use crate::*;
use scc_core::Document;
use scc_kernel::{Bool, Shape};
use scc_montagem::load::{full_path, PartCache};
use scc_montagem::view::{placed_shape, shown};
use scc_montagem::{Assembly, NodeKind};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub enum Model {
    Part(Box<Document>),
    Asm(Box<Assembly>),
}

/// Peça ou montagem carregada.
pub struct Loaded {
    pub model: Model,
    /// forma no mundo (montagem: peças posicionadas, com o estado de exibição)
    pub shape: Option<Shape>,
    /// arquivos de que depende (para saber quando recarregar)
    files: Vec<(PathBuf, Option<SystemTime>)>,
    pub error: Option<String>,
}

/// Eixos de uma vista: direção para o observador e eixos x/y da folha (no mundo).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Axes {
    pub toward: V3,
    pub x: V3,
    pub y: V3,
}

impl Axes {
    pub fn new(toward: V3, x: V3) -> Self {
        Axes { toward, x, y: cross(toward, x) }
    }
    pub fn of(d: Dir) -> Self {
        let (t, x) = d.axes();
        Axes::new(t, x)
    }
    /// Ponto do modelo nas coordenadas da vista.
    pub fn project(&self, p: V3) -> V2 {
        [dot(p, self.x), dot(p, self.y)]
    }
    /// Ponto da vista (profundidade 0) de volta ao mundo.
    pub fn unproject(&self, q: V2) -> V3 {
        [0, 1, 2].map(|k| self.x[k] * q[0] + self.y[k] * q[1])
    }
}

/// Projeção pronta de uma vista, em mm do modelo no plano da vista.
#[derive(Clone, Debug, Default)]
pub struct Proj {
    /// polilinhas e tipo: 0 visível, 1 tangente, 2 contorno, 3 oculta, 4 borda do detalhe
    pub lines: Vec<(Vec<V2>, u8)>,
    /// contornos cortados (hachura)
    pub hatch: Vec<Vec<V2>>,
    /// ponto da vista que fica em `View::pos` (centro do modelo — mantém vistas alinhadas)
    pub center: V2,
    pub lo: V2,
    pub hi: V2,
    pub axes: Option<Axes>,
    pub error: Option<String>,
}

impl Proj {
    fn bounds(&mut self) {
        let mut lo = [f64::MAX; 2];
        let mut hi = [f64::MIN; 2];
        for (pl, _) in &self.lines {
            for p in pl {
                for k in 0..2 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
        }
        if lo[0] > hi[0] {
            lo = self.center;
            hi = self.center;
        }
        self.lo = lo;
        self.hi = hi;
    }
}

/// Linha genérica de tabela.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Cache de peças, montagens e projeções de um desenho.
#[derive(Default)]
pub struct Models {
    /// pasta do desenho (caminhos relativos)
    pub dir: Option<PathBuf>,
    parts: PartCache,
    loaded: HashMap<(PathBuf, String), Loaded>,
    proj: HashMap<Id, (u64, Proj)>,
}

fn stamp(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

impl Models {
    pub fn new(dir: Option<PathBuf>) -> Self {
        Models { dir, ..Default::default() }
    }

    pub fn full(&self, path: &str) -> PathBuf {
        full_path(self.dir.as_deref(), path)
    }

    /// Esquece o que mudou no disco (chamar de tempos em tempos). Devolve se algo mudou.
    pub fn refresh(&mut self) -> bool {
        let stale: Vec<(PathBuf, String)> =
            self.loaded.iter().filter(|(_, l)| l.files.iter().any(|(p, t)| stamp(p) != *t)).map(|(k, _)| k.clone()).collect();
        for k in &stale {
            if let Some(l) = self.loaded.remove(k) {
                for (p, _) in l.files {
                    self.parts.forget(&p);
                }
            }
        }
        !stale.is_empty()
    }

    /// Esquece tudo (recarregar à força).
    pub fn clear(&mut self) {
        self.loaded.clear();
        self.proj.clear();
        self.parts = PartCache::default();
    }

    pub fn load(&mut self, src: &Source) -> &Loaded {
        let full = self.full(&src.path);
        let key = (full.clone(), src.display_state.clone());
        if !self.loaded.contains_key(&key) {
            let l = self.read(&full, &src.display_state);
            self.loaded.insert(key.clone(), l);
        }
        &self.loaded[&key]
    }

    fn read(&mut self, full: &Path, state: &str) -> Loaded {
        let mut files = vec![(full.to_path_buf(), stamp(full))];
        let fail = |files, e: String| Loaded { model: Model::Part(Box::new(Document::new("?"))), shape: None, files, error: Some(e) };
        let txt = match std::fs::read_to_string(full) {
            Ok(t) => t,
            Err(e) => return fail(files, format!("{}: {e}", full.display())),
        };
        let is_asm = full.extension().is_some_and(|e| e.eq_ignore_ascii_case("sccasm"));
        if !is_asm {
            return match Document::from_json(&txt) {
                Ok(doc) => {
                    let shape = scc_solidos::final_shape(&doc);
                    let error = shape.is_none().then(|| "a peça não tem corpo visível".to_string());
                    Loaded { model: Model::Part(Box::new(doc)), shape, files, error }
                }
                Err(e) => fail(files, e.to_string()),
            };
        }
        let a = match Assembly::from_json(&txt) {
            Ok(a) => a,
            Err(e) => return fail(files, e),
        };
        let dir = full.parent().map(|p| p.to_path_buf());
        let shown_asm = match a.display_states.iter().find(|s| s.name == state) {
            Some(st) => a.with_state(st),
            None => a.clone(),
        };
        let mut all = Shape::empty();
        for n in &shown_asm.nodes {
            let NodeKind::Part { path } = &n.kind else { continue };
            let fp = full_path(dir.as_deref(), path);
            files.push((fp.clone(), stamp(&fp)));
            if !shown(&shown_asm, n.id) {
                continue;
            }
            if let Some(s) = placed_shape(&shown_asm, n.id, dir.as_deref(), &mut self.parts) {
                all = if all.is_null() { s } else { all.compound(&s) };
            }
        }
        let shape = (!all.is_null()).then_some(all);
        let error = shape.is_none().then(|| "a montagem não tem peças visíveis".to_string());
        Loaded { model: Model::Asm(Box::new(a)), shape, files, error }
    }

    /// Projeção da vista (calcula só quando a vista, a mãe ou o arquivo mudam).
    pub fn proj(&mut self, d: &Drawing, id: Id) -> Proj {
        let Some(v) = d.view(id) else { return Proj::default() };
        let parent = match &v.kind {
            ViewKind::Projected { parent, .. } | ViewKind::Section { parent, .. } | ViewKind::Detail { parent, .. } => Some(*parent),
            _ => None,
        };
        let parent_proj = parent.map(|p| self.proj(d, p));
        let mut h = DefaultHasher::new();
        serde_json::to_string(&(&v.source, &v.kind, v.hidden_lines, v.tangent_edges)).unwrap_or_default().hash(&mut h);
        if let Some(pp) = &parent_proj {
            format!("{:?}{:?}", pp.axes, pp.center).hash(&mut h);
            pp.lines.len().hash(&mut h);
        }
        for (p, t) in &self.load(&v.source).files {
            p.hash(&mut h);
            t.hash(&mut h);
        }
        let key = h.finish();
        if let Some((k, p)) = self.proj.get(&id) {
            if *k == key {
                return p.clone();
            }
        }
        let p = self.compute(v, parent_proj.as_ref());
        self.proj.insert(id, (key, p.clone()));
        p
    }

    fn compute(&mut self, v: &View, parent: Option<&Proj>) -> Proj {
        let tangent = v.tangent_edges;
        let hidden = v.hidden_lines;
        let l = self.load(&v.source);
        let mut out = Proj { error: l.error.clone(), ..Default::default() };
        // detalhe: recorte circular da vista mãe
        if let ViewKind::Detail { center, radius, .. } = &v.kind {
            let Some(pp) = parent else { return out };
            out.axes = pp.axes;
            out.center = *center;
            for (pl, k) in &pp.lines {
                for piece in clip_circle(pl, *center, *radius) {
                    out.lines.push((piece, *k));
                }
            }
            for lp in &pp.hatch {
                for piece in clip_circle(lp, *center, *radius) {
                    out.lines.push((piece, 0));
                }
            }
            out.lines.push((circle(*center, *radius), 4));
            out.bounds();
            return out;
        }
        if let ViewKind::Schematic(d) = &v.kind {
            // o esquema só precisa das juntas e posições (as peças podem nem estar no disco)
            out.error = None;
            match &l.model {
                Model::Asm(a) => schematic(a, Axes::of(*d), &mut out),
                Model::Part(_) => out.error = Some("esquema cinemático: a origem precisa ser uma montagem com juntas".into()),
            }
            out.bounds();
            out.center = [(out.lo[0] + out.hi[0]) / 2., (out.lo[1] + out.hi[1]) / 2.];
            return out;
        }
        let shape = match (&v.kind, &l.model) {
            (ViewKind::Flat, Model::Part(doc)) => {
                let mut flat = (**doc).clone();
                match &mut flat.sheet {
                    Some(s) => {
                        s.flat = true;
                        scc_solidos::final_shape(&flat)
                    }
                    None => {
                        out.error = Some("a peça não é de chapa: não há planificação".into());
                        return out;
                    }
                }
            }
            _ => l.shape.clone(),
        };
        let Some(shape) = shape else { return out };
        let Some((lo, hi)) = shape.bbox() else { return out };
        let c3 = [0, 1, 2].map(|k| (lo[k] + hi[k]) / 2.);
        let big = (0..3).map(|k| hi[k] - lo[k]).fold(0., f64::max) * 2. + 10.;
        let axes = match &v.kind {
            ViewKind::Standard(d) | ViewKind::Projected { dir: d, .. } => Axes::of(*d),
            ViewKind::Flat => {
                // olhando pela menor dimensão (a espessura da chapa)
                let e = [0, 1, 2].map(|k| hi[k] - lo[k]);
                if e[2] <= e[0] && e[2] <= e[1] {
                    Axes::of(Dir::Topo)
                } else if e[1] <= e[0] {
                    Axes::of(Dir::Frente)
                } else {
                    Axes::of(Dir::Direita)
                }
            }
            ViewKind::Section { a, b, .. } => {
                let Some(pa) = parent.and_then(|p| p.axes) else { return out };
                let (a3, b3) = (pa.unproject(*a), pa.unproject(*b));
                let Some(u) = unit(sub(b3, a3)) else {
                    out.error = Some("linha de corte sem comprimento".into());
                    return out;
                };
                let n = cross(pa.toward, u);
                Axes::new(n, u)
            }
            ViewKind::Detail { .. } | ViewKind::Schematic(_) => unreachable!(),
        };
        out.axes = Some(axes);
        out.center = axes.project(c3);
        let target = if let ViewKind::Section { a, .. } = &v.kind {
            // remove o lado do observador e hachura a face cortada
            let pa = parent.and_then(|p| p.axes).unwrap();
            let o = pa.unproject(*a);
            // ponto do plano no meio da peça (a profundidade da linha na vista mãe é livre)
            let o = add(o, scale3(pa.toward, dot(sub(c3, o), pa.toward)));
            let (x, n) = (axes.x, axes.toward);
            let y = cross(n, x);
            let corner = sub(sub(o, scale3(x, big)), scale3(y, big));
            match Shape::box_tool(corner, x, n, [2. * big, 2. * big, big]).and_then(|b| shape.boolean(&b, Bool::Cut)) {
                Ok(s) => {
                    if let Ok(loops) = shape.section_loops(o, n, big) {
                        out.hatch = loops.iter().map(|lp| lp.iter().map(|p| axes.project(*p)).collect()).collect();
                    }
                    s
                }
                Err(e) => {
                    out.error = Some(format!("corte: {}", e.0));
                    shape
                }
            }
        } else {
            shape
        };
        match target.hlr(axes.toward, axes.x, hidden) {
            Ok(lines) => out.lines = lines.into_iter().filter(|(_, k)| tangent || *k != 1).collect(),
            Err(e) => out.error = Some(e.0),
        }
        out.bounds();
        out
    }

    /// Dados de uma tabela.
    pub fn table(&mut self, t: &Table, d: &Drawing) -> TableData {
        let full = self.full(&t.source.path);
        let dir = full.parent().map(|p| p.to_path_buf());
        let num = |v: f64| d.num(v);
        let l = self.load(&t.source);
        if let Some(e) = &l.error {
            if l.shape.is_none() && matches!(l.model, Model::Part(_)) && l.files.len() == 1 && e.contains(':') {
                return TableData { headers: vec!["Erro".into()], rows: vec![vec![e.clone()]] };
            }
        }
        match (&t.kind, &l.model) {
            (TableKind::Bom, Model::Asm(a)) => {
                let a = (**a).clone();
                let mut rows: Vec<(String, u32, Vec<String>)> = vec![];
                for n in &a.nodes {
                    let (path, sub) = match &n.kind {
                        NodeKind::Part { path } => (path, false),
                        NodeKind::SubAssembly { path } => (path, true),
                        NodeKind::Group => continue,
                    };
                    if let Some(r) = rows.iter_mut().find(|r| r.0 == *path) {
                        r.1 += 1;
                        continue;
                    }
                    let fp = full_path(dir.as_deref(), path);
                    let (pn, desc, mat) = if sub {
                        match std::fs::read_to_string(&fp).ok().and_then(|s| Assembly::from_json(&s).ok()) {
                            Some(x) => (x.props.pn, if x.props.description.is_empty() { x.name } else { x.props.description }, "—".to_string()),
                            None => (String::new(), n.name.clone(), String::new()),
                        }
                    } else {
                        match self.parts.get(&fp, false) {
                            Ok(p) => {
                                let pr = &p.doc.props;
                                (pr.pn.clone(), if pr.description.is_empty() { p.doc.name.clone() } else { pr.description.clone() }, pr.material.clone())
                            }
                            Err(_) => (String::new(), n.name.clone(), String::new()),
                        }
                    };
                    rows.push((path.clone(), 1, vec![pn, desc, mat]));
                }
                TableData {
                    headers: ["Item", "Código", "Descrição", "Material", "Qtd."].map(String::from).to_vec(),
                    rows: rows
                        .into_iter()
                        .enumerate()
                        .map(|(i, (_, q, c))| vec![(i + 1).to_string(), c[0].clone(), c[1].clone(), c[2].clone(), q.to_string()])
                        .collect(),
                }
            }
            (TableKind::Bom, Model::Part(doc)) => TableData {
                headers: ["Item", "Código", "Descrição", "Material", "Qtd."].map(String::from).to_vec(),
                rows: vec![vec!["1".into(), doc.props.pn.clone(), if doc.props.description.is_empty() { doc.name.clone() } else { doc.props.description.clone() }, doc.props.material.clone(), "1".into()]],
            },
            (TableKind::Holes, Model::Part(doc)) => TableData {
                headers: ["Furo", "X", "Y", "Z", "Ø", "Prof.", "Rosca"].map(String::from).to_vec(),
                rows: scc_solidos::hole_table(doc)
                    .iter()
                    .enumerate()
                    .map(|(i, h)| vec![format!("A{}", i + 1), num(h.pos[0]), num(h.pos[1]), num(h.pos[2]), num(h.diameter), if h.through { "passante".into() } else { num(h.depth) }, h.thread.clone()])
                    .collect(),
            },
            (TableKind::Bends, Model::Part(doc)) => TableData {
                headers: ["Dobra", "Ângulo", "Raio", "Desenvolvido", "Dedução"].map(String::from).to_vec(),
                rows: scc_solidos::bend_table(doc)
                    .iter()
                    .map(|b| vec![b.name.clone(), format!("{}°", num(b.angle)), num(b.radius), num(b.allowance), num(b.deduction)])
                    .collect(),
            },
            (TableKind::CutList, Model::Part(doc)) => TableData {
                headers: ["Item", "Perfil", "Comprimento", "Qtd."].map(String::from).to_vec(),
                rows: scc_solidos::cut_list(doc)
                    .iter()
                    .enumerate()
                    .map(|(i, r)| vec![(i + 1).to_string(), r.designation.clone(), num(r.length), r.count.to_string()])
                    .collect(),
            },
            (k, Model::Asm(_)) => TableData { headers: vec![k.label().into()], rows: vec![vec!["escolha uma peça (esta tabela não vale para montagem)".into()]] },
        }
    }

    /// Propriedades da origem (para preencher a legenda).
    pub fn props(&mut self, src: &Source) -> Option<(String, String, String)> {
        match &self.load(src).model {
            Model::Part(d) => Some((d.props.pn.clone(), if d.props.description.is_empty() { d.name.clone() } else { d.props.description.clone() }, d.props.material.clone())),
            Model::Asm(a) => Some((a.props.pn.clone(), if a.props.description.is_empty() { a.name.clone() } else { a.props.description.clone() }, String::new())),
        }
    }
}

/// Esquema cinemático: símbolo de cada junta, barras ligando as juntas de cada peça e terra nas
/// peças fixas (tamanho dos símbolos proporcional ao mecanismo).
fn schematic(a: &Assembly, ax: Axes, out: &mut Proj) {
    use scc_montagem::kin::JointKind as J;
    let joints: Vec<_> = a.joints.iter().filter(|j| !j.suppressed).collect();
    if joints.is_empty() {
        out.error = Some("a montagem não tem juntas (Cinemática › juntas ou Mecanismo a partir das restrições)".into());
        return;
    }
    out.axes = Some(ax);
    let place = |c: Id| a.node(c).map(|n| n.placement).unwrap_or_default();
    // posição e eixo (na vista) de cada junta, pelo lado A
    let info: Vec<(V2, V2, &J, Id, Id)> = joints
        .iter()
        .map(|j| {
            let (o, z, _) = j.a.frame.world(&place(j.a.comp));
            (ax.project(o), [dot(z, ax.x), dot(z, ax.y)], &j.kind, j.a.comp, j.b.comp)
        })
        .collect();
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for (p, ..) in &info {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let s = ((hi[0] - lo[0]).max(hi[1] - lo[1]) * 0.05).max(3.);
    let rect = |c: V2, u: V2, a: f64, b: f64| -> Vec<V2> {
        let n = [-u[1], u[0]];
        let q = |i: f64, j: f64| [c[0] + u[0] * a * i + n[0] * b * j, c[1] + u[1] * a * i + n[1] * b * j];
        vec![q(-1., -1.), q(1., -1.), q(1., 1.), q(-1., 1.), q(-1., -1.)]
    };
    for (p, z, kind, ca, _) in &info {
        let zl = (z[0] * z[0] + z[1] * z[1]).sqrt();
        let u = if zl > 1e-6 { [z[0] / zl, z[1] / zl] } else { [1., 0.] };
        let edge_on = zl > 0.3;
        match kind {
            J::Revolute | J::Cylindrical | J::Screw { .. } if !edge_on => out.lines.push((circle(*p, s * 0.5), 0)),
            J::Revolute | J::Cylindrical | J::Screw { .. } => {
                // mancal visto de lado: retângulo atravessado pelo eixo
                out.lines.push((rect(*p, u, s * 0.6, s * 0.35), 0));
                out.lines.push((vec![[p[0] - u[0] * s, p[1] - u[1] * s], [p[0] + u[0] * s, p[1] + u[1] * s]], 0));
            }
            J::Prismatic => {
                out.lines.push((rect(*p, u, s * 0.8, s * 0.4), 0));
                out.lines.push((vec![[p[0] - u[0] * s * 1.6, p[1] - u[1] * s * 1.6], [p[0] + u[0] * s * 1.6, p[1] + u[1] * s * 1.6]], 0));
            }
            J::Spherical => {
                out.lines.push((circle(*p, s * 0.55), 0));
                out.lines.push((circle(*p, s * 0.25), 0));
            }
            J::Cam { profile, .. } => {
                // o próprio perfil do came, posicionado
                let pl = place(*ca);
                let mut pts: Vec<V2> = profile.iter().map(|q| ax.project(pl.point(*q))).collect();
                if let Some(f) = pts.first().copied() {
                    pts.push(f);
                }
                out.lines.push((pts, 0));
            }
            _ => out.lines.push((rect(*p, [1., 0.], s * 0.4, s * 0.4), 0)),
        }
    }
    // barras: juntas de cada peça ligadas em sequência
    let mut comps: Vec<Id> = vec![];
    for (_, _, _, ca, cb) in &info {
        for c in [ca, cb] {
            if !comps.contains(c) {
                comps.push(*c);
            }
        }
    }
    for c in comps {
        let pts: Vec<V2> = info.iter().filter(|i| i.3 == c || i.4 == c).map(|i| i.0).collect();
        let fixed = a.node(c).is_some_and(|n| n.fixed);
        if pts.len() >= 2 && !fixed {
            out.lines.push((pts.clone(), 0));
        }
        if fixed {
            // terra: base com hachuras sob cada junta da peça fixa
            for p in pts {
                let y = p[1] - s * 0.9;
                out.lines.push((vec![[p[0], p[1] - s * 0.5], [p[0], y]], 0));
                out.lines.push((vec![[p[0] - s, y], [p[0] + s, y]], 0));
                for i in 0..5 {
                    let x = p[0] - s + i as f64 * s * 0.5;
                    out.lines.push((vec![[x, y], [x - s * 0.35, y - s * 0.4]], 0));
                }
            }
        }
    }
}

// ---------- geometria 2D/3D simples

pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn scale3(a: V3, k: f64) -> V3 {
    a.map(|x| x * k)
}
pub fn unit(a: V3) -> Option<V3> {
    let l = dot(a, a).sqrt();
    (l > 1e-12).then(|| a.map(|x| x / l))
}

pub fn circle(c: V2, r: f64) -> Vec<V2> {
    (0..=72).map(|i| {
        let t = i as f64 / 72. * std::f64::consts::TAU;
        [c[0] + r * t.cos(), c[1] + r * t.sin()]
    })
    .collect()
}

/// Partes da polilinha dentro do círculo (cortando os segmentos na borda).
pub fn clip_circle(pl: &[V2], c: V2, r: f64) -> Vec<Vec<V2>> {
    let inside = |p: V2| (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) <= r * r;
    // interseção do segmento p→q com o círculo (parâmetros t em [0,1])
    let hits = |p: V2, q: V2| -> Vec<f64> {
        let d = [q[0] - p[0], q[1] - p[1]];
        let f = [p[0] - c[0], p[1] - c[1]];
        let a = d[0] * d[0] + d[1] * d[1];
        if a < 1e-18 {
            return vec![];
        }
        let b = 2. * (f[0] * d[0] + f[1] * d[1]);
        let cc = f[0] * f[0] + f[1] * f[1] - r * r;
        let disc = b * b - 4. * a * cc;
        if disc < 0. {
            return vec![];
        }
        let s = disc.sqrt();
        let mut v: Vec<f64> = [(-b - s) / (2. * a), (-b + s) / (2. * a)].into_iter().filter(|t| (0. ..=1.).contains(t)).collect();
        v.sort_by(f64::total_cmp);
        v
    };
    let lerp = |p: V2, q: V2, t: f64| [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t];
    let mut out = vec![];
    let mut cur: Vec<V2> = vec![];
    for w in pl.windows(2) {
        let (p, q) = (w[0], w[1]);
        let mut ts = vec![0.];
        ts.extend(hits(p, q));
        ts.push(1.);
        for s in ts.windows(2) {
            let (a, b) = (lerp(p, q, s[0]), lerp(p, q, s[1]));
            let m = lerp(p, q, (s[0] + s[1]) / 2.);
            if inside(m) {
                if cur.is_empty() {
                    cur.push(a);
                }
                cur.push(b);
            } else if cur.len() >= 2 {
                out.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
    }
    if cur.len() >= 2 {
        out.push(cur);
    }
    out
}
