//! Vista composta da montagem: as malhas de cada componente, já posicionadas, num resultado único
//! (um "corpo" por componente) — a viewport, a seleção e a cena da peça funcionam sobre ele sem
//! mudanças. Também verifica interferências entre componentes.

use crate::load::{full_path, PartCache};
use crate::*;
use fenkra_core::{BodyCollector, Document, Finish};
use fenkra_kernel::{Bool, Mesh, Shape, Xform};
use fenkra_solidos::{BodyResult, RecomputeResult};
use std::path::Path;

/// Malha transformada pela posição do componente.
pub fn transformed(m: &Mesh, p: &Placement) -> Mesh {
    let pt = |v: &[f32]| p.point([v[0] as f64, v[1] as f64, v[2] as f64]).map(|x| x as f32);
    let dir = |v: &[f32]| p.dir([v[0] as f64, v[1] as f64, v[2] as f64]).map(|x| x as f32);
    Mesh {
        positions: m.positions.chunks_exact(3).flat_map(pt).collect(),
        normals: m.normals.chunks_exact(3).flat_map(dir).collect(),
        indices: m.indices.clone(),
        edges: m.edges.chunks_exact(3).flat_map(pt).collect(),
        face_ranges: m.face_ranges.clone(),
        edge_ranges: m.edge_ranges.clone(),
        edge_mids: m.edge_mids.chunks_exact(3).flat_map(pt).collect(),
        edge_smooth: m.edge_smooth.clone(),
        vertices: m.vertices.chunks_exact(3).flat_map(pt).collect(),
        edge_faces: m.edge_faces.clone(),
        face_info: m
            .face_info
            .iter()
            .map(|f| fenkra_kernel::FaceInfo { dir: p.dir(f.dir), point: p.point(f.point), ..*f })
            .collect(),
    }
}

/// Junta malhas (vários corpos da peça viram um componente só).
fn append(dst: &mut Mesh, src: &Mesh) {
    let base = (dst.positions.len() / 3) as u32;
    let fbase = dst.face_ranges.len() as u32;
    let ibase = dst.indices.len() as u32;
    let ebase = (dst.edges.len() / 6) as u32;
    dst.positions.extend(&src.positions);
    dst.normals.extend(&src.normals);
    dst.indices.extend(src.indices.iter().map(|i| i + base));
    dst.face_ranges.extend(src.face_ranges.iter().map(|f| f + ibase));
    dst.edge_ranges.extend(src.edge_ranges.iter().map(|e| e + ebase));
    dst.edges.extend(&src.edges);
    dst.edge_mids.extend(&src.edge_mids);
    dst.edge_smooth.extend(&src.edge_smooth);
    dst.vertices.extend(&src.vertices);
    dst.edge_faces.extend(src.edge_faces.iter().map(|p| p.map(|f| if f == fenkra_kernel::NO_FACE { f } else { f + fbase })));
    dst.face_info.extend(&src.face_info);
}

/// O nó e todos os seus pais estão visíveis?
pub fn shown(a: &Assembly, id: Id) -> bool {
    let mut cur = Some(id);
    let mut guard = 0;
    while let Some(c) = cur {
        let Some(n) = a.node(c) else { return true };
        if !n.visible {
            return false;
        }
        cur = n.parent;
        guard += 1;
        if guard > 64 {
            break;
        }
    }
    true
}

/// Malha de um componente (peça, ou subconjunto inteiro) no sistema dele.
fn comp_mesh(kind: &NodeKind, dir: Option<&Path>, cache: &mut PartCache, with_shape: bool, depth: u32) -> Result<(Mesh, [u8; 3], f64), String> {
    match kind {
        NodeKind::Group => Err("grupo".into()),
        NodeKind::Part { path } => {
            let p = cache.get(&full_path(dir, path), with_shape)?;
            let mut m = Mesh::default();
            let mut vol = 0.;
            for b in &p.result.bodies {
                append(&mut m, &b.mesh);
                vol += b.mass.volume;
            }
            let color = p.result.bodies.first().map(|b| b.color).unwrap_or([180, 190, 200]);
            Ok((m, color, vol))
        }
        NodeKind::SubAssembly { path } => {
            if depth > 16 {
                return Err("subconjuntos em ciclo".into());
            }
            let full = full_path(dir, path);
            let txt = std::fs::read_to_string(&full).map_err(|e| format!("{}: {e}", full.display()))?;
            let sub = Assembly::from_json(&txt)?;
            let sub_dir = full.parent().map(|x| x.to_path_buf());
            let mut m = Mesh::default();
            let mut vol = 0.;
            for n in sub.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group) && shown(&sub, n.id)) {
                if let Ok((cm, _, v)) = comp_mesh(&n.kind, sub_dir.as_deref(), cache, false, depth + 1) {
                    append(&mut m, &transformed(&cm, &n.placement));
                    vol += v;
                }
            }
            Ok((m, [150, 170, 210], vol))
        }
    }
}

/// Vista composta: documento "sombra" (um coletor por componente, com cor e visibilidade) e o
/// resultado com as malhas posicionadas. Erros de carregamento vão para `errors`.
pub fn compose(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, errors: &mut Vec<(Id, String)>) -> (Document, RecomputeResult) {
    let mut doc = Document::new(&a.name);
    doc.collectors.clear();
    doc.origin_visible = true;
    let mut bodies = vec![];
    let changed = featured(a);
    for n in a.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)) {
        let with_shape = n.mode == LoadMode::Editavel;
        let visible = shown(a, n.id);
        // carregamento seletivo: componentes ocultos no modo visualização nem são lidos
        if !visible && n.mode == LoadMode::Visualizacao {
            continue;
        }
        let cut = (visible && changed.contains(&n.id)).then(|| placed_shape(a, n.id, dir, cache)).flatten();
        if let Some(sh) = cut {
            // já no mundo: a malha da forma recortada não leva a posição de novo
            let size = sh.bbox().map(|(l, h)| norm(sub(h, l))).unwrap_or(100.);
            if let Ok(m) = sh.mesh((size * 1e-3).max(0.01)) {
                let color = n.color.unwrap_or_else(|| comp_mesh(&n.kind, dir, cache, false, 0).map(|x| x.1).unwrap_or([180, 190, 200]));
                let label = if n.number > 0 { format!("{} · {}", n.number, n.name) } else { n.name.clone() };
                doc.collectors.push(BodyCollector { id: n.id, name: label, color, visible, finish: Finish::default(), texture: None, marker: None, published: false });
                let nf = m.face_ranges.len();
                let vol = sh.mass().volume;
                bodies.push(BodyResult { collector: n.id, color, mesh: m, mass: fenkra_kernel::MassProps { volume: vol, area: 0., center: n.placement.origin }, faces: nf as u32, face_owner: vec![n.id; nf], face_role: vec![-1; nf] });
                continue;
            }
        }
        match comp_mesh(&n.kind, dir, cache, with_shape, 0) {
            Ok((m, color, vol)) => {
                let color = n.color.unwrap_or(color);
                let label = if n.number > 0 { format!("{} · {}", n.number, n.name) } else { n.name.clone() };
                doc.collectors.push(BodyCollector { id: n.id, name: label, color, visible, finish: Finish::default(), texture: None, marker: None, published: false });
                let nf = m.face_ranges.len();
                bodies.push(BodyResult {
                    collector: n.id,
                    color,
                    mesh: transformed(&m, &n.placement),
                    mass: fenkra_kernel::MassProps { volume: vol, area: 0., center: n.placement.origin },
                    faces: nf as u32,
                    face_owner: vec![n.id; nf],
                    face_role: vec![-1; nf],
                });
            }
            Err(e) => errors.push((n.id, e)),
        }
    }
    if let Some(c) = doc.collectors.first() {
        doc.active_collector = c.id;
    }
    let r = RecomputeResult {
        bodies,
        promoted: Default::default(),
        sketches: vec![],
        planes: vec![],
        points: vec![],
        curves: vec![],
        surfaces: vec![],
        status: Default::default(),
        params: Default::default(),
        param_error: None,
        elapsed_ms: 0.,
        kernel: String::new(),
        bend_lines: vec![],
    };
    (doc, r)
}

/// Forma do componente posicionada no mundo, com os recursos entre peças que o atingem.
pub fn placed_shape(a: &Assembly, id: Id, dir: Option<&Path>, cache: &mut PartCache) -> Option<Shape> {
    let raw = raw_shape(a, id, dir, cache)?;
    Some(apply_features(a, id, raw, dir, cache))
}

/// Componentes que algum recurso entre peças muda.
pub fn featured(a: &Assembly) -> Vec<Id> {
    a.features.iter().filter(|f| !f.suppressed).flat_map(|f| f.targets.iter().copied()).collect()
}

/// Aplica os recursos entre peças (no mundo) à forma do componente `id`.
fn apply_features(a: &Assembly, id: Id, mut sh: Shape, dir: Option<&Path>, cache: &mut PartCache) -> Shape {
    let vars = a.vars();
    let num = |e: &str| fenkra_core::expr::eval_str(e, &vars).ok();
    for f in a.features.iter().filter(|f| !f.suppressed && f.targets.contains(&id)) {
        let tool = match &f.kind {
            AsmFeatKind::Cut { tool, .. } => {
                if *tool == id {
                    continue;
                }
                raw_shape(a, *tool, dir, cache)
            }
            AsmFeatKind::Hole { p, d, dia, depth } => {
                let r = num(dia).unwrap_or(5.) / 2.;
                let big = sh.bbox().map(|(l, h)| norm(sub(h, l)) * 2. + 10.).unwrap_or(1000.);
                let len = num(depth).filter(|v| *v > 0.).unwrap_or(big);
                let d = unit(*d);
                let u = any_perp(d);
                let w = cross(d, u);
                // um pouco antes da face, para cortar limpo
                let o = [0, 1, 2].map(|i| p[i] - d[i] * 0.05);
                let circle: Vec<V3> = (0..64)
                    .map(|k| {
                        let t = k as f64 / 64. * std::f64::consts::TAU;
                        [0, 1, 2].map(|i| o[i] + (u[i] * t.cos() + w[i] * t.sin()) * r)
                    })
                    .collect();
                Shape::poly_prism(&circle, [d[0] * (len + 0.05), d[1] * (len + 0.05), d[2] * (len + 0.05)]).ok()
            }
        };
        if let Some(t) = tool {
            if let Ok(x) = sh.boolean(&t, Bool::Cut) {
                sh = x;
            }
        }
    }
    sh
}

/// O componente encosta (volume comum) em algum outro visível?
pub fn collides(a: &Assembly, id: Id, dir: Option<&Path>, cache: &mut PartCache, tol: f64) -> bool {
    let Some(me) = placed_shape(a, id, dir, cache) else { return false };
    let Some((la, ha)) = me.bbox() else { return false };
    let others: Vec<Id> = a.nodes.iter().filter(|n| n.id != id && matches!(n.kind, NodeKind::Part { .. }) && n.mode == LoadMode::Editavel && shown(a, n.id)).map(|n| n.id).collect();
    for o in others {
        let Some(sh) = placed_shape(a, o, dir, cache) else { continue };
        if let Some((lb, hb)) = sh.bbox() {
            if (0..3).any(|k| ha[k] < lb[k] || hb[k] < la[k]) {
                continue;
            }
        }
        if me.boolean(&sh, Bool::Common).is_ok_and(|c| c.mass().volume > tol) {
            return true;
        }
    }
    false
}

/// Forma do componente posicionada no mundo (peças no modo editável), sem recursos.
fn raw_shape(a: &Assembly, id: Id, dir: Option<&Path>, cache: &mut PartCache) -> Option<Shape> {
    let n = a.node(id)?;
    let NodeKind::Part { path } = &n.kind else { return None };
    let sh = cache.get(&full_path(dir, path), true).ok()?.shape.clone()?;
    let p = n.placement;
    // rotação (eixo-ângulo da matriz) + translação
    let r = &p.rot;
    let tr = r[0][0] + r[1][1] + r[2][2];
    let ang = ((tr - 1.) / 2.).clamp(-1., 1.).acos();
    let mut out = sh;
    if ang.abs() > 1e-12 {
        let axis = if (std::f64::consts::PI - ang).abs() < 1e-6 {
            // 180°: eixo pela coluna dominante de (R + I)
            let k = (0..3).max_by(|i, j| r[*i][*i].total_cmp(&r[*j][*j])).unwrap();
            unit([r[k][0] + if k == 0 { 1. } else { 0. }, r[k][1] + if k == 1 { 1. } else { 0. }, r[k][2] + if k == 2 { 1. } else { 0. }])
        } else {
            unit([r[1][2] - r[2][1], r[2][0] - r[0][2], r[0][1] - r[1][0]])
        };
        out = out.transformed(Xform::Rotate([0.; 3], axis, ang.to_degrees())).ok()?;
    }
    out.transformed(Xform::Translate(p.origin)).ok()
}

/// Interferências: pares de componentes cujo volume comum passa de `tol` (mm³).
pub fn interferences(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, tol: f64) -> Vec<(Id, Id, f64)> {
    let ids: Vec<Id> = a.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Part { .. }) && n.mode == LoadMode::Editavel && shown(a, n.id)).map(|n| n.id).collect();
    let shapes: Vec<(Id, Shape)> = ids.iter().filter_map(|id| placed_shape(a, *id, dir, cache).map(|s| (*id, s))).collect();
    let mut out = vec![];
    for i in 0..shapes.len() {
        for j in i + 1..shapes.len() {
            let (ba, bb) = (shapes[i].1.bbox(), shapes[j].1.bbox());
            if let (Some((la, ha)), Some((lb, hb))) = (ba, bb) {
                if (0..3).any(|k| ha[k] < lb[k] || hb[k] < la[k]) {
                    continue;
                }
            }
            if let Ok(c) = shapes[i].1.boolean(&shapes[j].1, Bool::Common) {
                let v = c.mass().volume;
                if v > tol {
                    out.push((shapes[i].0, shapes[j].0, v));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn montagem_de_duas_placas_com_interferencia() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = std::env::temp_dir().join("fenkra_montagem_teste");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
        let mut a = Assembly::new("Conjunto");
        let p1 = a.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
        let p2 = a.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement { origin: [5., 0., 0.], ..Default::default() }, None);
        let mut cache = PartCache::default();
        let mut errs = vec![];
        let (doc, r) = compose(&a, Some(&dir), &mut cache, &mut errs);
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!(r.bodies.len(), 2);
        assert_eq!(doc.collectors.len(), 2);
        // a segunda placa está deslocada 5 mm em X
        let mx = |m: &Mesh| m.positions.chunks_exact(3).map(|p| p[0]).fold(f32::MIN, f32::max);
        assert!((mx(&r.bodies[1].mesh) - mx(&r.bodies[0].mesh) - 5.).abs() < 1e-3);
        // sobrepostas → interferência; separadas → nenhuma
        assert_eq!(interferences(&a, Some(&dir), &mut cache, 1.).len(), 1);
        a.node_mut(p2).unwrap().placement.origin = [500., 0., 0.];
        assert!(interferences(&a, Some(&dir), &mut cache, 1.).is_empty());
        // restrição por face clicada: topo da placa 2 em contato com a base da placa 1
        let sh = cache.get(&dir.join("placa.fkp"), true).unwrap().shape.clone().unwrap();
        let (lo, hi) = sh.bbox().unwrap();
        let c = [(lo[0] + hi[0]) / 2. + 30., (lo[1] + hi[1]) / 2., hi[2]];
        let top = crate::load::geom_at(&sh, c).unwrap();
        let bot = crate::load::geom_at(&sh, [c[0], c[1], lo[2]]).unwrap();
        a.add_mate(MateKind::Contact, MateRef { comp: p1, geom: bot, pick: None }, MateRef { comp: p2, geom: top, pick: None });
        let rep = crate::solve::solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        let z = a.node(p2).unwrap().placement.origin[2];
        assert!((z - (lo[2] - hi[2])).abs() < 1e-6, "placa 2 embaixo da 1: {z}");
        // furo entre peças atravessando as duas placas (associativo: segue as posições)
        a.node_mut(p2).unwrap().placement.origin = [0., 0., z];
        let v0 = placed_shape(&a, p1, Some(&dir), &mut cache).unwrap().mass().volume;
        let top_w = a.node(p1).unwrap().placement.point(c);
        a.add_feature(AsmFeatKind::Hole { p: top_w, d: [0., 0., -1.], dia: "6".into(), depth: String::new() }, vec![p1, p2]);
        let v1 = placed_shape(&a, p1, Some(&dir), &mut cache).unwrap().mass().volume;
        let v2 = placed_shape(&a, p2, Some(&dir), &mut cache).unwrap().mass().volume;
        let cyl = std::f64::consts::PI * 9. * (hi[2] - lo[2]);
        assert!((v0 - v1 - cyl).abs() < cyl * 0.05, "furo na placa 1: {v0} {v1} {cyl}");
        assert!((v0 - v2 - cyl).abs() < cyl * 0.05, "furo na placa 2: {v2}");
        let (_, r2) = compose(&a, Some(&dir), &mut cache, &mut errs);
        assert_eq!(r2.bodies.len(), 2);
        // subconjunto com a placa 2: a restrição que ligava à placa 1 cai
        let (sub, dropped) = a.extract_subassembly(&[p2], "Sub");
        assert_eq!(sub.nodes.len(), 1);
        assert_eq!(dropped, 1);
        assert!(a.mates.is_empty() && a.features.iter().all(|f| !f.targets.contains(&p2)));
        let _ = p1;
    }
}
