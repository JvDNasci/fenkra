// SPDX-License-Identifier: LGPL-2.1-or-later
//! Ferramentas geradas do esquema publicado da árvore (B420). Cada `tipo` de comando do esquema que não
//! tem ferramenta escrita à mão vira uma, com o esquema de entrada tirado do próprio arquivo. Comando
//! novo no formato chega ao agente sem código aqui: só o título e a descrição ficam numa tabela, e o que
//! não estiver nela sai com texto genérico.
//!
//! A lista continua fechada: o que o esquema não descreve não vira ferramenta, e toda chamada passa
//! pelo comando `adicionar` da sessão, que valida no mesmo esquema antes de montar.

use fenkra_cli::ErroApi;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

/// `tipo` que já têm ferramenta escrita à mão, com descrição e conversões próprias.
const A_MAO: [&str; 5] = ["esboco", "extrusao", "furo", "filete", "chanfro"];

/// (tipo, nome da ferramenta, título, descrição). Tipo fora desta tabela ganha `adicionar_<tipo>`.
const TEXTOS: [(&str, &str, &str, &str); 5] = [
    ("revolucao", "revolucionar", "Revolucionar", "Gira um esboço em torno de um eixo: une, corta ou intersecta. `angulo` em graus (360 fecha a volta); `eixo` é X, Y, Z, uma curva do esboço ({esboco, curva}) ou {origem, direcao}. `fina` faz parede fina."),
    ("casca", "criar_casca", "Casca", "Esvazia o sólido deixando parede de `espessura`. `faces` é um ponto sobre cada face que fica aberta: use selecionar com alvo faces para achar esses pontos."),
    ("primitiva", "criar_primitiva", "Primitiva", "Caixa, cilindro, esfera, cone, toro, cunha ou prisma num plano, sem esboço. a, b, c e d são as medidas da forma (caixa: comprimento, largura, altura; cilindro: raio, altura)."),
    ("padrao", "criar_padrao", "Padrão", "Repete itens da árvore: linear (direção, quantidade, espaçamento, e opcionalmente uma segunda direção) ou circular (eixo, quantidade, ângulo). `fontes` são os ids a repetir; sem `fontes`, repete o corpo. `pular` tira cópias pelo índice [i, j]."),
    ("espelho", "espelhar", "Espelhar", "Espelha itens da árvore (ou o corpo inteiro, sem `fontes`) por um plano XY, XZ, YZ ou de referência."),
];

/// Uma ferramenta gerada: nome, textos, esquema de entrada para o agente e o original do arquivo.
pub struct Gerada {
    pub nome: String,
    pub tipo: String,
    titulo: String,
    descricao: String,
    entrada: Value,
    /// o ramo do esquema, com as referências resolvidas, para converter os números
    original: Value,
}

fn esquema() -> &'static Value {
    static E: OnceLock<Value> = OnceLock::new();
    E.get_or_init(|| serde_json::from_str(fenkra_cli::ESQUEMA_ARVORE).expect("o esquema publicado é JSON"))
}

/// Troca cada `$ref` local pelo que ele aponta. O esquema da árvore não tem ciclo; `fundo` corta por
/// garantia.
fn resolver(v: &Value, defs: &Value, fundo: u32) -> Value {
    if fundo > 32 {
        return v.clone();
    }
    match v {
        Value::Object(o) => {
            if let Some(r) = o.get("$ref").and_then(Value::as_str) {
                let nome = r.trim_start_matches("#/$defs/");
                let mut alvo = resolver(&defs[nome], defs, fundo + 1);
                if nome == "expressao" {
                    // a marca fica para a conversão saber que número vira texto
                    alvo["x-expressao"] = json!(true);
                }
                return alvo;
            }
            Value::Object(o.iter().map(|(k, x)| (k.clone(), resolver(x, defs, fundo + 1))).collect())
        }
        Value::Array(l) => Value::Array(l.iter().map(|x| resolver(x, defs, fundo + 1)).collect()),
        _ => v.clone(),
    }
}

/// O que o agente vê: expressões aceitam número, sem a marca interna, e sem `tipo` (vem do nome).
fn para_o_agente(v: &Value) -> Value {
    match v {
        Value::Object(o) => {
            if o.contains_key("x-expressao") {
                return json!({ "type": ["string", "number"], "description": "número (mm ou graus) ou expressão com parâmetros" });
            }
            Value::Object(o.iter().map(|(k, x)| (k.clone(), para_o_agente(x))).collect())
        }
        Value::Array(l) => Value::Array(l.iter().map(para_o_agente).collect()),
        _ => v.clone(),
    }
}

fn entrada_de(ramo: &Value) -> Value {
    let mut e = para_o_agente(ramo);
    if let Some(p) = e["properties"].as_object_mut() {
        p.remove("tipo");
        p.insert("id".into(), json!({ "type": "string", "description": "id do item; ausente, o servidor cria" }));
        p.insert("nome".into(), json!({ "type": "string" }));
    }
    if let Some(r) = e["required"].as_array_mut() {
        r.retain(|x| x != "tipo");
    }
    e
}

/// Todas as ferramentas que o esquema dá, na ordem dele.
pub fn geradas() -> &'static [Gerada] {
    static G: OnceLock<Vec<Gerada>> = OnceLock::new();
    G.get_or_init(|| {
        let e = esquema();
        let defs = &e["$defs"];
        let ramos = defs["comando"]["oneOf"].as_array().cloned().unwrap_or_default();
        ramos
            .iter()
            .map(|r| resolver(r, defs, 0))
            .filter_map(|ramo| {
                let tipo = ramo["properties"]["tipo"]["const"].as_str()?.to_string();
                if A_MAO.contains(&tipo.as_str()) {
                    return None;
                }
                let (nome, titulo, descricao) = TEXTOS
                    .iter()
                    .find(|t| t.0 == tipo)
                    .map(|t| (t.1.to_string(), t.2.to_string(), t.3.to_string()))
                    .unwrap_or_else(|| (format!("adicionar_{tipo}"), tipo.clone(), format!("Acrescenta um item '{tipo}' à árvore, nos campos do formato aberto.")));
                Some(Gerada { nome, tipo, titulo, descricao, entrada: entrada_de(&ramo), original: ramo })
            })
            .collect()
    })
}

impl Gerada {
    pub fn json(&self) -> Value {
        json!({
            "name": self.nome, "title": self.titulo, "description": self.descricao, "inputSchema": self.entrada,
            "annotations": { "readOnlyHint": false, "destructiveHint": false, "openWorldHint": false },
        })
    }

    /// Argumentos do agente para o comando `adicionar`: números onde o arquivo pede texto viram texto,
    /// e o `tipo` vem do nome da ferramenta.
    pub fn traduzir(&self, args: &Value) -> Result<Value, ErroApi> {
        if !args.is_object() {
            return Err(ErroApi::argumento("os argumentos são um objeto"));
        }
        let mut v = converter(args, &self.original);
        if v.get("tipo").is_some_and(|t| t != self.tipo.as_str()) {
            return Err(ErroApi::argumento(format!("esta ferramenta só cria '{}'", self.tipo)));
        }
        v["tipo"] = json!(self.tipo);
        Ok(v)
    }
}

fn pede_texto(s: &Value) -> bool {
    s.get("x-expressao").is_some() || s["type"] == "string"
}

/// Ramo de `oneOf` que serve ao valor: o de mesmo `tipo`, ou o primeiro cujos campos exigidos estão
/// todos presentes.
fn ramo_para<'a>(valor: &Value, ramos: &'a [Value]) -> Option<&'a Value> {
    let o = valor.as_object()?;
    if let Some(t) = o.get("tipo") {
        if let Some(r) = ramos.iter().find(|r| &r["properties"]["tipo"]["const"] == t) {
            return Some(r);
        }
    }
    ramos.iter().find(|r| r["type"] == "object" && r["required"].as_array().is_some_and(|l| l.iter().all(|k| k.as_str().is_some_and(|k| o.contains_key(k)))))
}

/// Anda no valor junto com o esquema e troca número por texto onde o arquivo pede texto.
fn converter(valor: &Value, s: &Value) -> Value {
    if let Some(ramos) = s["oneOf"].as_array() {
        return match ramo_para(valor, ramos) {
            Some(r) => converter(valor, r),
            None if valor.is_number() && ramos.iter().any(pede_texto) => json!(valor.to_string()),
            None => valor.clone(),
        };
    }
    match valor {
        Value::Number(n) if pede_texto(s) => json!(n.to_string()),
        Value::Object(o) => {
            let mut saida = Map::new();
            for (k, x) in o {
                let sub = &s["properties"][k];
                saida.insert(k.clone(), if sub.is_null() { x.clone() } else { converter(x, sub) });
            }
            Value::Object(saida)
        }
        Value::Array(l) => Value::Array(l.iter().map(|x| converter(x, &s["items"])).collect()),
        _ => valor.clone(),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn revolucao_padrao_espelho_casca_e_primitiva_saem_do_esquema() {
        let nomes: Vec<&str> = geradas().iter().map(|g| g.nome.as_str()).collect();
        for n in ["revolucionar", "criar_padrao", "espelhar", "criar_casca", "criar_primitiva"] {
            assert!(nomes.contains(&n), "{nomes:?}");
        }
        // as escritas à mão não se repetem
        assert!(!nomes.iter().any(|n| n.contains("extrusao") || n.contains("filete")), "{nomes:?}");
        for g in geradas() {
            let e = &g.entrada;
            assert_eq!(e["type"], "object");
            assert!(e["properties"]["tipo"].is_null(), "{}", g.nome);
            assert!(!e.to_string().contains("$ref"), "{}: referência sem resolver", g.nome);
            assert!(!e.to_string().contains("x-expressao"), "{}: marca interna vazou", g.nome);
        }
    }

    #[test]
    fn numero_vira_texto_onde_o_arquivo_pede_expressao() {
        let g = geradas().iter().find(|g| g.tipo == "padrao").unwrap();
        let v = g
            .traduzir(&json!({ "forma": { "tipo": "circular", "eixo": "Z", "quantidade": 6, "angulo": 360 }, "fontes": ["furo1"] }))
            .unwrap();
        assert_eq!(v["tipo"], "padrao");
        assert_eq!(v["forma"]["quantidade"], "6", "{v}");
        assert_eq!(v["forma"]["angulo"], "360", "{v}");
        // eixo continua enum, e a lista de ids não muda
        assert_eq!(v["forma"]["eixo"], "Z");
        assert_eq!(v["fontes"], json!(["furo1"]));
        let r = geradas().iter().find(|g| g.tipo == "revolucao").unwrap();
        let v = r.traduzir(&json!({ "esboco": "e1", "eixo": { "origem": [0, 0, 0], "direcao": [0, 0, 1] }, "angulo": 90, "operacao": "unir" })).unwrap();
        assert_eq!(v["angulo"], "90");
        // coordenadas são números de verdade: não viram texto
        assert_eq!(v["eixo"]["origem"], json!([0, 0, 0]), "{v}");
    }

    #[test]
    fn tipo_trocado_e_recusado() {
        let g = geradas().iter().find(|g| g.tipo == "espelho").unwrap();
        assert!(g.traduzir(&json!({ "tipo": "casca", "plano": "XY" })).is_err());
    }
}
