// SPDX-License-Identifier: LGPL-2.1-or-later
//! Peças de exemplo das aulas que a tela inicial não mostra (B297): a forma livre e as superfícies
//! (peças comuns), um conjunto pronto para a montagem e uma folha pronta para o desenho.
//!
//! As peças comuns abrem pelo mesmo caminho dos exemplos da tela inicial. A montagem e o desenho
//! precisam de arquivos ao lado (as peças que eles citam), então são gravados numa pasta temporária
//! e abertos pelo caminho de sempre; a pessoa grava onde quiser com "Salvar como".

use crate::settings::Bench;
use fenkra_core::subd::Gaiola;
use fenkra_core::subd_ops::PrimitivaForma;
use fenkra_core::*;
use std::path::PathBuf;

/// Nome do conjunto de exemplo da montagem.
pub const CONJUNTO: &str = "Conjunto de exemplo";
/// Nome da folha de exemplo do desenho.
pub const FOLHA: &str = "Folha da placa";

/// Peças de exemplo só do curso: nome, bancada em que abrem e o documento.
pub fn pecas() -> Vec<(&'static str, Bench, Document)> {
    vec![("Forma suave", Bench::FormaLivre, forma_suave()), ("Superfícies de exemplo", Bench::Superficies, superficies())]
}

/// Exemplos compostos (montagem e desenho): nome e bancada.
pub const COMPOSTOS: [(&str, Bench); 2] = [(CONJUNTO, Bench::Montagem), (FOLHA, Bench::Desenho)];

/// Todo nome de exemplo que o curso sabe abrir.
pub fn existe(nome: &str) -> bool {
    crate::examples::all().iter().any(|e| e.name == nome) || pecas().iter().any(|p| p.0 == nome) || COMPOSTOS.iter().any(|c| c.0 == nome)
}

/// Peça comum (da tela inicial ou só do curso) com esse nome.
pub fn peca(nome: &str) -> Option<(Document, Bench)> {
    if let Some(e) = crate::examples::all().into_iter().find(|e| e.name == nome) {
        return Some((e.doc, e.bench));
    }
    pecas().into_iter().find(|p| p.0 == nome).map(|(_, b, d)| (d, b))
}

/// Caixa de forma com a superfície lisa, sem vincos: sobra o trabalho de moldar, vincar e converter.
fn forma_suave() -> Document {
    let mut d = Document::new("Forma suave");
    let gaiola = Gaiola::primitiva(PrimitivaForma::Caixa, [0., 0., 15.], [60., 40., 30.]);
    d.add_feature(FeatureKind::Forma(Forma { gaiola, niveis: 3, congelada: false, conversao: None }));
    d
}

/// Duas superfícies que dividem uma aresta (prontas para juntar) e uma linha guia para uma terceira.
fn superficies() -> Document {
    let mut d = Document::new("Superfícies de exemplo");
    let s = |x: &str| x.to_string();
    let la = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 40., 0.]), start: s("0"), end: s("0") });
    let lb = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([40., 0., 0.]), b: PointRef::Fixed([40., 40., 0.]), start: s("0"), end: s("0") });
    d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(la), dir: AxisRef::Origin(OriginAxis::X), length: s("40"), length2: s("0"), until: None, until2: None });
    d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(lb), dir: AxisRef::Origin(OriginAxis::Z), length: s("30"), length2: s("0"), until: None, until2: None });
    d
}

fn pasta() -> PathBuf {
    std::env::temp_dir().join("fenkra_exemplos")
}

/// Grava o exemplo composto numa pasta temporária e devolve o arquivo a abrir.
pub fn gravar_composto(nome: &str) -> Option<PathBuf> {
    let dir = pasta().join(nome.replace(' ', "_"));
    std::fs::create_dir_all(&dir).ok()?;
    let placa = peca("Placa paramétrica")?.0;
    std::fs::write(dir.join("placa.fkp"), placa.to_json()).ok()?;
    if nome == CONJUNTO {
        std::fs::write(dir.join("flange.fkp"), peca("Flange")?.0.to_json()).ok()?;
        let arq = dir.join("conjunto.fka");
        std::fs::write(&arq, conjunto().to_json()).ok()?;
        Some(arq)
    } else if nome == FOLHA {
        let arq = dir.join("folha.fkd");
        std::fs::write(&arq, folha().to_json()).ok()?;
        Some(arq)
    } else {
        None
    }
}

/// Duas peças soltas, sem restrição nenhuma: a aula de restrições começa daqui.
pub fn conjunto() -> fenkra_montagem::Assembly {
    use fenkra_montagem::{NodeKind, Placement};
    let mut a = fenkra_montagem::Assembly::new(CONJUNTO);
    let g = a.add_group("Peças", None);
    a.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), Some(g));
    a.add_component("Flange", NodeKind::Part { path: "flange.fkp".into() }, Placement { origin: [140., 20., 30.], ..Default::default() }, Some(g));
    a
}

/// Folha A3 em branco ligada à placa: sem vistas, cotas, notas nem carimbo.
pub fn folha() -> fenkra_desenho::Drawing {
    fenkra_desenho::Drawing::new(FOLHA, fenkra_desenho::Format::A3)
}

/// Fonte (a placa) a usar nas vistas do desenho de exemplo.
pub fn fonte_da_folha() -> fenkra_desenho::Source {
    fenkra_desenho::Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn forma_suave_recalcula_sem_erro_e_nao_tem_vinco_nem_conversao() {
        let (d, b) = peca("Forma suave").unwrap();
        assert_eq!(b, Bench::FormaLivre);
        let r = fenkra_solidos::recompute(&d);
        assert!(r.status.values().all(|s| !matches!(s, fenkra_solidos::Status::Error(_))), "{:?}", r.status);
        assert_eq!(r.formas.len(), 1, "uma forma lisa na cena");
        let c = crate::curso::Contexto::so_documento(&d);
        assert!(!crate::curso::Cond::FormaVinco.reconhecida(&c) && !crate::curso::Cond::FormaConvertida.reconhecida(&c));
    }

    #[test]
    fn superficies_de_exemplo_recalculam_e_dao_duas_superficies_para_juntar() {
        let (d, _) = peca("Superfícies de exemplo").unwrap();
        let r = fenkra_solidos::recompute(&d);
        assert!(r.status.values().all(|s| !matches!(s, fenkra_solidos::Status::Error(_))), "{:?}", r.status);
        assert_eq!(r.surfaces.len(), 2, "duas superfícies extrudadas");
        let c = crate::curso::Contexto::so_documento(&d);
        assert!(crate::curso::Cond::NSuperficies(2).reconhecida(&c));
        assert!(!crate::curso::Cond::SuperficieJuntada(1).reconhecida(&c), "ainda não foram juntadas");
    }

    #[test]
    fn exemplos_compostos_gravam_arquivos_que_abrem_de_novo() {
        let a = gravar_composto(CONJUNTO).expect("grava o conjunto");
        let asm = fenkra_montagem::Assembly::from_json(&std::fs::read_to_string(&a).unwrap()).unwrap();
        assert_eq!(asm.nodes.iter().filter(|n| matches!(n.kind, fenkra_montagem::NodeKind::Part { .. })).count(), 2);
        assert!(asm.mates.is_empty(), "sem restrições: quem restringe é a pessoa");
        assert!(a.with_file_name("placa.fkp").exists() && a.with_file_name("flange.fkp").exists());
        let f = gravar_composto(FOLHA).expect("grava a folha");
        let d = fenkra_desenho::Drawing::from_json(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!((d.sheets.len(), d.views.len(), d.dims.len(), d.notes.len()), (1, 0, 0, 0));
        assert!(f.with_file_name("placa.fkp").exists());
        assert!(gravar_composto("não existe").is_none());
    }

    #[test]
    fn o_curso_reconhece_o_conjunto_e_a_folha_pelo_nome() {
        let doc = Document::new("Outra");
        let mut c = crate::curso::Contexto::so_documento(&doc);
        assert!(!crate::curso::Cond::Exemplo(CONJUNTO).reconhecida(&c));
        c.nomes = [CONJUNTO, FOLHA];
        assert!(crate::curso::Cond::Exemplo(CONJUNTO).reconhecida(&c) && crate::curso::Cond::Exemplo(FOLHA).reconhecida(&c));
    }
}
