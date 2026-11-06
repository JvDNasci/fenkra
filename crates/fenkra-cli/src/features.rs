// SPDX-License-Identifier: LGPL-2.1-or-later
//! Consulta `features` (B143): o que o reconhecimento infere num corpo, em dados para agente ou
//! script. Só lê: a conversão para comandos do Fenkra é feita no aplicativo, a pedido.
//! O texto diz sempre que a estrutura é inferida da geometria, nunca histórico restaurado.

use crate::{r6, r6v};
use fenkra_kernel::Shape;
use fenkra_solidos::reconhece::{analisar, Encontrada, Reconhecimento, Tipo};
use serde_json::{json, Value};

fn tipo_json(t: Tipo) -> &'static str {
    match t {
        Tipo::FuroPassante => "furo_passante",
        Tipo::FuroCego => "furo_cego",
        Tipo::Filete => "filete",
        Tipo::Chanfro => "chanfro",
    }
}

fn feature_json(i: usize, e: &Encontrada) -> Value {
    let mut v = json!({
        "indice": i,
        "tipo": tipo_json(e.tipo),
        "descricao": e.descricao(),
        "faces": e.faces,
        "centro": r6v(e.centro),
        "convertivel": e.convertivel,
    });
    let o = v.as_object_mut().expect("objeto");
    match e.tipo {
        Tipo::FuroPassante | Tipo::FuroCego => {
            o.insert("diametro".into(), json!(r6(e.medida)));
            o.insert("boca".into(), json!(r6v(e.boca)));
            o.insert("direcao".into(), json!(r6v(e.direcao)));
            if e.tipo == Tipo::FuroCego {
                o.insert("profundidade".into(), json!(r6(e.medida2)));
            }
            if let Some((d, a)) = e.escareado {
                o.insert("escareado".into(), json!({ "diametro": r6(d), "angulo": r6(a) }));
            }
            if let Some((d, p)) = e.rebaixo {
                o.insert("rebaixo".into(), json!({ "diametro": r6(d), "profundidade": r6(p) }));
            }
            if let Some(a) = e.ponta {
                o.insert("ponta".into(), json!(r6(a)));
            }
        }
        Tipo::Filete => {
            o.insert("raio".into(), json!(r6(e.medida)));
            o.insert("convexo".into(), json!(e.convexo));
            o.insert("angulo".into(), json!(r6(e.medida2)));
        }
        Tipo::Chanfro => {
            o.insert("distancia1".into(), json!(r6(e.medida)));
            o.insert("distancia2".into(), json!(r6(e.medida2)));
        }
    }
    if let Some(a) = &e.aviso {
        o.insert("aviso".into(), json!(a));
    }
    v
}

pub fn json_do_reconhecimento(r: &Reconhecimento) -> Value {
    json!({
        "aviso": "estrutura inferida da geometria; não é o histórico original",
        "pulada": r.pulada,
        "faces": r.faces,
        "arestas": {
            "convexas": r.arestas_convexas,
            "concavas": r.arestas_concavas,
            "tangentes": r.arestas_tangentes,
        },
        "ficam_como_corpo": { "toros": r.toros, "superficies_livres": r.livres },
        "contagem": {
            "furos_passantes": r.contar(Tipo::FuroPassante),
            "furos_cegos": r.contar(Tipo::FuroCego),
            "filetes": r.contar(Tipo::Filete),
            "chanfros": r.contar(Tipo::Chanfro),
        },
        "features": r.encontradas.iter().enumerate().map(|(i, e)| feature_json(i, e)).collect::<Vec<_>>(),
        "duvidosas": r.duvidosas,
    })
}

/// Analisa a forma e devolve o que o reconhecimento viu.
pub fn features(forma: &Shape) -> Value {
    json_do_reconhecimento(&analisar(forma))
}
