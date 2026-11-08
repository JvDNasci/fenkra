// SPDX-License-Identifier: LGPL-2.1-or-later
//! Decisões da barra de estado que dependem da bancada (B430).

/// A barra do passo a passo (voltar ao início, um comando, avançar, fim) e o "Em trabalho" falam da
/// linha do tempo da peça. Na montagem e no desenho não há essa linha: os botões andavam numa peça
/// que nem está à vista e confundiam. Só aparecem com uma peça em edição.
pub fn mostra_historico(sem_peca_em_edicao: bool) -> bool {
    !sem_peca_em_edicao
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn historico_aparece_na_peca_e_some_na_montagem_e_no_desenho() {
        assert!(mostra_historico(false));
        assert!(!mostra_historico(true));
    }
}
