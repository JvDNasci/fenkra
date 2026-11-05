// SPDX-License-Identifier: LGPL-2.1-or-later
//! B226: linhas contíguas do esqueleto formam grupos, e cada grupo vira um membro com perfil próprio.

use fenkra_core::*;
use fenkra_solidos::{recompute, skeleton_groups, Status};

fn membro(paths: Vec<CurveRef>, designation: &str, dims: [&str; 2]) -> StructMember {
    let s = |x: &str| x.to_string();
    StructMember {
        paths,
        family: s("Tubo quadrado"),
        designation: s(designation),
        kind: SectionKind::TuboQuadrado,
        dims: dims.iter().map(|d| s(d)).collect(),
        anchor: 0,
        angle: s("0"),
        off_x: s("0"),
        off_y: s("0"),
        ext1: s("0"),
        ext2: s("0"),
        corner: CornerMode::MeiaEsquadria,
        gap: s("0"),
        joints: vec![],
        material: String::new(),
    }
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: "0".into(), end: "0".into() }))
}

#[test]
fn esqueleto_em_l_mais_barra_solta_vira_dois_grupos_com_perfis_proprios() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("grupos");
    let a = linha(&mut d, [0., 0., 0.], [300., 0., 0.]);
    let solta = linha(&mut d, [0., 500., 0.], [200., 500., 0.]);
    let b = linha(&mut d, [300., 0., 0.], [300., 300., 0.]);
    let paths = vec![a, solta, b];
    // a ordem do clique é preservada dentro de cada grupo; os grupos saem pela primeira linha
    let grupos = skeleton_groups(&d, &paths);
    assert_eq!(grupos, vec![vec![0, 2], vec![1]]);

    let g1 = d.add_feature(FeatureKind::StructMember(membro(grupos[0].iter().map(|i| paths[*i].clone()).collect(), "40x3", ["40", "3"])));
    let g2 = d.add_feature(FeatureKind::StructMember(membro(grupos[1].iter().map(|i| paths[*i].clone()).collect(), "50x3", ["50", "3"])));
    let r = recompute(&d);
    assert_eq!(r.status[&g1], Status::Ok, "{:?}", r.status[&g1]);
    assert_eq!(r.status[&g2], Status::Ok, "{:?}", r.status[&g2]);
    // meia-esquadria conserva o volume da barra reta
    let esperado = (40. * 40. - 34. * 34.) * 600. + (50. * 50. - 44. * 44.) * 200.;
    let volume: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
    assert!((volume - esperado).abs() < 1e-3 * esperado, "volume {volume} vs {esperado}");
    // caixa envolvente: o canto do L passa meia seção da linha de centro; a barra solta tem 50 de seção
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for b in &r.bodies {
        for p in b.mesh.positions.chunks(3) {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
    }
    let quase = |x: f32, y: f64| (x as f64 - y).abs() < 0.5;
    assert!(quase(lo[0], 0.) && quase(hi[0], 320.), "x {:?}", (lo[0], hi[0]));
    assert!(quase(lo[1], -20.) && quase(hi[1], 525.), "y {:?}", (lo[1], hi[1]));
    assert!(quase(lo[2], -25.) && quase(hi[2], 25.), "z {:?}", (lo[2], hi[2]));
}

#[test]
fn sem_linhas_nao_ha_grupos() {
    let d = Document::new("vazio");
    assert!(skeleton_groups(&d, &[]).is_empty());
}
