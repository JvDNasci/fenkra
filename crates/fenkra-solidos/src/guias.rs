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

/// Patim (bloco) de guia linear, série HGH: o mesmo referencial do trilho (eixo X, base do trilho em
/// `z = 0`). Caixa `L × W` de `H1` a `H`, com o canal `WR` do trilho até `HR` e quatro furos roscados
/// `Ø M × prof` no topo, nos cantos do retângulo `C × B`. O topo `z = H` é a face onde a mesa se fixa.
pub(crate) fn corpo_patim(m: &mut Montador, fam: &Familia, t: &Tamanho) -> Result<(), String> {
    for c in ["H", "H1", "W", "B", "C", "L", "M", "prof", "WR", "HR"] {
        m.param(c, &medida(fam, t, c)?.to_string())?;
    }
    let v = |c: &str| t.valor(c).unwrap_or(0.);
    if !(v("H1") < v("HR") && v("HR") < v("H") && v("B") + v("M") < v("W") && v("C") < v("L") && v("prof") < v("H") - v("HR")) {
        return Err(format!("{} {}: medidas do bloco incoerentes com as do trilho", fam.id, t.designacao));
    }
    let base = m.plano("Plano_base_bloco", "H1")?;
    m.prim("Bloco", PrimKind::Caixa, base, ["L", "W", "H-H1", "0"], BoolOp::Add)?;
    let canal = m.plano("Plano_canal", "H1-1")?;
    m.prim("Canal do trilho", PrimKind::Caixa, canal, ["L+2", "WR", "HR-H1+1", "0"], BoolOp::Cut)?;
    for (i, (sx, sy)) in [(1, 1), (-1, 1), (-1, -1), (1, -1)].into_iter().enumerate() {
        let pl = plano_em(m, &format!("Plano_furo_{}", i + 1), &format!("{sx}*C/2"), &format!("{sy}*B/2"), "H-prof")?;
        m.prim(&format!("Furo {}", i + 1), PrimKind::Cilindro, pl, ["M/2", "prof+1", "0", "0"], BoolOp::Cut)?;
    }
    Ok(())
}

/// Porca de esferas com flange (DIN 69051-5): eixo Z, face do flange em `z = 0` (a de apoio) e corpo
/// para `-Z`. Corpo `Ø D × L`, flange `Ø A × B` com duas faces planas a `H` e o furo do fuso `Ø d`.
pub(crate) fn corpo_porca_esferas(m: &mut Montador, fam: &Familia, t: &Tamanho) -> Result<(), String> {
    for c in ["P", "D", "A", "B", "L", "H"] {
        m.param(c, &medida(fam, t, c)?.to_string())?;
    }
    let (d, v) = (t.diametro_nominal().unwrap_or(0.), |c: &str| t.valor(c).unwrap_or(0.));
    if !(d < v("D") && v("D") < v("H") && v("H") < v("A") && v("B") < v("L")) {
        return Err(format!("{} {}: precisa de d < D < H < A e B < L", fam.id, t.designacao));
    }
    let fundo = m.plano("Plano_corpo", "-L")?;
    m.prim("Corpo", PrimKind::Cilindro, fundo, ["D/2", "L", "0", "0"], BoolOp::Add)?;
    let flange = m.plano("Plano_flange", "-B")?;
    m.prim("Flange", PrimKind::Cilindro, flange, ["A/2", "B", "0", "0"], BoolOp::Add)?;
    for (i, sy) in [1, -1].into_iter().enumerate() {
        let pl = plano_em(m, &format!("Plano_face_{}", i + 1), "0", &format!("{sy}*(H/2+A/2)"), "-B-1")?;
        m.prim(&format!("Face plana {}", i + 1), PrimKind::Caixa, pl, ["A+2", "A", "B+2", "0"], BoolOp::Cut)?;
    }
    let furo = m.plano("Plano_furo", "-L-1")?;
    m.prim("Furo do fuso", PrimKind::Cilindro, furo, ["d/2", "L+2", "0", "0"], BoolOp::Cut)
}

/// Fuso de esferas (eixo): cilindro `Ø d` de `z = 0` a `z = L`, com o passo `P` como parâmetro.
pub(crate) fn corpo_fuso(m: &mut Montador, fam: &Familia, t: &Tamanho, l: f64) -> Result<(), String> {
    m.param("P", &medida(fam, t, "P")?.to_string())?;
    m.param("L", &l.to_string())?;
    let base = m.plano("Plano_base", "0")?;
    m.prim("Eixo", PrimKind::Cilindro, base, ["d/2", "L", "0", "0"], BoolOp::Add)
}

/// Área de um círculo de raio `r` sem os dois segmentos além das cordas a `±h` do centro.
pub fn area_com_faces(r: f64, h: f64) -> f64 {
    let seg = r * r * (h / r).acos() - h * (r * r - h * h).sqrt();
    std::f64::consts::PI * r * r - 2. * seg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biblioteca::{familia, tamanho, TipoFixador};
    use crate::fixadores::fixador_documento;
    use crate::{recompute, Status};
    use fenkra_core::Document;
    use std::f64::consts::PI;

    #[test]
    fn trilhos_hgr_conferidos() {
        let f = familia("hiwin_hgr").unwrap();
        assert_eq!((f.tipo, f.nivel.as_str(), f.fontes.len()), (TipoFixador::TrilhoLinear, "C", 3));
        assert_eq!(f.tamanhos.iter().map(|t| t.designacao.as_str()).collect::<Vec<_>>(), ["HGR15R", "HGR20R", "HGR25R", "HGR30R", "HGR35R", "HGR45R", "HGR55R", "HGR65R"]);
        assert!(f.tamanhos.iter().all(|t| t.confirmado_por >= 2 && t.valor("D") > t.valor("dh") && t.valor("h") < t.valor("HR")));
        let t = tamanho("hiwin_hgr", "HGR25R").unwrap();
        assert_eq!((t.valor("WR"), t.valor("HR"), t.valor("P")), (Some(23.), Some(22.), Some(60.)));
        assert_eq!(furos_no_trilho(1000., 60.), 16);
        assert_eq!(furos_no_trilho(50., 60.), 1);
    }

    #[test]
    fn trilhos_grandes_patins_e_porcas_conferidos() {
        let f = familia("hiwin_hgr").unwrap();
        let t = tamanho("hiwin_hgr", "HGR65R").unwrap();
        assert_eq!((t.valor("WR"), t.valor("HR"), t.valor("D"), t.valor("h"), t.valor("dh"), t.valor("P")), (Some(63.), Some(53.), Some(26.), Some(22.), Some(18.), Some(150.)));
        assert_eq!(f.tamanhos.len(), 8);
        // cada patim cabe no trilho do mesmo tamanho: N = (W - WR)/2 e o trilho do bloco é o da família de trilhos
        let p = familia("hiwin_hgh").unwrap();
        assert_eq!((p.tipo, p.fontes.len()), (TipoFixador::PatimLinear, 2));
        assert_eq!(p.tamanhos.len(), 15);
        for b in &p.tamanhos {
            let v = |c: &str| b.valor(c).unwrap();
            assert!(b.confirmado_por >= 2 && ((v("W") - v("WR")) / 2. - v("N")).abs() < 1e-9, "{}", b.designacao);
            let r = tamanho("hiwin_hgr", &format!("HGR{}R", b.diametro_nominal().unwrap())).unwrap();
            assert_eq!((r.valor("WR"), r.valor("HR")), (Some(v("WR")), Some(v("HR"))), "{}", b.designacao);
        }
        // B17-dados: L do HGH15CA fica 61,4 (fabricante, LM76 e distribuidor), não 60,5
        let b15 = tamanho("hiwin_hgh", "HGH15CA").unwrap();
        assert_eq!((b15.valor("L"), b15.valor("L1")), (Some(61.4), Some(39.4)));
        let b = tamanho("hiwin_hgh", "HGH45HA").unwrap();
        assert_eq!((b.valor("H"), b.valor("W"), b.valor("C"), b.valor("L")), (Some(70.), Some(86.), Some(80.), Some(171.2)));
        // porca DIN 69051-5 32x10: D 50, flange 80 × 12, faces a 62, L 90
        let n = tamanho("din69051_porca", "32x10").unwrap();
        assert_eq!((n.valor("D"), n.valor("A"), n.valor("B"), n.valor("H"), n.valor("L")), (Some(50.), Some(80.), Some(12.), Some(62.), Some(90.)));
        // todo fuso tem porca do mesmo diâmetro e passo
        for s in &familia("din69051_fuso").unwrap().tamanhos {
            assert!(tamanho("din69051_porca", &s.designacao).is_some(), "{}", s.designacao);
        }
    }

    fn medir(doc: &Document) -> (f64, [f64; 3], [f64; 3]) {
        let rc = recompute(doc);
        assert!(rc.param_error.is_none(), "{:?}", rc.param_error);
        assert!(doc.features.iter().all(|f| rc.status[&f.id] == Status::Ok), "{}", doc.name);
        assert_eq!(rc.bodies.len(), 1);
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for p in rc.bodies[0].mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i] as f64);
                hi[i] = hi[i].max(p[i] as f64);
            }
        }
        (rc.bodies[0].mass.volume, lo, hi)
    }

    #[test]
    fn patim_gera_bloco_com_canal_e_furos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for t in &familia("hiwin_hgh").unwrap().tamanhos {
            let v = |c: &str| t.valor(c).unwrap();
            let (vol, lo, hi) = medir(&fixador_documento("hiwin_hgh", &t.designacao, None).unwrap());
            assert!((hi[0] - lo[0] - v("L")).abs() < 1e-3 && (hi[1] - lo[1] - v("W")).abs() < 1e-3, "{}", t.designacao);
            assert!((lo[2] - v("H1")).abs() < 1e-3 && (hi[2] - v("H")).abs() < 1e-3, "{}", t.designacao);
            let esperado = v("L") * (v("W") * (v("H") - v("H1")) - v("WR") * (v("HR") - v("H1"))) - 4. * PI / 4. * v("M").powi(2) * v("prof");
            assert!((vol - esperado).abs() / esperado < 1e-3, "{}: volume {vol} contra {esperado}", t.designacao);
        }
    }

    #[test]
    fn porca_de_esferas_tem_flange_com_faces_e_fuso_segue_o_comprimento() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for t in &familia("din69051_porca").unwrap().tamanhos {
            let (d, v) = (t.diametro_nominal().unwrap(), |c: &str| t.valor(c).unwrap());
            let (vol, lo, hi) = medir(&fixador_documento("din69051_porca", &t.designacao, None).unwrap());
            assert!((lo[2] + v("L")).abs() < 1e-3 && hi[2].abs() < 1e-3, "{}", t.designacao);
            assert!((hi[0] - lo[0] - v("A")).abs() < 0.02 * v("A") && (hi[1] - lo[1] - v("H")).abs() < 1e-3, "{}", t.designacao);
            let esperado = PI / 4. * v("D").powi(2) * (v("L") - v("B")) + area_com_faces(v("A") / 2., v("H") / 2.) * v("B") - PI / 4. * d * d * v("L");
            assert!((vol - esperado).abs() / esperado < 2e-3, "{}: volume {vol} contra {esperado}", t.designacao);
        }
        let (vol, lo, hi) = medir(&fixador_documento("din69051_fuso", "25x5", Some(600.)).unwrap());
        assert!(lo[2].abs() < 1e-3 && (hi[2] - 600.).abs() < 1e-3);
        assert!((vol - PI / 4. * 625. * 600.).abs() / vol < 1e-3);
        assert!(fixador_documento("din69051_fuso", "25x5", Some(5000.)).is_err());
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
