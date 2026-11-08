// SPDX-License-Identifier: LGPL-2.1-or-later
//! B426: progresso da conversão da forma livre, mostrado sobre a peça. A conversão leva segundos e a
//! barra de estado só dizia "recalculando"; agora o meio da vista diz o que está acontecendo e quanto
//! falta. Some sozinho quando a conversão termina ou vem do cache (aí nem aparece).

use crate::i18n::tr;
use eframe::egui::{self, Rect, RichText};
use fenkra_solidos::forma_brep_cache::andamento;

/// Texto do progresso: `None` sem conversão em andamento.
pub fn texto(atual: Option<(u32, u32)>) -> Option<String> {
    let (f, t) = atual?;
    let pct = (f as f32 / t.max(1) as f32 * 100.).round() as u32;
    Some(format!("{} {pct}% ({f}/{t})", tr("Convertendo a forma livre em sólido…")))
}

/// Faixa sobre a vista, centrada na área do 3D, enquanto o cálculo está ocupado com a conversão.
pub fn desenhar(ctx: &egui::Context, area: Rect, ocupado: bool) {
    if !ocupado {
        return;
    }
    let atual = andamento::atual();
    let (Some(t), Some((f, n))) = (texto(atual), atual) else { return };
    egui::Area::new(egui::Id::new("forma_progresso"))
        .order(egui::Order::Foreground)
        .interactable(false)
        .fixed_pos(egui::pos2(area.center().x - 170., area.top() + 48.))
        .show(ctx, |ui| {
            crate::paineis::quadro(ui.style()).show(ui, |ui| {
                ui.set_width(340.);
                ui.label(RichText::new(t).strong());
                ui.add(egui::ProgressBar::new(f as f32 / n.max(1) as f32).desired_height(6.));
                ui.label(RichText::new(tr("O resultado fica guardado: a próxima vez abre sem converter.")).small().weak());
            });
        });
    // o progresso muda na thread do cálculo: pede quadro novo para a barra andar
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn sem_conversao_nao_ha_texto_e_com_conversao_ha_porcentagem() {
        assert!(texto(None).is_none());
        let t = texto(Some((3, 12))).unwrap();
        assert!(t.contains("25%") && t.contains("3/12"), "{t}");
    }
}
