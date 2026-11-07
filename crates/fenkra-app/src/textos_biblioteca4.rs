// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da inserção da biblioteca em furo, ponto e solto (B374). Tabela própria para não disputar
//! `i18n.rs` com outras bancadas.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Onde inserir", "Where to insert"),
    ("Em furo", "In hole"),
    ("Em ponto", "At point"),
    ("Solto", "Loose"),
    ("solto", "loose"),
    ("aponte uma superfície", "point at a surface"),
    ("Solte o item sobre uma superfície de peça", "Drop the item on a part surface"),
    ("Inserido como peça", "Inserted as a part"),
    ("livre para posicionar", "free to position"),
    ("Adicionar à montagem", "Add to assembly"),
    ("Põe o item agora na montagem, como peça livre no centro da vista, sem precisar clicar na viewport. Depois é só posicionar.", "Adds the item to the assembly now, as a free part at the center of the view, with no click in the viewport. Then just position it."),
    ("Arraste para a viewport e solte: o item entra onde o modo escolhido abaixo manda (furo, ponto ou solto).", "Drag to the viewport and release: the item goes where the mode chosen below says (hole, point or loose)."),
    ("Clique e depois num furo da viewport. O fantasma salta para o furo que casa; Tab inverte o sentido; Alt ao clicar abre a ficha; Esc sai. O item entra restrito, sem graus de liberdade.", "Click, then a hole in the viewport. The ghost jumps to the hole that fits; Tab flips the direction; Alt while clicking opens the datasheet; Esc leaves. The item goes in constrained, with no degrees of freedom."),
    ("Clique e depois numa superfície da viewport. O item pousa no ponto apontado, com o eixo na normal da face, como peça livre para posicionar depois. Alt ao clicar abre a ficha; Esc sai.", "Click, then a surface in the viewport. The item lands on the pointed spot, with its axis along the face normal, as a free part to position later. Alt while clicking opens the datasheet; Esc leaves."),
    ("Clique e depois num lugar da viewport. O item cai solto no espaço, como peça livre para posicionar depois. Alt ao clicar abre a ficha; Esc sai.", "Click, then a place in the viewport. The item drops loose in space, as a free part to position later. Alt while clicking opens the datasheet; Esc leaves."),
    ("Aponte uma superfície e clique · Alt abre a ficha · Esc sai", "Point at a surface and click · Alt opens the datasheet · Esc leaves"),
    ("Clique onde o item deve cair · Alt abre a ficha · Esc sai", "Click where the item should drop · Alt opens the datasheet · Esc leaves"),
];
