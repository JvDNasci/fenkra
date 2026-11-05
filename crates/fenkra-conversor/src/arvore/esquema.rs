// SPDX-License-Identifier: LGPL-2.1-or-later
//! Validação pelo esquema publicado. Interpreta só as palavras do JSON Schema que o nosso arquivo usa;
//! palavra desconhecida é erro aqui, para o esquema não ganhar uma regra que a leitura ignore em
//! silêncio. Um validador completo traria uma árvore de dependências para conferir um único arquivo.

use serde_json::Value;

/// Onde o arquivo foge do esquema e por quê. `caminho` é um ponteiro JSON, como `/pecas/0/arvore/2`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErroEsquema {
    pub caminho: String,
    pub motivo: String,
}

impl std::fmt::Display for ErroEsquema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let onde = if self.caminho.is_empty() { "/" } else { &self.caminho };
        write!(f, "{onde}: {}", self.motivo)
    }
}

/// Palavras que só descrevem ou agrupam, sem regra a conferir.
const SEM_REGRA: [&str; 4] = ["$schema", "title", "description", "$defs"];

type Resultado = Result<(), ErroEsquema>;

fn erro(caminho: &str, motivo: impl Into<String>) -> ErroEsquema {
    ErroEsquema { caminho: caminho.into(), motivo: motivo.into() }
}

fn nome_do_tipo(v: &Value) -> &'static str {
    match v {
        Value::Null => "nulo",
        Value::Bool(_) => "booleano",
        Value::Number(_) => "número",
        Value::String(_) => "texto",
        Value::Array(_) => "lista",
        Value::Object(_) => "objeto",
    }
}

fn tem_tipo(v: &Value, tipo: &str) -> Option<bool> {
    Some(match tipo {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "boolean" => v.is_boolean(),
        "number" => v.is_number(),
        "integer" => v.is_i64() || v.is_u64(),
        _ => return None,
    })
}

fn alvo<'a>(raiz: &'a Value, referencia: &str) -> Result<&'a Value, ErroEsquema> {
    referencia.strip_prefix('#').and_then(|p| raiz.pointer(p)).ok_or_else(|| erro("", format!("esquema: referência '{referencia}' não existe")))
}

/// Um ramo de `oneOf` cujo `tipo` fixo é o do valor: é dele que o erro interessa.
fn ramo_do_tipo<'a>(raiz: &'a Value, ramos: &'a [Value], v: &Value) -> Option<&'a Value> {
    let tipo = v.get("tipo")?;
    ramos.iter().find_map(|r| {
        let r = r.get("$ref").and_then(Value::as_str).map_or(Some(r), |x| alvo(raiz, x).ok())?;
        (r.pointer("/properties/tipo/const") == Some(tipo)).then_some(r)
    })
}

fn conferir_oneof(raiz: &Value, ramos: &[Value], v: &Value, caminho: &str) -> Resultado {
    let mut bons = 0;
    for r in ramos {
        if conferir(raiz, r, v, caminho).is_ok() {
            bons += 1;
        }
    }
    match bons {
        1 => Ok(()),
        0 => match ramo_do_tipo(raiz, ramos, v) {
            Some(r) => conferir(raiz, r, v, caminho),
            None => Err(erro(caminho, match v.get("tipo").and_then(Value::as_str) {
                Some(t) => format!("tipo '{t}' desconhecido"),
                None => "não combina com nenhuma forma aceita".to_string(),
            })),
        },
        n => Err(erro(caminho, format!("ambíguo: combina com {n} formas"))),
    }
}

fn conferir_objeto(raiz: &Value, esquema: &serde_json::Map<String, Value>, o: &serde_json::Map<String, Value>, caminho: &str) -> Resultado {
    for r in esquema.get("required").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
        if !o.contains_key(r) {
            return Err(erro(caminho, format!("falta o campo '{r}'")));
        }
    }
    let campos = esquema.get("properties").and_then(Value::as_object);
    for (k, v) in o {
        match campos.and_then(|p| p.get(k)) {
            Some(s) => conferir(raiz, s, v, &format!("{caminho}/{k}"))?,
            None if esquema.get("additionalProperties") == Some(&Value::Bool(false)) => return Err(erro(caminho, format!("campo '{k}' não existe nesta versão"))),
            None => {}
        }
    }
    Ok(())
}

fn conferir_lista(raiz: &Value, esquema: &serde_json::Map<String, Value>, l: &[Value], caminho: &str) -> Resultado {
    let limite = |k: &str| esquema.get(k).and_then(Value::as_u64).map(|n| n as usize);
    if limite("minItems").is_some_and(|n| l.len() < n) || limite("maxItems").is_some_and(|n| l.len() > n) {
        return Err(erro(caminho, format!("lista com {} itens fora do limite", l.len())));
    }
    if let Some(s) = esquema.get("items") {
        for (i, v) in l.iter().enumerate() {
            conferir(raiz, s, v, &format!("{caminho}/{i}"))?;
        }
    }
    Ok(())
}

fn conferir(raiz: &Value, esquema: &Value, v: &Value, caminho: &str) -> Resultado {
    let Some(s) = esquema.as_object() else { return Err(erro(caminho, "esquema: regra não é um objeto")) };
    for (palavra, regra) in s {
        match palavra.as_str() {
            "$ref" => {
                let r = regra.as_str().ok_or_else(|| erro(caminho, "esquema: $ref sem texto"))?;
                conferir(raiz, alvo(raiz, r)?, v, caminho)?;
            }
            "type" => {
                let t = regra.as_str().unwrap_or_default();
                match tem_tipo(v, t) {
                    Some(true) => {}
                    Some(false) => return Err(erro(caminho, format!("esperava {t}, veio {}", nome_do_tipo(v)))),
                    None => return Err(erro(caminho, format!("esquema: tipo '{t}' desconhecido"))),
                }
            }
            "const" if v != regra => return Err(erro(caminho, format!("esperava {regra}, veio {v}"))),
            "enum" if !regra.as_array().is_some_and(|l| l.contains(v)) => return Err(erro(caminho, format!("valor {v} não é aceito; aceitos: {regra}"))),
            "minimum" if v.as_f64().zip(regra.as_f64()).is_some_and(|(x, m)| x < m) => return Err(erro(caminho, format!("{v} é menor que {regra}"))),
            "maximum" if v.as_f64().zip(regra.as_f64()).is_some_and(|(x, m)| x > m) => return Err(erro(caminho, format!("{v} é maior que {regra}"))),
            "minLength" if v.as_str().zip(regra.as_u64()).is_some_and(|(x, m)| (x.chars().count() as u64) < m) => return Err(erro(caminho, "texto vazio")),
            "oneOf" => conferir_oneof(raiz, regra.as_array().map_or(&[], |l| l.as_slice()), v, caminho)?,
            "const" | "enum" | "minimum" | "maximum" | "minLength" => {}
            "required" | "properties" | "additionalProperties" | "items" | "minItems" | "maxItems" => {}
            p if SEM_REGRA.contains(&p) => {}
            p => return Err(erro(caminho, format!("esquema: palavra '{p}' não é entendida pela leitura"))),
        }
    }
    match v {
        Value::Object(o) => conferir_objeto(raiz, s, o, caminho),
        Value::Array(l) => conferir_lista(raiz, s, l, caminho),
        _ => Ok(()),
    }
}

/// Confere `v` contra o esquema publicado.
pub fn validar(v: &Value) -> Resultado {
    let raiz: Value = serde_json::from_str(super::ESQUEMA).map_err(|e| erro("", format!("esquema: {e}")))?;
    conferir(&raiz, &raiz, v, "")
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    fn minimo() -> Value {
        json!({"formato": "fenkra-arvore", "format_version": 1, "unidades": "mm", "pecas": [{"id": "a", "nome": "A", "arvore": []}]})
    }

    #[test]
    fn arquivo_minimo_passa() {
        assert_eq!(validar(&minimo()), Ok(()));
    }

    #[test]
    fn campo_que_falta_e_campo_a_mais_dizem_onde() {
        let mut v = minimo();
        v["pecas"][0].as_object_mut().unwrap().remove("nome");
        let e = validar(&v).unwrap_err();
        assert_eq!((e.caminho.as_str(), e.motivo.as_str()), ("/pecas/0", "falta o campo 'nome'"));
        let mut v = minimo();
        v["pecas"][0]["cor_favorita"] = json!(1);
        assert!(validar(&v).unwrap_err().motivo.contains("cor_favorita"));
    }

    #[test]
    fn erro_dentro_de_comando_vem_do_ramo_do_tipo() {
        let mut v = minimo();
        v["pecas"][0]["arvore"] = json!([{"id": "1", "nome": "F", "comando": {"tipo": "filete", "arestas": "todas"}}]);
        let e = validar(&v).unwrap_err();
        assert_eq!(e.caminho, "/pecas/0/arvore/0/comando");
        assert!(e.motivo.contains("raio"), "{e}");
        v["pecas"][0]["arvore"][0]["comando"]["tipo"] = json!("voar");
        assert!(validar(&v).unwrap_err().motivo.contains("'voar'"));
    }

    #[test]
    fn formato_de_outro_programa_e_unidade_em_polegada_sao_recusados() {
        let mut v = minimo();
        v["unidades"] = json!("in");
        assert_eq!(validar(&v).unwrap_err().caminho, "/unidades");
        let mut v = minimo();
        v["formato"] = json!("outro");
        assert_eq!(validar(&v).unwrap_err().caminho, "/formato");
    }

    #[test]
    fn palavra_nova_no_esquema_nao_passa_em_silencio() {
        let e = conferir(&json!({}), &json!({"pattern": "^a"}), &json!("b"), "").unwrap_err();
        assert!(e.motivo.contains("pattern"), "{e}");
    }
}
