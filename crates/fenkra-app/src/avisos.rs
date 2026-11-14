// SPDX-License-Identifier: LGPL-2.1-or-later
//! Avisos rápidos (toasts) no pé da vista (B450).
//!
//! A área dos avisos é uma `egui::Area` ancorada no centro de baixo, e a área guarda o tamanho do
//! quadro anterior: o texto do quadro seguinte quebra nessa largura. Bastava um aviso curto ("Montagem
//! salva") para a largura encolher, e o aviso longo seguinte saía numa coluna de uma ou duas sílabas
//! por linha, que nunca mais alargava, porque o texto quebrado cabe na coluna estreita. Aqui a largura
//! é decidida a cada quadro pelo próprio texto: a da linha mais longa sem quebra, limitada a
//! [`LARGURA_MAX`] e à janela menos as margens. Só passado esse limite o texto quebra.

use eframe::egui::{self, RichText};

/// Largura máxima do bloco de avisos, em pontos: acima disso a linha fica longa demais para ler.
pub const LARGURA_MAX: f32 = 520.;
/// Margem de cada lado entre o aviso e a borda da janela.
const MARGEM: f32 = 16.;

/// Quanto tempo o aviso fica na tela: 4 s, ou o tempo de ler o texto (perto de 15 caracteres por
/// segundo) quando ele é longo, até 12 s.
pub fn duracao(msg: &str) -> f64 {
    (msg.chars().count() as f64 / 15.).clamp(4., 12.)
}

/// Largura do texto dos avisos: a maior largura natural (sem quebra), limitada a [`LARGURA_MAX`] e à
/// janela menos as margens e o enchimento da moldura.
pub fn largura(naturais: impl IntoIterator<Item = f32>, janela: f32, enchimento: f32) -> f32 {
    let teto = LARGURA_MAX.min(janela - 2. * MARGEM - enchimento).max(80.);
    naturais.into_iter().fold(0f32, f32::max).ceil().min(teto)
}

/// Desenha os avisos. `avisos`: texto, tempo na tela e se é erro.
pub fn desenhar(ctx: &egui::Context, avisos: &[(String, f64, bool)]) {
    let estilo = ctx.global_style();
    let fonte = egui::TextStyle::Body.resolve(&estilo);
    let moldura = egui::Frame::popup(&estilo);
    let enchimento = moldura.total_margin().sum().x;
    let naturais: Vec<f32> = ctx.fonts_mut(|f| avisos.iter().map(|(m, _, _)| f.layout_no_wrap(m.clone(), fonte.clone(), egui::Color32::PLACEHOLDER).size().x).collect());
    let w = largura(naturais, ctx.content_rect().width(), enchimento);
    egui::Area::new(egui::Id::new("toasts")).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0., -80.)).show(ctx, |ui| {
        // a largura da área é a do quadro anterior; aqui ela passa a ser a deste texto
        ui.set_width(w + enchimento);
        for (m, _, err) in avisos {
            moldura.show(ui, |ui| {
                ui.set_max_width(w);
                let cor = if *err { crate::theme::st_err() } else { crate::theme::st_ok() };
                ui.add(egui::Label::new(RichText::new(m).color(cor)).wrap());
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aviso_curto_nao_encolhe_o_proximo_longo() {
        // largura natural do texto longo do caso 8 (por volta de 480 pt em 14 pt)
        let longo = 480.;
        assert_eq!(largura([60.], 1920., 12.), 60., "o curto fica do tamanho dele");
        assert_eq!(largura([longo], 1920., 12.), longo, "o longo não herda a largura do curto");
        assert_eq!(largura([2000.], 1920., 12.), LARGURA_MAX, "texto enorme quebra em LARGURA_MAX");
        assert!(largura([longo], 400., 12.) <= 400. - 2. * 16. - 12., "janela estreita: cabe dentro dela");
    }

    #[test]
    fn aviso_longo_fica_o_tempo_de_ler() {
        assert_eq!(duracao("Montagem salva"), 4.);
        let longo = "Salve a montagem antes: as peças dos fixadores são gravadas na pasta dela";
        assert!(duracao(longo) > 4.8, "73 caracteres pedem quase 5 s: {}", duracao(longo));
        assert_eq!(duracao(&"x".repeat(1000)), 12.);
    }

    /// Roda o aviso num quadro de egui de verdade e mede o retângulo da moldura do texto.
    fn medir(ctx: &egui::Context, avisos: &[(String, f64, bool)]) -> egui::Rect {
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920., 1009.))), ..Default::default() };
        let mut saida = ctx.run_ui(raw, |ui| desenhar(ui.ctx(), avisos));
        saida.textures_delta.clear();
        ctx.memory(|m| m.area_rect(egui::Id::new("toasts"))).expect("área dos avisos")
    }

    #[test]
    fn depois_de_um_aviso_curto_o_longo_sai_largo_e_com_poucas_linhas() {
        let ctx = egui::Context::default();
        let curto = vec![("Montagem salva".to_string(), 0., false)];
        for _ in 0..3 {
            medir(&ctx, &curto);
        }
        let estreita = medir(&ctx, &curto);
        let longo = vec![("Salve a montagem antes: as peças dos fixadores são gravadas na pasta dela".to_string(), 0., true)];
        medir(&ctx, &longo);
        let r = medir(&ctx, &longo);
        assert!(r.width() > estreita.width() * 2., "o longo alargou: {} > 2 × {}", r.width(), estreita.width());
        assert!(r.width() <= LARGURA_MAX + 40., "sem passar do limite: {}", r.width());
        // no defeito saíam dezenas de linhas de uma ou duas sílabas; agora cabe em uma ou duas
        assert!(r.height() < estreita.height() * 2.5, "poucas linhas: {} (uma linha: {})", r.height(), estreita.height());
    }
}
