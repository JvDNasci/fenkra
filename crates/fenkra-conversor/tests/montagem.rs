// SPDX-License-Identifier: LGPL-2.1-or-later
//! B105: montagem neutra, com a peça repetida em um documento só, PN pelo motor e a volta para o STEP.

use fenkra_conversor::arvore::gravar_em_pasta;
use fenkra_conversor::{Destino, Estrutura, Formato, ModeloNeutro, ModoIges, OpcoesExportar, OpcoesImportar, Registro};
use fenkra_kernel::{InstanciaXde, ModeloXde, ProdutoXde, Shape, IDENTIDADE};
use fenkra_montagem::{Assembly, NodeKind};
use fenkra_project::Project;
use std::path::PathBuf;

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra montagem açaí {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn caixa(l: [f64; 3]) -> Shape {
    Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], l).unwrap()
}

fn em(x: f64, y: f64) -> [f64; 12] {
    [1., 0., 0., x, 0., 1., 0., y, 0., 0., 1., 0.]
}

/// Quarto de volta em Z, deslocado.
fn girado(x: f64, y: f64) -> [f64; 12] {
    [0., -1., 0., x, 1., 0., 0., y, 0., 0., 1., 0.]
}

fn inst(produto: usize, nome: &str, local: [f64; 12]) -> InstanciaXde {
    InstanciaXde { produto, nome: nome.into(), local, cor: None }
}

/// Base, quatro pinos iguais (dois deles num subconjunto, um girado) e a raiz.
fn modelo() -> ModeloNeutro {
    let base = ProdutoXde { nome: "Base".into(), forma: Some(caixa([100., 60., 10.])), cor: Some([0.8, 0.2, 0.2, 1.]), ..Default::default() };
    let pino = ProdutoXde { nome: "Pino".into(), forma: Some(caixa([10., 20., 40.])), cor: Some([0.2, 0.2, 0.8, 1.]), ..Default::default() };
    let sub = ProdutoXde { nome: "Par".into(), filhos: vec![inst(1, "Pino 3", em(0., 0.)), inst(1, "Pino 4", girado(30., 0.))], ..Default::default() };
    let topo = ProdutoXde {
        nome: "Conjunto".into(),
        filhos: vec![inst(0, "Base 1", IDENTIDADE), inst(1, "Pino 1", em(5., 5.)), inst(1, "Pino 2", em(80., 5.)), inst(2, "Par 1", em(40., 30.))],
        ..Default::default()
    };
    ModeloNeutro { formato: Formato::Step, xde: ModeloXde { produtos: vec![base, pino, sub, topo], raizes: vec![3], ..Default::default() } }
}

const VOLUME: f64 = 60_000. + 4. * 8_000.;

fn importado(dir: &std::path::Path) -> (PathBuf, fenkra_conversor::Importado) {
    let arquivo = dir.join("conjunto de teste.step");
    Registro::default().gravar(&modelo(), &arquivo, &OpcoesExportar::default()).unwrap();
    let i = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
    (arquivo, i)
}

fn peca_de(a: &Assembly, nome: &str) -> String {
    let n = a.nodes.iter().find(|n| n.name == nome).unwrap_or_else(|| panic!("sem {nome}"));
    let NodeKind::Part { path } = &n.kind else { panic!("{nome} não é peça") };
    path.clone()
}

#[test]
fn peca_repetida_vira_um_documento_e_n_componentes() {
    let dir = pasta("uma");
    let (_, i) = importado(&dir);
    let c = i.conjunto.as_ref().expect("conjunto");
    assert_eq!(c.pecas.len(), 2, "uma peça por produto: {:?}", c.pecas.iter().map(|p| &p.0).collect::<Vec<_>>());
    let a = c.montagem.as_ref().unwrap();
    assert_eq!(a.name, "Conjunto");
    let partes = a.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Part { .. })).count();
    assert_eq!(partes, 5);
    let pino = peca_de(a, "Pino 1");
    for nome in ["Pino 2", "Pino 3", "Pino 4"] {
        assert_eq!(peca_de(a, nome), pino, "as ocorrências apontam para o mesmo arquivo");
    }
    assert_ne!(peca_de(a, "Base 1"), pino);

    // o B-rep da peça existe uma vez: duas formas guardadas, contra cinco no documento com cópias
    let copias = Registro::default().importar(&dir.join("conjunto de teste.step"), &OpcoesImportar { destino: Destino::UmaPeca, ..Default::default() }).unwrap();
    assert!(copias.conjunto.is_none());
    let formas = |d: &fenkra_core::Document| d.to_json().matches("CASCADE Topology").count();
    assert_eq!(c.pecas.iter().map(|(_, d)| formas(d)).sum::<usize>(), 2);
    assert_eq!(formas(&copias.doc), 5);

    // subconjunto vira grupo, e a posição dentro dele já vem somada
    let par = a.nodes.iter().find(|n| matches!(n.kind, NodeKind::Group)).expect("grupo do subconjunto");
    assert_eq!(par.name, "Par 1");
    let p4 = a.nodes.iter().find(|n| n.name == "Pino 4").unwrap();
    assert_eq!(p4.parent, Some(par.id));
    assert!((p4.placement.origin[0] - 70.).abs() < 1e-6 && (p4.placement.origin[1] - 30.).abs() < 1e-6, "{:?}", p4.placement);
    assert!((p4.placement.rot[0][1] + 1.).abs() < 1e-6, "o giro da ocorrência fica: {:?}", p4.placement.rot);
    assert!(a.nodes.iter().find(|n| n.name == "Base 1").unwrap().fixed, "a primeira peça é a referência");
    assert!(!i.relatorio.texto_pt().contains("Estrutura de montagem"), "a estrutura agora chega");
    assert_eq!(i.relatorio.corpos, 2);
}

#[test]
fn cada_peca_recalcula_e_a_cor_vem_do_produto() {
    let dir = pasta("peca");
    let (_, i) = importado(&dir);
    let c = i.conjunto.unwrap();
    let (_, pino) = c.pecas.iter().find(|(n, _)| n.starts_with("Pino")).unwrap();
    let r = fenkra_solidos::recompute(pino);
    assert!(!r.status.values().any(|s| matches!(s, fenkra_solidos::Status::Error(_))), "{:?}", r.status);
    assert_eq!(r.bodies.len(), 1);
    assert!((r.bodies[0].mass.volume - 8_000.).abs() < 1e-3);
    assert!(pino.collectors[0].color[2] > 150, "{:?}", pino.collectors[0].color);
    assert_eq!(pino.name, "Pino");
    assert!(pino.props.pn.is_empty(), "o PN vem do motor, no projeto");
}

#[test]
fn step_com_uma_peca_embrulhada_continua_um_documento() {
    let dir = pasta("embrulho");
    let arquivo = dir.join("um.step");
    let so = ProdutoXde { nome: "Só".into(), forma: Some(caixa([1., 2., 3.])), ..Default::default() };
    let topo = ProdutoXde { nome: "Raiz".into(), filhos: vec![inst(0, "x", IDENTIDADE)], ..Default::default() };
    let m = ModeloNeutro { formato: Formato::Step, xde: ModeloXde { produtos: vec![so, topo], raizes: vec![1], ..Default::default() } };
    Registro::default().gravar(&m, &arquivo, &OpcoesExportar::default()).unwrap();
    assert!(Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap().conjunto.is_none());
}

#[test]
fn pn_pelo_motor_uma_vez_por_peca_e_a_montagem_acha_as_pecas_no_projeto() {
    let dir = pasta("pn");
    let (arquivo, i) = importado(&dir);
    let mut c = i.conjunto.unwrap();
    let raiz = dir.join("projeto");
    std::fs::create_dir_all(&raiz).unwrap();
    let mut p = Project::create(&raiz, "Projeto").unwrap();
    let antes = p.pn_previsto();
    let pecas = std::mem::take(&mut c.pecas);
    let novos = p.adicionar_pecas_importadas(&arquivo, pecas, "relatório").unwrap();
    assert_eq!(novos.len(), 2);
    assert_eq!(p.index.documents.len(), 2, "cinco ocorrências, dois documentos");
    let pns: Vec<&str> = p.index.documents.iter().map(|d| d.pn.as_str()).collect();
    assert_eq!(pns[0], antes, "o primeiro PN é o que o motor previa");
    assert_ne!(pns[0], pns[1]);
    assert!(pns.iter().all(|n| !n.is_empty()));

    c.reapontar(&novos.into_iter().collect());
    let a = c.montagem.as_ref().unwrap();
    let nome = c.arquivo_da_montagem().unwrap();
    let fka = p.gravar_montagem_importada(&nome, &a.to_json()).unwrap();
    assert_eq!(fka, p.montagem_importada(&nome));

    // a montagem gravada no projeto abre e leva às peças de verdade
    let lida = Assembly::from_json(&std::fs::read_to_string(&fka).unwrap()).unwrap();
    let m = ModeloNeutro::da_montagem(&lida, fka.parent().unwrap()).unwrap();
    assert_eq!(m.ocorrencias().len(), 5);
    let onde = p.onde_usado(p.index.documents[1].id).unwrap();
    assert_eq!(onde.len(), 4, "a peça repetida aparece nos quatro nós: {onde:?}");
    assert!(p.relatorio_no_cofre(&lida_hash(&p)).is_file());
}

fn lida_hash(p: &Project) -> String {
    p.index.documents[0].origem.as_ref().map(|o| o.hash.clone()).unwrap()
}

/// Importa, grava a pasta, relê a montagem e devolve o modelo refeito dos arquivos.
fn refeito(dir: &std::path::Path) -> ModeloNeutro {
    let (_, i) = importado(dir);
    let pecas = dir.join("pecas");
    std::fs::create_dir_all(&pecas).unwrap();
    let fka = gravar_em_pasta(&i.conjunto.unwrap(), &pecas).unwrap();
    let a = Assembly::from_json(&std::fs::read_to_string(&fka).unwrap()).unwrap();
    ModeloNeutro::da_montagem(&a, &pecas).unwrap()
}

#[test]
fn exportar_com_estrutura_devolve_produtos_e_instancias() {
    let dir = pasta("estrutura");
    let m = refeito(&dir);
    let saida = dir.join("volta.step");
    Registro::default().gravar(&m, &saida, &OpcoesExportar::default()).unwrap();
    let i = Registro::default().importar(&saida, &OpcoesImportar::default()).unwrap();
    let r = &i.relatorio;
    assert_eq!(r.produtos, 4, "base, pino, grupo e raiz: {r:?}");
    assert_eq!(r.instancias, 6, "{r:?}");
    assert_eq!(r.solidos, 5);
    assert!((r.volume - VOLUME).abs() < 1e-3, "{}", r.volume);
    let (lo, hi) = r.caixa.unwrap();
    assert!((hi[0] - 100.).abs() < 1e-3 && (hi[1] - 60.).abs() < 1e-3 && (hi[2] - 40.).abs() < 1e-3 && lo[0].abs() < 1e-3, "{lo:?} {hi:?}");
    assert_eq!(i.conjunto.unwrap().pecas.len(), 2, "a peça repetida foi uma vez só");
}

#[test]
fn exportar_achatada_grava_um_produto_com_tudo_no_lugar() {
    let dir = pasta("achatada");
    let m = refeito(&dir);
    let o = OpcoesExportar { estrutura: Estrutura::Achatada, ..Default::default() };
    let saida = dir.join("plana.step");
    Registro::default().gravar(&m, &saida, &o).unwrap();
    let i = Registro::default().importar(&saida, &OpcoesImportar::default()).unwrap();
    let r = &i.relatorio;
    assert_eq!((r.produtos, r.instancias, r.solidos), (1, 0, 5), "{r:?}");
    assert!(i.conjunto.is_none());
    assert!((r.volume - VOLUME).abs() < 1e-3, "{}", r.volume);
    let (_, hi) = r.caixa.unwrap();
    assert!((hi[0] - 100.).abs() < 1e-3 && (hi[1] - 60.).abs() < 1e-3, "{hi:?}");
    assert!(r.texto_pt().contains("Cor por face"), "a cor de cada ocorrência viaja na face: {}", r.texto_pt());

    let igs = dir.join("plana.igs");
    Registro::default().gravar(&m, &igs, &OpcoesExportar { iges: ModoIges::Brep, ..o }).unwrap();
    assert_eq!(Registro::default().importar(&igs, &OpcoesImportar::default()).unwrap().relatorio.solidos, 5);
}

#[test]
fn montagem_que_contem_a_si_mesma_e_peca_sumida_viram_erro() {
    let dir = pasta("ciclo");
    let mut a = Assembly::new("A");
    a.add_component("eu", NodeKind::SubAssembly { path: "a.fka".into() }, Default::default(), None);
    std::fs::write(dir.join("a.fka"), a.to_json()).unwrap();
    assert!(ModeloNeutro::da_montagem(&a, &dir).is_err());
    let mut b = Assembly::new("B");
    b.add_component("x", NodeKind::Part { path: "sumiu.fkp".into() }, Default::default(), None);
    assert!(ModeloNeutro::da_montagem(&b, &dir).is_err());
}
