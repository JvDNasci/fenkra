//! B79: a falha de uma linha de membro não derruba as outras e aparece com o motivo.

use fenkra_core::*;
use fenkra_solidos::{member_failures, recompute, Status, MEMBRO_FALHOU};

fn membro(paths: Vec<CurveRef>) -> StructMember {
    let s = |x: &str| x.to_string();
    StructMember {
        paths,
        family: s("Tubo quadrado"),
        designation: s("40x3"),
        kind: SectionKind::TuboQuadrado,
        dims: vec![s("40"), s("3")],
        anchor: 0,
        angle: s("0"),
        off_x: s("0"),
        off_y: s("0"),
        ext1: s("0"),
        ext2: s("0"),
        corner: CornerMode::MeiaEsquadria,
        gap: s("0"),
        joints: vec![],
    }
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: "0".into(), end: "0".into() }))
}

fn volume(r: &fenkra_solidos::RecomputeResult) -> f64 {
    r.bodies.iter().map(|b| b.mass.volume).sum()
}

#[test]
fn linha_que_falha_nao_derruba_as_outras() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("falha");
    // duas linhas sobrepostas saindo do mesmo ponto (meia-esquadria impossível) e uma linha livre
    let l0 = linha(&mut d, [0., 0., 0.], [100., 0., 0.]);
    let l1 = linha(&mut d, [0., 0., 0.], [50., 0., 0.]);
    let l2 = linha(&mut d, [500., 0., 0.], [500., 200., 0.]);
    let f = d.add_feature(FeatureKind::StructMember(membro(vec![l0, l1, l2])));
    let r = recompute(&d);
    let Status::Warning(w) = &r.status[&f] else { panic!("esperava aviso, veio {:?}", r.status[&f]) };
    assert!(w.starts_with(MEMBRO_FALHOU), "{w}");
    assert!(w.contains("linha 1") && w.contains("linha 2"), "{w}");
    // sobrou só a linha livre: tubo 40x3, 200 mm
    let esperado = (40. * 40. - 34. * 34.) * 200.;
    assert!((volume(&r) - esperado).abs() < 1e-3 * esperado, "volume {}", volume(&r));
    let falhas = member_failures(&d);
    assert_eq!(falhas.len(), 2);
    assert!(falhas.iter().all(|x| x.feature == f && !x.pts.is_empty() && !x.reason.is_empty()));
    assert_eq!(falhas.iter().map(|x| x.line).collect::<Vec<_>>(), vec![0, 1]);
}

#[test]
fn sem_falha_nada_a_marcar() {    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("ok");
    let l0 = linha(&mut d, [0., 0., 0.], [100., 0., 0.]);
    let f = d.add_feature(FeatureKind::StructMember(membro(vec![l0])));
    let r = recompute(&d);
    assert_eq!(r.status[&f], Status::Ok);
    assert!(member_failures(&d).is_empty());
}
