// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da soltura na árvore (B446) e do teclado na árvore (B449), à parte de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Não dá para soltar aqui", "Can't drop here"),
    ("Só comandos da linha do tempo entram aqui", "Only timeline commands go here"),
    ("Comando da linha do tempo não vai para pasta de geometria", "A timeline command can't go into a geometry folder"),
    ("A pasta não pode ir para dentro dela mesma", "A folder can't go inside itself"),
    ("A pasta é do corpo {}", "The folder belongs to body {}"),
    ("Já está no corpo {}", "Already in body {}"),
    ("Geometria de referência não entra num corpo: fica numa pasta de geometria", "Reference geometry doesn't go into a body: it stays in a geometry folder"),
    ("Usa o resultado de {}, de outro corpo", "Uses the result of {}, from another body"),
    ("{} usa este comando e fica no corpo atual", "{} uses this command and stays in the current body"),
    ("A ordem violaria a dependência de {}", "The order would break the dependency of {}"),
    ("O corpo {} não tem sólido antes deste comando para ele modificar", "Body {} has no solid before this command for it to modify"),
    ("Este lugar não guarda este item", "This place doesn't hold this item"),
    ("Objeto não encontrado", "Object not found"),
];
