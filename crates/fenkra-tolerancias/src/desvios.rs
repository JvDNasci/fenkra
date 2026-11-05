// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desvios fundamentais e limites de furos (A–ZC) e eixos (a–zc), ISO 286-1.
//!
//! O eixo é a referência: a tabela guarda `es` para a–h (acima da linha zero) e `ei` para k–zc.
//! O furo deriva do eixo de mesma letra pela regra geral (espelho em torno da linha zero) e, em
//! K, M, N e P–ZC, pela regra especial que soma Δ = IT(n) − IT(n−1) quando o grau é fino. Os
//! desvios usam subfaixas de diâmetro mais finas que as do grau IT (25 contra 13).

use crate::it::{faixa_it, ItGrau};
use serde::{Deserialize, Serialize};

/// Letra da posição da zona. Maiúscula = furo, minúscula = eixo; aqui a letra é a mesma.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum Letra {
    A, B, C, CD, D, E, EF, F, FG, G, H, JS, J, K, M, N, P, R, S, T, U, V, X, Y, Z, ZA, ZB, ZC,
}

impl Letra {
    pub const TODAS: [Letra; 28] = [
        Letra::A, Letra::B, Letra::C, Letra::CD, Letra::D, Letra::E, Letra::EF, Letra::F, Letra::FG, Letra::G, Letra::H, Letra::JS, Letra::J, Letra::K,
        Letra::M, Letra::N, Letra::P, Letra::R, Letra::S, Letra::T, Letra::U, Letra::V, Letra::X, Letra::Y, Letra::Z, Letra::ZA, Letra::ZB, Letra::ZC,
    ];
    /// Texto em maiúsculas ("H", "JS", "ZC").
    pub fn texto(self) -> &'static str {
        const T: [&str; 28] = ["A", "B", "C", "CD", "D", "E", "EF", "F", "FG", "G", "H", "JS", "J", "K", "M", "N", "P", "R", "S", "T", "U", "V", "X", "Y", "Z", "ZA", "ZB", "ZC"];
        T[Letra::TODAS.iter().position(|l| *l == self).unwrap()]
    }
    pub fn parse(s: &str) -> Option<Letra> {
        let u = s.to_ascii_uppercase();
        Letra::TODAS.iter().copied().find(|l| l.texto() == u)
    }
}

/// Desvio superior e inferior em µm, relativos ao diâmetro nominal.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Desvios {
    pub sup_um: f64,
    pub inf_um: f64,
}

impl Desvios {
    pub fn sup_mm(&self) -> f64 {
        self.sup_um / 1000.
    }
    pub fn inf_mm(&self) -> f64 {
        self.inf_um / 1000.
    }
}

const N: i16 = i16::MIN;

/// Limites superiores (mm) das 25 subfaixas dos desvios fundamentais.
const SUBFAIXAS: [f64; 25] = [
    3., 6., 10., 14., 18., 24., 30., 40., 50., 65., 80., 100., 120., 140., 160., 180., 200., 225., 250., 280., 315., 355., 400., 450., 500.,
];

fn sub(d: f64) -> Option<usize> {
    if !(d > 0. && d <= 500.) {
        return None;
    }
    SUBFAIXAS.iter().position(|&h| d <= h)
}

// Eixo, a–g: `es` (µm, negativo). Colunas = SUBFAIXAS.
#[rustfmt::skip]
const A: [i16; 25] = [-270, -270, -280, -290, -290, -300, -300, -310, -320, -340, -360, -380, -410, -460, -520, -580, -660, -740, -820, -920, -1050, -1200, -1350, -1500, -1650];
#[rustfmt::skip]
const B: [i16; 25] = [-140, -140, -150, -150, -150, -160, -160, -170, -180, -190, -200, -220, -240, -260, -280, -310, -340, -380, -420, -480, -540, -600, -680, -760, -840];
#[rustfmt::skip]
const C: [i16; 25] = [-60, -70, -80, -95, -95, -110, -110, -120, -130, -140, -150, -170, -180, -200, -210, -230, -240, -260, -280, -300, -330, -360, -400, -440, -480];
#[rustfmt::skip]
const D: [i16; 25] = [-20, -30, -40, -50, -50, -65, -65, -80, -80, -100, -100, -120, -120, -145, -145, -145, -170, -170, -170, -190, -190, -210, -210, -230, -230];
#[rustfmt::skip]
const E: [i16; 25] = [-14, -20, -25, -32, -32, -40, -40, -50, -50, -60, -60, -72, -72, -85, -85, -85, -100, -100, -100, -110, -110, -125, -125, -135, -135];
#[rustfmt::skip]
const F: [i16; 25] = [-6, -10, -13, -16, -16, -20, -20, -25, -25, -30, -30, -36, -36, -43, -43, -43, -50, -50, -50, -56, -56, -62, -62, -68, -68];
#[rustfmt::skip]
const G: [i16; 25] = [-2, -4, -5, -6, -6, -7, -7, -9, -9, -10, -10, -12, -12, -14, -14, -14, -15, -15, -15, -17, -17, -18, -18, -20, -20];
// cd, ef, fg só existem até 10 mm.
#[rustfmt::skip]
const CD: [i16; 25] = [-34, -46, -56, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N];
#[rustfmt::skip]
const EF: [i16; 25] = [-10, -14, -18, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N];
#[rustfmt::skip]
const FG: [i16; 25] = [-4, -6, -8, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N];

// Eixo, k–zc: `ei` (µm, positivo). `K` vale para IT4–IT7; fora disso k = 0.
#[rustfmt::skip]
const K: [i16; 25] = [0, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 5, 5];
#[rustfmt::skip]
const M: [i16; 25] = [2, 4, 6, 7, 7, 8, 8, 9, 9, 11, 11, 13, 13, 15, 15, 15, 17, 17, 17, 20, 20, 21, 21, 23, 23];
#[rustfmt::skip]
const NN: [i16; 25] = [4, 8, 10, 12, 12, 15, 15, 17, 17, 20, 20, 23, 23, 27, 27, 27, 31, 31, 31, 34, 34, 37, 37, 40, 40];
#[rustfmt::skip]
const P: [i16; 25] = [6, 12, 15, 18, 18, 22, 22, 26, 26, 32, 32, 37, 37, 43, 43, 43, 50, 50, 50, 56, 56, 62, 62, 68, 68];
#[rustfmt::skip]
const R: [i16; 25] = [10, 15, 19, 23, 23, 28, 28, 34, 34, 41, 43, 51, 54, 63, 65, 68, 77, 80, 84, 94, 98, 108, 114, 126, 132];
#[rustfmt::skip]
const S: [i16; 25] = [14, 19, 23, 28, 28, 35, 35, 43, 43, 53, 59, 71, 79, 92, 100, 108, 122, 130, 140, 158, 170, 190, 208, 232, 252];
#[rustfmt::skip]
const T: [i16; 25] = [N, N, N, N, N, N, 41, 48, 54, 66, 75, 91, 104, 122, 134, 146, 166, 180, 196, 218, 240, 268, 294, 330, 360];
#[rustfmt::skip]
const U: [i16; 25] = [18, 23, 28, 33, 33, 41, 48, 60, 70, 87, 102, 124, 144, 170, 190, 210, 236, 258, 284, 315, 350, 390, 435, 490, 540];
#[rustfmt::skip]
const V: [i16; 25] = [N, N, N, N, 39, 47, 55, 68, 81, 102, 120, 146, 172, 202, 228, 252, 284, 310, 340, 385, 425, 475, 530, 595, 660];
#[rustfmt::skip]
const X: [i16; 25] = [20, 28, 34, 40, 45, 54, 64, 80, 97, 122, 146, 178, 210, 248, 280, 310, 350, 385, 425, 475, 525, 590, 660, 740, 820];
#[rustfmt::skip]
const Y: [i16; 25] = [N, N, N, N, N, 63, 75, 94, 114, 144, 174, 214, 254, 300, 340, 380, 425, 470, 520, 580, 650, 730, 820, 920, 1000];
#[rustfmt::skip]
const Z: [i16; 25] = [26, 35, 42, 50, 60, 73, 88, 112, 136, 172, 210, 258, 310, 365, 415, 465, 520, 575, 640, 710, 790, 900, 1000, 1100, 1250];
#[rustfmt::skip]
const ZA: [i16; 25] = [32, 42, 52, 64, 77, 98, 118, 148, 180, 226, 274, 335, 400, 470, 535, 600, 670, 740, 820, 920, 1000, 1150, 1300, 1450, 1600];
#[rustfmt::skip]
const ZB: [i16; 25] = [40, 50, 67, 90, 108, 136, 160, 200, 242, 300, 360, 445, 525, 620, 700, 780, 880, 960, 1050, 1200, 1300, 1500, 1650, 1850, 2100];
#[rustfmt::skip]
const ZC: [i16; 25] = [60, 80, 97, 130, 150, 188, 218, 274, 325, 405, 480, 585, 690, 800, 900, 1000, 1150, 1250, 1350, 1550, 1700, 1900, 2100, 2400, 2600];

// `ei` (negativo) do eixo j nas 13 faixas principais.
#[rustfmt::skip]
const J56: [i16; 13] = [-2, -2, -2, -3, -4, -5, -7, -9, -11, -13, -16, -18, -20];
#[rustfmt::skip]
const J7: [i16; 13] = [-4, -4, -5, -6, -8, -10, -12, -15, -18, -21, -26, -28, -32];
// `ES` (positivo) do furo J, grau 6, 7 e 8, nas 13 faixas principais.
#[rustfmt::skip]
const FJ6: [i16; 13] = [2, 5, 5, 6, 8, 10, 13, 16, 18, 22, 25, 29, 33];
#[rustfmt::skip]
const FJ7: [i16; 13] = [4, 6, 8, 10, 12, 14, 18, 22, 26, 30, 36, 39, 43];
#[rustfmt::skip]
const FJ8: [i16; 13] = [6, 10, 12, 15, 20, 24, 28, 34, 41, 47, 55, 60, 66];

fn tab(t: &[i16; 25], d: f64) -> Option<f64> {
    let v = t[sub(d)?];
    (v != N).then_some(v as f64)
}

/// Desvio fundamental tabelado do eixo da letra (`es` para A–H, `ei` para K–ZC) em µm. Não cobre
/// J, JS nem K (que dependem do grau).
fn tabela_eixo(l: Letra, d: f64) -> Option<f64> {
    let t = match l {
        Letra::A => &A, Letra::B => &B, Letra::C => &C, Letra::CD => &CD, Letra::D => &D, Letra::E => &E, Letra::EF => &EF,
        Letra::F => &F, Letra::FG => &FG, Letra::G => &G,
        Letra::M => &M, Letra::N => &NN, Letra::P => &P, Letra::R => &R, Letra::S => &S, Letra::T => &T, Letra::U => &U,
        Letra::V => &V, Letra::X => &X, Letra::Y => &Y, Letra::Z => &Z, Letra::ZA => &ZA, Letra::ZB => &ZB, Letra::ZC => &ZC,
        Letra::H => return Some(0.),
        Letra::JS | Letra::J | Letra::K => return None,
    };
    tab(t, d)
}

/// Meia zona do js/JS: IT/2; de IT7 a IT11 com IT ímpar usa (IT−1)/2 para ficar em µm inteiros.
fn meia_zona(g: ItGrau, d: f64) -> Option<f64> {
    let it = g.valor_um(d)?;
    if (7..=11).contains(&g.numero()) && it.fract() == 0. && (it as i64) % 2 == 1 {
        Some((it - 1.) / 2.)
    } else {
        Some(it / 2.)
    }
}

fn eh_abaixo_da_linha(l: Letra) -> bool {
    matches!(l, Letra::A | Letra::B | Letra::C | Letra::CD | Letra::D | Letra::E | Letra::EF | Letra::F | Letra::FG | Letra::G | Letra::H)
}

/// Limites do eixo `letra`+`grau` para o diâmetro nominal `d` (mm). `None` se a combinação não
/// existe na norma (j só em IT5–IT7, letra sem valor na faixa, `d` fora de 0–500 mm).
pub fn limites_eixo(l: Letra, g: ItGrau, d: f64) -> Option<Desvios> {
    let it = g.valor_um(d)?;
    match l {
        Letra::JS => {
            let h = meia_zona(g, d)?;
            Some(Desvios { sup_um: h, inf_um: -(it - h) })
        }
        Letra::J => {
            let f = faixa_it(d)?;
            let ei = match g.numero() {
                5 | 6 => J56[f],
                7 => J7[f],
                _ => return None,
            } as f64;
            Some(Desvios { sup_um: ei + it, inf_um: ei })
        }
        Letra::K => {
            let ei = if (4..=7).contains(&g.numero()) { K[sub(d)?] as f64 } else { 0. };
            Some(Desvios { sup_um: ei + it, inf_um: ei })
        }
        l if eh_abaixo_da_linha(l) => {
            let es = tabela_eixo(l, d)?;
            Some(Desvios { sup_um: es, inf_um: es - it })
        }
        l => {
            let ei = tabela_eixo(l, d)?;
            Some(Desvios { sup_um: ei + it, inf_um: ei })
        }
    }
}

/// Δ = IT(n) − IT(n−1) da regra especial dos furos; zero até 3 mm e fora de IT3–IT8.
pub fn delta_um(g: ItGrau, d: f64) -> f64 {
    if d <= 3. || !(3..=8).contains(&g.numero()) {
        return 0.;
    }
    match (g.valor_um(d), g.anterior().and_then(|a| a.valor_um(d))) {
        (Some(a), Some(b)) => a - b,
        _ => 0.,
    }
}

/// Limites do furo `letra`+`grau` para o diâmetro nominal `d` (mm).
pub fn limites_furo(l: Letra, g: ItGrau, d: f64) -> Option<Desvios> {
    let it = g.valor_um(d)?;
    let n = g.numero();
    match l {
        Letra::JS => {
            let h = meia_zona(g, d)?;
            Some(Desvios { sup_um: h, inf_um: -(it - h) })
        }
        Letra::J => {
            let f = faixa_it(d)?;
            let es = match n {
                6 => FJ6[f],
                7 => FJ7[f],
                8 => FJ8[f],
                _ => return None,
            } as f64;
            Some(Desvios { sup_um: es, inf_um: es - it })
        }
        l if eh_abaixo_da_linha(l) => {
            let ei = -tabela_eixo(l, d)?;
            Some(Desvios { sup_um: ei + it, inf_um: ei })
        }
        Letra::K => {
            let es = if n <= 8 { -(K[sub(d)?] as f64) + delta_um(g, d) } else { 0. };
            Some(Desvios { sup_um: es, inf_um: es - it })
        }
        Letra::M => {
            let ei = tabela_eixo(Letra::M, d)?;
            let es = if n <= 8 { -ei + delta_um(g, d) } else { -ei };
            Some(Desvios { sup_um: es, inf_um: es - it })
        }
        Letra::N => {
            let ei = tabela_eixo(Letra::N, d)?;
            let es = if n <= 8 { -ei + delta_um(g, d) } else if d <= 3. { -ei } else { 0. };
            Some(Desvios { sup_um: es, inf_um: es - it })
        }
        l => {
            // P–ZC
            let ei = tabela_eixo(l, d)?;
            let es = if n <= 7 { -ei + delta_um(g, d) } else { -ei };
            Some(Desvios { sup_um: es, inf_um: es - it })
        }
    }
}
