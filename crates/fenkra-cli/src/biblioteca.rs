// SPDX-License-Identifier: LGPL-2.1-or-later
//! Consulta à biblioteca de normas (B420): famílias de fixadores, tamanhos e a faixa de furo de
//! passagem de cada um. O agente que fura uma peça para um M6 lê o diâmetro aqui em vez de chutar.
//! Só lê a camada pública da biblioteca.

use crate::{r6, ErroApi};
use fenkra_solidos::biblioteca::{familia, familias, Familia, Tamanho};
use serde_json::{json, Value};

fn familia_resumo(f: &Familia) -> Value {
    json!({ "id": f.id, "nome": f.nome, "norma": f.norma, "tipo": f.tipo.rotulo(), "classe": f.classe, "tamanhos": f.tamanhos.len() })
}

fn tamanho_json(f: &Familia, t: &Tamanho) -> Value {
    let mut v = json!({ "designacao": t.designacao, "diametro_nominal": t.diametro_nominal().map(r6), "medidas": t.valores });
    if let Some((a, b)) = f.faixa_furo(t) {
        v["furo_passagem"] = json!({ "min": r6(a), "max": r6(b) });
    }
    if !t.comprimentos.is_empty() || !f.comprimentos.is_empty() {
        v["comprimentos"] = json!(if t.comprimentos.is_empty() { &f.comprimentos } else { &t.comprimentos });
    }
    v
}

/// `{}` lista as famílias; `busca` filtra por nome, norma ou id; `familia` traz os tamanhos;
/// `familia` + `tamanho` traz um só; `furo` (mm) diz que tamanhos passam num furo desse diâmetro.
pub fn consultar(args: &Value) -> Result<Value, ErroApi> {
    let Some(id) = args["familia"].as_str() else {
        let busca = args["busca"].as_str().map(str::to_lowercase);
        let lista: Vec<Value> = familias()
            .iter()
            .filter(|f| busca.as_ref().is_none_or(|b| [&f.id, &f.nome, &f.norma].iter().any(|t| t.to_lowercase().contains(b.as_str()))))
            .map(familia_resumo)
            .collect();
        return Ok(json!({ "familias": lista }));
    };
    let f = familia(id).ok_or_else(|| ErroApi::novo("argumento", format!("família '{id}' não existe; consulte sem `familia` para ver a lista")))?;
    let mut saida = familia_resumo(f);
    if let Some(d) = args["furo"].as_f64() {
        saida["cabem_no_furo"] = json!(f.tamanhos_para_furo(d, 1e-6).iter().map(|t| t.designacao.clone()).collect::<Vec<_>>());
    }
    match args["tamanho"].as_str() {
        Some(t) => {
            let chave = Tamanho::chave_de(t);
            let tam = f.tamanhos.iter().find(|x| x.chave() == chave).ok_or_else(|| ErroApi::argumento(format!("a família '{id}' não tem o tamanho '{t}'")))?;
            saida["tamanho"] = tamanho_json(f, tam);
        }
        None => saida["lista"] = json!(f.tamanhos.iter().map(|t| tamanho_json(f, t)).collect::<Vec<_>>()),
    }
    Ok(saida)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn lista_familias_e_traz_furo_de_passagem_do_m6() {
        let r = consultar(&json!({})).unwrap();
        let l = r["familias"].as_array().unwrap();
        assert!(!l.is_empty(), "{r}");
        // primeira família que tem M6
        let id = familias().iter().find(|f| f.tamanhos.iter().any(|t| t.designacao == "M6")).map(|f| f.id.clone()).expect("biblioteca sem M6");
        let r = consultar(&json!({ "familia": id, "tamanho": "m 6" })).unwrap();
        let furo = &r["tamanho"]["furo_passagem"];
        let (a, b) = (furo["min"].as_f64().unwrap(), furo["max"].as_f64().unwrap());
        assert!(a > 6. && b > a && b < 8., "{r}");
        let r = consultar(&json!({ "familia": id, "furo": 6.6 })).unwrap();
        assert!(r["cabem_no_furo"].as_array().unwrap().iter().any(|t| t == "M6"), "{r}");
    }

    #[test]
    fn familia_inexistente_explica() {
        let e = consultar(&json!({ "familia": "nao-existe" })).unwrap_err();
        assert!(e.mensagem.contains("nao-existe"));
    }
}
