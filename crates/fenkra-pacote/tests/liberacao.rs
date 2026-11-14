// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pacote de liberação (B141) montado de ponta a ponta: projeto com peças recalculadas, revisão,
//! edição depois da revisão, montagem, desenho, pedido e aviso de mudança, e o ZIP conferido por dentro.

use fenkra_core::{BoolOp, Document, FeatureKind, OriginPlane, PlaneRef, PrimKind, Primitive};
use fenkra_desenho::{Drawing, Format, Source};
use fenkra_montagem::{Assembly, NodeKind, Placement};
use fenkra_pacote::liberacao::{pacote_liberacao, OpcoesLiberacao};
use fenkra_project::pacote::ler_zip;
use fenkra_project::Project;
use fenkra_solidos::Status;
use std::path::PathBuf;

fn pasta(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fenkra_liberacao_{nome}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn caixa(nome: &str, a: &str, b: &str, c: &str) -> Document {
    let s = |x: &str| x.to_string();
    let mut d = Document::new(nome);
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s(a), b: s(b), c: s(c), d: s("0"), op: BoolOp::Add }));
    d
}

fn conferir(d: &Document, volume: f64, dims: [f64; 3]) {
    let r = fenkra_solidos::recompute(d);
    assert!(r.status.values().all(|s| matches!(s, Status::Ok)), "{}: {:?}", d.name, r.status);
    let v: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
    assert!((v - volume).abs() < 1e-3, "{}: volume {v}", d.name);
    let (lo, hi) = fenkra_solidos::final_shape(d).unwrap().bbox().unwrap();
    for k in 0..3 {
        assert!((hi[k] - lo[k] - dims[k]).abs() < 1e-3, "{}: caixa {lo:?} {hi:?}", d.name);
    }
}

fn arquivo<'a>(z: &'a [(String, Vec<u8>)], nome: &str) -> &'a [u8] {
    &z.iter().find(|(n, _)| n == nome).unwrap_or_else(|| panic!("{nome} ausente; tem {:?}", z.iter().map(|x| &x.0).collect::<Vec<_>>())).1
}

/// Caixa envolvente lida do STEP: menor e maior coordenada dos CARTESIAN_POINT.
fn caixa_do_step(b: &[u8]) -> ([f64; 3], [f64; 3]) {
    let t = String::from_utf8_lossy(b);
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for l in t.lines().filter(|l| l.contains("CARTESIAN_POINT")) {
        let Some(ini) = l.rfind("((").or_else(|| l.find(",(")) else { continue };
        let resto = &l[ini..].trim_start_matches([',', '(']);
        let nums: Vec<f64> = resto.split(')').next().unwrap_or("").split(',').filter_map(|x| x.trim().parse().ok()).collect();
        if nums.len() == 3 {
            for k in 0..3 {
                lo[k] = lo[k].min(nums[k]);
                hi[k] = hi[k].max(nums[k]);
            }
        }
    }
    (lo, hi)
}

#[test]
fn pacote_com_step_pdf_dxf_bom_e_aviso_de_mudanca() {
    let d = pasta("kart");
    let mut p = Project::create(&d, "Kart").unwrap();
    let placa = caixa("Placa", "80", "20", "5");
    let pino = caixa("Pino", "10", "10", "60");
    conferir(&placa, 8000., [80., 20., 5.]);
    conferir(&pino, 6000., [10., 10., 60.]);
    let id_placa = p.add_document(placa).unwrap();
    let id_pino = p.add_document(pino).unwrap();
    p.release(id_placa, "primeira").unwrap();
    // edição da placa depois da revisão: o pacote tem de levar a revisão A, não a edição
    // como faz a interface, o documento salvo carrega o PN que o projeto deu
    let mut maior = caixa("Placa", "120", "20", "5");
    maior.props.pn = p.entry(id_placa).unwrap().pn.clone();
    conferir(&maior, 12000., [120., 20., 5.]);
    p.save(id_placa, &maior, "maior, ainda não liberada").unwrap();
    let rel = |p: &Project, id: u64| p.entry(id).unwrap().file.replace('\\', "/");
    let mut kart = Assembly::new("Kart");
    kart.add_component("Placa", NodeKind::Part { path: rel(&p, id_placa) }, Placement::default(), None);
    kart.add_component("Pino 1", NodeKind::Part { path: rel(&p, id_pino) }, Placement::default(), None);
    kart.add_component("Pino 2", NodeKind::Part { path: rel(&p, id_pino) }, Placement { origin: [30., 0., 0.], ..Placement::default() }, None);
    std::fs::write(d.join("kart.fka"), kart.to_json()).unwrap();
    let mut des = Drawing::new("Desenho da placa", Format::A4);
    let folha = des.sheets[0].id;
    des.add_standard_set(folha, Source { path: rel(&p, id_placa), display_state: String::new(), hidden: vec![] }, [60., 150.], 1., 60.);
    std::fs::write(d.join("placa.fkd"), des.to_json()).unwrap();

    // pacote padrão: só a peça liberada, na revisão A
    let destino = d.join("saida/liberacao.zip");
    let r = pacote_liberacao(&p, &destino, &OpcoesLiberacao::default()).unwrap();
    assert!(r.avisos.is_empty(), "{:?}", r.avisos);
    let z = ler_zip(&std::fs::read(&destino).unwrap()).unwrap();
    assert_eq!(z[0].0, "manifesto.json");
    let pn_placa = p.entry(id_placa).unwrap().pn.clone();
    let step = arquivo(&z, &format!("step/{pn_placa}_revA.step"));
    assert!(String::from_utf8_lossy(step).starts_with("ISO-10303-21"));
    let (lo, hi) = caixa_do_step(step);
    assert!((hi[0] - lo[0] - 80.).abs() < 1e-3, "STEP da revisão A tem 80 mm, não os 120 da edição: {lo:?} {hi:?}");
    assert!(!z.iter().any(|(n, _)| n.contains(&p.entry(id_pino).unwrap().pn) && n.starts_with("step/")), "o pino não está liberado");
    assert!(arquivo(&z, "desenhos/placa.pdf").starts_with(b"%PDF"));
    assert!(String::from_utf8_lossy(arquivo(&z, "desenhos/placa_f1.dxf")).contains("ENTITIES"));
    // BOM nos três formatos, com quantidade, estado e revisão
    let csv = String::from_utf8_lossy(arquivo(&z, "bom/kart.csv")).to_string();
    assert!(csv.lines().next().unwrap().starts_with("Item;Nível;PN"));
    let pn_pino = p.entry(id_pino).unwrap().pn.clone();
    assert!(csv.lines().any(|l| l.contains(&pn_pino) && l.contains(";2;") && l.contains("Em trabalho")), "{csv}");
    // a placa tem edição depois da revisão: o estado é "em trabalho", mas a revisão vigente é a A
    assert!(csv.lines().any(|l| l.contains(&pn_placa) && l.ends_with(";Em trabalho;A")), "{csv}");
    let j: serde_json::Value = serde_json::from_slice(arquivo(&z, "bom/kart.json")).unwrap();
    assert_eq!(j["itens"].as_array().unwrap().len(), 2);
    let xml = String::from_utf8_lossy(arquivo(&z, "bom/kart.xml")).to_string();
    assert!(xml.contains(&format!("pn=\"{pn_pino}\" quantidade=\"2\"")), "{xml}");
    // manifesto com o hash de cada arquivo
    let m: serde_json::Value = serde_json::from_slice(&z[0].1).unwrap();
    for a in m["arquivos"].as_array().unwrap() {
        let b = arquivo(&z, a["nome"].as_str().unwrap());
        assert_eq!(a["blake3"].as_str().unwrap(), blake3::hash(b).to_hex().as_str());
    }
    assert_eq!(m["pecas"].as_array().unwrap().len(), 1);

    // mudança: pedido, aprovação, revisões novas, aviso; o pacote do aviso leva as revisões "depois"
    let ecr = p.abrir_pedido_mudanca("Placa maior", "o pino não cabe", &[id_placa, id_pino]).unwrap();
    p.decidir_pedido_mudanca(&ecr, true, "aprovado na reunião").unwrap();
    p.release(id_placa, "120 mm").unwrap();
    p.release(id_pino, "primeira").unwrap();
    let ecn = p.emitir_aviso_mudanca(&ecr, "placa de 120 mm").unwrap();
    let destino2 = d.join("saida/ecn.zip");
    let o = OpcoesLiberacao { aviso: Some(ecn.clone()), pdf: false, dxf: false, ..OpcoesLiberacao::default() };
    pacote_liberacao(&p, &destino2, &o).unwrap();
    let z = ler_zip(&std::fs::read(&destino2).unwrap()).unwrap();
    let (lo, hi) = caixa_do_step(arquivo(&z, &format!("step/{pn_placa}_revB.step")));
    assert!((hi[0] - lo[0] - 120.).abs() < 1e-3, "{lo:?} {hi:?}");
    let (lo, hi) = caixa_do_step(arquivo(&z, &format!("step/{pn_pino}_revA.step")));
    assert!((hi[2] - lo[2] - 60.).abs() < 1e-3, "{lo:?} {hi:?}");
    assert!(!z.iter().any(|(n, _)| n.starts_with("desenhos/")), "PDF e DXF desligados");
    let av: serde_json::Value = serde_json::from_slice(arquivo(&z, &format!("mudancas/{ecn}.json"))).unwrap();
    assert_eq!(av["pedido"], ecr.as_str());
    assert_eq!(av["pedido_detalhe"]["motivo"], "o pino não cabe");
    assert_eq!(av["itens"][0]["revisao_antes"], "A");
    assert_eq!(av["itens"][0]["revisao_depois"], "B");
    assert!(arquivo(&z, "mudancas/registro.json").len() > 10);
    // aviso inexistente é erro claro
    assert!(pacote_liberacao(&p, &destino2, &OpcoesLiberacao { aviso: Some("ECN-9999".into()), ..OpcoesLiberacao::default() }).is_err());
    let _ = std::fs::remove_dir_all(&d);
}
