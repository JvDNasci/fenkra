// SPDX-License-Identifier: LGPL-2.1-or-later
//! Guias lineares (B46): trilho por série de fabricante, modelo simplificado.
//!
//! **Dados** (família `hiwin_hgr`): largura `WR`, altura `HR`, escareado `D` × `h`, furo passante
//! `dh` (o `d` do catálogo, renomeado porque `d` já é o diâmetro nominal) e passo `P` dos furos.
//!
//! **Modelo**: trilho ao longo de X, centrado em `x = 0`, com a base em `z = 0` (a face de apoio,
//! como nos fixadores) e o topo em `z = HR`. Perfil retangular `WR × HR`, sem as pistas. Furos a cada
//! `P`, com a mesma sobra `E` nas duas pontas, como o catálogo recomenda. A quantidade de furos é a
//! que cabe no comprimento da geração (`floor(L/P)`, ao menos um); mudar `L` depois recentra os furos
//! mas não muda a quantidade.

use crate::biblioteca::{Familia, Tamanho};
use crate::fixadores::Montador;
use crate::pneumatica::plano_em;
use fenkra_core::*;

fn medida(fam: &Familia, t: &Tamanho, c: &str) -> Result<f64, String> {
    t.valor(c).ok_or_else(|| format!("{} {}: falta '{c}'", fam.id, t.designacao))
}

/// Quantidade de furos que cabe no comprimento `l` com passo `p`.
pub fn furos_no_trilho(l: f64, p: f64) -> usize {
    ((l / p).floor() as usize).max(1)
}

pub(crate) fn corpo(m: &mut Montador, fam: &Familia, t: &Tamanho, l: f64) -> Result<(), String> {
    for c in ["WR", "HR", "D", "h", "dh", "P"] {
        m.param(c, &medida(fam, t, c)?.to_string())?;
    }
    m.param("L", &l.to_string())?;
    let n = furos_no_trilho(l, medida(fam, t, "P")?);
    m.param("n_furos", &n.to_string())?;
    // sobra igual nas duas pontas
    m.param("E", "(L-(n_furos-1)*P)/2")?;
    let base = m.plano("Plano_base", "0")?;
    m.prim("Trilho", PrimKind::Caixa, base, ["L", "WR", "HR", "0"], BoolOp::Add)?;
    for k in 0..n {
        let x = format!("-L/2+E+{k}*P");
        let passante = plano_em(m, &format!("Plano_furo_{}", k + 1), &x, "0", "-1")?;
        let escareado = plano_em(m, &format!("Plano_escareado_{}", k + 1), &x, "0", "HR-h")?;
        m.prim(&format!("Furo {}", k + 1), PrimKind::Cilindro, passante, ["dh/2", "HR+2", "0", "0"], BoolOp::Cut)?;
        m.prim(&format!("Escareado {}", k + 1), PrimKind::Cilindro, escareado, ["D/2", "h+1", "0", "0"], BoolOp::Cut)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biblioteca::{familia, tamanho, TipoFixador};
    use crate::fixadores::fixador_documento;
    use crate::{recompute, Status};
    use std::f64::consts::PI;

    #[test]
    fn trilhos_hgr_conferidos() {
        let f = familia("hiwin_hgr").unwrap();
        assert_eq!((f.tipo, f.nivel.as_str(), f.fontes.len()), (TipoFixador::TrilhoLinear, "C", 2));
        assert_eq!(f.tamanhos.iter().map(|t| t.designacao.as_str()).collect::<Vec<_>>(), ["HGR15R", "HGR20R", "HGR25R", "HGR30R", "HGR35R"]);
        assert!(f.tamanhos.iter().all(|t| t.confirmado_por >= 2 && t.valor("D") > t.valor("dh") && t.valor("h") < t.valor("HR")));
        let t = tamanho("hiwin_hgr", "HGR25R").unwrap();
        assert_eq!((t.valor("WR"), t.valor("HR"), t.valor("P")), (Some(23.), Some(22.), Some(60.)));
        assert_eq!(furos_no_trilho(1000., 60.), 16);
        assert_eq!(furos_no_trilho(50., 60.), 1);
    }

    #[test]
    fn trilho_gera_perfil_e_furos_no_passo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for t in &familia("hiwin_hgr").unwrap().tamanhos {
            let v = |c: &str| t.valor(c).unwrap();
            let l = 300.;
            let doc = fixador_documento("hiwin_hgr", &t.designacao, Some(l)).unwrap();
            let rc = recompute(&doc);
            assert!(rc.param_error.is_none(), "{:?}", rc.param_error);
            assert!(doc.features.iter().all(|f| rc.status[&f.id] == Status::Ok), "{}", t.designacao);
            assert_eq!(rc.bodies.len(), 1);
            let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
            for p in rc.bodies[0].mesh.positions.chunks_exact(3) {
                for i in 0..3 {
                    lo[i] = lo[i].min(p[i] as f64);
                    hi[i] = hi[i].max(p[i] as f64);
                }
            }
            assert!((hi[0] - lo[0] - l).abs() < 1e-3 && (hi[1] - lo[1] - v("WR")).abs() < 1e-3, "{}", t.designacao);
            assert!(lo[2].abs() < 1e-3 && (hi[2] - v("HR")).abs() < 1e-3, "{}", t.designacao);
            let n = furos_no_trilho(l, v("P")) as f64;
            let furo = PI / 4. * (v("dh").powi(2) * (v("HR") - v("h")) + v("D").powi(2) * v("h"));
            let esperado = l * v("WR") * v("HR") - n * furo;
            let vol = rc.bodies[0].mass.volume;
            assert!((vol - esperado).abs() / esperado < 1e-3, "{}: volume {vol} contra {esperado}", t.designacao);
        }
    }
}
