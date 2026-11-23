// SPDX-License-Identifier: LGPL-2.1-or-later
//! B566 (pendência 1): a aba por superfície DOBRADA em asa grande.
//!
//! A web de `c` × 0,6 `c` é relimitada pelo dorso de um perfil de asa de quatro dígitos (extrudado na
//! envergadura, e também deslocado de 20 mm) e a aba de 20 mm sobe pela linha de dobra (OML). Antes, de
//! corda 400 para cima, a união da varredura da dobra com a web saía vazia ou só com uma das partes e o
//! comando dava erro; o planificado funcionava.
//!
//! Causa: a varredura seguia a polilinha das amostras da OML. As amostras vêm da seção do suporte pelo
//! plano da web, que o kernel entrega como polilinha, e caem nas cordas: até 0,08 mm fora da aresta
//! verdadeira da web na corda 400, e a flecha cresce com a peça. Além disso a polilinha era refeita em
//! retas e arcos aceitos com erro de até 0,2 % do raio (mais de 1 mm em raio grande), com quinas entre
//! eles: a ferramenta saía com 150 a 200 faces, transições arredondadas que se cruzam (o analisador da
//! booleana acusa autointerseção na corda 1000) e a face da dobra, tangente à face da web, nascendo
//! décimos de mm fora da borda dela. A booleana não classifica esse contato quase tangente e devolve
//! vazio. Agora a varredura segue a aresta exata da web (uma aresta, aparada nas amostras das pontas),
//! as amostras são levadas para cima dela e a faixa planificada tem as bordas em spline: a ferramenta
//! tem 10 faces e a fusão sai válida, sem tolerância difusa, em todas as escalas.
//!
//! Segunda parte: o mesmo caminho exato com limites nos lados 1 e 2 e com a posição do material por fora
//! ou por dentro (a linha de dobra recua: a curva exata é deslocada no plano pelo kernel), e a aba vai até
//! a borda da web (antes parava na última amostra, até 5 mm antes na corda 1000).

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};
use std::time::Instant;

fn s(x: &str) -> String {
    x.to_string()
}

fn naca(x: f64) -> f64 {
    5. * 0.12 * (0.2969 * x.sqrt() - 0.1260 * x - 0.3516 * x * x + 0.2843 * x.powi(3) - 0.1015 * x.powi(4))
}

/// Ponto do dorso da asa de corda `c` no parâmetro `x` (0 a 1).
fn dorso(c: f64, x: f64) -> [f64; 2] {
    [-0.05 * c + 1.1 * c * x, 0.28 * c + 1.4 * c * naca(x) * 0.5 + 0.06 * c * (1. - x)]
}

/// Opções do caso: suporte deslocado, posição do material e limites dos lados (planos x = constante).
#[derive(Clone, Copy, Debug, Default)]
struct Caso {
    off: Option<f64>,
    side: u8,
    lados: Option<(f64, f64)>,
}

/// Asa de corda `c` e envergadura `e` (mm).
fn asa(c: f64, e: f64, caso: Caso, flat: bool) -> (Document, Id) {
    let mut d = Document::new("asa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [c, c * 0.6]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let pts: Vec<PointRef> = (0..=60)
        .map(|i| {
            let u = i as f64 / 60.;
            let p = dorso(c, (1. - (std::f64::consts::PI * u).cos()) / 2.);
            PointRef::Fixed([p[0], p[1], 0.])
        })
        .collect();
    let curva = d.add_geometry(RefKind::Spline { points: pts, closed: false, tangents: vec![] });
    let mut sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(curva), dir: AxisRef::Origin(OriginAxis::Z), length: format!("{e}"), length2: s("0"), until: None, until2: None, mais: vec![] });
    if let Some(o) = caso.off {
        sup = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(sup), dist: format!("{o}"), both: false, join: OffsetJoin::Automatico });
    }
    let mut limites = LimitesAba::default();
    if let Some((x1, x2)) = caso.lados {
        for (i, x) in [x1, x2].into_iter().enumerate() {
            let p = d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::YZ), offset: format!("{x}") });
            let l = Some(LimiteAba::Superficie(SurfRef::Plane(PlaneRef::Datum(p))));
            if i == 0 {
                limites.lado1 = l;
            } else {
                limites.lado2 = l;
            }
        }
    }
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Ref(sup), face: [c / 2., c * 0.1, 2.], length: s("20"), radius: None, flip: false, keep_base: false, mode: 0, angle: s("90"), limites, side: caso.side, inv_lado: false }));
    (d, aba)
}

/// A linha de dobra teórica (o dorso deslocado de `off` no plano, para +y), amostrada fina: pontos (x, y).
fn oml(c: f64, off: f64) -> Vec<[f64; 2]> {
    let n = 20000;
    (0..=n)
        .map(|i| {
            let u = i as f64 / n as f64;
            let (p, q) = (dorso(c, u.max(1e-9)), dorso(c, (u + 1e-6).min(1.)));
            let (tx, ty) = (q[0] - p[0], q[1] - p[1]);
            let l = (tx * tx + ty * ty).sqrt().max(1e-300);
            [p[0] - ty / l * off, p[1] + tx / l * off]
        })
        .collect()
}

/// Comprimento da OML entre x = a e x = b, e a componente x da direção "para fora" (+y) nas duas pontas.
fn trecho(o: &[[f64; 2]], a: f64, b: f64) -> (f64, f64, f64) {
    let mut l = 0.;
    let (mut oa, mut ob) = (0., 0.);
    for w in o.windows(2) {
        let (x0, x1) = (w[0][0], w[1][0]);
        let (dx, dy) = (x1 - x0, w[1][1] - w[0][1]);
        let n = (dx * dx + dy * dy).sqrt();
        if dx <= 0. {
            continue;
        }
        if x0 <= a && x1 > a {
            oa = -dy / n;
        }
        if x0 <= b && x1 > b {
            ob = -dy / n;
        }
        let (lo, hi) = (x0.max(a), x1.min(b));
        if hi > lo {
            l += n * (hi - lo) / dx;
        }
    }
    (l, oa, ob)
}

/// Monta, recalcula dobrada e planificada e confere status, volume e caixa envolvente.
fn confere(c: f64, caso: Caso) {
    let e = 2. * c;
    let t = Instant::now();
    let (d, aba) = asa(c, e, caso, false);
    let dobrada = recompute(&d);
    let t_dobrada = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let (dp, abap) = asa(c, e, caso, true);
    let plana = recompute(&dp);
    let t_plana = t.elapsed().as_secs_f64();
    let (vd, vp) = (volume_total(&dobrada), volume_total(&plana));
    let ((lo, hi), (lop, hip)) = (caixa_da_malha(&dobrada), caixa_da_malha(&plana));
    println!("B566 corda {c} {caso:?}: dobrada {t_dobrada:.2} s vol {vd:.1} caixa {lo:?} {hi:?}; planificada {t_plana:.2} s vol {vp:.1} caixa {lop:?} {hip:?}");
    assert_eq!(dobrada.status[&aba], Status::Ok, "corda {c} {caso:?} dobrada");
    assert_eq!(plana.status[&abap], Status::Ok, "corda {c} {caso:?} planificada");

    // posição do material: a linha de dobra recua `rec` para dentro da web (r·tg(θ/2) por fora,
    // (r + t)·tg(θ/2) por dentro; θ = 90°)
    let rec = match caso.side {
        1 => 2.,
        2 => 4.,
        _ => 0.,
    };
    let off = caso.off.unwrap_or(0.);
    let (xa, xb) = caso.lados.unwrap_or((0., c));
    let recuada = oml(c, off - rec);
    let (l_aba, _, _) = trecho(&recuada, xa, xb);
    let (_, out_a, out_b) = trecho(&oml(c, off), xa, xb);

    // volume: a web relimitada (a mesma nos dois estados) mais a aba. A aba dobrada tem
    // θ·(r + t/2)·t + comprimento·t por mm de linha de dobra (49,4 mm²) e a faixa planificada usa
    // θ·(r + k·t) com k = 0,44: a diferença do fator K é θ·(0,5 − k)·t² ≈ 0,38 mm³ por mm de linha de
    // dobra; a faixa plana, desenvolvida para fora de uma OML curva, ganha ou perde um pouco pela
    // curvatura. A OML mede cerca de uma corda: a diferença fica abaixo de 0,6 mm³ por mm de corda.
    assert!((vd - vp).abs() < 0.6 * c, "corda {c} {caso:?}: dobrada {vd:.1}, planificada {vp:.1}");
    // a web relimitada fica com ~57 % (dorso) ou ~60 % (deslocado) do retângulo de c × 0,6 c × 2; com o
    // recuo ela perde a faixa de `rec` mm ao longo da linha de dobra (só entre os limites)
    let web = c * 0.6 * c * 2. * if caso.off.is_some() { 0.6013 } else { 0.5678 } - rec * 2. * l_aba;
    let por_mm = (vd - web) / l_aba;
    println!("  aba dobrada {:.0} mm³ em {l_aba:.1} mm de linha de dobra: {por_mm:.2} mm³/mm", vd - web);
    assert!(por_mm > 47.5 && por_mm < 51.5, "corda {c} {caso:?}: a aba dobrada tem {por_mm:.2} mm³ por mm de linha de dobra (esperado ~49,4)");

    // caixa: a aba sobe 2 + r + comprimento = 24 mm em z; a planificada fica plana
    assert!(lo[2] > -1e-3 && (hi[2] - 24.).abs() < 0.05, "corda {c} {caso:?}: caixa dobrada em z {lo:?} {hi:?}");
    assert!(lop[2] > -1e-3 && hip[2] < 2. + 1e-3, "corda {c} {caso:?}: a planificada fica plana {lop:?} {hip:?}");
    // em x: a web vai de 0 a c. A aba vai até a borda (ou até o limite) e termina normal à linha de dobra,
    // no plano que passa pelo canto da web: a dobra sai da web (r + t − recuo)·|componente x da direção
    // para fora| na dobrada e (ba + comprimento − recuo)·|…| na planificada. Com limites fica dentro.
    let fora = |w: f64| ((xa + w * out_a).min(0.), (xb + w * out_b).max(c));
    let ((ed0, ed1), (ep0, ep1)) = (fora(4. - rec), fora(24.52 - rec));
    assert!((lo[0] - ed0).abs() < 0.15 && (hi[0] - ed1).abs() < 0.15, "corda {c} {caso:?}: caixa dobrada em x {lo:?} {hi:?} (esperado {ed0:.2}..{ed1:.2})");
    assert!((lop[0] - ep0).abs() < 0.15 && (hip[0] - ep1).abs() < 0.15, "corda {c} {caso:?}: caixa planificada em x {lop:?} {hip:?} (esperado {ep0:.2}..{ep1:.2})");
    // em y: o topo da linha de dobra (recuada) mais r + t na dobrada; na planificada, a faixa
    // desenvolvida (ba + comprimento = 24,5 mm) para fora dele. Fora dos limites a web segue até a linha
    // de dobra sem recuo, e pode ser ela o topo.
    let topo_de = |o: &[[f64; 2]], a: f64, b: f64| o.iter().filter(|p| p[0] >= a && p[0] <= b).map(|p| p[1]).fold(f64::MIN, f64::max);
    let topo = topo_de(&recuada, xa, xb);
    let topo_web = topo_de(&oml(c, off), 0., c);
    let (td, tp) = ((topo + 4.).max(topo_web), (topo + 24.52).max(topo_web));
    assert!((hi[1] - td).abs() < 0.5, "corda {c} {caso:?}: topo da dobrada {} contra {td}", hi[1]);
    assert!((hip[1] - tp).abs() < 0.5, "corda {c} {caso:?}: topo da planificada {} contra {tp}", hip[1]);

    // a aba vai até a borda da web (ou até o limite): a parede (acima da web, z > 3) chega às pontas da
    // linha de dobra
    let xs: Vec<f64> = dobrada.bodies.iter().flat_map(|b| b.mesh.positions.chunks_exact(3)).filter(|p| p[2] > 3.).map(|p| p[0] as f64).collect();
    let (px0, px1) = (xs.iter().copied().fold(f64::MAX, f64::min), xs.iter().copied().fold(f64::MIN, f64::max));
    assert!(px0 < xa + 0.2 && px1 > xb - 0.2, "corda {c} {caso:?}: a parede vai de x = {px0:.2} a {px1:.2}, a linha de dobra de {xa} a {xb}");
    // e não passa do limite
    if caso.lados.is_some() {
        assert!(px0 > xa - 0.2 && px1 < xb + 0.2, "corda {c} {caso:?}: a parede passa dos limites: {px0:.2}..{px1:.2}");
    }
}

#[test]
fn aba_dobrada_em_asa_de_corda_200() {
    if !kernel_disponivel() {
        return;
    }
    confere(200., Caso::default());
}

#[test]
fn aba_dobrada_em_asa_de_corda_400() {
    if !kernel_disponivel() {
        return;
    }
    confere(400., Caso::default());
}

#[test]
fn aba_dobrada_em_asa_de_corda_1000() {
    if !kernel_disponivel() {
        return;
    }
    confere(1000., Caso::default());
}

/// O caso do relato: corda 1000, envergadura 2000 e o suporte deslocado de 20 mm.
#[test]
fn aba_dobrada_em_asa_de_corda_1000_suporte_deslocado() {
    if !kernel_disponivel() {
        return;
    }
    confere(1000., Caso { off: Some(20.), ..Default::default() });
}

/// Limites nos lados 1 e 2 (planos x = 250 e x = 750): a aba fica só entre eles, dobrada e planificada.
#[test]
fn aba_dobrada_em_asa_de_corda_1000_com_lados_limitados() {
    if !kernel_disponivel() {
        return;
    }
    confere(1000., Caso { lados: Some((250., 750.)), ..Default::default() });
}

/// Posição do material por fora: a linha de dobra recua r·tg(45°) = 2 mm para dentro da web.
#[test]
fn aba_dobrada_em_asa_de_corda_1000_material_por_fora() {
    if !kernel_disponivel() {
        return;
    }
    confere(1000., Caso { side: 1, ..Default::default() });
}

/// Posição do material por dentro: a linha de dobra recua (r + t)·tg(45°) = 4 mm.
#[test]
fn aba_dobrada_em_asa_de_corda_1000_material_por_dentro() {
    if !kernel_disponivel() {
        return;
    }
    confere(1000., Caso { side: 2, ..Default::default() });
}

/// Recuo, limites e suporte deslocado juntos: o recorte do recuo também para nos limites.
#[test]
fn aba_dobrada_em_asa_de_corda_1000_recuo_limites_e_suporte_deslocado() {
    if !kernel_disponivel() {
        return;
    }
    confere(1000., Caso { off: Some(20.), side: 2, lados: Some((250., 750.)) });
}
