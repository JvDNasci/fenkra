// SPDX-License-Identifier: LGPL-2.1-or-later
//! Nome das referências no painel (B516): a face, a aresta ou a ponta escolhida aparece pelo modo como
//! foi construída — dono, papel e índice, como "Extrusão 2 · tampa final" ou "Parede 1 · aresta 3 ·
//! tampa final" — e não pela coordenada. O nome é o mesmo que o recálculo usa para achar a entidade
//! (`fenkra_solidos::refs`), então o que o painel mostra é o que o comando segue.
//!
//! Sem chave (arquivo antigo antes do primeiro cálculo, ou face sem papel conhecido), o campo diz que a
//! face ainda não tem nome e mostra o ponto; o cálculo seguinte promove a chave quando dá.

use crate::i18n::tr;
use fenkra_core::{ChaveAncora, Document, EdgeKey, FaceKey, Feature};
use fenkra_solidos::refs;

fn t(s: &str) -> String {
    tr(s).to_string()
}

fn ponto(p: [f64; 3]) -> String {
    crate::app::p3(p, 1)
}

/// Nome da face pela chave.
pub fn face(doc: &Document, k: &FaceKey) -> String {
    refs::nome_face(doc, k, &t)
}

/// Nome da aresta pela chave.
pub fn aresta(doc: &Document, k: &EdgeKey) -> String {
    refs::nome_aresta(doc, k, &t)
}

/// Texto de um campo de face: o nome, ou "Face sem nome perto de (x, y, z)".
pub fn campo_face(doc: &Document, k: Option<&FaceKey>, p: [f64; 3]) -> String {
    match k {
        Some(k) => face(doc, k),
        None => format!("{} {}", tr("Face sem nome perto de"), ponto(p)),
    }
}

/// Texto de um campo de aresta: o nome, ou "Aresta sem nome perto de (x, y, z)".
pub fn campo_aresta(doc: &Document, k: Option<&EdgeKey>, p: [f64; 3]) -> String {
    match k {
        Some(k) => aresta(doc, k),
        None => format!("{} {}", tr("Aresta sem nome perto de"), ponto(p)),
    }
}

/// Texto de um campo de ponto guardado no comando (`Feature::ancoras`): o nome da âncora quando ela
/// ainda vale para o ponto, senão o ponto.
pub fn campo_ancora(doc: &Document, f: &Feature, campo: &str, p: [f64; 3]) -> String {
    match f.ancora(campo, p) {
        Some(a) => refs::nome_ancora(doc, a, &t),
        None => format!("{} {}", tr("Ponto sem nome em"), ponto(p)),
    }
}

/// Nome do eixo que segue uma face ou uma aresta do modelo (B516).
pub fn eixo(doc: &Document, a: &fenkra_core::AxisRef) -> String {
    match a {
        fenkra_core::AxisRef::Face { key: Some(k), .. } => format!("{} {}", tr("Eixo de"), face(doc, k)),
        fenkra_core::AxisRef::Face { .. } => tr("Eixo da face").to_string(),
        fenkra_core::AxisRef::Edge { key: Some(k), .. } => aresta(doc, k),
        _ => tr("Aresta do modelo").to_string(),
    }
}

/// Explicação do nome, para a dica do campo.
pub fn dica_face() -> &'static str {
    tr("O nome vem do comando que criou a face e do papel dela; ele segue a face quando a peça muda. Sem nome, a face ainda é achada pelo ponto, e ganha nome no próximo cálculo.")
}

/// Explicação do nome da aresta, para a dica do campo.
pub fn dica_aresta() -> &'static str {
    tr("O nome vem das duas faces vizinhas; ele segue a aresta quando a peça muda. Sem nome, a aresta ainda é achada pelo ponto, e ganha nome no próximo cálculo.")
}

/// Nome curto de uma âncora qualquer (para listas).
#[allow(dead_code)]
pub fn ancora(doc: &Document, a: &ChaveAncora) -> String {
    refs::nome_ancora(doc, a, &t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::Id;

    fn chave(owner: Id, role: i32) -> FaceKey {
        FaceKey { owner, role, kind: 0, normal_or_axis: [0., 0., 1.], area: 1. }
    }

    #[test]
    fn nomes_por_dono_papel_e_indice() {
        let mut d = Document::new("t");
        let sk = d.add_feature(fenkra_core::FeatureKind::Sketch(Default::default()));
        let nome = d.name_of(sk);
        assert_eq!(campo_face(&d, Some(&chave(sk, 1)), [0.; 3]), format!("{nome} · tampa final"));
        // aresta entre a tampa e a lateral 3: a aresta 3 do perfil
        let k = EdgeKey { a: chave(sk, 1), b: chave(sk, 4), edge: None };
        assert_eq!(campo_aresta(&d, Some(&k), [0.; 3]), format!("{nome} · aresta 3 · tampa final"));
        assert!(campo_face(&d, None, [1., 2., 3.]).contains("sem nome"));
        crate::i18n::set_english(true);
        assert_eq!(campo_face(&d, Some(&chave(sk, 0)), [0.; 3]), format!("{nome} · start cap"));
        crate::i18n::set_english(false);
    }
}
