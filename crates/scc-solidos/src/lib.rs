//! Módulo Modelagem Sólida: executa a linha de tempo do documento sobre o kernel.
//!
//! Regras (RF-NUC-008, RF-NUC-024): um comando com erro **não** é apagado nem invalida o resto;
//! o corpo continua do estado anterior e o comando fica marcado com a mensagem do erro.

pub mod hardware;
pub mod profile;

use profile::{ev, eval_sketch, to3d, EvalSketch, Vars};
use scc_core::*;
use scc_kernel::{Bool, Frame, HoleShape, KernelError, Mesh, MassProps, Shape};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Ok,
    Warning(String),
    Error(String),
    /// Depois do marcador de rollback ou suprimido.
    Inactive,
}

impl Status {
    pub fn message(&self) -> Option<&str> {
        match self {
            Status::Warning(m) | Status::Error(m) => Some(m),
            _ => None,
        }
    }
}

pub struct BodyResult {
    pub collector: Id,
    pub color: [u8; 3],
    pub mesh: Mesh,
    pub mass: MassProps,
    pub faces: u32,
    /// Comando que gerou cada face (mesma ordem de `mesh.face_ranges`).
    pub face_owner: Vec<Id>,
}

fn tags_for(owners: &[Id]) -> Vec<i32> {
    (0..owners.len() as i32).collect()
}

/// Converte as etiquetas devolvidas pelo kernel em donos; -1 = face nova do comando `fid`.
fn remap(owners: &[Id], out: &[i32], fid: Id) -> Vec<Id> {
    out.iter().map(|t| if *t >= 0 { owners.get(*t as usize).copied().unwrap_or(fid) } else { fid }).collect()
}

/// Geometria de exibição de um sketch (segmentos 3D).
pub struct SketchViz {
    pub feature: Id,
    /// plano do sketch (para alinhar a câmera)
    pub frame: Frame,
    pub lines: Vec<[f32; 3]>,
    pub construction: Vec<[f32; 3]>,
    pub points: Vec<[f32; 3]>,
}

/// Plano exibível (Origem ou Referências).
pub struct PlaneViz {
    pub id: Option<Id>,
    pub name: String,
    pub frame: Frame,
}

pub struct RecomputeResult {
    pub bodies: Vec<BodyResult>,
    pub sketches: Vec<SketchViz>,
    pub planes: Vec<PlaneViz>,
    pub status: HashMap<Id, Status>,
    pub params: HashMap<String, f64>,
    pub param_error: Option<String>,
    pub elapsed_ms: f64,
    pub kernel: String,
}

pub fn origin_frame(p: OriginPlane) -> Frame {
    match p {
        OriginPlane::XY => Frame { origin: [0.; 3], x: [1., 0., 0.], normal: [0., 0., 1.] },
        OriginPlane::XZ => Frame { origin: [0.; 3], x: [1., 0., 0.], normal: [0., -1., 0.] },
        OriginPlane::YZ => Frame { origin: [0.; 3], x: [0., 1., 0.], normal: [1., 0., 0.] },
    }
}

pub fn origin_axis(a: OriginAxis) -> [f64; 3] {
    match a {
        OriginAxis::X => [1., 0., 0.],
        OriginAxis::Y => [0., 1., 0.],
        OriginAxis::Z => [0., 0., 1.],
    }
}

struct Ctx<'a> {
    doc: &'a Document,
    vars: Vars,
    bodies: HashMap<Id, (Shape, Vec<Id>)>,
    sketches: HashMap<Id, (Frame, EvalSketch)>,
    /// sólido gerado por cada comando (base para filete/chanfro por comando)
    status: HashMap<Id, Status>,
}

impl Ctx<'_> {
    fn plane(&self, p: &PlaneRef, depth: u32) -> Result<Frame, String> {
        match p {
            PlaneRef::Origin(o) => Ok(origin_frame(*o)),
            PlaneRef::Datum(id) => {
                if depth > 32 {
                    return Err("referências de plano em ciclo".into());
                }
                let r = self.doc.reference(*id).ok_or_else(|| format!("referência perdida (#{}) — reedite ou substitua", id.0))?;
                match &r.kind {
                    RefKind::OffsetPlane { base, offset } => {
                        let b = self.plane(base, depth + 1)?;
                        Ok(b.offset(ev(offset, &self.vars, &format!("{} · deslocamento", r.name))?))
                    }
                }
            }
        }
    }

    fn sketch(&self, id: Id) -> Result<&(Frame, EvalSketch), String> {
        self.sketches.get(&id).ok_or_else(|| {
            match self.doc.feature(id) {
                Some(f) => format!("o sketch '{}' está inativo ou com erro", f.name),
                None => format!("referência perdida: sketch #{} não existe — reedite ou substitua a referência", id.0),
            }
        })
    }

    fn body(&self, target: Id) -> Option<&Shape> {
        self.body_owned(target).map(|(s, _)| s)
    }

    /// Distância "passante": maior que o corpo inteiro.
    fn through(&self, target: Id) -> Result<f64, String> {
        let b = self.body(target).ok_or("'Passante' precisa de um corpo existente no coletor de destino")?;
        let (lo, hi) = b.bbox().ok_or("corpo sem dimensões")?;
        let mut d2 = 0.;
        for i in 0..3 {
            d2 += (hi[i] - lo[i]).powi(2);
        }
        Ok(d2.sqrt() * 2. + 10.)
    }

    fn extent(&self, e: &Extent, target: Id) -> Result<(f64, f64), String> {
        Ok(match e {
            Extent::Blind(d) => (ev(d, &self.vars, "distância")?, 0.),
            Extent::Symmetric(d) => {
                let d = ev(d, &self.vars, "distância")?;
                (d / 2., d / 2.)
            }
            Extent::TwoSides(a, b) => (ev(a, &self.vars, "distância 1")?, ev(b, &self.vars, "distância 2")?),
            Extent::ThroughAll => (self.through(target)?, 0.),
        })
    }

    /// Corpo + dono (comando) de cada face, na ordem das faces do kernel.
    fn body_owned(&self, target: Id) -> Option<&(Shape, Vec<Id>)> {
        self.bodies.get(&target).filter(|(s, _)| !s.is_null())
    }

    /// Aplica a ferramenta ao corpo do coletor; faces novas passam a pertencer ao comando `fid`.
    fn apply(&mut self, target: Id, tool: Shape, op: BoolOp, fid: Id) -> Result<(), String> {
        let k = |e: KernelError| e.0;
        let next = match (self.body_owned(target), op) {
            (None, BoolOp::Add) => {
                let n = tool.face_count() as usize;
                (tool, vec![fid; n])
            }
            (None, _) => return Err("não há corpo no coletor de destino para esta operação".into()),
            (Some((b, owners)), op) => {
                let kop = match op {
                    BoolOp::Add => Bool::Fuse,
                    BoolOp::Cut => Bool::Cut,
                    BoolOp::Intersect => Bool::Common,
                };
                let (s, out) = b.boolean_tagged(&tool, kop, &tags_for(owners)).map_err(k)?;
                let own = remap(owners, &out, fid);
                (s, own)
            }
        };
        self.bodies.insert(target, next);
        Ok(())
    }

    fn run(&mut self, f: &Feature) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        match &f.kind {
            FeatureKind::Sketch(s) => {
                let frame = self.plane(&s.plane, 0)?;
                let es = eval_sketch(&s.entities, &self.vars)?;
                let warn = (!es.warnings.is_empty()).then(|| es.warnings.join("; "));
                self.sketches.insert(f.id, (frame, es));
                Ok(warn)
            }
            FeatureKind::Extrude(e) => {
                let (frame, sk) = self.sketch(e.sketch)?;
                if sk.loops.is_empty() {
                    return Err("o sketch não tem perfis fechados".into());
                }
                let face = Shape::face(frame, &sk.loops).map_err(k)?;
                // recorte entra no material (contra a normal); "inverter" troca o sentido
                let into = (e.op == BoolOp::Cut) != e.reverse;
                let n = frame.normal;
                let dir = if into { n.map(|x| -x) } else { n };
                let (front, back) = self.extent(&e.extent, f.target)?;
                let tool = face.extrude(dir, front, back).map_err(k)?;
                self.apply(f.target, tool, e.op, f.id)?;
                Ok(None)
            }
            FeatureKind::Revolve(r) => {
                let (frame, sk) = self.sketch(r.sketch)?;
                if sk.loops.is_empty() {
                    return Err("o sketch não tem perfis fechados".into());
                }
                let (origin, dir) = match r.axis {
                    AxisRef::Origin(a) => ([0.; 3], origin_axis(a)),
                    AxisRef::SketchLine { sketch, entity } => {
                        let (af, ask) = self.sketch(sketch)?;
                        let (a, b) = ask.lines.get(&entity).ok_or("linha do eixo não existe mais — reedite o eixo")?;
                        let (pa, pb) = (af.point(a[0], a[1]), af.point(b[0], b[1]));
                        (pa, [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]])
                    }
                };
                let face = Shape::face(frame, &sk.loops).map_err(k)?;
                let ang = ev(&r.angle, &self.vars, "ângulo")?;
                let start = if r.symmetric { -ang / 2. } else { 0. };
                let tool = face.revolve(origin, dir, ang, start).map_err(k)?;
                self.apply(f.target, tool, r.op, f.id)?;
                Ok(None)
            }
            FeatureKind::Hole(h) => {
                let (frame, sk) = self.sketch(h.sketch)?;
                if sk.points.is_empty() {
                    return Err("o sketch do furo precisa de pelo menos um ponto (centro)".into());
                }
                let (frame, pts) = (*frame, sk.points.clone());
                let d = ev(&h.diameter, &self.vars, "diâmetro")?;
                let (depth, _) = self.extent(&h.depth, f.target)?;
                let shape = match &h.kind {
                    HoleKind::Simples => HoleShape::Simple,
                    HoleKind::Rebaixado { diameter, depth } => HoleShape::Counterbore {
                        diameter: ev(diameter, &self.vars, "diâmetro do rebaixo")?,
                        depth: ev(depth, &self.vars, "profundidade do rebaixo")?,
                    },
                    HoleKind::Escareado { diameter, angle } => HoleShape::Countersink {
                        diameter: ev(diameter, &self.vars, "diâmetro do escareado")?,
                        angle: ev(angle, &self.vars, "ângulo do escareado")?,
                    },
                };
                let dir = frame.normal.map(|x| -x);
                let mut tool = Shape::empty();
                for p in pts {
                    let t = Shape::hole_tool(frame.point(p[0], p[1]), dir, d, depth, shape).map_err(k)?;
                    tool = if tool.is_null() { t } else { tool.compound(&t) };
                }
                self.apply(f.target, tool, BoolOp::Cut, f.id)?;
                Ok(h.thread.as_ref().map(|t| format!("rosca cosmética {t}")))
            }
            FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => {
                let v = ev(&o.value, &self.vars, "valor")?;
                let (b, owners) = self.body_owned(f.target).ok_or("não há corpo para aplicar")?;
                let tags = tags_for(owners);
                let r = if matches!(f.kind, FeatureKind::Fillet(_)) { b.fillet_tagged(v, &tags) } else { b.chamfer_tagged(v, &tags) };
                let (s, out) = r.map_err(k)?;
                let own = remap(owners, &out, f.id);
                self.bodies.insert(f.target, (s, own));
                Ok(match o.edges {
                    EdgeSel::AllOfBody => None,
                    EdgeSel::CreatedBy(_) => Some("seleção por comando ainda não implementada: aplicado a todas as arestas".into()),
                })
            }
        }
    }
}

/// Recalcula o documento inteiro. Roda fora da thread da interface (RNF-001).
pub fn recompute(doc: &Document) -> RecomputeResult {
    let t0 = Instant::now();
    let (vars, param_error) = match doc.eval_params() {
        Ok(v) => (v, None),
        Err(e) => (HashMap::new(), Some(e.to_string())),
    };
    let mut cx = Ctx { doc, vars, bodies: HashMap::new(), sketches: HashMap::new(), status: HashMap::new() };

    for (i, f) in doc.features.iter().enumerate() {
        if i >= doc.rollback || f.suppressed {
            cx.status.insert(f.id, Status::Inactive);
            continue;
        }
        let st = match cx.run(f) {
            Ok(None) => Status::Ok,
            Ok(Some(w)) => Status::Warning(w),
            Err(e) => Status::Error(e),
        };
        cx.status.insert(f.id, st);
    }

    // planos: Origem + Referências
    let mut planes = vec![];
    if doc.origin_visible {
        for (p, n) in [(OriginPlane::XY, "XY"), (OriginPlane::XZ, "XZ"), (OriginPlane::YZ, "YZ")] {
            planes.push(PlaneViz { id: None, name: n.into(), frame: origin_frame(p) });
        }
    }
    for r in doc.references.iter().filter(|r| r.visible) {
        if let Ok(fr) = cx.plane(&PlaneRef::Datum(r.id), 0) {
            planes.push(PlaneViz { id: Some(r.id), name: r.name.clone(), frame: fr });
        }
    }

    // sketches para exibição
    let mut sketches = vec![];
    for (id, (fr, es)) in &cx.sketches {
        let mut v = SketchViz { feature: *id, frame: *fr, lines: vec![], construction: vec![], points: vec![] };
        for (cons, pts) in &es.viz {
            let dst = if *cons { &mut v.construction } else { &mut v.lines };
            for w in pts.windows(2) {
                dst.push(to3d(fr, w[0]));
                dst.push(to3d(fr, w[1]));
            }
        }
        v.points = es.points.iter().map(|p| to3d(fr, *p)).collect();
        sketches.push(v);
    }

    // malhas dos corpos
    let mut bodies = vec![];
    for c in &doc.collectors {
        let Some((s, owners)) = cx.body_owned(c.id) else { continue };
        let defl = s
            .bbox()
            .map(|(a, b)| ((0..3).map(|i| (b[i] - a[i]).powi(2)).sum::<f64>().sqrt() * 0.0015).clamp(0.005, 1.0))
            .unwrap_or(0.1);
        match s.mesh(defl) {
            Ok(mesh) => bodies.push(BodyResult {
                collector: c.id,
                color: c.color,
                mass: s.mass(),
                faces: s.face_count(),
                face_owner: owners.clone(),
                mesh,
            }),
            Err(e) => {
                cx.status.insert(c.id, Status::Error(e.0));
            }
        }
    }

    RecomputeResult {
        bodies,
        sketches,
        planes,
        status: cx.status,
        params: cx.vars,
        param_error,
        elapsed_ms: t0.elapsed().as_secs_f64() * 1000.,
        kernel: scc_kernel::version(),
    }
}

/// Exporta o corpo de um coletor (ou de todos) após recalcular.
pub fn export(doc: &Document, collector: Option<Id>, path: &str) -> Result<(), String> {
    let vars = doc.eval_params().map_err(|e| e.to_string())?;
    let mut cx = Ctx { doc, vars, bodies: HashMap::new(), sketches: HashMap::new(), status: HashMap::new() };
    for f in doc.active_features().map(|(_, f)| f) {
        let _ = cx.run(f);
    }
    let mut all = Shape::empty();
    for c in &doc.collectors {
        if collector.is_some_and(|x| x != c.id) {
            continue;
        }
        if let Some(b) = cx.body(c.id) {
            all = if all.is_null() { b.clone() } else { all.compound(b) };
        }
    }
    if all.is_null() {
        return Err("nada para exportar".into());
    }
    let lower = path.to_lowercase();
    let r = if lower.ends_with(".stl") { all.export_stl(path) } else { all.export_step(path) };
    r.map_err(|e| e.0)
}

/// Exemplo usado pelo comando "Nova peça de exemplo" e pelos testes:
/// placa parametrizada com rasgo, furos rebaixados M6 e filete.
pub fn example_document() -> Document {
    let s = |v: &str| v.to_string();
    let mut d = Document::new("Placa de exemplo");
    d.props.pn = s("SCC-0001");
    d.props.description = s("Placa base");
    d.add_param("L", "120", Unit::Comprimento).unwrap();
    d.add_param("W", "L*0.6", Unit::Comprimento).unwrap();
    d.add_param("esp", "10", Unit::Comprimento).unwrap();
    d.add_param("margem", "12", Unit::Comprimento).unwrap();

    let ent = |d: &mut Document, geom| SketchEntity { id: d.alloc(), construction: false, geom };
    let r = ent(&mut d, SketchGeom::Rect { cx: s("0"), cy: s("0"), w: s("L"), h: s("W") });
    let slot = ent(&mut d, SketchGeom::Slot { cx: s("0"), cy: s("0"), length: s("L/3"), width: s("12"), angle: s("0") });
    let base = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), entities: vec![r, slot] }));
    d.feature_mut(base).unwrap().name = s("Sketch base");
    let ex = d.add_feature(FeatureKind::Extrude(Extrude { sketch: base, op: BoolOp::Add, extent: Extent::Blind(s("esp")), reverse: false }));
    d.feature_mut(ex).unwrap().name = s("Placa");

    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "esp");
    d.references.last_mut().unwrap().name = s("PlanoTopo");
    let mut pts = vec![];
    for (x, y) in [("-L/2+margem", "-W/2+margem"), ("L/2-margem", "-W/2+margem"), ("L/2-margem", "W/2-margem"), ("-L/2+margem", "W/2-margem")] {
        pts.push(ent(&mut d, SketchGeom::Point { x: s(x), y: s(y) }));
    }
    let skf = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), entities: pts }));
    d.feature_mut(skf).unwrap().name = s("Centros dos furos");
    // sketches consumidos por comandos ficam ocultos, como nos CADs usuais
    d.feature_mut(base).unwrap().hidden = true;
    d.feature_mut(skf).unwrap().hidden = true;
    let m6 = hardware::metric("M6").unwrap();
    let furo = d.add_feature(FeatureKind::Hole(Hole {
        sketch: skf,
        kind: HoleKind::Rebaixado { diameter: m6.cbore_diameter.to_string(), depth: m6.cbore_depth.to_string() },
        diameter: m6.clearance[1].to_string(),
        depth: Extent::ThroughAll,
        thread: None,
        standard: Some(s("M6 passagem normal")),
    }));
    d.feature_mut(furo).unwrap().name = s("Furos M6");
    let fol = d.add_folder("Acabamentos");
    let fil = d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("1"), edges: EdgeSel::AllOfBody }));
    d.feature_mut(fil).unwrap().folder = Some(fol);
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exemplo_recalcula_sem_erros() {
        let d = example_document();
        let r = recompute(&d);
        assert!(r.param_error.is_none());
        if !scc_kernel::AVAILABLE {
            return;
        }
        for f in &d.features {
            assert_eq!(r.status[&f.id], Status::Ok, "{}: {:?}", f.name, r.status[&f.id]);
        }
        assert_eq!(r.bodies.len(), 1);
        let b = &r.bodies[0];
        assert!(b.mass.volume > 0.);
        // cada face tem dono; furo, filete e placa aparecem como donos
        assert_eq!(b.face_owner.len(), b.mesh.face_ranges.len());
        let owners: std::collections::HashSet<Id> = b.face_owner.iter().copied().collect();
        for name in ["Placa", "Furos M6", "Filete1"] {
            let id = d.features.iter().find(|f| f.name == name).unwrap().id;
            assert!(owners.contains(&id), "{name} sem faces");
        }
    }

    #[test]
    fn referencia_perdida_nao_apaga_comando() {
        let mut d = example_document();
        let sk = d.features[0].id;
        d.delete(sk, false);
        let r = recompute(&d);
        let ex = &d.features[0];
        assert!(matches!(&r.status[&ex.id], Status::Error(m) if m.contains("referência perdida")));
    }

    #[test]
    fn parametro_muda_volume() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        let mut d = example_document();
        let v1 = recompute(&d).bodies[0].mass.volume;
        d.params[2].expr = "20".into(); // esp
        let v2 = recompute(&d).bodies[0].mass.volume;
        assert!(v2 > v1 * 1.8);
    }
}
