// SPDX-License-Identifier: LGPL-2.1-or-later
//! Escritor de glTF 2.0 binário (GLB) do pacote de visualização.
//!
//! As posições ficam em milímetros, com Z para cima, como no modelo. Para o arquivo continuar sendo um
//! glTF válido em qualquer leitor, a raiz `fkv:raiz` traz a matriz que passa para metros e gira Z para
//! Y; o visualizador do Fenkra ignora essa raiz e trabalha em milímetros. Normais saem em 8 bits
//! (extensão `KHR_mesh_quantization`) e índices em 16 bits quando cabem: o arquivo é para ver, e o
//! tamanho é o que decide se abre bem no celular.

use serde_json::{json, Map, Value};
use std::collections::HashMap;

const FLOAT: u32 = 5126;
const BYTE: u32 = 5120;
const U16: u32 = 5123;
const U32: u32 = 5125;

/// Matriz 3x4 por linhas (a do Fenkra) para a 4x4 por colunas do glTF.
pub fn coluna(m: &[f64; 12]) -> [f64; 16] {
    [m[0], m[4], m[8], 0., m[1], m[5], m[9], 0., m[2], m[6], m[10], 0., m[3], m[7], m[11], 1.]
}

#[derive(Default)]
pub struct Construtor {
    bin: Vec<u8>,
    vistas: Vec<Value>,
    acessores: Vec<Value>,
    malhas: Vec<Value>,
    materiais: Vec<Value>,
    cores: HashMap<[u8; 3], usize>,
    nos: Vec<Value>,
}

fn relevo(v: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        0.
    }
}

impl Construtor {
    fn dados(&mut self, dados: &[u8], alvo: u32, passo: Option<usize>) -> usize {
        while self.bin.len() % 4 != 0 {
            self.bin.push(0);
        }
        let mut v = json!({"buffer": 0, "byteOffset": self.bin.len(), "byteLength": dados.len(), "target": alvo});
        if let Some(p) = passo {
            v["byteStride"] = json!(p);
        }
        self.bin.extend_from_slice(dados);
        self.vistas.push(v);
        self.vistas.len() - 1
    }

    fn acessor(&mut self, vista: usize, tipo: u32, n: usize, t: &str, extra: Value) -> usize {
        let mut a = json!({"bufferView": vista, "componentType": tipo, "count": n, "type": t});
        if let (Some(o), Some(e)) = (a.as_object_mut(), extra.as_object()) {
            for (k, v) in e {
                o.insert(k.clone(), v.clone());
            }
        }
        self.acessores.push(a);
        self.acessores.len() - 1
    }

    fn material(&mut self, cor: [u8; 3]) -> usize {
        if let Some(i) = self.cores.get(&cor) {
            return *i;
        }
        let f = cor.map(|c| (f64::from(c) / 255.).powf(2.2));
        self.materiais.push(json!({
            "name": format!("cor-{:02x}{:02x}{:02x}", cor[0], cor[1], cor[2]),
            "pbrMetallicRoughness": {"baseColorFactor": [f[0], f[1], f[2], 1.], "metallicFactor": 0.1, "roughnessFactor": 0.55},
            "doubleSided": true
        }));
        self.cores.insert(cor, self.materiais.len() - 1);
        self.materiais.len() - 1
    }

    /// Acrescenta uma malha (posições em mm) e devolve o índice dela. `normais`, quando vazio, deixa o
    /// leitor calcular.
    pub fn malha(&mut self, nome: &str, pos: &[f32], normais: &[f32], indices: &[u32], cor: [u8; 3], extras: Value) -> usize {
        let n = pos.len() / 3;
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        let mut bytes = Vec::with_capacity(pos.len() * 4);
        for p in pos.chunks_exact(3) {
            for i in 0..3 {
                let v = relevo(p[i]);
                lo[i] = lo[i].min(v);
                hi[i] = hi[i].max(v);
                bytes.extend_from_slice(&v.to_le_bytes());
            }
        }
        let vp = self.dados(&bytes, 34962, Some(12));
        let ap = self.acessor(vp, FLOAT, n, "VEC3", json!({"min": lo, "max": hi}));
        let mut atributos = Map::new();
        atributos.insert("POSITION".into(), json!(ap));
        if normais.len() == pos.len() {
            let mut b = Vec::with_capacity(n * 4);
            for v in normais.chunks_exact(3) {
                let q = |x: f32| (relevo(x).clamp(-1., 1.) * 127.).round() as i8 as u8;
                b.extend_from_slice(&[q(v[0]), q(v[1]), q(v[2]), 0]);
            }
            let vn = self.dados(&b, 34962, Some(4));
            let an = self.acessor(vn, BYTE, n, "VEC3", json!({"normalized": true}));
            atributos.insert("NORMAL".into(), json!(an));
        }
        let (bytes, tipo): (Vec<u8>, u32) = if n <= 65535 {
            (indices.iter().flat_map(|i| (*i as u16).to_le_bytes()).collect(), U16)
        } else {
            (indices.iter().flat_map(|i| i.to_le_bytes()).collect(), U32)
        };
        let vi = self.dados(&bytes, 34963, None);
        let ai = self.acessor(vi, tipo, indices.len(), "SCALAR", json!({}));
        let m = self.material(cor);
        self.malhas.push(json!({"name": nome, "primitives": [{"attributes": atributos, "indices": ai, "material": m, "mode": 4}], "extras": extras}));
        self.malhas.len() - 1
    }

    /// Nó novo. `id` é o ID estável do Fenkra, gravado em `extras.fkv`.
    pub fn no(&mut self, nome: &str, id: &str, matriz: Option<[f64; 12]>, malhas: &[usize]) -> usize {
        let mut v = json!({"name": nome, "extras": {"fkv": id}});
        if let Some(m) = matriz {
            if m != fenkra_malha::IDENTIDADE {
                v["matrix"] = json!(coluna(&m));
            }
        }
        // um nó pode ter uma malha só; vários corpos viram nós filhos, para cada um ter o seu ID
        if let [m] = malhas {
            v["mesh"] = json!(m);
        }
        self.nos.push(v);
        self.nos.len() - 1
    }

    pub fn filho(&mut self, pai: usize, filho: usize) {
        let o = self.nos[pai].as_object_mut().expect("nó é objeto");
        o.entry("children").or_insert_with(|| json!([])).as_array_mut().expect("lista").push(json!(filho));
    }

    /// Fecha o arquivo: põe a raiz de conversão de unidade e eixo e devolve o GLB.
    pub fn finalizar(mut self, raizes: &[usize], nome: &str) -> Vec<u8> {
        let k = 0.001;
        let raiz = json!({"name": "fkv:raiz", "children": raizes, "extras": {"fkv_raiz": true},
            "matrix": [k, 0., 0., 0., 0., 0., -k, 0., 0., k, 0., 0., 0., 0., 0., 1.]});
        self.nos.push(raiz);
        let topo = self.nos.len() - 1;
        let mut j = json!({
            "asset": {"version": "2.0", "generator": "Fenkra", "extras": {"unidadeDoModelo": "mm", "eixoParaCima": "Z"}},
            "extensionsUsed": ["KHR_mesh_quantization"],
            "extensionsRequired": ["KHR_mesh_quantization"],
            "scene": 0,
            "scenes": [{"name": nome, "nodes": [topo]}],
            "nodes": self.nos,
            "buffers": [{"byteLength": (self.bin.len() + 3) / 4 * 4}],
        });
        for (k, v) in [("meshes", self.malhas), ("materials", self.materiais), ("accessors", self.acessores), ("bufferViews", self.vistas)] {
            if !v.is_empty() {
                j[k] = Value::Array(v);
            }
        }
        let mut texto = serde_json::to_vec(&j).expect("json");
        while texto.len() % 4 != 0 {
            texto.push(b' ');
        }
        let mut bin = self.bin;
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let total = 12 + 8 + texto.len() + if bin.is_empty() { 0 } else { 8 + bin.len() };
        let mut s = Vec::with_capacity(total);
        s.extend_from_slice(b"glTF");
        s.extend_from_slice(&2u32.to_le_bytes());
        s.extend_from_slice(&(total as u32).to_le_bytes());
        s.extend_from_slice(&(texto.len() as u32).to_le_bytes());
        s.extend_from_slice(&0x4e4f_534au32.to_le_bytes());
        s.extend_from_slice(&texto);
        if !bin.is_empty() {
            s.extend_from_slice(&(bin.len() as u32).to_le_bytes());
            s.extend_from_slice(&0x004e_4942u32.to_le_bytes());
            s.extend_from_slice(&bin);
        }
        s
    }
}

/// Lê o trecho JSON de um GLB e confere o cabeçalho e os tamanhos: base do teste de conformidade.
pub fn ler_json(b: &[u8]) -> Result<(Value, usize), String> {
    if b.len() < 20 || &b[0..4] != b"glTF" {
        return Err("não é um GLB".into());
    }
    let u = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) as usize;
    if u(4) != 2 || u(8) != b.len() {
        return Err("versão ou tamanho do GLB não confere".into());
    }
    let tj = u(12);
    if u(16) != 0x4e4f_534a || 20 + tj > b.len() {
        return Err("trecho JSON ausente".into());
    }
    let j: Value = serde_json::from_slice(&b[20..20 + tj]).map_err(|e| format!("JSON do GLB: {e}"))?;
    let bin = if 20 + tj + 8 <= b.len() { u(20 + tj) } else { 0 };
    Ok((j, bin))
}
