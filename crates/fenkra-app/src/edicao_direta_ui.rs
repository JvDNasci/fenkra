// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painel da edição direta de faces (B33): mover, apagar com cura, deslocar e trocar o raio de um
//! filete. O painel mostra só os campos que valem para a operação escolhida; a prévia no 3D e as
//! setas ficam em `previa.rs` e `handles/solidos.rs`.

use crate::editor::{expr, icon_choice, section, Vars};
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::{carry_keys, DirectEdit, OpDireta};

/// Ícone da operação (árvore e barra de ferramentas).
pub fn icone(op: OpDireta) -> &'static str {
    match op {
        OpDireta::Mover => "edit_mover_face",
        OpDireta::Apagar => "edit_apagar_face",
        OpDireta::Deslocar => "edit_deslocar_face",
        OpDireta::RaioFilete => "edit_raio_filete",
    }
}

fn dica(op: OpDireta) -> &'static str {
    match op {
        OpDireta::Mover => "Escolha as faces de um relevo (as paredes e o fundo de um furo, ou o lado e o topo de um boss) e digite quanto ele anda em X, Y e Z. O volume não muda.",
        OpDireta::Apagar => "Escolha as faces a tirar: o corpo se fecha estendendo as vizinhas, como se aquele detalhe nunca tivesse existido.",
        OpDireta::Deslocar => "Escolha as faces e a distância: positiva acrescenta material (a face sai), negativa tira. Em cilindro e esfera, muda o raio.",
        OpDireta::RaioFilete => "Escolha a face curva de UM filete e dê o raio novo; zero desfaz o filete. Só filetes de raio constante (cilíndricos).",
    }
}

/// Painel do comando aberto.
pub fn painel(ui: &mut egui::Ui, e: &mut DirectEdit, vars: &Vars, picked: &[[f64; 3]]) {
    section(ui, "Operação");
    let opts: Vec<(&str, &str, &str, bool)> = OpDireta::ALL.iter().map(|o| (icone(*o), o.rotulo(), dica(*o), *o == e.op)).collect();
    if let Some(i) = icon_choice(ui, &opts) {
        e.op = OpDireta::ALL[i];
    }
    section(ui, "Valores");
    match e.op {
        OpDireta::Mover => {
            expr(ui, "Deslocamento X", &mut e.delta[0], vars, "mm");
            expr(ui, "Deslocamento Y", &mut e.delta[1], vars, "mm");
            expr(ui, "Deslocamento Z", &mut e.delta[2], vars, "mm");
        }
        OpDireta::Deslocar => expr(ui, "Distância", &mut e.value, vars, "mm"),
        OpDireta::RaioFilete => expr(ui, "Raio novo", &mut e.value, vars, "mm"),
        OpDireta::Apagar => {}
    }
    ui.label(RichText::new(tr(dica(e.op))).weak().small());
    let before = e.faces.clone();
    crate::feat_ui::faces_ui(ui, &mut e.faces, picked);
    e.face_keys = carry_keys(&before, &e.face_keys, &e.faces);
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn clicar_nas_faces_enche_a_lista_e_o_campo_ativo_avanca() {
        let mut k = fenkra_core::FeatureKind::DirectEdit(DirectEdit::novo(OpDireta::Apagar));
        assert!(crate::feat_ui::wants_faces(&k));
        assert_eq!(crate::entradas::campo_ativo(&k), Some(crate::entradas::Espera::Face));
        assert_eq!(crate::entradas::contagem(&k), Some((crate::entradas::Espera::Face, 0)));
        crate::feat_ui::add_face(&mut k, [1., 2., 3.]);
        assert_eq!(crate::entradas::campo_ativo(&k), None);
        assert_eq!(crate::feat_ui::face_marks(&k), vec![[1., 2., 3.]]);
        // clicar de novo na mesma face tira da lista
        crate::feat_ui::toggle_face(&mut k, [1., 2., 3.], None, |a, b| a == b);
        assert!(crate::feat_ui::face_marks(&k).is_empty());
    }

    #[test]
    fn toda_operacao_tem_icone_proprio_e_dica() {
        let mut nomes: Vec<_> = OpDireta::ALL.iter().map(|o| icone(*o)).collect();
        nomes.sort();
        nomes.dedup();
        assert_eq!(nomes.len(), OpDireta::ALL.len());
        assert!(OpDireta::ALL.iter().all(|o| !dica(*o).is_empty()));
    }
}
