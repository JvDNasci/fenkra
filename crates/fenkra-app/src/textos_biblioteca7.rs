// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções dos prendedores aeronáuticos (B43, B63) e da biblioteca da leva s9: rebites e pega.
//! Tabela própria para não disputar `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Rebite", "Rivet"),
    ("Abaulada", "Domed"),
    ("Pega", "Grip"),
    ("pega", "grip"),
    (
        "Espessura total das chapas que o rebite une. O comprimento é a pega mais 1,5 diâmetro, arredondado para cima na série de 1/16 pol.",
        "Total thickness of the sheets the rivet joins. The length is the grip plus 1.5 diameters, rounded up to the 1/16 in series.",
    ),
    ("A pega é a espessura que o furo atravessa.", "The grip is the thickness the hole goes through."),
    ("Pega grossa demais para a série", "Grip too thick for the series"),
    ("rosca do nariz dianteiro", "front nose thread"),
    ("da face dianteira ao ombro da haste", "front face to rod shoulder"),
    ("do pino do olhal ao ombro da haste", "eye pin to rod shoulder"),
    ("largura do olhal traseiro", "rear eye width"),
    ("diâmetro do pino do olhal", "eye pin diameter"),
    ("diâmetro do tubo", "barrel diameter"),
    ("Minicilindro pneumático ISO 6432", "ISO 6432 mini pneumatic cylinder"),
];
