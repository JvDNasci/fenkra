// SPDX-License-Identifier: LGPL-2.1-or-later
//! Laço de abas num contorno com curva (B530): retângulo 100 × 60 com cantos R10, laço inteiro.

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

fn confere_ok(d: &Document) -> fenkra_solidos::RecomputeResult {
    let r = recompute(d);
    for f in &d.features {
        assert_eq!(r.status[&f.id], Status::Ok, "{}: {:?}", f.name, r.status[&f.id]);
    }
    r
}

fn laco(pontos: Vec<[f64; 3]>, todo: bool) -> FeatureKind {
    FeatureKind::SmFlangeLoop(SmFlangeLoop {
        edges: pontos,
        edge_keys: vec![],
        loop_all: todo,
        extent: ExtensaoAba::Inteira,
        invert: false,
        length: s("12"),
        angle: s("90"),
        radius: None,
        flip: false,
        side: 0,
        miter: true,
        relief: None,
        relief_kind: 0,
    })
}

#[test]
fn laco_inteiro_do_retangulo_arredondado() {
    if !kernel_disponivel() {
        return;
    }
    // retângulo 100 × 60 com cantos R10: a aba segue a borda de cima inteira (reta + arco + reta)
    let mut d = Document::new("laco_arredondado");
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
    let parede = d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let (r0, w0, h0) = (10., 100., 60.);
    let _ = (r0, w0, h0, parede);
    // base sem aba: volume da chapa
    let vbase = recompute(&d).bodies[0].mass.volume;
    // clicar numa aresta reta (borda de cima) com "laço da face inteira"
    d.add_feature(laco(vec![[w / 2., h, 2.]], true));
    let r1 = confere_ok(&d);
    let (_, max) = caixa_da_malha(&r1);
    assert!((max[2] - 16.).abs() < 0.2, "altura {max:?}");
    let v = r1.bodies[0].mass.volume;
    // contorno 2*(80+40) + 2*pi*10 = 302,8 mm; seção da aba = espessura 2 × (perna 12 + dobra ~4,5)
    // = ~33 mm², a ~1 mm da aresta para fora: volume ~ 302,8 * 33 = ~10 000 mm³
    let acresc = v - vbase;
    assert!(acresc > 9000. && acresc < 11500., "volume da aba {acresc}");
    // cobre o contorno todo: bordas e cantos têm parede (100 x 60 + r + t = 4 mm para fora de cada lado).
    // A varredura do kernel ainda deixa até ~1 mm de folga na zona da dobra (aproximação), por isso 1,5 mm.
    let (min, max) = caixa_da_malha(&r1);
    assert!((max[0] - min[0] - 108.).abs() < 1.5 && (max[1] - min[1] - 68.).abs() < 2., "caixa {min:?} {max:?}");
    // planifica
    d.sheet.as_mut().unwrap().flat = true;
    let rf = confere_ok(&d);
    let (_, maxf) = caixa_da_malha(&rf);
    assert!(maxf[2] < 2.01, "planificada é plana {maxf:?}");
}
