// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da acessibilidade básica (B321).

pub static TEXTOS: &[(&str, &str)] = &[
    ("zoom em direção ao cursor", "zoom towards the cursor"),
    ("A roda aproxima o ponto sob o cursor, também enquanto uma alça 3D é arrastada. Desligado, o zoom fecha sobre o centro da vista.", "The wheel zooms in on the point under the cursor, also while a 3D handle is being dragged. Off, the zoom closes in on the view centre."),
    ("escala só da fonte", "font-only scale"),
    ("Aumenta ou diminui só o texto, além da escala da interface.", "Grows or shrinks only the text, on top of the interface scale."),
    ("cores seguras para daltonismo", "colour-blind safe colours"),
    ("Troca verde e vermelho por azul e laranja em ok, aviso, erro e na prévia. Todo estado também tem símbolo.", "Swaps green and red for blue and orange in ok, warning, error and the preview. Every state also has a symbol."),
    ("reduzir movimento", "reduce motion"),
    ("A câmera salta em vez de deslizar e a interface não anima transições.", "The camera jumps instead of gliding and the interface does not animate transitions."),
    // B321: uso por teclado
    ("Uso por teclado", "Keyboard use"),
    ("passa o foco por botões e campos; Enter ou Espaço aciona", "moves the focus through buttons and fields; Enter or Space activates"),
    ("giram a vista; com Shift, movem", "orbit the view; with Shift, pan it"),
    ("aproximam e afastam a vista", "zoom the view in and out"),
    ("escolhem o comando anterior ou o seguinte da árvore", "pick the previous or next command in the tree"),
    ("abre o comando escolhido; com ele aberto, conclui", "opens the picked command; while it is open, finishes it"),
    ("cancela o comando ou solta o foco", "cancels the command or releases the focus"),
];
