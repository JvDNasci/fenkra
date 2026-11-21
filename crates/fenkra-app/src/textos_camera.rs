// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da navegação da câmera (B542, B544): órbita livre, centro da órbita e alinhar à vista.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Centro da órbita no cursor", "Orbit center at cursor"),
    ("Órbita livre (liga/desliga)", "Free orbit (on/off)"),
    ("⊙ Centro de órbita aqui", "⊙ Orbit center here"),
    ("Órbita centrada no ponto escolhido", "Orbit centered on the chosen point"),
    ("Aponte o cursor para uma face, aresta ou vértice para definir o centro da órbita", "Point the cursor at a face, edge or vertex to set the orbit center"),
    ("Órbita livre ligada: gira 360° em qualquer direção", "Free orbit on: turns 360° in any direction"),
    ("Órbita livre desligada: a inclinação para em 90°", "Free orbit off: the tilt stops at 90°"),
    ("órbita livre (360° em qualquer direção)", "free orbit (360° in any direction)"),
    (
        "Ligada (padrão), a câmera passa do polo e gira por cima e por baixo sem limite, com a vista de cabeça para baixo. Desligada, a inclinação para em 90° como numa mesa giratória. Atalho: Alt+L. Para definir o centro da órbita: Alt+C ou duplo clique no botão do meio sobre o modelo.",
        "On (default), the camera goes past the pole and turns over and under without limit, upside down. Off, the tilt stops at 90° like a turntable. Shortcut: Alt+L. To set the orbit center: Alt+C or double-click the middle button on the model.",
    ),
    ("Selecione uma face, um sketch ou um plano para alinhar a vista", "Select a face, a sketch or a plane to align the view"),
    ("Esta face não dá uma direção para alinhar a vista", "This face gives no direction to align the view"),
    ("⟂ Alinhar à vista   N", "⟂ Align to view   N"),
    ("⟂ Olhar pelo eixo desta face", "⟂ Look along this face's axis"),
];
