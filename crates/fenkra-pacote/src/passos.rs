// SPDX-License-Identifier: LGPL-2.1-or-later
//! Passos das explosões e balões da lista de peças no pacote (B181).
//!
//! O navegador não recalcula a explosão: cada passo já vai com quem se moveu, o deslocamento
//! acumulado de cada componente, a seta, a lista de peças do passo e os balões. A lista do passo é
//! calculada a partir das peças que saíram do lugar, nunca digitada, e cada balão leva o número do
//! item da lista da montagem (o mesmo `item` de `lista`), então balão e lista não se desencontram.

use fenkra_core::Id;
use fenkra_montagem::Assembly;
use serde_json::{json, Value};
use std::collections::HashMap;

type V3 = [f64; 3];

fn r6(x: f64) -> f64 {
    let v = (x * 1e6).round() / 1e6;
    if v == 0. {
        0.
    } else {
        v
    }
}

/// Centro de cada nó da árvore no mundo: média das posições da malha das peças abaixo dele.
pub fn centros(somas: &HashMap<String, (V3, f64)>) -> HashMap<String, V3> {
    somas.iter().filter(|(_, s)| s.1 > 0.).map(|(k, (s, n))| (k.clone(), s.map(|v| v / n))).collect()
}

/// As peças (entradas `peca`) em `id` ou abaixo dele, na ordem da árvore.
fn pecas_de<'a>(arvore: &'a [Value], id: &str) -> Vec<&'a Value> {
    let pai: HashMap<&'a str, &'a str> = arvore.iter().filter_map(|e| Some((e["id"].as_str()?, e["pai"].as_str()?))).collect();
    let abaixo = |inicio: &'a str| {
        let mut x = inicio;
        loop {
            if x == id {
                return true;
            }
            match pai.get(x) {
                Some(p) => x = p,
                None => return false,
            }
        }
    };
    arvore.iter().filter(|e| e["tipo"] == "peca" && e["id"].as_str().is_some_and(abaixo)).collect()
}

fn item_de<'a>(lista: &'a Value, arquivo: &str) -> Option<&'a Value> {
    lista.as_array()?.iter().find(|r| r["arquivo"] == arquivo)
}

/// Balões: um por item da lista, na primeira ocorrência dele entre `pecas`, no centro dela mais o
/// deslocamento do componente que a levou (`desl`, id da árvore e vetor).
fn baloes_de(pecas: &[&Value], lista: &Value, centros: &HashMap<String, V3>, desl: &dyn Fn(&str) -> V3) -> Value {
    let mut vistos = vec![];
    let mut out = vec![];
    for e in pecas {
        let (Some(id), Some(arq)) = (e["id"].as_str(), e["arquivo"].as_str()) else { continue };
        let Some(r) = item_de(lista, arq) else { continue };
        if vistos.contains(&arq) {
            continue;
        }
        vistos.push(arq);
        let Some(c) = centros.get(id) else { continue };
        let d = desl(id);
        let ponto = [0, 1, 2].map(|i| r6(c[i] + d[i]));
        out.push(json!({"item": r["item"], "id": id, "arquivo": arq, "ponto": ponto}));
    }
    json!(out)
}

/// Balões da montagem montada: um por item da lista.
pub fn baloes(arvore: &[Value], lista: &Value, centros: &HashMap<String, V3>, _desl: &[(String, V3)]) -> Value {
    let pecas: Vec<&Value> = arvore.iter().filter(|e| e["tipo"] == "peca").collect();
    baloes_de(&pecas, lista, centros, &|_| [0.; 3])
}

/// Lista de peças de um conjunto de entradas: quantidade por arquivo, com o número do item da
/// lista da montagem.
fn lista_de(pecas: &[&Value], lista: &Value) -> Value {
    let mut linhas: Vec<(String, u32, String)> = vec![];
    for e in pecas {
        let Some(arq) = e["arquivo"].as_str() else { continue };
        match linhas.iter_mut().find(|l| l.0 == arq) {
            Some(l) => l.1 += 1,
            None => linhas.push((arq.to_string(), 1, e["nome"].as_str().unwrap_or_default().to_string())),
        }
    }
    json!(linhas
        .iter()
        .map(|(arq, q, nome)| {
            let r = item_de(lista, arq);
            json!({"item": r.map_or(Value::Null, |r| r["item"].clone()), "quantidade": q, "nome": r.map_or_else(|| json!(nome), |r| r["nome"].clone()), "arquivo": arq})
        })
        .collect::<Vec<_>>())
}

/// As explosões da montagem, passo a passo, com tudo já calculado.
pub fn explosoes(asm: &Assembly, arvore: &[Value], lista: &Value, centros: &HashMap<String, V3>) -> Value {
    // a explosão mexe nos componentes deste nível: `n<id>` sem prefixo
    let por_id: HashMap<Id, V3> = asm.nodes.iter().filter_map(|n| centros.get(&format!("n{}", n.id.0)).map(|c| (n.id, *c))).collect();
    let pai: HashMap<&str, &str> = arvore.iter().filter_map(|e| Some((e["id"].as_str()?, e["pai"].as_str()?))).collect();
    let mut out = vec![];
    for e in &asm.explosoes {
        let fases = e.fases(&por_id);
        let mut passos = vec![];
        for f in &fases {
            let p = &e.passos[f.passo];
            let offs: HashMap<String, V3> = f.offsets.iter().map(|(c, d)| (format!("n{}", c.0), *d)).collect();
            // deslocamento de uma peça: o dela somado ao de cada ancestral que saiu do lugar, como na
            // cena, em que o nó do grupo leva os filhos
            let desl = |id: &str| {
                let (mut x, mut t) = (id, [0.; 3]);
                loop {
                    if let Some(d) = offs.get(x) {
                        t = [0, 1, 2].map(|i| t[i] + d[i]);
                    }
                    match pai.get(x) {
                        Some(q) => x = q,
                        None => return t,
                    }
                }
            };
            let movidas: Vec<String> = f.movidas.iter().map(|c| format!("n{}", c.0)).collect();
            let mut pecas: Vec<&Value> = vec![];
            for m in &movidas {
                for x in pecas_de(arvore, m) {
                    if !pecas.iter().any(|y| y["id"] == x["id"]) {
                        pecas.push(x);
                    }
                }
            }
            let seta = e.seta(f.passo, &por_id).map(|(c, d)| json!({"ponto": c.map(r6), "direcao": d.map(r6)}));
            passos.push(json!({
                "indice": f.passo, "nome": p.nome, "modo": p.modo.label(), "distancia_mm": r6(p.modo.dist()),
                "movidas": movidas,
                "deslocamentos": f.offsets.iter().map(|(c, d)| json!({"id": format!("n{}", c.0), "d": d.map(r6)})).collect::<Vec<_>>(),
                "seta": seta,
                "lista": lista_de(&pecas, lista),
                "baloes": baloes_de(&pecas, lista, centros, &desl),
            }));
        }
        let linhas: Vec<Value> = if e.linhas {
            e.linhas(&por_id).iter().map(|l| json!({"id": format!("n{}", l.comp.0), "pontos": l.pontos.iter().map(|q| q.map(r6)).collect::<Vec<_>>()})).collect()
        } else {
            vec![]
        };
        out.push(json!({"nome": e.nome, "passos": passos, "linhas": linhas}));
    }
    json!(out)
}
