// SPDX-License-Identifier: LGPL-2.1-or-later
//! Filete de raiz (trocoide) do dente externo gerado por cremalheira-ferramenta (B52).
//!
//! **Como a peça é cortada.** Uma cremalheira de referência com ponta arredondada (raio `ρ`, 0,38 m
//! no perfil de referência da ISO 53 A) rola sem escorregar sobre o círculo primitivo. O flanco reto
//! da ferramenta gera a evolvente; o arco da ponta gera o filete de raiz. O centro do arco descreve
//! uma trocoide alongada em relação à engrenagem, e o filete é a envolvente dos círculos de raio `ρ`
//! centrados nela.
//!
//! **Contas (plano transversal).** Com a engrenagem girada de `φ`, a ferramenta anda `−r·φ` ao longo
//! da reta de rolamento `y = r`, e o ponto de contato `I = (0, r)` é o centro instantâneo do
//! movimento relativo. A normal da envolvente passa por `I`, então o ponto de contato do arco é
//! `Q = C + ρ (C − I) / |C − I|`, com `C = (x_c − r φ, y_c)`. Volta-se ao referencial da peça
//! girando de `−φ`. O filete vai de `φ₀ = x_c / r` (fundo, no círculo de raiz) até `φ_f`, em que
//! `C − I` fica paralelo à normal do flanco reto: ali o filete encontra a evolvente sem quina.
//!
//! **Rebaixo.** Se o ponto de transição da ferramenta (onde o arco encontra o flanco) passa do ponto
//! de tangência da linha de ação com o círculo base, a trocoide corta a evolvente por dentro. A
//! fronteira real é o filete até o cruzamento e a evolvente dali para cima. O mínimo de correção
//! que evita isso é [`correcao_minima`].

use std::f64::consts::PI;

/// Raio da ponta da ferramenta no perfil de referência (ISO 53 A), em módulos.
pub const RHO_REFERENCIA: f64 = 0.38;

/// Geometria da ponta da ferramenta no plano transversal.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ferramenta {
    /// raio da ponta (já reduzido se não couber)
    pub(crate) rho: f64,
    /// centro do arco da ponta direita, no referencial da ferramenta (x a partir do eixo do vão)
    pub(crate) xc: f64,
    pub(crate) yc: f64,
}

/// Ponta da ferramenta para o dente de raio primitivo `r`, base em `rf`, ângulo de pressão
/// transversal `alfa` (rad), espessura `sp` no primitivo e `z` dentes. `rho` é o raio pedido; se o
/// arco não cabe na ponta (ângulo de pressão alto), reduz até deixar 2 % da ponta reta.
/// Devolve `None` se nem um arco mínimo cabe (então o dente fica sem filete).
pub(crate) fn ferramenta(r: f64, rf: f64, alfa: f64, sp: f64, z: u32, rho: f64) -> Option<Ferramenta> {
    if rho <= 0. {
        return None;
    }
    let p = 2. * PI * r / z as f64;
    // meia espessura da ferramenta na reta de rolamento = meio vão da peça no primitivo
    let w = (p - sp) / 2.;
    let (t, c) = (alfa.tan(), alfa.cos());
    let xc_de = |rho: f64| w - (r - rf - rho) * t - rho / c;
    let mut rho = rho;
    if xc_de(rho) <= 0. {
        let rho_max = (w - (r - rf) * t) / (1. / c - t);
        if rho_max <= 1e-6 * r {
            return None;
        }
        rho = 0.98 * rho_max;
    }
    Some(Ferramenta { rho, xc: xc_de(rho), yc: rf + rho })
}

/// Correção mínima de perfil (em módulos `m`) que evita o rebaixo: o ponto em que o arco da ponta
/// encontra o flanco reto não pode passar do ponto de tangência da linha de ação, que fica a
/// `r sen²α` abaixo da reta de rolamento. `ded` é o pé do dente para x = 0 (1,25 m mais a folga de
/// impressão). Com ρ = 0,38 m e α = 20° dá a fórmula clássica `1 − z sen²α / 2`.
pub fn correcao_minima(m: f64, r: f64, alfa: f64, ded: f64, rho: f64) -> f64 {
    (ded - rho * (1. - alfa.sin()) - r * alfa.sin().powi(2)) / m
}

/// Pontos do filete do flanco esquerdo do dente (o que olha para o vão em +ângulo), do fundo para
/// cima, como (raio, meio ângulo a partir do eixo do dente). `n` trechos.
pub(crate) fn filete(r: f64, alfa: f64, z: u32, f: &Ferramenta, n: usize) -> Vec<[f64; 2]> {
    let phi0 = f.xc / r;
    let phi_f = (f.xc - (r - f.yc) / alfa.tan()) / r;
    let eixo_dente = PI / 2. - PI / z as f64;
    (0..=n)
        .map(|i| {
            let phi = phi0 + (phi_f - phi0) * i as f64 / n as f64;
            let (cx, cy) = (f.xc - r * phi, f.yc);
            let (dx, dy) = (cx, cy - r);
            let d = dx.hypot(dy);
            let (qx, qy) = (cx + f.rho * dx / d, cy + f.rho * dy / d);
            // de volta ao referencial da peça: gira de −φ
            let (s, c) = (-phi).sin_cos();
            let (gx, gy) = (qx * c - qy * s, qx * s + qy * c);
            [gx.hypot(gy), gy.atan2(gx) - eixo_dente]
        })
        .collect()
}

/// Junta o filete com a evolvente do dente `g`. Devolve os pontos do filete até a junção, com o
/// último exatamente sobre a evolvente, e se houve rebaixo.
///
/// - **Sem rebaixo**, o fim do filete é o ponto em que o flanco reto da ferramenta começa a gerar a
///   evolvente; abaixo dele a evolvente não é cortada (o filete deixa mais material), então a junção
///   é o fim do filete.
/// - **Com rebaixo**, esse ponto passou da tangência da linha de ação com o círculo base: o fim do
///   filete cai no outro ramo da evolvente (o que nasce na cúspide do círculo base para o lado do
///   vão), e o flanco reto gera a evolvente inteira desde o círculo base. A fronteira é o menor dos
///   dois: o filete, que entra no dente logo acima do círculo base, até o ponto em que volta a
///   encontrar a evolvente.
pub(crate) fn juntar(mut pts: Vec<[f64; 2]>, g: &crate::engrenagens::Dente) -> (Vec<[f64; 2]>, bool) {
    let rb = g.rb;
    let ev = |r: f64| g.meio_angulo(g.theta(r));
    // ramo do outro lado da cúspide: o ângulo polar da evolvente troca de sinal
    let ev_outro = |r: f64| {
        let th = g.theta(r);
        g.meio_angulo(th) + 2. * (th - th.atan())
    };
    let Some(&fim) = pts.last() else { return (pts, false) };
    let rebaixo = fim[0] > rb && (fim[1] - ev_outro(fim[0])).abs() < (fim[1] - ev(fim[0])).abs();
    if !rebaixo {
        if let Some(u) = pts.last_mut() {
            if u[0] > rb {
                u[1] = ev(u[0]);
            }
        }
        return (pts, false);
    }
    // primeiro ponto acima do círculo base em que o filete já não corta mais que a evolvente
    let dif = |q: [f64; 2]| q[1] - ev(q[0]);
    let Some(i) = (1..pts.len()).find(|&i| pts[i][0] > rb && dif(pts[i]) >= 0.) else { return (pts, true) };
    let (a, b) = (pts[i - 1], pts[i]);
    let no_trecho = |r: f64| [r, a[1] + (b[1] - a[1]) * (r - a[0]) / (b[0] - a[0])];
    let (mut lo, mut hi) = (a[0].max(rb), b[0]);
    if dif(no_trecho(lo)) >= 0. {
        // o filete chega ao círculo base deixando mais material que a evolvente: junta ali
        hi = lo;
    }
    for _ in 0..60 {
        let meio = 0.5 * (lo + hi);
        if dif(no_trecho(meio)) < 0. {
            lo = meio;
        } else {
            hi = meio;
        }
    }
    let r = 0.5 * (lo + hi);
    pts.truncate(i);
    if (pts[i - 1][0] - r).abs() < 1e-9 * r {
        pts.pop();
    }
    pts.push([r, ev(r)]);
    (pts, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engrenagens::{calculo, contorno_pontos, dente_externo, engrenagem_documento, Dente, ParamEngrenagem, TipoEngrenagem};
    use crate::{recompute, Status};

    fn reta(z: u32, x: f64) -> ParamEngrenagem {
        ParamEngrenagem { dentes: z, coef_correcao: x, ..Default::default() }
    }

    #[test]
    fn o_filete_sai_do_circulo_de_raiz_e_encontra_a_evolvente_sem_degrau() {
        let g = dente_externo(&reta(20, 0.));
        let l = g.limites().unwrap();
        assert!(!l.filete.is_empty() && !l.rebaixado);
        assert!((l.rho - 0.76).abs() < 1e-12, "ρ = 0,38 m inteiro cabe em 20°");
        let (fundo, juncao) = (l.filete[0], *l.filete.last().unwrap());
        assert!((fundo[0] - g.rf).abs() < 1e-9, "fundo no círculo de raiz: {fundo:?}");
        let f = ferramenta(g.r, g.rf, g.alfa, g.sp, g.z, g.rho).unwrap();
        assert!((fundo[1] - (PI / g.z as f64 - f.xc / g.r)).abs() < 1e-12, "meio vão do fundo = x_c / r");
        // o ponto final cai na evolvente sem ser forçado: a amostra crua já está nela
        let crus = filete(g.r, g.alfa, g.z, &f, 6);
        let u = crus.last().unwrap();
        assert!(u[0] > g.rb && (u[1] - g.meio_angulo(g.theta(u[0]))).abs() < 1e-9, "{u:?}");
        assert!((juncao[0] - u[0]).abs() < 1e-12);
        // o raio cresce do fundo para cima e o filete deixa mais material que a quina viva antiga
        // (flanco radial até o círculo base e a evolvente dali)
        for w in l.filete.windows(2) {
            assert!(w[1][0] > w[0][0]);
        }
        let sem = Dente { rho: 0., ..dente_externo(&reta(20, 0.)) };
        let ls = sem.limites().unwrap();
        assert!(ls.filete.is_empty());
        for q in &l.filete[1..l.filete.len() - 1] {
            let quina = if q[0] <= g.rb { ls.psi0 } else { g.meio_angulo(g.theta(q[0])) };
            assert!(q[1] > quina, "r {} ψ {} contra {}", q[0], q[1], quina);
        }
    }

    #[test]
    fn correcao_minima_bate_com_a_formula_classica_e_com_a_geometria() {
        // 20°, ρ = 0,38 m: x_min = 1 − z sen²α / 2
        for z in [8, 10, 14, 17, 20, 30] {
            let c = calculo(&reta(z, 0.5)).unwrap();
            let classica = 1. - z as f64 * 20_f64.to_radians().sin().powi(2) / 2.;
            assert!((c.correcao_minima - classica).abs() < 1e-3, "z {z}: {} contra {classica}", c.correcao_minima);
        }
        // a geometria gerada concorda: acima do mínimo a trocoide não corta, abaixo corta
        for z in [10, 12, 14, 16] {
            let xm = calculo(&reta(z, 0.5)).unwrap().correcao_minima;
            let acima = calculo(&reta(z, xm + 0.02)).unwrap();
            assert!(!acima.rebaixado && acima.avisos.iter().all(|a| !a.contains("rebaixado")), "z {z} x {}", xm + 0.02);
            let abaixo = calculo(&reta(z, xm - 0.08)).unwrap();
            assert!(abaixo.rebaixado, "z {z} x {}", xm - 0.08);
            assert!(abaixo.avisos.iter().any(|a| a.contains("rebaixado") && a.contains(&format!("{xm:.3}"))), "{:?}", abaixo.avisos);
        }
        // a helicoidal usa o plano transversal: pede menos correção que a reta de mesmo z
        let h = calculo(&ParamEngrenagem { tipo: TipoEngrenagem::Helicoidal, angulo_helice: 25., ..reta(12, 0.5) }).unwrap();
        assert!(h.correcao_minima < calculo(&reta(12, 0.5)).unwrap().correcao_minima);
    }

    #[test]
    fn angulo_de_pressao_alto_reduz_o_raio_da_ponta() {
        let p = ParamEngrenagem { angulo_pressao: 30., ..reta(20, 0.) };
        let c = calculo(&p).unwrap();
        assert!(c.raio_filete > 0. && c.raio_filete < 0.38 * 2., "{}", c.raio_filete);
        assert!(contorno_pontos(&p, 8, 3).is_ok());
    }

    #[test]
    fn contorno_com_rebaixo_fica_entre_raiz_e_topo() {
        let p = reta(9, 0.);
        let c = calculo(&p).unwrap();
        assert!(c.rebaixado);
        let pts = contorno_pontos(&p, 8, 3).unwrap();
        for q in &pts {
            let r = q[0].hypot(q[1]);
            assert!(r > c.diametro_raiz / 2. - 1e-9 && r < c.diametro_externo / 2. + 1e-9);
        }
        // a junção fica acima do círculo base, sobre a evolvente
        let g = dente_externo(&p);
        let l = g.limites().unwrap();
        let u = l.filete.last().unwrap();
        assert!(u[0] > g.rb && (u[1] - g.meio_angulo(g.theta(u[0]))).abs() < 1e-12);
    }

    fn medir(p: &ParamEngrenagem) -> (f64, [f64; 3], [f64; 3]) {
        let doc = engrenagem_documento(p).unwrap();
        let r = recompute(&doc);
        for f in &doc.features {
            assert!(matches!(r.status[&f.id], Status::Ok | Status::Warning(_)), "{}: {:?}", f.name, r.status[&f.id]);
        }
        assert_eq!(r.bodies.len(), 1);
        let b = &r.bodies[0];
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for q in b.mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(q[i] as f64);
                hi[i] = hi[i].max(q[i] as f64);
            }
        }
        (b.mass.volume, lo, hi)
    }

    #[test]
    fn engrenagem_com_filete_recalcula_e_o_volume_confere_com_o_contorno() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let p = reta(20, 0.);
        let (v, lo, hi) = medir(&p);
        let (v_raiz, v_topo) = (PI * 17.5 * 17.5 * 10., PI * 22. * 22. * 10.);
        assert!(v > v_raiz && v < v_topo, "{v}");
        assert!(hi[0] > 21.9 && hi[0] < 22.01 && lo[2].abs() < 1e-3 && (hi[2] - 10.).abs() < 1e-3, "{lo:?} {hi:?}");
        // área do polígono do contorno vezes a largura confere com o volume do sólido (1 %)
        let pts = contorno_pontos(&p, 16, 8).unwrap();
        let area: f64 = pts.iter().zip(pts.iter().cycle().skip(1)).map(|(a, b)| a[0] * b[1] - b[0] * a[1]).sum::<f64>() / 2.;
        assert!((area * 10. - v).abs() / v < 0.01, "área {area} · 10 contra {v}");
        // com rebaixo (z 9, x 0) e helicoidal também fecham sólido
        medir(&reta(9, 0.));
        medir(&ParamEngrenagem { tipo: TipoEngrenagem::Helicoidal, angulo_helice: 20., ..reta(14, 0.) });
    }
}
