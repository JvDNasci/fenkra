// SPDX-License-Identifier: LGPL-2.1-or-later
//! Contas dos manipuladores 3D da anotação (B269): encaixe em eixos do modelo, distâncias de
//! 1 mm e ângulos de 15°, mais o que cada alça grava a partir do ponto onde o cursor caiu.
//!
//! A interface leva o raio do cursor até o plano de trabalho da anotação e entrega aqui o ponto;
//! tudo o que sai são valores do modelo (mm), independentes da câmera. `shift` desliga o encaixe.

type P = [f64; 3];

/// Passo de distância do encaixe.
pub const STEP_MM: f64 = 1.0;
/// Passo angular do encaixe.
pub const STEP_DEG: f64 = 15.0;
/// Dentro desta folga angular a direção cola no eixo do modelo.
const AXIS_TOL_DEG: f64 = 7.5;

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: P, b: P) -> P {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: P, k: f64) -> P {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: P, b: P) -> P {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn len(a: P) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: P) -> Option<P> {
    let l = len(a);
    (l > 1e-12).then(|| mul(a, 1. / l))
}

/// Distância arredondada ao milímetro, a menos que `shift` esteja apertado.
pub fn snap_mm(v: f64, shift: bool) -> f64 {
    if shift {
        v
    } else {
        (v / STEP_MM).round() * STEP_MM
    }
}

/// Cola a direção num eixo do modelo (±X, ±Y, ±Z) quando está a menos de 7,5° dele.
pub fn snap_axis(dir: P, shift: bool) -> P {
    let Some(d) = unit(dir) else { return dir };
    if shift {
        return d;
    }
    for k in 0..3 {
        let c = d[k];
        if c.abs() >= AXIS_TOL_DEG.to_radians().cos() {
            let mut e = [0.; 3];
            e[k] = c.signum();
            return e;
        }
    }
    d
}

/// Afastamento da linha de cota: o deslocamento (perpendicular à linha entre `a0` e `a1`, dentro
/// do plano de normal `n`) que leva a linha de cota até o ponto `hit`. A distância sai em mm
/// inteiros e a direção cola no eixo do modelo.
pub fn offset_from_hit(a0: P, a1: P, hit: P, n: P, shift: bool) -> Option<P> {
    let u = unit(sub(a1, a0))?;
    let nn = unit(n)?;
    let w = sub(hit, a0);
    let mut perp = sub(w, mul(u, dot(w, u)));
    perp = sub(perp, mul(nn, dot(perp, nn)));
    let d = len(perp);
    if d < 1e-9 {
        return Some([0.; 3]);
    }
    let dir = snap_axis(mul(perp, 1. / d), shift);
    Some(mul(dir, snap_mm(d, shift)))
}

/// Extensão das linhas de apoio além da linha de cota: distância do ponto `hit` ao longo da
/// direção do afastamento `off`, a partir da linha de cota. Nunca negativa.
pub fn ext_from_hit(a0: P, off: P, hit: P, shift: bool) -> f64 {
    let Some(e) = unit(off) else { return 0. };
    let q0 = add(a0, off);
    snap_mm(dot(sub(hit, q0), e), shift).max(0.)
}

/// Posição do texto ao longo da linha de cota, a partir do meio (mm, com sinal).
pub fn text_along_from_hit(a0: P, a1: P, off: P, hit: P, shift: bool) -> f64 {
    let Some(u) = unit(sub(a1, a0)) else { return 0. };
    let mid = add(mul(add(a0, a1), 0.5), off);
    snap_mm(dot(sub(hit, mid), u), shift)
}

/// Vértice da chamada: o ponto `p` visto de `prev`, no plano de normal `n`, com o comprimento em
/// mm inteiros e o ângulo em múltiplos de 15° contados a partir de X do modelo projetado no plano.
pub fn snap_polar(prev: P, p: P, n: P, shift: bool) -> P {
    if shift {
        return p;
    }
    let Some(nn) = unit(n) else { return p };
    let v = sub(p, prev);
    let v = sub(v, mul(nn, dot(v, nn)));
    let l = len(v);
    if l < 1e-9 {
        return p;
    }
    // referencial do plano: x = X do modelo projetado (ou Y se X é a normal)
    let proj = |e: P| unit(sub(e, mul(nn, dot(e, nn))));
    let Some(x) = proj([1., 0., 0.]).or_else(|| proj([0., 1., 0.])) else { return p };
    let y = cross(nn, x);
    let ang = dot(v, y).atan2(dot(v, x)).to_degrees();
    let ang = (ang / STEP_DEG).round() * STEP_DEG;
    let l = snap_mm(l, false).max(STEP_MM);
    let (s, c) = ang.to_radians().sin_cos();
    add(prev, add(mul(x, c * l), mul(y, s * l)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perto(a: P, b: P) -> bool {
        len(sub(a, b)) < 1e-9
    }

    #[test]
    fn distancia_encaixa_no_milimetro_e_shift_libera() {
        assert_eq!(snap_mm(12.4, false), 12.);
        assert_eq!(snap_mm(12.6, false), 13.);
        assert_eq!(snap_mm(12.4, true), 12.4);
    }

    #[test]
    fn direcao_cola_no_eixo_so_ate_sete_graus_e_meio() {
        let d = snap_axis([0.05, 0., 1.], false);
        assert!(perto(d, [0., 0., 1.]));
        let obliqua = snap_axis([0.3, 0., 1.], false);
        assert!(obliqua[0] > 0.2 && obliqua[0] < 0.4);
        // com Shift só normaliza
        let livre = snap_axis([0.05, 0., 1.], true);
        assert!(livre[0] > 0.04);
    }

    #[test]
    fn afastamento_da_linha_de_cota_vira_milimetros_inteiros_no_eixo() {
        // aresta de 0 a 40 em X; cursor a 12,4 mm em Z e 0,3 mm fora do eixo
        let off = offset_from_hit([0., 0., 0.], [40., 0., 0.], [17., 0.3, 12.4], [0., 1., 0.], false).unwrap();
        assert!(perto(off, [0., 0., 12.]));
        // lado oposto: sinal negativo
        let off = offset_from_hit([0., 0., 0.], [40., 0., 0.], [17., 0., -7.2], [0., 1., 0.], false).unwrap();
        assert!(perto(off, [0., 0., -7.]));
        // Shift deixa a distância solta
        let off = offset_from_hit([0., 0., 0.], [40., 0., 0.], [17., 0., 12.4], [0., 1., 0.], true).unwrap();
        assert!((off[2] - 12.4).abs() < 1e-9);
        // âncoras coincidentes: sem direção
        assert!(offset_from_hit([1.; 3], [1.; 3], [0.; 3], [0., 1., 0.], false).is_none());
    }

    #[test]
    fn extensao_das_linhas_de_apoio_mede_alem_da_linha_de_cota_e_nao_fica_negativa() {
        let e = ext_from_hit([0.; 3], [0., 0., 12.], [0., 0., 15.4], false);
        assert_eq!(e, 3.);
        assert_eq!(ext_from_hit([0.; 3], [0., 0., 12.], [0., 0., 5.], false), 0.);
        assert_eq!(ext_from_hit([0.; 3], [0.; 3], [0., 0., 5.], false), 0.);
    }

    #[test]
    fn texto_desliza_ao_longo_da_linha_a_partir_do_meio() {
        // meio em x=20; cursor em x=26,6 -> 7 mm (arredondado)
        let t = text_along_from_hit([0.; 3], [40., 0., 0.], [0., 0., 12.], [26.6, 0., 12.], false);
        assert_eq!(t, 7.);
        let t = text_along_from_hit([0.; 3], [40., 0., 0.], [0., 0., 12.], [13., 0., 12.], false);
        assert_eq!(t, -7.);
    }

    #[test]
    fn vertice_da_chamada_respeita_15_graus_e_1_mm() {
        let n = [0., 1., 0.];
        // 10,4 mm a ~13° do eixo X no plano XZ -> 10 mm a 15°
        let (c, s) = (13f64.to_radians().cos(), 13f64.to_radians().sin());
        let p = snap_polar([0.; 3], [10.4 * c, 0., 10.4 * s], n, false);
        let esperado = [10. * 15f64.to_radians().cos(), 0., 10. * 15f64.to_radians().sin()];
        // o sentido do ângulo depende da orientação do plano: aceita o espelho
        let espelho = [esperado[0], 0., -esperado[2]];
        assert!(perto(p, esperado) || perto(p, espelho), "{p:?}");
        assert!((len(p) - 10.).abs() < 1e-9);
        // Shift: ponto intacto
        assert_eq!(snap_polar([0.; 3], [3.3, 0., 1.1], n, true), [3.3, 0., 1.1]);
        // o ângulo sai múltiplo de 15° (no referencial do plano)
        let q = snap_polar([1., 0., 1.], [8.2, 0., 9.9], n, false);
        let v = sub(q, [1., 0., 1.]);
        let ang = v[2].atan2(v[0]).to_degrees().abs();
        assert!(((ang / 15.).round() * 15. - ang).abs() < 1e-6, "{ang}");
    }
}
