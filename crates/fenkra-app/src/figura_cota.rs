// SPDX-License-Identifier: LGPL-2.1-or-later
//! Figura da cota na dica do campo (B278): um desenho pequeno, só de traços, que mostra o que o
//! número mede. Distância e altura são uma cota entre duas linhas; raio, um arco com o traço do raio;
//! diâmetro, um círculo atravessado; ângulo, duas linhas com o arco entre elas; espessura, duas paredes
//! com a cota entre elas. As cores vêm do tema; o tipo sai do rótulo do campo.

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Figura {
    Distancia,
    Raio,
    Diametro,
    Angulo,
    Espessura,
}

/// Qual figura vale para o rótulo (em português) do campo; campo sem medida geométrica não tem.
pub fn do_rotulo(rotulo: &str) -> Option<Figura> {
    let r = rotulo.to_lowercase();
    let tem = |ks: &[&str]| ks.iter().any(|k| r.contains(k));
    if tem(&["ângulo", "angulo", "inclinação", "conicidade"]) {
        Some(Figura::Angulo)
    } else if tem(&["diâmetro", "ø"]) {
        Some(Figura::Diametro)
    } else if tem(&["raio"]) {
        Some(Figura::Raio)
    } else if tem(&["espessura", "parede", "perna"]) {
        Some(Figura::Espessura)
    } else if tem(&["distância", "comprimento", "altura", "profundidade", "largura", "deslocamento", "afastamento", "recuo", "folga", "prof."]) {
        Some(Figura::Distancia)
    } else {
        None
    }
}

/// Os traços da figura dentro de `r`. `cor` é a cor da peça (traço fino), `cota` a da medida.
pub fn tracos(f: Figura, r: Rect, cor: Color32, cota: Color32) -> Vec<Shape> {
    let peca = Stroke::new(1.5, cor);
    let med = Stroke::new(1.5, cota);
    let p = |x: f32, y: f32| Pos2::new(r.left() + x * r.width(), r.top() + y * r.height());
    let seta = |de: Pos2, para: Pos2| {
        let d = (para - de).normalized();
        let n = egui::vec2(-d.y, d.x);
        Shape::convex_polygon(vec![para, para - d * 6. + n * 3., para - d * 6. - n * 3.], cota, Stroke::NONE)
    };
    match f {
        Figura::Distancia => vec![
            Shape::line_segment([p(0.2, 0.15), p(0.2, 0.85)], peca),
            Shape::line_segment([p(0.8, 0.15), p(0.8, 0.85)], peca),
            Shape::line_segment([p(0.2, 0.5), p(0.8, 0.5)], med),
            seta(p(0.5, 0.5), p(0.2, 0.5)),
            seta(p(0.5, 0.5), p(0.8, 0.5)),
        ],
        Figura::Espessura => vec![
            Shape::rect_filled(Rect::from_min_max(p(0.35, 0.1), p(0.65, 0.9)), 0., cor.gamma_multiply(0.25)),
            Shape::line_segment([p(0.35, 0.1), p(0.35, 0.9)], peca),
            Shape::line_segment([p(0.65, 0.1), p(0.65, 0.9)], peca),
            Shape::line_segment([p(0.1, 0.5), p(0.9, 0.5)], med),
            seta(p(0.15, 0.5), p(0.35, 0.5)),
            seta(p(0.85, 0.5), p(0.65, 0.5)),
        ],
        Figura::Raio => {
            let c = p(0.3, 0.8);
            let raio = r.height() * 0.6;
            let arco: Vec<Pos2> = (0..=20).map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 * i as f32 / 20.;
                c + egui::vec2(a.cos(), a.sin()) * raio
            }).collect();
            let alvo = c + egui::vec2(0.7071, -0.7071) * raio;
            vec![Shape::line(arco, peca), Shape::line_segment([c, alvo], med), seta(c, alvo), Shape::circle_filled(c, 2., cota)]
        }
        Figura::Diametro => {
            let c = r.center();
            let raio = r.height() * 0.38;
            let (a, b) = (c + egui::vec2(-0.7071, 0.7071) * raio, c + egui::vec2(0.7071, -0.7071) * raio);
            vec![Shape::circle_stroke(c, raio, peca), Shape::line_segment([a, b], med), seta(c, a), seta(c, b)]
        }
        Figura::Angulo => {
            let o = p(0.2, 0.85);
            let (a, b) = (p(0.9, 0.85), p(0.75, 0.15));
            let raio = r.width() * 0.35;
            let fim = (b - o).angle();
            let arco: Vec<Pos2> = (0..=16).map(|i| o + egui::Vec2::angled(fim * i as f32 / 16.) * raio).collect();
            vec![Shape::line_segment([o, a], peca), Shape::line_segment([o, b], peca), Shape::line(arco, med)]
        }
    }
}

/// Pinta a figura do campo na dica, se o rótulo tem uma.
pub fn desenhar(ui: &mut egui::Ui, rotulo_pt: &str) {
    let Some(f) = do_rotulo(rotulo_pt) else { return };
    let (r, _) = ui.allocate_exact_size(egui::vec2(96., 44.), egui::Sense::hover());
    let v = ui.visuals();
    ui.painter().extend(tracos(f, r, v.text_color(), crate::theme::accent(v)));
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn rotulos_comuns_ganham_a_figura_certa() {
        assert_eq!(do_rotulo("Distância"), Some(Figura::Distancia));
        assert_eq!(do_rotulo("Prof. rebaixo"), Some(Figura::Distancia));
        assert_eq!(do_rotulo("Ø escareado"), Some(Figura::Diametro));
        assert_eq!(do_rotulo("Diâmetro do furo"), Some(Figura::Diametro));
        assert_eq!(do_rotulo("Raio da 2ª face"), Some(Figura::Raio));
        assert_eq!(do_rotulo("Ângulo do fundo"), Some(Figura::Angulo));
        assert_eq!(do_rotulo("Espessura"), Some(Figura::Espessura));
        assert_eq!(do_rotulo("Quantidade"), None, "contagem não é cota");
        assert_eq!(do_rotulo("Dentes"), None);
    }

    #[test]
    fn toda_figura_cabe_na_caixa_e_tem_traco_de_medida() {
        let r = Rect::from_min_size(Pos2::new(10., 10.), egui::vec2(96., 44.));
        let cota = Color32::from_gray(200);
        for f in [Figura::Distancia, Figura::Raio, Figura::Diametro, Figura::Angulo, Figura::Espessura] {
            let s = tracos(f, r, Color32::from_gray(100), cota);
            let caixa = s.iter().fold(Rect::NOTHING, |a, x| a.union(x.visual_bounding_rect()));
            assert!(r.expand(4.).contains_rect(caixa), "{f:?} sai da caixa: {caixa:?}");
            let medida = s.iter().any(|x| matches!(x, Shape::LineSegment { stroke, .. } if stroke.color == cota));
            assert!(medida || f == Figura::Angulo, "{f:?} sem o traço da medida");
            assert!(s.len() >= 3, "{f:?} pobre demais");
        }
    }
}
