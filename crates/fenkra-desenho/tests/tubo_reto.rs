// SPDX-License-Identifier: LGPL-2.1-or-later
//! B331: o desenho de um tubo dobrado ganha a vista do tubo reto (a "planificada" do tubo) com as
//! marcas de início e fim de cada dobra e dos rasgos, no mesmo comprimento da lista de corte.

use fenkra_core::*;
use fenkra_desenho::source::Models;
use fenkra_desenho::{Drawing, Format, Source, ViewKind};

fn s(x: &str) -> String {
    x.to_string()
}

fn tubo_em_u() -> Document {
    let mut d = Document::new("Tubo em U");
    let mut paths = vec![];
    for (a, b) in [([0., 0., 0.], [400., 0., 0.]), ([400., 0., 0.], [400., 300., 0.])] {
        let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") });
        paths.push(CurveRef::Ref(l));
    }
    let f = d.add_feature(FeatureKind::StructMember(StructMember {
        paths,
        family: s("Tubo quadrado"),
        designation: s("30x1.2"),
        kind: SectionKind::TuboQuadrado,
        dims: vec![s("30"), s("1.2")],
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
    }));
    if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|f| &mut f.kind) {
        let bend = TubeBend { mode: BendMode::VComum, radius: s("40"), cuts: s("3"), ..Default::default() };
        m.joints.push(StructJoint { mine: 1, other: f, other_path: 0, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(bend) });
    }
    d
}

#[test]
fn a_planificada_de_um_tubo_dobrado_e_o_tubo_reto_com_as_marcas() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::env::temp_dir().join("fenkra_desenho_tubo_reto");
    std::fs::create_dir_all(&dir).unwrap();
    let doc = tubo_em_u();
    std::fs::write(dir.join("tubo.fkp"), doc.to_json()).unwrap();
    let t = fenkra_solidos::tubos_dobrados(&doc);
    assert_eq!(t.len(), 1);
    let src = Source { path: "tubo.fkp".into(), display_state: String::new(), hidden: vec![] };
    let mut d = Drawing::new("Tubo", Format::A3);
    let sh = d.sheets[0].id;
    let v = d.add_view(sh, src, ViewKind::Flat, [200., 150.], 0.5);
    let mut m = Models::new(Some(dir));
    let p = m.proj(&d, v);
    assert!(p.error.is_none(), "{:?}", p.error);
    // comprimento do contorno = desenvolvido da lista de corte
    assert!((p.hi[0] - p.lo[0] - t[0].desenvolvido).abs() < 1.0, "{:?} {:?} contra {}", p.lo, p.hi, t[0].desenvolvido);
    // marcas de início e fim da dobra (tracejadas) e um rótulo da dobra
    let marcas = p.lines.iter().filter(|l| l.1 == 3 && l.0.len() == 2 && (l.0[0][0] - l.0[1][0]).abs() < 1e-9).count();
    assert!(marcas >= 2, "{marcas}");
    assert!(p.labels.iter().any(|l| l.1.starts_with("D1 90°")), "{:?}", p.labels);
    // três rasgos V: três triângulos de corte (linhas visíveis de três pontos além do contorno)
    assert!(p.labels.iter().any(|l| l.1.contains("V1 x=") && l.1.contains("V3 x=")), "{:?}", p.labels);
    assert!(["V1", "V2", "V3"].iter().all(|n| p.labels.iter().any(|l| l.1 == *n)), "{:?}", p.labels);
}
