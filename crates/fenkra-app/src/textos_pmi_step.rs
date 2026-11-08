// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções do PMI semântico no STEP (B191), à parte de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Anotações semânticas (PMI)", "Semantic annotations (PMI)"),
    ("Grava as cotas com tolerância, os quadros de tolerância geométrica, as referências e os alvos ligados às faces. Rugosidade, solda e notas ainda ficam de fora.", "Writes the toleranced dimensions, the geometric tolerance frames, the datums and the targets tied to faces. Roughness, welds and notes are still left out."),
    ("Exportado", "Exported"),
    ("PMI no arquivo", "PMI in the file"),
    ("Gráfico + semântico", "Graphic + semantic"),
    ("O arquivo leva só a peça, sem anotações.", "The file carries only the part, without annotations."),
    ("Cada anotação vai desenhada em linhas (texto em traços), e as capturas do PMI viram vistas salvas. Quem abre vê, mas não consegue ler os valores como tolerância.", "Each annotation is drawn in lines (stroked text), and the PMI captures become saved views. Whoever opens it sees them, but cannot read the values as tolerances."),
    ("O desenho e, junto, as cotas com tolerância, os quadros, as referências e os alvos ligados às faces, que outro programa lê como dado. Rugosidade, solda e notas vão como desenho.", "The drawing plus the toleranced dimensions, frames, datums and targets tied to faces, which another program reads as data. Roughness, welds and notes go as drawing."),
    ("A anotação {0} vai só como desenho (PMI gráfico): não tem cota, quadro ou referência equivalente ligada à peça no STEP.", "Annotation {0} goes only as a drawing (graphic PMI): it has no equivalent dimension, frame or datum tied to the part in STEP."),
    ("A anotação {0} não tem ponto de ancoragem nem texto e ficou fora do PMI gráfico.", "Annotation {0} has no anchor point nor text and was left out of the graphic PMI."),
    ("PMI gráfico ({0}): rugosidade, solda, notas, balões e legenda ainda não vão no STEP; só cotas, tolerâncias, referências e alvos.", "Graphic PMI ({0}): roughness, welds, notes, balloons and title blocks are not written to STEP yet; only dimensions, tolerances, datums and targets."),
    ("A anotação {0} não está ligada a uma face e ficou fora do STEP.", "Annotation {0} is not tied to a face and was left out of the STEP file."),
    ("A anotação {0} ({1}) não tem tipo equivalente no STEP gravado e ficou fora.", "Annotation {0} ({1}) has no equivalent type in the written STEP and was left out."),
    ("Quadro {0}: o modificador {1} não vai no STEP.", "Frame {0}: the {1} modifier is not written to STEP."),
    ("Quadro {0}: a zona {1} não atravessa o STEP; o gravador escreve só a forma cilíndrica ou esférica.", "Frame {0}: the {1} zone does not cross into STEP; the writer only keeps the cylindrical or spherical form."),
    ("Quadro {0}: a segunda linha do quadro composto não vai no STEP.", "Frame {0}: the second row of the composite frame is not written to STEP."),
    ("A referência {0} aparece em ordens diferentes nos quadros: o STEP guarda uma ordem só por referência.", "Datum {0} appears in different orders across frames: STEP keeps a single order per datum."),
    ("O quadro {0} cita a referência {1}, que não existe no modelo.", "Frame {0} cites datum {1}, which does not exist in the model."),
    ("Alvo {0}: o tipo {1} não existe no STEP gravado.", "Target {0}: type {1} does not exist in the written STEP."),
    ("A anotação {0} não tem um valor numérico e ficou fora do STEP.", "Annotation {0} has no numeric value and was left out of the STEP file."),
];

#[cfg(test)]
mod testes {
    #[test]
    fn toda_nota_do_pmi_no_step_tem_traducao() {
        for s in fenkra_conversor::NOTAS_PMI {
            assert!(crate::i18n::tem_traducao(s), "sem tradução: {s}");
        }
    }
}
