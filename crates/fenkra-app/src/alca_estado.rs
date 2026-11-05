// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estados visuais de toda alça no 3D (B20, contrato de comando): repouso, pairando, ativa,
//! travada por parâmetro e erro. A regra é uma só para a seta, o arco e o que vier depois, e as
//! cores saem do tema (nunca fixas), sempre distintas do fundo.

use crate::i18n::tr;
use eframe::egui::{Color32, Visuals};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EstadoAlca {
    /// traço fino, na cor de destaque do tema
    Repouso,
    /// o mouse está sobre a pega: traço mais grosso e dica "puxar para alterar"
    Pairando,
    /// sendo arrastada: cor quente, traço grosso e medida grande
    Ativa,
    /// valor ligado a um parâmetro: não se arrasta, só se edita a expressão
    Travada,
    /// o comando está com erro: a alça mostra a causa em vermelho
    Erro,
}

impl EstadoAlca {
    /// Prioridade: erro e trava vencem o mouse (uma alça travada não "acende" ao passar por cima).
    pub fn de(pairando: bool, arrastando: bool, travada: bool, erro: bool) -> Self {
        if erro {
            EstadoAlca::Erro
        } else if travada {
            EstadoAlca::Travada
        } else if arrastando {
            EstadoAlca::Ativa
        } else if pairando {
            EstadoAlca::Pairando
        } else {
            EstadoAlca::Repouso
        }
    }

    /// Espessura do traço em pixels: pairando é 1,5 vezes o repouso, e a ativa o dobro.
    pub fn espessura(self) -> f32 {
        match self {
            EstadoAlca::Repouso | EstadoAlca::Travada => 1.5,
            EstadoAlca::Pairando | EstadoAlca::Erro => 2.25,
            EstadoAlca::Ativa => 3.,
        }
    }

    pub fn cor(self, v: &Visuals) -> Color32 {
        match self {
            EstadoAlca::Repouso => crate::theme::accent(v),
            EstadoAlca::Pairando => crate::theme::accent(v).lerp_to_gamma(v.strong_text_color(), 0.35),
            EstadoAlca::Ativa => crate::theme::st_warn(),
            EstadoAlca::Travada => v.weak_text_color(),
            EstadoAlca::Erro => crate::theme::st_err(),
        }
    }

    /// Tamanho da fonte do rótulo: a medida cresce enquanto a alça está ativa.
    pub fn fonte(self) -> f32 {
        if self == EstadoAlca::Ativa { 16. } else { 13. }
    }

    /// Dica curta junto da pega, quando o estado tem o que dizer.
    pub fn dica(self) -> Option<&'static str> {
        match self {
            EstadoAlca::Pairando => Some("puxar para alterar"),
            EstadoAlca::Travada => Some("travada por parâmetro: edite a expressão"),
            EstadoAlca::Erro => Some("o comando tem erro: veja a causa no painel"),
            EstadoAlca::Repouso | EstadoAlca::Ativa => None,
        }
    }

    pub fn dica_traduzida(self) -> Option<&'static str> {
        self.dica().map(tr)
    }
}

/// Textos do contrato (inglês), somados à tabela de `i18n.rs`.
pub static TEXTOS: &[(&str, &str)] = &[
    ("puxar para alterar", "drag to change"),
    ("travada por parâmetro: edite a expressão", "locked by a parameter: edit the expression"),
    ("o comando tem erro: veja a causa no painel", "the command has an error: see the cause in the panel"),
];

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn prioridade_dos_estados() {
        use EstadoAlca::*;
        assert_eq!(EstadoAlca::de(false, false, false, false), Repouso);
        assert_eq!(EstadoAlca::de(true, false, false, false), Pairando);
        assert_eq!(EstadoAlca::de(true, true, false, false), Ativa, "arrastar vence passar por cima");
        assert_eq!(EstadoAlca::de(true, true, true, false), Travada, "travada não acende com o mouse");
        assert_eq!(EstadoAlca::de(true, true, true, true), Erro, "erro vence tudo");
    }

    #[test]
    fn espessura_cresce_do_repouso_para_o_ativo() {
        use EstadoAlca::*;
        assert!((Pairando.espessura() / Repouso.espessura() - 1.5).abs() < 1e-6, "pairando é 1,5 vezes o repouso");
        assert!(Ativa.espessura() > Pairando.espessura());
        assert!(Ativa.fonte() > Repouso.fonte(), "a medida fica grande ao arrastar");
    }

    #[test]
    fn cada_estado_tem_cor_propria_e_distinta_do_fundo() {
        use EstadoAlca::*;
        for v in [Visuals::dark(), Visuals::light()] {
            let cores: Vec<Color32> = [Repouso, Pairando, Ativa, Travada, Erro].iter().map(|e| e.cor(&v)).collect();
            for (i, a) in cores.iter().enumerate() {
                assert_ne!(*a, v.panel_fill, "estado {i} some no fundo");
                for b in &cores[i + 1..] {
                    assert_ne!(a, b, "dois estados com a mesma cor");
                }
            }
        }
    }

    #[test]
    fn so_os_estados_com_o_que_dizer_trazem_dica() {
        use EstadoAlca::*;
        assert!(Repouso.dica().is_none() && Ativa.dica().is_none());
        assert!(Pairando.dica().is_some() && Travada.dica().is_some() && Erro.dica().is_some());
        assert!(crate::i18n::tem_traducao(Pairando.dica().unwrap()), "dica sem tradução");
    }
}
