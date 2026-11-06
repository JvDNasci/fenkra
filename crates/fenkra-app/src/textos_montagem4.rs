// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções dos limites de junta, do diagnóstico de conflito e da explosão por passos.

pub static TEXTOS: &[(&str, &str)] = &[
    // limites de junta (B36)
    ("Atual", "Current"),
    ("Posição X", "Position X"),
    ("Posição Y", "Position Y"),
    ("Inclinação", "Tilt"),
    ("Trajeto", "Path travel"),
    ("Liga este limite; desligar guarda o valor", "Turns this limit on; turning it off keeps the value"),
    ("Leva a junta a este valor", "Moves the joint to this value"),
    ("Até onde a junta vai para o lado negativo", "How far the joint goes on the negative side"),
    ("Até onde a junta vai para o lado positivo", "How far the joint goes on the positive side"),
    ("Arraste para mudar o limite", "Drag to change the limit"),
    ("O mínimo passou do máximo: a junta fica sem solução.", "The minimum went past the maximum: the joint has no solution."),
    ("Parada no limite", "Stopped at the limit"),
];
