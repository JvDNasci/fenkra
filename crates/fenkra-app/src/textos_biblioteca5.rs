// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções das famílias novas da biblioteca de fixadores (B51): cabeças escareada e panela,
//! prisioneiros, porca com aba e parafuso de ombro. Tabela própria para não disputar `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Escareada", "Countersunk"),
    ("Panela com cruz", "Pan head, cross recess"),
    ("Parafuso de cabeça com cruz", "Cross recess head screw"),
    ("Prisioneiro", "Stud"),
    ("diâmetro da rosca depois do ombro", "thread diameter after the shoulder"),
    ("rosca da ponta fixa, que entra na peça", "fixed-end thread, screwed into the part"),
    ("rosca do lado da porca", "nut-end thread"),
    ("diâmetro da aba", "flange diameter"),
    ("espessura da aba", "flange thickness"),
    ("Só famílias em polegada (ASME). Os dados só entram conferidos em duas fontes.", "Only inch families (ASME). Data only goes in after being checked against two sources."),
];
