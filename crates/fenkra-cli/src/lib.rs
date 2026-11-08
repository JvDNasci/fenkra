// SPDX-License-Identifier: LGPL-2.1-or-later
//! Automação do Fenkra sem interface: a mesma lista de comandos serve à linha de comando, à API Python
//! e ao servidor MCP.
//!
//! A peça em edição é uma árvore `.fkx` ([`fenkra_conversor::arvore`]). Cada comando acrescenta um item
//! validado pelo esquema publicado, e o documento é sempre remontado da árvore, então o que o agente
//! ou o script monta é o mesmo que um arquivo aberto no programa. Nada aqui interpreta código: só há
//! comandos com nome fixo e dados em JSON, e todo caminho passa por [`pasta::Pasta`].
//!
//! Quem lê arquivo de fora usa [`filho`]: um STEP hostil que derrube o OpenCASCADE derruba o filho, e
//! a sessão segue viva (ADR 0023). A forma do JSON está em `docs/formatos/api-json.md`.

pub mod biblioteca;
pub mod checador;
pub mod filho;
pub mod pasta;
mod registro;
pub mod saida;
pub mod selecao;
mod sessao;
pub mod features;
pub mod topologia;
pub mod ver;
pub mod visualizar;

pub use registro::{ler_comandos, Registro};
pub use sessao::{tipos_do_esquema, Isolamento, Sessao, COMANDOS, COMANDOS_DE_MODELO};

/// Esquema publicado da árvore: as ferramentas geradas do MCP saem dele (B420).
pub use fenkra_conversor::arvore::ESQUEMA as ESQUEMA_ARVORE;

use serde_json::{json, Value};

/// Versão do contrato JSON. Campo novo não muda o número; mudar ou tirar campo muda.
pub const API_VERSAO: u32 = 1;

/// Falha de um comando. `codigo` é estável e serve para o programa decidir; `mensagem` é para gente.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErroApi {
    pub codigo: String,
    pub mensagem: String,
}

impl ErroApi {
    pub fn novo(codigo: &str, mensagem: impl Into<String>) -> Self {
        ErroApi {
            codigo: codigo.into(),
            mensagem: mensagem.into(),
        }
    }
    pub fn argumento(mensagem: impl Into<String>) -> Self {
        Self::novo("argumento", mensagem)
    }
}

/// O que fazer diante de cada código de falha da API (B419). O código não muda; a sugestão pode
/// melhorar. Quem recebe a resposta lê a sugestão em vez de adivinhar o próximo passo.
pub fn sugestao_do_codigo(codigo: &str) -> &'static str {
    match codigo {
        "argumento" => "Confira os campos do comando: nome, tipo e unidade (mm e graus) estão no esquema da ferramenta.",
        "parametro" => "Um parâmetro usado pelo comando está inválido; corrija o valor ou a fórmula e recalcule.",
        "arvore" => "O comando não monta na árvore. Confira os ids citados (esboço, curva, face) com o comando de árvore e refaça o pedido.",
        "arvore_incompleta" => "Falta um item que este comando usa. Crie o esboço ou a peça antes e use o id devolvido.",
        "sem_peca" => "Não há peça em edição. Crie uma com nova_peca ou abra um arquivo antes.",
        "sem_solido" => "A peça ainda não tem corpo sólido. Extrude, revolucione ou crie uma primitiva antes de medir ou comparar.",
        "sem_historico" => "Não há o que desfazer: nenhum comando foi aplicado nesta sessão.",
        "kernel" => "O kernel recusou a operação com estes dados. Reduza o raio, a profundidade ou o recuo, ou mude a entrada, e recalcule.",
        "formato" => "O arquivo não segue o formato esperado. Confira a extensão e se ele foi gravado por este programa ou por uma exportação suportada.",
        "leitura" => "Não foi possível ler o arquivo. Confira o caminho (relativo à pasta do projeto), a extensão e se ele não está aberto em outro programa.",
        "gravacao" => "Não foi possível gravar. Confira se a pasta existe e tem permissão e use sobrescrever só se quiser trocar um arquivo existente.",
        "caminho_fora" => "O caminho precisa ficar dentro da pasta do projeto. Use um caminho relativo a ela.",
        "queda" => "O leitor travou ao abrir o arquivo; ele foi isolado e a sessão segue. Tente um arquivo menor ou exporte de novo da origem.",
        "tempo" => "A leitura passou do tempo limite. Tente um arquivo menor ou simplifique a peça de origem.",
        "step_bloqueado" => "Ler STEP depende do aceite da pessoa: peça que ela inicie o servidor com --liberar-step. Não tente contornar.",
        _ => "Releia a mensagem, confira os argumentos e a árvore da peça e tente de novo; se persistir, recalcule e leia o relatório.",
    }
}

impl ErroApi {
    pub fn sugestao(&self) -> &'static str {
        sugestao_do_codigo(&self.codigo)
    }
}

impl std::fmt::Display for ErroApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.codigo, self.mensagem)
    }
}

impl std::error::Error for ErroApi {}

impl From<fenkra_kernel::KernelError> for ErroApi {
    fn from(e: fenkra_kernel::KernelError) -> Self {
        ErroApi::novo("kernel", e.0)
    }
}

impl From<fenkra_conversor::ErroConv> for ErroApi {
    fn from(e: fenkra_conversor::ErroConv) -> Self {
        use fenkra_conversor::ErroConv as E;
        let codigo = match &e {
            E::Formato(_) => "formato",
            E::Gravacao(_) => "gravacao",
            E::Queda(_) => "queda",
            E::Tempo(_) => "tempo",
            _ => "leitura",
        };
        ErroApi::novo(codigo, e.to_string())
    }
}

/// Resposta de um comando na forma estável: `{api, ok, resultado}` ou `{api, ok, erro}`.
pub fn envelope(r: Result<Value, ErroApi>) -> Value {
    match r {
        Ok(resultado) => json!({ "api": API_VERSAO, "ok": true, "resultado": resultado }),
        Err(e) => {
            json!({ "api": API_VERSAO, "ok": false, "erro": { "codigo": e.codigo, "mensagem": e.mensagem, "sugestao": e.sugestao() } })
        }
    }
}

/// Desfaz o envelope, para quem recebe a resposta de outro processo.
pub fn abrir_envelope(v: &Value) -> Result<Value, ErroApi> {
    if v["ok"].as_bool() == Some(true) {
        return Ok(v["resultado"].clone());
    }
    Err(ErroApi::novo(
        v["erro"]["codigo"].as_str().unwrap_or("leitura"),
        v["erro"]["mensagem"]
            .as_str()
            .unwrap_or("resposta sem mensagem"),
    ))
}

/// Arredonda para sair no JSON: 1e-6 mm não muda decisão de ninguém, e o texto fica curto para o agente.
pub(crate) fn r6(x: f64) -> f64 {
    let v = (x * 1e6).round() / 1e6;
    if v == 0. {
        0.
    } else {
        v
    }
}

pub(crate) fn r6v(v: [f64; 3]) -> [f64; 3] {
    v.map(r6)
}
