// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções dos efeitos da vista (B139).

pub static TEXTOS: &[(&str, &str)] = &[
    ("Rápido", "Fast"),
    ("Equilibrado", "Balanced"),
    ("Alto", "High"),
    ("Efeitos da vista", "View effects"),
    ("Quanto a vista gasta com sombra e oclusão. Rápido desliga as duas; Alto usa um mapa de sombra maior, borda mais suave e mais pontos de oclusão.", "How much the view spends on shadow and occlusion. Fast turns both off; High uses a larger shadow map, a softer edge and more occlusion samples."),
    ("Sombra suave", "Soft shadow"),
    ("A luz principal projeta sombra de borda suave na própria peça. Custa uma passada a mais por quadro.", "The key light casts a soft-edged shadow on the part itself. Costs one extra pass per frame."),
    ("Oclusão de ambiente", "Ambient occlusion"),
    ("Escurece cantos e frestas onde a luz do ambiente chega menos. Custa uma passada em meia resolução.", "Darkens corners and gaps that ambient light reaches less. Costs one half-resolution pass."),
    ("Material físico (PBR)", "Physical material (PBR)"),
    ("Brilho e reflexo calculados pela rugosidade e pela metalicidade do material. Não custa passada a mais.", "Highlights and reflections computed from the material roughness and metalness. No extra pass."),
    ("Precisam da iluminação realista. No modo leve e em placa sem suporte ficam desligados; ao girar com \"Simplificar ao girar\" a vista usa o nível Rápido.", "They need realistic lighting. In light mode and on unsupported graphics cards they stay off; while orbiting with \"Simplify while orbiting\" the view uses the Fast level."),
    ("A placa de vídeo não aceita sombra e oclusão: a vista passou para o nível Rápido.", "The graphics card does not support shadow and occlusion: the view switched to the Fast level."),
];
