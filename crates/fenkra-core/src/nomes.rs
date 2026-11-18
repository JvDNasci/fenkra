// SPDX-License-Identifier: LGPL-2.1-or-later
//! B510: nome padrão de um item novo no idioma da interface.
//!
//! O núcleo escreve os nomes-base em português ("Extrusão", "Plano", "Corpo principal"). Quem tem
//! a interface (o app) registra aqui um tradutor: com o inglês ativo ele devolve a versão em inglês
//! do nome-base, e o item nasce com ela ("Pad1"). O nome fica gravado no documento como texto: trocar
//! de idioma depois não renomeia nada do que já existe. Sem tradutor registrado (linha de comando,
//! testes do núcleo) tudo nasce em português, como antes.
//!
//! O tradutor é uma função simples, sem estado: quem decide o idioma é ele (o app consulta o idioma
//! da sessão, que nos testes é um por thread). Assim o núcleo não guarda idioma global nenhum.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Idioma em que os nomes nascem.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Idioma {
    #[default]
    Pt,
    En,
}

impl Idioma {
    pub fn e_padrao(&self) -> bool {
        *self == Idioma::Pt
    }
}

/// O que o app registra: o idioma ativo e a tradução de um nome-base (`None` = sem tradução).
#[derive(Clone, Copy)]
pub struct Tradutor {
    pub idioma: fn() -> Idioma,
    pub traduzir: fn(&str) -> Option<String>,
}

static TRADUTOR: OnceLock<Tradutor> = OnceLock::new();

/// Registra o tradutor (uma vez por processo; chamadas seguintes são ignoradas).
pub fn registrar(t: Tradutor) {
    let _ = TRADUTOR.set(t);
}

/// Idioma ativo para nomes novos.
pub fn idioma() -> Idioma {
    TRADUTOR.get().map_or(Idioma::Pt, |t| (t.idioma)())
}

/// Nome-base no idioma ativo. Em português (ou sem tradução) devolve o próprio `base`.
pub fn base(base: &str) -> String {
    match (idioma(), TRADUTOR.get()) {
        (Idioma::En, Some(t)) => (t.traduzir)(base).unwrap_or_else(|| base.to_string()),
        _ => base.to_string(),
    }
}

/// Junta um texto de várias palavras num identificador colado, no estilo dos nomes-base
/// ("Linear pattern" → "LinearPattern"). Nomes-base com espaço em português ficam com espaço.
pub fn colar(texto: &str) -> String {
    texto
        .split(|c: char| c.is_whitespace() || c == '-')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                Some(f) => f.to_uppercase().chain(c).collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn colar_junta_palavras() {
        assert_eq!(colar("Linear pattern"), "LinearPattern");
        assert_eq!(colar("Multi-section"), "MultiSection");
        assert_eq!(colar("Pad"), "Pad");
    }

    #[test]
    fn sem_tradutor_nasce_em_portugues() {
        // o núcleo sozinho não registra tradutor
        if TRADUTOR.get().is_none() {
            assert_eq!(base("Extrusão"), "Extrusão");
            assert_eq!(idioma(), Idioma::Pt);
        }
    }
}
