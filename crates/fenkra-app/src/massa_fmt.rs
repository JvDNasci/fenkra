// SPDX-License-Identifier: LGPL-2.1-or-later
//! Texto da massa na interface (B430): sem corpo não há massa, e nunca sai "-0,000 kg".
//!
//! O volume de um corpo vazio ou de uma soma com ruído numérico pode chegar como 0 negativo ou como
//! um valor abaixo de -1e-12. Formatar isso direto dava "-0,000 kg" na legenda. Aqui o valor é
//! forçado a zero ou mais, e a ausência de corpo vira um traço, não um zero que parece medido.

/// Massa em kg para mostrar, a partir do volume em mm³ e da densidade em g/cm³.
/// `tem_corpo` falso devolve "—".
pub fn texto_kg(volume_mm3: f64, densidade_g_cm3: f64, tem_corpo: bool) -> String {
    if !tem_corpo {
        return "—".into();
    }
    let kg = volume_mm3 * densidade_g_cm3 * 1e-6;
    // NaN, infinito e negativo (inclusive -0.0) viram zero
    let kg = if kg.is_finite() && kg > 0. { kg } else { 0. };
    let s = format!("{kg:.3} kg");
    if crate::i18n::english() {
        s
    } else {
        s.replace('.', ",")
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn sem_corpo_e_um_traco_e_nunca_menos_zero() {
        assert_eq!(texto_kg(0., 7.85, false), "—");
        assert_eq!(texto_kg(-0.0, 7.85, true), "0,000 kg");
        assert_eq!(texto_kg(-1e-9, 7.85, true), "0,000 kg");
        assert_eq!(texto_kg(f64::NAN, 7.85, true), "0,000 kg");
        assert!(!texto_kg(-3.0, 1.0, true).contains('-'));
    }

    #[test]
    fn massa_normal_usa_virgula_e_tres_casas() {
        // 1 dm³ de aço (1e6 mm³ a 7,85 g/cm³) = 7,85 kg
        assert_eq!(texto_kg(1.0e6, 7.85, true), "7,850 kg");
    }
}
