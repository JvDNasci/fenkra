// SPDX-License-Identifier: LGPL-2.1-or-later
//! B77: toda dobra entra no mapa de dobras, reta ou curva. B03: dobra por linha em curva. B78: a
//! cadeia da aba curva nasce do B-rep pela tangência, a polilinha gravada é só cache.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{chapa, planificada, recompute, Promoted, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn chapa_base() -> (Document, Id) {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    (d, sk)
}

fn recalcula(d: &Document, flat: bool) -> RecomputeResult {
    let mut d = d.clone();
    d.sheet.as_mut().unwrap().flat = flat;
    recompute(&d)
}

fn sem_erro(r: &RecomputeResult) {
    assert!(!r.status.values().any(|x| matches!(x, Status::Error(_))), "{:?}", r.status);
}

fn volume(r: &RecomputeResult) -> f64 {
    r.bodies.iter().map(|b| b.mass.volume).sum()
}

fn sk_spline(d: &mut Document, pts: &[[f64; 2]]) -> Id {
    let mut data = SketchData::default();
    let mut n = 8_000_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let ids: Vec<Id> = pts.iter().map(|p| data.add_point(&mut al, p[0], p[1])).collect();
    data.add_curve(&mut al, sketch::Curve::Spline { pts: ids }, false);
    d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }))
}

fn dobra(sketch: Option<Id>) -> SmBend {
    SmBend { sketch, a: [0.; 3], b: [0.; 3], angle: s("90"), radius: None, flip: false, down: false, position: 0 }
}

#[test]
fn aba_curva_e_joggle_entram_no_mapa_e_no_dxf() {
    let (mut d, _) = chapa_base();
    let edge = vec![[100., 0., 2.], [100., 25., 2.], [100., 50., 2.]];
    let aba = d.add_feature(FeatureKind::SmCurveFlange(SmCurveFlange { edge, length: s("10"), flip: false, angle: s("90"), radius: None, side: 0, edge_key: None }));
    let jog = d.add_feature(FeatureKind::SmJoggle(SmJoggle { a: [40., 0., 2.], b: [40., 50., 2.], depth: s("3"), run: s("5"), flip: true, angle: None, plane: None, keep_length: false }));
    for flat in [false, true] {
        let r = recalcula(&d, flat);
        sem_erro(&r);
        let de = |id: Id| r.bend_lines.iter().filter(|b| b.feature == id).count();
        assert_eq!((de(aba), de(jog)), (1, 2), "planificada = {flat}: {:?}", r.bend_lines);
        let b = r.bend_lines.iter().find(|b| b.feature == aba).unwrap();
        // a compensação é a mesma conta das outras dobras
        let c = chapa::compensacao(2., 2., 90., 0.44);
        assert!((b.allowance - c.ba).abs() < 1e-9 && (b.deduction - c.deducao).abs() < 1e-9);
        assert_eq!(b.radius, 2.);
        let j = r.bend_lines.iter().find(|b| b.feature == jog).unwrap();
        assert!((j.angle - 3f64.atan2(5.).to_degrees()).abs() < 1e-6, "{}", j.angle);
    }
    // a planificada do joggle é plana: a caixa tem só a espessura em Z
    let plana = recalcula(&d, true);
    let (lo, hi) = caixa_da_malha(&plana);
    assert!((hi[2] - lo[2] - 2.).abs() < 0.05, "{lo:?} {hi:?}");
    let p = planificada::planificar(&d).unwrap();
    assert_eq!(p.dobras.len(), 3);
    let texto = planificada::dxf(&p);
    assert!(texto.matches("LINE").count() > 3);
}

#[test]
fn dobra_em_spline_levanta_parede_e_guarda_a_linha_curva() {
    let (mut d, _) = chapa_base();
    let v0 = volume(&recalcula(&d, false));
    let sk = sk_spline(&mut d, &[[50., 0.], [62., 25.], [50., 50.]]);
    let id = d.add_feature(FeatureKind::SmBend(dobra(Some(sk))));
    let r = recalcula(&d, false);
    assert!(matches!(&r.status[&id], Status::Warning(m) if m.contains("curva")), "{:?}", r.status[&id]);
    // o sketch está na face de baixo: a parede sai para o lado dessa face, como na dobra reta
    let (lo, _) = caixa_da_malha(&r);
    assert!(lo[2] < -20., "a parede sai da chapa: {lo:?}");
    let v = volume(&r);
    assert!((v - v0).abs() < 0.05 * v0, "desenvolvido preservado: {v} perto de {v0}");
    let linha = r.bend_lines.iter().find(|b| b.feature == id).unwrap();
    assert!(linha.curva.len() > 4, "{:?}", linha.curva);
    // a planificada fica plana, com a mesma linha curva, que chega ao DXF em vários trechos
    let plana = recalcula(&d, true);
    sem_erro(&plana);
    let (lo, hi) = caixa_da_malha(&plana);
    assert!((hi[2] - lo[2] - 2.).abs() < 0.05);
    assert!(plana.bend_lines.iter().any(|b| b.feature == id && b.curva.len() > 4));
    let p = planificada::planificar(&d).unwrap();
    assert!(p.dobras[0].curva.len() > 4);
}

#[test]
fn dobra_em_curva_que_nao_divide_a_chapa_recusa() {
    let (mut d, _) = chapa_base();
    let sk = sk_spline(&mut d, &[[200., 0.], [212., 25.], [200., 50.]]);
    let id = d.add_feature(FeatureKind::SmBend(dobra(Some(sk))));
    let r = recalcula(&d, false);
    assert!(matches!(&r.status[&id], Status::Error(_)), "{:?}", r.status[&id]);
}

#[test]
fn cadeia_da_aba_curva_nasce_do_brep_quando_a_chapa_muda() {
    let (mut d, sk) = chapa_base();
    let edge = vec![[100., 0., 2.], [100., 25., 2.], [100., 50., 2.]];
    let aba = d.add_feature(FeatureKind::SmCurveFlange(SmCurveFlange { edge, length: s("10"), flip: false, angle: s("90"), radius: None, side: 0, edge_key: None }));
    let r = recompute(&d);
    let Some(Promoted::Edges(k)) = r.promoted.get(&aba) else { panic!("sem chave: {:?}", r.status[&aba]) };
    let key = k[0].clone();
    let FeatureKind::SmCurveFlange(a) = &mut d.feature_mut(aba).unwrap().kind else { panic!() };
    a.edge_key = key;
    let v50 = volume(&recompute(&d)) - 100. * 50. * 2.;
    // a chapa fica mais larga: a aba cresce com a aresta, a polilinha gravada não manda
    set_rect(&mut d, sk, [0., 0.], [100., 80.]);
    let r = recompute(&d);
    sem_erro(&r);
    let v80 = volume(&r) - 100. * 80. * 2.;
    assert!((v80 / v50 - 80. / 50.).abs() < 0.03, "aba {v80} ≈ {} (50 mm dava {v50})", v50 * 1.6);
}
