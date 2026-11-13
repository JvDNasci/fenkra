// SPDX-License-Identifier: LGPL-2.1-or-later
//! Leitura de FBX (B210), binário e texto, em Rust puro.
//!
//! O FBX é uma árvore de nós com nome e uma lista de valores. A forma binária guarda cada nó com o
//! deslocamento do fim dele, o número de valores e o tamanho da lista; os vetores grandes podem vir
//! comprimidos em zlib. A forma texto escreve a mesma árvore como `Nome: valor, valor { ... }`.
//! As duas viram a mesma [`Nodo`], e daí a mesma cena:
//!
//! - `Objects/Geometry` traz `Vertices` (x, y, z seguidos) e `PolygonVertexIndex`, em que o índice
//!   negativo fecha o polígono (vale `-i - 1`). Cada polígono vira um leque de triângulos.
//! - `Objects/Model` é um nó da árvore, com `Lcl Translation`, `Lcl Rotation` (graus, ordem XYZ) e
//!   `Lcl Scaling` em `Properties70`.
//! - `Objects/Material` dá a cor (`DiffuseColor` e `Opacity`).
//! - `Connections` liga tudo: `C: "OO", filho, pai`. Pai zero é a raiz da cena.
//! - `GlobalSettings` diz a unidade (`UnitScaleFactor`, em centímetros por unidade) e o eixo de cima.
//!
//! Animação, esqueleto, câmera, luz, textura e pivôs não atravessam: entram no relatório. Só leitura;
//! o Fenkra não grava FBX (o formato é fechado e a frente só grava formatos abertos).

use super::{erro, Cena, Malha, No};
use crate::ErroConv;
use fenkra_malha::{compor, IDENTIDADE, PROFUNDIDADE};
use std::collections::HashMap;

/// Assinatura fixa do começo do arquivo binário (21 bytes, seguidos de 0x1A 0x00 e da versão).
const ASSINATURA: &[u8] = &[0x4B, 0x61, 0x79, 0x64, 0x61, 0x72, 0x61, 0x20, 0x46, 0x42, 0x58, 0x20, 0x42, 0x69, 0x6E, 0x61, 0x72, 0x79, 0x20, 0x20, 0x00];
/// Teto de elementos num vetor: um arquivo hostil não pede gigabytes com poucos bytes.
const LIMITE_VETOR: usize = 200_000_000;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Valor {
    Int(i64),
    Num(f64),
    Texto(String),
    Ints(Vec<i64>),
    Nums(Vec<f64>),
    Bruto,
}

impl Valor {
    fn num(&self) -> Option<f64> {
        match self {
            Valor::Int(i) => Some(*i as f64),
            Valor::Num(x) => Some(*x),
            Valor::Texto(t) => t.trim().parse().ok(),
            _ => None,
        }
    }
    fn int(&self) -> Option<i64> {
        match self {
            Valor::Int(i) => Some(*i),
            Valor::Num(x) => Some(*x as i64),
            Valor::Texto(t) => t.trim().parse().ok(),
            _ => None,
        }
    }
    fn texto(&self) -> &str {
        match self {
            Valor::Texto(t) => t,
            _ => "",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Nodo {
    pub nome: String,
    pub valores: Vec<Valor>,
    pub filhos: Vec<Nodo>,
}

impl Nodo {
    fn filho(&self, nome: &str) -> Option<&Nodo> {
        self.filhos.iter().find(|f| f.nome == nome)
    }
    fn todos<'a>(&'a self, nome: &'a str) -> impl Iterator<Item = &'a Nodo> {
        self.filhos.iter().filter(move |f| f.nome == nome)
    }
    /// Números do nó: o vetor binário, ou no texto a lista solta ou o filho `a` (forma `*N { a: ... }`).
    fn numeros(&self) -> Vec<f64> {
        let de = |v: &[Valor]| -> Vec<f64> {
            let mut out = vec![];
            for x in v {
                match x {
                    Valor::Nums(n) => out.extend_from_slice(n),
                    Valor::Ints(n) => out.extend(n.iter().map(|i| *i as f64)),
                    Valor::Int(i) => out.push(*i as f64),
                    Valor::Num(f) => out.push(*f),
                    _ => {}
                }
            }
            out
        };
        match self.filho("a") {
            Some(a) => de(&a.valores),
            None => de(&self.valores),
        }
    }
    fn inteiros(&self) -> Vec<i64> {
        let de = |v: &[Valor]| -> Vec<i64> {
            let mut out = vec![];
            for x in v {
                match x {
                    Valor::Ints(n) => out.extend_from_slice(n),
                    Valor::Nums(n) => out.extend(n.iter().map(|f| *f as i64)),
                    Valor::Int(i) => out.push(*i),
                    Valor::Num(f) => out.push(*f as i64),
                    _ => {}
                }
            }
            out
        };
        match self.filho("a") {
            Some(a) => de(&a.valores),
            None => de(&self.valores),
        }
    }
    /// Valor de uma propriedade de `Properties70` (`P: "nome", tipo, rótulo, opções, v1, v2, v3`).
    fn propriedade(&self, nome: &str) -> Option<Vec<f64>> {
        let p = self.filho("Properties70")?.todos("P").find(|p| p.valores.first().map(Valor::texto) == Some(nome))?;
        Some(p.valores.iter().skip(4).filter_map(Valor::num).collect())
    }
}

// ------------------------------------------------------------------ forma binária

struct Leitor<'a> {
    b: &'a [u8],
    pos: usize,
    largo: bool,
}

impl Leitor<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8], ErroConv> {
        let fim = self.pos.checked_add(n).filter(|f| *f <= self.b.len()).ok_or_else(|| erro("FBX", "arquivo cortado"))?;
        let s = &self.b[self.pos..fim];
        self.pos = fim;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, ErroConv> {
        Ok(self.bytes(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, ErroConv> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, ErroConv> {
        Ok(u64::from_le_bytes(self.bytes(8)?.try_into().unwrap()))
    }
    fn tam(&mut self) -> Result<u64, ErroConv> {
        if self.largo {
            self.u64()
        } else {
            self.u32().map(u64::from)
        }
    }

    fn vetor(&mut self, tipo: u8) -> Result<Valor, ErroConv> {
        let n = self.u32()? as usize;
        let codificacao = self.u32()?;
        let comprimido = self.u32()? as usize;
        let largura = match tipo {
            b'f' | b'i' => 4,
            b'd' | b'l' => 8,
            _ => 1,
        };
        if n > LIMITE_VETOR {
            return Err(erro("FBX", "vetor grande demais"));
        }
        let cru = self.bytes(comprimido)?;
        let dados: Vec<u8> = match codificacao {
            0 => cru.to_vec(),
            1 => miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(cru, n * largura).map_err(|_| erro("FBX", "vetor comprimido inválido"))?,
            c => return Err(erro("FBX", format!("codificação de vetor desconhecida ({c})"))),
        };
        if dados.len() < n * largura {
            return Err(erro("FBX", "vetor mais curto que o declarado"));
        }
        let pedacos = dados.chunks_exact(largura).take(n);
        Ok(match tipo {
            b'f' => Valor::Nums(pedacos.map(|c| f32::from_le_bytes(c.try_into().unwrap()) as f64).collect()),
            b'd' => Valor::Nums(pedacos.map(|c| f64::from_le_bytes(c.try_into().unwrap())).collect()),
            b'i' => Valor::Ints(pedacos.map(|c| i32::from_le_bytes(c.try_into().unwrap()) as i64).collect()),
            b'l' => Valor::Ints(pedacos.map(|c| i64::from_le_bytes(c.try_into().unwrap())).collect()),
            _ => Valor::Ints(pedacos.map(|c| c[0] as i64).collect()),
        })
    }

    fn valor(&mut self) -> Result<Valor, ErroConv> {
        let tipo = self.u8()?;
        Ok(match tipo {
            b'Y' => Valor::Int(i16::from_le_bytes(self.bytes(2)?.try_into().unwrap()) as i64),
            b'C' => Valor::Int(self.u8()? as i64),
            b'I' => Valor::Int(i32::from_le_bytes(self.bytes(4)?.try_into().unwrap()) as i64),
            b'L' => Valor::Int(i64::from_le_bytes(self.bytes(8)?.try_into().unwrap())),
            b'F' => Valor::Num(f32::from_le_bytes(self.bytes(4)?.try_into().unwrap()) as f64),
            b'D' => Valor::Num(f64::from_le_bytes(self.bytes(8)?.try_into().unwrap())),
            b'S' => {
                let n = self.u32()? as usize;
                Valor::Texto(String::from_utf8_lossy(self.bytes(n)?).into_owned())
            }
            b'R' => {
                let n = self.u32()? as usize;
                self.bytes(n)?;
                Valor::Bruto
            }
            b'f' | b'd' | b'i' | b'l' | b'b' => self.vetor(tipo)?,
            t => return Err(erro("FBX", format!("tipo de valor desconhecido ({})", t as char))),
        })
    }

    /// Lê um nó; `None` no registro nulo que fecha uma lista.
    fn nodo(&mut self, nivel: usize) -> Result<Option<Nodo>, ErroConv> {
        if nivel > PROFUNDIDADE {
            return Err(erro("FBX", "árvore funda demais"));
        }
        let fim = self.tam()? as usize;
        let n_valores = self.tam()?;
        let _tam_lista = self.tam()?;
        let n_nome = self.u8()? as usize;
        if fim == 0 {
            return Ok(None);
        }
        if fim > self.b.len() || fim <= self.pos {
            return Err(erro("FBX", "nó com fim fora do arquivo"));
        }
        let nome = String::from_utf8_lossy(self.bytes(n_nome)?).into_owned();
        let mut valores = Vec::with_capacity(n_valores.min(64) as usize);
        for _ in 0..n_valores {
            valores.push(self.valor()?);
        }
        let mut filhos = vec![];
        while self.pos < fim {
            match self.nodo(nivel + 1)? {
                Some(f) => filhos.push(f),
                None => break,
            }
        }
        self.pos = fim;
        Ok(Some(Nodo { nome, valores, filhos }))
    }
}

pub(crate) fn e_binario(b: &[u8]) -> bool {
    b.starts_with(ASSINATURA)
}

/// O texto do FBX começa com comentários `;` e tem `FBXHeaderExtension` logo no começo.
pub(crate) fn e_texto(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(&b[..b.len().min(2048)]);
    t.contains("FBXHeaderExtension:") || (t.trim_start().starts_with("; FBX") && t.contains("Objects:"))
}

fn ler_binario(b: &[u8]) -> Result<Nodo, ErroConv> {
    if b.len() < 27 {
        return Err(erro("FBX", "arquivo cortado"));
    }
    let versao = u32::from_le_bytes(b[23..27].try_into().unwrap());
    let mut l = Leitor { b, pos: 27, largo: versao >= 7500 };
    let mut raiz = Nodo::default();
    // o fim do arquivo tem um rodapé sem nós; para no registro nulo ou quando não cabe mais um nó
    let cab = if l.largo { 25 } else { 13 };
    while l.pos + cab <= b.len() {
        match l.nodo(0)? {
            Some(n) => raiz.filhos.push(n),
            None => break,
        }
    }
    Ok(raiz)
}

// ------------------------------------------------------------------ forma texto

#[derive(Clone, Debug, PartialEq)]
enum Ficha {
    Chave(String),
    Palavra(String),
    Texto(String),
    Numero(f64, bool),
    Contagem,
    Abre,
    Fecha,
    Virgula,
}

fn fichas(t: &str) -> Vec<Ficha> {
    let c: Vec<char> = t.chars().collect();
    let mut out = vec![];
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        match ch {
            ';' => {
                while i < c.len() && c[i] != '\n' {
                    i += 1;
                }
            }
            '{' => {
                out.push(Ficha::Abre);
                i += 1;
            }
            '}' => {
                out.push(Ficha::Fecha);
                i += 1;
            }
            ',' => {
                out.push(Ficha::Virgula);
                i += 1;
            }
            '"' => {
                let ini = i + 1;
                i += 1;
                while i < c.len() && c[i] != '"' {
                    i += 1;
                }
                out.push(Ficha::Texto(c[ini..i.min(c.len())].iter().collect()));
                i += 1;
            }
            '*' => {
                i += 1;
                while i < c.len() && c[i].is_ascii_digit() {
                    i += 1;
                }
                out.push(Ficha::Contagem);
            }
            _ if ch.is_whitespace() => i += 1,
            _ if ch.is_ascii_digit() || ch == '-' || ch == '+' || ch == '.' => {
                let ini = i;
                i += 1;
                while i < c.len() && (c[i].is_ascii_alphanumeric() || c[i] == '.' || c[i] == '-' || c[i] == '+') {
                    i += 1;
                }
                let s: String = c[ini..i].iter().collect();
                let inteiro = !s.contains(['.', 'e', 'E']);
                out.push(Ficha::Numero(s.parse().unwrap_or(0.), inteiro));
            }
            _ => {
                let ini = i;
                while i < c.len() && !c[i].is_whitespace() && !matches!(c[i], ':' | ',' | '{' | '}' | '"' | ';') {
                    i += 1;
                }
                let s: String = c[ini..i].iter().collect();
                if i < c.len() && c[i] == ':' {
                    out.push(Ficha::Chave(s));
                    i += 1;
                } else if s.is_empty() {
                    i += 1;
                } else {
                    out.push(Ficha::Palavra(s));
                }
            }
        }
    }
    out
}

fn nodos_texto(f: &[Ficha], i: &mut usize, nivel: usize) -> Result<Vec<Nodo>, ErroConv> {
    if nivel > PROFUNDIDADE {
        return Err(erro("FBX", "árvore funda demais"));
    }
    let mut out = vec![];
    while *i < f.len() {
        match &f[*i] {
            Ficha::Fecha => {
                *i += 1;
                return Ok(out);
            }
            Ficha::Chave(nome) => {
                *i += 1;
                let mut n = Nodo { nome: nome.clone(), ..Default::default() };
                while *i < f.len() {
                    match &f[*i] {
                        Ficha::Numero(x, inteiro) => n.valores.push(if *inteiro { Valor::Int(*x as i64) } else { Valor::Num(*x) }),
                        Ficha::Texto(t) | Ficha::Palavra(t) => n.valores.push(Valor::Texto(t.clone())),
                        Ficha::Virgula | Ficha::Contagem => {}
                        _ => break,
                    }
                    *i += 1;
                }
                if f.get(*i) == Some(&Ficha::Abre) {
                    *i += 1;
                    n.filhos = nodos_texto(f, i, nivel + 1)?;
                }
                out.push(n);
            }
            _ => *i += 1,
        }
    }
    Ok(out)
}

fn ler_texto(b: &[u8]) -> Result<Nodo, ErroConv> {
    let t = String::from_utf8_lossy(b);
    let f = fichas(&t);
    let mut i = 0;
    Ok(Nodo { filhos: nodos_texto(&f, &mut i, 0)?, ..Default::default() })
}

// ------------------------------------------------------------------ da árvore à cena

/// Nome sem a classe: o binário grava `nome\0\x01Classe`, o texto `Classe::nome`.
fn nome_limpo(v: Option<&Valor>) -> String {
    let t = v.map(Valor::texto).unwrap_or("");
    let t = t.split('\u{0}').next().unwrap_or("");
    t.rsplit("::").next().unwrap_or("").to_string()
}

/// Matriz local de um modelo: T · R · S, com o giro em graus na ordem X, Y, Z.
fn matriz_local(m: &Nodo) -> [f64; 12] {
    let v3 = |nome: &str, padrao: f64| -> [f64; 3] {
        let v = m.propriedade(nome).unwrap_or_default();
        [0, 1, 2].map(|i| v.get(i).copied().unwrap_or(padrao))
    };
    let (t, r, s) = (v3("Lcl Translation", 0.), v3("Lcl Rotation", 0.), v3("Lcl Scaling", 1.));
    let giro = |eixo: usize, graus: f64| -> [f64; 12] {
        let (sn, cs) = graus.to_radians().sin_cos();
        let mut g = IDENTIDADE;
        let (a, b) = ((eixo + 1) % 3, (eixo + 2) % 3);
        g[a * 4 + a] = cs;
        g[a * 4 + b] = -sn;
        g[b * 4 + a] = sn;
        g[b * 4 + b] = cs;
        g
    };
    let rot = compor(&giro(2, r[2]), &compor(&giro(1, r[1]), &giro(0, r[0])));
    let esc = [s[0], 0., 0., 0., 0., s[1], 0., 0., 0., 0., s[2], 0.];
    let mut l = compor(&rot, &esc);
    l[3] = t[0];
    l[7] = t[1];
    l[11] = t[2];
    l
}

fn malha_de(g: &Nodo, nome: String) -> Result<Malha, ErroConv> {
    let v = g.filho("Vertices").map(Nodo::numeros).unwrap_or_default();
    let idx = g.filho("PolygonVertexIndex").map(Nodo::inteiros).unwrap_or_default();
    let pos: Vec<[f64; 3]> = v.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
    let mut tris = vec![];
    let mut poli: Vec<u32> = vec![];
    for i in idx {
        let (k, fecha) = if i < 0 { (-i - 1, true) } else { (i, false) };
        if k < 0 || k as usize >= pos.len() {
            return Err(erro("FBX", format!("índice de vértice {k} fora da malha de {} vértices", pos.len())));
        }
        poli.push(k as u32);
        if fecha {
            for j in 1..poli.len().saturating_sub(1) {
                tris.push([poli[0], poli[j], poli[j + 1]]);
            }
            poli.clear();
        }
    }
    Ok(Malha { nome, pos, tris, ..Default::default() })
}

pub(crate) fn ler(b: &[u8]) -> Result<Cena, ErroConv> {
    let raiz = if e_binario(b) { ler_binario(b)? } else { ler_texto(b)? };
    cena_de(&raiz)
}

fn cena_de(raiz: &Nodo) -> Result<Cena, ErroConv> {
    let objetos = raiz.filho("Objects").ok_or_else(|| erro("FBX", "sem a seção Objects"))?;
    let mut cena = Cena::default();
    let id_de = |n: &Nodo| n.valores.first().and_then(Valor::int).unwrap_or(0);
    // texto antigo (6.x) não tem id numérico: o nome serve de id
    let chave_de = |n: &Nodo| match n.valores.first() {
        Some(Valor::Texto(t)) => t.clone(),
        _ => id_de(n).to_string(),
    };
    let mut geometrias: HashMap<String, usize> = HashMap::new();
    let mut modelos: HashMap<String, usize> = HashMap::new();
    let mut materiais: HashMap<String, usize> = HashMap::new();
    let mut pivos = 0;
    for o in &objetos.filhos {
        match o.nome.as_str() {
            "Geometry" if o.valores.get(2).map(Valor::texto).unwrap_or("Mesh") == "Mesh" => {
                let m = malha_de(o, nome_limpo(o.valores.get(1)))?;
                cena.malhas.push(m);
                geometrias.insert(chave_de(o), cena.malhas.len() - 1);
            }
            "Geometry" => cena.ignorar("primitiva sem malha", 1),
            "Model" => {
                let tipo = o.valores.get(2).map(Valor::texto).unwrap_or("");
                match tipo {
                    "Camera" => cena.ignorar("câmera", 1),
                    "Light" => cena.ignorar("luz", 1),
                    "LimbNode" | "Root" => cena.ignorar("esqueleto", 1),
                    _ => {}
                }
                for p in ["PreRotation", "PostRotation", "RotationPivot", "ScalingPivot", "RotationOffset", "ScalingOffset"] {
                    if o.propriedade(p).is_some_and(|v| v.iter().any(|x| x.abs() > 1e-9)) {
                        pivos += 1;
                    }
                }
                let mut no = No::novo(&nome_limpo(o.valores.get(1)));
                no.local = matriz_local(o);
                cena.nos.push(no);
                modelos.insert(chave_de(o), cena.nos.len() - 1);
                // texto antigo: a malha mora dentro do próprio modelo
                if o.filho("Vertices").is_some() {
                    let m = malha_de(o, nome_limpo(o.valores.get(1)))?;
                    cena.malhas.push(m);
                    let i = cena.nos.len() - 1;
                    cena.nos[i].malhas.push(cena.malhas.len() - 1);
                }
            }
            "Material" => {
                let d = o.propriedade("DiffuseColor").or_else(|| o.propriedade("Diffuse")).unwrap_or_else(|| vec![0.8, 0.8, 0.8]);
                let a = o.propriedade("Opacity").and_then(|v| v.first().copied()).unwrap_or(1.);
                let cor = [d.first().copied().unwrap_or(0.8) as f32, d.get(1).copied().unwrap_or(0.8) as f32, d.get(2).copied().unwrap_or(0.8) as f32, a as f32];
                cena.materiais.push(super::Material::de_cor(&nome_limpo(o.valores.get(1)), cor));
                materiais.insert(chave_de(o), cena.materiais.len() - 1);
            }
            "Texture" | "Video" => cena.ignorar("textura", 1),
            "AnimationStack" => cena.ignorar("animação", 1),
            "Deformer" => cena.ignorar("esqueleto", 1),
            _ => {}
        }
    }
    cena.ignorar("elemento extra", pivos);
    let mut tem_pai = vec![false; cena.nos.len()];
    let mut mat_do_no: Vec<(usize, usize)> = vec![];
    let mut na_raiz: Vec<usize> = vec![];
    if let Some(c) = raiz.filho("Connections") {
        for l in c.todos("C").chain(c.todos("Connect")) {
            let chave = |v: Option<&Valor>| match v {
                Some(Valor::Texto(t)) => t.clone(),
                Some(x) => x.int().unwrap_or(0).to_string(),
                None => String::new(),
            };
            let (filho, pai) = (chave(l.valores.get(1)), chave(l.valores.get(2)));
            // ligado ao zero: raiz da cena
            if pai == "0" || pai.is_empty() {
                if let Some(&m) = modelos.get(&filho) {
                    na_raiz.push(m);
                }
                continue;
            }
            let Some(&p) = modelos.get(&pai) else { continue };
            if let Some(&g) = geometrias.get(&filho) {
                cena.nos[p].malhas.push(g);
            } else if let Some(&m) = modelos.get(&filho) {
                // o mesmo modelo sob dois pais é instância (a cena é um grafo; ciclo para no limite de profundidade)
                if m != p && !cena.nos[p].filhos.contains(&m) {
                    cena.nos[p].filhos.push(m);
                    tem_pai[m] = true;
                }
            } else if let Some(&mat) = materiais.get(&filho) {
                // a ligação do material pode vir antes da geometria: aplica no fim
                mat_do_no.push((p, mat));
            }
        }
    }
    for (p, mat) in mat_do_no {
        for &g in &cena.nos[p].malhas {
            cena.malhas[g].material.get_or_insert(mat);
        }
    }
    // as raízes são os modelos ligados ao zero; arquivo sem essa ligação usa os modelos sem pai
    cena.raizes = if na_raiz.is_empty() { (0..cena.nos.len()).filter(|i| !tem_pai[*i]).collect() } else { na_raiz };
    // geometria que nenhum modelo usa ainda é peça: vai solta para a raiz
    let usadas: Vec<usize> = cena.nos.iter().flat_map(|n| n.malhas.clone()).collect();
    for g in 0..cena.malhas.len() {
        if !usadas.contains(&g) {
            let mut no = No::novo(&cena.malhas[g].nome);
            no.malhas.push(g);
            cena.nos.push(no);
            cena.raizes.push(cena.nos.len() - 1);
        }
    }
    // unidade: centímetros por unidade do arquivo (padrão 1 = centímetro); eixo de cima padrão Y
    let global = raiz.filho("GlobalSettings");
    let cm = global.and_then(|g| g.propriedade("UnitScaleFactor")).and_then(|v| v.first().copied()).filter(|x| *x > 0.).unwrap_or(1.);
    let cima = global.and_then(|g| g.propriedade("UpAxis")).and_then(|v| v.first().copied()).unwrap_or(1.) as i64;
    cena.unidade = match cm {
        x if (x - 0.1).abs() < 1e-9 => "millimeter".into(),
        x if (x - 1.).abs() < 1e-9 => "centimeter".into(),
        x if (x - 100.).abs() < 1e-9 => "meter".into(),
        x if (x - 2.54).abs() < 1e-9 => "inch".into(),
        _ => String::new(),
    };
    match cima {
        2 => {}
        0 => cena.embrulhar([0., -1., 0., 0., 0., 0., -1., 0., 1., 0., 0., 0.]),
        _ => cena.embrulhar([1., 0., 0., 0., 0., 0., -1., 0., 0., 1., 0., 0.]),
    }
    cena.escalar(cm * 10.);
    Ok(cena)
}

/// Grava uma cena em FBX binário 7.4 mínimo. Não é exportação (o Fenkra não oferece gravar FBX):
/// serve de arquivo de prova para os testes e para a sonda da matriz de formatos conferir a leitura.
pub(crate) fn amostra_binaria(cena: &Cena, comprimir: bool) -> Vec<u8> {
    fn valor(out: &mut Vec<u8>, v: &Valor, comprimir: bool) {
        match v {
            Valor::Int(i) => {
                out.push(b'L');
                out.extend_from_slice(&i.to_le_bytes());
            }
            Valor::Num(x) => {
                out.push(b'D');
                out.extend_from_slice(&x.to_le_bytes());
            }
            Valor::Texto(t) => {
                out.push(b'S');
                out.extend_from_slice(&(t.len() as u32).to_le_bytes());
                out.extend_from_slice(t.as_bytes());
            }
            Valor::Nums(n) => {
                out.push(b'd');
                let cru: Vec<u8> = n.iter().flat_map(|x| x.to_le_bytes()).collect();
                vetor(out, n.len(), cru, comprimir);
            }
            Valor::Ints(n) => {
                out.push(b'i');
                let cru: Vec<u8> = n.iter().flat_map(|x| (*x as i32).to_le_bytes()).collect();
                vetor(out, n.len(), cru, comprimir);
            }
            Valor::Bruto => {}
        }
    }
    fn vetor(out: &mut Vec<u8>, n: usize, cru: Vec<u8>, comprimir: bool) {
        out.extend_from_slice(&(n as u32).to_le_bytes());
        let dados = if comprimir { miniz_oxide::deflate::compress_to_vec_zlib(&cru, 6) } else { cru };
        out.extend_from_slice(&(comprimir as u32).to_le_bytes());
        out.extend_from_slice(&(dados.len() as u32).to_le_bytes());
        out.extend_from_slice(&dados);
    }
    fn nodo(out: &mut Vec<u8>, n: &Nodo, comprimir: bool) {
        let ini = out.len();
        out.extend_from_slice(&[0u8; 12]);
        out.push(n.nome.len() as u8);
        out.extend_from_slice(n.nome.as_bytes());
        let ini_val = out.len();
        for v in &n.valores {
            valor(out, v, comprimir);
        }
        let tam_val = out.len() - ini_val;
        if !n.filhos.is_empty() {
            for f in &n.filhos {
                nodo(out, f, comprimir);
            }
            out.extend_from_slice(&[0u8; 13]);
        }
        let fim = out.len() as u32;
        out[ini..ini + 4].copy_from_slice(&fim.to_le_bytes());
        out[ini + 4..ini + 8].copy_from_slice(&(n.valores.len() as u32).to_le_bytes());
        out[ini + 8..ini + 12].copy_from_slice(&(tam_val as u32).to_le_bytes());
    }
    let t = |s: &str| Valor::Texto(s.into());
    let p = |nome: &str, v: [f64; 3]| Nodo { nome: "P".into(), valores: vec![t(nome), t("Lcl"), t(""), t("A"), Valor::Num(v[0]), Valor::Num(v[1]), Valor::Num(v[2])], filhos: vec![] };
    let mut objetos = Nodo { nome: "Objects".into(), ..Default::default() };
    let mut ligacoes = Nodo { nome: "Connections".into(), ..Default::default() };
    let liga = |a: i64, b: i64| Nodo { nome: "C".into(), valores: vec![t("OO"), Valor::Int(a), Valor::Int(b)], filhos: vec![] };
    for (i, m) in cena.malhas.iter().enumerate() {
        let id = 1000 + i as i64;
        let mut idx = vec![];
        for tr in &m.tris {
            idx.extend([tr[0] as i64, tr[1] as i64, -(tr[2] as i64) - 1]);
        }
        objetos.filhos.push(Nodo {
            nome: "Geometry".into(),
            valores: vec![Valor::Int(id), t(&format!("{}\u{0}\u{1}Geometry", m.nome)), t("Mesh")],
            filhos: vec![
                Nodo { nome: "Vertices".into(), valores: vec![Valor::Nums(m.pos.iter().flatten().copied().collect())], filhos: vec![] },
                Nodo { nome: "PolygonVertexIndex".into(), valores: vec![Valor::Ints(idx)], filhos: vec![] },
            ],
        });
    }
    for (i, n) in cena.nos.iter().enumerate() {
        let id = 2000 + i as i64;
        objetos.filhos.push(Nodo {
            nome: "Model".into(),
            valores: vec![Valor::Int(id), t(&format!("{}\u{0}\u{1}Model", n.nome)), t("Mesh")],
            filhos: vec![Nodo { nome: "Properties70".into(), valores: vec![], filhos: vec![p("Lcl Translation", [n.local[3], n.local[7], n.local[11]])] }],
        });
        for &g in &n.malhas {
            ligacoes.filhos.push(liga(1000 + g as i64, id));
        }
        for &f in &n.filhos {
            ligacoes.filhos.push(liga(2000 + f as i64, id));
        }
        if cena.raizes.contains(&i) {
            ligacoes.filhos.push(liga(id, 0));
        }
    }
    for (i, m) in cena.malhas.iter().enumerate() {
        if let Some(mat) = m.material {
            let id = 3000 + mat as i64;
            let c = cena.materiais[mat].cor;
            if !objetos.filhos.iter().any(|o| o.valores.first() == Some(&Valor::Int(id))) {
                objetos.filhos.push(Nodo {
                    nome: "Material".into(),
                    valores: vec![Valor::Int(id), t("aco\u{0}\u{1}Material"), t("")],
                    filhos: vec![Nodo { nome: "Properties70".into(), valores: vec![], filhos: vec![p("DiffuseColor", [c[0] as f64, c[1] as f64, c[2] as f64])] }],
                });
            }
            let dono = cena.nos.iter().position(|n| n.malhas.contains(&i)).unwrap_or(0);
            ligacoes.filhos.push(liga(id, 2000 + dono as i64));
        }
    }
    // unidade em milímetros (0,1 cm por unidade) e Z para cima, como a cena
    let global = Nodo {
        nome: "GlobalSettings".into(),
        valores: vec![],
        filhos: vec![Nodo {
            nome: "Properties70".into(),
            valores: vec![],
            filhos: vec![
                Nodo { nome: "P".into(), valores: vec![t("UpAxis"), t("int"), t("Integer"), t(""), Valor::Int(2)], filhos: vec![] },
                Nodo { nome: "P".into(), valores: vec![t("UnitScaleFactor"), t("double"), t("Number"), t(""), Valor::Num(0.1)], filhos: vec![] },
            ],
        }],
    };
    let mut out = ASSINATURA.to_vec();
    out.extend_from_slice(&[0x1a, 0x00]);
    out.extend_from_slice(&7400u32.to_le_bytes());
    for n in [&global, &objetos, &ligacoes] {
        nodo(&mut out, n, comprimir);
    }
    out.extend_from_slice(&[0u8; 13]);
    out
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn texto_7_com_quadrado_vira_dois_triangulos_em_milimetros_e_z_para_cima() {
        let t = r#"; FBX 7.4.0 project file
FBXHeaderExtension:  {
    FBXVersion: 7400
}
GlobalSettings:  {
    Properties70:  {
        P: "UpAxis", "int", "Integer", "",1
        P: "UnitScaleFactor", "double", "Number", "",1
    }
}
Objects:  {
    Geometry: 10, "Geometry::Quad", "Mesh" {
        Vertices: *12 {
            a: 0,0,0,1,0,0,1,1,0,0,1,0
        }
        PolygonVertexIndex: *4 {
            a: 0,1,2,-4
        }
    }
    Model: 20, "Model::Placa", "Mesh" {
        Properties70:  {
            P: "Lcl Translation", "Lcl Translation", "", "A",0,2,0
        }
    }
    Material: 30, "Material::Vermelho", "" {
        Properties70:  {
            P: "DiffuseColor", "Color", "", "A",1,0,0
        }
    }
    AnimationStack: 40, "AnimStack::Take", "" {
    }
}
Connections:  {
    C: "OO",10,20
    C: "OO",30,20
    C: "OO",20,0
}
"#;
        assert!(e_texto(t.as_bytes()) && !e_binario(t.as_bytes()));
        let mut c = ler(t.as_bytes()).unwrap();
        assert_eq!(c.unidade, "centimeter");
        assert_eq!(c.ignorados.get("animação"), Some(&1));
        c.enxugar();
        let m = &c.no_lugar()[0];
        assert_eq!(m.tris.len(), 2);
        // Y do arquivo vira Z; 1 cm = 10 mm; o modelo sobe 2 cm
        let perto = |a: [f64; 3], b: [f64; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9);
        assert!(perto(m.pos[2], [10., 0., 30.]), "{:?}", m.pos);
        assert_eq!(c.materiais[m.material.unwrap()].cor, [1., 0., 0., 1.]);
        assert_eq!(c.nos.iter().find(|n| n.nome == "Placa").map(|n| n.malhas.len()), Some(1));
    }

    #[test]
    fn binario_comprimido_e_cru_dao_a_mesma_cena() {
        let mut cena = Cena::default();
        let mut cubo = fenkra_malha::cena::cubo("Cubo", 10.);
        cubo.material = Some(cena.material("aço", [0.5, 0.6, 0.7, 1.]));
        let mut no = No::novo("Peça");
        no.local[3] = 5.;
        no.malhas.push(0);
        cena.malhas.push(cubo);
        cena.nos.push(no);
        cena.raizes.push(0);
        for comprimir in [false, true] {
            let b = amostra_binaria(&cena, comprimir);
            assert!(e_binario(&b));
            let mut lida = ler(&b).unwrap();
            assert_eq!(lida.unidade, "millimeter");
            lida.enxugar();
            let m = &lida.no_lugar()[0];
            assert_eq!(m.tris.len(), 12);
            assert!((m.volume() - 1000.).abs() < 1e-6, "volume {}", m.volume());
            let x: f64 = m.pos.iter().map(|p| p[0]).fold(f64::MAX, f64::min);
            assert!((x - (5. + cena.malhas[0].pos.iter().map(|p| p[0]).fold(f64::MAX, f64::min))).abs() < 1e-9);
            assert_eq!(lida.materiais[m.material.unwrap()].cor, [0.5, 0.6, 0.7, 1.]);
        }
    }

    #[test]
    fn arquivo_cortado_ou_hostil_e_erro_sem_travar() {
        let mut b = ASSINATURA.to_vec();
        b.extend_from_slice(&[0x1a, 0, 0xE8, 0x1C, 0, 0]);
        b.extend_from_slice(&[0xFF; 20]);
        assert!(ler(&b).is_err());
        assert!(ler(b"; FBX 7.4.0\nObjects: {\n Geometry: 1, \"g\", \"Mesh\" {\n Vertices: *3 { a: 0,0,0 }\n PolygonVertexIndex: *3 { a: 0,1,-3 }\n}\n}\n").is_err(), "índice fora da malha");
    }
}
