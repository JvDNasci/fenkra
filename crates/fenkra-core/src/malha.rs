// SPDX-License-Identifier: LGPL-2.1-or-later
//! Comandos do corpo de malha (B170). O documento só guarda o que foi pedido; quem faz a conta é o
//! `fenkra-solidos`, com o núcleo de `fenkra-malha`.

use crate::doc::{BoolOp, Id};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;

/// Texto de uma malha embutida. É um `Arc`: o documento é copiado a cada desfazer, e megabytes de
/// malha não podem ir junto. A depuração mostra só o tamanho.
#[derive(Clone, PartialEq, Eq)]
pub struct MalhaEmbutida(pub Arc<str>);

impl std::fmt::Debug for MalhaEmbutida {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MalhaEmbutida({} bytes)", self.0.len())
    }
}

impl Serialize for MalhaEmbutida {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for MalhaEmbutida {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(MalhaEmbutida(String::deserialize(d)?.into()))
    }
}

fn verdade() -> bool {
    true
}

/// Reparo pedido. As tolerâncias são expressões, como nos outros comandos.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MeshRepair {
    /// distância da solda, em mm; zero junta só vértices idênticos
    pub solda: String,
    #[serde(default = "verdade")]
    pub nao_manifold: bool,
    #[serde(default = "verdade")]
    pub normais: bool,
    #[serde(default = "verdade")]
    pub furos: bool,
    /// maior furo fechado, em arestas; zero é sem limite
    #[serde(default)]
    pub furo_maximo: u32,
    #[serde(default)]
    pub soltas: bool,
    /// parte que fica, como fração da área da maior
    #[serde(default)]
    pub solta_minima: String,
}

impl Default for MeshRepair {
    fn default() -> Self {
        MeshRepair { solda: "0.001".into(), nao_manifold: true, normais: true, furos: true, furo_maximo: 0, soltas: false, solta_minima: "0.001".into() }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MeshDecimate {
    /// fração dos triângulos que fica, de 0 a 1
    pub proporcao: String,
    /// desvio máximo em mm; zero desliga
    pub desvio: String,
}

impl Default for MeshDecimate {
    fn default() -> Self {
        MeshDecimate { proporcao: "0.5".into(), desvio: "0".into() }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MeshBoolean {
    /// coletor do corpo ferramenta; corpo B-rep entra pela malha dele
    pub tool: Id,
    pub op: BoolOp,
    pub keep_tool: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum MeshOpKind {
    Reparar(MeshRepair),
    Decimar(MeshDecimate),
    Booleana(MeshBoolean),
    /// o corpo de malha vira B-rep facetado: é o único caminho com teto de triângulos
    ParaBrep { solid: bool },
    /// o corpo B-rep do coletor vira corpo de malha
    DeBrep { desvio: String, angulo: String },
}

impl MeshOpKind {
    pub fn nome(&self) -> &'static str {
        match self {
            MeshOpKind::Reparar(_) => "Reparar malha",
            MeshOpKind::Decimar(_) => "Decimar malha",
            MeshOpKind::Booleana(_) => "Booleana de malha",
            MeshOpKind::ParaBrep { .. } => "Malha para sólido",
            MeshOpKind::DeBrep { .. } => "Sólido para malha",
        }
    }

    pub fn walk(&mut self, ids: &mut dyn FnMut(&mut Id), exprs: &mut dyn FnMut(&mut String)) {
        match self {
            MeshOpKind::Reparar(r) => {
                exprs(&mut r.solda);
                exprs(&mut r.solta_minima);
            }
            MeshOpKind::Decimar(d) => {
                exprs(&mut d.proporcao);
                exprs(&mut d.desvio);
            }
            MeshOpKind::Booleana(b) => ids(&mut b.tool),
            MeshOpKind::ParaBrep { .. } => {}
            MeshOpKind::DeBrep { desvio, angulo } => {
                exprs(desvio);
                exprs(angulo);
            }
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MeshOp {
    pub op: MeshOpKind,
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn embutida_volta_igual_e_nao_aparece_na_depuracao() {
        let m = MalhaEmbutida("abc".repeat(10).into());
        let j = serde_json::to_string(&m).unwrap();
        assert_eq!(serde_json::from_str::<MalhaEmbutida>(&j).unwrap(), m);
        assert_eq!(format!("{m:?}"), "MalhaEmbutida(30 bytes)");
    }

    #[test]
    fn documento_antigo_le_comando_sem_os_campos_novos() {
        let k: crate::doc::MeshImport = serde_json::from_str(r#"{"path":"a.stl","solid":true}"#).unwrap();
        assert!(!k.corpo && k.dados.is_empty());
    }

    #[test]
    fn walk_passa_pelas_expressoes_e_pela_ferramenta() {
        let mut ids = vec![];
        let mut b = MeshOpKind::Booleana(MeshBoolean { tool: Id(7), op: BoolOp::Cut, keep_tool: false });
        b.walk(&mut |i| ids.push(*i), &mut |_| {});
        assert_eq!(ids, vec![Id(7)]);
        let mut n = 0;
        MeshOpKind::Decimar(MeshDecimate::default()).walk(&mut |_| {}, &mut |_| n += 1);
        assert_eq!(n, 2);
    }
}
