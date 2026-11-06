// SPDX-License-Identifier: LGPL-2.1-or-later
//! Árvore de caixas sobre os triângulos: ponto mais próximo, raio, ponto dentro e cruzamentos.
//!
//! Cada nó guarda a caixa dos seus triângulos e parte a lista pela mediana do maior eixo. Uma consulta
//! só desce nos nós cuja caixa ainda pode melhorar a resposta, e por isso custa cerca de log(n).

use crate::{add, cross, dot, mul, normal_area, sub, Malha, V3};

const FOLHA: usize = 4;

#[derive(Clone, Copy, Debug)]
struct Caixa {
    lo: V3,
    hi: V3,
}

impl Caixa {
    const VAZIA: Caixa = Caixa { lo: [f64::INFINITY; 3], hi: [f64::NEG_INFINITY; 3] };

    fn com(self, p: V3) -> Caixa {
        Caixa { lo: [0, 1, 2].map(|i| self.lo[i].min(p[i])), hi: [0, 1, 2].map(|i| self.hi[i].max(p[i])) }
    }

    fn uniao(self, o: Caixa) -> Caixa {
        self.com(o.lo).com(o.hi)
    }

    fn dist2(&self, p: V3) -> f64 {
        (0..3).map(|i| (self.lo[i] - p[i]).max(p[i] - self.hi[i]).max(0.).powi(2)).sum()
    }

    fn toca(&self, o: &Caixa) -> bool {
        (0..3).all(|i| self.lo[i] <= o.hi[i] && o.lo[i] <= self.hi[i])
    }

    /// O raio passa pela caixa?
    fn raio(&self, o: V3, inv: V3) -> bool {
        let (mut t0, mut t1) = (0f64, f64::INFINITY);
        for i in 0..3 {
            let (a, b) = ((self.lo[i] - o[i]) * inv[i], (self.hi[i] - o[i]) * inv[i]);
            // direção nula no eixo com a origem na face da caixa dá 0 vezes infinito
            let (a, b) = if a.is_nan() || b.is_nan() { (f64::NEG_INFINITY, f64::INFINITY) } else { (a.min(b), a.max(b)) };
            t0 = t0.max(a);
            t1 = t1.min(b);
        }
        t0 <= t1
    }
}

#[derive(Clone, Copy, Debug)]
enum Filhos {
    Folha { inicio: u32, fim: u32 },
    Ramo { esquerda: u32, direita: u32 },
}

#[derive(Clone, Copy, Debug)]
struct NoBvh {
    caixa: Caixa,
    filhos: Filhos,
}

/// O ponto da malha mais perto de uma consulta.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Perto {
    pub distancia: f64,
    pub triangulo: u32,
    pub ponto: V3,
}

#[derive(Clone, Debug)]
pub struct Bvh {
    tris: Vec<[V3; 3]>,
    /// índices de `tris` na ordem das folhas
    ordem: Vec<u32>,
    /// vértices de cada triângulo, para saber quem é vizinho
    indices: Vec<[u32; 3]>,
    nos: Vec<NoBvh>,
}

/// Ponto do triângulo mais perto de `p`, pelas regiões de Voronoi dos vértices e arestas.
pub fn ponto_no_triangulo(p: V3, [a, b, c]: [V3; 3]) -> V3 {
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0. && d2 <= 0. {
        return a;
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0. && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        return add(a, mul(ab, d1 / (d1 - d3)));
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0. && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return add(a, mul(ac, d2 / (d2 - d6)));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && d4 - d3 >= 0. && d5 - d6 >= 0. {
        return add(b, mul(sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6))));
    }
    let k = 1. / (va + vb + vc);
    add(a, add(mul(ab, vb * k), mul(ac, vc * k)))
}

/// Parâmetro do raio no triângulo (Möller e Trumbore). Raio paralelo ao plano não cruza.
fn raio_triangulo(o: V3, d: V3, [a, b, c]: [V3; 3]) -> Option<f64> {
    let (e1, e2) = (sub(b, a), sub(c, a));
    let h = cross(d, e2);
    let det = dot(e1, h);
    if det.abs() < 1e-18 {
        return None;
    }
    let s = sub(o, a);
    let u = dot(s, h) / det;
    if !(0. ..=1.).contains(&u) {
        return None;
    }
    let q = cross(s, e1);
    let v = dot(d, q) / det;
    if v < 0. || u + v > 1. {
        return None;
    }
    let t = dot(e2, q) / det;
    (t >= 0.).then_some(t)
}

/// Algum segmento de um triângulo fura o outro? Triângulos coplanares não contam: não há como dizer
/// se é cruzamento ou duas faces que só encostam.
pub fn triangulos_cruzam(a: [V3; 3], b: [V3; 3]) -> bool {
    let fura = |t: [V3; 3], o: [V3; 3]| {
        (0..3).any(|i| {
            let (p, q) = (t[i], t[(i + 1) % 3]);
            raio_triangulo(p, sub(q, p), o).is_some_and(|s| s > 1e-9 && s < 1. - 1e-9)
        })
    };
    fura(a, b) || fura(b, a)
}

impl Bvh {
    pub fn nova(m: &Malha) -> Bvh {
        let tris: Vec<[V3; 3]> = m.tris.iter().map(|t| t.map(|i| m.pos[i as usize])).collect();
        let mut b = Bvh { ordem: (0..tris.len() as u32).collect(), indices: m.tris.clone(), tris, nos: Vec::new() };
        if !b.tris.is_empty() {
            let n = b.ordem.len();
            b.construir(0, n);
        }
        b
    }

    fn caixa_de(&self, f: u32) -> Caixa {
        self.tris[f as usize].iter().fold(Caixa::VAZIA, |c, p| c.com(*p))
    }

    fn construir(&mut self, inicio: usize, fim: usize) -> u32 {
        let caixa = self.ordem[inicio..fim].iter().fold(Caixa::VAZIA, |c, f| c.uniao(self.caixa_de(*f)));
        let id = self.nos.len() as u32;
        self.nos.push(NoBvh { caixa, filhos: Filhos::Folha { inicio: inicio as u32, fim: fim as u32 } });
        if fim - inicio <= FOLHA {
            return id;
        }
        let tam = sub(caixa.hi, caixa.lo);
        let eixo = (0..3).fold(0, |m, i| if tam[i] > tam[m] { i } else { m });
        let meio = (fim - inicio) / 2;
        let tris = &self.tris;
        let centro = |f: u32| tris[f as usize].iter().map(|p| p[eixo]).sum::<f64>();
        self.ordem[inicio..fim].select_nth_unstable_by(meio, |a, b| centro(*a).total_cmp(&centro(*b)));
        let esquerda = self.construir(inicio, inicio + meio);
        let direita = self.construir(inicio + meio, fim);
        self.nos[id as usize].filhos = Filhos::Ramo { esquerda, direita };
        id
    }

    pub fn vazia(&self) -> bool {
        self.nos.is_empty()
    }

    /// Ponto da malha mais próximo de `p`.
    pub fn mais_proximo(&self, p: V3) -> Option<Perto> {
        if self.vazia() {
            return None;
        }
        let (mut melhor, mut achado) = (f64::INFINITY, None);
        let mut pilha = vec![0u32];
        while let Some(i) = pilha.pop() {
            let no = &self.nos[i as usize];
            if no.caixa.dist2(p) >= melhor {
                continue;
            }
            match no.filhos {
                Filhos::Folha { inicio, fim } => {
                    for &f in &self.ordem[inicio as usize..fim as usize] {
                        let q = ponto_no_triangulo(p, self.tris[f as usize]);
                        let d = sub(q, p);
                        let d2 = dot(d, d);
                        if d2 < melhor {
                            melhor = d2;
                            achado = Some((f, q));
                        }
                    }
                }
                Filhos::Ramo { esquerda, direita } => {
                    // desce primeiro no lado mais perto: a resposta boa cedo poda o resto
                    let (de, dd) = (self.nos[esquerda as usize].caixa.dist2(p), self.nos[direita as usize].caixa.dist2(p));
                    let (perto, longe) = if de < dd { (esquerda, direita) } else { (direita, esquerda) };
                    pilha.push(longe);
                    pilha.push(perto);
                }
            }
        }
        achado.map(|(triangulo, ponto)| Perto { distancia: melhor.sqrt(), triangulo, ponto })
    }

    /// Todos os cruzamentos do raio, com o triângulo, em ordem de distância.
    pub fn raio_todos(&self, o: V3, d: V3) -> Vec<(f64, u32)> {
        let mut saida = Vec::new();
        if self.vazia() {
            return saida;
        }
        let inv = d.map(|v| 1. / v);
        let mut pilha = vec![0u32];
        while let Some(i) = pilha.pop() {
            let no = &self.nos[i as usize];
            if !no.caixa.raio(o, inv) {
                continue;
            }
            match no.filhos {
                Filhos::Folha { inicio, fim } => {
                    for &f in &self.ordem[inicio as usize..fim as usize] {
                        if let Some(t) = raio_triangulo(o, d, self.tris[f as usize]) {
                            saida.push((t, f));
                        }
                    }
                }
                Filhos::Ramo { esquerda, direita } => {
                    pilha.push(esquerda);
                    pilha.push(direita);
                }
            }
        }
        saida.sort_by(|a, b| a.0.total_cmp(&b.0));
        saida
    }

    /// Primeiro triângulo que o raio acerta.
    pub fn raio(&self, o: V3, d: V3) -> Option<(f64, u32)> {
        self.raio_todos(o, d).first().copied()
    }

    /// O ponto está dentro de uma casca fechada? Soma o sinal de cada cruzamento num raio inclinado: a
    /// direção não é eixo nenhum para não passar exatamente por arestas de peças alinhadas.
    pub fn contem(&self, p: V3) -> bool {
        let d = [0.5773, 0.3137, 0.7539];
        let mut volta = 0i32;
        let mut anterior = f64::NEG_INFINITY;
        for (t, f) in self.raio_todos(p, d) {
            if t - anterior < 1e-12 {
                continue;
            }
            anterior = t;
            let [a, b, c] = self.tris[f as usize];
            volta += if dot(normal_area(a, b, c), d) > 0. { 1 } else { -1 };
        }
        volta != 0
    }

    /// Pares (triângulo daqui, triângulo de lá) cujas caixas se tocam e que se cruzam.
    pub fn cruzamentos(&self, outra: &Bvh) -> Vec<(u32, u32)> {
        let mut saida = Vec::new();
        if self.vazia() || outra.vazia() {
            return saida;
        }
        let mut pilha = vec![(0u32, 0u32)];
        while let Some((i, j)) = pilha.pop() {
            let (a, b) = (&self.nos[i as usize], &outra.nos[j as usize]);
            if !a.caixa.toca(&b.caixa) {
                continue;
            }
            match (a.filhos, b.filhos) {
                (Filhos::Folha { inicio: ai, fim: af }, Filhos::Folha { inicio: bi, fim: bf }) => {
                    for &f in &self.ordem[ai as usize..af as usize] {
                        for &g in &outra.ordem[bi as usize..bf as usize] {
                            if triangulos_cruzam(self.tris[f as usize], outra.tris[g as usize]) {
                                saida.push((f, g));
                            }
                        }
                    }
                }
                (Filhos::Ramo { esquerda, direita }, Filhos::Folha { .. }) => {
                    pilha.push((esquerda, j));
                    pilha.push((direita, j));
                }
                (Filhos::Folha { .. }, Filhos::Ramo { esquerda, direita }) => {
                    pilha.push((i, esquerda));
                    pilha.push((i, direita));
                }
                (Filhos::Ramo { esquerda: e1, direita: d1 }, Filhos::Ramo { esquerda: e2, direita: d2 }) => {
                    pilha.extend([(e1, e2), (e1, d2), (d1, e2), (d1, d2)]);
                }
            }
        }
        saida
    }

    /// Pares de triângulos da própria malha que se cruzam, sem contar vizinhos, que dividem um vértice.
    pub fn autointersecoes(&self) -> usize {
        let mut pares: Vec<(u32, u32)> = self
            .cruzamentos(self)
            .into_iter()
            .filter(|(f, g)| {
                let (a, b) = (self.indices[*f as usize], self.indices[*g as usize]);
                f != g && !a.iter().any(|v| b.contains(v))
            })
            .map(|(f, g)| (f.min(g), f.max(g)))
            .collect();
        pares.sort_unstable();
        pares.dedup();
        pares.len()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::teste::*;

    #[test]
    fn mais_proximo_na_esfera_cai_no_raio() {
        let b = Bvh::nova(&esfera(10., 48));
        for p in [[30., 0., 0.], [0., 25., 5.], [1., 1., 1.], [-7., 2., 40.]] {
            let r = b.mais_proximo(p).unwrap();
            let esperado = (crate::norma(p) - 10.).abs();
            assert!((r.distancia - esperado).abs() < 0.05, "{p:?}: {} e {esperado}", r.distancia);
        }
    }

    #[test]
    fn mais_proximo_bate_com_a_forca_bruta() {
        let m = esfera(5., 12);
        let b = Bvh::nova(&m);
        for i in 0..200 {
            let x = i as f64;
            let p = [(x * 0.37).sin() * 9., (x * 0.71).cos() * 9., (x * 1.13).sin() * 9.];
            let forca = m.tris.iter().map(|t| crate::norma(sub(ponto_no_triangulo(p, t.map(|i| m.pos[i as usize])), p)));
            let d = forca.fold(f64::INFINITY, f64::min);
            assert!((b.mais_proximo(p).unwrap().distancia - d).abs() < 1e-9);
        }
    }

    #[test]
    fn raio_e_dentro_da_caixa() {
        let b = Bvh::nova(&caixa([10., 10., 10.]));
        let (t, _) = b.raio([5.2, 5.1, -5.], [0., 0., 1.]).unwrap();
        assert!((t - 5.).abs() < 1e-9);
        assert_eq!(b.raio_todos([5.2, 5.1, -5.], [0., 0., 1.]).len(), 2);
        assert!(b.contem([5., 5., 5.]) && b.contem([0.1, 9.9, 0.1]));
        assert!(!b.contem([11., 5., 5.]) && !b.contem([-1., -1., -1.]));
    }

    #[test]
    fn so_acha_cruzamento_entre_quem_se_cruza() {
        let (a, mut c) = (caixa([10., 10., 10.]), caixa([10., 10., 10.]));
        c.pos.iter_mut().for_each(|p| p[0] += 50.);
        assert!(Bvh::nova(&a).cruzamentos(&Bvh::nova(&c)).is_empty());
        c.pos.iter_mut().for_each(|p| *p = [p[0] - 45., p[1] + 5., p[2] + 5.]);
        assert!(!Bvh::nova(&a).cruzamentos(&Bvh::nova(&c)).is_empty());
    }
}
