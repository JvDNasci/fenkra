// SPDX-License-Identifier: LGPL-2.1-or-later
//! B143: reconhecimento de features em corpo importado. Cada teste monta a peça em código, grava em
//! STEP, lê de volta (o corpo chega sem histórico), reconhece, recria e compara com o original.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::importacao::importar_step;
use fenkra_solidos::reconhece::{analisar, converter, Tipo};
use fenkra_solidos::{final_shape, recompute};

fn furo(pt: [f64; 3], dir: [f64; 3], d: &str, kind: HoleKind, depth: Extent, tip: Option<&str>) -> FeatureKind {
    FeatureKind::Hole(Hole {
        sketch: Id(0),
        kind,
        diameter: d.into(),
        depth,
        thread: None,
        thread_modeled: false,
        standard: None,
        points: vec![PointRef::Fixed(pt)],
        dir: Some(AxisRef::Fixed { origin: pt, dir }),
        reverse: false,
        tip: tip.map(String::from),
        taper: "0".into(),
        left_hand: false,
        extra: Default::default(),
    })
}

/// Placa 80x50x20 com furo passante Ø8, furo cego Ø6 x 8, furo rebaixado, furo escareado, dois
/// filetes verticais (R3 e R4) e um chanfro de 2 mm, exportada em STEP e relida.
fn peca(nome: &str) -> (Document, Document) {
    let mut d = Document::new("peca");
    let xy = PlaneRef::Origin(OriginPlane::XY);
    let sk = sk_rect(&mut d, xy, [-40., -25.], [40., 25.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    d.add_feature(furo([-25., 10., 20.], [0., 0., -1.], "8", HoleKind::Simples, Extent::ThroughAll, None));
    d.add_feature(furo([-10., 10., 20.], [0., 0., -1.], "6", HoleKind::Simples, Extent::Blind("8".into()), None));
    d.add_feature(furo([10., 10., 20.], [0., 0., -1.], "5", HoleKind::Rebaixado { diameter: "10".into(), depth: "4".into() }, Extent::ThroughAll, None));
    d.add_feature(furo([25., -10., 20.], [0., 0., -1.], "5", HoleKind::Escareado { diameter: "10".into(), angle: "90".into() }, Extent::ThroughAll, None));
    d.add_feature(filete("3", vec![[40., -25., 10.]], vec![None]));
    d.add_feature(filete("4", vec![[40., 25., 10.]], vec![None]));
    d.add_feature(chanfro("2", vec![[-40., 0., 0.]], vec![None]));
    let r = recompute(&d);
    assert!(problema_de_status(&d, &r).is_none(), "peça de partida: {:?}", problema_de_status(&d, &r));
    let dir = std::env::temp_dir().join(format!("solidos6_b143_{nome}"));
    std::fs::create_dir_all(&dir).unwrap();
    let alvo = dir.join("peca.step");
    final_shape(&d).unwrap().export_step(alvo.to_str().unwrap()).unwrap();
    let imp = importar_step(alvo.to_str().unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(dir);
    (d, imp.doc)
}

#[test]
fn fase1_acha_furos_filetes_e_chanfro_sem_alterar_o_corpo() {
    if !kernel_disponivel() {
        return;
    }
    let (_, imp) = peca("fase1");
    let antes = final_shape(&imp).unwrap().mass();
    let rec = analisar(&final_shape(&imp).unwrap());
    for e in &rec.encontradas {
        eprintln!("{}", e.descricao());
    }
    eprintln!("{}", rec.resumo());
    assert_eq!(rec.contar(Tipo::FuroPassante), 3, "{:#?}", rec.encontradas);
    assert_eq!(rec.contar(Tipo::FuroCego), 1);
    assert_eq!(rec.contar(Tipo::Filete), 2);
    assert_eq!(rec.contar(Tipo::Chanfro), 1);
    let cego = rec.encontradas.iter().find(|e| e.tipo == Tipo::FuroCego).unwrap();
    assert!((cego.medida - 6.).abs() < 1e-6 && (cego.medida2 - 8.).abs() < 1e-6, "{cego:?}");
    let mut raios: Vec<f64> = rec.encontradas.iter().filter(|e| e.tipo == Tipo::Filete).map(|e| e.medida).collect();
    raios.sort_by(f64::total_cmp);
    assert!((raios[0] - 3.).abs() < 1e-6 && (raios[1] - 4.).abs() < 1e-6, "{raios:?}");
    assert!(rec.encontradas.iter().any(|e| e.rebaixo.is_some()));
    assert!(rec.encontradas.iter().any(|e| e.escareado.is_some()));
    // a fase 1 não mexe no corpo
    let depois = final_shape(&imp).unwrap().mass();
    assert_eq!(antes.volume, depois.volume);
    assert!(rec.resumo().contains("não é o histórico original"));
    assert!(rec.arestas_convexas > 0 && rec.arestas_concavas > 0 && rec.arestas_tangentes > 0);
}

#[test]
fn fase2_recria_e_confere_volume_area_e_centroide() {
    if !kernel_disponivel() {
        return;
    }
    let (orig, imp) = peca("fase2");
    let rec = analisar(&final_shape(&imp).unwrap());
    let conv = converter(&imp, &rec, None).unwrap();
    eprintln!("{}", conv.relatorio);
    assert!(conv.mantidas.is_empty(), "{:?}", conv.mantidas);
    assert_eq!(conv.convertidas.len(), rec.encontradas.len());
    let r = recompute(&conv.doc);
    assert!(problema_de_status(&conv.doc, &r).is_none(), "{:?}", problema_de_status(&conv.doc, &r));
    let (a, b) = (final_shape(&orig).unwrap().mass(), final_shape(&conv.doc).unwrap().mass());
    assert!((a.volume - b.volume).abs() / a.volume < 1e-6, "{} × {}", a.volume, b.volume);
    assert!((a.area - b.area).abs() / a.area < 1e-6, "{} × {}", a.area, b.area);
    for i in 0..3 {
        assert!((a.center[i] - b.center[i]).abs() < 1e-5, "{:?} × {:?}", a.center, b.center);
    }
    let (lo0, hi0) = caixa_envolvente(&orig);
    let (lo1, hi1) = caixa_envolvente(&conv.doc);
    for i in 0..3 {
        assert!((lo0[i] - lo1[i]).abs() < 1e-6 && (hi0[i] - hi1[i]).abs() < 1e-6);
    }
    // o corpo recriado tem comandos de furo, filete e chanfro na árvore
    let n = |f: fn(&FeatureKind) -> bool| conv.doc.active_features().filter(|(_, x)| f(&x.kind)).count();
    assert_eq!(n(|k| matches!(k, FeatureKind::Hole(_))), 4);
    assert_eq!(n(|k| matches!(k, FeatureKind::Fillet(_))), 2);
    assert_eq!(n(|k| matches!(k, FeatureKind::Chamfer(_))), 1);
}

fn via_step(d: &Document, nome: &str) -> Document {
    let dir = std::env::temp_dir().join(format!("solidos6_b143_{nome}"));
    std::fs::create_dir_all(&dir).unwrap();
    let alvo = dir.join("peca.step");
    final_shape(d).unwrap().export_step(alvo.to_str().unwrap()).unwrap();
    let imp = importar_step(alvo.to_str().unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(dir);
    imp.doc
}

#[test]
fn caixa_lisa_nao_tem_feature_inventada() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("caixa");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [30., 20.]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    let imp = via_step(&d, "lisa");
    let rec = analisar(&final_shape(&imp).unwrap());
    assert!(rec.encontradas.is_empty(), "{:?}", rec.encontradas);
}

#[test]
fn furo_cego_com_ponta_e_filete_concavo() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("l");
    let xy = PlaneRef::Origin(OriginPlane::XY);
    // L de 40x40 com braço de 10: canto interno em (10,10)
    let sk = sk_poligono(&mut d, xy, &[[0., 0.], [40., 0.], [40., 10.], [10., 10.], [10., 40.], [0., 40.]]);
    d.add_feature(ext(sk, BoolOp::Add, "15"));
    d.add_feature(filete("3", vec![[10., 10., 7.5]], vec![None]));
    d.add_feature(furo([30., 5., 15.], [0., 0., -1.], "4", HoleKind::Simples, Extent::Blind("9".into()), Some("118")));
    let imp = via_step(&d, "l");
    let rec = analisar(&final_shape(&imp).unwrap());
    let f = rec.encontradas.iter().find(|e| e.tipo == Tipo::Filete).expect("filete");
    assert!(!f.convexo && (f.medida - 3.).abs() < 1e-6, "{f:?}");
    let h = rec.encontradas.iter().find(|e| e.tipo == Tipo::FuroCego).expect("furo cego");
    assert!((h.ponta.unwrap() - 118.).abs() < 0.5, "{h:?}");
    let conv = converter(&imp, &rec, None).unwrap();
    eprintln!("{}", conv.relatorio);
    let (a, b) = (final_shape(&d).unwrap().mass(), final_shape(&conv.doc).unwrap().mass());
    assert!((a.volume - b.volume).abs() / a.volume < 1e-6, "{} × {} ({:?})", a.volume, b.volume, conv.mantidas);
    assert!(conv.mantidas.is_empty(), "{:?}", conv.mantidas);
}

#[test]
fn chanfro_em_cadeia_tangente_fica_como_corpo() {
    if !kernel_disponivel() {
        return;
    }
    // o chanfro do topo acompanha o filete do canto: as faces planas isoladas não refazem o corpo
    let mut d = Document::new("cadeia");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-40., -25.], [40., 25.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    d.add_feature(filete("4", vec![[40., 25., 10.]], vec![None]));
    d.add_feature(chanfro("2", vec![[-2., 25., 20.]], vec![None]));
    let imp = via_step(&d, "cadeia");
    let antes = final_shape(&imp).unwrap().mass();
    let rec = analisar(&final_shape(&imp).unwrap());
    let conv = converter(&imp, &rec, None).unwrap();
    let depois = final_shape(&conv.doc).unwrap().mass();
    // o que passou na verificação reproduz o corpo; o que não passou foi mantido com motivo
    assert!((antes.volume - depois.volume).abs() / antes.volume < 1e-6, "{} × {}", antes.volume, depois.volume);
    assert!(!conv.mantidas.is_empty(), "o chanfro em cadeia não pode ser dado como convertido");
    for (_, m) in &conv.mantidas {
        assert!(!m.is_empty());
    }
    assert!(conv.relatorio.contains("não é o histórico original"));
}

/// Aceite do B143 no corpus (B53): cada peça vai e volta por STEP e o reconhecimento acha pelo menos
/// 90% dos furos e dos filetes de raio constante que a peça tem de fato, sem inventar nenhum.
#[test]
fn corpus_por_step_reconhece_noventa_por_cento_dos_furos_e_filetes() {
    if !kernel_disponivel() {
        return;
    }
    // o que cada peça do corpus tem de verdade, na ordem de `pecas()`, pela forma como foi montada:
    // (furos, filetes de raio constante)
    let esperado = |i: usize| -> (usize, usize) {
        match i {
            5 => (15, 0), // placa com grade 5 x 3
            7 => (0, 12), // caixa com as 12 arestas arredondadas
            9 => (5, 0),  // flange: furo central e quatro no círculo
            11 => (1, 0), // engrenagem: furo do eixo
            13 => (1, 0), // porca M8
            _ => (0, 0),
        }
    };
    let (mut tem, mut achou) = (0usize, 0usize);
    for (i, d) in common::corpus::pecas().iter().enumerate() {
        let imp = via_step(d, &format!("corpus{i:02}"));
        let rec = analisar(&final_shape(&imp).unwrap());
        let furos = rec.contar(Tipo::FuroPassante) + rec.contar(Tipo::FuroCego);
        let filetes = rec.contar(Tipo::Filete);
        eprintln!("{i:02} {}: {furos} furos, {filetes} filetes; {}", d.name, rec.encontradas.iter().map(|e| e.descricao()).collect::<Vec<_>>().join(" | "));
        let (ef, el) = esperado(i);
        assert!(furos <= ef && filetes <= el, "{}: feature inventada ({furos} furos, {filetes} filetes)", d.name);
        // furo no centro de peça redonda: a borda de fora é coaxial a ele e não é rebaixo
        if i == 9 || i == 11 {
            assert_eq!(furos, ef, "{}: o furo do centro tem de aparecer", d.name);
        }
        tem += ef + el;
        achou += furos.min(ef) + filetes.min(el);
    }
    eprintln!("reconhecidos {achou} de {tem}");
    assert!(tem > 0 && achou * 10 >= tem * 9, "reconhecidos {achou} de {tem}");
}
