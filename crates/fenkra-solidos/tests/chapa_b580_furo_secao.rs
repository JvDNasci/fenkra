// SPDX-License-Identifier: LGPL-2.1-or-later
//! B580: nervura de asa e caverna de fuselagem. Parede base com furos (anel oval, furos de alívio),
//! corpo que fica com o último estado bom quando uma feature entra em erro e seção da estação
//! (plano x asa de loft) como perfil associativo da parede base.

mod common;

use common::*;
use fenkra_core::sketch::{Curve, LinkSrc};
use fenkra_core::*;
use fenkra_solidos::{recompute, recompute_cached, Cache, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn chapa(d: &mut Document, plana: bool) {
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: plana, ..Default::default() });
}

fn parede(d: &mut Document, sk: Id) -> Id {
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }))
}

fn area_elipse(a: f64, b: f64) -> f64 {
    std::f64::consts::PI * a * b
}

fn ids_de(n0: u64) -> impl FnMut() -> Id {
    let mut n = n0;
    move || {
        n += 1;
        Id(n)
    }
}

// ------------------------------------------------------------------ furos na parede base

fn elipse(data: &mut SketchData, al: &mut impl FnMut() -> Id, c: [f64; 2], semi_x: f64, semi_y: f64) {
    let ctr = data.add_point(al, c[0], c[1]);
    data.add_curve(al, Curve::Ellipse { c: ctr, s: ctr, e: ctr, major: [semi_x, 0.], minor: semi_y, t0: 0., t1: std::f64::consts::TAU }, false);
}

fn spline_fechada(data: &mut SketchData, al: &mut impl FnMut() -> Id, c: [f64; 2], a: f64, b: f64, n: usize) {
    let mut ids: Vec<Id> = (0..n)
        .map(|i| {
            let t = i as f64 / n as f64 * std::f64::consts::TAU;
            data.add_point(al, c[0] + a * t.cos(), c[1] + b * t.sin())
        })
        .collect();
    ids.push(ids[0]);
    data.add_curve(al, Curve::Spline { pts: ids }, false);
}

/// Caverna oval: externo 1200 x 900, interno 1100 x 800, espessura 2; os dois contornos como
/// elipse exata ou spline fechada.
fn caverna(plana: bool, spline: bool) -> (Document, Id) {
    let mut d = Document::new("caverna");
    chapa(&mut d, plana);
    let mut data = SketchData::default();
    let mut al = ids_de(9_000_000);
    if spline {
        spline_fechada(&mut data, &mut al, [0., 0.], 600., 450., 32);
        spline_fechada(&mut data, &mut al, [0., 0.], 550., 400., 32);
    } else {
        elipse(&mut data, &mut al, [0., 0.], 600., 450.);
        elipse(&mut data, &mut al, [0., 0.], 550., 400.);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    let w = parede(&mut d, sk);
    (d, w)
}

#[test]
fn anel_oval_fura_a_chapa_dobrado_e_planificado() {
    if !kernel_disponivel() {
        return;
    }
    let cheia = area_elipse(600., 450.) * 2.;
    let esperado = (area_elipse(600., 450.) - area_elipse(550., 400.)) * 2.;
    for spline in [false, true] {
        for plana in [false, true] {
            let (d, w) = caverna(plana, spline);
            let r = recompute(&d);
            assert_eq!(r.status[&w], Status::Ok, "spline={spline} plana={plana}: {:?}", r.status[&w]);
            assert_eq!(r.bodies.len(), 1);
            let v = r.bodies[0].mass.volume;
            let tol = if spline { 0.02 } else { 0.005 };
            assert!((v - esperado).abs() < tol * esperado, "spline={spline} plana={plana}: volume {v} contra {esperado} (cheia seria {cheia})");
            let (lo, hi) = caixa_da_malha(&r);
            assert!((hi[0] - lo[0] - 1200.).abs() < 2. && (hi[1] - lo[1] - 900.).abs() < 2. && (hi[2] - lo[2] - 2.).abs() < 0.1, "caixa {lo:?} {hi:?}");
        }
    }
}

#[test]
fn nervura_com_furos_de_alivio() {
    if !kernel_disponivel() {
        return;
    }
    for plana in [false, true] {
        let mut d = Document::new("nervura");
        chapa(&mut d, plana);
        let mut data = SketchData::default();
        let mut al = ids_de(9_200_000);
        data.add_rect(&mut al, [0., 0.], [400., 200.], false);
        for c in [[100., 100.], [200., 100.], [300., 100.]] {
            let p = data.add_point(&mut al, c[0], c[1]);
            data.add_curve(&mut al, Curve::Circle { c: p, r: 15. }, false);
        }
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        let w = parede(&mut d, sk);
        let r = recompute(&d);
        assert_eq!(r.status[&w], Status::Ok);
        let esperado = (400. * 200. - 3. * std::f64::consts::PI * 225.) * 2.;
        let v = r.bodies[0].mass.volume;
        assert!((v - esperado).abs() < 0.002 * esperado, "plana={plana}: volume {v} contra {esperado}");
    }
}

// ------------------------------------------------------------------ erro numa feature de chapa

/// Parede 100 x 60 x 2 e uma aba por superfície com o suporte longe da peça: a aba entra em erro e o
/// corpo continua o da parede base, com o volume dela.
#[test]
fn erro_na_aba_deixa_o_corpo_do_estado_anterior() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("erro");
    chapa(&mut d, false);
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 60.]);
    let w = parede(&mut d, sk);
    let longe = d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::XZ), offset: s("-900") });
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange {
        support: SurfRef::Plane(PlaneRef::Datum(longe)),
        face: [50., 30., 2.],
        length: s("10"),
        radius: None,
        flip: false,
        keep_base: false,
        mode: 0,
        angle: s("90"),
        limites: Default::default(),
        side: 0,
        inv_lado: false,
    }));
    let r = recompute(&d);
    assert_eq!(r.status[&w], Status::Ok);
    assert!(matches!(r.status[&aba], Status::Error(_)), "a aba devia falhar: {:?}", r.status[&aba]);
    assert_eq!(r.bodies.len(), 1, "o corpo some quando a aba falha");
    let v = r.bodies[0].mass.volume;
    assert!((v - 100. * 60. * 2.).abs() < 1., "volume {v}");
    let (lo, hi) = caixa_da_malha(&r);
    assert!((hi[0] - lo[0] - 100.).abs() < 0.1 && (hi[1] - lo[1] - 60.).abs() < 0.1 && (hi[2] - lo[2] - 2.).abs() < 0.1, "caixa {lo:?} {hi:?}");
}

/// Uma segunda parede que não encosta na primeira entra em erro explicado; a primeira fica.
#[test]
fn segunda_parede_solta_da_erro_e_o_corpo_fica() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("erro2");
    chapa(&mut d, false);
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 60.]);
    parede(&mut d, sk);
    let sk2 = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [500., 500.], [520., 520.]);
    let w2 = parede(&mut d, sk2);
    let r = recompute(&d);
    assert!(matches!(r.status[&w2], Status::Error(_)), "{:?}", r.status[&w2]);
    assert_eq!(r.bodies.len(), 1);
    assert!((r.bodies[0].mass.volume - 12000.).abs() < 1., "volume {}", r.bodies[0].mass.volume);
}

// ------------------------------------------------------------------ asa enflechada

/// NACA 2412 de corda `c`, do bordo de fuga pelo extradorso até o bordo de ataque e de volta pelo
/// intradorso (bordo de fuga aberto: espessura de 0,126 % da corda), com o recuo `x0` em x.
fn naca2412(c: f64, x0: f64, n: usize) -> Vec<[f64; 2]> {
    let (m, p, t) = (0.02, 0.4, 0.12);
    let yt = |x: f64| 5. * t * (0.2969 * x.sqrt() - 0.1260 * x - 0.3516 * x * x + 0.2843 * x.powi(3) - 0.1015 * x.powi(4));
    let ponto = |x: f64, cima: bool| {
        let (k, p1) = if x < p { (m / (p * p), 2. * p * x - x * x) } else { (m / ((1. - p) * (1. - p)), (1. - 2. * p) + 2. * p * x - x * x) };
        let (yc, dyc) = (k * p1, k * (2. * p - 2. * x));
        let th = dyc.atan();
        let sinal = if cima { 1. } else { -1. };
        [x - sinal * yt(x) * th.sin(), yc + sinal * yt(x) * th.cos()]
    };
    // do bordo de fuga (x = 1) ao de ataque (x = 0), espaçamento em cosseno
    let xs: Vec<f64> = (0..=n).map(|i| 0.5 * (1. + (std::f64::consts::PI * i as f64 / n as f64).cos())).collect();
    let mut v: Vec<[f64; 2]> = xs.iter().map(|x| ponto(*x, true)).collect();
    v.extend(xs.iter().rev().skip(1).map(|x| ponto(*x, false)));
    v.into_iter().map(|q| [x0 + q[0] * c, q[1] * c]).collect()
}

/// Sketch do aerofólio (spline do contorno mais a linha que fecha o bordo de fuga) num plano.
fn sketch_perfil(d: &mut Document, plane: PlaneRef, pts: &[[f64; 2]]) -> Id {
    let mut data = SketchData::default();
    let mut al = ids_de(9_500_000 + d.features.len() as u64 * 10_000);
    let ids: Vec<Id> = pts.iter().map(|q| data.add_point(&mut al, q[0], q[1])).collect();
    let (a, b) = (ids[0], *ids.last().unwrap());
    data.add_curve(&mut al, Curve::Spline { pts: ids }, false);
    data.add_curve(&mut al, Curve::Line { a: b, b: a }, false);
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

const ENVERGADURA: f64 = 4000.;
const RECUO: f64 = 600.;

fn corda_em(y: f64) -> f64 {
    2000. - 500. * y / ENVERGADURA
}
fn recuo_em(y: f64) -> f64 {
    RECUO * y / ENVERGADURA
}

/// A asa por loft (raiz no plano XZ, ponta a 4000 mm) e a nervura de parede base na estação `y`, com a
/// seção do corpo como perfil. Devolve o documento, o plano da estação e a parede.
fn asa_com_nervura(y: f64) -> (Document, Id, Id) {
    let mut d = Document::new("asa");
    chapa(&mut d, false);
    let raiz = sketch_perfil(&mut d, PlaneRef::Origin(OriginPlane::XZ), &naca2412(2000., 0., 20));
    let plano_ponta = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XZ), &format!("{ENVERGADURA}"));
    let ponta = sketch_perfil(&mut d, PlaneRef::Datum(plano_ponta), &naca2412(1500., RECUO, 20));
    d.add_feature(FeatureKind::Loft(Loft { sections: vec![raiz, ponta], ruled: false, closed: false, op: BoolOp::Add, torcao: vec![], inicio: Default::default(), fim: Default::default() }));
    let estacao = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XZ), &format!("{y}"));
    // a nervura é outro corpo (chapa), na mesma peça
    let nervura = d.add_collector("Nervura");
    d.active_collector = nervura;
    let mut data = SketchData::default();
    let mut al = ids_de(9_700_000);
    // a seção da asa pela estação: só a forma inicial, o cálculo a refaz a cada recálculo; o ponto
    // é um lugar da asa (a lateral, perto do meio da corda)
    let aprox = naca2412(corda_em(y), recuo_em(y), 20);
    let frame_estacao = [recuo_em(y) + 0.3 * corda_em(y), -y, 0.06 * corda_em(y)];
    data.add_link(&mut al, LinkSrc::Body { point: frame_estacao }, true, &aprox, false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(estacao), data }));
    let w = parede(&mut d, sk);
    (d, estacao, w)
}

fn estacao_para(d: &mut Document, plano: Id, y: f64) {
    let r = d.references.iter_mut().find(|r| r.id == plano).unwrap();
    r.kind = RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::XZ), offset: format!("{y}") };
}

/// Área de um polígono (fórmula do laço) em mm².
fn area_laco(p: &[[f64; 2]]) -> f64 {
    (0..p.len()).map(|i| p[i][0] * p[(i + 1) % p.len()][1] - p[(i + 1) % p.len()][0] * p[i][1]).sum::<f64>().abs() / 2.
}

fn caixa_do_perfil(p: &[[f64; 2]]) -> ([f64; 2], [f64; 2]) {
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for q in p {
        for i in 0..2 {
            lo[i] = lo[i].min(q[i]);
            hi[i] = hi[i].max(q[i]);
        }
    }
    (lo, hi)
}

/// A nervura de parede base na estação `y`: status Ok, volume = área da seção x espessura e caixa
/// com a corda local (x), a espessura relativa do perfil (z) e a chapa de 2 mm (y).
fn confere_nervura(r: &RecomputeResult, w: Id, y: f64) {
    assert_eq!(r.status[&w], Status::Ok, "estação {y}: {:?}", r.status[&w]);
    assert_eq!(r.bodies.len(), 2, "estação {y}: asa e nervura");
    let nervura = r.bodies.iter().min_by(|a, b| a.mass.volume.total_cmp(&b.mass.volume)).unwrap();
    let perfil = naca2412(corda_em(y), recuo_em(y), 60);
    let esperado = area_laco(&perfil) * 2.;
    let v = nervura.mass.volume;
    eprintln!("estação {y}: volume {v:.0} esperado {esperado:.0}");
    assert!((v - esperado).abs() < 0.02 * esperado, "estação {y}: volume {v} contra {esperado}");
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for q in nervura.mesh.positions.chunks_exact(3) {
        for i in 0..3 {
            lo[i] = lo[i].min(q[i] as f64);
            hi[i] = hi[i].max(q[i] as f64);
        }
    }
    let (plo, phi) = caixa_do_perfil(&perfil);
    assert!(((hi[0] - lo[0]) - (phi[0] - plo[0])).abs() < 0.015 * corda_em(y), "estação {y}: corda {} contra {}", hi[0] - lo[0], phi[0] - plo[0]);
    assert!((lo[0] - plo[0]).abs() < 0.01 * corda_em(y), "estação {y}: recuo {} contra {}", lo[0], plo[0]);
    assert!(((hi[2] - lo[2]) - (phi[1] - plo[1])).abs() < 0.02 * (phi[1] - plo[1]), "estação {y}: altura {} contra {}", hi[2] - lo[2], phi[1] - plo[1]);
    assert!((hi[1] - lo[1] - 2.).abs() < 0.2, "estação {y}: espessura {}", hi[1] - lo[1]);
    // a chapa fica na estação (plano y = -estação)
    assert!(((hi[1] + lo[1]) / 2. + y).abs() < 2.5, "estação {y}: y da chapa {lo:?} {hi:?}");
}

#[test]
fn nervura_pela_secao_da_estacao_acompanha_a_estacao() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, estacao, w) = asa_com_nervura(500.);
    let mut cache = Cache::default();
    for y in [500., 1500., 2800., 1500., 500.] {
        estacao_para(&mut d, estacao, y);
        let r = recompute_cached(&d, &mut cache);
        confere_nervura(&r, w, y);
    }
}

#[test]
fn nervura_em_cada_estacao_montada_do_zero() {
    if !kernel_disponivel() {
        return;
    }
    for y in [500., 1500., 2800.] {
        let (d, _, w) = asa_com_nervura(y);
        let r = recompute(&d);
        confere_nervura(&r, w, y);
    }
}

#[test]
fn estacao_fora_da_asa_da_erro_claro_e_nunca_corpo_vazio_ok() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, estacao, w) = asa_com_nervura(1500.);
    let mut cache = Cache::default();
    assert_eq!(recompute_cached(&d, &mut cache).status[&w], Status::Ok);
    for y in [4600., -300.] {
        estacao_para(&mut d, estacao, y);
        let r = recompute_cached(&d, &mut cache);
        match &r.status[&w] {
            Status::Error(m) => {
                eprintln!("estação {y}: {m}");
                assert!(m.contains("seção") || m.contains("plano"), "estação {y}: mensagem pouco clara: {m}");
            }
            outro => panic!("estação {y} fora da asa: esperava erro explicado, veio {outro:?} com {} corpos", r.bodies.len()),
        }
        // o corpo da asa continua visível
        assert!(r.bodies.iter().any(|b| b.mass.volume > 1e6), "a asa sumiu");
    }
    // voltando para dentro da asa, tudo recalcula
    estacao_para(&mut d, estacao, 2000.);
    confere_nervura(&recompute_cached(&d, &mut cache), w, 2000.);
}

// ------------------------------------------------------------------ caverna de fuselagem

/// Semieixos da fuselagem oval por loft (600 x 800 na estação 0, 380 x 520 a 3000 mm).
fn semi_fuselagem(x: f64) -> (f64, f64) {
    (600. + (380. - 600.) * x / 3000., 800. + (520. - 800.) * x / 3000.)
}

/// A fuselagem e a caverna na estação `x`: o contorno externo é a seção do corpo; o interno, uma
/// spline desenhada no mesmo sketch (anel de 50 mm de largura).
fn caverna_em(x: f64) -> (Document, Id) {
    let mut d = Document::new("fuselagem");
    chapa(&mut d, false);
    let oval = |d: &mut Document, plane: PlaneRef, a: f64, b: f64| {
        let mut data = SketchData::default();
        let mut al = ids_de(9_800_000 + d.features.len() as u64 * 10_000);
        spline_fechada(&mut data, &mut al, [0., 0.], a, b, 24);
        d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
    };
    let s0 = oval(&mut d, PlaneRef::Origin(OriginPlane::YZ), 600., 800.);
    let p1 = d.add_offset_plane(PlaneRef::Origin(OriginPlane::YZ), "3000");
    let s1 = oval(&mut d, PlaneRef::Datum(p1), 380., 520.);
    d.add_feature(FeatureKind::Loft(Loft { sections: vec![s0, s1], ruled: false, closed: false, op: BoolOp::Add, torcao: vec![], inicio: Default::default(), fim: Default::default() }));
    let estacao = d.add_offset_plane(PlaneRef::Origin(OriginPlane::YZ), &format!("{x}"));
    let caverna = d.add_collector("Caverna");
    d.active_collector = caverna;
    let (a, b) = semi_fuselagem(x);
    let mut data = SketchData::default();
    let mut al = ids_de(9_900_000);
    let externo: Vec<[f64; 2]> = (0..=24).map(|i| [a * (i as f64 / 24. * std::f64::consts::TAU).cos(), b * (i as f64 / 24. * std::f64::consts::TAU).sin()]).collect();
    data.add_link(&mut al, LinkSrc::Body { point: [x, a, 0.] }, true, &externo, false);
    spline_fechada(&mut data, &mut al, [0., 0.], a - 50., b - 50., 32);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(estacao), data }));
    let w = parede(&mut d, sk);
    (d, w)
}

#[test]
fn caverna_pela_secao_da_fuselagem_fura_o_anel() {
    if !kernel_disponivel() {
        return;
    }
    for x in [800., 1500., 2200.] {
        let (d, w) = caverna_em(x);
        let r = recompute(&d);
        assert_eq!(r.status[&w], Status::Ok, "estação {x}: {:?}", r.status[&w]);
        assert_eq!(r.bodies.len(), 2, "estação {x}");
        let anel = r.bodies.iter().min_by(|p, q| p.mass.volume.total_cmp(&q.mass.volume)).unwrap();
        let (a, b) = semi_fuselagem(x);
        let esperado = (area_elipse(a, b) - area_elipse(a - 50., b - 50.)) * 2.;
        eprintln!("caverna {x}: volume {:.0} esperado {esperado:.0}", anel.mass.volume);
        assert!((anel.mass.volume - esperado).abs() < 0.03 * esperado, "estação {x}: volume {} contra {esperado} (o furo falta?)", anel.mass.volume);
    }
}

