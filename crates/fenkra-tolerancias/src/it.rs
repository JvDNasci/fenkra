//! Graus de tolerância padrão IT01–IT18 (ISO 286-1).

use serde::{Deserialize, Serialize};

/// Limites superiores (mm) das 13 faixas principais de diâmetro nominal; a primeira vai de 0 a 3.
pub const FAIXAS_IT: [f64; 13] = [3., 6., 10., 18., 30., 50., 80., 120., 180., 250., 315., 400., 500.];

/// Valores em µm: linha = grau (IT01, IT0, IT1 … IT18), coluna = faixa principal.
#[rustfmt::skip]
const IT_UM: [[f64; 13]; 20] = [
    [0.3, 0.4, 0.4, 0.5, 0.6, 0.6, 0.8, 1., 1.2, 2., 2.5, 3., 4.],
    [0.5, 0.6, 0.6, 0.8, 1., 1., 1.2, 1.5, 2., 3., 4., 5., 6.],
    [0.8, 1., 1., 1.2, 1.5, 1.5, 2., 2.5, 3.5, 4.5, 6., 7., 8.],
    [1.2, 1.5, 1.5, 2., 2.5, 2.5, 3., 4., 5., 7., 8., 9., 10.],
    [2., 2.5, 2.5, 3., 4., 4., 5., 6., 8., 10., 12., 13., 15.],
    [3., 4., 4., 5., 6., 7., 8., 10., 12., 14., 16., 18., 20.],
    [4., 5., 6., 8., 9., 11., 13., 15., 18., 20., 23., 25., 27.],
    [6., 8., 9., 11., 13., 16., 19., 22., 25., 29., 32., 36., 40.],
    [10., 12., 15., 18., 21., 25., 30., 35., 40., 46., 52., 57., 63.],
    [14., 18., 22., 27., 33., 39., 46., 54., 63., 72., 81., 89., 97.],
    [25., 30., 36., 43., 52., 62., 74., 87., 100., 115., 130., 140., 155.],
    [40., 48., 58., 70., 84., 100., 120., 140., 160., 185., 210., 230., 250.],
    [60., 75., 90., 110., 130., 160., 190., 220., 250., 290., 320., 360., 400.],
    [100., 120., 150., 180., 210., 250., 300., 350., 400., 460., 520., 570., 630.],
    [140., 180., 220., 270., 330., 390., 460., 540., 630., 720., 810., 890., 970.],
    [250., 300., 360., 430., 520., 620., 740., 870., 1000., 1150., 1300., 1400., 1550.],
    [400., 480., 580., 700., 840., 1000., 1200., 1400., 1600., 1850., 2100., 2300., 2500.],
    [600., 750., 900., 1100., 1300., 1600., 1900., 2200., 2500., 2900., 3200., 3600., 4000.],
    [1000., 1200., 1500., 1800., 2100., 2500., 3000., 3500., 4000., 4600., 5200., 5700., 6300.],
    [1400., 1800., 2200., 2700., 3300., 3900., 4600., 5400., 6300., 7200., 8100., 8900., 9700.],
];

/// Grau IT. Índice 0 = IT01, 1 = IT0, 2 = IT1 … 19 = IT18.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Serialize, Deserialize)]
pub struct ItGrau(u8);

impl ItGrau {
    pub const IT01: ItGrau = ItGrau(0);
    pub const IT0: ItGrau = ItGrau(1);

    /// `n` de 1 a 18 (IT1…IT18).
    pub fn novo(n: u8) -> Option<ItGrau> {
        (1..=18).contains(&n).then_some(ItGrau(n + 1))
    }
    /// Número do grau; IT01 e IT0 devolvem -1 e 0.
    pub fn numero(self) -> i32 {
        self.0 as i32 - 1
    }
    /// "IT7", "IT01"…
    pub fn nome(self) -> String {
        match self.0 {
            0 => "IT01".into(),
            1 => "IT0".into(),
            n => format!("IT{}", n - 1),
        }
    }
    /// Lê "IT7", "it01" ou só "7".
    pub fn parse(s: &str) -> Option<ItGrau> {
        let t = s.trim().to_ascii_uppercase();
        let t = t.strip_prefix("IT").unwrap_or(&t);
        match t {
            "01" => Some(ItGrau::IT01),
            "0" => Some(ItGrau::IT0),
            _ => ItGrau::novo(t.parse().ok()?),
        }
    }
    /// Largura da zona de tolerância em µm; `None` fora de 0 < D ≤ 500 mm e para IT14–IT18 até 1 mm
    /// (a norma não os define aí).
    pub fn valor_um(self, d: f64) -> Option<f64> {
        let f = faixa_it(d)?;
        if self.numero() >= 14 && d <= 1. {
            return None;
        }
        Some(IT_UM[self.0 as usize][f])
    }
    pub fn valor_mm(self, d: f64) -> Option<f64> {
        self.valor_um(d).map(|v| v / 1000.)
    }
    /// Grau mais grosso cuja zona ainda cabe em `tol_um` (o grau "mais próximo por baixo");
    /// `None` se nem IT01 cabe.
    pub fn para_tolerancia(tol_um: f64, d: f64) -> Option<ItGrau> {
        (0..20u8).map(ItGrau).filter(|g| g.valor_um(d).is_some_and(|v| v <= tol_um + 1e-9)).next_back()
    }
    /// Grau anterior (IT7 → IT6), usado no Δ dos furos K–ZC.
    pub fn anterior(self) -> Option<ItGrau> {
        (self.0 > 0).then(|| ItGrau(self.0 - 1))
    }
}

/// Índice da faixa principal de `d`; `None` fora de 0 < d ≤ 500.
pub fn faixa_it(d: f64) -> Option<usize> {
    if !(d > 0. && d <= 500.) {
        return None;
    }
    FAIXAS_IT.iter().position(|&h| d <= h)
}
