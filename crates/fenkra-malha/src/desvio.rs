// SPDX-License-Identifier: LGPL-2.1-or-later
//! Distância entre duas malhas. Mede de cada vértice e do centro de cada triângulo da malha até a
//! superfície da outra. A medida não é simétrica: uma malha pode caber inteira na outra e deixar
//! sobrar um pedaço, e por isso a de Hausdorff mede nos dois sentidos e fica com a maior.

use crate::{add, mul, Bvh, Malha};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Desvio {
    pub maximo: f64,
    pub media: f64,
    pub rms: f64,
    pub amostras: usize,
}

/// De `a` até a superfície de `b`.
pub fn desvio(a: &Malha, b: &Bvh) -> Desvio {
    let (mut d, mut soma, mut quadrados) = (Desvio::default(), 0., 0.);
    let mut medir = |p: [f64; 3]| {
        if let Some(r) = b.mais_proximo(p) {
            d.maximo = d.maximo.max(r.distancia);
            soma += r.distancia;
            quadrados += r.distancia * r.distancia;
            d.amostras += 1;
        }
    };
    a.pos.iter().for_each(|p| medir(*p));
    for t in &a.tris {
        let [p, q, r] = t.map(|i| a.pos[i as usize]);
        medir(mul(add(add(p, q), r), 1. / 3.));
    }
    if d.amostras > 0 {
        d.media = soma / d.amostras as f64;
        d.rms = (quadrados / d.amostras as f64).sqrt();
    }
    d
}

/// Desvio nos dois sentidos, ficando com o pior de cada medida.
pub fn hausdorff(a: &Malha, b: &Malha) -> Desvio {
    let (ida, volta) = (desvio(a, &Bvh::nova(b)), desvio(b, &Bvh::nova(a)));
    Desvio { maximo: ida.maximo.max(volta.maximo), media: ida.media.max(volta.media), rms: ida.rms.max(volta.rms), amostras: ida.amostras + volta.amostras }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::teste::*;

    #[test]
    fn esferas_concentricas_distam_a_diferenca_dos_raios() {
        let (a, b) = (esfera(10., 40), esfera(11., 40));
        let d = hausdorff(&a, &b);
        assert!((d.maximo - 1.).abs() < 0.05, "{d:?}");
        assert!(d.media > 0.9 && d.rms >= d.media);
    }

    #[test]
    fn iguais_nao_tem_desvio() {
        let a = caixa([3., 4., 5.]);
        assert!(hausdorff(&a, &a).maximo < 1e-12);
    }
}
