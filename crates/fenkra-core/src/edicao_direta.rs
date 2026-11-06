// SPDX-License-Identifier: LGPL-2.1-or-later
//! Edição direta de faces (B33): mexer no corpo pelas faces, sem refazer o histórico. É o que torna
//! útil um corpo vindo de STEP, que chega sem sketch nem comandos.

use crate::doc::FaceKey;
use serde::{Deserialize, Serialize};

/// O que se faz com as faces escolhidas.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum OpDireta {
    /// Leva o relevo formado pelas faces (boss, furo, rasgo) para outro lugar.
    #[default]
    Mover,
    /// Apaga as faces e deixa as vizinhas se estenderem até se encontrar (cura).
    Apagar,
    /// Empurra ou puxa as faces ao longo da normal; em cilindros e esferas muda o raio.
    Deslocar,
    /// Troca o raio de um filete que já está no corpo (0 desfaz o filete).
    RaioFilete,
}

impl OpDireta {
    pub const ALL: [OpDireta; 4] = [OpDireta::Mover, OpDireta::Apagar, OpDireta::Deslocar, OpDireta::RaioFilete];
    pub fn rotulo(self) -> &'static str {
        match self {
            OpDireta::Mover => "Mover face",
            OpDireta::Apagar => "Apagar face",
            OpDireta::Deslocar => "Deslocar face",
            OpDireta::RaioFilete => "Raio do filete",
        }
    }
}

/// Comando de edição direta (B33).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DirectEdit {
    pub op: OpDireta,
    /// uma face por clique (o ponto serve de referência; a chave v2 vem em `face_keys`)
    pub faces: Vec<[f64; 3]>,
    #[serde(default)]
    pub face_keys: Vec<Option<FaceKey>>,
    /// Deslocar: distância (mm); RaioFilete: raio novo (mm)
    pub value: String,
    /// Mover: deslocamento X, Y e Z (mm)
    pub delta: [String; 3],
}

impl DirectEdit {
    pub fn novo(op: OpDireta) -> Self {
        DirectEdit { op, faces: vec![], face_keys: vec![], value: "1".into(), delta: ["0".into(), "0".into(), "5".into()] }
    }
}
