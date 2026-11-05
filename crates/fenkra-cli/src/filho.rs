// SPDX-License-Identifier: LGPL-2.1-or-later
//! Processo filho para quem lê STEP de fora (ADR 0023): o programa chama a si mesmo com `--filho`.
//!
//! Um arquivo hostil pode derrubar o OpenCASCADE com falha de memória, que nenhum tratamento de erro
//! alcança, e a sessão do agente é justamente o que não pode cair. O pedido vai pela entrada padrão e
//! a resposta volta num arquivo: a saída padrão do filho é do OpenCASCADE, que escreve nela.

use crate::sessao::Sessao;
use crate::{abrir_envelope, envelope, ErroApi};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Argumento que põe os binários no papel de filho.
pub const ARGUMENTO: &str = "--filho";
/// Um STEP grande leva minutos; dez já indicam laço.
pub const LIMITE: Duration = Duration::from_secs(600);

static SEQUENCIA: AtomicU64 = AtomicU64::new(0);

/// Só o pedido de teste do isolamento. Não é um comando da sessão: nada chega aqui fora do filho.
const QUEDA_DE_TESTE: &str = "__queda";

/// Ponto de entrada do filho. Devolve o código de saída.
pub fn principal() -> i32 {
    let mut entrada = String::new();
    if std::io::stdin().read_to_string(&mut entrada).is_err() {
        return 2;
    }
    let Ok(pedido) = serde_json::from_str::<Value>(&entrada) else {
        return 2;
    };
    let Some(resposta) = pedido["resposta"].as_str() else {
        return 2;
    };
    let comando = pedido["comando"].as_str().unwrap_or_default();
    if comando == QUEDA_DE_TESTE {
        std::process::abort();
    }
    let resultado =
        Sessao::de_pedido(&pedido).and_then(|mut s| s.executar(comando, &pedido["args"]));
    let Ok(texto) = serde_json::to_vec(&envelope(resultado)) else {
        return 3;
    };
    // nome provisório: quem chama nunca lê uma resposta pela metade
    let provisorio = format!("{resposta}.parcial");
    if std::fs::write(&provisorio, texto).is_err()
        || std::fs::rename(&provisorio, resposta).is_err()
    {
        return 3;
    }
    0
}

fn sem_janela(c: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const SEM_JANELA: u32 = 0x0800_0000;
        c.creation_flags(SEM_JANELA);
    }
    #[cfg(not(windows))]
    let _ = c;
}

/// Roda `pedido` (com `comando`, `args`, `pasta` e, se houver, `modelo`) no filho e espera.
pub fn rodar(programa: &Path, mut pedido: Value, limite: Duration) -> Result<Value, ErroApi> {
    let n = SEQUENCIA.fetch_add(1, Ordering::Relaxed);
    let resposta = std::env::temp_dir().join(format!("fenkra-cli-{}-{n}.json", std::process::id()));
    pedido["resposta"] = json!(resposta.to_string_lossy());
    let r = esperar(programa, &pedido, &resposta, limite);
    let _ = std::fs::remove_file(&resposta);
    let _ = std::fs::remove_file(format!("{}.parcial", resposta.display()));
    r
}

fn esperar(
    programa: &Path,
    pedido: &Value,
    resposta: &Path,
    limite: Duration,
) -> Result<Value, ErroApi> {
    let mut c = Command::new(programa);
    c.arg(ARGUMENTO)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    sem_janela(&mut c);
    let mut filho = c.spawn().map_err(|e| {
        ErroApi::novo(
            "queda",
            format!("não foi possível iniciar {}: {e}", programa.display()),
        )
    })?;
    if let Some(mut entrada) = filho.stdin.take() {
        // filho que morreu antes de ler é tratado pelo código de saída logo abaixo
        let _ = entrada.write_all(pedido.to_string().as_bytes());
    }
    let inicio = Instant::now();
    let estado = loop {
        if inicio.elapsed() > limite {
            let _ = filho.kill();
            let _ = filho.wait();
            return Err(ErroApi::novo(
                "tempo",
                format!(
                    "a leitura passou do limite de {} s e foi encerrada",
                    limite.as_secs()
                ),
            ));
        }
        match filho.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => return Err(ErroApi::novo("queda", e.to_string())),
        }
    };
    if !estado.success() {
        return Err(ErroApi::novo(
            "queda",
            format!(
                "o leitor parou ao abrir o arquivo (código {}); a sessão segue aberta",
                estado.code().unwrap_or(-1)
            ),
        ));
    }
    let texto = std::fs::read(resposta)
        .map_err(|e| ErroApi::novo("queda", format!("resposta do leitor: {e}")))?;
    let v: Value = serde_json::from_slice(&texto)
        .map_err(|e| ErroApi::novo("queda", format!("resposta do leitor: {e}")))?;
    abrir_envelope(&v)
}

/// Pedido que derruba o filho, para testar o isolamento.
pub fn pedido_de_queda() -> Value {
    json!({ "comando": QUEDA_DE_TESTE })
}
