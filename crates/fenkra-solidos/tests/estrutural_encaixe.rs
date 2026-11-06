// SPDX-License-Identifier: LGPL-2.1-or-later
//! B155: corte de encaixe lido do documento e conferido contra o sólido do kernel. Documentos
//! montados em código; confere a curva, o comprimento de corte e os arquivos de saída.

use fenkra_core::*;
use fenkra_solidos::st_dobra_saida::{dxf, svg};
use fenkra_solidos::st_encaixe::{ModoParede, Opcoes};
use fenkra_solidos::st_encaixe_doc::encaixes_da_estrutura;
use fenkra_solidos::structural_members;

fn s(x: &str) -> String {
    x.to_string()
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }))
}

/// L de duas linhas no mesmo comando: a linha 1 (host) sobe em Z, a linha 2 sai em X; o canto é de
/// encaixe, então a linha 2 é cortada contra a 1.
fn ele(kind: SectionKind, dims: &[&str], angulo_graus: f64, folga: &str) -> (Document, Id) {
    let mut d = Document::new("encaixe");
    let l1 = linha(&mut d, [0., 0., 0.], [0., 0., 500.]);
    let a = angulo_graus.to_radians();
    let l2 = linha(&mut d, [0., 0., 0.], [400. * a.sin(), 0., 400. * a.cos()]);
    let f = d.add_feature(FeatureKind::StructMember(StructMember {
        paths: vec![l1, l2],
        family: s("Tubo"),
        designation: s("teste"),
        kind,
        dims: dims.iter().map(|x| s(x)).collect(),
        anchor: 0,
        angle: s("0"),
        off_x: s("0"),
        off_y: s("0"),
        ext1: s("0"),
        ext2: s("0"),
        corner: CornerMode::Encaixe,
        gap: s(folga),
        joints: vec![],
        material: String::new(),
        mirror: false,
    }));
    (d, f)
}

#[test]
fn retangular_sobre_retangular_toca_a_face_do_hospedeiro() {
    // 40x40x3 encostando de lado no 40x40x3: o corte é uma reta a 20 mm do eixo (meia seção)
    let (d, f) = ele(SectionKind::TuboQuadrado, &["40", "3"], 90., "0");
    let r = encaixes_da_estrutura(&d, &Opcoes::default());
    assert_eq!(r.len(), 1);
    let m = &r[0];
    assert_eq!((m.feature, m.linha), (f, 1));
    let p = m.ponta1.as_ref().expect("a linha 2 começa no canto");
    assert!(m.ponta2.is_none());
    assert_eq!(p.hospedeiros, vec![(f, 0)]);
    assert!((p.corte.lambda_min() - 20.).abs() < 1e-6 && (p.corte.lambda_max() - 20.).abs() < 1e-6);
    // o sólido do kernel começa no mesmo ponto: a face de fora do hospedeiro
    let sol = structural_members(&d);
    let b = sol.iter().find(|x| x.feature == f && x.index == 1).expect("sólido da linha 2");
    assert!((b.signature.bbox[2] as f64 / 10. - 20.).abs() < 0.1, "bbox {:?}", b.signature.bbox);
}

#[test]
fn redondo_com_chanfro_bate_com_o_solido_do_kernel() {
    // sem folga, o corte com chanfro é a curva que o kernel faz: o ponto mais próximo do nó bate
    let (d, f) = ele(SectionKind::TuboRedondo, &["33.7", "2.6"], 90., "0");
    let opc = Opcoes { perpendicular: false, ..Default::default() };
    // o tubo redondo é o membro 2; o hospedeiro é redondo de 33,7 também: usa o mesmo perfil
    let r = encaixes_da_estrutura(&d, &opc);
    let p = r[0].ponta1.as_ref().unwrap();
    let minimo = p.corte.pontos.iter().map(|q| q.lambda_ext.min(q.lambda_int)).fold(f64::MAX, f64::min);
    let sol = structural_members(&d);
    let b = sol.iter().find(|x| x.feature == f && x.index == 1).unwrap();
    let z0 = b.signature.bbox[2] as f64 / 10.;
    assert!((z0 - minimo).abs() < 0.15, "kernel {z0} curva {minimo}");
}

#[test]
fn folga_de_solda_do_canto_entra_na_curva() {
    let (d, _) = ele(SectionKind::TuboQuadrado, &["40", "3"], 90., "2");
    let r = encaixes_da_estrutura(&d, &Opcoes::default());
    assert!((r[0].ponta1.as_ref().unwrap().corte.lambda_min() - 22.).abs() < 1e-6);
}

#[test]
fn contorno_sai_no_dxf_e_no_svg_com_o_comprimento_do_relatorio() {
    let (d, _) = ele(SectionKind::TuboRedondo, &["60.3", "3.6"], 60., "0");
    let opc = Opcoes { modo: ModoParede::Automatico, orcamento: 0.5, ..Default::default() };
    let r = encaixes_da_estrutura(&d, &opc);
    let m = &r[0];
    let plano = m.desenho();
    // o desenho tem o corte do relatório mais o último trecho até a costura
    assert!(plano.comprimento_de_corte() >= m.comprimento_de_corte());
    assert!(plano.comprimento_de_corte() - m.comprimento_de_corte() < 5.);
    let (x, v) = (dxf(&plano), svg(&plano));
    assert!(x.contains("CORTE") && x.contains("LWPOLYLINE") || x.contains("POLYLINE") || x.contains("LINE"));
    assert!(v.contains("CORTE") && v.contains("mm"));
    // o 3D tem o mesmo comprimento de corte que o desenho (a menos do fecho da costura)
    let c = &m.ponta1.as_ref().unwrap().corte;
    assert!((c.comprimento_3d() - c.comprimento_corte()).abs() / c.comprimento_corte() < 0.01);
}

#[test]
fn sem_encaixe_nao_ha_curva() {
    let (mut d, f) = ele(SectionKind::TuboQuadrado, &["40", "3"], 90., "0");
    if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|f| &mut f.kind) {
        m.corner = CornerMode::MeiaEsquadria;
    }
    assert!(encaixes_da_estrutura(&d, &Opcoes::default()).is_empty());
}
