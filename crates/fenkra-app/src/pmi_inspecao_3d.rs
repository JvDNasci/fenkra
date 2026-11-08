// SPDX-License-Identifier: LGPL-2.1-or-later
//! B117: balões de inspeção no PMI 3D. Ligados, cada característica do modelo (cota que não é de
//! referência, quadro de tolerância geométrica e rugosidade) ganha um número num círculo junto do
//! rótulo, na ordem da lista do PMI, como o plano de inspeção do desenho faz na folha.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use fenkra_core::pmi::{Pmi, PmiKind};
use fenkra_core::Id;
use std::sync::atomic::{AtomicBool, Ordering};

static LIGADO: AtomicBool = AtomicBool::new(false);

/// Os balões de inspeção aparecem no 3D?
pub fn ligado() -> bool {
    LIGADO.load(Ordering::Relaxed) || std::env::var("FENKRA_DEMO_PMI_INSPECAO").is_ok()
}

pub fn alternar() {
    LIGADO.fetch_xor(true, Ordering::Relaxed);
}

/// A anotação é uma característica a inspecionar? Cotas de referência (texto entre parênteses) não.
pub fn e_caracteristica(p: &Pmi) -> bool {
    let t = p.text.trim();
    let referencia = t.starts_with('(') && t.ends_with(')');
    match p.kind {
        PmiKind::Linear | PmiKind::Radius | PmiKind::Diameter | PmiKind::Angle | PmiKind::Ordinate { .. } | PmiKind::Gap { .. } | PmiKind::ArcLength => !referencia,
        PmiKind::Gdt { .. } | PmiKind::Roughness { .. } => true,
        _ => false,
    }
}

/// Número de cada característica, na ordem da lista do PMI (as ocultas também contam: o número não
/// muda ao esconder uma anotação).
pub fn numeracao(lista: &[Pmi]) -> Vec<(Id, usize)> {
    lista.iter().filter(|p| e_caracteristica(p)).enumerate().map(|(i, p)| (p.id, i + 1)).collect()
}

/// Desenha o balão de número `n` junto do canto de cima à direita do rótulo.
pub fn desenhar(painter: &egui::Painter, rotulo: Rect, n: usize, linha: Color32, fundo: Color32) {
    let texto = n.to_string();
    let r = 7.5 + 2.5 * texto.len().saturating_sub(1) as f32;
    let c = Pos2::new(rotulo.right() + r + 3., rotulo.top() - r * 0.6);
    painter.circle(c, r, fundo, Stroke::new(1.4, linha));
    painter.text(c, egui::Align2::CENTER_CENTER, texto, egui::FontId::proportional(crate::acessibilidade::fonte_3d(11.)), linha);
    painter.line_segment([c + egui::vec2(-r * 0.7, r * 0.7), rotulo.right_top()], Stroke::new(1., linha));
}

#[cfg(test)]
mod testes {
    use super::*;

    fn pmi(id: u64, kind: PmiKind, text: &str) -> Pmi {
        Pmi { id: Id(id), name: String::new(), kind, anchors: vec![[0.; 3]; 2], offset: [0.; 3], text: text.into(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] }
    }

    #[test]
    fn numera_cotas_quadros_e_rugosidade_e_pula_referencia_e_notas() {
        let lista = vec![
            pmi(1, PmiKind::Note, "Quebrar cantos"),
            pmi(2, PmiKind::Linear, ""),
            pmi(3, PmiKind::Linear, "(40)"),
            pmi(4, PmiKind::Gdt { ch: fenkra_core::pmi::GdtChar::Planeza, tol: "0,05".into(), datums: String::new(), extra: Default::default() }, ""),
            pmi(5, PmiKind::Datum { letter: "A".into() }, ""),
            pmi(6, PmiKind::Roughness { ra: "1,6".into(), process: String::new(), extra: Default::default() }, ""),
            pmi(7, PmiKind::Diameter, ""),
        ];
        assert_eq!(numeracao(&lista), vec![(Id(2), 1), (Id(4), 2), (Id(6), 3), (Id(7), 4)]);
    }
}
