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
    ("Dobra cônica", "Conical bend"),
    ("Dobra cônica (tronco de cone)", "Conical bend (cone frustum)"),
    ("Enrola a chapa num tronco de cone de eixo Z: raios internos das duas bocas e altura. Planificada, vira o setor de coroa do desenvolvimento exato.", "Rolls the sheet into a cone frustum along the Z axis: inner radii of both mouths and height. Flattened, it becomes the exact annular sector of the development."),
    ("Raio da boca menor", "Smaller mouth radius"),
    ("Raio da boca maior", "Larger mouth radius"),
    ("Altura do tronco", "Frustum height"),
    ("Centro da boca menor", "Smaller mouth center"),
    ("fator K próprio (senão o da chapa)", "own K factor (else the sheet's)"),
    ("Os raios são internos; a espessura vem da chapa. O eixo do cone é o Z. Planificada, o tronco vira um setor de coroa: a geratriz mantém o comprimento e o arco abre 2π·sen α.", "The radii are inner; the thickness comes from the sheet. The cone axis is Z. Flattened, the frustum becomes an annular sector: the generatrix keeps its length and the arc opens 2π·sin α."),
    ("Com fator K diferente de 0,5 o volume plano difere um pouco do dobrado; o comando avisa o quanto.", "With a K factor other than 0.5 the flat volume differs slightly from the bent one; the command says by how much."),
];
