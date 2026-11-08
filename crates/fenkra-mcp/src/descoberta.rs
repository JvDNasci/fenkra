// SPDX-License-Identifier: LGPL-2.1-or-later
//! Descoberta de comandos (B420): com dezenas de ferramentas, o agente pergunta o que existe para uma
//! tarefa ("revolução", "furo", "parafuso") e recebe só as que servem, com os campos exigidos. Diz
//! também o que ainda não passa pelo servidor, para ele não procurar o que não há.

use crate::ferramentas::{Ferramenta, FERRAMENTAS};
use crate::geradas::geradas;
use fenkra_cli::ErroApi;
use serde_json::{json, Value};

pub const NOME: &str = "listar_comandos";

/// O que o formato aberto ainda não descreve: fica no aplicativo.
const FORA: [(&str, &str); 3] = [
    ("montagem", "restrições e juntas entre peças: monte no aplicativo, a partir das peças salvas aqui"),
    ("chapa metálica", "parede, aba, dobra e planificação ainda não estão no formato da árvore"),
    ("estrutural", "perfis sobre esqueleto e cortes entre membros ainda não estão no formato da árvore"),
];

fn propria() -> Value {
    json!({
        "name": NOME, "title": "Listar comandos",
        "description": "Lista as ferramentas deste servidor, com o que cada uma faz e os campos exigidos. `busca` filtra por palavra (revolução, furo, padrão, parafuso). Diz também o que ainda não está disponível por aqui.",
        "inputSchema": { "type": "object", "properties": { "busca": { "type": "string" } }, "required": [] },
        "annotations": { "readOnlyHint": true, "destructiveHint": false, "openWorldHint": false },
    })
}

/// Todas as ferramentas, para `tools/list`: as escritas à mão, as geradas do esquema e esta.
pub fn todas() -> Vec<Value> {
    let mut v: Vec<Value> = FERRAMENTAS.iter().map(Ferramenta::json).collect();
    v.extend(geradas().iter().map(|g| g.json()));
    v.push(propria());
    v
}

/// Minúsculas e sem acento, para "revolução" achar "revolucionar".
fn chave(t: &str) -> String {
    t.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' => 'u',
            'ç' => 'c',
            _ => c,
        })
        .collect()
}

/// Raiz curta da palavra, para "revolução" e "revolucionar" se acharem: tira até três letras do fim,
/// sem descer de cinco.
fn raiz(p: &str) -> String {
    let n = p.chars().count();
    p.chars().take(n.saturating_sub(3).max(5).min(n)).collect()
}

pub fn listar(args: &Value) -> Result<Value, ErroApi> {
    let busca = match &args["busca"] {
        Value::Null => None,
        Value::String(s) => Some(chave(s)),
        _ => return Err(ErroApi::argumento("`busca` é um texto")),
    };
    let lista: Vec<Value> = todas()
        .into_iter()
        .filter(|t| {
            busca.as_ref().is_none_or(|b| {
                let texto = chave(&format!("{} {} {}", t["name"], t["title"], t["description"]));
                b.split_whitespace().all(|p| texto.contains(&raiz(p)))
            })
        })
        .map(|t| json!({ "nome": t["name"], "titulo": t["title"], "descricao": t["description"], "exigidos": t["inputSchema"]["required"], "somente_leitura": t["annotations"]["readOnlyHint"] }))
        .collect();
    let n = lista.len();
    let mut v = json!({ "quantidade": n, "ferramentas": lista, "fora_deste_servidor": FORA.iter().map(|(a, m)| json!({ "area": a, "motivo": m })).collect::<Vec<_>>() });
    if n == 0 {
        v["aviso"] = json!("nenhuma ferramenta com essa palavra; chame sem `busca` para ver todas");
    }
    Ok(v)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn busca_por_palavra_acha_a_ferramenta_gerada_e_a_escrita() {
        let r = listar(&json!({ "busca": "revolução" })).unwrap();
        let nomes: Vec<&str> = r["ferramentas"].as_array().unwrap().iter().map(|t| t["nome"].as_str().unwrap()).collect();
        assert!(nomes.contains(&"revolucionar"), "{r}");
        let r = listar(&json!({ "busca": "parafuso" })).unwrap();
        assert!(r["ferramentas"].as_array().unwrap().iter().any(|t| t["nome"] == "consultar_biblioteca"), "{r}");
        let r = listar(&json!({})).unwrap();
        assert_eq!(r["quantidade"].as_u64().unwrap() as usize, todas().len());
        assert!(r["fora_deste_servidor"].as_array().unwrap().iter().any(|f| f["area"] == "chapa metálica"));
    }

    #[test]
    fn nomes_nao_se_repetem() {
        let t = todas();
        let mut nomes: Vec<&str> = t.iter().map(|x| x["name"].as_str().unwrap()).collect();
        let n = nomes.len();
        nomes.sort_unstable();
        nomes.dedup();
        assert_eq!(nomes.len(), n);
    }
}
