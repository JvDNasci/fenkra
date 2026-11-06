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
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Ref(sup), face: [40., 25., 2.], length: s("15"), radius: None, flip: false, keep_base: false, mode: 0, angle: s("90") }));
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
