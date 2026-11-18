// SPDX-License-Identifier: LGPL-2.1-or-later
//! B513: tangência entre faces de sólidos. Cada caminho onde a tangência aparece é montado em código,
//! recalculado e conferido por status, volume e caixa envolvente; onde o resultado deve ser liso,
//! confere que a aresta entre as faces é G1 (a malha do kernel marca a aresta como suave).

use super::*;
use std::f64::consts::PI;

fn s(x: &str) -> String {
    x.to_string()
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.)
}

/// A caixa do kernel envolve os polos das superfícies curvas e passa um pouco da peça: nas direções
/// curvas a conferência aceita essa folga (nunca menos que a peça).
fn caixa_ok(v: f64, esperado: f64, lado: f64) -> bool {
    let d = (v - esperado) * lado;
    d >= -1e-3 && d <= 2.5
}

fn alocador(base: u64) -> impl FnMut() -> Id {
    let mut n = base;
    move || {
        n += 1;
        Id(n)
    }
}

fn ext(sketch: Id, d: &str) -> FeatureKind {
    FeatureKind::Extrude(Extrude { sketch, op: BoolOp::Add, extent: Extent::Blind(d.into()), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() })
}

fn edge_op(v: &str, mids: Vec<[f64; 3]>, prop: EdgeProp) -> EdgeOp {
    EdgeOp { value: s(v), edges: EdgeSel::Edges(mids), mode: EdgeMode::Constant, prop, keys: vec![], extra: Default::default() }
}

/// Recalcula e exige status ok (ou aviso) em todos os comandos; devolve (volume, caixa, forma).
fn calcula(d: &Document) -> (f64, ([f64; 3], [f64; 3]), Shape) {
    let r = recompute(d);
    for (id, st) in &r.status {
        assert!(matches!(st, Status::Ok | Status::Warning(_)), "{} em erro: {st:?}", d.name_of(*id));
    }
    let sh = final_shape(d).expect("corpo");
    assert!(sh.is_valid(), "sólido inválido");
    (r.bodies.iter().map(|b| b.mass.volume).sum(), sh.bbox().expect("caixa"), sh)
}

/// Arestas vivas (não G1) entre duas faces, pela marcação de suavidade da malha do kernel.
fn arestas_vivas(sh: &Shape) -> Vec<[f64; 3]> {
    let m = sh.mesh_angle(0.05, 0.2).expect("malha");
    (0..m.edge_ranges.len())
        .filter(|e| m.edge_faces[*e][0] != fenkra_kernel::NO_FACE && m.edge_faces[*e][1] != fenkra_kernel::NO_FACE && m.edge_smooth[*e] == 0)
        .map(|e| [m.edge_mids[e * 3] as f64, m.edge_mids[e * 3 + 1] as f64, m.edge_mids[e * 3 + 2] as f64])
        .collect()
}

/// Rasgo (oblongo) de centros (0,0) e (40,0), raio 10, feito pela ferramenta de rasgo (com as quatro
/// tangências linha–arco), extrudado 10.
fn rasgo(d: &mut Document) -> Id {
    let mut data = SketchData::default();
    let mut al = alocador(5_130_000);
    data.add_slot(&mut al, [0., 0.], [40., 0.], 10.);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(ext(sk, "10"))
}

const V_RASGO: f64 = 10. * (PI * 100. + 20. * 40.);

// ---------------------------------------------------------------- (a) filete e chanfro por tangência

#[test]
fn a_filete_tangente_percorre_o_rasgo_inteiro() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("a1");
    rasgo(&mut d);
    d.add_feature(FeatureKind::Fillet(edge_op("2", vec![[20., -10., 10.]], EdgeProp::Tangente)));
    let (v, (lo, hi), sh) = calcula(&d);
    // canto removido r²(1 − π/4), centroide a r(10 − 3π)/(12 − 3π) da quina, percorrido no contorno
    let (r, c) = (2., 2. * (10. - 3. * PI) / (12. - 3. * PI));
    let esperado = V_RASGO - r * r * (1. - PI / 4.) * (80. + 2. * PI * (10. - c));
    assert!(perto(v, esperado, 1e-3), "volume {v} × {esperado}");
    assert!(caixa_ok(lo[0], -10., -1.) && caixa_ok(hi[0], 50., 1.) && perto(hi[2], 10., 1e-6), "caixa {lo:?} {hi:?}");
    // só o contorno de baixo (4 arestas) continua vivo
    let vivas = arestas_vivas(&sh);
    assert!(vivas.iter().all(|p| p[2].abs() < 1e-6), "quina viva fora da base: {vivas:?}");
}

#[test]
fn a_chanfro_tangente_percorre_o_rasgo_inteiro() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("a2");
    rasgo(&mut d);
    d.add_feature(FeatureKind::Chamfer(edge_op("2", vec![[20., -10., 10.]], EdgeProp::Tangente)));
    let (v, _, _) = calcula(&d);
    let esperado = V_RASGO - 2. * (80. + 2. * PI * (10. - 2. / 3.));
    assert!(perto(v, esperado, 1e-3), "volume {v} × {esperado}");
}

#[test]
fn a_filete_minimo_no_rasgo_fica_na_aresta() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("a3");
    rasgo(&mut d);
    d.add_feature(FeatureKind::Fillet(edge_op("2", vec![[20., -10., 10.]], EdgeProp::Minima)));
    let (v, _, _) = calcula(&d);
    let reta = 40. * 4. * (1. - PI / 4.);
    assert!(v < V_RASGO - 0.9 * reta && v > V_RASGO - 1.5 * reta, "volume {v}: devia tirar só o trecho reto (~{reta})");
}

// ---------------------------------------------------------------- (c) tangência arco–arco no sketch

/// Perfil com dois arcos tangentes que curvam para o mesmo lado (tangência interna):
/// L1 (0,0)→(40,0), A1 centro (40,20) R20 até (60,20), A2 centro (50,20) R10 até (50,30),
/// L2 (50,30)→(0,30), L3 fecha. Tangências L1–A1, A1–A2 e A2–L2.
fn perfil_arcos_mesmo_lado() -> SketchData {
    perfil_arcos_mesmo_lado_desde(0.)
}

/// O mesmo perfil, com as duas retas começando em `x0` (perfil comprido e arcos pequenos perto dele).
fn perfil_arcos_mesmo_lado_desde(x0: f64) -> SketchData {
    let mut sk = SketchData::default();
    let mut al = alocador(5_131_000);
    let v: Vec<Id> = [[x0, 0.], [40., 0.], [40., 20.], [60., 20.], [50., 20.], [50., 30.], [x0, 30.]].iter().map(|q| sk.add_point(&mut al, q[0], q[1])).collect();
    let (p0, p1, c1, p2, c2, p3, p4) = (v[0], v[1], v[2], v[3], v[4], v[5], v[6]);
    let l1 = sk.add_curve(&mut al, Curve::Line { a: p0, b: p1 }, false);
    let a1 = sk.add_curve(&mut al, Curve::Arc { c: c1, s: p1, e: p2 }, false);
    let a2 = sk.add_curve(&mut al, Curve::Arc { c: c2, s: p2, e: p3 }, false);
    let l2 = sk.add_curve(&mut al, Curve::Line { a: p3, b: p4 }, false);
    let l3 = sk.add_curve(&mut al, Curve::Line { a: p4, b: p0 }, false);
    sk.add_constraint(&mut al, Kind::Fixed(p0));
    sk.add_constraint(&mut al, Kind::Horizontal(l1));
    sk.add_constraint(&mut al, Kind::Horizontal(l2));
    sk.add_constraint(&mut al, Kind::Vertical(l3));
    sk.add_constraint(&mut al, Kind::Tangent(l1, a1));
    sk.add_constraint(&mut al, Kind::Tangent(a1, a2));
    sk.add_constraint(&mut al, Kind::Tangent(a2, l2));
    sk
}

/// Área do perfil acima: polígono dos extremos mais os dois segmentos circulares de 90°.
const A_ARCOS: f64 = 1550. + 200. * (PI / 2. - 1.) + 50. * (PI / 2. - 1.);

#[test]
fn c_arcos_tangentes_do_mesmo_lado_viram_extrusao_lisa() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("c1");
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: perfil_arcos_mesmo_lado() }));
    d.add_feature(ext(sk, "10"));
    let (v, (lo, hi), sh) = calcula(&d);
    assert!(perto(v, 10. * A_ARCOS, 1e-4), "volume {v} × {}: o solver deformou o perfil", 10. * A_ARCOS);
    assert!(caixa_ok(hi[0], 60., 1.) && perto(hi[1], 30., 1e-6) && perto(lo[1], 0., 1e-6), "caixa {lo:?} {hi:?}");
    // as faces laterais só têm quina nos dois cantos retos (x = 0)
    let vivas: Vec<_> = arestas_vivas(&sh).into_iter().filter(|p| perto(p[2], 5., 1e-6)).collect();
    assert!(vivas.iter().all(|p| p[0].abs() < 1e-6), "quina viva entre faces que deviam ser tangentes: {vivas:?}");
}

#[test]
fn c_filete_segue_a_cadeia_de_arcos_tangentes() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("c2");
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: perfil_arcos_mesmo_lado() }));
    d.add_feature(ext(sk, "10"));
    d.add_feature(FeatureKind::Fillet(edge_op("1", vec![[20., 0., 10.]], EdgeProp::Tangente)));
    let (v, _, sh) = calcula(&d);
    assert!(v < 10. * A_ARCOS - 1., "o filete não tirou material: {v}");
    // no topo só ficam vivas as arestas do trecho que não é tangente (L3 e as pontas em x = 0)
    let vivas: Vec<_> = arestas_vivas(&sh).into_iter().filter(|p| p[2] > 5.).collect();
    assert!(vivas.iter().all(|p| p[0] < 1.5), "o filete parou no meio da cadeia tangente: {vivas:?}");
}

/// Regra "tangentes" de arestas (B95) na cadeia reta–arco–arco–reta de uma peça comprida: os arcos
/// pequenos perto do tamanho da peça viram poucas cordas na malha, e a direção da corda não é a tangente.
#[test]
fn c_regra_tangentes_de_arestas_atravessa_arcos_do_mesmo_lado() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("c4");
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: perfil_arcos_mesmo_lado_desde(-600.) }));
    d.add_feature(ext(sk, "10"));
    let (_, _, sh) = calcula(&d);
    let regra = vec![RegraAresta { semente: 0, regra: RegraSelecao::Tangentes }];
    let cadeia = selecao_regras::expandir_arestas(&sh, &[[-280., 0., 10.]], &regra).expect("regra");
    // L1, A1, A2 e L2 do topo
    assert_eq!(cadeia.len(), 4, "cadeia tangente do topo: {cadeia:?}");
}

/// Curva em S (arcos tangentes de lados opostos): a tangência externa já funcionava e não pode quebrar.
/// L1 (0,0)→(20,0); A1 centro (20,10) R10 até (30,10); A2 centro (40,10) R10, côncavo, até (40,20)
/// chegando no sentido +x; L2 (40,20)→(60,20); fecha por (60,40) e (0,40).
#[test]
fn c_arcos_em_s_continuam_tangentes() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut sk = SketchData::default();
    let mut al = alocador(5_132_000);
    let pts: Vec<Id> = [[0., 0.], [20., 0.], [20., 10.], [30., 10.], [40., 10.], [40., 20.], [60., 20.], [60., 40.], [0., 40.]].iter().map(|q| sk.add_point(&mut al, q[0], q[1])).collect();
    let l1 = sk.add_curve(&mut al, Curve::Line { a: pts[0], b: pts[1] }, false);
    let a1 = sk.add_curve(&mut al, Curve::Arc { c: pts[2], s: pts[1], e: pts[3] }, false);
    // o arco do sketch é anti-horário: de (40,20) a (30,10)
    let a2 = sk.add_curve(&mut al, Curve::Arc { c: pts[4], s: pts[5], e: pts[3] }, false);
    let l2 = sk.add_curve(&mut al, Curve::Line { a: pts[5], b: pts[6] }, false);
    let l3 = sk.add_curve(&mut al, Curve::Line { a: pts[6], b: pts[7] }, false);
    let l4 = sk.add_curve(&mut al, Curve::Line { a: pts[7], b: pts[8] }, false);
    let l5 = sk.add_curve(&mut al, Curve::Line { a: pts[8], b: pts[0] }, false);
    sk.add_constraint(&mut al, Kind::Fixed(pts[0]));
    for h in [l1, l2, l4] {
        sk.add_constraint(&mut al, Kind::Horizontal(h));
    }
    for v in [l3, l5] {
        sk.add_constraint(&mut al, Kind::Vertical(v));
    }
    sk.add_constraint(&mut al, Kind::Tangent(l1, a1));
    sk.add_constraint(&mut al, Kind::Tangent(a1, a2));
    sk.add_constraint(&mut al, Kind::Tangent(a2, l2));
    let mut d = Document::new("c3");
    let skf = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: sk }));
    d.add_feature(ext(skf, "10"));
    let (v, (lo, hi), sh) = calcula(&d);
    // polígono dos extremos = 1800; o segmento de A1 soma e o de A2 (côncavo), igual, subtrai
    assert!(perto(v, 18000., 1e-4), "volume {v} × 18000");
    assert!(perto(hi[0], 60., 1e-6) && perto(hi[1], 40., 1e-6) && perto(lo[1], 0., 1e-6), "caixa {lo:?} {hi:?}");
    let vivas: Vec<_> = arestas_vivas(&sh).into_iter().filter(|p| perto(p[2], 5., 1e-6)).collect();
    assert!(vivas.iter().all(|p| p[0].abs() < 1e-6 || perto(p[0], 60., 1e-6)), "quina viva entre faces tangentes: {vivas:?}");
}

// ---------------------------------------------------------------- (b) faces tangentes: espessura, casca, saída

fn regra_tangentes() -> Vec<RegraAresta> {
    vec![RegraAresta { semente: 0, regra: RegraSelecao::Tangentes }]
}

#[test]
fn b_espessura_com_regra_tangentes_pega_o_contorno_todo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b1");
    rasgo(&mut d);
    d.add_feature(FeatureKind::Thickness(Thickness { faces: vec![[20., -10., 5.]], value: s("2"), face_keys: vec![], remove: false, regras: regra_tangentes() }));
    let (v, (lo, hi), _) = calcula(&d);
    let esperado = 10. * (PI * 144. + 24. * 40.);
    assert!(perto(v, esperado, 1e-4), "volume {v} × {esperado}");
    assert!(caixa_ok(lo[0], -12., -1.) && caixa_ok(hi[0], 52., 1.) && perto(lo[1], -12., 1e-6), "caixa {lo:?} {hi:?}");
}

#[test]
fn b_casca_do_rasgo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b2");
    rasgo(&mut d);
    d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[20., 0., 10.]], thickness: s("1"), outward: false, face_keys: vec![], closed: false, regras: vec![] }));
    let (v, _, _) = calcula(&d);
    let esperado = V_RASGO - 9. * (PI * 81. + 18. * 40.);
    assert!(perto(v, esperado, 1e-4), "volume {v} × {esperado}");
}

#[test]
fn b_saida_com_regra_tangentes_inclina_o_contorno_todo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b3");
    rasgo(&mut d);
    d.add_feature(FeatureKind::Draft(Draft { faces: vec![[20., -10., 5.]], neutral: PlaneRef::Origin(OriginPlane::XY), angle: s("3"), reverse: false, face_keys: vec![], regras: regra_tangentes() }));
    let (v, (lo, hi), sh) = calcula(&d);
    // contorno todo inclinado de 3° a partir da base: a seção encolhe (ou cresce) por h·tg 3° em cada lado
    let t = 3f64.to_radians().tan();
    let area = |h: f64| {
        let r = 10. - h * t;
        PI * r * r + 2. * r * 40.
    };
    let area2 = |h: f64| {
        let r = 10. + h * t;
        PI * r * r + 2. * r * 40.
    };
    // integral exata (área quadrática em h) pela regra de Simpson
    let simpson = |f: &dyn Fn(f64) -> f64| 10. / 6. * (f(0.) + 4. * f(5.) + f(10.));
    let (vm, vp) = (simpson(&area), simpson(&area2));
    assert!(perto(v, vm, 1e-4) || perto(v, vp, 1e-4), "volume {v}: esperado {vm} ou {vp}");
    assert!(perto(hi[2], 10., 1e-6) && lo[0] < -9., "caixa {lo:?} {hi:?}");
    // a inclinação mantém as faces tangentes entre si
    let vivas: Vec<_> = arestas_vivas(&sh).into_iter().filter(|p| p[2] > 0.5 && p[2] < 9.5).collect();
    assert!(vivas.is_empty(), "a saída abriu quina entre faces tangentes: {vivas:?}");
}

#[test]
fn b_saida_sem_regra_numa_face_do_rasgo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // escolhida só a face plana: o kernel inclina junto as faces tangentes ou explica por que não
    let mut d = Document::new("b4");
    rasgo(&mut d);
    let id = d.add_feature(FeatureKind::Draft(Draft { faces: vec![[20., -10., 5.]], neutral: PlaneRef::Origin(OriginPlane::XY), angle: s("3"), reverse: false, face_keys: vec![], regras: vec![] }));
    let r = recompute(&d);
    match &r.status[&id] {
        Status::Ok | Status::Warning(_) => {
            let sh = final_shape(&d).expect("corpo");
            assert!(sh.is_valid(), "sólido inválido");
            let vivas: Vec<_> = arestas_vivas(&sh).into_iter().filter(|p| p[2] > 0.5 && p[2] < 9.5).collect();
            assert!(vivas.is_empty(), "a saída abriu quina entre faces tangentes: {vivas:?}");
        }
        Status::Error(e) => panic!("saída numa face tangente falhou: {e}"),
        outro => panic!("{outro:?}"),
    }
}

#[test]
fn b_faces_tangentes_do_rasgo_sao_as_quatro_laterais() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b5");
    rasgo(&mut d);
    let (_, _, sh) = calcula(&d);
    let faces = selecao_regras::expandir_faces(&sh, &[[20., -10., 5.]], &regra_tangentes()).expect("regra");
    assert_eq!(faces.len(), 4, "faces tangentes: {faces:?}");
}

// ---------------------------------------------------------------- (d) extrusão até face curva tangente

/// Cilindro R10 no eixo X (x de 0 a 20); perfil 20 × `largura` num plano a z = −30, extrudado até a
/// face de baixo do cilindro. Com largura 20 as paredes da extrusão ficam tangentes ao cilindro.
fn ate_cilindro(largura: f64) -> Document {
    let mut d = Document::new("d");
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Origin(OriginPlane::YZ), a: s("10"), b: s("20"), c: s("0"), d: s("0"), op: BoolOp::Add }));
    let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "-30");
    let mut data = SketchData::default();
    let mut al = alocador(5_133_000);
    data.add_rect(&mut al, [0., -largura / 2.], [20., largura / 2.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(pl), data }));
    d.add_feature(FeatureKind::Extrude(Extrude {
        sketch: sk,
        op: BoolOp::Add,
        extent: Extent::UpToSurface(SurfRef::Face { point: [10., 0., -10.], normal: [0., 0., -1.], owner: None, key: None }),
        reverse: false,
        taper: s("0"),
        thin: None,
        dir: None,
        thin_side: 0,
        ends: Default::default(),
    }));
    d
}

/// Volume da extrusão até a face de baixo do cilindro (largura w ≤ 20) somado ao cilindro.
fn v_ate_cilindro(w: f64) -> f64 {
    let a = w / 2.;
    // ∫ (30 − √(100 − y²)) dy de −a a a
    let int_raiz = a * (100. - a * a).sqrt() + 100. * (a / 10.).asin();
    20. * (30. * w - int_raiz) + PI * 100. * 20.
}

#[test]
fn d_extrusao_ate_cilindro_secante() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (v, (lo, hi), _) = calcula(&ate_cilindro(10.));
    assert!(perto(v, v_ate_cilindro(10.), 1e-4), "volume {v} × {}", v_ate_cilindro(10.));
    assert!(perto(lo[2], -30., 1e-6) && caixa_ok(hi[2], 10., 1.), "caixa {lo:?} {hi:?}");
}

#[test]
fn d_extrusao_ate_cilindro_tangente() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (v, (lo, hi), _) = calcula(&ate_cilindro(20.));
    assert!(perto(v, v_ate_cilindro(20.), 1e-4), "volume {v} × {}", v_ate_cilindro(20.));
    assert!(caixa_ok(lo[1], -10., -1.) && caixa_ok(hi[1], 10., 1.) && perto(lo[2], -30., 1e-6), "caixa {lo:?} {hi:?}");
}

// ---------------------------------------------------------------- (e) loft com ponta normal (tangente)

#[test]
fn e_loft_normal_sai_tangente_ao_bloco() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("e1");
    let q = |d: &mut Document, plane: PlaneRef, a: [f64; 2], b: [f64; 2], base: u64| {
        let mut data = SketchData::default();
        let mut al = alocador(base);
        data.add_rect(&mut al, a, b, false);
        d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
    };
    let base = q(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -20.], [20., 20.], 5_134_000);
    d.add_feature(ext(base, "20"));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
    let alto = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "60");
    let s1 = q(&mut d, PlaneRef::Datum(topo), [-20., -20.], [20., 20.], 5_135_000);
    let s2 = q(&mut d, PlaneRef::Datum(alto), [-10., -10.], [10., 10.], 5_136_000);
    d.add_feature(FeatureKind::Loft(Loft { sections: vec![s1, s2], ruled: false, closed: false, op: BoolOp::Add, torcao: vec![], inicio: PontaLoft { condicao: CondicaoPonta::Normal, intensidade: s("1") }, fim: Default::default() }));
    let (v, (lo, hi), sh) = calcula(&d);
    let bloco = 40. * 40. * 20.;
    // tronco de pirâmide reto = 40/3·(1600 + 400 + 800); com a saída normal o loft estufa e fica maior
    let tronco = 40. / 3. * (1600. + 400. + 800.);
    assert!(v > bloco + tronco && v < bloco + 1600. * 40., "volume {v}");
    assert!(perto(hi[2], 60., 1e-6) && caixa_ok(hi[0], 20., 1.) && caixa_ok(lo[0], -20., -1.), "caixa {lo:?} {hi:?}");
    // a junção entre o bloco e o loft (z = 20) é lisa
    let vivas: Vec<_> = arestas_vivas(&sh).into_iter().filter(|p| perto(p[2], 20., 1e-3)).collect();
    assert!(vivas.is_empty(), "quina na junção do loft normal com o bloco: {vivas:?}");
}

// ---------------------------------------------------------------- (f) plano tangente a cilindro

#[test]
fn f_plano_tangente_ao_cilindro_no_ponto_escolhido() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("f1");
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Origin(OriginPlane::XY), a: s("10"), b: s("20"), c: s("0"), d: s("0"), op: BoolOp::Add }));
    // face lateral clicada em x = +10 (arquivo sem chave); o ponto do plano fica em y = +10
    let pl = d.add_geometry(RefKind::PlaneTangentSurface {
        surface: SurfRef::Face { point: [10., 0., 10.], normal: [1., 0., 0.], owner: None, key: None },
        point: PointRef::Fixed([0., 10., 15.]),
    });
    let r = recompute(&d);
    assert!(matches!(r.status.get(&pl), None | Some(Status::Ok | Status::Warning(_))), "{:?}", r.status.get(&pl));
    let p = r.planes.iter().find(|p| p.id == Some(pl)).expect("plano");
    let (o, n) = (p.frame.origin, p.frame.normal);
    assert!(geo3d::dist(o, [0., 10., 15.]) < 1e-6, "origem {o:?}");
    assert!(n[1].abs() > 1. - 1e-9, "normal {n:?}: devia ser radial (±Y) no ponto escolhido");
}

/// A lateral sai na normal da seção (sem estufar para fora nem inclinar) em qualquer intensidade.
#[test]
fn e_loft_normal_sai_na_normal_em_toda_intensidade() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let q = |d: &mut Document, plane: PlaneRef, a: [f64; 2], b: [f64; 2], base: u64| {
        let mut data = SketchData::default();
        let mut al = alocador(base);
        data.add_rect(&mut al, a, b, false);
        d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
    };
    let mut anterior = 0.;
    for f in ["0.5", "1", "2", "3"] {
        let mut d = Document::new("e2");
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let alto = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "60");
        let s1 = q(&mut d, PlaneRef::Datum(topo), [-20., -20.], [20., 20.], 5_137_000);
        let s2 = q(&mut d, PlaneRef::Datum(alto), [-10., -10.], [10., 10.], 5_138_000);
        d.add_feature(FeatureKind::Loft(Loft { sections: vec![s1, s2], ruled: false, closed: false, op: BoolOp::Add, torcao: vec![], inicio: PontaLoft { condicao: CondicaoPonta::Normal, intensidade: s(f) }, fim: Default::default() }));
        let (v, _, sh) = calcula(&d);
        // mais intensidade enche mais o começo, sem passar do prisma 40 × 40 × 40
        assert!(v > anterior && v < 1600. * 40., "intensidade {f}: volume {v} (anterior {anterior})");
        anterior = v;
        let (p, n) = sh.surface_near([25., 0., 20.01]).expect("ponto");
        assert!(p[0] <= 20. + 1e-6, "intensidade {f}: a lateral estufou para fora da seção: {p:?}");
        let ang = n[2].atan2(n[0]).to_degrees().abs();
        assert!(ang < 0.1, "intensidade {f}: a lateral sai {ang:.2}° fora da normal");
    }
}
