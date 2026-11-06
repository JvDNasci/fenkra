// SPDX-License-Identifier: LGPL-2.1-or-later
//! B332: dados de dobra para a máquina (LRA, YBC e XYZ) a partir de um tubo do documento, com ida e
//! volta pelo arquivo de texto. Só contas: o tubo dobrado vem de `tubos_dobrados`.

use fenkra_core::*;
use fenkra_solidos::st_dobra_maquina::{exportar, importar, para_xyz, rota_do_tubo, Formato, Modo, Opcoes};
use fenkra_solidos::tubos_dobrados;

fn s(x: &str) -> String {
    x.to_string()
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }))
}

/// Tubo redondo 25,4 x 1,5 com as pernas dadas, todas as dobras em dobradeira com os raios dados.
fn tubo(pontos: &[[f64; 3]], raios: &[&str], modo: BendMode) -> (Document, Id) {
    let mut d = Document::new("maquina");
    let paths: Vec<CurveRef> = pontos.windows(2).map(|w| linha(&mut d, w[0], w[1])).collect();
    let f = d.add_feature(FeatureKind::StructMember(StructMember {
        paths,
        family: s("Tubo redondo"),
        designation: s("25,4x1,5"),
        kind: SectionKind::TuboRedondo,
        dims: vec![s("25.4"), s("1.5")],
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
    if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|x| &mut x.kind) {
        for (k, r) in raios.iter().enumerate() {
            m.joints.push(StructJoint { mine: k + 1, other: f, other_path: k, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(TubeBend { mode: modo, radius: s(r), ..Default::default() }) });
        }
    }
    (d, f)
}

#[test]
fn u_plano_da_os_valores_conhecidos() {
    // 400 + 300 + 400, dobras de 90 graus com raio de centro 150 e 100, no mesmo lado
    let (d, _) = tubo(&[[0., 0., 0.], [400., 0., 0.], [400., 300., 0.], [0., 300., 0.]], &["150", "100"], BendMode::Dobradeira);
    let t = tubos_dobrados(&d);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].pontos.len(), 4, "início, dois nós e fim");
    let r = rota_do_tubo(&t[0]).unwrap();
    // retos de tangente a tangente: 400-150, 300-150-100, 400-100
    let esperado = [250., 50., 300.];
    for (a, b) in r.retos.iter().zip(esperado) {
        assert!((a - b).abs() < 1e-6, "{:?}", r.retos);
    }
    assert!(r.dobras.iter().all(|b| (b.angulo - 90.).abs() < 1e-6 && b.rotacao.abs() < 1e-6));
    assert!((r.dobras[0].raio - 150.).abs() < 1e-9 && (r.dobras[1].raio - 100.).abs() < 1e-9);
    // o comprimento de corte da lista bate com retos + arcos
    assert!((r.desenvolvido() - t[0].desenvolvido).abs() < 1e-6, "{} x {}", r.desenvolvido(), t[0].desenvolvido);
    // o eixo refeito pela máquina cai nos pontos do documento (a rota começa na origem, em +X)
    let x = para_xyz(&r).unwrap();
    for (a, b) in x.pontos.iter().zip(&t[0].pontos) {
        assert!((a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6 && (a[2] - b[2]).abs() < 1e-6, "{a:?} x {b:?}");
    }
}

#[test]
fn rota_3d_com_rotacao_vai_e_volta_pelos_tres_formatos() {
    // três dobras fora do plano
    let (d, _) = tubo(&[[0., 0., 0.], [300., 0., 0.], [300., 250., 0.], [300., 250., 200.], [300., 450., 200.]], &["60", "60", "60"], BendMode::Dobradeira);
    let t = tubos_dobrados(&d);
    assert_eq!(t.len(), 1);
    let r = rota_do_tubo(&t[0]).unwrap();
    assert_eq!(r.dobras.len(), 3);
    // a segunda dobra sai do plano XY: a rotação não é 0 nem 180
    assert!((r.dobras[1].rotacao.abs() - 90.).abs() < 1e-6, "{:?}", r.dobras);
    for formato in [Formato::Lra, Formato::Ybc, Formato::Xyz] {
        for modo in [Modo::Incremental, Modo::Absoluto] {
            let txt = exportar(&r, formato, Opcoes { modo, ..Default::default() }, &t[0].designacao).unwrap();
            let lido = importar(&txt).unwrap_or_else(|e| panic!("{formato:?}/{modo:?}: {e}\n{txt}"));
            assert_eq!(lido.formato, formato);
            for (a, b) in r.retos.iter().zip(&lido.rota.retos) {
                assert!((a - b).abs() < 1e-5, "{formato:?}/{modo:?}: {:?} x {:?}", r.retos, lido.rota.retos);
            }
            for (a, b) in r.dobras.iter().zip(&lido.rota.dobras) {
                assert!((a.angulo - b.angulo).abs() < 1e-5 && (a.raio - b.raio).abs() < 1e-5);
                assert!((a.rotacao - b.rotacao).abs() < 1e-4, "{formato:?}/{modo:?}: {} x {}", a.rotacao, b.rotacao);
            }
            // os pontos lidos reproduzem o eixo do documento (mesma origem e orientação do XYZ da rota)
            let esperado = para_xyz(&r).unwrap();
            for (a, b) in lido.xyz.pontos.iter().zip(&esperado.pontos) {
                assert!((0..3).all(|k| (a[k] - b[k]).abs() < 1e-4), "{formato:?}/{modo:?}: {a:?} x {b:?}");
            }
        }
    }
}

#[test]
fn retorno_elastico_aparece_no_angulo_de_maquina() {
    let (mut d, f) = tubo(&[[0., 0., 0.], [400., 0., 0.], [400., 300., 0.]], &["100"], BendMode::Dobradeira);
    d.tube_rules.push(TubeRule { name: s("ensaio"), section: Some(SectionKind::TuboRedondo), tool: s("dobradeira"), springback: s("3"), source: s("teste"), ..Default::default() });
    if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|x| &mut x.kind) {
        m.joints[0].bend.as_mut().unwrap().rule = s("ensaio");
    }
    let t = tubos_dobrados(&d);
    let r = rota_do_tubo(&t[0]).unwrap();
    assert!((r.dobras[0].angulo - 90.).abs() < 1e-6);
    assert!((r.dobras[0].angulo_maquina - 93.).abs() < 1e-6, "{:?}", r.dobras[0]);
    let txt = exportar(&r, Formato::Lra, Opcoes { angulo_maquina: true, ..Default::default() }, "t").unwrap();
    assert!(txt.contains("93.000000"), "{txt}");
}

#[test]
fn modo_diferente_de_dobradeira_e_recusado() {
    let (d, _) = tubo(&[[0., 0., 0.], [400., 0., 0.], [400., 300., 0.]], &["50"], BendMode::CorteSimples);
    let t = tubos_dobrados(&d);
    assert_eq!(t.len(), 1);
    let e = rota_do_tubo(&t[0]).unwrap_err();
    assert!(e.contains("dobradeira"), "{e}");
}
