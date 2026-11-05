// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estilos nomeados e regras com fórmula: a base compartilhada entre a chapa metálica (B283) e a
//! dobra de tubo cadastrável (B330).
//!
//! Um [`EstiloRegras`] é um nome com uma tabela `chave -> fórmula`. Cada consumidor decide quais
//! chaves existem (a chapa usa `espessura`, `raio`, `k`...; o tubo usa raio mínimo, retorno
//! elástico, folga). O valor de cada campo vem de três camadas, da mais forte para a mais fraca:
//!
//! 1. sobrescrita local (da peça ou do comando), marcada com cadeado na interface;
//! 2. o estilo escolhido;
//! 3. o padrão do consumidor.
//!
//! As fórmulas são texto do avaliador de expressões e podem citar parâmetros e a variável de
//! contexto do consumidor (na chapa, `espessura`), por exemplo `1.15*espessura`. Guardar a fórmula
//! (e não o número) é o que faz o raio acompanhar a espessura quando o estilo muda.

use crate::expr;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Estilo nomeado: conjunto de campos com fórmula.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct EstiloRegras {
    pub nome: String,
    /// chave -> fórmula (texto)
    pub campos: BTreeMap<String, String>,
}

/// De onde veio o valor de um campo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Origem {
    /// nenhum estilo (ou o estilo não define o campo): valor próprio do consumidor
    Padrao,
    /// herdado do estilo escolhido
    Estilo,
    /// o estilo existe, mas este campo foi sobrescrito (cadeado)
    Sobrescrita,
}

impl EstiloRegras {
    pub fn novo(nome: &str) -> Self {
        EstiloRegras { nome: nome.to_string(), campos: BTreeMap::new() }
    }

    pub fn com(mut self, chave: &str, formula: &str) -> Self {
        self.campos.insert(chave.to_string(), formula.to_string());
        self
    }

    pub fn campo(&self, chave: &str) -> Option<&str> {
        self.campos.get(chave).map(String::as_str)
    }
}

/// Acha o estilo pelo nome (vazio = nenhum).
pub fn acha<'a>(estilos: &'a [EstiloRegras], nome: &str) -> Option<&'a EstiloRegras> {
    if nome.is_empty() {
        return None;
    }
    estilos.iter().find(|e| e.nome == nome)
}

/// Resolve um campo: sobrescrita > estilo > padrão. Devolve a fórmula e de onde ela veio.
pub fn resolve(estilo: Option<&EstiloRegras>, sobrescritas: &[String], chave: &str, proprio: &str) -> (String, Origem) {
    match estilo {
        None => (proprio.to_string(), Origem::Padrao),
        Some(e) => {
            if sobrescritas.iter().any(|c| c == chave) {
                return (proprio.to_string(), Origem::Sobrescrita);
            }
            match e.campo(chave) {
                Some(f) => (f.to_string(), Origem::Estilo),
                // o estilo não define este campo: vale o valor próprio, sem cadeado
                None => (proprio.to_string(), Origem::Padrao),
            }
        }
    }
}

/// Avalia uma fórmula com a variável de contexto do consumidor (ex.: `espessura`). A variável do
/// contexto vale mesmo que exista um parâmetro de mesmo nome.
pub fn avalia(formula: &str, vars: &HashMap<String, f64>, contexto: (&str, f64)) -> Result<f64, String> {
    let mut v = vars.clone();
    v.insert(contexto.0.to_string(), contexto.1);
    expr::eval_str(formula, &v).map_err(|e| format!("'{formula}': {e}"))
}

/// Marca (ou desmarca) um campo como sobrescrito, sem duplicar.
pub fn marca(sobrescritas: &mut Vec<String>, chave: &str, sobrescrito: bool) {
    let tem = sobrescritas.iter().any(|c| c == chave);
    if sobrescrito && !tem {
        sobrescritas.push(chave.to_string());
    } else if !sobrescrito && tem {
        sobrescritas.retain(|c| c != chave);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camadas_sobrescrita_estilo_padrao() {
        let e = EstiloRegras::novo("Inox 1,5").com("raio", "1.15*espessura");
        let (f, o) = resolve(Some(&e), &[], "raio", "2");
        assert_eq!((f.as_str(), o), ("1.15*espessura", Origem::Estilo));
        let (f, o) = resolve(Some(&e), &["raio".into()], "raio", "3");
        assert_eq!((f.as_str(), o), ("3", Origem::Sobrescrita));
        // o estilo não define "k": vale o próprio, sem cadeado
        let (f, o) = resolve(Some(&e), &[], "k", "0.44");
        assert_eq!((f.as_str(), o), ("0.44", Origem::Padrao));
        let (f, o) = resolve(None, &[], "raio", "2");
        assert_eq!((f.as_str(), o), ("2", Origem::Padrao));
    }

    #[test]
    fn formula_usa_a_variavel_de_contexto() {
        let mut v = HashMap::new();
        v.insert("espessura".to_string(), 99.);
        let r = avalia("1.15*espessura", &v, ("espessura", 2.)).unwrap();
        assert!((r - 2.3).abs() < 1e-12);
    }
}
