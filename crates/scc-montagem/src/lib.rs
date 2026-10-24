//! Montagem: árvore de componentes (grupos, peças, subconjuntos), posicionamento por restrições
//! e solucionador de graus de liberdade.
//!
//! - [`Assembly`]: documento `.sccasm` (serializado em JSON) com nós e restrições.
//! - [`solve`]: Levenberg–Marquardt sobre 6 GDL por corpo livre (componentes fixos não se movem;
//!   uma fixação conjunta faz várias peças se moverem como um corpo só).
//! - [`load`]: carrega e calcula as peças referenciadas (com cache por arquivo).

pub mod bom;
pub mod display;
pub mod kin;
pub mod load;
pub mod view;
pub mod solve;

use scc_core::{expr, Id, Properties};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type V3 = [f64; 3];

/// Posição de um componente: origem e rotação (colunas = eixos locais no mundo).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Placement {
    pub origin: V3,
    pub rot: [[f64; 3]; 3],
}

impl Default for Placement {
    fn default() -> Self {
        Placement { origin: [0.; 3], rot: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]] }
    }
}

impl Placement {
    /// Ponto local → mundo.
    pub fn point(&self, p: V3) -> V3 {
        let r = &self.rot;
        [0, 1, 2].map(|i| self.origin[i] + r[0][i] * p[0] + r[1][i] * p[1] + r[2][i] * p[2])
    }
    /// Direção local → mundo.
    pub fn dir(&self, v: V3) -> V3 {
        let r = &self.rot;
        [0, 1, 2].map(|i| r[0][i] * v[0] + r[1][i] * v[1] + r[2][i] * v[2])
    }
    /// Ponto do mundo → local.
    pub fn to_local(&self, p: V3) -> V3 {
        let d = [p[0] - self.origin[0], p[1] - self.origin[1], p[2] - self.origin[2]];
        self.rot.map(|c| c[0] * d[0] + c[1] * d[1] + c[2] * d[2])
    }
    /// Direção do mundo → local.
    pub fn dir_to_local(&self, v: V3) -> V3 {
        self.rot.map(|c| c[0] * v[0] + c[1] * v[1] + c[2] * v[2])
    }
    /// Gira em torno do ponto `about` pelo vetor de rotação `w` (eixo × ângulo em radianos) e translada.
    pub fn moved(&self, t: V3, w: V3, about: V3) -> Placement {
        let rot_v = |v: V3| rotvec(v, w);
        let o = [0, 1, 2].map(|i| self.origin[i] - about[i]);
        let o = rot_v(o);
        Placement { origin: [0, 1, 2].map(|i| o[i] + about[i] + t[i]), rot: self.rot.map(rot_v) }
    }
    /// Composição: `self` aplicado depois de `inner` (inner em coordenadas deste).
    pub fn then(&self, inner: &Placement) -> Placement {
        Placement { origin: self.point(inner.origin), rot: inner.rot.map(|c| self.dir(c)) }
    }
    /// Inversa.
    pub fn inverse(&self) -> Placement {
        let r = &self.rot;
        let rt = [[r[0][0], r[1][0], r[2][0]], [r[0][1], r[1][1], r[2][1]], [r[0][2], r[1][2], r[2][2]]];
        let inv = Placement { origin: [0.; 3], rot: rt };
        let o = inv.dir(self.origin);
        Placement { origin: [-o[0], -o[1], -o[2]], rot: rt }
    }
}

/// Rotação de `v` pelo vetor de rotação `w` (Rodrigues).
pub fn rotvec(v: V3, w: V3) -> V3 {
    let a = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt();
    if a < 1e-15 {
        return v;
    }
    let k = [w[0] / a, w[1] / a, w[2] / a];
    let (c, s) = (a.cos(), a.sin());
    let kv = cross(k, v);
    let kd = dot(k, v);
    [0, 1, 2].map(|i| v[i] * c + kv[i] * s + k[i] * kd * (1. - c))
}

pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}
pub fn unit(a: V3) -> V3 {
    let n = norm(a).max(1e-300);
    [a[0] / n, a[1] / n, a[2] / n]
}

/// Tipo de nó da árvore da montagem.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum NodeKind {
    /// elemento vazio só para organizar
    Group,
    /// peça (arquivo `.sccpart`, relativo à pasta da montagem ou absoluto)
    Part { path: String },
    /// subconjunto (arquivo `.sccasm`)
    SubAssembly { path: String },
}

/// Como o componente é carregado: editável (geometria completa) ou só visualização (leve).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum LoadMode {
    #[default]
    Editavel,
    Visualizacao,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: Id,
    pub name: String,
    /// número sequencial de identificação (numeração automática; 0 = sem número)
    #[serde(default)]
    pub number: u32,
    pub kind: NodeKind,
    /// grupo/subconjunto pai na árvore (só organização: não muda a geometria)
    #[serde(default)]
    pub parent: Option<Id>,
    #[serde(default)]
    pub placement: Placement,
    /// fixo: referência global, sem graus de liberdade
    #[serde(default)]
    pub fixed: bool,
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default)]
    pub mode: LoadMode,
    /// cor de destaque do componente (sobrepõe a da peça)
    #[serde(default)]
    pub color: Option<[u8; 3]>,
}

fn yes() -> bool {
    true
}

/// Geometria de um componente usada numa restrição, no sistema local da peça.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Geom {
    /// plano / face plana: ponto e normal
    Plane { p: V3, n: V3 },
    /// eixo / face cilíndrica ou cônica / aresta reta: ponto e direção
    Axis { p: V3, d: V3 },
    Point(V3),
}

/// Elemento de um componente escolhido para a restrição. `pick` é o ponto clicado (local), usado
/// para reencontrar a face quando a peça muda.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct MateRef {
    pub comp: Id,
    pub geom: Geom,
    #[serde(default)]
    pub pick: Option<V3>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum MateKind {
    /// eixos alinhados, pontos coincidentes, faces coplanares (mesmo sentido; `flip`: invertido)
    Coincident { flip: bool },
    /// faces em contato (coplanares e de frente uma para a outra)
    Contact,
    /// distância (aceita expressão; negativa troca o lado)
    Distance(String),
    /// ângulo entre planos/eixos (graus)
    Angle(String),
    Parallel,
    Perpendicular,
}

impl MateKind {
    pub fn label(&self) -> &'static str {
        match self {
            MateKind::Coincident { .. } => "Coincidência",
            MateKind::Contact => "Contato",
            MateKind::Distance(_) => "Distância",
            MateKind::Angle(_) => "Ângulo",
            MateKind::Parallel => "Paralelismo",
            MateKind::Perpendicular => "Perpendicularidade",
        }
    }
    pub fn icon(&self) -> &'static str {
        match self {
            MateKind::Coincident { .. } => "◎",
            MateKind::Contact => "⫴",
            MateKind::Distance(_) => "↔",
            MateKind::Angle(_) => "∠",
            MateKind::Parallel => "∥",
            MateKind::Perpendicular => "⟂",
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Mate {
    pub id: Id,
    pub name: String,
    pub kind: MateKind,
    pub a: MateRef,
    pub b: MateRef,
    #[serde(default)]
    pub suppressed: bool,
}

/// Fixação conjunta: as peças se comportam como um corpo rígido único.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RigidGroup {
    pub id: Id,
    pub name: String,
    pub comps: Vec<Id>,
}

/// Repetição inteligente: cópias de um componente em padrão linear ou circular. As cópias
/// acompanham o original e são refeitas quando a quantidade muda.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct CompPattern {
    pub id: Id,
    pub name: String,
    pub source: Id,
    pub kind: PatternKind,
    /// cópias geradas (nós da árvore, na ordem)
    #[serde(default)]
    pub instances: Vec<Id>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PatternKind {
    /// direção no mundo, quantidade total (com o original) e espaçamento
    Linear { dir: V3, count: String, spacing: String },
    /// eixo no mundo, quantidade total e ângulo total (360 = volta completa)
    Circular { origin: V3, axis: V3, count: String, angle: String },
}

/// Parâmetro da montagem (usável nas distâncias e ângulos).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AsmParam {
    pub name: String,
    pub expr: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Assembly {
    pub format_version: u32,
    pub name: String,
    #[serde(default)]
    pub props: Properties,
    next_id: u64,
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub mates: Vec<Mate>,
    #[serde(default)]
    pub rigid: Vec<RigidGroup>,
    #[serde(default)]
    pub params: Vec<AsmParam>,
    #[serde(default)]
    pub patterns: Vec<CompPattern>,
    /// juntas da cinemática e acoplamentos entre elas
    #[serde(default)]
    pub joints: Vec<kin::Joint>,
    #[serde(default)]
    pub couplings: Vec<kin::Coupling>,
    /// estados de exibição salvos e animações (composer)
    #[serde(default)]
    pub display_states: Vec<display::DisplayState>,
    #[serde(default)]
    pub animations: Vec<display::Animation>,
    /// anotações 3D (PMI) deste nível de montagem
    #[serde(default)]
    pub pmi: Vec<scc_core::pmi::Pmi>,
    /// números de item fixos da lista de peças (por arquivo do componente)
    #[serde(default)]
    pub items: Vec<bom::ItemNo>,
}

impl Assembly {
    pub fn new(name: &str) -> Self {
        Assembly { format_version: 1, name: name.into(), props: Properties::default(), next_id: 1, nodes: vec![], mates: vec![], rigid: vec![], params: vec![], patterns: vec![], joints: vec![], couplings: vec![], display_states: vec![], animations: vec![], pmi: vec![], items: vec![] }
    }

    pub fn alloc(&mut self) -> Id {
        let id = Id(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }

    pub fn node(&self, id: Id) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: Id) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn name_of(&self, id: Id) -> String {
        self.node(id)
            .map(|n| n.name.clone())
            .or_else(|| self.mates.iter().find(|m| m.id == id).map(|m| m.name.clone()))
            .or_else(|| self.rigid.iter().find(|r| r.id == id).map(|r| r.name.clone()))
            .or_else(|| self.joints.iter().find(|r| r.id == id).map(|r| r.name.clone()))
            .or_else(|| self.couplings.iter().find(|r| r.id == id).map(|r| r.name.clone()))
            .or_else(|| self.patterns.iter().find(|r| r.id == id).map(|r| r.name.clone()))
            .unwrap_or_else(|| format!("#{}", id.0))
    }

    pub(crate) fn unique_pub(&self, base: &str) -> String {
        self.unique(base)
    }

    fn unique(&self, base: &str) -> String {
        let taken = |n: &str| {
            self.nodes.iter().any(|x| x.name == n)
                || self.mates.iter().any(|x| x.name == n)
                || self.rigid.iter().any(|x| x.name == n)
                || self.patterns.iter().any(|x| x.name == n)
                || self.joints.iter().any(|x| x.name == n)
                || self.couplings.iter().any(|x| x.name == n)
        };
        if !taken(base) {
            return base.to_string();
        }
        (2..).map(|i| format!("{base}.{i}")).find(|n| !taken(n)).unwrap()
    }

    /// Grupo vazio de organização.
    pub fn add_group(&mut self, name: &str, parent: Option<Id>) -> Id {
        let id = self.alloc();
        let name = self.unique(name);
        self.nodes.push(Node { id, name, number: 0, kind: NodeKind::Group, parent, placement: Placement::default(), fixed: false, visible: true, mode: LoadMode::Editavel, color: None });
        id
    }

    /// Insere uma peça (ou subconjunto) existente. O primeiro componente entra fixo.
    pub fn add_component(&mut self, name: &str, kind: NodeKind, placement: Placement, parent: Option<Id>) -> Id {
        let id = self.alloc();
        let name = self.unique(name);
        let first = !self.nodes.iter().any(|n| !matches!(n.kind, NodeKind::Group));
        let number = self.nodes.iter().map(|n| n.number).max().unwrap_or(0) + 1;
        self.nodes.push(Node { id, name, number, kind, parent, placement, fixed: first, visible: true, mode: LoadMode::Editavel, color: None });
        id
    }

    pub fn add_mate(&mut self, kind: MateKind, a: MateRef, b: MateRef) -> Id {
        let id = self.alloc();
        let name = self.unique(kind.label());
        self.mates.push(Mate { id, name, kind, a, b, suppressed: false });
        id
    }

    pub fn add_rigid(&mut self, comps: Vec<Id>) -> Id {
        let id = self.alloc();
        let name = self.unique("Fixação conjunta");
        self.rigid.push(RigidGroup { id, name, comps });
        id
    }

    pub fn add_pattern(&mut self, source: Id, kind: PatternKind) -> Id {
        let id = self.alloc();
        let name = self.unique("Repetição");
        self.patterns.push(CompPattern { id, name, source, kind, instances: vec![] });
        self.update_patterns();
        id
    }

    /// Refaz as cópias de cada repetição: cria/apaga instâncias conforme a quantidade e posiciona
    /// cada uma a partir do original.
    pub fn update_patterns(&mut self) {
        let vars = self.vars();
        let pats = self.patterns.clone();
        for (k, p) in pats.iter().enumerate() {
            let Some(src) = self.node(p.source).cloned() else { continue };
            let num = |e: &String| expr::eval_str(e, &vars).unwrap_or(1.);
            let (count, steps): (usize, Vec<Placement>) = match &p.kind {
                PatternKind::Linear { dir, count, spacing } => {
                    let n = num(count).round().max(1.) as usize;
                    let d = unit(*dir);
                    let s = num(spacing);
                    (n, (1..n).map(|i| src.placement.moved(d.map(|v| v * s * i as f64), [0.; 3], src.placement.origin)).collect())
                }
                PatternKind::Circular { origin, axis, count, angle } => {
                    let n = num(count).round().max(1.) as usize;
                    let total = num(angle);
                    let step = if (total - 360.).abs() < 1e-9 { total / n as f64 } else { total / (n.max(2) - 1) as f64 };
                    let a = unit(*axis);
                    (n, (1..n).map(|i| src.placement.moved([0.; 3], a.map(|v| v * (step * i as f64).to_radians()), *origin)).collect())
                }
            };
            let mut inst = p.instances.clone();
            inst.retain(|i| self.node(*i).is_some());
            while inst.len() + 1 < count {
                let id = self.alloc();
                let name = self.unique(&src.name);
                self.nodes.push(Node { id, name, number: 0, fixed: true, ..src.clone() });
                inst.push(id);
            }
            while inst.len() + 1 > count {
                let id = inst.pop().unwrap();
                self.nodes.retain(|n| n.id != id);
                self.mates.retain(|m| m.a.comp != id && m.b.comp != id);
            }
            for (id, pl) in inst.iter().zip(steps) {
                if let Some(n) = self.node_mut(*id) {
                    n.placement = pl;
                    n.kind = src.kind.clone();
                    // as cópias seguem o original: não têm graus de liberdade próprios
                    n.fixed = true;
                }
            }
            self.patterns[k].instances = inst;
        }
    }

    /// Numeração automática: índices sequenciais na ordem da árvore (só peças e subconjuntos).
    pub fn renumber(&mut self) {
        let mut k = 0;
        for n in self.nodes.iter_mut().filter(|n| !matches!(n.kind, NodeKind::Group)) {
            k += 1;
            n.number = k;
        }
    }

    /// Apaga um nó (filhos sobem um nível) ou uma restrição; restrições do componente vão junto.
    pub fn delete(&mut self, id: Id) {
        let parent = self.node(id).and_then(|n| n.parent);
        for n in self.nodes.iter_mut().filter(|n| n.parent == Some(id)) {
            n.parent = parent;
        }
        self.nodes.retain(|n| n.id != id);
        self.mates.retain(|m| m.id != id && m.a.comp != id && m.b.comp != id);
        self.joints.retain(|j| j.id != id && j.a.comp != id && j.b.comp != id);
        let alive: Vec<Id> = self.joints.iter().map(|j| j.id).collect();
        self.couplings.retain(|c| c.id != id && alive.contains(&c.a) && alive.contains(&c.b));
        self.rigid.retain(|r| r.id != id);
        // apagar a repetição apaga as cópias; apagar o original apaga a repetição
        let gone: Vec<Id> = self.patterns.iter().filter(|p| p.id == id || p.source == id).flat_map(|p| p.instances.clone()).collect();
        self.patterns.retain(|p| p.id != id && p.source != id);
        self.nodes.retain(|n| !gone.contains(&n.id));
        for r in &mut self.rigid {
            r.comps.retain(|c| *c != id);
        }
    }

    /// Substitui a peça de um componente mantendo nome, posição e restrições (as faces são
    /// reencontradas pelo ponto clicado).
    pub fn replace(&mut self, id: Id, kind: NodeKind) {
        if let Some(n) = self.node_mut(id) {
            n.kind = kind;
        }
    }

    /// Reordena só a árvore (visualização): move `id` para antes de `before` (ou para o fim).
    pub fn reorder(&mut self, id: Id, before: Option<Id>) {
        let Some(i) = self.nodes.iter().position(|n| n.id == id) else { return };
        let n = self.nodes.remove(i);
        let j = before.and_then(|b| self.nodes.iter().position(|x| x.id == b)).unwrap_or(self.nodes.len());
        self.nodes.insert(j, n);
    }

    /// Valores dos parâmetros da montagem.
    pub fn vars(&self) -> HashMap<String, f64> {
        let mut v = HashMap::new();
        for p in &self.params {
            if let Ok(x) = expr::eval_str(&p.expr, &v) {
                v.insert(p.name.clone(), x);
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_ida_e_volta() {
        let p = Placement::default().moved([1., 2., 3.], [0., 0., std::f64::consts::FRAC_PI_2], [0.; 3]);
        let q = [1., 0., 0.];
        let w = p.point(q);
        assert!(norm(sub(w, [1., 3., 3.])) < 1e-12, "{w:?}");
        assert!(norm(sub(p.to_local(w), q)) < 1e-12);
        let inv = p.inverse();
        assert!(norm(sub(inv.point(w), q)) < 1e-12);
    }

    #[test]
    fn repeticao_acompanha_a_quantidade() {
        let mut a = Assembly::new("P");
        let _base = a.add_component("Base", NodeKind::Part { path: "b.sccpart".into() }, Placement::default(), None);
        let bolt = a.add_component("Parafuso", NodeKind::Part { path: "p.sccpart".into() }, Placement { origin: [10., 0., 0.], ..Default::default() }, None);
        let pat = a.add_pattern(bolt, PatternKind::Linear { dir: [1., 0., 0.], count: "3".into(), spacing: "20".into() });
        assert_eq!(a.patterns[0].instances.len(), 2);
        let last = *a.patterns[0].instances.last().unwrap();
        assert!(norm(sub(a.node(last).unwrap().placement.origin, [50., 0., 0.])) < 1e-9);
        if let PatternKind::Linear { count, .. } = &mut a.patterns[0].kind {
            *count = "5".into();
        }
        a.update_patterns();
        assert_eq!(a.nodes.len(), 6, "base + original + 4 cópias");
        a.delete(pat);
        assert_eq!(a.nodes.len(), 2);
    }

    #[test]
    fn json_e_numeracao() {
        let mut a = Assembly::new("A");
        let g = a.add_group("Fixadores", None);
        let p1 = a.add_component("Base", NodeKind::Part { path: "base.sccpart".into() }, Placement::default(), None);
        let p2 = a.add_component("Tampa", NodeKind::Part { path: "tampa.sccpart".into() }, Placement::default(), Some(g));
        assert!(a.node(p1).unwrap().fixed && !a.node(p2).unwrap().fixed, "o primeiro entra fixo");
        a.reorder(p2, Some(g));
        a.renumber();
        assert_eq!(a.node(p2).unwrap().number, 1);
        let b = Assembly::from_json(&a.to_json()).unwrap();
        assert_eq!(a, b);
        a.delete(g);
        assert_eq!(a.node(p2).unwrap().parent, None, "filhos sobem um nível");
    }
}
