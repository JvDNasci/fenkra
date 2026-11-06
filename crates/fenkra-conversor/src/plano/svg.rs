// SPDX-License-Identifier: LGPL-2.1-or-later
//! SVG para esboço e de volta. O Y do SVG desce e o do CAD sobe: a leitura espelha, e a escala vem de
//! `width` sobre `viewBox`. Quem grava é o Fenkra põe `data-fenkra-y0`, que diz onde fica o Y zero, e aí
//! as coordenadas voltam iguais; num SVG de fora, o Y zero é a borda de baixo da folha.
//!
//! Só a geometria entra: `rect`, `line`, `circle`, `ellipse`, `polyline`, `polygon` e `path`. Grupo vira
//! camada. Texto e imagem vão para o relatório. Estilo, cor e preenchimento não têm lugar no esboço.

use crate::malha::xml as xml_seguro;
use crate::ErroConv;
use fenkra_dxf::{Camada, Desenho, Entidade, Item, Unidade, Vertice, P2};
use std::f64::consts::{PI, TAU};
use std::fmt::Write as _;

const PROFUNDIDADE: usize = 64;
/// Polegada dividida pelos 96 pixels da norma do CSS.
const MM_POR_PX: f64 = 25.4 / 96.;

/// `etiqueta` raiz é `svg`?
pub(crate) fn parece(etiqueta: Option<&str>) -> bool {
    etiqueta.is_some_and(|e| e == "svg" || e.ends_with(":svg"))
}

/// Matriz 2x3 do SVG: `x' = a x + c y + e`, `y' = b x + d y + f`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Matriz {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Matriz {
    const UM: Matriz = Matriz { a: 1., b: 0., c: 0., d: 1., e: 0., f: 0. };

    fn vezes(&self, o: &Matriz) -> Matriz {
        Matriz { a: self.a * o.a + self.c * o.b, b: self.b * o.a + self.d * o.b, c: self.a * o.c + self.c * o.d, d: self.b * o.c + self.d * o.d, e: self.a * o.e + self.c * o.f + self.e, f: self.b * o.e + self.d * o.f + self.f }
    }

    fn ponto(&self, p: P2) -> P2 {
        [self.a * p[0] + self.c * p[1] + self.e, self.b * p[0] + self.d * p[1] + self.f]
    }

    fn vetor(&self, p: P2) -> P2 {
        [self.a * p[0] + self.c * p[1], self.b * p[0] + self.d * p[1]]
    }

    fn det(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// Fator de escala, se a matriz não deforma: círculo continua círculo.
    fn escala_uniforme(&self) -> Option<f64> {
        let (u, v) = (self.a.hypot(self.b), self.c.hypot(self.d));
        ((u - v).abs() < 1e-9 * u.max(v).max(1e-300) && (self.a * self.c + self.b * self.d).abs() < 1e-9 * u * v).then_some(u)
    }
}

fn numeros(t: &str) -> Vec<f64> {
    let mut saida = Vec::new();
    let b = t.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if !(b[i].is_ascii_digit() || matches!(b[i], b'-' | b'+' | b'.')) {
            i += 1;
            continue;
        }
        let inicio = i;
        let mut ponto = false;
        i += 1;
        if b[inicio] == b'.' {
            ponto = true;
        }
        while i < b.len() {
            match b[i] {
                b'0'..=b'9' => i += 1,
                b'.' if !ponto => {
                    ponto = true;
                    i += 1;
                }
                b'e' | b'E' => {
                    i += 1;
                    if i < b.len() && matches!(b[i], b'-' | b'+') {
                        i += 1;
                    }
                }
                _ => break,
            }
        }
        if let Ok(v) = t[inicio..i].parse() {
            saida.push(v);
        }
    }
    saida
}

fn transformacao(t: &str) -> Matriz {
    let mut m = Matriz::UM;
    for parte in t.split(')') {
        let Some((nome, resto)) = parte.split_once('(') else { continue };
        let v = numeros(resto);
        let n = |i: usize, p: f64| v.get(i).copied().unwrap_or(p);
        let passo = match nome.trim().trim_start_matches(',').trim() {
            "matrix" if v.len() == 6 => Matriz { a: v[0], b: v[1], c: v[2], d: v[3], e: v[4], f: v[5] },
            "translate" => Matriz { e: n(0, 0.), f: n(1, 0.), ..Matriz::UM },
            "scale" => Matriz { a: n(0, 1.), d: n(1, n(0, 1.)), ..Matriz::UM },
            "rotate" => {
                let (s, c) = n(0, 0.).to_radians().sin_cos();
                let giro = Matriz { a: c, b: s, c: -s, d: c, e: 0., f: 0. };
                let (x, y) = (n(1, 0.), n(2, 0.));
                Matriz { e: x, f: y, ..Matriz::UM }.vezes(&giro).vezes(&Matriz { e: -x, f: -y, ..Matriz::UM })
            }
            "skewX" => Matriz { c: n(0, 0.).to_radians().tan(), ..Matriz::UM },
            "skewY" => Matriz { b: n(0, 0.).to_radians().tan(), ..Matriz::UM },
            _ => continue,
        };
        m = m.vezes(&passo);
    }
    m
}

/// Milímetros de um comprimento com unidade. `None` em número puro ou porcentagem.
fn comprimento_mm(t: &str) -> Option<f64> {
    let t = t.trim();
    let fim = t.find(|c: char| c.is_alphabetic() || c == '%').unwrap_or(t.len());
    let v: f64 = t[..fim].trim().parse().ok()?;
    match &t[fim..] {
        "mm" => Some(v),
        "cm" => Some(v * 10.),
        "in" => Some(v * 25.4),
        "pt" => Some(v * 25.4 / 72.),
        "pc" => Some(v * 25.4 / 6.),
        "px" => Some(v * MM_POR_PX),
        _ => None,
    }
}

struct Leitor {
    d: Desenho,
    ignorados: Vec<String>,
}

impl Leitor {
    fn item(&mut self, camada: &str, e: Entidade) {
        if !self.d.camadas.iter().any(|c| c.nome == camada) {
            self.d.camadas.push(Camada::nova(camada, 7, "Continuous"));
        }
        self.d.itens.push(Item::novo(camada, e));
    }

    fn poligono(&mut self, camada: &str, m: &Matriz, pontos: &[P2], fechada: bool) {
        if pontos.len() < 2 {
            return;
        }
        let vertices = pontos.iter().map(|p| Vertice { p: m.ponto(*p), bojo: 0. }).collect();
        self.item(camada, Entidade::Polilinha { vertices, fechada });
    }

    /// Elipse em torno de `c`. Com matriz que não deforma e raios iguais, é um círculo de verdade.
    fn elipse(&mut self, camada: &str, m: &Matriz, c: P2, rx: f64, ry: f64) {
        if let (Some(k), true) = (m.escala_uniforme(), (rx - ry).abs() < 1e-12) {
            return self.item(camada, Entidade::Circulo { centro: m.ponto(c), raio: rx * k });
        }
        if let Some(_k) = m.escala_uniforme() {
            let (maior, razao) = if rx >= ry { (m.vetor([rx, 0.]), ry / rx) } else { (m.vetor([0., ry]), rx / ry) };
            return self.item(camada, Entidade::Elipse { centro: m.ponto(c), maior, razao, t0: 0., t1: TAU });
        }
        let pontos = crate::plano::amostras(72, |t| [c[0] + rx * t.cos(), c[1] + ry * t.sin()]);
        self.poligono(camada, m, &pontos, true);
    }

    /// Arco do comando `A`, pela conversão da norma das pontas para o centro.
    #[allow(clippy::too_many_arguments)]
    fn arco(&mut self, camada: &str, m: &Matriz, p0: P2, rx: f64, ry: f64, giro: f64, grande: bool, varre: bool, p1: P2) {
        let (mut rx, mut ry) = (rx.abs(), ry.abs());
        if rx < 1e-12 || ry < 1e-12 || p0 == p1 {
            return self.item(camada, Entidade::Linha(m.ponto(p0), m.ponto(p1)));
        }
        let (s, c) = giro.to_radians().sin_cos();
        let (dx, dy) = ((p0[0] - p1[0]) / 2., (p0[1] - p1[1]) / 2.);
        let (x, y) = (c * dx + s * dy, -s * dx + c * dy);
        let lambda = x * x / (rx * rx) + y * y / (ry * ry);
        if lambda > 1. {
            rx *= lambda.sqrt();
            ry *= lambda.sqrt();
        }
        let num = (rx * rx * ry * ry - rx * rx * y * y - ry * ry * x * x).max(0.);
        let den = rx * rx * y * y + ry * ry * x * x;
        let fator = (num / den).sqrt() * if grande == varre { -1. } else { 1. };
        let (cx, cy) = (fator * rx * y / ry, -fator * ry * x / rx);
        let centro = [c * cx - s * cy + (p0[0] + p1[0]) / 2., s * cx + c * cy + (p0[1] + p1[1]) / 2.];
        let t0 = ((y - cy) / ry).atan2((x - cx) / rx);
        let t1 = ((-y - cy) / ry).atan2((-x - cx) / rx);
        // o sentido no CAD: quem espelha inverte o varrer da norma
        let anti = varre != (m.det() < 0.);
        match m.escala_uniforme() {
            Some(k) if (rx - ry).abs() < 1e-9 * rx => {
                let (a, b) = (m.ponto(p0), m.ponto(p1));
                let o = m.ponto(centro);
                let angulo = |p: P2| (p[1] - o[1]).atan2(p[0] - o[0]).to_degrees();
                let (ini, fim) = if anti { (angulo(a), angulo(b)) } else { (angulo(b), angulo(a)) };
                self.item(camada, Entidade::Arco { centro: o, raio: rx * k, ini, fim });
            }
            _ => {
                let mut varredura = t1 - t0;
                if varre && varredura < 0. {
                    varredura += TAU;
                } else if !varre && varredura > 0. {
                    varredura -= TAU;
                }
                let n = ((varredura.abs() / PI * 24.).ceil() as usize).max(2);
                let pontos: Vec<P2> = (0..=n)
                    .map(|i| {
                        let t = t0 + varredura * i as f64 / n as f64;
                        let (u, v) = (rx * t.cos(), ry * t.sin());
                        [c * u - s * v + centro[0], s * u + c * v + centro[1]]
                    })
                    .collect();
                self.poligono(camada, m, &pontos, false);
            }
        }
    }

    fn bezier(&mut self, camada: &str, m: &Matriz, controle: &[P2]) {
        let grau = controle.len() - 1;
        let nos: Vec<f64> = std::iter::repeat_n(0., grau + 1).chain(std::iter::repeat_n(1., grau + 1)).collect();
        self.item(camada, Entidade::Spline { grau: grau as u32, controle: controle.iter().map(|p| m.ponto(*p)).collect(), nos, pesos: vec![], ajuste: vec![], fechada: false });
    }

    fn caminho(&mut self, camada: &str, m: &Matriz, d: &str) {
        let b = d.as_bytes();
        let mut i = 0;
        let (mut p, mut inicio, mut controle) = ([0., 0.], [0., 0.], None::<P2>);
        loop {
            let proximo = (i..b.len()).find(|k| b[*k].is_ascii_alphabetic() && !matches!(b[*k], b'e' | b'E'));
            let fim = proximo.map(|k| (k + 1..b.len()).find(|q| b[*q].is_ascii_alphabetic() && !matches!(b[*q], b'e' | b'E')).unwrap_or(b.len()));
            let (Some(k), Some(f)) = (proximo, fim) else { break };
            let comando = b[k];
            let v = numeros(&d[k + 1..f]);
            i = f;
            let rel = comando.is_ascii_lowercase();
            let base = |p: P2| if rel { p } else { [0., 0.] };
            let abs = |q: P2, p: P2| [q[0] + base(p)[0], q[1] + base(p)[1]];
            match comando.to_ascii_uppercase() {
                b'M' => {
                    for (j, q) in v.chunks_exact(2).enumerate() {
                        let alvo = abs([q[0], q[1]], p);
                        if j == 0 {
                            inicio = alvo;
                        } else {
                            self.item(camada, Entidade::Linha(m.ponto(p), m.ponto(alvo)));
                        }
                        p = alvo;
                    }
                }
                b'L' => {
                    for q in v.chunks_exact(2) {
                        let alvo = abs([q[0], q[1]], p);
                        self.item(camada, Entidade::Linha(m.ponto(p), m.ponto(alvo)));
                        p = alvo;
                    }
                }
                b'H' | b'V' => {
                    for q in &v {
                        let mut alvo = p;
                        let eixo = usize::from(comando.eq_ignore_ascii_case(&b'V'));
                        alvo[eixo] = if rel { p[eixo] + q } else { *q };
                        self.item(camada, Entidade::Linha(m.ponto(p), m.ponto(alvo)));
                        p = alvo;
                    }
                }
                b'C' | b'S' | b'Q' | b'T' => {
                    let curto = matches!(comando.to_ascii_uppercase(), b'S' | b'T');
                    let cubica = matches!(comando.to_ascii_uppercase(), b'C' | b'S');
                    let passo = match (cubica, curto) {
                        (true, false) => 6,
                        (true, true) | (false, false) => 4,
                        (false, true) => 2,
                    };
                    for q in v.chunks_exact(passo) {
                        let refletido = controle.map(|c: P2| [2. * p[0] - c[0], 2. * p[1] - c[1]]).unwrap_or(p);
                        let pares: Vec<P2> = q.chunks_exact(2).map(|x| abs([x[0], x[1]], p)).collect();
                        let lista: Vec<P2> = if curto { std::iter::once(refletido).chain(pares).collect() } else { pares };
                        let alvo = *lista.last().unwrap_or(&p);
                        let mut pontos = vec![p];
                        pontos.extend(&lista);
                        self.bezier(camada, m, &pontos);
                        controle = lista.get(lista.len().wrapping_sub(2)).copied();
                        p = alvo;
                    }
                    continue;
                }
                b'A' => {
                    for q in v.chunks_exact(7) {
                        let alvo = abs([q[5], q[6]], p);
                        self.arco(camada, m, p, q[0], q[1], q[2], q[3] != 0., q[4] != 0., alvo);
                        p = alvo;
                    }
                }
                b'Z' => {
                    if p != inicio {
                        self.item(camada, Entidade::Linha(m.ponto(p), m.ponto(inicio)));
                    }
                    p = inicio;
                }
                _ => {}
            }
            controle = None;
        }
    }

    fn elemento(&mut self, e: roxmltree::Node, pai: &Matriz, camada: &str, nivel: usize) {
        if nivel > PROFUNDIDADE {
            return;
        }
        let m = e.attribute("transform").map(|t| pai.vezes(&transformacao(t))).unwrap_or(*pai);
        let n = |nome: &str| e.attribute(nome).map(numeros).and_then(|v| v.first().copied()).unwrap_or(0.);
        let pontos = || -> Vec<P2> { numeros(e.attribute("points").unwrap_or("")).chunks_exact(2).map(|p| [p[0], p[1]]).collect() };
        match e.tag_name().name() {
            "g" | "svg" | "a" | "switch" => {
                let proprio = e.attribute("data-camada").or_else(|| e.attributes().find(|a| a.name() == "label").map(|a| a.value())).or(e.attribute("id"));
                let camada = proprio.filter(|_| e.tag_name().name() == "g").unwrap_or(camada).to_string();
                for f in e.children().filter(|c| c.is_element()) {
                    self.elemento(f, &m, &camada, nivel + 1);
                }
            }
            "line" => self.item(camada, Entidade::Linha(m.ponto([n("x1"), n("y1")]), m.ponto([n("x2"), n("y2")]))),
            "rect" => {
                let (x, y, w, h) = (n("x"), n("y"), n("width"), n("height"));
                if e.attribute("rx").is_some() || e.attribute("ry").is_some() {
                    self.ignorados.push("cantos arredondados".into());
                }
                self.poligono(camada, &m, &[[x, y], [x + w, y], [x + w, y + h], [x, y + h]], true);
            }
            "circle" => self.elipse(camada, &m, [n("cx"), n("cy")], n("r"), n("r")),
            "ellipse" => self.elipse(camada, &m, [n("cx"), n("cy")], n("rx"), n("ry")),
            "polyline" => self.poligono(camada, &m, &pontos(), false),
            "polygon" => self.poligono(camada, &m, &pontos(), true),
            "path" => self.caminho(camada, &m, e.attribute("d").unwrap_or("")),
            "text" | "image" | "use" | "foreignObject" => self.ignorados.push(e.tag_name().name().to_string()),
            _ => {}
        }
    }
}

/// Lê um SVG como desenho em milímetros.
pub(crate) fn ler(b: &[u8]) -> Result<Desenho, ErroConv> {
    let texto = String::from_utf8_lossy(b);
    let opcoes = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(&texto, opcoes).map_err(|e| ErroConv::Leitura(format!("SVG: {e}")))?;
    let raiz = doc.root_element();
    let caixa = raiz.attribute("viewBox").map(numeros).filter(|v| v.len() == 4);
    let largura = raiz.attribute("width").and_then(comprimento_mm);
    // sem tamanho físico declarado, a unidade fica por conta de quem importa
    let mm = match (&caixa, largura) {
        (Some(v), Some(w)) if v[2] > 0. => Some(w / v[2]),
        (None, Some(_)) => Some(MM_POR_PX),
        _ => None,
    };
    let altura = caixa.as_ref().map(|v| v[1] + v[3]).or_else(|| raiz.attribute("height").map(numeros).and_then(|v| v.first().copied())).unwrap_or(0.);
    let y0 = raiz.attribute("data-fenkra-y0").and_then(|v| v.parse().ok()).unwrap_or(altura);
    let k = mm.unwrap_or(1.);
    let base = Matriz { a: k, b: 0., c: 0., d: -k, e: 0., f: y0 * k };
    let mut l = Leitor { d: Desenho { unidade: if mm.is_some() { Unidade::Milimetro } else { Unidade::SemUnidade }, ..Default::default() }, ignorados: vec![] };
    l.elemento(raiz, &base, "", 0);
    for oque in l.ignorados {
        *l.d.ignorados.entry(oque).or_insert(0) += 1;
    }
    Ok(l.d)
}

fn y(p: P2) -> P2 {
    [p[0], -p[1]]
}

fn caixa(d: &Desenho) -> Option<(P2, P2)> {
    let mut pts: Vec<P2> = Vec::new();
    for i in &d.itens {
        match &i.entidade {
            Entidade::Linha(a, b) => pts.extend([*a, *b]),
            Entidade::Circulo { centro: c, raio: r } | Entidade::Arco { centro: c, raio: r, .. } => pts.extend([[c[0] - r, c[1] - r], [c[0] + r, c[1] + r]]),
            Entidade::Polilinha { vertices, .. } => pts.extend(vertices.iter().map(|v| v.p)),
            Entidade::Elipse { centro: c, maior, .. } => {
                let r = maior[0].hypot(maior[1]);
                pts.extend([[c[0] - r, c[1] - r], [c[0] + r, c[1] + r]]);
            }
            Entidade::Spline { ajuste, controle, .. } => pts.extend(ajuste.iter().chain(controle)),
            Entidade::Ponto(p) => pts.push(*p),
            _ => {}
        }
    }
    let primeiro = *pts.first()?;
    Some(pts.iter().fold((primeiro, primeiro), |(a, b), p| ([a[0].min(p[0]), a[1].min(p[1])], [b[0].max(p[0]), b[1].max(p[1])])))
}

/// Spline por pontos em cúbicas de Catmull-Rom: passa por todos os pontos, como a do esboço.
fn caminho_da_spline(p: &[P2]) -> String {
    let (a, n) = (y(p[0]), p.len());
    let mut d = format!("M{} {}", a[0], a[1]);
    for i in 0..n - 1 {
        let q = |k: isize| y(p[(i as isize + k).clamp(0, n as isize - 1) as usize]);
        let (p0, p1, p2, p3) = (q(-1), q(0), q(1), q(2));
        let c1 = [p1[0] + (p2[0] - p0[0]) / 6., p1[1] + (p2[1] - p0[1]) / 6.];
        let c2 = [p2[0] - (p3[0] - p1[0]) / 6., p2[1] - (p3[1] - p1[1]) / 6.];
        let _ = write!(d, " C{} {} {} {} {} {}", c1[0], c1[1], c2[0], c2[1], p2[0], p2[1]);
    }
    d
}

fn traco(s: &mut String, e: &Entidade) {
    match e {
        Entidade::Linha(a, b) => {
            let (a, b) = (y(*a), y(*b));
            let _ = writeln!(s, r#"<line x1="{}" y1="{}" x2="{}" y2="{}"/>"#, a[0], a[1], b[0], b[1]);
        }
        Entidade::Circulo { centro, raio } => {
            let c = y(*centro);
            let _ = writeln!(s, r#"<circle cx="{}" cy="{}" r="{raio}"/>"#, c[0], c[1]);
        }
        Entidade::Arco { centro, raio, ini, fim } => {
            let ponta = |g: f64| y([centro[0] + raio * g.to_radians().cos(), centro[1] + raio * g.to_radians().sin()]);
            let (a, b) = (ponta(*ini), ponta(*fim));
            let vao = (fim - ini).rem_euclid(360.);
            // anti-horário no CAD é anti-horário na tela: varrer 0 na norma
            let _ = writeln!(s, r#"<path d="M{} {} A{raio} {raio} 0 {} 0 {} {}"/>"#, a[0], a[1], u8::from(vao > 180.), b[0], b[1]);
        }
        Entidade::Polilinha { vertices, fechada } => {
            let pontos: Vec<String> = vertices.iter().map(|v| y(v.p)).map(|p| format!("{},{}", p[0], p[1])).collect();
            if vertices.iter().all(|v| v.bojo == 0.) {
                let _ = writeln!(s, r#"<{0} points="{1}"/>"#, if *fechada { "polygon" } else { "polyline" }, pontos.join(" "));
                return;
            }
            let n = vertices.len();
            for i in 0..if *fechada { n } else { n - 1 } {
                let (a, b) = (vertices[i], vertices[(i + 1) % n]);
                match fenkra_dxf::arco_do_bojo(a.p, b.p, a.bojo) {
                    Some((c, r, anti)) => {
                        let angulo = |p: P2| (p[1] - c[1]).atan2(p[0] - c[0]).to_degrees();
                        let (ini, fim) = if anti { (angulo(a.p), angulo(b.p)) } else { (angulo(b.p), angulo(a.p)) };
                        traco(s, &Entidade::Arco { centro: c, raio: r, ini, fim });
                    }
                    None => traco(s, &Entidade::Linha(a.p, b.p)),
                }
            }
        }
        Entidade::Elipse { centro, maior, razao, t0, t1 } => {
            let r = maior[0].hypot(maior[1]);
            let giro = -maior[1].atan2(maior[0]).to_degrees();
            let c = y(*centro);
            if (t1 - t0).abs() >= TAU - 1e-9 {
                let _ = writeln!(s, r#"<ellipse cx="{}" cy="{}" rx="{r}" ry="{}" transform="rotate({giro} {} {})"/>"#, c[0], c[1], r * razao, c[0], c[1]);
                return;
            }
            let menor = [-maior[1] * razao, maior[0] * razao];
            let ponta = |t: f64| y([centro[0] + maior[0] * t.cos() + menor[0] * t.sin(), centro[1] + maior[1] * t.cos() + menor[1] * t.sin()]);
            let (a, b) = (ponta(*t0), ponta(*t1));
            let _ = writeln!(s, r#"<path d="M{} {} A{r} {} {giro} {} 0 {} {}"/>"#, a[0], a[1], r * razao, u8::from((t1 - t0).rem_euclid(TAU) > PI), b[0], b[1]);
        }
        Entidade::Spline { ajuste, .. } if ajuste.len() >= 2 => {
            let _ = writeln!(s, r#"<path d="{}"/>"#, caminho_da_spline(ajuste));
        }
        Entidade::Ponto(p) => {
            let c = y(*p);
            let _ = writeln!(s, r#"<circle cx="{}" cy="{}" r="0.25" data-ponto="1"/>"#, c[0], c[1]);
        }
        _ => {}
    }
}

/// Grava em milímetros, uma camada por grupo. A folha é a caixa do desenho com uma margem.
pub(crate) fn gravar(d: &Desenho) -> String {
    let (a, b) = caixa(d).unwrap_or(([0., 0.], [1., 1.]));
    let margem = 0.05 * (b[0] - a[0]).max(b[1] - a[1]).max(1.);
    let (w, h) = (b[0] - a[0] + 2. * margem, b[1] - a[1] + 2. * margem);
    let mut s = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" width=\"{w}mm\" height=\"{h}mm\" viewBox=\"{} {} {w} {h}\" data-fenkra-y0=\"0\">\n", a[0] - margem, -b[1] - margem);
    let mut ordem: Vec<&str> = Vec::new();
    for i in &d.itens {
        if !ordem.contains(&i.camada.as_str()) {
            ordem.push(&i.camada);
        }
    }
    for camada in ordem {
        let _ = writeln!(s, "<g data-camada=\"{}\" fill=\"none\" stroke=\"black\" stroke-width=\"0.25\">", xml_seguro(camada));
        for i in d.itens.iter().filter(|i| i.camada == camada) {
            traco(&mut s, &i.entidade);
        }
        s.push_str("</g>\n");
    }
    s.push_str("</svg>\n");
    s
}

#[cfg(test)]
mod testes {
    use super::*;

    fn um(t: &str) -> Desenho {
        ler(t.as_bytes()).unwrap()
    }

    #[test]
    fn numeros_colados_como_a_norma_permite() {
        assert_eq!(numeros("10-5.5.25e1,3"), vec![10., -5.5, 2.5, 3.]);
    }

    #[test]
    fn folha_em_mm_espelha_e_escala() {
        let d = um(r#"<svg width="200mm" height="100mm" viewBox="0 0 100 50"><line x1="0" y1="50" x2="10" y2="40"/></svg>"#);
        assert_eq!(d.unidade, Unidade::Milimetro);
        assert_eq!(d.itens[0].entidade, Entidade::Linha([0., 0.], [20., 20.]));
    }

    #[test]
    fn sem_tamanho_fisico_nao_inventa_unidade() {
        assert_eq!(um(r#"<svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="2"/></svg>"#).unidade, Unidade::SemUnidade);
    }

    #[test]
    fn grupo_vira_camada_e_soma_a_transformacao() {
        let d = um(r#"<svg width="10mm" viewBox="0 0 10 10"><g id="Furos" transform="translate(1 2)"><g transform="scale(2)"><circle cx="1" cy="1" r="1"/></g></g></svg>"#);
        assert_eq!(d.itens[0].camada, "Furos");
        assert_eq!(d.itens[0].entidade, Entidade::Circulo { centro: [3., 6.], raio: 2. });
    }

    #[test]
    fn escala_desigual_transforma_circulo_em_contorno() {
        let d = um(r#"<svg width="10mm" viewBox="0 0 10 10"><circle cx="0" cy="0" r="1" transform="scale(2 1)"/></svg>"#);
        assert!(matches!(&d.itens[0].entidade, Entidade::Polilinha { fechada: true, .. }));
    }

    #[test]
    fn caminho_com_arco_na_norma_cai_no_centro_certo() {
        // meio círculo de (0,0) a (10,0), por cima na tela: varrer 1
        let d = um(r#"<svg width="20mm" viewBox="0 0 20 20" data-fenkra-y0="0"><path d="M0 0 A5 5 0 0 1 10 0"/></svg>"#);
        let Entidade::Arco { centro, raio, ini, fim } = d.itens[0].entidade else { panic!("{:?}", d.itens) };
        assert_eq!(raio, 5.);
        assert!((centro[0] - 5.).abs() < 1e-9 && centro[1].abs() < 1e-9, "{centro:?}");
        // na tela o arco passa acima, com Y da tela menor: no CAD, Y positivo
        let meio = (ini + (fim - ini).rem_euclid(360.) / 2.).to_radians();
        assert!(meio.sin() > 0.9, "{ini} {fim}");
    }

    #[test]
    fn comandos_relativos_e_fechamento() {
        let d = um(r#"<svg width="20mm" viewBox="0 0 20 20" data-fenkra-y0="0"><path d="m1 1 h2 v2 l-2 0 z"/></svg>"#);
        assert_eq!(d.itens.len(), 4);
        assert_eq!(d.itens[3].entidade, Entidade::Linha([1., -3.], [1., -1.]));
    }

    #[test]
    fn curvas_de_bezier_viram_spline_de_grau_certo() {
        let d = um(r#"<svg width="20mm" viewBox="0 0 20 20"><path d="M0 0 C1 1 2 1 3 0 S5 -1 6 0 Q7 1 8 0 T10 0"/></svg>"#);
        let graus: Vec<u32> = d.itens.iter().filter_map(|i| if let Entidade::Spline { grau, .. } = i.entidade { Some(grau) } else { None }).collect();
        assert_eq!(graus, vec![3, 3, 2, 2]);
    }

    #[test]
    fn texto_e_imagem_vao_para_o_relatorio() {
        let d = um(r#"<svg width="20mm" viewBox="0 0 20 20"><text>oi</text><text>tchau</text><line x2="1"/></svg>"#);
        assert_eq!(d.ignorados.get("text"), Some(&2));
    }

    #[test]
    fn gravar_e_ler_mantem_as_coordenadas_negativas() {
        let original = Desenho { itens: vec![Item::novo("Contorno", Entidade::Circulo { centro: [-5., -7.], raio: 2. }), Item::novo("Contorno", Entidade::Arco { centro: [3., 4.], raio: 2., ini: 30., fim: 200. })], ..Default::default() };
        let volta = ler(gravar(&original).as_bytes()).unwrap();
        assert_eq!(volta.unidade, Unidade::Milimetro);
        assert_eq!(volta.itens[0].camada, "Contorno");
        let Entidade::Circulo { centro, raio } = volta.itens[0].entidade else { panic!() };
        assert!((centro[0] + 5.).abs() < 1e-9 && (centro[1] + 7.).abs() < 1e-9 && (raio - 2.).abs() < 1e-9);
        let Entidade::Arco { centro, ini, fim, .. } = volta.itens[1].entidade else { panic!() };
        assert!((centro[0] - 3.).abs() < 1e-9 && (centro[1] - 4.).abs() < 1e-9, "{centro:?}");
        assert!((ini - 30.).abs() < 1e-9 && (fim.rem_euclid(360.) - 200.).abs() < 1e-9, "{ini} {fim}");
    }
}
