// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções do Desenho: plano de inspeção e relatório de primeiro artigo (B117).

pub static TEXTOS: &[(&str, &str)] = &[
    ("Inspeção", "Inspection"),
    ("CSV", "CSV"),
    ("XLSX", "XLSX"),
    ("Tabela de características (plano de inspeção)", "Characteristics table (inspection plan)"),
    ("Cada cota (menos as de referência), tolerância geométrica e rugosidade vira uma linha numerada, na ordem de leitura da folha, com zona, requisito e limites: ±, classe ISO 286 ou a tolerância geral ISO 2768 citada nas notas. Clique onde a tabela vai ficar.", "Each dimension (except reference ones), geometric tolerance and roughness becomes a numbered row, in the sheet's reading order, with zone, requirement and limits: ±, ISO 286 class or the ISO 2768 general tolerance cited in the notes. Click where the table goes."),
    ("Balões", "Balloons"),
    ("Balões de inspeção", "Inspection balloons"),
    ("Liga e desliga o número junto de cada característica, o mesmo da tabela e do relatório.", "Turns on and off the number next to each characteristic, the same as in the table and the report."),
    ("Plano de inspeção em CSV", "Inspection plan as CSV"),
    ("Formulário 3 da AS9102 (resultados das características) com nominal e limites, e as colunas de resultado em branco para quem mede.", "AS9102 form 3 (characteristic accountability) with nominal and limits, and blank result columns for the inspector."),
    ("Plano de inspeção em planilha", "Inspection plan as spreadsheet"),
    ("O mesmo formulário 3 numa planilha, com código, nome e revisão da peça no cabeçalho.", "The same form 3 in a spreadsheet, with part number, name and revision in the header."),
    ("Características de inspeção", "Inspection characteristics"),
    ("O desenho não tem características para inspecionar: cote a peça antes", "The drawing has no characteristics to inspect: dimension the part first"),
    ("Plano de inspeção gravado", "Inspection plan saved"),
    ("características", "characteristics"),
    ("Plano de inspeção", "Inspection plan"),
    ("Balões numerados das características", "Numbered characteristic balloons"),
    ("um número junto de cada cota, tolerância geométrica e rugosidade, na mesma ordem da tabela de características e do relatório", "a number next to each dimension, geometric tolerance and roughness, in the same order as the characteristics table and the report"),
    ("características (cotas de referência ficam de fora)", "characteristics (reference dimensions are left out)"),
    ("formulário 3 (resultados das características), separado por ponto e vírgula", "form 3 (characteristic accountability), semicolon separated"),
    ("formulário 3 numa planilha, com o código, o nome e a revisão da peça", "form 3 in a spreadsheet, with the part number, name and revision"),
];
