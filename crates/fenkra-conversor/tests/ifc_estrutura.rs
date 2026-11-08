// SPDX-License-Identifier: LGPL-2.1-or-later
//! B134: a estrutura de aço sai em IFC4 como `IfcMember` e `IfcPlate`, e o arquivo volta pelo leitor
//! de IFC com o mesmo volume e a mesma caixa do recálculo, o mesmo perfil e os comprimentos da lista
//! de corte.

use fenkra_conversor::malha::ler_para_teste;
use fenkra_conversor::Formato;
use fenkra_core::*;
use fenkra_solidos::st_ifc::ifc_da_estrutura;
use fenkra_solidos::{cut_list, recompute, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }))
}

/// Quadro 600 x 400 de tubo 40x3 com cantos em meia-esquadria, um pilar W200 e uma placa no topo do pilar.
fn estrutura() -> (Document, Vec<Id>) {
    let mut d = Document::new("Quadro de aço");
    let p = [[0., 0., 0.], [600., 0., 0.], [600., 400., 0.], [0., 400., 0.]];
    let c: Vec<CurveRef> = (0..4).map(|i| linha(&mut d, p[i], p[(i + 1) % 4])).collect();
    let base = |paths, family: &str, designation: &str, kind, dims: &[&str]| StructMember {
        paths,
        family: s(family),
        designation: s(designation),
        kind,
        dims: dims.iter().map(|x| s(x)).collect(),
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
        mirror: false,
    };
    let quadro = d.add_feature(FeatureKind::StructMember(base(c, "Tubo quadrado", "40x3", SectionKind::TuboQuadrado, &["40", "3"])));
    let l = linha(&mut d, [1000., 0., 0.], [1000., 0., 800.]);
    let pilar = d.add_feature(FeatureKind::StructMember(base(vec![l], "Perfil W", "W200x15", SectionKind::PerfilI, &["200", "100", "4.3", "5.2"])));
    let placa = d.add_feature(FeatureKind::StructPlate(StructPlate::nova([1000., 0., 800.], "10", "10")));
    d.props.material = s("ASTM A36");
    (d, vec![quadro, pilar, placa])
}

fn caixa(pts: impl Iterator<Item = [f64; 3]>) -> ([f64; 3], [f64; 3]) {
    pts.fold(([f64::MAX; 3], [f64::MIN; 3]), |(mut lo, mut hi), p| {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
        (lo, hi)
    })
}

#[test]
fn estrutura_de_aco_vai_e_volta_em_ifc() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, ids) = estrutura();
    let r = recompute(&d);
    for id in &ids {
        assert_eq!(r.status[id], Status::Ok, "{:?}", r.status[id]);
    }
    let volume: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
    let (lo, hi) = caixa(r.bodies.iter().flat_map(|b| b.mesh.positions.chunks(3).map(|p| [p[0] as f64, p[1] as f64, p[2] as f64]).collect::<Vec<_>>()));

    let ifc = ifc_da_estrutura(&d).unwrap();
    assert_eq!((ifc.membros, ifc.placas, ifc.soldas), (5, 1, 0));
    let t = &ifc.texto;
    assert!(t.contains("FILE_SCHEMA(('IFC4'))"));
    assert_eq!(t.matches("=IFCMEMBER(").count(), 5);
    assert_eq!(t.matches("=IFCPLATE(").count(), 1);
    // os quatro tubos são dois pares iguais: a malha vai uma vez por item
    let itens = cut_list(&d).iter().filter(|c| c.item > 0).count();
    assert_eq!(t.matches("=IFCTRIANGULATEDFACESET(").count(), itens + 1, "um corpo por item, mais a placa");
    // perfil paramétrico e material
    assert!(t.contains("IFCISHAPEPROFILEDEF(.AREA.,'Perfil W W200x15',$,100.,200.,4.3,5.2"), "perfil I");
    assert!(t.contains("IFCRECTANGLEHOLLOWPROFILEDEF(.AREA.,'Tubo quadrado 40x3',$,40.,40.,3."), "tubo");
    assert!(t.contains("IFCMATERIAL('ASTM A36',$,'Steel')"));
    // comprimentos de corte da lista
    for c in cut_list(&d) {
        let q = format!("IFCQUANTITYLENGTH('Length',$,$,{},$)", fmt_real(c.length));
        assert!(t.contains(&q), "falta {q}");
    }

    let dir = std::env::temp_dir().join(format!("fenkra_ifc_est_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let arq = dir.join("quadro.ifc");
    std::fs::write(&arq, t).unwrap();
    let mut cena = ler_para_teste(Formato::Ifc, &arq).unwrap();
    cena.enxugar();
    let pecas = cena.no_lugar();
    assert_eq!(pecas.len(), 6, "cinco membros e a placa");
    let v2: f64 = pecas.iter().map(|m| m.volume()).sum();
    assert!((v2 - volume).abs() / volume < 0.01, "volume {v2} contra {volume}");
    let (lo2, hi2) = caixa(pecas.iter().flat_map(|m| m.pos.clone()));
    for k in 0..3 {
        assert!((lo2[k] - lo[k]).abs() < 0.5 && (hi2[k] - hi[k]).abs() < 0.5, "caixa {lo2:?} {hi2:?} contra {lo:?} {hi:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

fn fmt_real(v: f64) -> String {
    let t = format!("{}", (v * 1e6).round() / 1e6);
    if t.contains('.') {
        t
    } else {
        format!("{t}.")
    }
}

#[test]
fn peca_sem_estrutura_nao_exporta() {
    let d = Document::new("vazia");
    assert!(ifc_da_estrutura(&d).is_err());
}
