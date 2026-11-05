// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções dos textos de ajuda de comandos, de fluxo do sketch e do campo numérico (B60, B239,
//! B227). Ficam num arquivo próprio para não disputar linhas com a tabela geral de `i18n.rs`,
//! que as incorpora ao mapa.

pub static TEXTOS: &[(&str, &str)] = &[
    // busca de comandos e próximo passo (B60)
    ("Buscar comando (ex.: extrude, filete, cota)", "Search command (e.g. extrude, fillet, dimension)"),
    ("Nenhum comando encontrado.", "No command found."),
    ("Próximo passo", "Next step"),
    ("crie um Sketch num plano da Origem", "create a Sketch on an Origin plane"),
    ("transforme o sketch em sólido: Extrusão ou Revolução", "turn the sketch into a solid: Extrude or Revolve"),
    ("escolha um comando para a seleção (Ctrl+K busca)", "pick a command for the selection (Ctrl+K searches)"),
    ("selecione uma face ou aresta, ou use Ctrl+K para buscar um comando", "select a face or edge, or press Ctrl+K to search for a command"),
    ("(mantenha o mouse para ver o passo a passo)", "(keep the mouse still to see the steps)"),
];
