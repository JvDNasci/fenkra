// SPDX-License-Identifier: LGPL-2.1-or-later
//! B329 e B334: o conteúdo real dos arquivos de gabarito e de contorno de corte de tubo (PDF, SVG,
//! DXF) gerados a partir de tubos dobrados do documento, não de desenhos soltos. Com
//! `FENKRA_SAIDA_DIR=<pasta>` os arquivos também são gravados para conferência a olho.

use fenkra_core::*;
use fenkra_solidos::st_dobra_corte::{gabarito, planificado};
use fenkra_solidos::st_dobra_saida::{dxf, folhas, pdf, svg};
use fenkra_solidos::tubos_dobrados;

fn s(x: &str) -> String {
    x.to_string()
}

fn l_de_tubo(dims: &[&str], b: TubeBend) -> Document {
    let mut d = Document::new("saidas");
    let mut paths = vec![];
    for (a, c) in [([0., 0., 0.], [300., 0., 0.]), ([300., 0., 0.], [300., 250., 0.])] {
        let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(c), start: s("0"), end: s("0") });
        paths.push(CurveRef::Ref(l));
    }
    let f = d.add_feature(FeatureKind::StructMember(StructMember {
        paths,
        family: s("Tubo quadrado"),
        designation: s("30x1.2"),
        kind: SectionKind::TuboQuadrado,
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
    }));
    if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|f| &mut f.kind) {
        m.joints.push(StructJoint { mine: 1, other: f, other_path: 0, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(b) });
    }
    d
}

fn grava(nome: &str, bytes: &[u8]) {
    if let Ok(pasta) = std::env::var("FENKRA_SAIDA_DIR") {
        let _ = std::fs::create_dir_all(&pasta);
        std::fs::write(std::path::Path::new(&pasta).join(nome), bytes).unwrap();
    }
}

fn conta(texto: &str, trecho: &str) -> usize {
    texto.matches(trecho).count()
}

#[test]
fn v_multiplo_gera_gabarito_e_contorno_com_um_corte_por_rasgo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let b = TubeBend { mode: BendMode::VComum, radius: s("40"), cuts: s("3"), ..Default::default() };
    let doc = l_de_tubo(&["30", "1.2"], b);
    let tubo = &tubos_dobrados(&doc)[0];
    assert!(tubo.erros.is_empty(), "{:?}", tubo.erros);
    let r = &tubo.dobras[0].resultado;
    assert_eq!(r.rasgos, 3);
    // contorno de corte (B334): um hexágono fechado por rasgo, na posição do passo
    let plano = planificado(tubo).unwrap();
    let cortes: Vec<_> = plano.tracos.iter().filter(|t| t.camada == fenkra_solidos::st_dobra_corte::Camada::Corte).collect();
    assert_eq!(cortes.len(), 3);
    let passo = r.passo_v.unwrap();
    let xs: Vec<f64> = cortes.iter().map(|t| t.pts.iter().map(|p| p[0]).sum::<f64>() / t.pts.len() as f64).collect();
    assert!((xs[1] - xs[0] - passo).abs() < 1e-6 && (xs[2] - xs[1] - passo).abs() < 1e-6, "{xs:?} passo {passo}");
    let sv = svg(&plano);
    grava("v_multiplo_contorno.svg", sv.as_bytes());
    assert_eq!(conta(&sv, "<polygon"), 3 + 1, "três cortes e o contorno do tubo");
    assert!(sv.contains(r#"id="CORTE""#) && sv.contains(r#"id="MARCACAO""#) && sv.contains("V1 x="));
    let dx = dxf(&plano);
    grava("v_multiplo_contorno.dxf", dx.as_bytes());
    // cada hexágono dá 6 linhas; o contorno do tubo, 4; mais as arestas e as marcas
    assert!(conta(&dx, "\nLINE\n") >= 3 * 6 + 4 + 3, "{}", conta(&dx, "\nLINE\n"));
    assert!(dx.contains("\n8\nCORTE\n") && dx.contains("\n8\nMARCACAO\n") && dx.contains("\n8\nDOBRA\n"));
    // gabarito (B329): PDF com a posição de cada rasgo e a barra de 100 mm
    let g = gabarito(tubo).unwrap();
    let p = pdf(&g);
    grava("v_multiplo_gabarito.pdf", &p);
    grava("v_multiplo_gabarito.svg", svg(&g).as_bytes());
    let txt = String::from_utf8_lossy(&p);
    assert!(txt.starts_with("%PDF-1.4"));
    for j in 1..=3 {
        assert!(txt.contains(&format!("V{j} ")), "rasgo {j} no PDF");
    }
    assert!(txt.contains("100 mm"), "barra de conferência da escala");
    let (c, l, _) = folhas(&g);
    assert!(c >= 1 && l >= 1);
    // o SVG está em milímetros reais: a barra de 100 mm tem 100 unidades de largura
    assert!(svg(&g).contains("mm\""));
}

#[test]
fn canto_com_gabarito_gera_a_andorinha_nos_tres_formatos() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let b = TubeBend { mode: BendMode::VComRaio, radius: s("30"), ..Default::default() };
    let doc = l_de_tubo(&["30", "1.2"], b);
    let tubo = &tubos_dobrados(&doc)[0];
    assert!(tubo.erros.is_empty(), "{:?}", tubo.erros);
    let r = &tubo.dobras[0].resultado;
    // caso de aceite da pesquisa: 30x30x1,2 a 90 graus, Ro = 30, placa 46,17
    assert!((r.placa.unwrap() - 46.17).abs() < 0.01, "{:?}", r.placa);
    let plano = planificado(tubo).unwrap();
    let sv = svg(&plano);
    grava("canto_contorno.svg", sv.as_bytes());
    // um contorno fechado de corte (a andorinha das duas laterais) e o do tubo
    assert_eq!(conta(&sv, "<polygon"), 2, "{sv}");
    let dx = dxf(&plano);
    grava("canto_contorno.dxf", dx.as_bytes());
    assert!(conta(&dx, "\nLINE\n") > 40, "arcos discretizados em segmentos: {}", conta(&dx, "\nLINE\n"));
    let g = gabarito(tubo).unwrap();
    let p = pdf(&g);
    grava("canto_gabarito.pdf", &p);
    grava("canto_gabarito.svg", svg(&g).as_bytes());
    let txt = String::from_utf8_lossy(&p);
    assert!(txt.contains("46.17") || txt.contains("46,17"), "placa plana no PDF");
    assert!(txt.contains("canto com gabarito"));
}
