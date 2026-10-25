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

/// Dados completos do símbolo de solda ISO 2553 (sistema A, linha tracejada de identificação).
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct WeldExtra {
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
        let w2 = elem(&mut out, o, x0, 0., -1., h) * h;
        right = right.max(x0 + w2);
        if !size.trim().is_empty() {
            out.push(Text([x0 - 0.3 * h, -1.05 * h], size.trim().to_string(), -1));
        }
    }
    if !x.length.trim().is_empty() {
        out.push(Text([right + 0.4 * h, base_y + if s > 0. { 0.15 * h } else { -1.05 * h }], x.length.trim().to_string(), 1));
        right += 0.4 * h + text_w(x.length.trim(), h);
    }
    let len = (right + 0.8 * h).max(5. * h);
    out.push(Line(vec![[0., 0.], [len, 0.]]));
    if !both {
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
}

fn centro() -> u8 {
    4
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
    Roughness { ra: String, process: String },
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
    Balloon { item: String },
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
            PmiKind::Weld { .. } => "Solda",
            PmiKind::Datum { .. } => "Referência",
            PmiKind::Gdt { .. } => "Tolerância geométrica",
            PmiKind::TitleBlock { .. } => "Legenda 3D",
            PmiKind::Balloon { .. } => "Balão",
        }
    }
    /// Quantas âncoras o tipo pede.
    pub fn anchors(&self) -> usize {
        match self {
            PmiKind::Linear | PmiKind::Radius | PmiKind::Diameter => 2,
            PmiKind::Angle => 3,
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
        let dim = matches!(self.kind, PmiKind::Linear | PmiKind::Radius | PmiKind::Diameter | PmiKind::Angle);
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
        let num = |v: f64| {
            let s = format!("{:.2}", v);
            let s = s.trim_end_matches('0').trim_end_matches('.').replace('.', ",");
            if s.is_empty() { "0".into() } else { s }
        };
        let tol = if self.tol_plus.trim().is_empty() && self.tol_minus.trim().is_empty() {
            String::new()
        } else if self.tol_plus.trim() == self.tol_minus.trim() {
            format!(" ±{}", self.tol_plus.trim())
        } else {
            format!(" +{}/−{}", self.tol_plus.trim(), self.tol_minus.trim())
        };
        let base = |pre: &str, v: Option<f64>, suf: &str| if !self.text.trim().is_empty() { self.text.clone() } else { format!("{pre}{}{suf}", v.map(num).unwrap_or("?".into())) };
        match &self.kind {
            PmiKind::Linear => base("", value, "") + &tol,
            PmiKind::Radius => base("R", value, "") + &tol,
            PmiKind::Diameter => base("Ø", value, "") + &tol,
            PmiKind::Angle => base("", value, "°") + &tol,
            PmiKind::Note => self.text.clone(),
            PmiKind::Roughness { ra, process } => format!("√ Ra {ra}{}", if process.trim().is_empty() { String::new() } else { format!(" ({process})") }),
            PmiKind::Weld { kind, size, all_around, .. } => format!("{}{} {}", if *all_around { "○ " } else { "" }, kind.symbol(), size),
            PmiKind::Datum { letter } => format!("[{letter}]"),
            PmiKind::Gdt { ch, tol, datums, extra } => {
                let cells = |tol: &str, datums: &str| {
                    let mut s = format!("| {} | {}{}{} |", ch.symbol(), if extra.zone_dia { "Ø" } else { "" }, tol, if extra.modifier.trim().is_empty() { String::new() } else { format!(" {}", extra.modifier.trim()) });
                    for d in datums.split(|c: char| c == '|' || c == ',' || c.is_whitespace()).filter(|d| !d.is_empty()) {
                        s += &format!(" {d} |");
                    }
                    s
                };
                match &extra.second {
                    Some((t2, d2)) => format!("{}\n{}", cells(tol, datums), cells(t2, d2)),
                    None => cells(tol, datums),
                }
            }
            PmiKind::TitleBlock { template } => template.clone(),
            PmiKind::Balloon { item } => item.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medidas_e_textos() {
        let mut p = Pmi { id: Id(1), name: "c".into(), kind: PmiKind::Linear, anchors: vec![], offset: [0.; 3], text: String::new(), tol_plus: "0,1".into(), tol_minus: "0,1".into(), visible: true, plane: None, style: Default::default(), folder: String::new() };
        let v = p.measure(&[[0., 0., 0.], [30., 40., 0.]]);
        assert_eq!(v, Some(50.));
        assert_eq!(p.display(v), "50 ±0,1");
        p.kind = PmiKind::Angle;
        let a = p.measure(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]).unwrap();
        assert!((a - 90.).abs() < 1e-9);
        p.kind = PmiKind::Gdt { ch: GdtChar::Planeza, tol: "0,05".into(), datums: "A".into(), extra: Default::default() };
        assert_eq!(p.display(None), "| ⏥ | 0,05 | A |");
        p.kind = PmiKind::Gdt { ch: GdtChar::Posicao, tol: "0,1".into(), datums: "A B C".into(), extra: GdtExtra { zone_dia: true, modifier: "Ⓜ".into(), second: None } };
        assert_eq!(p.display(None), "| ⌖ | Ø0,1 Ⓜ | A | B | C |");
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
}
