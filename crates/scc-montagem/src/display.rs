//! Exibição (estilo "composer"): estados de exibição salvos (visível/oculto, cores, explosão,
//! câmera), vista explodida automática e animações por quadros-chave.
//!
//! Os estados também são os "coletores de exibição" do detalhamento: uma vista 2D/3D pode
//! apontar para um estado e mostrar exatamente o que ele mostra.

use crate::*;
use std::collections::HashMap;

/// Câmera salva (alvo, ângulos, distância, ortogonal).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Cam {
    pub target: V3,
    pub yaw: f64,
    pub pitch: f64,
    pub dist: f64,
    pub ortho: bool,
}

/// Estado de exibição: o que aparece, com que cor e onde (explosão), e a câmera.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct DisplayState {
    pub name: String,
    /// componentes ocultos neste estado
    #[serde(default)]
    pub hidden: Vec<Id>,
    #[serde(default)]
    pub colors: Vec<(Id, [u8; 3])>,
    /// deslocamento de explosão de cada componente (mm, no mundo)
    #[serde(default)]
    pub offsets: Vec<(Id, V3)>,
    #[serde(default)]
    pub camera: Option<Cam>,
}

impl DisplayState {
    pub fn offset(&self, id: Id) -> V3 {
        self.offsets.iter().find(|(c, _)| *c == id).map(|(_, v)| *v).unwrap_or([0.; 3])
    }
    pub fn set_offset(&mut self, id: Id, v: V3) {
        match self.offsets.iter_mut().find(|(c, _)| *c == id) {
            Some(x) => x.1 = v,
            None => self.offsets.push((id, v)),
        }
    }
    pub fn color(&self, id: Id) -> Option<[u8; 3]> {
        self.colors.iter().find(|(c, _)| *c == id).map(|(_, v)| *v)
    }
}

/// Quadro-chave da animação: um estado de exibição num instante.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Key {
    pub t: f64,
    pub state: DisplayState,
}

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Animation {
    pub name: String,
    pub keys: Vec<Key>,
}

impl Animation {
    pub fn duration(&self) -> f64 {
        self.keys.iter().map(|k| k.t).fold(0., f64::max)
    }

    /// Estado no tempo `t`: explosão e câmera interpoladas (suave), visibilidade e cores do
    /// quadro anterior.
    pub fn at(&self, t: f64) -> Option<DisplayState> {
        let mut keys = self.keys.clone();
        keys.sort_by(|a, b| a.t.total_cmp(&b.t));
        let first = keys.first()?;
        if t <= first.t || keys.len() == 1 {
            return Some(first.state.clone());
        }
        let i = keys.iter().rposition(|k| k.t <= t).unwrap_or(0);
        let a = &keys[i];
        let Some(b) = keys.get(i + 1) else { return Some(a.state.clone()) };
        let u = ((t - a.t) / (b.t - a.t).max(1e-9)).clamp(0., 1.);
        let s = u * u * (3. - 2. * u); // suavização
        let mut out = a.state.clone();
        let ids: Vec<Id> = a.state.offsets.iter().chain(b.state.offsets.iter()).map(|(c, _)| *c).collect();
        out.offsets.clear();
        for id in ids {
            if out.offsets.iter().any(|(c, _)| *c == id) {
                continue;
            }
            let (pa, pb) = (a.state.offset(id), b.state.offset(id));
            out.offsets.push((id, [0, 1, 2].map(|k| pa[k] + (pb[k] - pa[k]) * s)));
        }
        if let (Some(ca), Some(cb)) = (a.state.camera, b.state.camera) {
            let l = |x: f64, y: f64| x + (y - x) * s;
            // ângulo de giro pelo caminho mais curto
            let mut dy = cb.yaw - ca.yaw;
            while dy > std::f64::consts::PI {
                dy -= std::f64::consts::TAU;
            }
            while dy < -std::f64::consts::PI {
                dy += std::f64::consts::TAU;
            }
            out.camera = Some(Cam {
                target: [0, 1, 2].map(|k| l(ca.target[k], cb.target[k])),
                yaw: ca.yaw + dy * s,
                pitch: l(ca.pitch, cb.pitch),
                dist: l(ca.dist, cb.dist),
                ortho: if s < 0.5 { ca.ortho } else { cb.ortho },
            });
        }
        Some(out)
    }
}

/// Explosão automática: cada componente se afasta do centro da montagem (pelo centro dele),
/// proporcional ao fator; componentes fixos ficam no lugar.
pub fn auto_explode(a: &Assembly, centers: &HashMap<Id, V3>, factor: f64) -> Vec<(Id, V3)> {
    let comps: Vec<&Node> = a.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).collect();
    if comps.is_empty() {
        return vec![];
    }
    let c0 = {
        let pts: Vec<V3> = comps.iter().filter_map(|n| centers.get(&n.id).copied()).collect();
        let k = pts.len().max(1) as f64;
        [0, 1, 2].map(|i| pts.iter().map(|p| p[i]).sum::<f64>() / k)
    };
    comps
        .iter()
        .filter(|n| !n.fixed)
        .filter_map(|n| {
            let c = centers.get(&n.id)?;
            let d = sub(*c, c0);
            Some((n.id, [d[0] * factor, d[1] * factor, d[2] * factor]))
        })
        .collect()
}

impl Assembly {
    /// Montagem com o estado aplicado (visibilidade, cores e explosão nas posições) — para exibir.
    pub fn with_state(&self, st: &DisplayState) -> Assembly {
        let mut a = self.clone();
        for n in &mut a.nodes {
            if st.hidden.contains(&n.id) {
                n.visible = false;
            }
            if let Some(c) = st.color(n.id) {
                n.color = Some(c);
            }
            let o = st.offset(n.id);
            n.placement.origin = [0, 1, 2].map(|k| n.placement.origin[k] + o[k]);
        }
        a
    }

    /// Estado com a situação atual (todos visíveis como estão, sem explosão).
    pub fn snapshot_state(&self, name: &str) -> DisplayState {
        DisplayState {
            name: name.into(),
            hidden: self.nodes.iter().filter(|n| !n.visible).map(|n| n.id).collect(),
            colors: self.nodes.iter().filter_map(|n| n.color.map(|c| (n.id, c))).collect(),
            offsets: vec![],
            camera: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explosao_e_animacao() {
        let mut a = Assembly::new("ex");
        let b = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        let t = a.add_component("Tampa", NodeKind::Part { path: "t".into() }, Placement { origin: [0., 0., 10.], ..Default::default() }, None);
        let centers: HashMap<Id, V3> = [(b, [0., 0., 0.]), (t, [0., 0., 10.])].into();
        let ex = auto_explode(&a, &centers, 2.);
        assert_eq!(ex.len(), 1, "a base fixa não sai do lugar");
        assert!((ex[0].1[2] - 10.).abs() < 1e-9, "{ex:?}");
        let mut s1 = a.snapshot_state("montada");
        s1.camera = Some(Cam { target: [0.; 3], yaw: 0., pitch: 0., dist: 100., ortho: false });
        let mut s2 = s1.clone();
        s2.offsets = ex.clone();
        s2.hidden.push(b);
        s2.camera = Some(Cam { target: [0.; 3], yaw: 1., pitch: 0., dist: 200., ortho: false });
        let anim = Animation { name: "explodir".into(), keys: vec![Key { t: 0., state: s1 }, Key { t: 2., state: s2 }] };
        let mid = anim.at(1.).unwrap();
        assert!((mid.offset(t)[2] - 5.).abs() < 1e-9, "meio do caminho (curva suave e simétrica)");
        assert!((mid.camera.unwrap().dist - 150.).abs() < 1e-9);
        assert!(!mid.hidden.contains(&b), "visibilidade muda no quadro-chave");
        let end = anim.at(5.).unwrap();
        assert!(end.hidden.contains(&b));
        let shown = a.with_state(&end);
        assert!(!shown.node(b).unwrap().visible);
        assert!((shown.node(t).unwrap().placement.origin[2] - 20.).abs() < 1e-9);
    }
}
