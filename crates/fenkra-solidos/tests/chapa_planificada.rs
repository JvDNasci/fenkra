// SPDX-License-Identifier: LGPL-2.1-or-later
//! B86: planificação exata, verificação de sobreposição e DXF com camadas fixas.
//!
//! Cada caso monta o documento em código, recalcula na planificada e confere status, volume e
//! caixa do blank.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::planificada::{dxf, planificar, Entidade, CAMADA_CONTORNO, CAMADA_DOBRA_CIMA, CAMADA_FERRAMENTA, CAMADA_FUROS};
use fenkra_solidos::{recompute, tabela_dobras, Status};
use std::f64::consts::PI;

fn s(x: &str) -> String {
    x.to_string()
}

fn base() -> Document {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    d
}

fn aba(d: &mut Document, aresta: [f64; 3], comprimento: &str, angulo: &str) {
    d.add_feature(FeatureKind::SmFlange(SmFlange { edge: aresta, length: s(comprimento), angle: s(angulo), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
}

fn flat(d: &Document) -> Document {
    let mut f = d.clone();
    f.sheet.as_mut().unwrap().flat = true;
    f
}

fn perfil(d: &mut Document) -> FeatureKind {
    // L aberto no plano XZ na ponta x = 100: sobe 18 mm e dobra para fora 10 mm
    let pr = sk_lines(d, PlaneRef::Origin(OriginPlane::XZ), &[([100., 2.], [100., 20.]), ([100., 20.], [110., 20.])]);
    FeatureKind::SmProfileFlange(SmProfileFlange { sketch: pr, l1: s("0"), l2: s("50"), radius: None, flip: false })
}

fn sk_lines(d: &mut Document, plane: PlaneRef, segs: &[([f64; 2], [f64; 2])]) -> Id {
    let mut data = SketchData::default();
    let mut n = 3_000_000u64 + d.features.len() as u64 * 1000;
    let mut al = || {
        n += 1;
        Id(n)
    };
    for (a, b) in segs {
        let pa = data.add_point(&mut al, a[0], a[1]);
        let pb = data.add_point(&mut al, b[0], b[1]);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: pa, b: pb }, false);
    }
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

/// Chapa com aba e furo: o contorno é o blank desenvolvido, o furo vira círculo exato e a dobra
/// sai com a mesma etiqueta e ordem da vista.
#[test]
fn planificada_exata_com_aba_furo_e_dxf_em_camadas() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = base();
    aba(&mut d, [100., 25., 2.], "20", "90");
    let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "2");
    let sk = sk_circulos(&mut d, PlaneRef::Datum(top), &[([30., 25.], 5.)]);
    d.add_feature(FeatureKind::SmCut(SmCut { sketch: sk, mode: 0, offset: s(""), draft: s("0"), height: s("0"), flip: true, normal_face: false }));

    let p = planificar(&d).expect("planificação");
    assert!(p.valida(), "{:?}", p.problemas());
    let ba = PI / 2. * (2. + 0.44 * 2.);
    // blank: 100 + dobra desenvolvida + 20 de aba por 50
    let esperado_x = 100. + ba + 20.;
    assert!((p.propriedades.largura - esperado_x).abs() < 0.05, "largura {}", p.propriedades.largura);
    assert!((p.propriedades.altura - 50.).abs() < 0.05, "altura {}", p.propriedades.altura);
    let area = esperado_x * 50. - PI * 25.;
    assert!((p.propriedades.area - area).abs() < 0.5, "área {} esperada {area}", p.propriedades.area);
    assert_eq!(p.propriedades.dobras, 1);
    // furo: um círculo exato de raio 5
    assert_eq!(p.furos.len(), 1, "{:?}", p.furos);
    assert!(matches!(p.furos[0], Entidade::Circulo { raio, .. } if (raio - 5.).abs() < 0.01), "{:?}", p.furos[0]);
    // contorno: só linhas (retângulo), sem retalhos
    assert_eq!(p.contorno.len(), 4, "{:?}", p.contorno);
    assert!(p.contorno.iter().all(|e| matches!(e, Entidade::Linha(..))), "{:?}", p.contorno);
    // o canto inferior esquerdo do blank é a origem do desenho
    let pts: Vec<[f64; 2]> = p.contorno.iter().flat_map(|e| if let Entidade::Linha(a, b) = e { vec![*a, *b] } else { vec![] }).collect();
    let (xmin, ymin) = (pts.iter().map(|q| q[0]).fold(f64::MAX, f64::min), pts.iter().map(|q| q[1]).fold(f64::MAX, f64::min));
    assert!(xmin.abs() < 1e-6 && ymin.abs() < 1e-6, "origem do blank {xmin} {ymin}");

    // etiqueta, ordem e números da tabela são os da vista
    let f = flat(&d);
    let r = recompute(&f);
    let tabela = tabela_dobras(&f, &r.bend_lines);
    assert_eq!(tabela.len(), 1);
    assert_eq!(tabela[0].ordem, 1);
    assert_eq!(tabela[0].rotulo, r.bend_lines[0].label());
    assert_eq!(p.dobras[0].rotulo, r.bend_lines[0].label());
    assert!((tabela[0].allowance - ba).abs() < 1e-6, "{}", tabela[0].allowance);
    // a linha de dobra cai no meio do arco desenvolvido e tem a largura da aresta
    let d0 = &p.dobras[0];
    assert!(((d0.a[1] - d0.b[1]).abs() - 50.).abs() < 0.05, "{d0:?}");

    // DXF: camadas fixas
    let texto = dxf(&p);
    for camada in [CAMADA_CONTORNO, CAMADA_FUROS, CAMADA_DOBRA_CIMA] {
        assert!(texto.contains(&format!("8\n{camada}\n")), "falta a camada {camada}");
    }
    assert!(texto.contains("0\nCIRCLE\n") && texto.contains("0\nMTEXT\n"));
    // 1:1: o furo tem raio 5 em milímetros
    assert!(texto.contains("40\n5.0000\n"), "raio do furo");
    // R2000 de verdade: lê de volta com as mesmas camadas, em milímetros, e o rótulo guarda o símbolo
    let lido = fenkra_dxf::ler(texto.as_bytes()).unwrap();
    assert_eq!((lido.versao.as_str(), lido.unidade), ("AC1015", fenkra_dxf::Unidade::Milimetro));
    assert!(lido.itens.iter().any(|i| i.camada == CAMADA_FUROS && matches!(i.entidade, fenkra_dxf::Entidade::Circulo { raio, .. } if (raio - 5.).abs() < 1e-4)));
    assert!(lido.itens.iter().any(|i| matches!(&i.entidade, fenkra_dxf::Entidade::Texto { texto, .. } if texto.contains('°'))), "o rótulo perdeu o símbolo");
    assert!(texto.ends_with("0\nEOF\n"));
}

/// Aba por perfil: na planificada vira uma tira com o comprimento desenvolvido e uma linha de
/// dobra por canto; antes ela sumia.
#[test]
fn aba_por_perfil_nao_some_na_planificada() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = base();
    let k = perfil(&mut d);
    d.add_feature(k);
    let f = flat(&d);
    let r = recompute(&f);
    for ft in &f.features {
        assert_eq!(r.status[&ft.id], Status::Ok, "{}: {:?}", ft.name, r.status[&ft.id]);
    }
    // trechos retos (18 - 3) + (10 - 3) e o canto: 90° com raio interno 3 (a cadeia é a face de dentro)
    let ba = PI / 2. * (3. + 0.44 * 2.);
    let comprimento = 22. + ba;
    let vol = r.bodies[0].mass.volume;
    assert!((vol - (10000. + comprimento * 50. * 2.)).abs() < 1., "volume {vol}");
    let xmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[0]).fold(f32::MIN, f32::max) as f64;
    assert!((xmax - (100. + comprimento)).abs() < 0.05, "a tira sai para fora da chapa: xmax {xmax}");
    // uma dobra no mapa, a 90°, para cima
    assert_eq!(r.bend_lines.len(), 1);
    let b = &r.bend_lines[0];
    assert!((b.angle - 90.).abs() < 1e-6 && b.up && (b.radius - 3.).abs() < 1e-6, "{b:?}");
    assert!((b.allowance - ba).abs() < 1e-6);
    let cx = (b.center[0][0] + b.center[1][0]) / 2.;
    assert!((cx - (100. + 15. + ba / 2.)).abs() < 0.05, "centro da dobra {cx}");
    let p = planificar(&d).unwrap();
    assert!(p.valida(), "{:?}", p.problemas());
    assert_eq!(p.dobras.len(), 1);
    assert!(dxf(&p).contains("8\nDOBRA_CIMA\n"));
}

/// Veneziana e estampos não mudam a chapa plana, mas aparecem na camada de ferramenta.
#[test]
fn veneziana_e_estampo_ficam_na_camada_de_ferramenta() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = base();
    d.add_feature(FeatureKind::SmLouver(SmLouver { a: [30., 10., 2.], b: [30., 40., 2.], width: s("12"), height: s("5"), flip: false, down: false }));
    let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "2");
    let quad = sk_rect(&mut d, PlaneRef::Datum(top), [60., 15.], [80., 35.]);
    d.add_feature(FeatureKind::SmStamp(SmStamp { sketch: quad, depth: s("3"), flip: false, kind: 0, width: String::new(), punch: None }));
    let pts = sk_pontos(&mut d, PlaneRef::Datum(top), &[[10., 10.], [10., 40.]]);
    d.add_feature(FeatureKind::SmStamp(SmStamp { sketch: pts, depth: s("3"), flip: false, kind: 2, width: s("8"), punch: None }));

    let f = flat(&d);
    let r = recompute(&f);
    for ft in &f.features {
        assert!(!matches!(r.status[&ft.id], Status::Error(_)), "{}: {:?}", ft.name, r.status[&ft.id]);
    }
    // a chapa plana continua um retângulo 100 × 50 × 2
    assert!((r.bodies[0].mass.volume - 10000.).abs() < 0.5, "volume {}", r.bodies[0].mass.volume);
    let p = planificar(&d).unwrap();
    assert!(p.valida(), "{:?}", p.problemas());
    // veneziana (4 linhas) + estampo quadrado (4 linhas) + dois círculos
    let linhas = p.ferramenta.iter().filter(|e| matches!(e, Entidade::Linha(..))).count();
    let circulos = p.ferramenta.iter().filter(|e| matches!(e, Entidade::Circulo { .. })).count();
    assert_eq!((linhas, circulos), (8, 2), "{:?}", p.ferramenta);
    // dentro do blank, e a linha da veneziana (x = 30, 30 mm de comprimento) está no lugar
    for e in &p.ferramenta {
        if let Entidade::Linha(a, b) = e {
            assert!([a, b].iter().all(|q| (-0.01..=100.01).contains(&q[0]) && (-0.01..=50.01).contains(&q[1])), "{e:?}");
        }
    }
    assert!(p.ferramenta.iter().any(|e| matches!(e, Entidade::Linha(a, b) if (a[0] - 30.).abs() < 0.02 && (b[0] - 30.).abs() < 0.02 && ((a[1] - b[1]).abs() - 30.).abs() < 0.02)), "{:?}", p.ferramenta);
    assert!(dxf(&p).contains(&format!("8\n{CAMADA_FERRAMENTA}\n")));
    assert_eq!(p.propriedades.dobras, 0);
}

/// Dois comandos que ocupam o mesmo lugar na chapa plana são listados.
#[test]
fn sobreposicao_na_planificada_e_listada() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = base();
    let k = perfil(&mut d);
    d.add_feature(k);
    let k2 = perfil(&mut d);
    d.add_feature(k2);
    let p = planificar(&d).unwrap();
    assert_eq!(p.sobreposicoes.len(), 1, "{:?}", p.sobreposicoes);
    assert!(p.sobreposicoes[0].volume > 1000.);
    assert!(!p.valida());
    assert!(p.problemas()[0].contains("sobreposição"), "{:?}", p.problemas());
    // e a exportação recusa gravar a planificada com sobreposição
    let tmp = std::env::temp_dir().join("fenkra_planificada_sobreposta.dxf");
    assert!(fenkra_solidos::planificada::exportar_dxf(&d, tmp.to_str().unwrap(), true).is_err());
    assert!(!tmp.exists());
    // sem exigir, grava
    fenkra_solidos::planificada::exportar_dxf(&d, tmp.to_str().unwrap(), false).unwrap();
    assert!(std::fs::read_to_string(&tmp).unwrap().contains("CONTORNO"));
    let _ = std::fs::remove_file(tmp);
}

/// Propriedades do blank: caixa, área, massa pelo material e número de dobras; a tabela do mapa
/// segue a ordem das linhas de dobra.
#[test]
fn propriedades_do_blank_e_tabela_na_ordem_da_vista() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = base();
    d.sheet.as_mut().unwrap().material = s("Aço carbono (SAE 1010/1020)");
    aba(&mut d, [100., 25., 2.], "20", "90");
    aba(&mut d, [0., 25., 2.], "15", "90");
    let p = planificar(&d).unwrap();
    assert_eq!(p.propriedades.dobras, 2);
    let massa = p.propriedades.volume * 7.85e-6;
    assert!((p.propriedades.massa.unwrap() - massa).abs() < 1e-9);
    let f = flat(&d);
    let r = recompute(&f);
    let tab = tabela_dobras(&f, &r.bend_lines);
    assert_eq!(tab.iter().map(|t| t.ordem).collect::<Vec<_>>(), vec![1, 2]);
    for (t, b) in tab.iter().zip(&r.bend_lines) {
        assert_eq!(t.rotulo, b.label());
    }
    assert_eq!(p.dobras.iter().map(|b| b.rotulo.clone()).collect::<Vec<_>>(), tab.iter().map(|t| t.rotulo.clone()).collect::<Vec<_>>());
}
