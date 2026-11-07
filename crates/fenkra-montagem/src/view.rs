// SPDX-License-Identifier: LGPL-2.1-or-later
//! Vista composta da montagem: as malhas de cada componente, já posicionadas, num resultado único
//! (um "corpo" por componente) — a viewport, a seleção e a cena da peça funcionam sobre ele sem
//! mudanças. Também verifica interferências entre componentes.

use crate::load::{full_path, PartCache};
use crate::lod::{self, Caixa, Instancia, Prototipo};
use crate::*;
use fenkra_core::trabalho::Controle;
use fenkra_core::{BodyCollector, Document, Finish, QualidadeMalha};
use fenkra_kernel::{Bool, Mesh, Shape, Xform};
use fenkra_solidos::{BodyResult, RecomputeResult};
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

/// Malha transformada pela posição do componente.
pub fn transformed(m: &Mesh, p: &Placement) -> Mesh {
    let pt = |v: &[f32]| p.point([v[0] as f64, v[1] as f64, v[2] as f64]).map(|x| x as f32);
    let dir = |v: &[f32]| p.dir([v[0] as f64, v[1] as f64, v[2] as f64]).map(|x| x as f32);
    Mesh {
        positions: m.positions.chunks_exact(3).flat_map(pt).collect(),
        normals: m.normals.chunks_exact(3).flat_map(dir).collect(),
        // o posicionamento é rígido: a curvatura não muda
        curvature: m.curvature.clone(),
        indices: m.indices.clone(),
        edges: m.edges.chunks_exact(3).flat_map(pt).collect(),
        face_ranges: m.face_ranges.clone(),
        edge_ranges: m.edge_ranges.clone(),
        edge_mids: m.edge_mids.chunks_exact(3).flat_map(pt).collect(),
        // a fórmula serve ao sketch da peça, que não usa a malha posicionada na montagem
        edge_geom: vec![],
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

/// Todos os corpos de um resultado numa malha só.
pub(crate) fn malha_unica(r: &RecomputeResult) -> Mesh {
    let mut m = Mesh::default();
    for b in &r.bodies {
        append(&mut m, &b.mesh);
    }
    m
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

/// Protótipo de um componente (peça, ou subconjunto inteiro) no sistema dele, com a cor e o volume.
fn comp_prototipo(kind: &NodeKind, cfg: &str, dir: Option<&Path>, cache: &mut PartCache, with_shape: bool, depth: u32) -> Result<(Arc<Prototipo>, [u8; 3], f64), String> {
    match kind {
        NodeKind::Group => Err("grupo".into()),
        NodeKind::Part { path } => {
            let p = cache.get_config(&full_path(dir, path), with_shape, cfg)?;
            let vol = p.result.bodies.iter().map(|b| b.mass.volume).sum();
            let color = p.result.bodies.first().map(|b| b.color).unwrap_or([180, 190, 200]);
            Ok((p.prototipo.clone(), color, vol))
        }
        NodeKind::SubAssembly { path } => {
            if depth > 16 {
                return Err("subconjuntos em ciclo".into());
            }
            let full = full_path(dir, path);
            let txt = std::fs::read_to_string(&full).map_err(|e| format!("{}: {e}", full.display()))?;
            let sub = Assembly::from_json(&txt)?;
            let sub_dir = full.parent().map(|x| x.to_path_buf());
            let mut filhos = vec![];
            let mut vol = 0.;
            let mut assinatura = std::collections::hash_map::DefaultHasher::new();
            for n in sub.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group) && shown(&sub, n.id)) {
                if let Ok((p, _, v)) = comp_prototipo(&n.kind, sub.config_de(n.id), sub_dir.as_deref(), cache, false, depth + 1) {
                    p.id.hash(&mut assinatura);
                    n.placement.origin.iter().chain(n.placement.rot.iter().flatten()).for_each(|x| x.to_bits().hash(&mut assinatura));
                    filhos.push((p, n.placement));
                    vol += v;
                }
            }
            let assinatura = assinatura.finish();
            let chave = (full, cfg.to_string());
            if let Some((a, p)) = cache.subconjuntos.get(&chave) {
                if *a == assinatura {
                    return Ok((p.clone(), [150, 170, 210], vol));
                }
            }
            let p = prototipo_do_conjunto(&filhos, cache.qualidade());
            cache.subconjuntos.insert(chave, (assinatura, p.clone()));
            Ok((p, [150, 170, 210], vol))
        }
    }
}

/// Junta os filhos de um subconjunto, um nível por vez; filho sem aquele nível entra com o mais grosso.
fn prototipo_do_conjunto(filhos: &[(Arc<Prototipo>, Placement)], q: QualidadeMalha) -> Arc<Prototipo> {
    let qualidades = lod::qualidades(q);
    let total = filhos.iter().map(|(p, _)| p.niveis.len()).max().unwrap_or(1).min(qualidades.len());
    let malhas = (0..total)
        .map(|k| {
            let mut m = Mesh::default();
            for (p, pos) in filhos {
                append(&mut m, &transformed(&p.niveis[k.min(p.mais_grosso())].malha, pos));
            }
            (m, qualidades[k])
        })
        .collect();
    Prototipo::novo(malhas)
}

/// Vista composta com o que a placa de vídeo precisa para desenhar as cópias sem repetir a malha.
pub struct Composta {
    pub doc: Document,
    pub result: RecomputeResult,
    /// componentes que são cópia de um protótipo; recortado por recurso de montagem não entra
    pub instancias: Vec<Instancia>,
    /// caixa de cada corpo de `result`, na mesma ordem: a seleção descarta sem olhar triângulo
    pub caixas: Vec<Caixa>,
}

/// Vista composta: documento "sombra" (um coletor por componente, com cor e visibilidade) e o
/// resultado com as malhas posicionadas. Erros de carregamento vão para `errors`.
pub fn compose(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, errors: &mut Vec<(Id, String)>) -> (Document, RecomputeResult) {
    let c = compor(a, dir, cache, errors, None).expect("sem controle não há cancelamento");
    (c.doc, c.result)
}

/// Compõe a vista. Com `ctl`, conta o progresso por componente e devolve `None` se for cancelada.
pub fn compor(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, errors: &mut Vec<(Id, String)>, ctl: Option<&Controle>) -> Option<Composta> {
    // a geometria da montagem entra como está: planos, pontos e sketches na cena, sem corpos
    let geometria = a.geometria.as_ref().map(fenkra_solidos::recompute);
    let mut doc = a.geometria.clone().unwrap_or_else(|| Document::new(&a.name));
    doc.collectors.clear();
    doc.origin_visible = !a.origem_oculta;
    let mut bodies = vec![];
    let mut instancias = vec![];
    let mut caixas = vec![];
    let changed = featured(a);
    let componentes: Vec<&Node> = a.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).collect();
    if let Some(c) = ctl {
        c.definir_total(componentes.len() as u32);
    }
    for n in componentes {
        if let Some(c) = ctl {
            if c.cancelado() {
                return None;
            }
            c.avancar();
        }
        // a thread de composição não guarda forma: seleção e restrições usam o cache da interface
        let with_shape = n.mode == LoadMode::Editavel && !cache.gera_niveis();
        let visible = shown(a, n.id);
        // carregamento seletivo: componentes ocultos no modo visualização nem são lidos
        if !visible && n.mode == LoadMode::Visualizacao {
            continue;
        }
        let cut = (visible && changed.contains(&n.id)).then(|| placed_shape(a, n.id, dir, cache)).flatten();
        if let Some(sh) = cut {
            // já no mundo: a malha da forma recortada não leva a posição de novo
            if let Ok(m) = fenkra_solidos::tesselar(&sh, &cache.qualidade()) {
                let color = n.color.unwrap_or_else(|| comp_prototipo(&n.kind, a.config_de(n.id), dir, cache, false, 0).map(|x| x.1).unwrap_or([180, 190, 200]));
                let label = if n.number > 0 { format!("{} · {}", n.number, n.name) } else { n.name.clone() };
                doc.collectors.push(BodyCollector { id: n.id, name: label, color, visible, finish: Finish::default(), texture: None, marker: None, published: false, material: String::new() });
                let nf = m.face_ranges.len();
                let vol = sh.mass().volume;
                caixas.push(lod::caixa_de(&m).unwrap_or_default());
                bodies.push(BodyResult { collector: n.id, color, mesh: m, mass: fenkra_kernel::MassProps { volume: vol, area: 0., center: n.placement.origin }, faces: nf as u32, face_owner: vec![n.id; nf], face_role: vec![-1; nf] });
                continue;
            }
        }
        match comp_prototipo(&n.kind, a.config_de(n.id), dir, cache, with_shape, 0) {
            Ok((p, color, vol)) => {
                let color = n.color.unwrap_or(color);
                let label = if n.number > 0 { format!("{} · {}", n.number, n.name) } else { n.name.clone() };
                doc.collectors.push(BodyCollector { id: n.id, name: label, color, visible, finish: Finish::default(), texture: None, marker: None, published: false, material: String::new() });
                let nf = p.fina().face_ranges.len();
                caixas.push(lod::caixa_posta(&p.caixa, &n.placement));
                instancias.push(Instancia { corpo: bodies.len(), prototipo: p.clone(), posicao: n.placement, simplificada: n.simplificada });
                bodies.push(BodyResult {
                    collector: n.id,
                    color,
                    mesh: transformed(p.fina(), &n.placement),
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
    let mut r = resultado_de(bodies);
    if let Some(g) = geometria {
        (r.sketches, r.planes, r.points, r.curves, r.surfaces) = (g.sketches, g.planes, g.points, g.curves, g.surfaces);
    } else {
        // a origem existe mesmo sem geometria própria da montagem
        r.planes = fenkra_solidos::recompute(&Document::new("origem")).planes;
    }
    Some(Composta { doc, result: r, instancias, caixas })
}

/// Resultado só com malhas: a forma como a montagem entrega as vistas compostas.
pub(crate) fn resultado_de(bodies: Vec<BodyResult>) -> RecomputeResult {
    RecomputeResult {
        notes: Default::default(),
        bodies,
        promoted: Default::default(),
        sketches: vec![],
        planes: vec![],
        points: vec![],
        curves: vec![],
        formas: vec![],
        exibicao: Default::default(),
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        surfaces: vec![],
        status: Default::default(),
        params: Default::default(),
        param_error: None,
        elapsed_ms: 0.,
        kernel: String::new(),
        bend_lines: vec![],
        cut_outlines: vec![],
        desatualizados: vec![],
        cancelado: false,
        roscas: vec![],
        previas: vec![],
    }
}

/// Forma do componente posicionada no mundo, com os recursos entre peças que o atingem.
pub fn placed_shape(a: &Assembly, id: Id, dir: Option<&Path>, cache: &mut PartCache) -> Option<Shape> {
    let raw = raw_shape(a, id, dir, cache)?;
    Some(apply_features(a, id, raw, dir, cache))
}

/// Componentes que algum recurso entre peças muda.
pub fn featured(a: &Assembly) -> Vec<Id> {
    // a série gravada já está na peça: nada a refazer na vista
    a.features.iter().filter(|f| !f.suppressed && !matches!(&f.kind, AsmFeatKind::Serie(s) if s.gravar)).flat_map(|f| f.targets.iter().copied()).collect()
}

/// Aplica os recursos entre peças (no mundo) à forma do componente `id`.
fn apply_features(a: &Assembly, id: Id, mut sh: Shape, dir: Option<&Path>, cache: &mut PartCache) -> Shape {
    let vars = a.vars();
    let num = |e: &str| fenkra_core::expr::eval_str(e, &vars).ok();
    for f in a.features.iter().filter(|f| !f.suppressed && f.targets.contains(&id)) {
        if let AsmFeatKind::Serie(s) = &f.kind {
            if !s.gravar {
                sh = serie_na_vista(a, s, sh, dir, cache);
            }
            continue;
        }
        let tool = match &f.kind {
            AsmFeatKind::Cut { tool, .. } => {
                if *tool == id {
                    continue;
                }
                raw_shape(a, *tool, dir, cache)
            }
            AsmFeatKind::Hole { p, d, dia, depth } => {
                let d = unit(*d);
                let len = num(depth).filter(|v| *v > 0.).or_else(|| crate::furos::alcance(&sh, *p, d)).unwrap_or(1000.);
                crate::furos::cilindro(*p, d, num(dia).unwrap_or(5.), len)
            }
            AsmFeatKind::Serie(_) => None,
        };
        if let Some(t) = tool {
            if let Ok(x) = sh.boolean(&t, Bool::Cut) {
                sh = x;
            }
        }
    }
    sh
}

/// Série só na montagem: cilindros exatos cortam a forma exibida do participante.
fn serie_na_vista(a: &Assembly, s: &SerieFuros, mut sh: Shape, dir: Option<&Path>, cache: &mut PartCache) -> Shape {
    let vars = a.vars();
    let num = |e: &str| fenkra_core::expr::eval_str(e, &vars).ok().filter(|v| *v > 0.);
    let Some(dia) = num(&s.dia) else { return sh };
    for (p, d) in crate::furos::eixos(a, s, dir, cache).unwrap_or_default() {
        let Some(len) = num(&s.depth).or_else(|| crate::furos::alcance(&sh, p, d)) else { continue };
        if let Some(t) = crate::furos::cilindro(p, d, dia, len) {
            if let Ok(x) = sh.boolean(&t, Bool::Cut) {
                sh = x;
            }
        }
    }
    sh
}

/// Centro da caixa envolvente das malhas. A média dos vértices puxa o centro para o lado com a malha
/// mais fina: um furo cheio de triângulos o tiraria do meio da peça.
pub fn centro_caixa<'a>(malhas: impl Iterator<Item = &'a Mesh>) -> Option<V3> {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    let mut algum = false;
    for m in malhas {
        for p in m.positions.chunks_exact(3) {
            algum = true;
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
    }
    algum.then(|| [0, 1, 2].map(|k| (lo[k] as f64 + hi[k] as f64) / 2.))
}

/// O componente encosta (volume comum) em algum outro visível?
pub fn collides(a: &Assembly, id: Id, dir: Option<&Path>, cache: &mut PartCache, tol: f64) -> bool {
    collides_com(a, id, dir, cache, tol, None)
}

/// Como `collides`, olhando só as peças de `alvos`, quando houver essa lista.
pub fn collides_com(a: &Assembly, id: Id, dir: Option<&Path>, cache: &mut PartCache, tol: f64, alvos: Option<&[Id]>) -> bool {
    let Some(me) = placed_shape(a, id, dir, cache) else { return false };
    let Some((la, ha)) = me.bbox() else { return false };
    let others: Vec<Id> = a.nodes.iter().filter(|n| n.id != id && matches!(n.kind, NodeKind::Part { .. }) && n.mode == LoadMode::Editavel && shown(a, n.id) && alvos.is_none_or(|l| l.contains(&n.id))).map(|n| n.id).collect();
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
pub(crate) fn raw_shape(a: &Assembly, id: Id, dir: Option<&Path>, cache: &mut PartCache) -> Option<Shape> {
    let n = a.node(id)?;
    let NodeKind::Part { path } = &n.kind else { return None };
    let sh = cache.get(&full_path(dir, path), true).ok()?.shape.clone()?;
    posicionado(sh, &n.placement)
}

/// Forma levada à posição `p`: rotação (eixo-ângulo da matriz) e translação.
pub fn posicionado(sh: Shape, p: &Placement) -> Option<Shape> {
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
    fn centro_e_o_da_caixa_e_nao_a_media_dos_vertices() {
        // nove vértices amontoados perto de x = 0 e um só em x = 100
        let mut m = Mesh::default();
        for i in 0..9 {
            m.positions.extend([i as f32 * 0.1, 0., 0.]);
        }
        m.positions.extend([100., 40., 10.]);
        assert_eq!(centro_caixa([&m].into_iter()), Some([50., 20., 5.]));
        assert_eq!(centro_caixa(std::iter::empty()), None);
    }

    #[test]
    fn origem_aparece_mesmo_sem_geometria_e_oculta_pela_opcao() {
        let mut a = Assembly::new("m");
        let (d, r) = compose(&a, None, &mut PartCache::default(), &mut vec![]);
        assert_eq!(r.planes.len(), 3, "XY, XZ e YZ");
        assert!(r.planes.iter().all(|p| p.id.is_none()) && d.origin_visible);
        a.origem_oculta = true;
        let (d, _) = compose(&a, None, &mut PartCache::default(), &mut vec![]);
        assert!(!d.origin_visible);
    }

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
        a.add_mate(MateKind::Contact, MateRef { comp: p1, geom: bot, pick: None, viva: None }, MateRef { comp: p2, geom: top, pick: None, viva: None });
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
        // subconjunto com a placa 2: a restrição com a placa 1 passa para o nó, e o furo se divide
        let (sub, no, rel) = a.extract_subassembly(&[p2], "Sub", "sub.fka");
        assert_eq!(sub.nodes.len(), 1);
        assert_eq!(rel.religadas.len(), 1);
        assert!(a.mates.iter().all(|m| m.b.comp == no || m.a.comp == no));
        assert!(a.features.iter().all(|f| !f.targets.contains(&p2)));
        assert_eq!(sub.features[0].targets, vec![p2]);
        let _ = p1;
    }
}
