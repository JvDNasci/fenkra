// SPDX-License-Identifier: LGPL-2.1-or-later
//! Navegação da câmera (B17): passo adaptativo ao zoom, compartilhado com as alças 3D, e zoom que
//! mantém o ponto sob o cursor parado.
//!
//! Por que o passo é 1, 2 ou 5 vezes uma potência de 10: o salto do arrasto precisa ser um número
//! redondo em mm (0,2; 5; 20) e, ao mesmo tempo, ter um tamanho na tela que não dependa da peça.
//! Escolhe-se o maior valor da série que ocupa no máximo 8 px; como a razão entre vizinhos da série é
//! 2 ou 2,5, o salto na tela fica entre 3,2 e 8 px, seja a peça de 5 mm ou de 5 m.
//!
//! Órbita livre e centro da órbita (B542): a câmera guarda guinada (yaw) e inclinação (pitch). No modo
//! preso a inclinação para em ±90°, como uma mesa giratória; no modo livre ela passa do polo e dá a
//! volta inteira (inclusive por baixo do chão). Depois do polo a vista fica de cabeça para baixo, e o
//! arrasto horizontal tem de inverter o sentido da guinada, senão o modelo gira ao contrário do mouse.

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

/// Inclinação que cabe na câmera: presa em ±90° ou, no modo livre, reduzida ao intervalo (-180°, 180°].
pub fn limitar_inclinacao(pitch: f32, livre: bool) -> f32 {
    use std::f32::consts::{PI, TAU};
    if !livre {
        return pitch.clamp(-1.5706, 1.5706);
    }
    let r = (pitch + PI).rem_euclid(TAU) - PI;
    // rem_euclid devolve [-PI, PI): o limite inferior é o mesmo ângulo que +PI
    if r <= -PI + 1e-6 { PI } else { r }
}

impl Camera {
    /// Gira pelo arrasto (`dx`, `dy` em pixels). Com a vista de cabeça para baixo (livre, além do polo)
    /// a guinada troca de sentido para o modelo acompanhar o mouse.
    pub fn girar(&mut self, dx: f32, dy: f32, livre: bool) {
        let de_cabeca = livre && self.pitch.cos() < 0.;
        let sinal = if de_cabeca { -1. } else { 1. };
        self.yaw -= dx * 0.008 * sinal;
        self.pitch = limitar_inclinacao(self.pitch + dy * 0.008, livre);
    }

    /// Passa a orbitar em volta de `p`: o ponto vai para o centro da vista, com o mesmo olhar e a
    /// distância que o olho tinha até ele (no mínimo 1 mm).
    pub fn centrar_orbita(&mut self, p: Vec3) {
        let olho = self.eye();
        self.target = p;
        self.dist = (olho - p).length().max(1.);
    }

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

    /// Módulos que pintam rótulos sobre a vista 3D com tamanho escrito à mão.
    const ROTULOS_3D: &[(&str, &str)] = &[
        ("handles", include_str!("handles.rs")),
        ("gizmo", include_str!("gizmo.rs")),
        ("previa", include_str!("previa.rs")),
        ("feat_ui", include_str!("feat_ui.rs")),
        ("sup3d", include_str!("sup3d.rs")),
        ("pmi_handles", include_str!("pmi_handles.rs")),
        ("asm_manip", include_str!("asm_manip.rs")),
        ("asm_simbolos", include_str!("asm_simbolos.rs")),
        ("chapa_manip_ui", include_str!("chapa_manip_ui.rs")),
        ("forma_ui", include_str!("forma_ui.rs")),
        ("sketch_snap", include_str!("sketch_snap.rs")),
    ];

    #[test]
    fn rotulo_3d_acompanha_a_escala_da_fonte() {
        for (nome, fonte) in ROTULOS_3D {
            for l in fonte.lines().filter(|l| !l.trim_start().starts_with("//")) {
                let cru = l.match_indices("FontId::proportional(").any(|(i, m)| l[i + m.len()..].starts_with(|c: char| c.is_ascii_digit()));
                assert!(!cru, "{nome}: rótulo fora de `fonte_3d`: {}", l.trim());
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
    fn inclinacao_presa_para_no_polo_e_a_livre_da_a_volta() {
        assert_eq!(limitar_inclinacao(3., false), 1.5706);
        assert_eq!(limitar_inclinacao(-3., false), -1.5706);
        // livre: 200 graus equivale a -160 graus; 360 graus volta a zero
        assert!((limitar_inclinacao(200f32.to_radians(), true) - (-160f32).to_radians()).abs() < 1e-5);
        assert!(limitar_inclinacao(std::f32::consts::TAU, true).abs() < 1e-5);
        assert!((limitar_inclinacao(-std::f32::consts::PI, true) - std::f32::consts::PI).abs() < 1e-5);
    }

    #[test]
    fn girar_360_para_baixo_volta_ao_ponto_de_partida_so_no_modo_livre() {
        let inicio = Camera::default();
        // 72 passos de 5 graus para baixo (dy em pixels = 5 graus / 0,008 rad por pixel)
        let px = 5f32.to_radians() / 0.008;
        let mut livre = inicio;
        let mut presa = inicio;
        let mut alcancou_baixo = false;
        for _ in 0..72 {
            livre.girar(0., px, true);
            presa.girar(0., px, false);
            // por baixo do chão: o olho fica abaixo do alvo
            alcancou_baixo |= livre.eye().z < livre.target.z - 1.;
        }
        assert!(alcancou_baixo, "o modo livre passa por baixo do chão");
        assert!((livre.eye() - inicio.eye()).length() < 0.05, "uma volta inteira devolve o olho: {:?}", livre.eye());
        assert!(presa.pitch <= 1.5706 + 1e-6, "o modo preso fica em 90 graus");
    }

    #[test]
    fn de_cabeca_para_baixo_a_guinada_inverte_e_a_vista_continua_sem_salto() {
        let mut cam = Camera { pitch: 2.0, ..Default::default() };
        let antes = cam.yaw;
        cam.girar(10., 0., true);
        assert!(cam.yaw > antes, "de cabeça para baixo o arrasto para a direita aumenta a guinada");
        let mut normal = Camera { pitch: 0.5, ..Default::default() };
        let a = normal.yaw;
        normal.girar(10., 0., true);
        assert!(normal.yaw < a);
        // atravessando o polo o "para cima" da tela muda aos poucos (sem virar de uma vez)
        let mut cam = Camera { pitch: 1.5, ..Default::default() };
        let mut ant = cam.view_proj(1.) * Vec3::new(0., 0., 50.).extend(1.);
        for _ in 0..20 {
            cam.pitch += 0.01;
            let atual = cam.view_proj(1.) * Vec3::new(0., 0., 50.).extend(1.);
            let (a, b) = (ant.truncate() / ant.w, atual.truncate() / atual.w);
            assert!((a - b).length() < 0.2, "salto na vista perto do polo: {a:?} -> {b:?}");
            ant = atual;
        }
    }

    #[test]
    fn centro_da_orbita_leva_o_ponto_para_o_meio_da_tela() {
        for ortho in [false, true] {
            let mut cam = Camera { ortho, ..Default::default() };
            let p = Vec3::new(30., -12., 40.);
            cam.centrar_orbita(p);
            assert_eq!(cam.target, p);
            let c = cam.project(tela(), p).unwrap();
            assert!((c - tela().center()).length() < 0.05, "ortho={ortho}: {c:?}");
            assert!(cam.dist >= 1.);
        }
        // ponto colado no olho: a distância não zera
        let mut cam = Camera::default();
        cam.centrar_orbita(cam.eye());
        assert_eq!(cam.dist, 1.);
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
