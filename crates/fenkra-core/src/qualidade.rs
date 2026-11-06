// SPDX-License-Identifier: LGPL-2.1-or-later
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

/// Malha só para a tela (B346): o que a vista pede, além da malha de trabalho. A malha de trabalho
/// segue a qualidade; esta segue a câmera, porque o que importa na tela é o erro em pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Polimento {
    /// desvio linear (mm)
    pub desvio_mm: f64,
    /// ângulo máximo entre normais vizinhas (graus)
    pub angulo_graus: f64,
    /// divide também onde a curvatura muda, para o mapa de curvatura
    pub curvatura: bool,
    /// teto de triângulos da peça inteira
    pub teto_triangulos: usize,
}

impl Polimento {
    /// Erro de silhueta aceito na tela: abaixo de meio pixel o polígono some no anti-serrilhamento.
    pub const PIXELS: f64 = 0.5;
    /// Ângulo das análises de reflexo. Com a normal exata nos vértices, o que entorta a listra é o
    /// pixel cair na corda e não na superfície: o erro cresce com o quadrado do ângulo e passa de 5%
    /// do período da listra acima de 5° (medido no cilindro, ver ADR 0069).
    pub const ANGULO_ANALISE: f64 = 5.;
    /// Ângulo do sombreado: sem listra, o olho só percebe a silhueta e o brilho.
    pub const ANGULO_SOMBREADO: f64 = 10.;
    /// Tetos de triângulos da peça: `TETO_LEVE` é o do modo leve.
    pub const TETO: usize = 1_500_000;
    pub const TETO_LEVE: usize = 300_000;

    /// O que pedir para uma vista de `px_por_mm`. O desvio anda em potências de 2: zoom pequeno não
    /// refaz a malha. `analise` é zebra, isofotas ou mapa; no modo leve só a análise pede malha.
    pub fn para_vista(px_por_mm: f64, analise: bool, curvatura: bool, leve: bool) -> Option<Polimento> {
        if !px_por_mm.is_finite() || px_por_mm <= 0. || (leve && !analise) {
            return None;
        }
        let pixels = if leve { 3. * Self::PIXELS } else { Self::PIXELS };
        let desvio = (pixels / px_por_mm).clamp(DESVIO_MIN, DESVIO_MAX);
        Some(Polimento {
            desvio_mm: 2f64.powf(desvio.log2().floor()).max(DESVIO_MIN),
            angulo_graus: if analise { Self::ANGULO_ANALISE } else { Self::ANGULO_SOMBREADO },
            curvatura,
            teto_triangulos: if leve { Self::TETO_LEVE } else { Self::TETO },
        })
    }

    /// Desvio e ângulo que cabem no teto, partindo de uma malha com `triangulos_curvos` nas faces
    /// curvas feita com `desvio` e `angulo` (graus). Numa superfície de dupla curvatura os triângulos
    /// crescem com o inverso do desvio e com o inverso do quadrado do ângulo. `folga` é a fração do
    /// teto que este corpo pode usar.
    pub fn dentro_do_teto(&self, triangulos_curvos: usize, desvio: f64, angulo: f64, folga: f64) -> (f64, f64) {
        let previsto = triangulos_curvos as f64 * (desvio / self.desvio_mm).max((angulo / self.angulo_graus).powi(2)).max(1.);
        let teto = self.teto_triangulos as f64 * folga.clamp(0., 1.);
        if previsto <= teto || triangulos_curvos == 0 {
            return (self.desvio_mm.min(desvio), self.angulo_graus.min(angulo));
        }
        let escala = previsto / teto.max(1.);
        ((self.desvio_mm * escala).min(desvio), (self.angulo_graus * escala.sqrt()).min(angulo))
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
    #[test]
    fn polimento_pede_meio_pixel_e_anda_em_degraus() {
        let p = Polimento::para_vista(10., false, false, false).unwrap();
        assert_eq!(p.desvio_mm, 2f64.powi(-5), "0,05 mm cai no degrau de baixo");
        assert_eq!(Polimento::para_vista(11., false, false, false).unwrap().desvio_mm, p.desvio_mm, "zoom de 10% não refaz");
        assert!(Polimento::para_vista(25., false, false, false).unwrap().desvio_mm < p.desvio_mm);
        assert_eq!(p.angulo_graus, Polimento::ANGULO_SOMBREADO);
        assert_eq!(Polimento::para_vista(10., true, false, false).unwrap().angulo_graus, Polimento::ANGULO_ANALISE);
    }

    #[test]
    fn modo_leve_so_pede_malha_na_analise_e_com_teto_menor() {
        assert_eq!(Polimento::para_vista(10., false, false, true), None);
        let p = Polimento::para_vista(10., true, true, true).unwrap();
        assert_eq!(p.teto_triangulos, Polimento::TETO_LEVE);
        assert!(p.desvio_mm > Polimento::para_vista(10., true, true, false).unwrap().desvio_mm);
    }

    #[test]
    fn vista_impossivel_nao_pede_malha() {
        assert_eq!(Polimento::para_vista(0., true, false, false), None);
        assert_eq!(Polimento::para_vista(f64::NAN, true, false, false), None);
        assert!(Polimento::para_vista(1e9, true, false, false).unwrap().desvio_mm >= DESVIO_MIN);
    }

    #[test]
    fn teto_alarga_o_pedido_sem_passar_da_malha_de_trabalho() {
        let p = Polimento { desvio_mm: 0.01, angulo_graus: 5., curvatura: false, teto_triangulos: 1000 };
        assert_eq!(p.dentro_do_teto(10, 0.1, 20., 1.), (0.01, 5.), "cabe: vale o pedido");
        let (d, a) = p.dentro_do_teto(900, 0.1, 20., 1.);
        assert!(d > 0.01 && a > 5., "{d} {a}");
        assert!(d <= 0.1 && a <= 20., "nunca mais grosso que a malha de trabalho");
        assert_eq!(p.dentro_do_teto(0, 0.1, 20., 1.), (0.01, 5.), "peça plana não cresce");
    }

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
