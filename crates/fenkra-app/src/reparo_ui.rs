// SPDX-License-Identifier: LGPL-2.1-or-later
//! Candidatas no 3D do assistente de reparo (B15, ADR 0010 fase 2).
//!
//! Com o assistente aberto, cada face ou aresta perdida de um comando com erro mostra as candidatas
//! na viewport, numeradas (a mais provável é a 1). Clicar no número, no 3D ou na lista do assistente,
//! troca o ponto do comando pelo da candidata; a chave v2 é refeita no cálculo seguinte.

use crate::app::App;
use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, Rect, RichText, Sense, Stroke};
use fenkra_core::Id;
use fenkra_solidos::reparo::{self, Alvo, Lacuna};
use fenkra_solidos::Status;
use glam::Vec3;

/// Candidata sob o ponteiro (no 3D ou na lista): comando, referência e número da candidata.
type Destaque = (u64, usize, usize);

fn chave_destaque() -> egui::Id {
    egui::Id::new("reparo_destaque")
}

/// Lacunas de todos os comandos com erro (só com o assistente aberto).
pub fn lacunas(app: &App) -> Vec<(Id, Vec<Lacuna>)> {
    // recolhido (B532) é só o aviso: as candidatas só aparecem com o assistente aberto de verdade
    if !app.wizard_open || app.wizard_recolhido {
        return vec![];
    }
    let Some(r) = &app.result else { return vec![] };
    app.open
        .doc
        .features
        .iter()
        .filter(|f| matches!(r.status.get(&f.id), Some(Status::Error(_))))
        .map(|f| (f.id, reparo::lacunas(f, &r.bodies)))
        .filter(|(_, l)| !l.is_empty())
        .collect()
}

fn nome(l: &Lacuna) -> String {
    match l.alvo {
        Alvo::Face => format!("{} {}", tr("Face"), l.indice + 1),
        Alvo::Aresta => format!("{} {}", tr("Aresta"), l.indice + 1),
    }
}

/// Linhas do assistente para um comando: as referências perdidas e os botões das candidatas.
pub fn linhas(ui: &mut egui::Ui, id: Id, lac: &[Lacuna], escolha: &mut Option<(Id, Lacuna, usize)>) {
    let k = chave_destaque();
    for l in lac {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("📍 {} — {}", nome(l), tr("candidatas no 3D:"))).small());
            for (i, _) in l.candidatas.iter().enumerate() {
                let b = ui.small_button(format!("{}", i + 1)).on_hover_text(tr("Usar esta candidata: o comando passa a apontar para ela (a mais provável é a 1)."));
                if b.hovered() {
                    ui.ctx().data_mut(|m| m.insert_temp::<Destaque>(k, (id.0, l.indice, i)));
                }
                if b.clicked() {
                    *escolha = Some((id, l.clone(), i));
                }
            }
        });
    }
}

/// Troca a referência pela candidata `i` e guarda o desfazer.
pub fn aplicar(app: &mut App, id: Id, l: &Lacuna, i: usize) {
    let Some(p) = l.candidatas.get(i).copied() else { return };
    let antes = app.open.doc.clone();
    let ok = app.open.doc.feature_mut(id).is_some_and(|f| reparo::aplicar(&mut f.kind, l.alvo, l.indice, p));
    if ok {
        app.push_undo_pub(antes);
    }
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Marcadores numerados das candidatas na viewport; clique aplica.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let todas = lacunas(app);
    if todas.is_empty() {
        return;
    }
    let k = chave_destaque();
    let destaque = ui.ctx().data_mut(|m| m.remove_temp::<Destaque>(k));
    let painter = ui.painter_at(rect);
    let acento = crate::theme::accent(&ui.ctx().global_style().visuals);
    let erro = crate::theme::st_err();
    let texto = ui.visuals().strong_text_color();
    let mut escolha: Option<(Id, Lacuna, usize)> = None;
    for (id, lac) in &todas {
        for l in lac {
            // último ponto conhecido: um x vermelho
            if let Some(a) = cam.project(rect, v3(l.atual)) {
                let s = 5.;
                painter.line_segment([a + egui::vec2(-s, -s), a + egui::vec2(s, s)], Stroke::new(2., erro));
                painter.line_segment([a + egui::vec2(-s, s), a + egui::vec2(s, -s)], Stroke::new(2., erro));
            }
            for (i, c) in l.candidatas.iter().enumerate() {
                let Some(a) = cam.project(rect, v3(*c)) else { continue };
                let aceso = destaque == Some((id.0, l.indice, i));
                let r = ui.interact(Rect::from_center_size(a, egui::vec2(20., 20.)), egui::Id::new(("reparo_cand", *id, l.indice, i)), Sense::click());
                let forte = aceso || r.hovered();
                painter.circle(a, if forte { 11. } else { 9. }, if forte { acento } else { acento.gamma_multiply(0.55) }, Stroke::new(1.5, texto));
                painter.text(a, egui::Align2::CENTER_CENTER, format!("{}", i + 1), egui::FontId::proportional(11.), texto);
                if r.clicked() {
                    escolha = Some((*id, l.clone(), i));
                }
                r.on_hover_text(format!("{} · {}", nome(l), tr("clique para usar esta candidata")));
            }
        }
    }
    if let Some((id, l, i)) = escolha {
        aplicar(app, id, &l, i);
    }
}

/// B507: o assistente de reparo aparece só quando há erro. Sem nenhum item quebrado ele fecha sozinho,
/// inclusive quando a pessoa o abriu pela barra de estado e corrigiu o último erro.
pub fn assistente_aberto(itens_com_erro: usize) -> bool {
    itens_com_erro > 0
}

/// B532: o rótulo do aviso recolhido, como "Reparar Sketch base" ou "Reparar Sketch base +2".
pub fn rotulo_do_aviso(nomes: &[&str]) -> String {
    match nomes {
        [] => tr("Reparar").to_string(),
        [a] => format!("{} {a}", tr("Reparar")),
        [a, resto @ ..] => format!("{} {a} +{}", tr("Reparar"), resto.len()),
    }
}

/// B532: o aviso pequeno no lugar do assistente grande. Devolve `Some(true)` se a pessoa pediu para
/// expandir e `Some(false)` se dispensou o aviso.
pub fn aviso_recolhido(ctx: &egui::Context, nomes: &[&str]) -> Option<bool> {
    let mut resposta = None;
    egui::Area::new(egui::Id::new("reparo_aviso")).order(egui::Order::Foreground).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0., -48.)).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                // o ponto de erro, na cor do tema
                let (ponto, _) = ui.allocate_exact_size(egui::vec2(10., 10.), Sense::hover());
                ui.painter().circle_filled(ponto.center(), 4.5, crate::theme::st_err());
                let r = ui.add(egui::Button::new(RichText::new(rotulo_do_aviso(nomes)).strong()).frame(false));
                if r.on_hover_text(tr("Clique para abrir o assistente de reparo, com o diagnóstico completo.")).clicked() {
                    resposta = Some(true);
                }
                if ui.small_button("✕").on_hover_text(tr("Dispensar o aviso. Ele volta quando surgir outro erro; o erro continua pela barra de baixo.")).clicked() {
                    resposta = Some(false);
                }
            });
        });
    });
    resposta
}

/// B528: apaga de uma vez os itens com erro (sem os dependentes: quem depende deles também está na
/// lista, porque também ficou com erro). Itens já levados por outro apagar são ignorados. Devolve
/// quantos saíram do documento.
pub fn apagar_todos(doc: &mut fenkra_core::Document, ids: &[Id]) -> usize {
    let existe = |d: &fenkra_core::Document, id: Id| d.feature(id).is_some() || d.reference(id).is_some();
    let mut n = 0;
    for id in ids {
        if existe(doc, *id) {
            doc.delete(*id, false);
            n += 1;
        }
    }
    n
}

/// B528: nomes para a janela de confirmação, na ordem em que aparecem.
pub fn nomes_para_apagar(doc: &fenkra_core::Document, ids: &[Id]) -> Vec<String> {
    ids.iter().map(|i| doc.name_of(*i).to_string()).collect()
}

/// B541: o Enter confirma o menu de apagar; Shift, Ctrl ou Alt com Enter não são do menu.
pub fn enter_confirma(menu_aberto: bool, enter: bool, com_modificador: bool) -> bool {
    menu_aberto && enter && !com_modificador
}

#[cfg(test)]
mod testes_lote {
    use super::*;
    use fenkra_core::*;

    /// Um sketch sobre um plano que não existe e três recursos que dependem dele: apagar o sketch e
    /// escolher apagar todos leva os quatro, e o documento volta com um desfazer só.
    fn doc_com_tres_dependentes() -> (Document, Id, Vec<Id>) {
        let mut d = Document::new("t");
        let mut data = SketchData::default();
        let mut n = 9_800_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, [-10., -10.], [10., 10.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        let mk = |d: &mut Document| d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("5".into()), reverse: false, taper: "0".into(), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
        let dep = vec![mk(&mut d), mk(&mut d), mk(&mut d)];
        (d, sk, dep)
    }

    #[test]
    fn apagar_todos_leva_os_tres_com_erro_e_o_desfazer_devolve() {
        let (mut d, sk, dep) = doc_com_tres_dependentes();
        let antes = d.clone();
        assert_eq!(d.dependents(sk).len(), 3);
        // o sketch foi apagado e os três ficaram com erro: a lista do assistente
        d.delete(sk, false);
        assert_eq!(d.features.len(), 3);
        assert_eq!(nomes_para_apagar(&d, &dep).len(), 3);
        let n = apagar_todos(&mut d, &dep);
        assert_eq!(n, 3);
        assert!(d.features.is_empty());
        // repetir não apaga de novo
        assert_eq!(apagar_todos(&mut d, &dep), 0);
        // o desfazer guarda o documento de antes inteiro
        assert_eq!(antes.features.len(), 4);
    }

    #[test]
    fn rotulo_do_aviso_cita_o_primeiro_e_conta_o_resto() {
        assert_eq!(rotulo_do_aviso(&["Sketch base"]), format!("{} Sketch base", tr("Reparar")));
        assert_eq!(rotulo_do_aviso(&["A", "B", "C"]), format!("{} A +2", tr("Reparar")));
        assert_eq!(rotulo_do_aviso(&[]), tr("Reparar"));
    }

    #[test]
    fn enter_so_confirma_com_o_menu_aberto_e_sem_modificador() {
        assert!(enter_confirma(true, true, false));
        assert!(!enter_confirma(false, true, false), "sem menu, o Enter é da árvore");
        assert!(!enter_confirma(true, false, false));
        assert!(!enter_confirma(true, true, true));
    }
}
