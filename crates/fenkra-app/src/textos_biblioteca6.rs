// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções dos cilindros pneumáticos da biblioteca (B45). Tabela própria para não disputar `i18n.rs`.

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
];
