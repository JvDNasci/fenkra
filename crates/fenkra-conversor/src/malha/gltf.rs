// SPDX-License-Identifier: LGPL-2.1-or-later
//! glTF 2.0, em JSON com o buffer embutido ou em GLB. A norma manda metros e Y para cima: a gravação
//! converte os números e põe uma raiz que gira Z para Y, e a leitura desfaz as duas coisas.

use super::obj::ao_lado;
use super::{erro, Cena, Malha, Material, No};
use crate::ErroConv;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::Path;

const MAGICA: &[u8; 4] = b"glTF";
const TRECHO_JSON: u32 = 0x4e4f_534a;
const TRECHO_BIN: u32 = 0x004e_4942;
const FLOAT: u32 = 5126;
const U8: u32 = 5121;
const U16: u32 = 5123;
const U32: u32 = 5125;
/// Y para cima a Z para cima, por linhas: (x, y, z) -> (x, -z, y).
const Y_PARA_Z: [f64; 12] = [1., 0., 0., 0., 0., 0., -1., 0., 0., 1., 0., 0.];

const LETRAS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub(crate) fn base64(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let n = u32::from(c[0]) << 16 | u32::from(*c.get(1).unwrap_or(&0)) << 8 | u32::from(*c.get(2).unwrap_or(&0));
        for k in 0..4 {
            s.push(if k <= c.len() { LETRAS[(n >> (18 - 6 * k) & 63) as usize] as char } else { '=' });
        }
    }
    s
}

fn de_base64(t: &str) -> Option<Vec<u8>> {
    let mut s = Vec::with_capacity(t.len() / 4 * 3);
    let (mut n, mut bits) = (0u32, 0);
    for c in t.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        n = n << 6 | LETRAS.iter().position(|l| *l == c)? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            s.push((n >> bits) as u8);
        }
    }
    Some(s)
}

/// Monta o JSON e o buffer. O JSON sai sem o `uri`, que depende do jeito de embalar.
fn montar(cena: &Cena, normais: bool) -> Result<(Value, Vec<u8>), ErroConv> {
    let mut c = cena.clone();
    c.assentar();
    c.escalar(0.001);
    let (mut buffer, mut vistas, mut acessores): (Vec<u8>, Vec<Value>, Vec<Value>) = (Vec::new(), vec![], vec![]);
    let mut acrescentar = |dados: &[u8], alvo: u32, acessor: Value, buffer: &mut Vec<u8>| -> usize {
        vistas.push(json!({"buffer": 0, "byteOffset": buffer.len(), "byteLength": dados.len(), "target": alvo}));
        buffer.extend_from_slice(dados);
        buffer.resize(buffer.len().div_ceil(4) * 4, 0);
        let mut a = acessor;
        a["bufferView"] = json!(vistas.len() - 1);
        acessores.push(a);
        acessores.len() - 1
    };
    let mut primitivas = Vec::with_capacity(c.malhas.len());
    for m in &c.malhas {
        let lo = m.pos.iter().fold([f32::MAX; 3], |a, p| [0, 1, 2].map(|i| a[i].min(p[i] as f32)));
        let hi = m.pos.iter().fold([f32::MIN; 3], |a, p| [0, 1, 2].map(|i| a[i].max(p[i] as f32)));
        let dados: Vec<u8> = m.pos.iter().flatten().flat_map(|v| (*v as f32).to_le_bytes()).collect();
        let pos = acrescentar(&dados, 34962, json!({"componentType": FLOAT, "count": m.pos.len(), "type": "VEC3", "min": lo, "max": hi}), &mut buffer);
        let mut atributos = Map::new();
        atributos.insert("POSITION".into(), json!(pos));
        if normais && m.nrm.len() == m.pos.len() {
            let dados: Vec<u8> = m.nrm.iter().flatten().flat_map(|v| v.to_le_bytes()).collect();
            atributos.insert("NORMAL".into(), json!(acrescentar(&dados, 34962, json!({"componentType": FLOAT, "count": m.nrm.len(), "type": "VEC3"}), &mut buffer)));
        }
        if m.cor_v.len() == m.pos.len() {
            let dados: Vec<u8> = m.cor_v.iter().flatten().copied().collect();
            atributos.insert("COLOR_0".into(), json!(acrescentar(&dados, 34962, json!({"componentType": U8, "normalized": true, "count": m.cor_v.len(), "type": "VEC4"}), &mut buffer)));
        }
        let dados: Vec<u8> = m.tris.iter().flatten().flat_map(|v| v.to_le_bytes()).collect();
        let indices = acrescentar(&dados, 34963, json!({"componentType": U32, "count": m.tris.len() * 3, "type": "SCALAR"}), &mut buffer);
        let mut p = json!({"attributes": atributos, "indices": indices, "mode": 4});
        if let Some(k) = m.material {
            p["material"] = json!(k);
        }
        primitivas.push(p);
    }
    let (mut malhas, mut por_lista): (Vec<Value>, HashMap<Vec<usize>, usize>) = (vec![], HashMap::new());
    let mut nos = Vec::with_capacity(c.nos.len() + 1);
    for n in &c.nos {
        let mut v = json!({"name": n.nome});
        if !n.filhos.is_empty() {
            v["children"] = json!(n.filhos);
        }
        if !n.malhas.is_empty() {
            let i = *por_lista.entry(n.malhas.clone()).or_insert_with(|| {
                let nome = c.malhas[n.malhas[0]].nome.clone();
                malhas.push(json!({"name": nome, "primitives": n.malhas.iter().map(|i| primitivas[*i].clone()).collect::<Vec<_>>()}));
                malhas.len() - 1
            });
            v["mesh"] = json!(i);
        }
        if n.local != fenkra_kernel::IDENTIDADE {
            let m = &n.local;
            v["matrix"] = json!([m[0], m[4], m[8], 0., m[1], m[5], m[9], 0., m[2], m[6], m[10], 0., m[3], m[7], m[11], 1.]);
        }
        nos.push(v);
    }
    nos.push(json!({"name": c.nome, "children": c.raizes, "matrix": [1., 0., 0., 0., 0., 0., -1., 0., 0., 1., 0., 0., 0., 0., 0., 1.]}));
    let topo = nos.len() - 1;
    let materiais: Vec<Value> = c
        .materiais
        .iter()
        .map(|k| {
            let mut v = json!({"name": k.nome, "pbrMetallicRoughness": {"baseColorFactor": k.cor, "metallicFactor": k.metal, "roughnessFactor": k.rugosidade}});
            if k.cor[3] < 1. {
                v["alphaMode"] = json!("BLEND");
            }
            v
        })
        .collect();
    let mut raiz = json!({
        "asset": {"version": "2.0", "generator": "Fenkra"},
        "scene": 0,
        "scenes": [{"nodes": [topo]}],
        "nodes": nos,
        "meshes": malhas,
        "accessors": acessores,
        "bufferViews": vistas,
        "buffers": [{"byteLength": buffer.len()}],
    });
    if !materiais.is_empty() {
        raiz["materials"] = json!(materiais);
    }
    Ok((raiz, buffer))
}

pub(crate) fn gravar_texto(cena: &Cena, normais: bool) -> Result<Vec<u8>, ErroConv> {
    let (mut v, b) = montar(cena, normais)?;
    v["buffers"][0]["uri"] = json!(format!("data:application/octet-stream;base64,{}", base64(&b)));
    serde_json::to_vec(&v).map_err(|e| ErroConv::Gravacao(e.to_string()))
}

pub(crate) fn gravar_binario(cena: &Cena, normais: bool) -> Result<Vec<u8>, ErroConv> {
    let (v, b) = montar(cena, normais)?;
    let mut texto = serde_json::to_vec(&v).map_err(|e| ErroConv::Gravacao(e.to_string()))?;
    texto.resize(texto.len().div_ceil(4) * 4, b' ');
    let total = 12 + 8 + texto.len() + 8 + b.len();
    let total = u32::try_from(total).map_err(|_| ErroConv::Gravacao("GLB maior que 4 GB".into()))?;
    let mut s = Vec::with_capacity(total as usize);
    s.extend_from_slice(MAGICA);
    s.extend_from_slice(&2u32.to_le_bytes());
    s.extend_from_slice(&total.to_le_bytes());
    s.extend_from_slice(&(texto.len() as u32).to_le_bytes());
    s.extend_from_slice(&TRECHO_JSON.to_le_bytes());
    s.extend_from_slice(&texto);
    s.extend_from_slice(&(b.len() as u32).to_le_bytes());
    s.extend_from_slice(&TRECHO_BIN.to_le_bytes());
    s.extend_from_slice(&b);
    Ok(s)
}

/// Separa JSON e buffer binário de um GLB.
fn abrir_glb(b: &[u8]) -> Result<(&[u8], Option<&[u8]>), ErroConv> {
    let u = |i: usize| b.get(i..i + 4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]));
    if u(4) != Some(2) {
        return Err(erro("GLB", "só a versão 2 é suportada"));
    }
    let (mut i, mut texto, mut binario) = (12usize, None, None);
    while let (Some(n), Some(tipo)) = (u(i), u(i + 4)) {
        let corpo = b.get(i + 8..i + 8 + n as usize).ok_or_else(|| erro("GLB", "trecho cortado"))?;
        match tipo {
            TRECHO_JSON => texto = Some(corpo),
            TRECHO_BIN => binario = Some(corpo),
            _ => {}
        }
        i += 8 + n as usize;
    }
    Ok((texto.ok_or_else(|| erro("GLB", "sem JSON"))?, binario))
}

struct Dados<'a> {
    raiz: &'a Value,
    buffers: Vec<Vec<u8>>,
}

fn usize_de(v: &Value) -> Option<usize> {
    v.as_u64().map(|n| n as usize)
}

impl Dados<'_> {
    /// Números de um acessor, em ponto flutuante, um vetor por elemento.
    fn numeros(&self, i: usize) -> Result<Vec<Vec<f64>>, ErroConv> {
        let ruim = |m: &str| erro("glTF", m);
        let a = self.raiz["accessors"].get(i).ok_or_else(|| ruim("acessor inexistente"))?;
        if a.get("sparse").is_some() {
            return Err(ruim("acessor esparso não é suportado"));
        }
        let quantos = usize_de(&a["count"]).ok_or_else(|| ruim("acessor sem contagem"))?;
        let tipo = a["componentType"].as_u64().ok_or_else(|| ruim("acessor sem tipo"))? as u32;
        let largura = match a["type"].as_str() {
            Some("SCALAR") => 1,
            Some("VEC2") => 2,
            Some("VEC3") => 3,
            Some("VEC4") => 4,
            _ => return Err(ruim("tipo de acessor não suportado")),
        };
        let tamanho = match tipo {
            5120 | U8 => 1,
            5122 | U16 => 2,
            U32 | FLOAT => 4,
            _ => return Err(ruim("componente de acessor não suportado")),
        };
        let v = usize_de(&a["bufferView"]).and_then(|k| self.raiz["bufferViews"].get(k)).ok_or_else(|| ruim("acessor sem vista"))?;
        let dados = usize_de(&v["buffer"]).and_then(|k| self.buffers.get(k)).ok_or_else(|| ruim("vista sem buffer"))?;
        let inicio = usize_de(&v["byteOffset"]).unwrap_or(0) + usize_de(&a["byteOffset"]).unwrap_or(0);
        let passo = usize_de(&v["byteStride"]).filter(|s| *s > 0).unwrap_or(largura * tamanho);
        let normalizado = a["normalized"].as_bool().unwrap_or(false);
        // o buffer precisa caber antes de reservar qualquer coisa: a contagem vem do arquivo
        let fim = quantos.checked_sub(1).and_then(|q| q.checked_mul(passo)).and_then(|q| q.checked_add(inicio + largura * tamanho));
        if fim.is_some_and(|f| f > dados.len()) {
            return Err(ruim("acessor passa do fim do buffer"));
        }
        let mut saida = Vec::with_capacity(quantos);
        for e in 0..quantos {
            let mut linha = Vec::with_capacity(largura);
            for k in 0..largura {
                let o = inicio + e * passo + k * tamanho;
                let x = &dados[o..o + tamanho];
                linha.push(match tipo {
                    5120 => f64::from(x[0] as i8) / if normalizado { 127. } else { 1. },
                    U8 => f64::from(x[0]) / if normalizado { 255. } else { 1. },
                    5122 => f64::from(i16::from_le_bytes([x[0], x[1]])) / if normalizado { 32767. } else { 1. },
                    U16 => f64::from(u16::from_le_bytes([x[0], x[1]])) / if normalizado { 65535. } else { 1. },
                    U32 => f64::from(u32::from_le_bytes([x[0], x[1], x[2], x[3]])),
                    _ => f64::from(f32::from_le_bytes([x[0], x[1], x[2], x[3]])),
                });
            }
            saida.push(linha);
        }
        Ok(saida)
    }

    fn primitiva(&self, p: &Value, nome: &str, cena: &mut Cena) -> Result<Option<Malha>, ErroConv> {
        let modo = p["mode"].as_u64().unwrap_or(4);
        if !(4..=6).contains(&modo) {
            cena.ignorar("ponto ou linha", 1);
            return Ok(None);
        }
        let Some(pos) = usize_de(&p["attributes"]["POSITION"]) else { return Ok(None) };
        let mut m = Malha { nome: nome.into(), material: usize_de(&p["material"]), ..Default::default() };
        m.pos = self.numeros(pos)?.into_iter().map(|v| [v[0], v[1], v[2]]).collect();
        if let Some(n) = usize_de(&p["attributes"]["NORMAL"]) {
            m.nrm = self.numeros(n)?.into_iter().filter(|v| v.len() == 3).map(|v| [v[0] as f32, v[1] as f32, v[2] as f32]).collect();
        }
        if let Some(c) = usize_de(&p["attributes"]["COLOR_0"]) {
            let canal = |x: f64| (x.clamp(0., 1.) * 255.).round() as u8;
            m.cor_v = self.numeros(c)?.into_iter().filter(|v| v.len() >= 3).map(|v| [canal(v[0]), canal(v[1]), canal(v[2]), v.get(3).map(|a| canal(*a)).unwrap_or(255)]).collect();
        }
        let indices: Vec<u32> = match usize_de(&p["indices"]) {
            Some(i) => self.numeros(i)?.into_iter().map(|v| v[0] as u32).collect(),
            None => (0..m.pos.len() as u32).collect(),
        };
        m.tris = match modo {
            4 => indices.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect(),
            5 => indices.windows(3).enumerate().map(|(i, t)| if i % 2 == 0 { [t[0], t[1], t[2]] } else { [t[1], t[0], t[2]] }).collect(),
            _ => indices.windows(2).skip(1).map(|t| [indices[0], t[0], t[1]]).collect(),
        };
        m.conferir("glTF")?;
        Ok(Some(m))
    }
}

/// Matriz de um nó, por `matrix` ou por deslocamento, giro e escala.
fn matriz_do_no(n: &Value) -> [f64; 12] {
    let lista = |chave: &str| -> Option<Vec<f64>> { n.get(chave)?.as_array().map(|a| a.iter().filter_map(Value::as_f64).collect()) };
    if let Some(m) = lista("matrix").filter(|m| m.len() == 16) {
        return [m[0], m[4], m[8], m[12], m[1], m[5], m[9], m[13], m[2], m[6], m[10], m[14]];
    }
    let t = lista("translation").filter(|v| v.len() == 3).unwrap_or(vec![0.; 3]);
    let q = lista("rotation").filter(|v| v.len() == 4).unwrap_or(vec![0., 0., 0., 1.]);
    let s = lista("scale").filter(|v| v.len() == 3).unwrap_or(vec![1.; 3]);
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let r = [
        [1. - 2. * (y * y + z * z), 2. * (x * y - z * w), 2. * (x * z + y * w)],
        [2. * (x * y + z * w), 1. - 2. * (x * x + z * z), 2. * (y * z - x * w)],
        [2. * (x * z - y * w), 2. * (y * z + x * w), 1. - 2. * (x * x + y * y)],
    ];
    [r[0][0] * s[0], r[0][1] * s[1], r[0][2] * s[2], t[0], r[1][0] * s[0], r[1][1] * s[1], r[1][2] * s[2], t[1], r[2][0] * s[0], r[2][1] * s[1], r[2][2] * s[2], t[2]]
}

pub(crate) fn ler(b: &[u8], caminho: &Path) -> Result<Cena, ErroConv> {
    let (texto, binario) = if b.starts_with(MAGICA) { abrir_glb(b)? } else { (b, None) };
    let raiz: Value = serde_json::from_slice(texto).map_err(|e| erro("glTF", e))?;
    if !raiz["asset"]["version"].as_str().unwrap_or("").starts_with('2') {
        return Err(erro("glTF", "só a versão 2 é suportada"));
    }
    if let Some(l) = raiz["extensionsRequired"].as_array().filter(|l| !l.is_empty()) {
        let nomes: Vec<&str> = l.iter().filter_map(Value::as_str).collect();
        return Err(erro("glTF", format!("o arquivo exige extensões que o Fenkra não lê: {}", nomes.join(", "))));
    }
    let mut buffers = Vec::new();
    for (i, q) in raiz["buffers"].as_array().map(Vec::as_slice).unwrap_or_default().iter().enumerate() {
        let dados = match q["uri"].as_str() {
            Some(u) if u.starts_with("data:") => u.split_once(',').and_then(|(_, d)| de_base64(d)).ok_or_else(|| erro("glTF", "buffer embutido inválido"))?,
            Some(u) => {
                let p = ao_lado(caminho, u).ok_or_else(|| erro("glTF", format!("buffer fora da pasta do arquivo: {u}")))?;
                std::fs::read(&p).map_err(|e| erro("glTF", format!("buffer {}: {e}", p.display())))?
            }
            None if i == 0 => binario.ok_or_else(|| erro("glTF", "buffer sem dados"))?.to_vec(),
            None => return Err(erro("glTF", "buffer sem dados")),
        };
        buffers.push(dados);
    }
    let dados = Dados { raiz: &raiz, buffers };
    let mut c = Cena { unidade: "METRE".into(), ..Default::default() };
    for k in raiz["materials"].as_array().map(Vec::as_slice).unwrap_or_default() {
        let f = |v: &Value, padrao: f32| v.as_f64().map(|x| x as f32).unwrap_or(padrao);
        let pbr = &k["pbrMetallicRoughness"];
        let base: Vec<f32> = pbr["baseColorFactor"].as_array().map(|a| a.iter().map(|v| f(v, 1.)).collect()).unwrap_or_default();
        let cor = if let [r, g, b, a] = base[..] { [r, g, b, a] } else { [1.; 4] };
        c.materiais.push(Material { nome: k["name"].as_str().unwrap_or("").into(), cor, metal: f(&pbr["metallicFactor"], 1.), rugosidade: f(&pbr["roughnessFactor"], 1.) });
    }
    let contar = |chave: &str| raiz[chave].as_array().map(Vec::len).unwrap_or(0);
    for (chave, oque) in [("animations", "animação"), ("skins", "esqueleto"), ("cameras", "câmera"), ("textures", "textura")] {
        c.ignorar(oque, contar(chave));
    }
    let mut por_malha: Vec<Vec<usize>> = Vec::new();
    for g in raiz["meshes"].as_array().map(Vec::as_slice).unwrap_or_default() {
        let nome = g["name"].as_str().unwrap_or("");
        let mut lista = Vec::new();
        for p in g["primitives"].as_array().map(Vec::as_slice).unwrap_or_default() {
            if let Some(mut m) = dados.primitiva(p, nome, &mut c)? {
                if m.material.is_some_and(|k| k >= c.materiais.len()) {
                    m.material = None;
                }
                lista.push(c.malhas.len());
                c.malhas.push(m);
            }
        }
        por_malha.push(lista);
    }
    let nos = raiz["nodes"].as_array().map(Vec::as_slice).unwrap_or_default();
    for n in nos {
        let mut no = No::novo(n["name"].as_str().unwrap_or(""));
        no.local = matriz_do_no(n);
        no.malhas = usize_de(&n["mesh"]).and_then(|i| por_malha.get(i)).cloned().unwrap_or_default();
        no.filhos = n["children"].as_array().map(|a| a.iter().filter_map(usize_de).filter(|i| *i < nos.len()).collect()).unwrap_or_default();
        c.nos.push(no);
    }
    let cena = usize_de(&raiz["scene"]).unwrap_or(0);
    c.raizes = match raiz["scenes"][cena]["nodes"].as_array() {
        Some(l) => l.iter().filter_map(usize_de).filter(|i| *i < nos.len()).collect(),
        None => {
            let filhos: std::collections::HashSet<usize> = c.nos.iter().flat_map(|n| n.filhos.iter().copied()).collect();
            (0..nos.len()).filter(|i| !filhos.contains(i)).collect()
        }
    };
    c.embrulhar(Y_PARA_Z);
    c.escalar(1000.);
    Ok(c)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn base64_nos_vetores_da_norma() {
        for (a, b) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")] {
            assert_eq!(base64(a.as_bytes()), b);
            assert_eq!(de_base64(b).unwrap(), a.as_bytes());
        }
    }

    #[test]
    fn giro_por_quaternio_acerta_noventa_graus() {
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let m = matriz_do_no(&json!({"rotation": [0., 0., s, s], "translation": [1., 2., 3.]}));
        assert!((m[1] + 1.).abs() < 1e-12 && (m[4] - 1.).abs() < 1e-12, "{m:?}");
        assert_eq!([m[3], m[7], m[11]], [1., 2., 3.]);
    }

    #[test]
    fn acessor_que_passa_do_buffer_e_recusado() {
        let raiz = json!({"accessors": [{"count": 4_000_000_000u64, "componentType": FLOAT, "type": "VEC3", "bufferView": 0}], "bufferViews": [{"buffer": 0}]});
        let d = Dados { raiz: &raiz, buffers: vec![vec![0; 16]] };
        assert!(d.numeros(0).is_err());
    }
}
