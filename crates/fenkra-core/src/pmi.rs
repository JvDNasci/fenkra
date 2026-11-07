// SPDX-License-Identifier: LGPL-2.1-or-later
//! PMI (detalhamento 3D): anotações no modelo — cotas, notas, rugosidade, solda, referências e
//! tolerâncias geométricas. Cada nível (peça, montagem/submontagem) tem a própria árvore de PMI.
//!
//! As âncoras guardam o ponto clicado; na exibição elas se reencaixam no vértice/aresta mais
//! próximo da geometria atual, e as cotas medem de novo (associativas).

use crate::Id;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum GdtChar {
    Retitude,
    Planeza,
    Circularidade,
    Cilindricidade,
    Paralelismo,
    Perpendicularidade,
    Angularidade,
    Posicao,
    Concentricidade,
    Simetria,
    PerfilLinha,
    PerfilSuperficie,
    BatimentoCircular,
    BatimentoTotal,
}

impl GdtChar {
    pub const ALL: [GdtChar; 14] = [
        GdtChar::Retitude,
        GdtChar::Planeza,
        GdtChar::Circularidade,
        GdtChar::Cilindricidade,
        GdtChar::Paralelismo,
        GdtChar::Perpendicularidade,
        GdtChar::Angularidade,
        GdtChar::Posicao,
        GdtChar::Concentricidade,
        GdtChar::Simetria,
        GdtChar::PerfilLinha,
        GdtChar::PerfilSuperficie,
        GdtChar::BatimentoCircular,
        GdtChar::BatimentoTotal,
    ];
    /// Símbolo ISO 1101.
    pub fn symbol(self) -> &'static str {
        match self {
            GdtChar::Retitude => "⏤",
            GdtChar::Planeza => "⏥",
            GdtChar::Circularidade => "○",
            GdtChar::Cilindricidade => "⌭",
            GdtChar::Paralelismo => "∥",
            GdtChar::Perpendicularidade => "⟂",
            GdtChar::Angularidade => "∠",
            GdtChar::Posicao => "⌖",
            GdtChar::Concentricidade => "◎",
            GdtChar::Simetria => "⌯",
            GdtChar::PerfilLinha => "⌒",
            GdtChar::PerfilSuperficie => "⌓",
            GdtChar::BatimentoCircular => "↗",
            GdtChar::BatimentoTotal => "⌰",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            GdtChar::Retitude => "Retitude",
            GdtChar::Planeza => "Planeza",
            GdtChar::Circularidade => "Circularidade",
            GdtChar::Cilindricidade => "Cilindricidade",
            GdtChar::Paralelismo => "Paralelismo",
            GdtChar::Perpendicularidade => "Perpendicularidade",
            GdtChar::Angularidade => "Angularidade",
            GdtChar::Posicao => "Posição",
            GdtChar::Concentricidade => "Concentricidade",
            GdtChar::Simetria => "Simetria",
            GdtChar::PerfilLinha => "Perfil de linha",
            GdtChar::PerfilSuperficie => "Perfil de superfície",
            GdtChar::BatimentoCircular => "Batimento circular",
            GdtChar::BatimentoTotal => "Batimento total",
        }
    }
}

/// Tipo de solda (símbolo ISO 2553 simplificado).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum WeldKind {
    Filete,
    TopoV,
    TopoReto,
    Bujao,
    Ponto,
    /// meio V (bisel)
    Bisel,
    TopoY,
    TopoU,
    TopoJ,
    /// costura (resistência)
    Costura,
    /// passe de raiz / selagem (lado oposto)
    Selagem,
}

impl WeldKind {
    pub const ALL: [WeldKind; 11] = [
        WeldKind::Filete,
        WeldKind::TopoReto,
        WeldKind::TopoV,
        WeldKind::Bisel,
        WeldKind::TopoY,
        WeldKind::TopoU,
        WeldKind::TopoJ,
        WeldKind::Bujao,
        WeldKind::Ponto,
        WeldKind::Costura,
        WeldKind::Selagem,
    ];
    pub fn label(self) -> &'static str {
        match self {
            WeldKind::Filete => "Filete",
            WeldKind::TopoV => "Topo em V",
            WeldKind::TopoReto => "Topo reto (I)",
            WeldKind::Bujao => "Bujão / fenda",
            WeldKind::Ponto => "Ponto",
            WeldKind::Bisel => "Meio V (bisel)",
            WeldKind::TopoY => "Topo em Y",
            WeldKind::TopoU => "Topo em U",
            WeldKind::TopoJ => "Topo em J",
            WeldKind::Costura => "Costura",
            WeldKind::Selagem => "Selagem / passe de raiz",
        }
    }
    pub fn symbol(self) -> &'static str {
        match self {
            WeldKind::Filete => "◺",
            WeldKind::TopoV => "V",
            WeldKind::TopoReto => "‖",
            WeldKind::Bujao => "▭",
            WeldKind::Ponto => "○",
            WeldKind::Bisel => "⊿",
            WeldKind::TopoY => "Y",
            WeldKind::TopoU => "U",
            WeldKind::TopoJ => "J",
            WeldKind::Costura => "⊖",
            WeldKind::Selagem => "◡",
        }
    }
}

/// Acabamento da face da solda (símbolo suplementar).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum WeldContour {
    #[default]
    Nenhum,
    Plano,
    Convexo,
    Concavo,
}

/// Sistema do símbolo de solda (B190): como a linha de referência diz de que lado está a junta.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum SistemaSolda {
    /// ISO 2553, sistema A: linha cheia (lado da seta, em cima) e tracejada de identificação
    #[default]
    A,
    /// sistema B: linha de referência dupla, as duas cheias; lado da seta em cima da de cima
    B,
    /// padrão americano de símbolos de solda: uma linha só, lado da seta embaixo e o outro em cima
    Aws,
}

impl SistemaSolda {
    pub const ALL: [SistemaSolda; 3] = [SistemaSolda::A, SistemaSolda::B, SistemaSolda::Aws];
    pub fn label(self) -> &'static str {
        match self {
            SistemaSolda::A => "Sistema A (linha tracejada)",
            SistemaSolda::B => "Sistema B (linha dupla)",
            SistemaSolda::Aws => "AWS A2.4 (lado da seta embaixo)",
        }
    }
}

/// Dados completos do símbolo de solda (sistema A por padrão, ou B e AWS).
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct WeldExtra {
    /// sistema do símbolo
    #[serde(default)]
    pub sistema: SistemaSolda,
    /// letra do método de acabamento do contorno (G esmerilhado, C usinado, M, R laminado, H martelado…)
    #[serde(default)]
    pub acabamento: String,
    /// penetração total (meia-lua cheia do lado oposto ao da junta)
    #[serde(default)]
    pub penetracao: bool,
    /// símbolo do outro lado (solda dos dois lados); `None` = só um lado
    #[serde(default)]
    pub other: Option<WeldKind>,
    /// o único símbolo vale para o outro lado (fica na linha tracejada)
    #[serde(default)]
    pub other_side: bool,
    /// comprimento: "l", "n×l (e)"
    #[serde(default)]
    pub length: String,
    /// solda de campo (bandeira)
    #[serde(default)]
    pub field: bool,
    #[serde(default)]
    pub contour: WeldContour,
    /// cauda: processo (ISO 4063, ex.: 135), norma, posição
    #[serde(default)]
    pub process: String,
    /// esconde a representação engrossada do cordão (só exibição, B369): o padrão mostra
    #[serde(default)]
    pub hide_bead: bool,
}

/// Perna do cordão em mm a partir da medida do símbolo: `z5` e `5` são a perna; `a4` é a garganta
/// e a perna do filete a 90° vale `a·√2`. Vazio ou sem número = `None`. Só serve para exibir o
/// cordão (B369): não entra em massa, volume nem lista de corte.
pub fn weld_leg_mm(size: &str) -> Option<f64> {
    let t = size.trim().to_lowercase().replace(',', ".");
    let (garganta, num) = match t.strip_prefix('a') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('z').unwrap_or(&t)),
    };
    let fim = num.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(num.len());
    let v: f64 = num[..fim].parse().ok()?;
    (v.is_finite() && v > 0.).then(|| if garganta { v * std::f64::consts::SQRT_2 } else { v })
}

/// Primitiva do símbolo de solda num sistema local: a seta encontra a linha de referência em
/// (0, 0), a linha vai para +x e +y é "para cima"; medidas em unidades da altura do texto.
#[derive(Clone, PartialEq, Debug)]
pub enum WeldPrim {
    Line(Vec<[f64; 2]>),
    Dashed(Vec<[f64; 2]>),
    Fill(Vec<[f64; 2]>),
    /// texto: posição da linha de base, alinhamento (-1 direita, 0 centro, 1 esquerda)
    Text([f64; 2], String, i8),
}

fn arc(c: [f64; 2], r: f64, a0: f64, a1: f64) -> Vec<[f64; 2]> {
    (0..=16).map(|i| {
        let t = (a0 + (a1 - a0) * i as f64 / 16.).to_radians();
        [c[0] + r * t.cos(), c[1] + r * t.sin()]
    })
    .collect()
}

/// Largura aproximada de um texto (altura `h`).
pub fn text_w(s: &str, h: f64) -> f64 {
    s.chars().count() as f64 * h * 0.62
}

/// Desenha um símbolo elementar com a base em y = `y0`, do lado `s` (+1 acima, −1 abaixo),
/// começando em x = `x0`. Devolve a largura usada.
fn elem(out: &mut Vec<WeldPrim>, k: WeldKind, x0: f64, y0: f64, s: f64, h: f64) -> f64 {
    use WeldPrim::*;
    let p = |x: f64, y: f64| [x0 + x * h, y0 + s * y * h];
    match k {
        WeldKind::Filete => {
            out.push(Line(vec![p(0., 0.), p(0., 1.), p(1., 0.)]));
            1.
        }
        WeldKind::TopoReto => {
            out.push(Line(vec![p(0., 0.), p(0., 1.)]));
            out.push(Line(vec![p(0.5, 0.), p(0.5, 1.)]));
            0.5
        }
        WeldKind::TopoV => {
            out.push(Line(vec![p(0., 1.), p(0.5, 0.), p(1., 1.)]));
            1.
        }
        WeldKind::Bisel => {
            out.push(Line(vec![p(0., 1.), p(0., 0.)]));
            out.push(Line(vec![p(0., 0.), p(0.7, 1.)]));
            0.7
        }
        WeldKind::TopoY => {
            out.push(Line(vec![p(0.5, 0.), p(0.5, 0.4)]));
            out.push(Line(vec![p(0., 1.), p(0.5, 0.4), p(1., 1.)]));
            1.
        }
        WeldKind::TopoU => {
            out.push(Line(vec![p(0.5, 0.), p(0.5, 0.4)]));
            let c = p(0.5, 0.85);
            let r = 0.45 * h;
            let (a0, a1) = if s > 0. { (180., 360.) } else { (0., 180.) };
            out.push(Line(arc(c, r, a0, a1)));
            1.
        }
        WeldKind::TopoJ => {
            out.push(Line(vec![p(0., 0.), p(0., 1.)]));
            let c = p(0.55, 0.85);
            let r = 0.45 * h;
            let (a0, a1) = if s > 0. { (180., 270.) } else { (90., 180.) };
            let mut l = arc(c, r, a0, a1);
            l.insert(0, p(0.1, 1.));
            out.push(Line(l));
            out.push(Line(vec![p(0., 0.4), p(0.1, 0.4)]));
            1.
        }
        WeldKind::Bujao => {
            out.push(Line(vec![p(0., 0.), p(0., 0.6), p(1., 0.6), p(1., 0.)]));
            1.
        }
        WeldKind::Ponto => {
            // ponto e costura ficam centrados na linha (lado indicado pelo deslocamento)
            out.push(Line(arc([x0 + 0.5 * h, y0 + s * 0.15 * h], 0.45 * h, 0., 360.)));
            1.
        }
        WeldKind::Costura => {
            let c = [x0 + 0.5 * h, y0 + s * 0.15 * h];
            out.push(Line(arc(c, 0.45 * h, 0., 360.)));
            out.push(Line(vec![[c[0] - 0.7 * h, c[1] - 0.12 * h], [c[0] + 0.7 * h, c[1] - 0.12 * h]]));
            out.push(Line(vec![[c[0] - 0.7 * h, c[1] + 0.12 * h], [c[0] + 0.7 * h, c[1] + 0.12 * h]]));
            1.
        }
        WeldKind::Selagem => {
            let (a0, a1) = if s > 0. { (0., 180.) } else { (180., 360.) };
            let mut l = arc([x0 + 0.5 * h, y0], 0.5 * h, a0, a1);
            l.push(l[0]);
            out.push(Line(l));
            1.
        }
    }
}

/// Acabamento sobre o símbolo (lado `s`), de x0 a x0 + w.
fn contour(out: &mut Vec<WeldPrim>, c: WeldContour, x0: f64, w: f64, y: f64, s: f64, h: f64) {
    let y = y + s * 0.35 * h;
    match c {
        WeldContour::Nenhum => {}
        WeldContour::Plano => out.push(WeldPrim::Line(vec![[x0, y], [x0 + w, y]])),
        WeldContour::Convexo | WeldContour::Concavo => {
            let bulge = if c == WeldContour::Convexo { s } else { -s };
            out.push(WeldPrim::Line(
                (0..=12)
                    .map(|i| {
                        let t = i as f64 / 12.;
                        [x0 + w * t, y + bulge * 0.25 * h * (std::f64::consts::PI * t).sin()]
                    })
                    .collect(),
            ));
        }
    }
}

/// Símbolo de solda completo (ISO 2553, sistema A) com altura de texto `h`.
pub fn weld_symbol(kind: WeldKind, size: &str, all_around: bool, x: &WeldExtra, h: f64) -> Vec<WeldPrim> {
    use WeldPrim::*;
    let mut out = vec![];
    let dash_y = -0.8 * h;
    let both = x.other.is_some();
    let dupla = x.sistema == SistemaSolda::B;
    let aws = x.sistema == SistemaSolda::Aws;
    let x0 = (text_w(size, h) + 0.6 * h).max(1.6 * h);
    // lado da seta: acima da linha cheia; outro lado: abaixo da tracejada
    let (base_y, s) = if !both && x.other_side { (dash_y, -1.) } else { (0., 1.) };
    let w = elem(&mut out, kind, x0, base_y, s, h) * h;
    contour(&mut out, x.contour, x0, w, base_y + s * h, s, h);
    if !size.trim().is_empty() {
        out.push(Text([x0 - 0.3 * h, base_y + if s > 0. { 0.15 * h } else { -1.05 * h }], size.trim().to_string(), -1));
    }
    let mut right = x0 + w;
    if let Some(o) = x.other {
        let w2 = elem(&mut out, o, x0, if dupla { dash_y } else { 0. }, -1., h) * h;
        right = right.max(x0 + w2);
        if !size.trim().is_empty() {
            out.push(Text([x0 - 0.3 * h, if dupla { dash_y - 1.05 * h } else { -1.05 * h }], size.trim().to_string(), -1));
        }
    }
    if !x.acabamento.trim().is_empty() && x.contour != WeldContour::Nenhum {
        // letra do método de acabamento sobre o símbolo de contorno
        out.push(Text([x0 + w * 0.5, base_y + s * 1.6 * h - if s > 0. { 0. } else { 0.8 * h }], x.acabamento.trim().to_string(), 0));
    }
    if x.penetracao {
        // meia-lua cheia do lado oposto ao da junta
        let c = [x0 + w + 0.6 * h, base_y - s * 0.45 * h];
        let mut m = arc(c, 0.4 * h, 0., 180.);
        if s > 0. {
            m = m.iter().map(|q| [q[0], 2. * c[1] - q[1]]).collect();
        }
        out.push(Fill(m));
        right = right.max(c[0] + 0.5 * h);
    }
    if !x.length.trim().is_empty() {
        out.push(Text([right + 0.4 * h, base_y + if s > 0. { 0.15 * h } else { -1.05 * h }], x.length.trim().to_string(), 1));
        right += 0.4 * h + text_w(x.length.trim(), h);
    }
    if aws {
        // um traço de referência só, com a junta do lado da seta embaixo: espelha o que já está feito
        for pr in out.iter_mut() {
            match pr {
                WeldPrim::Line(p) | WeldPrim::Dashed(p) | WeldPrim::Fill(p) => p.iter_mut().for_each(|q| q[1] = -q[1]),
                WeldPrim::Text(q, _, _) => q[1] = -0.9 * h - q[1],
            }
        }
    }
    let len = (right + 0.8 * h).max(5. * h);
    out.push(Line(vec![[0., 0.], [len, 0.]]));
    if dupla {
        out.push(Line(vec![[0., dash_y], [len, dash_y]]));
    } else if !both && !aws {
        out.push(Dashed(vec![[0.3 * h, dash_y], [len, dash_y]]));
    }
    if all_around {
        out.push(Line(arc([0., 0.], 0.45 * h, 0., 360.)));
    }
    if x.field {
        out.push(Line(vec![[0., 0.], [0., 1.8 * h]]));
        out.push(Fill(vec![[0., 1.8 * h], [0.9 * h, 1.55 * h], [0., 1.3 * h]]));
    }
    if !x.process.trim().is_empty() {
        out.push(Line(vec![[len + 0.7 * h, 0.6 * h], [len, 0.], [len + 0.7 * h, -0.6 * h]]));
        out.push(Text([len + 0.9 * h, -0.35 * h], x.process.trim().to_string(), 1));
    }
    out
}

/// Extras do quadro de tolerância geométrica.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct GdtExtra {
    /// zona de tolerância cilíndrica (Ø antes do valor)
    #[serde(default)]
    pub zone_dia: bool,
    /// modificador do valor: Ⓜ, Ⓛ, Ⓟ, Ⓕ, Ⓢ… (vazio = nenhum)
    #[serde(default)]
    pub modifier: String,
    /// segunda linha (quadro composto): tolerância e referências
    #[serde(default)]
    pub second: Option<(String, String)>,
    /// característica própria da segunda linha: quadro de segmento único (`None` = composto, uma
    /// só casa de símbolo para as duas linhas)
    #[serde(default)]
    pub second_ch: Option<GdtChar>,
    /// forma da zona; `None` deriva de `zone_dia` (arquivos antigos)
    #[serde(default)]
    pub zona: Option<crate::gdt::Zona>,
    /// deslocamento da zona não uniforme
    #[serde(default)]
    pub uz: String,
    /// altura da zona projetada
    #[serde(default)]
    pub proj: String,
    /// modificadores do valor e da chamada (tipados; `modifier` guarda só o que veio de antes)
    #[serde(default)]
    pub mods: Vec<crate::gdt::Modif>,
    /// modificadores de cada referência, na ordem delas
    #[serde(default)]
    pub datum_mods: Vec<Vec<crate::gdt::Modif>>,
    /// letras de "entre dois pontos"
    #[serde(default)]
    pub entre: (String, String),
    /// tolerância por unidade: comprimento (valor/comprimento)
    #[serde(default)]
    pub por_unidade: String,
    /// valor máximo da tolerância por unidade
    #[serde(default)]
    pub max_valor: String,
    #[serde(default)]
    pub norma: crate::gdt::Norma,
    #[serde(default)]
    pub elemento: crate::gdt::Elemento,
}

/// Moldura do rótulo (`Padrao` = a do tipo: quadro no GD&T e na legenda, sem moldura nas cotas).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum PmiFrame {
    #[default]
    Padrao,
    Caixa,
    /// cápsula (pontas redondas)
    Arredondada,
    Nenhuma,
    Circulo,
    Quadrado,
    Triangulo,
    Losango,
    BandeiraDir,
    BandeiraEsq,
    /// círculo com a linha do meio (riscado)
    CirculoRiscado,
    /// retângulo com a linha do meio (riscado)
    RetanguloRiscado,
}

impl PmiFrame {
    pub const ALL: [PmiFrame; 12] = [
        PmiFrame::Nenhuma,
        PmiFrame::Caixa,
        PmiFrame::Arredondada,
        PmiFrame::Circulo,
        PmiFrame::Quadrado,
        PmiFrame::Triangulo,
        PmiFrame::Losango,
        PmiFrame::BandeiraDir,
        PmiFrame::BandeiraEsq,
        PmiFrame::CirculoRiscado,
        PmiFrame::RetanguloRiscado,
        PmiFrame::Padrao,
    ];
    pub fn label(self) -> &'static str {
        match self {
            PmiFrame::Padrao => "padrão",
            PmiFrame::Caixa => "retângulo",
            PmiFrame::Arredondada => "cápsula",
            PmiFrame::Nenhuma => "nenhuma",
            PmiFrame::Circulo => "círculo",
            PmiFrame::Quadrado => "quadrado",
            PmiFrame::Triangulo => "triângulo",
            PmiFrame::Losango => "losango",
            PmiFrame::BandeiraDir => "bandeira (direita)",
            PmiFrame::BandeiraEsq => "bandeira (esquerda)",
            PmiFrame::CirculoRiscado => "círculo riscado",
            PmiFrame::RetanguloRiscado => "retângulo riscado",
        }
    }
    /// Contorno da moldura em volta do retângulo do texto (x0, y0)-(x1, y1), y para baixo:
    /// cada caminho com "fechado?". Vazio = sem moldura.
    pub fn paths(self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<(Vec<[f64; 2]>, bool)> {
        let (cx, cy) = ((x0 + x1) / 2., (y0 + y1) / 2.);
        let (w, h) = (x1 - x0, y1 - y0);
        let circle = |r: f64| -> Vec<[f64; 2]> { (0..48).map(|i| {
            let t = i as f64 / 48. * std::f64::consts::TAU;
            [cx + r * t.cos(), cy + r * t.sin()]
        }).collect() };
        let rect = |a: f64, b: f64, c: f64, d: f64| vec![[a, b], [c, b], [c, d], [a, d]];
        match self {
            PmiFrame::Padrao | PmiFrame::Nenhuma => vec![],
            PmiFrame::Caixa => vec![(rect(x0, y0, x1, y1), true)],
            PmiFrame::Arredondada => {
                let r = h / 2.;
                let mut v = vec![];
                for i in 0..=12 {
                    let t = -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * i as f64 / 12.;
                    v.push([x1 - r + r * t.cos(), cy + r * t.sin()]);
                }
                for i in 0..=12 {
                    let t = std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * i as f64 / 12.;
                    v.push([x0 + r + r * t.cos(), cy + r * t.sin()]);
                }
                vec![(v, true)]
            }
            PmiFrame::Circulo => vec![(circle((w * w + h * h).sqrt() / 2.), true)],
            PmiFrame::CirculoRiscado => {
                let r = (w * w + h * h).sqrt() / 2.;
                vec![(circle(r), true), (vec![[cx - r, cy], [cx + r, cy]], false)]
            }
            PmiFrame::Quadrado => {
                let s = w.max(h) / 2.;
                vec![(rect(cx - s, cy - s, cx + s, cy + s), true)]
            }
            PmiFrame::RetanguloRiscado => vec![(rect(x0, y0, x1, y1), true), (vec![[x0, cy], [x1, cy]], false)],
            PmiFrame::Triangulo => {
                // o texto cabe na parte de baixo do triângulo
                let bw = w * 2. + h;
                let top = y1 - (bw / 2.) * 3f64.sqrt();
                vec![(vec![[cx, top.min(y0 - h)], [cx + bw / 2., y1], [cx - bw / 2., y1]], true)]
            }
            PmiFrame::Losango => vec![(vec![[cx, cy - h], [cx + w, cy], [cx, cy + h], [cx - w, cy]], true)],
            PmiFrame::BandeiraDir => vec![(vec![[x0, y0], [x1, y0], [x1 + h / 2., cy], [x1, y1], [x0, y1]], true)],
            PmiFrame::BandeiraEsq => vec![(vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0 - h / 2., cy]], true)],
        }
    }
}

/// Tipo de linha (molduras e chamadas): padrão em múltiplos da espessura.
pub fn dash_pattern(kind: u8) -> &'static [f64] {
    match kind {
        1 => &[6., 4.],
        2 => &[1.2, 2.6],
        3 => &[12., 3., 2., 3., 2., 3.],
        4 => &[12., 3., 2., 3.],
        _ => &[],
    }
}

/// Nome do tipo de linha.
pub fn dash_label(kind: u8) -> &'static str {
    match kind {
        1 => "tracejada",
        2 => "pontilhada",
        3 => "fantasma",
        4 => "traço-ponto",
        _ => "contínua",
    }
}

/// Divide uma polilinha em traços conforme o padrão (unidades já multiplicadas).
pub fn dashed(pts: &[[f64; 2]], closed: bool, pattern: &[f64]) -> Vec<Vec<[f64; 2]>> {
    let mut path = pts.to_vec();
    if closed && !pts.is_empty() {
        path.push(pts[0]);
    }
    if pattern.is_empty() || path.len() < 2 {
        return vec![path];
    }
    let mut out = vec![];
    let mut cur: Vec<[f64; 2]> = vec![path[0]];
    let (mut k, mut left, mut on) = (0usize, pattern[0], true);
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let mut t = 0.;
        while len - t > 1e-9 {
            let step = left.min(len - t);
            t += step;
            left -= step;
            let p = [a[0] + (b[0] - a[0]) * t / len, a[1] + (b[1] - a[1]) * t / len];
            if on {
                cur.push(p);
            }
            if left <= 1e-9 {
                if on && cur.len() > 1 {
                    out.push(std::mem::take(&mut cur));
                }
                on = !on;
                k = (k + 1) % pattern.len();
                left = pattern[k];
                cur = if on { vec![p] } else { vec![] };
            }
        }
    }
    if on && cur.len() > 1 {
        out.push(cur);
    }
    out
}

/// Ponta das chamadas e linhas de cota.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum PmiArrow {
    #[default]
    Padrao,
    Cheia,
    Aberta,
    Ponto,
    Barra,
    Nenhuma,
}

/// Fundo do texto.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum PmiBg {
    #[default]
    Padrao,
    Nenhum,
    Branco,
    Escuro,
    /// fundo opaco que esconde a geometria atrás do texto
    Recorte,
}

/// Aparência de uma anotação (tudo "padrão" herda o estilo geral das anotações).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PmiStyle {
    /// altura do texto (px na tela / mm quando deitado num plano); 0 = padrão
    #[serde(default)]
    pub size: f32,
    /// cor das linhas (chamadas, cotas, quadros)
    #[serde(default)]
    pub color: Option<[u8; 3]>,
    #[serde(default)]
    pub frame: PmiFrame,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
    /// fonte (nome da família; vazio = padrão)
    #[serde(default)]
    pub font: String,
    #[serde(default)]
    pub text_color: Option<[u8; 3]>,
    /// espessura das linhas (px); 0 = padrão
    #[serde(default)]
    pub line_width: f32,
    #[serde(default)]
    pub arrow: PmiArrow,
    #[serde(default)]
    pub bg: PmiBg,
    // ---- moldura
    /// espessura da moldura (0 = a das linhas)
    #[serde(default)]
    pub frame_width: f32,
    /// tipo de linha da moldura (0 contínua, 1 tracejada, 2 pontilhada, 3 fantasma, 4 traço-ponto)
    #[serde(default)]
    pub frame_dash: u8,
    #[serde(default)]
    pub frame_color: Option<[u8; 3]>,
    /// largura fixa da moldura (0 = acompanha o texto)
    #[serde(default)]
    pub frame_fixed: f32,
    // ---- texto
    #[serde(default)]
    pub bold: bool,
    /// alinhamento das linhas: 0 esquerda, 1 centro, 2 direita
    #[serde(default)]
    pub justify: u8,
    /// ponto da caixa preso ao lugar do rótulo (0..8 = linhas de cima a baixo; 4 = centro)
    #[serde(default = "centro")]
    pub anchor: u8,
    /// espaçamento entre linhas (multiplicador; 0 = normal)
    #[serde(default)]
    pub line_spacing: f32,
    /// rotação do rótulo (graus) e espelho
    #[serde(default)]
    pub angle: f32,
    #[serde(default)]
    pub mirror: bool,
    /// deslocamento fino do texto em relação ao ponto (px)
    #[serde(default)]
    pub nudge: [f32; 2],
    /// o tamanho acompanha o zoom (como se estivesse em mm no modelo)
    #[serde(default)]
    pub apply_scale: bool,
    // ---- chamada
    #[serde(default)]
    pub hide_leader: bool,
    #[serde(default)]
    pub leader_width: f32,
    #[serde(default)]
    pub leader_dash: u8,
    #[serde(default)]
    pub leader_color: Option<[u8; 3]>,
    /// vértices intermediários da chamada de cada ponta (em coordenadas do modelo, do rótulo para a
    /// ponta); vazio = chamada reta (B23, ver `pmi_leader`)
    #[serde(default)]
    pub leader_via: Vec<Vec<[f64; 3]>>,
    /// de qual lado da caixa do texto sai a chamada de cada ponta: 0 automático (o lado voltado
    /// para a ponta, como nos arquivos antigos), 1 esquerda, 2 direita, 3 topo, 4 base (B368)
    #[serde(default)]
    pub leader_side: Vec<u8>,
    /// ponta própria de cada chamada; vazio ou `Padrao` = a ponta do estilo (B368)
    #[serde(default)]
    pub leader_arrow: Vec<PmiArrow>,
    // ---- cota (B269)
    /// quanto as linhas de apoio passam da linha de cota (mm no modelo); 0 = o padrão curto
    #[serde(default)]
    pub ext_over: f64,
    /// posição do texto ao longo da linha de cota, a partir do meio (mm no modelo; 0 = no meio)
    #[serde(default)]
    pub text_along: f64,
    /// casas decimais do valor medido; `None` = o padrão (até 2, sem zeros sobrando) (B270)
    #[serde(default)]
    pub decimals: Option<u8>,
    /// raio do arco de uma cota angular ou de arco (mm no modelo); 0 = o padrão da tela (B269)
    #[serde(default)]
    pub arc_r: f64,
}

fn sem_componente() -> Id {
    Id(0)
}

fn centro() -> u8 {
    4
}

impl PmiStyle {
    /// Este estilo por cima de `base`: o que ficou em "padrão" vem de `base` (hierarquia:
    /// anotação → grupo/vista → geral).
    pub fn over(&self, base: &PmiStyle) -> PmiStyle {
        let n = |a: f32, b: f32| if a > 0. { a } else { b };
        let s = |a: &String, b: &String| if a.is_empty() { b.clone() } else { a.clone() };
        PmiStyle {
            size: n(self.size, base.size),
            color: self.color.or(base.color),
            frame: if self.frame != PmiFrame::Padrao { self.frame } else { base.frame },
            prefix: s(&self.prefix, &base.prefix),
            suffix: s(&self.suffix, &base.suffix),
            font: s(&self.font, &base.font),
            text_color: self.text_color.or(base.text_color),
            line_width: n(self.line_width, base.line_width),
            arrow: if self.arrow != PmiArrow::Padrao { self.arrow } else { base.arrow },
            bg: if self.bg != PmiBg::Padrao { self.bg } else { base.bg },
            frame_width: n(self.frame_width, base.frame_width),
            frame_dash: if self.frame_dash != 0 { self.frame_dash } else { base.frame_dash },
            frame_color: self.frame_color.or(base.frame_color),
            frame_fixed: n(self.frame_fixed, base.frame_fixed),
            bold: self.bold || base.bold,
            justify: if self.justify != 0 { self.justify } else { base.justify },
            anchor: if self.anchor != 4 { self.anchor } else { base.anchor },
            line_spacing: n(self.line_spacing, base.line_spacing),
            angle: if self.angle != 0. { self.angle } else { base.angle },
            mirror: self.mirror || base.mirror,
            nudge: self.nudge,
            apply_scale: self.apply_scale || base.apply_scale,
            hide_leader: self.hide_leader || base.hide_leader,
            leader_width: n(self.leader_width, base.leader_width),
            leader_dash: if self.leader_dash != 0 { self.leader_dash } else { base.leader_dash },
            leader_color: self.leader_color.or(base.leader_color),
            leader_via: self.leader_via.clone(),
            leader_side: self.leader_side.clone(),
            leader_arrow: self.leader_arrow.clone(),
            ext_over: self.ext_over,
            text_along: self.text_along,
            decimals: self.decimals.or(base.decimals),
            arc_r: self.arc_r,
        }
    }
}

impl Default for PmiStyle {
    fn default() -> Self {
        PmiStyle {
            size: 0.,
            color: None,
            frame: PmiFrame::Padrao,
            prefix: String::new(),
            suffix: String::new(),
            font: String::new(),
            text_color: None,
            line_width: 0.,
            arrow: PmiArrow::Padrao,
            bg: PmiBg::Padrao,
            frame_width: 0.,
            frame_dash: 0,
            frame_color: None,
            frame_fixed: 0.,
            bold: false,
            justify: 0,
            anchor: 4,
            line_spacing: 0.,
            angle: 0.,
            mirror: false,
            nudge: [0., 0.],
            apply_scale: false,
            hide_leader: false,
            leader_width: 0.,
            leader_dash: 0,
            leader_color: None,
            leader_via: vec![],
            leader_side: vec![],
            leader_arrow: vec![],
            ext_over: 0.,
            text_along: 0.,
            decimals: None,
            arc_r: 0.,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PmiKind {
    /// distância entre dois pontos (vértices/arestas)
    Linear,
    /// raio / diâmetro de uma aresta circular (âncora no centro e um ponto da aresta)
    Radius,
    Diameter,
    /// ângulo no vértice (âncoras: vértice, ponto 1, ponto 2)
    Angle,
    Note,
    /// rugosidade: Ra e processo
    Roughness {
        ra: String,
        process: String,
        /// campos estruturados: remoção, limites, amostragem, estrias, sobremetal, norma (B190)
        #[serde(default)]
        extra: crate::pmi_acabamento::RugExtra,
    },
    /// alvo de referência (datum target): ponto, linha ou área de contato (B189)
    DatumTarget { alvo: crate::pmi_alvos::AlvoRef },
    Weld {
        kind: WeldKind,
        size: String,
        all_around: bool,
        #[serde(default)]
        extra: WeldExtra,
    },
    /// referência (datum) com letra
    Datum { letter: String },
    /// quadro de tolerância geométrica
    Gdt {
        ch: GdtChar,
        tol: String,
        datums: String,
        #[serde(default)]
        extra: GdtExtra,
    },
    /// legenda 3D: texto com campos da peça ({nome}, {codigo}, {descricao}, {material}, {massa}…)
    TitleBlock { template: String },
    /// balão com o número de item (montagem)
    Balloon {
        item: String,
        /// criado à mão: os balões automáticos preservam e não repõem
        #[serde(default)]
        manual: bool,
    },
    /// cota ordenada / ao longo de um eixo do modelo (0 = X, 1 = Y, 2 = Z): âncoras origem e ponto;
    /// mede só a componente no eixo (B185)
    Ordinate { axis: u8 },
    /// distância entre dois furos/cilindros/arestas circulares: centros, interior ou exterior (B185)
    Gap { mode: GapMode },
    /// comprimento de arco: âncoras centro e as duas pontas (B185)
    ArcLength,
    /// chamada gerada da feature (furo, rosca, chanfro, filete, chapa): o texto vem dos parâmetros
    /// dela, sem digitar (B186). `text` guarda a última versão gerada.
    FeatureNote {
        feature: Id,
        /// componente da montagem dono da feature; `Id(0)` na peça (B186)
        #[serde(default = "sem_componente")]
        comp: Id,
    },
}

/// Qual medida de uma cota entre dois círculos.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum GapMode {
    /// entre os centros
    Centers,
    /// entre os bordos mais próximos (centros menos os dois raios)
    Inner,
    /// entre os bordos mais afastados (centros mais os dois raios)
    Outer,
}

impl GapMode {
    pub fn label(self) -> &'static str {
        match self {
            GapMode::Centers => "centro a centro",
            GapMode::Inner => "interior",
            GapMode::Outer => "exterior",
        }
    }
}

/// Nome do eixo para a interface.
pub fn axis_name(axis: u8) -> &'static str {
    ["X", "Y", "Z"][axis.min(2) as usize]
}

impl PmiKind {
    pub fn label(&self) -> &'static str {
        match self {
            PmiKind::Linear => "Cota linear",
            PmiKind::Radius => "Raio",
            PmiKind::Diameter => "Diâmetro",
            PmiKind::Angle => "Ângulo",
            PmiKind::Note => "Nota",
            PmiKind::Roughness { .. } => "Rugosidade",
            PmiKind::DatumTarget { .. } => "Alvo de referência",
            PmiKind::Weld { .. } => "Solda",
            PmiKind::Datum { .. } => "Referência",
            PmiKind::Gdt { .. } => "Tolerância geométrica",
            PmiKind::TitleBlock { .. } => "Legenda 3D",
            PmiKind::Balloon { .. } => "Balão",
            PmiKind::Ordinate { .. } => "Cota ordenada",
            PmiKind::Gap { .. } => "Cota entre círculos",
            PmiKind::ArcLength => "Comprimento de arco",
            PmiKind::FeatureNote { .. } => "Chamada de feature",
        }
    }
    /// A cota se desenha como linha de cota entre duas âncoras (linear, ordenada, entre círculos).
    pub fn is_linear_like(&self) -> bool {
        matches!(self, PmiKind::Linear | PmiKind::Ordinate { .. } | PmiKind::Gap { .. })
    }
    /// Cota de medida (leva tolerância dimensional e valor medido).
    pub fn is_dimension(&self) -> bool {
        matches!(self, PmiKind::Linear | PmiKind::Radius | PmiKind::Diameter | PmiKind::Angle | PmiKind::Ordinate { .. } | PmiKind::Gap { .. } | PmiKind::ArcLength)
    }
    /// Próxima variante de medida (tecla Tab): linear -> ao longo de X, Y, Z -> entre círculos
    /// (centros, interior, exterior) -> linear; raio <-> diâmetro.
    pub fn next_variant(&self) -> PmiKind {
        match self {
            PmiKind::Linear => PmiKind::Ordinate { axis: 0 },
            PmiKind::Ordinate { axis } if *axis < 2 => PmiKind::Ordinate { axis: axis + 1 },
            PmiKind::Ordinate { .. } => PmiKind::Gap { mode: GapMode::Centers },
            PmiKind::Gap { mode: GapMode::Centers } => PmiKind::Gap { mode: GapMode::Inner },
            PmiKind::Gap { mode: GapMode::Inner } => PmiKind::Gap { mode: GapMode::Outer },
            PmiKind::Gap { mode: GapMode::Outer } => PmiKind::Linear,
            PmiKind::Radius => PmiKind::Diameter,
            PmiKind::Diameter => PmiKind::Radius,
            k => k.clone(),
        }
    }
    /// Esse tipo tem mais de uma variante de medida para o Tab alternar.
    pub fn has_variants(&self) -> bool {
        self.next_variant() != *self
    }
    /// Rótulo da variante, com o eixo ou o modo.
    pub fn variant_label(&self) -> String {
        match self {
            PmiKind::Ordinate { axis } => format!("{} {}", self.label(), axis_name(*axis)),
            PmiKind::Gap { mode } => format!("{}: {}", self.label(), mode.label()),
            k => k.label().to_string(),
        }
    }
    /// Quantas âncoras o tipo pede.
    pub fn anchors(&self) -> usize {
        match self {
            PmiKind::Linear | PmiKind::Radius | PmiKind::Diameter | PmiKind::Ordinate { .. } | PmiKind::Gap { .. } => 2,
            PmiKind::Angle | PmiKind::ArcLength => 3,
            _ => 1,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Pmi {
    pub id: Id,
    pub name: String,
    pub kind: PmiKind,
    pub anchors: Vec<[f64; 3]>,
    /// posição do texto em relação à primeira âncora (mm)
    pub offset: [f64; 3],
    /// texto (nota) ou texto que substitui o valor medido; vazio = valor medido
    #[serde(default)]
    pub text: String,
    /// tolerância dimensional (+, −); vazias = sem tolerância
    #[serde(default)]
    pub tol_plus: String,
    #[serde(default)]
    pub tol_minus: String,
    #[serde(default = "yes")]
    pub visible: bool,
    /// texto deitado num plano (normal, direção x); `None` = sempre de frente para a tela
    #[serde(default)]
    pub plane: Option<([f64; 3], [f64; 3])>,
    #[serde(default)]
    pub style: PmiStyle,
    /// pasta de anotações na árvore (vazio = raiz)
    #[serde(default)]
    pub folder: String,
    /// alvo topológico de cada âncora (mesma ordem de `anchors`); `None` = âncora solta no espaço
    #[serde(default)]
    pub targets: Vec<Option<PmiTarget>>,
}

/// Captura: uma vista (câmera) com as anotações que aparecem nela.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PmiCapture {
    pub id: Id,
    pub name: String,
    pub view: crate::doc::NamedView,
    /// anotações visíveis nesta captura
    pub shown: Vec<Id>,
    /// o que estava oculto no modelo: comandos, coletores, referências (peça) ou componentes
    /// (montagem) — separado das anotações
    #[serde(default)]
    pub hidden: Vec<Id>,
    /// guarda a visibilidade do modelo (senão a captura só mexe em câmera e anotações)
    #[serde(default)]
    pub with_model: bool,
}

/// Modelo padrão da legenda 3D.
pub const TITLE_TEMPLATE: &str = "{nome}\n{codigo} — {descricao}\nMaterial: {material}  ·  Massa: {massa}\n{data}";

fn yes() -> bool {
    true
}

impl Pmi {
    /// Valor medido com as âncoras dadas (já reencaixadas na geometria atual).
    pub fn measure(&self, a: &[[f64; 3]]) -> Option<f64> {
        let d = |p: [f64; 3], q: [f64; 3]| ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt();
        match self.kind {
            PmiKind::Linear => Some(d(*a.first()?, *a.get(1)?)),
            PmiKind::Gap { .. } => Some(d(*a.first()?, *a.get(1)?)),
            PmiKind::Ordinate { axis } => {
                let (o, p) = (*a.first()?, *a.get(1)?);
                Some((p[axis.min(2) as usize] - o[axis.min(2) as usize]).abs())
            }
            PmiKind::ArcLength => {
                let (c, p, q) = (*a.first()?, *a.get(1)?, *a.get(2)?);
                let r = (d(p, c) + d(q, c)) / 2.;
                let (u, w) = (sub(p, c), sub(q, c));
                let cs = dot(u, w) / (len(u) * len(w)).max(1e-12);
                // arco menor entre as duas pontas
                Some(r * cs.clamp(-1., 1.).acos())
            }
            PmiKind::Radius => Some(d(*a.first()?, *a.get(1)?)),
            PmiKind::Diameter => Some(2. * d(*a.first()?, *a.get(1)?)),
            PmiKind::Angle => {
                let (v, p, q) = (*a.first()?, *a.get(1)?, *a.get(2)?);
                let u = [p[0] - v[0], p[1] - v[1], p[2] - v[2]];
                let w = [q[0] - v[0], q[1] - v[1], q[2] - v[2]];
                let c = (u[0] * w[0] + u[1] * w[1] + u[2] * w[2]) / (d(p, v) * d(q, v)).max(1e-12);
                Some(c.clamp(-1., 1.).acos().to_degrees())
            }
            _ => None,
        }
    }

    /// Texto principal e, quando as tolerâncias são diferentes, as duas empilhadas (+a, −b).
    pub fn display_parts(&self, value: Option<f64>) -> (String, Option<(String, String)>) {
        let (p, m) = (self.tol_plus.trim(), self.tol_minus.trim());
        let dim = self.kind.is_dimension();
        if dim && !p.is_empty() && !m.is_empty() && p != m {
            let mut x = self.clone();
            x.tol_plus.clear();
            x.tol_minus.clear();
            return (x.display(value), Some((format!("+{p}"), format!("−{m}"))));
        }
        (self.display(value), None)
    }

    /// Texto exibido (com prefixo e tolerância).
    pub fn display(&self, value: Option<f64>) -> String {
        let tol = if self.tol_plus.trim().is_empty() && self.tol_minus.trim().is_empty() {
            String::new()
        } else if self.tol_plus.trim() == self.tol_minus.trim() {
            format!(" ±{}", self.tol_plus.trim())
        } else {
            format!(" +{}/−{}", self.tol_plus.trim(), self.tol_minus.trim())
        };
        let base = |pre: &str, v: Option<f64>, suf: &str| if !self.text.trim().is_empty() { self.text.clone() } else { format!("{pre}{}{suf}", v.map(|x| num_dec(x, self.style.decimals)).unwrap_or("?".into())) };
        match &self.kind {
            PmiKind::Linear => base("", value, "") + &tol,
            PmiKind::Radius => base("R", value, "") + &tol,
            PmiKind::Diameter => base("Ø", value, "") + &tol,
            PmiKind::Angle => base("", value, "°") + &tol,
            PmiKind::Ordinate { .. } | PmiKind::Gap { .. } => base("", value, "") + &tol,
            PmiKind::ArcLength => base("⌒ ", value, "") + &tol,
            PmiKind::Note | PmiKind::FeatureNote { .. } => self.text.clone(),
            PmiKind::Roughness { ra, process, extra } if extra.basico() => format!("√ Ra {ra}{}", if process.trim().is_empty() { String::new() } else { format!(" ({process})") }),
            PmiKind::Roughness { ra, process, extra } => crate::pmi_acabamento::resumo(ra, process, extra),
            PmiKind::DatumTarget { alvo } => alvo.rotulo(),
            PmiKind::Weld { kind, size, all_around, .. } => format!("{}{} {}", if *all_around { "○ " } else { "" }, kind.symbol(), size),
            PmiKind::Datum { letter } => format!("[{letter}]"),
            PmiKind::Gdt { ch, tol, datums, extra } => {
                let linhas = extra.linhas(*ch, tol, datums);
                let chamada = extra.chamada();
                let corpo = linhas
                    .iter()
                    .map(|l| {
                        let mut s = format!("| {} |", l.sym.unwrap_or(*ch).symbol());
                        for c in &l.celulas {
                            s += &format!(" {} |", crate::gdt::celula_texto(c));
                        }
                        s
                    })
                    .collect::<Vec<_>>()
                    .join("
");
                if chamada.is_empty() { corpo } else { format!("{chamada}
{corpo}") }
            }
            PmiKind::TitleBlock { template } => template.clone(),
            PmiKind::Balloon { item, .. } => item.clone(),
        }
    }
}


/// Tipo de alvo reconhecido na geometria.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TargetClass {
    FacePlana,
    FaceCilindrica,
    /// cone, esfera, toro ou superfície livre
    FaceOutra,
    ArestaCircular,
    Aresta,
    Vertice,
}

/// Alvo escolhido na viewport: a chave geométrica do elemento (tipo, direção, posição e raio),
/// não o ponto clicado. Com ela a anotação reencontra o elemento depois que o modelo muda.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct PmiTarget {
    pub class: TargetClass,
    /// ponto sobre o elemento (face, aresta ou vértice) ou centro do círculo/eixo
    pub point: [f64; 3],
    /// normal externa (plano) ou eixo (cilindro, aresta circular); zeros nos demais
    pub dir: [f64; 3],
    /// raio do cilindro ou da aresta circular; 0 nos demais
    pub radius: f64,
    /// cilindro cuja superfície encara o eixo (furo)
    pub concave: bool,
}

/// Número no formato do Fenkra: até 2 casas, vírgula decimal, sem zeros sobrando.
/// Número com `casas` decimais fixas (vírgula decimal) ou, sem `casas`, no formato padrão.
pub fn num_dec(v: f64, casas: Option<u8>) -> String {
    match casas {
        Some(n) => {
            // arredonda a metade para longe do zero (o formato do Rust arredonda para o par)
            let n = n.min(6) as usize;
            let k = 10f64.powi(n as i32);
            format!("{:.*}", n, (v * k).round() / k).replace('.', ",")
        }
        None => num(v),
    }
}

pub fn num(v: f64) -> String {
    let s = format!("{:.2}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.').replace('.', ",");
    if s.is_empty() { "0".into() } else { s }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn len(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Círculo no plano perpendicular a `axis` que passa pelos pontos (mínimos quadrados algébricos):
/// devolve (centro 3D, raio). Serve para achar o raio de uma face cilíndrica pela malha.
pub fn fit_axis_circle(pts: &[[f64; 3]], axis: [f64; 3]) -> Option<([f64; 3], f64)> {
    let l = len(axis);
    if pts.len() < 3 || l < 1e-9 {
        return None;
    }
    let w = [axis[0] / l, axis[1] / l, axis[2] / l];
    let h = if w[0].abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] };
    let u = cross(w, h);
    let k = len(u);
    let u = [u[0] / k, u[1] / k, u[2] / k];
    let v = cross(w, u);
    let o = pts[0];
    // x²+y² + a·x + b·y + c = 0, resolvido pelas equações normais 3×3
    let (mut sxx, mut sxy, mut syy, mut sx, mut sy, mut n) = (0., 0., 0., 0., 0., 0.);
    let (mut bx, mut by, mut bc) = (0., 0., 0.);
    for p in pts {
        let d = sub(*p, o);
        let (x, y) = (dot(d, u), dot(d, v));
        let z = x * x + y * y;
        sxx += x * x;
        sxy += x * y;
        syy += y * y;
        sx += x;
        sy += y;
        n += 1.;
        bx -= z * x;
        by -= z * y;
        bc -= z;
    }
    let m = [[sxx, sxy, sx], [sxy, syy, sy], [sx, sy, n]];
    let det = |m: [[f64; 3]; 3]| m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let d = det(m);
    if d.abs() < 1e-12 {
        return None;
    }
    let b = [bx, by, bc];
    let solve = |col: usize| {
        let mut t = m;
        for r in 0..3 {
            t[r][col] = b[r];
        }
        det(t) / d
    };
    let (a, bb, c) = (solve(0), solve(1), solve(2));
    let (cx, cy) = (-a / 2., -bb / 2.);
    let r2 = cx * cx + cy * cy - c;
    if r2 <= 0. {
        return None;
    }
    let ctr = [o[0] + u[0] * cx + v[0] * cy, o[1] + u[1] * cx + v[1] * cy, o[2] + u[2] * cx + v[2] * cy];
    Some((ctr, r2.sqrt()))
}

impl PmiTarget {
    /// Nome curto do que o cursor reconheceu, com as medidas ('furo Ø6,5').
    pub fn label(&self) -> String {
        match self.class {
            TargetClass::FacePlana => "face plana".into(),
            TargetClass::FaceCilindrica => format!("{} Ø{}", if self.concave { "furo" } else { "cilindro" }, num(self.radius * 2.)),
            TargetClass::FaceOutra => "face curva".into(),
            TargetClass::ArestaCircular => format!("aresta circular Ø{}", num(self.radius * 2.)),
            TargetClass::Aresta => "aresta".into(),
            TargetClass::Vertice => "vértice".into(),
        }
    }

    /// Distância entre este alvo (guardado) e um candidato; `None` se não servem como o mesmo.
    fn score(&self, c: &PmiTarget) -> Option<f64> {
        if self.class != c.class {
            return None;
        }
        let cosd = if len(self.dir) > 0.5 && len(c.dir) > 0.5 { dot(self.dir, c.dir) } else { 1. };
        let d = sub(c.point, self.point);
        match self.class {
            // 35° de tolerância na orientação; o resto é a distância entre os planos + deslocamento
            TargetClass::FacePlana => (cosd > 0.82).then(|| dot(d, self.dir).abs() + 0.05 * len(d) + (1. - cosd) * 50.),
            TargetClass::FaceCilindrica | TargetClass::ArestaCircular => {
                (cosd.abs() > 0.9 && self.concave == c.concave).then(|| len(cross(d, self.dir)) + (c.radius - self.radius).abs() * 2. + (1. - cosd.abs()) * 50.)
            }
            _ => Some(len(d)),
        }
    }
}

/// Estado de uma anotação ligada à geometria.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PmiState {
    Ok,
    /// o elemento ainda existe, mas a anotação deixou de ter um único sentido
    Ambigua,
    /// o elemento sumiu
    Quebrada,
}

impl PmiState {
    pub fn label(self) -> &'static str {
        match self {
            PmiState::Ok => "OK",
            PmiState::Ambigua => "Ambígua",
            PmiState::Quebrada => "Quebrada",
        }
    }
}

/// Resultado do diagnóstico: estado, índice da âncora que falhou e a explicação curta.
#[derive(Clone, PartialEq, Debug)]
pub struct PmiDiag {
    pub state: PmiState,
    pub failed: Option<usize>,
    pub message: String,
    /// alvos reencontrados na geometria atual, na ordem das âncoras
    pub resolved: Vec<Option<PmiTarget>>,
}

impl PmiDiag {
    pub fn ok(&self) -> bool {
        self.state == PmiState::Ok
    }
}

/// Reencontra `t` entre os candidatos da geometria atual: o de menor distância; se o segundo
/// colocado chega muito perto, o resultado é ambíguo.
pub fn rematch(t: &PmiTarget, cands: &[PmiTarget]) -> Result<PmiTarget, PmiState> {
    let mut sc: Vec<(f64, &PmiTarget)> = cands.iter().filter_map(|c| t.score(c).map(|s| (s, c))).collect();
    sc.sort_by(|a, b| a.0.total_cmp(&b.0));
    match sc.as_slice() {
        [] => Err(PmiState::Quebrada),
        [(_, c)] => Ok(**c),
        [(a, c), (b, _), ..] => {
            // empate técnico entre dois candidatos distintos
            if *a > 1e-6 && *b < *a * 1.15 + 1e-6 { Err(PmiState::Ambigua) } else { Ok(**c) }
        }
    }
}

impl PmiTarget {
    /// Leva o ponto `p`, que estava preso a este alvo, para o alvo `new` (o mesmo elemento depois
    /// da mudança do modelo, ou o substituto escolhido): plano → projeção no plano; cilindro ou
    /// aresta circular → sobre o eixo (`on_axis`) ou sobre a superfície; demais → o ponto do alvo.
    pub fn carry(&self, new: &PmiTarget, p: [f64; 3], on_axis: bool) -> [f64; 3] {
        let n = new.dir;
        match new.class {
            TargetClass::FacePlana => {
                let k = dot(sub(p, new.point), n);
                [p[0] - n[0] * k, p[1] - n[1] * k, p[2] - n[2] * k]
            }
            TargetClass::FaceCilindrica | TargetClass::ArestaCircular if len(n) > 0.5 => {
                let l = len(n);
                let ax = [n[0] / l, n[1] / l, n[2] / l];
                let k = dot(sub(p, new.point), ax);
                let foot = [new.point[0] + ax[0] * k, new.point[1] + ax[1] * k, new.point[2] + ax[2] * k];
                if on_axis {
                    return foot;
                }
                let mut rad = sub(p, foot);
                if len(rad) < 1e-9 {
                    let h = if ax[0].abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] };
                    rad = cross(ax, h);
                }
                let s = new.radius / len(rad);
                [foot[0] + rad[0] * s, foot[1] + rad[1] * s, foot[2] + rad[2] * s]
            }
            TargetClass::FaceOutra => p,
            _ => new.point,
        }
    }
}

impl Pmi {
    /// Confere os alvos da anotação contra os candidatos da geometria atual.
    /// Âncoras sem alvo não são conferidas (anotações antigas continuam valendo).
    pub fn diagnose(&self, cands: &[PmiTarget]) -> PmiDiag {
        let mut resolved = vec![];
        for (i, t) in self.targets.iter().enumerate() {
            let Some(t) = t else {
                resolved.push(None);
                continue;
            };
            match rematch(t, cands) {
                Ok(r) => resolved.push(Some(r)),
                Err(state) => {
                    let message = if state == PmiState::Quebrada { "o elemento marcado não existe mais" } else { "há mais de um elemento que poderia ser o marcado" };
                    return PmiDiag { state, failed: Some(i), message: message.into(), resolved };
                }
            }
        }
        // cota entre duas faces que eram paralelas
        if self.kind == PmiKind::Linear {
            if let (Some(Some(a0)), Some(Some(b0)), Some(Some(a)), Some(Some(b))) = (self.targets.first(), self.targets.get(1), resolved.first(), resolved.get(1)) {
                if a0.class == TargetClass::FacePlana && b0.class == TargetClass::FacePlana {
                    let par = |x: &PmiTarget, y: &PmiTarget| len(cross(x.dir, y.dir)) < 0.005;
                    if par(a0, b0) && !par(a, b) {
                        return PmiDiag { state: PmiState::Ambigua, failed: Some(1), message: "as faces deixaram de ser paralelas".into(), resolved };
                    }
                }
            }
        }
        PmiDiag { state: PmiState::Ok, failed: None, message: String::new(), resolved }
    }

    /// Valor medido pela geometria dos alvos (distância entre planos paralelos, raio e diâmetro
    /// do cilindro); `None` quando as âncoras bastam.
    pub fn measure_targets(&self, r: &[Option<PmiTarget>]) -> Option<f64> {
        let circ = |a: &PmiTarget| matches!(a.class, TargetClass::FaceCilindrica | TargetClass::ArestaCircular);
        match (&self.kind, r.first().copied().flatten(), r.get(1).copied().flatten()) {
            (PmiKind::Linear, Some(a), Some(b)) if a.class == TargetClass::FacePlana && b.class == TargetClass::FacePlana && len(cross(a.dir, b.dir)) < 0.005 => Some(dot(sub(b.point, a.point), a.dir).abs()),
            (PmiKind::Gap { mode }, Some(a), Some(b)) if circ(&a) && circ(&b) => {
                // distância entre os eixos medida no plano perpendicular ao eixo do primeiro
                let n = a.dir;
                let l = len(n);
                let mut v = sub(b.point, a.point);
                if l > 0.5 {
                    let k = dot(v, n) / (l * l);
                    v = [v[0] - n[0] * k, v[1] - n[1] * k, v[2] - n[2] * k];
                }
                let c = len(v);
                Some(match mode {
                    GapMode::Centers => c,
                    GapMode::Inner => c - a.radius - b.radius,
                    GapMode::Outer => c + a.radius + b.radius,
                })
            }
            (PmiKind::Diameter, Some(a), _) if circ(&a) => Some(a.radius * 2.),
            (PmiKind::Radius, Some(a), _) if circ(&a) => Some(a.radius),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perna_do_cordao_vem_da_medida_do_simbolo() {
        assert_eq!(weld_leg_mm("z5"), Some(5.));
        assert_eq!(weld_leg_mm(" 6,5 "), Some(6.5));
        assert!((weld_leg_mm("a4").unwrap() - 4. * std::f64::consts::SQRT_2).abs() < 1e-9, "garganta 4 = perna 5,66");
        assert_eq!(weld_leg_mm("a3 (2x)"), Some(3. * std::f64::consts::SQRT_2));
        assert_eq!(weld_leg_mm(""), None);
        assert_eq!(weld_leg_mm("abc"), None);
        assert_eq!(weld_leg_mm("0"), None);
        // arquivo antigo: o cordão aparece (esconder é escolha nova)
        let x: WeldExtra = serde_json::from_str("{}").unwrap();
        assert!(!x.hide_bead);
    }

    #[test]
    fn medidas_e_textos() {
        let mut p = Pmi { id: Id(1), name: "c".into(), kind: PmiKind::Linear, anchors: vec![], offset: [0.; 3], text: String::new(), tol_plus: "0,1".into(), tol_minus: "0,1".into(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] };
        let v = p.measure(&[[0., 0., 0.], [30., 40., 0.]]);
        assert_eq!(v, Some(50.));
        assert_eq!(p.display(v), "50 ±0,1");
        p.kind = PmiKind::Angle;
        let a = p.measure(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]).unwrap();
        assert!((a - 90.).abs() < 1e-9);
        p.kind = PmiKind::Gdt { ch: GdtChar::Planeza, tol: "0,05".into(), datums: "A".into(), extra: Default::default() };
        assert_eq!(p.display(None), "| ⏥ | 0,05 | A |");
        p.kind = PmiKind::Gdt { ch: GdtChar::Posicao, tol: "0,1".into(), datums: "A B C".into(), extra: GdtExtra { zone_dia: true, modifier: "Ⓜ".into(), ..Default::default() } };
        assert_eq!(p.display(None), "| ⌖ | Ø0,1 Ⓜ | A | B | C |");
    }

    #[test]
    fn precisao_da_cota_fixa_as_casas_decimais() {
        let mut p = Pmi { id: Id(1), name: "c".into(), kind: PmiKind::Linear, anchors: vec![], offset: [0.; 3], text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] };
        assert_eq!(p.display(Some(12.5)), "12,5", "padrão: sem zeros sobrando");
        p.style.decimals = Some(0);
        assert_eq!(p.display(Some(12.5)), "13");
        p.style.decimals = Some(3);
        assert_eq!(p.display(Some(12.5)), "12,500");
        p.kind = PmiKind::Diameter;
        assert_eq!(p.display(Some(8.)), "Ø8,000");
        // o estilo geral entra por baixo: a anotação sem valor próprio herda a precisão
        let geral = PmiStyle { decimals: Some(1), ..Default::default() };
        assert_eq!(PmiStyle::default().over(&geral).decimals, Some(1));
        assert_eq!(p.style.over(&geral).decimals, Some(3));
    }

    #[test]
    fn molduras_e_tracejado() {
        for f in PmiFrame::ALL {
            let ps = f.paths(0., 0., 20., 8.);
            if matches!(f, PmiFrame::Nenhuma | PmiFrame::Padrao) {
                assert!(ps.is_empty());
                continue;
            }
            // toda moldura envolve o texto
            let all: Vec<[f64; 2]> = ps.iter().flat_map(|p| p.0.clone()).collect();
            let (mnx, mxx) = all.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p[0]), a.1.max(p[0])));
            let (mny, mxy) = all.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p[1]), a.1.max(p[1])));
            assert!(mnx <= 1e-9 && mxx >= 20. - 1e-9 && mny <= 1e-9 && mxy >= 8. - 1e-9, "{f:?}");
        }
        // 100 de linha tracejada 6/4 = 10 traços de 6
        let d = dashed(&[[0., 0.], [100., 0.]], false, &[6., 4.]);
        assert_eq!(d.len(), 10);
        assert!(d.iter().all(|s| ((s[s.len() - 1][0] - s[0][0]) - 6.).abs() < 1e-9));
    }

    #[test]
    fn simbolo_de_solda_iso_2553() {
        // filete a5 só do lado da seta: linha cheia + tracejada, triângulo acima, medida à esquerda
        let s = weld_symbol(WeldKind::Filete, "a5", false, &WeldExtra::default(), 3.5);
        assert!(s.iter().any(|p| matches!(p, WeldPrim::Dashed(_))));
        assert!(s.iter().any(|p| matches!(p, WeldPrim::Text(_, t, -1) if t == "a5")));
        let tri = s.iter().find_map(|p| if let WeldPrim::Line(v) = p { (v.len() == 3).then_some(v.clone()) } else { None }).unwrap();
        assert!(tri.iter().all(|q| q[1] >= -1e-9), "lado da seta fica acima: {tri:?}");
        // dois lados (V dos dois lados = X): sem tracejada; campo, contorno e cauda com processo
        let x = WeldExtra { other: Some(WeldKind::TopoV), field: true, contour: WeldContour::Plano, process: "135".into(), length: "3×50 (100)".into(), ..Default::default() };
        let s = weld_symbol(WeldKind::TopoV, "s8", true, &x, 3.5);
        assert!(!s.iter().any(|p| matches!(p, WeldPrim::Dashed(_))));
        assert!(s.iter().any(|p| matches!(p, WeldPrim::Fill(_))), "bandeira de solda de campo");
        assert!(s.iter().any(|p| matches!(p, WeldPrim::Text(_, t, 1) if t == "135")));
        assert!(s.iter().any(|p| matches!(p, WeldPrim::Text(_, t, 1) if t == "3×50 (100)")));
        // outro lado: o símbolo vai para baixo da tracejada
        let s = weld_symbol(WeldKind::Filete, "a3", false, &WeldExtra { other_side: true, ..Default::default() }, 3.5);
        let tri = s.iter().find_map(|p| if let WeldPrim::Line(v) = p { (v.len() == 3).then_some(v.clone()) } else { None }).unwrap();
        assert!(tri.iter().all(|q| q[1] <= -0.8 * 3.5 + 1e-9), "{tri:?}");
    }

    fn plano(z: f64, n: [f64; 3]) -> PmiTarget {
        PmiTarget { class: TargetClass::FacePlana, point: [0., 0., z], dir: n, radius: 0., concave: false }
    }

    fn pmi_de(kind: PmiKind, targets: Vec<Option<PmiTarget>>) -> Pmi {
        Pmi { id: Id(1), name: "t".into(), kind, anchors: vec![], offset: [0.; 3], text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets }
    }

    #[test]
    fn circulo_e_rotulo_do_furo() {
        // oito pontos de um furo de raio 3,25 com eixo Z centrado em (10, 5)
        let pts: Vec<[f64; 3]> = (0..8).map(|k| { let t = k as f64 * std::f64::consts::TAU / 8.; [10. + 3.25 * t.cos(), 5. + 3.25 * t.sin(), (k % 3) as f64] }).collect();
        let (c, r) = fit_axis_circle(&pts, [0., 0., 1.]).unwrap();
        assert!((r - 3.25).abs() < 1e-9 && (c[0] - 10.).abs() < 1e-9 && (c[1] - 5.).abs() < 1e-9);
        let t = PmiTarget { class: TargetClass::FaceCilindrica, point: c, dir: [0., 0., 1.], radius: r, concave: true };
        assert_eq!(t.label(), "furo Ø6,5");
        let p = pmi_de(PmiKind::Diameter, vec![Some(t)]);
        assert!((p.measure_targets(&[Some(t)]).unwrap() - 6.5).abs() < 1e-9);
        assert!(p.diagnose(&[t]).ok());
        // o furo cresce: ainda é o mesmo alvo e o diâmetro acompanha
        let maior = PmiTarget { radius: 4., ..t };
        let d = p.diagnose(&[maior]);
        assert!(d.ok());
        assert!((p.measure_targets(&d.resolved).unwrap() - 8.).abs() < 1e-9);
        // sem furo nenhum: quebrada
        assert_eq!(p.diagnose(&[plano(0., [0., 0., 1.])]).state, PmiState::Quebrada);
    }

    #[test]
    fn cota_entre_faces_perde_o_paralelismo() {
        let (a, b) = (plano(0., [0., 0., -1.]), plano(20., [0., 0., 1.]));
        let p = pmi_de(PmiKind::Linear, vec![Some(a), Some(b)]);
        let d = p.diagnose(&[a, b]);
        assert!(d.ok());
        assert_eq!(p.measure_targets(&d.resolved), Some(20.));
        // a face de cima inclina 10 graus
        let t = 10f64.to_radians();
        let b2 = plano(20., [t.sin(), 0., t.cos()]);
        let d = p.diagnose(&[a, b2]);
        assert_eq!(d.state, PmiState::Ambigua);
        assert_eq!(d.failed, Some(1));
        assert_eq!(d.message, "as faces deixaram de ser paralelas");
        // arrastar a âncora para uma face substituta paralela restaura o estado
        let sub = plano(18., [0., 0., 1.]);
        let mut p2 = p.clone();
        p2.targets[1] = Some(sub);
        let d = p2.diagnose(&[a, b2, sub]);
        assert!(d.ok(), "{d:?}");
        assert_eq!(p2.measure_targets(&d.resolved), Some(18.));
    }

    #[test]
    fn ancora_acompanha_o_alvo() {
        let furo = PmiTarget { class: TargetClass::FaceCilindrica, point: [10., 5., 0.], dir: [0., 0., 1.], radius: 3., concave: true };
        let maior = PmiTarget { radius: 5., point: [12., 5., 0.], ..furo };
        // o centro vai para o eixo novo e o ponto da superfície para o raio novo
        let c = furo.carry(&maior, [10., 5., 2.], true);
        assert!((c[0] - 12.).abs() < 1e-9 && (c[1] - 5.).abs() < 1e-9 && (c[2] - 2.).abs() < 1e-9);
        let s = furo.carry(&maior, [13., 5., 2.], false);
        assert!((s[0] - 17.).abs() < 1e-9, "{s:?}");
        // plano: o ponto desliza para o plano novo
        let a = plano(0., [0., 0., 1.]);
        let b = plano(7., [0., 0., 1.]);
        assert_eq!(a.carry(&b, [3., 4., 0.], false), [3., 4., 7.]);
    }
}
