// SPDX-License-Identifier: LGPL-2.1-or-later
//! `visualizar`: gera o pacote de visualização `.fkv` (e, se pedido, a página autocontida) de uma
//! peça, montagem ou desenho. O formato está em `docs/formatos/pacote-visualizacao.md`.

use crate::pasta::Pasta;
use crate::ErroApi;
use fenkra_pacote::{de_arquivo, pagina_html, Opcoes};
use serde_json::{json, Value};

pub struct Pedido<'a> {
    pub arquivo: &'a str,
    pub saida: &'a str,
    pub html: Option<&'a str>,
    /// fração dos triângulos que fica; liga também a retirada dos pontos de captura exatos
    pub decimar: Option<f64>,
    pub sobrescrever: bool,
}

pub fn visualizar(pasta: &Pasta, p: &Pedido) -> Result<Value, ErroApi> {
    if let Some(f) = p.decimar {
        if !(0.01..=1.0).contains(&f) {
            return Err(ErroApi::argumento("--decimar vai de 0,01 a 1 (fração dos triângulos que fica)"));
        }
    }
    let caminho = pasta.para_ler(p.arquivo, &["fkp", "fka", "fkd"])?;
    let opc = Opcoes { decimar: p.decimar, capturas: p.decimar.is_none() };
    let pacote = de_arquivo(&caminho, &opc).map_err(|e| ErroApi::novo("leitura", e))?;
    pacote.validar().map_err(|e| ErroApi::novo("formato", e))?;
    let bytes = pacote.bytes().map_err(|e| ErroApi::novo("gravacao", e))?;
    let gravado = pasta.gravar(p.saida, &["fkv"], p.sobrescrever, |tmp| std::fs::write(tmp, &bytes).map_err(|e| ErroApi::novo("gravacao", e.to_string())))?;
    let mut r = json!({
        "saida": gravado.file_name().map(|n| n.to_string_lossy().to_string()),
        "bytes": bytes.len(),
        "tipo": pacote.manifesto["tipo"],
        "versao_do_formato": fenkra_pacote::VERSAO,
        "estatisticas": pacote.manifesto["estatisticas"],
    });
    if let Some(h) = p.html {
        let pagina = pagina_html(Some(&bytes));
        let g = pasta.gravar(h, &["html", "htm"], p.sobrescrever, |tmp| std::fs::write(tmp, &pagina).map_err(|e| ErroApi::novo("gravacao", e.to_string())))?;
        r["html"] = json!(g.file_name().map(|n| n.to_string_lossy().to_string()));
        r["html_bytes"] = json!(pagina.len());
    }
    Ok(r)
}
