// SPDX-License-Identifier: LGPL-2.1-or-later
//! B558: parede base com um perfil de asa (spline fechada, ou spline mais a linha do bordo de fuga).
//! A chapa tem de sair como placa plana preenchida pelo perfil, na mesma curva da extrusão.

use super::*;
use fenkra_core::sketch::Curve;

/// Contorno de um aerofólio simétrico (corda 200 mm, espessura relativa 12 %), do bordo de fuga
/// pela face superior até o bordo de ataque e de volta pela inferior.
fn pontos_asa(n: usize) -> Vec<[f64; 2]> {
    let c = 200.0;
    let y = |x: f64| {
        let t = x / c;
        5. * 0.12 * c * (0.2969 * t.sqrt() - 0.126 * t - 0.3516 * t * t + 0.2843 * t.powi(3) - 0.1015 * t.powi(4))
    };
    let xs: Vec<f64> = (0..=n).map(|i| c * (1. - i as f64 / n as f64)).collect();
    let mut p: Vec<[f64; 2]> = xs.iter().map(|x| [*x, y(*x)]).collect();
    for x in xs.iter().rev().skip(1) {
        p.push([*x, -y(*x)]);
    }
    p
}

fn sketch_asa(fechada: bool, plane: PlaneRef) -> (Document, Id) {
    let mut d = Document::new("asa");
    d.sheet = Some(SheetParams::default());
    let mut data = SketchData::default();
    let mut n = 8_800_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let p = pontos_asa(10);
    let mut ids: Vec<Id> = p.iter().map(|q| data.add_point(&mut al, q[0], q[1])).collect();
    if fechada {
        let primeiro = ids[0];
        *ids.last_mut().unwrap() = primeiro; // mesmo ponto no começo e no fim
        data.add_curve(&mut al, Curve::Spline { pts: ids }, false);
    } else {
        // bordo de fuga fechado por uma linha reta
        let (a, b) = (ids[0], *ids.last().unwrap());
        data.add_curve(&mut al, Curve::Spline { pts: ids }, false);
        data.add_curve(&mut al, Curve::Line { a: b, b: a }, false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane, data }));
    (d, sk)
}

fn ok(st: Option<&Status>) -> bool {
    matches!(st, Some(Status::Ok | Status::Warning(_)))
}

fn confere(plane_de: impl Fn(&mut Document) -> PlaneRef, fechada: bool) {
    // referência: a extrusão do mesmo sketch
    let mut pre = Document::new("asa");
    let pl = plane_de(&mut pre);
    let (mut dc, sk) = sketch_asa(fechada, pl);
    dc.references = pre.references.clone();
    let ex = dc.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("2".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let r = recompute(&dc);
    assert_eq!(r.status[&ex], Status::Ok, "extrusão: {:?}", r.status[&ex]);
    let vex = r.bodies[0].mass.volume;
    let bex = final_shape(&dc).unwrap().bbox().unwrap();
    // a chapa, no mesmo sketch
    let (mut ds, sk) = sketch_asa(fechada, pl);
    ds.references = pre.references.clone();
    let w = ds.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let r = recompute(&ds);
    assert!(ok(r.status.get(&w)), "parede: {:?}", r.status.get(&w));
    assert_eq!(r.bodies.len(), 1);
    let v = r.bodies[0].mass.volume;
    assert!((v - vex).abs() < 1e-6 * vex.abs().max(1.), "volume da chapa {v} contra a extrusão {vex}");
    let b = final_shape(&ds).unwrap().bbox().unwrap();
    for i in 0..3 {
        assert!((b.0[i] - bex.0[i]).abs() < 1e-3 && (b.1[i] - bex.1[i]).abs() < 1e-3, "caixa {b:?} contra {bex:?}");
    }
}

#[test]
fn parede_base_com_asa_de_spline_fechada() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    confere(|_| PlaneRef::Origin(OriginPlane::XY), true);
}

#[test]
fn parede_base_com_asa_de_spline_e_linha() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    confere(|_| PlaneRef::Origin(OriginPlane::XY), false);
}

#[test]
fn parede_base_com_asa_em_plano_inclinado() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    for fechada in [true, false] {
        confere(
            |d| {
                let n = geo3d::unit([1., 2., 3.]).unwrap();
                let x = geo3d::unit(fenkra_kernel::cross(n, [0., 0., 1.])).unwrap();
                PlaneRef::Datum(d.add_face_plane([30., -20., 15.], x, n, None))
            },
            fechada,
        );
    }
}

/// Spline fechada (mesmo ponto no começo e no fim, como o clique no primeiro ponto) levada ao modo de
/// pontos de controle: a curva não pode sair da caixa do perfil (laço enorme).
#[test]
fn asa_fechada_no_modo_de_controle_fica_dentro_do_perfil() {
    let (mut d, _) = sketch_asa(true, PlaneRef::Origin(OriginPlane::XY));
    let FeatureKind::Sketch(s) = &mut d.features[0].kind else { unreachable!() };
    let sp = s.data.curves[0].id;
    assert!(s.data.spline_c2(sp));
    let uv = s.data.spline_uv(sp);
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for q in &uv {
        for i in 0..2 {
            lo[i] = lo[i].min(q[i]);
            hi[i] = hi[i].max(q[i]);
        }
    }
    assert!(lo[0] > -3. && hi[0] < 203. && lo[1] > -16. && hi[1] < 16., "a spline escapou do perfil: {lo:?} {hi:?}");
}
