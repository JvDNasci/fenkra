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

/// B03: dobra por linha numa spline desenhada na chapa (parede que segue a curva). Com `plana`, a
/// chapa aparece planificada com a linha de dobra curva.
pub fn spline(d: &mut Document, plana: bool) {
    d.features.truncate(2);
    let mut data = SketchData::default();
    let mut n = 9_200_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let p: Vec<Id> = [[55., 0.], [70., 30.], [55., 60.]].iter().map(|q| data.add_point(&mut al, q[0], q[1])).collect();
    data.add_curve(&mut al, Curve::Spline { pts: p }, false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    let id = d.add_feature(FeatureKind::SmBend(SmBend { a: [0.; 3], b: [0.; 3], angle: s("90"), radius: None, flip: false, down: false, sketch: Some(sk), position: 0 }));
    d.feature_mut(id).map(|f| f.name = "DobraSpline".into());
    if let Some(sp) = d.sheet.as_mut() {
        sp.flat = plana;
    }
}

/// B243 e B249: os três comandos de linha desenhada na face, com nome fixo para `FENKRA_EDITAR`:
/// `DobraLinha` (dobra por linha), `Joggle` e `Veneziana`. Com `nova`, a dobra nasce sem linha
/// (as duas pontas iguais), no passo de escolher a face fixa.
pub fn linhas(d: &mut Document, nova: bool) {
    d.features.truncate(2);
    let mut nomeia = |d: &mut Document, k: FeatureKind, nome: &str| {
        let id = d.add_feature(k);
        d.feature_mut(id).map(|f| f.name = nome.into());
    };
    nomeia(d, FeatureKind::SmLouver(SmLouver { a: [75., 15., 2.], b: [75., 45., 2.], width: s("10"), height: s("4"), flip: false, down: false }), "Veneziana");
    nomeia(d, FeatureKind::SmJoggle(SmJoggle { a: [45., 0., 2.], b: [45., 60., 2.], depth: s("4"), run: s("6"), flip: false, angle: None, plane: None, keep_length: false }), "Joggle");
    let (a, b) = if nova { ([0.; 3], [0.; 3]) } else { ([20., 0., 2.], [20., 60., 2.]) };
    nomeia(d, FeatureKind::SmBend(SmBend { a, b, angle: s("45"), radius: None, flip: false, down: false, sketch: None, position: 0 }), "DobraLinha");
}

/// B35: aba entre perfis, um funil de quadrado 40 embaixo para quadrado 20 a 30 mm de altura, com o
/// nome `Funil` para `FENKRA_EDITAR`. Com `plana`, a planificada.
pub fn funil(plana: bool) -> Document {
    let mut d = Document::new("Funil");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: plana, ..Default::default() });
    let quadrado = |d: &mut Document, plano: PlaneRef, meio: f64, base: u64| {
        let mut data = SketchData::default();
        let mut n = base;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, [-meio, -meio], [meio, meio], false);
        d.add_feature(FeatureKind::Sketch(Sketch { plane: plano, data }))
    };
    let a = quadrado(&mut d, PlaneRef::Origin(OriginPlane::XY), 20., 9_300_000);
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30");
    let b = quadrado(&mut d, PlaneRef::Datum(topo), 10., 9_310_000);
    let id = d.add_feature(FeatureKind::SmLoftFlange(SmLoftFlange { a, b, ruled: true, flip: false }));
    d.feature_mut(id).map(|f| f.name = "Funil".into());
    d
}

/// B86: a peça de exemplo (parede, aba e bainha) com uma veneziana, em aço carbono, já planificada:
/// a veneziana vira marca de ferramenta e a janela mostra as propriedades do blank.
pub fn planificada(d: &mut Document) {
    d.add_feature(FeatureKind::SmLouver(SmLouver { a: [60., 15., 2.], b: [60., 45., 2.], width: s("12"), height: s("5"), flip: false, down: false }));
    if let Some(sp) = d.sheet.as_mut() {
        sp.material = s("Aço carbono (SAE 1010/1020)");
        sp.flat = true;
    }
}
