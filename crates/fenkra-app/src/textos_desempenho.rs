// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções das opções de desempenho (B347): limite de quadros, sincronismo vertical e contador.
//! Tabela própria para não disputar `i18n.rs` com outras bancadas.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Desempenho", "Performance"),
    ("Limite de quadros", "Frame rate limit"),
    ("30 quadros por segundo", "30 frames per second"),
    ("60 quadros por segundo", "60 frames per second"),
    ("120 quadros por segundo", "120 frames per second"),
    ("144 quadros por segundo", "144 frames per second"),
    ("Sem limite", "No limit"),
    (
        "Quantos quadros por segundo o Fenkra desenha, no máximo, enquanto algo se move (girar, animar, arrastar). Parado, ele não desenha nada. Um limite menor poupa a placa de vídeo e a bateria.",
        "The most frames per second Fenkra draws while something moves (orbiting, animating, dragging). When nothing moves it draws nothing. A lower limit spares the graphics card and the battery.",
    ),
    ("Sincronismo vertical (vsync)", "Vertical sync (vsync)"),
    (
        "Liga: cada quadro espera a tela atualizar, sem rasgos na imagem e com a placa descansando entre os quadros. Desliga: desenha o mais rápido que o limite deixar, o que pode rasgar a imagem e esquentar a placa. Vale na hora.",
        "On: each frame waits for the screen refresh, with no tearing and the graphics card resting between frames. Off: draws as fast as the limit allows, which can tear the image and heat the card. Takes effect right away.",
    ),
    ("Mostrar contador de quadros", "Show frame counter"),
    (
        "Mostra no canto da vista quantos quadros por segundo o Fenkra está desenhando. Parado, mostra \"em repouso\".",
        "Shows in the corner of the view how many frames per second Fenkra is drawing. When nothing moves, it shows \"idle\".",
    ),
    ("quadros/s", "frames/s"),
    ("em repouso", "idle"),
];
