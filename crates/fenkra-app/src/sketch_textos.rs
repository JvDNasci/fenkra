// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções dos textos do sketch de relimitação, seleção, snaps e vínculos (B18, B19, B96, B97).
//! Ficam num arquivo próprio para não disputar linhas com a tabela geral de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    // relimitação (B18)
    ("clique no trecho a remover; Shift estende; clique no vazio encerra", "click the segment to remove; Shift extends; click empty space to finish"),
    ("clique perto da ponta a estender; Shift apara; clique no vazio encerra", "click near the end to extend; Shift trims; click empty space to finish"),
    ("estender funciona em linha e arco", "extend works on lines and arcs"),
    ("não há curva no caminho da ponta para estender até ela", "there is no curve in the end's path to extend to"),
    ("esse elemento não tem trecho para aparar", "this element has no segment to trim"),
    ("Pare o cursor no trecho: ele fica vermelho tracejado e os cortes aparecem. Clique para remover. Vale para linha, arco, círculo e spline. Shift estende; clique no vazio, botão direito ou Esc encerra.", "Rest the cursor on a segment: it turns dashed red and the cut points appear. Click to remove it. Works on lines, arcs, circles and splines. Shift extends; click empty space, right-click or Esc finishes."),
    ("Pare o cursor perto da ponta de uma linha ou arco: a prévia vai até a próxima curva no caminho. Clique para estender. Shift apara; clique no vazio, botão direito ou Esc encerra.", "Rest the cursor near the end of a line or arc: the preview runs to the next curve in its path. Click to extend. Shift trims; click empty space, right-click or Esc finishes."),
    // seleção e cadeia (B19)
    ("clique ou arraste para selecionar; duplo clique pega a cadeia; Shift soma", "click or drag to select; double-click takes the chain; Shift adds"),
    ("clique na curva; duplo clique pega a cadeia", "click the curve; double-click takes the chain"),
    ("clique no canto, ou numa linha perto dele; duplo clique faz a cadeia", "click the corner, or a line near it; double-click does the whole chain"),
    ("a seleção não forma uma cadeia", "the selection does not form a chain"),
    ("o cursor está sobre a cadeia: afaste para dar a distância", "the cursor is on the chain: move away to set the distance"),
    ("o offset passa do raio de um arco da cadeia", "the offset goes past the radius of an arc in the chain"),
    ("offset de cadeia funciona em linhas e arcos", "chain offset works on lines and arcs"),
    ("o offset é grande demais para esta cadeia", "the offset is too large for this chain"),
    ("não há canto entre duas linhas na seleção", "there is no corner between two lines in the selection"),
    ("a cota do canto sumiu", "the corner dimension is gone"),
    ("alguns cantos não comportam esse valor e ficaram como estavam", "some corners cannot take this value and were left as they were"),
];
