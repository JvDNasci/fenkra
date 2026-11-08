// SPDX-License-Identifier: LGPL-2.1-or-later
//! Vocabulário único das alças 3D (B228). Toda alça do comando aberto é uma de seis formas, cada uma
//! com um desenho próprio que diz o que o arrasto muda, antes de ler o rótulo:
//!
//! | Forma    | Muda                 | Desenho                                   |
//! |----------|----------------------|-------------------------------------------|
//! | Seta     | distância            | ponta triangular no sentido do valor      |
//! | Anel     | ângulo               | anel vazado com ponto no meio             |
//! | Plano    | duas coordenadas     | quadrado translúcido com borda            |
//! | Esfera   | posição livre        | bola cheia com brilho                     |
//! | Raio     | raio ou diâmetro     | meio círculo com o traço do raio          |
//! | Losango  | quantidade ou fator  | losango cheio                             |
//!
//! Todas têm a mesma pega, `PEGA` = 28 px, e o mesmo traço por estado (`alca_estado`). Quem desenha
//! uma alça pede a forma aqui; assim a seta da extrusão, a do gizmo e a do plano deslocado são o
//! mesmo desenho, e um comando novo não inventa um sétimo.

use super::PEGA;
use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Vec2};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Forma {
    Seta,
    Anel,
    Plano,
    Esfera,
    Raio,
    Losango,
}

impl Forma {
    pub(crate) const TODAS: [Forma; 6] = [Forma::Seta, Forma::Anel, Forma::Plano, Forma::Esfera, Forma::Raio, Forma::Losango];

    /// O que a forma muda, para a dica da pega.
    pub(crate) fn nome(self) -> &'static str {
        match self {
            Forma::Seta => "Seta de distância",
            Forma::Anel => "Anel de ângulo",
            Forma::Plano => "Plano arrastável",
            Forma::Esfera => "Esfera de posição",
            Forma::Raio => "Alça de raio",
            Forma::Losango => "Losango de quantidade",
        }
    }

    /// Forma de uma alça linear pelo rótulo do campo: raio e diâmetro ganham a alça de raio, o
    /// resto é distância.
    pub(crate) fn do_rotulo(rotulo: &str) -> Forma {
        let r = rotulo.to_lowercase();
        if r.starts_with("raio") || r.starts_with("diâmetro") {
            Forma::Raio
        } else {
            Forma::Seta
        }
    }

    /// Área que o mouse acerta, igual para as seis.
    pub(crate) fn pega(self, centro: Pos2) -> Rect {
        Rect::from_center_size(centro, egui::vec2(PEGA, PEGA))
    }

    /// Desenho da forma em `centro`, com `dir` o sentido do valor na tela (unitário).
    pub(crate) fn desenho(self, centro: Pos2, dir: Vec2, cor: Color32, esp: f32) -> Vec<Shape> {
        let d = if dir.length() > 1e-3 { dir.normalized() } else { Vec2::X };
        let n = egui::vec2(-d.y, d.x);
        let borda = Stroke::new(1., Color32::from_black_alpha(120));
        let traco = Stroke::new(esp, cor);
        match self {
            Forma::Seta => vec![
                Shape::convex_polygon(vec![centro + d * 14., centro + n * 7., centro - n * 7.], cor, borda),
                Shape::circle_stroke(centro, PEGA / 2., traco),
            ],
            Forma::Anel => vec![Shape::circle_stroke(centro, 9., traco), Shape::circle_filled(centro, 4., cor)],
            Forma::Plano => {
                let h = 7.;
                let q = vec![centro + (d + n) * h, centro + (d - n) * h, centro - (d + n) * h, centro - (d - n) * h];
                vec![Shape::convex_polygon(q, cor.gamma_multiply(0.45), Stroke::new(esp, cor))]
            }
            Forma::Esfera => vec![
                Shape::circle_filled(centro, 8., cor),
                Shape::circle_stroke(centro, 8., borda),
                Shape::circle_filled(centro + egui::vec2(-2.5, -2.5), 2.5, Color32::from_white_alpha(170)),
            ],
            Forma::Raio => {
                // meio círculo virado para fora, com o traço do raio saindo do centro
                let pts: Vec<Pos2> = (0..=16).map(|i| {
                    let a = std::f32::consts::PI * (i as f32 / 16. - 0.5);
                    centro + (d * a.cos() + n * a.sin()) * 9.
                }).collect();
                vec![
                    Shape::line(pts, Stroke::new(esp + 1., cor)),
                    Shape::line_segment([centro, centro + d * 9.], traco),
                    Shape::circle_filled(centro, 3., cor),
                    Shape::circle_stroke(centro, PEGA / 2., Stroke::new(1., cor.gamma_multiply(0.6))),
                ]
            }
            Forma::Losango => {
                let h = 9.;
                vec![Shape::convex_polygon(vec![centro + d * h, centro + n * h, centro - d * h, centro - n * h], cor, borda)]
            }
        }
    }

    pub(crate) fn pinta(self, p: &egui::Painter, centro: Pos2, dir: Vec2, cor: Color32, esp: f32) {
        p.extend(self.desenho(centro, dir, cor, esp));
    }
}

/// Troca o lado de uma expressão: número muda o sinal; expressão ganha ou perde `-( )`.
pub(crate) fn inverter_expr(e: &str) -> String {
    let t = e.trim();
    if let Ok(v) = t.replace(',', ".").parse::<f64>() {
        return crate::app::fmt(-v);
    }
    if let Some(dentro) = t.strip_prefix("-(").and_then(|x| x.strip_suffix(')')) {
        // só tira os parênteses se eles fecham a expressão inteira
        let mut nivel = 0i32;
        let fecha_no_fim = dentro.chars().all(|c| {
            match c {
                '(' => nivel += 1,
                ')' => nivel -= 1,
                _ => {}
            }
            nivel >= 0
        });
        if fecha_no_fim && nivel == 0 {
            return dentro.to_string();
        }
    }
    format!("-({t})")
}

/// Textos do vocabulário (inglês).
pub static TEXTOS: &[(&str, &str)] = &[
    ("Seta de distância", "Distance arrow"),
    ("Anel de ângulo", "Angle ring"),
    ("Plano arrastável", "Draggable plane"),
    ("Esfera de posição", "Position sphere"),
    ("Alça de raio", "Radius handle"),
    ("Losango de quantidade", "Quantity diamond"),
    ("Inverter o lado", "Flip side"),
    ("Troca o lado do plano: o deslocamento muda de sinal.", "Flips the plane side: the offset changes sign."),
    ("arraste ao longo do padrão, ou gire a roda, para mudar a quantidade", "drag along the pattern, or turn the wheel, to change the count"),
    ("arraste para mover o ponto no plano da tela", "drag to move the point on the screen plane"),
];

#[cfg(test)]
mod testes {
    use super::*;

    fn caixa(s: &[Shape]) -> Rect {
        s.iter().fold(Rect::NOTHING, |r, x| r.union(x.visual_bounding_rect()))
    }

    #[test]
    fn seis_formas_com_nome_traduzido_e_a_mesma_pega() {
        let mut nomes: Vec<_> = Forma::TODAS.iter().map(|f| f.nome()).collect();
        nomes.dedup();
        assert_eq!(nomes.len(), 6);
        for f in Forma::TODAS {
            assert!(crate::i18n::tem_traducao(f.nome()), "{f:?} sem tradução");
            assert_eq!(f.pega(Pos2::ZERO).size(), egui::vec2(28., 28.), "{f:?} com pega diferente");
        }
    }

    #[test]
    fn cada_desenho_cabe_na_pega_e_e_diferente_dos_outros() {
        let c = Pos2::new(100., 100.);
        let mut assinaturas = vec![];
        for f in Forma::TODAS {
            let s = f.desenho(c, Vec2::X, Color32::RED, 1.5);
            let b = caixa(&s);
            // a borda do traço pode passar meio pixel
            assert!(f.pega(c).expand(2.).contains_rect(b), "{f:?} sai da pega: {b:?}");
            // o preenchimento translúcido (plano) também diferencia: o losango é cheio
            let tipos: Vec<u8> = s.iter().map(|x| match x {
                Shape::Circle(c) if c.fill != Color32::TRANSPARENT => 1,
                Shape::Circle(_) => 2,
                Shape::Path(p) if p.closed && p.fill.a() < 255 => 100 + p.points.len() as u8,
                Shape::Path(p) if p.closed => 3 + p.points.len() as u8,
                Shape::Path(p) => 20 + p.points.len() as u8,
                Shape::LineSegment { .. } => 50,
                _ => 99,
            }).collect();
            assert!(!assinaturas.contains(&tipos), "{f:?} desenha igual a outra forma");
            assinaturas.push(tipos);
        }
    }

    #[test]
    fn a_seta_aponta_no_sentido_do_valor() {
        let c = Pos2::new(0., 0.);
        let s = Forma::Seta.desenho(c, Vec2::new(0., -1.), Color32::RED, 1.5);
        let Shape::Path(p) = &s[0] else { panic!() };
        assert!(p.points.iter().any(|q| q.y < -13.), "ponta para cima");
    }

    #[test]
    fn rotulo_escolhe_a_forma() {
        assert_eq!(Forma::do_rotulo("Raio"), Forma::Raio);
        assert_eq!(Forma::do_rotulo("Diâmetro"), Forma::Raio);
        assert_eq!(Forma::do_rotulo("Deslocamento"), Forma::Seta);
        assert_eq!(Forma::do_rotulo("Espessura"), Forma::Seta);
    }

    #[test]
    fn inverter_vai_e_volta() {
        assert_eq!(inverter_expr("15"), "-15");
        assert_eq!(inverter_expr("-2,5"), crate::app::fmt(2.5));
        assert_eq!(inverter_expr("altura/2"), "-(altura/2)");
        assert_eq!(inverter_expr(&inverter_expr("altura/2")), "altura/2");
        // os parênteses que não fecham a expressão inteira ficam
        assert_eq!(inverter_expr("-(a)+(b)"), "-(-(a)+(b))");
    }
}
