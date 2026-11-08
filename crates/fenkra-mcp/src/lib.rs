// SPDX-License-Identifier: LGPL-2.1-or-later
//! Servidor MCP do Fenkra (B287): um agente reconstrói peças por comandos, sobre [`fenkra_cli`].
//!
//! Só local: mensagens JSON-RPC, uma por linha, em stdio, sem nada de rede no programa. As ferramentas
//! são a lista fechada de [`ferramentas::FERRAMENTAS`], sem execução de código, e cada caminho passa
//! pela pasta que a pessoa informou ao iniciar.
//!
//! O dado de um STEP é o que há de sensível aqui. O servidor nunca sabe se o modelo do outro lado é
//! local, e tudo o que uma ferramenta devolve vai para esse modelo. Por isso as duas ferramentas que
//! leem STEP ficam recusadas até a pessoa dar o aceite em `--liberar-step`, que o agente não alcança.

pub mod ferramentas;

use fenkra_cli::{ErroApi, Sessao};
use ferramentas::{Ferramenta, FERRAMENTAS};
use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};

/// Versões com aperto de mão `initialize`, da mais nova para a mais velha.
pub const VERSOES_INICIALIZADAS: [&str; 4] =
    ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
/// Versão em que cada pedido declara a própria versão em `_meta`.
pub const VERSAO_POR_PEDIDO: &str = "2026-07-28";
const CHAVE_VERSAO: &str = "io.modelcontextprotocol/protocolVersion";
const CHAVE_SERVIDOR: &str = "io.modelcontextprotocol/serverInfo";

/// Uma mensagem maior que isto é recusada: nenhum comando legítimo chega perto.
pub const LIMITE_LINHA: u64 = 16 * 1024 * 1024;

const RECURSOS: [(&str, &str, &str); 3] = [
    (
        "fenkra://arvore",
        "Árvore",
        "A peça em edição, no formato aberto .fkx",
    ),
    (
        "fenkra://relatorio",
        "Relatório",
        "Status de cada item, medidas e a última comparação com o alvo",
    ),
    (
        "fenkra://comandos",
        "Comandos",
        "A sequência de comandos que montou a peça, guardada como artefato",
    ),
];

const INSTRUCOES: &str = "Reconstrói peças do Fenkra por comandos. Fluxo: consultar_step no alvo, nova_peca, criar_esboco com cotas, extrudar, furar, filetar, recalcular, comparar_alvo; corrija pelo relatório e repita. Medidas em mm, ângulos em graus. Os caminhos são relativos à pasta do projeto.";

fn erro_rpc(id: Value, codigo: i64, mensagem: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": codigo, "message": mensagem } })
}

fn ok_rpc(id: Value, resultado: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": resultado })
}

pub struct Servidor {
    sessao: Sessao,
    liberar_step: bool,
}

impl Servidor {
    pub fn novo(sessao: Sessao, liberar_step: bool) -> Servidor {
        Servidor {
            sessao,
            liberar_step,
        }
    }

    fn info() -> Value {
        json!({ "name": "fenkra", "title": "Fenkra", "version": env!("CARGO_PKG_VERSION") })
    }

    fn capacidades() -> Value {
        json!({ "tools": {}, "resources": {} })
    }

    fn inicializar(params: &Value) -> Value {
        let pedida = params["protocolVersion"].as_str().unwrap_or_default();
        let versao = VERSOES_INICIALIZADAS
            .iter()
            .find(|v| **v == pedida)
            .copied()
            .unwrap_or(VERSOES_INICIALIZADAS[0]);
        json!({ "protocolVersion": versao, "capabilities": Self::capacidades(), "serverInfo": Self::info(), "instructions": INSTRUCOES })
    }

    fn descobrir() -> Value {
        let mut versoes = vec![VERSAO_POR_PEDIDO];
        versoes.extend(VERSOES_INICIALIZADAS);
        json!({ "supportedVersions": versoes, "capabilities": Self::capacidades(), "_meta": { CHAVE_SERVIDOR: Self::info() }, "instructions": INSTRUCOES })
    }

    /// Resposta de ferramenta. Falha do comando é resultado com `isError`: o modelo lê e corrige.
    fn resultado_ferramenta(r: Result<Value, ErroApi>) -> Value {
        match r {
            Ok(v) => {
                json!({ "content": [{ "type": "text", "text": v.to_string() }], "structuredContent": v, "isError": false })
            }
            Err(e) => {
                let v = json!({ "codigo": e.codigo, "mensagem": e.mensagem, "sugestao": e.sugestao() });
                json!({ "content": [{ "type": "text", "text": v.to_string() }], "structuredContent": v, "isError": true })
            }
        }
    }

    fn chamar(&mut self, f: &Ferramenta, args: &Value) -> Result<Value, ErroApi> {
        if f.le_step && !self.liberar_step {
            return Err(ErroApi::novo("step_bloqueado", "ler STEP pede o aceite da pessoa: ela precisa iniciar o servidor com --liberar-step. Os dados do arquivo não saíram daqui."));
        }
        let args = (f.traduzir)(args)?;
        self.sessao.executar(f.comando, &args)
    }

    fn ferramenta(&mut self, params: &Value) -> Result<Value, (i64, String)> {
        let nome = params["name"].as_str().unwrap_or_default();
        let f = FERRAMENTAS
            .iter()
            .find(|f| f.nome == nome)
            .ok_or((-32602, format!("ferramenta desconhecida: {nome}")))?;
        let args = if params["arguments"].is_null() {
            json!({})
        } else {
            params["arguments"].clone()
        };
        Ok(Self::resultado_ferramenta(self.chamar(f, &args)))
    }

    fn ler_recurso(&mut self, params: &Value) -> Result<Value, (i64, String)> {
        let uri = params["uri"].as_str().unwrap_or_default();
        let conteudo = match uri {
            "fenkra://arvore" => self.sessao.executar("arvore", &json!({})),
            "fenkra://relatorio" => self.sessao.executar("relatorio", &json!({})),
            "fenkra://comandos" => Ok(self.sessao.registro_json()),
            _ => return Err((-32002, format!("recurso desconhecido: {uri}"))),
        };
        let v = conteudo.unwrap_or_else(|e| json!({ "codigo": e.codigo, "mensagem": e.mensagem, "sugestao": e.sugestao() }));
        Ok(
            json!({ "contents": [{ "uri": uri, "mimeType": "application/json", "text": v.to_string() }] }),
        )
    }

    /// Atende uma mensagem. Notificação não tem resposta.
    pub fn responder(&mut self, msg: &Value) -> Option<Value> {
        if !msg.is_object() {
            return Some(erro_rpc(
                Value::Null,
                -32600,
                "mensagem inválida: lote não é aceito",
            ));
        }
        let metodo = msg["method"].as_str();
        let Some(id) = msg.get("id").cloned() else {
            return None; // notificação, ou resposta a um pedido que este servidor nunca faz
        };
        let Some(metodo) = metodo else {
            return Some(erro_rpc(id, -32600, "pedido sem método"));
        };
        let params = &msg["params"];
        let declarada = params["_meta"][CHAVE_VERSAO].as_str();
        if let Some(v) = declarada {
            if v != VERSAO_POR_PEDIDO && !VERSOES_INICIALIZADAS.contains(&v) {
                let mut suportadas = vec![VERSAO_POR_PEDIDO];
                suportadas.extend(VERSOES_INICIALIZADAS);
                return Some(
                    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32022, "message": "Unsupported protocol version", "data": { "supported": suportadas, "requested": v } } }),
                );
            }
        }
        let resultado = match metodo {
            "initialize" => Ok(Self::inicializar(params)),
            "server/discover" => Ok(Self::descobrir()),
            "ping" => Ok(json!({})),
            "tools/list" => {
                Ok(json!({ "tools": FERRAMENTAS.iter().map(Ferramenta::json).collect::<Vec<_>>() }))
            }
            "tools/call" => self.ferramenta(params),
            "resources/list" => Ok(
                json!({ "resources": RECURSOS.iter().map(|(uri, nome, descricao)| json!({ "uri": uri, "name": nome, "description": descricao, "mimeType": "application/json" })).collect::<Vec<_>>() }),
            ),
            "resources/read" => self.ler_recurso(params),
            _ => Err((-32601, format!("método desconhecido: {metodo}"))),
        };
        Some(match resultado {
            Ok(mut v) => {
                if declarada == Some(VERSAO_POR_PEDIDO) || metodo == "server/discover" {
                    v["resultType"] = json!("complete");
                }
                ok_rpc(id, v)
            }
            Err((codigo, m)) => erro_rpc(id, codigo, &m),
        })
    }

    /// Atende até a entrada fechar, que é como o cliente encerra um servidor stdio.
    pub fn laco(
        &mut self,
        mut entrada: impl BufRead,
        mut saida: impl Write,
    ) -> std::io::Result<()> {
        let mut bytes = Vec::new();
        loop {
            bytes.clear();
            let lidos = (&mut entrada)
                .take(LIMITE_LINHA + 1)
                .read_until(b'\n', &mut bytes)?;
            if lidos == 0 {
                return Ok(());
            }
            if bytes.len() as u64 > LIMITE_LINHA {
                // resto da linha fora: a próxima mensagem começa depois do fim desta
                if bytes.last() != Some(&b'\n') {
                    let mut lixo = Vec::new();
                    while (&mut entrada)
                        .take(LIMITE_LINHA)
                        .read_until(b'\n', &mut lixo)?
                        > 0
                        && lixo.last() != Some(&b'\n')
                    {
                        lixo.clear();
                    }
                }
                Self::escrever(
                    &mut saida,
                    &erro_rpc(Value::Null, -32600, "mensagem grande demais"),
                )?;
                continue;
            }
            // o PowerShell abre o tubo com marca de ordem de bytes, e o `initialize` se perderia
            let mensagem = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
            if mensagem.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let resposta = match serde_json::from_slice::<Value>(mensagem) {
                Ok(m) => self.responder(&m),
                Err(e) => Some(erro_rpc(
                    Value::Null,
                    -32700,
                    &format!("JSON inválido: {e}"),
                )),
            };
            if let Some(r) = resposta {
                Self::escrever(&mut saida, &r)?;
            }
        }
    }

    fn escrever(saida: &mut impl Write, v: &Value) -> std::io::Result<()> {
        writeln!(saida, "{v}")?;
        saida.flush()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_cli::pasta::Pasta;
    use fenkra_cli::Isolamento;

    fn respostas(entrada: &[u8]) -> Vec<Value> {
        let sessao = Sessao::nova(
            Pasta::nova(std::env::temp_dir()).unwrap(),
            Isolamento::Local,
        );
        let mut saida = Vec::new();
        Servidor::novo(sessao, false)
            .laco(entrada, &mut saida)
            .unwrap();
        String::from_utf8(saida)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    const PING: &[u8] = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n";

    #[test]
    fn linha_grande_demais_e_recusada_e_a_seguinte_e_atendida() {
        // dois tamanhos: o que cabe na leitura com a quebra e o que deixa resto para descartar
        for tamanho in [LIMITE_LINHA as usize + 1, 2 * LIMITE_LINHA as usize + 10] {
            let mut entrada = vec![b'x'; tamanho];
            entrada.push(b'\n');
            entrada.extend_from_slice(PING);
            let r = respostas(&entrada);
            assert_eq!(r.len(), 2, "{tamanho}");
            assert_eq!(r[0]["error"]["code"], -32600);
            assert_eq!(r[1]["id"], 1);
        }
    }

    #[test]
    fn falha_de_ferramenta_leva_codigo_mensagem_e_sugestao() {
        // STEP sem o aceite da pessoa: recusado, e a resposta diz o que fazer (B419)
        let pedido = br#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"consultar_step","arguments":{"arquivo":"a.step"}}}"#;
        let r = respostas(pedido);
        let res = &r[0]["result"];
        assert_eq!(res["isError"], true, "{res}");
        let e = &res["structuredContent"];
        assert_eq!(e["codigo"], "step_bloqueado");
        assert!(e["sugestao"].as_str().is_some_and(|t| t.contains("--liberar-step")), "{e}");
    }

    #[test]
    fn linha_vazia_e_ultima_linha_sem_quebra_nao_atrapalham() {
        let r = respostas(b"\n   \n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["id"], 1);
    }
}
