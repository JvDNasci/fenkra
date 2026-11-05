// SPDX-License-Identifier: LGPL-2.1-or-later
//! Colocar uma peça com o cursor (B262). O que depende só da montagem e da geometria mora aqui, sem
//! interface: onde a peça cai a partir do raio do cursor e o que muda no documento a cada clique.
//!
//! Como funciona: a escolha fica fora do documento enquanto o cursor anda. Só o clique chama
//! [`colocar`], que cria um componente; quem chama guarda um passo de desfazer antes, então cada
//! cópia sai com um desfazer e cancelar não deixa nada na árvore.

use crate::*;

/// Peça ou subconjunto na mão da pessoa.
#[derive(Clone, Debug, PartialEq)]
pub struct Escolha {
    /// caminho como fica gravado no componente (relativo à pasta da montagem, quando dá)
    pub path: String,
    pub nome: String,
    pub sub: bool,
}

/// A próxima peça não segue o cursor? A primeira ancora a montagem, e no projeto relacional toda peça
/// fica na origem, com a forma vindo dos links.
pub fn fixa_na_origem(a: &Assembly) -> bool {
    a.relational || a.nodes.iter().all(|n| matches!(n.kind, NodeKind::Group))
}

/// Ponto do mundo sob o cursor: o chão (Z = 0) quando o raio o cruza perto da cena; olhando rasante ou
/// para cima, o plano da tela que passa pelo alvo da câmera. Sem isso a peça fugiria para o infinito
/// com a vista quase de lado.
pub fn ponto_do_cursor(origem: V3, dir: V3, alvo: V3, dist: f64) -> V3 {
    let ao_longo = |t: f64| [0, 1, 2].map(|i| origem[i] + dir[i] * t);
    if dir[2].abs() > 0.15 {
        let t = -origem[2] / dir[2];
        if t > 0. && t < 8. * dist.max(1.) {
            return ao_longo(t);
        }
    }
    let d: f64 = dir.iter().map(|x| x * x).sum::<f64>().max(1e-12);
    let t: f64 = (0..3).map(|i| (alvo[i] - origem[i]) * dir[i]).sum::<f64>() / d;
    ao_longo(t)
}

/// Origem do componente para o `apoio` (ponto da peça, no sistema dela) cair em `ponto`.
pub fn origem_para(apoio: V3, ponto: V3) -> V3 {
    [0, 1, 2].map(|i| ponto[i] - apoio[i])
}

/// Põe uma cópia na montagem e devolve o componente. `origem` vale só para a peça que segue o cursor;
/// a que fica fixa na origem ignora o ponto. A primeira já sai fixa de `add_component`; no relacional
/// a fixação é aqui.
pub fn colocar(a: &mut Assembly, e: &Escolha, origem: V3, pai: Option<Id>) -> Id {
    let at = if fixa_na_origem(a) { [0.; 3] } else { origem };
    let kind = if e.sub { NodeKind::SubAssembly { path: e.path.clone() } } else { NodeKind::Part { path: e.path.clone() } };
    let id = a.add_component(&e.nome, kind, Placement { origin: at, ..Default::default() }, pai);
    if a.relational {
        if let Some(n) = a.node_mut(id) {
            n.fixed = true;
        }
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peca() -> Escolha {
        Escolha { path: "placa.fkp".into(), nome: "placa".into(), sub: false }
    }

    #[test]
    fn primeira_fica_fixa_na_origem_e_as_outras_onde_o_cursor_mandou() {
        let mut a = Assembly::new("t");
        assert!(fixa_na_origem(&a));
        let p = colocar(&mut a, &peca(), [50., 60., 0.], None);
        assert!(a.node(p).unwrap().fixed);
        assert_eq!(a.node(p).unwrap().placement.origin, [0.; 3]);
        assert!(!fixa_na_origem(&a));
        let q = colocar(&mut a, &peca(), [50., 60., 0.], None);
        assert!(!a.node(q).unwrap().fixed);
        assert_eq!(a.node(q).unwrap().placement.origin, [50., 60., 0.]);
        assert_ne!(a.node(p).unwrap().name, a.node(q).unwrap().name, "cada cópia tem nome próprio");
    }

    #[test]
    fn grupo_vazio_nao_conta_como_primeira_peca() {
        let mut a = Assembly::new("t");
        a.add_group("Grupo", None);
        assert!(fixa_na_origem(&a));
    }

    #[test]
    fn projeto_relacional_nunca_sai_da_origem() {
        let mut a = Assembly::new("t");
        a.relational = true;
        colocar(&mut a, &peca(), [1., 2., 3.], None);
        let q = colocar(&mut a, &peca(), [1., 2., 3.], None);
        assert!(fixa_na_origem(&a));
        assert!(a.node(q).unwrap().fixed);
        assert_eq!(a.node(q).unwrap().placement.origin, [0.; 3]);
    }

    #[test]
    fn cursor_cai_no_chao_e_vista_rasante_nao_foge_para_o_infinito() {
        let chao = ponto_do_cursor([0., 0., 100.], [0.6, 0., -0.8], [0.; 3], 100.);
        assert!(chao[2].abs() < 1e-9);
        assert!((chao[0] - 75.).abs() < 1e-9);
        // quase horizontal: fica no plano da tela que passa pelo alvo
        let d = unit([1., 0., -0.01]);
        let rasante = ponto_do_cursor([-300., 0., 10.], d, [0.; 3], 300.);
        assert!(rasante[0].abs() < 1., "ficou em {rasante:?}");
        // olhando para cima não há chão à frente
        let acima = ponto_do_cursor([0., 0., 100.], [0., 0., 1.], [0.; 3], 100.);
        assert!(acima[2].abs() < 1e-9);
    }

    #[test]
    fn apoio_da_peca_vai_ao_ponto() {
        assert_eq!(origem_para([10., 20., -5.], [100., 100., 0.]), [90., 80., 5.]);
    }
}
