// SPDX-License-Identifier: LGPL-2.1-or-later
//! Tolerâncias normativas como dados e funções puras, sem dependência do resto do Fenkra.
//!
//! * `it`: graus de tolerância padrão IT01–IT18 por faixa de diâmetro nominal (ISO 286-1);
//! * `desvios`: desvios fundamentais de furos (A–ZC) e eixos (a–zc) e limites resultantes;
//! * `ajustes`: classe tal como "H7"/"g6", ajuste "H7/g6", folgas e tipo (folga, incerto,
//!   interferência) e os limites no formato que o AP242 (`limits_and_fits`) pede;
//! * `geral`: tolerâncias gerais da ISO 2768-1 (f, m, c, v) e 2768-2 (H, K, L).
//!
//! Unidade de saída: micrômetros (`_um`), com `mm()` onde convém. Faixas cobertas: 0 < D ≤ 500 mm.
//! `VERSAO_TABELAS` muda sempre que um valor das tabelas muda, para o documento saber com que
//! revisão dos dados uma cota foi calculada.

pub mod ajustes;
pub mod desvios;
pub mod geral;
pub mod it;

pub use ajustes::{Ajuste, ClasseTol, LimitesAp242, TipoAjuste};
pub use desvios::{Desvios, Letra};
pub use geral::{ClasseForma, ClasseGeral};
pub use it::ItGrau;

/// Revisão dos dados tabelados (ISO 286-1 faixas até 500 mm; ISO 2768-1 e -2 de 1989).
pub const VERSAO_TABELAS: u32 = 1;
