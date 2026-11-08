// SPDX-License-Identifier: LGPL-2.1-or-later
//! Seleção semântica (B420): o agente pede "as arestas da face de cima" ou "os furos de 8 mm" e recebe
//! os pontos que `filetar`, `chanfrar` e `casca` aceitam. Sem isto ele precisava ler a topologia
//! inteira, achar a face pela normal e copiar o meio de cada aresta, e errava no caminho.
//!
//! Lê só o JSON de [`crate::topologia`]: a seleção não toca no modelo.

use crate::{r6v, ErroApi};
use serde_json::{json, Value};

/// Lado da peça por nome, com a mesma convenção das vistas: frente olha para -Y, topo para +Z.
pub const LADOS: [(&str, [f64; 3]); 6] = [
    ("topo", [0., 0., 1.]),
    ("base", [0., 0., -1.]),
    ("frente", [0., -1., 0.]),
    ("tras", [0., 1., 0.]),
    ("direita", [1., 0., 0.]),
    ("esquerda", [-1., 0., 0.]),
];

/// Faces com normal a menos de ~8° da pedida contam como "olhando para lá".
const COS_LADO: f64 = 0.99;
/// Duas faces planas na mesma altura (mm) contam como a mesma camada.
const TOL_ALTURA: f64 = 1e-3;

fn xyz(v: &Value) -> Option<[f64; 3]> {
    let l = v.as_array()?;
    Some([l.first()?.as_f64()?, l.get(1)?.as_f64()?, l.get(2)?.as_f64()?])
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn direcao_do_lado(v: &Value) -> Result<Option<[f64; 3]>, ErroApi> {
    match v {
        Value::Null => Ok(None),
        Value::String(s) => LADOS
            .iter()
            .find(|(n, _)| *n == s.as_str())
            .map(|(_, d)| Some(*d))
            .ok_or_else(|| ErroApi::argumento(format!("lado '{s}' não existe; use {}", LADOS.map(|(n, _)| n).join(", ")))),
        _ => {
            let d = xyz(v).ok_or_else(|| ErroApi::argumento("`lado` é um nome (topo, base...) ou uma normal [x, y, z]"))?;
            let n = dot(d, d).sqrt();
            if n < 1e-9 {
                return Err(ErroApi::argumento("a normal de `lado` não pode ser nula"));
            }
            Ok(Some(d.map(|c| c / n)))
        }
    }
}

fn confere_raio(item: &Value, raio: Option<f64>) -> bool {
    match raio {
        None => true,
        Some(r) => item["raio"].as_f64().is_some_and(|x| (x - r).abs() <= 1e-3_f64.max(r * 1e-4)),
    }
}

/// Índices das faces planas que olham para `d`. `extrema` fica só com a camada mais afastada, que é o
/// que a pessoa chama de "a face de cima".
fn faces_do_lado(faces: &[Value], d: [f64; 3], extrema: bool) -> Vec<usize> {
    let do_lado: Vec<(usize, f64)> = faces
        .iter()
        .enumerate()
        .filter(|(_, f)| f["tipo"] == "plano")
        .filter_map(|(i, f)| {
            let n = xyz(&f["direcao"])?;
            let p = xyz(&f["ponto"])?;
            (dot(n, d) >= COS_LADO).then(|| (i, dot(p, d)))
        })
        .collect();
    let topo = do_lado.iter().map(|(_, h)| *h).fold(f64::NEG_INFINITY, f64::max);
    do_lado
        .into_iter()
        .filter(|(_, h)| !extrema || (topo - h).abs() <= TOL_ALTURA)
        .map(|(i, _)| i)
        .collect()
}

/// Escolhe faces ou arestas da topologia (`faces` e `arestas` completas) pelo pedido.
pub fn selecionar(topologia: &Value, args: &Value) -> Result<Value, ErroApi> {
    let alvo = args["alvo"].as_str().unwrap_or("arestas");
    if !["arestas", "faces"].contains(&alvo) {
        return Err(ErroApi::argumento("`alvo` é \"arestas\" ou \"faces\""));
    }
    let lado = direcao_do_lado(&args["lado"])?;
    let extrema = args["extrema"].as_bool().unwrap_or(true);
    let tipo = args["tipo"].as_str();
    let raio = match &args["raio"] {
        Value::Null => None,
        v => Some(v.as_f64().ok_or_else(|| ErroApi::argumento("`raio` é um número em mm"))?),
    };
    let vazio = vec![];
    let faces = topologia["faces"].as_array().unwrap_or(&vazio);
    let arestas = topologia["arestas"].as_array().unwrap_or(&vazio);
    let base: Option<Vec<usize>> = lado.map(|d| faces_do_lado(faces, d, extrema));

    let escolhidos: Vec<Value> = if alvo == "faces" {
        faces
            .iter()
            .enumerate()
            .filter(|(i, _)| base.as_ref().is_none_or(|b| b.contains(i)))
            .filter(|(_, f)| tipo.is_none_or(|t| f["tipo"] == t))
            .filter(|(_, f)| confere_raio(f, raio))
            .map(|(i, f)| json!({ "indice": i, "tipo": f["tipo"], "ponto": f["ponto"], "area": f["area"], "raio": f["raio"] }))
            .collect()
    } else {
        arestas
            .iter()
            .filter(|a| {
                base.as_ref().is_none_or(|b| {
                    a["faces"].as_array().is_some_and(|l| l.iter().any(|f| f.as_u64().is_some_and(|f| b.contains(&(f as usize)))))
                })
            })
            .filter(|a| tipo.is_none_or(|t| a["tipo"] == t))
            .filter(|a| confere_raio(a, raio))
            .filter(|a| !a["meio"].is_null())
            .map(|a| json!({ "indice": a["indice"], "tipo": a["tipo"], "ponto": a["meio"], "raio": a["raio"] }))
            .collect()
    };
    let mut escolhidos = escolhidos;
    // "a aresta mais perto deste ponto": ordena pela distância e fica com as `quantas` primeiras
    if !args["perto"].is_null() {
        let p = xyz(&args["perto"]).ok_or_else(|| ErroApi::argumento("`perto` é um ponto [x, y, z] em mm"))?;
        let d = |e: &Value| xyz(&e["ponto"]).map_or(f64::INFINITY, |q| ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) + (q[2] - p[2]).powi(2)).sqrt());
        escolhidos.sort_by(|a, b| d(a).total_cmp(&d(b)));
        escolhidos.truncate(args["quantas"].as_u64().unwrap_or(1).max(1) as usize);
        for e in &mut escolhidos {
            e["distancia"] = json!(crate::r6(d(e)));
        }
    }
    let pontos: Vec<Value> = escolhidos
        .iter()
        .filter_map(|e| xyz(&e["ponto"]).map(|p| json!(r6v(p))))
        .collect();
    let mut saida = json!({ "alvo": alvo, "quantidade": escolhidos.len(), "pontos": pontos, "itens": escolhidos });
    if escolhidos.is_empty() {
        saida["aviso"] = json!("nada bate com o pedido; confira o lado, o tipo e o raio com topologia_modelo");
    } else if alvo == "arestas" {
        saida["uso"] = json!("passe `pontos` em `arestas` de filetar ou chanfrar");
    } else {
        saida["uso"] = json!("passe `pontos` em `faces` de criar_casca para abrir essas faces");
    }
    Ok(saida)
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Caixa 10 × 10 × 10 de 0 a 10: seis faces planas e as doze arestas.
    fn caixa() -> Value {
        let faces: Vec<Value> = LADOS
            .iter()
            .map(|(_, d)| json!({ "tipo": "plano", "direcao": d, "ponto": d.map(|c| 5. + 5. * c), "area": 100. }))
            .collect();
        // aresta entre a face i e a face j (lados perpendiculares)
        let mut arestas = vec![];
        for i in 0..6 {
            for j in (i + 1)..6 {
                let (a, b) = (LADOS[i].1, LADOS[j].1);
                if dot(a, b).abs() < 0.5 {
                    let meio = [0, 1, 2].map(|k| 5. + 5. * a[k] + 5. * b[k]);
                    arestas.push(json!({ "indice": arestas.len(), "tipo": "reta", "faces": [i, j], "meio": meio }));
                }
            }
        }
        json!({ "faces": faces, "arestas": arestas })
    }

    #[test]
    fn arestas_da_face_de_cima_sao_as_quatro_do_topo() {
        let r = selecionar(&caixa(), &json!({ "lado": "topo" })).unwrap();
        assert_eq!(r["quantidade"], 4, "{r}");
        for p in r["pontos"].as_array().unwrap() {
            assert_eq!(p[2].as_f64(), Some(10.), "{r}");
        }
    }

    #[test]
    fn face_por_normal_livre_e_nome_errado_explica() {
        let r = selecionar(&caixa(), &json!({ "alvo": "faces", "lado": [0, 0, -2] })).unwrap();
        assert_eq!(r["quantidade"], 1, "{r}");
        assert_eq!(r["pontos"][0], json!([5., 5., 0.]));
        let e = selecionar(&caixa(), &json!({ "lado": "cima" })).unwrap_err();
        assert!(e.mensagem.contains("topo"), "{}", e.mensagem);
    }

    #[test]
    fn aresta_mais_perto_do_ponto() {
        let r = selecionar(&caixa(), &json!({ "perto": [10, 10, 9] })).unwrap();
        assert_eq!(r["quantidade"], 1, "{r}");
        assert_eq!(r["pontos"][0], json!([10., 10., 5.]), "{r}");
        let r = selecionar(&caixa(), &json!({ "lado": "topo", "perto": [0, 0, 20], "quantas": 2 })).unwrap();
        assert_eq!(r["quantidade"], 2, "{r}");
    }

    #[test]
    fn filtro_que_nao_bate_avisa_em_vez_de_devolver_tudo() {
        let r = selecionar(&caixa(), &json!({ "tipo": "circulo" })).unwrap();
        assert_eq!(r["quantidade"], 0);
        assert!(r["aviso"].is_string());
    }
}
