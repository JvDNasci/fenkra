// SPDX-License-Identifier: LGPL-2.1-or-later
//! Comando de aba unificado (B31). Os cinco jeitos de fazer uma aba (aresta, laço, aresta curva,
//! perfil e superfície) são modos de um comando só, com o painel na ordem Suporte, Modo,
//! Parâmetros e Avançado. Daqui saem também o botão padrão de inverter direção (o mesmo em todo
//! comando de chapa) e o raio próprio que nasce da regra da chapa.

use crate::editor::section;
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::aba_unif::{converter_aba, modo_de, ModoAba, MODOS_ABA};
use fenkra_core::{Document, FeatureKind};

/// (ícone, nome, dica) de cada modo, na ordem de `MODOS_ABA`.
const MODOS: [(&str, &str, &str); 5] = [
    ("sm_aba", "Aresta", "Uma aba numa aresta reta da chapa, com recuos laterais e alívio."),
    ("sm_aba_laco", "Laço", "Várias abas de uma vez: as arestas escolhidas ou o laço da face inteira, com mitra nos cantos e extensão lateral."),
    ("sm_aba_curva", "Curva", "Aba numa aresta curva, seguindo as arestas tangentes."),
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

/// Suporte do comando de aba: o que foi escolhido no 3D, com contador e botão de limpar. Só as
/// abas de aresta e de laço têm suporte de aresta; os outros modos mostram o suporte no próprio painel.
pub fn suporte(ui: &mut egui::Ui, kind: &mut FeatureKind) {
    match kind {
        FeatureKind::SmFlange(fl) => {
            section(ui, "Suporte");
            let vazio = fl.edge == [0.; 3];
            ui.label(RichText::new(if vazio { tr("⚠ clique numa aresta reta da chapa na viewport").to_string() } else { format!("{} {} — {}", tr("aresta em"), crate::app::p3(fl.edge, 1), tr("clique outra aresta para trocar")) }).small());
            if ui.add_enabled(!vazio, egui::Button::new(tr("Limpar"))).on_hover_text(tr("Tira a aresta escolhida para escolher outra.")).clicked() {
                fl.edge = [0.; 3];
                fl.edge_key = None;
            }
        }
        FeatureKind::SmFlangeLoop(lp) => {
            section(ui, "Suporte");
            let n = lp.edges.iter().filter(|e| **e != [0.; 3]).count();
            ui.label(RichText::new(format!("{n} {}", tr("aresta(s) escolhida(s)"))).small());
            crate::editor::check(ui, &mut lp.loop_all, "usar o laço da face inteira").on_hover_text(tr("Liga: a partir da primeira aresta escolhida, cria uma aba em cada aresta reta do contorno da face. Desliga: só as arestas escolhidas."));
            if ui.add_enabled(n > 0, egui::Button::new(tr("Limpar"))).on_hover_text(tr("Tira as arestas escolhidas para escolher de novo.")).clicked() {
                lp.edges = vec![[0.; 3]];
                lp.edge_keys.clear();
            }
        }
        _ => {}
    }
}

/// A barra de modos (miniaturas). Troca a feature para o modo clicado, levando o que é comum.
pub fn modos(ui: &mut egui::Ui, kind: &mut FeatureKind) {
    let Some(atual) = modo_de(kind) else { return };
    section(ui, "Modo da aba");
    let opts: Vec<(&str, &str, &str, bool)> = MODOS.iter().zip(MODOS_ABA).map(|((ic, n, d), m)| (*ic, tr(n), tr(d), m == atual)).collect();
    if let Some(i) = crate::editor::icon_choice(ui, &opts) {
        if let Some(mut novo) = converter_aba(kind, MODOS_ABA[i]) {
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
        ModoAba::Aresta => "Aresta: uma aba, com recuos laterais.",
        ModoAba::Laco => "Laço: uma aba por aresta, com extensão lateral e mitra.",
        ModoAba::Curva => "Curva: a aba segue a aresta curva e as tangentes a ela.",
        ModoAba::Perfil => "Perfil: a aba segue uma cadeia aberta de um sketch.",
        ModoAba::Superficie => "Superfície: a aba se apoia numa superfície de suporte.",
    })).weak().small());
}
