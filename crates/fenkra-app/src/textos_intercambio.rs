// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da árvore paramétrica de intercâmbio (B266).

pub static TEXTOS: &[(&str, &str)] = &[
    ("Árvore paramétrica", "Parametric tree"),
    ("Importar STEP, BREP ou árvore…", "Import STEP, BREP or tree…"),
    ("Exportar árvore paramétrica…", "Export parametric tree…"),
    ("Grava a receita da peça ou da montagem em JSON aberto: esboços, cotas, comandos, nomes e cores. Não leva a geometria pronta.", "Writes the recipe of the part or assembly as open JSON: sketches, dimensions, commands, names and colors. It does not carry the finished geometry."),
    ("Salve a montagem antes de exportar: as peças são lidas dos arquivos dela.", "Save the assembly before exporting: its parts are read from its files."),
    ("Árvore exportada, com itens de fora: {0}", "Tree exported, with items left out: {0}"),
    ("Pasta para as peças do arquivo", "Folder for the parts of the file"),
    ("Este arquivo tem várias peças ou uma montagem: escolha uma pasta para elas.", "This file has several parts or an assembly: choose a folder for them."),
    ("O comando {0} não calculou: {1}", "Command {0} did not compute: {1}"),
    ("Referências ajustadas pela geometria: {0}.", "References adjusted by geometry: {0}."),
    ("Ficou de fora quando o arquivo foi gravado: {0}", "Left out when the file was written: {0}"),
];
