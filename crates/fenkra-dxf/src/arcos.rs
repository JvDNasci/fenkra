// SPDX-License-Identifier: LGPL-2.1-or-later
//! Arcos dentro de polilinhas. O desenho e a remoção de linhas ocultas entregam toda curva como pontos;
//! gravar isso como retas faz o programa de corte andar em facetas. Aqui os trechos que são arco de
//! verdade voltam a ser `ARC` e `CIRCLE`.
//!
//! Um polígono regular também tem os vértices num círculo, e o retângulo tem os quatro. O que separa
//! arco amostrado de polígono é o passo: quem amostra curva faz cordas curtas, com flecha na ordem da
//! tolerância. Por isso cada corda precisa virar pouco e ter flecha pequena.

use crate::{Entidade, Vertice, P2};

/// Distância que um ponto pode ficar do círculo, em mm de folha.
pub const TOLERANCIA_ARCO: f64 = 0.01;

/// Maior giro por corda, em radianos. A amostragem do desenho usa bem menos que isso.
const GIRO_MAXIMO: f64 = 0.45;
/// Flecha máxima de cada corda, em múltiplos da tolerância.
const FLECHA: f64 = 5.;
/// Menor número de cordas de um arco. Com menos, qualquer canto arredondado à toa viraria arco.
const CORDAS_MINIMAS: usize = 3;

fn distancia(a: P2, b: P2) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Círculo pelos três pontos; `None` se estão em linha.
fn circulo(a: P2, b: P2, c: P2) -> Option<(P2, f64)> {
    let d = 2. * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() < 1e-12 {
        return None;
    }
    let (qa, qb, qc) = (a[0] * a[0] + a[1] * a[1], b[0] * b[0] + b[1] * b[1], c[0] * c[0] + c[1] * c[1]);
    let x = (qa * (b[1] - c[1]) + qb * (c[1] - a[1]) + qc * (a[1] - b[1])) / d;
    let y = (qa * (c[0] - b[0]) + qb * (a[0] - c[0]) + qc * (b[0] - a[0])) / d;
    Some(([x, y], distancia([x, y], a)))
}

/// Ângulo em graus entre 0 e 360.
fn angulo(c: P2, p: P2) -> f64 {
    let g = (p[1] - c[1]).atan2(p[0] - c[0]).to_degrees().rem_euclid(360.);
    // um arredondamento abaixo de zero daria 360: o mesmo lugar, mas com a varredura trocada
    if 360. - g < 1e-9 {
        0.
    } else {
        g
    }
}

/// Giro com sinal de `a` para `b` em torno de `c`, de -PI a PI.
fn giro(c: P2, a: P2, b: P2) -> f64 {
    let (ux, uy, vx, vy) = (a[0] - c[0], a[1] - c[1], b[0] - c[0], b[1] - c[1]);
    (ux * vy - uy * vx).atan2(ux * vx + uy * vy)
}

/// Os pontos cabem no círculo como arco amostrado? Devolve o giro total, com sinal.
fn no_circulo(pts: &[P2], c: P2, r: f64, tol: f64) -> Option<f64> {
    if pts.iter().any(|p| (distancia(*p, c) - r).abs() > tol) {
        return None;
    }
    let (mut total, mut sinal) = (0., 0f64);
    for par in pts.windows(2) {
        let g = giro(c, par[0], par[1]);
        let flecha = r * (1. - (g / 2.).cos());
        if g == 0. || g.abs() > GIRO_MAXIMO || flecha > FLECHA * tol || sinal * g < 0. {
            return None;
        }
        sinal = g.signum();
        total += g;
    }
    Some(total)
}

/// Maior arco que começa em `pts[0]`: quantos pontos ele usa, com centro, raio e giro.
fn maior_arco(pts: &[P2], tol: f64) -> Option<(usize, P2, f64, f64)> {
    let mut achado = None;
    for fim in (CORDAS_MINIMAS + 1)..=pts.len() {
        let trecho = &pts[..fim];
        let Some((c, r)) = circulo(trecho[0], trecho[fim / 2], trecho[fim - 1]) else { break };
        match no_circulo(trecho, c, r, tol) {
            Some(g) => achado = Some((fim, c, r, g)),
            None if achado.is_some() => break,
            None if fim > CORDAS_MINIMAS + 1 => break,
            None => {}
        }
    }
    achado
}

fn descarregar(reta: &mut Vec<P2>, saida: &mut Vec<Entidade>) {
    match reta.len() {
        0 | 1 => {}
        2 => saida.push(Entidade::Linha(reta[0], reta[1])),
        _ => saida.push(Entidade::Polilinha { vertices: reta.iter().map(|p| Vertice { p: *p, bojo: 0. }).collect(), fechada: false }),
    }
    reta.clear();
}

/// Troca os trechos de arco de uma polilinha por `Arco` e `Circulo`; o resto sai como reta.
pub fn reconhecer(pts: &[P2], tol: f64) -> Vec<Entidade> {
    let mut saida = Vec::new();
    let fechada = pts.len() > 2 && distancia(pts[0], pts[pts.len() - 1]) <= tol;
    if fechada && pts.len() > CORDAS_MINIMAS + 1 {
        let n = pts.len() - 1;
        if let Some((c, r)) = circulo(pts[0], pts[n / 3], pts[2 * n / 3]) {
            if no_circulo(pts, c, r, tol).is_some_and(|g| (g.abs() - std::f64::consts::TAU).abs() < 1e-6) {
                return vec![Entidade::Circulo { centro: c, raio: r }];
            }
        }
    }
    let (mut i, mut reta): (usize, Vec<P2>) = (0, vec![]);
    while i < pts.len() {
        match maior_arco(&pts[i..], tol) {
            Some((n, c, r, g)) => {
                reta.push(pts[i]);
                descarregar(&mut reta, &mut saida);
                let (a, b) = (pts[i], pts[i + n - 1]);
                let (ini, fim) = if g > 0. { (angulo(c, a), angulo(c, b)) } else { (angulo(c, b), angulo(c, a)) };
                saida.push(Entidade::Arco { centro: c, raio: r, ini, fim });
                i += n - 1;
                if i == pts.len() - 1 {
                    break;
                }
            }
            None => {
                reta.push(pts[i]);
                i += 1;
            }
        }
    }
    descarregar(&mut reta, &mut saida);
    saida
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::f64::consts::{PI, TAU};

    fn amostra(c: P2, r: f64, de: f64, ate: f64, n: usize) -> Vec<P2> {
        (0..=n).map(|i| de + (ate - de) * i as f64 / n as f64).map(|t| [c[0] + r * t.cos(), c[1] + r * t.sin()]).collect()
    }

    #[test]
    fn circulo_amostrado_volta_a_ser_circulo() {
        let v = reconhecer(&amostra([3., 4.], 10., 0., TAU, 72), TOLERANCIA_ARCO);
        let [Entidade::Circulo { centro, raio }] = v[..] else { panic!("{v:?}") };
        assert!((raio - 10.).abs() < 1e-9 && (centro[0] - 3.).abs() < 1e-9 && (centro[1] - 4.).abs() < 1e-9);
    }

    #[test]
    fn arco_amostrado_guarda_o_sentido_nas_duas_direcoes() {
        let horario: Vec<P2> = amostra([0., 0.], 5., 0., PI / 2., 20).into_iter().rev().collect();
        for pts in [amostra([0., 0.], 5., 0., PI / 2., 20), horario] {
            let v = reconhecer(&pts, TOLERANCIA_ARCO);
            let [Entidade::Arco { raio, ini, fim, .. }] = v[..] else { panic!("{v:?}") };
            assert!((raio - 5.).abs() < 1e-9 && ini.abs() < 1e-6 && (fim - 90.).abs() < 1e-6, "{ini} {fim}");
        }
    }

    #[test]
    fn retangulo_e_poligono_nao_viram_circulo() {
        let quadrado = [[0., 0.], [10., 0.], [10., 10.], [0., 10.], [0., 0.]];
        assert!(reconhecer(&quadrado, TOLERANCIA_ARCO).iter().all(|e| matches!(e, Entidade::Polilinha { .. } | Entidade::Linha(..))));
        // dodecágono de raio 50: vértices num círculo, mas com flecha de quase 2 mm por lado
        let v = reconhecer(&amostra([0., 0.], 50., 0., TAU, 12), TOLERANCIA_ARCO);
        assert!(v.iter().all(|e| !matches!(e, Entidade::Circulo { .. } | Entidade::Arco { .. })), "{v:?}");
    }

    #[test]
    fn rasgo_vira_reta_arco_reta_arco() {
        let mut p = vec![[0., 0.], [20., 0.]];
        p.extend(amostra([20., 5.], 5., -PI / 2., PI / 2., 24).into_iter().skip(1));
        p.push([0., 10.]);
        p.extend(amostra([0., 5.], 5., PI / 2., 3. * PI / 2., 24).into_iter().skip(1));
        let v = reconhecer(&p, TOLERANCIA_ARCO);
        let tipos: Vec<&str> = v.iter().map(|e| if matches!(e, Entidade::Arco { .. }) { "arco" } else { "reta" }).collect();
        assert_eq!(tipos, ["reta", "arco", "reta", "arco"], "{v:?}");
    }

    #[test]
    fn linha_reta_com_muitos_pontos_nao_inventa_arco() {
        let p: Vec<P2> = (0..30).map(|i| [i as f64, 2. * i as f64]).collect();
        assert!(reconhecer(&p, TOLERANCIA_ARCO).iter().all(|e| !matches!(e, Entidade::Arco { .. })));
    }
}
