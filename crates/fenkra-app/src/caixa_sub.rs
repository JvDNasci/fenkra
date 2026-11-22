// SPDX-License-Identifier: LGPL-2.1-or-later
//! B577: a árvore de subelementos da caixa de seleção (ADR 0107). Cada elemento do modelo mostra a
//! hierarquia com nomes estáveis, "Dono · Face 1 · Aresta 2 · Vértice 1" (superfície: face, arestas e
//! vértices; corpo: faces, arestas e vértices), e cada nível pode ser aberto (seta) e escolhido (clique
//! no nome). A escolha vai como seleção do app: acende no 3D, entra no campo ativo do comando (que valida
//! o tipo como num clique no 3D) e serve a qualquer comando que use a caixa. Ctrl soma à seleção.
//!
//! Os nomes vêm de `fenkra_solidos::nomes_sub`, montados do último cálculo; coordenada só na dica.
//! Cores só pelo tema (o estado de seleção usa o fundo de seleção do tema).

use crate::caixa_selecao::{self as cs, Pedido};
use crate::contrato::TipoEntrada;
use crate::i18n::tr;
use crate::picking::Pick;
use eframe::egui::{self, collapsing_header::CollapsingState};
use fenkra_core::Document;
use fenkra_solidos::nomes_sub::{ArestaN, FaceN, SubNomes};

/// Campos em que a árvore de subelementos ajuda: os que aceitam face, aresta, superfície ou curva.
pub fn vale_para(tipo: TipoEntrada) -> bool {
    matches!(tipo, TipoEntrada::Face | TipoEntrada::Aresta | TipoEntrada::Superficie | TipoEntrada::Curva)
}

/// A seleção que corresponde a uma face da tabela.
pub fn pick_da_face(f: &FaceN) -> Pick {
    Pick::Face { body: f.corpo, face: f.face, owner: f.dono, point: f.ponto.map(|v| v as f32), normal: f.dir.map(|v| v as f32) }
}

/// A seleção que corresponde a uma aresta da tabela.
pub fn pick_da_aresta(a: &ArestaN) -> Pick {
    Pick::Edge { body: a.corpo, edge: a.aresta, mid: a.meio }
}

/// A seleção que corresponde a uma ponta (1 ou 2) de aresta da tabela.
pub fn pick_do_vertice(a: &ArestaN, ponta: u32) -> Pick {
    Pick::Vertex { body: a.corpo, pos: a.pontas[(ponta as usize - 1).min(1)] }
}

fn clicavel(ui: &mut egui::Ui, ctx: &egui::Context, texto: &str, dica: &str, picks: &[Pick]) {
    let r = ui.selectable_label(false, texto).on_hover_text(dica);
    if r.clicked() {
        let somar = ui.input(|i| i.modifiers.command);
        cs::pedir(ctx, Pedido::Selecionar { picks: picks.to_vec(), somar });
    }
}

fn dica_com_ponto(base: &str, p: [f32; 3]) -> String {
    format!("{base}\n{}: {}", tr("Ponto de referência"), crate::app::p3(p, 1))
}

fn arestas_da_face(ui: &mut egui::Ui, ctx: &egui::Context, t: &SubNomes, salt: &str, fi: usize) {
    let dica_a = tr("Escolhe esta aresta: ela acende no 3D e entra no campo ativo (Ctrl soma).");
    for ai in t.arestas_da_face(fi) {
        let a = &t.arestas[ai];
        let id = egui::Id::new((salt, "sub_a", a.corpo, a.aresta));
        let est = CollapsingState::load_with_default_open(ctx, id, false);
        let h = est.show_header(ui, |ui| clicavel(ui, ctx, &format!("{} {}", tr("Aresta"), a.numero), &dica_com_ponto(dica_a, a.meio), &[pick_da_aresta(a)]));
        h.body(|ui| {
            for k in [1u32, 2] {
                let p = a.pontas[k as usize - 1];
                clicavel(ui, ctx, &format!("{} {k}", tr("Vértice")), &dica_com_ponto(tr("Escolhe este vértice: ele acende no 3D e entra no campo ativo (Ctrl soma)."), p), &[pick_do_vertice(a, k)]);
            }
        });
    }
}

/// A árvore de subelementos do modelo calculado, recolhida por padrão, para a caixa do campo `salt`.
pub fn navegador(ui: &mut egui::Ui, ctx: &egui::Context, doc: &Document, salt: &str) {
    let Some(t) = crate::ref_nome::tabela() else { return };
    if t.vazia() {
        return;
    }
    egui::CollapsingHeader::new(egui::RichText::new(tr("Elementos do modelo")).small()).id_salt((salt, "sub_raiz")).default_open(false).show(ui, |ui| {
        ui.label(egui::RichText::new(tr("Clique no nome para escolher e acender no 3D; a seta abre os subelementos (face, aresta, vértice). Ctrl soma à seleção.")).weak().small());
        for dono in t.donos() {
            let id = egui::Id::new((salt, "sub_d", dono.0));
            let faces = t.faces_do_dono(dono);
            let todas: Vec<Pick> = faces.iter().map(|i| pick_da_face(&t.faces[*i])).collect();
            let est = CollapsingState::load_with_default_open(ctx, id, false);
            let h = est.show_header(ui, |ui| clicavel(ui, ctx, &format!("{} ({})", doc.name_of(dono), faces.len()), tr("Escolhe todas as faces deste elemento: elas acendem no 3D e entram no campo ativo (Ctrl soma)."), &todas));
            h.body(|ui| {
                for fi in faces {
                    let f = &t.faces[fi];
                    let id = egui::Id::new((salt, "sub_f", f.corpo, f.face));
                    let dica = format!("{} · {}", tr("Escolhe esta face: ela acende no 3D e entra no campo ativo (Ctrl soma)."), SubNomes::papel_da_face(f, &|s| tr(s).to_string()));
                    let est = CollapsingState::load_with_default_open(ctx, id, false);
                    let h = est.show_header(ui, |ui| clicavel(ui, ctx, &format!("{} {}", tr("Face"), f.numero), &dica_com_ponto(&dica, f.ponto.map(|v| v as f32)), &[pick_da_face(f)]));
                    h.body(|ui| arestas_da_face(ui, ctx, &t, salt, fi));
                }
            });
        }
    });
}
