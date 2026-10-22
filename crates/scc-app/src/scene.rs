//! Converte o resultado do recompute em uma cena para a viewport.
//!
//! Visibilidade: cada item tem oculto/visível; o **ambiente de ocultos** inverte a regra (mostra só o
//! que está oculto, para reexibir). O **filtro** de exibição vale nos dois ambientes.

use crate::app::Sel;
use crate::render::{LineVertex, MeshVertex, Scene};
use crate::settings::Filter;
use glam::Vec3;
use scc_core::{Appearance, Document, Id, OriginPlane, PlaneRef};
use scc_solidos::RecomputeResult;
use std::collections::{HashMap, HashSet};

const SKETCH: [f32; 4] = [0.35, 0.75, 1.0, 1.0];
const CONSTR: [f32; 4] = [0.85, 0.6, 0.3, 0.8];
const HILITE: [f32; 4] = [1.0, 0.85, 0.2, 1.0];

pub struct SceneOpts<'a> {
    /// sketch em edição: suas linhas são desenhadas pelo editor, não pela cena
    pub editing: Option<Id>,
    pub sel: &'a Sel,
    pub filter: &'a Filter,
    pub hidden_env: bool,
    pub dark_edges: bool,
}

fn lv(p: [f32; 3], col: [f32; 4]) -> LineVertex {
    LineVertex { pos: p, col }
}

fn grow(b: &mut Option<(Vec3, Vec3)>, p: Vec3) {
    *b = Some(match *b {
        None => (p, p),
        Some((lo, hi)) => (lo.min(p), hi.max(p)),
    });
}

/// Regra de exibição: no ambiente normal mostra os visíveis; no de ocultos, só os ocultos.
fn shows(hidden: bool, env: bool) -> bool {
    hidden == env
}

fn rgb(c: [u8; 3]) -> [f32; 3] {
    c.map(|x| x as f32 / 255.)
}

pub fn build(doc: &Document, r: &RecomputeResult, o: &SceneOpts) -> Scene {
    let mut s = Scene::default();
    let mut bounds = None;
    let env = o.hidden_env;

    // itens destacados pela seleção
    let mut hi_features: HashSet<Id> = HashSet::new();
    if let Sel::Feature(id) = o.sel {
        hi_features.insert(*id);
        if let Some(f) = doc.feature(*id) {
            hi_features.extend(Document::refs_of(&f.kind));
        }
    }

    let feat_app: HashMap<Id, Appearance> = doc.features.iter().filter_map(|f| f.appearance.map(|a| (f.id, a))).collect();
    let feat_hidden: HashSet<Id> = doc.features.iter().filter(|f| f.hidden).map(|f| f.id).collect();
    let edge_col = if o.dark_edges { [0.06, 0.07, 0.09, 1.0] } else { [0.85, 0.88, 0.92, 1.0] };

    if o.filter.corpos {
        for b in &r.bodies {
            let Some(col) = doc.collectors.iter().find(|c| c.id == b.collector) else { continue };
            let body_sel = matches!(o.sel, Sel::Collector(c) if *c == b.collector);
            let base = doc.appearance.unwrap_or(Appearance { color: col.color, finish: col.finish });
            let m = &b.mesh;
            let nfaces = m.face_ranges.len();
            for fi in 0..nfaces {
                let owner = b.face_owner.get(fi).copied();
                let face_hidden = !col.visible || owner.is_some_and(|w| feat_hidden.contains(&w));
                if !shows(face_hidden, env) {
                    continue;
                }
                let app = owner.and_then(|w| feat_app.get(&w).copied()).unwrap_or(base);
                let (rough, metal, alpha) = app.finish.params();
                let mut c = rgb(app.color);
                if body_sel || owner.is_some_and(|w| hi_features.contains(&w)) {
                    c = [c[0] * 0.55 + 0.45, c[1] * 0.55 + 0.38, c[2] * 0.55 + 0.08];
                }
                let start = m.face_ranges[fi] as usize;
                let end = m.face_ranges.get(fi + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
                // duplica os vértices da face para poder colorir por face
                let mut remap: HashMap<u32, u32> = HashMap::new();
                for &vi in &m.indices[start..end] {
                    let ni = *remap.entry(vi).or_insert_with(|| {
                        let k = vi as usize * 3;
                        let p = [m.positions[k], m.positions[k + 1], m.positions[k + 2]];
                        grow(&mut bounds, Vec3::from(p));
                        s.mesh.push(MeshVertex {
                            pos: p,
                            nrm: [m.normals[k], m.normals[k + 1], m.normals[k + 2]],
                            col: [c[0], c[1], c[2], 1.0],
                            mat: [rough, metal, alpha, 0.],
                        });
                        (s.mesh.len() - 1) as u32
                    });
                    s.indices.push(ni);
                }
            }
            if shows(!col.visible, env) {
                for p in m.edges.chunks_exact(3) {
                    s.edges.push(lv([p[0], p[1], p[2]], edge_col));
                }
            }
        }
    }

    // sketches: mostrados conforme oculto/visível; o sketch do comando selecionado sempre aparece em destaque
    if o.filter.sketches {
        for sk in &r.sketches {
            let hidden = feat_hidden.contains(&sk.feature);
            let editing = o.editing == Some(sk.feature);
            let h = hi_features.contains(&sk.feature) && !editing;
            if !editing && !h && !shows(hidden, env) {
                continue;
            }
            // sombra dos perfis fechados
            let n = Vec3::from(sk.frame.normal.map(|v| v as f32));
            let fc = if editing { [0.3, 0.65, 1.0, 1.0] } else { [0.35, 0.7, 1.0, 1.0] };
            for p in &sk.fill {
                s.fill.push(MeshVertex { pos: *p, nrm: n.into(), col: fc, mat: [1.0, 0.0, if editing { 0.22 } else { 0.14 }, 0.] });
            }
            if editing {
                continue;
            }
            let dst = if h { &mut s.overlay } else { &mut s.lines };
            for p in &sk.lines {
                dst.push(lv(*p, if h { HILITE } else { SKETCH }));
                grow(&mut bounds, Vec3::from(*p));
            }
            for p in &sk.construction {
                dst.push(lv(*p, CONSTR));
            }
        }
    }

    // tamanho de referência para planos, pontos e eixos
    let size = bounds.map(|(lo, hi): (Vec3, Vec3)| (hi - lo).max_element().max(20.) * 0.6).unwrap_or(60.);

    if o.filter.sketches && o.filter.pontos {
        for sk in &r.sketches {
            let h = hi_features.contains(&sk.feature);
            if !h && !shows(feat_hidden.contains(&sk.feature), env) {
                continue;
            }
            let d = size * 0.025;
            for p in &sk.points {
                let c = Vec3::from(*p);
                for ax in [Vec3::X, Vec3::Y, Vec3::Z] {
                    s.overlay.push(lv((c - ax * d).into(), if h { HILITE } else { SKETCH }));
                    s.overlay.push(lv((c + ax * d).into(), if h { HILITE } else { SKETCH }));
                }
            }
        }
    }

    // planos da Origem e das Referências
    for p in &r.planes {
        let (visible_flag, allowed) = match p.id {
            Some(id) => (doc.reference(id).is_some_and(|x| x.visible), o.filter.planos_ref),
            None => (doc.origin_visible, o.filter.planos_origem),
        };
        let selected = match (o.sel, p.id) {
            (Sel::Ref(a), Some(b)) => *a == b,
            (Sel::Origin(op), None) => p.name == plane_name(*op),
            _ => false,
        };
        if !selected && (!allowed || !shows(!visible_flag, env)) {
            continue;
        }
        let col = match (p.id, p.name.as_str()) {
            (Some(_), _) => [0.95, 0.7, 0.3, 0.6],
            (_, "XY") => [0.4, 0.55, 1.0, 0.5],
            (_, "XZ") => [0.4, 0.85, 0.45, 0.5],
            _ => [1.0, 0.45, 0.4, 0.5],
        };
        let f = &p.frame;
        let (o3, x, yv) = (
            Vec3::from(f.origin.map(|v| v as f32)),
            Vec3::from(f.x.map(|v| v as f32)),
            Vec3::from(f.y().map(|v| v as f32)),
        );
        let h = size * if p.id.is_some() { 0.45 } else { 0.5 };
        let c = [o3 + (x + yv) * h, o3 + (x - yv) * h, o3 + (-x - yv) * h, o3 + (-x + yv) * h];
        let dst = if selected { &mut s.overlay } else { &mut s.lines };
        for i in 0..4 {
            let colr = if selected { HILITE } else { col };
            dst.push(lv(c[i].into(), colr));
            dst.push(lv(c[(i + 1) % 4].into(), colr));
        }
    }

    // eixos da Origem
    if o.filter.eixos && shows(!doc.origin_visible, env) {
        for (ax, col) in [(Vec3::X, [1., 0.3, 0.3, 1.]), (Vec3::Y, [0.3, 0.9, 0.3, 1.]), (Vec3::Z, [0.35, 0.55, 1., 1.])] {
            s.overlay.push(lv([0.; 3], col));
            s.overlay.push(lv((ax * size * 0.25).into(), col));
        }
    }

    s.bounds = bounds.or(Some((Vec3::splat(-size), Vec3::splat(size))));
    s
}

/// O comando é um sketch?
pub fn is_sketch_feature(doc: &Document, id: Id) -> bool {
    doc.feature(id).is_some_and(|f| matches!(f.kind, scc_core::FeatureKind::Sketch(_)))
}

pub fn plane_name(o: OriginPlane) -> &'static str {
    match o {
        OriginPlane::XY => "XY",
        OriginPlane::XZ => "XZ",
        OriginPlane::YZ => "YZ",
    }
}

pub fn plane_ref_name(doc: &Document, p: &PlaneRef) -> String {
    match p {
        PlaneRef::Origin(o) => format!("Origem · {}", plane_name(*o)),
        PlaneRef::Datum(id) => doc.reference(*id).map(|r| r.name.clone()).unwrap_or_else(|| format!("⚠ perdida #{}", id.0)),
    }
}
