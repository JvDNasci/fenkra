// SPDX-License-Identifier: LGPL-2.1-or-later
//! B211: esboço em DXF e SVG, conferido pelo sólido que ele gera depois da volta, e IFC facetado.

use fenkra_conversor::{Destino, Formato, Importado, InstanciaNeutra, ModeloNeutro, OpcoesExportar, OpcoesImportar, ProdutoNeutro, Registro, UnidadeSaida};
use fenkra_core::sketch::{Curve, SketchData};
use fenkra_core::{BoolOp, Document, Extent, Extrude, FeatureKind, Id, OriginPlane, PlaneRef, Sketch};
use fenkra_kernel::{ModeloXde, Shape, IDENTIDADE};
use std::f64::consts::PI;
use std::path::{Path, PathBuf};

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra plano açaí {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Placa 100 x 50 com canto de raio 10 e furo de raio 8, fora da origem e com Y negativo.
fn documento() -> Document {
    let mut d = Document::new("Placa");
    let mut sk = SketchData::default();
    let mut al = || d.alloc();
    let (x, y) = (-20., -30.);
    let p = |sk: &mut SketchData, al: &mut dyn FnMut() -> Id, a: f64, b: f64| {
        let mut f = || al();
        sk.add_point(&mut f, x + a, y + b)
    };
    let (a, b, c, e, f) = (p(&mut sk, &mut al, 0., 0.), p(&mut sk, &mut al, 100., 0.), p(&mut sk, &mut al, 100., 40.), p(&mut sk, &mut al, 90., 50.), p(&mut sk, &mut al, 0., 50.));
    let centro_canto = p(&mut sk, &mut al, 90., 40.);
    let centro_furo = p(&mut sk, &mut al, 40., 25.);
    for (s, t) in [(a, b), (b, c), (e, f), (f, a)] {
        sk.add_curve(&mut al, Curve::Line { a: s, b: t }, false);
    }
    sk.add_curve(&mut al, Curve::Arc { c: centro_canto, s: c, e }, false);
    sk.add_curve(&mut al, Curve::Circle { c: centro_furo, r: 8. }, false);
    let id = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: sk }));
    if let Some(q) = d.features.iter_mut().find(|q| q.id == id) {
        q.name = "Contorno".into();
    }
    d
}

/// Área da placa: retângulo, menos o canto arredondado, menos o furo.
fn area() -> f64 {
    100. * 50. - (100. - PI * 100. / 4.) - PI * 64.
}

fn extrudar(doc: &Document) -> (f64, [f64; 3], [f64; 3]) {
    let sk = doc.features.iter().find(|f| matches!(f.kind, FeatureKind::Sketch(_))).unwrap().id;
    let mut d = doc.clone();
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let r = fenkra_solidos::recompute(&d);
    assert!(!r.status.values().any(|s| matches!(s, fenkra_solidos::Status::Error(_))), "{:?}", r.status);
    assert_eq!(r.bodies.len(), 1);
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for q in r.bodies[0].mesh.positions.chunks_exact(3) {
        for i in 0..3 {
            lo[i] = lo[i].min(f64::from(q[i]));
            hi[i] = hi[i].max(f64::from(q[i]));
        }
    }
    (r.bodies[0].mass.volume, lo, hi)
}

fn ida_e_volta(dir: &Path, f: Formato) -> Importado {
    let a = dir.join(format!("placa ç.{}", f.extensoes()[0]));
    let r = Registro::default();
    r.gravar_documento(&documento(), &a, &OpcoesExportar::default()).unwrap();
    r.importar(&a, &OpcoesImportar::default()).unwrap()
}

#[test]
fn esboco_volta_e_gera_o_mesmo_solido_no_mesmo_lugar() {
    let dir = pasta("solido");
    for f in [Formato::Dxf, Formato::Svg] {
        let i = ida_e_volta(&dir, f);
        assert_eq!(i.relatorio.formato, f.nome());
        assert!(i.relatorio.avisos.is_empty(), "{:?}", i.relatorio.avisos);
        assert!(i.doc.features.iter().any(|q| q.name == "Contorno"), "{}: a camada vira o nome do esboço", f.nome());
        let (v, lo, hi) = extrudar(&i.doc);
        assert!((v - area() * 10.).abs() < 1e-3, "{}: {v}", f.nome());
        assert!((lo[0] + 20.).abs() < 1e-3 && (lo[1] + 30.).abs() < 1e-3, "{}: {lo:?}", f.nome());
        assert!((hi[0] - 80.).abs() < 1e-3 && (hi[1] - 20.).abs() < 1e-3, "{}: {hi:?}", f.nome());
    }
}

#[test]
fn circulo_e_arco_voltam_exatos_e_nao_em_pedacos() {
    let dir = pasta("exato");
    for f in [Formato::Dxf, Formato::Svg] {
        let i = ida_e_volta(&dir, f);
        let FeatureKind::Sketch(s) = &i.doc.features.iter().find(|q| q.name == "Contorno").unwrap().kind else { panic!() };
        let conta = |t: fn(&Curve) -> bool| s.data.curves.iter().filter(|c| t(&c.geom)).count();
        assert_eq!(conta(|c| matches!(c, Curve::Circle { .. })), 1, "{}", f.nome());
        assert_eq!(conta(|c| matches!(c, Curve::Arc { .. })), 1, "{}", f.nome());
        assert_eq!(conta(|c| matches!(c, Curve::Line { .. })), 4, "{}", f.nome());
    }
}

#[test]
fn construcao_fica_de_fora_e_documento_sem_esboco_explica() {
    let dir = pasta("construcao");
    let mut d = documento();
    let FeatureKind::Sketch(s) = &mut d.features.iter_mut().find(|q| q.name == "Contorno").unwrap().kind else { panic!() };
    let n = s.data.curves.len();
    for c in &mut s.data.curves {
        c.construction = true;
    }
    assert!(n > 0);
    let e = Registro::default().gravar_documento(&d, &dir.join("a.svg"), &OpcoesExportar::default()).unwrap_err();
    assert!(e.to_string().contains("esboço"), "{e}");
}

#[test]
fn dxf_e_svg_convertem_entre_si() {
    let dir = pasta("cruzado");
    let r = Registro::default();
    let (a, b, c) = (dir.join("a.dxf"), dir.join("b.svg"), dir.join("c.dxf"));
    r.gravar_documento(&documento(), &a, &OpcoesExportar::default()).unwrap();
    r.converter(&a, &b, &OpcoesImportar::default(), &OpcoesExportar::default()).unwrap();
    r.converter(&b, &c, &OpcoesImportar::default(), &OpcoesExportar::default()).unwrap();
    let (v, ..) = extrudar(&r.importar(&c, &OpcoesImportar::default()).unwrap().doc);
    assert!((v - area() * 10.).abs() < 1e-3, "{v}");
}

#[test]
fn svg_de_fora_entra_com_texto_no_relatorio() {
    let dir = pasta("fora");
    let a = dir.join("logo.svg");
    std::fs::write(&a, r#"<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="20mm" viewBox="0 0 40 20"><g id="Corte"><rect x="0" y="0" width="40" height="20"/><circle cx="20" cy="10" r="5"/></g><text x="1" y="1">oi</text></svg>"#).unwrap();
    let i = Registro::default().importar(&a, &OpcoesImportar::default()).unwrap();
    assert!(i.relatorio.nao_carrega.iter().any(|n| n.args.iter().any(|t| t.contains("text"))), "{:?}", i.relatorio.nao_carrega);
    let (v, lo, hi) = extrudar(&i.doc);
    assert!((v - (800. - PI * 25.) * 10.).abs() < 1e-3, "{v}");
    assert!((hi[0] - lo[0] - 40.).abs() < 1e-3 && (hi[1] - lo[1] - 20.).abs() < 1e-3);
}

fn obra() -> ModeloNeutro {
    let forma = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [300., 300., 3000.]).unwrap();
    let pilar = ProdutoNeutro { nome: "Pilar ç".into(), forma: Some(forma), cor: Some([0.6, 0.6, 0.2, 1.]), ..Default::default() };
    let em = |x: f64| [1., 0., 0., x, 0., 1., 0., 0., 0., 0., 1., 0.];
    let filhos = vec![InstanciaNeutra { produto: 0, nome: "P1".into(), local: IDENTIDADE, cor: None }, InstanciaNeutra { produto: 0, nome: "P2".into(), local: em(4000.), cor: None }];
    let topo = ProdutoNeutro { nome: "Obra".into(), filhos, ..Default::default() };
    ModeloNeutro { formato: Formato::Step, xde: ModeloXde { produtos: vec![pilar, topo], raizes: vec![1], ..Default::default() } }
}

#[test]
fn ifc_volta_com_volume_caixa_nomes_e_cor_em_qualquer_unidade() {
    let dir = pasta("ifc");
    for unidade in [UnidadeSaida::Milimetro, UnidadeSaida::Metro, UnidadeSaida::Polegada] {
        let a = dir.join("obra.ifc");
        let saida = OpcoesExportar { unidade, ..Default::default() };
        Registro::default().gravar(&obra(), &a, &saida).unwrap();
        let o = OpcoesImportar { destino: Destino::UmaPeca, ..Default::default() };
        let i = Registro::default().importar(&a, &o).unwrap();
        let r = &i.relatorio;
        assert_eq!(r.formato, "IFC");
        assert_eq!(r.solidos, 2, "{unidade:?}");
        assert!((r.volume - 2. * 300. * 300. * 3000.).abs() < 1., "{unidade:?}: {}", r.volume);
        let m = r.medidas().unwrap();
        assert!((m[0] - 4300.).abs() < 1e-3 && (m[2] - 3000.).abs() < 1e-3, "{unidade:?}: {m:?}");
        assert!(i.doc.collectors.iter().any(|c| c.name.contains("P2")), "{:?}", i.doc.collectors.iter().map(|c| &c.name).collect::<Vec<_>>());
        assert!(i.doc.collectors.iter().all(|c| c.color == [153, 153, 51]), "{:?}", i.doc.collectors[0].color);
    }
}

#[test]
fn ifc_nao_vira_step_nem_step_vira_ifc() {
    let dir = pasta("esquema");
    let r = Registro::default();
    let (ifc, step) = (dir.join("a.ifc"), dir.join("a.step"));
    r.gravar(&obra(), &ifc, &OpcoesExportar::default()).unwrap();
    r.gravar(&obra(), &step, &OpcoesExportar::default()).unwrap();
    assert_eq!(fenkra_conversor::detectar(&ifc, &std::fs::read(&ifc).unwrap()).unwrap().0, Formato::Ifc);
    assert_eq!(fenkra_conversor::detectar(&step, &std::fs::read(&step).unwrap()).unwrap().0, Formato::Step);
}
