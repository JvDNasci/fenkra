// SPDX-License-Identifier: LGPL-2.1-or-later
//! Compressão das malhas do GLB pela extensão aberta `EXT_meshopt_compression` (B181).
//!
//! Como funciona: os vértices de uma malha vizinha na lista costumam ser parecidos, então em vez de
//! gravar cada byte grava-se a diferença para o mesmo byte do vértice anterior. As diferenças são
//! pequenas e cabem em 0, 2 ou 4 bits; um valor que não cabe vira "escape" e o byte inteiro vem logo
//! depois. Os índices viram diferenças em número de tamanho variável (7 bits por byte). O resultado é
//! um fluxo que o ZIP ainda comprime bem por cima. Não há perda: o leitor devolve os mesmos bytes.
//!
//! Escrito a partir da especificação pública da extensão (Khronos, apêndice A): modo `ATTRIBUTES`
//! versão 0 e modo `INDICES` versão 1. O modo `TRIANGLES` não é usado (o `INDICES` vale para qualquer
//! lista de índices e é bem mais simples de conferir).

use serde_json::{json, Value};

pub const EXTENSAO: &str = "EXT_meshopt_compression";
const CAB_ATRIB: u8 = 0xa0;
const CAB_IND: u8 = 0xd1;
const CAUDA_MIN: usize = 32;

fn zigzag8(v: u8) -> u8 {
    ((v as i8 >> 7) as u8) ^ (v << 1)
}

fn unzigzag8(v: u8) -> u8 {
    (0u8.wrapping_sub(v & 1)) ^ (v >> 1)
}

/// Quantos elementos por bloco: o maior múltiplo de 16 até 8192 / passo, no máximo 256.
fn tam_bloco(passo: usize) -> usize {
    ((8192 / passo) & !15).min(256)
}

/// Grava um grupo de 16 diferenças no modo de menor tamanho; devolve o modo (0 a 3).
fn grupo(out: &mut Vec<u8>, d: &[u8; 16]) -> u8 {
    if d.iter().all(|&x| x == 0) {
        return 0;
    }
    let esc2 = d.iter().filter(|&&x| x >= 3).count();
    let esc4 = d.iter().filter(|&&x| x >= 15).count();
    let (t2, t4) = (4 + esc2, 8 + esc4);
    if t2 <= t4 && t2 < 16 {
        for q in d.chunks_exact(4) {
            out.push(q.iter().fold(0u8, |b, &x| (b << 2) | x.min(3)));
        }
        out.extend(d.iter().filter(|&&x| x >= 3));
        1
    } else if t4 < 16 {
        for q in d.chunks_exact(2) {
            out.push((q[0].min(15) << 4) | q[1].min(15));
        }
        out.extend(d.iter().filter(|&&x| x >= 15));
        2
    } else {
        out.extend_from_slice(d);
        3
    }
}

/// Codifica `n` elementos de `passo` bytes (múltiplo de 4, até 256).
pub fn codificar_atributos(dados: &[u8], passo: usize) -> Vec<u8> {
    assert!(passo % 4 == 0 && passo <= 256 && dados.len() % passo == 0);
    let n = dados.len() / passo;
    let mut out = vec![CAB_ATRIB];
    let primeiro: Vec<u8> = dados.get(..passo).map_or(vec![0; passo], <[u8]>::to_vec);
    let mut ultimo = primeiro.clone();
    let bloco = tam_bloco(passo);
    let mut ini = 0;
    while ini < n {
        let k = bloco.min(n - ini);
        let grupos = k.div_ceil(16);
        for b in 0..passo {
            let mut dif = vec![0u8; grupos * 16];
            let mut p = ultimo[b];
            for i in 0..k {
                let v = dados[(ini + i) * passo + b];
                dif[i] = zigzag8(v.wrapping_sub(p));
                p = v;
            }
            ultimo[b] = p;
            let mut cab = vec![0u8; grupos.div_ceil(4)];
            let mut corpo = vec![];
            for g in 0..grupos {
                let d: [u8; 16] = dif[g * 16..g * 16 + 16].try_into().expect("16");
                cab[g / 4] |= grupo(&mut corpo, &d) << ((g % 4) * 2);
            }
            out.extend_from_slice(&cab);
            out.extend_from_slice(&corpo);
        }
        ini += k;
    }
    // cauda: o primeiro elemento, que é a base das diferenças, alinhado ao fim
    let cauda = passo.max(CAUDA_MIN);
    out.extend(std::iter::repeat_n(0, cauda - passo));
    out.extend_from_slice(&primeiro);
    out
}

/// Decodifica `n` elementos de `passo` bytes.
pub fn decodificar_atributos(f: &[u8], n: usize, passo: usize) -> Result<Vec<u8>, String> {
    let ruim = || "fluxo meshopt de atributos inválido".to_string();
    if f.first() != Some(&CAB_ATRIB) || passo == 0 || passo % 4 != 0 || passo > 256 {
        return Err(ruim());
    }
    let cauda = passo.max(CAUDA_MIN);
    if f.len() < 1 + cauda {
        return Err(ruim());
    }
    let fim = f.len() - cauda;
    let mut ultimo = f[f.len() - passo..].to_vec();
    let mut out = vec![0u8; n * passo];
    let mut c = 1;
    let bloco = tam_bloco(passo);
    let mut ini = 0;
    while ini < n {
        let k = bloco.min(n - ini);
        let grupos = k.div_ceil(16);
        for b in 0..passo {
            let nc = grupos.div_ceil(4);
            if c + nc > fim {
                return Err(ruim());
            }
            let cab = &f[c..c + nc];
            c += nc;
            let mut dif = vec![0u8; grupos * 16];
            for g in 0..grupos {
                let modo = (cab[g / 4] >> ((g % 4) * 2)) & 3;
                let d = &mut dif[g * 16..g * 16 + 16];
                let (bits, cheio) = match modo {
                    0 => continue,
                    1 => (2u32, 3u8),
                    2 => (4, 15),
                    _ => {
                        d.copy_from_slice(f.get(c..c + 16).ok_or_else(ruim)?);
                        c += 16;
                        continue;
                    }
                };
                let nb = (16 * bits / 8) as usize;
                let emp = f.get(c..c + nb).ok_or_else(ruim)?;
                let mut esc = c + nb;
                for i in 0..16 {
                    let por_byte = (8 / bits) as usize;
                    let byte = emp[i / por_byte];
                    let desl = 8 - bits as usize * (i % por_byte + 1);
                    let v = (byte >> desl) & cheio;
                    d[i] = if v == cheio {
                        let x = *f.get(esc).filter(|_| esc < fim).ok_or_else(ruim)?;
                        esc += 1;
                        x
                    } else {
                        v
                    };
                }
                c = esc;
            }
            let mut p = ultimo[b];
            for i in 0..k {
                p = p.wrapping_add(unzigzag8(dif[i]));
                out[(ini + i) * passo + b] = p;
            }
            ultimo[b] = p;
        }
        ini += k;
    }
    if c > fim {
        return Err(ruim());
    }
    Ok(out)
}

fn varint(out: &mut Vec<u8>, mut v: u32) {
    loop {
        let b = (v & 127) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 128);
    }
}

/// Índices como diferenças para o anterior (sempre a base 0), em `INDICES`.
pub fn codificar_indices(idx: &[u32]) -> Vec<u8> {
    let mut out = vec![CAB_IND];
    let mut ultimo = 0u32;
    for &i in idx {
        let d = i.wrapping_sub(ultimo) as i32;
        let z = ((d << 1) ^ (d >> 31)) as u32;
        varint(&mut out, z << 1);
        ultimo = i;
    }
    out.extend_from_slice(&[0; 4]);
    out
}

pub fn decodificar_indices(f: &[u8], n: usize) -> Result<Vec<u32>, String> {
    let ruim = || "fluxo meshopt de índices inválido".to_string();
    if f.first() != Some(&CAB_IND) || f.len() < 5 {
        return Err(ruim());
    }
    let fim = f.len() - 4;
    let mut c = 1;
    let mut base = [0u32; 2];
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let mut v = 0u32;
        let mut s = 0;
        loop {
            if c >= fim || s > 28 {
                return Err(ruim());
            }
            let b = f[c];
            c += 1;
            v |= u32::from(b & 127) << s;
            s += 7;
            if b & 128 == 0 {
                break;
            }
        }
        let qual = (v & 1) as usize;
        let z = v >> 1;
        let d = (z >> 1) ^ 0u32.wrapping_sub(z & 1);
        base[qual] = base[qual].wrapping_add(d);
        out.push(base[qual]);
    }
    Ok(out)
}

/// Bytes de uma vista do GLB, decodificando quando a vista é compactada.
fn vista_crua(j: &Value, bin: &[u8], v: &Value) -> Result<Vec<u8>, String> {
    let fatia = |o: &Value| -> Result<&[u8], String> {
        let ini = o["byteOffset"].as_u64().unwrap_or(0) as usize;
        let tam = o["byteLength"].as_u64().ok_or("vista sem tamanho")? as usize;
        bin.get(ini..ini + tam).ok_or_else(|| "vista passa do fim do GLB".to_string())
    };
    let e = &v["extensions"][EXTENSAO];
    if e.is_null() {
        return Ok(fatia(v)?.to_vec());
    }
    if e["buffer"].as_u64() != Some(0) || e["filter"].as_str().is_some_and(|f| f != "NONE") {
        return Err("vista compactada em buffer externo ou com filtro".into());
    }
    let n = e["count"].as_u64().ok_or("vista compactada sem count")? as usize;
    let passo = e["byteStride"].as_u64().ok_or("vista compactada sem byteStride")? as usize;
    let f = fatia(e)?;
    let _ = j;
    match e["mode"].as_str() {
        Some("ATTRIBUTES") => decodificar_atributos(f, n, passo),
        Some("INDICES") => {
            let i = decodificar_indices(f, n)?;
            Ok(match passo {
                2 => i.iter().flat_map(|x| (*x as u16).to_le_bytes()).collect(),
                4 => i.iter().flat_map(|x| x.to_le_bytes()).collect(),
                _ => return Err("índices com passo diferente de 2 ou 4".into()),
            })
        }
        m => Err(format!("modo meshopt não suportado: {m:?}")),
    }
}

/// GLB sem a compressão: as vistas compactadas voltam ao binário comum, e a extensão sai da lista.
/// Quem lê o GLB (miniatura, conformidade) passa por aqui; GLB sem a extensão volta igual.
pub fn descompactar(glb: &[u8]) -> Result<Vec<u8>, String> {
    let (mut j, _) = crate::glb::ler_json(glb)?;
    let usa = j["extensionsUsed"].as_array().is_some_and(|l| l.iter().any(|x| x == EXTENSAO));
    if !usa {
        return Ok(glb.to_vec());
    }
    let tj = u32::from_le_bytes([glb[12], glb[13], glb[14], glb[15]]) as usize;
    let bin: &[u8] = if 20 + tj + 8 <= glb.len() { &glb[20 + tj + 8..] } else { &[] };
    let mut novo = vec![];
    let vistas = j["bufferViews"].as_array().cloned().unwrap_or_default();
    let mut saida = vec![];
    for v in &vistas {
        let b = vista_crua(&j, bin, v)?;
        while novo.len() % 4 != 0 {
            novo.push(0);
        }
        let mut w = json!({"buffer": 0, "byteOffset": novo.len(), "byteLength": b.len()});
        for k in ["byteStride", "target", "name"] {
            if !v[k].is_null() {
                w[k] = v[k].clone();
            }
        }
        novo.extend_from_slice(&b);
        saida.push(w);
    }
    j["bufferViews"] = Value::Array(saida);
    j["buffers"] = json!([{"byteLength": novo.len().div_ceil(4) * 4}]);
    for k in ["extensionsUsed", "extensionsRequired"] {
        if let Some(l) = j[k].as_array_mut() {
            l.retain(|x| x != EXTENSAO);
        }
    }
    Ok(crate::glb::montar(&j, novo))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn atributos_ida_e_volta_em_varios_tamanhos() {
        for (n, passo) in [(0, 12), (1, 12), (15, 4), (16, 4), (17, 12), (700, 12), (1000, 4), (300, 256)] {
            // mistura de dados lisos (diferença pequena) e ruído (escape e modo cru)
            let dados: Vec<u8> = (0..n * passo).map(|i| if i % 7 == 0 { (i * 131 % 251) as u8 } else { (i / passo) as u8 }).collect();
            let f = codificar_atributos(&dados, passo);
            assert_eq!(decodificar_atributos(&f, n, passo).unwrap(), dados, "n={n} passo={passo}");
        }
    }

    #[test]
    fn dados_lisos_comprimem() {
        let dados: Vec<u8> = (0..4000u32).flat_map(|i| [(i as f32) * 0.5, 1., 2.]).flat_map(f32::to_le_bytes).collect();
        let f = codificar_atributos(&dados, 12);
        assert!(f.len() < dados.len() / 2, "{} de {}", f.len(), dados.len());
    }

    #[test]
    fn indices_ida_e_volta_com_saltos() {
        let idx: Vec<u32> = vec![0, 1, 2, 2, 1, 3, 70000, 5, 65535, 0, u32::MAX >> 2, 9];
        let f = codificar_indices(&idx);
        assert_eq!(decodificar_indices(&f, idx.len()).unwrap(), idx);
        assert!(decodificar_indices(&f[..f.len() - 6], idx.len()).is_err(), "fluxo cortado é recusado");
    }
}
