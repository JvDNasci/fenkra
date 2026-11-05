//! Qualidade da malha de exibição: um só lugar decide o quão fina é a tesselação das peças,
//! montagens e referências (o modelo B-Rep não muda, só a malha que se desenha).

use serde::{Deserialize, Serialize};

/// Menor desvio linear aceito (mm): abaixo disso a malha só engorda sem ganho visível.
pub const DESVIO_MIN: f64 = 0.002;
/// Maior desvio linear aceito (mm): peças enormes ainda mostram curvas redondas.
pub const DESVIO_MAX: f64 = 2.0;

/// Quão fina é a malha: desvio máximo da superfície (fração da diagonal da caixa envolvente)
/// e ângulo máximo entre normais de triângulos vizinhos.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct QualidadeMalha {
    pub desvio_rel: f64,
    pub angulo_graus: f64,
}

impl QualidadeMalha {
    pub const RASCUNHO: QualidadeMalha = QualidadeMalha { desvio_rel: 0.004, angulo_graus: 30. };
    pub const NORMAL: QualidadeMalha = QualidadeMalha { desvio_rel: 0.0015, angulo_graus: 20. };
    pub const ALTA: QualidadeMalha = QualidadeMalha { desvio_rel: 0.0005, angulo_graus: 12. };

    /// Limites aceitos para valores digitados (Personalizada).
    pub const REL_MIN: f64 = 0.0001;
    pub const REL_MAX: f64 = 0.02;
    pub const ANG_MIN: f64 = 5.;
    pub const ANG_MAX: f64 = 45.;

    /// Mesmos valores com os campos dentro dos limites (e sem NaN).
    pub fn saneada(self) -> Self {
        let lim = |v: f64, lo: f64, hi: f64, padrao: f64| if v.is_finite() { v.clamp(lo, hi) } else { padrao };
        QualidadeMalha {
            desvio_rel: lim(self.desvio_rel, Self::REL_MIN, Self::REL_MAX, Self::NORMAL.desvio_rel),
            angulo_graus: lim(self.angulo_graus, Self::ANG_MIN, Self::ANG_MAX, Self::NORMAL.angulo_graus),
        }
    }

    /// Desvio linear (mm) para uma peça de diagonal `diagonal`, entre o piso e o teto.
    pub fn desvio(&self, diagonal: f64) -> f64 {
        desvio_com_fator(diagonal, self.desvio_rel)
    }

    pub fn angulo_rad(&self) -> f64 {
        self.angulo_graus.to_radians()
    }
}

impl Default for QualidadeMalha {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Desvio linear (mm) com um fator próprio (desenho e PMI usam fatores mais grossos), com o
/// mesmo piso e teto da qualidade da malha. Diagonal inválida vale como uma peça de 100 mm.
pub fn desvio_com_fator(diagonal: f64, fator: f64) -> f64 {
    let d = if diagonal.is_finite() && diagonal > 0. { diagonal } else { 100. };
    (d * fator).clamp(DESVIO_MIN, DESVIO_MAX)
}

/// Escolha da qualidade nas configurações: um dos três ajustes prontos ou valores livres.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetMalha {
    Rascunho,
    #[default]
    Normal,
    Alta,
    Personalizada,
}

impl PresetMalha {
    pub const TODOS: [PresetMalha; 4] = [PresetMalha::Rascunho, PresetMalha::Normal, PresetMalha::Alta, PresetMalha::Personalizada];

    /// Qualidade do preset; `livre` só vale na Personalizada.
    pub fn qualidade(self, livre: QualidadeMalha) -> QualidadeMalha {
        match self {
            PresetMalha::Rascunho => QualidadeMalha::RASCUNHO,
            PresetMalha::Normal => QualidadeMalha::NORMAL,
            PresetMalha::Alta => QualidadeMalha::ALTA,
            PresetMalha::Personalizada => livre.saneada(),
        }
    }
}

/// Ajuste pronto da malha do STL. É mais fino que a exibição porque o arquivo vai para fabricação:
/// a faceta que a tela disfarça com normais suaves aparece na peça impressa.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetStl {
    Rascunho,
    #[default]
    Normal,
    Fino,
}

impl PresetStl {
    pub const TODOS: [PresetStl; 3] = [PresetStl::Rascunho, PresetStl::Normal, PresetStl::Fino];

    pub fn qualidade(self) -> QualidadeMalha {
        match self {
            PresetStl::Rascunho => QualidadeMalha { desvio_rel: 0.001, angulo_graus: 20. },
            PresetStl::Normal => QualidadeMalha { desvio_rel: 0.0003, angulo_graus: 12. },
            PresetStl::Fino => QualidadeMalha { desvio_rel: 0.0001, angulo_graus: 6. },
        }
    }

    pub fn nome(self) -> &'static str {
        match self {
            PresetStl::Rascunho => "Rascunho",
            PresetStl::Normal => "Normal",
            PresetStl::Fino => "Fino",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_do_stl_ficam_do_grosso_ao_fino() {
        let [r, n, f] = PresetStl::TODOS.map(|p| p.qualidade());
        assert!(r.desvio(200.) > n.desvio(200.) && n.desvio(200.) > f.desvio(200.));
        assert!(r.angulo_graus > n.angulo_graus && n.angulo_graus > f.angulo_graus);
        assert!(n.desvio(200.) < QualidadeMalha::ALTA.desvio(200.), "o Normal do arquivo é mais fino que a tela");
    }

    #[test]
    fn presets_ficam_do_grosso_ao_fino() {
        let (r, n, a) = (QualidadeMalha::RASCUNHO, QualidadeMalha::NORMAL, QualidadeMalha::ALTA);
        assert!(r.desvio(200.) > n.desvio(200.) && n.desvio(200.) > a.desvio(200.));
        assert!(r.angulo_graus > n.angulo_graus && n.angulo_graus > a.angulo_graus);
        assert_eq!(QualidadeMalha::default(), n);
        assert_eq!((r.desvio_rel, r.angulo_graus), (0.004, 30.));
        assert_eq!((a.desvio_rel, a.angulo_graus), (0.0005, 12.));
    }

    #[test]
    fn desvio_respeita_piso_e_teto() {
        let q = QualidadeMalha::NORMAL;
        assert_eq!(q.desvio(1.), DESVIO_MIN, "peça minúscula cai no piso");
        assert_eq!(q.desvio(1e6), DESVIO_MAX, "peça enorme cai no teto");
        assert!((q.desvio(200.) - 0.3).abs() < 1e-12);
        assert_eq!(q.desvio(f64::NAN), q.desvio(100.));
        assert_eq!(desvio_com_fator(1e6, 0.01), DESVIO_MAX);
    }

    #[test]
    fn personalizada_e_saneada() {
        let livre = QualidadeMalha { desvio_rel: 5., angulo_graus: f64::NAN };
        let q = PresetMalha::Personalizada.qualidade(livre);
        assert_eq!(q.desvio_rel, QualidadeMalha::REL_MAX);
        assert_eq!(q.angulo_graus, QualidadeMalha::NORMAL.angulo_graus);
        assert_eq!(PresetMalha::Alta.qualidade(livre), QualidadeMalha::ALTA);
    }
}
