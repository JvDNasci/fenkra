// SPDX-License-Identifier: LGPL-2.1-or-later
//! Linha de dobra desenhada na face (B243): geometria pura (sem kernel) que a interface usa para
//! mostrar e editar a linha da dobra por linha, do joggle e da veneziana.
//!
//! Como o kernel lê a linha: `e = unit(b - a)`, `n` é a normal externa da face no meio da linha, e
//! o lado que gira (dobra), desce (joggle) ou levanta (veneziana) é `cross(n, e)`, trocado de sinal
//! por `flip`. Escolher a *face fixa* é escolher, com um clique, o lado que fica parado: o outro é
//! o móvel. Por isso o documento só precisa guardar `a`, `b` e `flip`; a face fixa é a que contém
//! o lado oposto ao do `flip`.

use crate::chapa::Compensacao;
use crate::geo3d::{self, V3};

/// Direção, no plano da face e perpendicular à linha, do lado que se move.
pub fn lado_movel(a: V3, b: V3, normal: V3, flip: bool) -> Option<V3> {
    let e = geo3d::unit(geo3d::sub(b, a))?;
    let s = geo3d::unit(fenkra_kernel::cross(normal, e))?;
    Some(if flip { geo3d::mul(s, -1.) } else { s })
}

/// `flip` que deixa parado o lado onde está `fixo` (um ponto da face fixa): o lado móvel é o oposto.
/// `None` se a linha é degenerada ou `fixo` cai sobre a linha (não decide lado).
pub fn flip_para_fixo(a: V3, b: V3, normal: V3, fixo: V3) -> Option<bool> {
    let s = lado_movel(a, b, normal, false)?;
    let mid = geo3d::lerp(a, b, 0.5);
    let d = geo3d::dot(geo3d::sub(fixo, mid), s);
    (d.abs() > 1e-6).then_some(d > 0.)
}

/// O ponto está do lado que se move?
pub fn no_lado_movel(p: V3, a: V3, b: V3, normal: V3, flip: bool) -> bool {
    lado_movel(a, b, normal, flip).is_some_and(|s| geo3d::dot(geo3d::sub(p, geo3d::lerp(a, b, 0.5)), s) > 0.)
}

/// Recorta um polígono convexo (ou um triângulo da malha da face) pela linha: devolve o pedaço do
/// lado fixo e o do lado móvel. Sutherland-Hodgman contra o semiplano da linha.
pub fn recorta_metades(poligono: &[V3], a: V3, b: V3, normal: V3, flip: bool) -> (Vec<V3>, Vec<V3>) {
    let Some(s) = lado_movel(a, b, normal, flip) else { return (poligono.to_vec(), vec![]) };
    let dist = |p: V3| geo3d::dot(geo3d::sub(p, a), s);
    let corta = |lado: f64| -> Vec<V3> {
        let mut out = vec![];
        let n = poligono.len();
        for i in 0..n {
            let (p, q) = (poligono[i], poligono[(i + 1) % n]);
            let (dp, dq) = (dist(p) * lado, dist(q) * lado);
            if dp >= 0. {
                out.push(p);
            }
            if (dp >= 0.) != (dq >= 0.) {
                let t = dp / (dp - dq);
                out.push(geo3d::lerp(p, q, t));
            }
        }
        out
    };
    (corta(-1.), corta(1.))
}

/// Pontos do arco do ângulo da dobra, em volta da linha (eixo), no plano (lado móvel, normal):
/// parte do lado móvel (0°) e gira em direção à normal (dobra para cima) ou contra ela (para baixo).
/// `raio` é o tamanho do arco desenhado, não o raio de dobra.
pub fn arco_do_angulo(a: V3, b: V3, normal: V3, flip: bool, angulo_graus: f64, raio: f64, para_baixo: bool, passos: usize) -> Vec<V3> {
    let (Some(s), Some(e)) = (lado_movel(a, b, normal, flip), geo3d::unit(geo3d::sub(b, a))) else { return vec![] };
    let centro = geo3d::lerp(a, b, 0.5);
    let sinal = if para_baixo { -1. } else { 1. };
    // a rotação que leva `s` para `normal` em torno de `eixo` tem o sinal certo?
    let eixo = fenkra_kernel::cross(s, normal);
    let eixo = geo3d::unit(eixo).unwrap_or(e);
    (0..=passos.max(1))
        .map(|i| {
            let ang = (angulo_graus * sinal * i as f64 / passos.max(1) as f64).to_radians();
            geo3d::add(centro, geo3d::mul(geo3d::rotate(s, eixo, ang), raio))
        })
        .collect()
}

/// Deslocamento da zona dobrada em relação à linha desenhada (mm, para o lado móvel), segundo a
/// posição escolhida: 0 linha de centro, 1 início da dobra (a zona vai para o lado móvel), 2 fim
/// da dobra, 3 vértice externo. É a mesma conta do cálculo da dobra.
pub fn deslocamento_posicao(posicao: u8, c: &Compensacao) -> f64 {
    match posicao {
        1 => c.ba / 2.,
        2 => -c.ba / 2.,
        3 => c.ba / 2. - c.ossb,
        _ => 0.,
    }
}

/// Move uma ponta da linha: o ponto novo é levado ao plano da face (normal `n` por `origem`), para
/// a linha nunca sair da face ao arrastar. `ponta` 0 move `a`; 1 move `b`.
pub fn move_ponta(a: V3, b: V3, ponta: u8, novo: V3, origem: V3, n: V3) -> (V3, V3) {
    let n = geo3d::unit(n).unwrap_or([0., 0., 1.]);
    let q = geo3d::sub(novo, geo3d::mul(n, geo3d::dot(geo3d::sub(novo, origem), n)));
    if ponta == 0 {
        (q, b)
    } else {
        (a, q)
    }
}

/// Encaixe (snap) de um ponto nos candidatos (vértices, meios de aresta, extremos da linha) a até
/// `tol` mm. Devolve o candidato mais perto, ou o próprio ponto se nenhum está dentro da tolerância.
pub fn encaixa(p: V3, candidatos: &[V3], tol: f64) -> (V3, bool) {
    let melhor = candidatos.iter().map(|c| (geo3d::dist(*c, p), *c)).filter(|(d, _)| *d <= tol).min_by(|x, y| x.0.total_cmp(&y.0));
    match melhor {
        Some((_, c)) => (c, true),
        None => (p, false),
    }
}

/// Setas de sentido da linha: para cada ponta, o par (base, ponta) de uma seta que sai da linha para
/// o lado móvel, com `comp` mm.
pub fn setas_de_sentido(a: V3, b: V3, normal: V3, flip: bool, comp: f64) -> Vec<(V3, V3)> {
    let Some(s) = lado_movel(a, b, normal, flip) else { return vec![] };
    [0.2, 0.5, 0.8].iter().map(|t| geo3d::lerp(a, b, *t)).map(|p| (p, geo3d::add(p, geo3d::mul(s, comp)))).collect()
}

/// Reescreve uma expressão para que passe a valer `novo`, sem perder as variáveis: número puro
/// vira o número novo; `L + 5` vira `L + 8` (só o termo numérico muda); qualquer outra vira
/// `(expr) + delta`. `atual` é o valor que a expressão tem agora.
pub fn reescreve_expr(expr: &str, atual: f64, novo: f64) -> String {
    let fmt = |v: f64| {
        let s = format!("{:.3}", (v * 1000.).round() / 1000.);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    let e = expr.trim();
    if e.replace(',', ".").parse::<f64>().is_ok() || e.is_empty() {
        return fmt(novo);
    }
    let delta = novo - atual;
    if delta.abs() < 1e-9 {
        return e.to_string();
    }
    // expressão de um só nome (ou termo) não precisa de parênteses
    let simples = !e.contains(['+', '-', '*', '/', '^', '(', ')', ' ']);
    // termo numérico no fim: "cabeça + 5" ou "cabeça - 2,5"
    if let Some(pos) = e.rfind(['+', '-']) {
        let (cabeca, cauda) = e.split_at(pos);
        let sinal = if cauda.starts_with('-') { -1. } else { 1. };
        if let Ok(v) = cauda[1..].trim().replace(',', ".").parse::<f64>() {
            let cabeca = cabeca.trim_end();
            // cabeça precisa terminar numa operando (não em outro operador)
            if !cabeca.is_empty() && !cabeca.ends_with(['+', '-', '*', '/', '(', '^']) {
                let total = sinal * v + delta;
                return if total.abs() < 1e-9 {
                    cabeca.to_string()
                } else if total > 0. {
                    format!("{cabeca} + {}", fmt(total))
                } else {
                    format!("{cabeca} - {}", fmt(-total))
                };
            }
        }
    }
    let base = if simples { e.to_string() } else { format!("({e})") };
    if delta > 0. {
        format!("{base} + {}", fmt(delta))
    } else {
        format!("{base} - {}", fmt(-delta))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn lado_movel_e_flip_para_fixo() {
        let (a, b, n) = ([60., 0., 2.], [60., 50., 2.], [0., 0., 1.]);
        // sem flip o lado móvel é -x (cross(z, y) = -x)
        assert_eq!(lado_movel(a, b, n, false).unwrap(), [-1., 0., 0.]);
        // face fixa à esquerda (x = 20): precisa mover a direita, então flip
        assert_eq!(flip_para_fixo(a, b, n, [20., 25., 2.]), Some(true));
        assert_eq!(flip_para_fixo(a, b, n, [90., 25., 2.]), Some(false));
        // sobre a linha: não decide
        assert_eq!(flip_para_fixo(a, b, n, [60., 10., 2.]), None);
        assert!(no_lado_movel([90., 10., 2.], a, b, n, true));
        assert!(!no_lado_movel([20., 10., 2.], a, b, n, true));
    }

    #[test]
    fn metades_somam_a_area_da_face() {
        let quad = [[0., 0., 2.], [100., 0., 2.], [100., 50., 2.], [0., 50., 2.]];
        let (fixa, movel) = recorta_metades(&quad, [60., 0., 2.], [60., 50., 2.], [0., 0., 1.], true);
        let area = |p: &[V3]| (0..p.len()).map(|i| p[i][0] * p[(i + 1) % p.len()][1] - p[(i + 1) % p.len()][0] * p[i][1]).sum::<f64>().abs() / 2.;
        assert!((area(&fixa) - 60. * 50.).abs() < 1e-9, "fixa {}", area(&fixa));
        assert!((area(&movel) - 40. * 50.).abs() < 1e-9, "móvel {}", area(&movel));
        assert!(movel.iter().all(|p| p[0] >= 60. - 1e-9));
    }

    #[test]
    fn arco_do_angulo_termina_no_angulo_pedido() {
        let (a, b, n) = ([60., 0., 2.], [60., 50., 2.], [0., 0., 1.]);
        let arco = arco_do_angulo(a, b, n, true, 90., 10., false, 16);
        let c = geo3d::lerp(a, b, 0.5);
        // flip: o lado móvel é +x; a 90° o arco termina na vertical (+z)
        let fim = geo3d::sub(*arco.last().unwrap(), c);
        assert!(fim[0].abs() < 1e-9 && (fim[2] - 10.).abs() < 1e-9, "{fim:?}");
        let ini = geo3d::sub(arco[0], c);
        assert!((ini[0] - 10.).abs() < 1e-9 && ini[2].abs() < 1e-9, "{ini:?}");
        // para baixo vai para -z
        let baixo = arco_do_angulo(a, b, n, true, 90., 10., true, 16);
        assert!(geo3d::sub(*baixo.last().unwrap(), c)[2] < -9.9);
    }

    #[test]
    fn encaixe_e_ponta_na_face() {
        let (q, achou) = encaixa([10.2, 4.9, 2.], &[[10., 5., 2.], [50., 5., 2.]], 0.5);
        assert!(achou && q == [10., 5., 2.]);
        assert!(!encaixa([30., 5., 2.], &[[10., 5., 2.]], 0.5).1);
        // a ponta arrastada para fora do plano volta ao plano da face
        let (a, b) = move_ponta([0., 0., 2.], [10., 0., 2.], 1, [12., 3., 9.], [0., 0., 2.], [0., 0., 1.]);
        assert_eq!((a, b), ([0., 0., 2.], [12., 3., 2.]));
    }

    #[test]
    fn reescreve_expressao_preservando_variaveis() {
        assert_eq!(reescreve_expr("90", 90., 60.), "60");
        assert_eq!(reescreve_expr("ang + 10", 40., 60.), "ang + 30");
        assert_eq!(reescreve_expr("ang + 10", 40., 30.), "ang");
        assert_eq!(reescreve_expr("ang + 10", 40., 20.), "ang - 10");
        assert_eq!(reescreve_expr("2 * esp", 4., 7.), "(2 * esp) + 3");
        assert_eq!(reescreve_expr("esp", 2., 1.5), "esp - 0.5");
        assert_eq!(reescreve_expr("esp", 2., 2.), "esp");
        assert_eq!(reescreve_expr("a - b", 1., 3.), "(a - b) + 2");
    }
}
