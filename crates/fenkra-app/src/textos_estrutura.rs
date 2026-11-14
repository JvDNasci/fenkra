// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da estrutura do produto: ativar, voltar e produto novo no contexto (B439).

pub static TEXTOS: &[(&str, &str)] = &[
    ("2 cliques: volta para este produto e atualiza a montagem", "Double-click: goes back to this product and updates the assembly"),
    ("2 cliques: ativa o componente dentro do subconjunto", "Double-click: activates the component inside the subassembly"),
    ("Produto novo no contexto", "New product in context"),
    ("Produto", "Product"),
    ("Arquivo de peça (.fkp) que abre na Modelagem Sólida", "Part file (.fkp) that opens in Part Design"),
    ("Subconjunto vazio (.fka) que vira a montagem ativa para receber peças", "Empty subassembly (.fka) that becomes the active assembly to receive parts"),
    (
        "O produto nasce vazio e abre como a montagem ativa; se voltar sem nada dentro, ele some com o arquivo.",
        "The product is born empty and opens as the active assembly; if you go back with nothing inside, it is removed along with its file.",
    ),
    ("dê um nome ao produto", "give the product a name"),
    ("escolha a pasta do produto", "choose the product folder"),
    ("já existe um produto com este nome nesta pasta", "a product with this name already exists in this folder"),
];
