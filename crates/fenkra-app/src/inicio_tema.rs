// SPDX-License-Identifier: LGPL-2.1-or-later
//! Tela inicial e carregamento (B335): paleta derivada só do tema em uso. O fundo e o movimento
//! estão em `tela_inicial.rs` (B455).
//!
//! A paleta sai de `ThemeDef` (painel, janela, texto, destaque, peça), nunca de cor fixa, para a
//! tela inicial ser a mesma do restante do programa em qualquer tema, inclusive os do usuário.

use eframe::egui::Color32;

use crate::theme::ThemeDef;

/// Taxa do movimento contínuo: 30 quadros por segundo bastam para deriva lenta e poupam a GPU.
pub const QUADRO_MS: u64 = 33;

/// Cores da tela inicial, todas derivadas do tema.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paleta {
    pub escuro: bool,
    pub topo: Color32,
    pub base: Color32,
    pub texto: Color32,
    pub destaque: Color32,
    pub cartao: Color32,
    /// degradê atrás da miniatura do exemplo
    pub mini_topo: Color32,
    pub mini_base: Color32,
    pub peca: [u8; 3],
    pub aresta: [u8; 3],
}

fn rgb(c: [u8; 3]) -> Color32 {
    crate::theme::cor_rgb(c)
}

fn mistura(a: [u8; 3], b: [u8; 3], k: f32) -> [u8; 3] {
    let m = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * k).round() as u8;
    [m(0), m(1), m(2)]
}

impl Paleta {
    pub fn de(def: &ThemeDef) -> Paleta {
        // fundo: o painel do tema puxado de leve para o destaque (cima) e para o texto (baixo),
        // assim o texto do tema sempre tem contraste sobre ele
        let topo = mistura(def.panel, def.accent, 0.08);
        let base = mistura(def.panel, def.text, if def.dark { 0.07 } else { 0.10 });
        let peca = def.peca.unwrap_or(crate::theme::PECA_PADRAO);
        let aresta = def.aresta.unwrap_or_else(|| mistura(peca, [0; 3], 0.8));
        // miniatura: parte da janela; se a peça não se destacar, o fundo se afasta dela
        let (mut a, mut b) = (mistura(def.window, def.text, 0.07), mistura(def.window, def.text, 0.18));
        // confere as duas pontas do degradê, não só o meio
        let pior = crate::acessibilidade::contraste(peca, a).min(crate::acessibilidade::contraste(peca, b));
        if pior < 1.6 {
            let polo = if crate::acessibilidade::contraste(peca, [0; 3]) > crate::acessibilidade::contraste(peca, [255; 3]) { [0; 3] } else { [255; 3] };
            // afasta o fundo da peça, em passos, até o contraste mínimo ser atingido nas duas pontas
            let mut k = 0.30;
            loop {
                a = mistura(def.window, polo, k);
                b = mistura(def.window, polo, (k + 0.18).min(1.));
                if k >= 0.95 || (crate::acessibilidade::contraste(peca, a).min(crate::acessibilidade::contraste(peca, b)) >= 1.6) {
                    break;
                }
                k += 0.05;
            }
        }
        Paleta { escuro: def.dark, topo: rgb(topo), base: rgb(base), texto: rgb(def.text), destaque: rgb(def.accent), cartao: rgb(def.window), mini_topo: rgb(a), mini_base: rgb(b), peca, aresta }
    }

    /// Paleta do tema em vigor (o mesmo que as preferências aplicaram).
    pub fn atual() -> Paleta {
        Paleta::de(&crate::theme::current().unwrap_or_else(crate::theme::default_def))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn paleta_vem_do_tema_em_todos_os_temas() {
        for def in crate::theme::builtin() {
            let p = Paleta::de(&def);
            assert_eq!(p.escuro, def.dark, "{}", def.name);
            assert_eq!(p.texto, rgb(def.text), "{}", def.name);
            assert_eq!(p.destaque, rgb(def.accent), "{}", def.name);
            assert_eq!(p.cartao, rgb(def.window), "{}", def.name);
            // o texto do tema continua legível sobre o fundo da tela inicial
            for fundo in [p.topo, p.base] {
                let c = crate::acessibilidade::contraste([p.texto.r(), p.texto.g(), p.texto.b()], [fundo.r(), fundo.g(), fundo.b()]);
                assert!(c >= 4.5, "{}: contraste {c}", def.name);
            }
        }
    }

    #[test]
    fn peca_sempre_se_destaca_da_miniatura() {
        for def in crate::theme::builtin() {
            let p = Paleta::de(&def);
            for fundo in [p.mini_topo, p.mini_base] {
                let c = crate::acessibilidade::contraste(p.peca, [fundo.r(), fundo.g(), fundo.b()]);
                assert!(c >= 1.3, "{}: a peça some no fundo ({c})", def.name);
            }
        }
    }

    #[test]
    fn temas_diferentes_dao_fundos_diferentes() {
        let t = crate::theme::builtin();
        let escuro = t.iter().find(|d| d.dark).unwrap();
        let claro = t.iter().find(|d| !d.dark).unwrap();
        assert_ne!(Paleta::de(escuro).topo, Paleta::de(claro).topo);
        assert!(Paleta::de(escuro).topo.r() < Paleta::de(claro).topo.r());
    }
}
