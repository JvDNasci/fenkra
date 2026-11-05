// SPDX-License-Identifier: LGPL-2.1-or-later
//! Regras do manipulador da montagem (B261): em que sistema ele trabalha, em volta de que ponto gira,
//! que alças as restrições deixam aparecer e o motivo de uma peça estar presa. A tela só desenha o que
//! sai daqui.

use crate::solve::{self, Liberdade};
use crate::*;

/// Sistema dos eixos do manipulador.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Referencia {
    #[default]
    Mundo,
    Peca,
    /// normal da face escolhida (guardada no sistema da peça, para acompanhá-la)
    Face,
}

/// Ponto em volta do qual o manipulador gira.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Pivo {
    /// o que as restrições mandam e, solta, o centro da caixa envolvente
    #[default]
    Centro,
    OrigemDaPeca,
    /// ponto escolhido no 3D, no sistema da peça
    Ponto(V3),
}

/// Por que a peça não se move.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Trava {
    Fixa,
    /// presa a uma peça fixa por uma fixação conjunta
    Grupo,
    /// quantas restrições e juntas a seguram
    Restricoes(usize),
}

/// Três eixos ortonormais do manipulador, no mundo.
pub fn eixos(r: Referencia, p: &Placement, normal_local: Option<V3>) -> [V3; 3] {
    let mundo = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    match (r, normal_local) {
        (Referencia::Mundo, _) => mundo,
        (Referencia::Face, Some(n)) => {
            let z = unit(p.dir(n));
            let x = unit(any_perp(z));
            [x, cross(z, x), z]
        }
        _ => mundo.map(|e| p.dir(e)),
    }
}

/// Ponto de giro no mundo. `sugerido` é o que as restrições impõem para o eixo escolhido.
pub fn ponto_de_giro(pivo: Pivo, p: &Placement, centro: V3, sugerido: Option<V3>) -> V3 {
    match pivo {
        Pivo::Centro => sugerido.unwrap_or(centro),
        Pivo::OrigemDaPeca => p.origin,
        Pivo::Ponto(q) => p.point(q),
    }
}

/// Posição depois de mover `d[i]` ao longo de `eixos[i]`.
pub fn movido(p: &Placement, eixos: &[V3; 3], d: [f64; 3]) -> Placement {
    let t = [0, 1, 2].map(|k| eixos[0][k] * d[0] + eixos[1][k] * d[1] + eixos[2][k] * d[2]);
    p.moved(t, [0.; 3], p.origin)
}

/// Posição depois de girar `graus` em volta do eixo que passa por `ponto`.
pub fn girado(p: &Placement, eixo: V3, graus: f64, ponto: V3) -> Placement {
    p.moved([0.; 3], unit(eixo).map(|v| v * graus.to_radians()), ponto)
}

/// Leva a peça para `alvo` respeitando as restrições: o que elas deixam acompanha, o resto fica.
pub fn levar(a: &mut Assembly, id: Id, alvo: Placement) {
    if a.mates.is_empty() && a.joints.is_empty() {
        if let Some(n) = a.node_mut(id) {
            n.placement = alvo;
        }
    } else {
        solve::solve_toward(a, id, alvo, 0.05);
    }
}

/// Motivo de a peça não ter alça nenhuma. `None`: alguma coisa ela pode fazer.
pub fn trava(a: &Assembly, id: Id, l: Option<&Liberdade>) -> Option<Trava> {
    let n = a.node(id)?;
    if n.fixed {
        return Some(Trava::Fixa);
    }
    let Some(l) = l else { return Some(Trava::Grupo) };
    if l.mover.iter().chain(&l.girar).any(|x| *x) {
        return None;
    }
    let mates = a.mates.iter().filter(|m| !m.suppressed && (m.a.comp == id || m.b.comp == id)).count();
    let juntas = a.joints.iter().filter(|j| !j.suppressed && (j.a.comp == id || j.b.comp == id)).count();
    Some(Trava::Restricoes(mates + juntas))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kin::{JFrame, JRef, JointKind};

    const X: V3 = [1., 0., 0.];
    const Y: V3 = [0., 1., 0.];
    const Z: V3 = [0., 0., 1.];

    fn par() -> (Assembly, Id, Id) {
        let mut a = Assembly::new("par");
        let f = a.add_component("base", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        a.node_mut(f).unwrap().fixed = true;
        let m = a.add_component("tampa", NodeKind::Part { path: "b.fkp".into() }, Placement { origin: [100., 0., 0.], ..Default::default() }, None);
        (a, f, m)
    }

    fn com_junta(k: JointKind) -> (Assembly, Id) {
        let (mut a, f, m) = par();
        // eixo vertical em (40, 0): longe do centro da peça
        let fa = JFrame::from_axis([40., 0., 0.], Z);
        let fb = JFrame::from_axis([-60., 0., 0.], Z);
        a.add_joint(k, JRef { comp: f, frame: fa }, JRef { comp: m, frame: fb });
        (a, m)
    }

    fn liberdade(a: &Assembly, m: Id, e: [V3; 3]) -> Liberdade {
        solve::liberdade(a, m, e, a.node(m).unwrap().placement.origin).expect("peça com corpo livre")
    }

    #[test]
    fn peca_solta_tem_todas_as_alcas() {
        let (a, _, m) = par();
        let l = liberdade(&a, m, [X, Y, Z]);
        assert_eq!((l.mover, l.girar), ([true; 3], [true; 3]));
        assert_eq!(trava(&a, m, Some(&l)), None);
    }

    #[test]
    fn revolucao_so_mostra_o_anel_do_eixo_e_gira_na_dobradica() {
        let (a, m) = com_junta(JointKind::Revolute);
        let l = liberdade(&a, m, [X, Y, Z]);
        assert_eq!(l.mover, [false; 3]);
        assert_eq!(l.girar, [false, false, true]);
        let p = l.pivo[2].expect("o eixo da junta");
        assert!(norm(sub([p[0], p[1], 0.], [40., 0., 0.])) < 1e-4, "pivô na dobradiça: {p:?}");
    }

    #[test]
    fn prismatica_so_mostra_a_seta() {
        let (a, m) = com_junta(JointKind::Prismatic);
        let l = liberdade(&a, m, [X, Y, Z]);
        assert_eq!(l.mover, [false, false, true]);
        assert_eq!(l.girar, [false; 3]);
    }

    #[test]
    fn cilindrica_mostra_seta_e_anel() {
        let (a, m) = com_junta(JointKind::Cylindrical);
        let l = liberdade(&a, m, [X, Y, Z]);
        assert_eq!(l.mover, [false, false, true]);
        assert_eq!(l.girar, [false, false, true]);
    }

    #[test]
    fn contato_deixa_deslizar_e_girar_no_plano() {
        let (mut a, f, m) = par();
        let (ra, rb) = (MateRef { comp: f, geom: Geom::Plane { p: [0.; 3], n: Z }, pick: None }, MateRef { comp: m, geom: Geom::Plane { p: [0.; 3], n: [0., 0., -1.] }, pick: None });
        a.add_mate(MateKind::Contact, ra, rb);
        let l = liberdade(&a, m, [X, Y, Z]);
        assert_eq!(l.mover, [true, true, false]);
        assert_eq!(l.girar, [false, false, true]);
    }

    #[test]
    fn eixos_da_peca_mudam_o_que_e_livre() {
        // peça girada 90° em X: o z da peça aponta para -y do mundo, e a prismática anda no z do mundo
        let (mut a, m) = com_junta(JointKind::Prismatic);
        a.node_mut(m).unwrap().placement = girado(&a.node(m).unwrap().placement, X, 90., [100., 0., 0.]);
        let e = eixos(Referencia::Peca, &a.node(m).unwrap().placement, None);
        assert!(norm(sub(e[1], Z)) < 1e-9, "o y da peça é o z do mundo");
        let l = liberdade(&a, m, e);
        assert_eq!(l.mover, [false, true, false], "a seta que fica é a do y da peça");
    }

    #[test]
    fn peca_fixa_e_presa_dao_o_motivo() {
        let (mut a, f, m) = par();
        assert_eq!(trava(&a, f, None), Some(Trava::Fixa));
        assert!(solve::liberdade(&a, f, [X, Y, Z], [0.; 3]).is_none());
        let fr = JFrame::from_axis([0.; 3], Z);
        a.add_joint(JointKind::Rigid, JRef { comp: f, frame: fr }, JRef { comp: m, frame: fr });
        let l = liberdade(&a, m, [X, Y, Z]);
        assert_eq!(trava(&a, m, Some(&l)), Some(Trava::Restricoes(1)));
    }

    #[test]
    fn grupo_preso_a_peca_fixa_tem_motivo_proprio() {
        let (mut a, f, m) = par();
        a.add_rigid(vec![f, m]);
        assert!(solve::liberdade(&a, m, [X, Y, Z], [0.; 3]).is_none());
        assert_eq!(trava(&a, m, None), Some(Trava::Grupo));
    }

    #[test]
    fn elo_do_meio_de_um_mecanismo_nao_fica_preso() {
        // a alça olha o mecanismo inteiro: o braço sai do lugar levando a peça da ponta junto
        let (mut a, f, m) = par();
        let ponta = a.add_component("ponta", NodeKind::Part { path: "c.fkp".into() }, Placement { origin: [300., 0., 0.], ..Default::default() }, None);
        let fa = JFrame::from_axis([0.; 3], Z);
        let fb = JFrame::from_axis([-100., 0., 0.], Z);
        a.add_joint(JointKind::Revolute, JRef { comp: f, frame: fa }, JRef { comp: m, frame: fb });
        let fc = JFrame::from_axis([100., 0., 0.], Z);
        a.add_joint(JointKind::Revolute, JRef { comp: m, frame: fc }, JRef { comp: ponta, frame: fb });
        let l = liberdade(&a, ponta, [X, Y, Z]);
        assert!(l.girar[2], "gira em volta do pino dela");
        assert!(l.mover[1], "o elo de trás deixa a ponta subir");
    }

    #[test]
    fn girar_37_graus_num_eixo_da_peca_sem_restricoes() {
        let (mut a, _, m) = par();
        let inicio = girado(&a.node(m).unwrap().placement, X, 30., [100., 0., 0.]);
        a.node_mut(m).unwrap().placement = inicio;
        let e = eixos(Referencia::Peca, &inicio, None);
        let alvo = girado(&inicio, e[2], 37., [100., 0., 0.]);
        levar(&mut a, m, alvo);
        let p = a.node(m).unwrap().placement;
        let cosseno = dot(p.dir(X), inicio.dir(X));
        assert!((cosseno - 37f64.to_radians().cos()).abs() < 1e-9);
        assert!(norm(sub(p.dir(Z), inicio.dir(Z))) < 1e-9, "o eixo da peça não sai do lugar");
    }

    #[test]
    fn girar_37_graus_respeita_a_junta() {
        let (mut a, m) = com_junta(JointKind::Revolute);
        let inicio = a.node(m).unwrap().placement;
        let l = liberdade(&a, m, eixos(Referencia::Peca, &inicio, None));
        let ponto = ponto_de_giro(Pivo::Centro, &inicio, [100., 0., 0.], l.pivo[2]);
        levar(&mut a, m, girado(&inicio, Z, 37., ponto));
        let p = a.node(m).unwrap().placement;
        let cosseno = dot(p.dir(X), X);
        assert!((cosseno - 37f64.to_radians().cos()).abs() < 1e-6, "{cosseno}");
        let dist = norm(sub(p.point([-60., 0., 0.]), [40., 0., 0.]));
        assert!(dist < 1e-6, "continua no pino: {dist}");
    }

    #[test]
    fn mover_digitado_vai_pelos_eixos_escolhidos() {
        let p = girado(&Placement::default(), Z, 90., [0.; 3]);
        let e = eixos(Referencia::Peca, &p, None);
        let q = movido(&p, &e, [10., 0., 0.]);
        assert!(norm(sub(q.origin, [0., 10., 0.])) < 1e-9, "o x da peça aponta para y");
    }

    #[test]
    fn referencia_de_face_acompanha_a_peca() {
        let p = girado(&Placement::default(), X, 90., [0.; 3]);
        let e = eixos(Referencia::Face, &p, Some(Z));
        assert!(norm(sub(e[2], [0., -1., 0.])) < 1e-9);
        assert!(dot(e[0], e[1]).abs() < 1e-12 && dot(e[0], e[2]).abs() < 1e-12);
    }

    #[test]
    fn pivo_escolhido_manda_mais_que_o_sugerido() {
        let p = Placement { origin: [5., 0., 0.], ..Default::default() };
        assert_eq!(ponto_de_giro(Pivo::Centro, &p, [1., 1., 1.], None), [1., 1., 1.]);
        assert_eq!(ponto_de_giro(Pivo::Centro, &p, [1., 1., 1.], Some([2., 2., 2.])), [2., 2., 2.]);
        assert_eq!(ponto_de_giro(Pivo::OrigemDaPeca, &p, [1., 1., 1.], Some([2., 2., 2.])), [5., 0., 0.]);
        assert_eq!(ponto_de_giro(Pivo::Ponto([1., 0., 0.]), &p, [1., 1., 1.], None), [6., 0., 0.]);
    }
}
