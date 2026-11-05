//! B01 e B78: abas em sequência (aba sobre a ponta de uma dobra), cadeia reta + curva como uma só
//! feature, abas vizinhas com validação de colisão e lateral igual à espessura.
//!
//! Cada caso monta o documento em código, recalcula dobrado e planificado e confere status, volume e
//! caixa envolvente.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn base() -> Document {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    d
}

fn aba(e: [f64; 3], len: &str, off: &str) -> FeatureKind {
    FeatureKind::SmFlange(SmFlange { edge: e, length: s(len), angle: s("90"), radius: None, off1: s(off), off2: s(off), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true })
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

fn angulo(k: &mut FeatureKind, a: &str) {
    if let FeatureKind::SmFlange(f) = k {
        f.angle = s(a);
    }
}

/// Aba na ponta de uma dobra pura (comprimento 0): a aresta da ponta fica entre a face plana da
/// ponta e a face cilíndrica da dobra. A face cilíndrica vira a referência da nova aba.
#[test]
fn aba_sobre_a_ponta_de_dobra_pura_nas_duas_arestas() {
    if !kernel_disponivel() {
        return;
    }
    // a aresta interna dobra de volta sobre a chapa, a externa para fora
    for (nome, p2, xmax) in [("interna", [102., 25., 4.], 104.), ("externa", [104., 25., 4.], 116.)] {
        let mut d = base();
        d.add_feature(aba([100., 25., 2.], "0", "0"));
        d.add_feature(aba(p2, "10", "0"));
        let dobrada = recalcula(&d, false);
        tudo_ok(&d, &dobrada, nome);
        let (lo, hi) = caixa_da_malha(&dobrada);
        assert!((hi[0] - xmax).abs() < 0.1 && (hi[2] - 8.).abs() < 0.1 && lo[0].abs() < 0.1, "{nome}: caixa {lo:?} {hi:?}");
        let plana = recalcula(&d, true);
        tudo_ok(&d, &plana, nome);
        let (vd, vp) = (volume_total(&dobrada), volume_total(&plana));
        assert!((vd - vp).abs() < 60., "{nome}: dobrada {vd} planificada {vp}");
        // chapa + 2 dobras de 90° (π/2·(2 + 0,44·2)) + perna reta de 10, na largura de 50 e espessura 2
        let esperado = 10000. + 50. * 2. * (2. * std::f64::consts::FRAC_PI_2 * 2.88 + 10.);
        assert!((vp - esperado).abs() < 40., "{nome}: planificada {vp} ≈ {esperado}");
    }
}

/// Reta + arco + reta tangentes escolhidos como uma aba só (propagação por tangência): a varredura
/// segue um fio de arestas exatas, e não uma spline única que oscila na emenda e arruína o corpo.
#[test]
fn aba_em_cadeia_reta_curva_reta_e_uma_feature_so() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("cadeia");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let mut data = SketchData::default();
    let mut n = 6_900_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let a = data.add_point(&mut al, 0., 0.);
    let b = data.add_point(&mut al, 100., 0.);
    let c = data.add_point(&mut al, 100., 30.);
    let o = data.add_point(&mut al, 80., 30.);
    let e = data.add_point(&mut al, 80., 50.);
    let g = data.add_point(&mut al, 0., 50.);
    data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a, b }, false);
    data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: b, b: c }, false);
    data.add_curve(&mut al, fenkra_core::sketch::Curve::Arc { c: o, s: c, e }, false);
    data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: e, b: g }, false);
    data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: g, b: a }, false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    // chapa: 100×50 menos a sobra do canto arredondado de raio 20, espessura 2
    let chapa = (5000. - (400. - std::f64::consts::PI * 100.)) * 2.;
    let mut edge: Vec<[f64; 3]> = vec![[100., 0., 2.], [100., 30., 2.]];
    for i in 1..=12 {
        let t = i as f64 / 12. * std::f64::consts::FRAC_PI_2;
        edge.push([80. + 20. * t.cos(), 30. + 20. * t.sin(), 2.]);
    }
    edge.push([0., 50., 2.]);
    d.add_feature(FeatureKind::SmCurveFlange(SmCurveFlange { edge, length: s("10"), flip: false, angle: s("90"), radius: None, side: 0 }));
    let dobrada = recalcula(&d, false);
    tudo_ok(&d, &dobrada, "dobrada");
    let (vd, vp) = (volume_total(&dobrada), volume_total(&recalcula(&d, true)));
    // comprimento da cadeia: 30 + (π/2)·20 + 80; seção da dobra + perna = π/4·(16 − 4) + 20
    let comp = 30. + std::f64::consts::FRAC_PI_2 * 20. + 80.;
    let aba_esperada = comp * (std::f64::consts::FRAC_PI_4 * 12. + 20.);
    assert!(vd - chapa > 0.9 * aba_esperada && vd - chapa < 1.15 * aba_esperada, "dobrada {vd}: aba {} ≈ {aba_esperada}", vd - chapa);
    assert!(vp > vd && vp < vd + 400., "planificada {vp} perto da dobrada {vd}");
    let (_, hi) = caixa_da_malha(&dobrada);
    assert!((hi[2] - 14.).abs() < 0.2, "altura da aba {}", hi[2]);
}

/// Duas abas inclinadas que se cruzam no canto: o kernel mede o volume comum, a feature falha com a
/// folga que falta, e o recuo sugerido resolve.
#[test]
fn abas_vizinhas_que_se_cruzam_falham_com_a_folga_e_o_recuo_resolve() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = base();
    let mut a1 = aba([100., 25., 2.], "20", "0");
    angulo(&mut a1, "120");
    d.add_feature(a1);
    let mut a2 = aba([25., 50., 2.], "20", "0");
    angulo(&mut a2, "120");
    let id2 = d.add_feature(a2);
    let r = recalcula(&d, false);
    let msg = match &r.status[&id2] {
        Status::Error(m) => m.clone(),
        o => panic!("a aba nova deveria colidir: {o:?}"),
    };
    assert!(msg.contains("colide") && msg.contains("recue a ponta 1 para"), "{msg}");
    // a dica traz o recuo da ponta 1 (junto ao canto da primeira aba)
    let dica: f64 = msg.split("para ").nth(1).and_then(|x| x.split(' ').next()).and_then(|x| x.parse().ok()).expect("dica numérica");
    assert!(dica > 5. && dica < 12., "dica {dica}");
    if let Some(FeatureKind::SmFlange(f)) = d.features.last_mut().map(|f| &mut f.kind) {
        f.off1 = format!("{}", (dica + 0.1).ceil());
    }
    tudo_ok(&d, &recalcula(&d, false), "com recuo");
    tudo_ok(&d, &recalcula(&d, true), "com recuo planificada");
}

/// Abas em arestas vizinhas a 90° só encostam no canto (sem volume comum): continuam valendo.
#[test]
fn abas_vizinhas_a_noventa_graus_encostam_sem_colidir() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = base();
    d.add_feature(aba([100., 25., 2.], "20", "0"));
    d.add_feature(aba([25., 50., 2.], "20", "0"));
    d.add_feature(aba([25., 0., 2.], "20", "0"));
    for flat in [false, true] {
        tudo_ok(&d, &recalcula(&d, flat), "três abas");
    }
}

/// A lateral da aresta precisa ser a espessura da chapa: a quina vertical da chapa (50 mm de lateral)
/// não é aresta de aba.
#[test]
fn aba_em_aresta_de_lateral_errada_diz_a_medida() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = base();
    let id = d.add_feature(aba([0., 0., 1.], "10", "0"));
    let r = recalcula(&d, false);
    match &r.status[&id] {
        Status::Error(m) => assert!(m.contains("lateral") && m.contains("espessura"), "{m}"),
        o => panic!("deveria recusar a aresta: {o:?}"),
    }
}
