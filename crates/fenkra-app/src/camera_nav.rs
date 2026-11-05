// SPDX-License-Identifier: LGPL-2.1-or-later
//! Navegação da câmera (B17): passo adaptativo ao zoom, compartilhado com as alças 3D, e zoom que
//! mantém o ponto sob o cursor parado.
//!
//! Por que o passo é 1, 2 ou 5 vezes uma potência de 10: o salto do arrasto precisa ser um número
//! redondo em mm (0,2; 5; 20) e, ao mesmo tempo, ter um tamanho na tela que não dependa da peça.
//! Escolhe-se o maior valor da série que ocupa no máximo 8 px; como a razão entre vizinhos da série é
//! 2 ou 2,5, o salto na tela fica entre 3,2 e 8 px, seja a peça de 5 mm ou de 5 m.

use crate::render::Camera;
use eframe::egui::{Pos2, Rect};
use glam::Vec3;

/// Tamanho máximo, em pixels, de um passo do arrasto.
pub const PASSO_MAX_PX: f64 = 8.;

/// Passo em mm para `px_por_mm` pixels por mm (série 1-2-5, limitada a 0,001 mm e 1000 mm).
pub fn passo_125(px_por_mm: f32) -> f64 {
    let ppm = (px_por_mm as f64).max(1e-6);
    let bruto = PASSO_MAX_PX / ppm;
    let base = 10f64.powf(bruto.log10().floor());
    // a tolerância evita que 8/80 = 0,1 caia para 0,05 por erro de arredondamento
    let passo = [5., 2., 1.].iter().map(|m| m * base).find(|p| *p <= bruto * (1. + 1e-9)).unwrap_or(base);
    passo.clamp(0.001, 1000.)
}

/// Ponto do plano que passa pelo alvo e é perpendicular ao olhar, sob a posição da tela.
fn ponto_no_plano_do_alvo(cam: &Camera, rect: Rect, pos: Pos2) -> Option<Vec3> {
    let (o, d) = cam.ray(rect, pos);
    let normal = (cam.target - cam.eye()).normalize_or_zero();
    let denom = d.dot(normal);
    if denom.abs() < 1e-9 {
        return None;
    }
    Some(o + d * ((cam.target - o).dot(normal) / denom))
}

impl Camera {
    /// Muda a distância e desloca o alvo para o ponto que estava sob o cursor continuar sob ele.
    /// Vale para perspectiva e ortogonal; sem cursor útil, só muda a distância.
    pub fn zoom_em_ponto(&mut self, rect: Rect, pos: Pos2, nova_dist: f32) {
        let nova_dist = nova_dist.clamp(0.5, 1e6);
        if rect.width() < 1. || rect.height() < 1. || !rect.contains(pos) {
            self.dist = nova_dist;
            return;
        }
        let antes = ponto_no_plano_do_alvo(self, rect, pos);
        self.dist = nova_dist;
        if let (Some(a), Some(d)) = (antes, ponto_no_plano_do_alvo(self, rect, pos)) {
            // mover o alvo translada a câmera inteira: o raio do cursor anda junto com ele
            self.target += a - d;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    #[test]
    fn passo_fica_na_serie_1_2_5_e_cabe_em_8_px() {
        for ppm in [0.01f32, 0.037, 0.3, 0.8, 1.0, 2.4, 7.5, 80., 640.] {
            let p = passo_125(ppm);
            if p > 0.001 && p < 1000. {
                let m = p / 10f64.powf(p.log10().floor());
                assert!([1., 2., 5.].iter().any(|k| (m - k).abs() < 1e-6), "{p} fora da série (ppm {ppm})");
                let px = p * ppm as f64;
                assert!(px <= PASSO_MAX_PX + 1e-6 && px >= 3.2 - 1e-6, "salto de {px:.2} px com {ppm} px/mm");
            }
        }
    }

    #[test]
    fn peca_de_5_mm_e_de_5_m_dao_o_mesmo_salto_na_tela() {
        // a peça ocupa ~400 px na tela: 80 px/mm para 5 mm e 0,08 px/mm para 5 m
        let (p_pequena, p_grande) = (passo_125(80.), passo_125(0.08));
        assert!((p_pequena - 0.1).abs() < 1e-9, "5 mm: {p_pequena}");
        assert!((p_grande - 100.).abs() < 1e-9, "5 m: {p_grande}");
        let (px_a, px_b) = (p_pequena * 80., p_grande * 0.08);
        assert!((px_a - px_b).abs() < 1e-6, "{px_a} px contra {px_b} px");
        // e o salto é proporcional ao tamanho: mil vezes a peça, mil vezes o passo
        assert!((p_grande / p_pequena - 1000.).abs() < 1e-6);
    }

    #[test]
    fn passo_aperta_ao_dar_zoom_e_nunca_sai_dos_limites() {
        assert_eq!(passo_125(1.), 5.);
        assert_eq!(passo_125(2.), 2.);
        assert_eq!(passo_125(4.), 2.);
        assert_eq!(passo_125(1e9), 0.001);
        assert_eq!(passo_125(1e-9), 1000.);
        let mut ant = f64::MAX;
        for k in 0..40 {
            let p = passo_125(0.01 * 1.3f32.powi(k));
            assert!(p <= ant + 1e-12, "passo cresceu ao aproximar");
            ant = p;
        }
    }

    fn tela() -> Rect {
        Rect::from_min_size(pos2(0., 0.), eframe::egui::vec2(800., 600.))
    }

    #[test]
    fn zoom_mantem_parado_o_ponto_sob_o_cursor() {
        for ortho in [false, true] {
            let mut cam = Camera { ortho, ..Default::default() };
            cam.target = Vec3::new(10., -20., 5.);
            let pos = pos2(620., 140.);
            let antes = ponto_no_plano_do_alvo(&cam, tela(), pos).unwrap();
            let tela_antes = cam.project(tela(), antes).unwrap();
            cam.zoom_em_ponto(tela(), pos, cam.dist * 0.4);
            assert!((cam.dist - 120.).abs() < 1e-3);
            // o mesmo ponto do mundo continua no mesmo pixel
            let tela_depois = cam.project(tela(), antes).unwrap();
            assert!((tela_antes - tela_depois).length() < 0.05, "ortho={ortho}: {tela_antes:?} -> {tela_depois:?}");
            // e o alvo andou, porque o cursor não estava no centro
            assert!((cam.target - Vec3::new(10., -20., 5.)).length() > 1.);
        }
    }

    #[test]
    fn zoom_no_centro_nao_move_o_alvo_e_fora_da_tela_so_muda_a_distancia() {
        let mut cam = Camera::default();
        let alvo = cam.target;
        cam.zoom_em_ponto(tela(), pos2(400., 300.), 150.);
        assert!((cam.target - alvo).length() < 0.05 && (cam.dist - 150.).abs() < 1e-6);
        cam.zoom_em_ponto(tela(), pos2(-50., 900.), 1e9);
        assert!((cam.target - alvo).length() < 0.05);
        assert_eq!(cam.dist, 1e6);
    }
}
