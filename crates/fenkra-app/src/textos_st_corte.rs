// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da lista de corte (B296) e dos dados de dobra para máquina (B332).

pub static TEXTOS: &[(&str, &str)] = &[
    ("Gerar montagem…", "Generate assembly…"),
    ("Abre o resumo com peças, instâncias, pasta e nomes dos arquivos; nada é gravado antes de Confirmar.", "Opens the summary with parts, instances, folder and file names; nothing is written before you confirm."),
    // B332: dados de dobra para máquina
    ("Dados para a dobradeira indisponíveis", "Bender data unavailable"),
    ("Dados para a dobradeira", "Bender data"),
    ("Exportar", "Export"),
    ("L: reto entre tangentes; R: rotação entre planos de dobra; A: ângulo. Texto com ponto e vírgula.", "L: straight between tangents; R: rotation between bend planes; A: angle. Semicolon-separated text."),
    ("Os mesmos dados de LRA com as letras Y (avanço), B (rotação) e C (ângulo).", "The same data as LRA with the letters Y (feed), B (rotation) and C (angle)."),
    ("Pontos do eixo do tubo e o raio de cada dobra, no sistema da rota (início na origem).", "Tube axis points and the radius of each bend, in the route frame (start at the origin)."),
    ("Conferir arquivo…", "Check file…"),
    ("Lê um arquivo LRA, YBC ou XYZ e diz se ele bate com este tubo.", "Reads an LRA, YBC or XYZ file and says whether it matches this tube."),
    ("Absoluto", "Absolute"),
    ("Avanço e rotação acumulados desde o começo do tubo, no lugar de cada valor por dobra.", "Feed and rotation accumulated from the start of the tube, instead of one value per bend."),
    ("Ângulo com retorno", "Angle with springback"),
    ("Escreve o ângulo que a máquina dobra (com o retorno elástico da regra), no lugar do geométrico.", "Writes the angle the machine bends (with the rule's springback) instead of the geometric one."),
    ("Inverter rotação", "Reverse rotation"),
    ("Para máquinas que giram o tubo no sentido contrário. Valide a convenção no software da máquina antes de produzir.", "For machines that turn the tube the other way. Check the convention in the machine software before production."),
    ("Dados de dobra exportados", "Bend data exported"),
    ("O arquivo bate com o tubo", "The file matches tube"),
    ("O arquivo difere do tubo", "The file differs from tube"),
    ("maior diferença", "largest difference"),
    ("O arquivo tem outro número de dobras que o tubo", "The file has a different number of bends than tube"),
];
