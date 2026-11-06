// SPDX-License-Identifier: LGPL-2.1-or-later
//! Núcleo de malha do Fenkra (B170): tudo o que trabalha com triângulos sem passar pelo kernel.
//!
//! A [`Malha`] é a lista indexada de sempre: posições e triângulos. Os formatos (STL, OBJ, PLY, 3MF)
//! leem e gravam [`Cena`]s; as operações recebem uma malha e devolvem outra, sem estado escondido,
//! e por isso o recálculo pode guardar o resultado pelo conteúdo. Este crate não conhece o documento
//! nem o OpenCASCADE: quem liga a malha à árvore é o `fenkra-solidos`.

pub mod booleana;
pub mod bvh;
pub mod cena;
pub mod decimar;
pub mod desvio;
pub mod diagnostico;
pub mod embutir;
pub mod obj;
pub mod ply;
pub mod reparo;
pub mod solda;
pub mod stl;
pub mod tmf;
pub mod zip;

pub use bvh::Bvh;
pub use cena::{Cena, Malha, Material, No, Ocorrencia, LIMITE_OCORRENCIAS};
pub use diagnostico::Diagnostico;
pub use reparo::{OpcoesReparo, RelatorioReparo};

use std::path::Path;

/// Matriz 3x4 por linhas que não muda nada.
pub const IDENTIDADE: [f64; 12] = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.];

/// Limite de profundidade: um arquivo com ciclo não pode travar quem desce a árvore.
pub const PROFUNDIDADE: usize = 64;

/// Tudo o que um leitor pode relatar como não traduzido. A interface traduz cada termo, então a lista
/// é fechada: termo novo entra aqui e na tradução.
pub const ITENS_FORA: &[&str] = &[
    "textura",
    "biblioteca de materiais não encontrada",
    "material sem definição",
    "linha ou ponto solto",
    "elemento extra",
    "treliça de vigas",
    "pilha de fatias",
    "componente em outro arquivo",
    "cor por vértice",
    "aresta curva",
    "animação",
    "esqueleto",
    "câmera",
    "luz",
    "ponto ou linha",
    "primitiva sem malha",
    "arquivo externo",
    "alternativa de Switch ou LOD",
    "orientação de escala",
    "cor por face",
    "protótipo",
    "sólido paramétrico sem malha pronta",
    "furo em face",
    "cavidade interna que ficou como sólido à parte",
    "peça com vários materiais, que ficou com a cor do maior",
];

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Erro {
    #[error("{0}")]
    Formato(String),
    #[error("{0}")]
    Leitura(String),
    #[error("{0}")]
    Gravacao(String),
    /// a operação não vale para esta malha; a mensagem diz o que fazer
    #[error("{0}")]
    Operacao(String),
}

/// `a` depois de `b`, com as matrizes 3x4 por linhas.
pub fn compor(a: &[f64; 12], b: &[f64; 12]) -> [f64; 12] {
    let mut r = [0.; 12];
    for i in 0..3 {
        for j in 0..4 {
            let soma: f64 = (0..3).map(|k| a[i * 4 + k] * b[k * 4 + j]).sum();
            r[i * 4 + j] = soma + if j == 3 { a[i * 4 + 3] } else { 0. };
        }
    }
    r
}

/// Milímetros por unidade, pelo nome que 3MF e AMF usam. `None` se o nome é desconhecido.
pub fn mm_da_unidade(nome: &str) -> Option<f64> {
    match nome.to_lowercase().as_str() {
        "micron" | "micrometer" => Some(0.001),
        "millimeter" | "millimetre" | "mm" => Some(1.),
        "centimeter" | "centimetre" | "cm" => Some(10.),
        "inch" | "in" => Some(25.4),
        "foot" | "feet" | "ft" => Some(304.8),
        "meter" | "metre" | "m" => Some(1000.),
        _ => None,
    }
}

pub fn escrever(caminho: &Path, bytes: &[u8]) -> Result<(), Erro> {
    std::fs::write(caminho, bytes).map_err(|e| Erro::Gravacao(format!("não foi possível gravar '{}': {e}", caminho.display())))
}

/// Falha de leitura com o nome do formato na frente.
pub fn erro(formato: &str, motivo: impl std::fmt::Display) -> Erro {
    Erro::Leitura(format!("{formato}: {motivo}"))
}

/// Texto seguro para XML, em atributo ou corpo.
pub fn xml(t: &str) -> String {
    t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Lê um arquivo de malha pela extensão. O corpo de malha só precisa destes quatro; os outros formatos
/// passam pelo conversor, que entrega a malha pronta.
pub fn ler_arquivo(caminho: &Path) -> Result<Cena, Erro> {
    let bytes = std::fs::read(caminho).map_err(|e| Erro::Leitura(format!("não foi possível ler '{}': {e}", caminho.display())))?;
    let ext = caminho.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let mut cena = match ext.as_str() {
        "stl" => stl::ler(&bytes),
        "obj" => obj::ler(&bytes, caminho),
        "ply" => ply::ler(&bytes),
        "3mf" => tmf::ler(&bytes),
        outro => Err(Erro::Formato(format!("o corpo de malha lê STL, OBJ, PLY e 3MF, não '{outro}'"))),
    }?;
    cena.enxugar();
    if cena.malhas.iter().all(|m| m.tris.is_empty()) {
        return Err(Erro::Leitura("a malha não tem triângulos".into()));
    }
    Ok(cena)
}

/// Junta malhas numa só, sem soldar.
pub fn juntar<'a>(malhas: impl IntoIterator<Item = &'a Malha>) -> Malha {
    let mut j = Malha::default();
    for m in malhas {
        let base = j.pos.len() as u32;
        j.pos.extend_from_slice(&m.pos);
        j.tris.extend(m.tris.iter().map(|t| t.map(|v| v + base)));
    }
    j
}

pub(crate) type V3 = [f64; 3];

pub(crate) fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub(crate) fn mul(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
pub(crate) fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(crate) fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub(crate) fn norma(a: V3) -> f64 {
    dot(a, a).sqrt()
}
pub(crate) fn unit(a: V3) -> V3 {
    mul(a, 1. / norma(a).max(1e-300))
}

/// Normal não normalizada de um triângulo: o comprimento é o dobro da área.
pub(crate) fn normal_area(a: V3, b: V3, c: V3) -> V3 {
    cross(sub(b, a), sub(c, a))
}

impl Malha {
    /// Canto mínimo e máximo. `None` em malha vazia.
    pub fn caixa(&self) -> Option<(V3, V3)> {
        let mut it = self.pos.iter();
        let p = *it.next()?;
        Some(it.fold((p, p), |(a, b), q| ([0, 1, 2].map(|i| a[i].min(q[i])), [0, 1, 2].map(|i| b[i].max(q[i])))))
    }

    /// Diagonal da caixa, que dá a escala das tolerâncias relativas.
    pub fn diagonal(&self) -> f64 {
        self.caixa().map(|(a, b)| norma(sub(b, a))).unwrap_or(0.)
    }

    /// Centro de massa da casca fechada, ou da superfície quando ela não fecha volume.
    pub fn centro(&self) -> V3 {
        let (mut v, mut c, mut area, mut ca) = (0., [0.; 3], 0., [0.; 3]);
        for t in &self.tris {
            let [a, b, d] = t.map(|i| self.pos[i as usize]);
            let s = dot(a, cross(b, d)) / 6.;
            v += s;
            c = add(c, mul(add(add(a, b), d), s / 4.));
            let k = norma(normal_area(a, b, d)) / 2.;
            area += k;
            ca = add(ca, mul(add(add(a, b), d), k / 3.));
        }
        if v.abs() > 1e-12 && self.fechada() {
            mul(c, 1. / v)
        } else {
            mul(ca, 1. / f64::max(area, 1e-300))
        }
    }
}

#[cfg(test)]
pub(crate) mod teste {
    use super::*;

    /// Caixa fechada com os vértices compartilhados, normais para fora.
    pub fn caixa(l: V3) -> Malha {
        let pos: Vec<V3> = (0..8).map(|i| [if i & 1 > 0 { l[0] } else { 0. }, if i & 2 > 0 { l[1] } else { 0. }, if i & 4 > 0 { l[2] } else { 0. }]).collect();
        let quads: [[u32; 4]; 6] = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
        let tris = quads.iter().flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]]).collect();
        Malha { pos, tris, ..Default::default() }
    }

    /// Esfera de latitudes e longitudes, fechada e com normais para fora.
    pub fn esfera(raio: f64, n: usize) -> Malha {
        let mut m = Malha::default();
        m.pos.push([0., 0., raio]);
        for i in 1..n {
            let th = std::f64::consts::PI * i as f64 / n as f64;
            for j in 0..2 * n {
                let ph = std::f64::consts::PI * j as f64 / n as f64;
                m.pos.push([raio * th.sin() * ph.cos(), raio * th.sin() * ph.sin(), raio * th.cos()]);
            }
        }
        m.pos.push([0., 0., -raio]);
        let w = 2 * n;
        let k = |i: usize, j: usize| (1 + (i - 1) * w + j % w) as u32;
        let sul = (m.pos.len() - 1) as u32;
        for j in 0..w {
            m.tris.push([0, k(1, j), k(1, j + 1)]);
            m.tris.push([sul, k(n - 1, j + 1), k(n - 1, j)]);
            for i in 1..n - 1 {
                m.tris.push([k(i, j), k(i + 1, j), k(i + 1, j + 1)]);
                m.tris.push([k(i, j), k(i + 1, j + 1), k(i, j + 1)]);
            }
        }
        m
    }
}
