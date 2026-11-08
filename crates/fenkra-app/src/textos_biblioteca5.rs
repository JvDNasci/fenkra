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
    ("largura interna da corrente 219 provisória: 4,6 mm (as fontes dão 4,6 e 5,00 mm). Meça a corrente usada e mude o parâmetro w da roda e b1 da corrente", "provisional inner width of the 219 chain: 4.6 mm (the sources give 4.6 and 5.00 mm). Measure the chain in use and change parameter w of the sprocket and b1 of the chain"),
];
