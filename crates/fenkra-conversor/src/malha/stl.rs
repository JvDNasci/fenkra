// SPDX-License-Identifier: LGPL-2.1-or-later
//! STL: triângulos soltos, sem cor, unidade nem estrutura.

use super::{erro, Cena, Malha};
use crate::ErroConv;
use std::fmt::Write as _;

/// O binário não tem assinatura: o que o identifica é o tamanho bater com a contagem do cabeçalho.
pub(crate) fn e_binario(b: &[u8]) -> bool {
    b.len() >= 84 && u64::from(u32::from_le_bytes([b[80], b[81], b[82], b[83]])) * 50 + 84 == b.len() as u64
}

pub(crate) fn e_texto(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(&b[..b.len().min(1024)]);
    t.trim_start().to_lowercase().starts_with("solid") && (t.contains("facet") || t.contains("endsolid"))
}

pub(crate) fn ler(b: &[u8]) -> Result<Cena, ErroConv> {
    let mut m = Malha::default();
    if e_binario(b) {
        let n = u32::from_le_bytes([b[80], b[81], b[82], b[83]]) as usize;
        for i in 0..n {
            let base = 84 + i * 50 + 12;
            for v in 0..3 {
                m.pos.push([0, 1, 2].map(|k| {
                    let o = base + (v * 3 + k) * 4;
                    f64::from(f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]))
                }));
            }
            m.tris.push([i as u32 * 3, i as u32 * 3 + 1, i as u32 * 3 + 2]);
        }
    } else {
        let t = String::from_utf8_lossy(b);
        for linha in t.lines() {
            let mut w = linha.split_whitespace();
            match w.next() {
                Some("solid") if m.nome.is_empty() => m.nome = w.collect::<Vec<_>>().join(" "),
                Some("vertex") => {
                    let c: Vec<f64> = w.take(3).filter_map(|x| x.parse().ok()).collect();
                    let [x, y, z] = c[..] else { return Err(erro("STL", "vértice inválido")) };
                    m.pos.push([x, y, z]);
                }
                _ => {}
            }
        }
        if m.pos.len() % 3 != 0 {
            return Err(erro("STL", "número de vértices não é múltiplo de 3"));
        }
        m.tris = (0..m.pos.len() as u32 / 3).map(|i| [i * 3, i * 3 + 1, i * 3 + 2]).collect();
    }
    m.conferir("STL")?;
    let mut c = Cena::default();
    c.raiz_com(m);
    Ok(c)
}

fn normal(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> [f64; 3] {
    let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
    let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-300);
    n.map(|x| x / l)
}

pub(crate) fn gravar(cena: &Cena, binario: bool, mm: f64) -> Vec<u8> {
    let malhas = cena.no_lugar();
    let total: usize = malhas.iter().map(|m| m.tris.len()).sum();
    let triangulos = || malhas.iter().flat_map(|m| m.tris.iter().map(|t| t.map(|i| m.pos[i as usize].map(|v| v / mm))));
    if binario {
        let mut s = vec![0u8; 80];
        let titulo = b"Fenkra STL";
        s[..titulo.len()].copy_from_slice(titulo);
        s.extend_from_slice(&(total as u32).to_le_bytes());
        for [a, b, c] in triangulos() {
            for v in [normal(a, b, c), a, b, c] {
                for x in v {
                    s.extend_from_slice(&(x as f32).to_le_bytes());
                }
            }
            s.extend_from_slice(&[0, 0]);
        }
        return s;
    }
    let titulo = cena.nome.replace(['\n', '\r'], " ");
    let mut s = format!("solid {titulo}\n");
    for [a, b, c] in triangulos() {
        let n = normal(a, b, c);
        let _ = writeln!(s, "facet normal {} {} {}\n outer loop", n[0], n[1], n[2]);
        for p in [a, b, c] {
            let _ = writeln!(s, "  vertex {} {} {}", p[0], p[1], p[2]);
        }
        s.push_str(" endloop\nendfacet\n");
    }
    let _ = writeln!(s, "endsolid {titulo}");
    s.into_bytes()
}
