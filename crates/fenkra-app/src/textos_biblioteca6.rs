// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções dos cilindros pneumáticos (B45) e trilhos de guia linear (B46) da biblioteca. Tabela própria para não disputar `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Cilindro pneumático", "Pneumatic cylinder"),
    ("Curso da haste", "Rod stroke"),
    ("curso", "stroke"),
    (
        "Curso da haste em mm. A norma não fixa o curso: o menu traz os cursos padrão do fabricante, e qualquer valor entre o menor e o maior vale.",
        "Rod stroke in mm. The standard does not fix the stroke: the menu lists the manufacturer's standard strokes, and any value between the smallest and the largest is valid.",
    ),
    ("diâmetro da haste", "piston rod diameter"),
    ("rosca da ponta da haste", "piston rod end thread"),
    ("distância entre os furos dos tirantes", "distance between tie-rod holes"),
    ("da tampa dianteira ao ombro da haste", "front cover to rod shoulder"),
    ("diâmetro do ressalto de guia", "guide boss diameter"),
    ("comprimento da rosca da ponta", "rod end thread length"),
    ("comprimento do corpo", "body length"),
    ("rosca dos furos de fixação", "mounting hole thread"),
    ("lado do perfil quadrado", "square profile side"),
    ("Trilho de guia linear", "Linear guide rail"),
    ("Comprimento do trilho", "Rail length"),
    (
        "Comprimento do trilho em mm, livre até o máximo do fabricante. Os furos ficam no passo P, com a mesma sobra nas duas pontas.",
        "Rail length in mm, free up to the manufacturer's maximum. Holes are spaced at pitch P, with the same end distance at both ends.",
    ),
    ("largura do trilho", "rail width"),
    ("altura do trilho", "rail height"),
    ("diâmetro do escareado", "counterbore diameter"),
    ("profundidade do escareado", "counterbore depth"),
    ("diâmetro do furo passante", "through hole diameter"),
    ("passo entre os furos", "hole pitch"),
];
