// SPDX-License-Identifier: LGPL-2.1-or-later
//! B197: análise de saída e undercut por face, como serviço para quem precisar dela.
//!
//! Dada a direção de abertura, cada face recebe:
//! - o ângulo de saída (menor e maior entre os triângulos dela, `asin(|n·d|)`);
//! - a faixa de cor desse ângulo ([`Faixa`]): sem saída, justa, suficiente ou perpendicular à
//!   abertura, com os limites do processo ([`Limites`]);
//! - a classe: cavidade (olha para a abertura), núcleo (olha para o lado oposto), lateral (paralela
//!   à abertura) ou ambígua (parte de cada lado);
//! - a área em undercut: o que, andando da face para o lado em que ela sai do molde, bate em outra
//!   parte da peça (visibilidade ao longo da direção).
//!
//! O cálculo por triângulo fica em `dfm_geo::saida`; aqui ele vira uma análise com resumo, faixas e
//! limites, que serve à tela de inclinação, ao motor de regras (B112), aos moldes (B129), aos
//! recursos de plástico (B148) e ao balanço da impressão 3D (B164). Os limites vêm do perfil do
//! processo ([`Limites::do_perfil`]), que a pessoa edita sem recompilar.

use crate::dfm::Perfil;
use crate::dfm_geo::{self as geo, Classe, FaceSaida, Malha};
use fenkra_kernel::{Mesh, Shape};

pub type V3 = [f64; 3];

/// A partir daqui a face conta como "tampa": perpendicular o bastante à abertura para não pedir saída.
pub const TAMPA_GRAUS: f64 = 60.;

/// Faixa de cor do ângulo de saída.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Faixa {
    /// perpendicular à abertura (tampa ou fundo): não pede saída
    Tampa,
    /// saída de ao menos o dobro do mínimo
    Suficiente,
    /// entre o mínimo e o dobro dele
    Justa,
    /// abaixo do mínimo do processo
    SemSaida,
}

impl Faixa {
    pub const TODAS: [Faixa; 4] = [Faixa::SemSaida, Faixa::Justa, Faixa::Suficiente, Faixa::Tampa];

    pub fn nome(self) -> &'static str {
        match self {
            Faixa::SemSaida => "Sem saída suficiente",
            Faixa::Justa => "Saída justa",
            Faixa::Suficiente => "Saída suficiente",
            Faixa::Tampa => "Perpendicular à abertura",
        }
    }
}

/// Limites do processo para a análise.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limites {
    /// ângulo de saída mínimo, em graus
    pub saida_min: f64,
    /// área bloqueada aceita por face, em mm² (0 = qualquer undercut conta)
    pub undercut_max: f64,
}

impl Default for Limites {
    fn default() -> Self {
        Limites { saida_min: 1., undercut_max: 0. }
    }
}

impl Limites {
    /// Os limites da regra de saída (`saida.angulo`, `min`) e da de undercut (`undercut.area`, `max`)
    /// do perfil; o que o perfil não tiver fica no padrão.
    pub fn do_perfil(p: &Perfil) -> Limites {
        let mut l = Limites::default();
        let ativa = |e: &'static str, m: &'static str| p.regras.iter().filter(move |r| r.ativa && r.entidade == e && r.medida == m);
        if let Some(v) = ativa("saida", "angulo").filter_map(|r| r.min).reduce(f64::max) {
            l.saida_min = v;
        }
        if let Some(v) = ativa("undercut", "area").filter_map(|r| r.max).reduce(f64::min) {
            l.undercut_max = v;
        }
        l
    }

    /// Faixa de um ângulo de saída.
    pub fn faixa(&self, graus: f64) -> Faixa {
        // a malha mede o ângulo com erro de milésimos de grau: 2° desenhados não podem cair em "justa"
        let (m, tol) = (self.saida_min.max(1e-6), 1e-3);
        if graus < m - tol {
            Faixa::SemSaida
        } else if graus < 2. * m - tol {
            Faixa::Justa
        } else if graus < TAMPA_GRAUS {
            Faixa::Suficiente
        } else {
            Faixa::Tampa
        }
    }
}

/// Resultado da análise de uma peça para uma direção de abertura.
#[derive(Clone, Debug)]
pub struct Analise {
    pub direcao: V3,
    pub limites: Limites,
    /// uma entrada por face, na ordem da malha do corpo
    pub faces: Vec<FaceSaida>,
}

impl Analise {
    /// Faixa da face pelo pior ângulo dela (uma face curva que vai da parede à tampa conta pela parede).
    pub fn faixa(&self, f: &FaceSaida) -> Faixa {
        self.limites.faixa(f.saida_min)
    }

    /// A face está em undercut acima do aceito pelo processo?
    pub fn em_undercut(&self, f: &FaceSaida) -> bool {
        f.undercut() && f.area_undercut > self.limites.undercut_max + 1e-9
    }

    pub fn contar_classe(&self, c: Classe) -> usize {
        self.faces.iter().filter(|f| f.classe == c).count()
    }

    pub fn contar_faixa(&self, x: Faixa) -> usize {
        self.faces.iter().filter(|f| self.faixa(f) == x).count()
    }

    pub fn undercuts(&self) -> impl Iterator<Item = &FaceSaida> {
        self.faces.iter().filter(|f| self.em_undercut(f))
    }

    /// Área total de cada classe, na ordem cavidade, núcleo, lateral, ambígua.
    pub fn area_por_classe(&self) -> [f64; 4] {
        let mut a = [0.; 4];
        for f in &self.faces {
            let k = match f.classe {
                Classe::Cavidade => 0,
                Classe::Nucleo => 1,
                Classe::Lateral => 2,
                Classe::Ambigua => 3,
            };
            a[k] += f.area_total;
        }
        a
    }

    /// A peça sai do molde nesta direção sem gaveta e sem parede abaixo do mínimo?
    pub fn moldavel(&self) -> bool {
        self.undercuts().next().is_none() && self.faces.iter().all(|f| self.faixa(f) != Faixa::SemSaida)
    }
}

/// Analisa uma malha já pronta (a de exibição do corpo, por exemplo: a ordem das faces é a dela).
pub fn analisar_malha(m: &Mesh, direcao: V3, limites: Limites) -> Option<Analise> {
    let g = Malha::de(m)?;
    Some(analisar(&g, direcao, limites))
}

/// Analisa a malha da análise de fabricação.
pub fn analisar(m: &Malha, direcao: V3, limites: Limites) -> Analise {
    let faces = geo::saida(m, direcao, limites.saida_min);
    Analise { direcao, limites, faces }
}

/// Analisa uma forma do kernel: tessela com o mesmo desvio da varredura DFM.
pub fn analisar_forma(s: &Shape, direcao: V3, limites: Limites) -> Result<Analise, String> {
    let (lo, hi) = s.bbox().ok_or("a forma está vazia")?;
    let diag = (0..3).map(|k| (hi[k] - lo[k]).powi(2)).sum::<f64>().sqrt().max(1e-6);
    let mesh = s.mesh_angle(diag * 1e-3, 0.25).map_err(|e| e.0)?;
    analisar_malha(&mesh, direcao, limites).ok_or_else(|| "a malha não tem triângulos".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faixas_seguem_o_minimo_do_processo() {
        let l = Limites { saida_min: 2., undercut_max: 0. };
        assert_eq!(l.faixa(0.), Faixa::SemSaida);
        assert_eq!(l.faixa(1.99), Faixa::SemSaida);
        assert_eq!(l.faixa(2.), Faixa::Justa);
        assert_eq!(l.faixa(4.), Faixa::Suficiente);
        assert_eq!(l.faixa(59.), Faixa::Suficiente);
        assert_eq!(l.faixa(90.), Faixa::Tampa);
    }

    #[test]
    fn limites_vem_do_perfil_e_mudam_com_ele() {
        let mut p = crate::dfm::perfis_padrao().into_iter().find(|p| p.processo == "injecao").unwrap();
        let l = Limites::do_perfil(&p);
        assert_eq!(l.saida_min, 1.);
        assert_eq!(l.undercut_max, 0.);
        p.ajusta("inj.saida.angulo", Some(3.), None);
        assert_eq!(Limites::do_perfil(&p).saida_min, 3.);
        // perfil sem regra de saída: fica o padrão
        let fdm = crate::dfm::perfis_padrao().into_iter().find(|p| p.processo == "aditivo").unwrap();
        assert_eq!(Limites::do_perfil(&fdm), Limites::default());
    }
}
