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
    // diagnóstico de conflito (B103)
    ("✖ Sem solução; a montagem não muda.", "✖ No solution; the assembly does not change."),
    ("Em conflito com:", "In conflict with:"),
    ("Não fecha nem sozinha: confira as peças fixas e o valor pedido.", "It does not close even alone: check the fixed parts and the requested value."),
    ("O solucionador não achou o caminho: tente aproximar as peças.", "The solver did not find a way: try moving the parts closer."),
    ("Tirar ou mudar esta restrição resolve o conflito", "Removing or changing this constraint solves the conflict"),
    ("A lista pode ter restrições a mais: a montagem é grande para testar uma a uma.", "The list may have extra constraints: the assembly is too large to test one by one."),
    ("GDL restariam com a proposta", "DOF would remain with the proposal"),
    ("agora", "now"),
    ("Não tira grau de liberdade: repete o que já está restrito.", "Removes no degree of freedom: it repeats what is already constrained."),
];
