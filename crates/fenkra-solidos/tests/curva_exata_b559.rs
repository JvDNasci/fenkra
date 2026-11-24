// SPDX-License-Identifier: LGPL-2.1-or-later
//! B559: a curva chega ao kernel com a geometria exata (reta, arco, círculo, B-spline, aresta do
//! modelo), e a polilinha só serve para desenhar. B576: o elemento extraído continua um Extrair, com o
//! mesmo tipo geométrico, e acompanha a peça quando ela muda.
//!
//! Documentos montados em código, recalculados, conferindo status, área ou volume, caixa envolvente e o
//! tipo das faces e arestas que o kernel recebeu.

mod common;

use common::*;
use fenkra_core::sketch::Curve;
use fenkra_core::spline::Hermite;
use fenkra_core::*;
use fenkra_kernel::{Shape, Trecho3};
use fenkra_solidos::{curva_de_referencia, final_shape, recompute, surface_brep, RecomputeResult, Status};
use std::f64::consts::PI;

fn s(x: &str) -> String {
    x.into()
}

fn ok(r: &RecomputeResult, d: &Document, id: Id) {
    assert!(r.status.get(&id).is_some_and(ok_ou_conformacao), "{}: {:?}", d.name_of(id), r.status.get(&id));
}

fn superficie(d: &Document, id: Id) -> Shape {
    Shape::from_brep(&surface_brep(d, id).expect("superfície avaliada")).expect("B-rep legível")
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Distância do ponto ao polígono (projeção em cada trecho).
fn ate_poligono(p: [f64; 3], v: &[[f64; 3]]) -> f64 {
    v.windows(2)
        .map(|w| {
            let ab = [0, 1, 2].map(|i| w[1][i] - w[0][i]);
            let l2 = ab.iter().map(|x| x * x).sum::<f64>().max(1e-30);
            let t = ([0, 1, 2].map(|i| (p[i] - w[0][i]) * ab[i]).iter().sum::<f64>() / l2).clamp(0., 1.);
            dist(p, [0, 1, 2].map(|i| w[0][i] + ab[i] * t))
        })
        .fold(f64::MAX, f64::min)
}

/// Sketch com uma curva só; devolve (sketch, curva).
fn sketch_com(d: &mut Document, monta: impl FnOnce(&mut SketchData, &mut Box<dyn FnMut() -> Id>) -> Id) -> (Id, Id) {
    let mut data = SketchData::default();
    let mut n = 9_100_000u64;
    let mut al: Box<dyn FnMut() -> Id> = Box::new(move || {
        n += 1;
        Id(n)
    });
    let c = monta(&mut data, &mut al);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    (sk, c)
}

fn extrusao(curve: CurveRef, len: &str) -> RefKind {
    RefKind::SurfExtrude { curve, dir: AxisRef::Origin(OriginAxis::Z), length: s(len), length2: s("0"), until: None, until2: None, mais: vec![] }
}

// ------------------------------------------------------------------ definição única da spline

/// A réplica em Rust (desenho, Aparar, prévia) é a mesma curva que o kernel interpola pelos pontos.
#[test]
fn spline_unica_igual_ao_kernel() {
    if !kernel_disponivel() {
        return;
    }
    let casos: Vec<(Vec<[f64; 3]>, bool)> = vec![
        // espaçamento desigual, com uma volta apertada
        (vec![[0., 0., 0.], [20., 0., 0.], [21., 6., 0.], [40., 6., 0.], [55., -3., 2.], [60., 10., 4.]], false),
        // três pontos: parábola
        (vec![[0., 0., 0.], [10., 8., 0.], [30., 0., 0.]], false),
        // quatro pontos
        (vec![[0., 0., 0.], [10., 8., 0.], [30., 2., 0.], [35., 20., 5.]], false),
        // fechada (periódica)
        ((0..7).map(|i| {
            let a = i as f64 / 7. * 2. * PI;
            [40. * a.cos(), 25. * a.sin() + 3. * (2. * a).sin(), 0.]
        }).collect(), true),
    ];
    for (pts, fechada) in casos {
        let h = Hermite::interpolada(&pts, fechada).unwrap();
        // a curva como o wireframe a manda ao kernel (a fechada vai como B-spline pronta)
        let mut d = Document::new("spline");
        let sp = d.add_geometry(RefKind::Spline { points: pts.iter().map(|p| PointRef::Fixed(*p)).collect(), closed: fechada, tangents: vec![] });
        let c = curva_de_referencia(&d, sp).unwrap();
        let fio = Shape::curve_shape_c(&c.kernel()).expect("curva no kernel");
        let topo = fio.topology().unwrap();
        assert_eq!(topo.edge_mids.len(), 1, "uma aresta só");
        let k = fio.edge_at(topo.edge_mids[0]).unwrap();
        let rust = h.amostra(4000);
        let pior = k.iter().map(|p| ate_poligono(*p, &rust)).fold(0., f64::max);
        assert!(pior < 1e-5, "{} pontos (fechada {fechada}): a réplica se afasta {pior} mm da curva do kernel", pts.len());
    }
}

// ------------------------------------------------------------------ superfícies

/// Extrusão de superfície de um círculo do sketch é um cilindro: face cilíndrica e área exata.
#[test]
fn extrusao_de_circulo_e_cilindro() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("b559");
    let (sk, c) = sketch_com(&mut d, |data, al| {
        let ctr = data.add_point(al, 5., -3.);
        data.add_curve(al, Curve::Circle { c: ctr, r: 10. }, false)
    });
    let ex = d.add_geometry(extrusao(CurveRef::SketchCurve { sketch: sk, curve: c }, "30"));
    let r = recompute(&d);
    ok(&r, &d, ex);
    let sh = superficie(&d, ex);
    assert_eq!(sh.face_types().unwrap(), vec![0, 1, 0, 0, 0, 0], "uma face cilíndrica");
    let fp = sh.face_props([15., -3., 15.]).unwrap();
    assert_eq!(fp.kind, 1);
    let area = 2. * PI * 10. * 30.;
    assert!((fp.area - area).abs() < 1e-6 * area, "área {} contra {area}", fp.area);
    assert!((fp.radius - 10.).abs() < 1e-9);
    let (lo, hi) = sh.bbox().unwrap();
    for (a, b) in lo.iter().zip([-5., -13., 0.]).chain(hi.iter().zip([15., 7., 30.])) {
        assert!((a - b).abs() < 1e-6, "caixa {lo:?} {hi:?}");
    }
    // a revolução do arco também fica exata: meia volta de um arco vira toro
    let (sk2, arco) = sketch_com(&mut d, |data, al| {
        let (cc, a, b) = (data.add_point(al, 50., 0.), data.add_point(al, 55., 0.), data.add_point(al, 45., 0.));
        data.add_curve(al, Curve::Arc { c: cc, s: a, e: b }, false)
    });
    let rv = d.add_geometry(RefKind::SurfRevolve { curve: CurveRef::SketchCurve { sketch: sk2, curve: arco }, axis: AxisRef::Origin(OriginAxis::Y), angle: s("180"), angle2: s("0"), mais: vec![] });
    let r = recompute(&d);
    ok(&r, &d, rv);
    let t = superficie(&d, rv).face_types().unwrap();
    assert_eq!(t[4], 1, "o arco revolvido é um toro: {t:?}");
}

/// Superfície de spline sai com uma face B-spline (antes, com volta apertada, virava N faces planas).
#[test]
fn superficie_de_spline_tem_uma_face() {
    if !kernel_disponivel() {
        return;
    }
    let pontos = [[0., 0.], [20., 0.], [21., 6.], [40., 6.], [55., -3.]];
    let mut d = Document::new("b559");
    let (sk, c) = sketch_com(&mut d, |data, al| {
        let ids: Vec<Id> = pontos.iter().map(|p| data.add_point(al, p[0], p[1])).collect();
        data.add_curve(al, Curve::Spline { pts: ids }, false)
    });
    let ex = d.add_geometry(extrusao(CurveRef::SketchCurve { sketch: sk, curve: c }, "15"));
    // a mesma spline no wireframe (pontos no espaço)
    let sp = d.add_geometry(RefKind::Spline { points: pontos.iter().map(|p| PointRef::Fixed([p[0], p[1], 0.])).collect(), closed: false, tangents: vec![] });
    let ex2 = d.add_geometry(extrusao(CurveRef::Ref(sp), "15"));
    let r = recompute(&d);
    ok(&r, &d, ex);
    ok(&r, &d, ex2);
    let (a, b) = (superficie(&d, ex), superficie(&d, ex2));
    for sh in [&a, &b] {
        assert_eq!(sh.face_count(), 1, "uma face só");
        assert_eq!(sh.face_types().unwrap()[5], 1, "a face é B-spline (livre)");
    }
    // sketch e wireframe dão a mesma superfície: mesma definição de spline
    let (fa, fb) = (a.face_props([21., 6., 7.]).unwrap(), b.face_props([21., 6., 7.]).unwrap());
    assert!((fa.area - fb.area).abs() < 1e-6 * fa.area, "áreas {} e {}", fa.area, fb.area);
    // a polilinha de antes, com o canto apertado, virava várias faces
    let poli = curva_de_referencia(&d, sp).unwrap().pts;
    let antiga = Shape::extrude_curve(&poli, [0., 0., 1.], 15.).unwrap();
    assert!(antiga.face_count() >= 1);
    // e o desenho é a mesma curva do sólido: a polilinha passa sobre a face
    for p in poli.iter().step_by(7) {
        let (q, _) = a.surface_near(*p).unwrap();
        assert!(dist(*p, q) < 1e-5, "o desenho sai {} mm da superfície", dist(*p, q));
    }
}

/// Guia com canto vivo: o fio tem uma aresta exata por trecho (reta, arco, reta), não N retas.
#[test]
fn guia_com_canto_vira_arestas_exatas() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("b559");
    let l1 = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([40., 0., 0.]), start: s("0"), end: s("0") });
    let arco = d.add_geometry(RefKind::Circle3 { a: PointRef::Fixed([40., 0., 0.]), b: PointRef::Fixed([50., 10., 0.]), c: PointRef::Fixed([40., 20., 0.]), full: false });
    // canto vivo de 90° no fim do arco
    let l2 = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([40., 20., 0.]), b: PointRef::Fixed([60., 20., 0.]), start: s("0"), end: s("0") });
    let j = d.add_geometry(RefKind::CurveJoin { curves: vec![CurveRef::Ref(l1), CurveRef::Ref(arco), CurveRef::Ref(l2)] });
    let tubo = d.add_geometry(RefKind::SurfSweep { profile: SweepProfile::Circle(s("2")), spine: CurveRef::Ref(j), mode: SweepMode::Corrigido, mais: vec![] });
    let poli = d.add_geometry(RefKind::Polyline { points: vec![PointRef::Fixed([0., 0., 10.]), PointRef::Fixed([30., 0., 10.]), PointRef::Fixed([33., 4., 10.]), PointRef::Fixed([60., 6., 10.])], closed: false });
    let r = recompute(&d);
    for id in [j, tubo, poli] {
        ok(&r, &d, id);
    }
    let c = curva_de_referencia(&d, j).unwrap();
    let e = c.exata.clone().expect("a junção tem forma exata");
    assert!(matches!(e.0.as_slice(), [Trecho3::Reta(..), Trecho3::Arco(..), Trecho3::Reta(..)]), "{e:?}");
    let fio = Shape::curve_shape_c(&e).unwrap();
    assert_eq!(fio.edge_types().unwrap(), vec![2, 1, 0, 0, 0], "duas retas e um arco");
    // a polilinha com cantos vivos vira retas nos cantos (antes, um canto já bastava para a curva
    // inteira virar retas; agora só os cantos quebram, e trecho de voltas pequenas fica suave)
    let p = curva_de_referencia(&d, poli).unwrap();
    assert_eq!(Shape::curve_shape_c(&p.kernel()).unwrap().edge_types().unwrap(), vec![3, 0, 0, 0, 0]);
    // o tubo segue o arco exato: faces de toro no trecho curvo
    let t = superficie(&d, tubo).face_types().unwrap();
    assert!(t[4] >= 1 && t[1] >= 2, "tubo {t:?}");
}

// ------------------------------------------------------------------ B576: Extrair continua Extrair

fn caixa(d: &mut Document) -> Id {
    let _ = d.add_param("L", "40", Unit::Comprimento);
    let sk = sk_rect(d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [60., 30.]);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("L".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }))
}

/// Aresta extraída mantém o tipo: reta continua reta, círculo continua círculo, e as duas acompanham a
/// peça quando o parâmetro muda.
#[test]
fn aresta_extraida_mantem_o_tipo_e_acompanha() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("b576");
    let ex = caixa(&mut d);
    let furo = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([30., 15.], 8.)]);
    let corte = d.add_feature(FeatureKind::Extrude(Extrude { sketch: furo, op: BoolOp::Cut, extent: Extent::ThroughAll, reverse: true, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let r = recompute(&d);
    ok(&r, &d, ex);
    ok(&r, &d, corte);
    let corpo = &r.bodies[0];
    // aresta reta vertical em (60, 0) e o círculo do furo no topo (z = 40), como o clique escolhe
    let aresta = |p: [f64; 3]| {
        let e = aresta_perto(corpo, p);
        let m = ponto_medio(corpo, e);
        CurveRef::BodyEdge { pts: vec![m, m], key: corpo.edge_key(e).expect("aresta com chave") }
    };
    let e_reta = d.add_geometry(RefKind::CurveExtract { edge: aresta([60., 0., 20.]), surface: None, prop: Propagation::Nenhuma, expected: None });
    let e_furo = d.add_geometry(RefKind::CurveExtract { edge: aresta([38., 15., 40.]), surface: None, prop: Propagation::Nenhuma, expected: None });
    let r = recompute(&d);
    ok(&r, &d, e_reta);
    ok(&r, &d, e_furo);
    let (cr, cf) = (curva_de_referencia(&d, e_reta).unwrap(), curva_de_referencia(&d, e_furo).unwrap());
    assert!(matches!(cr.exata.as_ref().map(|e| e.0.as_slice()), Some([Trecho3::Reta(..)])), "{:?}", cr.exata);
    assert!(matches!(cf.exata.as_ref().map(|e| e.0.as_slice()), Some([Trecho3::Circulo { raio, .. }]) if (raio - 8.).abs() < 1e-9), "{:?}", cf.exata);
    // usada como perfil de superfície, o círculo extraído dá cilindro
    let cil = d.add_geometry(extrusao(CurveRef::Ref(e_furo), "10"));
    let r = recompute(&d);
    ok(&r, &d, cil);
    assert_eq!(superficie(&d, cil).face_types().unwrap()[1], 1);
    // muda a altura da peça: os dois extraídos acompanham
    definir_param(&mut d, "L", "55");
    let r = recompute(&d);
    for id in [e_reta, e_furo, cil] {
        ok(&r, &d, id);
    }
    let cr = curva_de_referencia(&d, e_reta).unwrap();
    assert!((cr.length() - 55.).abs() < 1e-6, "a reta extraída não acompanhou: {}", cr.length());
    let cf = curva_de_referencia(&d, e_furo).unwrap();
    assert!(cf.pts.iter().all(|p| (p[2] - 55.).abs() < 1e-6), "o círculo extraído não subiu");
    assert!(d.references.iter().filter(|x| [e_reta, e_furo].contains(&x.id)).all(|x| matches!(x.kind, RefKind::CurveExtract { .. })), "continuam Extrair");
}

/// Arco extraído e borda de face: continuam Extrair, com arco exato, e acompanham o parâmetro.
#[test]
fn arco_e_borda_extraidos_acompanham() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("b576");
    let _ = d.add_param("R", "12", Unit::Comprimento);
    // cilindro de superfície (raio R) e meia volta de revolução: as bordas são arcos
    let ctr = d.add_geometry(RefKind::PointCoords { x: s("0"), y: s("0"), z: s("0"), frame: None });
    let circ = d.add_geometry(RefKind::Circle { center: PointRef::Ref(ctr), plane: PlaneRef::Origin(OriginPlane::XY), radius: s("R"), start: s("0"), end: s("180") });
    let sup = d.add_geometry(extrusao(CurveRef::Ref(circ), "20"));
    let borda = d.add_geometry(RefKind::CurveBoundary { surface: SurfRef::Ref(sup), which: 0, prop: Propagation::Nenhuma, near: Some([0., 12., 20.]), expected: None });
    let r = recompute(&d);
    ok(&r, &d, sup);
    ok(&r, &d, borda);
    let c = curva_de_referencia(&d, borda).unwrap();
    assert!(matches!(c.exata.as_ref().map(|e| e.0.as_slice()), Some([Trecho3::Arco(..)])), "a borda do meio cilindro é um arco: {:?}", c.exata);
    // a mesma borda como Extrair de aresta da superfície
    let topo = superficie(&d, sup).topology().unwrap();
    let k = topo.edge_mids.iter().position(|m| (m[2] - 20.).abs() < 1e-6 && m[1] > 1.).expect("arco do topo");
    let m = topo.edge_mids[k];
    let ext = d.add_geometry(RefKind::CurveExtract { edge: CurveRef::Edge(vec![m, m]), surface: Some(sup), prop: Propagation::Nenhuma, expected: None });
    let r = recompute(&d);
    ok(&r, &d, ext);
    let e = curva_de_referencia(&d, ext).unwrap();
    assert!(matches!(e.exata.as_ref().map(|e| e.0.as_slice()), Some([Trecho3::Arco(..)])), "{:?}", e.exata);
    // muda o raio: a borda acompanha (o arco novo tem raio 18)
    definir_param(&mut d, "R", "18");
    let r = recompute(&d);
    ok(&r, &d, borda);
    let c = curva_de_referencia(&d, borda).unwrap();
    let Some([Trecho3::Arco(a, mm, b)]) = c.exata.as_ref().map(|e| e.0.as_slice()) else { panic!("{:?}", c.exata) };
    for p in [a, mm, b] {
        assert!(((p[0] * p[0] + p[1] * p[1]).sqrt() - 18.).abs() < 1e-6, "a borda não acompanhou o raio: {p:?}");
    }
}

/// Extrair várias curvas de uma vez é um Extrair só (B576), com a cadeia exata.
#[test]
fn extrair_varias_e_um_extrair() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("b576");
    let sk = sk_poligono(&mut d, PlaneRef::Origin(OriginPlane::XY), &[[0., 0.], [40., 0.], [40., 20.], [0., 20.]]);
    let r = recompute(&d);
    let curvas: Vec<CurveRef> = match &d.feature(sk).unwrap().kind {
        FeatureKind::Sketch(s) => s.data.curves.iter().map(|c| CurveRef::SketchCurve { sketch: sk, curve: c.id }).collect(),
        _ => unreachable!(),
    };
    let _ = r;
    let e = d.add_geometry(RefKind::CurveCopy { curve: curvas[0].clone(), mais: curvas[1..].to_vec() });
    let r = recompute(&d);
    ok(&r, &d, e);
    let c = curva_de_referencia(&d, e).unwrap();
    assert!(c.closed);
    let ex = c.exata.expect("cadeia exata");
    assert_eq!(ex.0.len(), 4);
    assert!(ex.0.iter().all(|t| matches!(t, Trecho3::Reta(..))));
    // o contorno extraído vira sólido
    let s = d.add_feature(FeatureKind::Extrude(Extrude { sketch: e, op: BoolOp::Add, extent: Extent::Blind("5".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let r = recompute(&d);
    ok(&r, &d, s);
    assert!((r.bodies[0].mass.volume - 4000.).abs() < 1e-6);
}

// ------------------------------------------------------------------ chapa: aba curva no perfil de asa

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

/// Aba curva sobre a borda de spline de um perfil de asa (21 pontos, corda 200): a aba segue a aresta
/// exata, sai com poucas faces (não uma por trecho da polilinha) e a união com a chapa é válida.
#[test]
fn aba_curva_no_perfil_de_asa_tem_poucas_faces() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("asa");
    d.sheet = Some(SheetParams { thickness: s("1"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let (sk, _) = sketch_com(&mut d, |data, al| {
        let p = pontos_asa(10);
        let ids: Vec<Id> = p.iter().map(|q| data.add_point(al, q[0], q[1])).collect();
        let (a, b) = (ids[0], *ids.last().unwrap());
        let sp = data.add_curve(al, Curve::Spline { pts: ids }, false);
        data.add_curve(al, Curve::Line { a: b, b: a }, false);
        sp
    });
    let w = d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let r = recompute(&d);
    ok(&r, &d, w);
    let base = final_shape(&d).unwrap();
    let (vb, fb) = (base.mass().volume, base.face_count());
    let (lo, hi) = base.bbox().unwrap();
    // a aresta de spline da face de cima (a mais longa no topo)
    let topo = base.topology().unwrap();
    let longa = topo
        .edge_mids
        .iter()
        .filter(|m| (m[2] - hi[2]).abs() < 1e-6)
        .filter_map(|m| base.edge_at(*m).ok())
        .max_by(|a, b| {
            let l = |p: &Vec<[f64; 3]>| p.windows(2).map(|w| dist(w[0], w[1])).sum::<f64>();
            l(a).total_cmp(&l(b))
        })
        .expect("aresta de cima");
    // como o clique: a aresta da malha mais perto do meio dela, com a chave (dono + papel)
    let corpo = &r.bodies[0];
    let e = aresta_perto(corpo, longa[longa.len() / 2]);
    let key = corpo.edge_key(e);
    assert!(key.is_some(), "a aresta de spline tem chave");
    let aba = d.add_feature(FeatureKind::SmCurveFlange(SmCurveFlange { edge: longa, length: s("15"), flip: false, angle: s("90"), radius: None, side: 0, edge_key: key, so_aresta: true, reta: None }));
    let r = recompute(&d);
    ok(&r, &d, aba);
    let sh = final_shape(&d).unwrap();
    assert!(sh.is_valid(), "a chapa com a aba é válida");
    assert_eq!(r.bodies.len(), 1);
    let v = sh.mass().volume;
    assert!(v > vb + 3000., "a aba acrescentou volume: {v} (base {vb})");
    // poucas faces: a varredura da seção ao longo de uma aresta só (antes, uma faixa por trecho)
    let novas = sh.face_count() as i64 - fb as i64;
    eprintln!("aba no perfil de asa: {novas} faces novas (base {fb}), volume {v:.1} (base {vb:.1})");
    assert!(novas <= 12, "a aba acrescentou {novas} faces (base {fb}, total {})", sh.face_count());
    let (lo2, hi2) = sh.bbox().unwrap();
    assert!(hi2[2] > hi[2] + 14., "a aba subiu: {hi2:?}");
    assert!(lo2[0] >= lo[0] - 1e-3 - 3. && hi2[0] <= hi[0] + 3. + 1e-3, "a aba ficou na borda: {lo2:?} {hi2:?}");
}
