// SPDX-License-Identifier: LGPL-2.1-or-later
//! Miniatura simples de peça ou montagem tirada do pacote `.fkv` (B364), sem kernel e sem GPU.
//!
//! Como funciona:
//! 1. lê a cena GLB do pacote e percorre os nós a partir dos filhos de `fkv:raiz`, acumulando as
//!    matrizes. A raiz (que passa para metros e gira Z para Y) é pulada: tudo fica em milímetros com
//!    Z para cima, como no modelo;
//! 2. projeta em vista isométrica (olho em +X, -Y, +Z) e enquadra a caixa projetada no quadrado,
//!    com margem, mantendo a proporção da peça;
//! 3. rasteriza cada triângulo com teste de profundidade numa grade 2× maior e reduz pela média:
//!    é o antisserrilhado mais simples que existe;
//! 4. o tom é só luz (cinza do escuro ao claro, pelo ângulo da face com a luz) e o fundo é
//!    transparente. Quem desenha pinta com a cor do tema: a miniatura não traz cor fixa.

use crate::glb;
use serde_json::Value;

/// Imagem RGBA quadrada, linha a linha, de cima para baixo.
#[derive(Clone, Debug, PartialEq)]
pub struct Miniatura {
    pub lado: u32,
    pub rgba: Vec<u8>,
}

impl Miniatura {
    /// Fração dos pixels com peça (alfa acima da metade).
    pub fn cobertura(&self) -> f64 {
        let n = self.rgba.chunks_exact(4).filter(|p| p[3] > 127).count();
        n as f64 / f64::from(self.lado * self.lado)
    }

    /// Caixa (x0, y0, x1, y1) dos pixels com peça, em pixels.
    pub fn caixa(&self) -> Option<(u32, u32, u32, u32)> {
        let mut c: Option<(u32, u32, u32, u32)> = None;
        for (i, p) in self.rgba.chunks_exact(4).enumerate() {
            if p[3] > 127 {
                let (x, y) = (i as u32 % self.lado, i as u32 / self.lado);
                c = Some(c.map_or((x, y, x, y), |(a, b, cx, d)| (a.min(x), b.min(y), cx.max(x), d.max(y))));
            }
        }
        c
    }
}

type M4 = [f64; 16];
const ID: M4 = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];

/// Produto de matrizes 4x4 por colunas (como no glTF).
fn mul(a: &M4, b: &M4) -> M4 {
    let mut r = [0.; 16];
    for c in 0..4 {
        for l in 0..4 {
            r[c * 4 + l] = (0..4).map(|k| a[k * 4 + l] * b[c * 4 + k]).sum();
        }
    }
    r
}

fn aplicar(m: &M4, p: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|l| m[l] * p[0] + m[4 + l] * p[1] + m[8 + l] * p[2] + m[12 + l])
}

/// Os triângulos da cena em milímetros (Z para cima).
fn triangulos(cena: &[u8]) -> Result<Vec<[[f64; 3]; 3]>, String> {
    let cru = crate::meshopt::descompactar(cena)?;
    let cena = cru.as_slice();
    let (j, _) = glb::ler_json(cena)?;
    let tj = u32::from_le_bytes([cena[12], cena[13], cena[14], cena[15]]) as usize;
    let bin: &[u8] = if 20 + tj + 8 <= cena.len() { &cena[20 + tj + 8..] } else { &[] };
    let nos = j["nodes"].as_array().cloned().unwrap_or_default();
    let mut out = vec![];
    // ponto de partida: os filhos da raiz de conversão, ou as raízes da cena se ela não existir
    let topo: Vec<usize> = j["scenes"][0]["nodes"].as_array().into_iter().flatten().filter_map(Value::as_u64).map(|n| n as usize).collect();
    let mut pilha: Vec<(usize, M4, usize)> = vec![];
    for t in topo {
        let n = nos.get(t).ok_or("nó da cena não existe")?;
        if n["extras"]["fkv_raiz"].as_bool() == Some(true) {
            pilha.extend(n["children"].as_array().into_iter().flatten().filter_map(Value::as_u64).map(|c| (c as usize, ID, 0)));
        } else {
            pilha.push((t, ID, 0));
        }
    }
    while let Some((i, pai, fundo)) = pilha.pop() {
        if fundo > 64 {
            return Err("cena com nós em ciclo".into());
        }
        let n = nos.get(i).ok_or("nó filho não existe")?;
        let mut m = pai;
        if let Some(a) = n["matrix"].as_array().filter(|a| a.len() == 16) {
            let mut l = [0.; 16];
            for (k, v) in a.iter().enumerate() {
                l[k] = v.as_f64().unwrap_or(0.);
            }
            m = mul(&pai, &l);
        }
        if let Some(mi) = n["mesh"].as_u64() {
            for prim in j["meshes"][mi as usize]["primitives"].as_array().into_iter().flatten() {
                let pos = ler_posicoes(&j, bin, prim["attributes"]["POSITION"].as_u64().ok_or("primitiva sem posições")? as usize)?;
                let idx = match prim["indices"].as_u64() {
                    Some(a) => ler_indices(&j, bin, a as usize)?,
                    None => (0..pos.len() as u32).collect(),
                };
                for t in idx.chunks_exact(3) {
                    let v = |k: u32| pos.get(k as usize).copied().ok_or("índice fora da malha");
                    out.push([aplicar(&m, v(t[0])?), aplicar(&m, v(t[1])?), aplicar(&m, v(t[2])?)]);
                }
            }
        }
        pilha.extend(n["children"].as_array().into_iter().flatten().filter_map(Value::as_u64).map(|c| (c as usize, m, fundo + 1)));
    }
    Ok(out)
}

/// Fatia de bytes de um acessor: (dados, passo, quantidade).
fn fatia<'a>(j: &Value, bin: &'a [u8], acessor: usize, tamanho: usize) -> Result<(&'a [u8], usize, usize), String> {
    let a = &j["accessors"][acessor];
    let v = &j["bufferViews"][a["bufferView"].as_u64().ok_or("acessor sem vista")? as usize];
    let ini = v["byteOffset"].as_u64().unwrap_or(0) as usize + a["byteOffset"].as_u64().unwrap_or(0) as usize;
    let passo = v["byteStride"].as_u64().map_or(tamanho, |s| s as usize);
    let n = a["count"].as_u64().unwrap_or(0) as usize;
    let fim = if n == 0 { ini } else { ini + passo * (n - 1) + tamanho };
    if fim > bin.len() {
        return Err("acessor passa do fim do GLB".into());
    }
    Ok((&bin[ini..fim], passo, n))
}

fn ler_posicoes(j: &Value, bin: &[u8], a: usize) -> Result<Vec<[f64; 3]>, String> {
    if j["accessors"][a]["componentType"].as_u64() != Some(5126) {
        return Err("posições que não são float".into());
    }
    let (b, passo, n) = fatia(j, bin, a, 12)?;
    let f = |o: usize| f64::from(f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]));
    Ok((0..n).map(|i| [f(i * passo), f(i * passo + 4), f(i * passo + 8)]).collect())
}

fn ler_indices(j: &Value, bin: &[u8], a: usize) -> Result<Vec<u32>, String> {
    let (tam, u16_) = match j["accessors"][a]["componentType"].as_u64() {
        Some(5123) => (2, true),
        Some(5125) => (4, false),
        Some(5121) => (1, false),
        _ => return Err("tipo de índice desconhecido".into()),
    };
    let (b, passo, n) = fatia(j, bin, a, tam)?;
    Ok((0..n)
        .map(|i| {
            let o = i * passo;
            match (tam, u16_) {
                (2, _) => u32::from(u16::from_le_bytes([b[o], b[o + 1]])),
                (4, _) => u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]),
                _ => u32::from(b[o]),
            }
        })
        .collect())
}

fn norm(v: [f64; 3]) -> [f64; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 0. {
        v.map(|x| x / l)
    } else {
        v
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Desenha a miniatura de `lado` pixels a partir dos triângulos em milímetros.
pub fn de_triangulos(tri: &[[[f64; 3]; 3]], lado: u32) -> Miniatura {
    let lado = lado.clamp(8, 1024);
    let s = (lado * 2) as usize;
    // vista isométrica: olho em (1, -1, 1); direita (1, 1, 0); cima (-1, 1, 2)
    let olho = norm([1., -1., 1.]);
    let dir = norm([1., 1., 0.]);
    let cima = norm([-1., 1., 2.]);
    let luz = norm([0.4, -0.7, 1.]);
    let proj = |p: [f64; 3]| (dot(p, dir), dot(p, cima), dot(p, olho));
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for t in tri {
        for &p in t {
            let (x, y, _) = proj(p);
            lo = [lo[0].min(x), lo[1].min(y)];
            hi = [hi[0].max(x), hi[1].max(y)];
        }
    }
    let mut prof = vec![f64::MIN; s * s];
    let mut tom = vec![0u8; s * s];
    if !tri.is_empty() && hi[0] >= lo[0] {
        let margem = 0.08 * s as f64;
        let ext = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-9);
        let k = (s as f64 - 2. * margem) / ext;
        let (cx, cy) = ((lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.);
        let tela = |p: [f64; 3]| {
            let (x, y, z) = proj(p);
            (s as f64 / 2. + (x - cx) * k, s as f64 / 2. - (y - cy) * k, z)
        };
        for t in tri {
            let n = norm(cross(sub(t[1], t[0]), sub(t[2], t[0])));
            // face sem área não pinta nada; a luz vale para os dois lados (malha pode vir invertida)
            if n == [0.; 3] {
                continue;
            }
            let claro = 0.28 + 0.72 * dot(n, luz).abs();
            let c = (claro * 255.).round().clamp(0., 255.) as u8;
            let [a, b, d] = [tela(t[0]), tela(t[1]), tela(t[2])];
            let area = (b.0 - a.0) * (d.1 - a.1) - (b.1 - a.1) * (d.0 - a.0);
            if area.abs() < 1e-12 {
                continue;
            }
            let x0 = a.0.min(b.0).min(d.0).floor().max(0.) as usize;
            let x1 = (a.0.max(b.0).max(d.0).ceil() as usize).min(s - 1);
            let y0 = a.1.min(b.1).min(d.1).floor().max(0.) as usize;
            let y1 = (a.1.max(b.1).max(d.1).ceil() as usize).min(s - 1);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                    let w0 = ((b.0 - px) * (d.1 - py) - (b.1 - py) * (d.0 - px)) / area;
                    let w1 = ((d.0 - px) * (a.1 - py) - (d.1 - py) * (a.0 - px)) / area;
                    let w2 = 1. - w0 - w1;
                    if w0 < -1e-9 || w1 < -1e-9 || w2 < -1e-9 {
                        continue;
                    }
                    let z = w0 * a.2 + w1 * b.2 + w2 * d.2;
                    let i = y * s + x;
                    if z > prof[i] {
                        prof[i] = z;
                        tom[i] = c;
                    }
                }
            }
        }
    }
    // redução 2× pela média: cor média dos pixels cheios, alfa pela fração de cheios
    let l = lado as usize;
    let mut rgba = vec![0u8; l * l * 4];
    for y in 0..l {
        for x in 0..l {
            let (mut soma, mut cheios) = (0u32, 0u32);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let i = (2 * y + dy) * s + 2 * x + dx;
                if prof[i] > f64::MIN {
                    soma += u32::from(tom[i]);
                    cheios += 1;
                }
            }
            if cheios > 0 {
                let v = (soma / cheios) as u8;
                let o = (y * l + x) * 4;
                rgba[o..o + 4].copy_from_slice(&[v, v, v, (cheios * 255 / 4) as u8]);
            }
        }
    }
    Miniatura { lado, rgba }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Miniatura de um `.fkv` lido em memória. Pacote sem cena (só desenho) é erro.
pub fn de_pacote(fkv: &[u8], lado: u32) -> Result<Miniatura, String> {
    let p = crate::Pacote::ler(fkv)?;
    let nome = p.manifesto["cena"].as_str().ok_or("o pacote não tem cena 3D")?.to_string();
    let cena = p.arquivos.iter().find(|a| a.0 == nome).map(|a| a.1.as_slice()).ok_or("a cena apontada não está no pacote")?;
    let tri = triangulos(cena)?;
    if tri.is_empty() {
        return Err("a cena não tem triângulos".into());
    }
    Ok(de_triangulos(&tri, lado))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matriz_por_colunas_compoe_na_ordem_do_gltf() {
        // translação depois de escala: o ponto é escalado e depois movido
        let esc = [2., 0., 0., 0., 0., 2., 0., 0., 0., 0., 2., 0., 0., 0., 0., 1.];
        let tr = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 10., 0., 0., 1.];
        assert_eq!(aplicar(&mul(&tr, &esc), [1., 1., 1.]), [12., 2., 2.]);
    }

    #[test]
    fn cena_vazia_da_imagem_transparente() {
        let m = de_triangulos(&[], 32);
        assert_eq!(m.rgba.len(), 32 * 32 * 4);
        assert_eq!(m.cobertura(), 0.);
        assert!(m.caixa().is_none());
    }
}
