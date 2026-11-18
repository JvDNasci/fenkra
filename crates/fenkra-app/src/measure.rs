// SPDX-License-Identifier: LGPL-2.1-or-later
//! Medição rápida da seleção da viewport (vértices, arestas, faces) e extração: cada medida pode
//! virar parâmetro global; vértice, aresta e face podem virar ponto, curva e superfície de referência.
//! O cálculo é o mesmo do comando Medir (B509, `medir.rs`): um item mede o elemento, dois medem
//! entre eles.

use crate::app::{fmt, App, Sel};
use crate::medir::{Unidade, Valor};
use eframe::egui::{self, RichText};
use fenkra_core::*;
use crate::i18n::tr;

enum Act {
    Param(&'static str, f64, Unit),
    Point([f64; 3]),
    Curve(Vec<[f64; 3]>),
    Surface([f64; 3], Option<Id>),
    AbrirMedir,
}

/// Leitura da seleção atual (1 item: medir item; 2 ou mais: o primeiro contra os outros).
fn leitura(app: &App) -> Option<crate::medir::Leitura> {
    let r = app.result.as_ref()?;
    let refs: Vec<RefMedida> = app.picks.iter().filter_map(|p| crate::medir_ui::ref_de_pick(app, p).map(|x| x.0)).collect();
    if refs.is_empty() {
        return None;
    }
    let (modo, a, b) = if refs.len() == 1 { (ModoMedida::Item, refs, vec![]) } else { (ModoMedida::Entre, refs[..1].to_vec(), refs[1..].to_vec()) };
    Some(crate::medir_ui::ler(r, &Medida { id: Id(0), nome: String::new(), modo, a, b, infinito: false, campos: vec![] }))
}

/// Extrações possíveis da seleção (um elemento só).
fn extracoes(app: &App) -> Vec<(&'static str, Act)> {
    let Some(r) = &app.result else { return vec![] };
    match app.picks.as_slice() {
        [crate::picking::Pick::Vertex { pos, .. }] => vec![("• Extrair como ponto", Act::Point(pos.map(|v| v as f64)))],
        [crate::picking::Pick::Edge { body, edge, .. }] => vec![("∿ Extrair como curva", Act::Curve(crate::picking::edge_polyline(r, *body, *edge)))],
        [crate::picking::Pick::Face { point, owner, .. }] => vec![("◫ Extrair como superfície", Act::Surface(point.map(|v| v as f64), Some(*owner)))],
        _ => vec![],
    }
}

/// Painel de medição (aparece com seleção na viewport e nenhum comando aberto).
pub fn panel(app: &mut App, ui: &mut egui::Ui) {
    if app.picks.is_empty() || app.session.is_some() || app.medir.is_some() {
        return;
    }
    let l = leitura(app).unwrap_or_default();
    ui.add_space(8.);
    ui.separator();
    ui.label(RichText::new(tr("📏 Medição da seleção")).strong());
    if l.linhas.is_empty() {
        ui.label(RichText::new(tr("Selecione 1 ou 2 itens: vértice, aresta, face (Shift soma).")).weak().small());
    }
    let mut act: Option<Act> = None;
    egui::Grid::new("medidas").num_columns(3).striped(true).show(ui, |ui| {
        for x in &l.linhas {
            ui.label(tr(x.rotulo));
            match x.valor {
                Valor::Num(v, u) => {
                    ui.label(RichText::new(format!("{} {}", fmt(v), u.texto())).strong());
                    if ui.small_button("→ ƒ").on_hover_text(tr("criar parâmetro global com este valor")).clicked() {
                        let unit = match u {
                            Unidade::Grau => Unit::Angulo,
                            Unidade::Mm => Unit::Comprimento,
                            Unidade::Mm2 => Unit::Numero,
                        };
                        act = Some(Act::Param(x.rotulo, v, unit));
                    }
                }
                Valor::Ponto(p) => {
                    ui.label(RichText::new(format!("({}; {}; {})", fmt(p[0]), fmt(p[1]), fmt(p[2]))).strong());
                    ui.label("");
                }
            }
            ui.end_row();
        }
    });
    for (t, a) in extracoes(app) {
        if ui.button(tr(t)).clicked() {
            act = Some(a);
        }
    }
    if ui.button(format!("📏 {}", tr("Medir"))).on_hover_text(tr("Medir item ou entre elementos (paquímetro)")).clicked() {
        act = Some(Act::AbrirMedir);
    }
    let Some(a) = act else { return };
    if let Act::AbrirMedir = a {
        crate::medir_ui::abrir(app);
        return;
    }
    // a seleção é uma só aqui: a chave dela faz o elemento seguir o modelo (ADR 0010)
    let chave_face = app.face_key_of(app.picks.first());
    let chave_aresta = match app.picks.first() {
        Some(crate::picking::Pick::Edge { body, edge, .. }) => app.edge_key_of(*body, *edge),
        _ => None,
    };
    let d = &mut app.open.doc;
    let new = match a {
        Act::Param(label, v, unit) => {
            // o nome do parâmetro segue o idioma da interface (B510)
            let base: String = tr(label).chars().filter(|c| c.is_alphanumeric()).collect();
            let n = if base.is_empty() { d.nome_padrao("Medida") } else { d.unique_name(&base) };
            let _ = d.add_param(&n, &fmt(v).replace(',', "."), unit);
            app.toast(format!("{} {n} = {}", tr("Parâmetro criado:"), fmt(v)), false);
            Sel::Params
        }
        Act::Point(p) => Sel::Ref(d.add_geometry(RefKind::PointCoords { x: fmt(p[0]).replace(',', "."), y: fmt(p[1]).replace(',', "."), z: fmt(p[2]).replace(',', "."), frame: None })),
        Act::Curve(pts) => Sel::Ref(d.add_geometry(RefKind::CurveCopy { curve: chave_aresta.map_or_else(|| CurveRef::Edge(pts.clone()), |key| CurveRef::BodyEdge { pts: pts.clone(), key }) })),
        Act::Surface(p, owner) => Sel::Ref(d.add_geometry(RefKind::SurfExtract { face: p, owner, prop: Propagation::Nenhuma, complement: false, expected: None, key: chave_face })),
        Act::AbrirMedir => unreachable!(),
    };
    app.sel = new;
}
