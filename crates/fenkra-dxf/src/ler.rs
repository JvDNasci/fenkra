// SPDX-License-Identifier: LGPL-2.1-or-later
//! Leitura de DXF em texto, de R12 em diante. O arquivo é uma fila de pares código e valor; cada
//! entidade começa num código 0. Só o espaço do modelo entra, e blocos são explodidos com a posição
//! de cada inserção, com teto de profundidade e de quantidade: um arquivo pode inserir um bloco que
//! insere outro até estourar a memória.

use crate::{Anexo, Camada, Desenho, Entidade, Item, TipoLinha, Unidade, Vertice, P2};
use std::collections::HashMap;

/// Profundidade máxima de blocos dentro de blocos.
const PROFUNDIDADE: usize = 8;
/// Teto de entidades depois de explodir.
const TETO: usize = 1_000_000;
/// Teto de inserções percorridas: dez níveis de dez blocos já seriam dez bilhões de chamadas.
const TETO_INSERCOES: usize = 200_000;
/// Passos por volta quando uma curva precisa virar polilinha para aceitar uma transformação.
const PASSOS_POR_VOLTA: f64 = 96.;

type Par<'a> = (i32, &'a str);

fn pares(texto: &str) -> Result<Vec<Par<'_>>, String> {
    let mut linhas = texto.lines();
    let mut saida = Vec::new();
    let mut numero = 0;
    while let Some(codigo) = linhas.next() {
        numero += 1;
        let codigo = codigo.trim();
        if codigo.is_empty() {
            continue;
        }
        let c: i32 = codigo.parse().map_err(|_| format!("código inválido na linha {numero}: '{}'", codigo.chars().take(20).collect::<String>()))?;
        let Some(valor) = linhas.next() else { return Err(format!("o arquivo termina no meio de um par, na linha {numero}")) };
        numero += 1;
        saida.push((c, valor.trim_end_matches('\r')));
    }
    Ok(saida)
}

/// O texto do arquivo. R2007 em diante é UTF-8; antes disso, página de código. Quem não é UTF-8 válido
/// fica como Latin-1, o que acerta os acentos do português na página mais comum.
fn decodificar(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(t) => t.trim_start_matches('\u{feff}').to_string(),
        Err(_) => bytes.iter().map(|b| *b as char).collect(),
    }
}

fn num(v: &str) -> f64 {
    v.trim().parse().unwrap_or(0.)
}

fn int(v: &str) -> i32 {
    v.trim().parse().unwrap_or(0)
}

/// Uma entidade crua: o tipo e os pares dela, sem o código 0.
struct Crua<'a> {
    tipo: &'a str,
    pares: &'a [Par<'a>],
}

impl<'a> Crua<'a> {
    fn f(&self, c: i32) -> f64 {
        self.pares.iter().find(|p| p.0 == c).map(|p| num(p.1)).unwrap_or(0.)
    }

    fn f_ou(&self, c: i32, padrao: f64) -> f64 {
        self.pares.iter().find(|p| p.0 == c).map(|p| num(p.1)).unwrap_or(padrao)
    }

    fn i(&self, c: i32) -> i32 {
        self.pares.iter().find(|p| p.0 == c).map(|p| int(p.1)).unwrap_or(0)
    }

    fn s(&self, c: i32) -> Option<&'a str> {
        self.pares.iter().find(|p| p.0 == c).map(|p| p.1)
    }

    fn p(&self, c: i32) -> P2 {
        [self.f(c), self.f(c + 10)]
    }

    /// Todos os pontos de um código, na ordem.
    fn pontos(&self, c: i32) -> Vec<P2> {
        let xs = self.pares.iter().filter(|p| p.0 == c).map(|p| num(p.1));
        let ys = self.pares.iter().filter(|p| p.0 == c + 10).map(|p| num(p.1));
        xs.zip(ys).map(|(x, y)| [x, y]).collect()
    }

    fn todos(&self, c: i32) -> Vec<f64> {
        self.pares.iter().filter(|p| p.0 == c).map(|p| num(p.1)).collect()
    }
}

/// Fatia uma sequência de pares nas entidades dela.
fn entidades<'a>(p: &'a [Par<'a>]) -> Vec<Crua<'a>> {
    let inicios: Vec<usize> = p.iter().enumerate().filter(|(_, q)| q.0 == 0).map(|(i, _)| i).collect();
    let mut v = Vec::with_capacity(inicios.len());
    for (k, i) in inicios.iter().enumerate() {
        let fim = inicios.get(k + 1).copied().unwrap_or(p.len());
        v.push(Crua { tipo: p[*i].1, pares: &p[i + 1..fim] });
    }
    v
}

/// Intervalos das seções, pelo nome.
fn secoes<'a>(p: &'a [Par<'a>]) -> HashMap<&'a str, &'a [Par<'a>]> {
    let mut mapa = HashMap::new();
    let mut i = 0;
    while i + 1 < p.len() {
        if p[i] == (0, "SECTION") && p[i + 1].0 == 2 {
            let nome = p[i + 1].1;
            let fim = p[i + 2..].iter().position(|q| *q == (0, "ENDSEC")).map(|k| i + 2 + k).unwrap_or(p.len());
            mapa.insert(nome, &p[i + 2..fim]);
            i = fim;
        }
        i += 1;
    }
    mapa
}

fn cabecalho(p: &[Par<'_>], d: &mut Desenho) {
    for par in p.windows(2) {
        match par[0] {
            (9, "$ACADVER") => d.versao = par[1].1.to_string(),
            (9, "$INSUNITS") => d.unidade = Unidade::do_codigo(int(par[1].1)),
            _ => {}
        }
    }
}

fn tabelas(p: &[Par<'_>], d: &mut Desenho) {
    for e in entidades(p) {
        match e.tipo {
            "LAYER" => {
                let Some(nome) = e.s(2) else { continue };
                let tipo_linha = e.s(6).unwrap_or("Continuous").to_string();
                let cor = e.i(62).unsigned_abs().min(255) as u16;
                let espessura = e.s(370).map(|v| int(v) as i16).filter(|w| *w >= 0);
                d.camadas.push(Camada { nome: desfazer(nome), cor, tipo_linha, espessura });
            }
            "LTYPE" => {
                let Some(nome) = e.s(2) else { continue };
                if !["BYBLOCK", "BYLAYER", "CONTINUOUS"].contains(&nome.to_uppercase().as_str()) {
                    d.tipos_linha.push(TipoLinha { nome: desfazer(nome), padrao: e.todos(49) });
                }
            }
            _ => {}
        }
    }
}

/// Desfaz os `\U+XXXX` de um texto, juntando os pares substitutos.
fn desfazer(t: &str) -> String {
    let mut unidades: Vec<u16> = Vec::with_capacity(t.len());
    let letras: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < letras.len() {
        if letras[i] == '\\' && letras.get(i + 1) == Some(&'U') && letras.get(i + 2) == Some(&'+') && i + 6 < letras.len() {
            let h: String = letras[i + 3..i + 7].iter().collect();
            if let Ok(u) = u16::from_str_radix(&h, 16) {
                unidades.push(u);
                i += 7;
                continue;
            }
        }
        let mut b = [0u16; 2];
        unidades.extend_from_slice(letras[i].encode_utf16(&mut b));
        i += 1;
    }
    String::from_utf16_lossy(&unidades)
}

/// Tira a formatação do MTEXT: `\P` é quebra, `{}` agrupa, `\f...;` e parecidos mudam a fonte.
fn texto_de_mtext(cru: &str) -> String {
    let t = desfazer(cru);
    let letras: Vec<char> = t.chars().collect();
    let mut s = String::new();
    let mut i = 0;
    while i < letras.len() {
        match letras[i] {
            '{' | '}' => i += 1,
            '\\' => {
                let c = letras.get(i + 1).copied().unwrap_or(' ');
                i += 2;
                match c {
                    'P' => s.push('\n'),
                    '\\' | '{' | '}' => s.push(c),
                    '~' => s.push(' '),
                    'L' | 'l' | 'O' | 'o' | 'K' | 'k' => {}
                    _ => {
                        while i < letras.len() && letras[i] != ';' {
                            i += 1;
                        }
                        i += 1;
                    }
                }
            }
            c => {
                s.push(c);
                i += 1;
            }
        }
    }
    s
}

/// Os atalhos `%%` do texto de uma linha.
fn texto_simples(t: &str) -> String {
    desfazer(t).replace("%%c", "Ø").replace("%%C", "Ø").replace("%%d", "°").replace("%%D", "°").replace("%%p", "±").replace("%%P", "±")
}

/// Normal apontando para baixo espelha o plano do objeto em X. Fora do eixo Z, a entidade não está no
/// plano do desenho.
#[derive(Clone, Copy, PartialEq)]
enum Plano {
    Normal,
    Espelhado,
    Fora,
}

fn plano(e: &Crua) -> Plano {
    let n = [e.f(210), e.f(220), e.f_ou(230, 1.)];
    if n[0].abs() > 1e-9 || n[1].abs() > 1e-9 {
        Plano::Fora
    } else if n[2] < 0. {
        Plano::Espelhado
    } else {
        Plano::Normal
    }
}

/// Transformação afim 2D: `x' = a x + b y + e`, `y' = c x + d y + f`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Afim {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Afim {
    const IDENTIDADE: Afim = Afim { a: 1., b: 0., c: 0., d: 1., e: 0., f: 0. };
    const ESPELHO_X: Afim = Afim { a: -1., b: 0., c: 0., d: 1., e: 0., f: 0. };

    fn ponto(&self, p: P2) -> P2 {
        [self.a * p[0] + self.b * p[1] + self.e, self.c * p[0] + self.d * p[1] + self.f]
    }

    fn vetor(&self, v: P2) -> P2 {
        [self.a * v[0] + self.b * v[1], self.c * v[0] + self.d * v[1]]
    }

    /// `self` depois de `o`.
    fn compor(&self, o: &Afim) -> Afim {
        let [a, c] = self.vetor([o.a, o.c]);
        let [b, d] = self.vetor([o.b, o.d]);
        let [e, f] = self.ponto([o.e, o.f]);
        Afim { a, b, c, d, e, f }
    }

    fn determinante(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// Fator de escala, quando é semelhança: mantém círculo como círculo.
    fn semelhanca(&self) -> Option<f64> {
        let (s1, s2) = (self.a.hypot(self.c), self.b.hypot(self.d));
        let ortogonal = (self.a * self.b + self.c * self.d).abs() < 1e-9 * s1.max(1.) * s2.max(1.);
        ((s1 - s2).abs() < 1e-9 * s1.max(1.) && ortogonal && s1 > 0.).then_some(s1)
    }
}

fn amostrar_arco(centro: P2, raio: f64, ini: f64, fim: f64) -> Vec<Vertice> {
    let varredura = (fim - ini).rem_euclid(360.);
    let varredura = if varredura == 0. { 360. } else { varredura };
    let n = ((varredura / 360. * PASSOS_POR_VOLTA).ceil() as usize).max(2);
    (0..=n)
        .map(|i| (ini + varredura * i as f64 / n as f64).to_radians())
        .map(|t| Vertice { p: [centro[0] + raio * t.cos(), centro[1] + raio * t.sin()], bojo: 0. })
        .collect()
}

fn amostrar_elipse(centro: P2, maior: P2, razao: f64, t0: f64, t1: f64) -> Vec<Vertice> {
    let varredura = (t1 - t0).rem_euclid(std::f64::consts::TAU);
    let varredura = if varredura.abs() < 1e-12 { std::f64::consts::TAU } else { varredura };
    let n = ((varredura / std::f64::consts::TAU * PASSOS_POR_VOLTA).ceil() as usize).max(2);
    let menor = [-maior[1] * razao, maior[0] * razao];
    (0..=n)
        .map(|i| t0 + varredura * i as f64 / n as f64)
        .map(|t| Vertice { p: [centro[0] + maior[0] * t.cos() + menor[0] * t.sin(), centro[1] + maior[1] * t.cos() + menor[1] * t.sin()], bojo: 0. })
        .collect()
}

/// Polilinha com os arcos dos bojos já em pontos, para aceitar qualquer afim.
fn sem_bojo(vertices: &[Vertice], fechada: bool) -> Vec<Vertice> {
    let mut saida = Vec::new();
    let n = vertices.len();
    for i in 0..n {
        let (a, b) = (vertices[i], vertices[(i + 1) % n]);
        saida.push(Vertice { p: a.p, bojo: 0. });
        if i + 1 == n && !fechada {
            break;
        }
        if let Some((c, r, esquerda)) = crate::arco_do_bojo(a.p, b.p, a.bojo) {
            let ang = |p: P2| (p[1] - c[1]).atan2(p[0] - c[0]).to_degrees();
            let (ini, fim) = if esquerda { (ang(a.p), ang(b.p)) } else { (ang(b.p), ang(a.p)) };
            let mut miolo = amostrar_arco(c, r, ini, fim);
            if !esquerda {
                miolo.reverse();
            }
            saida.extend(&miolo[1..miolo.len() - 1]);
        }
    }
    saida
}

/// Aplica a transformação. Semelhança mantém arco e círculo exatos; o resto vira polilinha.
fn transformar(e: Entidade, t: &Afim) -> Entidade {
    if *t == Afim::IDENTIDADE {
        return e;
    }
    let inverte = t.determinante() < 0.;
    match e {
        Entidade::Linha(a, b) => Entidade::Linha(t.ponto(a), t.ponto(b)),
        Entidade::Ponto(p) => Entidade::Ponto(t.ponto(p)),
        Entidade::Circulo { centro, raio } => match t.semelhanca() {
            Some(s) => Entidade::Circulo { centro: t.ponto(centro), raio: raio * s },
            None => transformar_pontos(amostrar_arco(centro, raio, 0., 360.), true, t),
        },
        Entidade::Arco { centro, raio, ini, fim } => match t.semelhanca() {
            Some(s) => {
                let ang = |g: f64| {
                    let (sn, cs) = g.to_radians().sin_cos();
                    let [x, y] = t.vetor([cs, sn]);
                    y.atan2(x).to_degrees().rem_euclid(360.)
                };
                let (a, b) = if inverte { (ang(fim), ang(ini)) } else { (ang(ini), ang(fim)) };
                Entidade::Arco { centro: t.ponto(centro), raio: raio * s, ini: a, fim: b }
            }
            None => transformar_pontos(amostrar_arco(centro, raio, ini, fim), false, t),
        },
        Entidade::Polilinha { vertices, fechada } => {
            if t.semelhanca().is_some() {
                let sinal = if inverte { -1. } else { 1. };
                Entidade::Polilinha { vertices: vertices.iter().map(|v| Vertice { p: t.ponto(v.p), bojo: v.bojo * sinal }).collect(), fechada }
            } else {
                transformar_pontos(sem_bojo(&vertices, fechada), fechada, t)
            }
        }
        Entidade::Elipse { centro, maior, razao, t0, t1 } => {
            if t.semelhanca().is_some() && !inverte {
                Entidade::Elipse { centro: t.ponto(centro), maior: t.vetor(maior), razao, t0, t1 }
            } else {
                let fechada = (t1 - t0 - std::f64::consts::TAU).abs() < 1e-9;
                transformar_pontos(amostrar_elipse(centro, maior, razao, t0, t1), fechada, t)
            }
        }
        Entidade::Spline { grau, controle, nos, pesos, ajuste, fechada } => {
            Entidade::Spline { grau, controle: controle.iter().map(|p| t.ponto(*p)).collect(), nos, pesos, ajuste: ajuste.iter().map(|p| t.ponto(*p)).collect(), fechada }
        }
        Entidade::Texto { pos, altura, angulo, anexo, texto } => {
            let (sn, cs) = angulo.to_radians().sin_cos();
            let [x, y] = t.vetor([cs, sn]);
            Entidade::Texto { pos: t.ponto(pos), altura: altura * t.determinante().abs().sqrt(), angulo: y.atan2(x).to_degrees(), anexo, texto }
        }
        Entidade::Hachura { lacos } => Entidade::Hachura { lacos: lacos.iter().map(|l| l.iter().map(|p| t.ponto(*p)).collect()).collect() },
    }
}

fn transformar_pontos(vertices: Vec<Vertice>, fechada: bool, t: &Afim) -> Entidade {
    Entidade::Polilinha { vertices: vertices.iter().map(|v| Vertice { p: t.ponto(v.p), bojo: 0. }).collect(), fechada }
}

fn lwpolyline(e: &Crua) -> Entidade {
    let mut vertices: Vec<Vertice> = Vec::new();
    let mut x = None;
    for (c, v) in e.pares {
        match c {
            10 => x = Some(num(v)),
            20 => {
                if let Some(x) = x.take() {
                    vertices.push(Vertice { p: [x, num(v)], bojo: 0. });
                }
            }
            42 => {
                if let Some(u) = vertices.last_mut() {
                    u.bojo = num(v);
                }
            }
            _ => {}
        }
    }
    Entidade::Polilinha { vertices, fechada: e.i(70) & 1 != 0 }
}

fn spline(e: &Crua) -> Entidade {
    let flags = e.i(70);
    Entidade::Spline { grau: e.i(71).max(1) as u32, controle: e.pontos(10), nos: e.todos(40), pesos: e.todos(41), ajuste: e.pontos(11), fechada: flags & 1 != 0 }
}

fn hatch(e: &Crua) -> Option<Entidade> {
    let mut lacos: Vec<Vec<P2>> = Vec::new();
    let mut atual: Option<Vec<P2>> = None;
    let (mut i, p) = (0, e.pares);
    while i < p.len() {
        match p[i].0 {
            92 => {
                lacos.extend(atual.take());
                atual = Some(Vec::new());
            }
            // semente depois dos contornos: o ponto dela não é vértice
            98 => break,
            10 if p.get(i + 1).is_some_and(|q| q.0 == 20) => {
                if let Some(l) = atual.as_mut() {
                    l.push([num(p[i].1), num(p[i + 1].1)]);
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    lacos.extend(atual);
    lacos.retain(|l| l.len() >= 3);
    (!lacos.is_empty()).then_some(Entidade::Hachura { lacos })
}

fn texto(e: &Crua) -> Entidade {
    let (anexo, conteudo, pos) = if e.tipo == "MTEXT" {
        let anexo = match e.i(71) {
            2 | 5 | 8 => Anexo::Centro,
            3 | 6 | 9 => Anexo::Direita,
            _ => Anexo::Esquerda,
        };
        let cru: String = e.pares.iter().filter(|p| p.0 == 3 || p.0 == 1).map(|p| p.1).collect();
        (anexo, texto_de_mtext(&cru), e.p(10))
    } else {
        let anexo = match e.i(72) {
            1 => Anexo::Centro,
            2 => Anexo::Direita,
            _ => Anexo::Esquerda,
        };
        let alinhado = e.i(72) != 0 && e.s(11).is_some();
        (anexo, texto_simples(e.s(1).unwrap_or_default()), if alinhado { e.p(11) } else { e.p(10) })
    };
    let angulo = if e.tipo == "MTEXT" && e.s(11).is_some() { e.f(21).atan2(e.f(11)).to_degrees() } else { e.f(50) };
    Entidade::Texto { pos, altura: e.f(40), angulo, anexo, texto: conteudo }
}

/// Bloco: ponto de base e a sequência de pares das entidades.
struct Bloco<'a> {
    base: P2,
    entidades: Vec<Crua<'a>>,
}

fn blocos<'a>(p: &'a [Par<'a>]) -> HashMap<&'a str, Bloco<'a>> {
    let mut mapa = HashMap::new();
    let mut i = 0;
    while i < p.len() {
        if p[i] != (0, "BLOCK") {
            i += 1;
            continue;
        }
        let fim = p[i..].iter().position(|q| *q == (0, "ENDBLK")).map(|k| i + k).unwrap_or(p.len());
        let trecho = &p[i..fim];
        let corpo = trecho[1..].iter().position(|q| q.0 == 0).map(|k| k + 1).unwrap_or(trecho.len());
        let cabeca = Crua { tipo: "BLOCK", pares: &trecho[1..corpo] };
        if let Some(nome) = cabeca.s(2) {
            mapa.insert(nome, Bloco { base: cabeca.p(10), entidades: entidades(&trecho[corpo..]) });
        }
        i = fim + 1;
    }
    mapa
}

struct Leitura<'a> {
    blocos: HashMap<&'a str, Bloco<'a>>,
    d: Desenho,
    insercoes: usize,
}

impl Leitura<'_> {
    fn ignorar(&mut self, tipo: &str) {
        *self.d.ignorados.entry(tipo.to_string()).or_insert(0) += 1;
    }

    fn guardar(&mut self, e: &Crua, camada_pai: &str, entidade: Entidade, t: &Afim) {
        if self.d.itens.len() >= TETO {
            self.ignorar("acima do teto de entidades");
            return;
        }
        let camada = e.s(8).filter(|c| *c != "0" || camada_pai.is_empty()).map(desfazer).unwrap_or_else(|| camada_pai.to_string());
        let cor = e.s(62).map(int).filter(|c| (1..=255).contains(c)).map(|c| c as u16);
        let espessura = e.s(370).map(|v| int(v) as i16).filter(|w| *w >= 0);
        self.d.itens.push(Item { camada, cor, espessura, entidade: transformar(entidade, t) });
    }

    fn inserir(&mut self, e: &Crua, camada_pai: &str, t: &Afim, nivel: usize) {
        let Some(nome) = e.s(2) else { return };
        if nivel >= PROFUNDIDADE {
            self.ignorar("INSERT fundo demais");
            return;
        }
        let Some(base) = self.blocos.get(nome).map(|b| b.base) else {
            self.ignorar("INSERT sem bloco");
            return;
        };
        let (sn, cs) = e.f(50).to_radians().sin_cos();
        let (sx, sy) = (e.f_ou(41, 1.), e.f_ou(42, 1.));
        let (colunas, linhas) = (e.i(70).max(1), e.i(71).max(1));
        let (dx, dy) = (e.f(44), e.f(45));
        let camada = e.s(8).unwrap_or(camada_pai).to_string();
        let pos = e.p(10);
        for i in 0..linhas {
            for j in 0..colunas {
                // a grade gira junto com a inserção
                let (gx, gy) = (j as f64 * dx, i as f64 * dy);
                let origem = [pos[0] + gx * cs - gy * sn, pos[1] + gx * sn + gy * cs];
                let local = Afim { a: cs * sx, b: -sn * sy, c: sn * sx, d: cs * sy, e: origem[0], f: origem[1] };
                let sem_base = Afim { e: -base[0], f: -base[1], ..Afim::IDENTIDADE };
                let m = t.compor(&local).compor(&sem_base);
                self.insercoes += 1;
                if self.insercoes > TETO_INSERCOES {
                    self.ignorar("INSERT acima do teto");
                    return;
                }
                self.corpo_do_bloco(nome, &camada, &m, nivel + 1);
            }
        }
    }

    fn corpo_do_bloco(&mut self, nome: &str, camada: &str, m: &Afim, nivel: usize) {
        // o mapa empresta `self`: tira o bloco para percorrer e devolve no fim
        let Some((chave, bloco)) = self.blocos.remove_entry(nome) else {
            // o bloco já está aberto mais acima: um bloco que insere a si mesmo
            self.ignorar("INSERT em ciclo");
            return;
        };
        for (i, e) in bloco.entidades.iter().enumerate() {
            self.uma(e, &bloco.entidades[i + 1..], camada, m, nivel);
        }
        self.blocos.insert(chave, bloco);
    }

    /// Lê uma entidade. `seguintes` serve à POLYLINE, cujos vértices são as entidades depois dela.
    fn uma(&mut self, e: &Crua, seguintes: &[Crua], camada_pai: &str, t: &Afim, nivel: usize) {
        if e.i(67) == 1 {
            return self.ignorar("espaço de papel");
        }
        let plano_e = plano(e);
        if plano_e == Plano::Fora {
            return self.ignorar(&format!("{} fora do plano", e.tipo));
        }
        // linha, ponto, spline e elipse já vêm nas coordenadas do desenho; o resto, nas do plano do objeto
        let espelhado = plano_e == Plano::Espelhado;
        let m = if espelhado && !matches!(e.tipo, "LINE" | "POINT" | "SPLINE" | "ELLIPSE") { t.compor(&Afim::ESPELHO_X) } else { *t };
        let entidade = match e.tipo {
            "LINE" => Entidade::Linha(e.p(10), e.p(11)),
            "POINT" => Entidade::Ponto(e.p(10)),
            "CIRCLE" => Entidade::Circulo { centro: e.p(10), raio: e.f(40) },
            "ARC" => Entidade::Arco { centro: e.p(10), raio: e.f(40), ini: e.f(50), fim: e.f(51) },
            "ELLIPSE" => {
                let (t0, t1) = (e.f(41), e.f_ou(42, std::f64::consts::TAU));
                // vista por baixo, a elipse anda ao contrário: o mesmo trecho, com o parâmetro negado
                let (t0, t1) = if espelhado { (-t1, -t0) } else { (t0, t1) };
                Entidade::Elipse { centro: e.p(10), maior: e.p(11), razao: e.f(40), t0, t1 }
            }
            "LWPOLYLINE" => lwpolyline(e),
            "SPLINE" => spline(e),
            "TEXT" | "MTEXT" => texto(e),
            "HATCH" => match hatch(e) {
                Some(h) => h,
                None => return self.ignorar("HATCH sem contorno de polilinha"),
            },
            "POLYLINE" => {
                let flags = e.i(70);
                if flags & (16 | 64) != 0 {
                    return self.ignorar("POLYLINE de malha");
                }
                let vertices: Vec<Vertice> = seguintes
                    .iter()
                    .take_while(|v| v.tipo == "VERTEX")
                    .filter(|v| v.i(70) & 16 == 0)
                    .map(|v| Vertice { p: v.p(10), bojo: v.f(42) })
                    .collect();
                Entidade::Polilinha { vertices, fechada: flags & 1 != 0 }
            }
            "VERTEX" | "SEQEND" | "ATTRIB" | "ATTDEF" | "VIEWPORT" => return,
            "INSERT" => return self.inserir(e, camada_pai, &m, nivel),
            outro => return self.ignorar(outro),
        };
        self.guardar(e, camada_pai, entidade, &m);
    }
}

/// A versão binária abre com uma frase fixa de 22 bytes, que termina em `Binary DXF` e quebra de linha.
pub fn e_binario(bytes: &[u8]) -> bool {
    bytes.get(..22).is_some_and(|c| c.windows(10).any(|w| w == b"Binary DXF"))
}

/// Lê o arquivo.
pub fn ler(bytes: &[u8]) -> Result<Desenho, String> {
    if e_binario(bytes) {
        return Err("DXF binário não é lido: grave o arquivo em DXF de texto".into());
    }
    let texto = decodificar(bytes);
    let p = pares(&texto)?;
    let mapa = secoes(&p);
    let Some(ents) = mapa.get("ENTITIES") else { return Err("o arquivo não tem a seção ENTITIES".into()) };
    let mut d = Desenho::default();
    if let Some(h) = mapa.get("HEADER") {
        cabecalho(h, &mut d);
    }
    if let Some(t) = mapa.get("TABLES") {
        tabelas(t, &mut d);
    }
    let blocos = mapa.get("BLOCKS").map(|b| blocos(b)).unwrap_or_default();
    let mut l = Leitura { blocos, d, insercoes: 0 };
    let lista = entidades(ents);
    for (i, e) in lista.iter().enumerate() {
        l.uma(e, &lista[i + 1..], "", &Afim::IDENTIDADE, 0);
    }
    Ok(l.d)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn dxf(pares: &[(i32, &str)]) -> String {
        pares.iter().map(|(c, v)| format!("{c}\n{v}\n")).collect()
    }

    fn com_entidades(corpo: &str) -> Vec<u8> {
        format!("0\nSECTION\n2\nENTITIES\n{corpo}0\nENDSEC\n0\nEOF\n").into_bytes()
    }

    fn linha(a: P2, b: P2, camada: &str) -> String {
        let (ax, ay, bx, by) = (a[0].to_string(), a[1].to_string(), b[0].to_string(), b[1].to_string());
        dxf(&[(0, "LINE"), (8, camada), (10, &ax), (20, &ay), (11, &bx), (21, &by)])
    }

    #[test]
    fn r12_sem_cabecalho_entra_e_a_unidade_fica_em_aberto() {
        let d = ler(&com_entidades(&linha([0., 0.], [10., 5.], "A"))).unwrap();
        assert_eq!(d.unidade, Unidade::SemUnidade);
        assert_eq!(d.itens, vec![Item::novo("A", Entidade::Linha([0., 0.], [10., 5.]))]);
    }

    #[test]
    fn unidade_e_camadas_saem_do_arquivo() {
        let mut t = dxf(&[(0, "SECTION"), (2, "HEADER"), (9, "$ACADVER"), (1, "AC1015"), (9, "$INSUNITS"), (70, "1"), (0, "ENDSEC")]);
        t += &dxf(&[(0, "SECTION"), (2, "TABLES"), (0, "TABLE"), (2, "LAYER"), (0, "LAYER"), (2, "Furos"), (70, "0"), (62, "-3"), (6, "DASHED"), (0, "ENDTAB"), (0, "ENDSEC")]);
        t += "0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF\n";
        let d = ler(t.as_bytes()).unwrap();
        assert_eq!((d.versao.as_str(), d.unidade), ("AC1015", Unidade::Polegada));
        assert_eq!(d.camadas, vec![Camada { nome: "Furos".into(), cor: 3, tipo_linha: "DASHED".into(), espessura: None }]);
    }

    #[test]
    fn lixo_vira_erro_com_linha_e_binario_explica() {
        assert!(ler(b"nada a ver\ncom dxf\n").unwrap_err().contains("linha 1"));
        assert!(ler(b"0\nSECTION\n2\nHEADER\n0\nENDSEC\n0\nEOF\n").unwrap_err().contains("ENTITIES"));
        assert!(ler(b"Marca e Binary DXF\r\n\x1a\0\0\0").unwrap_err().contains("binário"));
        assert!(ler(b"0").unwrap_err().contains("meio de um par"));
    }

    #[test]
    fn latin1_vira_acento_e_quebra_de_linha_do_windows_nao_atrapalha() {
        let t = b"0\r\nSECTION\r\n2\r\nENTITIES\r\n0\r\nTEXT\r\n8\r\nCota\xe7\xe3o\r\n1\r\n%%c10\r\n40\r\n2.5\r\n0\r\nENDSEC\r\n0\r\nEOF\r\n";
        let d = ler(t).unwrap();
        assert_eq!(d.itens[0].camada, "Cotação");
        assert!(matches!(&d.itens[0].entidade, Entidade::Texto { texto, .. } if texto == "Ø10"));
    }

    #[test]
    fn polilinha_antiga_e_leve_guardam_bojo_e_fechamento() {
        let antiga = dxf(&[(0, "POLYLINE"), (8, "A"), (66, "1"), (70, "1"), (0, "VERTEX"), (10, "0"), (20, "0"), (42, "1"), (0, "VERTEX"), (10, "10"), (20, "0"), (0, "SEQEND")]);
        let leve = dxf(&[(0, "LWPOLYLINE"), (8, "A"), (90, "2"), (70, "1"), (10, "0"), (20, "0"), (42, "1"), (10, "10"), (20, "0")]);
        for t in [antiga, leve] {
            let d = ler(&com_entidades(&t)).unwrap();
            let Entidade::Polilinha { vertices, fechada } = &d.itens[0].entidade else { panic!("{d:?}") };
            assert!(*fechada && vertices.len() == 2 && vertices[0].bojo == 1. && vertices[1].p == [10., 0.], "{vertices:?}");
        }
    }

    #[test]
    fn bloco_explode_com_escala_giro_e_camada_zero_herda() {
        let mut t = dxf(&[(0, "SECTION"), (2, "BLOCKS"), (0, "BLOCK"), (2, "Furo"), (10, "1"), (20, "0")]);
        t += &dxf(&[(0, "CIRCLE"), (8, "0"), (10, "1"), (20, "0"), (40, "2"), (0, "LINE"), (8, "Eixo"), (10, "1"), (20, "0"), (11, "2"), (21, "0"), (0, "ENDBLK"), (0, "ENDSEC")]);
        t += &dxf(&[(0, "SECTION"), (2, "ENTITIES"), (0, "INSERT"), (8, "Pecas"), (2, "Furo"), (10, "100"), (20, "50"), (41, "3"), (42, "3"), (50, "90"), (0, "ENDSEC"), (0, "EOF")]);
        let d = ler(t.as_bytes()).unwrap();
        assert_eq!(d.itens.len(), 2);
        let Entidade::Circulo { centro, raio } = d.itens[0].entidade else { panic!("{d:?}") };
        assert!((centro[0] - 100.).abs() < 1e-9 && (centro[1] - 50.).abs() < 1e-9 && (raio - 6.).abs() < 1e-9, "{centro:?} {raio}");
        assert_eq!(d.itens[0].camada, "Pecas", "o que está na camada 0 herda a da inserção");
        assert_eq!(d.itens[1].camada, "Eixo");
        let Entidade::Linha(_, b) = d.itens[1].entidade else { panic!() };
        assert!((b[0] - 100.).abs() < 1e-9 && (b[1] - 53.).abs() < 1e-9, "a linha gira 90 graus: {b:?}");
    }

    #[test]
    fn escala_desigual_transforma_circulo_em_polilinha_e_espelho_troca_o_arco() {
        let mut t = dxf(&[(0, "SECTION"), (2, "BLOCKS"), (0, "BLOCK"), (2, "B"), (10, "0"), (20, "0")]);
        t += &dxf(&[(0, "CIRCLE"), (8, "0"), (10, "0"), (20, "0"), (40, "1"), (0, "ARC"), (8, "0"), (10, "0"), (20, "0"), (40, "1"), (50, "0"), (51, "90"), (0, "ENDBLK"), (0, "ENDSEC")]);
        t += &dxf(&[(0, "SECTION"), (2, "ENTITIES"), (0, "INSERT"), (2, "B"), (41, "2"), (42, "1"), (0, "INSERT"), (2, "B"), (41, "-1"), (42, "1"), (0, "ENDSEC"), (0, "EOF")]);
        let d = ler(t.as_bytes()).unwrap();
        assert!(matches!(d.itens[0].entidade, Entidade::Polilinha { fechada: true, .. }), "{:?}", d.itens[0]);
        let Entidade::Arco { ini, fim, .. } = d.itens[3].entidade else { panic!("{:?}", d.itens[3]) };
        assert!((ini - 90.).abs() < 1e-9 && (fim - 180.).abs() < 1e-9, "o quadrante vai para o outro lado: {ini} {fim}");
    }

    #[test]
    fn bloco_que_insere_a_si_mesmo_nao_trava() {
        let mut t = dxf(&[(0, "SECTION"), (2, "BLOCKS"), (0, "BLOCK"), (2, "A"), (10, "0"), (20, "0"), (0, "INSERT"), (2, "A"), (0, "ENDBLK"), (0, "ENDSEC")]);
        t += &dxf(&[(0, "SECTION"), (2, "ENTITIES"), (0, "INSERT"), (2, "A"), (0, "ENDSEC"), (0, "EOF")]);
        let d = ler(t.as_bytes()).unwrap();
        assert!(d.itens.is_empty());
        assert!(!d.ignorados.is_empty());
    }

    #[test]
    fn bomba_de_blocos_para_no_teto() {
        // dez níveis de dez inserções: passaria de 10^10 entidades sem o teto de profundidade
        let mut t = dxf(&[(0, "SECTION"), (2, "BLOCKS")]);
        for n in 0..10 {
            t += &dxf(&[(0, "BLOCK"), (2, &format!("B{n}")), (10, "0"), (20, "0")]);
            for _ in 0..10 {
                t += &dxf(&[(0, "INSERT"), (2, &format!("B{}", n + 1))]);
            }
            t += &dxf(&[(0, "ENDBLK")]);
        }
        t += &dxf(&[(0, "BLOCK"), (2, "B10"), (10, "0"), (20, "0"), (0, "POINT"), (10, "1"), (20, "1"), (0, "ENDBLK"), (0, "ENDSEC")]);
        t += &dxf(&[(0, "SECTION"), (2, "ENTITIES"), (0, "INSERT"), (2, "B0"), (0, "ENDSEC"), (0, "EOF")]);
        let inicio = std::time::Instant::now();
        let d = ler(t.as_bytes()).unwrap();
        assert!(d.itens.len() <= TETO);
        assert!(d.ignorados.keys().any(|k| k.contains("fundo")), "{:?}", d.ignorados);
        assert!(inicio.elapsed().as_secs() < 30);
    }

    #[test]
    fn espaco_de_papel_e_entidade_desconhecida_vao_para_a_lista() {
        let mut t = dxf(&[(0, "LINE"), (67, "1"), (10, "0"), (20, "0"), (11, "1"), (21, "0")]);
        t += &dxf(&[(0, "DIMENSION"), (8, "A"), (0, "3DFACE"), (0, "3DFACE"), (0, "CIRCLE"), (210, "1"), (220, "0"), (230, "0"), (40, "1")]);
        let d = ler(&com_entidades(&t)).unwrap();
        assert!(d.itens.is_empty());
        assert_eq!(d.ignorados.get("espaço de papel"), Some(&1));
        assert_eq!(d.ignorados.get("3DFACE"), Some(&2));
        assert_eq!(d.ignorados.get("DIMENSION"), Some(&1));
        assert_eq!(d.ignorados.get("CIRCLE fora do plano"), Some(&1));
    }

    #[test]
    fn normal_para_baixo_espelha_o_centro_do_arco() {
        let t = dxf(&[(0, "ARC"), (10, "10"), (20, "0"), (40, "1"), (50, "0"), (51, "90"), (230, "-1")]);
        let d = ler(&com_entidades(&t)).unwrap();
        let Entidade::Arco { centro, ini, fim, .. } = d.itens[0].entidade else { panic!() };
        assert!((centro[0] + 10.).abs() < 1e-9 && (ini - 90.).abs() < 1e-9 && (fim - 180.).abs() < 1e-9, "{centro:?} {ini} {fim}");
    }

    #[test]
    fn mtext_perde_a_formatacao_e_guarda_o_texto() {
        assert_eq!(texto_de_mtext("{\\fArial|b0;Ol\\U+00E1}\\Pmundo \\{x\\}"), "Olá\nmundo {x}");
    }

    #[test]
    fn ida_e_volta_de_tudo_que_o_gravador_escreve() {
        let i = |e| Item::novo("L", e);
        let d = Desenho {
            unidade: Unidade::Milimetro,
            camadas: vec![Camada::nova("0", 7, "Continuous"), Camada::nova("L", 2, "Continuous")],
            itens: vec![
                i(Entidade::Linha([0., 0.], [1., 2.])),
                i(Entidade::Arco { centro: [1., 1.], raio: 2., ini: 10., fim: 200. }),
                i(Entidade::Circulo { centro: [3., 4.], raio: 5. }),
                i(Entidade::Polilinha { vertices: vec![Vertice { p: [0., 0.], bojo: 0.5 }, Vertice { p: [1., 0.], bojo: 0. }], fechada: false }),
                i(Entidade::Elipse { centro: [0., 0.], maior: [5., 0.], razao: 0.5, t0: 0., t1: 3. }),
                i(Entidade::Spline { grau: 3, controle: vec![[0., 0.], [1., 1.], [2., 0.], [3., 1.]], nos: vec![0., 0., 0., 0., 1., 1., 1., 1.], pesos: vec![], ajuste: vec![], fechada: false }),
                i(Entidade::Ponto([7., 8.])),
                i(Entidade::Texto { pos: [1., 2.], altura: 3., angulo: 30., anexo: Anexo::Direita, texto: "Duas\nlinhas".into() }),
                i(Entidade::Hachura { lacos: vec![vec![[0., 0.], [1., 0.], [1., 1.]]] }),
            ],
            ..Default::default()
        };
        let lido = ler(crate::gravar(&d).as_bytes()).unwrap();
        assert_eq!(lido.versao, "AC1015");
        assert_eq!(lido.unidade, Unidade::Milimetro);
        assert!(lido.ignorados.is_empty(), "{:?}", lido.ignorados);
        assert_eq!(lido.itens.len(), d.itens.len());
        for (a, b) in lido.itens.iter().zip(&d.itens) {
            match (&a.entidade, &b.entidade) {
                (Entidade::Texto { angulo: x, texto: t, anexo: k, .. }, Entidade::Texto { angulo: y, texto: u, anexo: l, .. }) => {
                    assert!((x - y).abs() < 1e-3 && t == u && k == l);
                }
                (x, y) => assert_eq!(x, y),
            }
        }
    }
}
