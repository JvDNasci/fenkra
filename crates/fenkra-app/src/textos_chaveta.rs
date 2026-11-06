// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções do comando Chaveta (B292). Tabela própria para não disputar `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Chaveta", "Key"),
    ("Inserir chaveta", "Insert key"),
    ("Inserir chaveta entre eixo e cubo", "Insert a key between shaft and hub"),
    ("Escolha o eixo e o cubo: o comando acha os dois rasgos, confere b, t1 e t2 com a tabela e mostra a chaveta fantasma dentro deles. Você escolhe o comprimento da série e a chaveta entra restrita ao rasgo do eixo, sem graus de liberdade. Avisa quando as medidas não casam ou o diâmetro está fora da tabela.", "Pick the shaft and the hub: the command finds both keyways, checks b, t1 and t2 against the table and shows the ghost key inside them. You choose the length from the series and the key is inserted constrained to the shaft keyway, with no degrees of freedom. It warns when the dimensions do not match or the diameter is outside the table."),
    ("Eixo", "Shaft"),
    ("Cubo", "Hub"),
    ("nenhum", "none"),
    ("Escolher", "Pick"),
    ("Peça com o rasgo de chaveta no trecho do eixo. Clique em Escolher e depois no eixo na vista.", "Part with the keyway on the shaft section. Click Pick and then the shaft in the view."),
    ("Peça com o furo e o rasgo de chaveta no cubo (engrenagem, polia). Clique em Escolher e depois no cubo na vista.", "Part with the bore and the keyway in the hub (gear, pulley). Click Pick and then the hub in the view."),
    ("Clique no eixo na vista.", "Click the shaft in the view."),
    ("Clique no cubo na vista.", "Click the hub in the view."),
    ("Escolha o eixo e o cubo: o comando acha os dois rasgos e mostra a chaveta no lugar.", "Pick the shaft and the hub: the command finds both keyways and shows the key in place."),
    ("Chaveta paralela forma A pela tabela do diâmetro do eixo: largura b, altura h, profundidade do rasgo no eixo t1 e no cubo t2.", "Form A parallel key from the table for the shaft diameter: width b, height h, keyway depth in the shaft t1 and in the hub t2."),
    ("Comprimento", "Length"),
    ("Comprimentos da série que cabem na sobreposição dos dois rasgos. O maior já vem escolhido.", "Series lengths that fit the overlap of the two keyways. The longest is already chosen."),
    ("Posição", "Position"),
    ("Afastamento do centro da chaveta em relação ao centro da faixa comum dos dois rasgos, ao longo do eixo. Também se arrasta a alça na vista.", "Offset of the key center from the center of the common range of the two keyways, along the shaft. You can also drag the handle in the view."),
    ("Afasta a chaveta do centro da faixa comum dos rasgos, ao longo do eixo. Limitado ao que cabe nos dois.", "Moves the key away from the center of the common keyway range, along the shaft. Limited to what fits in both."),
    ("centralizada (sem folga)", "centered (no slack)"),
    ("Faixa comum dos rasgos (Z do eixo)", "Common keyway range (shaft Z)"),
    ("Grava a chaveta ao lado da montagem e a insere deitada no piso do rasgo do eixo, com contato e dois planos médios coincidentes: ela entra sem graus de liberdade.", "Saves the key next to the assembly and inserts it lying on the shaft keyway floor, with contact and two coincident mid-planes: it goes in with no degrees of freedom."),
    ("Nada é gravado em disco até você apertar Inserir chaveta.", "Nothing is saved to disk until you press Insert key."),
    ("Chaveta inserida:", "Key inserted:"),
];
