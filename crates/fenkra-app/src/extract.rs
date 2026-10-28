//! Extrair com propagação: a partir de uma aresta ou face clicada, junta as vizinhas que
//! continuam por ponto (G0), tangente (G1) ou curvatura (G2), olhando a malha do corpo.

use fenkra_kernel::Mesh;
use std::collections::{HashMap, HashSet, VecDeque};

type V3 = [f64; 3];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Prop {
    /// só o que foi clicado
    #[default]
    Nenhuma,
    /// tudo o que está ligado (ponto)
    Ponto,
    /// continua enquanto a tangente for contínua
    Tangente,
    /// tangente e curvatura contínuas
    Curvatura,
}

fn key(p: V3) -> [i64; 3] {
    p.map(|v| (v * 1e3).round() as i64)
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn unit(a: V3) -> V3 {
    let l = dot(a, a).sqrt().max(1e-12);
    a.map(|v| v / l)
}

fn edges(m: &Mesh) -> Vec<Vec<V3>> {
    let nseg = m.edges.len() / 6;
    (0..m.edge_ranges.len())
        .map(|e| {
            let s0 = m.edge_ranges[e] as usize;
            let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(nseg);
            let mut out: Vec<V3> = vec![];
            for g in m.edges.chunks_exact(6).take(s1).skip(s0) {
                if out.is_empty() {
                    out.push([g[0] as f64, g[1] as f64, g[2] as f64]);
                }
                out.push([g[3] as f64, g[4] as f64, g[5] as f64]);
            }
            out
        })
        .collect()
}

/// Tangente e curvatura saindo da ponta `at_end` da polilinha (para fora dela).
fn end_props(p: &[V3], at_end: bool) -> (V3, f64) {
    let q: Vec<V3> = if at_end { p.to_vec() } else { p.iter().rev().copied().collect() };
    let n = q.len();
    let d2 = unit(sub(q[n - 1], q[n - 2]));
    if n < 3 {
        return (d2, 0.);
    }
    let d1 = unit(sub(q[n - 2], q[n - 3]));
    // a corda aponta para o meio do trecho: extrapola meio passo até a ponta
    let t = unit([0, 1, 2].map(|k| 1.5 * d2[k] - 0.5 * d1[k]));
    let h = (dot(sub(q[n - 1], q[n - 2]), sub(q[n - 1], q[n - 2])).sqrt() + dot(sub(q[n - 2], q[n - 3]), sub(q[n - 2], q[n - 3])).sqrt()) / 2.;
    let k = dot(d1, d2).clamp(-1., 1.).acos() / h.max(1e-12);
    (t, k)
}

/// Arestas do corpo que continuam a aresta `start` pelo critério escolhido.
pub fn edge_set(m: &Mesh, start: usize, prop: Prop) -> Vec<usize> {
    let es = edges(m);
    if prop == Prop::Nenhuma || start >= es.len() {
        return vec![start];
    }
    let mut at: HashMap<[i64; 3], Vec<(usize, bool)>> = HashMap::new();
    for (i, e) in es.iter().enumerate() {
        if e.len() >= 2 {
            at.entry(key(e[0])).or_default().push((i, false));
            at.entry(key(*e.last().unwrap())).or_default().push((i, true));
        }
    }
    let mut seen: HashSet<usize> = HashSet::from([start]);
    let mut q = VecDeque::from([start]);
    while let Some(e) = q.pop_front() {
        let p = &es[e];
        if p.len() < 2 {
            continue;
        }
        for at_end in [false, true] {
            let tip = if at_end { *p.last().unwrap() } else { p[0] };
            let (t, k) = end_props(p, at_end);
            for &(nb, nb_end) in at.get(&key(tip)).map(|v| v.as_slice()).unwrap_or(&[]) {
                if seen.contains(&nb) {
                    continue;
                }
                let ok = match prop {
                    Prop::Ponto => true,
                    _ => {
                        // a vizinha sai do ponto comum: o oposto da tangente "para fora" dela
                        let (tn, kn) = end_props(&es[nb], nb_end);
                        let tang = dot(t, tn.map(|v| -v)) > 0.9986;
                        tang && (prop == Prop::Tangente || (k - kn).abs() <= 0.15 * k.max(kn) + 1e-6)
                    }
                };
                if ok {
                    seen.insert(nb);
                    q.push_back(nb);
                }
            }
        }
    }
    let mut v: Vec<usize> = seen.into_iter().collect();
    v.sort_unstable();
    v
}

/// Faces do corpo ligadas à face `start` (ponto) ou que continuam tangentes a ela.
pub fn face_set(m: &Mesh, start: usize, prop: Prop) -> Vec<usize> {
    let nf = m.face_ranges.len();
    if prop == Prop::Nenhuma || start >= nf {
        return vec![start];
    }
    let g = |a: &[f32], i: u32| [a[i as usize * 3] as f64, a[i as usize * 3 + 1] as f64, a[i as usize * 3 + 2] as f64];
    // posição → (face, normal) de cada vértice das faces
    let mut at: HashMap<[i64; 3], Vec<(usize, V3)>> = HashMap::new();
    let mut verts: Vec<Vec<[i64; 3]>> = vec![vec![]; nf];
    for f in 0..nf {
        let s0 = m.face_ranges[f] as usize;
        let s1 = m.face_ranges.get(f + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
        let mut vs: Vec<u32> = m.indices[s0.min(s1)..s1].to_vec();
        vs.sort_unstable();
        vs.dedup();
        for v in vs {
            let k = key(g(&m.positions, v));
            let n = if m.normals.len() >= m.positions.len() { unit(g(&m.normals, v)) } else { [0.; 3] };
            at.entry(k).or_default().push((f, n));
            verts[f].push(k);
        }
    }
    let mut seen: HashSet<usize> = HashSet::from([start]);
    let mut q = VecDeque::from([start]);
    while let Some(f) = q.pop_front() {
        // vizinhas: compartilham pelo menos dois vértices (uma aresta)
        let mut shared: HashMap<usize, (usize, bool)> = HashMap::new();
        for k in &verts[f] {
            let here = &at[k];
            let mine = here.iter().find(|(x, _)| *x == f).map(|(_, n)| *n).unwrap_or([0.; 3]);
            for (o, n) in here {
                if *o == f || seen.contains(o) {
                    continue;
                }
                let e = shared.entry(*o).or_insert((0, true));
                e.0 += 1;
                if dot(mine, *n) < 0.999 {
                    e.1 = false;
                }
            }
        }
        for (o, (count, smooth)) in shared {
            if count >= 2 && (prop == Prop::Ponto || smooth) {
                seen.insert(o);
                q.push_back(o);
            }
        }
    }
    let mut v: Vec<usize> = seen.into_iter().collect();
    v.sort_unstable();
    v
}

/// Um ponto sobre a face (centro do primeiro triângulo), para guardar a face pela posição.
pub fn face_point(m: &Mesh, f: usize) -> Option<V3> {
    let s0 = *m.face_ranges.get(f)? as usize;
    let t = m.indices.get(s0..s0 + 3)?;
    let g = |i: u32| [m.positions[i as usize * 3] as f64, m.positions[i as usize * 3 + 1] as f64, m.positions[i as usize * 3 + 2] as f64];
    let (a, b, c) = (g(t[0]), g(t[1]), g(t[2]));
    Some([0, 1, 2].map(|k| (a[k] + b[k] + c[k]) / 3.))
}

/// Polilinha de uma aresta.
pub fn edge_pts(m: &Mesh, e: usize) -> Vec<V3> {
    edges(m).into_iter().nth(e).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh_with_edges(polys: &[Vec<V3>]) -> Mesh {
        let mut m = Mesh::default();
        for p in polys {
            m.edge_ranges.push((m.edges.len() / 6) as u32);
            for w in p.windows(2) {
                m.edges.extend([w[0][0] as f32, w[0][1] as f32, w[0][2] as f32, w[1][0] as f32, w[1][1] as f32, w[1][2] as f32]);
            }
        }
        m
    }

    #[test]
    fn propaga_por_tangente_e_ponto() {
        // linha → arco tangente (raio 10) → linha tangente na saída do arco
        let arc: Vec<V3> = (0..=16).map(|i| {
            let t = -std::f64::consts::FRAC_PI_2 + i as f64 / 16. * std::f64::consts::FRAC_PI_2;
            [20. + 10. * t.cos(), 10. + 10. * t.sin(), 0.]
        }).collect();
        let tip = *arc.last().unwrap();
        let m = mesh_with_edges(&[vec![[0., 0., 0.], [20., 0., 0.]], arc, vec![tip, [tip[0], tip[1] + 15., 0.]]]);
        assert_eq!(edge_set(&m, 0, Prop::Nenhuma), vec![0]);
        assert_eq!(edge_set(&m, 0, Prop::Tangente), vec![0, 1, 2], "o arco é tangente às duas linhas");
        assert_eq!(edge_set(&m, 0, Prop::Curvatura), vec![0], "linha → arco muda a curvatura");
        assert_eq!(edge_set(&m, 0, Prop::Ponto), vec![0, 1, 2]);
        let m2 = mesh_with_edges(&[vec![[0., 0., 0.], [10., 0., 0.]], vec![[10., 0., 0.], [10., 10., 0.]]]);
        assert_eq!(edge_set(&m2, 0, Prop::Tangente), vec![0], "canto vivo para a propagação tangente");
        assert_eq!(edge_set(&m2, 0, Prop::Ponto), vec![0, 1]);
    }
}
