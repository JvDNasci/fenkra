// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções do B40: varredura cônica por três guias e dobra cônica da chapa.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Cônica por 3 guias", "Conic by 3 guides"),
    ("Guia do ápice", "Apex guide"),
    ("ρ pelo raio de curvatura na guia 1 (G2 exato)", "ρ from the curvature radius at guide 1 (exact G2)"),
    ("Raio na guia 1", "Radius at guide 1"),
    ("o ápice, a guia 1 e a guia 2 estão alinhados numa seção, ou o raio é inválido", "the apex, guide 1 and guide 2 are collinear in a section, or the radius is invalid"),
    ("dobra cega (só a faixa entre as pontas da linha gira)", "blind bend (only the strip between the line ends turns)"),
    ("Furo de alívio", "Relief hole"),
    ("Vazio usa o mínimo aeronáutico: o maior entre 3 mm e 2 × espessura + raio. Um furo menor sobe a esse mínimo.", "Empty uses the aeronautical minimum: the larger of 3 mm and 2 × thickness + radius. A smaller hole is raised to that minimum."),
    ("aeronáutico", "aeronautical"),
];
