// SPDX-License-Identifier: LGPL-2.1-or-later
//! B556: aba por superfície com limites nos lados 1 e 2, na borda final e cantos com filete ou
//! chanfro. Cada teste monta o documento em código, recalcula e confere status, volume e caixa.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.to_string()
}

/// Web de 100 × 50 e suporte curvo (spline abaulada em torno de x = 75, extrudada em Z).
fn nervura(limites: LimitesAba) -> (Document, Id) {
    let mut d = Document::new("nervura");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let pts: Vec<PointRef> = (0..=12)
        .map(|i| {
            let y = -10. + 70. * i as f64 / 12.;
            PointRef::Fixed([75. + 5. * (y / 60. * std::f64::consts::PI).sin(), y, 0.])
        })
        .collect();
    let curva = d.add_geometry(RefKind::Spline { points: pts, closed: false, tangents: vec![] });
    let sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(curva), dir: AxisRef::Origin(OriginAxis::Z), length: s("60"), length2: s("0"), until: None, until2: None, mais: vec![] });
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Ref(sup), face: [40., 25., 2.], length: s("15"), radius: None, flip: false, keep_base: false, mode: 0, angle: s("90"), limites, side: 0, inv_lado: false }));
    (d, aba)
}

fn plano_y(d: &mut Document, y: f64) -> LimiteAba {
    let p = d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::XZ), offset: format!("{}", -y) });
    LimiteAba::Superficie(SurfRef::Plane(PlaneRef::Datum(p)))
}

fn recalcula(d: &Document, flat: bool) -> RecomputeResult {
    let mut d = d.clone();
    d.sheet.as_mut().unwrap().flat = flat;
    recompute(&d)
}

/// Faixa de y dos vértices da aba (acima da web, do lado do suporte).
fn faixa_y(r: &RecomputeResult) -> (f64, f64) {
    let v: Vec<f64> = r.bodies.iter().flat_map(|b| b.mesh.positions.chunks_exact(3)).filter(|p| p[2] > 2.6 && p[0] > 60.).map(|p| p[1] as f64).collect();
    (v.iter().cloned().fold(f64::INFINITY, f64::min), v.iter().cloned().fold(f64::NEG_INFINITY, f64::max))
}

fn com_limites(f: impl FnOnce(&mut Document) -> LimitesAba) -> (Document, Id) {
    let (mut d, aba) = nervura(LimitesAba::default());
    let l = f(&mut d);
    if let FeatureKind::SmSurfFlange(sf) = &mut d.feature_mut(aba).unwrap().kind {
        sf.limites = l;
    }
    (d, aba)
}

#[test]
fn suporte_curvo_limitado_por_dois_planos_tem_a_largura_entre_eles() {
    if !kernel_disponivel() {
        return;
    }
    let (livre, a0) = nervura(LimitesAba::default());
    let rl = recalcula(&livre, false);
    assert!(ok_ou_conformacao(&rl.status[&a0]), "{:?}", rl.status[&a0]);
    let (y0, y1) = faixa_y(&rl);
    assert!(y1 - y0 > 45., "sem limites a aba cobre a web inteira: {y0}..{y1}");
    let (d, aba) = com_limites(|d| LimitesAba { lado1: Some(plano_y(d, 10.)), lado2: Some(plano_y(d, 40.)), ..Default::default() });
    let r = recalcula(&d, false);
    assert!(ok_ou_conformacao(&r.status[&aba]), "{:?}", r.status[&aba]);
    let (a, b) = faixa_y(&r);
    assert!((b - a - 30.).abs() < 0.6, "largura da aba {} (esperado 30 = distância entre os planos): {a}..{b}", b - a);
    assert!(a > 9.5 && b < 40.5, "{a}..{b}");
    assert!(volume_total(&r) < volume_total(&rl) - 100., "aba mais curta, menos material: {} contra {}", volume_total(&r), volume_total(&rl));
    // a web continua inteira e a aba sobe o mesmo
    let (lo, hi) = caixa_da_malha(&r);
    assert!(lo[1] < 0.1 && hi[1] > 49.9, "web inteira {lo:?} {hi:?}");
    assert!(hi[2] > 18. && hi[2] < 20., "altura {}", hi[2]);
    // planificada também sai com a largura certa
    let plana = recalcula(&d, true);
    assert!(ok_ou_conformacao(&plana.status[&aba]), "{:?}", plana.status[&aba]);
}

#[test]
fn so_um_lado_limitado_para_na_curva_e_estende_o_resto() {
    if !kernel_disponivel() {
        return;
    }
    let (d, aba) = com_limites(|d| LimitesAba { lado2: Some(plano_y(d, 30.)), ..Default::default() });
    let r = recalcula(&d, false);
    assert!(ok_ou_conformacao(&r.status[&aba]), "{:?}", r.status[&aba]);
    let (a, b) = faixa_y(&r);
    assert!(a < 1. && (b - 30.).abs() < 0.5, "{a}..{b}");
}

#[test]
fn borda_final_limitada_por_uma_curva() {
    if !kernel_disponivel() {
        return;
    }
    let (d, aba) = com_limites(|d| {
        let c = d.add_geometry(RefKind::Polyline { points: vec![PointRef::Fixed([75., -20., 10.]), PointRef::Fixed([75., 80., 10.])], closed: false });
        LimitesAba { ponta: Some(LimiteAba::Curva(CurveRef::Ref(c))), ..Default::default() }
    });
    let r = recalcula(&d, false);
    assert!(ok_ou_conformacao(&r.status[&aba]), "{:?}", r.status[&aba]);
    let (_, hi) = caixa_da_malha(&r);
    // sem o limite a aba sobe a 19; com a curva em z = 10 a borda final para ali
    assert!((hi[2] - 10.).abs() < 0.3, "altura {} (esperado 10)", hi[2]);
    assert!(volume_total(&r) > 7000., "{}", volume_total(&r));
    // limite acima do comprimento: a aba se estende até ele
    let (d, aba) = com_limites(|d| {
        let c = d.add_geometry(RefKind::Polyline { points: vec![PointRef::Fixed([75., -20., 30.]), PointRef::Fixed([75., 80., 30.])], closed: false });
        LimitesAba { ponta: Some(LimiteAba::Curva(CurveRef::Ref(c))), ..Default::default() }
    });
    let r = recalcula(&d, false);
    assert!(ok_ou_conformacao(&r.status[&aba]), "{:?}", r.status[&aba]);
    let (_, hi) = caixa_da_malha(&r);
    assert!((hi[2] - 30.).abs() < 0.3, "estendida até o limite: {}", hi[2]);
}

#[test]
fn chanfro_num_lado_e_filete_no_outro_tiram_material() {
    if !kernel_disponivel() {
        return;
    }
    let base = |d: &mut Document| LimitesAba { lado1: Some(plano_y(d, 10.)), lado2: Some(plano_y(d, 40.)), ..Default::default() };
    let (d, aba) = com_limites(base);
    let sem = volume_total(&recalcula(&d, false));
    let (d, aba2) = com_limites(|d| LimitesAba { canto1: CantoAba::Chanfro(s("2")), canto2: CantoAba::Filete(s("2")), ..base(d) });
    assert_eq!(aba, aba2);
    let r = recalcula(&d, false);
    assert!(ok_ou_conformacao(&r.status[&aba]), "{:?}", r.status[&aba]);
    let com = volume_total(&r);
    assert!(com < sem - 1., "cantos tiram material: {com} contra {sem}");
    // cada canto sozinho tira menos que os dois juntos
    let (d1, _) = com_limites(|d| LimitesAba { canto1: CantoAba::Chanfro(s("2")), ..base(d) });
    let (d2, _) = com_limites(|d| LimitesAba { canto2: CantoAba::Filete(s("2")), ..base(d) });
    let (v1, v2) = (volume_total(&recalcula(&d1, false)), volume_total(&recalcula(&d2, false)));
    assert!(v1 < sem - 0.3 && v2 < sem - 0.3 && com < v1 && com < v2, "sem {sem}, chanfro {v1}, filete {v2}, os dois {com}");
    // a caixa não muda: o canto só tira as pontas da borda
    let (_, hi) = caixa_da_malha(&r);
    assert!(hi[2] > 18. && hi[2] < 20., "{}", hi[2]);
    // planificada: o canto também sai
    let plana = recalcula(&d, true);
    assert!(ok_ou_conformacao(&plana.status[&aba]), "{:?}", plana.status[&aba]);
}

#[test]
fn limite_que_nao_intercepta_o_suporte_da_erro_claro() {
    if !kernel_disponivel() {
        return;
    }
    // plano x = 200, paralelo à linha de dobra: nunca se cruzam
    let (d, aba) = com_limites(|d| {
        let p = d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::YZ), offset: s("200") });
        LimitesAba { lado1: Some(LimiteAba::Superficie(SurfRef::Plane(PlaneRef::Datum(p)))), ..Default::default() }
    });
    let r = recompute(&d);
    match &r.status[&aba] {
        Status::Error(m) => assert!(m.contains("limite do lado 1") && m.contains("não intercepta"), "{m}"),
        o => panic!("deveria falhar com erro claro: {o:?}"),
    }
    // lados que se cruzam: lado 1 depois do lado 2
    let (d, aba) = com_limites(|d| LimitesAba { lado1: Some(plano_y(d, 40.)), lado2: Some(plano_y(d, 10.)), ..Default::default() });
    let r = recompute(&d);
    match &r.status[&aba] {
        Status::Error(m) => assert!(m.contains("lados 1 e 2"), "{m}"),
        o => panic!("deveria falhar com erro claro: {o:?}"),
    }
    // limite da borda final que não toca a aba
    let (d, aba) = com_limites(|d| {
        let c = d.add_geometry(RefKind::Polyline { points: vec![PointRef::Fixed([75., -20., -50.]), PointRef::Fixed([75., 80., -50.])], closed: false });
        LimitesAba { ponta: Some(LimiteAba::Curva(CurveRef::Ref(c))), ..Default::default() }
    });
    let r = recompute(&d);
    assert!(matches!(&r.status[&aba], Status::Error(m) if m.contains("borda final")), "{:?}", r.status[&aba]);
}

#[test]
fn documento_antigo_sem_limites_abre_igual() {
    let (d, aba) = nervura(LimitesAba::default());
    let j = serde_json::to_string(&d.feature(aba).unwrap().kind).unwrap();
    assert!(!j.contains("limites"), "sem limites o arquivo não muda: {j}");
    let k: FeatureKind = serde_json::from_str(&j).unwrap();
    assert!(matches!(k, FeatureKind::SmSurfFlange(sf) if sf.limites.vazio()));
}
