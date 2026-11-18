// SPDX-License-Identifier: LGPL-2.1-or-later
//! B509: medida mantida na árvore (pasta Medidas).
//!
//! A medida guarda só as referências e o que medir; o valor é refeito a cada cálculo a partir do
//! corpo atual (ver `fenkra-app/src/medir.rs`). Cada referência é uma chave v2 (ADR 0010): a face ou
//! a aresta é achada de novo pelo dono, pelo papel e pela assinatura, e o ponto clicado só desempata.
//! Referência que não se acha mais deixa a medida "órfã": a interface sinaliza e não mostra valor
//! velho. Não é anotação (PMI): não entra no desenho nem na troca de dados.

use crate::{EdgeKey, FaceKey, Id};
use serde::{Deserialize, Serialize};

/// Que tipo de elemento a referência aponta.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ElemMedida {
    Face,
    Aresta,
    Vertice,
}

/// Uma referência de medida: o elemento, o ponto clicado (desempate e reserva) e a chave v2.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RefMedida {
    pub elem: ElemMedida,
    pub ponto: [f64; 3],
    /// face (para `Face`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face: Option<FaceKey>,
    /// aresta (para `Aresta`, e para `Vertice`: o vértice é a ponta da aresta mais perto do ponto)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aresta: Option<EdgeKey>,
}

/// Medir um elemento ou medir entre duas listas.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum ModoMedida {
    #[default]
    Item,
    Entre,
}

/// Medida mantida (item da pasta Medidas).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Medida {
    pub id: Id,
    pub nome: String,
    pub modo: ModoMedida,
    /// elemento medido (Item) ou primeira lista (Entre)
    pub a: Vec<RefMedida>,
    /// segunda lista (Entre)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub b: Vec<RefMedida>,
    /// tratar plano e reta como ilimitados (Entre)
    #[serde(default)]
    pub infinito: bool,
    /// rótulos (em português, como o cálculo os escreve) que a medida mostra; vazio = todos
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub campos: Vec<String>,
}

impl crate::Document {
    /// Guarda uma medida nova na pasta Medidas, com o nome padrão no idioma da interface (B510).
    pub fn add_medida(&mut self, modo: ModoMedida, a: Vec<RefMedida>, b: Vec<RefMedida>, infinito: bool) -> Id {
        let id = self.alloc();
        let nome = self.nome_padrao("Medida");
        self.medidas.push(Medida { id, nome, modo, a, b, infinito, campos: vec![] });
        id
    }

    pub fn medida(&self, id: Id) -> Option<&Medida> {
        self.medidas.iter().find(|m| m.id == id)
    }

    pub fn remover_medida(&mut self, id: Id) -> bool {
        let n = self.medidas.len();
        self.medidas.retain(|m| m.id != id);
        self.medidas.len() != n
    }
}
