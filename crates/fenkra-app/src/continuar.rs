// SPDX-License-Identifier: LGPL-2.1-or-later
//! "Aplicar e continuar" (B20, contrato de comando): conclui o comando aberto e abre outro igual,
//! com os mesmos valores e a escolha na viewport vazia, para a próxima aresta, face ou furo.
//!
//! Só vale para comandos que vivem da escolha na viewport (filete, chanfro, furo por pontos, casca,
//! saída, espessura, edição direta); nos outros repetir daria uma cópia do mesmo resultado, e o
//! botão não aparece.

use crate::app::App;
use fenkra_core::{EdgeSel, FeatureKind, Id};

/// O comando como ele recomeça: mesmos valores, escolha vazia. `None` = não se repete.
pub fn sem_escolha(k: &FeatureKind) -> Option<FeatureKind> {
    let mut k = k.clone();
    match &mut k {
        FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => {
            if !matches!(o.edges, EdgeSel::Edges(_)) {
                return None;
            }
            o.edges = EdgeSel::Edges(vec![]);
            o.keys.clear();
            o.extra.flips.clear();
            o.extra.regras.clear();
            o.extra.face_keys.clear();
        }
        // furo por pontos 3D: o próximo clique põe o centro novo; furo por sketch não se repete
        FeatureKind::Hole(h) if h.sketch == Id(0) => h.points.clear(),
        FeatureKind::Shell(s) => {
            s.faces.clear();
            s.face_keys.clear();
            s.regras.clear();
            s.espessuras.clear();
        }
        FeatureKind::Draft(d) => {
            d.faces.clear();
            d.face_keys.clear();
            d.regras.clear();
        }
        FeatureKind::Thickness(t) => {
            t.faces.clear();
            t.face_keys.clear();
        }
        FeatureKind::DirectEdit(e) => {
            e.faces.clear();
            e.face_keys.clear();
            e.regras.clear();
        }
        _ => return None,
    }
    Some(k)
}

/// O comando aberto pode ser repetido? (o botão só aparece em comando novo)
pub fn pode(app: &App) -> bool {
    app.session.as_ref().filter(|s| s.is_new).and_then(|s| app.open.doc.feature(s.id)).is_some_and(|f| sem_escolha(&f.kind).is_some())
}

/// Conclui o comando aberto e abre o mesmo comando de novo, com os mesmos valores, no mesmo corpo.
pub fn aplicar_e_continuar(app: &mut App) {
    let Some(f) = app.session.as_ref().filter(|s| s.is_new).and_then(|s| app.open.doc.feature(s.id)).cloned() else { return };
    let Some(novo) = sem_escolha(&f.kind) else { return };
    app.conclude_session();
    let snapshot = app.open.doc.clone();
    let id = app.open.doc.add_feature(novo);
    if let Some(n) = app.open.doc.feature_mut(id) {
        n.target = f.target;
        n.appearance = f.appearance;
    }
    app.picks.clear();
    app.start_session(id, true, Some(snapshot));
    app.request_recompute();
}

/// Textos novos (inglês), somados à tabela de `i18n.rs`.
pub static TEXTOS: &[(&str, &str)] = &[
    ("↻ Aplicar e continuar", "↻ Apply and continue"),
    ("Conclui este comando e abre outro igual, com os mesmos valores, para a próxima escolha.  Ctrl+Enter", "Finishes this command and opens another one with the same values, for the next pick.  Ctrl+Enter"),
];

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_core::{DirectEdit, EdgeOp, OpDireta};

    #[test]
    fn filete_recomeca_com_o_raio_e_sem_arestas() {
        let mut o = EdgeOp { value: "4".into(), edges: EdgeSel::Edges(vec![[1., 2., 3.]]), mode: Default::default(), prop: Default::default(), keys: vec![None], extra: Default::default() };
        o.extra.regras.push(fenkra_core::RegraAresta { semente: 0, regra: fenkra_core::RegraSelecao::Cadeia });
        let Some(FeatureKind::Fillet(n)) = sem_escolha(&FeatureKind::Fillet(o)) else { panic!() };
        assert_eq!(n.value, "4");
        assert_eq!(n.edges, EdgeSel::Edges(vec![]));
        assert!(n.keys.is_empty() && n.extra.regras.is_empty());
    }

    #[test]
    fn todas_as_arestas_e_furo_por_sketch_nao_repetem() {
        let o = EdgeOp { value: "4".into(), edges: EdgeSel::AllOfBody, mode: Default::default(), prop: Default::default(), keys: vec![], extra: Default::default() };
        assert!(sem_escolha(&FeatureKind::Fillet(o)).is_none());
        let mut e = DirectEdit::novo(OpDireta::Deslocar);
        e.faces.push([0.; 3]);
        e.value = "2".into();
        let Some(FeatureKind::DirectEdit(n)) = sem_escolha(&FeatureKind::DirectEdit(e)) else { panic!() };
        assert!(n.faces.is_empty() && n.value == "2" && n.op == OpDireta::Deslocar);
    }
}
