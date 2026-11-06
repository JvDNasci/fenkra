// SPDX-License-Identifier: LGPL-2.1-or-later
//! Mini barra flutuante da anotação (B270): ao criar ou clicar duas vezes numa anotação, uma barra
//! sobre ela traz os campos principais (valor, precisão, tolerância, estilo, plano) e o texto das
//! notas é digitado no próprio 3D, no ponto do clique. O painel lateral fica para o avançado.
//!
//! Enter fecha aceitando; Esc fecha devolvendo a anotação ao que era quando a barra abriu. A
//! sessão inteira vira um passo de desfazer (só se algo mudou).

use crate::app::App;
use crate::i18n::tr;
use crate::pmi_handles::{commit, snapshot, Antes};
use crate::render::Camera;
use eframe::egui::{self, Rect, RichText};
use fenkra_core::pmi::{Pmi, PmiFrame, PmiKind};
use fenkra_core::Id;
use std::sync::Mutex;

struct Sessao {
    id: Id,
    /// a anotação como estava ao abrir: Esc volta a ela
    original: Pmi,
    antes: Option<Antes>,
    /// o campo de texto ainda precisa receber o foco (primeiro quadro)
    foco: bool,
}

static SESSAO: Mutex<Option<Sessao>> = Mutex::new(None);

/// A mini barra está aberta? Com ela aberta o painel lateral da anotação fica recolhido.
pub fn open() -> bool {
    SESSAO.lock().unwrap_or_else(|e| e.into_inner()).is_some()
}

/// Anotação que a barra está editando.
pub fn current() -> Option<Id> {
    SESSAO.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|s| s.id)
}

/// Abre a barra para `id` (criação ou clique duplo). Já aberta para a mesma anotação, não faz nada.
pub fn open_for(app: &App, id: Id) {
    let mut g = SESSAO.lock().unwrap_or_else(|e| e.into_inner());
    if g.as_ref().is_some_and(|s| s.id == id) {
        return;
    }
    let Some(pm) = crate::pmi_ui::list(app).iter().find(|p| p.id == id).cloned() else { return };
    *g = Some(Sessao { id, original: pm, antes: snapshot(app), foco: true });
}

/// Fecha sem tocar na anotação, deixando o gesto como está (usado quando a anotação some).
pub fn drop_session() {
    *SESSAO.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Tipo tem o texto digitado direto no 3D (notas), sem a barra de campos.
fn texto_no_lugar(k: &PmiKind) -> bool {
    matches!(k, PmiKind::Note)
}

/// Desenha a barra da anotação em edição. `lbl` é o retângulo do texto na tela. Devolve `true` se
/// pediu para alternar o plano do texto (quem desenha a vista tem a câmera).
pub fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, lbl: Option<(Id, Rect)>) {
    let Some((id, original, foco)) = ({
        let g = SESSAO.lock().unwrap_or_else(|e| e.into_inner());
        g.as_ref().map(|s| (s.id, s.original.clone(), s.foco))
    }) else {
        return;
    };
    // a anotação sumiu (apagada, desfeita) ou a seleção saiu dela: encerra
    let viva = crate::pmi_ui::list(app).iter().any(|p| p.id == id);
    let sel_id = if app.in_asm() { app.asm.as_ref().and_then(|s| s.sel) } else if let crate::app::Sel::Pmi(i) = app.sel { Some(i) } else { None };
    if !viva || sel_id != Some(id) {
        close(app, false);
        return;
    }
    let Some(lbl) = lbl.filter(|(i, _)| *i == id).map(|(_, r)| r) else { return };
    // as referências do modelo alimentam as listas do quadro de tolerância e a letra livre
    crate::pmi_gdt_ui::atualizar_existentes(crate::pmi_ui::list(app));
    let outras = crate::pmi_gdt_ui::letras_de_outras(crate::pmi_ui::list(app), id);
    let ctx = ui.ctx().clone();
    let (enter, esc) = ctx.input_mut(|i| (i.consume_key(egui::Modifiers::NONE, egui::Key::Enter), i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)));
    let mut avancado = false;
    let mut plano = false;
    let mut mudou = false;
    let mut clique_fora = false;
    {
        let Some(pm) = crate::pmi_ui::list_mut(app).iter_mut().find(|p| p.id == id) else { return };
        let area_pos = if texto_no_lugar(&pm.kind) { lbl.left_top() - egui::vec2(4., 4.) } else { egui::pos2(lbl.center().x, lbl.top() - 8.) };
        let pivot = if texto_no_lugar(&pm.kind) { egui::Align2::LEFT_TOP } else { egui::Align2::CENTER_BOTTOM };
        let area_pos = egui::pos2(area_pos.x.clamp(rect.left() + 4., rect.right() - 4.), area_pos.y.clamp(rect.top() + 4., rect.bottom() - 4.));
        let area = egui::Area::new(egui::Id::unique("pmi_minibar")).order(egui::Order::Foreground).fixed_pos(area_pos).pivot(pivot).show(&ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                if texto_no_lugar(&pm.kind) {
                    // nota: o texto é digitado aqui, no ponto do clique; Shift+Enter quebra a linha
                    let r = ui.add(egui::TextEdit::multiline(&mut pm.text).desired_rows(1).desired_width(190.).hint_text(tr("digite a nota · Enter fecha · Esc cancela")));
                    if foco {
                        r.request_focus();
                    }
                    mudou |= r.changed();
                    return;
                }
                if let PmiKind::Gdt { ch, tol, datums, extra } = &mut pm.kind {
                    // quadro de tolerância: cada casa é um campo com lista, direto no 3D
                    mudou |= crate::pmi_gdt_ui::editor(ui, "mini_gdt", ch, tol, datums, extra, false);
                }
                ui.horizontal(|ui| {
                    campos(ui, pm, foco, &mut mudou, &outras);
                    estilo(ui, pm, &mut mudou);
                    if ui.button(tr(if pm.plane.is_some() { "▱" } else { "🖵" })).on_hover_text(tr("Plano do texto: de frente para a tela ou deitado no plano da vista atual")).clicked() {
                        plano = true;
                    }
                    if ui.button(tr("Avançado…")).on_hover_text(tr("Abre o painel lateral com todos os campos da anotação")).clicked() {
                        avancado = true;
                    }
                });
                ui.label(RichText::new(tr("Enter aceita · Esc volta ao que era")).weak().small());
            });
        });
        // clique fora da barra e do texto: aceita e fecha
        let fora = ui.input(|i| i.pointer.primary_clicked() && i.pointer.interact_pos().is_some_and(|q| !area.response.rect.contains(q) && !lbl.expand(6.).contains(q) && rect.contains(q)));
        clique_fora = fora;
    }
    if plano {
        crate::pmi_ui::toggle_plane(app, id, cam, rect);
        mudou = true;
    }
    if mudou {
        crate::pmi_handles::mark();
        if app.in_asm() {
            if let Some(st) = app.asm.as_mut() {
                st.modified = true;
            }
        }
    }
    if let Some(s) = SESSAO.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        s.foco = false;
    }
    if esc {
        // Esc devolve a anotação ao que era quando a barra abriu
        if let Some(pm) = crate::pmi_ui::list_mut(app).iter_mut().find(|p| p.id == id) {
            *pm = original;
        }
        close(app, true);
    } else if enter || avancado || clique_fora {
        close(app, false);
        if avancado {
            // painel lateral: o avançado
            if app.in_asm() {
                if let Some(st) = app.asm.as_mut() {
                    st.sel = Some(id);
                }
            } else {
                app.sel = crate::app::Sel::Pmi(id);
            }
        }
    }
}

/// Fecha a barra. `reverteu`: Esc, sem passo de desfazer. A anotação continua realçada, sem painel.
fn close(app: &mut App, reverteu: bool) {
    let s = SESSAO.lock().unwrap_or_else(|e| e.into_inner()).take();
    let Some(s) = s else { return };
    if reverteu {
        // o gesto é descartado: nada a desfazer
        crate::pmi_handles::clear_mark();
    } else {
        commit(app, s.antes);
    }
    if app.in_asm() {
        if let Some(st) = app.asm.as_mut() {
            if st.sel == Some(s.id) {
                st.sel = None;
            }
        }
    } else if app.sel == crate::app::Sel::Pmi(s.id) {
        app.sel = crate::app::Sel::None;
    }
    app.pmi_focus = Some(s.id);
}

/// Valor, precisão e tolerância (cotas); campo principal dos demais tipos.
fn campos(ui: &mut egui::Ui, pm: &mut Pmi, foco: bool, mudou: &mut bool, outras: &[String]) {
    let dim = pm.kind.is_dimension();
    let tem_valor = dim || matches!(pm.kind, PmiKind::Roughness { .. } | PmiKind::Datum { .. } | PmiKind::Balloon { .. } | PmiKind::Weld { .. });
    ui.label(RichText::new(tr(pm.kind.label())).strong());
    if let PmiKind::Datum { letter } = &mut pm.kind {
        // referência: a letra nasce livre e a troca é recusada na hora, com a explicação
        ui.label(tr("Letra"));
        *mudou |= crate::pmi_gdt_ui::campo_letra(ui, letter, outras);
        return;
    }
    if tem_valor {
        let r = match &mut pm.kind {
            PmiKind::Roughness { ra, .. } => Some((tr("Ra"), ra)),
            PmiKind::Balloon { item, .. } => Some((tr("Item"), item)),
            PmiKind::Weld { size, .. } => Some((tr("Medida"), size)),
            _ => None,
        };
        if let Some((nome, campo)) = r {
            ui.label(nome);
            let e = ui.add(egui::TextEdit::singleline(campo).desired_width(70.));
            if foco {
                e.request_focus();
            }
            *mudou |= e.changed();
        } else if dim {
            // valor: vazio = o medido
            ui.label(tr("Valor"));
            let e = ui.add(egui::TextEdit::singleline(&mut pm.text).desired_width(70.).hint_text(tr("medido")));
            if foco {
                e.request_focus();
            }
            *mudou |= e.changed();
            e.on_hover_text(tr("Texto que substitui o valor medido; vazio mostra a medida"));
            // precisão
            let mut casas = pm.style.decimals.map(|n| n as i32).unwrap_or(-1);
            let antes = casas;
            ui.label(tr("Casas"));
            egui::ComboBox::from_id_salt(("pmi_mini_casas", pm.id.0)).width(54.).selected_text(if casas < 0 { tr("auto").to_string() } else { casas.to_string() }).show_ui(ui, |ui| {
                ui.selectable_value(&mut casas, -1, tr("auto"));
                for n in 0..=4 {
                    ui.selectable_value(&mut casas, n, n.to_string());
                }
            });
            if casas != antes {
                pm.style.decimals = (casas >= 0).then_some(casas as u8);
                *mudou = true;
            }
            // tolerância
            ui.label("+");
            *mudou |= ui.add(egui::TextEdit::singleline(&mut pm.tol_plus).desired_width(38.)).on_hover_text(tr("Tolerância superior; igual à inferior vira ±")).changed();
            ui.label("−");
            *mudou |= ui.add(egui::TextEdit::singleline(&mut pm.tol_minus).desired_width(38.)).on_hover_text(tr("Tolerância inferior")).changed();
        }
    }
}

/// Estilo rápido: negrito e moldura. O resto fica no painel avançado.
fn estilo(ui: &mut egui::Ui, pm: &mut Pmi, mudou: &mut bool) {
    ui.menu_button(tr("Estilo"), |ui| {
        if ui.checkbox(&mut pm.style.bold, tr("Negrito")).changed() {
            *mudou = true;
        }
        ui.label(RichText::new(tr("Moldura")).weak().small());
        for f in PmiFrame::ALL {
            if ui.selectable_label(pm.style.frame == f, tr(f.label())).clicked() {
                pm.style.frame = f;
                *mudou = true;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_nota_edita_o_texto_no_lugar() {
        assert!(texto_no_lugar(&PmiKind::Note));
        assert!(!texto_no_lugar(&PmiKind::Linear));
        assert!(!texto_no_lugar(&PmiKind::Diameter));
    }
}
