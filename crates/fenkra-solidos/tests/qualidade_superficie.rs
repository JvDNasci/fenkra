// SPDX-License-Identifier: LGPL-2.1-or-later
//! B180: o verificador de conexão G0 a G2 vale entre faces de um sólido, inclusive filetes.
//! O documento é montado em código, recalculado e conferido por status, volume e caixa.

use fenkra_core::*;
use fenkra_solidos::analise::{Analise, Nivel};
use fenkra_solidos::{recompute, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.into()
}

fn ok(r: &RecomputeResult, id: Id) {
    assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{:?}", r.status[&id]);
}

fn caixa_do_corpo(r: &RecomputeResult) -> ([f64; 3], [f64; 3]) {
    r.bodies[0].mesh.positions.chunks_exact(3).fold(([f64::MAX; 3], [f64::MIN; 3]), |(lo, hi), p| ([0, 1, 2].map(|i| lo[i].min(p[i] as f64)), [0, 1, 2].map(|i| hi[i].max(p[i] as f64))))
}

#[test]
fn conexao_entre_filete_e_face_plana_e_g1_com_salto_do_raio() {
    let mut d = Document::new("filete");
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("40"), b: s("40"), c: s("40"), d: s("0"), op: BoolOp::Add }));
    let fil = d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("5"), edges: EdgeSel::AllOfBody, mode: Default::default(), prop: Default::default(), keys: vec![], extra: Default::default() }));
    let r = recompute(&d);
    ok(&r, fil);
    let (lo, hi) = caixa_do_corpo(&r);
    for k in 0..3 {
        assert!((hi[k] - lo[k] - 40.).abs() < 1e-3, "caixa {lo:?} {hi:?}");
    }
    // volume do cubo arredondado: núcleo, seis placas, doze quartos de cilindro e oito oitavos de esfera
    let (r5, m): (f64, f64) = (5., 30.);
    let pi = std::f64::consts::PI;
    let esperado = m.powi(3) + 6. * r5 * m * m + 3. * pi * r5 * r5 * m + 4. / 3. * pi * r5.powi(3);
    let vol = r.bodies[0].mass.volume;
    assert!((vol - esperado).abs() < 1e-3 * esperado, "volume {vol} contra {esperado}");

    let c = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.];
    let topo = SurfRef::Face { point: [c[0], c[1], hi[2]], normal: [0., 0., 1.], owner: Some(fil), key: None };
    let a45 = std::f64::consts::FRAC_1_SQRT_2;
    let meio_do_filete = [hi[0] - r5 + r5 * a45, c[1], hi[2] - r5 + r5 * a45];
    let filete = SurfRef::Face { point: meio_do_filete, normal: [a45, 0., a45], owner: Some(fil), key: None };
    let id = d.add_geometry(RefKind::SurfCheck { mode: CheckMode::Conexao, a: filete, b: topo, curve: CurveRef::default(), samples: s("8"), scale: s("0.01") });
    let r = recompute(&d);
    ok(&r, id);
    let Some(Analise::Conexao(x)) = r.notes.get(&id).and_then(|n| n.analise.as_ref()) else { panic!("nota da conexão") };
    assert!(!x.amostras.is_empty());
    assert!(x.folga < 1e-6, "folga {}", x.folga);
    assert!(x.angulo < 0.01, "o filete é tangente: {}", x.angulo);
    assert!((x.dif_curvatura - 1. / r5).abs() < 1e-3, "salto de curvatura do raio 5: {}", x.dif_curvatura);
    assert_eq!(x.nivel(), Nivel::G1);
    // todas as amostras estão na aresta comum, a 5 mm do canto de cima
    assert!(x.amostras.iter().all(|a| (a.ponto[2] - hi[2]).abs() < 1e-6 && (a.ponto[0] - (hi[0] - r5)).abs() < 1e-6), "{:?}", x.amostras.iter().map(|a| a.ponto).collect::<Vec<_>>());
}
