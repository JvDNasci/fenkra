// SPDX-License-Identifier: LGPL-2.1-or-later
//! Tolerâncias gerais: ISO 2768-1 (dimensões lineares e angulares) e ISO 2768-2 (forma e posição).

use serde::{Deserialize, Serialize};

/// Classe da ISO 2768-1.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum ClasseGeral {
    /// f, fina
    F,
    /// m, média
    M,
    /// c, grosseira
    C,
    /// v, muito grosseira
    V,
}

impl ClasseGeral {
    pub const TODAS: [ClasseGeral; 4] = [ClasseGeral::F, ClasseGeral::M, ClasseGeral::C, ClasseGeral::V];
    pub fn letra(self) -> char {
        match self {
            ClasseGeral::F => 'f',
            ClasseGeral::M => 'm',
            ClasseGeral::C => 'c',
            ClasseGeral::V => 'v',
        }
    }
    pub fn parse(s: &str) -> Option<ClasseGeral> {
        match s.trim().to_ascii_lowercase().as_str() {
            "f" => Some(ClasseGeral::F),
            "m" => Some(ClasseGeral::M),
            "c" => Some(ClasseGeral::C),
            "v" => Some(ClasseGeral::V),
            _ => None,
        }
    }
    fn col(self) -> usize {
        self as usize
    }

    /// Desvio simétrico ± (mm) de uma dimensão linear nominal `d` (mm), exceto arestas quebradas.
    /// `None` abaixo de 0,5 mm (a norma pede indicação individual), acima de 4000 mm ou onde a
    /// classe não existe (f acima de 1000 mm; v até 3 mm).
    pub fn linear_mm(self, d: f64) -> Option<f64> {
        // limites superiores das faixas: 3, 6, 30, 120, 400, 1000, 2000, 4000
        const LIM: [f64; 8] = [3., 6., 30., 120., 400., 1000., 2000., 4000.];
        const N: f64 = f64::NAN;
        #[rustfmt::skip]
        const T: [[f64; 4]; 8] = [
            [0.05, 0.1, 0.2, N],
            [0.05, 0.1, 0.3, 0.5],
            [0.1, 0.2, 0.5, 1.],
            [0.15, 0.3, 0.8, 1.5],
            [0.2, 0.5, 1.2, 2.5],
            [0.3, 0.8, 2., 4.],
            [0.5, 1.2, 3., 6.],
            [N, 2., 4., 8.],
        ];
        if d < 0.5 {
            return None;
        }
        let v = T[LIM.iter().position(|&h| d <= h)?][self.col()];
        (!v.is_nan()).then_some(v)
    }

    /// Desvio ± (mm) de raio externo ou altura de chanfro (arestas quebradas).
    pub fn aresta_quebrada_mm(self, d: f64) -> Option<f64> {
        if d < 0.5 {
            return None;
        }
        let i = if d <= 3. { 0 } else if d <= 6. { 1 } else { 2 };
        let fina = [0.2, 0.5, 1.][i];
        let grossa = [0.4, 1., 2.][i];
        Some(match self {
            ClasseGeral::F | ClasseGeral::M => fina,
            ClasseGeral::C | ClasseGeral::V => grossa,
        })
    }

    /// Desvio angular ± em minutos de arco, pelo comprimento (mm) do lado menor do ângulo.
    pub fn angular_min(self, lado_menor: f64) -> Option<f64> {
        if lado_menor <= 0. {
            return None;
        }
        // até 10, 10–50, 50–120, 120–400, acima de 400
        let i = [10., 50., 120., 400.].iter().position(|&h| lado_menor <= h).unwrap_or(4);
        #[rustfmt::skip]
        const T: [[f64; 5]; 3] = [
            [60., 30., 20., 10., 5.],   // f, m: 1° 0°30' 0°20' 0°10' 0°5'
            [90., 60., 30., 15., 10.],  // c: 1°30' 1° 0°30' 0°15' 0°10'
            [180., 120., 60., 30., 20.],// v: 3° 2° 1° 0°30' 0°20'
        ];
        let linha = match self {
            ClasseGeral::F | ClasseGeral::M => 0,
            ClasseGeral::C => 1,
            ClasseGeral::V => 2,
        };
        Some(T[linha][i])
    }

    /// Desvio angular em graus decimais.
    pub fn angular_graus(self, lado_menor: f64) -> Option<f64> {
        self.angular_min(lado_menor).map(|m| m / 60.)
    }
}

/// Classe da ISO 2768-2 (forma e posição sem indicação individual).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum ClasseForma {
    H,
    K,
    L,
}

impl ClasseForma {
    pub const TODAS: [ClasseForma; 3] = [ClasseForma::H, ClasseForma::K, ClasseForma::L];
    pub fn parse(s: &str) -> Option<ClasseForma> {
        match s.trim().to_ascii_uppercase().as_str() {
            "H" => Some(ClasseForma::H),
            "K" => Some(ClasseForma::K),
            "L" => Some(ClasseForma::L),
            _ => None,
        }
    }
    fn col(self) -> usize {
        self as usize
    }

    /// Retitude e planeza (mm) pelo comprimento da linha ou da maior dimensão da superfície.
    pub fn retitude_planeza_mm(self, comprimento: f64) -> Option<f64> {
        #[rustfmt::skip]
        const T: [[f64; 3]; 6] = [
            [0.02, 0.05, 0.1],
            [0.05, 0.1, 0.2],
            [0.1, 0.2, 0.4],
            [0.2, 0.4, 0.8],
            [0.3, 0.6, 1.2],
            [0.4, 0.8, 1.6],
        ];
        let i = [10., 30., 100., 300., 1000., 3000.].iter().position(|&h| comprimento <= h)?;
        (comprimento > 0.).then(|| T[i][self.col()])
    }

    /// Perpendicularidade (mm) pelo lado menor do ângulo reto; o lado maior é a referência.
    pub fn perpendicularidade_mm(self, lado_menor: f64) -> Option<f64> {
        #[rustfmt::skip]
        const T: [[f64; 3]; 4] = [[0.2, 0.4, 0.6], [0.3, 0.6, 1.], [0.4, 0.8, 1.5], [0.5, 1., 2.]];
        let i = [100., 300., 1000., 3000.].iter().position(|&h| lado_menor <= h)?;
        (lado_menor > 0.).then(|| T[i][self.col()])
    }

    /// Simetria (mm) pelo comprimento do elemento simétrico.
    pub fn simetria_mm(self, comprimento: f64) -> Option<f64> {
        #[rustfmt::skip]
        const T: [[f64; 3]; 4] = [[0.5, 0.6, 0.6], [0.5, 0.6, 1.], [0.5, 0.8, 1.5], [0.5, 1., 2.]];
        let i = [100., 300., 1000., 3000.].iter().position(|&h| comprimento <= h)?;
        (comprimento > 0.).then(|| T[i][self.col()])
    }

    /// Batimento circular (mm); não depende do comprimento.
    pub fn batimento_mm(self) -> f64 {
        [0.1, 0.2, 0.5][self.col()]
    }
}
