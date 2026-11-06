// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural, B296: a ligação entre as linhas da lista de corte e os corpos do 3D. Uma linha agrupa
//! membros iguais (comando, trecho); o 3D realça e seleciona por comando. As duas direções saem daqui,
//! sem interface, para o teste conferir a correspondência.

use fenkra_core::Id;
use fenkra_solidos::CutRow;

/// Os corpos (comandos) que a linha acende no modelo, sem repetição e na ordem em que aparecem.
pub fn corpos_da_linha(r: &CutRow) -> Vec<Id> {
    let mut v: Vec<Id> = vec![];
    for (f, _) in &r.members {
        if !v.contains(f) {
            v.push(*f);
        }
    }
    v
}

/// As linhas que contêm o corpo clicado. Um comando com trechos de comprimentos diferentes cai em mais
/// de uma linha: todas ficam marcadas, porque o 3D seleciona o comando inteiro.
pub fn linhas_do_corpo(rows: &[CutRow], corpo: Id) -> Vec<usize> {
    rows.iter().enumerate().filter(|(_, r)| r.members.iter().any(|m| m.0 == corpo)).map(|(k, _)| k).collect()
}

/// A linha está marcada: foi a clicada na lista (pelo primeiro membro) ou contém o corpo selecionado.
pub fn marcada(r: &CutRow, clicada: Option<(Id, usize)>, selecionado: Option<Id>) -> bool {
    let pelo_clique = clicada.is_some() && r.members.first().copied() == clicada;
    pelo_clique || selecionado.is_some_and(|f| r.members.iter().any(|m| m.0 == f))
}

/// Corpos a realçar: a linha sob o mouse; sem ela, a clicada.
pub fn alvo_do_realce(rows: &[CutRow], sob_mouse: Option<usize>, clicada: Option<(Id, usize)>) -> Vec<Id> {
    sob_mouse
        .and_then(|k| rows.get(k))
        .or_else(|| clicada.and_then(|c| rows.iter().find(|r| r.members.first() == Some(&c))))
        .map(corpos_da_linha)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::*;

    fn membro(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> Id {
        let s = |x: &str| x.to_string();
        let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") });
        d.add_feature(FeatureKind::StructMember(StructMember {
            paths: vec![CurveRef::Ref(l)],
            family: s("Tubo quadrado"),
            designation: s("40x3"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("40"), s("3")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::Nenhum,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
        }))
    }

    #[test]
    fn linha_e_corpo_se_correspondem_nos_dois_sentidos() {
        let mut d = Document::new("t");
        // duas barras de 500 mm (uma linha com 2 corpos) e uma de 800 mm (outra linha)
        let a = membro(&mut d, [0., 0., 0.], [500., 0., 0.]);
        let b = membro(&mut d, [0., 100., 0.], [500., 100., 0.]);
        let c = membro(&mut d, [0., 200., 0.], [800., 200., 0.]);
        let rows = fenkra_solidos::cut_list(&d);
        assert_eq!(rows.len(), 2, "{rows:?}");
        let r500 = rows.iter().find(|r| (r.length - 500.).abs() < 0.1).expect("linha de 500");
        let r800 = rows.iter().find(|r| (r.length - 800.).abs() < 0.1).expect("linha de 800");
        assert_eq!(r500.count, 2);
        let mut dos500 = corpos_da_linha(r500);
        dos500.sort();
        let mut esperado = vec![a, b];
        esperado.sort();
        assert_eq!(dos500, esperado);
        assert_eq!(corpos_da_linha(r800), vec![c]);
        // corpo -> linha, e a ida e a volta fecham
        for (corpo, r) in [(a, r500), (b, r500), (c, r800)] {
            let ks = linhas_do_corpo(&rows, corpo);
            assert_eq!(ks.len(), 1);
            assert_eq!(rows[ks[0]], *r);
            assert!(corpos_da_linha(&rows[ks[0]]).contains(&corpo));
            assert!(marcada(r, None, Some(corpo)));
        }
        assert!(!marcada(r800, None, Some(a)));
    }

    #[test]
    fn realce_prefere_o_mouse_e_cai_na_linha_clicada() {
        let mut d = Document::new("t");
        let a = membro(&mut d, [0., 0., 0.], [500., 0., 0.]);
        let c = membro(&mut d, [0., 200., 0.], [800., 200., 0.]);
        let rows = fenkra_solidos::cut_list(&d);
        let k_a = linhas_do_corpo(&rows, a)[0];
        let k_c = linhas_do_corpo(&rows, c)[0];
        let clicada = rows[k_a].members.first().copied();
        assert_eq!(alvo_do_realce(&rows, Some(k_c), clicada), vec![c]);
        assert_eq!(alvo_do_realce(&rows, None, clicada), vec![a]);
        assert!(alvo_do_realce(&rows, None, None).is_empty());
        assert!(marcada(&rows[k_a], clicada, None));
        assert!(!marcada(&rows[k_c], clicada, None));
    }
}
