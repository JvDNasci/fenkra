// SPDX-License-Identifier: LGPL-2.1-or-later
//! Comando de aba unificado (B31). Os cinco jeitos de fazer uma aba (aresta, laço, aresta curva,
//! perfil e superfície) são modos de um comando só, com o painel na ordem Suporte, Modo,
//! Parâmetros e Avançado. Daqui saem também o botão padrão de inverter direção (o mesmo em todo
//! comando de chapa) e o raio próprio que nasce da regra da chapa.

use crate::editor::section;
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::aba_unif::{converter_aba_ui, ModoAbaUi, MODOS_ABA_UI};
use fenkra_core::{Document, FeatureKind};

/// (ícone, nome, dica) de cada modo da interface, na ordem de `MODOS_ABA_UI` (B560): Aresta, Laço e
/// Curva são um modo só, "Arestas".
const MODOS: [(&str, &str, &str); 3] = [
    ("sm_aba", "Arestas", "Abas em arestas retas, curvas ou as duas misturadas: cada clique soma, clicar de novo tira, e uma face pega o contorno inteiro. Os trechos curvos viram uma aba curva emendada às retas."),
    ("sm_aba_perfil", "Perfil", "Aba complexa: cadeia aberta de um sketch, com os cantos no raio da chapa."),
    ("sm_aba_superficie", "Superfície", "Aba apoiada numa superfície de suporte."),
];

/// O raio que o campo "raio próprio" recebe ao ser ligado: o da regra da chapa (ou do estilo), que
/// pode ser uma fórmula da espessura. Sem chapa definida, o valor padrão da chapa nova.
pub fn raio_da_regra(doc: &Document) -> String {
    doc.sheet_efetivo().map(|s| s.radius).filter(|r| !r.trim().is_empty()).unwrap_or_else(|| "2".to_string())
}

/// O botão padrão de inverter a direção: o mesmo em todo comando de chapa, no lugar das caixas com
/// nomes diferentes. Ligado, fica destacado e a direção está invertida.
pub fn inverter(ui: &mut egui::Ui, flip: &mut bool) -> egui::Response {
    let r = ui.selectable_label(*flip, format!("⇄ {}", tr("Inverter direção"))).on_hover_text(tr("Troca o lado ou o sentido do comando sem refazer os outros parâmetros. Destacado = invertido."));
    if r.clicked() {
        *flip = !*flip;
    }
    r
}

/// Suporte do comando de aba no modo "Arestas" (B560): uma lista só, para arestas retas e curvas
/// misturadas, qualquer que seja o recurso por trás (aba reta, aba curva ou laço). Cada clique no 3D
/// soma, clicar de novo tira (`slots::aresta_clicada_lista`), o "✖" tira a escolhida e a face pega o
/// contorno. Os outros modos mostram o suporte no próprio painel.
pub fn suporte(ui: &mut egui::Ui, doc: &fenkra_core::Document, kind: &mut FeatureKind) {
    let z = [0.; 3];
    // os itens da lista, qualquer que seja o recurso
    let mut itens: Vec<([f64; 3], Option<fenkra_core::EdgeKey>)> = match kind {
        FeatureKind::SmFlange(f) => (f.edge != z).then_some((f.edge, f.edge_key)).into_iter().collect(),
        FeatureKind::SmCurveFlange(c) => (!c.edge.is_empty()).then(|| (c.edge[c.edge.len() / 2], c.edge_key)).into_iter().collect(),
        FeatureKind::SmFlangeLoop(lp) => lp.edges.iter().enumerate().filter(|(_, e)| **e != z).map(|(i, e)| (*e, lp.edge_keys.get(i).copied().flatten())).collect(),
        _ => return,
    };
    section(ui, "Suporte");
    let antes = itens.clone();
    // o nome do widget é o `id` que o descritor declara para o recurso (campo ativo e ordem do Tab)
    let salt = match kind {
        FeatureKind::SmFlange(_) => "smaba_aresta",
        FeatureKind::SmCurveFlange(_) => "aresta",
        _ => "smloop_arestas",
    };
    crate::geo_ui::bag(
        ui,
        salt,
        "Arestas",
        &mut itens,
        &[],
        1,
        |(p, k)| if *p == z { tr("escolher no 3D").to_string() } else { crate::ref_nome::campo_aresta(doc, k.as_ref(), *p) },
        |ui, _, _| {
            ui.label(RichText::new(tr("Clique outra aresta no 3D para somar (reta ou curva); clique de novo numa escolhida para tirar.")).weak().small());
        },
        || (z, None),
        Some((z, None)),
        |a, b| (a.1.is_some() && a.1 == b.1) || (0..3).all(|i| (a.0[i] - b.0[i]).abs() < 0.01),
    );
    if itens != antes {
        let reais: Vec<_> = itens.iter().filter(|(p, _)| *p != z).copied().collect();
        match kind {
            FeatureKind::SmFlange(f) => (f.edge, f.edge_key) = reais.first().copied().unwrap_or((z, None)),
            FeatureKind::SmCurveFlange(c) => {
                if reais.is_empty() {
                    c.edge.clear();
                    c.edge_key = None;
                }
            }
            FeatureKind::SmFlangeLoop(lp) => {
                lp.edges = if reais.is_empty() { vec![z] } else { reais.iter().map(|x| x.0).collect() };
                lp.edge_keys = if reais.iter().all(|x| x.1.is_none()) { vec![] } else { reais.iter().map(|x| x.1).collect() };
                if reais.len() <= 1 {
                    lp.loop_all = true;
                }
            }
            _ => {}
        }
    }
    ui.label(RichText::new(tr("Clique numa face da chapa para pegar o contorno inteiro dela.")).weak().small());
    // seguir as tangentes: a curva clicada leva junto as arestas tangentes a ela
    let dica = tr("Ligado: ao clicar numa aresta curva, a lista soma também as arestas tangentes a ela, e a cadeia vira uma aba curva contínua. Desligado: só a aresta clicada.");
    match kind {
        FeatureKind::SmCurveFlange(c) => {
            let mut seguir = !c.so_aresta;
            if crate::editor::check(ui, &mut seguir, "seguir tangentes").on_hover_text(dica).changed() {
                c.so_aresta = !seguir;
            }
        }
        FeatureKind::SmFlangeLoop(lp) => {
            crate::editor::check(ui, &mut lp.seguir, "seguir tangentes").on_hover_text(dica);
            crate::editor::check(ui, &mut lp.loop_all, "usar o laço da face inteira").on_hover_text(tr("Liga: a partir da primeira aresta escolhida, cria uma aba em cada aresta do contorno da face, retas e curvas; os trechos curvos viram aba curva emendada às retas. Desliga: só as arestas escolhidas."));
        }
        _ => {}
    }
}

/// A barra de modos (miniaturas): Arestas, Perfil e Superfície (B560). Troca a feature para o modo
/// clicado, levando o que é comum; dentro de "Arestas" o recurso não muda.
pub fn modos(ui: &mut egui::Ui, kind: &mut FeatureKind) {
    let Some(atual) = fenkra_core::aba_unif::modo_ui_de(kind) else { return };
    section(ui, "Modo da aba");
    let opts: Vec<(&str, &str, &str, bool)> = MODOS.iter().zip(MODOS_ABA_UI).map(|((ic, n, d), m)| (*ic, tr(n), tr(d), m == atual)).collect();
    if let Some(i) = crate::editor::icon_choice(ui, &opts) {
        if let Some(mut novo) = converter_aba_ui(kind, MODOS_ABA_UI[i]) {
            // o perfil ainda não escolhido fica com o marcador de entrada pendente, para o painel
            // pedir o perfil em vez de acusar referência perdida
            if let FeatureKind::SmProfileFlange(p) = &mut novo {
                if p.sketch == fenkra_core::Id(0) {
                    p.sketch = crate::slots::UNSET_SKETCH;
                }
            }
            *kind = novo;
        }
    }
    ui.label(RichText::new(tr(match atual {
        ModoAbaUi::Arestas => "Arestas: uma aba por aresta, reta ou curva, com recuos, extensão lateral e mitra onde couber.",
        ModoAbaUi::Perfil => "Perfil: a aba segue uma cadeia aberta de um sketch.",
        ModoAbaUi::Superficie => "Superfície: a aba se apoia numa superfície de suporte.",
    })).weak().small());
}
