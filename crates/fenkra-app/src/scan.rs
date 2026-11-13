// SPDX-License-Identifier: LGPL-2.1-or-later
//! Modo Scan da árvore (B59): percorrer o histórico vendo a peça como estava em cada comando, sem
//! mudar o objeto em trabalho.
//!
//! Ligado, cada comando escolhido na árvore (clique ou setas) recua a barra de fim de peça até ele só
//! para olhar; ao desligar, abrir um comando ou salvar, a barra volta para onde estava. O Scan não
//! deixa rastro no arquivo: quem salva no meio do Scan grava a peça na posição de antes.

use crate::app::{App, Sel};
use fenkra_core::{Document, Id};

/// Onde a barra de fim de peça fica para mostrar o comando `id`: logo depois dele. `None` = o
/// comando não pertence à linha do tempo (sketch solto, geometria de referência).
pub fn barra_para(doc: &Document, id: Id) -> Option<usize> {
    let i = doc.feature_index(id)?;
    (!doc.is_free(&doc.features[i])).then_some(i + 1)
}

/// Liga ou desliga o Scan. Ligar guarda a posição da barra de agora.
pub fn alternar(app: &mut App) {
    if app.scan_volta.is_some() {
        sair(app);
    } else {
        app.scan_volta = Some(app.open.doc.rollback);
    }
}

/// Sai do Scan: a barra volta para onde estava e a peça recalcula.
pub fn sair(app: &mut App) {
    if let Some(v) = app.scan_volta.take() {
        if app.open.doc.rollback != v {
            app.open.doc.rollback = v;
            app.request_recompute();
        }
    }
}

/// Um quadro do Scan: segue o comando escolhido na árvore. Comando aberto encerra o Scan.
pub fn quadro(app: &mut App) {
    if app.scan_volta.is_none() {
        return;
    }
    if app.session.is_some() || app.sketch.is_some() {
        sair(app);
        return;
    }
    let Sel::Feature(id) = app.sel else { return };
    if let Some(b) = barra_para(&app.open.doc, id) {
        if app.open.doc.rollback != b {
            app.open.doc.rollback = b;
            app.request_recompute();
        }
    }
}

/// Textos novos (inglês), somados à tabela de `i18n.rs`.
pub static TEXTOS: &[(&str, &str)] = &[
    ("🔍 Scan", "🔍 Scan"),
    ("Scan: escolha comandos na árvore (clique ou setas) para ver a peça como estava em cada um. O objeto em trabalho não muda; ao desligar, a peça volta.", "Scan: pick commands in the tree (click or arrows) to see the part as it was at each one. The work object does not change; when turned off, the part comes back."),
    ("Scan ligado: vendo a peça até", "Scan on: viewing the part up to"),
];

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_core::{BoolOp, FeatureKind, OriginPlane, PlaneRef, PrimKind, Primitive};

    #[test]
    fn barra_fica_logo_depois_do_comando_e_ignora_o_que_esta_fora_da_linha() {
        let mut d = Document::new("t");
        let caixa = |a: &str| FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: a.into(), b: "10".into(), c: "10".into(), d: "0".into(), op: BoolOp::Add });
        let a = d.add_feature(caixa("10"));
        let b = d.add_feature(caixa("20"));
        assert_eq!(barra_para(&d, a), Some(1));
        assert_eq!(barra_para(&d, b), Some(2));
        assert_eq!(barra_para(&d, Id(987_654)), None);
    }
}
