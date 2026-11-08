// SPDX-License-Identifier: LGPL-2.1-or-later
//! B388 (K-10 do kart, RD-01 e RD-02): volume comum falso entre o aro e as peças dentro dele.
//!
//! A chaveta solta no vão do aro acusava o volume dela inteiro, e aro × pneu passava do volume do
//! aro. A causa era o pino do talão extrudado na direção X a partir de um sketch no plano XY: a
//! direção fica no plano, o perfil varre a si mesmo e sobra um sólido sem espessura que estraga o
//! corpo todo, com status ok. Hoje essa extrusão é erro (B413), e o pino certo (sketch num plano
//! normal a X) dá um aro válido, com volume comum 0 para a chaveta e menor que o aro para o pneu.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_kernel::Bool;
use fenkra_solidos::{final_shape, recompute, Status};

fn xy() -> PlaneRef {
    PlaneRef::Origin(OriginPlane::XY)
}

/// Aro do kart (largura 210): revolução, alma com furos em padrão e três pinos de talão.
/// `radial`: o pino desenhado num plano normal a X (certo); senão o do kart, com direção X no XY.
fn aro(radial: bool) -> (Document, Id) {
    let mut d = Document::new("aro");
    let h = 105.;
    let perfil = sk_poligono(&mut d, xy(), &[[58., -h], [68.1, -h], [68.1, -h + 5.], [63.1, -h + 8.], [63.1, h - 8.], [68.1, h - 5.], [68.1, h], [58., h]]);
    d.add_feature(FeatureKind::Revolve(Revolve { sketch: perfil, axis: AxisRef::Origin(OriginAxis::Y), angle: "360".into(), symmetric: false, op: BoolOp::Add, thin: None, thin_side: 0, angle2: None }));
    let alma = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XZ), &[([0., 0.], 59.), ([0., 0.], 27.)]);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: alma, op: BoolOp::Add, extent: Extent::Symmetric("8".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XZ), "0");
    let sf = sk_pontos(&mut d, PlaneRef::Datum(pl), &[[42., 0.]]);
    let furo = d.add_feature(furo_passante(sf, "8.5"));
    d.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Y), count: "4".into(), angle: "360".into() }, sources: vec![furo], skip: vec![] }));
    let (sp, dir) = if radial {
        // plano YZ deslocado até a superfície do aro: x local = Y, y local = Z
        let pp = d.add_offset_plane(PlaneRef::Origin(OriginPlane::YZ), "63.1");
        (sk_circulos(&mut d, PlaneRef::Datum(pp), &[([-h + 12., 0.], 1.5)]), None)
    } else {
        let pp = d.add_offset_plane(xy(), "0");
        (sk_circulos(&mut d, PlaneRef::Datum(pp), &[([63.1, -h + 12.], 1.5)]), Some(AxisRef::Origin(OriginAxis::X)))
    };
    let pino = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sp, op: BoolOp::Add, extent: Extent::Symmetric("6".into()), reverse: false, taper: "0".into(), thin: None, dir, thin_side: 0, ends: Default::default() }));
    d.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Y), count: "3".into(), angle: "360".into() }, sources: vec![pino], skip: vec![] }));
    (d, pino)
}

/// Pneu traseiro do kart (Ø276 x 210): perfil em spline girado em Y e engrossado 9 mm.
fn pneu() -> Document {
    let mut d = Document::new("pneu");
    let (r, h) = (138., 105.);
    let pts: Vec<PointRef> = [(66., 0.92), ((66. + r) / 2., 1.0), (r - 8., 0.96), (r, 0.7), (r, 0.), (r, -0.7), (r - 8., -0.96), ((66. + r) / 2., -1.0), (66., -0.92)].iter().map(|(x, y)| PointRef::Fixed([*x, y * h, 0.])).collect();
    let c = d.add_geometry(RefKind::Spline { points: pts, closed: false, tangents: vec![] });
    let sup = d.add_geometry(RefKind::SurfRevolve { curve: CurveRef::Ref(c), axis: AxisRef::Origin(OriginAxis::Y), angle: "360".into(), angle2: "0".into() });
    d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(sup), t1: "9".into(), t2: "0".into(), op: BoolOp::Add }));
    d
}

/// Chaveta DIN 6885 14x9x40 do eixo Ø50, na origem: fica no vão da alma (raio 27), sem tocar o aro.
fn chaveta() -> Document {
    fenkra_solidos::eixos::chaveta_documento(50., 40.).unwrap()
}

fn comum(a: &Document, b: &Document) -> f64 {
    let (sa, sb) = (final_shape(a).unwrap(), final_shape(b).unwrap());
    sa.boolean(&sb, Bool::Common).map(|c| c.mass().volume).unwrap_or(0.)
}

#[test]
fn aro_com_pino_radial_e_valido_e_nao_interfere_com_a_chaveta_nem_engole_o_pneu() {
    if !kernel_disponivel() {
        return;
    }
    let (d, _) = aro(true);
    let r = recompute(&d);
    for f in &d.features {
        assert_eq!(r.status[&f.id], Status::Ok, "{}: {:?}", f.name, r.status[&f.id]);
    }
    let forma = final_shape(&d).unwrap();
    assert!(forma.is_valid(), "aro inválido: {}", fenkra_solidos::validade::diagnostico(&forma));
    // aro sem pinos (RD-02A) 499.477 mm³ + 3 pinos de Ø3 x 6 dentro e fora da superfície
    let v = volume_total(&r);
    assert!(v > 499_477. && v < 499_477. + 3. * std::f64::consts::PI * 2.25 * 12., "{v}");
    let (a, b) = caixa_da_malha(&r);
    assert!((b[1] - a[1] - 210.).abs() < 0.1, "largura do aro {a:?} {b:?}");
    let ch = chaveta();
    assert!(comum(&d, &ch) < 1e-6, "chaveta solta no vão: volume comum {}", comum(&d, &ch));
    let pn = pneu();
    let vp = recompute(&pn).bodies.iter().map(|b| b.mass.volume).sum::<f64>();
    let c = comum(&d, &pn);
    assert!(c < v && c < vp, "aro × pneu {c} passa do aro {v} ou do pneu {vp}");
}

#[test]
fn pino_do_kart_com_direcao_no_plano_do_sketch_e_erro_e_o_aro_segue_sem_interferencia_falsa() {
    if !kernel_disponivel() {
        return;
    }
    let (d, pino) = aro(false);
    let r = recompute(&d);
    assert!(matches!(&r.status[&pino], Status::Error(m) if m.contains("plano do sketch")), "{:?}", r.status[&pino]);
    // o corpo fica o de antes do pino: válido, e a chaveta não acusa volume comum
    assert!(final_shape(&d).unwrap().is_valid());
    assert!(comum(&d, &chaveta()) < 1e-6);
}
