// SPDX-License-Identifier: LGPL-2.1-or-later
//! A sequência de comandos que montou a peça, guardada como artefato. O resultado de um agente muda
//! de uma conversa para outra; este arquivo não, e refazê-lo dá a mesma peça sem modelo nenhum.
//!
//! Uma linha JSON por comando, gravada na hora: um servidor que caia mantém tudo o que já foi feito.

use crate::pasta::Pasta;
use crate::{ErroApi, API_VERSAO};
use serde_json::{json, Value};
use std::fs::File;
use std::io::Write;

/// Valor de `formato` na primeira linha do arquivo.
pub const FORMATO: &str = "fenkra-comandos";

pub struct Registro {
    arquivo: File,
    caminho: String,
    linhas: Vec<Value>,
}

impl Registro {
    /// Abre o próximo `comandos/sessao-NNN.jsonl` livre. Sessão anterior nunca é sobrescrita.
    pub fn novo(pasta: &Pasta) -> Result<Registro, ErroApi> {
        for n in 1..100_000 {
            let caminho = format!("comandos/sessao-{n:03}.jsonl");
            if let Some(arquivo) = pasta.criar_novo(&caminho, &["jsonl"])? {
                let mut r = Registro {
                    arquivo,
                    caminho,
                    linhas: vec![],
                };
                r.linha(&json!({ "formato": FORMATO, "format_version": API_VERSAO, "gerador": format!("Fenkra {}", env!("CARGO_PKG_VERSION")) }))?;
                return Ok(r);
            }
        }
        Err(ErroApi::novo(
            "gravacao",
            "sem número livre para o registro de comandos",
        ))
    }

    fn linha(&mut self, v: &Value) -> Result<(), ErroApi> {
        let falha =
            |e: std::io::Error| ErroApi::novo("gravacao", format!("registro de comandos: {e}"));
        writeln!(self.arquivo, "{v}").map_err(falha)?;
        self.arquivo.flush().map_err(falha)
    }

    pub fn anotar(&mut self, comando: &str, args: &Value) -> Result<(), ErroApi> {
        let v = json!({ "comando": comando, "args": args });
        self.linha(&v)?;
        self.linhas.push(v);
        Ok(())
    }

    /// Caminho relativo à pasta do projeto.
    pub fn caminho(&self) -> &str {
        &self.caminho
    }

    pub fn linhas(&self) -> &[Value] {
        &self.linhas
    }
}

/// Lê um registro e devolve os pares (comando, argumentos) na ordem.
pub fn ler_comandos(texto: &str) -> Result<Vec<(String, Value)>, ErroApi> {
    let invalido = |m: String| ErroApi::novo("formato", format!("registro de comandos: {m}"));
    let mut linhas = texto.lines().filter(|l| !l.trim().is_empty());
    let cabecalho: Value = serde_json::from_str(
        linhas
            .next()
            .ok_or_else(|| invalido("arquivo vazio".into()))?,
    )
    .map_err(|e| invalido(e.to_string()))?;
    if cabecalho["formato"] != FORMATO {
        return Err(invalido("o arquivo não é um registro de comandos".into()));
    }
    if cabecalho["format_version"].as_u64() != Some(API_VERSAO as u64) {
        return Err(invalido(format!(
            "versão {} não suportada",
            cabecalho["format_version"]
        )));
    }
    linhas
        .map(|l| {
            let v: Value = serde_json::from_str(l).map_err(|e| invalido(e.to_string()))?;
            let comando = v["comando"]
                .as_str()
                .ok_or_else(|| invalido("linha sem comando".into()))?
                .to_string();
            Ok((comando, v["args"].clone()))
        })
        .collect()
}
