// SPDX-License-Identifier: LGPL-2.1-or-later
//! Dimensionar um par de engrenagens por distância entre centros e razão (B280).
//!
//! Para engrenagens retas sem correção de perfil, a distância entre centros é `a = m (z1 + z2) / 2` e
//! a razão é `i = z2 / z1`. Dados `a` e `i`, cada módulo `m` da lista de trabalho pede
//! `z1 = 2a / (m (1 + i))`; arredondam-se `z1` e `z2` e mede-se quanto o par resultante se afasta do
//! pedido. As melhores combinações viram sugestões.
//!
//! A lista de módulos é de **uso comum**, não a tabela completa da norma de módulos: só os valores
//! que duas fontes públicas confirmaram entram como "padrão" (a série 1 de um catálogo técnico
//! público que cita a DIN 780 e a prática corrente); o resto da tabela fica fora até ser conferido.

use crate::engrenagens::{calculo, ParamEngrenagem, TipoEngrenagem};

/// Módulos de trabalho em mm (menor para maior).
pub const MODULOS_USUAIS: [f64; 14] = [0.5, 0.8, 1., 1.25, 1.5, 2., 2.5, 3., 4., 5., 6., 8., 10., 12.];

/// Uma combinação módulo/dentes que atende (ou quase) ao pedido.
#[derive(Clone, Debug, PartialEq)]
pub struct SugestaoPar {
    pub modulo: f64,
    pub z1: u32,
    pub z2: u32,
    /// distância entre centros do par sugerido, `m (z1 + z2) / 2`
    pub distancia: f64,
    /// razão `z2 / z1` do par sugerido
    pub razao: f64,
    /// `distancia - pedida`, em mm
    pub erro_distancia: f64,
    /// `razao - pedida`
    pub erro_razao: f64,
}

/// Até 6 sugestões para distância `a` (mm) e razão `i = z2 / z1 >= 1`, da melhor para a pior (soma dos
/// erros relativos). O pinhão fica com pelo menos 17 dentes, o limite sem rebaixo para 20°; a coroa
/// com no máximo 400. Sem solução quando `a` ou `i` não são positivos ou finitos.
pub fn sugerir_par(a: f64, razao: f64, angulo_pressao: f64) -> Vec<SugestaoPar> {
    if !(a.is_finite() && razao.is_finite()) || a <= 0. || razao < 1. {
        return vec![];
    }
    let z_min = (2. / angulo_pressao.to_radians().sin().powi(2)).floor().max(6.) as u32;
    let mut todas = vec![];
    for m in MODULOS_USUAIS {
        let z1_ideal = 2. * a / (m * (1. + razao));
        // os dois inteiros vizinhos do ideal: o melhor dos dois depende também de z2
        for z1 in [z1_ideal.floor() as i64, z1_ideal.ceil() as i64] {
            if z1 < z_min as i64 || z1 > 400 {
                continue;
            }
            let z1 = z1 as u32;
            for z2 in [(razao * z1 as f64).floor() as i64, (razao * z1 as f64).ceil() as i64] {
                if z2 < z1 as i64 || z2 > 400 {
                    continue;
                }
                let z2 = z2 as u32;
                let distancia = m * (z1 + z2) as f64 / 2.;
                let r = z2 as f64 / z1 as f64;
                let p = ParamEngrenagem { tipo: TipoEngrenagem::Reta, modulo: m, dentes: z1, angulo_pressao, ..Default::default() };
                if calculo(&p).is_err() {
                    continue;
                }
                if !todas.iter().any(|s: &SugestaoPar| s.modulo == m && s.z1 == z1 && s.z2 == z2) {
                    todas.push(SugestaoPar { modulo: m, z1, z2, distancia, razao: r, erro_distancia: distancia - a, erro_razao: r - razao });
                }
            }
        }
    }
    let nota = |s: &SugestaoPar| s.erro_distancia.abs() / a + s.erro_razao.abs() / razao;
    todas.sort_by(|x, y| nota(x).total_cmp(&nota(y)).then(y.modulo.total_cmp(&x.modulo)));
    todas.truncate(6);
    todas
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pedido_exato_vira_sugestoes_exatas() {
        // a = 90, i = 2: m2 z30/z60, m3 z20/z40, m1,5 z40/z80, m2,5 z24/z48 e m1 z60/z120 fecham sem erro
        let s = sugerir_par(90., 2., 20.);
        assert!(s.len() >= 4);
        for x in s.iter().take(4) {
            assert!(x.erro_distancia.abs() < 1e-9 && x.erro_razao.abs() < 1e-9, "{x:?}");
            assert!((x.modulo * (x.z1 + x.z2) as f64 / 2. - 90.).abs() < 1e-9 && x.z2 == 2 * x.z1);
        }
        assert!(s.iter().any(|x| x.modulo == 2. && x.z1 == 30 && x.z2 == 60));
        assert!(s.iter().any(|x| x.modulo == 3. && x.z1 == 20 && x.z2 == 40));
    }

    #[test]
    fn pedido_sem_solucao_exata_traz_o_erro() {
        let s = sugerir_par(100., 2.5, 20.);
        assert!(!s.is_empty());
        // ordenadas da melhor para a pior
        let nota = |x: &SugestaoPar| x.erro_distancia.abs() / 100. + x.erro_razao.abs() / 2.5;
        assert!(s.windows(2).all(|w| nota(&w[0]) <= nota(&w[1]) + 1e-12));
        for x in &s {
            assert!(x.z1 >= 17 && x.z2 >= x.z1 && x.z2 <= 400);
            assert!((x.distancia - x.erro_distancia - 100.).abs() < 1e-9);
        }
    }

    #[test]
    fn entradas_invalidas_nao_sugerem() {
        assert!(sugerir_par(0., 2., 20.).is_empty());
        assert!(sugerir_par(90., 0.5, 20.).is_empty(), "razão abaixo de 1: troque pinhão e coroa");
        assert!(sugerir_par(f64::NAN, 2., 20.).is_empty());
    }
}
