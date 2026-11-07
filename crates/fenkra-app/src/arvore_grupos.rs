// SPDX-License-Identifier: LGPL-2.1-or-later
//! Agrupar a geometria na árvore (B429). Uma peça mestre com esqueleto chega a 60 itens de geometria
//! e 89 publicações; em lista plana, com nomes curtos, ninguém acha nada. Passando do limiar, os
//! itens se separam por classe (planos, pontos, curvas, superfícies), cada grupo recolhido e com a
//! contagem. Até o limiar a lista continua plana: grupo para três itens só atrapalha.

use crate::i18n::tr;
use eframe::egui::Ui;
use fenkra_core::{RefClass, RefElement};

/// Acima desta quantidade de itens numa lista, ela se divide em grupos por classe.
pub const LIMIAR: usize = 8;

/// Ordem e nome de cada classe na árvore.
pub const CLASSES: [(RefClass, &str, &str); 4] = [
    (RefClass::Plane, "Planos", "plano"),
    (RefClass::Point, "Pontos", "ponto_coord"),
    (RefClass::Curve, "Curvas", "spline"),
    (RefClass::Surface, "Superfícies", "sup_extraida"),
];

/// Separa os itens por classe, na ordem de `CLASSES`; classes sem item não aparecem.
pub fn separar<'a>(refs: &[&'a RefElement]) -> Vec<(&'static str, &'static str, Vec<&'a RefElement>)> {
    CLASSES
        .iter()
        .filter_map(|(c, nome, icone)| {
            let v: Vec<&RefElement> = refs.iter().copied().filter(|r| r.kind.class() == *c).collect();
            (!v.is_empty()).then_some((*nome, *icone, v))
        })
        .collect()
}

/// A lista deve ser dividida em grupos?
pub fn deve_agrupar(n: usize) -> bool {
    n > LIMIAR
}

/// Mostra as referências: planas até o limiar, em grupos por classe acima dele.
pub fn por_classe(ui: &mut Ui, salt: (&'static str, u64), refs: &[&RefElement], mut linha: impl FnMut(&mut Ui, &RefElement)) {
    if !deve_agrupar(refs.len()) {
        for r in refs {
            linha(ui, r);
        }
        return;
    }
    for (nome, icone, grupo) in separar(refs) {
        let titulo = format!("{} ({})", tr(nome), grupo.len());
        let _ = crate::tree_ui::grupo(ui, (salt, nome), Some(icone), titulo, false).corpo(ui, |ui| {
            for r in &grupo {
                linha(ui, r);
            }
        });
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_core::RefKind;

    fn ponto(i: u64) -> RefElement {
        let mut d = fenkra_core::Document::new("t");
        let id = d.add_geometry(RefKind::PointCoords { x: i.to_string(), y: "0".into(), z: "0".into(), frame: None });
        d.references.iter().find(|r| r.id == id).cloned().unwrap()
    }

    #[test]
    fn so_divide_acima_do_limiar() {
        assert!(!deve_agrupar(LIMIAR));
        assert!(deve_agrupar(LIMIAR + 1));
    }

    #[test]
    fn separa_por_classe_na_ordem_da_arvore_e_omite_as_vazias() {
        let pts: Vec<RefElement> = (0..3).map(ponto).collect();
        let refs: Vec<&RefElement> = pts.iter().collect();
        let g = separar(&refs);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].0, "Pontos");
        assert_eq!(g[0].2.len(), 3);
    }
}
