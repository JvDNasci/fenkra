// SPDX-License-Identifier: LGPL-2.1-or-later
//! B-rep ⇄ fluxo NURBS (B211, ADR 0090). O fluxo é o formato neutro que a ponte com o openNURBS
//! (`fenkra-3dm`) escreve e lê; a descrição dele está em `crates/fenkra-3dm/src/lib.rs`. A montagem fica
//! do lado do kernel para o crate do 3DM não incluir nada do OpenCASCADE.

use crate::{KResult, KernelError, Shape};

impl Shape {
    /// Monta a forma de um fluxo NURBS: uma aresta por aresta do arquivo, com a curva de aparo de cada face,
    /// acerto de parâmetro, reparo leve e sólido quando a casca fecha. `tolerancia` é a do modelo, em mm.
    pub fn de_fluxo_nurbs(fluxo: &[f64], tolerancia: f64) -> KResult<Shape> {
        #[cfg(occt)]
        {
            crate::ffi::bridge::brep_de_fluxo(fluxo, tolerancia).map(Shape::da_ponte).map_err(|e| KernelError(e.what().to_string()))
        }
        #[cfg(not(occt))]
        {
            let _ = (fluxo, tolerancia);
            Err(KernelError("kernel indisponível".into()))
        }
    }

    /// Volume por integração adaptativa (erro relativo 1e-9). O `mass().volume` usa a integração padrão do
    /// OpenCASCADE, rápida, mas que erra na quarta casa em face aparada por curva NURBS racional, o caso de
    /// toda peça vinda de 3DM. Use este para conferir ida e volta.
    pub fn volume_preciso(&self) -> f64 {
        #[cfg(occt)]
        {
            crate::ffi::bridge::volume_preciso(self.ponte(), 1e-9)
        }
        #[cfg(not(occt))]
        {
            0.0
        }
    }

    /// Fluxo NURBS da forma, com todas as faces convertidas para NURBS e as arestas compartilhadas.
    pub fn fluxo_nurbs(&self) -> KResult<Vec<f64>> {
        #[cfg(occt)]
        {
            crate::ffi::bridge::fluxo_de_brep(self.ponte()).map_err(|e| KernelError(e.what().to_string()))
        }
        #[cfg(not(occt))]
        {
            Err(KernelError("kernel indisponível".into()))
        }
    }
}

#[cfg(all(test, occt))]
mod testes {
    use super::*;

    fn conferir(original: &Shape, volta: &Shape, tol: f64) {
        assert!(volta.is_valid(), "forma da volta inválida");
        let (v0, v1) = (original.volume_preciso(), volta.volume_preciso());
        assert!((v0 - v1).abs() <= tol * v0.abs().max(1.0), "volume {v0} → {v1}");
        let (a, b) = (original.bbox().unwrap(), volta.bbox().unwrap());
        for k in 0..3 {
            assert!((a.0[k] - b.0[k]).abs() < 1e-4 && (a.1[k] - b.1[k]).abs() < 1e-4, "caixa {a:?} → {b:?}");
        }
    }

    /// O volume padrão erra na quarta casa com aparo NURBS racional; a malha fina mostra que a geometria é a
    /// mesma, e o volume preciso concorda com a fórmula.
    #[test]
    fn cilindro_volta_com_a_mesma_geometria_e_o_volume_preciso_bate() {
        let plano = crate::Frame { origin: [0., 0., 0.], x: [1., 0., 0.], normal: [0., 0., 1.] };
        let c = Shape::primitive(1, &plano, &[6., 10.]).unwrap();
        let v = Shape::de_fluxo_nurbs(&c.fluxo_nurbs().unwrap(), 1e-3).unwrap();
        let exato = std::f64::consts::PI * 36. * 10.;
        assert!((v.volume_preciso() - exato).abs() < 1e-6 * exato, "{} contra {exato}", v.volume_preciso());
        let pts: Vec<[f64; 3]> = (0..12).map(|i| [6. * (i as f64 * 0.5).cos(), 6. * (i as f64 * 0.5).sin(), 3.]).collect();
        assert!(v.signed_distances(&pts).unwrap().iter().all(|d| d.abs() < 1e-9));
    }

    #[test]
    fn caixa_vai_e_volta_pelo_fluxo() {
        let caixa = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [80., 40., 10.]).unwrap();
        let f = caixa.fluxo_nurbs().unwrap();
        assert_eq!(&f[..2], &[3141., 1.]);
        let volta = Shape::de_fluxo_nurbs(&f, 1e-3).unwrap();
        conferir(&caixa, &volta, 1e-9);
    }

    #[test]
    fn placa_com_furo_e_esfera_vao_e_voltam_pelo_fluxo() {
        // furo passante: duas faces cilíndricas com costura e laços internos nas faces planas
        let placa = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [80., 40., 10.]).unwrap();
        let furo = Shape::hole_tool([20., 20., 10.], [0., 0., -1.], 12., 10., crate::HoleShape::Simple).unwrap();
        let peca = placa.boolean(&furo, crate::Bool::Cut).unwrap();
        assert!(peca.mass().volume < 80. * 40. * 10. - 1.);
        let volta = Shape::de_fluxo_nurbs(&peca.fluxo_nurbs().unwrap(), 1e-3).unwrap();
        conferir(&peca, &volta, 1e-6);
        // esfera: polos (arestas degeneradas) e costura
        let plano = crate::Frame { origin: [0., 0., 0.], x: [1., 0., 0.], normal: [0., 0., 1.] };
        let esfera = Shape::primitive(2, &plano, &[25.]).unwrap();
        let volta = Shape::de_fluxo_nurbs(&esfera.fluxo_nurbs().unwrap(), 1e-3).unwrap();
        conferir(&esfera, &volta, 1e-6);
    }
}
