// SPDX-License-Identifier: LGPL-2.1-or-later
//! IFC4 facetado. O arquivo é STEP físico, mas com outro esquema, e por isso não passa pelo leitor do
//! kernel: quem resolve IFC por inteiro é uma biblioteca pesada, e aqui só entra o que é malha.
//!
//! A leitura percorre os produtos pela posição dos atributos, que todo `IfcProduct` compartilha: a
//! posição em 6 e a representação em 7. Traz faces trianguladas, poligonais e B-rep facetado, com
//! instâncias por `IfcMappedItem`. Sólido por extrusão e outras formas paramétricas não têm malha
//! pronta: ficam de fora e o relatório conta.
//!
//! A gravação põe cada peça num `IfcBuildingElementProxy`, com a malha uma vez só num
//! `IfcRepresentationMap`, e todas num único pavimento. Não há árvore: montagem sai achatada.

use super::{erro, Cena, Malha, No};
use crate::modelo::{compor, PROFUNDIDADE};
use crate::ErroConv;
use fenkra_kernel::IDENTIDADE;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
enum V {
    Nulo,
    Numero(f64),
    Texto(String),
    Enum(String),
    Ref(u64),
    Lista(Vec<V>),
    /// valor com tipo, como `IFCLABEL('x')`
    Tipado(Vec<V>),
}

impl V {
    fn numero(&self) -> Option<f64> {
        match self {
            V::Numero(n) => Some(*n),
            V::Tipado(l) => l.first()?.numero(),
            _ => None,
        }
    }

    fn lista(&self) -> &[V] {
        match self {
            V::Lista(l) => l,
            _ => &[],
        }
    }

    fn referencia(&self) -> Option<u64> {
        match self {
            V::Ref(r) => Some(*r),
            _ => None,
        }
    }

    fn texto(&self) -> &str {
        match self {
            V::Texto(t) => t,
            _ => "",
        }
    }

    fn numeros(&self) -> Vec<f64> {
        self.lista().iter().filter_map(V::numero).collect()
    }
}

struct Analise<'a> {
    b: &'a [u8],
    i: usize,
}

impl Analise<'_> {
    fn pular_espaco(&mut self) {
        while self.b.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
    }

    fn valor(&mut self, nivel: usize) -> Option<V> {
        if nivel > 32 {
            return None;
        }
        self.pular_espaco();
        match *self.b.get(self.i)? {
            b'$' | b'*' => {
                self.i += 1;
                Some(V::Nulo)
            }
            b'#' => {
                self.i += 1;
                let inicio = self.i;
                while self.b.get(self.i).is_some_and(u8::is_ascii_digit) {
                    self.i += 1;
                }
                std::str::from_utf8(&self.b[inicio..self.i]).ok()?.parse().ok().map(V::Ref)
            }
            b'\'' => {
                self.i += 1;
                let mut t = Vec::new();
                loop {
                    match *self.b.get(self.i)? {
                        b'\'' if self.b.get(self.i + 1) == Some(&b'\'') => {
                            t.push(b'\'');
                            self.i += 2;
                        }
                        b'\'' => {
                            self.i += 1;
                            break;
                        }
                        c => {
                            t.push(c);
                            self.i += 1;
                        }
                    }
                }
                Some(V::Texto(decodificar(&String::from_utf8_lossy(&t))))
            }
            b'.' => {
                self.i += 1;
                let inicio = self.i;
                while self.b.get(self.i).is_some_and(|c| *c != b'.') {
                    self.i += 1;
                }
                let t = String::from_utf8_lossy(&self.b[inicio..self.i]).to_string();
                self.i += 1;
                Some(V::Enum(t))
            }
            b'(' => {
                self.i += 1;
                Some(V::Lista(self.lista(nivel)?))
            }
            c if c == b'-' || c == b'+' || c.is_ascii_digit() => {
                let inicio = self.i;
                while self.b.get(self.i).is_some_and(|c| c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'E' | b'e')) {
                    self.i += 1;
                }
                std::str::from_utf8(&self.b[inicio..self.i]).ok()?.parse().ok().map(V::Numero)
            }
            c if c.is_ascii_alphabetic() => {
                while self.b.get(self.i).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_') {
                    self.i += 1;
                }
                self.pular_espaco();
                if self.b.get(self.i) != Some(&b'(') {
                    return None;
                }
                self.i += 1;
                Some(V::Tipado(self.lista(nivel)?))
            }
            _ => None,
        }
    }

    fn lista(&mut self, nivel: usize) -> Option<Vec<V>> {
        let mut l = Vec::new();
        loop {
            self.pular_espaco();
            match *self.b.get(self.i)? {
                b')' => {
                    self.i += 1;
                    return Some(l);
                }
                b',' => self.i += 1,
                _ => l.push(self.valor(nivel + 1)?),
            }
        }
    }
}

/// Acentos do STEP físico: `\X2\00E7\X0\` é UTF-16 em hexadecimal.
fn decodificar(t: &str) -> String {
    let mut s = String::new();
    let mut resto = t;
    while let Some(i) = resto.find("\\X2\\") {
        s.push_str(&resto[..i]);
        let depois = &resto[i + 4..];
        let Some(f) = depois.find("\\X0\\") else {
            resto = depois;
            continue;
        };
        let bytes = depois[..f].as_bytes();
        let unidades: Vec<u16> = bytes.chunks_exact(4).filter_map(|c| u16::from_str_radix(std::str::from_utf8(c).ok()?, 16).ok()).collect();
        s.push_str(&String::from_utf16_lossy(&unidades));
        resto = &depois[f + 4..];
    }
    s.push_str(resto);
    s
}

fn codificar(t: &str) -> String {
    let mut s = String::new();
    for c in t.chars() {
        match c {
            '\'' => s.push_str("''"),
            '\\' => s.push_str("\\\\"),
            c if c.is_ascii() && !c.is_control() => s.push(c),
            c => {
                s.push_str("\\X2\\");
                let mut u = [0u16; 2];
                for q in c.encode_utf16(&mut u) {
                    let _ = write!(s, "{q:04X}");
                }
                s.push_str("\\X0\\");
            }
        }
    }
    s
}

type Entidades = HashMap<u64, (String, Vec<V>)>;

/// Lê a seção de dados: `#12=IFCTIPO(...);`, em uma ou várias linhas.
fn entidades(b: &[u8]) -> Result<Entidades, ErroConv> {
    let inicio = b.windows(5).position(|w| w == b"DATA;").ok_or_else(|| erro("IFC", "sem seção de dados"))? + 5;
    let mut a = Analise { b, i: inicio };
    let mut mapa = HashMap::new();
    loop {
        a.pular_espaco();
        if a.b.get(a.i) != Some(&b'#') {
            break;
        }
        let Some(V::Ref(id)) = a.valor(0) else { return Err(erro("IFC", "entidade sem número")) };
        a.pular_espaco();
        if a.b.get(a.i) != Some(&b'=') {
            return Err(erro("IFC", "entidade sem igual"));
        }
        a.i += 1;
        a.pular_espaco();
        let inicio = a.i;
        while a.b.get(a.i).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_') {
            a.i += 1;
        }
        let tipo = String::from_utf8_lossy(&a.b[inicio..a.i]).to_uppercase();
        a.pular_espaco();
        if a.b.get(a.i) != Some(&b'(') {
            return Err(erro("IFC", format!("entidade #{id} sem atributos")));
        }
        a.i += 1;
        let args = a.lista(0).ok_or_else(|| erro("IFC", format!("entidade #{id} ilegível")))?;
        a.pular_espaco();
        if a.b.get(a.i) == Some(&b';') {
            a.i += 1;
        }
        mapa.insert(id, (tipo, args));
    }
    Ok(mapa)
}

struct Leitura<'a> {
    e: &'a Entidades,
    c: Cena,
    /// malhas já prontas de cada item de representação
    feitas: HashMap<u64, Vec<usize>>,
    cores: HashMap<u64, usize>,
}

fn normalizado(v: [f64; 3]) -> Option<[f64; 3]> {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    (n > 1e-12).then(|| v.map(|x| x / n))
}

fn vetorial(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

impl Leitura<'_> {
    fn entidade(&self, v: &V) -> Option<&(String, Vec<V>)> {
        self.e.get(&v.referencia()?)
    }

    fn ponto(&self, v: &V) -> Option<[f64; 3]> {
        let (_, a) = self.entidade(v)?;
        let c = a.first()?.numeros();
        Some([*c.first()?, *c.get(1)?, c.get(2).copied().unwrap_or(0.)])
    }

    fn direcao(&self, v: &V) -> Option<[f64; 3]> {
        self.ponto(v)
    }

    /// Matriz de um `IfcAxis2Placement3D`: origem, eixo Z e direção de referência do X.
    fn eixos(&self, v: &V) -> [f64; 12] {
        let Some((_, a)) = self.entidade(v) else { return IDENTIDADE };
        let o = a.first().and_then(|p| self.ponto(p)).unwrap_or([0.; 3]);
        let z = a.get(1).and_then(|p| self.direcao(p)).and_then(normalizado).unwrap_or([0., 0., 1.]);
        let x = a.get(2).and_then(|p| self.direcao(p)).unwrap_or([1., 0., 0.]);
        let y = normalizado(vetorial(z, x)).unwrap_or([0., 1., 0.]);
        let x = vetorial(y, z);
        [x[0], y[0], z[0], o[0], x[1], y[1], z[1], o[1], x[2], y[2], z[2], o[2]]
    }

    /// Posição absoluta de um `IfcLocalPlacement`, somando a cadeia até a raiz.
    fn lugar(&self, v: &V, nivel: usize) -> [f64; 12] {
        let Some((_, a)) = self.entidade(v) else { return IDENTIDADE };
        if nivel > PROFUNDIDADE {
            return IDENTIDADE;
        }
        let pai = a.first().filter(|p| **p != V::Nulo).map(|p| self.lugar(p, nivel + 1)).unwrap_or(IDENTIDADE);
        let proprio = a.get(1).map(|p| self.eixos(p)).unwrap_or(IDENTIDADE);
        compor(&pai, &proprio)
    }

    fn lista_de_pontos(&self, v: &V) -> Vec<[f64; 3]> {
        let Some((_, a)) = self.entidade(v) else { return vec![] };
        a.first().map(|l| l.lista().iter().map(|p| p.numeros()).filter(|c| c.len() >= 2).map(|c| [c[0], c[1], c.get(2).copied().unwrap_or(0.)]).collect()).unwrap_or_default()
    }

    fn faces_trianguladas(&self, a: &[V]) -> Option<Malha> {
        let pos = self.lista_de_pontos(a.first()?);
        let indices = a.get(3)?.lista();
        let tris = indices.iter().map(V::numeros).filter(|t| t.len() == 3).map(|t| [t[0], t[1], t[2]].map(|i| (i as u32).wrapping_sub(1))).collect();
        Some(Malha { pos, tris, ..Default::default() })
    }

    fn faces_poligonais(&self, a: &[V]) -> Option<Malha> {
        let pos = self.lista_de_pontos(a.first()?);
        let mut tris = Vec::new();
        for f in a.get(2)?.lista() {
            let (_, q) = self.entidade(f)?;
            let v: Vec<u32> = q.first()?.numeros().iter().map(|i| *i as u32).filter(|i| *i > 0).map(|i| i - 1).collect();
            for k in 1..v.len().saturating_sub(1) {
                tris.push([v[0], v[k], v[k + 1]]);
            }
        }
        Some(Malha { pos, tris, ..Default::default() })
    }

    /// `IfcFacetedBrep` e cascas: face, contorno externo, laço de pontos. Furo em face fica de fora.
    fn casca(&mut self, casca: &V) -> Option<Malha> {
        let faces = self.entidade(casca)?.1.first()?.lista().to_vec();
        let mut m = Malha::default();
        let mut furos = 0;
        for f in &faces {
            let contornos = self.entidade(f)?.1.first()?.lista().to_vec();
            furos += contornos.len().saturating_sub(1);
            let contorno = contornos.iter().find(|b| self.entidade(b).is_some_and(|e| e.0 == "IFCFACEOUTERBOUND")).or(contornos.first())?;
            let (_, b) = self.entidade(contorno)?;
            let laco = self.entidade(b.first()?)?;
            let sentido = !matches!(b.get(1), Some(V::Enum(t)) if t == "F");
            let mut pontos: Vec<[f64; 3]> = laco.1.first()?.lista().iter().filter_map(|p| self.ponto(p)).collect();
            if !sentido {
                pontos.reverse();
            }
            let base = m.pos.len() as u32;
            for k in 1..pontos.len().saturating_sub(1) as u32 {
                m.tris.push([base, base + k, base + k + 1]);
            }
            m.pos.extend(pontos);
        }
        self.c.ignorar("furo em face", furos);
        Some(m)
    }

    fn cor_do_item(&mut self, item: u64) -> Option<usize> {
        self.cores.get(&item).copied()
    }

    fn malhas_do_item(&mut self, v: &V, nome: &str, nivel: usize) -> Result<Vec<(usize, [f64; 12])>, ErroConv> {
        let Some(id) = v.referencia() else { return Ok(vec![]) };
        let Some((tipo, a)) = self.e.get(&id).cloned() else { return Ok(vec![]) };
        if nivel > PROFUNDIDADE {
            return Err(erro("IFC", "representação funda demais"));
        }
        if tipo == "IFCMAPPEDITEM" {
            let Some((_, mapa)) = a.first().and_then(|m| self.entidade(m)).cloned() else { return Ok(vec![]) };
            let origem = mapa.first().map(|p| self.eixos(p)).unwrap_or(IDENTIDADE);
            let alvo = a.get(1).map(|t| self.operador(t)).unwrap_or(IDENTIDADE);
            let lugar = compor(&alvo, &origem);
            let Some((_, r)) = mapa.get(1).and_then(|m| self.entidade(m)).cloned() else { return Ok(vec![]) };
            let mut saida = Vec::new();
            for item in r.get(3).map(V::lista).unwrap_or_default() {
                for (m, l) in self.malhas_do_item(item, nome, nivel + 1)? {
                    saida.push((m, compor(&lugar, &l)));
                }
            }
            return Ok(saida);
        }
        if let Some(l) = self.feitas.get(&id) {
            return Ok(l.iter().map(|m| (*m, IDENTIDADE)).collect());
        }
        let malha = match tipo.as_str() {
            "IFCTRIANGULATEDFACESET" => self.faces_trianguladas(&a),
            "IFCPOLYGONALFACESET" => self.faces_poligonais(&a),
            "IFCFACETEDBREP" | "IFCFACETEDBREPWITHVOIDS" => a.first().cloned().and_then(|c| self.casca(&c)),
            "IFCSHELLBASEDSURFACEMODEL" | "IFCFACEBASEDSURFACEMODEL" => {
                let mut junta = Malha::default();
                for s in a.first().map(V::lista).unwrap_or_default().to_vec() {
                    if let Some(m) = self.casca(&s) {
                        let base = junta.pos.len() as u32;
                        junta.pos.extend(m.pos);
                        junta.tris.extend(m.tris.iter().map(|t| t.map(|v| v + base)));
                    }
                }
                Some(junta)
            }
            _ => {
                self.c.ignorar("sólido paramétrico sem malha pronta", 1);
                None
            }
        };
        let Some(mut m) = malha else { return Ok(vec![]) };
        m.nome = nome.to_string();
        m.material = self.cor_do_item(id);
        m.conferir("IFC")?;
        self.c.malhas.push(m);
        let i = self.c.malhas.len() - 1;
        self.feitas.insert(id, vec![i]);
        Ok(vec![(i, IDENTIDADE)])
    }

    /// `IfcCartesianTransformationOperator3D`: eixo 1, eixo 2, origem, escala, eixo 3.
    fn operador(&self, v: &V) -> [f64; 12] {
        let Some((_, a)) = self.entidade(v) else { return IDENTIDADE };
        let d = |i: usize| a.get(i).and_then(|p| self.direcao(p));
        let o = a.get(2).and_then(|p| self.ponto(p)).unwrap_or([0.; 3]);
        let k = a.get(3).and_then(V::numero).unwrap_or(1.);
        let (x, y, z) = (d(0).unwrap_or([1., 0., 0.]), d(1).unwrap_or([0., 1., 0.]), d(4).unwrap_or([0., 0., 1.]));
        [x[0] * k, y[0] * k, z[0] * k, o[0], x[1] * k, y[1] * k, z[1] * k, o[1], x[2] * k, y[2] * k, z[2] * k, o[2]]
    }

    /// Cor de cada item, por `IfcStyledItem` até `IfcColourRgb`.
    fn ler_cores(&mut self) {
        let estilos: Vec<(u64, V)> = self.e.values().filter(|(t, _)| t == "IFCSTYLEDITEM").filter_map(|(_, a)| Some((a.first()?.referencia()?, a.get(1)?.clone()))).collect();
        for (item, lista) in estilos {
            let mut pilha: Vec<V> = lista.lista().to_vec();
            let mut vistos = HashSet::new();
            while let Some(v) = pilha.pop() {
                let Some(r) = v.referencia() else { continue };
                if !vistos.insert(r) {
                    continue;
                }
                let Some((tipo, a)) = self.e.get(&r) else { continue };
                if tipo == "IFCCOLOURRGB" {
                    let c: Vec<f32> = a.iter().filter_map(V::numero).map(|x| x as f32).collect();
                    if let [r, g, b] = c[..] {
                        let transparencia = 0.;
                        let k = self.c.material("Cor", [r, g, b, 1. - transparencia]);
                        self.cores.insert(item, k);
                    }
                    break;
                }
                for x in a {
                    match x {
                        V::Ref(_) => pilha.push(x.clone()),
                        V::Lista(l) => pilha.extend(l.iter().cloned()),
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Metros por unidade de comprimento do projeto.
fn metros_da_unidade(e: &Entidades) -> f64 {
    let convertida = e.values().find(|(t, a)| t == "IFCCONVERSIONBASEDUNIT" && matches!(a.get(1), Some(V::Enum(u)) if u == "LENGTHUNIT"));
    let medida = convertida.and_then(|(_, a)| e.get(&a.get(3)?.referencia()?)).and_then(|(_, a)| a.first()?.numero());
    if let Some(m) = medida {
        return m;
    }
    let si = e.values().find(|(t, a)| t == "IFCSIUNIT" && matches!(a.get(1), Some(V::Enum(u)) if u == "LENGTHUNIT"));
    match si.and_then(|(_, a)| a.get(2)) {
        Some(V::Enum(p)) if p == "MILLI" => 0.001,
        Some(V::Enum(p)) if p == "CENTI" => 0.01,
        Some(V::Enum(p)) if p == "DECI" => 0.1,
        _ => 1.,
    }
}

pub(crate) fn ler(b: &[u8]) -> Result<Cena, ErroConv> {
    let e = entidades(b)?;
    let mut l = Leitura { e: &e, c: Cena::default(), feitas: HashMap::new(), cores: HashMap::new() };
    l.ler_cores();
    let mut ids: Vec<u64> = e.keys().copied().collect();
    ids.sort_unstable();
    let mut por_lista: HashMap<(usize, [u64; 12]), usize> = HashMap::new();
    for id in ids {
        let (_, a) = &e[&id];
        let (Some(lugar), Some(forma)) = (a.get(5), a.get(6)) else { continue };
        let (Some((t1, _)), Some((t2, f))) = (l.entidade(lugar), l.entidade(forma)) else { continue };
        if t1 != "IFCLOCALPLACEMENT" || t2 != "IFCPRODUCTDEFINITIONSHAPE" {
            continue;
        }
        let nome = a.get(2).map(V::texto).unwrap_or("").to_string();
        let posicao = l.lugar(lugar, 0);
        let representacoes = f.get(2).map(V::lista).unwrap_or_default().to_vec();
        for r in representacoes {
            let Some((_, q)) = l.entidade(&r).cloned() else { continue };
            // só o corpo: eixos e contornos de planta são outras representações do mesmo produto
            if q.get(1).map(V::texto).is_some_and(|t| !t.is_empty() && t != "Body") {
                continue;
            }
            for item in q.get(3).map(V::lista).unwrap_or_default() {
                for (m, local) in l.malhas_do_item(item, &nome, 0)? {
                    let matriz = compor(&posicao, &local);
                    let chave = (m, matriz.map(f64::to_bits));
                    if por_lista.contains_key(&chave) {
                        continue;
                    }
                    let mut no = No::novo(&nome);
                    no.local = matriz;
                    no.malhas = vec![m];
                    l.c.nos.push(no);
                    let i = l.c.nos.len() - 1;
                    l.c.raizes.push(i);
                    por_lista.insert(chave, i);
                }
            }
        }
    }
    let mut c = l.c;
    let metros = metros_da_unidade(&e);
    c.unidade = match metros {
        m if m == 0.001 => "MILLIMETRE",
        m if m == 0.0254 => "INCH",
        _ => "METRE",
    }
    .into();
    c.escalar(metros * 1000.);
    Ok(c)
}

/// Identificador global do IFC: 22 letras de um alfabeto de 64, aqui saídas de um contador. A norma só
/// exige que não se repita no arquivo.
fn guid(n: usize, semente: u64) -> String {
    const ALFABETO: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    let mut v = (n as u128) << 64 | u128::from(semente);
    let mut s = String::with_capacity(22);
    s.push('0');
    for _ in 0..21 {
        s.push(ALFABETO[(v & 63) as usize] as char);
        v = (v >> 6) ^ (v << 5 & 0xffff);
    }
    s
}

struct Saida {
    s: String,
    n: u64,
}

impl Saida {
    fn linha(&mut self, corpo: String) -> u64 {
        self.n += 1;
        let _ = writeln!(self.s, "#{}={corpo};", self.n);
        self.n
    }

    fn eixos(&mut self, m: &[f64; 12], mm: f64) -> u64 {
        let o = self.linha(format!("IFCCARTESIANPOINT(({},{},{}))", real(m[3] / mm), real(m[7] / mm), real(m[11] / mm)));
        let z = self.linha(format!("IFCDIRECTION(({},{},{}))", real(m[2]), real(m[6]), real(m[10])));
        let x = self.linha(format!("IFCDIRECTION(({},{},{}))", real(m[0]), real(m[4]), real(m[8])));
        self.linha(format!("IFCAXIS2PLACEMENT3D(#{o},#{z},#{x})"))
    }
}

/// Real do STEP físico: precisa de ponto, mesmo sem casas.
fn real(v: f64) -> String {
    let t = format!("{v}");
    if t.contains('.') {
        t
    } else {
        format!("{t}.")
    }
}

pub(crate) fn gravar(cena: &Cena, mm: f64, caminho: &Path) -> String {
    let mut c = cena.clone();
    c.assentar();
    let nome = caminho.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let semente = u64::from_le_bytes(blake3::hash(c.nome.as_bytes()).as_bytes()[..8].try_into().unwrap_or([0; 8]));
    let mut g = 0;
    let mut id = || {
        g += 1;
        guid(g, semente)
    };
    let mut o = Saida { s: String::new(), n: 0 };
    let mut mapas: HashMap<usize, u64> = HashMap::new();
    let (unidade, prefixo) = match mm {
        x if x == 1. => ("METRE", ".MILLI."),
        x if x == 1000. => ("METRE", "$"),
        _ => ("INCH", ""),
    };
    let comprimento = if unidade == "METRE" {
        o.linha(format!("IFCSIUNIT(*,.LENGTHUNIT.,{prefixo},.METRE.)"))
    } else {
        let base = o.linha("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)".into());
        let medida = o.linha(format!("IFCMEASUREWITHUNIT(IFCLENGTHMEASURE({}),#{base})", real(0.0254)));
        let dim = o.linha("IFCDIMENSIONALEXPONENTS(1,0,0,0,0,0,0)".into());
        o.linha(format!("IFCCONVERSIONBASEDUNIT(#{dim},.LENGTHUNIT.,'INCH',#{medida})"))
    };
    let unidades = o.linha(format!("IFCUNITASSIGNMENT((#{comprimento}))"));
    let origem = o.eixos(&IDENTIDADE, 1.);
    let contexto = o.linha(format!("IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,#{origem},$)"));
    let projeto = o.linha(format!("IFCPROJECT('{}',$,'{}',$,$,$,$,(#{contexto}),#{unidades})", id(), codificar(&c.nome)));
    let lugar_site = o.linha(format!("IFCLOCALPLACEMENT($,#{origem})"));
    let site = o.linha(format!("IFCSITE('{}',$,'Terreno',$,$,#{lugar_site},$,$,.ELEMENT.,$,$,$,$,$)", id()));
    let lugar_predio = o.linha(format!("IFCLOCALPLACEMENT(#{lugar_site},#{origem})"));
    let predio = o.linha(format!("IFCBUILDING('{}',$,'Edifício',$,$,#{lugar_predio},$,$,.ELEMENT.,$,$,$)", id()).replace("Edifício", &codificar("Edifício")));
    let lugar_andar = o.linha(format!("IFCLOCALPLACEMENT(#{lugar_predio},#{origem})"));
    let andar = o.linha(format!("IFCBUILDINGSTOREY('{}',$,'Pavimento',$,$,#{lugar_andar},$,$,.ELEMENT.,$)", id()));
    o.linha(format!("IFCRELAGGREGATES('{}',$,$,$,#{projeto},(#{site}))", id()));
    o.linha(format!("IFCRELAGGREGATES('{}',$,$,$,#{site},(#{predio}))", id()));
    o.linha(format!("IFCRELAGGREGATES('{}',$,$,$,#{predio},(#{andar}))", id()));

    let mut produtos = Vec::new();
    let ocorrencias = c.ocorrencias().0;
    for (caminho_nos, i, local) in ocorrencias {
        let Some(m) = c.malhas.get(i) else { continue };
        let mapa = match mapas.get(&i) {
            Some(q) => *q,
            None => {
                let pontos: Vec<String> = m.pos.iter().map(|p| format!("({},{},{})", real(p[0] / mm), real(p[1] / mm), real(p[2] / mm))).collect();
                let lista = o.linha(format!("IFCCARTESIANPOINTLIST3D(({}),$)", pontos.join(",")));
                let faces: Vec<String> = m.tris.iter().map(|t| format!("({},{},{})", t[0] + 1, t[1] + 1, t[2] + 1)).collect();
                let conjunto = o.linha(format!("IFCTRIANGULATEDFACESET(#{lista},$,.T.,({}),$)", faces.join(",")));
                if let Some(k) = c.cor_de(m) {
                    let cor = o.linha(format!("IFCCOLOURRGB($,{},{},{})", real(f64::from(k[0])), real(f64::from(k[1])), real(f64::from(k[2]))));
                    let render = o.linha(format!("IFCSURFACESTYLESHADING(#{cor},{})", real(f64::from(1. - k[3]))));
                    let estilo = o.linha(format!("IFCSURFACESTYLE($,.BOTH.,(#{render}))"));
                    o.linha(format!("IFCSTYLEDITEM(#{conjunto},(#{estilo}),$)"));
                }
                let forma = o.linha(format!("IFCSHAPEREPRESENTATION(#{contexto},'Body','Tessellation',(#{conjunto}))"));
                let q = o.linha(format!("IFCREPRESENTATIONMAP(#{origem},#{forma})"));
                mapas.insert(i, q);
                q
            }
        };
        let operador_o = o.linha("IFCCARTESIANPOINT((0.,0.,0.))".into());
        let operador = o.linha(format!("IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#{operador_o},1.,$)"));
        let item = o.linha(format!("IFCMAPPEDITEM(#{mapa},#{operador})"));
        let forma = o.linha(format!("IFCSHAPEREPRESENTATION(#{contexto},'Body','MappedRepresentation',(#{item}))"));
        let definicao = o.linha(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{forma}))"));
        let eixos = o.eixos(&local, mm);
        let lugar = o.linha(format!("IFCLOCALPLACEMENT(#{lugar_andar},#{eixos})"));
        let nome_peca = caminho_nos.iter().rev().find(|n| !n.is_empty()).cloned().unwrap_or_else(|| m.nome.clone());
        produtos.push(o.linha(format!("IFCBUILDINGELEMENTPROXY('{}',$,'{}',$,$,#{lugar},#{definicao},$,$)", id(), codificar(&nome_peca))));
    }
    let lista: Vec<String> = produtos.iter().map(|p| format!("#{p}")).collect();
    o.linha(format!("IFCRELCONTAINEDINSPATIALSTRUCTURE('{}',$,$,$,({}),#{andar})", id(), lista.join(",")));
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [ReferenceView_V1.2]'),'2;1');\nFILE_NAME('{}','',(''),(''),'Fenkra','Fenkra','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n{}ENDSEC;\nEND-ISO-10303-21;\n",
        codificar(&nome),
        o.s
    )
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_malha::cena::cubo;

    #[test]
    fn acento_vai_e_volta_pelo_formato_do_step() {
        assert_eq!(codificar("Peça d'água"), "Pe\\X2\\00E7\\X0\\a d''\\X2\\00E1\\X0\\gua");
        assert_eq!(decodificar("Pe\\X2\\00E7\\X0\\a"), "Peça");
    }

    #[test]
    fn valores_do_step_fisico() {
        let t = b"DATA;\n#1=IFCX('a''b',$,*,.T.,(1,2.5,-3.E2),#7,IFCLABEL('x'));\n#2 = IFCY ( ( (1,2),(3,4) ) ) ;\nENDSEC;";
        let e = entidades(t).unwrap();
        let a = &e[&1].1;
        assert_eq!(a[0], V::Texto("a'b".into()));
        assert_eq!(a[3], V::Enum("T".into()));
        assert_eq!(a[4].numeros(), vec![1., 2.5, -300.]);
        assert_eq!(a[5], V::Ref(7));
        assert_eq!(e[&2].1[0].lista().len(), 2);
    }

    #[test]
    fn identificadores_tem_22_letras_e_nao_repetem() {
        let v: HashSet<String> = (1..500).map(|n| guid(n, 12345)).collect();
        assert_eq!(v.len(), 499);
        assert!(v.iter().all(|g| g.len() == 22));
    }

    #[test]
    fn peca_repetida_vai_uma_vez_e_volta_no_lugar() {
        let mut c = Cena { nome: "Obra".into(), ..Default::default() };
        let i = c.raiz_com(cubo("Pilar", 100.));
        let mut outro = c.nos[i].clone();
        outro.local = [0., -1., 0., 500., 1., 0., 0., 0., 0., 0., 1., 0.];
        outro.nome = "Pilar 2".into();
        c.nos.push(outro);
        c.raizes.push(1);
        c.malhas[0].material = Some(c.material("Cor", [0.5, 0.25, 0.125, 1.]));
        let t = gravar(&c, 1., Path::new("obra.ifc"));
        assert!(t.contains("FILE_SCHEMA(('IFC4'))"));
        assert_eq!(t.matches("IFCTRIANGULATEDFACESET").count(), 1);
        assert_eq!(t.matches("IFCBUILDINGELEMENTPROXY").count(), 2);
        let mut l = ler(t.as_bytes()).unwrap();
        l.enxugar();
        assert_eq!(l.malhas.len(), 1);
        let v = l.no_lugar();
        assert_eq!(v.len(), 2);
        assert!((v[1].volume() - 1e6).abs() < 1e-6, "{}", v[1].volume());
        let lo = v[1].pos.iter().fold(f64::MAX, |a, p| a.min(p[0]));
        assert!((lo - 400.).abs() < 1e-9, "{lo}");
        assert_eq!(l.materiais[0].cor, [0.5, 0.25, 0.125, 1.]);
        assert_eq!(l.unidade, "MILLIMETRE");
    }

    #[test]
    fn extrusao_fica_de_fora_e_o_relatorio_conta() {
        let t = "DATA;\n#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n#2=IFCEXTRUDEDAREASOLID($,$,$,1.);\n#3=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#2));\n#4=IFCPRODUCTDEFINITIONSHAPE($,$,(#3));\n#5=IFCLOCALPLACEMENT($,$);\n#6=IFCWALL('g',$,'Parede',$,$,#5,#4,$,$);\nENDSEC;";
        let c = ler(t.as_bytes()).unwrap();
        assert!(c.malhas.is_empty());
        assert_eq!(c.ignorados.get("sólido paramétrico sem malha pronta"), Some(&1));
    }

    #[test]
    fn faceted_brep_com_laco_de_pontos_fecha() {
        // tetraedro por faces, como os gravadores antigos escrevem
        let mut t = String::from("DATA;\n#1=IFCCARTESIANPOINT((0.,0.,0.));\n#2=IFCCARTESIANPOINT((1.,0.,0.));\n#3=IFCCARTESIANPOINT((0.,1.,0.));\n#4=IFCCARTESIANPOINT((0.,0.,1.));\n");
        let faces = [[1, 3, 2], [1, 2, 4], [2, 3, 4], [1, 4, 3]];
        let mut ids = vec![];
        for (i, f) in faces.iter().enumerate() {
            let b = 10 + i * 3;
            let _ = writeln!(t, "#{b}=IFCPOLYLOOP((#{},#{},#{}));\n#{}=IFCFACEOUTERBOUND(#{b},.T.);\n#{}=IFCFACE((#{}));", f[0], f[1], f[2], b + 1, b + 2, b + 1);
            ids.push(format!("#{}", b + 2));
        }
        let _ = writeln!(t, "#30=IFCCLOSEDSHELL(({}));\n#31=IFCFACETEDBREP(#30);\n#32=IFCSHAPEREPRESENTATION($,'Body','Brep',(#31));\n#33=IFCPRODUCTDEFINITIONSHAPE($,$,(#32));\n#34=IFCLOCALPLACEMENT($,$);\n#35=IFCMEMBER('g',$,'Barra',$,$,#34,#33,$,$);\nENDSEC;", ids.join(","));
        let c = ler(t.as_bytes()).unwrap();
        let m = c.malhas[0].soldada();
        assert!(m.fechada());
        assert_eq!(c.nos[0].nome, "Barra");
        // sem IfcSIUnit vale metro
        assert!((m.volume() - 1e9 / 6.).abs() < 1., "{}", m.volume());
    }
}
