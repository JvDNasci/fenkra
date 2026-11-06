// SPDX-License-Identifier: LGPL-2.1-or-later
//! Vínculos da geometria externa e posição do sketch no painel (B97).
//!
//! Cada vínculo guarda a origem e um estado: em dia, isolado ou quebrado. Trocar o estado não pede
//! para escolher a origem de novo: isolar só para de acompanhar, e um vínculo quebrado volta sozinho
//! quando a origem reaparece. Soltar é o único passo sem volta.

use crate::app::App;
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::sketch::{Curve, LinkPolicy, LinkSrc};
use fenkra_core::vinculo::OpVinculo;
use fenkra_core::{FeatureKind, Id};

/// A origem mudou de tipo ou de número de pontos: o documento recebe o mesmo pedido que o cálculo fez
/// na cópia dele. Não se copia a cópia: o resultado pode estar um quadro atrás do que se desenhou.
fn aplica_estrutura(app: &mut App, id: Id, ops: &[OpVinculo]) {
    if ops.is_empty() {
        return;
    }
    let Some(FeatureKind::Sketch(s)) = app.open.doc.feature(id).map(|f| &f.kind) else { return };
    let mut novo = s.data.clone();
    for op in ops {
        novo.aplica_vinculo(op);
    }
    if novo != s.data {
        if let Some(FeatureKind::Sketch(s)) = app.open.doc.feature_mut(id).map(|f| &mut f.kind) {
            s.data = novo;
        }
    }
}

/// A geometria externa do documento acompanha o último cálculo. Só escreve o que mudou, para não
/// marcar o documento como alterado a cada quadro.
pub fn sincroniza(app: &mut App, id: Id) {
    let Some(v) = app.result.as_ref().and_then(|r| r.sketches.iter().find(|v| v.feature == id)) else { return };
    let (lp, lr, le, ops) = (v.link_pts.clone(), v.link_r.clone(), v.link_e.clone(), v.link_ops.clone());
    aplica_estrutura(app, id, &ops);
    let dados = match app.open.doc.feature(id).map(|f| &f.kind) {
        Some(FeatureKind::Sketch(s)) => &s.data,
        _ => return,
    };
    let muda_ponto = lp.iter().any(|(p, q)| dados.point(*p).is_some_and(|pt| (pt.x - q[0]).abs() > 1e-9 || (pt.y - q[1]).abs() > 1e-9));
    let muda_raio = lr.iter().any(|(c, r)| matches!(dados.curve(*c).map(|x| &x.geom), Some(Curve::Circle { r: rr, .. }) if (rr - r).abs() > 1e-9));
    let muda_elipse = le.iter().any(|(c, m, n, a, b)| matches!(dados.curve(*c).map(|x| &x.geom), Some(Curve::Ellipse { major, minor, t0, t1, .. }) if major != m || minor != n || t0 != a || t1 != b));
    if !(muda_ponto || muda_raio || muda_elipse) {
        return;
    }
    let Some(FeatureKind::Sketch(s)) = app.open.doc.feature_mut(id).map(|f| &mut f.kind) else { return };
    for (p, q) in lp {
        if let Some(pt) = s.data.point_mut(p) {
            (pt.x, pt.y) = (q[0], q[1]);
        }
    }
    for c in &mut s.data.curves {
        match &mut c.geom {
            Curve::Circle { r, .. } => {
                if let Some((_, n)) = lr.iter().find(|x| x.0 == c.id) {
                    *r = *n;
                }
            }
            Curve::Ellipse { major, minor, t0, t1, .. } => {
                if let Some((_, m, n, a, b)) = le.iter().find(|x| x.0 == c.id) {
                    (*major, *minor, *t0, *t1) = (*m, *n, *a, *b);
                }
            }
            _ => {}
        }
    }
}

/// Estado do vínculo: o glifo do painel e o texto que o explica.
pub fn estado(politica: LinkPolicy, quebrado: bool) -> (&'static str, &'static str) {
    match (politica, quebrado) {
        (LinkPolicy::Isolado, _) => ("⇥", "vínculo isolado: a origem está guardada"),
        (_, true) => ("→✕", "a origem não foi encontrada: a geometria ficou onde estava e volta sozinha quando a origem voltar"),
        _ => ("→", "vínculo em dia"),
    }
}

fn nome_da_origem(s: &LinkSrc) -> &'static str {
    match s {
        LinkSrc::Ref(_) => "referência",
        LinkSrc::SketchCurve { .. } => "curva de outro sketch",
        LinkSrc::Edge { .. } => "aresta",
        LinkSrc::Vertex { .. } => "vértice",
        LinkSrc::Face { .. } => "face",
    }
}

#[derive(Clone, Copy)]
enum Acao {
    Politica(Id, LinkPolicy),
    Soltar(Id),
}

/// Política, lista de vínculos e posição do sketch.
pub fn painel(app: &mut App, ui: &mut egui::Ui) {
    let Some(id) = app.sketch.as_ref().map(|e| e.feature) else { return };
    let quebrados: Vec<Id> = app.result.as_ref().and_then(|r| r.sketches.iter().find(|v| v.feature == id)).map(|v| v.link_quebrados.clone()).unwrap_or_default();
    let Some(FeatureKind::Sketch(s)) = app.open.doc.feature_mut(id).map(|f| &mut f.kind) else { return };
    let d = &mut s.data;
    let mut acao = None;
    let titulo = if quebrados.is_empty() { tr("Geometria externa e posição").to_string() } else { format!("{} (→✕ {})", tr("Geometria externa e posição"), quebrados.len()) };
    egui::CollapsingHeader::new(titulo).id_salt("sk_vinculos").default_open(!quebrados.is_empty() || crate::sketch_demo::abre_vinculos()).show(ui, |ui| {
        ui.label(RichText::new(tr("Política da geometria nova")).strong());
        ui.horizontal_wrapped(|ui| {
            for (p, n, dica) in [
                (LinkPolicy::Associativo, "Associativa", "Acompanha a origem quando ela muda."),
                (LinkPolicy::Isolado, "Isolada", "Guarda a origem, mas não acompanha: volta a acompanhar sem escolher de novo."),
                (LinkPolicy::Instantaneo, "Instantânea", "Só copia: a geometria nasce comum, sem vínculo."),
            ] {
                ui.selectable_value(&mut d.link_policy, p, tr(n)).on_hover_text(tr(dica));
            }
        });
        for l in &d.links {
            let (glifo, texto) = estado(l.policy, quebrados.contains(&l.id));
            ui.horizontal(|ui| {
                ui.label(RichText::new(glifo).strong()).on_hover_text(tr(texto));
                ui.label(tr(nome_da_origem(&l.source)));
                let isolado = l.policy == LinkPolicy::Isolado;
                let (rotulo, nova) = if isolado { ("Associar", LinkPolicy::Associativo) } else { ("Isolar", LinkPolicy::Isolado) };
                if ui.small_button(tr(rotulo)).clicked() {
                    acao = Some(Acao::Politica(l.id, nova));
                }
                if ui.small_button(tr("Soltar")).on_hover_text(tr("Tira o vínculo: a geometria fica comum e livre")).clicked() {
                    acao = Some(Acao::Soltar(l.id));
                }
            });
        }
        ui.add_space(4.);
        ui.label(RichText::new(tr("Posição do sketch")).strong());
        ui.horizontal(|ui| {
            let r = ui.add(crate::ui_ext::dv(&mut d.pos.origem[0]).speed(0.5).prefix(format!("{} ", tr("origem H"))));
            crate::ui_ext::wheel(ui, &r, &mut d.pos.origem[0]);
            let r = ui.add(crate::ui_ext::dv(&mut d.pos.origem[1]).speed(0.5).prefix(format!("{} ", tr("origem V"))));
            crate::ui_ext::wheel(ui, &r, &mut d.pos.origem[1]);
        });
        ui.horizontal_wrapped(|ui| {
            let dica = tr("Trocar ou inverter um eixo só vira a normal: a extrusão passa para o outro lado.");
            ui.checkbox(&mut d.pos.trocar, tr("Trocar H e V")).on_hover_text(dica);
            ui.checkbox(&mut d.pos.inverter_h, tr("Inverter H")).on_hover_text(dica);
            ui.checkbox(&mut d.pos.inverter_v, tr("Inverter V")).on_hover_text(dica);
        });
    });
    match acao {
        Some(Acao::Politica(l, p)) => {
            if let Some(x) = d.links.iter_mut().find(|x| x.id == l) {
                x.policy = p;
            }
        }
        Some(Acao::Soltar(l)) => d.release_link(l),
        None => {}
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn estado_do_vinculo_nao_depende_de_escolher_de_novo() {
        assert_eq!(estado(LinkPolicy::Associativo, false).0, "→");
        assert_eq!(estado(LinkPolicy::Associativo, true).0, "→✕");
        // isolado não procura a origem: não há o que quebrar
        assert_eq!(estado(LinkPolicy::Isolado, true).0, "⇥");
    }

    #[test]
    fn textos_do_painel_tem_traducao() {
        for t in [estado(LinkPolicy::Associativo, false).1, estado(LinkPolicy::Associativo, true).1, estado(LinkPolicy::Isolado, false).1, "Isolar", "Associar", "Soltar", "Trocar H e V", "aresta", "elipse"] {
            assert!(crate::i18n::tem_traducao(t), "{t}");
        }
    }
}
