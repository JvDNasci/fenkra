// SPDX-License-Identifier: LGPL-2.1-or-later
//! B408: o painel da peça (nome, PN, material, massa) abre ao escolher a peça raiz da árvore. Antes,
//! o clique marcava `Sel::Part`, mas o painel de comando só nascia com sessão, sketch, parâmetros ou
//! anotação, e o campo Material não tinha caminho nenhum pela interface.

use super::Sel;

impl Sel {
    /// A seleção na árvore que, sozinha, abre o painel de comando com o seu editor. `minibar` diz se a
    /// mini barra da anotação já está mostrando a anotação escolhida (aí o painel não repete).
    pub fn abre_painel(&self, minibar: bool) -> bool {
        match self {
            Sel::Part | Sel::Params => true,
            Sel::Pmi(_) => !minibar,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Sel;

    #[test]
    fn peca_raiz_abre_o_painel_com_o_material() {
        // falhava antes: só parâmetros e anotação abriam o painel
        assert!(Sel::Part.abre_painel(false));
        assert!(Sel::Params.abre_painel(false));
        assert!(Sel::Pmi(fenkra_core::Id(3)).abre_painel(false));
        assert!(!Sel::Pmi(fenkra_core::Id(3)).abre_painel(true));
        assert!(!Sel::None.abre_painel(false));
    }

    #[test]
    fn material_fora_da_tabela_tem_aviso_e_o_da_tabela_nao() {
        use fenkra_core::materiais::{aviso_fora_da_tabela, densidade};
        assert!(aviso_fora_da_tabela("Aço AISI 4130").is_none());
        assert!(aviso_fora_da_tabela("Unobtânio").is_some());
        assert!((densidade("Aço AISI 4130") - 7.85).abs() < 0.1);
        assert!(densidade("Borracha natural") < 1.5);
    }
}
