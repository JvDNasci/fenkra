// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da montagem: referências que se perdem quando a peça muda e o reancorar.

pub static TEXTOS: &[(&str, &str)] = &[
    (
        "A peça mudou e a face ou aresta desta restrição não foi reencontrada. A restrição segue com a última posição boa: escolha o elemento de novo e use Reancorar.",
        "The part changed and the face or edge of this constraint was not found again. The constraint keeps the last good position: pick the element again and use Re-anchor.",
    ),
    ("Referência perdida", "Lost reference"),
    ("PERDIDA", "LOST"),
    ("Reancorar na seleção atual:", "Re-anchor on the current selection:"),
    (
        "Clique no 3D a face ou aresta certa da mesma peça e use este botão: a restrição passa a usar ela.",
        "Click the right face or edge of the same part in 3D and use this button: the constraint will use it.",
    ),
    ("Restrição reancorada", "Constraint re-anchored"),
    ("Não deu para reancorar", "Could not re-anchor"),
    ("escolha o elemento novo na mesma peça", "pick the new element on the same part"),
    ("este elemento não serve para esta restrição", "this element does not fit this constraint"),
    ("restrição não encontrada", "constraint not found"),
];
