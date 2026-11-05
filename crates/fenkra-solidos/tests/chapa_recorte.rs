// SPDX-License-Identifier: LGPL-2.1-or-later
//! B02: recorte desenhado no planificado vale no dobrado, inclusive no raio da dobra.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn chapa_com_aba() -> Document {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
    d
}

fn recalcula(d: &Document, flat: bool) -> RecomputeResult {
    let mut d = d.clone();
    d.sheet.as_mut().unwrap().flat = flat;
    recompute(&d)
}

fn tudo_ok(d: &Document, r: &RecomputeResult, rotulo: &str) {
    for f in &d.features {
        assert_eq!(r.status[&f.id], Status::Ok, "{rotulo}: {}: {:?}", f.name, r.status[&f.id]);
    }
}

fn recorte(d: &mut Document, a: [f64; 2], b: [f64; 2], normal_face: bool) {
    let sk = sk_rect(d, PlaneRef::Origin(OriginPlane::XY), a, b);
    d.add_feature(FeatureKind::SmCut(SmCut { sketch: sk, mode: 0, offset: String::new(), draft: s("0"), height: s("0"), flip: true, normal_face }));
}

/// Retângulo desenhado na chapa planificada que atravessa a dobra (x de 98 a 104, y de 10 a 20): no
/// planificado tira 6×10×2 = 120 mm³. No dobrado, tira 2×10×2 da base e a fatia da dobra que cabe
/// em 4 mm de fibra neutra: ½·((r+t)² − r²)·φ·10, com φ = 4 / (r + k·t).
#[test]
fn recorte_do_planificado_atravessa_o_raio_da_dobra() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = chapa_com_aba();
    let (v_dobrada, v_plana) = (volume_total(&recalcula(&d, false)), volume_total(&recalcula(&d, true)));
    d.sheet.as_mut().unwrap().flat = true;
    recorte(&mut d, [98., 10.], [104., 20.], false);
    d.sheet.as_mut().unwrap().flat = false;

    let plana = recalcula(&d, true);
    tudo_ok(&d, &plana, "planificada");
    assert!((v_plana - volume_total(&plana) - 120.).abs() < 3., "planificada tirou {}", v_plana - volume_total(&plana));

    let dobrada = recalcula(&d, false);
    tudo_ok(&d, &dobrada, "dobrada");
    let phi = 4. / (2. + 0.44 * 2.);
    let esperado = 2. * 10. * 2. + 0.5 * (16. - 4.) * phi * 10.;
    let tirou = v_dobrada - volume_total(&dobrada);
    assert!((tirou - esperado).abs() < 0.04 * esperado, "dobrada tirou {tirou} ≈ {esperado}");
    let (_, hi) = caixa_da_malha(&dobrada);
    assert!((hi[2] - 24.).abs() < 0.2, "a aba continua com a altura {}", hi[2]);
}

/// "Normal à face": o mesmo contorno corta reto a partir do plano do sketch, sem acompanhar a chapa.
#[test]
fn recorte_normal_a_face_nao_acompanha_a_dobra() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = chapa_com_aba();
    let v0 = volume_total(&recalcula(&d, false));
    d.sheet.as_mut().unwrap().flat = true;
    recorte(&mut d, [98., 10.], [104., 20.], true);
    d.sheet.as_mut().unwrap().flat = false;
    let dobrada = recalcula(&d, false);
    tudo_ok(&d, &dobrada, "normal");
    let tirou = v0 - volume_total(&dobrada);
    // reto pelo plano XY: leva também a perna vertical (z até 10), bem mais que a fatia da dobra
    assert!(tirou > 150., "tirou {tirou}");
}

/// B02: o contorno do recorte sai nos dois estados ao mesmo tempo (prévia simultânea). O canto que cai
/// na zona da dobra segue o arco (fibra neutra mantém o comprimento) e o resto fica onde está; a
/// planificada projetada no sketch devolve o retângulo original para cotar contra ele.
#[test]
fn contorno_do_recorte_nos_dois_estados_e_projecao_no_sketch() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = chapa_com_aba();
    d.sheet.as_mut().unwrap().flat = true;
    recorte(&mut d, [98., 10.], [104., 20.], false);
    let cut = d.features.last().unwrap().id;
    for flat in [false, true] {
        let r = recalcula(&d, flat);
        tudo_ok(&d, &r, "contorno");
        assert_eq!(r.cut_outlines.len(), 1, "um contorno por recorte (planificada: {flat})");
        let o = &r.cut_outlines[0];
        assert_eq!(o.feature, cut);
        let pts = |l: &Vec<Vec<[f64; 3]>>| l.iter().flatten().copied().collect::<Vec<_>>();
        // planificado: o retângulo desenhado, no plano do sketch
        assert!(pts(&o.flat).iter().all(|p| (97.99..=104.01).contains(&p[0]) && (9.99..=20.01).contains(&p[1]) && p[2].abs() < 1e-6), "planificado {:?}", o.flat);
        // dobrado: x = 98 não mexe; x = 104 cai na zona (ba = 4,52) e sobe pelo arco de raio r + t = 4
        let dob = pts(&o.folded);
        assert!(dob.iter().any(|p| (p[0] - 98.).abs() < 1e-6 && p[2].abs() < 1e-6), "o canto fora da dobra fica parado");
        let phi: f64 = 4. / (2. + 0.44 * 2.);
        let (x, z) = (100. + 4. * phi.sin(), 4. - 4. * phi.cos());
        assert!(dob.iter().any(|p| (p[0] - x).abs() < 0.02 && (p[2] - z).abs() < 0.02), "canto na zona da dobra esperado em ({x:.2}, {z:.2}): {dob:?}");
    }
    // projetar a planificada no sketch: o retângulo de volta, em coordenadas do plano
    let uv = fenkra_solidos::contorno_planificado_no_sketch(&d, cut).unwrap();
    let todos: Vec<[f64; 2]> = uv.iter().flatten().copied().collect();
    assert!(!todos.is_empty() && todos.iter().all(|p| (97.99..=104.01).contains(&p[0]) && (9.99..=20.01).contains(&p[1])), "{todos:?}");
    // normal à face: os dois estados mostram o mesmo contorno reto
    let mut d = chapa_com_aba();
    d.sheet.as_mut().unwrap().flat = true;
    recorte(&mut d, [98., 10.], [104., 20.], true);
    let r = recalcula(&d, false);
    assert_eq!(r.cut_outlines[0].flat, r.cut_outlines[0].folded);
}
