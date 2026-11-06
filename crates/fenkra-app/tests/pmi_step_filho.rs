// SPDX-License-Identifier: LGPL-2.1-or-later
//! B191: ida e volta do PMI do STEP pelo processo filho `fenkra-conv`. O documento anotado (a flange do
//! corpus, B53) é gravado em AP242 com PMI; o filho lê esse arquivo e grava outro; os dois arquivos,
//! lidos aqui, têm de ter as mesmas anotações.

#[path = "../../fenkra-solidos/tests/common/mod.rs"]
mod common;

#[path = "../../fenkra-conversor/tests/pmi_corpus/mod.rs"]
mod pmi_corpus;

use common::kernel_disponivel;
use fenkra_conversor::processo::{Filho, Resposta, Trabalho};
use fenkra_conversor::{EsquemaSaida, OpcoesExportar, Registro};
use fenkra_kernel::{DadoXde, ModeloXde, OpcoesLerXde};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[test]
fn pmi_atravessa_o_processo_filho_com_os_mesmos_valores() {
    if !kernel_disponivel() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("Fenkra pmi filho açaí {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (a, b) = (dir.join("a.step"), dir.join("b.step"));
    let saida = OpcoesExportar { esquema: EsquemaSaida::Ap242, pmi: true, ..Default::default() };
    Registro::default().gravar_documento(&pmi_corpus::flange_anotada(), &a, &saida).unwrap();

    let filho = Filho { programa: env!("CARGO_BIN_EXE_fenkra-conv").into(), limite: Duration::from_secs(120) };
    let r = filho.executar(Trabalho::Converter { origem: a.clone(), destino: b.clone(), importar: Default::default(), exportar: saida }, &AtomicBool::new(false)).unwrap();
    assert_eq!(r, Resposta::Convertido);

    let ler = |p: &std::path::Path| ModeloXde::ler(p.to_str().unwrap(), OpcoesLerXde::default()).unwrap();
    let (la, lb) = (ler(&a), ler(&b));
    assert_eq!((la.pmi[0], la.pmi[1]), (2, 2), "a flange tem 2 cotas e 2 quadros no arquivo");
    assert_eq!(la.pmi, lb.pmi, "o filho gravou o mesmo PMI que leu");
    assert_eq!(la.anotacoes.len(), lb.anotacoes.len());
    for x in &la.anotacoes {
        assert!(lb.anotacoes.contains(x), "{x:?} não voltou pelo filho");
    }
    let valores: Vec<f64> = lb.anotacoes.iter().filter_map(|x| if let DadoXde::Dimensao { valor, .. } = &x.dado { Some(*valor) } else { None }).collect();
    assert!(valores.iter().any(|v| (v - 8.).abs() < 1e-9) && valores.iter().any(|v| (v - 20.).abs() < 1e-9), "{valores:?}");
    std::fs::remove_dir_all(dir).unwrap();
}
