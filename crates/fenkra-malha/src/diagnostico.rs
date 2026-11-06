// SPDX-License-Identifier: LGPL-2.1-or-later
//! O que a malha tem de errado. Tudo sai das arestas: numa malha fechada e bem orientada cada aresta
//! tem duas faces, e as duas a percorrem em sentidos contrários.

use crate::bvh::Bvh;
use crate::{norma, normal_area, Malha};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Diagnostico {
    pub vertices: usize,
    pub triangulos: usize,
    /// triângulos de área nula, ou com vértice repetido
    pub degenerados: usize,
    /// triângulos com os mesmos três vértices de outro
    pub duplicados: usize,
    /// arestas de uma face só: as bordas dos furos
    pub arestas_livres: usize,
    /// arestas de três faces ou mais
    pub arestas_nao_manifold: usize,
    /// vértices onde duas partes se tocam só por um ponto
    pub vertices_nao_manifold: usize,
    /// arestas percorridas no mesmo sentido pelas duas faces: uma delas está virada
    pub arestas_invertidas: usize,
    pub furos: usize,
    pub componentes: usize,
    /// pares de triângulos que se cruzam; só conta quando pedido, porque custa uma busca por triângulo
    pub autointersecoes: Option<usize>,
    pub volume: f64,
    pub area: f64,
}

impl Diagnostico {
    /// Toda aresta tem duas faces em sentidos contrários e cada vértice é um leque só.
    pub fn estanque(&self) -> bool {
        self.triangulos > 0 && self.arestas_livres == 0 && self.arestas_nao_manifold == 0 && self.vertices_nao_manifold == 0 && self.arestas_invertidas == 0
    }

    /// Normais para dentro: volume negativo numa casca que fecha.
    pub fn virada_para_dentro(&self) -> bool {
        self.arestas_livres == 0 && self.volume < 0.
    }
}

/// Faces de cada aresta, com o sentido: `true` quando a face vai do menor vértice para o maior.
pub(crate) fn arestas(m: &Malha) -> HashMap<(u32, u32), Vec<(u32, bool)>> {
    let mut mapa: HashMap<(u32, u32), Vec<(u32, bool)>> = HashMap::with_capacity(m.tris.len() * 3 / 2);
    for (f, t) in m.tris.iter().enumerate() {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            if a != b {
                mapa.entry((a.min(b), a.max(b))).or_default().push((f as u32, a < b));
            }
        }
    }
    mapa
}

/// Arestas para desenhar: contorno aberto, não manifold e dobras acima de `angulo` graus. Com `so_borda`
/// fica só o contorno e o não manifold, que são defeito.
pub fn arestas_vivas(m: &Malha, angulo: f64, so_borda: bool) -> Vec<(u32, u32)> {
    let limite = angulo.to_radians().cos();
    let normal = |f: u32| {
        let [a, b, c] = m.tris[f as usize].map(|i| m.pos[i as usize]);
        crate::unit(normal_area(a, b, c))
    };
    let mut v: Vec<(u32, u32)> = arestas(m)
        .into_iter()
        .filter(|(_, faces)| match faces[..] {
            [(f, _), (g, _)] => !so_borda && crate::dot(normal(f), normal(g)) < limite,
            _ => true,
        })
        .map(|(k, _)| k)
        .collect();
    v.sort_unstable();
    v
}

/// Laços de borda: cada aresta livre aponta para a próxima pelo sentido da própria face.
pub(crate) fn lacos_de_borda(m: &Malha) -> Vec<Vec<u32>> {
    let mut proxima: HashMap<u32, Vec<u32>> = HashMap::new();
    for ((a, b), faces) in arestas(m) {
        if let [(_, direta)] = faces[..] {
            let (de, para) = if direta { (a, b) } else { (b, a) };
            proxima.entry(de).or_default().push(para);
        }
    }
    let mut inicios: Vec<u32> = proxima.keys().copied().collect();
    inicios.sort_unstable();
    let mut lacos = Vec::new();
    for inicio in inicios {
        while proxima.get(&inicio).is_some_and(|v| !v.is_empty()) {
            let mut laco = vec![inicio];
            let mut atual = inicio;
            while let Some(p) = proxima.get_mut(&atual).and_then(|v| v.pop()) {
                if p == inicio {
                    break;
                }
                // borda que volta a um vértice já visto: fecha o trecho como um laço à parte
                if let Some(k) = laco.iter().position(|v| *v == p) {
                    let resto = laco.split_off(k + 1);
                    let mut sub = vec![p];
                    sub.extend(resto);
                    lacos.push(sub);
                    atual = p;
                    continue;
                }
                laco.push(p);
                atual = p;
            }
            lacos.push(laco);
        }
    }
    lacos.retain(|l| l.len() >= 3);
    lacos
}

/// Número do componente de cada triângulo e quantos são. Duas faces que dividem um vértice são da
/// mesma peça, a mesma regra de `Malha::componentes`.
pub(crate) fn componentes_de(m: &Malha) -> (Vec<u32>, usize) {
    fn raiz(pai: &mut [u32], mut x: u32) -> u32 {
        while pai[x as usize] != x {
            pai[x as usize] = pai[pai[x as usize] as usize];
            x = pai[x as usize];
        }
        x
    }
    let mut pai: Vec<u32> = (0..m.tris.len() as u32).collect();
    let mut primeiro: Vec<u32> = vec![u32::MAX; m.pos.len()];
    for (f, t) in m.tris.iter().enumerate() {
        for &v in t {
            let p = primeiro[v as usize];
            if p == u32::MAX {
                primeiro[v as usize] = f as u32;
            } else {
                let (a, b) = (raiz(&mut pai, p), raiz(&mut pai, f as u32));
                pai[a as usize] = b;
            }
        }
    }
    let mut numero: HashMap<u32, u32> = HashMap::new();
    let ids = (0..m.tris.len() as u32)
        .map(|f| {
            let r = raiz(&mut pai, f);
            let n = numero.len() as u32;
            *numero.entry(r).or_insert(n)
        })
        .collect();
    (ids, numero.len())
}

/// Vértices cujas faces não formam um leque só.
fn vertices_em_leques_separados(m: &Malha, mapa: &HashMap<(u32, u32), Vec<(u32, bool)>>) -> usize {
    let mut faces: Vec<Vec<u32>> = vec![Vec::new(); m.pos.len()];
    for (f, t) in m.tris.iter().enumerate() {
        for &v in t {
            faces[v as usize].push(f as u32);
        }
    }
    let mut ruins = 0;
    for (v, lista) in faces.iter().enumerate() {
        if lista.len() < 2 {
            continue;
        }
        let v = v as u32;
        let mut vistos: HashSet<u32> = HashSet::from([lista[0]]);
        let mut pilha = vec![lista[0]];
        while let Some(f) = pilha.pop() {
            for &o in m.tris[f as usize].iter().filter(|o| **o != v) {
                if let Some(viz) = mapa.get(&(v.min(o), v.max(o))) {
                    for (g, _) in viz {
                        if vistos.insert(*g) {
                            pilha.push(*g);
                        }
                    }
                }
            }
        }
        ruins += usize::from(vistos.len() != lista.len());
    }
    ruins
}

impl Malha {
    /// Confere a malha como está: quem quer ignorar folgas solda antes.
    pub fn diagnostico(&self) -> Diagnostico {
        let mapa = arestas(self);
        let mut d = Diagnostico { vertices: self.pos.len(), triangulos: self.tris.len(), volume: self.volume(), area: self.area(), ..Default::default() };
        let escala = self.diagonal().max(1e-9);
        let mut vistos = HashSet::with_capacity(self.tris.len());
        for t in &self.tris {
            let [a, b, c] = t.map(|i| self.pos[i as usize]);
            if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] || norma(normal_area(a, b, c)) <= 1e-14 * escala * escala {
                d.degenerados += 1;
            }
            let mut k = *t;
            k.sort_unstable();
            d.duplicados += usize::from(!vistos.insert(k));
        }
        for faces in mapa.values() {
            match faces[..] {
                [_] => d.arestas_livres += 1,
                [(_, a), (_, b)] => d.arestas_invertidas += usize::from(a == b),
                _ => d.arestas_nao_manifold += 1,
            }
        }
        d.vertices_nao_manifold = vertices_em_leques_separados(self, &mapa);
        d.furos = lacos_de_borda(self).len();
        d.componentes = componentes_de(self).1;
        d
    }

    /// Como `diagnostico`, contando também os triângulos que se cruzam.
    pub fn diagnostico_completo(&self) -> Diagnostico {
        let mut d = self.diagnostico();
        d.autointersecoes = Some(Bvh::nova(self).autointersecoes());
        d
    }
}

#[cfg(test)]
mod testes {
    use crate::teste::*;
    use crate::Malha;

    #[test]
    fn caixa_sa_e_estanque() {
        let d = caixa([10., 20., 30.]).diagnostico_completo();
        assert!(d.estanque(), "{d:?}");
        assert_eq!((d.componentes, d.furos, d.autointersecoes), (1, 0, Some(0)));
        assert!((d.volume - 6000.).abs() < 1e-9 && (d.area - 2200.).abs() < 1e-9);
    }

    #[test]
    fn cada_defeito_aparece_na_conta_certa() {
        let mut m = caixa([10., 10., 10.]);
        m.tris.remove(0);
        let d = m.diagnostico();
        assert!(!d.estanque());
        assert_eq!((d.arestas_livres, d.furos), (3, 1));

        let mut m = caixa([10., 10., 10.]);
        m.tris[0].swap(1, 2);
        let d = m.diagnostico();
        assert_eq!(d.arestas_invertidas, 3, "{d:?}");

        // duas caixas pelo mesmo canto
        let a = caixa([10., 10., 10.]);
        let mut b = caixa([10., 10., 10.]);
        b.pos.iter_mut().for_each(|p| *p = p.map(|v| v + 10.));
        let d = crate::juntar([&a, &b]).soldada().diagnostico();
        assert_eq!((d.vertices_nao_manifold, d.componentes), (1, 1), "{d:?}");

        // três faces numa aresta
        let mut m = Malha { pos: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., -1., 0.], [0., 0., 1.]], ..Default::default() };
        m.tris = vec![[0, 1, 2], [1, 0, 3], [0, 1, 4]];
        assert_eq!(m.diagnostico().arestas_nao_manifold, 1);
    }

    #[test]
    fn caixas_que_se_cruzam_sao_autointersecao() {
        let a = caixa([10., 10., 10.]);
        let mut b = caixa([10., 10., 10.]);
        b.pos.iter_mut().for_each(|p| *p = p.map(|v| v + 5.));
        let d = crate::juntar([&a, &b]).diagnostico_completo();
        assert!(d.autointersecoes.unwrap() > 0, "{d:?}");
    }
}
