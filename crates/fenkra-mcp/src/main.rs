// SPDX-License-Identifier: LGPL-2.1-or-later
//! `fenkra-mcp`: servidor MCP local. Quem o inicia é a pessoa, na configuração do cliente; o agente
//! não muda nada do que está nesta linha de comando, e é por isso que a pasta e o aceite ficam aqui.

use fenkra_cli::pasta::Pasta;
use fenkra_cli::{filho, saida, Isolamento, Sessao};
use fenkra_mcp::Servidor;

const AJUDA: &str = "\
uso: fenkra-mcp --pasta DIR [--liberar-step]

  --pasta DIR      única pasta que o servidor lê e grava
  --liberar-step   aceite explícito: o que for lido de um STEP pode ir para o modelo do cliente.
                   Use com modelo local, ou se concordar em mandar esses dados ao modelo remoto.
                   Sem isto, consultar_step e comparar_alvo são recusadas.

Fala MCP por stdio, sem rede.
";

fn main() {
    let mut a = std::env::args().skip(1);
    let (mut pasta, mut liberar) = (None, false);
    while let Some(x) = a.next() {
        match x.as_str() {
            filho::ARGUMENTO => std::process::exit(filho::principal()),
            "--pasta" => pasta = a.next(),
            "--liberar-step" => liberar = true,
            _ => {
                eprintln!("opção desconhecida: {x}\n\n{AJUDA}");
                std::process::exit(2);
            }
        }
    }
    let Some(pasta) = pasta else {
        eprintln!("falta --pasta\n\n{AJUDA}");
        std::process::exit(2);
    };
    let pasta = match Pasta::nova(&pasta) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let alvo = match saida::reservar_stdout() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("saída padrão indisponível: {e}");
            std::process::exit(1);
        }
    };
    let isolamento = std::env::current_exe().map_or(Isolamento::Local, Isolamento::Processo);
    let mut servidor = Servidor::novo(Sessao::nova(pasta, isolamento).com_registro(), liberar);
    if let Err(e) = servidor.laco(std::io::stdin().lock(), alvo) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
