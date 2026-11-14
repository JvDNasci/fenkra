// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da análise de qualidade de superfície (B180) e da conversão de malha (B169).

pub static TEXTOS: &[(&str, &str)] = &[
    ("Plano de corte", "Section plane"),
    ("Corta a vista num plano que você move: mostra a seção da peça e o porco-espinho de curvatura no corte.", "Cuts the view with a plane you move: shows the part section and the curvature comb on the cut."),
    ("Esconde o outro lado do plano.", "Hides the other side of the plane."),
    ("Onde o plano corta, do começo ao fim da peça nesse eixo.", "Where the plane cuts, from the start to the end of the part along that axis."),
    ("Porco-espinho na seção", "Curvature comb on the section"),
    ("Dentes perpendiculares à seção, do tamanho da curvatura: dente que salta é curvatura que muda de repente.", "Teeth normal to the section, as long as the curvature: a tooth that jumps is curvature that changes suddenly."),
    ("Altura do pente", "Comb height"),
    ("Altura do maior dente, em fração do tamanho da peça.", "Height of the tallest tooth, as a fraction of the part size."),
    ("Faixas", "Stripes"),
    ("Quantas listras o ambiente tem: mais faixas mostram defeitos menores.", "How many stripes the environment has: more stripes reveal smaller defects."),
    ("Reflexo suave", "Soft reflection"),
    ("Tubos de luz suaves no lugar das faixas: o brilho corre pela peça como num acabamento polido.", "Soft light tubes instead of stripes: the highlight runs over the part as on a polished finish."),
    ("Quantas linhas de mesmo ângulo com a luz.", "How many lines of equal angle to the light."),
    ("Luz", "Light"),
    ("Oblíqua", "Oblique"),
    ("Reconhecer primitivas", "Recognize primitives"),
    ("Planos, cilindros, cones, esferas e toros viram faces exatas; o que não se ajusta fica triangular.", "Planes, cylinders, cones, spheres and tori become exact faces; what does not fit stays triangular."),
    ("Facetado", "Faceted"),
    ("Uma face plana por triângulo, com as faces coplanares fundidas.", "One flat face per triangle, with coplanar faces merged."),
    ("Tolerância do ajuste", "Fit tolerance"),
    ("Quanto um vértice pode ficar fora da superfície reconhecida. Vazio ou zero: um décimo de milésimo da diagonal.", "How far a vertex may lie off the recognized surface. Empty or zero: one ten-thousandth of the diagonal."),
    ("Malhas grandes ficam pesadas (máx. 200 mil triângulos): decime antes.", "Large meshes get heavy (max. 200 thousand triangles): decimate first."),
];
