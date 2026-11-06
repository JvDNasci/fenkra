// SPDX-License-Identifier: LGPL-2.1-or-later
//! `fenkra-cli visualizar`: gera o `.fkv` e a página, dentro da pasta, sem sobrescrever sem pedir.

use fenkra_cli::pasta::Pasta;
use fenkra_cli::visualizar::{visualizar, Pedido};
use fenkra_pacote::Pacote;

#[test]
fn gera_pacote_e_pagina_conferidos() {
    let dir = std::env::temp_dir().join(format!("fenkra_cli_vis_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    let pasta = Pasta::nova(&dir).unwrap();
    let pedido = Pedido { arquivo: "placa.fkp", saida: "placa.fkv", html: Some("placa.html"), decimar: None, sobrescrever: false };
    let r = visualizar(&pasta, &pedido).unwrap();
    assert_eq!(r["tipo"], "peca");
    let b = std::fs::read(dir.join("placa.fkv")).unwrap();
    assert_eq!(r["bytes"], b.len());
    Pacote::ler(&b).unwrap().validar().unwrap();
    let html = std::fs::read_to_string(dir.join("placa.html")).unwrap();
    assert!(html.contains("fkv-dados") && !html.contains("/*@PACOTE@*/"));
    // não sobrescreve sem pedir
    assert_eq!(visualizar(&pasta, &pedido).unwrap_err().codigo, "ja_existe");
    // fora da pasta e extensão errada são recusados
    let fora = Pedido { arquivo: "../placa.fkp", ..pedido };
    assert!(visualizar(&pasta, &fora).is_err());
    let ruim = Pedido { arquivo: "placa.fkp", saida: "placa.step", html: None, decimar: None, sobrescrever: true };
    assert!(visualizar(&pasta, &ruim).is_err());
    // decimar fora da faixa
    let d = Pedido { arquivo: "placa.fkp", saida: "leve.fkv", html: None, decimar: Some(5.), sobrescrever: true };
    assert_eq!(visualizar(&pasta, &d).unwrap_err().codigo, "argumento");
    let d = Pedido { decimar: Some(0.3), ..d };
    let r = visualizar(&pasta, &d).unwrap();
    assert!(r["bytes"].as_u64().unwrap() < b.len() as u64);
}
