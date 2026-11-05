// SPDX-License-Identifier: LGPL-2.1-or-later
//! B102: a montagem escolhe a configuração de cada componente pelo nome; nome que a peça não tem é
//! erro visível, nunca a primeira configuração.

use fenkra_core::configuracoes::{Coluna, Linha};
use fenkra_core::*;
use fenkra_montagem::load::PartCache;
use fenkra_montagem::view::compose;
use fenkra_montagem::{Assembly, NodeKind, Placement};

fn volume(r: &fenkra_solidos::RecomputeResult) -> f64 {
    r.bodies.iter().map(|b| b.mass.volume).sum()
}

/// Barra de seção 10 × 10 e comprimento L = 100, com a tabela Curta (50) e Longa (200).
fn barra() -> Document {
    let mut d = Document::new("Barra");
    d.add_param("L", "100", Unit::Comprimento);
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        data.add_rect(&mut al, [0., 0.], [10., 10.], false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("L".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    d.nova_coluna_config(Coluna::Parametro("L".into()));
    d.nova_coluna_config(Coluna::Pn);
    d.tabela.linhas.push(Linha { nome: "Curta".into(), celulas: vec!["50".into(), "B-50".into()] });
    d.tabela.linhas.push(Linha { nome: "Longa".into(), celulas: vec!["200".into(), "B-200".into()] });
    d
}

fn pasta(nome: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("fenkra_b102_{nome}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn componente_usa_a_configuracao_pelo_nome() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = pasta("nome");
    std::fs::write(dir.join("barra.fkp"), barra().to_json()).unwrap();
    let mut cache = PartCache::default();
    let f = dir.join("barra.fkp");
    let curta = volume(&cache.get_config(&f, false, "Curta").unwrap().result);
    let longa = volume(&cache.get_config(&f, false, "Longa").unwrap().result);
    let base = volume(&cache.get_config(&f, false, "").unwrap().result);
    assert!((curta - 5000.).abs() < 1e-6 && (longa - 20000.).abs() < 1e-6 && (base - 10000.).abs() < 1e-6, "{curta} {longa} {base}");
    // o PN da variante vem junto
    assert_eq!(cache.get_config(&f, false, "Longa").unwrap().doc.props.pn, "B-200");
    // nome que não existe: erro com os nomes, não a primeira configuração
    let e = cache.get_config(&f, false, "Media").err().expect("deveria falhar");
    assert!(e.contains("'Media'") && e.contains("Curta, Longa"), "{e}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn montagem_mostra_o_erro_do_componente_com_configuracao_inexistente() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = pasta("asm");
    std::fs::write(dir.join("barra.fkp"), barra().to_json()).unwrap();
    let mut a = Assembly::new("Conjunto");
    let mut ids = vec![];
    for (i, nome) in ["Curta", "Longa", "Media", ""].iter().enumerate() {
        let id = a.add_component(&format!("Barra {i}"), NodeKind::Part { path: "barra.fkp".into() }, Placement { origin: [20. * i as f64, 0., 0.], ..Default::default() }, None);
        a.definir_config(id, nome);
        ids.push(id);
    }
    assert_eq!(a.config_de(ids[2]), "Media");
    assert_eq!(a.config_de(ids[3]), "");
    // vai e volta pelo arquivo
    let a = Assembly::from_json(&a.to_json()).unwrap();
    assert_eq!(a.config_de(ids[1]), "Longa");

    let mut cache = PartCache::default();
    let mut erros = vec![];
    let (_doc, r) = compose(&a, Some(&dir), &mut cache, &mut erros);
    // só a "Media" falha, e o erro diz qual configuração e quais existem
    assert_eq!(erros.len(), 1, "{erros:?}");
    assert_eq!(erros[0].0, ids[2]);
    assert!(erros[0].1.contains("'Media'") && erros[0].1.contains("Curta, Longa"), "{}", erros[0].1);
    // as outras três entram com a sua variante: 50 + 200 + 100 de comprimento, seção 10 × 10
    assert!((volume(&r) - 100. * (50. + 200. + 100.)).abs() < 1e-3, "{}", volume(&r));
    // trocar o nome conserta
    let mut a = a;
    a.definir_config(ids[2], "Curta");
    let mut erros = vec![];
    let (_d, r) = compose(&a, Some(&dir), &mut cache, &mut erros);
    assert!(erros.is_empty(), "{erros:?}");
    assert!((volume(&r) - 100. * (50. + 200. + 50. + 100.)).abs() < 1e-3);
    std::fs::remove_dir_all(&dir).ok();
}
