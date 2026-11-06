// SPDX-License-Identifier: LGPL-2.1-or-later
//! Contornos planos dos cortes de dobra de tubo (B334) e gabarito de marcação em escala 1:1 (B329).
//!
//! Como funciona: o tubo reto é desenrolado como quatro tiras lado a lado (placa externa, lateral,
//! parede interna, outra lateral), cada uma com a largura nominal da face. O eixo X corre ao longo
//! do tubo, medido da ponta da primeira perna (o mesmo `desenvolvido` da lista de corte); o eixo Y
//! corre pelo perímetro. Em cima disso entram os contornos de corte de cada modo:
//!
//! * V comum e V múltiplo: um hexágono por rasgo (triângulo na lateral, fenda na parede interna,
//!   triângulo na outra lateral), com a placa externa como dobradiça;
//! * canto com gabarito: a "andorinha" de cada lateral (placa liberada, arcos dos setores e
//!   bissetrizes) mais a abertura da parede interna.
//!
//! Tudo vem da derivação de `docs/pesquisa/dobra-de-tubos.md` (seções 7 e 8), **sem protótipo**: os
//! desenhos levam o aviso "a validar em peça real". O desenho é próprio (régua de tira e contorno
//! de lateral); nada foi copiado de produto algum.

use crate::st_dobra::{ResultadoDobra, TuboDobrado, AVISO_DERIVADA};
use fenkra_core::BendMode;
use std::f64::consts::PI;

/// Camada de um traço (vira camada de DXF e cor de SVG).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Camada {
    /// o que a máquina corta
    Corte,
    /// o que só se risca ou se confere (posições, ticks, eixos)
    Marcacao,
    /// linhas de dobra e de zona de dobra
    Dobra,
    /// contorno de referência do tubo (não se corta)
    Contorno,
    /// textos e cotas
    Cota,
}

impl Camada {
    pub fn nome(self) -> &'static str {
        match self {
            Camada::Corte => "CORTE",
            Camada::Marcacao => "MARCACAO",
            Camada::Dobra => "DOBRA",
            Camada::Contorno => "CONTORNO",
            Camada::Cota => "COTA",
        }
    }
    /// Índice de cor do DXF (1 vermelho, 5 azul, 3 verde, 8 cinza, 7 branco/preto).
    pub fn cor_dxf(self) -> u8 {
        match self {
            Camada::Corte => 1,
            Camada::Marcacao => 5,
            Camada::Dobra => 3,
            Camada::Contorno => 8,
            Camada::Cota => 7,
        }
    }
    pub fn cor_svg(self) -> &'static str {
        match self {
            Camada::Corte => "#d40000",
            Camada::Marcacao => "#0050c8",
            Camada::Dobra => "#008c3c",
            Camada::Contorno => "#888888",
            Camada::Cota => "#000000",
        }
    }
}

/// Polilinha de uma camada. Arcos já vêm discretizados (passo de 2°).
#[derive(Clone, Debug, PartialEq)]
pub struct Traco {
    pub camada: Camada,
    pub pts: Vec<[f64; 2]>,
    pub fechado: bool,
    pub tracejado: bool,
}

/// Texto numa posição (canto inferior esquerdo), altura em mm.
#[derive(Clone, Debug, PartialEq)]
pub struct Rotulo {
    pub camada: Camada,
    pub pos: [f64; 2],
    pub altura: f64,
    pub texto: String,
}

/// Desenho 2D em milímetros, Y para cima.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Desenho {
    pub titulo: String,
    pub tracos: Vec<Traco>,
    pub rotulos: Vec<Rotulo>,
}

impl Desenho {
    pub fn linha(&mut self, camada: Camada, a: [f64; 2], b: [f64; 2], tracejado: bool) {
        self.tracos.push(Traco { camada, pts: vec![a, b], fechado: false, tracejado });
    }
    pub fn poli(&mut self, camada: Camada, pts: Vec<[f64; 2]>, fechado: bool) {
        self.tracos.push(Traco { camada, pts, fechado, tracejado: false });
    }
    pub fn texto(&mut self, camada: Camada, pos: [f64; 2], altura: f64, texto: impl Into<String>) {
        self.rotulos.push(Rotulo { camada, pos, altura, texto: texto.into() });
    }
    /// Retângulo.
    pub fn retangulo(&mut self, camada: Camada, x0: f64, y0: f64, x1: f64, y1: f64) {
        self.poli(camada, vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]], true);
    }
    /// Caixa envolvente (mínimo, máximo) de tudo, com a largura estimada dos textos.
    pub fn caixa(&self) -> ([f64; 2], [f64; 2]) {
        let mut lo = [f64::MAX; 2];
        let mut hi = [f64::MIN; 2];
        let mut ponto = |p: [f64; 2]| {
            for i in 0..2 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        };
        for t in &self.tracos {
            t.pts.iter().for_each(|p| ponto(*p));
        }
        for r in &self.rotulos {
            ponto(r.pos);
            ponto([r.pos[0] + largura_texto(&r.texto, r.altura), r.pos[1] + r.altura]);
        }
        if lo[0] > hi[0] {
            return ([0.; 2], [0.; 2]);
        }
        (lo, hi)
    }
    /// Move tudo.
    pub fn deslocar(&mut self, dx: f64, dy: f64) {
        for t in &mut self.tracos {
            for p in &mut t.pts {
                p[0] += dx;
                p[1] += dy;
            }
        }
        for r in &mut self.rotulos {
            r.pos[0] += dx;
            r.pos[1] += dy;
        }
    }
    /// Junta outro desenho a este.
    pub fn juntar(&mut self, outro: Desenho) {
        self.tracos.extend(outro.tracos);
        self.rotulos.extend(outro.rotulos);
    }
    /// Comprimento total de corte (soma dos traços da camada `Corte`).
    pub fn comprimento_de_corte(&self) -> f64 {
        self.tracos
            .iter()
            .filter(|t| t.camada == Camada::Corte)
            .map(|t| {
                let mut s: f64 = t.pts.windows(2).map(|w| dist(w[0], w[1])).sum();
                if t.fechado && t.pts.len() > 2 {
                    s += dist(t.pts[t.pts.len() - 1], t.pts[0]);
                }
                s
            })
            .sum()
    }
}

/// Largura aproximada de um texto de fonte sem serifa (para a caixa e o alinhamento).
pub fn largura_texto(t: &str, altura: f64) -> f64 {
    t.chars().count() as f64 * altura * 0.55
}

fn fmt_ang(v: f64) -> String {
    let s = format!("{v:.1}");
    s.strip_suffix(".0").unwrap_or(&s).to_string()
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Pontos de um arco de centro `c` e raio `r`, do ângulo `a0` ao `a1` (radianos), passo de 2°.
fn arco(c: [f64; 2], r: f64, a0: f64, a1: f64) -> Vec<[f64; 2]> {
    let n = (((a1 - a0).abs().to_degrees() / 2.).ceil() as usize).max(1);
    (0..=n).map(|i| {
        let a = a0 + (a1 - a0) * i as f64 / n as f64;
        [c[0] + r * a.cos(), c[1] + r * a.sin()]
    }).collect()
}

// ------------------------------------------------------------------------- geometria dos cortes

/// Dados do canto com gabarito, no quadro da lateral: X ao longo da perna 1 contado do início do
/// arco; Y a partir da face externa da placa.
#[derive(Clone, Copy, Debug)]
pub struct Andorinha {
    /// altura da lateral (dimensão no plano da dobra), parede e raio externo
    pub h: f64,
    pub t: f64,
    pub ro: f64,
    /// comprimento plano da placa
    pub placa: f64,
    /// metade do ângulo da dobra (radianos)
    pub meio: f64,
    /// recuo angular do fim do arco por causa da folga de raiz
    pub recuo: f64,
}

impl Andorinha {
    pub fn de(r: &ResultadoDobra) -> Option<Andorinha> {
        let placa = r.placa?;
        let rf = r.raio - r.parede;
        let recuo = if r.folga_raiz > 0. && rf > 0. { (r.folga_raiz / (2. * rf)).clamp(0., 1.).asin() } else { 0. };
        Some(Andorinha { h: r.h_plano, t: r.parede, ro: r.raio, placa, meio: r.theta.to_radians() / 2., recuo })
    }
    /// Metade do ângulo do setor, já com o recuo da folga de raiz.
    pub fn meio_util(&self) -> f64 {
        self.meio - self.recuo
    }
    pub fn rf(&self) -> f64 {
        self.ro - self.t
    }
    /// Posição X em que a bissetriz cruza a face interna (Ix).
    pub fn ix(&self) -> f64 {
        crate::st_dobra::ix_do_canto(self.ro, self.h, self.t, self.meio, if self.recuo > 0. { 2. * self.rf() * self.recuo.sin() } else { 0. })
    }
    /// Fim do arco do setor A.
    pub fn fim_do_arco(&self) -> [f64; 2] {
        [self.rf() * self.meio_util().sin(), self.ro - self.rf() * self.meio_util().cos()]
    }
    /// Cadeia aberta da lateral, de (ix, H) pela bissetriz A, arco A, placa liberada, arco B e
    /// bissetriz B até (ℓ − ix, H). A aresta do alto (y = H) fica de fora: é a parede interna.
    pub fn cadeia(&self) -> Vec<[f64; 2]> {
        let (rf, ro, l, t) = (self.rf(), self.ro, self.placa, self.t);
        let mut p = vec![[self.ix(), self.h]];
        // arco A, do fim para o início (ângulo de meio a 0)
        for q in arco([0., ro], rf, -PI / 2. + self.meio_util(), -PI / 2.) {
            p.push(q);
        }
        p.push([l, t]);
        // arco B, do início ao fim, espelhado em torno de x = ℓ/2
        for q in arco([l, ro], rf, -PI / 2., -PI / 2. - self.meio_util()).into_iter().skip(1) {
            p.push(q);
        }
        p.push([l - self.ix(), self.h]);
        p
    }
    /// Contorno fechado de uma lateral isolada (a andorinha de 7.3).
    pub fn contorno_fechado(&self) -> Vec<[f64; 2]> {
        self.cadeia()
    }
}

/// Os rasgos em V de uma dobra: posição do centro de cada V ao longo do tubo e abertura.
pub fn posicoes_dos_v(r: &ResultadoDobra, inicio: f64, fim: f64) -> Vec<f64> {
    let n = r.rasgos.max(1);
    if n == 1 {
        vec![(inicio + fim) / 2.]
    } else {
        let p = r.passo_v.unwrap_or(0.);
        (0..n).map(|k| inicio + k as f64 * p).collect()
    }
}

/// Meio ângulo de cada V.
pub fn meio_do_v(r: &ResultadoDobra) -> f64 {
    r.theta.to_radians() / (2. * r.rasgos.max(1) as f64)
}

/// Tem corte plano (rasgo ou canto com gabarito)?
pub fn tem_corte(r: &ResultadoDobra) -> bool {
    r.erros.is_empty() && matches!(r.modo, BendMode::VComum | BendMode::CorteDobradeira | BendMode::VComRaio)
}

// ------------------------------------------------------------------------- planificado do tubo

/// Faixas do tubo desenrolado, em Y: (placa, lateral 1, parede interna, lateral 2).
#[derive(Clone, Copy, Debug)]
pub struct Tiras {
    pub b: f64,
    pub h: f64,
}

impl Tiras {
    pub fn perimetro(&self) -> f64 {
        2. * (self.b + self.h)
    }
    /// Y da lateral 1 para uma posição local (0 = lado da placa, H = lado da parede interna).
    pub fn lateral1(&self, y: f64) -> f64 {
        self.b + y
    }
    pub fn lateral2(&self, y: f64) -> f64 {
        2. * self.b + 2. * self.h - y
    }
    pub fn inicio_interna(&self) -> f64 {
        self.b + self.h
    }
    pub fn fim_interna(&self) -> f64 {
        2. * self.b + self.h
    }
}

/// Contorno fechado de um rasgo em V: lateral 1, fenda da parede interna e lateral 2.
fn hexagono(tiras: Tiras, xc: f64, meia_abertura: f64, t: f64) -> Vec<[f64; 2]> {
    let (b, h) = (tiras.b, tiras.h);
    vec![
        [xc, tiras.lateral1(t)],
        [xc + meia_abertura, b + h],
        [xc + meia_abertura, 2. * b + h],
        [xc, tiras.lateral2(t)],
        [xc - meia_abertura, 2. * b + h],
        [xc - meia_abertura, b + h],
    ]
}

/// Contorno fechado único do canto com gabarito: laterais, placa liberada e abertura da parede
/// interna, na posição `x0` do tubo.
fn contorno_do_canto(tiras: Tiras, a: &Andorinha, x0: f64) -> Vec<[f64; 2]> {
    let cadeia = a.cadeia();
    let mut p: Vec<[f64; 2]> = cadeia.iter().map(|q| [x0 + q[0], tiras.lateral1(q[1])]).collect();
    // desce pela fenda da parede interna, volta pela lateral 2 e fecha
    p.extend(cadeia.iter().rev().map(|q| [x0 + q[0], tiras.lateral2(q[1])]));
    p
}

/// Planificado de um tubo dobrado: contornos de corte e marcas, em escala real.
pub fn planificado(t: &TuboDobrado) -> Result<Desenho, String> {
    let primeira = t.dobras.iter().map(|d| &d.resultado).find(|r| r.erros.is_empty() && r.h_plano > 0.).ok_or("o tubo não tem dobra calculada")?;
    let tiras = Tiras { b: primeira.b_fora, h: primeira.h_plano };
    let comp = t.desenvolvido;
    let mut d = Desenho { titulo: format!("Planificado · {}", t.designacao), ..Default::default() };
    // referência: o tubo desenrolado e as arestas entre as faces
    d.retangulo(Camada::Contorno, 0., 0., comp, tiras.perimetro());
    for y in [tiras.b, tiras.inicio_interna(), tiras.fim_interna()] {
        d.linha(Camada::Contorno, [0., y], [comp, y], true);
    }
    for (nome, y) in [("placa externa", tiras.b / 2.), ("lateral", tiras.b + tiras.h / 2.), ("parede interna", tiras.inicio_interna() + tiras.b / 2.), ("lateral", tiras.fim_interna() + tiras.h / 2.)] {
        d.texto(Camada::Cota, [2., y - 1.], 2.5, nome);
    }
    let mut ha_corte = false;
    for (k, db) in t.dobras.iter().enumerate() {
        let r = &db.resultado;
        if !r.erros.is_empty() {
            continue;
        }
        if (r.h_plano - tiras.h).abs() > 1e-6 || (r.b_fora - tiras.b).abs() > 1e-6 {
            return Err("as dobras do tubo estão em planos com a seção virada: o planificado ainda não desenrola esse caso".into());
        }
        // linhas da zona de dobra, na placa
        for x in [db.inicio, db.fim] {
            d.linha(Camada::Dobra, [x, 0.], [x, tiras.b], true);
        }
        match r.modo {
            BendMode::VComum | BendMode::CorteDobradeira => {
                let (w, th) = (r.abertura_v.unwrap_or(0.), r.parede);
                let meia = w / 2.;
                for (j, xc) in posicoes_dos_v(r, db.inicio, db.fim).into_iter().enumerate() {
                    d.poli(Camada::Corte, hexagono(tiras, xc, meia, th), true);
                    d.linha(Camada::Marcacao, [xc, 0.], [xc, tiras.b], true);
                    d.texto(Camada::Cota, [xc + 1., tiras.perimetro() + 1.], 2.5, format!("V{} x={:.2}", j + 1, xc));
                    ha_corte = true;
                }
            }
            BendMode::VComRaio => {
                let Some(a) = Andorinha::de(r) else { continue };
                let x0 = db.inicio;
                d.poli(Camada::Corte, contorno_do_canto(tiras, &a, x0), true);
                d.linha(Camada::Marcacao, [x0 + a.placa / 2., 0.], [x0 + a.placa / 2., tiras.b], true);
                d.texto(Camada::Cota, [x0, tiras.perimetro() + 1.], 2.5, format!("canto {} · placa {:.2} · x={:.2}", k + 1, a.placa, x0));
                ha_corte = true;
            }
            _ => {}
        }
    }
    d.texto(Camada::Cota, [0., -5.], 2.5, format!("Tubo desenrolado: {:.2} x {:.2} mm (perímetro nominal da face externa) · {}", comp, tiras.perimetro(), AVISO_DERIVADA));
    if !ha_corte {
        return Err("nenhuma dobra do tubo tem corte plano (V comum, V múltiplo ou canto com gabarito)".into());
    }
    Ok(d)
}

// ------------------------------------------------------------------------- gabarito 1:1

/// Barra de conferência da escala de impressão: 100 mm com tiques de 10 em 10.
fn barra_de_escala(d: &mut Desenho, x: f64, y: f64) {
    d.linha(Camada::Cota, [x, y], [x + 100., y], false);
    for i in 0..=10 {
        let xi = x + i as f64 * 10.;
        d.linha(Camada::Cota, [xi, y], [xi, y + if i % 5 == 0 { 3. } else { 2. }], false);
    }
    d.texto(Camada::Cota, [x, y + 4.], 3., "Barra de conferência: 100 mm (meça com régua; se não der 100, a impressão saiu em escala errada)");
}

/// Régua ao longo de X, de `x0` a `x1`, tiques de 1 mm (curtos), 5 mm (médios) e 10 mm (com número).
fn regua(d: &mut Desenho, x0: f64, x1: f64, y: f64, sentido: f64) {
    d.linha(Camada::Marcacao, [x0, y], [x1, y], false);
    let mut m = x0.ceil() as i64;
    while (m as f64) <= x1 {
        let x = m as f64;
        let c = if m % 10 == 0 { 3. } else if m % 5 == 0 { 2. } else { 1. };
        d.linha(Camada::Marcacao, [x, y], [x, y + sentido * c], false);
        if m % 10 == 0 {
            d.texto(Camada::Marcacao, [x - 1.5, y + sentido * (c + if sentido > 0. { 0.8 } else { 3.2 })], 2.2, format!("{m}"));
        }
        m += 1;
    }
}

/// Faixa de marcação de uma dobra em V: a lateral do tubo com os triângulos em escala 1:1.
fn faixa_v(db: &crate::st_dobra::DobraDoTubo, n_dobra: usize) -> Desenho {
    let r = &db.resultado;
    let (h, t, w, b) = (r.h_plano, r.parede, r.abertura_v.unwrap_or(0.), r.b_fora);
    let xs = posicoes_dos_v(r, db.inicio, db.fim);
    let x0 = (xs[0] - w / 2. - 12.).floor().max(0.);
    let x1 = (xs[xs.len() - 1] + w / 2. + 12.).ceil();
    let mut d = Desenho::default();
    d.texto(Camada::Cota, [x0, h + 14.], 3.5, format!("Dobra {} · {} · {}° · {} rasgo(s)", n_dobra, crate::st_dobra::nome_modo(r.modo), fmt_ang(r.theta), xs.len()));
    d.texto(Camada::Cota, [x0, h + 9.], 2.5, "Tira para a LATERAL do tubo: coloque a margem de baixo no lado da placa externa (a dobradiça).");
    d.texto(Camada::Cota, [x0, h + 5.5], 2.5, format!("Posições medidas da ponta da primeira perna. V: abertura {:.2} mm, lado {:.2} mm, vértice a {:.2} mm da face externa.", w, r.lado_v.unwrap_or(0.), t));
    d.retangulo(Camada::Contorno, x0, 0., x1, h);
    d.linha(Camada::Marcacao, [x0, t], [x1, t], true);
    for (j, &xc) in xs.iter().enumerate() {
        d.poli(Camada::Corte, vec![[xc - w / 2., h], [xc, t], [xc + w / 2., h]], false);
        d.linha(Camada::Marcacao, [xc, 0.], [xc, h], true);
        d.texto(Camada::Cota, [xc - 6., h + 1.], 2.5, format!("V{} {:.2}", j + 1, xc));
    }
    regua(&mut d, x0, x1, 0., -1.);
    // a fenda correspondente na parede interna (largura w), conferência por sobreposição
    let y_i = -14. - b;
    d.texto(Camada::Cota, [x0, y_i + b + 1.], 2.5, "Tira para a PAREDE INTERNA (a oposta à dobradiça): as fendas são de lado a lado.");
    d.retangulo(Camada::Contorno, x0, y_i, x1, y_i + b);
    for &xc in &xs {
        d.retangulo(Camada::Corte, xc - w / 2., y_i, xc + w / 2., y_i + b);
    }
    d
}

/// Faixa de marcação do canto com gabarito: contorno da lateral em escala 1:1, régua da placa,
/// arcos de raio Ro nas pontas e a bissetriz.
fn faixa_canto(db: &crate::st_dobra::DobraDoTubo, n_dobra: usize) -> Option<Desenho> {
    let r = &db.resultado;
    let a = Andorinha::de(r)?;
    let (h, t, ro, l) = (a.h, a.t, a.ro, a.placa);
    let rf = a.rf();
    let mut d = Desenho::default();
    let x0 = -12.;
    let x1 = l + 12.;
    d.texto(Camada::Cota, [x0, h + 18.], 3.5, format!("Dobra {} · canto com gabarito · {}° · lateral {:.2} mm", n_dobra, fmt_ang(r.theta), h));
    d.texto(Camada::Cota, [x0, h + 13.], 2.5, "Contorno da LATERAL (use nas duas). Y = 0 é a face externa da placa; X = 0 é onde o arco da placa começa.");
    d.texto(Camada::Cota, [x0, h + 9.5], 2.5, format!("Placa plana {:.2} mm · raio externo {:.2} mm · raio dos setores {:.2} mm · abertura da parede interna {:.2} mm", l, ro, rf, l - 2. * a.ix()));
    d.texto(Camada::Cota, [x0, h + 6.], 2.5, format!("Inicio do arco na ponta da perna 1 fica a {:.2} mm da ponta do tubo.", db.inicio));
    d.retangulo(Camada::Contorno, x0, 0., x1, h);
    d.linha(Camada::Marcacao, [x0, t], [x1, t], true);
    d.poli(Camada::Corte, a.contorno_fechado(), true);
    // eixos de início e fim do arco
    d.linha(Camada::Dobra, [0., 0.], [0., h], true);
    d.linha(Camada::Dobra, [l, 0.], [l, h], true);
    // arcos de raio Ro riscados nas pontas (centro no alto de cada extremo)
    d.poli(Camada::Marcacao, arco([0., ro], ro, -PI / 2., -PI / 2. + a.meio), false);
    d.poli(Camada::Marcacao, arco([l, ro], ro, -PI / 2. - a.meio, -PI / 2.), false);
    // V da bissetriz (linhas dos setores até as faces)
    let fa = a.fim_do_arco();
    d.linha(Camada::Marcacao, [0., ro], fa, true);
    d.linha(Camada::Marcacao, [l, ro], [l - fa[0], fa[1]], true);
    d.texto(Camada::Cota, [l / 2. - 10., h - 4.], 2.5, format!("folga plana {:.2} mm", r.folga_plana.unwrap_or(0.)));
    regua(&mut d, 0., l, 0., -1.);
    d.texto(Camada::Cota, [0., -9.], 2.5, format!("Régua da placa: 0 a {:.2} mm", l));
    Some(d)
}

/// Gabarito de marcação 1:1 de todas as dobras com corte de um tubo.
pub fn gabarito(t: &TuboDobrado) -> Result<Desenho, String> {
    let mut g = Desenho { titulo: format!("Gabarito · {}", t.designacao), ..Default::default() };
    let mut y = 0.;
    let mut qtd = 0;
    for (k, db) in t.dobras.iter().enumerate() {
        if !tem_corte(&db.resultado) {
            continue;
        }
        let faixa = match db.resultado.modo {
            BendMode::VComRaio => faixa_canto(db, k + 1),
            _ => Some(faixa_v(db, k + 1)),
        };
        let Some(mut f) = faixa else { continue };
        let (lo, hi) = f.caixa();
        f.deslocar(10. - lo[0], y - lo[1]);
        g.juntar(f);
        y += hi[1] - lo[1] + 14.;
        qtd += 1;
    }
    if qtd == 0 {
        return Err("nenhuma dobra do tubo tem corte para marcar (V comum, V múltiplo ou canto com gabarito)".into());
    }
    barra_de_escala(&mut g, 10., y + 4.);
    g.texto(Camada::Cota, [10., y + 14.], 4.5, format!("Gabarito de marcação · {} · {}", t.designacao, t.material));
    g.texto(Camada::Cota, [10., y + 9.], 2.8, format!("Imprima a 100% (tamanho real), sem 'ajustar à página'. {}.", AVISO_DERIVADA));
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::st_dobra::{calcular, DobraDoTubo, EntradaDobra};
    use fenkra_core::SectionKind;

    fn entrada(modo: BendMode, raio: f64, rasgos: usize) -> EntradaDobra {
        EntradaDobra { tipo: SectionKind::TuboQuadrado, dims: vec![30., 1.2], theta: 90., modo, raio, regra: None, k: None, retorno: None, folga_raiz: None, rasgos, gomos: 1, plano: 0 }
    }

    fn tubo(e: &EntradaDobra, pernas: [f64; 2]) -> TuboDobrado {
        let r = calcular(e);
        let inicio = pernas[0] - r.tangente;
        let fim = inicio + r.arco;
        TuboDobrado {
            feature: fenkra_core::Id(1),
            linhas: vec![0, 1],
            designacao: "Tubo quadrado 30x1,2".into(),
            material: "Aço".into(),
            pernas: pernas.to_vec(),
            dobras: vec![DobraDoTubo { linhas: (0, 1), ponto: [0.; 3], resultado: r.clone(), inicio, fim }],
            desenvolvido: pernas[0] + pernas[1] + r.ajuste_corte,
            soma_pernas: pernas[0] + pernas[1],
            avisos: vec![],
            erros: vec![],
            fundido: true,
        }
    }

    fn area(p: &[[f64; 2]]) -> f64 {
        let mut s = 0.;
        for i in 0..p.len() {
            let (a, b) = (p[i], p[(i + 1) % p.len()]);
            s += a[0] * b[1] - b[0] * a[1];
        }
        s.abs() / 2.
    }

    #[test]
    fn andorinha_do_caso_de_aceite() {
        // 30x30x1,2 a 90 graus, Ro = 30: placa 46,17, fim do arco em (20,36; 9,64)
        let r = calcular(&entrada(BendMode::VComRaio, 30., 1));
        let a = Andorinha::de(&r).unwrap();
        assert!((a.placa - 46.17).abs() < 0.01);
        let fa = a.fim_do_arco();
        assert!((fa[0] - 20.36).abs() < 0.01 && (fa[1] - 9.64).abs() < 0.01, "{fa:?}");
        // Ro = H: a bissetriz sai do canto (Ix = 0) e o contorno cobre x de 0 a ℓ
        assert!(a.ix().abs() < 1e-9);
        let c = a.contorno_fechado();
        let (xmin, xmax) = c.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p[0]), hi.max(p[0])));
        assert!(xmin.abs() < 1e-9 && (xmax - 46.17).abs() < 0.01);
        // área do resíduo = retângulo ℓ·(H−t) menos os dois setores de raio rf e ângulo h
        let esperado = 46.17 * 28.8 - 2. * 0.5 * 28.8 * 28.8 * PI / 4.;
        assert!((area(&c) - esperado).abs() < 1.0, "{} contra {esperado}", area(&c));
        // nenhum ponto do contorno sai da faixa da lateral
        assert!(c.iter().all(|p| p[1] >= 1.2 - 1e-9 && p[1] <= 30. + 1e-9));
    }

    #[test]
    fn andorinha_com_raio_diferente_da_altura() {
        // Ro = 45: Ix = 15, a abertura da parede interna é ℓ − 2·Ix (pesquisa: 39,7)
        let r = calcular(&entrada(BendMode::VComRaio, 45., 1));
        let a = Andorinha::de(&r).unwrap();
        assert!((a.ix() - 15.).abs() < 1e-9);
        assert!((a.placa - 2. * a.ix() - 39.74).abs() < 0.1, "{}", a.placa - 2. * a.ix());
        // Ro = 15: Ix negativo, a abertura cresce
        let r = calcular(&entrada(BendMode::VComRaio, 15., 1));
        let a = Andorinha::de(&r).unwrap();
        assert!((a.placa - 2. * a.ix() - 52.6).abs() < 0.1);
    }

    #[test]
    fn folga_de_raiz_recua_as_bissetrizes() {
        let mut e = entrada(BendMode::VComRaio, 30., 1);
        e.folga_raiz = Some(2.);
        let r = calcular(&e);
        let a = Andorinha::de(&r).unwrap();
        let sem = Andorinha::de(&calcular(&entrada(BendMode::VComRaio, 30., 1))).unwrap();
        assert!(a.recuo > 0. && sem.recuo == 0. && a.meio_util() < sem.meio_util());
        // folga plana com e = 2: 6,88 (pesquisa)
        assert!((r.folga_plana.unwrap() - 6.88).abs() < 0.01);
        // o fim do arco A e o espelho do B ficam separados pela folga plana
        let c = a.cadeia();
        let n = c.len();
        assert!(c[n - 1][0] > c[0][0]);
    }

    #[test]
    fn planificado_do_canto_tem_um_contorno_e_a_marca_da_placa() {
        let t = tubo(&entrada(BendMode::VComRaio, 30., 1), [200., 150.]);
        let d = planificado(&t).unwrap();
        let cortes: Vec<&Traco> = d.tracos.iter().filter(|x| x.camada == Camada::Corte).collect();
        assert_eq!(cortes.len(), 1);
        assert!(cortes[0].fechado);
        // o desenho cabe no tubo desenrolado: perímetro 120, comprimento = desenvolvido
        let (lo, hi) = d.caixa();
        assert!(lo[1] <= -4. && hi[1] > 120.);
        assert!(cortes[0].pts.iter().all(|p| p[0] >= 0. && p[0] <= t.desenvolvido && p[1] >= 0. && p[1] <= 120.));
        // as duas laterais e a fenda juntas cortam muito mais que uma lateral
        assert!(d.comprimento_de_corte() > 200.);
        // dobra: zona de dobra marcada nas duas pontas da placa
        assert_eq!(d.tracos.iter().filter(|x| x.camada == Camada::Dobra).count(), 2);
    }

    #[test]
    fn planificado_do_v_comum_e_do_v_multiplo() {
        // V comum: hexágono com abertura 57,6 e vértice a 1,2 da placa
        let mut e = entrada(BendMode::VComum, 1.8, 1);
        e.k = Some(0.33);
        let t = tubo(&e, [200., 150.]);
        let d = planificado(&t).unwrap();
        let hex = d.tracos.iter().find(|x| x.camada == Camada::Corte).unwrap();
        assert_eq!(hex.pts.len(), 6);
        let (xmin, xmax) = hex.pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p[0]), b.max(p[0])));
        assert!((xmax - xmin - 57.6).abs() < 0.01);
        // V múltiplo n = 6, Rp = 45: seis hexágonos, passo 11,75
        let t = tubo(&entrada(BendMode::VComum, 45., 6), [200., 150.]);
        let d = planificado(&t).unwrap();
        let hexs: Vec<&Traco> = d.tracos.iter().filter(|x| x.camada == Camada::Corte).collect();
        assert_eq!(hexs.len(), 6);
        let cx = |h: &Traco| h.pts[0][0];
        assert!((cx(hexs[1]) - cx(hexs[0]) - 11.75).abs() < 0.01);
        // dobradeira sem corte: não há o que desenrolar
        let t = tubo(&entrada(BendMode::Dobradeira, 60., 1), [200., 150.]);
        assert!(planificado(&t).is_err());
    }

    #[test]
    fn gabarito_em_escala_real() {
        let t = tubo(&entrada(BendMode::VComRaio, 30., 1), [200., 150.]);
        let g = gabarito(&t).unwrap();
        // a barra de conferência tem exatamente 100 mm
        let barra = g.tracos.iter().find(|x| x.camada == Camada::Cota && x.pts.len() == 2 && (x.pts[1][0] - x.pts[0][0] - 100.).abs() < 1e-9 && x.pts[0][1] == x.pts[1][1]);
        assert!(barra.is_some());
        // o contorno do corte do gabarito tem as mesmas medidas do planificado (1:1)
        let c = g.tracos.iter().find(|x| x.camada == Camada::Corte).unwrap();
        let (xmin, xmax) = c.pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p[0]), b.max(p[0])));
        assert!((xmax - xmin - 46.17).abs() < 0.01);
        let (ymin, ymax) = c.pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p[1]), b.max(p[1])));
        assert!((ymax - ymin - 28.8).abs() < 0.01, "{}", ymax - ymin);
        assert!(g.rotulos.iter().any(|r| r.texto.contains("a validar em peça real")));
        // V múltiplo: um triângulo por rasgo na lateral
        let t = tubo(&entrada(BendMode::VComum, 45., 6), [200., 150.]);
        let g = gabarito(&t).unwrap();
        let tri = g.tracos.iter().filter(|x| x.camada == Camada::Corte && x.pts.len() == 3).count();
        assert_eq!(tri, 6);
        let t = tubo(&entrada(BendMode::Dobradeira, 60., 1), [200., 150.]);
        assert!(gabarito(&t).is_err());
    }
}
