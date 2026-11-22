// SPDX-License-Identifier: LGPL-2.1-or-later
//! B524 (ponto em face nasce no centroide) e B536 (engrossar com lados e direção). Documento montado
//! em código, recalculado e conferido por status, coordenadas do ponto, volume e caixa envolvente.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.into()
}

fn ok(r: &RecomputeResult, d: &Document, id: Id) {
    assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{}: {:?}", d.name_of(id), r.status.get(&id));
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn reta(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }))
}

/// Caixa pela malha do corpo (a do kernel inclui polos de B-spline e tolerância).
fn caixa(r: &RecomputeResult) -> ([f64; 3], [f64; 3]) {
    let mut a = [f64::MAX; 3];
    let mut b = [f64::MIN; 3];
    for p in r.bodies[0].mesh.positions.chunks_exact(3) {
        for k in 0..3 {
            a[k] = a[k].min(p[k] as f64);
            b[k] = b[k].max(p[k] as f64);
        }
    }
    (a, b)
}

fn ponto(r: &RecomputeResult, id: Id) -> [f64; 3] {
    r.points.iter().find(|p| p.id == id).expect("ponto no resultado").p
}

fn face(p: [f64; 3], n: [f64; 3]) -> SurfRef {
    SurfRef::Face { point: p, normal: n, owner: None, key: None }
}

fn ponto_na_face(d: &mut Document, sf: SurfRef, centro: bool) -> Id {
    d.add_geometry(RefKind::PointOnSurface { surface: sf, u: s("0"), v: s("0"), centro })
}

// ------------------------------------------------------------------ B524

/// Retângulo de 100 × 40: o ponto nasce no centro, não no clique, e segue a face quando ela cresce.
/// Sem a opção (documentos antigos) continua no ponto clicado.
#[test]
fn b524_ponto_no_centro_do_retangulo_acompanha_a_face() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("b524");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 40.]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    let topo = face([10., 5., 10.], [0., 0., 1.]);
    let c = ponto_na_face(&mut d, topo, true);
    let clicado = ponto_na_face(&mut d, topo, false);
    let r = recompute(&d);
    ok(&r, &d, c);
    ok(&r, &d, clicado);
    let p = ponto(&r, c);
    assert!(perto(p[0], 50., 1e-6) && perto(p[1], 20., 1e-6) && perto(p[2], 10., 1e-6), "centro do retângulo: {p:?}");
    assert_eq!(ponto(&r, clicado), [10., 5., 10.], "a opção do ponto clicado fica onde clicou");
    // a face muda de tamanho: o centro anda junto
    set_rect(&mut d, sk, [0., 0.], [120., 60.]);
    let r = recompute(&d);
    ok(&r, &d, c);
    let p = ponto(&r, c);
    assert!(perto(p[0], 60., 1e-6) && perto(p[1], 30., 1e-6) && perto(p[2], 10., 1e-6), "segue a face: {p:?}");
}

/// Disco (topo de um cilindro): o centroide é o centro do círculo.
#[test]
fn b524_ponto_no_centro_do_disco() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("disco");
    d.add_feature(primitiva(PrimKind::Cilindro, "10", "30", "0", "0"));
    let r = recompute(&d);
    let (_, hi) = caixa(&r);
    let c = ponto_na_face(&mut d, face([3., 2., hi[2]], [0., 0., 1.]), true);
    let r = recompute(&d);
    ok(&r, &d, c);
    let p = ponto(&r, c);
    assert!(perto(p[0], 0., 1e-6) && perto(p[1], 0., 1e-6) && perto(p[2], hi[2], 1e-6), "centro do disco: {p:?}");
}

/// L fino: o centroide cai fora da face e é projetado de volta nela (o ponto mais próximo da face).
#[test]
fn b524_centroide_fora_da_face_nao_convexa_vai_para_a_face() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("L fino");
    let sk = sk_poligono(&mut d, PlaneRef::Origin(OriginPlane::XY), &[[0., 0.], [40., 0.], [40., 4.], [4., 4.], [4., 40.], [0., 40.]]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    let c = ponto_na_face(&mut d, face([2., 30., 10.], [0., 0., 1.]), true);
    let r = recompute(&d);
    ok(&r, &d, c);
    let p = ponto(&r, c);
    // centroide: (160·20 + 144·2) / 304 nos dois eixos, fora do L
    let g = (160. * 20. + 144. * 2.) / 304.;
    assert!(perto(p[2], 10., 1e-6), "na face de cima: {p:?}");
    assert!(p[0] <= 4. + 1e-6 || p[1] <= 4. + 1e-6, "dentro do L: {p:?}");
    let dist = ((p[0] - g).powi(2) + (p[1] - g).powi(2)).sqrt();
    assert!(perto(dist, g - 4., 1e-3), "o mais perto do centroide: {p:?} a {dist}");
}

// ------------------------------------------------------------------ B536

/// Folha plana de 100 × 40 no plano XY.
fn retangulo(d: &mut Document) -> Id {
    let c = reta(d, [0., 0., 0.], [100., 0., 0.]);
    d.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(OriginAxis::Y), length: s("40"), length2: s("0"), until: None, until2: None, mais: vec![] })
}

/// Faixa de cilindro de raio 20 e comprimento 50 em volta do eixo X, de -60° a 60° a partir do topo.
fn faixa_curva(d: &mut Document) -> Id {
    let c = reta(d, [0., 0., 20.], [50., 0., 20.]);
    d.add_geometry(RefKind::SurfRevolve { curve: c, axis: AxisRef::Origin(OriginAxis::X), angle: s("60"), angle2: s("60"), mais: vec![] })
}

fn engrossar(d: &mut Document, sup: Id, t1: &str, t2: &str, simetrico: bool, dir: Option<DirecaoEspessura>) -> Id {
    let mut t = ThickSurface::normal(SurfRef::Ref(sup), t1, t2);
    t.simetrico = simetrico;
    t.dir = dir;
    d.add_feature(FeatureKind::ThickSurface(t))
}

const Z: Option<DirecaoEspessura> = Some(DirecaoEspessura::Eixo(AxisRef::Origin(OriginAxis::Z)));

/// Superfície curva engrossada na direção Z: paredes verticais. O volume é a área projetada no XY vezes
/// a espessura, e a caixa não passa da projeção da faixa em Y (o deslocamento pela normal passaria).
/// Um lado para cima, o outro para baixo e o simétrico, cada um com a sua caixa.
#[test]
fn b536_superficie_curva_na_direcao_z_tem_paredes_verticais() {
    if !kernel_disponivel() {
        return;
    }
    let meia = 20. * 60f64.to_radians().sin();
    let area_xy = 50. * 2. * meia;
    // (t1, t2, simétrico, z mínimo, z máximo, espessura total)
    for (t1, t2, sim, z0, z1, total) in [("5", "0", false, 10., 25., 5.), ("0", "5", false, 5., 20., 5.), ("10", "0", true, 5., 25., 10.)] {
        let mut d = Document::new("b536");
        let f = faixa_curva(&mut d);
        let g = engrossar(&mut d, f, t1, t2, sim, Z);
        let r = recompute(&d);
        ok(&r, &d, g);
        assert_eq!(r.bodies.len(), 1);
        let v = r.bodies[0].mass.volume;
        assert!(perto(v, area_xy * total, 1e-3 * area_xy * total), "{t1}/{t2}/{sim}: volume {v} esperado {}", area_xy * total);
        let (a, b) = caixa(&r);
        assert!(perto(a[0], 0., 1e-6) && perto(b[0], 50., 1e-6), "comprimento {a:?} {b:?}");
        assert!(perto(a[1], -meia, 1e-3) && perto(b[1], meia, 1e-3), "{t1}/{t2}/{sim}: paredes verticais em Y = ±{meia}: {a:?} {b:?}");
        assert!(perto(a[2], z0, 1e-3) && perto(b[2], z1, 0.1), "{t1}/{t2}/{sim}: altura {a:?} {b:?}, esperado {z0}..{z1}");
    }
    // pela normal, a mesma faixa engrossada para fora passa da projeção em Y: as paredes seguem o raio
    let mut d = Document::new("normal");
    let f = faixa_curva(&mut d);
    let g = engrossar(&mut d, f, "5", "0", false, None);
    let g2 = engrossar(&mut d, f, "0", "5", false, None);
    let r = recompute(&d);
    ok(&r, &d, g);
    ok(&r, &d, g2);
    let (a, b) = caixa(&r);
    assert!(b[1] - a[1] > 2. * meia + 1., "pela normal as paredes abrem: {a:?} {b:?}");
}

/// Folha plana: a direção pela normal de um plano dá o mesmo que o eixo; uma direção oblíqua varre o
/// comprimento ao longo dela (volume = área × espessura × cos do ângulo com a normal); a direção
/// paralela à folha não varre volume e o comando diz por quê.
#[test]
fn b536_folha_plana_por_plano_obliqua_e_paralela() {
    if !kernel_disponivel() {
        return;
    }
    let casos: [(Option<DirecaoEspessura>, f64, [f64; 3], [f64; 3]); 3] = [
        (Z, 20000., [0., 0., 0.], [100., 40., 5.]),
        (Some(DirecaoEspessura::Plano(PlaneRef::Origin(OriginPlane::XY))), 20000., [0., 0., 0.], [100., 40., 5.]),
        (
            Some(DirecaoEspessura::Eixo(AxisRef::Fixed { origin: [0.; 3], dir: [0., 1., 1.] })),
            4000. * 5. * std::f64::consts::FRAC_1_SQRT_2,
            [0., 0., 0.],
            [100., 40. + 5. * std::f64::consts::FRAC_1_SQRT_2, 5. * std::f64::consts::FRAC_1_SQRT_2],
        ),
    ];
    for (dir, vol, lo, hi) in casos {
        let mut d = Document::new("folha");
        let f = retangulo(&mut d);
        let g = engrossar(&mut d, f, "5", "0", false, dir);
        let r = recompute(&d);
        ok(&r, &d, g);
        let v = r.bodies[0].mass.volume;
        assert!(perto(v, vol, 1e-6 * vol), "{dir:?}: volume {v} esperado {vol}");
        let (a, b) = caixa(&r);
        for k in 0..3 {
            assert!(perto(a[k], lo[k], 1e-4) && perto(b[k], hi[k], 1e-4), "{dir:?}: caixa {a:?} {b:?} esperada {lo:?} {hi:?}");
        }
    }
    let mut d = Document::new("paralela");
    let f = retangulo(&mut d);
    let g = engrossar(&mut d, f, "5", "0", false, Some(DirecaoEspessura::Eixo(AxisRef::Origin(OriginAxis::X))));
    let r = recompute(&d);
    match r.status.get(&g) {
        Some(Status::Error(m)) => assert!(m.contains("paralela"), "diz que a direção é paralela: {m}"),
        o => panic!("esperava erro, veio {o:?}"),
    }
}

/// Simétrico pela normal: metade para cada lado; a espessura 2 não conta.
#[test]
fn b536_simetrico_pela_normal() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("simetrico");
    let f = retangulo(&mut d);
    let g = engrossar(&mut d, f, "6", "99", true, None);
    let r = recompute(&d);
    ok(&r, &d, g);
    let v = r.bodies[0].mass.volume;
    assert!(perto(v, 4000. * 6., 1e-6 * 24000.), "volume {v}");
    let (a, b) = caixa(&r);
    assert!(perto(a[2], -3., 1e-4) && perto(b[2], 3., 1e-4), "metade para cada lado: {a:?} {b:?}");
}
