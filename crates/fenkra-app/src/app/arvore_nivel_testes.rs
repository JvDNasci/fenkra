// SPDX-License-Identifier: LGPL-2.1-or-later
//! B557: o recurso gerado nasce um nível abaixo de quem o gerou; a pessoa tira ou devolve, a escolha
//! é gravada e a ordem de recálculo não muda.

use super::*;
use crate::arvore_soltura_ui::{aplicar, Alvo};
use fenkra_core::arvore_nivel::NoExibicao;

fn id_de(d: &Document, nome: &str) -> Id {
    d.features.iter().find(|f| f.name == nome).map(|f| f.id).unwrap_or_else(|| panic!("{nome} não existe"))
}

fn ordem(d: &Document) -> Vec<Id> {
    d.features.iter().map(|f| f.id).collect()
}

fn corpo_de(d: &Document, id: Id) -> Id {
    d.feature(id).unwrap().target
}

fn raiz<'a>(h: &'a [NoExibicao], id: Id) -> Option<&'a NoExibicao> {
    h.iter().find(|n| n.id == id)
}

fn filhos_de(d: &Document, id: Id) -> Vec<Id> {
    let h = d.hierarquia_exibicao(corpo_de(d, id));
    fn achar(h: &[NoExibicao], id: Id) -> Option<Vec<Id>> {
        h.iter().find_map(|n| if n.id == id { Some(n.filhos.iter().map(|f| f.id).collect()) } else { achar(&n.filhos, id) })
    }
    achar(&h, id).unwrap_or_default()
}

#[test]
fn extrusao_nasce_dentro_do_sketch() {
    let d = fenkra_solidos::example_document();
    let base = id_de(&d, "Sketch base");
    let placa = id_de(&d, "Placa");
    assert_eq!(d.gerador_de(placa), Some(base));
    assert_eq!(d.pai_na_arvore(placa), Some(base));
    assert_eq!(d.pai_na_arvore(base), None, "o sketch é raiz");
    let h = d.hierarquia_exibicao(corpo_de(&d, placa));
    assert!(raiz(&h, base).is_some_and(|n| n.filhos.iter().any(|f| f.id == placa)), "Placa dentro do Sketch base: {h:?}");
    assert!(raiz(&h, placa).is_none(), "e não repetida no nível do sketch");
    // todo recurso da linha do tempo aparece uma vez só, na hierarquia
    fn conta(h: &[NoExibicao]) -> usize {
        h.iter().map(|n| 1 + conta(&n.filhos)).sum()
    }
    let esperado = d.features.iter().filter(|f| f.target == corpo_de(&d, placa) && !d.is_free(f)).count();
    assert_eq!(conta(&h), esperado);
}

#[test]
fn tirar_deixa_no_mesmo_nivel_e_devolver_volta_para_dentro_sem_mexer_na_ordem() {
    let mut d = fenkra_solidos::example_document();
    let base = id_de(&d, "Sketch base");
    let placa = id_de(&d, "Placa");
    let corpo = corpo_de(&d, placa);
    let antes = ordem(&d);
    // tirar: soltar no nó do corpo
    aplicar(&mut d, placa, Alvo::Corpo(corpo)).expect("tirar do sketch");
    assert_eq!(d.pai_na_arvore(placa), None);
    assert!(d.esta_fora_do_gerador(placa));
    let h = d.hierarquia_exibicao(corpo);
    assert!(raiz(&h, placa).is_some(), "Placa no nível do sketch");
    assert!(filhos_de(&d, base).is_empty());
    assert_eq!(ordem(&d), antes, "a ordem de recálculo não muda");
    // devolver: soltar sobre o sketch
    aplicar(&mut d, placa, Alvo::Depois(base)).expect("de volta para dentro");
    assert_eq!(d.pai_na_arvore(placa), Some(base));
    assert!(!d.esta_fora_do_gerador(placa));
    assert_eq!(ordem(&d), antes);
    // tirar de novo e conferir que o tirar é idempotente
    aplicar(&mut d, placa, Alvo::Corpo(corpo)).unwrap();
    d.definir_fora_do_gerador(placa, true);
    assert_eq!(d.soltos_do_gerador, vec![placa]);
}

#[test]
fn gravar_e_reabrir_mantem_a_escolha() {
    let mut d = fenkra_solidos::example_document();
    let placa = id_de(&d, "Placa");
    let corpo = corpo_de(&d, placa);
    // documento novo: nada gravado do campo, abre com a regra (dentro do gerador)
    let json = d.to_json();
    assert!(!json.contains("soltos_do_gerador"), "sem escolha, o campo nem é gravado");
    let reaberto = Document::from_json(&json).expect("abre");
    assert_eq!(reaberto.hierarquia_exibicao(corpo), d.hierarquia_exibicao(corpo));
    assert!(reaberto.pai_na_arvore(placa).is_some());
    // com a escolha de tirar, ela sobrevive a gravar e reabrir
    aplicar(&mut d, placa, Alvo::Corpo(corpo)).unwrap();
    let reaberto = Document::from_json(&d.to_json()).expect("abre");
    assert!(reaberto.esta_fora_do_gerador(placa));
    assert_eq!(reaberto.pai_na_arvore(placa), None);
    assert_eq!(reaberto.hierarquia_exibicao(corpo), d.hierarquia_exibicao(corpo));
    assert_eq!(ordem(&reaberto), ordem(&d));
}

#[test]
fn recalculo_nao_depende_do_nivel_de_exibicao() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = fenkra_solidos::example_document();
    let placa = id_de(&d, "Placa");
    let corpo = corpo_de(&d, placa);
    let volume = |d: &Document| fenkra_solidos::recompute(d).bodies.iter().map(|b| b.mass.volume).sum::<f64>();
    let v0 = volume(&d);
    assert!(v0 > 0.);
    aplicar(&mut d, placa, Alvo::Corpo(corpo)).unwrap();
    let v1 = volume(&d);
    assert!((v1 - v0).abs() < 1e-9 * v0, "volume {v1} x {v0}");
}

#[test]
fn sketch_nao_tem_gerador_e_recurso_sem_entrada_fica_na_raiz() {
    let d = fenkra_solidos::example_document();
    for f in &d.features {
        if matches!(f.kind, FeatureKind::Sketch(_)) {
            assert_eq!(d.pai_na_arvore(f.id), None, "{}", f.name);
        }
        // o pai, quando existe, vem antes na ordem de recálculo
        if let Some(p) = d.pai_na_arvore(f.id) {
            assert!(d.feature_index(p).unwrap() < d.feature_index(f.id).unwrap(), "{}", f.name);
        }
    }
}
