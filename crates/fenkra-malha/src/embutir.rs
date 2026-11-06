// SPDX-License-Identifier: LGPL-2.1-or-later
//! Malha como texto, para morar dentro do documento (o `.fkr` é JSON).
//!
//! O corpo de malha abre sem o arquivo de origem, como o corpo importado de STEP. Os números vão em
//! binário: posições em f64, índices em u32, comprimidos com deflate e postos em base64. Os índices
//! sobem por diferença, porque vizinhos no arquivo têm números próximos e isso comprime muito melhor.

use crate::{Erro, Malha};

const CABECALHO: &[u8; 4] = b"FKM1";
const ALFABETO: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len().div_ceil(3) * 4);
    for g in b.chunks(3) {
        let n = (u32::from(g[0]) << 16) | (u32::from(*g.get(1).unwrap_or(&0)) << 8) | u32::from(*g.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= g.len() {
                s.push(ALFABETO[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

fn de_base64(t: &str) -> Option<Vec<u8>> {
    let mut valor = [255u8; 256];
    ALFABETO.iter().enumerate().for_each(|(i, c)| valor[*c as usize] = i as u8);
    let mut b = Vec::with_capacity(t.len() / 4 * 3);
    for g in t.as_bytes().chunks(4) {
        if g.len() != 4 {
            return None;
        }
        let uteis = g.iter().take_while(|c| **c != b'=').count();
        let mut n = 0u32;
        for (i, c) in g.iter().enumerate() {
            let v = if i < uteis { valor[*c as usize] } else { 0 };
            if v == 255 {
                return None;
            }
            n |= u32::from(v) << (18 - 6 * i);
        }
        b.extend_from_slice(&n.to_be_bytes()[1..uteis.max(1)]);
    }
    Some(b)
}

/// Texto da malha. Guarda só posições e triângulos: é o que o corpo de malha usa.
pub fn para_texto(m: &Malha) -> String {
    let mut b = Vec::with_capacity(12 + m.pos.len() * 24 + m.tris.len() * 12);
    b.extend_from_slice(CABECALHO);
    b.extend_from_slice(&(m.pos.len() as u32).to_le_bytes());
    b.extend_from_slice(&(m.tris.len() as u32).to_le_bytes());
    for eixo in 0..3 {
        // por plano: os expoentes de um eixo se repetem e o deflate acha isso
        for p in &m.pos {
            b.extend_from_slice(&p[eixo].to_le_bytes());
        }
    }
    let mut anterior = 0u32;
    for v in m.tris.iter().flatten() {
        b.extend_from_slice(&v.wrapping_sub(anterior).to_le_bytes());
        anterior = *v;
    }
    base64(&miniz_oxide::deflate::compress_to_vec(&b, 6))
}

/// Lê o texto de volta. Texto mexido à mão vira erro, nunca índice fora da lista.
pub fn de_texto(t: &str) -> Result<Malha, Erro> {
    let ruim = |o: &str| Erro::Leitura(format!("malha embutida inválida: {o}"));
    let z = de_base64(t).ok_or_else(|| ruim("base64"))?;
    let b = miniz_oxide::inflate::decompress_to_vec(&z).map_err(|_| ruim("compressão"))?;
    if b.len() < 12 || &b[..4] != CABECALHO {
        return Err(ruim("cabeçalho"));
    }
    let u = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let (nv, nt) = (u(4) as usize, u(8) as usize);
    if b.len() != 12 + nv * 24 + nt * 12 {
        return Err(ruim("tamanho"));
    }
    let f = |i: usize| {
        let mut x = [0u8; 8];
        x.copy_from_slice(&b[i..i + 8]);
        f64::from_le_bytes(x)
    };
    let pos: Vec<[f64; 3]> = (0..nv).map(|i| [0, 1, 2].map(|e| f(12 + (e * nv + i) * 8))).collect();
    let base = 12 + nv * 24;
    let mut anterior = 0u32;
    let mut tris = Vec::with_capacity(nt);
    for i in 0..nt {
        let t = [0, 1, 2].map(|k| {
            anterior = anterior.wrapping_add(u(base + (i * 3 + k) * 4));
            anterior
        });
        tris.push(t);
    }
    let m = Malha { pos, tris, ..Default::default() };
    m.conferir("malha embutida")?;
    Ok(m)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::teste::*;

    #[test]
    fn ida_e_volta_bit_a_bit_e_menor_que_o_binario() {
        let e = esfera(12.5, 40);
        let t = para_texto(&e);
        let v = de_texto(&t).unwrap();
        assert_eq!((v.pos, v.tris), (e.pos.clone(), e.tris.clone()));
        assert!(t.len() < e.pos.len() * 24 + e.tris.len() * 12, "{}", t.len());
    }

    #[test]
    fn base64_de_todo_tamanho_volta() {
        for n in 0..10usize {
            let b: Vec<u8> = (0..n as u8).map(|i| i.wrapping_mul(37)).collect();
            assert_eq!(de_base64(&base64(&b)).unwrap(), b);
        }
    }

    #[test]
    fn texto_estragado_vira_erro() {
        let t = para_texto(&caixa([1., 2., 3.]));
        assert!(de_texto("não é base64").is_err());
        assert!(de_texto(&t[..t.len() / 2 / 4 * 4]).is_err());
        assert!(de_texto("AAAA").is_err());
    }

    #[test]
    fn indice_fora_da_lista_e_recusado() {
        let mut m = caixa([1., 1., 1.]);
        m.tris[0][0] = 99;
        assert!(de_texto(&para_texto(&m)).is_err());
    }
}
