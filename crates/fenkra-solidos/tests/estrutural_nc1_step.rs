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
