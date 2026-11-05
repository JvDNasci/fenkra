//! Edição no contexto da montagem (B54): as regras que não dependem da tela.
//!
//! Um componente fica ativo por vez, e o resto do conjunto continua à vista como referência. Para
//! isso o resto é levado ao sistema do componente ativo ([`vista_no_sistema`]): a viewport mostra a
//! peça nas coordenadas dela, então é o contexto que muda de sistema, não a peça.
//!
//! A peça criada no contexto só existe na memória (`PartCache::definir_pendente`) até o Salvar da
//! montagem. Cancelar a criação não deixa arquivo órfão, porque nenhum arquivo foi feito.

use crate::view::transformed;
use crate::*;
use fenkra_solidos::{BodyResult, RecomputeResult};
use std::path::{Path, PathBuf};

/// Quantos componentes usam o mesmo arquivo do componente `comp`. Mudar a peça muda todos.
pub fn instancias(a: &Assembly, comp: Id) -> usize {
    let Some(NodeKind::Part { path }) = a.node(comp).map(|n| &n.kind) else { return 0 };
    a.nodes.iter().filter(|n| matches!(&n.kind, NodeKind::Part { path: p } if p == path)).count()
}

/// Arquivo de uma peça nova: nome e pasta são obrigatórios, e o arquivo não pode existir.
pub fn arquivo_novo(pasta: &Path, nome: &str) -> Result<PathBuf, String> {
    let nome = nome.trim();
    if nome.is_empty() {
        return Err("dê um nome à peça".into());
    }
    if nome.chars().any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')) {
        return Err("o nome tem caracteres que não valem em arquivo".into());
    }
    if pasta.as_os_str().is_empty() || !pasta.is_dir() {
        return Err("escolha a pasta da peça".into());
    }
    let f = pasta.join(format!("{nome}.fkp"));
    if f.exists() {
        return Err("já existe uma peça com este nome nesta pasta".into());
    }
    Ok(f)
}

/// Junta o que está à volta do componente ativo, no sistema dele. `vistas` são as malhas do
/// conjunto, cada uma no sistema da montagem a que `pl` se refere; `fora` é o próprio ativo.
pub fn vista_no_sistema(vistas: &[&RecomputeResult], fora: Id, pl: &Placement) -> RecomputeResult {
    let inv = pl.inverse();
    let bodies: Vec<BodyResult> = vistas
        .iter()
        .flat_map(|v| v.bodies.iter())
        .filter(|b| b.collector != fora)
        .map(|b| BodyResult { collector: b.collector, color: b.color, mesh: transformed(&b.mesh, &inv), mass: b.mass, faces: b.faces, face_owner: b.face_owner.clone(), face_role: b.face_role.clone() })
        .collect();
    crate::view::resultado_de(bodies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::{full_path, PartCache};
    use fenkra_core::Document;

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_contexto_{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn nome_e_pasta_sao_obrigatorios() {
        let d = pasta("nome");
        assert!(arquivo_novo(&d, "  ").is_err());
        assert!(arquivo_novo(&d, "a/b").is_err());
        assert!(arquivo_novo(&d.join("nao_existe"), "x").is_err());
        assert!(arquivo_novo(Path::new(""), "x").is_err());
        assert_eq!(arquivo_novo(&d, " Tampa ").unwrap(), d.join("Tampa.fkp"));
        std::fs::write(d.join("Tampa.fkp"), "{}").unwrap();
        assert!(arquivo_novo(&d, "Tampa").is_err(), "não sobrescreve");
    }

    #[test]
    fn conta_as_instancias_do_arquivo() {
        let mut a = Assembly::new("A");
        let c = a.add_component("P", NodeKind::Part { path: "p.fkp".into() }, Placement::default(), None);
        a.add_component("P", NodeKind::Part { path: "p.fkp".into() }, Placement::default(), None);
        let o = a.add_component("Q", NodeKind::Part { path: "q.fkp".into() }, Placement::default(), None);
        assert_eq!((instancias(&a, c), instancias(&a, o)), (2, 1));
    }

    #[test]
    fn peca_nova_nao_toca_o_disco_ate_gravar_os_pendentes() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("pendente");
        let f = arquivo_novo(&d, "Nova").unwrap();
        let mut cache = PartCache::default();
        cache.definir_pendente(&f, fenkra_solidos::example_document());
        let mut a = Assembly::new("A");
        let c = a.add_component("Nova", NodeKind::Part { path: "Nova.fkp".into() }, Placement::default(), None);
        // a vista mostra a peça da memória, sem arquivo
        let mut erros = vec![];
        let (_, r) = crate::view::compose(&a, Some(&d), &mut cache, &mut erros);
        assert!(erros.is_empty() && r.bodies.len() == 1, "{erros:?}");
        assert!(r.bodies[0].mass.volume > 1. && !f.exists());
        // gravar um documento pendente troca a memória, não o disco
        let mut doc = cache.get(&f, false).unwrap().doc.clone();
        doc.name = "Outra".into();
        crate::links::gravar(&f, &doc, &mut cache).unwrap();
        assert!(!f.exists());
        assert_eq!(cache.get(&f, false).unwrap().doc.name, "Outra");
        // componente apagado antes de salvar: nada a gravar, e nenhum arquivo órfão
        let mut sem = a.clone();
        sem.delete(c);
        assert!(cache.gravar_pendentes(&sem, Some(&d)).unwrap().is_empty());
        assert!(!f.exists());
        // com o componente, vira arquivo comum
        cache.definir_pendente(&f, doc);
        assert_eq!(cache.gravar_pendentes(&a, Some(&d)).unwrap(), vec![full_path(Some(&d), "Nova.fkp")]);
        assert_eq!(Document::from_json(&std::fs::read_to_string(&f).unwrap()).unwrap().name, "Outra");
        assert!(!cache.e_pendente(&f));
    }

    #[test]
    fn o_contexto_vai_para_o_sistema_do_ativo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("sistema");
        std::fs::write(d.join("p.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
        let mut a = Assembly::new("A");
        let ativo = a.add_component("Ativa", NodeKind::Part { path: "p.fkp".into() }, Placement { origin: [100., 0., 0.], ..Default::default() }, None);
        let outro = a.add_component("Outra", NodeKind::Part { path: "p.fkp".into() }, Placement::default(), None);
        let mut cache = PartCache::default();
        let (_, v) = crate::view::compose(&a, Some(&d), &mut cache, &mut vec![]);
        let x0 = v.bodies.iter().find(|b| b.collector == outro).unwrap().mesh.positions[0];
        let c = vista_no_sistema(&[&v], ativo, &a.node(ativo).unwrap().placement);
        assert_eq!(c.bodies.len(), 1, "o ativo não entra no próprio contexto");
        assert!((c.bodies[0].mesh.positions[0] - (x0 - 100.)).abs() < 1e-3);
    }
}
