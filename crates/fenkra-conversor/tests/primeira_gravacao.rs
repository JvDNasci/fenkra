// SPDX-License-Identifier: LGPL-2.1-or-later
//! A primeira gravação de um processo tem de sair igual às seguintes. Este arquivo vira um executável
//! próprio e tem um teste só, para nada do OpenCASCADE rodar antes: sem registrar os padrões do STEP,
//! o arquivo saía sem nomes nem camadas e com cada ocorrência como uma peça nova.

use fenkra_kernel::{InstanciaXde, ModeloXde, OpcoesGravarXde, OpcoesLerXde, ProdutoXde, Shape};

fn em(x: f64) -> [f64; 12] {
    [1., 0., 0., x, 0., 1., 0., 0., 0., 0., 1., 0.]
}

#[test]
fn primeira_gravacao_do_processo_leva_nomes_camadas_e_definicao_unica() {
    let caixa = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [10., 10., 40.]).unwrap();
    let pino = ProdutoXde { nome: "Pino".into(), forma: Some(caixa), camadas: vec!["Fixação".into()], ..Default::default() };
    let topo = ProdutoXde {
        nome: "Conjunto".into(),
        filhos: vec![
            InstanciaXde { produto: 0, nome: "Pino 1".into(), local: em(0.), cor: None },
            InstanciaXde { produto: 0, nome: "Pino 2".into(), local: em(50.), cor: None },
        ],
        ..Default::default()
    };
    let m = ModeloXde { produtos: vec![pino, topo], raizes: vec![1], ..Default::default() };

    let dir = std::env::temp_dir().join(format!("Fenkra primeira gravação {}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let alvo = dir.join("par.step");
    let caminho = alvo.to_str().unwrap();
    m.gravar(caminho, OpcoesGravarXde::default()).unwrap();

    let lido = ModeloXde::ler(caminho, OpcoesLerXde::default()).unwrap();
    assert_eq!(lido.produtos.len(), 2, "{:?}", lido.produtos.iter().map(|p| &p.nome).collect::<Vec<_>>());
    let pino = lido.produtos.iter().find(|p| p.forma.is_some()).unwrap();
    assert_eq!(pino.nome, "Pino");
    assert_eq!(pino.camadas, vec!["Fixação".to_string()]);
    std::fs::remove_dir_all(dir).unwrap();
}
