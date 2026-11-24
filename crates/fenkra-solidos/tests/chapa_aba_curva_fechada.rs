// SPDX-License-Identifier: LGPL-2.1-or-later
//! Aba em aresta que não é reta (teste da alpha.38): círculo fechado inteiro, arco parcial e cadeia
//! reta + arco de um retângulo de cantos arredondados.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn chapa(d: &mut Document) {
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
}

fn aba(edge: Vec<[f64; 3]>, len: &str, ang: &str) -> FeatureKind {
    FeatureKind::SmCurveFlange(SmCurveFlange { edge, length: s(len), flip: false, angle: s(ang), radius: None, side: 0, edge_key: None, so_aresta: false, reta: None })
}

fn circulo(r: f64, z: f64, n: usize) -> Vec<[f64; 3]> {
    (0..=n).map(|i| {
        let t = i as f64 / n as f64 * std::f64::consts::TAU;
        [r * t.cos(), r * t.sin(), z]
    }).collect()
}

fn confere_ok(d: &Document) -> fenkra_solidos::RecomputeResult {
    let r = recompute(d);
    for f in &d.features {
        assert!(ok_ou_conformacao(&r.status[&f.id]), "{}: {:?}", f.name, r.status[&f.id]);
    }
    r
}

#[test]
fn aba_no_circulo_inteiro() {
    if !kernel_disponivel() {
        return;
    }
    for ang in ["90", "45"] {
        let mut d = Document::new("disco");
        chapa(&mut d);
        let sk = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([0., 0.], 40.)]);
        d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
        d.add_feature(aba(circulo(40., 2., 48), "10", ang));
        let r = confere_ok(&d);
        let base = std::f64::consts::PI * 1600. * 2.;
        let v = r.bodies[0].mass.volume;
        assert!(v > base + 1500., "ângulo {ang}: volume {v} não cresceu com a aba");
        let (min, max) = caixa_da_malha(&r);
        assert!(max[2] > 8., "ângulo {ang}: a aba não subiu ({max:?})");
        if ang == "90" {
            assert!((max[2] - 14.).abs() < 0.2, "altura {max:?}");
            assert!((max[0] - 44.).abs() < 0.3 && (min[0] + 44.).abs() < 0.3, "raio externo {min:?} {max:?}");
        }
    }
}

#[test]
fn aba_no_retangulo_arredondado() {
    if !kernel_disponivel() {
        return;
    }
    // retângulo 100 × 60 com cantos R10: a aba segue a borda de cima inteira (reta + arco + reta)
    let mut d = Document::new("arredondado");
    chapa(&mut d);
    let mut data = SketchData::default();
    let mut n = 7_100_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    fn p(data: &mut SketchData, al: &mut impl FnMut() -> Id, x: f64, y: f64) -> Id {
        data.add_point(al, x, y)
    }
    let (r, w, h) = (10., 100., 60.);
    let a1 = p(&mut data, &mut al, r, 0.);
    let a2 = p(&mut data, &mut al, w - r, 0.);
    let c2 = p(&mut data, &mut al, w - r, r);
    let b2 = p(&mut data, &mut al, w, r);
    let b3 = p(&mut data, &mut al, w, h - r);
    let c3 = p(&mut data, &mut al, w - r, h - r);
    let a3 = p(&mut data, &mut al, w - r, h);
    let a4 = p(&mut data, &mut al, r, h);
    let c4 = p(&mut data, &mut al, r, h - r);
    let b4 = p(&mut data, &mut al, 0., h - r);
    let b1 = p(&mut data, &mut al, 0., r);
    let c1 = p(&mut data, &mut al, r, r);
    use fenkra_core::sketch::Curve;
    for c in [
        Curve::Line { a: a1, b: a2 },
        Curve::Arc { c: c2, s: a2, e: b2 },
        Curve::Line { a: b2, b: b3 },
        Curve::Arc { c: c3, s: b3, e: a3 },
        Curve::Line { a: a3, b: a4 },
        Curve::Arc { c: c4, s: a4, e: b4 },
        Curve::Line { a: b4, b: b1 },
        Curve::Arc { c: c1, s: b1, e: a1 },
    ] {
        data.add_curve(&mut al, c, false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    // borda direita + canto de cima + borda de cima (cadeia tangente aberta)
    let mut edge = vec![[w, r, 2.], [w, h / 2., 2.], [w, h - r, 2.]];
    for i in 1..=12 {
        let t = i as f64 / 12. * std::f64::consts::FRAC_PI_2;
        edge.push([w - r + r * t.cos(), h - r + r * t.sin(), 2.]);
    }
    edge.extend([[w / 2., h, 2.], [r, h, 2.]]);
    d.add_feature(aba(edge, "12", "90"));
    let r0 = confere_ok(&d);
    let (_, max) = caixa_da_malha(&r0);
    assert!((max[2] - 16.).abs() < 0.2, "altura {max:?}");
    // planifica
    d.sheet.as_mut().unwrap().flat = true;
    let rf = confere_ok(&d);
    let (_, maxf) = caixa_da_malha(&rf);
    assert!(maxf[2] < 2.01, "planificada é plana {maxf:?}");
}
