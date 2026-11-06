// SPDX-License-Identifier: LGPL-2.1-or-later
//! B191: o PMI semântico do documento atravessa o STEP AP242 e volta com os mesmos valores. A peça
//! anotada é a flange do corpus (B53): disco com furo central e quatro furos de fixação.

#[path = "../../fenkra-solidos/tests/common/mod.rs"]
mod common;

#[path = "pmi_corpus/mod.rs"]
mod pmi_corpus;

use common::*;
use pmi_corpus::flange_anotada;
use fenkra_conversor::{anotacoes_do_documento, EsquemaSaida, Formato, ModeloNeutro, OpcoesExportar, Registro, NOTAS_PMI};
use fenkra_core::gdt::Modif;
use fenkra_core::pmi::*;
use fenkra_core::*;
use fenkra_kernel::{AlvoDatumXde, DadoXde, MaterialXde, ModeloXde, OpcoesLerXde, Shape, TipoDimensaoXde, TipoToleranciaXde, ZonaXde};
use fenkra_solidos::final_shape;

macro_rules! precisa_kernel {
    () => {
        if !kernel_disponivel() {
            return;
        }
    };
}

fn ap242() -> OpcoesExportar {
    OpcoesExportar { esquema: EsquemaSaida::Ap242, pmi: true, ..Default::default() }
}

fn pasta(teste: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra pmi step {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn flange_anotada_atravessa_o_step_com_valores_faces_e_ordem_dos_datums() {
    precisa_kernel!();
    let doc = flange_anotada();
    let dir = pasta("ida");
    let arquivo = dir.join("flange.step");
    Registro::default().gravar_documento(&doc, &arquivo, &ap242()).unwrap();

    let lido = ModeloXde::ler(arquivo.to_str().unwrap(), OpcoesLerXde::default()).unwrap();
    // o arquivo tem 2 datums (A e B) mais 1 alvo, 2 dimensões e 2 tolerâncias
    assert_eq!(lido.pmi[0], 2, "dimensões: {:?}", lido.pmi);
    assert_eq!(lido.pmi[1], 2, "tolerâncias: {:?}", lido.pmi);
    let forma = lido.produtos[0].forma.as_ref().unwrap();
    assert!((forma.mass().volume - final_shape(&doc).unwrap().mass().volume).abs() < 1e-6, "a forma segue a mesma");

    let datums: Vec<_> = lido.anotacoes.iter().filter_map(|a| if let DadoXde::Datum { letra, posicao, alvo } = &a.dado { Some((letra.as_str(), *posicao, *alvo, a.faces.clone())) } else { None }).collect();
    let letra = |l: &str, alvo: bool| datums.iter().find(|d| d.0 == l && d.2.is_some() == alvo).unwrap_or_else(|| panic!("datum {l} ausente em {datums:?}"));
    assert_eq!(letra("A", false).1, 1, "A é primária");
    assert_eq!(letra("B", false).1, 2, "B é secundária");
    let alvo = letra("A", true).2.unwrap();
    assert_eq!((alvo.tipo, alvo.numero, alvo.comprimento, alvo.largura), (AlvoDatumXde::Retangulo, 1, 6., 4.));
    // A e o alvo estão na mesma face (a de cima); B é o furo central, outra face
    assert_eq!(letra("A", false).3, letra("A", true).3);
    assert_ne!(letra("A", false).3, letra("B", false).3);

    let dim = |t: TipoDimensaoXde| lido.anotacoes.iter().find_map(|a| match &a.dado {
        DadoXde::Dimensao { tipo, valor, tolerancia, faces2 } if *tipo == t => Some((*valor, *tolerancia, a.faces.clone(), faces2.clone())),
        _ => None,
    });
    let (v, tol, f1, f2) = dim(TipoDimensaoXde::LocalDistanciaLinear).expect("espessura");
    assert!((v - 8.).abs() < 1e-9);
    // o OCCT guarda o inferior como módulo: 0,05 volta positivo
    assert_eq!(tol.map(|(i, s)| (i.abs(), s)), Some((0.05, 0.1)));
    assert!(f1.len() == 1 && f2.len() == 1 && f1 != f2, "duas faces distintas");
    let (v, tol, f, _) = dim(TipoDimensaoXde::TamanhoDiametro).expect("diâmetro");
    assert!((v - 20.).abs() < 1e-9, "{v}");
    assert_eq!(tol.map(|(i, s)| (i.abs(), s)), Some((0., 0.02)));
    assert_eq!(f.len(), 1);

    let tol = |t: TipoToleranciaXde| lido.anotacoes.iter().find(|a| matches!(&a.dado, DadoXde::Tolerancia { tipo, .. } if *tipo == t)).unwrap_or_else(|| panic!("tolerância {t:?}"));
    let DadoXde::Tolerancia { valor, zona, material, datums: refs, .. } = &tol(TipoToleranciaXde::Posicao).dado else { unreachable!() };
    assert_eq!((*valor, *zona, *material), (0.2, ZonaXde::Diametro, MaterialXde::Maximo));
    let letras: Vec<&str> = refs.iter().map(|i| match &lido.anotacoes[*i].dado { DadoXde::Datum { letra, .. } => letra.as_str(), _ => "?" }).collect();
    assert_eq!(letras, vec!["A", "B"], "ordem de precedência dos datums");
    let DadoXde::Tolerancia { valor, datums: refs, .. } = &tol(TipoToleranciaXde::Planeza).dado else { unreachable!() };
    assert_eq!(*valor, 0.05);
    assert!(refs.is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn o_que_nao_atravessa_vira_nota() {
    precisa_kernel!();
    let doc = flange_anotada();
    let forma: Shape = final_shape(&doc).unwrap();
    let saida = anotacoes_do_documento(&doc, &forma);
    // rugosidade e nota são PMI gráfico
    let grafico = saida.fora.iter().find(|n| n.modelo.starts_with("PMI gráfico")).expect("nota do gráfico");
    assert_eq!(grafico.args, vec!["2".to_string()]);
    // o modificador de zona projetada não atravessa o OCCT
    let mut d2 = doc.clone();
    if let PmiKind::Gdt { extra, .. } = &mut d2.pmi[4].kind {
        extra.mods.push(Modif::Projetada);
        extra.mods.push(Modif::TodoContorno);
    }
    let s2 = anotacoes_do_documento(&d2, &forma);
    assert!(s2.fora.iter().any(|n| n.modelo.contains("não atravessa o STEP") && n.args[1] == "projetada"));
    assert!(s2.fora.iter().any(|n| n.modelo.contains("modificador") && n.args.len() == 2));
    for n in s2.fora.iter().chain(saida.fora.iter()) {
        assert!(NOTAS_PMI.contains(&n.modelo.as_str()), "{}", n.modelo);
    }
}

#[test]
fn sem_pmi_ou_fora_do_ap242_o_arquivo_sai_sem_anotacoes() {
    precisa_kernel!();
    let doc = flange_anotada();
    let dir = pasta("sem");
    for (nome, o) in [
        ("ap214.step", OpcoesExportar { esquema: EsquemaSaida::Ap214, pmi: true, ..Default::default() }),
        ("sem_chave.step", OpcoesExportar { esquema: EsquemaSaida::Ap242, pmi: false, ..Default::default() }),
    ] {
        let a = dir.join(nome);
        Registro::default().gravar_documento(&doc, &a, &o).unwrap();
        let lido = ModeloXde::ler(a.to_str().unwrap(), OpcoesLerXde::default()).unwrap();
        assert_eq!(lido.pmi, [0, 0, 0], "{nome}");
        assert!(lido.anotacoes.is_empty());
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn step_para_step_preserva_o_pmi_pelo_conversor() {
    precisa_kernel!();
    let doc = flange_anotada();
    let dir = pasta("volta");
    let a = dir.join("a.step");
    let b = dir.join("b.step");
    Registro::default().gravar_documento(&doc, &a, &ap242()).unwrap();
    Registro::default().converter(&a, &b, &Default::default(), &ap242()).unwrap();
    let (la, lb) = (ModeloXde::ler(a.to_str().unwrap(), OpcoesLerXde::default()).unwrap(), ModeloXde::ler(b.to_str().unwrap(), OpcoesLerXde::default()).unwrap());
    assert_eq!(la.pmi, lb.pmi);
    assert_eq!(la.anotacoes.len(), lb.anotacoes.len());
    for x in &la.anotacoes {
        assert!(lb.anotacoes.contains(x), "{x:?} não voltou");
    }
    let _ = (Formato::Step, ModeloNeutro::de_documento_pmi);
    std::fs::remove_dir_all(dir).unwrap();
}
