// SPDX-License-Identifier: LGPL-2.1-or-later
//! B509: medir item e medir entre num documento montado em código; a medida mantida segue o modelo
//! e fica órfã quando a referência some.

use crate::medir::{self, Leitura};
use fenkra_core::sketch::Curve;
use fenkra_core::*;
use fenkra_solidos::{recompute, RecomputeResult, Status};
use std::f64::consts::PI;

fn extrudar(d: &mut Document, data: SketchData) -> Id {
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("h".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }))
}

/// Cilindro de raio 10 e altura `h` (parâmetro), extrudado de um círculo.
fn cilindro(h: &str) -> Document {
    let mut d = Document::new("cilindro");
    d.add_param("h", h, Unit::Comprimento).unwrap();
    let mut data = SketchData::default();
    let mut ids = 9_000_000u64;
    let mut al = || {
        ids += 1;
        Id(ids)
    };
    let c = data.add_point(&mut al, 0., 0.);
    data.add_curve(&mut al, Curve::Circle { c, r: 10. }, false);
    extrudar(&mut d, data);
    d
}

/// Bloco 10 × 20 × `h` (x de −5 a 5, y de −10 a 10, z de 0 a h).
fn bloco(h: &str) -> Document {
    let mut d = Document::new("bloco");
    d.add_param("h", h, Unit::Comprimento).unwrap();
    let mut data = SketchData::default();
    let mut ids = 9_100_000u64;
    let mut al = || {
        ids += 1;
        Id(ids)
    };
    data.add_rect(&mut al, [-5., -10.], [5., 10.], false);
    extrudar(&mut d, data);
    d
}

fn calcular(d: &Document) -> RecomputeResult {
    let r = recompute(d);
    for f in &d.features {
        assert!(matches!(r.status.get(&f.id), Some(Status::Ok)), "{}: {:?}", f.name, r.status.get(&f.id));
    }
    r
}

/// Referência à face do corpo que satisfaz `qual` (pela assinatura do kernel), com a chave v2.
fn face(r: &RecomputeResult, qual: impl Fn(&fenkra_kernel::FaceInfo) -> bool) -> RefMedida {
    let b = &r.bodies[0];
    let j = b.mesh.face_info.iter().position(qual).expect("face");
    let key = b.face_key(j);
    assert!(key.is_some(), "face {j} sem chave v2");
    RefMedida { elem: ElemMedida::Face, ponto: b.mesh.face_info[j].point, face: key, aresta: None }
}

fn plano(n: [f64; 3]) -> impl Fn(&fenkra_kernel::FaceInfo) -> bool {
    move |f| f.kind == 0 && (0..3).all(|i| (f.dir[i] - n[i]).abs() < 1e-6)
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn valor(l: &Leitura, r: &str) -> f64 {
    l.valor(r).unwrap_or_else(|| panic!("sem {r}: {:?}", l.linhas))
}

#[test]
fn mede_raio_e_area_do_cilindro_e_a_medida_mantida_segue_a_altura() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = cilindro("30");
    let r = calcular(&d);
    let b = &r.bodies[0];
    assert!(perto(b.mass.volume, PI * 100. * 30., 1.), "volume {}", b.mass.volume);
    let lateral = face(&r, |f| f.kind == 1);
    let g = medir::resolver(&r, &lateral).expect("lateral");
    let l = medir::medir_item(&g);
    assert!(perto(valor(&l, "Raio"), 10., 1e-3), "{:?}", l.linhas);
    assert!(perto(valor(&l, "Diâmetro"), 20., 2e-3));
    assert!(perto(valor(&l, "Área"), 2. * PI * 10. * 30., 1e-3), "{:?}", l.linhas);
    // a aresta circular do topo: comprimento da circunferência e raio exatos da fórmula
    let topo = face(&r, plano([0., 0., 1.]));
    let l = medir::medir_item(&medir::resolver(&r, &topo).unwrap());
    assert!(perto(valor(&l, "Área"), PI * 100., 1e-3), "{:?}", l.linhas);

    // manter a medida: ela guarda a chave e é refeita no modelo novo
    let id = d.add_medida(ModoMedida::Item, vec![lateral], vec![], false);
    assert_eq!(d.medida(id).unwrap().nome, "Medida1");
    if let Some(p) = d.params.iter_mut().find(|p| p.name == "h") {
        p.expr = "50".into();
    }
    let r2 = calcular(&d);
    assert!(perto(r2.bodies[0].mass.volume, PI * 100. * 50., 1.));
    let l = medir::avaliar(&r2, d.medida(id).unwrap());
    assert!(!l.orfa());
    assert!(perto(valor(&l, "Área"), 2. * PI * 10. * 50., 1e-3), "{:?}", l.linhas);
    assert!(perto(valor(&l, "Raio"), 10., 1e-3));
    // a medida é gravada e volta ao abrir
    let lido = Document::from_json(&d.to_json()).unwrap();
    assert_eq!(lido.medidas, d.medidas);
}

#[test]
fn distancia_e_angulo_entre_faces_e_medida_mantida_segue_o_modelo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = bloco("30");
    let r = calcular(&d);
    let b = &r.bodies[0];
    assert!(perto(b.mass.volume, 10. * 20. * 30., 1e-3), "volume {}", b.mass.volume);
    let (lo, hi) = (b.mesh.positions.chunks_exact(3).fold([f32::MAX; 3], |a, p| [a[0].min(p[0]), a[1].min(p[1]), a[2].min(p[2])]), b.mesh.positions.chunks_exact(3).fold([f32::MIN; 3], |a, p| [a[0].max(p[0]), a[1].max(p[1]), a[2].max(p[2])]));
    assert_eq!((lo, hi), ([-5., -10., 0.], [5., 10., 30.]));

    let esq = face(&r, plano([-1., 0., 0.]));
    let dir = face(&r, plano([1., 0., 0.]));
    let topo = face(&r, plano([0., 0., 1.]));
    let base = face(&r, plano([0., 0., -1.]));
    let g = |rf: &RefMedida| medir::resolver(&r, rf).unwrap();

    // faces opostas: distância mínima 10, plano a plano 10, paralelas
    let l = medir::medir_entre(&[g(&esq)], &[g(&dir)], false);
    assert!(perto(valor(&l, "Distância mínima"), 10., 1e-4), "{:?}", l.linhas);
    assert!(perto(valor(&l, "Plano a plano"), 10., 1e-9));
    assert!(perto(valor(&l, "Ângulo"), 0., 1e-6));
    let (p, q) = l.apoio.unwrap();
    assert!(perto((p[0] - q[0]).abs(), 10., 1e-4));

    // faces perpendiculares: ângulo 90 e encostadas (distância 0)
    let l = medir::medir_entre(&[g(&dir)], &[g(&topo)], false);
    assert!(perto(valor(&l, "Ângulo"), 90., 1e-6), "{:?}", l.linhas);
    assert!(perto(valor(&l, "Distância mínima"), 0., 1e-4));
    assert!(l.valor("Plano a plano").is_none());

    // geometria infinita: o plano da face direita contra um vértice do topo
    let canto = medir::Geo::Ponto([-5., -10., 30.]);
    let l = medir::medir_entre(&[g(&dir)], &[canto], true);
    assert!(perto(valor(&l, "Distância (ilimitada)"), 10., 1e-9), "{:?}", l.linhas);

    // medida mantida entre o topo e a base; a altura muda e a medida acompanha
    let id = d.add_medida(ModoMedida::Entre, vec![topo], vec![base], false);
    for p in d.params.iter_mut().filter(|p| p.name == "h") {
        p.expr = "45".into();
    }
    let r2 = calcular(&d);
    assert!(perto(r2.bodies[0].mass.volume, 10. * 20. * 45., 1e-3));
    let l = medir::avaliar(&r2, d.medida(id).unwrap());
    assert!(perto(valor(&l, "Distância mínima"), 45., 1e-4), "{:?}", l.linhas);
    assert!(perto(valor(&l, "Plano a plano"), 45., 1e-9));

    // a extrusão some: a medida fica órfã, sem valor velho
    let ext = d.features.iter().find(|f| matches!(f.kind, FeatureKind::Extrude(_))).unwrap().id;
    d.feature_mut(ext).unwrap().suppressed = true;
    let r3 = recompute(&d);
    let l = medir::avaliar(&r3, d.medida(id).unwrap());
    assert!(l.orfa(), "{l:?}");
    assert!(l.linhas.is_empty());
    assert_eq!(crate::medir_ui::resumo(&l), "órfã: referência perdida");
}

#[test]
fn rotulos_do_calculo_tem_traducao() {
    for r in medir::ROTULOS {
        assert!(crate::i18n::tem_traducao(r), "{r} sem tradução");
    }
}
