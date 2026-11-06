// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cota e paradas da alça de comprimento do fixador (B282).
//!
//! A alça mora na ponta do parafuso (`asm_ui::fasteners_handles`). Este módulo desenha ao lado dela a
//! cota do comprimento (linha de cota paralela ao eixo, com as linhas de chamada) e um traço em cada
//! valor da série da norma: são as únicas posições em que a alça para. A conta de "onde a alça para"
//! fica em [`alvo_do_arrasto`] e [`parar_na_serie`], sem depender da tela, para o teste conferir que o
//! arrasto nunca resulta em um comprimento que a norma não tem.

use eframe::egui::{self, Color32, FontId, Pos2, Stroke, Vec2};

/// Afastamento da linha de cota em relação ao parafuso, em pixels.
const AFASTAMENTO: f32 = 26.;

/// Comprimento a que o arrasto aponta, em mm: o que a alça valia ao começar mais o deslocamento do
/// mouse projetado no eixo do parafuso na tela. Sem escala (eixo de frente para a câmera) não há
/// como converter pixels em milímetros, e o alvo fica onde estava.
pub fn alvo_do_arrasto(inicio_mm: f64, deslocamento: Vec2, eixo_na_tela: Vec2, px_por_mm: f32) -> f64 {
    if px_por_mm <= 1e-3 || eixo_na_tela.length() <= 1e-6 {
        return inicio_mm;
    }
    let ao_longo = deslocamento.dot(eixo_na_tela / eixo_na_tela.length()) / px_por_mm;
    inicio_mm + ao_longo as f64
}

/// O valor da série mais próximo do alvo (o maior, em empate); `None` com a série vazia. É a única
/// saída possível do arrasto: não existe comprimento intermediário.
pub fn parar_na_serie(serie: &[f64], alvo: f64) -> Option<f64> {
    fenkra_montagem::fixadores::mais_proximo(serie, alvo)
}

/// Os dois pontos da linha de cota: o par `(cabeça, ponta)` deslocado para o lado, perpendicular ao
/// parafuso na tela. `None` quando cabeça e ponta caem no mesmo ponto (parafuso visto de frente).
pub fn linha_da_cota(cabeca: Pos2, ponta: Pos2, afastamento: f32) -> Option<(Pos2, Pos2)> {
    let eixo = ponta - cabeca;
    if eixo.length() < 2. {
        return None;
    }
    let lado = Vec2::new(-eixo.y, eixo.x) / eixo.length() * afastamento;
    Some((cabeca + lado, ponta + lado))
}

/// Desenha a cota e os traços da série ao lado do parafuso. `cor` e `fundo` vêm do tema.
pub fn desenhar(p: &egui::Painter, cabeca: Pos2, ponta: Pos2, atual: f64, serie: &[f64], cor: Color32, fundo: Color32, texto: String) {
    let Some((a, b)) = linha_da_cota(cabeca, ponta, AFASTAMENTO) else { return };
    let traco = Stroke::new(1., cor);
    let afasta = (a - cabeca).normalized();
    // linhas de chamada: da cabeça e da ponta até a linha de cota, passando um pouco dela
    p.line_segment([cabeca, a + afasta * 4.], traco);
    p.line_segment([ponta, b + afasta * 4.], traco);
    p.line_segment([a, b], Stroke::new(1.5, cor));
    // traços da série: um em cada valor que existe, posicionados na escala do comprimento atual
    if atual.abs() > 1e-9 {
        for l in serie.iter().filter(|l| (**l - atual).abs() > 1e-9) {
            let q = a + (b - a) * (*l / atual) as f32;
            p.line_segment([q - afasta * 3., q + afasta * 3.], Stroke::new(1., cor.gamma_multiply(0.55)));
        }
    }
    // setas nas pontas da linha de cota
    let sentido = (b - a).normalized();
    for (ponto, s) in [(a, sentido), (b, -sentido)] {
        let n = Vec2::new(-s.y, s.x);
        p.line_segment([ponto, ponto + s * 7. + n * 3.], traco);
        p.line_segment([ponto, ponto + s * 7. - n * 3.], traco);
    }
    let meio = a + (b - a) * 0.5;
    let galeria = p.layout_no_wrap(texto, FontId::proportional(12.), cor);
    let canto = meio + afasta * 6.;
    p.rect_filled(egui::Rect::from_min_size(canto, galeria.size()).expand(2.), 3., fundo);
    p.galley(canto, galeria, cor);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_linha_de_cota_fica_ao_lado_e_paralela_ao_parafuso() {
        let (a, b) = linha_da_cota(Pos2::new(100., 100.), Pos2::new(100., 200.), 26.).unwrap();
        // parafuso vertical: a cota é vertical, 26 px de lado, com o mesmo comprimento
        assert!((a.x - b.x).abs() < 1e-4 && ((a.x - 100.).abs() - 26.).abs() < 1e-4);
        assert!(((b - a).length() - 100.).abs() < 1e-4);
        // parafuso visto de frente (ponta sobre a cabeça): sem cota
        assert!(linha_da_cota(Pos2::new(5., 5.), Pos2::new(5.5, 5.), 26.).is_none());
    }

    #[test]
    fn o_arrasto_converte_pixels_em_milimetros_ao_longo_do_eixo() {
        // eixo na tela para baixo, 4 px por mm: 40 px para baixo = 10 mm a mais
        let eixo = Vec2::new(0., 40.);
        assert!((alvo_do_arrasto(20., Vec2::new(0., 40.), eixo, 4.) - 30.).abs() < 1e-6);
        // movimento lateral não conta
        assert!((alvo_do_arrasto(20., Vec2::new(80., 0.), eixo, 4.) - 20.).abs() < 1e-6);
        // contra o eixo diminui
        assert!((alvo_do_arrasto(20., Vec2::new(0., -20.), eixo, 4.) - 15.).abs() < 1e-6);
        // sem escala o alvo não anda
        assert_eq!(alvo_do_arrasto(20., Vec2::new(0., 40.), eixo, 0.), 20.);
    }

    #[test]
    fn arrastar_nunca_deixa_o_comprimento_fora_da_serie() {
        let serie = [8., 10., 12., 16., 20., 25., 30., 35., 40., 45., 50., 55., 60.];
        let eixo = Vec2::new(10., 30.);
        // varre o mouse de muito acima a muito abaixo, de 1 em 1 px, partindo de cada valor da série
        for inicio in serie {
            for dy in -400..=400 {
                let alvo = alvo_do_arrasto(inicio, Vec2::new(3., dy as f32), eixo, 3.);
                let l = parar_na_serie(&serie, alvo).unwrap();
                assert!(serie.contains(&l), "início {inicio}, dy {dy}: L{l} não está na série");
            }
        }
        // e as pontas da série seguram a alça
        assert_eq!(parar_na_serie(&serie, -1e6), Some(8.));
        assert_eq!(parar_na_serie(&serie, 1e6), Some(60.));
        assert_eq!(parar_na_serie(&[], 20.), None);
    }
}
