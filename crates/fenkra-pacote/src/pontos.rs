// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pontos de captura exatos do B-rep, para medir no visualizador sem o kernel.
//!
//! A malha de exibição é aproximada: o centro de um furo ou o fim de uma aresta não estão nela com
//! precisão de projeto. Aqui saem os pontos tirados da fórmula de cada aresta (precisão dupla): pontas
//! de reta e de arco, pontos médios das retas e centros de círculos com o raio e o eixo. A medida
//! entre dois deles no visualizador é a mesma que o CAD daria.

use fenkra_kernel::{EdgeFormula, Mesh, EDGE_GEOM};
use serde_json::{json, Value};
use std::collections::HashSet;

/// Teto por malha: peça com milhares de arestas curtas não deve inchar o pacote.
const TETO: usize = 40_000;

fn r9(x: f64) -> f64 {
    let v = (x * 1e9).round() / 1e9;
    if v == 0. {
        0.
    } else {
        v
    }
}

fn norma(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// `pontos`: `[x, y, z, tipo]` com tipo 0 vértice, 1 ponto médio, 2 centro. `circulos`:
/// `[cx, cy, cz, raio, nx, ny, nz]`.
pub fn capturas(m: &Mesh) -> Value {
    let mut vistos: HashSet<(i64, i64, i64, u8)> = HashSet::new();
    let (mut pontos, mut circulos): (Vec<[f64; 4]>, Vec<[f64; 7]>) = (vec![], vec![]);
    let mut por = |p: [f64; 3], t: u8, pontos: &mut Vec<[f64; 4]>| {
        let k = (p[0] * 1e6).round() as i64;
        let chave = (k, (p[1] * 1e6).round() as i64, (p[2] * 1e6).round() as i64, t);
        if pontos.len() < TETO && vistos.insert(chave) {
            pontos.push([r9(p[0]), r9(p[1]), r9(p[2]), f64::from(t)]);
        }
    };
    for e in 0..m.edge_geom.len() / EDGE_GEOM {
        match m.edge_formula(e) {
            Some(EdgeFormula::Line { a, b }) => {
                por(a, 0, &mut pontos);
                por(b, 0, &mut pontos);
                por([0, 1, 2].map(|i| (a[i] + b[i]) / 2.), 1, &mut pontos);
            }
            Some(EdgeFormula::Conic { c, u, v, t0, t1 }) => {
                por(c, 2, &mut pontos);
                let (ru, rv) = (norma(u), norma(v));
                if (ru - rv).abs() < 1e-9 * ru.max(1.) {
                    let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
                    let nn = norma(n).max(1e-300);
                    if circulos.len() < TETO {
                        circulos.push([r9(c[0]), r9(c[1]), r9(c[2]), r9(ru), r9(n[0] / nn), r9(n[1] / nn), r9(n[2] / nn)]);
                    }
                }
                let em = |t: f64| [0, 1, 2].map(|i| c[i] + u[i] * t.cos() + v[i] * t.sin());
                // o círculo inteiro tem um vértice só, na costura
                por(em(t0), 0, &mut pontos);
                if (t1 - t0).abs() < std::f64::consts::TAU - 1e-9 {
                    por(em(t1), 0, &mut pontos);
                    por(em((t0 + t1) / 2.), 1, &mut pontos);
                }
            }
            None => {}
        }
    }
    json!({"pontos": pontos, "circulos": circulos})
}
