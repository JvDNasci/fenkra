// SPDX-License-Identifier: LGPL-2.1-or-later
//! PLY: vértices com propriedades e faces em lista, em texto ou binário. Sem nomes, unidade nem árvore;
//! a cor vai por vértice, e por isso as peças podem ter cores diferentes num arquivo só.

use crate::{erro, Cena, Malha};
use crate::Erro as ErroConv;
use std::fmt::Write as _;

#[derive(Clone, Copy, PartialEq)]
enum Tipo {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}

impl Tipo {
    fn de(nome: &str) -> Option<Tipo> {
        Some(match nome {
            "char" | "int8" => Tipo::I8,
            "uchar" | "uint8" => Tipo::U8,
            "short" | "int16" => Tipo::I16,
            "ushort" | "uint16" => Tipo::U16,
            "int" | "int32" => Tipo::I32,
            "uint" | "uint32" => Tipo::U32,
            "float" | "float32" => Tipo::F32,
            "double" | "float64" => Tipo::F64,
            _ => return None,
        })
    }

    fn bytes(self) -> usize {
        match self {
            Tipo::I8 | Tipo::U8 => 1,
            Tipo::I16 | Tipo::U16 => 2,
            Tipo::I32 | Tipo::U32 | Tipo::F32 => 4,
            Tipo::F64 => 8,
        }
    }
}

struct Propriedade {
    nome: String,
    tipo: Tipo,
    /// tipo da contagem, nas propriedades em lista
    lista: Option<Tipo>,
}

struct Elemento {
    nome: String,
    quantos: usize,
    props: Vec<Propriedade>,
}

#[derive(Clone, Copy, PartialEq)]
enum Codigo {
    Texto,
    Pequeno,
    Grande,
}

/// Lê números do corpo, em qualquer das três codificações.
struct Corpo<'a> {
    b: &'a [u8],
    i: usize,
    codigo: Codigo,
}

impl Corpo<'_> {
    fn numero(&mut self, t: Tipo) -> Result<f64, ErroConv> {
        if self.codigo == Codigo::Texto {
            while self.b.get(self.i).is_some_and(u8::is_ascii_whitespace) {
                self.i += 1;
            }
            let inicio = self.i;
            while self.b.get(self.i).is_some_and(|c| !c.is_ascii_whitespace()) {
                self.i += 1;
            }
            return std::str::from_utf8(&self.b[inicio..self.i]).ok().and_then(|x| x.parse().ok()).ok_or_else(|| erro("PLY", "número inválido"));
        }
        let n = t.bytes();
        let fatia = self.b.get(self.i..self.i + n).ok_or_else(|| erro("PLY", "arquivo cortado"))?;
        self.i += n;
        let mut c = [0u8; 8];
        c[..n].copy_from_slice(fatia);
        if self.codigo == Codigo::Grande {
            c[..n].reverse();
        }
        let [a, b, d, e, ..] = c;
        Ok(match t {
            Tipo::I8 => f64::from(a as i8),
            Tipo::U8 => f64::from(a),
            Tipo::I16 => f64::from(i16::from_le_bytes([a, b])),
            Tipo::U16 => f64::from(u16::from_le_bytes([a, b])),
            Tipo::I32 => f64::from(i32::from_le_bytes([a, b, d, e])),
            Tipo::U32 => f64::from(u32::from_le_bytes([a, b, d, e])),
            Tipo::F32 => f64::from(f32::from_le_bytes([a, b, d, e])),
            Tipo::F64 => f64::from_le_bytes(c),
        })
    }
}

fn cabecalho(b: &[u8]) -> Result<(Vec<Elemento>, Codigo, usize, Vec<String>), ErroConv> {
    let (mut els, mut codigo, mut comentarios): (Vec<Elemento>, _, Vec<String>) = (Vec::new(), None, Vec::new());
    let mut i = 0;
    loop {
        let fim = b[i..].iter().position(|c| *c == b'\n').map(|p| p + i).ok_or_else(|| erro("PLY", "cabeçalho sem fim"))?;
        let linha = String::from_utf8_lossy(&b[i..fim]);
        i = fim + 1;
        let w: Vec<&str> = linha.split_whitespace().collect();
        match w.as_slice() {
            ["end_header"] => break,
            ["format", f, ..] => {
                codigo = Some(match *f {
                    "ascii" => Codigo::Texto,
                    "binary_little_endian" => Codigo::Pequeno,
                    "binary_big_endian" => Codigo::Grande,
                    _ => return Err(erro("PLY", "codificação desconhecida")),
                })
            }
            ["comment", resto @ ..] => comentarios.push(resto.join(" ")),
            ["element", nome, n] => {
                let quantos = n.parse().map_err(|_| erro("PLY", "contagem inválida"))?;
                els.push(Elemento { nome: nome.to_string(), quantos, props: vec![] });
            }
            ["property", "list", c, t, nome] => {
                let (Some(c), Some(t), Some(e)) = (Tipo::de(c), Tipo::de(t), els.last_mut()) else { return Err(erro("PLY", "propriedade inválida")) };
                e.props.push(Propriedade { nome: nome.to_string(), tipo: t, lista: Some(c) });
            }
            ["property", t, nome] => {
                let (Some(t), Some(e)) = (Tipo::de(t), els.last_mut()) else { return Err(erro("PLY", "propriedade inválida")) };
                e.props.push(Propriedade { nome: nome.to_string(), tipo: t, lista: None });
            }
            _ => {}
        }
    }
    Ok((els, codigo.ok_or_else(|| erro("PLY", "sem linha de formato"))?, i, comentarios))
}

pub fn ler(b: &[u8]) -> Result<Cena, ErroConv> {
    let (elementos, codigo, inicio, comentarios) = cabecalho(b)?;
    let mut corpo = Corpo { b, i: inicio, codigo };
    let mut m = Malha::default();
    let mut c = Cena::default();
    for e in &elementos {
        if e.props.is_empty() {
            continue;
        }
        for _ in 0..e.quantos {
            if corpo.i >= b.len() {
                return Err(erro("PLY", "arquivo cortado"));
            }
            let (mut p, mut n, mut cor, mut face) = ([0f64; 3], [0f32; 3], [0u8, 0, 0, 255], Vec::new());
            let (mut tem_n, mut tem_cor) = (false, false);
            for q in &e.props {
                if let Some(contagem) = q.lista {
                    let quantos = corpo.numero(contagem)? as usize;
                    let mut lista = Vec::new();
                    for _ in 0..quantos {
                        lista.push(corpo.numero(q.tipo)? as u32);
                    }
                    if matches!(q.nome.as_str(), "vertex_indices" | "vertex_index") {
                        face = lista;
                    }
                    continue;
                }
                let v = corpo.numero(q.tipo)?;
                // cor em ponto flutuante vai de 0 a 1; inteira, de 0 a 255
                let canal = |v: f64| if matches!(q.tipo, Tipo::F32 | Tipo::F64) { (v * 255.).round() as u8 } else { v as u8 };
                match q.nome.as_str() {
                    "x" => p[0] = v,
                    "y" => p[1] = v,
                    "z" => p[2] = v,
                    "nx" => (n[0], tem_n) = (v as f32, true),
                    "ny" => n[1] = v as f32,
                    "nz" => n[2] = v as f32,
                    "red" | "diffuse_red" => (cor[0], tem_cor) = (canal(v), true),
                    "green" | "diffuse_green" => cor[1] = canal(v),
                    "blue" | "diffuse_blue" => cor[2] = canal(v),
                    "alpha" => cor[3] = canal(v),
                    _ => {}
                }
            }
            match e.nome.as_str() {
                "vertex" => {
                    m.pos.push(p);
                    if tem_n {
                        m.nrm.push(n);
                    }
                    if tem_cor {
                        m.cor_v.push(cor);
                    }
                }
                "face" => {
                    for k in 1..face.len().saturating_sub(1) {
                        m.tris.push([face[0], face[k], face[k + 1]]);
                    }
                }
                _ => {}
            }
        }
        if !matches!(e.nome.as_str(), "vertex" | "face") {
            c.ignorar("elemento extra", e.quantos);
        }
    }
    let texturas = comentarios.iter().filter(|t| t.to_lowercase().starts_with("texturefile")).count();
    c.ignorar("textura", texturas);
    m.conferir("PLY")?;
    c.raiz_com(m);
    c.separar_por_cor();
    Ok(c)
}

/// Tudo numa malha só, com a cor de cada peça nos vértices dela.
fn juntar(cena: &Cena) -> (Malha, bool) {
    let mut j = Malha::default();
    let malhas = cena.no_lugar();
    let com_cor = malhas.iter().any(|m| cena.cor_de(m).is_some());
    for m in malhas {
        let s = m.soldada();
        let cor = cena.cor_de(&m).unwrap_or([0.8, 0.8, 0.8, 1.]).map(|v| (v.clamp(0., 1.) * 255.).round() as u8);
        let base = j.pos.len() as u32;
        if com_cor {
            if s.cor_v.len() == s.pos.len() {
                j.cor_v.extend(&s.cor_v);
            } else {
                j.cor_v.resize(j.cor_v.len() + s.pos.len(), cor);
            }
        }
        j.pos.extend(&s.pos);
        j.tris.extend(s.tris.iter().map(|t| t.map(|i| i + base)));
    }
    (j, com_cor)
}

pub fn gravar(cena: &Cena, binario: bool, mm: f64) -> Vec<u8> {
    let (m, com_cor) = juntar(cena);
    let mut h = String::from("ply\n");
    h.push_str(if binario { "format binary_little_endian 1.0\n" } else { "format ascii 1.0\n" });
    h.push_str("comment Fenkra\n");
    let _ = writeln!(h, "element vertex {}", m.pos.len());
    h.push_str("property double x\nproperty double y\nproperty double z\n");
    if com_cor {
        h.push_str("property uchar red\nproperty uchar green\nproperty uchar blue\nproperty uchar alpha\n");
    }
    let _ = writeln!(h, "element face {}\nproperty list uchar uint vertex_indices\nend_header", m.tris.len());
    let mut s = h.into_bytes();
    for (i, p) in m.pos.iter().enumerate() {
        let q = p.map(|v| v / mm);
        if binario {
            q.iter().for_each(|v| s.extend_from_slice(&v.to_le_bytes()));
            if com_cor {
                s.extend_from_slice(&m.cor_v[i]);
            }
        } else {
            let mut l = format!("{} {} {}", q[0], q[1], q[2]);
            if com_cor {
                let c = m.cor_v[i];
                let _ = write!(l, " {} {} {} {}", c[0], c[1], c[2], c[3]);
            }
            l.push('\n');
            s.extend_from_slice(l.as_bytes());
        }
    }
    for t in &m.tris {
        if binario {
            s.push(3);
            t.iter().for_each(|i| s.extend_from_slice(&i.to_le_bytes()));
        } else {
            s.extend_from_slice(format!("3 {} {} {}\n", t[0], t[1], t[2]).as_bytes());
        }
    }
    s
}
