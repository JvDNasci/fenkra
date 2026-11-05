// SPDX-License-Identifier: LGPL-2.1-or-later
//! Diagnóstico guiado e pais/filhos (B69): lógica sem interface.
//!
//! Um comando falha por dois motivos: o próprio erro, ou porque algo de que ele depende falhou
//! antes. Para a pessoa corrigir de cima para baixo, o painel mostra a **causa** (o primeiro pai
//! que falhou) e deixa as consequências abaixo dela. "Pai" é tudo o que o comando referencia
//! (sketch, plano, ponto, outro comando); "filho" é o inverso.

use fenkra_core::doc::{Document, Feature, FeatureKind, Id};
use fenkra_solidos::Status;

#[derive(Clone, Debug, PartialEq)]
pub struct Falha {
    pub id: Id,
    pub mensagem: String,
    /// primeiro pai (na ordem da linha de tempo) que também falhou; `None` = esta é a causa
    pub causa: Option<Id>,
}

/// Pais diretos: o que o comando referencia e existe no documento.
pub fn pais(doc: &Document, id: Id) -> Vec<Id> {
    if let Some(f) = doc.feature(id) {
        return Document::refs_of(&f.kind).into_iter().filter(|x| *x != id && (doc.feature(*x).is_some() || doc.reference(*x).is_some())).collect();
    }
    if let Some(r) = doc.reference(id) {
        return r.kind.deps().into_iter().filter(|x| *x != id).collect();
    }
    vec![]
}

/// Filhos diretos: o que referencia este objeto.
pub fn filhos(doc: &Document, id: Id) -> Vec<Id> {
    let mut v: Vec<Id> = doc.references.iter().filter(|r| r.kind.deps().contains(&id)).map(|r| r.id).collect();
    v.extend(doc.features.iter().filter(|f| f.id != id && Document::refs_of(&f.kind).contains(&id)).map(|f| f.id));
    v
}

/// Todos os ancestrais (pais, pais dos pais...), sem repetição.
pub fn ancestrais(doc: &Document, id: Id) -> Vec<Id> {
    let mut out: Vec<Id> = vec![];
    let mut fila = pais(doc, id);
    while let Some(p) = fila.pop() {
        if !out.contains(&p) {
            out.push(p);
            fila.extend(pais(doc, p));
        }
    }
    out
}

/// Comandos com erro, de cima para baixo na linha de tempo, cada um apontando a sua causa.
pub fn falhas<'a>(doc: &Document, status: impl Fn(Id) -> Option<&'a Status>) -> Vec<Falha> {
    let erro = |id: Id| match status(id) {
        Some(Status::Error(m)) => Some(m.clone()),
        _ => None,
    };
    let mut v = vec![];
    for f in &doc.features {
        let Some(mensagem) = erro(f.id) else { continue };
        let causa = ancestrais(doc, f.id)
            .into_iter()
            .filter(|a| erro(*a).is_some())
            .min_by_key(|a| doc.feature_index(*a).map_or(0, |i| i + 1));
        v.push(Falha { id: f.id, mensagem, causa });
    }
    v
}

/// Consulta do filtro da árvore: texto livre mais `:erro`, `:suprimido`, `:oculto`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filtro {
    pub texto: Vec<String>,
    pub erro: bool,
    pub suprimido: bool,
    pub oculto: bool,
}

impl Filtro {
    pub fn analisar(consulta: &str) -> Filtro {
        let mut f = Filtro::default();
        for t in consulta.split_whitespace() {
            match t.to_lowercase().as_str() {
                ":erro" => f.erro = true,
                ":suprimido" => f.suprimido = true,
                ":oculto" => f.oculto = true,
                x => f.texto.push(x.to_string()),
            }
        }
        f
    }

    pub fn vazio(&self) -> bool {
        *self == Filtro::default()
    }

    /// O comando passa em todos os critérios (E lógico).
    pub fn passa(&self, f: &Feature, status: Option<&Status>) -> bool {
        let nome = f.name.to_lowercase();
        self.texto.iter().all(|t| nome.contains(t.as_str()))
            && (!self.erro || matches!(status, Some(Status::Error(_))))
            && (!self.suprimido || f.suppressed)
            && (!self.oculto || f.hidden)
    }
}

/// Nome curto do tipo, para a lista de pais e filhos.
pub fn tipo_de(doc: &Document, id: Id) -> &'static str {
    match doc.feature(id).map(|f| &f.kind) {
        Some(FeatureKind::Sketch(_)) => "Sketch",
        Some(_) => "Comando",
        None => "Geometria",
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashMap;

    fn extrude_de(origem: Id) -> fenkra_core::doc::Extrude {
        use fenkra_core::doc::{BoolOp, Extent, Extrude};
        Extrude { sketch: origem, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, thin_side: 0, dir: None }
    }

    /// sketch -> extrusão A -> extrusão B (B referencia A)
    fn cadeia() -> (Document, Id, Id, Id) {
        let mut d = Document::new("t");
        let sk = d.add_feature(FeatureKind::Sketch(Default::default()));
        let a = d.add_feature(FeatureKind::Extrude(extrude_de(sk)));
        let b = d.add_feature(FeatureKind::Extrude(extrude_de(a)));
        (d, sk, a, b)
    }

    #[test]
    fn pais_e_filhos_diretos() {
        let (d, sk, a, b) = cadeia();
        assert_eq!(pais(&d, a), vec![sk]);
        assert_eq!(filhos(&d, a), vec![b]);
        assert_eq!(ancestrais(&d, b).len(), 2);
    }

    #[test]
    fn falha_aponta_o_primeiro_pai_que_falhou() {
        let (d, sk, a, b) = cadeia();
        let mut st: HashMap<Id, Status> = HashMap::new();
        st.insert(sk, Status::Ok);
        st.insert(a, Status::Error("perfil aberto".into()));
        st.insert(b, Status::Error("depende de A".into()));
        let v = falhas(&d, |i| st.get(&i));
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].id, a);
        assert_eq!(v[0].causa, None, "A é a causa");
        assert_eq!(v[1].id, b);
        assert_eq!(v[1].causa, Some(a), "B é consequência de A");
    }

    #[test]
    fn filtro_combina_texto_e_marcas() {
        let (mut d, sk, a, _) = cadeia();
        d.feature_mut(a).unwrap().suppressed = true;
        let f = Filtro::analisar(":suprimido");
        assert!(f.passa(d.feature(a).unwrap(), None));
        assert!(!f.passa(d.feature(sk).unwrap(), None));
        let e = Filtro::analisar(":ERRO");
        assert!(e.passa(d.feature(a).unwrap(), Some(&Status::Error("x".into()))));
        assert!(!e.passa(d.feature(a).unwrap(), Some(&Status::Ok)));
        assert!(Filtro::analisar("   ").vazio());
        let nome = d.feature(a).unwrap().name.to_lowercase();
        assert!(Filtro::analisar(&nome).passa(d.feature(a).unwrap(), None));
    }
}
