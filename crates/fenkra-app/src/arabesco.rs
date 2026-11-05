// SPDX-License-Identifier: LGPL-2.1-or-later
//! Arabesco do tema Fênix (B314): fios finos e orgânicos para separadores, tela inicial e estados
//! vazios. Nunca aparece na área de trabalho nem sob a peça, e só existe quando o tema liga
//! `arabesco`.
//!
//! As curvas são nossas: uma onda de amplitude amortecida (o "fio") com uma volta em cada ponta, e
//! um leque de três penas que sobem de um ponto (a "pluma"). Não reproduzem o desenho do ícone da
//! fênix, só a mesma ideia de linha longa que se enrola no fim.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

/// Pontos do fio, de `-largura/2` a `+largura/2` no eixo x, com a onda centrada em y = 0.
/// A altura total nunca passa de `altura`.
pub fn fio(largura: f32, altura: f32) -> Vec<Pos2> {
    let largura = largura.max(8.);
    let amp = altura.max(2.) * 0.32;
    let n = 96;
    let mut p: Vec<Pos2> = Vec::with_capacity(n + 60);
    // voltinha da ponta direita: espiral que encolhe, saindo do fim da onda por cima
    let (volta_r, voltas) = (altura.max(2.) * 0.30, 1.6 * std::f32::consts::TAU);
    let centro_x = largura / 2. - volta_r;
    let curva: Vec<Pos2> = (0..=28)
        .map(|i| {
            let t = i as f32 / 28.;
            let r = volta_r * (1. - t * 0.85);
            let fi = std::f32::consts::PI - t * voltas;
            Pos2::new(centro_x + r * fi.cos(), -r * fi.sin())
        })
        .collect();
    // a ponta esquerda é o espelho da direita, percorrida ao contrário
    p.extend(curva.iter().rev().map(|q| Pos2::new(-q.x, q.y)));
    // onda amortecida entre as voltas
    let (x0, x1) = (-largura / 2. + volta_r * 2., largura / 2. - volta_r * 2.);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let x = x0 + (x1 - x0) * t;
        let u = (t - 0.5) * 2.; // -1..1
        let envelope = (-(u * 1.6).powi(2)).exp();
        let y = -amp * envelope * (u * std::f32::consts::TAU * 1.5).sin();
        p.push(Pos2::new(x, y));
    }
    p.extend(curva);
    p
}

/// Três penas que saem de `(0, 0)` e abrem em leque para cima; cada pena é uma curva S suave.
/// O desenho cabe numa caixa de `tam` x `tam`, com a base em y = 0 e o topo em y = -tam.
pub fn pluma(tam: f32) -> Vec<Vec<Pos2>> {
    let tam = tam.max(8.);
    let n = 40;
    [(-1.0f32, 0.62f32), (0.0, 1.0), (1.0, 0.62)]
        .iter()
        .map(|&(lado, escala)| {
            (0..=n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    // sobe com um S: a pena abre para o lado e volta a fechar na ponta
                    let x = lado * tam * 0.34 * (t * std::f32::consts::PI).sin() * (0.35 + 0.65 * t) + lado * tam * 0.10 * t * t;
                    let balanco = if lado == 0. { tam * 0.05 * (t * std::f32::consts::TAU).sin() } else { 0. };
                    let x = x + balanco;
                    let y = -tam * escala * t * (1.0 - 0.12 * t);
                    Pos2::new(x, y)
                })
                .collect()
        })
        .collect()
}

/// Cor do fio: a linha do tema, puxada para o destaque, bem discreta.
fn cor_do_fio(ui: &egui::Ui) -> Color32 {
    let v = ui.visuals();
    let acento = crate::theme::accent(v);
    v.weak_text_color().lerp_to_gamma(acento, 0.35).gamma_multiply(0.85)
}

/// Separador horizontal da largura disponível; só aparece com o tema que liga o arabesco e, nos
/// demais, não desenha nada (a tela dos outros temas não muda).
pub fn separador(ui: &mut egui::Ui) {
    if !crate::theme::arabesco_ativo() {
        return;
    }
    let largura = ui.available_width().min(360.);
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.), egui::Sense::hover());
    let c = r.center();
    let pts: Vec<Pos2> = fio(largura, 14.).into_iter().map(|p| c + p.to_vec2()).collect();
    ui.painter().add(egui::Shape::line(pts, Stroke::new(1., cor_do_fio(ui))));
}

/// Fio da tela inicial, centrado; só com o tema que liga o arabesco.
pub fn fio_da_marca(ui: &mut egui::Ui) {
    if !crate::theme::arabesco_ativo() {
        return;
    }
    let (r, _) = ui.allocate_exact_size(egui::vec2(280., 18.), egui::Sense::hover());
    let pts: Vec<Pos2> = fio(280., 14.).into_iter().map(|p| r.center() + p.to_vec2()).collect();
    ui.painter().add(egui::Shape::line(pts, Stroke::new(1., cor_do_fio(ui))));
}

/// Ornamento de estado vazio, centrado em `rect`; só com o tema que liga o arabesco.
pub fn estado_vazio(ui: &mut egui::Ui, rect: Rect) {
    if !crate::theme::arabesco_ativo() {
        return;
    }
    let tam = rect.width().min(rect.height()).clamp(24., 90.);
    let base = Pos2::new(rect.center().x, rect.center().y + tam / 2.);
    let cor = cor_do_fio(ui);
    for curva in pluma(tam) {
        let pts: Vec<Pos2> = curva.into_iter().map(|p| base + p.to_vec2()).collect();
        ui.painter().add(egui::Shape::line(pts, Stroke::new(1., cor)));
    }
}

/// Estado vazio com a pluma sobre o texto: reserva a altura e desenha.
pub fn vazio_com_texto(ui: &mut egui::Ui, texto: egui::RichText) {
    if crate::theme::arabesco_ativo() {
        let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().min(300.), 56.), egui::Sense::hover());
        estado_vazio(ui, r);
    }
    ui.label(texto);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fio_e_finito_simetrico_e_cabe_na_altura() {
        let (w, h) = (300., 16.);
        let p = fio(w, h);
        assert!(p.len() > 60);
        assert!(p.iter().all(|q| q.x.is_finite() && q.y.is_finite()));
        let (xmin, xmax) = p.iter().fold((f32::MAX, f32::MIN), |(a, b), q| (a.min(q.x), b.max(q.x)));
        assert!(xmin >= -w / 2. - 1. && xmax <= w / 2. + 1., "x em [{xmin}, {xmax}]");
        let ymax = p.iter().map(|q| q.y.abs()).fold(0., f32::max);
        assert!(ymax <= h * 0.5 + 0.01, "altura {ymax} passa de {}", h * 0.5);
        // fio fino e contínuo: nenhum salto grande entre pontos vizinhos
        let maior = p.windows(2).map(|a| (a[1] - a[0]).length()).fold(0., f32::max);
        assert!(maior < w * 0.2, "salto de {maior}");
    }

    #[test]
    fn pluma_tem_tres_penas_dentro_da_caixa() {
        let tam = 80.;
        let penas = pluma(tam);
        assert_eq!(penas.len(), 3);
        for pena in &penas {
            for q in pena {
                assert!(q.x.abs() <= tam * 0.5 && q.y <= 0.001 && q.y >= -tam - 0.001, "{q:?}");
            }
        }
        // a do meio é a mais alta e as laterais são espelhos uma da outra
        let topo = |p: &Vec<Pos2>| p.iter().map(|q| q.y).fold(0., f32::min);
        assert!(topo(&penas[1]) < topo(&penas[0]));
        assert!((topo(&penas[0]) - topo(&penas[2])).abs() < 1e-4);
        assert!((penas[0][20].x + penas[2][20].x).abs() < 1e-4);
    }

    #[test]
    fn so_aparece_com_o_tema_que_liga_o_arabesco() {
        let fenix = crate::theme::builtin().into_iter().find(|t| t.name == crate::theme::FENIX).expect("tema Fênix");
        assert!(fenix.arabesco);
        assert!(crate::theme::builtin().iter().filter(|t| t.name != crate::theme::FENIX).all(|t| !t.arabesco));
    }
}
