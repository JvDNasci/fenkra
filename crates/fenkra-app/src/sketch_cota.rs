// SPDX-License-Identifier: LGPL-2.1-or-later
//! A posição da cota escolhe o que ela mede (B96).
//!
//! Círculo e arco: dentro é raio, fora é diâmetro. Duas linhas: elas dividem o plano em quatro
//! quadrantes; os dois que contêm o ângulo entre as pontas medem esse ângulo, e os outros dois medem
//! o suplemento. O valor troca junto, e a geometria não se mexe.

use crate::app::fmt;
use crate::sketch_relimita::{traco, Traco};
use crate::sketch_ui::angle_vertex;
use fenkra_core::sketch::Kind;
use fenkra_core::SketchData;
use std::f64::consts::{PI, TAU};

type P = [f64; 2];

fn numero(v: &str) -> Option<f64> {
    v.trim().replace(',', ".").parse().ok()
}

/// Aplica a conta ao valor: número vira número, expressão fica guardada, vazio fica vazio.
fn conta(v: &str, num: impl Fn(f64) -> f64, expr: impl Fn(&str) -> String) -> String {
    match (v.trim().is_empty(), numero(v)) {
        (true, _) => String::new(),
        (_, Some(x)) => fmt(num(x)),
        _ => expr(v.trim()),
    }
}

/// O cursor está no quadrante do ângulo entre as pontas das linhas (ou no oposto a ele)?
fn no_angulo_das_pontas(v: P, e1: P, e2: P, cursor: P) -> Option<bool> {
    let ang = |q: P| (q[1] - v[1]).atan2(q[0] - v[0]);
    let (a1, a2, c) = (ang(e1), ang(e2), ang(cursor));
    // sentido curto de a1 até a2, e a posição do cursor nessa mesma medida
    let mut d = (a2 - a1).rem_euclid(TAU);
    let mut u = (c - a1).rem_euclid(TAU);
    if d > PI {
        d = TAU - d;
        u = (TAU - u).rem_euclid(TAU);
    }
    if d < 1e-6 || PI - d < 1e-6 {
        return None;
    }
    Some(u <= d || (u >= PI && u <= PI + d))
}

/// A cota que o cursor pede no lugar de `k`, se for outra.
pub fn pelo_cursor(sk: &SketchData, k: &Kind, cursor: P) -> Option<Kind> {
    match k {
        Kind::Radius(c, v) | Kind::Diameter(c, v) => {
            let Traco::Arco { c: ctr, r, .. } = traco(sk, *c)? else { return None };
            let dentro = (cursor[0] - ctr[0]).hypot(cursor[1] - ctr[1]) < r;
            match (k, dentro) {
                (Kind::Radius(..), false) => Some(Kind::Diameter(*c, conta(v, |x| 2. * x, |e| format!("2*({e})")))),
                (Kind::Diameter(..), true) => Some(Kind::Radius(*c, conta(v, |x| x / 2., |e| format!("({e})/2")))),
                _ => None,
            }
        }
        Kind::Angle(a, b, v) | Kind::AngleSupp(a, b, v) => {
            let (vtx, e1, e2) = angle_vertex(sk, *a, *b)?;
            let nas_pontas = no_angulo_das_pontas(vtx, e1, e2, cursor)?;
            let d = |p: P| [p[0] - vtx[0], p[1] - vtx[1]];
            let (r, s) = (d(e1), d(e2));
            let entre_pontas = (r[0] * s[1] - r[1] * s[0]).abs().atan2(r[0] * s[0] + r[1] * s[1]).to_degrees();
            let quer = if nas_pontas { entre_pontas } else { 180. - entre_pontas };
            // `Angle` mede as linhas no sentido em que foram criadas: pode já ser o suplemento
            let direto = sk.measure(&Kind::Angle(*a, *b, String::new()))?;
            let suplemento = (direto - quer).abs() > (180. - direto - quer).abs();
            let troca = |e: &str| format!("180-({e})");
            match (k, suplemento) {
                (Kind::Angle(..), true) => Some(Kind::AngleSupp(*a, *b, conta(v, |x| 180. - x, troca))),
                (Kind::AngleSupp(..), false) => Some(Kind::Angle(*a, *b, conta(v, |x| 180. - x, troca))),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Setor em que o arco do ângulo é desenhado: o que contém o texto. Em ângulos da tela (radianos).
/// Devolve o início e a abertura.
pub fn setor(a1: f32, a2: f32, texto: f32) -> (f32, f32) {
    use std::f32::consts::{PI, TAU};
    let mut raios: Vec<f32> = [a1, a1 + PI, a2, a2 + PI].iter().map(|x| x.rem_euclid(TAU)).collect();
    raios.sort_by(f32::total_cmp);
    let t = texto.rem_euclid(TAU);
    for i in 0..4 {
        let (ini, fim) = (raios[i], raios[(i + 1) % 4]);
        let aberto = (fim - ini).rem_euclid(TAU);
        if (t - ini).rem_euclid(TAU) <= aberto {
            return (ini, aberto);
        }
    }
    (a1, (a2 - a1).rem_euclid(TAU))
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_core::sketch::Curve;
    use fenkra_core::Id;
    use std::collections::HashMap;

    fn ids() -> impl FnMut() -> Id {
        let mut n = 7000u64;
        move || {
            n += 1;
            Id(n)
        }
    }

    #[test]
    fn dentro_e_raio_e_fora_e_diametro() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let c = sk.add_point(&mut al, 0., 0.);
        let k = sk.add_curve(&mut al, Curve::Circle { c, r: 10. }, false);
        assert_eq!(pelo_cursor(&sk, &Kind::Diameter(k, "20".into()), [3., 0.]), Some(Kind::Radius(k, "10".into())));
        assert_eq!(pelo_cursor(&sk, &Kind::Radius(k, "10".into()), [30., 0.]), Some(Kind::Diameter(k, "20".into())));
        // já é o que o cursor pede
        assert_eq!(pelo_cursor(&sk, &Kind::Radius(k, "10".into()), [3., 0.]), None);
        assert_eq!(pelo_cursor(&sk, &Kind::Diameter(k, "20".into()), [30., 0.]), None);
        // expressão continua expressão; prévia sem valor continua sem valor
        assert_eq!(pelo_cursor(&sk, &Kind::Radius(k, "a+1".into()), [30., 0.]), Some(Kind::Diameter(k, "2*(a+1)".into())));
        assert_eq!(pelo_cursor(&sk, &Kind::Diameter(k, "d".into()), [3., 0.]), Some(Kind::Radius(k, "(d)/2".into())));
        assert_eq!(pelo_cursor(&sk, &Kind::Diameter(k, String::new()), [3., 0.]), Some(Kind::Radius(k, String::new())));
    }

    /// Duas linhas saindo da origem: uma pelo eixo x e outra a 60°.
    fn v(inverte: bool) -> (SketchData, Id, Id) {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let o = sk.add_point(&mut al, 0., 0.);
        let p = sk.add_point(&mut al, 20., 0.);
        let q = sk.add_point(&mut al, 10., 10. * 3f64.sqrt());
        let a = sk.add_curve(&mut al, Curve::Line { a: o, b: p }, false);
        let b = sk.add_curve(&mut al, if inverte { Curve::Line { a: q, b: o } } else { Curve::Line { a: o, b: q } }, false);
        (sk, a, b)
    }

    #[test]
    fn quadrante_escolhe_o_angulo() {
        let (sk, a, b) = v(false);
        let k = Kind::Angle(a, b, "60".into());
        // entre as pontas e no oposto: 60°
        assert_eq!(pelo_cursor(&sk, &k, [10., 5.]), None);
        assert_eq!(pelo_cursor(&sk, &k, [-10., -5.]), None);
        // nos dois de lado: 120°
        let s = pelo_cursor(&sk, &k, [-5., 10.]).unwrap();
        assert_eq!(s, Kind::AngleSupp(a, b, "120".into()));
        assert!((sk.measure(&s).unwrap() - 120.).abs() < 1e-9);
        assert_eq!(pelo_cursor(&sk, &k, [10., -5.]).unwrap(), s);
        // e volta
        assert_eq!(pelo_cursor(&sk, &s, [10., 5.]), Some(Kind::Angle(a, b, "60".into())));
    }

    #[test]
    fn quadrante_vale_com_a_linha_no_outro_sentido() {
        // a→b da segunda linha chega no vértice: `Angle` mede 120 onde as pontas abrem 60
        let (sk, a, b) = v(true);
        assert!((sk.measure(&Kind::Angle(a, b, String::new())).unwrap() - 120.).abs() < 1e-9);
        let s = pelo_cursor(&sk, &Kind::Angle(a, b, String::new()), [10., 5.]).unwrap();
        assert!(matches!(s, Kind::AngleSupp(..)));
        assert!((sk.measure(&s).unwrap() - 60.).abs() < 1e-9);
    }

    #[test]
    fn cota_trocada_comanda_sem_mexer_na_geometria() {
        let (mut sk, a, b) = v(false);
        let mut al = ids();
        for _ in 0..50 {
            al();
        }
        let k = pelo_cursor(&sk, &Kind::Angle(a, b, "60".into()), [-5., 10.]).unwrap();
        let id = sk.add_constraint(&mut al, k);
        let antes = sk.points.clone();
        assert!(sk.solve(&HashMap::new(), &[]).converged);
        for (x, y) in antes.iter().zip(&sk.points) {
            assert!((x.x - y.x).abs() < 1e-6 && (x.y - y.y).abs() < 1e-6);
        }
        // pedir 90 no suplemento abre as linhas para 90
        *sk.constraints.iter_mut().find(|c| c.id == id).unwrap().kind.value_mut().unwrap() = "100".into();
        assert!(sk.solve(&HashMap::new(), &[]).converged);
        assert!((sk.measure(&Kind::Angle(a, b, String::new())).unwrap() - 80.).abs() < 1e-6);
    }

    #[test]
    fn setor_e_o_que_contem_o_texto() {
        let (a1, a2) = (0f32, 1f32);
        let (i, ab) = setor(a1, a2, 0.5);
        assert!(i.abs() < 1e-6 && (ab - 1.).abs() < 1e-6);
        let (i, ab) = setor(a1, a2, 2.);
        assert!((i - 1.).abs() < 1e-6 && (ab - (std::f32::consts::PI - 1.)).abs() < 1e-5);
        // passando pelo zero
        let (i, ab) = setor(a1, a2, -0.5);
        assert!((i - (1. + std::f32::consts::PI)).abs() < 1e-5 && (ab - (std::f32::consts::PI - 1.)).abs() < 1e-5);
    }
}
