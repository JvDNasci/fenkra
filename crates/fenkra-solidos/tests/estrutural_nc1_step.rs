// SPDX-License-Identifier: LGPL-2.1-or-later
//! B176: cada item da lista de corte sai em DSTV NC1 (cabeçalho com perfil, comprimento e cortes
//! inclinados) e em STEP do sólido cortado, que volta com o mesmo volume e a mesma caixa.

use fenkra_core::*;
use fenkra_solidos::st_dstv::*;
use fenkra_solidos::{cut_list, recompute, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }))
}

fn membro(paths: Vec<CurveRef>, fam: &str, des: &str, kind: SectionKind, dims: &[&str], corner: CornerMode) -> StructMember {
    StructMember {
        paths,
        family: s(fam),
        designation: s(des),
        kind,
        dims: dims.iter().map(|x| s(x)).collect(),
        anchor: 0,
        angle: s("0"),
        off_x: s("0"),
        off_y: s("0"),
        ext1: s("0"),
        ext2: s("0"),
        corner,
        gap: s("0"),
        joints: vec![],
        material: String::new(),
        mirror: false,
    }
}

/// L de tubo 80x40x3 com canto em meia-esquadria (dois trechos iguais de 600) e uma viga W200 reta de 1000.
fn estrutura() -> Document {
    let mut d = Document::new("Pórtico");
    let a = linha(&mut d, [0., 0., 0.], [600., 0., 0.]);
    let b = linha(&mut d, [600., 0., 0.], [600., 600., 0.]);
    d.add_feature(FeatureKind::StructMember(membro(vec![a, b], "Tubo retangular", "80x40x3", SectionKind::TuboRetangular, &["80", "40", "3"], CornerMode::MeiaEsquadria)));
    let v = linha(&mut d, [0., 1000., 0.], [1000., 1000., 0.]);
    d.add_feature(FeatureKind::StructMember(membro(vec![v], "Perfil W", "W200x15", SectionKind::PerfilI, &["200", "100", "4.3", "5.2"], CornerMode::Nenhum)));
    d.props.material = s("ASTM A36");
    d
}

#[test]
fn itens_saem_em_nc1_e_step() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let d = estrutura();
    let r = recompute(&d);
    assert!(r.status.values().all(|s| *s == Status::Ok), "{:?}", r.status);
    let pecas = pecas_nc(&d);
    let tubos: Vec<&PecaNc> = pecas.iter().filter(|p| p.kind == SectionKind::TuboRetangular).collect();
    let viga = pecas.iter().find(|p| p.kind == SectionKind::PerfilI).unwrap();
    // os dois trechos do L (a esquadria fica numa ponta de um e na outra do outro)
    assert_eq!(tubos.iter().map(|t| t.quantidade).sum::<usize>(), 2);
    let lista = cut_list(&d);
    for tubo in &tubos {
        let ct = lista.iter().find(|c| c.item == tubo.item).unwrap();
        assert!((tubo.comprimento - ct.length).abs() < 1e-9);
        assert_eq!(ct.count, tubo.quantidade);
        // meia-esquadria num canto de 90°: 45° numa ponta, reto na outra
        let inclinados: Vec<f64> = tubo.cortes.iter().copied().filter(|a| a.abs() > 1e-6).collect();
        assert_eq!(inclinados.len(), 1, "{:?}", tubo.cortes);
        assert!((inclinados[0].abs() - 45.).abs() < 0.01, "{:?}", tubo.cortes);
        assert_eq!(nc1(tubo, "p", "p").lines().nth(9).map(str::trim), Some("M"));
    }
    assert!(viga.cortes.iter().all(|a| a.abs() < 1e-6), "{:?}", viga.cortes);
    // massa por metro da W200 com as cotas do modelo (sem raios de concordância): área × 7,85
    let area = 2. * 100. * 5.2 + (200. - 2. * 5.2) * 4.3;
    assert!((viga.kg_m - area * 7.85e-3).abs() < 0.01, "{}", viga.kg_m);
    // pintura: perímetro externo do I
    let perim = 2. * 200. + 4. * 100. - 2. * 4.3;
    assert!((viga.pintura_m - perim / 1000.).abs() < 1e-3, "{}", viga.pintura_m);

    let t = nc1(viga, "Pórtico", "Pórtico");
    let l: Vec<&str> = t.lines().collect();
    assert_eq!(l[0], "ST");
    // 24 linhas de cabeçalho depois do ST e do comentário: código do perfil na 8ª, comprimento na 9ª
    assert_eq!(l[2 + 7].trim(), "I");
    assert_eq!(l[2 + 8].trim(), "1000.00");
    assert_eq!(l[2 + 9].trim(), "200.00");
    assert_eq!(l[2 + 4].trim(), "ASTM_A36");
    assert_eq!(l.last().copied(), Some("EN"));

    let dir = std::env::temp_dir().join(format!("fenkra_nc1_{}", std::process::id()));
    let arqs = gravar_saidas(&d, &dir).unwrap();
    assert_eq!(arqs.len(), 2 * pecas.len());
    for p in &pecas {
        let st = dir.join(nome_do_arquivo(p, "step"));
        let volta = fenkra_kernel::Shape::import_step(&st.to_string_lossy()).unwrap();
        let (v0, v1) = (p.forma.mass().volume, volta.mass().volume);
        assert!((v0 - v1).abs() / v0 < 1e-6, "volume {v0} {v1}");
        let (a, b) = (p.forma.bbox().unwrap(), volta.bbox().unwrap());
        for k in 0..3 {
            assert!((a.0[k] - b.0[k]).abs() < 0.01 && (a.1[k] - b.1[k]).abs() < 0.01, "caixa {a:?} {b:?}");
        }
        // Z ao longo da barra, a partir da ponta (a esquadria pode passar do nó em até meia largura)
        assert!(b.0[2] > -45. && b.0[2] < 1. && b.1[2] > 590., "{b:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Travessa de tubo quadrado 40x40x3 encaixada em dois montantes redondos Ø60,3 (sela nas duas
/// pontas): o NC1 leva o contorno AK das quatro faces.
#[test]
fn encaixe_de_tubo_quadrado_vira_contorno_ak() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("Sela");
    let m1 = linha(&mut d, [0., 0., 0.], [0., 0., 600.]);
    let m2 = linha(&mut d, [800., 0., 0.], [800., 0., 600.]);
    d.add_feature(FeatureKind::StructMember(membro(vec![m1, m2], "Tubo redondo", "60,3x3,6", SectionKind::TuboRedondo, &["60.3", "3.6"], CornerMode::Nenhum)));
    let t = linha(&mut d, [0., 0., 300.], [800., 0., 300.]);
    d.add_feature(FeatureKind::StructMember(membro(vec![t], "Tubo quadrado", "40x3", SectionKind::TuboQuadrado, &["40", "3"], CornerMode::Encaixe)));
    let r = recompute(&d);
    assert!(r.status.values().all(|s| *s == Status::Ok), "{:?}", r.status);
    let pecas = pecas_nc(&d);
    let tr = pecas.iter().find(|p| p.kind == SectionKind::TuboQuadrado).expect("travessa");
    // caixa do sólido cortado: 40 x 40 de seção e comprimento entre as selas
    let (lo, hi) = tr.forma.bbox().unwrap();
    // (a caixa do OpenCASCADE vem folgada pela tolerância, até uns 0,1 mm por lado)
    assert!((hi[0] - lo[0] - 40.).abs() < 0.25 && (hi[1] - lo[1] - 40.).abs() < 0.25, "{lo:?} {hi:?}");
    let comp = hi[2] - lo[2] - 0.2;
    // nó a nó 800; cada ponta entra no montante entre √(R² − 20²) = 22,56 e R = 30,15
    assert!(comp > 800. - 2. * 30.15 - 0.25 && comp < 800. - 2. * 22.56 + 0.25, "{comp}");
    // volume: entre a seção vezes o trecho mais curto e a seção vezes a caixa
    let area = 40. * 40. - 34. * 34.;
    let vol = tr.forma.mass().volume;
    assert!(vol < area * comp && vol > area * (800. - 2. * 30.15) * 0.98, "{vol}");
    assert_eq!(tr.contornos.len(), 4, "{:?}", tr.faltas);
    assert!(tr.faltas.iter().all(|f| !f.contains("AK")), "{:?}", tr.faltas);
    let mut variacao = vec![];
    for (face, pts) in &tr.contornos {
        // fechado, anti-horário, na largura da face e no comprimento da peça
        assert_eq!(pts.first(), pts.last(), "face {face}");
        let area2: f64 = pts.windows(2).map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1]).sum();
        assert!(area2 > 0., "face {face}: sentido horário");
        assert!(pts.iter().all(|q| q[1] >= -0.01 && q[1] <= 40.01 && q[0] >= -0.01 && q[0] <= comp + 0.5), "{face} {pts:?}");
        // ponta do início: os pontos da metade de x menor
        let (a, b) = pts.iter().filter(|q| q[0] < comp / 2.).fold((f64::MAX, f64::MIN), |(a, b), q| (a.min(q[0]), b.max(q[0])));
        variacao.push(b - a);
    }
    // duas faces cruzam o montante de lado (a sela fica nelas): a ponta varia de R menos 22,56 a
    // 24,9; nas outras duas, paralelas ao eixo do montante, a ponta é reta
    variacao.sort_by(f64::total_cmp);
    assert!(variacao[0] < 0.05 && variacao[1] < 0.05, "{variacao:?}");
    assert!(variacao[2] > 5.2 && variacao[3] < 7.65, "{variacao:?}");
    let t = nc1(tr, "p", "p");
    assert!(t.contains("\nAK\n") && t.contains("  v ") && t.contains("  o ") && t.contains("  u ") && t.contains("  h "), "{t}");
    let ak = t.lines().skip_while(|l| *l != "AK").skip(1).take_while(|l| l.starts_with("  ")).count();
    assert_eq!(ak, tr.contornos.iter().map(|c| c.1.len()).sum::<usize>());
}

/// Viga W200 que chega a uma viga principal W310 com as faces de cima alinhadas (âncora no meio de
/// cima): o encaixe recorta a mesa de cima da W200 (entalhe) e o NC1 leva o AK das três faces.
#[test]
fn entalhe_de_perfil_i_vira_contorno_ak() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("Entalhe");
    let p = linha(&mut d, [0., 0., 0.], [1000., 0., 0.]);
    let mut principal = membro(vec![p], "Perfil W", "W310", SectionKind::PerfilI, &["310", "165", "5.8", "9.7"], CornerMode::Nenhum);
    principal.anchor = 2;
    d.add_feature(FeatureKind::StructMember(principal));
    let v = linha(&mut d, [500., 0., 0.], [500., 800., 0.]);
    let mut viga = membro(vec![v], "Perfil W", "W200x15", SectionKind::PerfilI, &["200", "100", "4.3", "5.2"], CornerMode::Encaixe);
    viga.anchor = 2;
    d.add_feature(FeatureKind::StructMember(viga));
    let r = recompute(&d);
    assert!(r.status.values().all(|s| *s == Status::Ok), "{:?}", r.status);
    let pecas = pecas_nc(&d);
    let v = pecas.iter().find(|p| p.designacao == "W200x15").expect("viga");
    // a viga para na face da alma da principal (tw/2 = 2,9) e a mesa de cima dela, que bate na mesa
    // da principal (tf = 9,7), recua até a borda dessa mesa (b/2 = 82,5): entalhe de 79,6 × 9,7
    let area = 2. * 100. * 5.2 + (200. - 2. * 5.2) * 4.3;
    let tirado = (82.5 - 2.9) * (100. * 5.2 + (9.7 - 5.2) * 4.3);
    let vol = area * (800. - 2.9) - tirado;
    assert!((v.forma.mass().volume - vol).abs() / vol < 1e-4, "{} {vol}", v.forma.mass().volume);
    assert!(v.faltas.is_empty(), "{:?}", v.faltas);
    let face = |c: char| v.contornos.iter().find(|x| x.0 == c).map(|x| x.1.clone()).unwrap_or_else(|| panic!("face {c}: {:?}", v.contornos));
    let perto = |a: &[[f64; 2]], b: &[[f64; 2]]| a.len() == b.len() && a.iter().zip(b).all(|(p, q)| (p[0] - q[0]).abs() < 0.03 && (p[1] - q[1]).abs() < 0.03);
    let c = 800. - 2.9;
    let e = 82.5 - 2.9;
    // alma (vista de lado): o degrau do entalhe no canto de cima do início
    let alma = face('v');
    let esperado = [[0., 0.], [c, 0.], [c, 200.], [e, 200.], [e, 200. - 9.7], [0., 200. - 9.7], [0., 0.]];
    assert!(perto(&alma, &esperado), "{alma:?}");
    // mesa de cima: começa recuada; mesa de baixo: inteira
    assert!(perto(&face('o'), &[[e, 0.], [c, 0.], [c, 100.], [e, 100.], [e, 0.]]), "{:?}", face('o'));
    assert!(perto(&face('u'), &[[0., 0.], [c, 0.], [c, 100.], [0., 100.], [0., 0.]]), "{:?}", face('u'));
    let t = nc1(v, "p", "p");
    let ak = t.lines().skip_while(|l| *l != "AK").skip(1).take_while(|l| l.starts_with("  ")).count();
    assert_eq!(ak, 7 + 5 + 5, "{t}");
    assert!(t.contains("  v       0.00u       0.00"), "{t}");
}
