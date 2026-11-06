// SPDX-License-Identifier: LGPL-2.1-or-later
//! Decimação por contração de arestas com erro quadrático (Garland e Heckbert, 1997).
//!
//! Cada vértice acumula os planos das faces em volta numa quádrica `Q`: `v'Qv` é a soma dos quadrados
//! das distâncias de `v` a esses planos. Contrair a aresta `a`-`b` junta as quádricas e põe o vértice
//! novo onde `Q` é mínima. As arestas ficam num monte pelo custo, e sai sempre a mais barata.
//!
//! Três guardas mantêm a malha sã: a condição do elo (os vizinhos comuns de `a` e `b` são só os dois
//! vértices opostos da aresta, senão a contração colapsa um túnel), nenhuma face pode virar e arestas
//! de borda ganham planos perpendiculares pesados, para o contorno aberto não encolher.

use crate::{add, cross, dot, mul, norma, normal_area, sub, unit, Malha, V3};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpcoesDecimar {
    /// fração dos triângulos que fica, de 0 a 1
    pub proporcao: f64,
    /// para antes se a próxima contração se afastar mais que isto da superfície, em mm; zero desliga
    pub desvio: f64,
}

impl Default for OpcoesDecimar {
    fn default() -> Self {
        OpcoesDecimar { proporcao: 0.5, desvio: 0. }
    }
}

/// Matriz simétrica 4x4 guardada em dez números.
#[derive(Clone, Copy, Debug, Default)]
struct Quadrica([f64; 10]);

impl Quadrica {
    fn do_plano(n: V3, d: f64, peso: f64) -> Quadrica {
        let [a, b, c] = n;
        Quadrica([a * a, a * b, a * c, a * d, b * b, b * c, b * d, c * c, c * d, d * d].map(|v| v * peso))
    }

    fn mais(&self, o: &Quadrica) -> Quadrica {
        let mut r = self.0;
        r.iter_mut().zip(o.0).for_each(|(x, y)| *x += y);
        Quadrica(r)
    }

    fn erro(&self, v: V3) -> f64 {
        let q = &self.0;
        let [x, y, z] = v;
        (q[0] * x * x + 2. * q[1] * x * y + 2. * q[2] * x * z + 2. * q[3] * x + q[4] * y * y + 2. * q[5] * y * z + 2. * q[6] * y + q[7] * z * z + 2. * q[8] * z + q[9]).max(0.)
    }

    /// Onde a quádrica é mínima, se o sistema não for quase singular.
    fn minimo(&self) -> Option<V3> {
        let q = &self.0;
        let (a, b, c, d, e, f) = (q[0], q[1], q[2], q[4], q[5], q[7]);
        let det = a * (d * f - e * e) - b * (b * f - e * c) + c * (b * e - d * c);
        let escala = (a + d + f).powi(3);
        if det.abs() <= 1e-10 * escala.abs().max(1e-300) {
            return None;
        }
        let r = [-q[3], -q[6], -q[8]];
        let x = (r[0] * (d * f - e * e) - b * (r[1] * f - e * r[2]) + c * (r[1] * e - d * r[2])) / det;
        let y = (a * (r[1] * f - e * r[2]) - r[0] * (b * f - e * c) + c * (b * r[2] - r[1] * c)) / det;
        let z = (a * (d * r[2] - r[1] * e) - b * (b * r[2] - r[1] * c) + r[0] * (b * e - d * c)) / det;
        Some([x, y, z])
    }
}

#[derive(Clone, Copy, Debug)]
struct Candidata {
    custo: f64,
    a: u32,
    b: u32,
    /// versão dos dois vértices quando o custo foi calculado: mudou, o custo está velho
    versao: (u32, u32),
    lugar: V3,
}

impl PartialEq for Candidata {
    fn eq(&self, o: &Self) -> bool {
        self.custo == o.custo
    }
}
impl Eq for Candidata {}
impl PartialOrd for Candidata {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Candidata {
    fn cmp(&self, o: &Self) -> Ordering {
        // ao contrário: o monte de Rust solta o maior
        o.custo.total_cmp(&self.custo).then(o.a.cmp(&self.a)).then(o.b.cmp(&self.b))
    }
}

struct Estado {
    pos: Vec<V3>,
    tris: Vec<[u32; 3]>,
    viva: Vec<bool>,
    /// triângulos de cada vértice
    faces: Vec<Vec<u32>>,
    q: Vec<Quadrica>,
    versao: Vec<u32>,
    /// vértice no contorno aberto
    borda: Vec<bool>,
    monte: BinaryHeap<Candidata>,
}

impl Estado {
    fn vizinhos(&self, v: u32) -> Vec<u32> {
        let mut n: Vec<u32> = self.faces[v as usize].iter().flat_map(|f| self.tris[*f as usize]).filter(|x| *x != v).collect();
        n.sort_unstable();
        n.dedup();
        n
    }

    fn candidata(&self, a: u32, b: u32) -> Candidata {
        let (a, b) = (a.min(b), a.max(b));
        let q = self.q[a as usize].mais(&self.q[b as usize]);
        let (pa, pb) = (self.pos[a as usize], self.pos[b as usize]);
        let meio = mul(add(pa, pb), 0.5);
        let lugar = q.minimo().filter(|p| {
            // longe da aresta o mínimo não é confiável: quádrica quase plana manda o ponto para longe
            let l = norma(sub(pa, pb));
            norma(sub(*p, meio)) <= 2. * l
        });
        let lugar = lugar.unwrap_or_else(|| [pa, pb, meio].into_iter().min_by(|x, y| q.erro(*x).total_cmp(&q.erro(*y))).unwrap_or(meio));
        Candidata { custo: q.erro(lugar), a, b, versao: (self.versao[a as usize], self.versao[b as usize]), lugar }
    }

    /// Só os dois vértices opostos da aresta são vizinhos comuns dos dois lados.
    fn elo_ok(&self, a: u32, b: u32) -> bool {
        let (na, nb) = (self.vizinhos(a), self.vizinhos(b));
        let comuns = na.iter().filter(|v| nb.binary_search(v).is_ok()).count();
        let opostos = self.faces[a as usize].iter().filter(|f| self.tris[**f as usize].contains(&b)).count();
        comuns == opostos
    }

    /// Mover `v` para `lugar` vira alguma face que não some na contração?
    fn vira_face(&self, v: u32, outro: u32, lugar: V3) -> bool {
        self.faces[v as usize].iter().any(|f| {
            let t = self.tris[*f as usize];
            if t.contains(&outro) {
                return false;
            }
            let [a, b, c] = t.map(|i| self.pos[i as usize]);
            let antes = normal_area(a, b, c);
            let [d, e, g] = t.map(|i| if i == v { lugar } else { self.pos[i as usize] });
            let depois = normal_area(d, e, g);
            let (na, nd) = (norma(antes), norma(depois));
            nd <= 1e-12 * na.max(1e-300) || dot(antes, depois) < 0.2 * na * nd
        })
    }

    fn contrair(&mut self, c: &Candidata) {
        let (a, b) = (c.a, c.b);
        self.pos[a as usize] = c.lugar;
        self.q[a as usize] = self.q[a as usize].mais(&self.q[b as usize]);
        self.borda[a as usize] |= self.borda[b as usize];
        let faces_b = std::mem::take(&mut self.faces[b as usize]);
        for f in faces_b {
            let t = &mut self.tris[f as usize];
            if t.contains(&a) {
                self.viva[f as usize] = false;
                let t = *t;
                for v in t {
                    if v != b {
                        self.faces[v as usize].retain(|x| *x != f);
                    }
                }
            } else {
                t.iter_mut().for_each(|v| {
                    if *v == b {
                        *v = a;
                    }
                });
                self.faces[a as usize].push(f);
            }
        }
        self.versao[a as usize] += 1;
        self.versao[b as usize] += 1;
        for n in self.vizinhos(a) {
            self.versao[n as usize] += 1;
        }
        for n in self.vizinhos(a) {
            let nova = self.candidata(a, n);
            self.monte.push(nova);
            for m in self.vizinhos(n) {
                if m != a {
                    let nova = self.candidata(n, m);
                    self.monte.push(nova);
                }
            }
        }
    }
}

fn montar(m: &Malha) -> Estado {
    let mut e = Estado { pos: m.pos.clone(), tris: m.tris.clone(), viva: vec![true; m.tris.len()], faces: vec![Vec::new(); m.pos.len()], q: vec![Quadrica::default(); m.pos.len()], versao: vec![0; m.pos.len()], borda: vec![false; m.pos.len()], monte: BinaryHeap::new() };
    let mut uso: HashMap<(u32, u32), (u32, u32)> = HashMap::with_capacity(m.tris.len() * 3 / 2);
    for (f, t) in m.tris.iter().enumerate() {
        let [a, b, c] = t.map(|i| m.pos[i as usize]);
        let n = normal_area(a, b, c);
        let area = norma(n) / 2.;
        if area > 0. {
            let n = unit(n);
            let q = Quadrica::do_plano(n, -dot(n, a), area);
            for &v in t {
                e.q[v as usize] = e.q[v as usize].mais(&q);
            }
        }
        for &v in t {
            e.faces[v as usize].push(f as u32);
        }
        for (x, y) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let k = uso.entry((x.min(y), x.max(y))).or_insert((0, f as u32));
            k.0 += 1;
        }
    }
    let diagonal = m.diagonal().max(1e-9);
    for (&(x, y), &(vezes, f)) in &uso {
        if vezes != 1 {
            continue;
        }
        e.borda[x as usize] = true;
        e.borda[y as usize] = true;
        let t = m.tris[f as usize];
        let [a, b, c] = t.map(|i| m.pos[i as usize]);
        let n = unit(normal_area(a, b, c));
        let (p, q) = (m.pos[x as usize], m.pos[y as usize]);
        let lado = unit(cross(sub(q, p), n));
        let plano = Quadrica::do_plano(lado, -dot(lado, p), diagonal * diagonal);
        e.q[x as usize] = e.q[x as usize].mais(&plano);
        e.q[y as usize] = e.q[y as usize].mais(&plano);
    }
    let pares: Vec<(u32, u32)> = uso.keys().copied().collect();
    for (x, y) in pares {
        let c = e.candidata(x, y);
        e.monte.push(c);
    }
    e
}

impl Malha {
    /// Malha com cerca de `proporcao` dos triângulos. A entrada precisa estar soldada: triângulo solto
    /// não tem vizinho para contrair.
    pub fn decimada(&self, o: &OpcoesDecimar) -> Malha {
        let base = self.soldada();
        let alvo = ((base.tris.len() as f64 * o.proporcao.clamp(0., 1.)).round() as usize).max(4);
        let mut e = montar(&base);
        let mut vivos = base.tris.len();
        // o custo é área vezes distância ao quadrado, e o que limita é a distância pura
        let area_tipica = base.area() / base.tris.len().max(1) as f64;
        let teto = if o.desvio > 0. { o.desvio * o.desvio * area_tipica } else { f64::INFINITY };
        while vivos > alvo {
            let Some(c) = e.monte.pop() else { break };
            if c.versao != (e.versao[c.a as usize], e.versao[c.b as usize]) || e.faces[c.a as usize].is_empty() || e.faces[c.b as usize].is_empty() {
                continue;
            }
            if c.custo > teto {
                break;
            }
            if !e.elo_ok(c.a, c.b) || e.vira_face(c.a, c.b, c.lugar) || e.vira_face(c.b, c.a, c.lugar) {
                continue;
            }
            let antes = e.faces[c.a as usize].iter().filter(|f| e.tris[**f as usize].contains(&c.b)).count();
            if antes == 0 {
                continue;
            }
            e.contrair(&c);
            vivos -= antes;
        }
        let mut novo = vec![u32::MAX; e.pos.len()];
        let mut m = Malha { nome: self.nome.clone(), material: self.material, ..Default::default() };
        for (f, t) in e.tris.iter().enumerate() {
            if !e.viva[f] {
                continue;
            }
            m.tris.push(t.map(|v| {
                if novo[v as usize] == u32::MAX {
                    novo[v as usize] = m.pos.len() as u32;
                    m.pos.push(e.pos[v as usize]);
                }
                novo[v as usize]
            }));
        }
        m
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::desvio::hausdorff;
    use crate::teste::*;

    #[test]
    fn esfera_cai_para_a_proporcao_e_continua_fechada_e_perto() {
        let e = esfera(10., 48);
        let n = e.tris.len();
        let r = e.decimada(&OpcoesDecimar { proporcao: 0.1, desvio: 0. });
        assert!(r.tris.len() <= n / 10 + 2, "{} de {n}", r.tris.len());
        assert!(r.tris.len() > n / 12, "{} de {n}", r.tris.len());
        assert!(r.diagnostico().estanque(), "{:?}", r.diagnostico());
        assert!((r.volume() - e.volume()).abs() / e.volume() < 0.02);
        assert!(hausdorff(&e, &r).maximo < 0.3);
    }

    #[test]
    fn plano_subdividido_cai_ate_o_contorno_sem_mudar_a_area() {
        let n = 40u32;
        let mut m = Malha::default();
        for j in 0..=n {
            for i in 0..=n {
                m.pos.push([f64::from(i), f64::from(j), 0.]);
            }
        }
        let k = |i: u32, j: u32| j * (n + 1) + i;
        for j in 0..n {
            for i in 0..n {
                m.tris.push([k(i, j), k(i + 1, j), k(i + 1, j + 1)]);
                m.tris.push([k(i, j), k(i + 1, j + 1), k(i, j + 1)]);
            }
        }
        let r = m.decimada(&OpcoesDecimar { proporcao: 0.05, desvio: 0. });
        assert!(r.tris.len() * 10 < m.tris.len(), "{}", r.tris.len());
        assert!((r.area() - 1600.).abs() < 1e-6, "o contorno andou: {}", r.area());
        assert_eq!(r.caixa(), m.caixa());
    }

    #[test]
    fn teto_de_desvio_segura_a_forma() {
        let e = esfera(10., 32);
        let solta = e.decimada(&OpcoesDecimar { proporcao: 0.02, desvio: 0. });
        let presa = e.decimada(&OpcoesDecimar { proporcao: 0.02, desvio: 0.02 });
        assert!(presa.tris.len() > solta.tris.len() * 2, "{} e {}", presa.tris.len(), solta.tris.len());
        assert!(hausdorff(&e, &presa).maximo < hausdorff(&e, &solta).maximo);
    }

    #[test]
    fn caixa_nao_vira_outra_coisa() {
        let c = caixa([10., 10., 10.]);
        let r = c.decimada(&OpcoesDecimar { proporcao: 0.01, desvio: 0. });
        assert!(r.tris.len() >= 4);
        assert!(r.diagnostico().estanque());
    }
}
