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

/// Minúsculas e sem acento, para "superior" e "SUPERIOR", "aresta" e "arestas" se acharem.
fn normalizar(t: &str) -> String {
    t.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' => 'u',
            'ç' => 'c',
            c if c.is_alphanumeric() || c == '.' || c == ',' => c,
            _ => ' ',
        })
        .collect()
}

/// Palavras de lado, na convenção de [`LADOS`].
const PALAVRAS_LADO: [(&str, &str); 16] = [
    ("superior", "topo"),
    ("cima", "topo"),
    ("topo", "topo"),
    ("inferior", "base"),
    ("baixo", "base"),
    ("base", "base"),
    ("fundo", "base"),
    ("frontal", "frente"),
    ("frente", "frente"),
    ("traseira", "tras"),
    ("tras", "tras"),
    ("posterior", "tras"),
    ("direita", "direita"),
    ("direito", "direita"),
    ("esquerda", "esquerda"),
    ("esquerdo", "esquerda"),
];

/// Tipos por palavra: (raiz, alvo, tipo).
const PALAVRAS_TIPO: [(&str, &str, &str); 9] = [
    ("plana", "faces", "plano"),
    ("cilindric", "faces", "cilindro"),
    ("conica", "faces", "cone"),
    ("esferic", "faces", "esfera"),
    ("toroidal", "faces", "toro"),
    ("reta", "arestas", "reta"),
    ("circul", "arestas", "circulo"),
    ("arco", "arestas", "arco"),
    ("elipse", "arestas", "elipse"),
];

fn numero(p: &str) -> Option<f64> {
    p.trim_end_matches("mm").replace(',', ".").trim_end_matches('.').parse().ok().filter(|v: &f64| v.is_finite() && *v > 0.)
}

/// Seleção semântica por descrição (B420): "face superior", "aresta mais longa", "maior face",
/// "arestas da face de cima", "furos de 8 mm", "círculos de raio 4", "as 2 arestas mais curtas da
/// frente". Vira os mesmos campos de [`selecionar`]; o que não se entende é recusado com o vocabulário,
/// em vez de devolver tudo.
pub fn interpretar(descricao: &str) -> Result<Value, ErroApi> {
    let texto = normalizar(descricao);
    let palavras: Vec<&str> = texto.split_whitespace().collect();
    let tem = |raiz: &str| palavras.iter().any(|p| p.starts_with(raiz));
    let pos = |raiz: &str| palavras.iter().position(|p| p.starts_with(raiz));
    let mut v = json!({});
    let furo = tem("furo");
    // alvo: o que vem primeiro manda. "arestas da face de cima" são arestas; furo é face cilíndrica
    let alvo = match (pos("aresta"), pos("face")) {
        (Some(a), Some(f)) if a < f => "arestas",
        (Some(_), Some(_)) => "faces",
        (Some(_), None) => "arestas",
        (None, Some(_)) => "faces",
        (None, None) if furo => "faces",
        _ => "",
    };
    if !alvo.is_empty() {
        v["alvo"] = json!(alvo);
    }
    let mut filtro = false;
    if let Some((_, lado)) = PALAVRAS_LADO.iter().find(|(p, _)| palavras.contains(p)) {
        v["lado"] = json!(lado);
        filtro = true;
    }
    if furo {
        v["tipo"] = json!("cilindro");
        filtro = true;
    } else if let Some((_, a, t)) = PALAVRAS_TIPO.iter().find(|(r, _, _)| tem(r)) {
        if v["alvo"].is_null() {
            v["alvo"] = json!(a);
        }
        // "face circular" não existe: o tipo só vale no alvo dele
        if v["alvo"] == *a {
            v["tipo"] = json!(t);
            filtro = true;
        }
    }
    // medida: "raio 4", "diâmetro 8", "de 8 mm" (em furo é diâmetro, no resto é raio)
    for (i, p) in palavras.iter().enumerate() {
        let Some(n) = palavras.get(i + 1).and_then(|q| numero(q)) else { continue };
        if p.starts_with("raio") {
            v["raio"] = json!(n);
        } else if p.starts_with("diametro") {
            v["raio"] = json!(n / 2.);
        } else if *p == "de" && v["raio"].is_null() && (palavras.get(i + 2) == Some(&"mm") || palavras[i + 1].ends_with("mm")) {
            v["raio"] = json!(if furo { n / 2. } else { n });
        } else {
            continue;
        }
        filtro = true;
    }
    // ordem: mais longa, maior, mais curta, menor; "as 3 maiores" diz quantas
    let maior = ["maior", "longa", "longo", "comprid", "extens"].iter().any(|r| tem(r));
    let menor = ["menor", "curta", "curto", "pequen"].iter().any(|r| tem(r));
    if maior || menor {
        v["ordem"] = json!(if maior { "maior" } else { "menor" });
        let quantas = palavras.iter().find_map(|p| p.parse::<u64>().ok().filter(|n| *n > 0 && *n < 1000)).unwrap_or(1);
        v["quantas"] = json!(quantas);
        filtro = true;
    }
    if tem("tod") {
        filtro = true;
    }
    // só "a face", sem lado, tipo, medida nem ordem, não escolhe nada: recusa em vez de devolver tudo
    if v["alvo"].is_null() || !filtro {
        return Err(ErroApi::argumento(format!(
            "não entendi '{descricao}'. Diga o alvo (face, aresta, furo) e um filtro: lado (superior, inferior, frente, trás, direita, esquerda), tipo (plana, cilíndrica, reta, circular), medida (raio 4, diâmetro 8, de 8 mm) ou ordem (mais longa, maior, mais curta, menor); \"todas as arestas\" pega todas"
        )));
    }
    Ok(v)
}

/// Escolhe faces ou arestas da topologia (`faces` e `arestas` completas) pelo pedido.
pub fn selecionar(topologia: &Value, args: &Value) -> Result<Value, ErroApi> {
    // a descrição em texto preenche os campos que o pedido não trouxe: o explícito manda
    let juntos;
    let args = match args["descricao"].as_str() {
        Some(d) => {
            let mut v = interpretar(d)?;
            for (k, x) in args.as_object().into_iter().flatten().filter(|(k, _)| *k != "descricao") {
                v[k] = x.clone();
            }
            juntos = v;
            &juntos
        }
        None => args,
    };
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
            .map(|a| json!({ "indice": a["indice"], "tipo": a["tipo"], "ponto": a["meio"], "raio": a["raio"], "comprimento": a["comprimento"] }))
            .collect()
    };
    let mut escolhidos = escolhidos;
    // "a aresta mais longa", "a maior face": ordena por comprimento (arestas) ou área (faces). Empates
    // (as quatro arestas iguais de um quadrado) vêm todos, para o agente não receber um ao acaso
    if let Some(ordem) = args["ordem"].as_str() {
        let maior = match ordem {
            "maior" => true,
            "menor" => false,
            o => return Err(ErroApi::argumento(format!("ordem '{o}' não existe; use maior ou menor"))),
        };
        let campo = if alvo == "faces" { "area" } else { "comprimento" };
        let medida = |e: &Value| e[campo].as_f64().unwrap_or(0.);
        escolhidos.sort_by(|a, b| if maior { medida(b).total_cmp(&medida(a)) } else { medida(a).total_cmp(&medida(b)) });
        let quantas = args["quantas"].as_u64().unwrap_or(1).max(1) as usize;
        if let Some(corte) = escolhidos.get(quantas - 1).map(medida) {
            let tol = 1e-6_f64.max(corte.abs() * 1e-6);
            escolhidos.retain(|e| if maior { medida(e) >= corte - tol } else { medida(e) <= corte + tol });
        }
    }
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

    /// Bloco 30 x 20 x 10: as arestas de 30 são as mais longas, a face de cima (600) é das maiores.
    fn bloco() -> Value {
        let (lx, ly, lz) = (30., 20., 10.);
        let faces = json!([
            { "tipo": "plano", "direcao": [0, 0, 1], "ponto": [15, 10, lz], "area": lx * ly },
            { "tipo": "plano", "direcao": [0, 0, -1], "ponto": [15, 10, 0], "area": lx * ly },
            { "tipo": "plano", "direcao": [0, -1, 0], "ponto": [15, 0, 5], "area": lx * lz },
            { "tipo": "plano", "direcao": [0, 1, 0], "ponto": [15, ly, 5], "area": lx * lz },
            { "tipo": "plano", "direcao": [1, 0, 0], "ponto": [lx, 10, 5], "area": ly * lz },
            { "tipo": "plano", "direcao": [-1, 0, 0], "ponto": [0, 10, 5], "area": ly * lz },
            { "tipo": "cilindro", "direcao": [0, 0, 1], "ponto": [19, 10, 5], "area": 251.3, "raio": 4 },
        ]);
        let a = |i: usize, f: [usize; 2], meio: [f64; 3], c: f64| json!({ "indice": i, "tipo": "reta", "faces": f, "meio": meio, "comprimento": c });
        let arestas = json!([
            a(0, [0, 2], [15., 0., lz], lx), a(1, [0, 3], [15., ly, lz], lx), a(2, [0, 4], [lx, 10., lz], ly), a(3, [0, 5], [0., 10., lz], ly),
            a(4, [1, 2], [15., 0., 0.], lx), a(5, [1, 3], [15., ly, 0.], lx), a(6, [2, 4], [lx, 0., 5.], lz),
            { "indice": 7, "tipo": "circulo", "faces": [0, 6], "meio": [19, 10, lz], "raio": 4, "comprimento": 25.13 },
        ]);
        json!({ "faces": faces, "arestas": arestas })
    }

    #[test]
    fn descricao_em_texto_vira_os_campos() {
        assert_eq!(interpretar("face superior").unwrap(), json!({ "alvo": "faces", "lado": "topo" }));
        assert_eq!(interpretar("a aresta mais longa").unwrap(), json!({ "alvo": "arestas", "ordem": "maior", "quantas": 1 }));
        assert_eq!(interpretar("Arestas da face de cima").unwrap(), json!({ "alvo": "arestas", "lado": "topo" }));
        assert_eq!(interpretar("furos de 8 mm").unwrap(), json!({ "alvo": "faces", "tipo": "cilindro", "raio": 4.0 }));
        assert_eq!(interpretar("círculos de raio 4").unwrap(), json!({ "alvo": "arestas", "tipo": "circulo", "raio": 4.0 }));
        assert_eq!(interpretar("as 2 maiores faces").unwrap(), json!({ "alvo": "faces", "ordem": "maior", "quantas": 2 }));
        assert_eq!(interpretar("todas as arestas").unwrap(), json!({ "alvo": "arestas" }));
        for ruim in ["a face", "coisa bonita", ""] {
            let e = interpretar(ruim).unwrap_err();
            assert!(e.mensagem.contains("superior"), "{ruim}: {}", e.mensagem);
        }
    }

    #[test]
    fn face_superior_aresta_mais_longa_e_maior_face_no_bloco() {
        let t = bloco();
        let r = selecionar(&t, &json!({ "descricao": "face superior" })).unwrap();
        assert_eq!((r["quantidade"].as_u64(), &r["pontos"][0]), (Some(1), &json!([15., 10., 10.])), "{r}");
        // as quatro arestas de 30 mm empatam: vêm todas, nenhuma ao acaso
        let r = selecionar(&t, &json!({ "descricao": "aresta mais longa" })).unwrap();
        assert_eq!(r["quantidade"], 4, "{r}");
        assert!(r["itens"].as_array().unwrap().iter().all(|i| i["comprimento"] == 30.), "{r}");
        // com lado: na face direita, a de cima tem 20 mm e a vertical 10
        let r = selecionar(&t, &json!({ "descricao": "aresta mais curta da direita" })).unwrap();
        assert_eq!((r["quantidade"].as_u64(), &r["pontos"][0]), (Some(1), &json!([30., 0., 5.])), "{r}");
        let r = selecionar(&t, &json!({ "descricao": "maior face" })).unwrap();
        assert_eq!(r["quantidade"], 2, "topo e base empatam em 600: {r}");
        let r = selecionar(&t, &json!({ "descricao": "furo de 8 mm" })).unwrap();
        assert_eq!((r["quantidade"].as_u64(), &r["pontos"][0]), (Some(1), &json!([19., 10., 5.])), "{r}");
        // o campo explícito manda sobre a descrição
        let r = selecionar(&t, &json!({ "descricao": "face superior", "lado": "base" })).unwrap();
        assert_eq!(r["pontos"][0], json!([15., 10., 0.]), "{r}");
        assert!(selecionar(&t, &json!({ "descricao": "uma face" })).is_err());
    }

    #[test]
    fn filtro_que_nao_bate_avisa_em_vez_de_devolver_tudo() {
        let r = selecionar(&caixa(), &json!({ "tipo": "circulo" })).unwrap();
        assert_eq!(r["quantidade"], 0);
        assert!(r["aviso"].is_string());
    }
}
