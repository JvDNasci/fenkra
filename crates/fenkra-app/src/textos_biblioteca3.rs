// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da onda 3 da biblioteca de normas (B282, B280, B281). Tabela própria para não disputar
//! `i18n.rs` com outras bancadas.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Regras do comprimento", "Length rules"),
    ("Rosca além da porca", "Thread past the nut"),
    ("Quantos passos de rosca devem sobrar depois da porca. O comprimento sai do próximo valor da série que cobre pega, arruelas, porca e essa sobra.", "How many thread pitches must stick out past the nut. The length is the next value in the series that covers grip, washers, nut and this allowance."),
    ("Projetar por módulo e dentes", "Design by module and teeth"),
    ("Dimensionar por centros e razão", "Size by center distance and ratio"),
    ("Você escolhe o módulo e os dentes; a distância entre centros e a razão saem do cálculo.", "You pick the module and teeth; the center distance and ratio come from the calculation."),
    ("Você diz a distância entre centros e a razão; o módulo e os dentes saem sugeridos.", "You give the center distance and ratio; the module and teeth are suggested."),
    ("Distância entre os eixos das duas engrenagens: a = m (z1 + z2) / 2.", "Distance between the two gear axes: a = m (z1 + z2) / 2."),
    ("Razão i = z2 / z1", "Ratio i = z2 / z1"),
    ("Quantas voltas do pinhão para uma da coroa. Use 1 ou mais: o pinhão é a menor.", "Pinion turns per gear turn. Use 1 or more: the pinion is the smaller one."),
    ("Sem sugestão para esses valores.", "No suggestion for these values."),
    ("exato", "exact"),
    ("Aplica módulo e dentes ao projeto.", "Applies the module and teeth to the design."),
    ("O esquema do dente vale para engrenagem externa.", "The tooth diagram applies to external gears."),
    ("Gráfico", "Graphic"),
    ("Mostra o eixo no 3D com alças nas arestas dos trechos: arraste uma alça para mudar L ou Ø e a grade acompanha.", "Shows the shaft in 3D with handles on the section edges: drag a handle to change L or Ø and the grid follows."),
    ("Clique num trecho do desenho para adicionar recursos nele.", "Click a section in the drawing to add features to it."),
    ("Trecho", "Section"),
    ("Adicionar recurso", "Add feature"),
    ("Anel elástico", "Retaining ring"),
    ("Furo transversal", "Cross hole"),
    ("Rasgo de chaveta, anel elástico, ranhura, rosca ou furo transversal neste trecho, com a posição medida da borda esquerda ou direita.", "Keyway, retaining ring, groove, thread or cross hole on this section, with the position measured from the left or right edge."),
    ("da esquerda", "from left"),
    ("da direita", "from right"),
    ("Distância da borda escolhida do trecho até a face do recurso mais próxima dela.", "Distance from the chosen edge of the section to the nearest face of the feature."),
    ("Giro do rasgo em torno do eixo; 0° é o lado +X.", "Keyway rotation around the shaft; 0° is the +X side."),
    ("passo grosso", "coarse pitch"),
    ("Remover recurso", "Remove feature"),
];
