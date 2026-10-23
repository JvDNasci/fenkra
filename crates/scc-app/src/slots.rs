//! Campos de referência vazios: um comando novo só recebe o que estava pré-selecionado; o resto
//! nasce "a escolher" (um id sentinela por tipo) e é preenchido pela próxima seleção no 3D ou na
//! árvore, na ordem dos campos.

use crate::geo_ui::{axis_of, Seed};
use scc_core::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::collections::VecDeque;

pub const UNSET_POINT: Id = Id(u64::MAX - 11);
pub const UNSET_CURVE: Id = Id(u64::MAX - 12);
pub const UNSET_PLANE: Id = Id(u64::MAX - 13);
pub const UNSET_SURF: Id = Id(u64::MAX - 14);
pub const UNSET_AXIS: Id = Id(u64::MAX - 15);
pub const UNSET_SKETCH: Id = Id(u64::MAX - 16);

pub fn point() -> PointRef {
    PointRef::Ref(UNSET_POINT)
}
pub fn curve() -> CurveRef {
    CurveRef::Ref(UNSET_CURVE)
}
pub fn plane() -> PlaneRef {
    PlaneRef::Datum(UNSET_PLANE)
}
pub fn surf() -> SurfRef {
    SurfRef::Ref(UNSET_SURF)
}
pub fn axis() -> AxisRef {
    AxisRef::Ref(UNSET_AXIS)
}

pub fn is_unset(id: Id) -> bool {
    (UNSET_SKETCH.0..=UNSET_POINT.0).contains(&id.0)
}

/// Texto do campo vazio (pede a referência certa).
pub fn label(id: Id) -> Option<String> {
    let what = match id {
        x if x == UNSET_POINT => "um vértice ou ponto",
        x if x == UNSET_CURVE => "uma aresta ou curva",
        x if x == UNSET_PLANE => "um plano ou face plana",
        x if x == UNSET_SURF => "uma face ou superfície",
        x if x == UNSET_AXIS => "uma aresta, linha ou eixo",
        x if x == UNSET_SKETCH => "um sketch (na árvore)",
        _ => return None,
    };
    Some(format!("⊙ {} {}", crate::i18n::tr("selecione"), crate::i18n::tr(what)))
}

fn walk_count(v: &Value, n: &mut usize) {
    match v {
        Value::Number(x) => {
            if x.as_u64().is_some_and(|u| is_unset(Id(u))) {
                *n += 1;
            }
        }
        Value::Object(m) => m.values().for_each(|x| walk_count(x, n)),
        Value::Array(a) => a.iter().for_each(|x| walk_count(x, n)),
        _ => {}
    }
}

/// Quantos campos ainda estão vazios.
pub fn count_unset<T: Serialize>(k: &T) -> usize {
    let mut n = 0;
    if let Ok(v) = serde_json::to_value(k) {
        walk_count(&v, &mut n);
    }
    n
}

/// Candidatos vindos da seleção (consumidos na ordem dos campos).
#[derive(Default)]
pub struct Cands {
    pub points: VecDeque<PointRef>,
    pub curves: VecDeque<CurveRef>,
    pub planes: VecDeque<PlaneRef>,
    pub surfs: VecDeque<SurfRef>,
    pub sketches: VecDeque<Id>,
}

impl Cands {
    pub fn from_seed(s: &Seed, sketch: Option<Id>) -> Self {
        let mut surfs: VecDeque<SurfRef> = s.surface.into_iter().collect();
        surfs.extend(s.planes.iter().map(|p| SurfRef::Plane(*p)));
        Cands {
            points: s.points.iter().copied().collect(),
            curves: s.curves.iter().cloned().collect(),
            planes: s.planes.iter().copied().collect(),
            surfs,
            sketches: sketch.into_iter().collect(),
        }
    }
}

fn walk_fill(v: &mut Value, c: &mut Cands, changed: &mut bool) {
    let rep: Option<Value> = match v {
        Value::Number(x) if x.as_u64() == Some(UNSET_SKETCH.0) => c.sketches.pop_front().map(|s| Value::from(s.0)),
        Value::Object(m) if m.len() == 1 => match m.values().next().and_then(|x| x.as_u64()) {
            Some(u) if u == UNSET_POINT.0 => c.points.pop_front().and_then(|p| serde_json::to_value(p).ok()),
            Some(u) if u == UNSET_CURVE.0 => c.curves.pop_front().and_then(|p| serde_json::to_value(p).ok()),
            Some(u) if u == UNSET_PLANE.0 => c.planes.pop_front().and_then(|p| serde_json::to_value(p).ok()),
            Some(u) if u == UNSET_SURF.0 => c.surfs.pop_front().and_then(|p| serde_json::to_value(p).ok()),
            Some(u) if u == UNSET_AXIS.0 => c.curves.pop_front().and_then(|p| serde_json::to_value(axis_of(&p)).ok()),
            _ => None,
        },
        _ => None,
    };
    if let Some(r) = rep {
        *v = r;
        *changed = true;
        return;
    }
    match v {
        Value::Object(m) => m.values_mut().for_each(|x| walk_fill(x, c, changed)),
        Value::Array(a) => a.iter_mut().for_each(|x| walk_fill(x, c, changed)),
        _ => {}
    }
}

/// Preenche os campos vazios com os candidatos. Devolve se mudou algo.
pub fn fill<T: Serialize + DeserializeOwned>(k: &mut T, c: &mut Cands) -> bool {
    let Ok(mut v) = serde_json::to_value(&*k) else { return false };
    let mut changed = false;
    walk_fill(&mut v, c, &mut changed);
    if changed {
        if let Ok(n) = serde_json::from_value(v) {
            *k = n;
            return true;
        }
    }
    false
}

/// Campos que não são referências (pontos clicados guardados como coordenadas, listas):
/// preenchidos pela seleção quando ainda estão vazios.
pub fn fill_special(k: &mut FeatureKind, s: &Seed, edge_mid: Option<[f64; 3]>, face_pt: Option<[f64; 3]>, verts: &[[f64; 3]], sketch: Option<Id>, shape: Option<Id>) -> bool {
    let z = [0.; 3];
    match k {
        FeatureKind::SmFlange(f) if f.edge == z => edge_mid.map(|e| f.edge = e).is_some(),
        FeatureKind::SmHem(h) if h.edge == z => edge_mid.map(|e| h.edge = e).is_some(),
        FeatureKind::Thread(t) if t.face == z => face_pt.map(|p| t.face = p).is_some(),
        FeatureKind::SmBend(b) if b.a == b.b && verts.len() >= 2 => {
            (b.a, b.b) = (verts[0], verts[1]);
            true
        }
        FeatureKind::SmLouver(b) if b.a == b.b && verts.len() >= 2 => {
            (b.a, b.b) = (verts[0], verts[1]);
            true
        }
        FeatureKind::SmJoggle(b) if b.a == b.b && verts.len() >= 2 => {
            (b.a, b.b) = (verts[0], verts[1]);
            true
        }
        FeatureKind::Hole(h) if h.sketch == UNSET_SKETCH && h.points.is_empty() && !s.points.is_empty() => {
            h.points = s.points.clone();
            h.sketch = Id(0);
            true
        }
        FeatureKind::StructMember(m) if m.paths.is_empty() && !s.curves.is_empty() => {
            m.paths = s.curves.clone();
            true
        }
        FeatureKind::Loft(l) => match sketch {
            Some(sk) if !l.sections.contains(&sk) => {
                l.sections.push(sk);
                true
            }
            _ => false,
        },
        FeatureKind::Pattern(p) => match shape {
            Some(f) if !p.sources.contains(&f) => {
                p.sources.push(f);
                true
            }
            _ => false,
        },
        FeatureKind::Mirror(m) => match shape {
            Some(f) if !m.sources.contains(&f) => {
                m.sources.push(f);
                true
            }
            _ => false,
        },
        _ => false,
    }
}

/// Algum campo ainda vazio (também os especiais)?
pub fn feature_incomplete(k: &FeatureKind) -> bool {
    let z = [0.; 3];
    count_unset(k) > 0
        || match k {
            FeatureKind::SmFlange(f) => f.edge == z,
            FeatureKind::SmHem(h) => h.edge == z,
            FeatureKind::Thread(t) => t.face == z,
            FeatureKind::SmBend(b) => b.a == b.b,
            FeatureKind::SmLouver(b) => b.a == b.b,
            FeatureKind::SmJoggle(b) => b.a == b.b,
            FeatureKind::Loft(l) => l.sections.len() < 2,
            FeatureKind::StructMember(m) => m.paths.is_empty(),
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campos_vazios_preenchidos_na_ordem() {
        let mut k = RefKind::LinePoints { a: point(), b: point(), start: "0".into(), end: "0".into() };
        assert_eq!(count_unset(&k), 2);
        let seed = Seed { points: vec![PointRef::Fixed([1., 2., 3.])], ..Default::default() };
        assert!(fill(&mut k, &mut Cands::from_seed(&seed, None)));
        assert_eq!(count_unset(&k), 1, "um clique preenche só o primeiro campo");
        let seed = Seed { points: vec![PointRef::Ref(Id(7))], ..Default::default() };
        assert!(fill(&mut k, &mut Cands::from_seed(&seed, None)));
        assert_eq!(k, RefKind::LinePoints { a: PointRef::Fixed([1., 2., 3.]), b: PointRef::Ref(Id(7)), start: "0".into(), end: "0".into() });
        // tipos não se misturam: uma curva não entra num campo de ponto
        let mut k = RefKind::PointOnCurve { curve: curve(), at: OnCurve::Ratio("0.5".into()), reverse: false };
        let seed = Seed { points: vec![PointRef::Ref(Id(9))], ..Default::default() };
        assert!(!fill(&mut k, &mut Cands::from_seed(&seed, None)));
        // sketch do comando vem da árvore
        let mut f = FeatureKind::Extrude(Extrude { sketch: UNSET_SKETCH, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None });
        assert!(feature_incomplete(&f));
        assert!(fill(&mut f, &mut Cands::from_seed(&Seed::default(), Some(Id(42)))));
        assert!(!feature_incomplete(&f));
    }
}
