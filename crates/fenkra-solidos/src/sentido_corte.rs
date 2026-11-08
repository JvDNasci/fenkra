// SPDX-License-Identifier: LGPL-2.1-or-later
//! B392 (K-14 do kart): sentido do recorte por extrusão.
//!
//! O recorte entra no material, contra a normal do plano do sketch; a adição sai a favor dela. Com
//! "inverter" desligado, os dois vão para lados opostos, e quem pensa como na adição liga "inverter"
//! no recorte e erra o lado. A regra fica (documentos gravados dependem dela), mas:
//! - quando o recorte não remove nada, o cálculo confere o outro lado e diz com certeza o que fazer;
//! - o app, ao criar um recorte novo, usa essa conferência para virar o sentido sozinho uma vez.

/// O recorte não removeu nada, e do outro lado do plano há material: ligar "inverter" resolve.
pub const LIGUE_INVERTER: &str = "o corte não removeu material: a peça está do outro lado do plano do sketch. Ligue 'inverter' (o recorte entra contra a normal do plano, a adição sai a favor dela)";
/// Idem, com "inverter" já ligado.
pub const DESLIGUE_INVERTER: &str = "o corte não removeu material: a peça está do outro lado do plano do sketch. Desligue 'inverter' (o recorte entra contra a normal do plano, a adição sai a favor dela)";
/// Nenhum dos dois lados alcança o corpo: o problema é a distância ou a posição do perfil.
pub const NAO_ALCANCA: &str = "o corte não removeu material: a ferramenta não alcança o corpo em nenhum dos dois sentidos — confira a distância e a posição do perfil";

/// Aviso do recorte que não removeu nada. `outro_lado`: virar o sentido removeria material.
pub fn aviso(outro_lado: bool, inverter: bool) -> &'static str {
    match (outro_lado, inverter) {
        (true, false) => LIGUE_INVERTER,
        (true, true) => DESLIGUE_INVERTER,
        (false, _) => NAO_ALCANCA,
    }
}

/// O status diz que o recorte foi para o lado sem material e que virar o sentido resolve.
pub fn virar_resolve(m: &str) -> bool {
    m.starts_with(LIGUE_INVERTER) || m.starts_with(DESLIGUE_INVERTER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aviso_diz_o_que_fazer_conforme_o_inverter() {
        assert_eq!(aviso(true, false), LIGUE_INVERTER);
        assert_eq!(aviso(true, true), DESLIGUE_INVERTER);
        assert_eq!(aviso(false, true), NAO_ALCANCA);
        assert!(virar_resolve(LIGUE_INVERTER) && virar_resolve(DESLIGUE_INVERTER) && !virar_resolve(NAO_ALCANCA));
    }
}
