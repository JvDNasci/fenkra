// SPDX-License-Identifier: LGPL-2.1-or-later
//! Edição suave e seleção da gaiola (B302), sem interface.
//!
//! **Influência:** quem está perto do que foi puxado anda uma fração do deslocamento. A fração vem de
//! `x = distância / raio` pela curva de queda escolhida: 1 no centro, 0 no raio, e a derivada da curva
//! suave é nula nas duas pontas, para a transição não deixar degrau na superfície. Com "só conectados"
//! a distância é medida andando pelas arestas (Dijkstra), o que deixa de fora tanto a malha solta quanto
//! a outra asa da mesma peça que por acaso passa perto no espaço.
//!
//! **Seleção:** laço é a fileira de arestas que segue reto pelos nós de quatro faces; anel é a fileira
//! de arestas paralelas que um corte atravessa, que já existe em `laco_de`.

use crate::subd::*;
use crate::subd::{norma, par, sub};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap};

/// Curva de queda da influência.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Queda {
    /// sino liso: derivada nula no centro e no raio
    #[default]
    Suave,
    /// reta
    Linear,
    /// quarto de círculo: cheia no meio e cai de repente no raio
    Redonda,
    /// pico: só o centro anda de verdade
    Aguda,
    /// tudo dentro do raio anda igual
    Constante,
}

impl Queda {
    pub const TODAS: [Queda; 5] = [Queda::Suave, Queda::Linear, Queda::Redonda, Queda::Aguda, Queda::Constante];

    /// Fração do deslocamento em `x = distância / raio`.
    pub fn peso(self, x: f64) -> f64 {
        if x >= 1. {
            return 0.;
        }
        let x = x.max(0.);
        match self {
            Queda::Suave => {
                let s = 1. - x;
                s * s * (3. - 2. * s)
            }
            Queda::Linear => 1. - x,
            Queda::Redonda => (1. - x * x).sqrt(),
            Queda::Aguda => (1. - x) * (1. - x),
            Queda::Constante => 1.,
        }
    }
}

/// Alcance de um arraste.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Influencia {
    pub raio: f64,
    pub queda: Queda,
    pub so_conectados: bool,
}

/// Distância e número na fila de Dijkstra: o menor sai primeiro.
#[derive(PartialEq)]
struct Passo(f64, u32);
impl Eq for Passo {}
impl Ord for Passo {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        o.0.total_cmp(&self.0).then(o.1.cmp(&self.1))
    }
}
impl PartialOrd for Passo {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}

type Grafo = HashMap<u32, Vec<(u32, f64)>>;

/// Menor caminho a partir de `de`, sem passar de `limite`: a distância e de onde cada um veio.
fn caminhos(viz: &Grafo, de: &[u32], limite: f64) -> (HashMap<u32, f64>, HashMap<u32, u32>) {
    let (mut dist, mut veio, mut fila) = (HashMap::new(), HashMap::new(), BinaryHeap::new());
    for &n in de {
        dist.insert(n, 0.);
        fila.push(Passo(0., n));
    }
    while let Some(Passo(d, n)) = fila.pop() {
        if d > dist.get(&n).copied().unwrap_or(f64::MAX) {
            continue;
        }
        for &(m, w) in viz.get(&n).map_or(&[][..], |v| v) {
            let nd = d + w;
            if nd <= limite && nd < dist.get(&m).copied().unwrap_or(f64::MAX) {
                dist.insert(m, nd);
                veio.insert(m, n);
                fila.push(Passo(nd, m));
            }
        }
    }
    (dist, veio)
}

/// Caminho de `a` até `b` no grafo.
fn caminho(viz: &Grafo, a: u32, b: u32) -> Option<Vec<u32>> {
    let (_, veio) = caminhos(viz, &[a], f64::MAX);
    let mut out = vec![b];
    while *out.last()? != a {
        out.push(*veio.get(out.last()?)?);
    }
    out.reverse();
    Some(out)
}

impl Gaiola {
    /// Nós ligados a cada nó, com o comprimento da aresta.
    fn grafo_de_nos(&self) -> Grafo {
        let mut v = Grafo::new();
        for (a, b) in self.arestas() {
            let (Some(p), Some(q)) = (self.pos(a), self.pos(b)) else { continue };
            let w = norma(sub(p, q));
            v.entry(a).or_default().push((b, w));
            v.entry(b).or_default().push((a, w));
        }
        v
    }

    /// Fração que cada nó anda quando `nos` são puxados. Os puxados valem 1 e nó sem peso não aparece.
    pub fn pesos_influencia(&self, nos: &[u32], inf: &Influencia) -> BTreeMap<u32, f64> {
        let mut out: BTreeMap<u32, f64> = nos.iter().filter(|n| self.no(**n).is_some()).map(|n| (*n, 1.)).collect();
        if inf.raio <= 0. {
            return out;
        }
        let dist: HashMap<u32, f64> = if inf.so_conectados {
            caminhos(&self.grafo_de_nos(), nos, inf.raio).0
        } else {
            let fontes: Vec<V3> = nos.iter().filter_map(|n| self.pos(*n)).collect();
            self.nos.iter().map(|n| (n.id, fontes.iter().map(|p| norma(sub(*p, n.p))).fold(f64::MAX, f64::min))).collect()
        };
        for (n, d) in dist {
            let w = inf.queda.peso(d / inf.raio);
            if w > 0. {
                out.entry(n).or_insert(w);
            }
        }
        out
    }

    /// Arestas do laço que passa por `a`-`b`, em ordem: segue reto no nó de quatro faces e pela borda
    /// no nó de borda comum, e para em nó irregular.
    pub fn laco_de_arestas(&self, a: u32, b: u32) -> Result<Vec<(u32, u32)>, String> {
        let (t, idx) = self.topologia()?;
        let ids: Vec<u32> = self.nos.iter().map(|n| n.id).collect();
        let (ia, ib) = (*idx.get(&a).ok_or("nó inexistente")? as u32, *idx.get(&b).ok_or("nó inexistente")? as u32);
        if !t.vizinhos(ia).contains(&ib) {
            return Err("esses nós não formam uma aresta".into());
        }
        // o nó seguinte depois de `para`, vindo de `de`
        let reto = |de: u32, para: u32| -> Option<u32> {
            let v = t.vizinhos(para);
            let (_, fechado) = t.leque(para);
            let k = v.iter().position(|x| *x == de)?;
            match (fechado, v.len()) {
                (true, 4) => Some(v[(k + 2) % 4]),
                // borda comum: as pontas da lista são as duas arestas de borda; a do meio para
                (false, 3) if k != 1 => Some(v[2 - k]),
                _ => None,
            }
        };
        let andar = |mut de: u32, mut para: u32| -> (Vec<u32>, bool) {
            let (mut out, inicio) = (vec![], (de, para));
            while let Some(p) = reto(de, para) {
                de = para;
                para = p;
                if (de, para) == inicio {
                    return (out, true);
                }
                out.push(para);
                if out.len() > ids.len() {
                    break;
                }
            }
            (out, false)
        };
        let (frente, fechou) = andar(ia, ib);
        let mut nos = vec![ia, ib];
        nos.extend(&frente);
        if !fechou {
            let (mut atras, _) = andar(ib, ia);
            atras.reverse();
            atras.extend(nos);
            nos = atras;
        }
        Ok(nos.windows(2).map(|w| (ids[w[0] as usize], ids[w[1] as usize])).collect())
    }

    /// Menor caminho de arestas entre dois nós, as pontas incluídas.
    pub fn caminho_de_nos(&self, a: u32, b: u32) -> Option<Vec<u32>> {
        caminho(&self.grafo_de_nos(), a, b)
    }

    /// Faces vizinhas de cada face pela aresta, com a distância entre os centros.
    fn grafo_de_faces(&self) -> Grafo {
        let mut por_aresta: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
        for f in &self.faces {
            for (k, n) in f.nos.iter().enumerate() {
                por_aresta.entry(par(*n, f.nos[(k + 1) % f.nos.len()])).or_default().push(f.id);
            }
        }
        let mut v = Grafo::new();
        for l in por_aresta.values() {
            let [x, y] = l[..] else { continue };
            let (Some(p), Some(q)) = (self.centro_face(x), self.centro_face(y)) else { continue };
            let w = norma(sub(p, q));
            v.entry(x).or_default().push((y, w));
            v.entry(y).or_default().push((x, w));
        }
        v
    }

    /// Faixa de faces entre duas faces, as duas incluídas: o caminho mais curto pelas arestas.
    pub fn faixa_de_faces(&self, a: u32, b: u32) -> Option<Vec<u32>> {
        caminho(&self.grafo_de_faces(), a, b)
    }

    /// Faces que dividem uma aresta com `f`.
    pub fn faces_vizinhas(&self, f: u32) -> Vec<u32> {
        let mut v: Vec<u32> = self.grafo_de_faces().remove(&f).unwrap_or_default().into_iter().map(|x| x.0).collect();
        v.sort_unstable();
        v
    }

    /// A face do outro lado de `a`-`b`, olhando de `face`.
    pub fn face_oposta(&self, face: u32, a: u32, b: u32) -> Option<u32> {
        let e = par(a, b);
        self.faces.iter().find(|f| f.id != face && (0..f.nos.len()).any(|k| par(f.nos[k], f.nos[(k + 1) % f.nos.len()]) == e)).map(|f| f.id)
    }

    /// Nós a que `n` se liga.
    pub fn vizinhos_de(&self, n: u32) -> BTreeSet<u32> {
        self.arestas().into_iter().filter_map(|(a, b)| if a == n { Some(b) } else if b == n { Some(a) } else { None }).collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Grade de `n` por `n` quadrados de lado 10 em z = 0, com o canto em (0, 0).
    fn grade(n: u32) -> Gaiola {
        let l = 10. * n as f64;
        Gaiola::plano([l / 2., l / 2., 0.], [l, l], [n, n])
    }

    fn no_em(g: &Gaiola, p: V3) -> u32 {
        g.nos.iter().find(|n| norma(sub(n.p, p)) < 1e-9).expect("nó").id
    }

    fn folha_solta(g: &mut Gaiola) -> (Vec<u32>, u32) {
        let ids: Vec<u32> = [[0., 0., 5.], [20., 0., 5.], [20., 20., 5.], [0., 20., 5.]].map(|p| g.novo_para_teste(p)).to_vec();
        let f = g.face_para_teste(ids.clone());
        (ids, f)
    }

    #[test]
    fn curvas_vao_de_um_a_zero_e_a_suave_nao_tem_degrau() {
        for q in Queda::TODAS {
            assert_eq!(q.peso(0.), 1., "{q:?}");
            assert_eq!(q.peso(1.5), 0., "{q:?}");
            if q != Queda::Constante {
                assert!(q.peso(0.9999) < 0.05, "{q:?}");
            }
        }
        assert!((Queda::Suave.peso(0.5) - 0.5).abs() < 1e-12);
        assert!((Queda::Linear.peso(0.25) - 0.75).abs() < 1e-12);
        assert!((Queda::Redonda.peso(0.6) - 0.8).abs() < 1e-12);
        assert!((Queda::Aguda.peso(0.5) - 0.25).abs() < 1e-12);
        assert_eq!(Queda::Constante.peso(0.99), 1.);
        // a derivada da suave some nas duas pontas
        let h = 1e-5;
        assert!((Queda::Suave.peso(h) - 1.).abs() < 1e-8);
        assert!(Queda::Suave.peso(1. - h) < 1e-8);
    }

    #[test]
    fn peso_de_cada_vizinho_segue_a_curva() {
        let g = grade(4);
        let c = no_em(&g, [20., 20., 0.]);
        for q in Queda::TODAS {
            let w = g.pesos_influencia(&[c], &Influencia { raio: 25., queda: q, so_conectados: false });
            assert_eq!(w[&c], 1.);
            let v = no_em(&g, [30., 20., 0.]);
            assert!((w[&v] - q.peso(10. / 25.)).abs() < 1e-12, "{q:?}");
            let d = no_em(&g, [30., 30., 0.]);
            assert!((w[&d] - q.peso(200f64.sqrt() / 25.)).abs() < 1e-12, "{q:?}");
            // além do raio ninguém anda
            assert!(!w.contains_key(&no_em(&g, [0., 0., 0.])), "{q:?}");
        }
    }

    #[test]
    fn so_conectados_deixa_de_fora_a_malha_solta_e_mede_pelas_arestas() {
        let mut g = grade(2);
        let (ids, _) = folha_solta(&mut g);
        let c = no_em(&g, [10., 10., 0.]);
        let perto = Influencia { raio: 30., queda: Queda::Linear, so_conectados: false };
        assert!(g.pesos_influencia(&[c], &perto).contains_key(&ids[0]));
        let ligados = g.pesos_influencia(&[c], &Influencia { so_conectados: true, ..perto });
        assert!(ids.iter().all(|n| !ligados.contains_key(n)), "a folha solta não pode andar");
        // pela aresta o canto fica a 20, não a 14,1
        let canto = no_em(&g, [0., 0., 0.]);
        assert!((ligados[&canto] - (1. - 20. / 30.)).abs() < 1e-12);
    }

    #[test]
    fn laco_de_arestas_atravessa_a_grade_reto() {
        let g = grade(3);
        let (a, b) = (no_em(&g, [10., 10., 0.]), no_em(&g, [20., 10., 0.]));
        let l = g.laco_de_arestas(a, b).unwrap();
        assert_eq!(l.len(), 3);
        assert!(l.iter().flat_map(|(x, y)| [x, y]).all(|n| (g.pos(*n).unwrap()[1] - 10.).abs() < 1e-9));
    }

    #[test]
    fn laco_na_borda_segue_a_borda_e_no_cilindro_fecha_a_volta() {
        let g = grade(3);
        let (a, b) = (no_em(&g, [10., 0., 0.]), no_em(&g, [20., 0., 0.]));
        assert_eq!(g.laco_de_arestas(a, b).unwrap().len(), 3, "para nos cantos");
        let c = Gaiola::cilindro([0.; 3], 10., 20., 6, 2);
        let z = |n: u32| c.pos(n).unwrap()[2];
        let zm = c.nos.iter().map(|x| x.p[2]).sum::<f64>() / c.nos.len() as f64;
        let m = c.nos.iter().find(|n| (n.p[2] - zm).abs() < 1e-9).unwrap().id;
        let viz = c.vizinhos_de(m).into_iter().find(|n| (z(*n) - z(m)).abs() < 1e-9).unwrap();
        let l = c.laco_de_arestas(m, viz).unwrap();
        assert_eq!(l.len(), 6, "{l:?}");
        assert_eq!(l[0].0, l[5].1, "fecha");
    }

    #[test]
    fn laco_para_no_no_irregular() {
        // o canto da caixa tem três faces
        let g = Gaiola::caixa([0.; 3], [20.; 3], [2, 2, 2]);
        let n = g.nos.iter().find(|n| n.p[1] == -10. && n.p[0] == 0. && n.p[2] == 10.).unwrap().id;
        let m = g.nos.iter().find(|n| n.p[1] == -10. && n.p[0] == 10. && n.p[2] == 10.).unwrap().id;
        let l = g.laco_de_arestas(n, m).unwrap();
        assert_eq!(l.len(), 2, "{l:?}");
    }

    #[test]
    fn faixa_liga_duas_faces_e_caminho_liga_dois_nos() {
        let g = grade(4);
        let face_em = |x: f64, y: f64| {
            g.faces.iter().find(|f| {
                let c = g.centro_face(f.id).unwrap();
                (c[0] - x).abs() < 1. && (c[1] - y).abs() < 1.
            }).unwrap().id
        };
        let f = g.faixa_de_faces(face_em(5., 5.), face_em(35., 5.)).unwrap();
        assert_eq!(f.len(), 4);
        assert!(f.iter().all(|x| (g.centro_face(*x).unwrap()[1] - 5.).abs() < 1e-9));
        assert_eq!(g.caminho_de_nos(no_em(&g, [0., 0., 0.]), no_em(&g, [40., 0., 0.])).unwrap().len(), 5);
    }

    #[test]
    fn faixa_entre_malhas_soltas_nao_existe() {
        let mut g = grade(1);
        let (_, solta) = folha_solta(&mut g);
        assert!(g.faixa_de_faces(g.faces[0].id, solta).is_none());
    }
}
