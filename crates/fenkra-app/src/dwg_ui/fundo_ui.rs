// SPDX-License-Identifier: LGPL-2.1-or-later
//! B427: barra de progresso e botão de cancelar da projeção das vistas de montagem, que roda numa
//! thread própria (`fenkra_desenho::fundo`). A folha continua respondendo enquanto isso; as vistas
//! pedidas aparecem vazias, com "Calculando…", e se preenchem quando ficam prontas.

use super::*;

/// Liga a thread de projeção na primeira vez e recolhe o que ficou pronto. `true` se alguma vista
/// chegou (a folha precisa ser refeita).
pub fn sondar(st: &mut DwgState, ctx: &egui::Context) -> bool {
    if !st.models.fundo_ativo() {
        let c = ctx.clone();
        st.models.ativar_fundo(move || c.request_repaint());
    }
    let chegou = st.models.sondar();
    // enquanto há trabalho a barra anda sozinha, mesmo sem entrada do usuário
    if st.models.andamento_do_fundo().is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
    chegou
}

/// Texto do progresso: "12 de 85 peças · 2 vistas na fila".
pub fn texto(a: &fenkra_desenho::fundo::Andamento) -> String {
    if a.refinando {
        return tr("Refinando vistas: a prévia por malha já está na folha").to_string();
    }
    let base = if a.total > 0 { format!("{} {} {} {}", tr("Calculando vistas"), a.feitos.min(a.total), tr("de"), a.total) } else { tr("Calculando vistas…").to_string() };
    if a.esperando > 0 {
        format!("{base} · {} {}", a.esperando, tr("na fila"))
    } else {
        base
    }
}

/// Barra no rodapé da folha: progresso e Cancelar enquanto calcula; "Calcular de novo" quando a
/// pessoa cancelou e há vistas paradas.
pub fn barra(ui: &mut egui::Ui, st: &mut DwgState, rect: Rect) {
    let andamento = st.models.andamento_do_fundo();
    let paradas = st.models.vistas_canceladas();
    if andamento.is_none() && !paradas {
        return;
    }
    let mut cancelar = false;
    let mut retomar = false;
    let ctx = ui.ctx().clone();
    egui::Area::new(egui::Id::new("dwg_fundo_barra")).order(egui::Order::Foreground).fixed_pos(Pos2::new(rect.left() + 12., rect.bottom() - 56.)).show(&ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(a) = andamento {
                    let frac = if a.total > 0 { (a.feitos as f32 / a.total as f32).clamp(0., 1.) } else { 0. };
                    ui.add(egui::ProgressBar::new(frac).desired_width(320.).text(texto(&a)).animate(a.total == 0)).on_hover_text(tr("A projeção das vistas de montagem roda em segundo plano; a folha continua utilizável."));
                    if ui.button(tr("Cancelar")).on_hover_text(tr("Para o cálculo e deixa as vistas pendentes vazias. A chamada de linhas ocultas já iniciada termina por conta própria, sem travar a folha.")).clicked() {
                        cancelar = true;
                    }
                } else {
                    ui.label(tr("Cálculo cancelado: há vistas sem projeção."));
                    if ui.button(tr("Calcular de novo")).on_hover_text(tr("Pede de novo a projeção das vistas canceladas.")).clicked() {
                        retomar = true;
                    }
                }
            });
        });
    });
    if cancelar {
        st.models.cancelar_fundo();
        st.invalidate();
    }
    if retomar {
        st.models.retomar_fundo();
        st.invalidate();
    }
}
