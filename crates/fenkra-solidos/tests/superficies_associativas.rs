// SPDX-License-Identifier: LGPL-2.1-or-later
//! B13: centro e eixo de aresta circular, normal de face plana e ponto clicado numa face seguem o
//! modelo em vez de ficarem presos a coordenadas.

use fenkra_core::*;
use fenkra_solidos::*;

fn s(x: &str) -> String {
    x.to_string()
}

fn ok(r: &RecomputeResult, id: Id) {
    assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{:?}", r.status.get(&id));
}

#[test]
fn centro_eixo_de_aresta_circular_e_ponto_na_face_acompanham_o_modelo() {
    let mut d = Document::new("t");
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    let c = data.add_point(&mut al, 5., 0.);
    data.add_curve(&mut al, sketch::Curve::Circle { c, r: 10. }, false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    let cil = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("30")), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let r = recompute(&d);
    let b = &r.bodies[0];
    // aresta circular de cima e tampa de cima
    let m = &b.mesh.edge_mids;
    let e = (0..b.mesh.edge_faces.len()).find(|e| (m[3 * e + 2] as f64 - 30.).abs() < 1e-3 && b.edge_key(*e).is_some()).unwrap();
    let (key, mid) = (b.edge_key(e).unwrap(), [m[3 * e] as f64, m[3 * e + 1] as f64, m[3 * e + 2] as f64]);
    let topo = (0..b.mesh.face_info.len()).find(|j| b.mesh.face_info[*j].kind == 0 && (b.mesh.face_info[*j].point[2] - 30.).abs() < 1e-3).unwrap();
    let chave = b.face_key(topo).unwrap();
    let clique = [8., 3., 30.];
    let centro = d.add_geometry(RefKind::PointBetween { a: PointRef::Center { point: [5., 0., 30.], edge: key }, b: PointRef::Origin, ratio: s("0") });
    let na_face = d.add_geometry(RefKind::PointBetween { a: PointRef::OnFace { point: clique, key: chave }, b: PointRef::Origin, ratio: s("0") });
    let eixo = d.add_geometry(RefKind::AxisLine { axis: AxisRef::Edge { mid, key: Some(key) }, length: s("10") });
    let normal = d.add_geometry(RefKind::AxisLine { axis: AxisRef::Face { point: clique, key: Some(chave) }, length: s("10") });
    let confere = |d: &Document, z: f64| {
        let r = recompute(d);
        for x in [centro, na_face, eixo, normal] {
            ok(&r, x);
        }
        let p = |x: Id| r.points.iter().find(|p| p.id == x).unwrap().p;
        assert!(geo3d::dist(p(centro), [5., 0., z]) < 1e-6, "{:?}", p(centro));
        assert!(geo3d::dist(p(na_face), [8., 3., z]) < 1e-6, "{:?}", p(na_face));
        for x in [eixo, normal] {
            let c = &r.curves.iter().find(|c| c.id == x).unwrap().pts;
            let v = geo3d::sub(c[1], c[0]);
            assert!(v[0].abs() < 1e-6 && v[1].abs() < 1e-6 && v[2].abs() > 9.99, "o eixo é a normal em Z: {c:?}");
        }
    };
    confere(&d, 30.);
    let FeatureKind::Extrude(e) = &mut d.feature_mut(cil).unwrap().kind else { panic!() };
    e.extent = Extent::Blind(s("45"));
    confere(&d, 45.);
}
