// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estado do documento para a barra (B279), e comparação com a revisão liberada.
//!
//! A barra responde a uma pergunta: posso confiar no que estou vendo? A resposta vem de fatos soltos
//! (revisão, edição, leitura, conflito) e só um estado aparece, o mais grave. A fase colaborativa vai
//! trazer os mesmos fatos de outra fonte, sem mudar as frases: conflito por outra pessoa ou por outro
//! programa é o mesmo aviso para quem olha a barra.

use crate::DocEntry;
use fenkra_core::{Document, Id};

/// O que se sabe do documento aberto agora.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fatos {
    /// rótulo da última revisão liberada
    pub revisao: Option<String>,
    /// a versão gravada já não é a da revisão
    pub sujo_desde_revisao: bool,
    /// há edição ainda não gravada
    pub modificado: bool,
    /// revisão congelada, formato mais novo ou qualquer outra trava de escrita
    pub somente_leitura: bool,
    /// alguém gravou o arquivo por fora desde que ele foi aberto
    pub conflito: bool,
}

impl Fatos {
    /// Fatos de um documento do projeto.
    pub fn da_entrada(e: &DocEntry) -> Fatos {
        Fatos { revisao: e.current_revision().map(|r| r.label.clone()), sujo_desde_revisao: e.dirty_since_release(), ..Default::default() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EstadoDoc {
    SemRevisao,
    Liberada(String),
    Editado(String),
    SomenteLeitura,
    EmConflito,
}

impl EstadoDoc {
    /// Conflito vem primeiro porque salvar por cima perde o trabalho de outra pessoa. Leitura vem
    /// depois, e só então a revisão.
    pub fn de(f: &Fatos) -> EstadoDoc {
        if f.conflito {
            return EstadoDoc::EmConflito;
        }
        if f.somente_leitura {
            return EstadoDoc::SomenteLeitura;
        }
        match &f.revisao {
            None => EstadoDoc::SemRevisao,
            Some(r) if f.sujo_desde_revisao || f.modificado => EstadoDoc::Editado(r.clone()),
            Some(r) => EstadoDoc::Liberada(r.clone()),
        }
    }

    /// Frase com a lacuna `{0}`, que a interface traduz antes de preencher.
    pub fn modelo(&self) -> &'static str {
        match self {
            EstadoDoc::SemRevisao => "Sem revisão",
            EstadoDoc::Liberada(_) => "Rev {0} liberada",
            EstadoDoc::Editado(_) => "Editado desde a Rev {0}",
            EstadoDoc::SomenteLeitura => "Somente leitura",
            EstadoDoc::EmConflito => "Em conflito",
        }
    }

    pub fn revisao(&self) -> &str {
        match self {
            EstadoDoc::Liberada(r) | EstadoDoc::Editado(r) => r,
            _ => "",
        }
    }

    /// Primeiro estágio do tooltip: o que o estado quer dizer.
    pub fn dica(&self) -> &'static str {
        match self {
            EstadoDoc::SemRevisao => "Este documento ainda não tem revisão liberada.",
            EstadoDoc::Liberada(_) => "O que está aberto é igual à revisão liberada.",
            EstadoDoc::Editado(_) => "O que está aberto já não é igual à revisão liberada.",
            EstadoDoc::SomenteLeitura => "Este documento abre só para consulta: nada aqui será gravado.",
            EstadoDoc::EmConflito => "O arquivo foi gravado por fora desde que você o abriu.",
        }
    }

    /// Segundo estágio: o que fazer.
    pub fn passos(&self) -> &'static str {
        match self {
            EstadoDoc::SemRevisao => "Salve uma versão e use Liberar revisão no painel do projeto.",
            EstadoDoc::Liberada(_) => "Para mudar, edite e salve uma versão. A revisão continua guardada.",
            EstadoDoc::Editado(_) => "Use Comparar para ver o que mudou e Liberar revisão para fechar uma nova.",
            EstadoDoc::SomenteLeitura => "Para mudar, abra a versão atual ou salve uma cópia.",
            EstadoDoc::EmConflito => "Salve para escolher entre manter o seu, recarregar ou gravar com outro nome.",
        }
    }

    pub fn icone(&self) -> &'static str {
        match self {
            EstadoDoc::SemRevisao => "estado_sem_revisao",
            EstadoDoc::Liberada(_) => "estado_liberada",
            EstadoDoc::Editado(_) => "estado_editado",
            EstadoDoc::SomenteLeitura => "estado_leitura",
            EstadoDoc::EmConflito => "estado_conflito",
        }
    }
}

/// Frases para a tradução ser conferida.
pub const FRASES: &[&str] = &[
    "Sem revisão",
    "Rev {0} liberada",
    "Editado desde a Rev {0}",
    "Somente leitura",
    "Em conflito",
    "Este documento ainda não tem revisão liberada.",
    "O que está aberto é igual à revisão liberada.",
    "O que está aberto já não é igual à revisão liberada.",
    "Este documento abre só para consulta: nada aqui será gravado.",
    "O arquivo foi gravado por fora desde que você o abriu.",
    "Salve uma versão e use Liberar revisão no painel do projeto.",
    "Para mudar, edite e salve uma versão. A revisão continua guardada.",
    "Use Comparar para ver o que mudou e Liberar revisão para fechar uma nova.",
    "Para mudar, abra a versão atual ou salve uma cópia.",
    "Salve para escolher entre manter o seu, recarregar ou gravar com outro nome.",
];

/// O que mudou entre dois estados de um documento, pelos nomes que a pessoa vê.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Diferencas {
    pub novos: Vec<String>,
    pub removidos: Vec<String>,
    pub alterados: Vec<String>,
    /// nome, valor antigo e valor novo
    pub parametros: Vec<(String, String, String)>,
}

impl Diferencas {
    pub fn vazia(&self) -> bool {
        self.novos.is_empty() && self.removidos.is_empty() && self.alterados.is_empty() && self.parametros.is_empty()
    }
}

/// Compara os comandos pelo id, que sobrevive a renomear, e os parâmetros pelo nome.
pub fn comparar(antes: &Document, agora: &Document) -> Diferencas {
    let achar = |d: &Document, id: Id| d.features.iter().find(|f| f.id == id).cloned();
    let mut r = Diferencas::default();
    for f in &agora.features {
        match achar(antes, f.id) {
            None => r.novos.push(f.name.clone()),
            Some(a) if a != *f => r.alterados.push(f.name.clone()),
            Some(_) => {}
        }
    }
    r.removidos = antes.features.iter().filter(|f| achar(agora, f.id).is_none()).map(|f| f.name.clone()).collect();
    for p in &agora.params {
        let velho = antes.params.iter().find(|q| q.name == p.name).map(|q| q.expr.clone()).unwrap_or_default();
        if velho != p.expr {
            r.parametros.push((p.name.clone(), velho, p.expr.clone()));
        }
    }
    r
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::Project;
    use fenkra_core::Unit;

    fn fatos(revisao: Option<&str>) -> Fatos {
        Fatos { revisao: revisao.map(str::to_string), ..Default::default() }
    }

    #[test]
    fn um_estado_para_cada_combinacao_com_o_mais_grave_na_frente() {
        assert_eq!(EstadoDoc::de(&fatos(None)), EstadoDoc::SemRevisao);
        assert_eq!(EstadoDoc::de(&fatos(Some("B"))), EstadoDoc::Liberada("B".into()));
        assert_eq!(EstadoDoc::de(&Fatos { modificado: true, ..fatos(Some("B")) }), EstadoDoc::Editado("B".into()));
        assert_eq!(EstadoDoc::de(&Fatos { sujo_desde_revisao: true, ..fatos(Some("B")) }), EstadoDoc::Editado("B".into()));
        assert_eq!(EstadoDoc::de(&Fatos { somente_leitura: true, modificado: true, ..fatos(Some("B")) }), EstadoDoc::SomenteLeitura);
        assert_eq!(EstadoDoc::de(&Fatos { conflito: true, somente_leitura: true, ..fatos(Some("B")) }), EstadoDoc::EmConflito);
    }

    #[test]
    fn todas_as_frases_estao_na_lista_da_traducao() {
        for e in [EstadoDoc::SemRevisao, EstadoDoc::Liberada("A".into()), EstadoDoc::Editado("A".into()), EstadoDoc::SomenteLeitura, EstadoDoc::EmConflito] {
            for t in [e.modelo(), e.dica(), e.passos()] {
                assert!(FRASES.contains(&t), "{t}");
            }
        }
    }

    fn projeto(nome: &str) -> Project {
        let dir = std::env::temp_dir().join(format!("fenkra estado {} {nome}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Project::create(&dir, "Teste").unwrap()
    }

    #[test]
    fn estado_acompanha_o_projeto_de_verdade() {
        let mut p = projeto("ciclo");
        let id = p.add_document(Document::new("Peça")).unwrap();
        assert_eq!(EstadoDoc::de(&Fatos::da_entrada(p.entry(id).unwrap())), EstadoDoc::SemRevisao);
        let rev = p.release(id, "primeira").unwrap();
        assert_eq!(EstadoDoc::de(&Fatos::da_entrada(p.entry(id).unwrap())), EstadoDoc::Liberada(rev.clone()));
        let mut d = p.load(id).unwrap();
        d.add_param("L", "10", Unit::Comprimento).unwrap();
        p.save(id, &d, "editei").unwrap();
        assert_eq!(EstadoDoc::de(&Fatos::da_entrada(p.entry(id).unwrap())), EstadoDoc::Editado(rev));
    }

    #[test]
    fn comparar_diz_o_que_entrou_saiu_mudou_e_ignora_o_resto() {
        let mut a = Document::new("Peça");
        a.add_param("L", "10", Unit::Comprimento).unwrap();
        let velho = a.add_offset_plane(fenkra_core::PlaneRef::default(), "1");
        let _ = velho;
        let f = a.add_feature(fenkra_core::FeatureKind::Sketch(Default::default()));
        let g = a.add_feature(fenkra_core::FeatureKind::Sketch(Default::default()));
        assert!(comparar(&a, &a.clone()).vazia());

        let mut b = a.clone();
        b.params[0].expr = "20".into();
        b.feature_mut(f).unwrap().suppressed = true;
        b.features.retain(|x| x.id != g);
        let nome_g = a.name_of(g);
        let n = b.add_feature(fenkra_core::FeatureKind::Sketch(Default::default()));
        let r = comparar(&a, &b);
        assert_eq!(r.novos, vec![b.name_of(n)]);
        assert_eq!(r.removidos, vec![nome_g]);
        assert_eq!(r.alterados, vec![a.name_of(f)]);
        assert_eq!(r.parametros, vec![("L".to_string(), "10".to_string(), "20".to_string())]);
    }
}
