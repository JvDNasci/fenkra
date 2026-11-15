// SPDX-License-Identifier: LGPL-2.1-or-later
//! B474: reancorar uma restrição de referência perdida. O fluxo é: selecionar a restrição, clicar no
//! 3D a face certa e usar «Reancorar na seleção atual». O clique no 3D escolhia a peça clicada e
//! trocava a seleção, então o painel da restrição fechava justo quando o botão ficava ativo. Enquanto
//! a restrição selecionada tem referência perdida, o clique no 3D só escolhe o elemento e a
//! restrição continua no painel; depois de reancorar (estado OK) o clique volta a escolher a peça.

use fenkra_core::Id;
use fenkra_montagem::releitura::perdida;
use fenkra_montagem::Assembly;

/// A seleção é uma restrição com referência perdida, esperando o elemento novo?
pub fn manter_restricao(asm: &Assembly, sel: Option<Id>) -> bool {
    let Some(id) = sel else { return false };
    asm.mates.iter().find(|m| m.id == id).is_some_and(|m| perdida(&m.a) || perdida(&m.b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_montagem::releitura::Viva;
    use fenkra_montagem::{Geom, MateKind, MateRef};

    fn ref_de(comp: u64, perdida: bool) -> MateRef {
        MateRef { comp: Id(comp), geom: Geom::Plane { p: [0.; 3], n: [0., 0., 1.] }, pick: None, viva: Some(Viva::Face { chave: None, perdida }) }
    }

    fn montagem(perdida: bool) -> (Assembly, Id) {
        let mut a = Assembly::new("t");
        let id = a.add_mate(MateKind::Coincident { flip: false }, ref_de(1, perdida), ref_de(2, false));
        (a, id)
    }

    #[test]
    fn restricao_perdida_selecionada_continua_no_painel_ate_reancorar() {
        let (mut a, id) = montagem(true);
        assert!(manter_restricao(&a, Some(id)), "perdida: o clique no 3D não troca a seleção");
        // reancorado (estado OK): o clique volta a escolher a peça
        a.mates[0].a = ref_de(1, false);
        assert!(!manter_restricao(&a, Some(id)));
    }

    #[test]
    fn sem_restricao_perdida_o_clique_escolhe_a_peca() {
        let (a, id) = montagem(false);
        assert!(!manter_restricao(&a, Some(id)));
        assert!(!manter_restricao(&a, Some(Id(1))), "um componente não é restrição");
        assert!(!manter_restricao(&a, None));
    }
}
