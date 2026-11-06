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

/// Família do objeto: só troca de lugar com outro da mesma. Um sketch serve onde outro servia, e um
/// comando, onde outro do mesmo tipo servia.
fn mesma_familia(doc: &Document, a: Id, b: Id) -> bool {
    match (doc.feature(a), doc.feature(b), doc.reference(a), doc.reference(b)) {
        (Some(x), Some(y), ..) => std::mem::discriminant(&x.kind) == std::mem::discriminant(&y.kind),
        (.., Some(x), Some(y)) => x.kind.class() == y.kind.class(),
        _ => false,
    }
}

/// O que pode entrar no lugar de `id` para quem está na posição `antes_de`: quem depende dele fica de
/// fora, e um comando depois de quem vai usá-lo quebraria a ordem.
pub fn candidatos(doc: &Document, id: Id, antes_de: Option<usize>) -> Vec<(Id, String)> {
    let cedo = |x: Id| antes_de.is_none_or(|n| doc.feature_index(x).is_none_or(|i| i < n));
    let serve = |x: Id, nome: &str| (x != id && mesma_familia(doc, id, x) && cedo(x) && !ancestrais(doc, x).contains(&id)).then(|| (x, nome.to_string()));
    let refs = doc.references.iter().filter_map(|r| serve(r.id, &r.name));
    refs.chain(doc.features.iter().filter_map(|f| serve(f.id, &f.name))).collect()
}

/// Uma troca oferecida no painel.
#[derive(Clone, Debug, PartialEq)]
pub struct Troca {
    /// comando que muda; `None` muda quem usava `de`, todos eles
    pub alvo: Option<Id>,
    pub de: Id,
    pub para: Id,
    pub rotulo: String,
}

/// O que a falha de `id` aceita: ele mesmo sai e outro entra para quem o usava, ou, mais comum, um
/// pai dele é trocado só aqui: a extrusão que falhou por causa do sketch passa a usar outro sketch.
pub fn trocas(doc: &Document, id: Id) -> Vec<Troca> {
    let primeiro_uso = filhos(doc, id).into_iter().filter_map(|f| doc.feature_index(f)).min();
    let nome = |x: Id| doc.name_of(x);
    let mut v: Vec<Troca> = vec![];
    if !filhos(doc, id).is_empty() {
        v.extend(candidatos(doc, id, primeiro_uso).into_iter().map(|(para, n)| Troca { alvo: None, de: id, para, rotulo: format!("{} → {n}", nome(id)) }));
    }
    for p in pais(doc, id) {
        let lista = candidatos(doc, p, doc.feature_index(id)).into_iter().filter(|(x, _)| *x != id && !ancestrais(doc, *x).contains(&id));
        v.extend(lista.map(|(para, n)| Troca { alvo: Some(id), de: p, para, rotulo: format!("{} → {n}", nome(p)) }));
    }
    v
}

/// Faz a troca: dentro do alvo, ou em todos que usavam `de`.
pub fn aplicar_troca(doc: &mut Document, t: &Troca) {
    let (de, para) = (t.de, t.para);
    let mut troca = |x: &mut Id| {
        if *x == de {
            *x = para;
        }
    };
    for f in doc.features.iter_mut().filter(|f| t.alvo.is_none_or(|a| a == f.id)) {
        f.kind.walk(&mut troca, &mut |_| {});
    }
    for r in doc.references.iter_mut().filter(|r| r.id != para && t.alvo.is_none_or(|a| a == r.id)) {
        r.kind.walk(&mut troca, &mut |_| {});
    }
}

/// O elemento de geometria que Isolar corta: o próprio, ou o primeiro pai que for geometria.
pub fn alvo_de_isolar(doc: &Document, id: Id) -> Option<Id> {
    if doc.reference(id).is_some() {
        return Some(id);
    }
    pais(doc, id).into_iter().find(|p| doc.reference(*p).is_some())
}

/// Comandos com erro, de cima para baixo na linha de tempo, cada um apontando a sua causa.
pub fn falhas<'a>(doc: &Document, status: impl Fn(Id) -> Option<&'a Status>) -> Vec<Falha> {
    let erro = |id: Id| match status(id) {
        Some(Status::Error(m)) => Some(m.clone()),
        _ => None,
    };
    let mut v = vec![];
    // a geometria não tem lugar na linha de tempo e vem antes de quem a usa
    for id in doc.references.iter().map(|r| r.id).chain(doc.features.iter().map(|f| f.id)) {
        let Some(mensagem) = erro(id) else { continue };
        let causa = ancestrais(doc, id)
            .into_iter()
            .filter(|a| erro(*a).is_some())
            .min_by_key(|a| doc.feature_index(*a).map_or(0, |i| i + 1));
        v.push(Falha { id, mensagem, causa });
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
        Extrude { sketch: origem, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, thin_side: 0, dir: None, ends: Default::default() }
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
    fn extrusao_que_falhou_troca_o_sketch_so_nela() {
        let (mut d, sk, a, b) = cadeia();
        let cedo = d.add_feature(FeatureKind::Sketch(Default::default()));
        d.features.rotate_right(1);
        let tarde = d.add_feature(FeatureKind::Sketch(Default::default()));
        let t = trocas(&d, a);
        // o sketch que vem depois da extrusão não serve: ela o usaria antes de ele existir
        assert!(t.iter().all(|x| x.para != tarde), "{t:?}");
        let troca = t.iter().find(|x| x.para == cedo && x.alvo == Some(a) && x.de == sk).expect("troca do sketch").clone();
        aplicar_troca(&mut d, &troca);
        assert!(pais(&d, a).contains(&cedo) && !pais(&d, a).contains(&sk));
        assert_eq!(pais(&d, b), vec![a], "quem não falhou fica como estava");
    }

    #[test]
    fn comando_nao_entra_no_lugar_de_quem_depende_dele() {
        let (d, _, a, b) = cadeia();
        assert!(candidatos(&d, a, None).iter().all(|x| x.0 != b));
    }

    #[test]
    fn isolar_sobe_do_comando_para_a_geometria_que_ele_usa() {
        use fenkra_core::doc::{OriginPlane, PlaneRef, Sketch};
        let mut d = Document::new("t");
        let plano = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "5");
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(plano), data: Default::default() }));
        assert_eq!(alvo_de_isolar(&d, plano), Some(plano));
        assert_eq!(alvo_de_isolar(&d, sk), Some(plano));
        let solto = d.add_feature(FeatureKind::Sketch(Default::default()));
        assert_eq!(alvo_de_isolar(&d, solto), None);
    }

    #[test]
    fn geometria_com_erro_entra_no_painel_antes_dos_comandos() {
        use fenkra_core::doc::{OriginPlane, PlaneRef};
        let (mut d, _, a, _) = cadeia();
        let plano = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "5");
        let mut st: HashMap<Id, Status> = HashMap::new();
        st.insert(plano, Status::Error("sem pai".into()));
        st.insert(a, Status::Error("perfil aberto".into()));
        let v = falhas(&d, |i| st.get(&i));
        assert_eq!(v.iter().map(|f| f.id).collect::<Vec<_>>(), vec![plano, a]);
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
