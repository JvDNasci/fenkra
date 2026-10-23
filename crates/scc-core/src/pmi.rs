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
}

impl WeldKind {
    pub const ALL: [WeldKind; 5] = [WeldKind::Filete, WeldKind::TopoV, WeldKind::TopoReto, WeldKind::Bujao, WeldKind::Ponto];
    pub fn label(self) -> &'static str {
        match self {
            WeldKind::Filete => "Filete",
            WeldKind::TopoV => "Topo em V",
            WeldKind::TopoReto => "Topo reto",
            WeldKind::Bujao => "Bujão",
            WeldKind::Ponto => "Ponto",
        }
    }
    pub fn symbol(self) -> &'static str {
        match self {
            WeldKind::Filete => "◺",
            WeldKind::TopoV => "V",
            WeldKind::TopoReto => "‖",
            WeldKind::Bujao => "▭",
            WeldKind::Ponto => "○",
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
    Weld { kind: WeldKind, size: String, all_around: bool },
    /// referência (datum) com letra
    Datum { letter: String },
    /// quadro de tolerância geométrica
    Gdt { ch: GdtChar, tol: String, datums: String },
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
}

/// Captura: uma vista (câmera) com as anotações que aparecem nela.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PmiCapture {
    pub id: Id,
    pub name: String,
    pub view: crate::doc::NamedView,
    /// anotações visíveis nesta captura
    pub shown: Vec<Id>,
}

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
            PmiKind::Weld { kind, size, all_around } => format!("{}{} {}", if *all_around { "○ " } else { "" }, kind.symbol(), size),
            PmiKind::Datum { letter } => format!("[{letter}]"),
            PmiKind::Gdt { ch, tol, datums } => format!("| {} | {tol} |{}", ch.symbol(), if datums.trim().is_empty() { String::new() } else { format!(" {datums} |") }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medidas_e_textos() {
        let mut p = Pmi { id: Id(1), name: "c".into(), kind: PmiKind::Linear, anchors: vec![], offset: [0.; 3], text: String::new(), tol_plus: "0,1".into(), tol_minus: "0,1".into(), visible: true, plane: None };
        let v = p.measure(&[[0., 0., 0.], [30., 40., 0.]]);
        assert_eq!(v, Some(50.));
        assert_eq!(p.display(v), "50 ±0,1");
        p.kind = PmiKind::Angle;
        let a = p.measure(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]).unwrap();
        assert!((a - 90.).abs() < 1e-9);
        p.kind = PmiKind::Gdt { ch: GdtChar::Planeza, tol: "0,05".into(), datums: "A".into() };
        assert_eq!(p.display(None), "| ⏥ | 0,05 | A |");
    }
}
