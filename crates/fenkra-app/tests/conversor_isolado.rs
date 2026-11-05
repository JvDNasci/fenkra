// SPDX-License-Identifier: LGPL-2.1-or-later
//! B87: o conversor roda em processo filho. Estes testes usam o binário real `fenkra-conv`.

use fenkra_conversor::processo::{Filho, Resposta, Trabalho};
use fenkra_conversor::{EsquemaSaida, ErroConv, Formato, ModeloNeutro, OpcoesExportar, OpcoesImportar, Registro};
use fenkra_kernel::{InstanciaXde, MassProps, ModeloXde, OpcoesLerXde, ProdutoXde, Shape, IDENTIDADE};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn filho(limite: Duration) -> Filho {
    Filho { programa: env!("CARGO_BIN_EXE_fenkra-conv").into(), limite }
}

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra filho açaí {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Peça com faces curvas: caixa vazada e arredondada, para a massa não ser trivial.
fn peca() -> Shape {
    let bloco = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [40., 30., 20.]).unwrap();
    let furo = Shape::box_tool([10., 10., -1.], [1., 0., 0.], [0., 0., 1.], [7.5, 3.25, 30.]).unwrap();
    bloco.boolean(&furo, fenkra_kernel::Bool::Cut).unwrap().fillet_all(1.5).unwrap()
}

fn modelo() -> ModeloNeutro {
    let p = ProdutoXde { nome: "Bloco".into(), forma: Some(peca()), cor: Some([0.1, 0.6, 0.3, 1.]), ..Default::default() };
    let topo = ProdutoXde {
        nome: "Conjunto".into(),
        filhos: vec![
            InstanciaXde { produto: 0, nome: "Bloco 1".into(), local: IDENTIDADE, cor: None },
            InstanciaXde { produto: 0, nome: "Bloco 2".into(), local: [0., -1., 0., 100., 1., 0., 0., 0., 0., 0., 1., 5.], cor: None },
        ],
        ..Default::default()
    };
    ModeloNeutro { formato: Formato::Step, xde: ModeloXde { produtos: vec![p, topo], raizes: vec![1], ..Default::default() } }
}

fn massa_do_bloco(caminho: &std::path::Path) -> MassProps {
    let m = ModeloXde::ler(caminho.to_str().unwrap(), OpcoesLerXde::default()).unwrap();
    m.produtos.iter().find_map(|p| p.forma.as_ref()).unwrap().mass()
}

fn confere(a: MassProps, b: MassProps) {
    let perto = |x: f64, y: f64| (x - y).abs() <= 1e-6 * x.abs().max(y.abs()).max(1.);
    assert!(perto(a.volume, b.volume), "volume {} x {}", a.volume, b.volume);
    assert!(perto(a.area, b.area), "área {} x {}", a.area, b.area);
    for i in 0..3 {
        assert!(perto(a.center[i], b.center[i]), "centroide {:?} x {:?}", a.center, b.center);
    }
}

#[test]
fn ida_e_volta_pelo_filho_preserva_massa_em_cada_esquema() {
    let dir = pasta("ida");
    let origem = dir.join("origem com espaço.step");
    Registro::default().gravar(&modelo(), &origem, &OpcoesExportar::default()).unwrap();
    let referencia = peca().mass();
    confere(massa_do_bloco(&origem), referencia);
    for esquema in [EsquemaSaida::Ap203, EsquemaSaida::Ap214, EsquemaSaida::Ap242] {
        let destino = dir.join(format!("destino {esquema:?}.step"));
        let t = Trabalho::Converter {
            origem: origem.clone(),
            destino: destino.clone(),
            importar: OpcoesImportar::default(),
            exportar: OpcoesExportar { esquema, ..Default::default() },
        };
        assert_eq!(filho(Filho::LIMITE).executar(t, &AtomicBool::new(false)).unwrap(), Resposta::Convertido);
        confere(massa_do_bloco(&destino), referencia);
    }
}

#[test]
fn importar_pelo_filho_entrega_documento_relatorio_e_miniatura() {
    let dir = pasta("importa");
    let arquivo = dir.join("peça de fora.step");
    Registro::default().gravar(&modelo(), &arquivo, &OpcoesExportar::default()).unwrap();
    let i = filho(Filho::LIMITE).importar(&arquivo, OpcoesImportar::default(), &AtomicBool::new(false)).unwrap();
    assert_eq!((i.relatorio.produtos, i.relatorio.corpos), (2, 2));
    let nomes: Vec<&str> = i.doc.collectors.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(nomes, ["Bloco 1", "Bloco 2"], "a leitura é a primeira coisa que o filho faz");
    assert_eq!(i.doc.origem.as_ref().unwrap().nome, "peça de fora.step");
    assert!(!i.previa.tris.is_empty());
    let r = fenkra_solidos::recompute(&i.doc);
    let total: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
    assert!((total - 2. * peca().mass().volume).abs() < 1e-3, "{total}");
}

#[test]
fn queda_do_filho_vira_erro_e_nao_derruba_quem_chamou() {
    let e = filho(Filho::LIMITE).executar(Trabalho::Queda, &AtomicBool::new(false)).unwrap_err();
    assert!(matches!(e, ErroConv::Queda(_)), "{e:?}");
    assert!(e.to_string().contains("segue aberto"));
}

#[test]
fn filho_travado_para_no_limite_de_tempo() {
    let inicio = Instant::now();
    let e = filho(Duration::from_millis(300)).executar(Trabalho::Trava, &AtomicBool::new(false)).unwrap_err();
    assert!(matches!(e, ErroConv::Tempo(_)), "{e:?}");
    assert!(inicio.elapsed() < Duration::from_secs(10));
}

#[test]
fn cancelar_encerra_o_filho_sem_esperar_o_limite() {
    let cancelar = Arc::new(AtomicBool::new(false));
    let c = cancelar.clone();
    let quem = std::thread::spawn(move || filho(Filho::LIMITE).executar(Trabalho::Trava, &c));
    std::thread::sleep(Duration::from_millis(200));
    let inicio = Instant::now();
    cancelar.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(quem.join().unwrap().unwrap_err(), ErroConv::Cancelado);
    assert!(inicio.elapsed() < Duration::from_secs(2));
}

#[test]
fn arquivo_truncado_ou_inexistente_nao_derruba_quem_chamou() {
    let dir = pasta("truncado");
    let inteiro = dir.join("inteiro.step");
    Registro::default().gravar(&modelo(), &inteiro, &OpcoesExportar::default()).unwrap();
    let bytes = std::fs::read(&inteiro).unwrap();
    let cortado = dir.join("cortado.step");
    std::fs::write(&cortado, &bytes[..bytes.len() / 2]).unwrap();
    let f = filho(Filho::LIMITE);
    // tanto faz recusar, ler parte ou cair: o importante é a resposta chegar aqui
    let _ = f.importar(&cortado, OpcoesImportar::default(), &AtomicBool::new(false));
    let e = f.importar(&dir.join("sumiu.step"), OpcoesImportar::default(), &AtomicBool::new(false)).unwrap_err();
    assert!(matches!(e, ErroConv::Leitura(_)), "{e:?}");
}
