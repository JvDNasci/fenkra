// SPDX-License-Identifier: LGPL-2.1-or-later
//! Booleanas de malha pela biblioteca Manifold, no port em Rust (Apache-2.0, ADR 0060).
//!
//! A Manifold só trabalha com casca fechada e orientada e, em troca, garante que a saída também é. Por
//! isso a entrada passa pela solda e a falha vira uma mensagem que manda reparar, sem deixar malha pela
//! metade na árvore.

use crate::{Erro, Malha};
use manifold_rust::manifold::Manifold;
use manifold_rust::types::{Error, MeshGL64};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpMalha {
    Unir,
    Subtrair,
    Intersectar,
}

fn entrada(m: &Malha, papel: &str) -> Result<Manifold, Erro> {
    let s = m.soldada();
    let g = MeshGL64 { num_prop: 3, vert_properties: s.pos.iter().flatten().copied().collect(), tri_verts: s.tris.iter().flatten().map(|v| u64::from(*v)).collect(), ..Default::default() };
    let r = Manifold::from_mesh_gl64(&g);
    match r.status() {
        Error::NoError => Ok(r),
        Error::NotManifold => Err(Erro::Operacao(format!("{papel} não é uma casca fechada: repare a malha antes da booleana"))),
        e => Err(Erro::Operacao(format!("{papel} recusado pela booleana de malha: {e:?}"))),
    }
}

/// `a` combinado com `b`. A malha de saída fica soldada e fechada.
pub fn booleana(a: &Malha, b: &Malha, op: OpMalha) -> Result<Malha, Erro> {
    let (x, y) = (entrada(a, "o corpo alvo")?, entrada(b, "o corpo ferramenta")?);
    let r = match op {
        OpMalha::Unir => x.union(&y),
        OpMalha::Subtrair => x.difference(&y),
        OpMalha::Intersectar => x.intersection(&y),
    };
    if r.status() != Error::NoError {
        return Err(Erro::Operacao(format!("a booleana de malha falhou: {:?}", r.status())));
    }
    let g = r.get_mesh_gl64(0);
    let n = g.num_prop as usize;
    let pos = g.vert_properties.chunks_exact(n).map(|p| [p[0], p[1], p[2]]).collect();
    let tris = g.tri_verts.chunks_exact(3).map(|t| [t[0] as u32, t[1] as u32, t[2] as u32]).collect();
    Ok(Malha { nome: a.nome.clone(), pos, tris, material: a.material, ..Default::default() })
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::teste::*;

    fn deslocada(mut m: Malha, d: [f64; 3]) -> Malha {
        m.pos.iter_mut().for_each(|p| *p = [p[0] + d[0], p[1] + d[1], p[2] + d[2]]);
        m
    }

    #[test]
    fn caixas_dao_o_volume_de_cada_operacao() {
        let (a, b) = (caixa([10., 10., 10.]), deslocada(caixa([10., 10., 10.]), [5., 5., 5.]));
        for (op, v) in [(OpMalha::Unir, 1875.), (OpMalha::Subtrair, 875.), (OpMalha::Intersectar, 125.)] {
            let r = booleana(&a, &b, op).unwrap();
            assert!((r.volume() - v).abs() < 1e-6, "{op:?}: {}", r.volume());
            assert!(r.diagnostico().estanque(), "{op:?}: {:?}", r.diagnostico());
        }
    }

    #[test]
    fn esfera_tira_o_miolo_e_continua_fechada() {
        let a = caixa([20., 20., 20.]);
        let e = deslocada(esfera(6., 24), [10., 10., 20.]);
        let r = booleana(&a, &e, OpMalha::Subtrair).unwrap();
        let meia = e.volume() / 2.;
        assert!((r.volume() - (8000. - meia)).abs() < 1., "{}", r.volume());
        assert!(r.diagnostico().estanque());
    }

    #[test]
    fn malha_aberta_pede_reparo() {
        let mut a = caixa([10., 10., 10.]);
        a.tris.pop();
        let e = booleana(&a, &caixa([1., 1., 1.]), OpMalha::Unir).unwrap_err();
        assert!(e.to_string().contains("repare"), "{e}");
    }
}
