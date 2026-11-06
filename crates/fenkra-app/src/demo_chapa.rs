// SPDX-License-Identifier: LGPL-2.1-or-later
//! Peças de partida de `FENKRA_DEMO_CHAPA` que não nascem de comandos de chapa (B35).

use fenkra_core::sketch::Curve;
use fenkra_core::*;

fn s(x: &str) -> String {
    x.to_string()
}

/// Canal em U vindo "de fora" (extrusão de uma seção com arcos exatos, sem comando de chapa) e o
/// comando Converter em chapa sobre ele. Com `plana`, a chapa já está planificada: o corpo aparece
/// desdobrado pelo mapa de dobras que a conversão montou.
pub fn canal_convertido(plana: bool) -> Document {
    let mut d = Document::new("Canal convertido");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let mut data = SketchData::default();
    let mut n = 9_100_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let pts = [(0., 20.), (0., 4.), (4., 0.), (40., 0.), (44., 4.), (44., 20.), (42., 20.), (42., 4.), (40., 2.), (4., 2.), (2., 4.), (2., 20.), (4., 4.), (40., 4.)];
    let p: Vec<Id> = pts.iter().map(|(x, y)| data.add_point(&mut al, *x, *y)).collect();
    for (a, b) in [(0, 1), (2, 3), (4, 5), (5, 6), (6, 7), (8, 9), (10, 11), (11, 0)] {
        data.add_curve(&mut al, Curve::Line { a: p[a], b: p[b] }, false);
    }
    for (c, a, b) in [(12, 1, 2), (13, 3, 4), (13, 8, 7), (12, 10, 9)] {
        data.add_curve(&mut al, Curve::Arc { c: p[c], s: p[a], e: p[b] }, false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("50")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    let id = d.add_feature(FeatureKind::SmConvert(SmConvert { face: [0.; 3], thickness: String::new() }));
    d.feature_mut(id).map(|f| f.name = "ConverterEmChapa".into());
    if plana {
        if let Some(sp) = d.sheet.as_mut() {
            sp.flat = true;
        }
    }
    d
}
