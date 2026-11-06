// SPDX-License-Identifier: LGPL-2.1-or-later
//! A página do visualizador (B338): um arquivo só, com three.js (MIT) embutido e sem rede.
//!
//! Sem pacote embutido, a página serve de componente: abre o `.fkv` por arquivo (escolher ou
//! arrastar), por URL (`?src=...`, para hospedar num servidor comum) ou por mensagem do documento
//! que a embute (`postMessage`). Com pacote embutido, é um arquivo autocontido que abre sozinho.

const MODELO: &str = include_str!("../web/visualizador.html");
const THREE: &str = include_str!("../web/three.min.js");

const LETRAS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let n = u32::from(c[0]) << 16 | u32::from(*c.get(1).unwrap_or(&0)) << 8 | u32::from(*c.get(2).unwrap_or(&0));
        for k in 0..4 {
            s.push(if k <= c.len() { LETRAS[(n >> (18 - 6 * k) & 63) as usize] as char } else { '=' });
        }
    }
    s
}

/// A página, com o `.fkv` embutido quando dado.
pub fn pagina_html(pacote: Option<&[u8]>) -> String {
    pagina_html_completa(pacote, None, None)
}

/// A página com o pacote, um segundo pacote para comparar (a versão antiga; `pacote` é a nova) e os
/// dados do PLM (estado, revisão, versão) como JSON (B339). O JSON vai dentro de um `<script>`, então
/// o `<` é escrito como `\u003c` e nada do texto consegue fechar a marca.
pub fn pagina_html_completa(pacote: Option<&[u8]>, comparar: Option<&[u8]>, meta_json: Option<&str>) -> String {
    let dados = pacote.map(base64).unwrap_or_default();
    let dados_b = comparar.map(base64).unwrap_or_default();
    let meta = meta_json.unwrap_or("").replace('<', "\\u003c");
    MODELO.replacen("/*@THREE@*/", THREE, 1).replacen("/*@PACOTE_B@*/", &dados_b, 1).replacen("/*@PACOTE@*/", &dados, 1).replacen("/*@META@*/", &meta, 1)
}
