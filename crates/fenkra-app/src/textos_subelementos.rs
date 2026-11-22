// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções dos subelementos com nome próprio (B577): "Dono · Face 1 · Aresta 2 · Vértice 1" e a árvore
//! de subelementos da caixa de seleção, à parte de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    // nomes que esperam o cálculo (a coordenada fica só na dica)
    ("Face sem nome (espera o cálculo)", "Unnamed face (waiting for the calculation)"),
    ("Aresta sem nome (espera o cálculo)", "Unnamed edge (waiting for the calculation)"),
    ("Vértice sem nome (espera o cálculo)", "Unnamed vertex (waiting for the calculation)"),
    ("Ponto sem nome (espera o cálculo)", "Unnamed point (waiting for the calculation)"),
    ("Célula", "Cell"),
    // árvore de subelementos
    ("Elementos do modelo", "Model elements"),
    (
        "Clique no nome para escolher e acender no 3D; a seta abre os subelementos (face, aresta, vértice). Ctrl soma à seleção.",
        "Click the name to pick it and light it up in 3D; the arrow opens the sub-elements (face, edge, vertex). Ctrl adds to the selection.",
    ),
    ("Escolhe todas as faces deste elemento: elas acendem no 3D e entram no campo ativo (Ctrl soma).", "Picks every face of this element: they light up in 3D and go into the active field (Ctrl adds)."),
    ("Escolhe esta face: ela acende no 3D e entra no campo ativo (Ctrl soma).", "Picks this face: it lights up in 3D and goes into the active field (Ctrl adds)."),
    ("Escolhe esta aresta: ela acende no 3D e entra no campo ativo (Ctrl soma).", "Picks this edge: it lights up in 3D and goes into the active field (Ctrl adds)."),
    ("Escolhe este vértice: ele acende no 3D e entra no campo ativo (Ctrl soma).", "Picks this vertex: it lights up in 3D and goes into the active field (Ctrl adds)."),
];
