// SPDX-License-Identifier: LGPL-2.1-or-later
//! Exibição: estados de exibição salvos (visível/oculto, cores, explosão,
//! câmera), explosão por passos com linhas automáticas e animações por quadros-chave.
//!
//! A [`Explosao`] é uma lista ordenada de passos. Ela vale por si: os deslocamentos e as linhas saem
//! dos passos, e o estado de exibição guarda só o resultado, que é o que o desenho lê. Instruções de
//! montagem e simulação da sequência partem de [`Explosao::fases`].
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
    /// linhas de explosão no mundo, já prontas para o 3D e para o desenho
    #[serde(default)]
    pub linhas: Vec<LinhaExplosao>,
}

/// Como um passo afasta as peças.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum ModoPasso {
    /// todas andam a mesma distância na mesma direção
    Linear { dir: V3, dist: f64 },
    /// cada uma se afasta do eixo pela reta que sai dele; quem está no eixo fica
    Radial { centro: V3, eixo: V3, dist: f64 },
    /// em fila ao longo da direção: a última anda `dist`, e as outras se espaçam por igual até ela
    Espacado { dir: V3, dist: f64 },
}

impl ModoPasso {
    pub fn dist(&self) -> f64 {
        match *self {
            ModoPasso::Linear { dist, .. } | ModoPasso::Radial { dist, .. } | ModoPasso::Espacado { dist, .. } => dist,
        }
    }
    pub fn set_dist(&mut self, v: f64) {
        match self {
            ModoPasso::Linear { dist, .. } | ModoPasso::Radial { dist, .. } | ModoPasso::Espacado { dist, .. } => *dist = v,
        }
    }
    /// Direção da seta do passo; no radial, o eixo.
    pub fn dir(&self) -> V3 {
        match *self {
            ModoPasso::Linear { dir, .. } | ModoPasso::Espacado { dir, .. } => dir,
            ModoPasso::Radial { eixo, .. } => eixo,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            ModoPasso::Linear { .. } => "Linear",
            ModoPasso::Radial { .. } => "Radial",
            ModoPasso::Espacado { .. } => "Espaçamento automático",
        }
    }
}

/// Um passo da explosão: um grupo de peças e como ele sai do lugar.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PassoExplosao {
    pub nome: String,
    pub comps: Vec<Id>,
    pub modo: ModoPasso,
    #[serde(default)]
    pub suprimido: bool,
}

impl PassoExplosao {
    /// Quanto cada peça anda neste passo, partindo de `onde`.
    fn deslocamentos(&self, onde: &HashMap<Id, V3>) -> Vec<(Id, V3)> {
        let vivas: Vec<(Id, V3)> = self.comps.iter().filter_map(|c| onde.get(c).map(|p| (*c, *p))).collect();
        let vezes = |d: V3, k: f64| d.map(|v| v * k);
        match self.modo {
            ModoPasso::Linear { dir, dist } => vivas.iter().map(|(c, _)| (*c, vezes(unit(dir), dist))).collect(),
            ModoPasso::Radial { centro, eixo, dist } => {
                let e = unit(eixo);
                let saida = |p: V3| {
                    let d = sub(p, centro);
                    let k = dot(d, e);
                    [0, 1, 2].map(|i| d[i] - e[i] * k)
                };
                vivas.iter().filter(|(_, p)| norm(saida(*p)) > 1e-6).map(|(c, p)| (*c, vezes(unit(saida(*p)), dist))).collect()
            }
            ModoPasso::Espacado { dir, dist } => {
                let d = unit(dir);
                let mut fila = vivas;
                // empate na fila: a ordem do passo decide, e assim o resultado não muda entre aberturas
                fila.sort_by(|a, b| dot(a.1, d).total_cmp(&dot(b.1, d)));
                let n = fila.len() as f64;
                fila.iter().enumerate().map(|(k, (c, _))| (*c, vezes(d, dist * (k + 1) as f64 / n))).collect()
            }
        }
    }
}

/// Polilinha que liga uma peça ao lugar dela na montagem.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct LinhaExplosao {
    pub comp: Id,
    pub pontos: Vec<V3>,
}

/// Situação depois de um passo: é o que as instruções e a simulação da sequência consomem.
#[derive(Clone, PartialEq, Debug)]
pub struct Fase {
    /// posição do passo na lista
    pub passo: usize,
    /// quem saiu do lugar neste passo
    pub movidas: Vec<Id>,
    /// deslocamento acumulado de cada peça até aqui
    pub offsets: Vec<(Id, V3)>,
}

/// Lista de passos com nome. A ordem é a da desmontagem.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Explosao {
    pub nome: String,
    #[serde(default)]
    pub passos: Vec<PassoExplosao>,
    /// desenha as linhas de explosão
    #[serde(default = "sim")]
    pub linhas: bool,
}

fn sim() -> bool {
    true
}

impl Explosao {
    pub fn nova(nome: &str) -> Self {
        Explosao { nome: nome.into(), passos: vec![], linhas: true }
    }

    /// Um passo por vez, sobre `centros` (centro de cada peça montada, no mundo). Cada passo parte de
    /// onde o anterior deixou: o radial e a fila olham a posição de agora, não a de montada.
    pub fn fases(&self, centros: &HashMap<Id, V3>) -> Vec<Fase> {
        let mut onde = centros.clone();
        let mut acumulado: Vec<(Id, V3)> = vec![];
        let mut out = vec![];
        for (i, p) in self.passos.iter().enumerate().filter(|(_, p)| !p.suprimido) {
            let d = p.deslocamentos(&onde);
            for (c, v) in &d {
                if let Some(q) = onde.get_mut(c) {
                    *q = [0, 1, 2].map(|k| q[k] + v[k]);
                }
                match acumulado.iter_mut().find(|x| x.0 == *c) {
                    Some(x) => x.1 = [0, 1, 2].map(|k| x.1[k] + v[k]),
                    None => acumulado.push((*c, *v)),
                }
            }
            out.push(Fase { passo: i, movidas: d.iter().map(|x| x.0).collect(), offsets: acumulado.clone() });
        }
        out
    }

    /// Deslocamento final de cada peça.
    pub fn offsets(&self, centros: &HashMap<Id, V3>) -> Vec<(Id, V3)> {
        self.fases(centros).pop().map(|f| f.offsets).unwrap_or_default()
    }

    /// Linha de cada peça que saiu do lugar: da posição montada, passando por cada passo que a moveu.
    /// Duas peças num mesmo caminho dão linhas sobrepostas, e isso é o que se vê numa vista explodida.
    pub fn linhas(&self, centros: &HashMap<Id, V3>) -> Vec<LinhaExplosao> {
        let mut out: Vec<LinhaExplosao> = vec![];
        for f in self.fases(centros) {
            for c in &f.movidas {
                let Some(base) = centros.get(c) else { continue };
                let o = f.offsets.iter().find(|x| x.0 == *c).map_or([0.; 3], |x| x.1);
                let p = [0, 1, 2].map(|k| base[k] + o[k]);
                match out.iter_mut().find(|l| l.comp == *c) {
                    Some(l) => l.pontos.push(p),
                    None => out.push(LinhaExplosao { comp: *c, pontos: vec![*base, p] }),
                }
            }
        }
        out.retain(|l| l.pontos.windows(2).any(|w| norm(sub(w[1], w[0])) > 1e-9));
        out
    }

    /// Onde fica a seta do passo `k`: o meio das peças depois dele e a direção em que a distância
    /// cresce. No radial não há uma direção só: vale a da primeira peça.
    pub fn seta(&self, k: usize, centros: &HashMap<Id, V3>) -> Option<(V3, V3)> {
        let fases = self.fases(centros);
        let i = fases.iter().position(|f| f.passo == k)?;
        let f = &fases[i];
        let achar = |o: &[(Id, V3)], c: Id| o.iter().find(|x| x.0 == c).map_or([0.; 3], |x| x.1);
        let pts: Vec<V3> = f.movidas.iter().filter_map(|c| centros.get(c).map(|p| (*c, *p))).map(|(c, p)| {
            let o = achar(&f.offsets, c);
            [0, 1, 2].map(|e| p[e] + o[e])
        }).collect();
        let n = pts.len();
        if n == 0 {
            return None;
        }
        let meio = [0, 1, 2].map(|e| pts.iter().map(|p| p[e]).sum::<f64>() / n as f64);
        let dir = match self.passos[k].modo {
            ModoPasso::Radial { .. } => {
                let c = f.movidas[0];
                let antes = if i > 0 { achar(&fases[i - 1].offsets, c) } else { [0.; 3] };
                unit(sub(achar(&f.offsets, c), antes))
            }
            m => unit(m.dir()),
        };
        Some((meio, dir))
    }

    /// Leva o passo `de` para a posição `para`.
    pub fn mover_passo(&mut self, de: usize, para: usize) {
        if de < self.passos.len() && para < self.passos.len() {
            let p = self.passos.remove(de);
            self.passos.insert(para, p);
        }
    }

    /// Nome livre para um passo novo.
    pub fn proximo_nome(&self) -> String {
        (1..).map(|k| format!("Passo {k}")).find(|n| !self.passos.iter().any(|p| p.nome == *n)).unwrap_or_default()
    }

    /// O arraste da seta do manipulador vira um passo linear das peças em `comps`.
    pub fn passo_da_seta(&mut self, comps: Vec<Id>, delta: V3) -> Option<usize> {
        let dist = norm(delta);
        if dist < 1e-6 || comps.is_empty() {
            return None;
        }
        self.passos.push(PassoExplosao { nome: self.proximo_nome(), comps, modo: ModoPasso::Linear { dir: unit(delta), dist }, suprimido: false });
        Some(self.passos.len() - 1)
    }

    /// Estado de exibição com os passos aplicados sobre `base`.
    pub fn no_estado(&self, base: &DisplayState, centros: &HashMap<Id, V3>) -> DisplayState {
        let mut s = base.clone();
        s.offsets = self.offsets(centros);
        s.linhas = if self.linhas { self.linhas(centros) } else { vec![] };
        s
    }

    /// Animação que abre a montagem um passo por vez, `duracao` segundos em cada um.
    pub fn animacao(&self, base: &DisplayState, centros: &HashMap<Id, V3>, duracao: f64) -> Animation {
        let mut keys = vec![Key { t: 0., state: DisplayState { offsets: vec![], linhas: vec![], ..base.clone() } }];
        for (k, f) in self.fases(centros).into_iter().enumerate() {
            let state = DisplayState { name: self.passos[f.passo].nome.clone(), offsets: f.offsets, linhas: vec![], ..base.clone() };
            keys.push(Key { t: duracao * (k + 1) as f64, state });
        }
        Animation { name: self.nome.clone(), keys }
    }
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
            linhas: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pilha() -> (Assembly, HashMap<Id, V3>, [Id; 3]) {
        let mut a = Assembly::new("pilha");
        let mut ids = [Id(0); 3];
        let mut c = HashMap::new();
        for (k, id) in ids.iter_mut().enumerate() {
            let z = 10. * k as f64;
            *id = a.add_component(&format!("P{k}"), NodeKind::Part { path: "p".into() }, Placement { origin: [0., 0., z], ..Default::default() }, None);
            c.insert(*id, [0., 0., z]);
        }
        (a, c, ids)
    }

    fn linear(comps: Vec<Id>, dir: V3, dist: f64) -> PassoExplosao {
        PassoExplosao { nome: "p".into(), comps, modo: ModoPasso::Linear { dir, dist }, suprimido: false }
    }

    fn deslocamento(o: &[(Id, V3)], id: Id) -> V3 {
        o.iter().find(|x| x.0 == id).map_or([0.; 3], |x| x.1)
    }

    #[test]
    fn passos_acumulam_na_ordem() {
        let (_, c, [_, b, t]) = pilha();
        let mut e = Explosao::nova("E");
        e.passos.push(linear(vec![t], [0., 0., 1.], 30.));
        e.passos.push(linear(vec![b, t], [1., 0., 0.], 5.));
        let o = e.offsets(&c);
        assert_eq!(deslocamento(&o, t), [5., 0., 30.]);
        assert_eq!(deslocamento(&o, b), [5., 0., 0.]);
        let f = e.fases(&c);
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].movidas, vec![t]);
        assert_eq!(deslocamento(&f[0].offsets, b), [0.; 3], "a fase mostra o meio do caminho");
    }

    #[test]
    fn passo_suprimido_nao_anda_e_nao_some() {
        let (_, c, [_, b, t]) = pilha();
        let mut e = Explosao::nova("E");
        e.passos.push(linear(vec![t], [0., 0., 1.], 30.));
        e.passos.push(linear(vec![b], [0., 0., 1.], 7.));
        e.passos[0].suprimido = true;
        let o = e.offsets(&c);
        assert_eq!(deslocamento(&o, t), [0.; 3]);
        assert_eq!(deslocamento(&o, b), [0., 0., 7.]);
        assert_eq!(e.fases(&c)[0].passo, 1, "a fase aponta o passo certo na lista");
        assert_eq!(e.passos.len(), 2);
    }

    #[test]
    fn radial_afasta_do_eixo_e_quem_esta_nele_fica() {
        let (_, mut c, [a, b, t]) = pilha();
        c.insert(a, [10., 0., 0.]);
        c.insert(b, [0., -4., 9.]);
        c.insert(t, [0., 0., 3.]);
        let mut e = Explosao::nova("E");
        e.passos.push(PassoExplosao { nome: "r".into(), comps: vec![a, b, t], modo: ModoPasso::Radial { centro: [0.; 3], eixo: [0., 0., 1.], dist: 20. }, suprimido: false });
        let o = e.offsets(&c);
        assert_eq!(deslocamento(&o, a), [20., 0., 0.]);
        assert_eq!(deslocamento(&o, b), [0., -20., 0.]);
        assert_eq!(deslocamento(&o, t), [0.; 3]);
    }

    #[test]
    fn espacamento_automatico_segue_a_ordem_da_fila() {
        let (_, c, [a, b, t]) = pilha();
        let mut e = Explosao::nova("E");
        // fora de ordem de propósito: quem manda é a posição ao longo da seta
        e.passos.push(PassoExplosao { nome: "f".into(), comps: vec![t, a, b], modo: ModoPasso::Espacado { dir: [0., 0., 1.], dist: 30. }, suprimido: false });
        let o = e.offsets(&c);
        assert_eq!(deslocamento(&o, a), [0., 0., 10.]);
        assert_eq!(deslocamento(&o, b), [0., 0., 20.]);
        assert_eq!(deslocamento(&o, t), [0., 0., 30.]);
    }

    #[test]
    fn linha_passa_por_cada_passo_e_liga_ao_lugar_montado() {
        let (_, c, [_, b, t]) = pilha();
        let mut e = Explosao::nova("E");
        e.passos.push(linear(vec![t], [0., 0., 1.], 30.));
        e.passos.push(linear(vec![t], [1., 0., 0.], 5.));
        let l = e.linhas(&c);
        assert_eq!(l.len(), 1, "só quem saiu do lugar tem linha: {l:?}");
        assert_eq!(l[0].pontos, vec![[0., 0., 20.], [0., 0., 50.], [5., 0., 50.]]);
        assert!(!l.iter().any(|x| x.comp == b));
    }

    #[test]
    fn reordenar_muda_o_caminho_e_o_final_nao() {
        let (_, c, [_, _, t]) = pilha();
        let mut e = Explosao::nova("E");
        e.passos.push(linear(vec![t], [0., 0., 1.], 30.));
        e.passos.push(linear(vec![t], [1., 0., 0.], 5.));
        let antes = e.offsets(&c);
        e.mover_passo(1, 0);
        assert_eq!(deslocamento(&e.offsets(&c), t), deslocamento(&antes, t));
        assert_eq!(e.linhas(&c)[0].pontos[1], [5., 0., 20.]);
        e.mover_passo(9, 0);
        assert_eq!(e.passos.len(), 2);
    }

    #[test]
    fn seta_fica_nas_pecas_depois_do_passo() {
        let (_, c, [_, b, t]) = pilha();
        let mut e = Explosao::nova("E");
        e.passos.push(linear(vec![b, t], [0., 0., 1.], 10.));
        e.passos.push(linear(vec![t], [1., 0., 0.], 5.));
        let (onde, dir) = e.seta(0, &c).unwrap();
        assert_eq!(onde, [0., 0., 25.]);
        assert_eq!(dir, [0., 0., 1.]);
        e.passos[0].suprimido = true;
        assert_eq!(e.seta(0, &c), None, "passo desligado não tem seta");
    }

    #[test]
    fn arraste_da_seta_vira_passo() {
        let (_, _, [_, b, _]) = pilha();
        let mut e = Explosao::nova("E");
        assert_eq!(e.passo_da_seta(vec![b], [0.; 3]), None, "clique sem arrastar não cria nada");
        assert_eq!(e.passo_da_seta(vec![], [1., 0., 0.]), None);
        assert_eq!(e.passo_da_seta(vec![b], [0., 3., 4.]), Some(0));
        let p = &e.passos[0];
        assert_eq!(p.nome, "Passo 1");
        assert!((p.modo.dist() - 5.).abs() < 1e-12);
        assert!(norm(sub(p.modo.dir(), [0., 0.6, 0.8])) < 1e-12);
        e.passo_da_seta(vec![b], [1., 0., 0.]);
        assert_eq!(e.passos[1].nome, "Passo 2");
    }

    #[test]
    fn passos_salvos_sobrevivem_a_abrir_e_fechar() {
        let (mut a, c, [_, b, t]) = pilha();
        let mut e = Explosao::nova("Vista 1");
        e.passos.push(linear(vec![t], [0., 0., 1.], 30.));
        e.passos.push(PassoExplosao { nome: "fila".into(), comps: vec![b, t], modo: ModoPasso::Espacado { dir: [1., 0., 0.], dist: 8. }, suprimido: true });
        e.passos.push(PassoExplosao { nome: "roda".into(), comps: vec![b], modo: ModoPasso::Radial { centro: [1., 2., 3.], eixo: [0., 1., 0.], dist: 4. }, suprimido: false });
        a.explosoes.push(e.clone());
        let mut estado = e.no_estado(&a.snapshot_state("explodida"), &c);
        estado.camera = None;
        a.display_states.push(estado.clone());
        let lida = Assembly::from_json(&a.to_json()).unwrap();
        assert_eq!(lida.explosoes, vec![e.clone()]);
        assert_eq!(lida.display_states[0].linhas, estado.linhas);
        assert_eq!(lida.explosoes[0].offsets(&c), e.offsets(&c));
    }

    #[test]
    fn arquivo_sem_explosao_abre_vazio() {
        let (a, ..) = pilha();
        let mut v: serde_json::Value = serde_json::from_str(&a.to_json()).unwrap();
        v.as_object_mut().unwrap().remove("explosoes");
        assert!(Assembly::from_json(&v.to_string()).unwrap().explosoes.is_empty());
    }

    #[test]
    fn peca_apagada_some_do_passo_sem_quebrar() {
        let (_, mut c, [_, b, t]) = pilha();
        let mut e = Explosao::nova("E");
        e.passos.push(linear(vec![b, t], [0., 0., 1.], 3.));
        c.remove(&b);
        let o = e.offsets(&c);
        assert_eq!(o.len(), 1);
        assert_eq!(o[0].0, t);
    }

    #[test]
    fn animacao_dos_passos_abre_um_de_cada_vez() {
        let (a, c, [_, b, t]) = pilha();
        let mut e = Explosao::nova("E");
        e.passos.push(linear(vec![t], [0., 0., 1.], 30.));
        e.passos.push(linear(vec![b], [0., 0., 1.], 10.));
        let an = e.animacao(&a.snapshot_state("base"), &c, 2.);
        assert_eq!(an.keys.len(), 3);
        assert_eq!(an.duration(), 4.);
        assert!(an.at(0.).unwrap().offsets.is_empty());
        let meio = an.at(2.).unwrap();
        assert_eq!(meio.offset(t), [0., 0., 30.]);
        assert_eq!(meio.offset(b), [0.; 3]);
        assert_eq!(an.at(4.).unwrap().offset(b), [0., 0., 10.]);
    }

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
