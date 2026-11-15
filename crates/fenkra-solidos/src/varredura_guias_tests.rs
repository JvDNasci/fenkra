// SPDX-License-Identifier: LGPL-2.1-or-later
//! B40: varredura cônica por três guias, montada no documento e recalculada.

use super::*;
use crate::geo3d::{cross, dist, sub};
use crate::varredura_guias::*;

fn s(x: &str) -> String {
    x.to_string()
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }))
}

fn varre(d: &mut Document, g: [&CurveRef; 3], rho: &str, raio: &str) -> Id {
    d.add_geometry(RefKind::SurfSweepX {
        kind: SweepKind::ConicGuides { guide1: g[0].clone(), guide2: g[1].clone(), apex: g[2].clone(), rho: s(rho), radius: s(raio) },
        spine: None,
        solution: 0,
        smooth: false,
        sections: s("12"),
    })
}

/// (área, caixa mín, caixa máx) da superfície
fn medir(r: &RecomputeResult, id: Id) -> (f64, [f64; 3], [f64; 3]) {
    assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{:?}", r.status[&id]);
    let m = &r.surfaces.iter().find(|v| v.id == id).expect("superfície").mesh;
    let p = |i: u32| [m.positions[i as usize * 3] as f64, m.positions[i as usize * 3 + 1] as f64, m.positions[i as usize * 3 + 2] as f64];
    let area = m.indices.chunks_exact(3).map(|t| norm(cross(sub(p(t[1]), p(t[0])), sub(p(t[2]), p(t[0])))) / 2.).sum();
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for i in 0..m.positions.len() / 3 {
        let q = p(i as u32);
        for j in 0..3 {
            lo[j] = lo[j].min(q[j]);
            hi[j] = hi[j].max(q[j]);
        }
    }
    (area, lo, hi)
}

fn comprimento(p: &[V3]) -> f64 {
    p.windows(2).map(|w| dist(w[0], w[1])).sum()
}

use crate::geo3d::norm;

#[test]
fn conica_por_tres_guias_tem_area_e_caixa_da_parabola() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("conica3");
    let g1 = linha(&mut d, [0., 0., 0.], [0., 0., 100.]);
    let g2 = linha(&mut d, [20., 0., 0.], [20., 0., 100.]);
    let ap = linha(&mut d, [10., 8., 0.], [10., 8., 100.]);
    let id = varre(&mut d, [&g1, &g2, &ap], "0.5", "");
    let r = recompute(&d);
    let (area, lo, hi) = medir(&r, id);
    // a seção é uma parábola de P0=(0,0), P1=(10,8), P2=(20,0): flecha 4 no meio
    let esperado = comprimento(&secao_conica([0., 0., 0.], [10., 8., 0.], [20., 0., 0.], 0.5, 400)) * 100.;
    assert!((area - esperado).abs() < 0.01 * esperado, "área {area} contra {esperado}");
    assert!((hi[1] - 4.).abs() < 0.05 && lo[1].abs() < 1e-6, "flecha da parábola: {lo:?} {hi:?}");
    assert!(lo[0].abs() < 1e-6 && (hi[0] - 20.).abs() < 1e-6 && lo[2].abs() < 1e-6 && (hi[2] - 100.).abs() < 1e-6, "caixa {lo:?} {hi:?}");
}

#[test]
fn raio_na_guia_1_da_a_curvatura_exata_e_muda_a_forma() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("conica_g2");
    let g1 = linha(&mut d, [0., 0., 0.], [0., 0., 100.]);
    let g2 = linha(&mut d, [20., 0., 0.], [20., 0., 100.]);
    let ap = linha(&mut d, [10., 8., 0.], [10., 8., 100.]);
    let a = varre(&mut d, [&g1, &g2, &ap], "0.5", "");
    let b = varre(&mut d, [&g1, &g2, &ap], "0.5", "6");
    let r = recompute(&d);
    let (aa, _, _) = medir(&r, a);
    let (ab, _, hb) = medir(&r, b);
    assert!((aa - ab).abs() > 0.5, "o raio pedido precisa mudar a superfície: {aa} contra {ab}");
    // o ρ calculado reproduz exatamente o raio de 6 mm na guia 1
    let rho = rho_para_raio([0., 0., 0.], [10., 8., 0.], [20., 0., 0.], 6.).unwrap();
    assert!((1. / curvatura_na_ponta([0., 0., 0.], [10., 8., 0.], [20., 0., 0.], rho) - 6.).abs() < 1e-9);
    // com ρ maior a curva chega mais perto do ápice: a flecha passa de 4
    assert!(hb[1] > 0. && hb[1] < 8.);
}

#[test]
fn apice_inclinado_varia_sem_salto_de_tangente_entre_secoes() {
    // guias de ápice que se afasta ao longo de z: as seções vizinhas mudam devagar (G1 entre elas)
    let secao = |z: f64| secao_conica([0., 0., z], [10., 8. + 0.04 * z, z], [20., 0., z], 0.5, 33);
    let mut maior = 0f64;
    for k in 0..100 {
        maior = maior.max(salto_de_tangente(&secao(k as f64), &secao(k as f64 + 1.)));
    }
    assert!(maior < 0.2, "salto de tangente entre seções vizinhas: {maior}°");
    // e o ponto médio varia de forma contínua (sem degrau)
    let meio = |z: f64| secao(z)[16];
    for k in 0..100 {
        assert!(dist(meio(k as f64), meio(k as f64 + 1.)) < 1.1, "degrau na superfície em z={k}");
    }
}

#[test]
fn guias_alinhadas_dao_erro_claro() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("conica_ruim");
    let g1 = linha(&mut d, [0., 0., 0.], [0., 0., 100.]);
    let g2 = linha(&mut d, [20., 0., 0.], [20., 0., 100.]);
    let ap = linha(&mut d, [10., 0., 0.], [10., 0., 100.]);
    let id = varre(&mut d, [&g1, &g2, &ap], "0.5", "6");
    let r = recompute(&d);
    assert!(matches!(r.status[&id], Status::Error(_)), "{:?}", r.status[&id]);
}
