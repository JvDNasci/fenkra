// SPDX-License-Identifier: LGPL-2.1-or-later
//! VRML97: sintaxe própria sobre o grafo de [`nos3d`](super::nos3d). A leitura é um analisador
//! recursivo genérico: nó é `Tipo { campo valor ... }`, e não precisa conhecer cada tipo para pular o
//! que não usa.

use super::nos3d::{cena_de, eixo_angulo, em_metros, nomes_def, No3d, Valor};
use super::{erro, Cena};
use crate::modelo::PROFUNDIDADE;
use crate::ErroConv;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
enum Ficha<'a> {
    Palavra(&'a str),
    Texto(&'a str),
    Numero(f64),
    Abre,
    Fecha,
    AbreLista,
    FechaLista,
}

fn fichas(t: &str) -> Vec<Ficha<'_>> {
    let b = t.as_bytes();
    let (mut i, mut v) = (0, Vec::new());
    while i < b.len() {
        match b[i] {
            b'#' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'{' | b'}' | b'[' | b']' => {
                v.push(match b[i] {
                    b'{' => Ficha::Abre,
                    b'}' => Ficha::Fecha,
                    b'[' => Ficha::AbreLista,
                    _ => Ficha::FechaLista,
                });
                i += 1;
            }
            b'"' => {
                let inicio = i + 1;
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                v.push(Ficha::Texto(t.get(inicio..i.min(b.len())).unwrap_or("")));
                i += 1;
            }
            c if c.is_ascii_whitespace() || c == b',' => i += 1,
            _ => {
                let inicio = i;
                while i < b.len() && !b[i].is_ascii_whitespace() && !matches!(b[i], b',' | b'{' | b'}' | b'[' | b']' | b'#' | b'"') {
                    i += 1;
                }
                let palavra = t.get(inicio..i).unwrap_or("");
                v.push(palavra.parse().map(Ficha::Numero).unwrap_or(Ficha::Palavra(palavra)));
            }
        }
    }
    v
}

struct Analise<'a> {
    f: Vec<Ficha<'a>>,
    i: usize,
    def: HashMap<String, Rc<No3d>>,
    ignorados: Vec<(String, usize)>,
}

impl<'a> Analise<'a> {
    fn proxima(&self) -> Option<&Ficha<'a>> {
        self.f.get(self.i)
    }

    /// Pula um bloco balanceado que começa na ficha atual.
    fn pular_bloco(&mut self) {
        let mut nivel = 0i32;
        while let Some(f) = self.proxima() {
            match f {
                Ficha::Abre | Ficha::AbreLista => nivel += 1,
                Ficha::Fecha | Ficha::FechaLista => nivel -= 1,
                _ => {}
            }
            self.i += 1;
            if nivel <= 0 {
                break;
            }
        }
    }

    /// Lê um nó, ou `USE`. `None` quando o nome de `USE` não existe.
    fn no(&mut self, nivel: usize) -> Result<Option<Rc<No3d>>, ErroConv> {
        if nivel > PROFUNDIDADE {
            return Err(erro("VRML", "árvore funda demais"));
        }
        let Some(Ficha::Palavra(mut p)) = self.proxima().cloned() else { return Err(erro("VRML", "esperava um nó")) };
        self.i += 1;
        if p == "USE" {
            let Some(Ficha::Palavra(nome)) = self.proxima().cloned() else { return Err(erro("VRML", "USE sem nome")) };
            self.i += 1;
            return Ok(self.def.get(nome).cloned());
        }
        let mut def = None;
        if p == "DEF" {
            let Some(Ficha::Palavra(nome)) = self.proxima().cloned() else { return Err(erro("VRML", "DEF sem nome")) };
            def = Some(nome.to_string());
            self.i += 1;
            let Some(Ficha::Palavra(tipo)) = self.proxima().cloned() else { return Err(erro("VRML", "DEF sem nó")) };
            p = tipo;
            self.i += 1;
        }
        if self.proxima() != Some(&Ficha::Abre) {
            return Err(erro("VRML", format!("esperava {{ depois de {p}")));
        }
        self.i += 1;
        let mut no = No3d { tipo: p.to_string(), nome: def.clone().unwrap_or_default(), ..Default::default() };
        loop {
            match self.proxima().cloned() {
                None => return Err(erro("VRML", "arquivo cortado")),
                Some(Ficha::Fecha) => {
                    self.i += 1;
                    break;
                }
                Some(Ficha::Palavra(campo)) => {
                    self.i += 1;
                    if campo == "ROUTE" {
                        self.i += 3;
                        continue;
                    }
                    if let Some(v) = self.valor(nivel)? {
                        no.campos.insert(campo.to_string(), v);
                    }
                }
                Some(_) => return Err(erro("VRML", "campo inválido")),
            }
        }
        let no = Rc::new(no);
        if let Some(nome) = def {
            self.def.insert(nome, no.clone());
        }
        Ok(Some(no))
    }

    fn valor(&mut self, nivel: usize) -> Result<Option<Valor>, ErroConv> {
        match self.proxima().cloned() {
            None => Err(erro("VRML", "arquivo cortado")),
            Some(Ficha::Numero(_)) => {
                let mut v = Vec::new();
                while let Some(Ficha::Numero(n)) = self.proxima() {
                    v.push(*n);
                    self.i += 1;
                }
                Ok(Some(Valor::Numeros(v)))
            }
            Some(Ficha::Texto(t)) => {
                self.i += 1;
                Ok(Some(Valor::Texto(t.to_string())))
            }
            Some(Ficha::AbreLista) => {
                self.i += 1;
                match self.proxima().cloned() {
                    Some(Ficha::Palavra(_)) => {
                        let mut nos = Vec::new();
                        while !matches!(self.proxima(), Some(Ficha::FechaLista) | None) {
                            if let Some(n) = self.no(nivel + 1)? {
                                nos.push(n);
                            }
                        }
                        self.i += 1;
                        Ok(Some(Valor::Nos(nos)))
                    }
                    _ => {
                        let mut v = Vec::new();
                        loop {
                            match self.proxima() {
                                Some(Ficha::Numero(n)) => v.push(*n),
                                Some(Ficha::FechaLista) | None => break,
                                _ => {}
                            }
                            self.i += 1;
                        }
                        self.i += 1;
                        Ok(Some(Valor::Numeros(v)))
                    }
                }
            }
            Some(Ficha::Palavra(p)) if matches!(p, "TRUE" | "FALSE") => {
                self.i += 1;
                Ok(Some(Valor::Texto(p.to_string())))
            }
            Some(Ficha::Palavra("NULL")) => {
                self.i += 1;
                Ok(None)
            }
            Some(Ficha::Palavra(_)) => Ok(self.no(nivel + 1)?.map(|n| Valor::Nos(vec![n]))),
            Some(_) => Err(erro("VRML", "valor inválido")),
        }
    }
}

pub(crate) fn ler(b: &[u8]) -> Result<Cena, ErroConv> {
    let texto = String::from_utf8_lossy(b);
    if texto.starts_with("#VRML V1") {
        return Err(erro("VRML", "só a versão 2.0 (VRML97) é suportada"));
    }
    let mut a = Analise { f: fichas(&texto), i: 0, def: HashMap::new(), ignorados: vec![] };
    let mut raizes = Vec::new();
    while let Some(f) = a.proxima().cloned() {
        match f {
            Ficha::Palavra("PROTO" | "EXTERNPROTO") => {
                a.ignorados.push(("protótipo".into(), 1));
                a.i += 2;
                while !matches!(a.proxima(), Some(Ficha::AbreLista) | None) {
                    a.i += 1;
                }
                a.pular_bloco();
                if a.proxima() == Some(&Ficha::Abre) {
                    a.pular_bloco();
                }
            }
            Ficha::Palavra("ROUTE") => a.i += 4,
            Ficha::Palavra(_) => {
                if let Some(n) = a.no(0)? {
                    raizes.push(n);
                }
            }
            _ => a.i += 1,
        }
    }
    cena_de(&raizes, a.ignorados, 1., "VRML")
}

fn lista(v: impl Iterator<Item = String>) -> String {
    v.collect::<Vec<_>>().join(" ")
}

struct Saida<'a> {
    c: &'a Cena,
    nomes: Vec<String>,
    /// nós e geometrias já definidos: da segunda vez em diante é `USE`
    nos: HashSet<usize>,
    malhas: HashSet<usize>,
    s: String,
}

impl Saida<'_> {
    fn forma(&mut self, i: usize) {
        let Some(m) = self.c.malhas.get(i) else { return };
        self.s.push_str("Shape {\n");
        if let Some(k) = self.c.cor_de(m) {
            let _ = writeln!(self.s, "appearance Appearance {{ material Material {{ diffuseColor {} {} {} transparency {} }} }}", k[0], k[1], k[2], 1. - k[3]);
        }
        if !self.malhas.insert(i) {
            let _ = writeln!(self.s, "geometry USE M_{i}\n}}");
            return;
        }
        let pontos = lista(m.pos.iter().map(|p| format!("{} {} {}", p[0], p[1], p[2])));
        let faces = lista(m.tris.iter().map(|t| format!("{} {} {} -1", t[0], t[1], t[2])));
        let _ = writeln!(self.s, "geometry DEF M_{i} IndexedFaceSet {{\nsolid TRUE\ncoord Coordinate {{ point [ {pontos} ] }}\ncoordIndex [ {faces} ]\n}}\n}}");
    }

    fn no(&mut self, i: usize, pilha: &mut Vec<usize>) {
        if self.nos.contains(&i) {
            let _ = writeln!(self.s, "USE {}", self.nomes[i]);
            return;
        }
        if pilha.contains(&i) {
            return;
        }
        let c = self.c;
        let Some(n) = c.nos.get(i) else { return };
        let (e, l) = (eixo_angulo(&n.local), &n.local);
        let _ = writeln!(self.s, "DEF {} Transform {{\ntranslation {} {} {}\nrotation {} {} {} {}\nchildren [", self.nomes[i], l[3], l[7], l[11], e[0], e[1], e[2], e[3]);
        pilha.push(i);
        for &m in &n.malhas {
            self.forma(m);
        }
        for &f in &n.filhos {
            self.no(f, pilha);
        }
        pilha.pop();
        self.s.push_str("]\n}\n");
        self.nos.insert(i);
    }
}

pub(crate) fn gravar(cena: &Cena) -> String {
    let c = em_metros(cena);
    let mut s = Saida { nomes: nomes_def(&c), c: &c, nos: HashSet::new(), malhas: HashSet::new(), s: String::new() };
    s.s.push_str("#VRML V2.0 utf8\n# Fenkra\nTransform {\nrotation 1 0 0 -1.5707963267948966\nchildren [\n");
    for &r in &c.raizes {
        s.no(r, &mut Vec::new());
    }
    s.s.push_str("]\n}\n");
    s.s
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn campos_que_nao_conhece_sao_pulados_e_primitiva_vira_nota() {
        let t = r#"#VRML V2.0 utf8
        WorldInfo { title "x" info [ "a" "b" ] }
        PROTO Coisa [ field SFInt32 a 1 ] { Group { } }
        Shape { geometry Box { size 1 1 1 } }
        Shape { appearance Appearance { material Material { diffuseColor 1 0 0 } }
                geometry IndexedFaceSet { coord Coordinate { point [ 0 0 0, 1 0 0, 0 1 0 ] } coordIndex [ 0 1 2 -1 ] } }
        ROUTE a.b TO c.d
        "#;
        let c = ler(t.as_bytes()).unwrap();
        assert_eq!(c.malhas.len(), 1);
        assert_eq!(c.malhas[0].tris.len(), 1);
        assert_eq!(c.materiais[0].cor, [1., 0., 0., 1.]);
        assert_eq!(c.ignorados.get("primitiva sem malha"), Some(&1));
        assert_eq!(c.ignorados.get("protótipo"), Some(&1));
    }

    #[test]
    fn use_de_si_mesmo_nao_vira_ciclo() {
        let c = ler(b"#VRML V2.0 utf8\nDEF A Group { children [ USE A ] }").unwrap();
        assert!(c.malhas.is_empty());
    }

    #[test]
    fn versao_1_e_recusada_com_motivo() {
        assert!(ler(b"#VRML V1.0 ascii\nSeparator { }").unwrap_err().to_string().contains("2.0"));
    }

    #[test]
    fn indice_fora_da_lista_e_erro() {
        let t = "#VRML V2.0 utf8\nShape { geometry IndexedFaceSet { coord Coordinate { point [ 0 0 0 ] } coordIndex [ 0 1 2 -1 ] } }";
        assert!(ler(t.as_bytes()).is_err());
    }
}
