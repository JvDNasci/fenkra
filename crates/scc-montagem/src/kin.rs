//! Cinemática: juntas (graus de liberdade permitidos), comandos (ângulo/curso por valor ou lei
//! no tempo `t`), acoplamentos (engrenagem, pinhão-cremalheira, cabo) e simulação por quadros
//! com velocidade e aceleração.
//!
//! Cada junta liga um sistema local (origem, eixo z, referência x) de cada componente. Os
//! resíduos entram no mesmo solucionador das restrições.

use crate::*;
use std::collections::HashMap;

/// Sistema local de uma junta no componente: origem, eixo (z) e direção de referência (x ⟂ z).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct JFrame {
    pub o: V3,
    pub z: V3,
    pub x: V3,
}

impl JFrame {
    /// Sistema a partir de um ponto e um eixo (x escolhido perpendicular, estável).
    pub fn from_axis(o: V3, z: V3) -> JFrame {
        let z = unit(z);
        let h = if z[2].abs() < 0.9 { [0., 0., 1.] } else { [1., 0., 0.] };
        JFrame { o, z, x: unit(cross(h, z)) }
    }
    fn world(&self, p: &Placement) -> (V3, V3, V3) {
        (p.point(self.o), unit(p.dir(self.z)), unit(p.dir(self.x)))
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct JRef {
    pub comp: Id,
    pub frame: JFrame,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum JointKind {
    /// rotação em torno do eixo (retenção axial)
    Revolute,
    /// translação ao longo do eixo (sem girar)
    Prismatic,
    /// rotação e translação no mesmo eixo
    Cylindrical,
    /// rotação e avanço ligados pelo passo (mm por volta)
    Screw { pitch: String },
    /// três rotações livres, sem translação
    Spherical,
    /// duas translações e uma rotação sobre o plano (z = normal)
    Planar,
    /// sem graus de liberdade
    Rigid,
    /// o ponto (origem de B) segue a curva (polilinha no sistema de A)
    PointCurve { curve: Vec<V3> },
}

impl JointKind {
    pub fn label(&self) -> &'static str {
        match self {
            JointKind::Revolute => "Revolução",
            JointKind::Prismatic => "Prismática",
            JointKind::Cylindrical => "Cilíndrica",
            JointKind::Screw { .. } => "Parafuso",
            JointKind::Spherical => "Esférica",
            JointKind::Planar => "Planar",
            JointKind::Rigid => "Rígida",
            JointKind::PointCurve { .. } => "Ponto em curva",
        }
    }
    /// Pode receber comando de ângulo / de curso.
    pub fn drives(&self) -> (bool, bool) {
        match self {
            JointKind::Revolute | JointKind::Screw { .. } => (true, false),
            JointKind::Prismatic => (false, true),
            JointKind::Cylindrical => (true, true),
            _ => (false, false),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Joint {
    pub id: Id,
    pub name: String,
    pub kind: JointKind,
    pub a: JRef,
    pub b: JRef,
    /// ângulo (graus) e curso (mm) medidos na criação: os comandos são relativos a eles
    pub angle0: f64,
    pub len0: f64,
    /// comando de ângulo (graus, expressão; pode usar `t`)
    #[serde(default)]
    pub drive_angle: Option<String>,
    /// comando de curso (mm, expressão; pode usar `t`)
    #[serde(default)]
    pub drive_len: Option<String>,
    #[serde(default)]
    pub suppressed: bool,
}

/// Relação entre duas juntas: engrenagem (ângulo × ângulo), pinhão-cremalheira (ângulo → curso)
/// ou cabo (curso × curso).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum CouplingKind {
    Gear,
    RackPinion,
    Cable,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Coupling {
    pub id: Id,
    pub name: String,
    pub kind: CouplingKind,
    pub a: Id,
    pub b: Id,
    /// razão (engrenagem: −z1/z2; pinhão: mm por volta; cabo: curso b / curso a)
    pub ratio: String,
}

fn wrap180(a: f64) -> f64 {
    let mut a = a % 360.;
    if a > 180. {
        a -= 360.;
    }
    if a <= -180. {
        a += 360.;
    }
    a
}

/// Ângulo de B em relação a A em torno do eixo de A (graus).
fn angle_of(j: &Joint, pa: &Placement, pb: &Placement) -> f64 {
    let (_, za, xa) = j.a.frame.world(pa);
    let (_, _, xb) = j.b.frame.world(pb);
    dot(cross(xa, xb), za).atan2(dot(xa, xb)).to_degrees()
}

/// Curso de B em relação a A ao longo do eixo de A (mm).
fn len_of(j: &Joint, pa: &Placement, pb: &Placement) -> f64 {
    let (oa, za, _) = j.a.frame.world(pa);
    let (ob, _, _) = j.b.frame.world(pb);
    dot(sub(ob, oa), za)
}

/// Ângulo e curso atuais da junta, relativos à criação (o que os comandos medem).
pub fn measure(j: &Joint, pl: &HashMap<Id, Placement>) -> Option<(f64, f64)> {
    let (pa, pb) = (pl.get(&j.a.comp)?, pl.get(&j.b.comp)?);
    Some((wrap180(angle_of(j, pa, pb) - j.angle0), len_of(j, pa, pb) - j.len0))
}

impl Assembly {
    /// Cria a junta medindo o ângulo e o curso atuais (que viram o zero dos comandos).
    pub fn add_joint(&mut self, kind: JointKind, a: JRef, b: JRef) -> Id {
        let id = self.alloc();
        let name = self.unique_pub(kind.label());
        let pl = |c: Id| self.node(c).map(|n| n.placement).unwrap_or_default();
        let mut j = Joint { id, name, kind, a, b, angle0: 0., len0: 0., drive_angle: None, drive_len: None, suppressed: false };
        j.angle0 = angle_of(&j, &pl(a.comp), &pl(b.comp));
        j.len0 = len_of(&j, &pl(a.comp), &pl(b.comp));
        self.joints.push(j);
        id
    }

    pub fn add_coupling(&mut self, kind: CouplingKind, a: Id, b: Id, ratio: &str) -> Id {
        let id = self.alloc();
        let name = self.unique_pub(match kind {
            CouplingKind::Gear => "Engrenagem",
            CouplingKind::RackPinion => "Pinhão-cremalheira",
            CouplingKind::Cable => "Cabo",
        });
        self.couplings.push(Coupling { id, name, kind, a, b, ratio: ratio.into() });
        id
    }

    /// Mecanismo automático: cada par de componentes com restrições vira uma junta equivalente
    /// (eixo + face = revolução; só eixo = cilíndrica; só faces = planar; pontos = esférica).
    /// As restrições convertidas saem da lista. Devolve as juntas criadas.
    pub fn joints_from_mates(&mut self) -> Vec<Id> {
        let mut pairs: Vec<(Id, Id)> = vec![];
        for m in self.mates.iter().filter(|m| !m.suppressed) {
            let k = if m.a.comp.0 < m.b.comp.0 { (m.a.comp, m.b.comp) } else { (m.b.comp, m.a.comp) };
            if !pairs.contains(&k) {
                pairs.push(k);
            }
        }
        let mut made = vec![];
        for (c1, c2) in pairs {
            let ms: Vec<Mate> = self.mates.iter().filter(|m| !m.suppressed && ((m.a.comp == c1 && m.b.comp == c2) || (m.a.comp == c2 && m.b.comp == c1))).cloned().collect();
            let axis = ms.iter().find(|m| matches!(m.a.geom, Geom::Axis { .. }) && matches!(m.b.geom, Geom::Axis { .. }) && matches!(m.kind, MateKind::Coincident { .. }));
            let plane = ms.iter().find(|m| matches!(m.a.geom, Geom::Plane { .. }) && matches!(m.b.geom, Geom::Plane { .. }) && matches!(m.kind, MateKind::Coincident { .. } | MateKind::Contact));
            let point = ms.iter().find(|m| matches!(m.a.geom, Geom::Point(_)) && matches!(m.b.geom, Geom::Point(_)));
            let fr = |g: &Geom| match *g {
                Geom::Axis { p, d } => JFrame::from_axis(p, d),
                Geom::Plane { p, n } => JFrame::from_axis(p, n),
                Geom::Point(p) => JFrame::from_axis(p, [0., 0., 1.]),
            };
            let (kind, src) = match (axis, plane, point) {
                (Some(ax), Some(_), _) => (JointKind::Revolute, ax),
                (Some(ax), None, _) => (JointKind::Cylindrical, ax),
                (None, Some(pl), _) => (JointKind::Planar, pl),
                (None, None, Some(pt)) => (JointKind::Spherical, pt),
                _ => continue,
            };
            let src = src.clone();
            let id = self.add_joint(kind, JRef { comp: src.a.comp, frame: fr(&src.a.geom) }, JRef { comp: src.b.comp, frame: fr(&src.b.geom) });
            // o x de B começa alinhado ao de A (mesmo zero de ângulo)
            let ids: Vec<Id> = ms.iter().map(|m| m.id).collect();
            self.mates.retain(|m| !ids.contains(&m.id));
            made.push(id);
        }
        made
    }
}

/// Resíduos das juntas e acoplamentos (entram no solucionador). `drives = false` ignora os
/// comandos (para contar os graus de liberdade do mecanismo).
pub fn residuals(a: &Assembly, pl: &HashMap<Id, Placement>, vars: &HashMap<String, f64>, drives: bool, r: &mut Vec<f64>, per: &mut Vec<(Id, usize, usize)>) {
    let num = |e: &String| expr::eval_str(e, vars).unwrap_or(0.);
    for j in a.joints.iter().filter(|j| !j.suppressed) {
        let (Some(pa), Some(pb)) = (pl.get(&j.a.comp), pl.get(&j.b.comp)) else { continue };
        let s = r.len();
        let (oa, za, _) = j.a.frame.world(pa);
        let (ob, zb, _) = j.b.frame.world(pb);
        let d = sub(ob, oa);
        let along = dot(d, za);
        let perp = [d[0] - za[0] * along, d[1] - za[1] * along, d[2] - za[2] * along];
        let ang = || wrap180(angle_of(j, pa, pb) - j.angle0);
        match &j.kind {
            JointKind::Revolute => {
                r.extend(cross(za, zb));
                r.extend(perp);
                r.push(along - j.len0);
            }
            JointKind::Prismatic => {
                r.extend(cross(za, zb));
                r.extend(perp);
                r.push(ang());
            }
            JointKind::Cylindrical => {
                r.extend(cross(za, zb));
                r.extend(perp);
            }
            JointKind::Screw { pitch } => {
                r.extend(cross(za, zb));
                r.extend(perp);
                // avanço = passo × voltas (o ângulo é contínuo dentro de ±180°)
                r.push(along - j.len0 - num(pitch) * ang() / 360.);
            }
            JointKind::Spherical => r.extend(d),
            JointKind::Planar => {
                r.extend(cross(za, zb));
                r.push(along);
            }
            JointKind::Rigid => {
                r.extend(cross(za, zb));
                r.push(ang());
                r.extend(d);
            }
            JointKind::PointCurve { curve } => {
                // distância do ponto à curva (no sistema de A)
                let q = pa.to_local(ob);
                let mut best = (f64::MAX, [0.; 3]);
                for w in curve.windows(2) {
                    let (p0, p1) = (w[0], w[1]);
                    let e = sub(p1, p0);
                    let t = (dot(sub(q, p0), e) / dot(e, e).max(1e-18)).clamp(0., 1.);
                    let c = [p0[0] + e[0] * t, p0[1] + e[1] * t, p0[2] + e[2] * t];
                    let dd = norm(sub(q, c));
                    if dd < best.0 {
                        best = (dd, c);
                    }
                }
                r.extend(pa.dir(sub(q, best.1)));
            }
        }
        if drives {
            if let Some(e) = &j.drive_angle {
                r.push(wrap180(ang() - num(e)) * std::f64::consts::PI / 180. * 10.);
            }
            if let Some(e) = &j.drive_len {
                r.push(along - j.len0 - num(e));
            }
        }
        per.push((j.id, s, r.len()));
    }
    for c in &a.couplings {
        let (Some(ja), Some(jb)) = (a.joints.iter().find(|j| j.id == c.a), a.joints.iter().find(|j| j.id == c.b)) else { continue };
        let (Some((aa, la)), Some((ab, lb))) = (measure(ja, pl), measure(jb, pl)) else { continue };
        let k = num(&c.ratio);
        let s = r.len();
        r.push(match c.kind {
            CouplingKind::Gear => wrap180(ab - k * aa) / 10.,
            CouplingKind::RackPinion => lb - k * aa / 360.,
            CouplingKind::Cable => lb - k * la,
        });
        per.push((c.id, s, r.len()));
    }
}

/// Um quadro da simulação: tempo e posições dos componentes.
#[derive(Clone, Debug)]
pub struct Frame {
    pub t: f64,
    pub placements: HashMap<Id, Placement>,
    pub ok: bool,
}

/// Simula de `t0` a `t1` em `steps` passos (os comandos podem usar `t`). A montagem volta ao
/// estado inicial no fim.
pub fn simulate(a: &mut Assembly, t0: f64, t1: f64, steps: usize) -> Vec<Frame> {
    let keep = a.clone();
    let mut out = vec![];
    for k in 0..=steps.max(1) {
        let t = t0 + (t1 - t0) * k as f64 / steps.max(1) as f64;
        let mut vars = HashMap::new();
        vars.insert("t".to_string(), t);
        let rep = crate::solve::solve_with(a, &vars);
        out.push(Frame { t, placements: a.nodes.iter().map(|n| (n.id, n.placement)).collect(), ok: rep.converged });
    }
    *a = keep;
    out
}

/// Velocidade e aceleração (diferenças finitas) de um ponto local do componente ao longo dos quadros.
pub fn kinematics_of(frames: &[Frame], comp: Id, local: V3) -> Vec<(f64, V3, f64, f64)> {
    let pos: Vec<(f64, V3)> = frames.iter().filter_map(|f| f.placements.get(&comp).map(|p| (f.t, p.point(local)))).collect();
    let mut out = vec![];
    for i in 0..pos.len() {
        let v = |i: usize, j: usize| {
            let dt = (pos[j].0 - pos[i].0).max(1e-12);
            let d = sub(pos[j].1, pos[i].1);
            [d[0] / dt, d[1] / dt, d[2] / dt]
        };
        let (i0, i1) = (i.saturating_sub(1), (i + 1).min(pos.len() - 1));
        let vel = if i1 > i0 { v(i0, i1) } else { [0.; 3] };
        let acc = if i > 0 && i + 1 < pos.len() {
            let (va, vb) = (v(i - 1, i), v(i, i + 1));
            let dt = ((pos[i + 1].0 - pos[i - 1].0) / 2.).max(1e-12);
            norm(sub(vb, va)) / dt
        } else {
            0.
        };
        out.push((pos[i].0, pos[i].1, norm(vel), acc));
    }
    out
}

/// Análise do mecanismo: graus de liberdade sem os comandos e com eles.
pub fn analyze(a: &Assembly) -> (usize, usize) {
    (crate::solve::dof_with(a, false), crate::solve::dof_with(a, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solve::solve_with;

    fn manivela() -> (Assembly, Id, Id) {
        let mut a = Assembly::new("manivela");
        let base = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        let braco = a.add_component("Braço", NodeKind::Part { path: "c".into() }, Placement::default(), None);
        let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let j = a.add_joint(JointKind::Revolute, JRef { comp: base, frame: f }, JRef { comp: braco, frame: f });
        (a, braco, j)
    }

    #[test]
    fn revolucao_com_comando_de_angulo() {
        let (mut a, braco, j) = manivela();
        let (sem, _) = analyze(&a);
        assert_eq!(sem, 1, "revolução deixa 1 GDL");
        a.joints.iter_mut().find(|x| x.id == j).unwrap().drive_angle = Some("90".into());
        let rep = solve_with(&mut a, &HashMap::new());
        assert!(rep.converged, "{rep:?}");
        assert_eq!(analyze(&a).1, 0, "com o comando: 0 GDL");
        let x = a.node(braco).unwrap().placement.dir([1., 0., 0.]);
        assert!(norm(sub(x, [0., 1., 0.])) < 1e-6, "{x:?}");
    }

    #[test]
    fn lei_no_tempo_e_velocidade() {
        let (mut a, braco, j) = manivela();
        // gira 90° por segundo
        a.joints.iter_mut().find(|x| x.id == j).unwrap().drive_angle = Some("90*t".into());
        let frames = simulate(&mut a, 0., 1., 10);
        assert!(frames.iter().all(|f| f.ok));
        let k = kinematics_of(&frames, braco, [10., 0., 0.]);
        // ponto a 10 mm do eixo: v = ω r = (π/2) × 10
        let v = k[5].2;
        assert!((v - std::f64::consts::FRAC_PI_2 * 10.).abs() < 0.3, "{v}");
    }

    #[test]
    fn parafuso_e_engrenagem() {
        let mut a = Assembly::new("par");
        let base = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        let porca = a.add_component("Porca", NodeKind::Part { path: "p".into() }, Placement::default(), None);
        let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let js = a.add_joint(JointKind::Screw { pitch: "2".into() }, JRef { comp: base, frame: f }, JRef { comp: porca, frame: f });
        a.joints.iter_mut().find(|x| x.id == js).unwrap().drive_angle = Some("90".into());
        assert!(solve_with(&mut a, &HashMap::new()).converged);
        assert!((a.node(porca).unwrap().placement.origin[2] - 0.5).abs() < 1e-6, "um quarto de volta × passo 2 = 0,5 mm");
        // engrenagem 1:−2 entre duas revoluções
        let mut g = Assembly::new("eng");
        let base = g.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        let e1 = g.add_component("E1", NodeKind::Part { path: "e1".into() }, Placement::default(), None);
        let e2 = g.add_component("E2", NodeKind::Part { path: "e2".into() }, Placement { origin: [30., 0., 0.], ..Default::default() }, None);
        let j1 = g.add_joint(JointKind::Revolute, JRef { comp: base, frame: f }, JRef { comp: e1, frame: f });
        let f2 = JFrame::from_axis([30., 0., 0.], [0., 0., 1.]);
        let j2 = g.add_joint(JointKind::Revolute, JRef { comp: base, frame: f2 }, JRef { comp: e2, frame: JFrame::from_axis([0.; 3], [0., 0., 1.]) });
        g.add_coupling(CouplingKind::Gear, j1, j2, "-0.5");
        g.joints.iter_mut().find(|x| x.id == j1).unwrap().drive_angle = Some("60".into());
        assert!(solve_with(&mut g, &HashMap::new()).converged);
        let pl: HashMap<Id, Placement> = g.nodes.iter().map(|n| (n.id, n.placement)).collect();
        let (a2, _) = measure(g.joints.iter().find(|x| x.id == j2).unwrap(), &pl).unwrap();
        assert!((a2 + 30.).abs() < 1e-4, "{a2}");
    }

    #[test]
    fn mecanismo_a_partir_das_restricoes() {
        let mut a = Assembly::new("auto");
        let c1 = a.add_component("A", NodeKind::Part { path: "a".into() }, Placement::default(), None);
        let c2 = a.add_component("B", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        let ax = Geom::Axis { p: [0.; 3], d: [0., 0., 1.] };
        a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: c1, geom: ax, pick: None }, MateRef { comp: c2, geom: ax, pick: None });
        a.add_mate(MateKind::Contact, MateRef { comp: c1, geom: Geom::Plane { p: [0., 0., 5.], n: [0., 0., 1.] }, pick: None }, MateRef { comp: c2, geom: Geom::Plane { p: [0., 0., 5.], n: [0., 0., -1.] }, pick: None });
        let js = a.joints_from_mates();
        assert_eq!(js.len(), 1);
        assert!(a.mates.is_empty());
        assert_eq!(a.joints[0].kind, JointKind::Revolute);
    }
}
