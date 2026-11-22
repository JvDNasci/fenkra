// SPDX-License-Identifier: LGPL-2.1-or-later
//! Nome das referências no painel (B516): a face, a aresta ou a ponta escolhida aparece pelo modo como
//! foi construída — dono, papel e índice, como "Extrusão 2 · tampa final" ou "Parede 1 · aresta 3 ·
//! tampa final" — e não pela coordenada. O nome é o mesmo que o recálculo usa para achar a entidade
//! (`fenkra_solidos::refs`), então o que o painel mostra é o que o comando segue.
//!
//! B577: com o resultado do último cálculo, o nome é a hierarquia "Dono · Face 1 · Aresta 2 · Vértice 1"
//! (`fenkra_solidos::nomes_sub`), gravada pela chave e nunca pelo ponto. Sem chave (arquivo antigo antes
//! do primeiro cálculo), o campo diz que o elemento ainda não tem nome e espera o cálculo promover a
//! chave; a coordenada só aparece na dica.

use crate::i18n::tr;
use fenkra_core::{ChaveAncora, Document, EdgeKey, FaceKey, Feature};
use fenkra_solidos::nomes_sub::SubNomes;
use fenkra_solidos::refs;
use std::cell::RefCell;
use std::rc::Rc;

fn t(s: &str) -> String {
    tr(s).to_string()
}

fn ponto(p: [f64; 3]) -> String {
    crate::app::p3(p, 1)
}

thread_local! {
    /// Nomes do último cálculo (B577): o app a renova a cada resultado adotado.
    static TABELA: RefCell<Option<Rc<SubNomes>>> = const { RefCell::new(None) };
}

/// Renova a tabela de nomes com o resultado do cálculo.
pub fn atualizar(r: &fenkra_solidos::RecomputeResult) {
    let tabela = SubNomes::de(r);
    TABELA.with(|c| *c.borrow_mut() = Some(Rc::new(tabela)));
}

/// Esquece a tabela (documento fechado ou sem resultado).
#[allow(dead_code)]
pub fn esquecer() {
    TABELA.with(|c| *c.borrow_mut() = None);
}

/// A tabela de nomes do último cálculo.
pub fn tabela() -> Option<Rc<SubNomes>> {
    TABELA.with(|c| c.borrow().clone())
}

/// Nome da face pela chave: "Dono · Face n". Sem a tabela (nada calculado ainda), dono e papel.
pub fn face(doc: &Document, k: &FaceKey) -> String {
    tabela().and_then(|tb| tb.nome_face_chave(doc, k, &t)).unwrap_or_else(|| refs::nome_face(doc, k, &t))
}

/// Nome da aresta pela chave: "Dono · Face n · Aresta m".
pub fn aresta(doc: &Document, k: &EdgeKey) -> String {
    tabela().and_then(|tb| tb.nome_aresta_chave(doc, k, &t)).unwrap_or_else(|| refs::nome_aresta(doc, k, &t))
}

/// Texto de um campo de face: o nome, ou "Face sem nome" (a coordenada fica na dica, ver [`dica_face_em`]).
pub fn campo_face(doc: &Document, k: Option<&FaceKey>, _p: [f64; 3]) -> String {
    match k {
        Some(k) => face(doc, k),
        None => tr("Face sem nome (espera o cálculo)").to_string(),
    }
}

/// Texto de um campo de aresta: o nome, ou "Aresta sem nome" (a coordenada fica na dica).
pub fn campo_aresta(doc: &Document, k: Option<&EdgeKey>, _p: [f64; 3]) -> String {
    match k {
        Some(k) => aresta(doc, k),
        None => tr("Aresta sem nome (espera o cálculo)").to_string(),
    }
}

/// Texto de um campo de vértice guardado como ponto (canto de chapa, nó): o nome do vértice que está ali
/// ("Dono · Face n · Aresta m · Vértice k"), senão "Vértice sem nome". O ponto vai só na dica.
pub fn campo_vertice(doc: &Document, p: [f64; 3]) -> String {
    match tabela().and_then(|tb| tb.vertice_perto(p, 0.5).map(|(a, k)| tb.nome_do_vertice(doc, a, k, &t))) {
        Some(n) => n,
        None => tr("Vértice sem nome (espera o cálculo)").to_string(),
    }
}

/// Texto de uma aresta guardada como ponto (borda de superfície de arquivo antigo): o nome da aresta
/// cujo meio está ali.
pub fn campo_aresta_no_ponto(doc: &Document, p: [f64; 3]) -> String {
    match tabela().and_then(|tb| tb.aresta_perto(p, 1.0).map(|a| tb.nome_da_aresta(doc, a, &t))) {
        Some(n) => n,
        None => tr("Aresta sem nome (espera o cálculo)").to_string(),
    }
}

/// Texto de uma face plana guardada como ponto (face aberta de chapa): o nome da face que passa por ele.
pub fn campo_face_no_ponto(doc: &Document, p: [f64; 3]) -> String {
    match tabela().and_then(|tb| tb.face_plana_em(p, 0.5).map(|f| tb.nome_da_face(doc, f, &t))) {
        Some(n) => n,
        None => tr("Face sem nome (espera o cálculo)").to_string(),
    }
}

/// Nome de uma âncora de campo: face e aresta pela hierarquia, ponta de membro por trecho.
pub fn nome_da_ancora(doc: &Document, a: &ChaveAncora) -> String {
    match a {
        ChaveAncora::Face(k) => face(doc, k),
        ChaveAncora::Aresta(k) => aresta(doc, k),
        outra => refs::nome_ancora(doc, outra, &t),
    }
}

/// Texto de um campo de ponto guardado no comando (`Feature::ancoras`): o nome da âncora quando ela
/// ainda vale para o ponto, senão "Ponto sem nome" (o ponto vai na dica).
pub fn campo_ancora(doc: &Document, f: &Feature, campo: &str, p: [f64; 3]) -> String {
    let _ = p;
    match f.ancora(campo, p) {
        Some(a) => nome_da_ancora(doc, a),
        None => tr("Ponto sem nome (espera o cálculo)").to_string(),
    }
}

/// Dica de um campo de face ou aresta: a explicação do nome e, só aqui, o ponto de referência.
pub fn dica_em(explicacao: &str, p: [f64; 3]) -> String {
    if p == [0.; 3] {
        explicacao.to_string()
    } else {
        format!("{explicacao}
{}: {}", tr("Ponto de referência"), ponto(p))
    }
}

/// [`dica_face`] com o ponto de referência.
pub fn dica_face_em(p: [f64; 3]) -> String {
    dica_em(dica_face(), p)
}

/// [`dica_aresta`] com o ponto de referência.
pub fn dica_aresta_em(p: [f64; 3]) -> String {
    dica_em(dica_aresta(), p)
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
    nome_da_ancora(doc, a)
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
        let sem = campo_face(&d, None, [1., 2., 3.]);
        assert!(sem.contains("sem nome") && !sem.contains('1'), "sem coordenada no nome: {sem}");
        crate::i18n::set_english(true);
        assert_eq!(campo_face(&d, Some(&chave(sk, 0)), [0.; 3]), format!("{nome} · start cap"));
        crate::i18n::set_english(false);
    }
}
