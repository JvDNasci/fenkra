// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções do B448: comando Nota com vários alvos e painel próprio do comando.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Ctrl ou Shift somam · Enter cria", "Ctrl or Shift add · Enter creates"),
    ("Elementos escolhidos", "Picked elements"),
    ("O texto junto ao cursor mostra o passo do comando e o que serve para esta anotação.", "The text next to the cursor shows the command step and what fits this annotation."),
    ("Texto da nota", "Note text"),
    ("O que a nota diz. Pode ficar vazio e ser escrito depois, sobre a própria nota.", "What the note says. It can stay empty and be written later, on the note itself."),
    ("ex.: Quebrar cantos", "e.g.: Break edges"),
    ("Cada alvo vira uma seta da nota.", "Each target becomes an arrow of the note."),
    ("clique uma face, aresta ou vértice no 3D", "click a face, edge or vertex in 3D"),
    ("Clique simples cria a nota com uma seta. Com Ctrl ou Shift os cliques somam alvos, uma seta por alvo, e a nota nasce com Enter ou OK.", "A plain click creates the note with one arrow. With Ctrl or Shift the clicks add targets, one arrow per target, and the note is created with Enter or OK."),
    ("✔ OK", "✔ OK"),
    ("Cria a nota com uma seta para cada alvo (Enter)", "Creates the note with one arrow per target (Enter)"),
];
