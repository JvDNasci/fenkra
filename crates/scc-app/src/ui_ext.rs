//! Pequenas extensões da interface usadas em todos os painéis.

use eframe::egui;

/// Passo da roda conforme as teclas: Shift ×10, Ctrl ×0,1, Alt ×0,01.
fn step(ui: &egui::Ui) -> f64 {
    ui.input(|i| {
        if i.modifiers.shift {
            10.
        } else if i.modifiers.command {
            0.1
        } else if i.modifiers.alt {
            0.01
        } else {
            1.
        }
    })
}

/// Roda do mouse sobre o campo em edição (clicado): devolve os "dentes" girados (+ para cima) e
/// consome a rolagem (o painel não rola junto). Sem o foco, a roda rola o painel normalmente —
/// passar o mouse por cima ao rolar não muda valores.
fn notches(ui: &egui::Ui, resp: &egui::Response) -> f64 {
    if !resp.hovered() || !resp.has_focus() {
        return 0.;
    }
    // sobre o campo, a roda não rola o painel
    ui.ctx().input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
    let d: f32 = ui.input(|i| {
        i.raw
            .events
            .iter()
            .filter_map(|e| match e {
                egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                _ => None,
            })
            .sum()
    });
    if d == 0. {
        return 0.;
    }
    d.signum() as f64
}

/// Campo de texto numérico (expressões): a roda soma/subtrai quando o texto é um número.
pub fn wheel_text(ui: &egui::Ui, resp: &egui::Response, s: &mut String) -> bool {
    let n = notches(ui, resp);
    if n == 0. {
        return false;
    }
    let Ok(v) = s.trim().replace(',', ".").parse::<f64>() else { return false };
    let v = ((v + n * step(ui)) * 1e6).round() / 1e6;
    *s = crate::app::fmt(v);
    true
}

/// Campo numérico (DragValue): a roda soma/subtrai.
pub fn wheel<T: egui::emath::Numeric>(ui: &egui::Ui, resp: &egui::Response, v: &mut T) {
    let n = notches(ui, resp);
    if n == 0. {
        return;
    }
    let st = if T::INTEGRAL { step(ui).max(1.) } else { step(ui) };
    let x = v.to_f64() + n * st;
    *v = T::from_f64(if T::INTEGRAL { x.round() } else { (x * 1e6).round() / 1e6 });
}
