// SPDX-License-Identifier: LGPL-2.1-or-later
//! `fenkra-cli`: comandos do Fenkra sem interface. Toda resposta é um JSON na saída padrão.

use fenkra_cli::pasta::Pasta;
use fenkra_cli::{envelope, filho, saida, ErroApi, Isolamento, Sessao, API_VERSAO};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

const AJUDA: &str = "\
uso: fenkra-cli [--pasta DIR] COMANDO [argumentos]

  versao | comandos
  recalcular PECA               status de cada item, volume e caixa
  medir PECA                    volume, área, centroide, inércia e caixa
  parametros PECA
  definir PECA NOME EXPR --saida ARQ [--sobrescrever]
  exportar PECA SAIDA.step [--sobrescrever]
  topologia ARQ | faces ARQ | cilindros ARQ | planos ARQ | caixa ARQ
  features ARQ                  features inferidas (furos, filetes, chanfros)
  comparar ALVO ARQ             checador de similaridade
  reproduzir REGISTRO.jsonl --saida ARQ.fkx [--sobrescrever]
  visualizar ARQ --saida X.fkv [--html X.html] [--decimar F]
                                pacote de visualização para o navegador (.fkp, .fka ou .fkd)
  servir                        um pedido JSON por linha na entrada padrão

PECA é .fkx ou .fkp; ARQ pode ser também .step. Todo caminho fica dentro de --pasta (padrão: a pasta
atual). Nada é sobrescrito sem --sobrescrever. Saída: 0 ok, 1 erro do comando, 2 uso errado.
";

struct Linha {
    pasta: String,
    sobrescrever: bool,
    saida: Option<String>,
    html: Option<String>,
    decimar: Option<f64>,
    resto: Vec<String>,
}

fn ler_argumentos() -> Result<Linha, String> {
    let mut l = Linha {
        pasta: ".".into(),
        sobrescrever: false,
        saida: None,
        html: None,
        decimar: None,
        resto: vec![],
    };
    let mut a = std::env::args().skip(1);
    while let Some(x) = a.next() {
        match x.as_str() {
            "--pasta" => l.pasta = a.next().ok_or("--pasta pede um caminho")?,
            "--saida" => l.saida = Some(a.next().ok_or("--saida pede um caminho")?),
            "--html" => l.html = Some(a.next().ok_or("--html pede um caminho")?),
            "--decimar" => {
                let v = a.next().ok_or("--decimar pede uma fração")?;
                l.decimar = Some(v.replace(',', ".").parse().map_err(|_| "--decimar pede um número de 0,01 a 1")?);
            }
            "--sobrescrever" => l.sobrescrever = true,
            _ if x.starts_with("--") => return Err(format!("opção desconhecida: {x}")),
            _ => l.resto.push(x),
        }
    }
    Ok(l)
}

/// Uma linha de comando vira uma sequência de comandos da sessão; a resposta é a do último.
fn plano(l: &Linha) -> Result<Vec<(&'static str, Value)>, String> {
    let r: Vec<&str> = l.resto.iter().map(String::as_str).collect();
    let abrir = |p: &str| ("abrir", json!({ "arquivo": p }));
    let saida = || l.saida.clone().ok_or("falta --saida");
    let topologia = |a: &str, s: &[&str]| vec![("topologia", json!({ "arquivo": a, "secoes": s }))];
    Ok(match r.as_slice() {
        ["versao"] => vec![("versao", json!({}))],
        ["comandos"] => vec![("comandos", json!({}))],
        ["recalcular", p] => vec![abrir(p), ("recalcular", json!({}))],
        ["medir", p] => vec![abrir(p), ("medir", json!({}))],
        ["parametros", p] => vec![abrir(p), ("parametros", json!({}))],
        ["definir", p, nome, expr] => vec![
            abrir(p),
            (
                "definir_parametro",
                json!({ "nome": nome, "expressao": expr }),
            ),
            (
                "salvar",
                json!({ "arquivo": saida()?, "sobrescrever": l.sobrescrever }),
            ),
        ],
        ["exportar", p, s] => vec![
            abrir(p),
            (
                "exportar",
                json!({ "arquivo": s, "sobrescrever": l.sobrescrever }),
            ),
        ],
        ["topologia", a] => vec![("topologia", json!({ "arquivo": a }))],
        ["faces", a] => topologia(a, &["faces"]),
        ["cilindros", a] => topologia(a, &["cilindros"]),
        ["planos", a] => topologia(a, &["planos"]),
        ["caixa", a] => topologia(a, &["caixa"]),
        ["features", a] => vec![("features", json!({ "arquivo": a }))],
        ["comparar", alvo, a] => vec![("comparar", json!({ "alvo": alvo, "arquivo": a }))],
        ["reproduzir", a] => vec![
            ("reproduzir", json!({ "arquivo": a })),
            (
                "salvar",
                json!({ "arquivo": saida()?, "sobrescrever": l.sobrescrever }),
            ),
        ],
        _ => return Err("comando ou argumentos errados".into()),
    })
}

fn escrever(alvo: &mut impl Write, v: &Value) -> std::io::Result<()> {
    writeln!(alvo, "{v}")?;
    alvo.flush()
}

/// Modo de serviço, usado pela API Python: `{"id": ..., "comando": ..., "args": {...}}` por linha.
fn servir(mut sessao: Sessao, alvo: &mut impl Write) -> std::io::Result<()> {
    for linha in std::io::stdin().lock().lines() {
        let linha = linha?;
        // o PowerShell abre o tubo com marca de ordem de bytes, e a primeira linha se perderia
        let linha = linha.trim_start_matches('\u{feff}');
        if linha.trim().is_empty() {
            continue;
        }
        let (id, resposta) = match serde_json::from_str::<Value>(linha) {
            Ok(p) => {
                let comando = p["comando"].as_str().unwrap_or_default().to_string();
                (p["id"].clone(), sessao.executar(&comando, &p["args"]))
            }
            Err(e) => (
                Value::Null,
                Err(ErroApi::argumento(format!("pedido não é JSON: {e}"))),
            ),
        };
        let mut v = envelope(resposta);
        v["id"] = id;
        escrever(alvo, &v)?;
    }
    Ok(())
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some(filho::ARGUMENTO) {
        std::process::exit(filho::principal());
    }
    let l = match ler_argumentos() {
        Ok(l) => l,
        Err(m) => {
            eprintln!("{m}\n\n{AJUDA}");
            std::process::exit(2);
        }
    };
    if l.resto.is_empty() || l.resto[0] == "ajuda" {
        print!("{AJUDA}");
        std::process::exit(if l.resto.is_empty() { 2 } else { 0 });
    }
    let mut alvo = match saida::reservar_stdout() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("saída padrão indisponível: {e}");
            std::process::exit(1);
        }
    };
    let isolamento = std::env::current_exe().map_or(Isolamento::Local, Isolamento::Processo);
    let mut sessao = match Pasta::nova(&l.pasta) {
        Ok(p) => Sessao::nova(p, isolamento),
        Err(e) => {
            let _ = escrever(&mut alvo, &envelope(Err(e)));
            std::process::exit(1);
        }
    };
    if l.resto[0] == "servir" {
        std::process::exit(i32::from(servir(sessao, &mut alvo).is_err()));
    }
    if let ["visualizar", arquivo] = l.resto.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        let Some(saida_fkv) = l.saida.as_deref() else {
            eprintln!("falta --saida

{AJUDA}");
            std::process::exit(2);
        };
        let pasta = match Pasta::nova(&l.pasta) {
            Ok(p) => p,
            Err(e) => {
                let _ = escrever(&mut alvo, &envelope(Err(e)));
                std::process::exit(1);
            }
        };
        let r = fenkra_cli::visualizar::visualizar(
            &pasta,
            &fenkra_cli::visualizar::Pedido { arquivo, saida: saida_fkv, html: l.html.as_deref(), decimar: l.decimar, sobrescrever: l.sobrescrever },
        );
        let falhou = r.is_err();
        let _ = escrever(&mut alvo, &envelope(r));
        std::process::exit(i32::from(falhou));
    }
    let passos = match plano(&l) {
        Ok(p) => p,
        Err(m) => {
            eprintln!("{m}\n\n{AJUDA}");
            std::process::exit(2);
        }
    };
    let mut resposta = Ok(json!({ "api": API_VERSAO }));
    for (comando, args) in &passos {
        resposta = sessao.executar(comando, args);
        if resposta.is_err() {
            break;
        }
    }
    let falhou = resposta.is_err();
    let _ = escrever(&mut alvo, &envelope(resposta));
    std::process::exit(i32::from(falhou));
}
