// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da análise de saída por face (B197), da regra plástica (B198) e dos recursos de
//! plástico (B148), à parte de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    // B197
    ("Pintar", "Paint"),
    ("Ponto a ponto", "Per point"),
    ("Por face", "Per face"),
    ("Cor de cada ponto pelo ângulo de saída ali.", "Color of each point by the draft angle there."),
    ("Cada face na cor do pior ângulo dela: uma face curva que chega a ficar paralela à abertura fica em erro.", "Each face in the color of its worst angle: a curved face that reaches parallel to the opening is shown as an error."),
    ("Cavidade olha para a abertura, núcleo para o lado oposto, lateral fica paralela e ambígua tem parte de cada. Undercut em vermelho.", "Cavity faces the opening, core faces away, side stays parallel and ambiguous has some of each. Undercut in red."),
    ("A análise por face pede um corpo só.", "Per-face analysis needs a single body."),
    ("Cavidade", "Cavity"),
    ("Núcleo", "Core"),
    ("Lateral", "Side"),
    ("Ambígua", "Ambiguous"),
    ("Em undercut", "In undercut"),
    ("Sai do molde nesta direção.", "Releases from the mold in this direction."),
    ("Não sai do molde assim: pede saída ou gaveta.", "Does not release like this: needs draft or a side action."),
];
