// SPDX-License-Identifier: LGPL-2.1-or-later
//! B540: com o comando aberto (cálculo leve), a aba curva e a aba por superfície mostram a peça de
//! verdade, seguindo o arco e o suporte, e não a caixa de um cubo.

use super::*;

fn s(x: &str) -> String {
    x.to_string()
}

fn parametros() -> Option<SheetParams> {
    Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() })
}

/// Quarto de disco R50 com a aba curva de 10 mm na borda do arco.
fn doc_arco() -> (Document, Id) {
    let mut d = Document::new("arco");
    d.sheet = parametros();
    let mut data = SketchData::default();
    let mut n = 6_700_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let o = data.add_point(&mut al, 0., 0.);
    let a = data.add_point(&mut al, 50., 0.);
    let b = data.add_point(&mut al, 0., 50.);
    data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: o, b: a }, false);
    data.add_curve(&mut al, fenkra_core::sketch::Curve::Arc { c: o, s: a, e: b }, false);
    data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: b, b: o }, false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let edge: Vec<[f64; 3]> = (0..=24)
        .map(|i| {
            let t = i as f64 / 24. * std::f64::consts::FRAC_PI_2;
            [50. * t.cos(), 50. * t.sin(), 2.]
        })
        .collect();
    let id = d.add_feature(FeatureKind::SmCurveFlange(SmCurveFlange { edge, length: s("10"), flip: false, angle: s("90"), radius: None, side: 0, edge_key: None, so_aresta: false, reta: None }));
    (d, id)
}

fn leve(d: &Document, id: Id) -> RecomputeResult {
    let mut cache = Cache::default();
    cache.leve = Some(id);
    recompute_cached(d, &mut cache)
}

fn caixa(r: &RecomputeResult) -> ([f32; 3], [f32; 3]) {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in r.bodies[0].mesh.positions.chunks_exact(3) {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    (lo, hi)
}

#[test]
fn aba_curva_aberta_segue_o_arco() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, id) = doc_arco();
    assert!(!crate::previa_leve::cobre_no(&d, d.feature(id).unwrap()), "a aba curva não troca a peça pela caixa");
    let r = leve(&d, id);
    assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{:?}", r.status[&id]);
    assert!(r.previas.iter().all(|p| p.feature != id), "sem caixa de prévia leve: {:?}", r.previas);
    // a prévia é o mesmo corpo do cálculo final: quarto de disco mais a aba de 10 mm no arco
    let (lo, hi) = caixa(&r);
    assert!((hi[0] - 54.).abs() < 0.3 && (hi[1] - 54.).abs() < 0.3, "segue o arco: {lo:?} {hi:?}");
    assert!((hi[2] - 14.).abs() < 0.2, "altura da aba {hi:?}");
    // o cubo antigo era a caixa dos pontos inflada pelo comprimento em todos os lados: passava de 60
    assert!(lo[0] > -0.1 && lo[1] > -0.1, "a prévia não sai da peça para o lado oposto ao arco: {lo:?}");
    let base = std::f64::consts::PI * 2500. / 4. * 2.;
    assert!(r.bodies[0].mass.volume > base + 900., "volume {}", r.bodies[0].mass.volume);
    let final_ = recompute(&d);
    assert!((final_.bodies[0].mass.volume - r.bodies[0].mass.volume).abs() < 1e-6);
}

#[test]
fn aba_por_superficie_aberta_nao_e_a_caixa_de_um_cubo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("web");
    d.sheet = parametros();
    let mut data = SketchData::default();
    let mut n = 6_800_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [0., 0.], [100., 50.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::YZ), "80");
    let id = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Plane(PlaneRef::Datum(pl)), face: [40., 25., 2.], length: s("15"), radius: None, flip: false, keep_base: false, mode: 0, angle: s("90") }));
    assert!(!crate::previa_leve::cobre_no(&d, d.feature(id).unwrap()));
    let r = leve(&d, id);
    assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{:?}", r.status[&id]);
    assert!(r.previas.iter().all(|p| p.feature != id), "sem caixa de prévia leve: {:?}", r.previas);
    let (lo, hi) = caixa(&r);
    // web relimitada pelo suporte em x = 80, com a aba subindo 15 + dobra: largura 50, não um cubo
    assert!((hi[2] - 19.).abs() < 0.2, "a aba sobe {hi:?}");
    assert!(hi[0] < 85. && hi[1] < 50.1 && lo[1] > -0.1, "caixa da peça {lo:?} {hi:?}");
    let (dx, dy, dz) = ((hi[0] - lo[0]) as f64, (hi[1] - lo[1]) as f64, (hi[2] - lo[2]) as f64);
    assert!((dx - dy).abs() > 10. && (dy - dz).abs() > 10., "não é um cubo: {dx} x {dy} x {dz}");
}
