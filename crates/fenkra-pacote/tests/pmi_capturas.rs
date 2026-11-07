// SPDX-License-Identifier: LGPL-2.1-or-later
//! B368: PMI e capturas no pacote de visualização. Nota com duas setas (uma para cada lado) e três
//! capturas em sequência; o `.fkv` gravado e relido traz as anotações com a ancoragem de cada seta e
//! as capturas na ordem, e a página autocontida carrega o visualizador que as desenha.
//!
//! `FENKRA_FKV_SAIDA=<pasta>` grava o `.fkv` e a página para ver no navegador.

use fenkra_core::doc::NamedView;
use fenkra_core::pmi::{Pmi, PmiCapture, PmiKind};
use fenkra_core::pmi_leader;
use fenkra_core::Document;
use fenkra_pacote::{de_peca, pagina_html, Opcoes, Pacote};
use std::path::PathBuf;

fn peca_com_pmi() -> Document {
    let mut d = fenkra_solidos::example_document();
    let id = d.alloc();
    let mut nota = Pmi {
        id,
        name: "Nota 1".into(),
        kind: PmiKind::Note,
        anchors: vec![[-40., 0., 10.]],
        offset: [40., 0., 30.],
        text: "Furos passantes".into(),
        tol_plus: String::new(),
        tol_minus: String::new(),
        visible: true,
        plane: None,
        style: Default::default(),
        folder: String::new(),
        targets: vec![],
    };
    pmi_leader::add_tip(&mut nota, [40., 0., 10.], None);
    pmi_leader::set_side(&mut nota, 0, 1);
    pmi_leader::set_side(&mut nota, 1, 2);
    d.pmi.push(nota);
    let id2 = d.alloc();
    d.pmi.push(Pmi {
        id: id2,
        name: "Comprimento".into(),
        kind: PmiKind::Linear,
        anchors: vec![[-60., -36., 10.], [60., -36., 10.]],
        offset: [0., -12., 0.],
        text: String::new(),
        tol_plus: String::new(),
        tol_minus: String::new(),
        visible: true,
        plane: None,
        style: Default::default(),
        folder: String::new(),
        targets: vec![],
    });
    let ids: Vec<_> = d.pmi.iter().map(|p| p.id).collect();
    for k in 1..=3usize {
        let cid = d.alloc();
        d.pmi_captures.push(PmiCapture {
            id: cid,
            name: format!("Captura {k}"),
            view: NamedView { name: String::new(), target: [0., 0., 5.], yaw: -0.5 * k as f64, pitch: 0.8, dist: 200., ortho: false },
            shown: ids[..k.min(ids.len())].to_vec(),
            hidden: vec![],
            with_model: false,
        });
    }
    d
}

#[test]
fn pmi_e_capturas_chegam_ao_fkv_e_a_pagina() {
    let d = peca_com_pmi();
    let pacote = de_peca(&d, &Opcoes::default()).unwrap();
    let bytes = pacote.bytes().unwrap();
    // o que está no arquivo, lido de volta
    let lido = Pacote::ler(&bytes).unwrap();
    let m = &lido.manifesto;
    let pmi = m["pmi"].as_array().unwrap();
    assert_eq!(pmi.len(), 2);
    let nota = pmi.iter().find(|p| p["nome"] == "Nota 1").expect("a nota está no pacote");
    assert_eq!(nota["texto"], "Furos passantes");
    let setas = nota["setas"].as_array().unwrap();
    assert_eq!(setas.len(), 2);
    assert_eq!((setas[0]["lado"].as_u64(), setas[1]["lado"].as_u64()), (Some(1), Some(2)), "uma seta para cada lado");
    assert_eq!(setas[1]["ancora"][0].as_f64(), Some(40.));
    assert!(pmi.iter().find(|p| p["nome"] == "Comprimento").unwrap()["setas"].as_array().unwrap().is_empty(), "cota não tem seta de nota");
    let caps = m["capturas"].as_array().unwrap();
    let nomes: Vec<_> = caps.iter().map(|c| c["nome"].as_str().unwrap()).collect();
    assert_eq!(nomes, ["Captura 1", "Captura 2", "Captura 3"], "na ordem de criação");
    assert!(caps.iter().all(|c| c["camera"]["distancia"].as_f64() == Some(200.)));
    assert_eq!(caps[0]["pmi"].as_array().unwrap().len(), 1);
    assert_eq!(caps[2]["pmi"].as_array().unwrap().len(), 2);
    // a página traz o visualizador com a aba de capturas e a tela 2D das anotações
    let html = pagina_html(Some(&bytes));
    for marca in ["anot2d", "abaCapturas", "barraEtapas", "etapa_de"] {
        assert!(html.contains(marca), "a página não traz '{marca}'");
    }
    if let Ok(dir) = std::env::var("FENKRA_FKV_SAIDA") {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pmi_capturas.fkv"), &bytes).unwrap();
        std::fs::write(dir.join("pmi_capturas.html"), html).unwrap();
    }
}

#[test]
fn legenda_3d_chega_com_os_campos_preenchidos() {
    let mut d = fenkra_solidos::example_document();
    let id = d.alloc();
    d.pmi.push(Pmi {
        id,
        name: "Legenda 1".into(),
        kind: PmiKind::TitleBlock { template: fenkra_core::pmi::TITLE_TEMPLATE.into() },
        anchors: vec![[0., 0., 8.]],
        offset: [20., 0., 20.],
        text: String::new(),
        tol_plus: String::new(),
        tol_minus: String::new(),
        visible: true,
        plane: None,
        style: Default::default(),
        folder: String::new(),
        targets: vec![],
    });
    let p = de_peca(&d, &Opcoes::default()).unwrap();
    let t = p.manifesto["pmi"][0]["texto"].as_str().unwrap();
    assert!(!t.contains('{'), "campo sem preencher: {t}");
    assert!(t.contains(&d.name) && t.contains("FK-0001") && t.contains(" kg"), "legenda: {t}");
}

#[test]
fn arquivo_de_peca_gravado_em_disco_chega_ao_pacote() {
    // o caminho do `fenkra-cli visualizar`: .fkp no disco -> pacote
    let d = peca_com_pmi();
    let dir = std::env::temp_dir().join(format!("fenkra_pmi_cap_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("com_pmi.fkp");
    std::fs::write(&f, d.to_json()).unwrap();
    let p = fenkra_pacote::de_arquivo(&f, &Opcoes::default()).unwrap();
    assert_eq!(p.manifesto["pmi"].as_array().unwrap().len(), 2);
    assert_eq!(p.manifesto["capturas"].as_array().unwrap().len(), 3);
    assert_eq!(p.manifesto["pmi"][0]["setas"][1]["lado"], 2);
}

