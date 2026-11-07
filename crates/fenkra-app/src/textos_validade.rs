// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções do aviso de sólido inválido (B413) e do erro de extrusão no plano do sketch (K-10).

pub static TEXTOS: &[(&str, &str)] = &[
    (
        "sólido inválido depois deste comando: o resultado pode dar volume, massa e interferência errados",
        "invalid solid after this command: the result may give wrong volume, mass and interference",
    ),
    (
        "a direção da extrusão está no plano do sketch: o perfil não ganharia espessura. Escolha uma direção que atravesse o plano ou desenhe o sketch em outro plano",
        "the extrusion direction lies in the sketch plane: the profile would get no thickness. Pick a direction that crosses the plane or draw the sketch on another plane",
    ),
    (
        "não foi possível varrer o perfil com o triedro de Frenet: a guia tem inflexão ou trecho quase reto, onde o Frenet gira sem controle; use o modo Corrigido",
        "could not sweep the profile with the Frenet frame: the guide has an inflection or a nearly straight stretch where Frenet twists uncontrollably; use the Corrected mode",
    ),
];
