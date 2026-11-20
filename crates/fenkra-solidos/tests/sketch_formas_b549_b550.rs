// SPDX-License-Identifier: LGPL-2.1-or-later
//! B549 e B550: formas novas do sketch até o sólido.
//!
//! Cada forma é montada em código como a ferramenta monta, extrudada 10 mm e recalculada; o teste
//! confere status, volume e caixa envolvente. A spline editada pelo polígono de controle confere que
//! o kernel recebe a mesma curva que a tela desenha: a área do perfil vem das amostras da curva.

mod common;

use common::*;
use fenkra_core::sketch::{spline_samples, Curve, Kind};
use fenkra_core::*;
use std::collections::HashMap;

fn com_sketch(nome: &str, data: SketchData) -> Document {
    let mut d = Document::new(nome);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    d
}

fn prisma(vol: f64, lo: [f64; 2], hi: [f64; 2]) -> Esperado {
    Esperado { vol, tol_vol: 1e-6, tam: [hi[0] - lo[0], hi[1] - lo[1], 10.], tol_tam: 1e-3, minimo: Some([lo[0], lo[1], 0.]), via_malha: true }
}

#[test]
fn retangulo_por_3_pontos_inclinado_vira_prisma() {
    if !kernel_disponivel() {
        return;
    }
    let mut d0 = Document::new("ids");
    let mut data = SketchData::default();
    let mut al = || d0.alloc();
    let (s, c) = (30f64.to_radians().sin(), 30f64.to_radians().cos());
    let b = [40. * c, 40. * s];
    // o terceiro clique fica fora da perpendicular: só a distância a ela conta (altura 20)
    let alto = [b[0] - 20. * s + 7. * c, b[1] + 20. * c + 7. * s];
    data.add_rect3(&mut al, [0., 0.], b, alto, false).unwrap();
    assert!(data.solve(&HashMap::new(), &[]).converged);
    let d = com_sketch("b550_ret3", data);
    // cantos (0, 0), (34,64; 20), (24,64; 37,32) e (−10; 17,32)
    confere("retângulo por 3 pontos", &d, &prisma(40. * 20. * 10., [-20. * s, 0.], [40. * c, 40. * s + 20. * c]));
}

#[test]
fn paralelogramo_vira_prisma_de_area_base_vezes_altura() {
    if !kernel_disponivel() {
        return;
    }
    let mut d0 = Document::new("ids");
    let mut data = SketchData::default();
    let mut al = || d0.alloc();
    data.add_paralelogramo(&mut al, [0., 0.], [40., 0.], [50., 20.], false).unwrap();
    let d = com_sketch("b550_paralelogramo", data);
    confere("paralelogramo", &d, &prisma(40. * 20. * 10., [0., 0.], [50., 20.]));
}

#[test]
fn circulo_tangente_a_tres_lados_vira_cilindro_inscrito() {
    if !kernel_disponivel() {
        return;
    }
    let mut d0 = Document::new("ids");
    let mut data = SketchData::default();
    let mut al = || d0.alloc();
    // triângulo 30-40-50 de construção (fica fora do perfil); o inscrito tem raio 10 e centro (10, 10)
    let p = [[0., 0.], [40., 0.], [0., 30.]].map(|q| data.add_point(&mut al, q[0], q[1]));
    let l = [0, 1, 2].map(|i| data.add_curve(&mut al, Curve::Line { a: p[i], b: p[(i + 1) % 3] }, true));
    for q in p {
        data.add_constraint(&mut al, Kind::Fixed(q));
    }
    let circ = data.add_circulo_tangente(&mut al, l, [[20., 0.], [20., 15.], [0., 15.]], false).unwrap();
    let rep = data.solve(&HashMap::new(), &[]);
    assert!(rep.converged && rep.dof == 0, "{rep:?}");
    assert!((data.radius_of(circ).unwrap() - 10.).abs() < 1e-9);
    let d = com_sketch("b550_tangente", data);
    let vol = std::f64::consts::PI * 100. * 10.;
    confere("círculo tangente a 3", &d, &Esperado { vol, tol_vol: 1e-6, tam: [20., 20., 10.], tol_tam: 0.05, minimo: Some([0., 0., 0.]), via_malha: true });
}

#[test]
fn spline_editada_pelo_poligono_de_controle_chega_igual_ao_solido() {
    if !kernel_disponivel() {
        return;
    }
    let mut d0 = Document::new("ids");
    let mut data = SketchData::default();
    let mut al = || d0.alloc();
    let p = [[60., 0.], [45., 25.], [15., 25.], [0., 0.]].map(|q| data.add_point(&mut al, q[0], q[1]));
    data.add_curve(&mut al, Curve::Line { a: p[3], b: p[0] }, false);
    let sp = data.add_curve(&mut al, Curve::Spline { pts: p.to_vec() }, false);
    assert!(data.spline_c2(sp));
    let antes = data.spline_controle(sp);
    assert_eq!(antes.len(), 6);
    // puxa o vértice 3 do polígono para cima: a curva sobe sem passar por ele
    let rep = data.mover_controle(sp, 3, [antes[3][0], antes[3][1] + 15.], &HashMap::new()).expect("solução");
    assert!(rep.converged);
    let depois = data.spline_controle(sp);
    assert!((depois[3][1] - antes[3][1] - 15.).abs() < 1e-6);
    assert_eq!(data.pos(p[0]), [60., 0.], "as pontas ficam");
    assert_eq!(data.pos(p[3]), [0., 0.]);
    // área e caixa pela curva que a tela desenha
    let s = spline_samples(&data.spline_uv(sp), 12);
    let area = s.windows(2).map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1]).sum::<f64>().abs() / 2.;
    let topo = s.iter().map(|q| q[1]).fold(f64::MIN, f64::max);
    assert!(topo > 25., "a curva subiu com o vértice: {topo}");
    let d = com_sketch("b549_controle", data);
    confere("spline por pontos de controle", &d, &Esperado { vol: area * 10., tol_vol: 2e-3, tam: [60., topo, 10.], tol_tam: 0.2, minimo: Some([0., 0., 0.]), via_malha: true });
}
