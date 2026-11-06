// SPDX-License-Identifier: LGPL-2.1-or-later
//! Símbolo de solda preenchido a partir do cordão do modelo (B285). O cordão (cosmético ou sólido)
//! já sabe a perna, o formato e o desenho do intermitente; o símbolo ISO 2553 sai dele sem digitar
//! nada: medida `z` da perna, contorno do formato, e o comprimento `n×l (e)` quando é intermitente.
//! O resultado é uma nota comum (`DwgWeld`), então PDF, SVG e DXF seguem o caminho de sempre.

use crate::simbolos::{normalizar, Comprimento};
use crate::{DwgWeld, Note, NoteKind};
use fenkra_core::pmi::{WeldContour, WeldExtra, WeldKind};
use fenkra_core::{BeadShape, Id};
use fenkra_solidos::st_solda::num_curto;
use fenkra_solidos::CordaoCalculado;

/// Medida do símbolo: filete cotado pela perna (`z5`).
pub fn medida(c: &CordaoCalculado) -> String {
    format!("z{}", num_curto(c.perna))
}

/// Comprimento do símbolo. O contínuo não leva comprimento (vale a junta toda); o intermitente leva
/// `n×l (e)`: `n` trechos do caminho mais longo, `l` o comprimento de cada um e `e` o vão entre eles.
pub fn comprimento(c: &CordaoCalculado) -> String {
    if !c.intermitente() {
        return String::new();
    }
    let n = c.caminhos.iter().map(|p| p.trechos.len()).max().unwrap_or(0);
    Comprimento { n: n.to_string(), l: num_curto(c.comprimento_trecho), e: num_curto(c.vao) }.compor()
}

/// Texto da nota e dados do símbolo para o cordão.
pub fn solda_do_cordao(c: &CordaoCalculado) -> (String, DwgWeld) {
    let contorno = match c.formato {
        BeadShape::Plano => WeldContour::Nenhum,
        BeadShape::Convexo => WeldContour::Convexo,
        BeadShape::Concavo => WeldContour::Concavo,
    };
    let extra = WeldExtra {
        // solda alternada: elementos dos dois lados da junta, defasados
        other: c.alternado.then_some(WeldKind::Filete),
        length: comprimento(c),
        contour: contorno,
        ..Default::default()
    };
    (medida(c), DwgWeld { kind: WeldKind::Filete, all_around: false, extra })
}

/// Nota de solda pronta para a folha; quem chama dá o `id` e a folha.
pub fn nota_do_cordao(c: &CordaoCalculado, id: Id, sheet: Id, pos: [f64; 2], ponta: Option<[f64; 2]>, altura: f64) -> Note {
    let (text, weld) = solda_do_cordao(c);
    let mut n = Note { id, sheet, kind: NoteKind::Weld, pos, leader: ponta, text, size: altura, weld: Some(weld), ..Default::default() };
    normalizar(&mut n);
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout;
    use crate::source::Models;
    use crate::{export, Drawing, Format};
    use fenkra_core::{BeadMode, BeadPattern};
    use fenkra_solidos::{CaminhoCordao, TrechoCordao};

    fn cordao(padrao: BeadPattern, formato: BeadShape, alternado: bool) -> CordaoCalculado {
        let trechos = if padrao == BeadPattern::Continuo { 1 } else { 8 };
        let t = |i: usize| TrechoCordao { pts: vec![[i as f64 * 20., 0., 0.], [i as f64 * 20. + 10., 0., 0.]], comprimento: 10. };
        CordaoCalculado {
            feature: Id(1),
            modo: BeadMode::Cosmetico,
            formato,
            padrao,
            alternado,
            perna: 5.,
            comprimento_trecho: if padrao == BeadPattern::Continuo { 0. } else { 10. },
            vao: if padrao == BeadPattern::Continuo { 0. } else { 10. },
            caminhos: vec![CaminhoCordao { pts: vec![[0., 0., 0.], [160., 0., 0.]], comprimento: 160., fechado: false, trechos: (0..trechos).map(t).collect() }],
            volume: 0.,
        }
    }

    fn itens(n: &Note) -> Vec<layout::Item> {
        layout::note_items(&Drawing::new("t", Format::A4), n, &mut Models::new(None))
    }

    #[test]
    fn continuo_plano_e_o_simbolo_simples_da_medida() {
        let c = cordao(BeadPattern::Continuo, BeadShape::Plano, false);
        assert_eq!(medida(&c), "z5");
        assert_eq!(comprimento(&c), "");
        let n = nota_do_cordao(&c, Id(9), Id(1), [100., 100.], Some([80., 80.]), 3.5);
        // sem nenhum extra a nota é a que o texto antigo criava
        assert!(n.weld.is_none());
        assert_eq!(n.text, "z5");
        assert!(!itens(&n).is_empty());
    }

    #[test]
    fn intermitente_preenche_n_vezes_l_e_o_vao() {
        let c = cordao(BeadPattern::ComprimentoEVao, BeadShape::Plano, false);
        assert_eq!(comprimento(&c), "8×10 (10)");
        let n = nota_do_cordao(&c, Id(9), Id(1), [100., 100.], None, 3.5);
        let w = n.weld.as_ref().expect("com comprimento o símbolo deixa de ser o simples");
        assert_eq!(w.extra.length, "8×10 (10)");
        assert_eq!(w.kind, WeldKind::Filete);
    }

    #[test]
    fn formato_e_alternado_viram_contorno_e_dois_lados() {
        let c = cordao(BeadPattern::ComprimentoEPasso, BeadShape::Convexo, true);
        let (_, w) = solda_do_cordao(&c);
        assert_eq!(w.extra.contour, WeldContour::Convexo);
        assert_eq!(w.extra.other, Some(WeldKind::Filete));
    }

    #[test]
    fn exporta_em_pdf_svg_e_dxf_com_a_medida_e_o_comprimento() {
        let c = cordao(BeadPattern::ComprimentoEVao, BeadShape::Plano, false);
        let n = nota_do_cordao(&c, Id(9), Id(1), [100., 100.], Some([80., 80.]), 3.5);
        let it = itens(&n);
        let svg = export::svg(&it, [297., 210.]);
        let dxf = export::dxf(&it);
        let pdf = export::pdf(&[([297., 210.], it.clone())]);
        for (nome, t) in [("svg", svg.as_str()), ("dxf", dxf.as_str()), ("pdf", &String::from_utf8_lossy(&pdf))] {
            assert!(t.contains("z5"), "{nome} sem a medida");
            // o DXF grava o "×" como escape (\U+00D7); os outros levam o texto como está
            // o PDF grava o texto em Latin-1: o "×" é o byte 0xD7
            let comp = if nome == "pdf" { pdf.windows(11).any(|w| w == b"8\xD710 \\(10\\)") } else if nome == "dxf" { t.contains("8\\U+00D710 (10)") || t.contains("8×10 (10)") } else { t.contains("8×10 (10)") };
            assert!(comp, "{nome} sem o comprimento");
        }
    }
}
