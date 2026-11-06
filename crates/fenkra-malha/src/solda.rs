// SPDX-License-Identifier: LGPL-2.1-or-later
//! Solda com tolerância e simplificação grosseira por grade.
//!
//! `Malha::soldada` junta só vértices idênticos. Aqui a tolerância é uma distância: cada vértice cai
//! numa célula da grade do tamanho da tolerância e só procura par nas 27 células em volta, o que deixa
//! o custo linear. O primeiro vértice de cada grupo fica como está, então a malha não anda.

use crate::{dot, sub, Malha};
use std::collections::HashMap;

impl Malha {
    /// Vértices a menos de `tol` viram um só. Com `tol` zero, é a solda exata.
    pub fn soldada_com(&self, tol: f64) -> Malha {
        if tol <= 0. {
            return self.soldada();
        }
        let chave = |p: [f64; 3]| p.map(|v| (v / tol).floor() as i64);
        let mut grade: HashMap<[i64; 3], Vec<u32>> = HashMap::new();
        let mut m = Malha { nome: self.nome.clone(), material: self.material, ..Default::default() };
        let mut novos: Vec<u32> = Vec::with_capacity(self.pos.len());
        'vertices: for (i, p) in self.pos.iter().enumerate() {
            let c = chave(*p);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let Some(lista) = grade.get(&[c[0] + dx, c[1] + dy, c[2] + dz]) else { continue };
                        for &q in lista {
                            let d = sub(m.pos[q as usize], *p);
                            if dot(d, d) <= tol * tol {
                                novos.push(q);
                                continue 'vertices;
                            }
                        }
                    }
                }
            }
            let novo = m.pos.len() as u32;
            m.pos.push(*p);
            if let Some(cor) = self.cor_v.get(i) {
                m.cor_v.push(*cor);
            }
            grade.entry(c).or_default().push(novo);
            novos.push(novo);
        }
        m.tris = self.tris.iter().map(|t| t.map(|i| novos[i as usize])).filter(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2]).collect();
        m
    }

    /// Cópia grosseira para miniatura: todos os vértices de uma célula viram a média deles. É linear,
    /// serve para milhões de triângulos e não preserva a topologia, por isso nunca vai para o modelo.
    pub fn grosseira(&self, celulas: usize) -> Malha {
        let Some((lo, hi)) = self.caixa() else { return Malha::default() };
        let passo = [0, 1, 2].map(|i| hi[i] - lo[i]).into_iter().fold(0., f64::max).max(1e-9) / celulas.max(1) as f64;
        let mut grupos: HashMap<[i64; 3], u32> = HashMap::new();
        let mut somas: Vec<([f64; 3], f64)> = Vec::new();
        let mut novos: Vec<u32> = Vec::with_capacity(self.pos.len());
        for p in &self.pos {
            let c = [0, 1, 2].map(|i| ((p[i] - lo[i]) / passo).floor() as i64);
            let g = *grupos.entry(c).or_insert_with(|| {
                somas.push(([0.; 3], 0.));
                (somas.len() - 1) as u32
            });
            let s = &mut somas[g as usize];
            s.0 = crate::add(s.0, *p);
            s.1 += 1.;
            novos.push(g);
        }
        let pos = somas.iter().map(|(s, n)| s.map(|v| v / n)).collect();
        let tris = self.tris.iter().map(|t| t.map(|i| novos[i as usize])).filter(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2]).collect();
        Malha { nome: self.nome.clone(), pos, tris, material: self.material, ..Default::default() }
    }
}

#[cfg(test)]
mod testes {
    use crate::teste::*;
    use crate::Malha;

    #[test]
    fn solda_fecha_o_que_ficou_a_menos_da_tolerancia() {
        let c = caixa([10., 10., 10.]);
        // triângulos soltos, com ruído de até 1e-4 em cada canto
        let mut solta = Malha::default();
        for (n, t) in c.tris.iter().enumerate() {
            let base = solta.pos.len() as u32;
            for (k, i) in t.iter().enumerate() {
                let r = 1e-4 * ((n * 3 + k) % 5) as f64 / 5.;
                solta.pos.push(c.pos[*i as usize].map(|v| v + r));
            }
            solta.tris.push([base, base + 1, base + 2]);
        }
        assert!(!solta.soldada().fechada());
        let s = solta.soldada_com(1e-3);
        assert!(s.fechada());
        assert_eq!(s.pos.len(), 8);
        assert!((s.volume() - 1000.).abs() < 0.1);
    }

    #[test]
    fn grosseira_encolhe_sem_sair_da_caixa() {
        let e = esfera(10., 64);
        let g = e.grosseira(8);
        assert!(g.tris.len() * 4 < e.tris.len(), "{} de {}", g.tris.len(), e.tris.len());
        let (a, b) = g.caixa().unwrap();
        assert!(a.iter().all(|v| *v >= -10.) && b.iter().all(|v| *v <= 10.));
    }
}
