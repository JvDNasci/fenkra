// SPDX-License-Identifier: LGPL-2.1-or-later
//! B80: a lista de corte lê comprimento, ângulos das pontas e material do próprio sólido.

use fenkra_core::*;
use fenkra_solidos::cut_list;

fn membro(paths: Vec<CurveRef>, material: &str) -> StructMember {
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
        material: material.into(),
    }
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: "0".into(), end: "0".into() }))
}

#[test]
fn angulos_de_ponta_vem_do_solido() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("l");
    // L de 90°: a meia-esquadria corta 45° nas pontas que se encontram
    let a = linha(&mut d, [0., 0., 0.], [300., 0., 0.]);
    let b = linha(&mut d, [300., 0., 0.], [300., 300., 0.]);
    // barra solta, corte reto nas duas pontas
    let c = linha(&mut d, [0., 500., 0.], [200., 500., 0.]);
    d.add_feature(FeatureKind::StructMember(membro(vec![a], "")));
    d.add_feature(FeatureKind::StructMember(membro(vec![b], "")));
    d.add_feature(FeatureKind::StructMember(membro(vec![c], "")));
    let rows = cut_list(&d);
    assert_eq!(rows.len(), 3, "{rows:?}");
    let reta = rows.iter().find(|r| (r.length - 200.).abs() < 0.1).expect("barra reta");
    assert_eq!((reta.angle1, reta.angle2), (Some(0.), Some(0.)));
    assert!((reta.long - 200.).abs() < 0.2, "ponta longa de corte reto = comprimento: {}", reta.long);
    let l1 = rows.iter().find(|r| r.members[0].0 == rows.iter().map(|x| x.feature).min().unwrap() && (r.length - 300.).abs() < 0.1).unwrap();
    assert_eq!(l1.angle1, Some(0.));
    assert!((l1.angle2.unwrap() - 45.).abs() < 0.2, "{:?}", l1.angle2);
    // meia-esquadria: a ponta longa passa meia seção (20 mm) da linha de centro
    assert!((l1.long - 320.).abs() < 0.5, "{}", l1.long);
    let l2 = rows.iter().find(|r| r.angle1.is_some_and(|x| (x - 45.).abs() < 0.2)).expect("segunda perna do L");
    assert_eq!(l2.angle2, Some(0.));
}

#[test]
fn materiais_diferentes_ficam_em_itens_separados() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("mat");
    d.props.material = "Aço 1020".into();
    let a = linha(&mut d, [0., 0., 0.], [300., 0., 0.]);
    let b = linha(&mut d, [0., 100., 0.], [300., 100., 0.]);
    let c = linha(&mut d, [0., 200., 0.], [300., 200., 0.]);
    d.add_feature(FeatureKind::StructMember(membro(vec![a], "")));
    d.add_feature(FeatureKind::StructMember(membro(vec![b], "")));
    d.add_feature(FeatureKind::StructMember(membro(vec![c], "Aço inox 304")));
    let rows = cut_list(&d);
    assert_eq!(rows.len(), 2, "{rows:?}");
    let aco = rows.iter().find(|r| r.material == "Aço 1020").expect("material da peça");
    assert_eq!(aco.count, 2, "iguais do mesmo material agrupam");
    let inox = rows.iter().find(|r| r.material == "Aço inox 304").expect("material próprio");
    assert_eq!(inox.count, 1);
    assert_ne!(aco.item, inox.item);
}
