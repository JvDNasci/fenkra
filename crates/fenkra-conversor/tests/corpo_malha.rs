// SPDX-License-Identifier: LGPL-2.1-or-later
//! B170: malha que entra e sai como corpo de malha, sem passar pelo B-rep.

use fenkra_conversor::malha::{Cena, Malha, No};
use fenkra_conversor::{Destino, Formato, OpcoesExportar, OpcoesImportar, Registro};
use fenkra_core::malha::{MeshOp, MeshOpKind, MeshRepair};
use fenkra_core::{Document, FeatureKind};
use fenkra_solidos::corpo_malha::corpos_de_malha;
use fenkra_solidos::{recompute, Status};
use std::path::PathBuf;

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra corpo malha {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn caixa(nome: &str, l: f64, x: f64) -> Malha {
    let mut m = fenkra_malha::cena::cubo(nome, l);
    m.pos.iter_mut().for_each(|p| p[0] += x);
    m
}

fn cena(malhas: Vec<Malha>) -> Cena {
    let mut c = Cena::default();
    for m in malhas {
        c.raiz_com(m);
    }
    c
}

fn sem_erro(d: &Document) -> f64 {
    let r = recompute(d);
    assert!(!r.status.values().any(|s| matches!(s, Status::Error(_))), "{:?}", r.status);
    r.bodies.iter().map(|b| b.mass.volume).sum()
}

fn uma_peca() -> OpcoesImportar {
    OpcoesImportar { destino: Destino::UmaPeca, ..Default::default() }
}

#[test]
fn cada_leitor_do_nucleo_entra_como_corpo_de_malha_sem_kernel_na_forma() {
    let dir = pasta("leitores");
    for f in [Formato::Stl, Formato::Obj, Formato::Ply, Formato::Tmf] {
        let a = dir.join(format!("peça.{}", f.extensoes()[0]));
        fenkra_conversor::malha::gravar_para_teste(f, &cena(vec![caixa("Peça", 10., 0.)]), &a).unwrap();
        let i = Registro::default().importar(&a, &uma_peca()).unwrap();
        assert!(i.doc.features.iter().all(|x| matches!(&x.kind, FeatureKind::MeshImport(m) if m.corpo && !m.dados.is_empty())), "{}", f.nome());
        assert!(fenkra_solidos::final_shape(&i.doc).is_none(), "{}: não tem B-rep", f.nome());
        assert_eq!(i.doc.malhas.len(), 1);
        assert!((sem_erro(&i.doc) - 1000.).abs() < 1e-3, "{}", f.nome());
        assert_eq!((i.relatorio.corpos, i.relatorio.solidos), (1, 1), "{}", f.nome());
        assert!((i.relatorio.volume - 1000.).abs() < 1e-3);
        let (lo, hi) = i.relatorio.caixa.unwrap();
        assert!((hi[0] - lo[0] - 10.).abs() < 1e-3);
    }
}

#[test]
fn documento_com_corpo_de_malha_abre_sem_o_arquivo_e_salva_em_cada_formato() {
    let dir = pasta("salva");
    let a = dir.join("origem.stl");
    fenkra_conversor::malha::gravar_para_teste(Formato::Stl, &cena(vec![caixa("Peça", 10., 0.)]), &a).unwrap();
    let i = Registro::default().importar(&a, &uma_peca()).unwrap();
    std::fs::remove_file(&a).unwrap();
    let texto = serde_json::to_string(&i.doc).unwrap();
    let d: Document = serde_json::from_str(&texto).unwrap();
    assert!((sem_erro(&d) - 1000.).abs() < 1e-3, "o arquivo já não existe");
    for f in [Formato::Stl, Formato::Obj, Formato::Ply, Formato::Tmf, Formato::Glb] {
        let b = dir.join(format!("saída.{}", f.extensoes()[0]));
        Registro::default().gravar_documento(&d, &b, &OpcoesExportar::default()).unwrap_or_else(|e| panic!("{}: {e}", f.nome()));
        let volta = fenkra_conversor::malha::ler_para_teste(f, &b).unwrap();
        assert_eq!(volta.no_lugar().iter().map(|m| m.tris.len()).sum::<usize>(), 12, "{}", f.nome());
    }
    let e = Registro::default().gravar_documento(&d, &dir.join("x.step"), &OpcoesExportar::default()).unwrap_err();
    assert!(e.to_string().contains("sólido"), "{e}");
}

#[test]
fn partes_fechadas_viram_corpos_mas_a_cavidade_fica_com_a_peca() {
    let dir = pasta("partes");
    let a = dir.join("duas.stl");
    let duas = fenkra_malha::juntar([&caixa("", 10., 0.), &caixa("", 5., 50.)]);
    fenkra_conversor::malha::gravar_para_teste(Formato::Stl, &cena(vec![duas]), &a).unwrap();
    let i = Registro::default().importar(&a, &uma_peca()).unwrap();
    assert_eq!(i.relatorio.corpos, 2);
    assert_eq!(i.doc.collectors.len(), 2);
    assert!((sem_erro(&i.doc) - 1125.).abs() < 1e-3);

    // cubo oco: a casca de dentro aponta para o miolo e tira volume
    let mut dentro = fenkra_malha::cena::cubo("", 10.);
    dentro.pos.iter_mut().for_each(|p| *p = p.map(|v| v + 10.));
    dentro.tris.iter_mut().for_each(|t| t.swap(1, 2));
    let oca = fenkra_malha::juntar([&fenkra_malha::cena::cubo("", 30.), &dentro]);
    let a = dir.join("oca.stl");
    fenkra_conversor::malha::gravar_para_teste(Formato::Stl, &cena(vec![oca]), &a).unwrap();
    let i = Registro::default().importar(&a, &uma_peca()).unwrap();
    assert_eq!(i.relatorio.corpos, 1, "a cavidade não é um corpo");
    assert!((sem_erro(&i.doc) - 26000.).abs() < 1e-3);
}

#[test]
fn malha_aberta_vira_corpo_e_o_relatorio_avisa_e_o_reparo_fecha() {
    let dir = pasta("aberta");
    let mut m = caixa("Tampa", 10., 0.);
    m.tris.truncate(10);
    let a = dir.join("aberta.obj");
    fenkra_conversor::malha::gravar_para_teste(Formato::Obj, &cena(vec![m]), &a).unwrap();
    let mut i = Registro::default().importar(&a, &uma_peca()).unwrap();
    assert_eq!(i.relatorio.solidos, 0);
    assert_eq!(i.relatorio.nao_fecharam.len(), 1);
    assert_eq!(sem_erro(&i.doc), 0.);
    i.doc.add_feature(FeatureKind::MeshOp(MeshOp { op: MeshOpKind::Reparar(MeshRepair::default()) }));
    assert!((sem_erro(&i.doc) - 1000.).abs() < 1e-3);
    let corpo = corpos_de_malha(&i.doc);
    assert!(corpo[0].entrada.diagnostico.estanque());
}

#[test]
fn montagem_acima_do_teto_entra_achatada_e_avisa() {
    // a mesma malha repetida: dois nós com árvore, e a soma dos triângulos passa do teto
    let n = 330usize;
    let mut g = Malha { nome: "Grade".into(), ..Default::default() };
    for j in 0..=n {
        for i in 0..=n {
            g.pos.push([i as f64, j as f64, 0.]);
        }
    }
    let k = |i: usize, j: usize| (j * (n + 1) + i) as u32;
    for j in 0..n {
        for i in 0..n {
            g.tris.push([k(i, j), k(i + 1, j), k(i + 1, j + 1)]);
            g.tris.push([k(i, j), k(i + 1, j + 1), k(i, j + 1)]);
        }
    }
    let mut c = Cena { malhas: vec![g, caixa("Pino", 5., 0.)], ..Default::default() };
    for (i, nome) in ["Grade", "Pino"].iter().enumerate() {
        let mut no = No::novo(nome);
        no.malhas = vec![i];
        c.nos.push(no);
        c.raizes.push(i);
    }
    let dir = pasta("teto");
    let a = dir.join("conjunto.3mf");
    fenkra_conversor::malha::gravar_para_teste(Formato::Tmf, &c, &a).unwrap();
    let i = Registro::default().importar(&a, &OpcoesImportar::default()).unwrap();
    assert!(i.conjunto.is_none());
    assert_eq!(i.relatorio.corpos, 2);
    assert!(i.relatorio.nao_carrega.iter().any(|x| x.modelo.to_lowercase().contains("montagem")), "{:?}", i.relatorio.nao_carrega);
}
