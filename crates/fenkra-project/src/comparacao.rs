// SPDX-License-Identifier: LGPL-2.1-or-later
//! Comparação de versões por feature (B214): o diff do modelo.
//!
//! Dois documentos são comparados **pela árvore de features**, não pelos bytes. Cada feature tem um
//! `Id` estável dentro do documento; casar os dois lados pelo `Id` diz o que foi adicionado, removido,
//! movido na linha do tempo, suprimido ou reativado, renomeado ou alterado. Para as alteradas, o
//! conteúdo de cada lado é convertido em JSON e comparado campo a campo, o que produz frases como
//! `Hole.diameter: 8 → 10` sem uma linha de código por tipo de feature: um tipo novo de comando já
//! entra no diff sozinho.
//!
//! Além do modelo, a comparação traz os metadados das duas versões (quem, quando, comentário, estado
//! e revisão), que é o que a linha do histórico precisa para o menu "Comparar com a atual" e
//! "Comparar com outra".
//!
//! O mesmo comparador serve à conferência de migração: [`ComparacaoDocs::como_conferencia`] converte o
//! resultado em itens da lista de conferência, para comparar um documento importado ou reconstruído
//! com o alvo.

use crate::estados::{EstadoItem, ItemConferencia};
use crate::{PResult, Project, ProjectError};
use fenkra_core::Document;
use serde_json::Value;
use std::collections::HashMap;

/// Quantos campos alterados uma feature lista antes de resumir o resto.
const MAX_DETALHES: usize = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detalhe {
    /// caminho do campo, ex.: `Hole.diameter`
    pub campo: String,
    pub antes: String,
    pub depois: String,
}

impl Detalhe {
    pub fn frase(&self) -> String {
        format!("{}: {} → {}", self.campo, self.antes, self.depois)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mudanca {
    Adicionada,
    Removida,
    /// mesma feature, outra posição na linha do tempo (posições a partir de 1)
    Movida { de: usize, para: usize },
    Suprimida,
    Reativada,
    Renomeada { de: String, para: String },
    Alterada,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MudancaFeature {
    pub id: u64,
    /// nome no lado mais novo (no antigo, se foi removida)
    pub nome: String,
    pub tipo: String,
    pub mudanca: Mudanca,
    pub detalhes: Vec<Detalhe>,
    /// campos alterados que não coube listar
    pub omitidos: usize,
}

impl MudancaFeature {
    pub fn frase(&self) -> String {
        match &self.mudanca {
            Mudanca::Adicionada => format!("{} ({}) adicionada", self.nome, self.tipo),
            Mudanca::Removida => format!("{} ({}) removida", self.nome, self.tipo),
            Mudanca::Movida { de, para } => format!("{} movida da posição {de} para {para}", self.nome),
            Mudanca::Suprimida => format!("{} suprimida", self.nome),
            Mudanca::Reativada => format!("{} reativada", self.nome),
            Mudanca::Renomeada { de, para } => format!("{de} renomeada para {para}"),
            Mudanca::Alterada => match self.detalhes.first() {
                Some(d) => format!("{}: {}", self.nome, d.frase()),
                None => format!("{} alterada", self.nome),
            },
        }
    }
}

/// Diferença entre dois documentos.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComparacaoDocs {
    pub features: Vec<MudancaFeature>,
    pub parametros: Vec<Detalhe>,
    pub propriedades: Vec<Detalhe>,
}

impl ComparacaoDocs {
    pub fn e_igual(&self) -> bool {
        self.features.is_empty() && self.parametros.is_empty() && self.propriedades.is_empty()
    }

    pub fn total(&self) -> usize {
        self.features.len() + self.parametros.len() + self.propriedades.len()
    }

    /// Itens da lista de conferência para a migração: um por assunto (features, parâmetros,
    /// propriedades). Falha quando o resultado difere do alvo.
    pub fn como_conferencia(&self) -> Vec<ItemConferencia> {
        let mut v = vec![];
        let mut item = |chave: &'static str, rotulo: &'static str, frases: Vec<String>| {
            let (estado, detalhe) = if frases.is_empty() { (EstadoItem::Ok, String::new()) } else { (EstadoItem::Falha, frases.join("; ")) };
            v.push(ItemConferencia { chave, rotulo, estado, detalhe });
        };
        item("migracao_features", "Features iguais ao alvo", self.features.iter().map(MudancaFeature::frase).collect());
        item("migracao_parametros", "Parâmetros iguais ao alvo", self.parametros.iter().map(Detalhe::frase).collect());
        item("migracao_propriedades", "Propriedades iguais ao alvo", self.propriedades.iter().map(Detalhe::frase).collect());
        v
    }
}

fn texto(v: &Value) -> String {
    match v {
        Value::String(s) if s.is_empty() => "(vazio)".into(),
        Value::String(s) => s.clone(),
        Value::Null => "(nenhum)".into(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => if *b { "sim" } else { "não" }.into(),
        outro => outro.to_string(),
    }
}

/// Campos folha que diferem entre `a` e `b`, em ordem estável.
fn diff_json(caminho: &str, a: &Value, b: &Value, saida: &mut Vec<Detalhe>) {
    if a == b {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut chaves: Vec<&String> = x.keys().chain(y.keys()).collect();
            chaves.sort();
            chaves.dedup();
            for k in chaves {
                let sub = if caminho.is_empty() { k.clone() } else { format!("{caminho}.{k}") };
                diff_json(&sub, x.get(k).unwrap_or(&Value::Null), y.get(k).unwrap_or(&Value::Null), saida);
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                diff_json(&format!("{caminho}[{}]", i + 1), p, q, saida);
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            saida.push(Detalhe { campo: caminho.into(), antes: format!("{} itens", x.len()), depois: format!("{} itens", y.len()) });
        }
        _ => saida.push(Detalhe { campo: caminho.into(), antes: texto(a), depois: texto(b) }),
    }
}

/// Compara dois documentos pela árvore de features, parâmetros e propriedades.
pub fn comparar_documentos(antes: &Document, depois: &Document) -> ComparacaoDocs {
    let mut out = ComparacaoDocs::default();

    let pos_antes: HashMap<u64, usize> = antes.features.iter().enumerate().map(|(i, f)| (f.id.0, i)).collect();
    let pos_depois: HashMap<u64, usize> = depois.features.iter().enumerate().map(|(i, f)| (f.id.0, i)).collect();

    // removidas primeiro, na ordem em que estavam, depois o que existe no lado novo na ordem dele
    for f in antes.features.iter().filter(|f| !pos_depois.contains_key(&f.id.0)) {
        out.features.push(MudancaFeature { id: f.id.0, nome: f.name.clone(), tipo: f.kind.type_name().into(), mudanca: Mudanca::Removida, detalhes: vec![], omitidos: 0 });
    }
    // posição relativa só entre as features que existem nos dois lados: inserir uma no começo não
    // faz todas as outras parecerem movidas
    let comuns_antes: Vec<u64> = antes.features.iter().map(|f| f.id.0).filter(|i| pos_depois.contains_key(i)).collect();
    let comuns_depois: Vec<u64> = depois.features.iter().map(|f| f.id.0).filter(|i| pos_antes.contains_key(i)).collect();

    for (i, f) in depois.features.iter().enumerate() {
        let tipo = f.kind.type_name().to_string();
        let Some(&j) = pos_antes.get(&f.id.0) else {
            out.features.push(MudancaFeature { id: f.id.0, nome: f.name.clone(), tipo, mudanca: Mudanca::Adicionada, detalhes: vec![], omitidos: 0 });
            continue;
        };
        let velho = &antes.features[j];
        let mut mudancas: Vec<Mudanca> = vec![];
        let r_antes = comuns_antes.iter().position(|&x| x == f.id.0).unwrap_or(0);
        let r_depois = comuns_depois.iter().position(|&x| x == f.id.0).unwrap_or(0);
        if r_antes != r_depois {
            mudancas.push(Mudanca::Movida { de: j + 1, para: i + 1 });
        }
        if velho.name != f.name {
            mudancas.push(Mudanca::Renomeada { de: velho.name.clone(), para: f.name.clone() });
        }
        if !velho.suppressed && f.suppressed {
            mudancas.push(Mudanca::Suprimida);
        } else if velho.suppressed && !f.suppressed {
            mudancas.push(Mudanca::Reativada);
        }
        let mut detalhes = vec![];
        let (va, vb) = (serde_json::to_value(&velho.kind).unwrap_or(Value::Null), serde_json::to_value(&f.kind).unwrap_or(Value::Null));
        diff_json("", &va, &vb, &mut detalhes);
        if velho.suppress_if != f.suppress_if {
            detalhes.push(Detalhe { campo: "suprimir se".into(), antes: texto(&Value::String(velho.suppress_if.clone())), depois: texto(&Value::String(f.suppress_if.clone())) });
        }
        if !detalhes.is_empty() {
            mudancas.push(Mudanca::Alterada);
        }
        let n = mudancas.len();
        for (k, m) in mudancas.into_iter().enumerate() {
            // os detalhes de campo ficam só na entrada "Alterada"; as demais mudanças são de uma frase
            let (d, omitidos) = if m == Mudanca::Alterada {
                let omitidos = detalhes.len().saturating_sub(MAX_DETALHES);
                (detalhes.iter().take(MAX_DETALHES).cloned().collect(), omitidos)
            } else {
                (vec![], 0)
            };
            let _ = (k, n);
            out.features.push(MudancaFeature { id: f.id.0, nome: f.name.clone(), tipo: tipo.clone(), mudanca: m, detalhes: d, omitidos });
        }
    }

    // parâmetros por nome
    let por_nome = |d: &Document| -> HashMap<String, String> { d.params.iter().map(|p| (p.name.clone(), p.expr.clone())).collect() };
    let (pa, pb) = (por_nome(antes), por_nome(depois));
    let mut nomes: Vec<&String> = pa.keys().chain(pb.keys()).collect();
    nomes.sort();
    nomes.dedup();
    for n in nomes {
        match (pa.get(n), pb.get(n)) {
            (Some(x), Some(y)) if x != y => out.parametros.push(Detalhe { campo: n.clone(), antes: x.clone(), depois: y.clone() }),
            (Some(x), None) => out.parametros.push(Detalhe { campo: n.clone(), antes: x.clone(), depois: "(removido)".into() }),
            (None, Some(y)) => out.parametros.push(Detalhe { campo: n.clone(), antes: "(novo)".into(), depois: y.clone() }),
            _ => {}
        }
    }

    // propriedades (PN, descrição, material, tipo, personalizadas)
    let (xa, xb) = (serde_json::to_value(&antes.props).unwrap_or(Value::Null), serde_json::to_value(&depois.props).unwrap_or(Value::Null));
    diff_json("", &xa, &xb, &mut out.propriedades);
    out
}

/// Metadados de um dos lados da comparação.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetaVersao {
    /// "v3" ou "atual"
    pub rotulo: String,
    pub hash: String,
    pub tempo: u64,
    pub autor: String,
    pub comentario: String,
    pub nota: String,
    /// letra da revisão liberada com este conteúdo, se houver
    pub revisao: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparacaoVersoes {
    pub antes: MetaVersao,
    pub depois: MetaVersao,
    /// diferenças entre os metadados (quem, quando, comentário, revisão)
    pub metadados: Vec<Detalhe>,
    pub modelo: ComparacaoDocs,
    /// os dois lados têm o mesmo conteúdo (restauração, por exemplo)
    pub mesmo_conteudo: bool,
}

impl Project {
    fn meta_da_versao(&self, id: u64, numero: usize) -> PResult<MetaVersao> {
        let e = self.entry(id)?;
        let v = numero.checked_sub(1).and_then(|i| e.versions.get(i)).ok_or(ProjectError::NotFound)?;
        Ok(MetaVersao {
            rotulo: format!("v{numero}"),
            hash: v.hash.clone(),
            tempo: v.time,
            autor: v.author.clone(),
            comentario: v.comment.clone(),
            nota: v.note.clone(),
            revisao: e.revisions.iter().find(|r| r.hash == v.hash).map(|r| r.label.clone()),
        })
    }

    /// Compara duas versões (números a partir de 1) de um documento do projeto.
    pub fn comparar_versoes(&self, id: u64, a: usize, b: usize) -> PResult<ComparacaoVersoes> {
        let (ma, mb) = (self.meta_da_versao(id, a)?, self.meta_da_versao(id, b)?);
        let (da, db) = (self.load_version(&ma.hash)?, self.load_version(&mb.hash)?);
        let mut metadados = vec![];
        let mut campo = |nome: &str, x: String, y: String| {
            if x != y {
                metadados.push(Detalhe { campo: nome.into(), antes: x, depois: y });
            }
        };
        let vazio = |s: &str| if s.is_empty() { "(vazio)".to_string() } else { s.to_string() };
        campo("autor", vazio(&ma.autor), vazio(&mb.autor));
        campo("data", ma.tempo.to_string(), mb.tempo.to_string());
        campo("nota", vazio(&ma.nota), vazio(&mb.nota));
        campo("comentário", vazio(&ma.comentario), vazio(&mb.comentario));
        campo("revisão", vazio(ma.revisao.as_deref().unwrap_or("")), vazio(mb.revisao.as_deref().unwrap_or("")));
        let modelo = comparar_documentos(&da, &db);
        Ok(ComparacaoVersoes { mesmo_conteudo: ma.hash == mb.hash, antes: ma, depois: mb, metadados, modelo })
    }

    /// "Comparar com a atual": a versão `numero` contra a última gravada.
    pub fn comparar_com_atual(&self, id: u64, numero: usize) -> PResult<ComparacaoVersoes> {
        let ultima = self.entry(id)?.versions.len();
        self.comparar_versoes(id, numero, ultima)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{BoolOp, Document, FeatureKind, Id, OriginPlane, PlaneRef, PrimKind, Primitive, Unit};
    use std::path::PathBuf;

    fn tmp(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra-cmp-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    fn caixa(d: &mut Document, lado: &str) -> Id {
        let s = |x: &str| x.to_string();
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s(lado), b: s("20"), c: s("5"), d: s("0"), op: BoolOp::Add }))
    }

    #[test]
    fn documentos_iguais_nao_tem_diferenca() {
        let mut d = Document::new("Placa");
        caixa(&mut d, "40");
        assert!(comparar_documentos(&d, &d.clone()).e_igual());
    }

    #[test]
    fn campo_alterado_vira_frase_com_antes_e_depois() {
        let mut a = Document::new("Placa");
        let id = caixa(&mut a, "40");
        let mut b = a.clone();
        if let FeatureKind::Primitive(p) = &mut b.features.iter_mut().find(|f| f.id == id).unwrap().kind {
            p.a = "60".into();
        }
        let c = comparar_documentos(&a, &b);
        assert_eq!(c.features.len(), 1);
        let m = &c.features[0];
        assert_eq!(m.mudanca, Mudanca::Alterada);
        assert!(m.detalhes.iter().any(|d| d.campo.ends_with(".a") && d.antes == "40" && d.depois == "60"), "{:?}", m.detalhes);
        assert!(m.frase().contains("40 → 60"), "{}", m.frase());
    }

    #[test]
    fn adicionada_removida_suprimida_renomeada_e_movida() {
        let mut a = Document::new("Placa");
        let (x, y, z) = (caixa(&mut a, "10"), caixa(&mut a, "20"), caixa(&mut a, "30"));
        let mut b = a.clone();
        b.features.retain(|f| f.id != x); // removida
        let novo = caixa(&mut b, "50"); // adicionada
        b.features.iter_mut().find(|f| f.id == y).unwrap().suppressed = true; // suprimida
        b.features.iter_mut().find(|f| f.id == z).unwrap().name = "Base".into(); // renomeada
        let c = comparar_documentos(&a, &b);
        let tem = |id: Id, m: Mudanca| c.features.iter().any(|f| f.id == id.0 && f.mudanca == m);
        assert!(tem(x, Mudanca::Removida));
        assert!(tem(novo, Mudanca::Adicionada));
        assert!(tem(y, Mudanca::Suprimida));
        assert!(c.features.iter().any(|f| f.id == z.0 && matches!(&f.mudanca, Mudanca::Renomeada { para, .. } if para == "Base")));
        // inserir ou remover uma feature não faz as outras parecerem movidas
        assert!(!c.features.iter().any(|f| matches!(f.mudanca, Mudanca::Movida { .. })));

        // trocar a ordem de verdade é mudança de posição
        let mut d = a.clone();
        d.move_feature(z, 0).unwrap();
        let c = comparar_documentos(&a, &d);
        assert!(c.features.iter().any(|f| f.id == z.0 && matches!(f.mudanca, Mudanca::Movida { de: 3, para: 1 })), "{:?}", c.features);
        // reativar é o inverso de suprimir
        let c = comparar_documentos(&b, &a);
        assert!(c.features.iter().any(|f| f.id == y.0 && f.mudanca == Mudanca::Reativada));
    }

    #[test]
    fn parametros_e_propriedades_entram_no_diff() {
        let mut a = Document::new("Placa");
        a.add_param("L", "40", Unit::Comprimento).unwrap();
        a.add_param("H", "5", Unit::Comprimento).unwrap();
        let mut b = a.clone();
        b.params.iter_mut().find(|p| p.name == "L").unwrap().expr = "55".into();
        b.params.retain(|p| p.name != "H");
        b.add_param("N", "3", Unit::Numero).unwrap();
        b.props.description = "nova".into();
        let c = comparar_documentos(&a, &b);
        let frases: Vec<String> = c.parametros.iter().map(Detalhe::frase).collect();
        assert_eq!(frases, vec!["H: 5 → (removido)", "L: 40 → 55", "N: (novo) → 3"]);
        assert_eq!(c.propriedades.iter().map(Detalhe::frase).collect::<Vec<_>>(), vec!["description: (vazio) → nova"]);
    }

    #[test]
    fn versoes_do_projeto_trazem_metadados_e_diff_do_modelo() {
        let raiz = tmp("versoes");
        let mut p = Project::create(&raiz, "T").unwrap();
        p.autor = "ana".into();
        let mut d = Document::new("Placa");
        let id_f = caixa(&mut d, "40");
        let id = p.add_document(d.clone()).unwrap();
        d.props.pn = p.entry(id).unwrap().pn.clone();
        p.autor = "beto".into();
        if let FeatureKind::Primitive(pr) = &mut d.features.iter_mut().find(|f| f.id == id_f).unwrap().kind {
            pr.a = "60".into();
        }
        p.save_comentado(id, &d, "editado", "furo maior por folga").unwrap();
        p.release(id, "primeira").unwrap();

        let c = p.comparar_com_atual(id, 1).unwrap();
        assert_eq!((c.antes.rotulo.as_str(), c.depois.rotulo.as_str()), ("v1", "v2"));
        assert!(!c.mesmo_conteudo);
        assert!(c.metadados.iter().any(|m| m.campo == "autor" && m.depois == "beto"), "{:?}", c.metadados);
        assert!(c.metadados.iter().any(|m| m.campo == "comentário" && m.depois == "furo maior por folga"));
        assert!(c.metadados.iter().any(|m| m.campo == "revisão" && m.depois == "A"));
        assert!(c.modelo.features.iter().any(|f| f.detalhes.iter().any(|x| x.antes == "40" && x.depois == "60")));

        // restaurar a v1 cria a v3 com o mesmo conteúdo: sem diferença de modelo
        p.restaurar_versao(id, 1, "").unwrap();
        let c = p.comparar_versoes(id, 1, 3).unwrap();
        assert!(c.mesmo_conteudo && c.modelo.e_igual());
        assert!(matches!(p.comparar_versoes(id, 1, 9), Err(ProjectError::NotFound)));
        assert!(matches!(p.comparar_versoes(id, 0, 1), Err(ProjectError::NotFound)));
        let _ = std::fs::remove_dir_all(raiz);
    }

    #[test]
    fn conferencia_de_migracao_reprova_quando_o_resultado_difere_do_alvo() {
        let mut alvo = Document::new("Placa");
        let id = caixa(&mut alvo, "40");
        let itens = comparar_documentos(&alvo, &alvo.clone()).como_conferencia();
        assert!(itens.iter().all(|i| i.estado == EstadoItem::Ok));
        let mut importado = alvo.clone();
        if let FeatureKind::Primitive(pr) = &mut importado.features.iter_mut().find(|f| f.id == id).unwrap().kind {
            pr.a = "41".into();
        }
        let itens = comparar_documentos(&importado, &alvo).como_conferencia();
        let f = itens.iter().find(|i| i.chave == "migracao_features").unwrap();
        assert_eq!(f.estado, EstadoItem::Falha);
        assert!(f.detalhe.contains("41 → 40"), "{}", f.detalhe);
    }
}
