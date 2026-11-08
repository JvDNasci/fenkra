// SPDX-License-Identifier: LGPL-2.1-or-later
//! Requisitos declarados na peça (B418): o que ela precisa cumprir, escrito junto com ela. Massa numa
//! faixa, furos de um diâmetro, folga e interferência entre corpos, tamanho máximo e "mudei o
//! parâmetro, a peça seguiu?". A conferência fica em `fenkra_solidos::verificar`; aqui só o dado.
//!
//! Serve a quem modela à mão e a quem automatiza: a mesma lista roda no aplicativo, na linha de
//! comando e no servidor MCP.

use serde::{Deserialize, Serialize};

/// O que a propagação mede depois de trocar o parâmetro.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Medida {
    #[default]
    Volume,
    CaixaX,
    CaixaY,
    CaixaZ,
    Massa,
}

impl Medida {
    pub const TODAS: [Medida; 5] = [Medida::Volume, Medida::CaixaX, Medida::CaixaY, Medida::CaixaZ, Medida::Massa];

    pub fn rotulo(self) -> &'static str {
        match self {
            Medida::Volume => "volume (mm³)",
            Medida::CaixaX => "tamanho em X (mm)",
            Medida::CaixaY => "tamanho em Y (mm)",
            Medida::CaixaZ => "tamanho em Z (mm)",
            Medida::Massa => "massa (kg)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum TipoRequisito {
    /// massa total em kg, pelo material de cada corpo
    Massa {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
    },
    /// ao menos `quantidade` furos de `diametro` ± `tol` (mm), reconhecidos na geometria
    Furos {
        diametro: f64,
        quantidade: u32,
        #[serde(default = "tol_furo")]
        tol: f64,
    },
    /// a menor distância entre quaisquer dois corpos da peça é pelo menos `min` (mm)
    Folga { min: f64 },
    /// nenhum par de corpos se sobrepõe em mais que `tol` mm³
    SemInterferencia {
        #[serde(default = "tol_volume")]
        tol: f64,
    },
    /// a peça cabe numa caixa de `max` mm (x, y, z)
    Caixa { max: [f64; 3] },
    /// trocar `parametro` para `valor` recalcula sem erro e muda `medida` (para `esperado` ± `tol`,
    /// quando informado). A peça aberta não muda: o teste roda numa cópia.
    Propagacao {
        parametro: String,
        valor: String,
        #[serde(default)]
        medida: Medida,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        esperado: Option<f64>,
        #[serde(default = "tol_medida")]
        tol: f64,
    },
}

fn tol_furo() -> f64 {
    0.05
}
fn tol_volume() -> f64 {
    1e-3
}
fn tol_medida() -> f64 {
    0.01
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Requisito {
    pub nome: String,
    #[serde(flatten)]
    pub tipo: TipoRequisito,
}

impl TipoRequisito {
    /// Um de cada tipo, com valores neutros, para o botão "acrescentar".
    pub fn modelos() -> [TipoRequisito; 6] {
        [
            TipoRequisito::Massa { min: None, max: Some(1.) },
            TipoRequisito::Furos { diametro: 6.6, quantidade: 1, tol: tol_furo() },
            TipoRequisito::Folga { min: 1. },
            TipoRequisito::SemInterferencia { tol: tol_volume() },
            TipoRequisito::Caixa { max: [100.; 3] },
            TipoRequisito::Propagacao { parametro: String::new(), valor: String::new(), medida: Medida::Volume, esperado: None, tol: tol_medida() },
        ]
    }

    pub fn rotulo(&self) -> &'static str {
        match self {
            TipoRequisito::Massa { .. } => "Massa",
            TipoRequisito::Furos { .. } => "Furos",
            TipoRequisito::Folga { .. } => "Folga entre corpos",
            TipoRequisito::SemInterferencia { .. } => "Sem interferência",
            TipoRequisito::Caixa { .. } => "Cabe na caixa",
            TipoRequisito::Propagacao { .. } => "Propagação de parâmetro",
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn requisito_vai_e_volta_em_json_com_padroes() {
        let r: Requisito = serde_json::from_str(r#"{ "nome": "M6", "tipo": "furos", "diametro": 6.6, "quantidade": 4 }"#).unwrap();
        assert_eq!(r.tipo, TipoRequisito::Furos { diametro: 6.6, quantidade: 4, tol: 0.05 });
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["tipo"], "furos");
        let p: Requisito = serde_json::from_str(r#"{ "nome": "altura", "tipo": "propagacao", "parametro": "h", "valor": "40", "medida": "caixa_z", "esperado": 40 }"#).unwrap();
        assert!(matches!(p.tipo, TipoRequisito::Propagacao { medida: Medida::CaixaZ, .. }));
    }
}
