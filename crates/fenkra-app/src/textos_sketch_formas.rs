// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções do B549 (spline por pontos de controle) e do B550 (retângulo por 3 pontos,
//! paralelogramo e círculo tangente a 3 elementos).

pub static TEXTOS: &[(&str, &str)] = &[
    // B550: retângulo e círculo
    ("Ret. 3 pontos", "3-point rect."),
    ("Retângulo por 3 pontos", "3-point rectangle"),
    ("Clique nas duas pontas de um lado, em qualquer direção, e depois na altura. Lados vizinhos ficam perpendiculares e os opostos paralelos: o retângulo pode girar.", "Click both ends of one side, in any direction, then the height. Neighbouring sides stay perpendicular and opposite sides parallel: the rectangle can rotate."),
    ("Paralelogramo", "Parallelogram"),
    ("Paralelogramo por 3 cantos", "Parallelogram by 3 corners"),
    ("Clique em três cantos seguidos; o quarto fecha a figura. Os lados opostos ficam paralelos.", "Click three corners in a row; the fourth closes the shape. Opposite sides stay parallel."),
    ("Clique em três pontos da circunferência. Ponto existente clicado fica preso ao círculo.", "Click three points on the circumference. An existing point you click stays on the circle."),
    ("Círculo tangente", "Tangent circle"),
    ("Círculo tangente a 3 elementos", "Circle tangent to 3 elements"),
    ("Clique em três linhas, círculos ou arcos, do lado em que o círculo deve ficar. Ele nasce com as três tangências e fica definido por elas.", "Click three lines, circles or arcs, on the side where the circle should be. It is created with the three tangencies and is fully defined by them."),
    ("Tangente a 3", "Tangent to 3"),
    ("1ª ponta do lado", "1st end of the side"),
    ("2ª ponta do lado", "2nd end of the side"),
    ("2º canto", "2nd corner"),
    ("3º canto", "3rd corner"),
    ("1º elemento", "1st element"),
    ("2º elemento", "2nd element"),
    ("3º elemento", "3rd element"),
    ("ângulo interno", "interior angle"),
    ("Comprimento do segundo lado do paralelogramo. Digite para travar; aceita expressão.", "Length of the parallelogram's second side. Type to lock; accepts expressions."),
    ("Ângulo interno entre o primeiro e o segundo lado, em graus. Digite para travar.", "Interior angle between the first and the second side, in degrees. Type to lock."),
    // B549: spline por pontos de controle
    ("Pontos de controle (polígono)", "Control points (polygon)"),
    ("Edita a curva pelos vértices do polígono de controle, que ela não atravessa. Ao ligar, a spline fica contínua em curvatura; desligue para voltar aos pontos de passagem.", "Edits the curve by the vertices of the control polygon, which it does not pass through. Turning it on makes the spline curvature-continuous; turn it off to go back to the through points."),
    ("ponta da curva: arraste para mover", "curve end: drag to move"),
    ("ponto de controle: arraste para dar forma à curva", "control point: drag to shape the curve"),
];
