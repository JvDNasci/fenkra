// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da bancada Chapa metálica nas ondas 2 a 4 (B283 estilos nomeados, B284 extensão
//! lateral da aba e laço de arestas). Tabela própria para não disputar `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    // B283: estilos de chapa
    ("Estilo da chapa", "Sheet style"),
    ("(sem estilo)", "(no style)"),
    ("Nome do estilo", "Style name"),
    ("Salvar como estilo", "Save as style"),
    ("Estilo salvo com material, espessura, raio por fórmula, compensação e alívios. Trocar o estilo recalcula a peça e a tabela de dobras.", "Style saved with material, thickness, formula-driven radius, compensation and reliefs. Switching style recomputes the part and the bend table."),
    ("Guarda os valores atuais da chapa num estilo com este nome (substitui se já existir) e passa a usá-lo.", "Saves the current sheet values as a style with this name (replaces it if it exists) and starts using it."),
    ("Campos com cadeado 🔒 sobrescrevem o estilo; clique no cadeado para voltar ao valor dele.", "Fields with a lock 🔒 override the style; click the lock to return to the style value."),
    ("Este campo sobrescreve o estilo", "This field overrides the style"),
    ("Clique para voltar ao valor do estilo.", "Click to return to the style value."),
    ("O raio aceita fórmula da espessura, por exemplo 1.15*espessura.", "The radius accepts a thickness formula, for example 1.15*espessura."),
    ("Alívio padrão nas abas", "Default relief on flanges"),
    ("Tamanho padrão do canto", "Default corner size"),
    ("Alívios padrão", "Default reliefs"),
    // B284: aba por laço e extensão lateral
    ("Aba por laço", "Flange by loop"),
    ("Aba por laço e extensão lateral", "Flange by loop and side extent"),
    ("Escolha uma aresta (laço da face inteira) ou várias: uma aba por aresta em um comando só, com mitra nos cantos. A extensão pode ser a aresta inteira, uma largura centrada ou deslocada, ou o trecho entre duas referências.", "Pick one edge (the whole face loop) or several: one flange per edge in a single command, mitered at the corners. The extent can be the whole edge, a centered or offset width, or the stretch between two references."),
    ("Arestas da aba", "Flange edges"),
    ("aresta(s) escolhida(s)", "edge(s) picked"),
    ("usar o laço da face inteira", "use the whole face loop"),
    ("Liga: a partir da primeira aresta escolhida, cria uma aba em cada aresta reta do contorno da face. Desliga: só as arestas escolhidas.", "On: starting from the first picked edge, creates a flange on every straight edge of the face outline. Off: only the picked edges."),
    ("Extensão lateral", "Side extent"),
    ("Aresta inteira", "Whole edge"),
    ("Largura centrada", "Centered width"),
    ("Largura deslocada", "Offset width"),
    ("Entre duas referências", "Between two references"),
    ("A aba cobre a aresta toda.", "The flange covers the whole edge."),
    ("Escolha dois vértices ou pontos no 3D: a aba cobre o trecho entre as projeções deles na aresta.", "Pick two vertices or points in 3D: the flange covers the stretch between their projections on the edge."),
    ("Referência 1", "Reference 1"),
    ("Referência 2", "Reference 2"),
    ("medir a partir da outra ponta", "measure from the other end"),
    ("A medida começa na ponta de menor X (depois Y, Z) de cada aresta; ligado, começa na outra.", "The measure starts at the end with the lowest X (then Y, Z) of each edge; when on, it starts at the other."),
];
