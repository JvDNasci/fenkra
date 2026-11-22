// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções do tubo dobrado por caminho (B575). Tabela própria para não disputar `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Cantos viram dobras (tubo dobrado)", "Corners become bends (bent tube)"),
    ("Os cantos da guia viram dobras de um tubo só, com raio de dobra: nada de encaixe de canto nas dobras. A lista de corte traz o comprimento desenvolvido e a tabela de dobras (avanço, rotação, ângulo e raio) para a dobradeira.", "The guide corners become bends of a single tube, with a bend radius: no corner cope at the bends. The cut list gives the developed length and the bend table (advance, rotation, angle and radius) for the bender."),
    ("A dobra de tubo vale só em tubo (redondo, quadrado, retangular).", "Tube bending only applies to tubes (round, square, rectangular)."),
    ("Raio de centro das dobras. Vazio usa o mínimo automático pela seção (regra prática: 2 vezes o diâmetro, 3 vezes em parede fina); uma regra de dobra cadastrada e o valor digitado mandam sobre ele.", "Centerline radius of the bends. Empty uses the automatic minimum for the section (rule of thumb: 2 times the diameter, 3 times for thin walls); a registered bend rule and a typed value override it."),
    ("Número, parâmetro ou expressão, em mm.", "Number, parameter or expression, in mm."),
    ("Unir as guias numa cadeia (um tubo só)", "Join the guides into a chain (one tube)"),
    ("Liga as guias na ordem da lista como uma curva só; o tubo dobra nos cantos entre elas. Desligado, cada guia é um tubo separado. Também dá para unir só algumas pelo botão direito na lista.", "Links the guides in list order as a single curve; the tube bends at the corners between them. Off, each guide is a separate tube. You can also join only some of them with the right button in the list."),
    ("Mínimo automático e regras: o raio vem de regra prática (não é norma); confira com a sua dobradeira.", "Automatic minimum and rules: the radius comes from a rule of thumb (not a standard); check it with your bender."),
    ("Exportar tabela de dobras…", "Export bend table…"),
    ("Tabela de dobras exportada", "Bend table exported"),
    ("Para cada dobra: avanço reto antes, rotação do plano, ângulo, raio de centro, arco, coordenadas do ponto de dobra e posição no tubo reto. CSV com ponto e vírgula.", "For each bend: straight advance before it, plane rotation, angle, centerline radius, arc, bend point coordinates and position on the straight tube. Semicolon-separated CSV."),
    ("Avanço", "Advance"),
    ("Ponto de dobra", "Bend point"),
];
