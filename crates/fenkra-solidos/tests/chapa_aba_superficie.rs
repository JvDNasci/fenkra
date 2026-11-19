// SPDX-License-Identifier: LGPL-2.1-or-later
//! B04: aba por superfície com suporte curvo (nervura de asa) e estado de falha explícito quando o
//! suporte some.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.to_string()
}

/// Web de 100 × 50 e um suporte curvo (spline em planta, extrudada em Z) que corta a web perto de
/// x = 80: a aba sobe seguindo o contorno do suporte.
fn nervura(spline: bool) -> (Document, Id, Id) {
    let mut d = Document::new("nervura");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let pts: Vec<PointRef> = (0..=12)
        .map(|i| {
            let y = -10. + 70. * i as f64 / 12.;
            // contorno de asa: abaulado suave, x entre 70 e 80
            PointRef::Fixed([75. + 5. * (y / 60. * std::f64::consts::PI).sin(), y, 0.])
        })
        .collect();
    let curva = d.add_geometry(if spline { RefKind::Spline { points: pts, closed: false, tangents: vec![] } } else { RefKind::Polyline { points: pts, closed: false } });
    let sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(curva), dir: AxisRef::Origin(OriginAxis::Z), length: s("60"), length2: s("0"), until: None, until2: None });
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Ref(sup), face: [40., 25., 2.], length: s("15"), radius: None, flip: false, keep_base: false, mode: 0, angle: s("90"), limites: Default::default() }));
    (d, sup, aba)
}

fn recalcula(d: &Document, flat: bool) -> RecomputeResult {
    let mut d = d.clone();
    d.sheet.as_mut().unwrap().flat = flat;
    recompute(&d)
}

#[test]
fn aba_por_superficie_curva_segue_a_oml_dobrada_e_planificada() {
    if !kernel_disponivel() {
        return;
    }
    for spline in [true, false] {
        let (d, _, aba) = nervura(spline);
        let r = recalcula(&d, false);
        assert_eq!(r.status[&aba], Status::Ok, "spline={spline}: {:?}", r.status[&aba]);
        let (lo, hi) = caixa_da_malha(&r);
        // a aba sobe 15 sobre a web (2) mais o raio e a espessura da dobra: topo perto de 19
        assert!(hi[2] > 18. && hi[2] < 20., "spline={spline}: altura {}", hi[2]);
        assert!(hi[0] < 90. && lo[0].abs() < 0.1, "spline={spline}: a web foi relimitada pelo suporte {lo:?} {hi:?}");
        let plana = recalcula(&d, true);
        assert_eq!(plana.status[&aba], Status::Ok, "spline={spline} plana: {:?}", plana.status[&aba]);
        let (_, hip) = caixa_da_malha(&plana);
        assert!(hip[2] < 2.1, "spline={spline}: planificada fica plana {}", hip[2]);
    }
}

#[test]
fn suporte_que_some_deixa_a_aba_em_falha_explicita() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, sup, aba) = nervura(true);
    // o suporte passa a apontar para uma superfície que não existe
    if let FeatureKind::SmSurfFlange(sf) = &mut d.feature_mut(aba).unwrap().kind {
        sf.support = SurfRef::Ref(Id(sup.0 + 9_999));
    }
    let r = recompute(&d);
    match &r.status[&aba] {
        Status::Error(m) => assert!(m.to_lowercase().contains("suporte") || m.to_lowercase().contains("superfície"), "{m}"),
        o => panic!("deveria falhar de forma explícita: {o:?}"),
    }
    // e a chapa base continua inteira (a falha não apaga o corpo)
    assert!(volume_total(&r) > 9000., "web preservada {}", volume_total(&r));
}

/// A aba por superfície entra no mapa de dobras (tabela) com ângulo e raio.
#[test]
fn aba_por_superficie_entra_no_mapa_de_dobras() {
    if !kernel_disponivel() {
        return;
    }
    let (d, _, aba) = nervura(true);
    let tabela = fenkra_solidos::bend_table(&d);
    let linha = tabela.iter().find(|l| l.feature == aba).expect("a aba por superfície na tabela de dobras");
    assert!((linha.angle - 90.).abs() < 1e-9 && (linha.radius - 2.).abs() < 1e-9, "{linha:?}");
    assert!((linha.allowance - std::f64::consts::FRAC_PI_2 * (2. + 0.44 * 2.)).abs() < 1e-6, "{}", linha.allowance);
}

/// B529: placa de 100 × 80 e suporte em arco (cilindro de raio `raio`, eixo em Z, centro fora da
/// placa em x = `cx`) que corta a placa perto da borda direita.
fn placa_com_arco(raio: f64, cx: f64, ang0: f64, ang1: f64) -> (Document, Id) {
    let mut d = Document::new("arco");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 80.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let pt = |a: f64| PointRef::Fixed([cx + raio * a.to_radians().cos(), 40. + raio * a.to_radians().sin(), 0.]);
    let curva = d.add_geometry(RefKind::Circle3 { a: pt(ang0), b: pt((ang0 + ang1) / 2.), c: pt(ang1), full: false });
    let sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(curva), dir: AxisRef::Origin(OriginAxis::Z), length: s("60"), length2: s("0"), until: None, until2: None });
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Ref(sup), face: [40., 40., 2.], length: s("15"), radius: None, flip: false, keep_base: false, mode: 0, angle: s("90"), limites: Default::default() }));
    (d, aba)
}

/// Placa de 100 × 80 e suporte em spline com y entre `y0` e `y1` (abaulado em torno de x = 80).
fn placa_com_spline(y0: f64, y1: f64, amp: f64) -> (Document, Id) {
    let mut d = Document::new("spline");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 80.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let pts: Vec<PointRef> = (0..=10)
        .map(|i| {
            let y = y0 + (y1 - y0) * i as f64 / 10.;
            PointRef::Fixed([80. + amp * (y / 80. * 2. * std::f64::consts::PI).sin(), y, 0.])
        })
        .collect();
    let curva = d.add_geometry(RefKind::Spline { points: pts, closed: false, tangents: vec![] });
    let sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(curva), dir: AxisRef::Origin(OriginAxis::Z), length: s("60"), length2: s("0"), until: None, until2: None });
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Ref(sup), face: [40., 40., 2.], length: s("15"), radius: None, flip: false, keep_base: false, mode: 0, angle: s("90"), limites: Default::default() }));
    (d, aba)
}

/// Confere o que o B529 pede: status Ok, base relimitada (nada além de `x_max`), dobra de verdade
/// (a malha tem vértices na altura do arco, entre a face da chapa e o início da parede reta),
/// altura da aba = comprimento + raio + espessura e planificação plana e Ok.
fn confere_b529(nome: &str, d: &Document, aba: Id, x_max: f64) {
    let r = recalcula(d, false);
    assert_eq!(r.status[&aba], Status::Ok, "{nome}: {:?}", r.status[&aba]);
    assert!(r.bodies.iter().all(|b| !b.mesh.positions.is_empty() && b.mass.volume > 1000.), "{nome}: corpo vazio com status Ok");
    let (lo, hi) = caixa_da_malha(&r);
    assert!(lo[0].abs() < 0.1, "{nome}: {lo:?}");
    assert!(hi[0] < x_max, "{nome}: a base não foi relimitada pelo suporte ({hi:?} >= {x_max})");
    assert!((hi[2] - 19.).abs() < 0.3, "{nome}: altura da aba {} (15 + raio 2 + espessura 2)", hi[2]);
    let dobra = r.bodies.iter().flat_map(|b| b.mesh.positions.chunks_exact(3)).any(|p| p[2] > 2.6 && p[2] < 3.8 && p[0] > 60.);
    assert!(dobra, "{nome}: sem a face da dobra (raio) entre a base e a parede");
    let plana = recalcula(d, true);
    assert_eq!(plana.status[&aba], Status::Ok, "{nome} plana: {:?}", plana.status[&aba]);
    let (_, hip) = caixa_da_malha(&plana);
    assert!(hip[2] < 2.1, "{nome}: planificada fica plana {}", hip[2]);
    assert!(volume_total(&plana) > 14000., "{nome}: volume planificado {}", volume_total(&plana));
}

/// B529: suporte em arco que atravessa a placa de ponta a ponta (a divisão exata relimita a base).
#[test]
fn b529_arco_que_atravessa_relimita_a_base_e_dobra() {
    if !kernel_disponivel() {
        return;
    }
    let (d, aba) = placa_com_arco(115., 200., 150., 210.);
    confere_b529("arco passante", &d, aba, 97.);
}

/// B529: suporte em arco finito que termina dentro da placa (não atravessa): a linha de dobra é
/// prolongada e a base é cortada mesmo assim, antes ficava inteira.
#[test]
fn b529_arco_curto_relimita_a_base_e_dobra() {
    if !kernel_disponivel() {
        return;
    }
    let (d, aba) = placa_com_arco(115., 200., 165., 195.);
    confere_b529("arco curto", &d, aba, 94.);
    // arco de raio 40 com o centro fora da placa: o lado do centro sai, os cantos ficam
    let (d, aba) = placa_com_arco(40., 125., 150., 210.);
    let r = recalcula(&d, false);
    assert_eq!(r.status[&aba], Status::Ok, "{:?}", r.status[&aba]);
    assert!(volume_total(&r) < 16500., "o lado cortado saiu: {}", volume_total(&r));
}

/// B529: suporte em spline de curvatura variável, atravessando e terminando dentro da placa.
#[test]
fn b529_spline_relimita_a_base_e_dobra() {
    if !kernel_disponivel() {
        return;
    }
    let (d, aba) = placa_com_spline(-10., 90., 3.);
    confere_b529("spline passante", &d, aba, 90.);
    let (d, aba) = placa_com_spline(15., 65., 3.);
    confere_b529("spline curta", &d, aba, 90.);
}
