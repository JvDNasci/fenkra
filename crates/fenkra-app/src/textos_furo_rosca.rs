// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções do painel de furo e rosca (B373), à parte de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Tipo do furo", "Hole type"),
    ("Nenhuma", "None"),
    ("Sem rosca: diâmetro de passagem pela folga", "No thread: clearance diameter"),
    ("Entrada e fim", "Entry and end"),
    ("Chanfro de entrada", "Entry chamfer"),
    ("Folga em cima", "Top clearance"),
    ("Folga embaixo", "Bottom clearance"),
    ("%", "%"),
    ("Medida em milímetros.", "Measure in millimeters."),
    ("Medida em passos da rosca: 1 passo é a distância entre dois filetes.", "Measure in thread pitches: 1 pitch is the distance between two threads."),
    ("Porcentagem de uma medida de referência, explicada na dica do campo.", "Percentage of a reference measure, explained in the field tip."),
    (
        "Cone de 45 graus na boca do furo, para o parafuso entrar liso. Zero ou vazio: sem chanfro. Em %, é a porcentagem da profundidade do filete: 100% chega ao diâmetro externo da rosca.",
        "45 degree cone at the hole mouth, so the screw goes in smoothly. Zero or empty: no chamfer. In %, it is a percentage of the thread depth: 100% reaches the outer diameter of the thread.",
    ),
    (
        "Trecho liso entre a boca (ou o fim do rebaixo) e o começo do sulco. Vazio: o sulco começa na boca e atravessa a entrada. Em %, é a porcentagem da profundidade útil do furo.",
        "Smooth stretch between the mouth (or the end of the counterbore) and the start of the groove. Empty: the groove starts at the mouth and runs through the entry. In %, it is a percentage of the useful hole depth.",
    ),
    (
        "Trecho liso entre o fim do sulco e o fundo do furo, para o parafuso ou o macho não bater. Em passos da rosca, mm ou % da profundidade útil do furo. Vale quando a profundidade da rosca está vazia.",
        "Smooth stretch between the end of the groove and the hole bottom, so the screw or tap does not hit. In thread pitches, mm or % of the useful hole depth. Applies when the thread depth is empty.",
    ),
    ("Vazio na profundidade da rosca = a do furo menos a folga embaixo.", "Empty thread depth = the hole depth minus the bottom clearance."),
    (
        "Ângulo onde o sulco cruza a boca, contado a partir da referência (regra da mão direita em torno do eixo do furo).",
        "Angle where the groove crosses the mouth, counted from the reference (right-hand rule around the hole axis).",
    ),
    ("referência do ângulo: linha, aresta ou eixo", "angle reference: line, edge or axis"),
    ("Referência", "Reference"),
    ("Referência automática: a direção X do mundo, projetada no plano do furo.", "Automatic reference: the world X direction, projected onto the hole plane."),
    ("Ângulo de início", "Start angle"),
];
