// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutura do produto (B439): uma regra só para peça e subproduto.
//!
//! Dois cliques num componente o ativam: a peça abre na bancada dela, o subproduto vira a montagem
//! ativa, e o resto do conjunto fica em volta, esmaecido. Dois cliques num produto de cima (o raiz
//! ou um subproduto intermediário) voltam até ele e atualizam o que mudou no caminho.
//!
//! Antes, a árvore da peça ativa só mostrava a peça: o produto raiz não estava lá para receber os
//! dois cliques, e a única saída era o botão Voltar. Na montagem, as peças de dentro de um
//! subproduto eram texto solto na árvore, sem os dois cliques que valem para as de fora.

use crate::asm_ui::{edit_part, nivel_ativo, AsmState, Volta};
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::Id;

/// Nomes dos produtos do topo até o ativo, inclusive. O índice é o nível na estrutura.
pub fn produtos(st: &AsmState) -> Vec<String> {
    st.pais.iter().map(|p| p.asm.name.clone()).chain(std::iter::once(st.asm.name.clone())).collect()
}

/// Uma linha por produto, do topo para baixo, cada nível um passo mais para dentro. Devolve o nível
/// que recebeu os dois cliques.
fn linhas(ui: &mut egui::Ui, nomes: &[String]) -> Option<usize> {
    let mut pedido = None;
    for (k, nome) in nomes.iter().enumerate() {
        let r = ui
            .horizontal(|ui| {
                ui.add_space(k as f32 * 14.);
                crate::tree_ui::icone(ui, if k == 0 { "asm_nova" } else { "asm_subconjunto" });
                ui.selectable_label(false, RichText::new(nome).strong())
            })
            .inner
            .on_hover_text(tr("2 cliques: volta para este produto e atualiza a montagem"));
        if r.double_clicked() {
            pedido = Some(k);
        }
    }
    pedido
}

/// Topo da árvore da peça ativa: os produtos em que ela está. A volta fica para o fim do quadro,
/// como a do botão da viewport.
pub fn acima_da_peca(st: &mut AsmState, ui: &mut egui::Ui) {
    if st.editing_part.is_none() {
        return;
    }
    if let Some(k) = linhas(ui, &produtos(st)) {
        st.volta_adiada = Some(Volta::Nivel(k));
    }
}

/// Topo da árvore da montagem com um subproduto ativo: os produtos de cima dele.
pub fn acima_do_subproduto(st: &mut AsmState, ui: &mut egui::Ui) {
    if st.pais.is_empty() || st.editing_part.is_some() {
        return;
    }
    let mut nomes = produtos(st);
    nomes.pop();
    if let Some(k) = linhas(ui, &nomes) {
        st.volta_adiada = Some(Volta::Nivel(k));
    }
    ui.add_space(2.);
}

/// Ativa um componente de dentro de subprodutos: cada subproduto do caminho vira o ativo, e o
/// último (peça ou subproduto) abre como em dois cliques direto nele. Para no primeiro passo que
/// não desce, como a pergunta de gravação ou um arquivo que falta.
pub fn ativar_caminho(app: &mut crate::app::App, caminho: &[Id]) {
    for id in caminho {
        let antes = app.asm.as_ref().map_or(0, nivel_ativo);
        edit_part(app, *id);
        if app.asm.as_ref().map_or(0, nivel_ativo) <= antes {
            return;
        }
    }
}

/// `FENKRA_DEMO_ASM=subproduto`: o exemplo das duas placas com um subproduto ao lado, que tem uma
/// peça própria (Tampa). Serve para conferir com o mouse a ida e a volta pelos níveis da estrutura.
pub fn demo(st: &mut AsmState) {
    use fenkra_montagem::{Assembly, NodeKind, Placement};
    let Some(dir) = st.dir() else { return };
    let mut tampa = fenkra_solidos::example_document();
    tampa.name = "Tampa".into();
    let _ = std::fs::write(dir.join("tampa.fkp"), tampa.to_json());
    let mut sub = Assembly::new("Subproduto");
    sub.add_component("Tampa", NodeKind::Part { path: "tampa.fkp".into() }, Placement::default(), None);
    let _ = std::fs::write(dir.join("subproduto.fka"), sub.to_json());
    st.asm.add_component("Subproduto", NodeKind::SubAssembly { path: "subproduto.fka".into() }, Placement { origin: [0., 140., 0.], ..Default::default() }, None);
    // gravada já com o subproduto: o teste de mouse abre este arquivo pela linha de comando
    if let Some(f) = &st.file {
        let _ = std::fs::write(f, st.asm.to_json());
    }
    st.dirty = true;
}
