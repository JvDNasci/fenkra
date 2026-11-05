// SPDX-License-Identifier: LGPL-2.1-or-later
//! Investigação do "serrilhado" com malha muito fina (desvio 0,0001 da diagonal, ângulo 5°).
//!
//! Mede, numa esfera e num cilindro, o que a malha entrega: contagem de triângulos, desvio real da
//! superfície, qualidade dos triângulos (menor ângulo interno) e o erro de guardar posições em f32 longe da origem.
//! Rode com `cargo test -p fenkra-solidos --test malha_fina_serrilhado -- --nocapture`.

use fenkra_core::QualidadeMalha;
use fenkra_kernel::{Shape, Xform};

const FINA: QualidadeMalha = QualidadeMalha { desvio_rel: 0.0001, angulo_graus: 5. };

struct Medida {
    triangulos: usize,
    /// maior distância entre o centro de um triângulo e a esfera exata (flecha real)
    flecha_max: f64,
    /// menor ângulo interno de todos os triângulos (graus)
    angulo_min: f64,
    /// triângulos com menor ângulo abaixo de 5°
    lascas: usize,
    /// maior ângulo (graus) entre a normal geométrica do triângulo e a normal de vértice
    normal_max: f64,
}

fn medir_esfera(centro: [f64; 3], r: f64, q: &QualidadeMalha) -> Medida {
    let s = Shape::sphere_surface(centro, r).unwrap();
    let diag = 2. * r * 3f64.sqrt();
    let m = s.mesh_angle(q.desvio(diag), q.angulo_rad()).unwrap();
    let p = |i: u32| {
        let k = 3 * i as usize;
        [m.positions[k] as f64, m.positions[k + 1] as f64, m.positions[k + 2] as f64]
    };
    let n = |i: u32| {
        let k = 3 * i as usize;
        [m.normals[k] as f64, m.normals[k + 1] as f64, m.normals[k + 2] as f64]
    };
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let cross = |a: [f64; 3], b: [f64; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let len = |a: [f64; 3]| dot(a, a).sqrt();
    let mut med = Medida { triangulos: m.indices.len() / 3, flecha_max: 0., angulo_min: 180., lascas: 0, normal_max: 0. };
    for t in m.indices.chunks_exact(3) {
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        let cen = [(a[0] + b[0] + c[0]) / 3., (a[1] + b[1] + c[1]) / 3., (a[2] + b[2] + c[2]) / 3.];
        med.flecha_max = med.flecha_max.max((len(sub(cen, centro)) - r).abs());
        let ang = |x: [f64; 3], y: [f64; 3], z: [f64; 3]| {
            let (u, v) = (sub(y, x), sub(z, x));
            (dot(u, v) / (len(u) * len(v)).max(1e-30)).clamp(-1., 1.).acos().to_degrees()
        };
        let menor = ang(a, b, c).min(ang(b, c, a)).min(ang(c, a, b));
        med.angulo_min = med.angulo_min.min(menor);
        if menor < 5. {
            med.lascas += 1;
        }
        let g = cross(sub(b, a), sub(c, a));
        let lg = len(g);
        if lg > 0. {
            let nv = n(t[0]);
            let cos = (dot(g, nv) / (lg * len(nv))).abs().clamp(0., 1.);
            med.normal_max = med.normal_max.max(cos.acos().to_degrees());
        }
    }
    med
}

#[test]
fn malha_fina_na_esfera() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    for (nome, centro) in [("na origem", [0.; 3]), ("a 5 m da origem", [5000., 0., 0.])] {
        let r = 25.;
        let med = medir_esfera(centro, r, &FINA);
        let pedido = FINA.desvio(2. * r * 3f64.sqrt());
        eprintln!(
            "esfera r=25 {nome}: {} triângulos, flecha máx {:.5} mm (pedido {:.5}), menor ângulo {:.2}°, lascas {}, normal x triângulo até {:.2}°",
            med.triangulos, med.flecha_max, pedido, med.angulo_min, med.lascas, med.normal_max
        );
        assert!(med.triangulos > 0);
    }
}

#[test]
fn posicao_f32_longe_da_origem() {
    // erro de guardar uma coordenada em f32 a diferentes distâncias da origem, contra a flecha pedida
    for x in [0.0f64, 100., 1000., 5000., 20000.] {
        let erro = ((x + 0.01) as f32 as f64 - (x + 0.01)).abs().max(((x + 0.0123) as f32 as f64 - (x + 0.0123)).abs());
        eprintln!("coordenada {x:>7} mm: quantização f32 de até ~{:.6} mm", f32::EPSILON as f64 * x.max(1.) / 2.);
        let _ = erro;
    }
    let _ = Xform::Translate([0.; 3]);
}

/// Qualidade dos triângulos em formas que não são esfera: contagem, lascas (menor ângulo abaixo de 1° e
/// de 5°) e triângulos de área nula, no ajuste fino e no Normal.
fn qualidade_da_malha(nome: &str, s: &Shape) {
    let (a, b) = s.bbox().unwrap();
    let diag = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
    for (rot, q) in [("fina 1e-4/5°", FINA), ("normal", QualidadeMalha::NORMAL)] {
        s.discard_mesh().unwrap();
        let m = s.mesh_angle(q.desvio(diag), q.angulo_rad()).unwrap();
        let p = |i: u32| {
            let k = 3 * i as usize;
            [m.positions[k] as f64, m.positions[k + 1] as f64, m.positions[k + 2] as f64]
        };
        let (mut l1, mut l5, mut nulos) = (0usize, 0usize, 0usize);
        let mut maior_razao = 0f64;
        for t in m.indices.chunks_exact(3) {
            let (x, y, z) = (p(t[0]), p(t[1]), p(t[2]));
            let d = |u: [f64; 3], v: [f64; 3]| ((u[0] - v[0]).powi(2) + (u[1] - v[1]).powi(2) + (u[2] - v[2]).powi(2)).sqrt();
            let (c1, c2, c3) = (d(y, z), d(x, z), d(x, y));
            let (maior, menor) = (c1.max(c2).max(c3), c1.min(c2).min(c3));
            if maior < 1e-9 {
                nulos += 1;
                continue;
            }
            maior_razao = maior_razao.max(maior / menor.max(1e-12));
            let ang = |o: f64, l1: f64, l2: f64| ((l1 * l1 + l2 * l2 - o * o) / (2. * l1 * l2).max(1e-30)).clamp(-1., 1.).acos().to_degrees();
            let menor_ang = ang(c1, c2, c3).min(ang(c2, c1, c3)).min(ang(c3, c1, c2));
            if menor_ang < 1. {
                l1 += 1;
            }
            if menor_ang < 5. {
                l5 += 1;
            }
        }
        let n = m.indices.len() / 3;
        eprintln!("{nome} [{rot}]: {n} triângulos, {l5} com ângulo < 5° ({:.1}%), {l1} < 1°, {nulos} de área nula, maior razão de lados {maior_razao:.0}", 100. * l5 as f64 / n.max(1) as f64);
    }
}

#[test]
fn qualidade_dos_triangulos_em_superficies_curvas() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    qualidade_da_malha("cilindro", &Shape::cylinder_surface([0.; 3], [0., 0., 1.], 10., 0., 30.).unwrap());
    qualidade_da_malha("esfera", &Shape::sphere_surface([0.; 3], 25.).unwrap());
    let perfil: Vec<[f64; 3]> = (0..8).map(|i| [10. + 6. * (i as f64 * 0.7).sin(), 0., i as f64 * 5.]).collect();
    qualidade_da_malha("revolução de spline (B-spline)", &Shape::revolve_curve(&perfil, [0.; 3], [0., 0., 1.], 360.).unwrap());
    let onda: Vec<[f64; 3]> = (0..8).map(|i| [i as f64 * 6., 5. * (i as f64 * 0.9).sin(), 0.]).collect();
    qualidade_da_malha("extrusão de spline (B-spline)", &Shape::extrude_curve(&onda, [0., 0., 1.], 30.).unwrap());
}
