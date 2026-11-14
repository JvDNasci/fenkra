// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estudo de movimento (B121): o que falta comandar no mecanismo.
//!
//! A conta de graus de liberdade diz quantos sobram, mas não quais. Aqui cada grau de giro ou curso
//! sem comando é testado com um comando de experiência: se a conta cai, ele é independente. Os
//! comandos de experiência ficam para os testes seguintes, então um grau que só repete outro já
//! listado (as três outras juntas de um quatro barras) não aparece.

use crate::kin::Grau;
use crate::solve::dof_with;
use crate::*;

/// O que ainda falta comandar para o mecanismo ficar determinado (B121).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FaltaComandar {
    /// graus de juntas que, comandados, tiram um grau de liberdade cada um: um por grau livre
    /// independente (num quatro barras sai uma junta só, e não as quatro)
    pub graus: Vec<(Id, Grau)>,
    /// graus livres que não se resolvem com comando de junta: peça solta, esférica, plano
    pub outros: usize,
}

/// Graus de liberdade que ainda faltam comandar. Para cada grau de giro ou curso sem comando, na
/// ordem das juntas, põe um comando de experiência e vê se a conta de graus livres cai: se cai, o
/// grau é independente e entra na lista, e o comando fica para os próximos testes, de modo que um
/// grau que só repete outro já listado não aparece duas vezes.
pub fn falta_comandar(a: &Assembly) -> FaltaComandar {
    let mut teste = a.clone();
    let mut livre = dof_with(&teste, true);
    let mut graus = vec![];
    for i in 0..teste.joints.len() {
        if livre == 0 {
            break;
        }
        if teste.joints[i].suppressed {
            continue;
        }
        let (giro, curso) = teste.joints[i].kind.drives();
        for (pode, g) in [(giro, Grau::Giro), (curso, Grau::Curso)] {
            let j = &mut teste.joints[i];
            let campo = if g == Grau::Giro { &mut j.drive_angle } else { &mut j.drive_len };
            if !pode || campo.is_some() {
                continue;
            }
            *campo = Some("0".into());
            let agora = dof_with(&teste, true);
            if agora < livre {
                graus.push((teste.joints[i].id, g));
                livre = agora;
            } else {
                let j = &mut teste.joints[i];
                match g {
                    Grau::Giro => j.drive_angle = None,
                    _ => j.drive_len = None,
                }
            }
        }
    }
    FaltaComandar { graus, outros: livre }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kin::{analyze, JFrame, JRef, JointKind};

    fn manivela() -> (Assembly, Id, Id) {
        let mut a = Assembly::new("manivela");
        let base = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        let braco = a.add_component("Braço", NodeKind::Part { path: "c".into() }, Placement::default(), None);
        let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let j = a.add_joint(JointKind::Revolute, JRef { comp: base, frame: f }, JRef { comp: braco, frame: f });
        (a, braco, j)
    }

    /// Quatro barras com a base fixa: manivela de 10, biela de 50 e balancim de 40 mm.
    fn quatro_barras_fixas() -> (Assembly, Vec<Id>) {
        let mut a = Assembly::new("Quatro barras");
        let parte = |n: &str| NodeKind::Part { path: n.into() };
        let base = a.add_component("Base", parte("b"), Placement::default(), None);
        a.node_mut(base).unwrap().fixed = true;
        let man = a.add_component("Manivela", parte("m"), Placement::default(), None);
        let bal = a.add_component("Balancim", parte("c"), Placement::default(), None);
        let bie = a.add_component("Biela", parte("d"), Placement::default(), None);
        let z = |o: V3| JFrame::from_axis(o, [0., 0., 1.]);
        let js = vec![
            a.add_joint(JointKind::Revolute, JRef { comp: base, frame: z([0.; 3]) }, JRef { comp: man, frame: z([0.; 3]) }),
            a.add_joint(JointKind::Revolute, JRef { comp: base, frame: z([50., 0., 0.]) }, JRef { comp: bal, frame: z([50., 0., 0.]) }),
            a.add_joint(JointKind::Revolute, JRef { comp: man, frame: z([10., 0., 0.]) }, JRef { comp: bie, frame: z([10., 0., 0.]) }),
            a.add_joint(JointKind::Revolute, JRef { comp: bal, frame: z([50., 40., 0.]) }, JRef { comp: bie, frame: z([50., 40., 0.]) }),
        ];
        assert!(crate::solve::solve(&mut a).converged);
        (a, js)
    }

    #[test]
    fn falta_comandar_lista_um_grau_por_liberdade() {
        let (mut a, _, j) = manivela();
        let base = a.nodes[0].id;
        a.node_mut(base).unwrap().fixed = true;
        assert_eq!(falta_comandar(&a), FaltaComandar { graus: vec![(j, Grau::Giro)], outros: 0 });
        a.joints[0].drive_angle = Some("90*t".into());
        assert_eq!(falta_comandar(&a), FaltaComandar::default(), "com o comando nada falta");

        // quatro barras: quatro juntas, um grau só
        let (a, js) = quatro_barras_fixas();
        assert_eq!(analyze(&a).0, 1);
        assert_eq!(falta_comandar(&a).graus, vec![(js[0], Grau::Giro)]);

        // peça solta não tem junta para comandar: fica em "outros"
        let (mut a, _, _) = manivela();
        let base = a.nodes[0].id;
        a.node_mut(base).unwrap().fixed = true;
        a.add_component("Solta", NodeKind::Part { path: "s".into() }, Placement::default(), None);
        let f = falta_comandar(&a);
        assert_eq!((f.graus.len(), f.outros), (1, 6));
    }
}
