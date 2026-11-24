// SPDX-License-Identifier: LGPL-2.1-or-later
//! B580: comando em erro não leva o corpo embora.
//!
//! Alguns comandos tiram o corpo do coletor antes de montar o novo (a aba, a dobra, as booleanas) e
//! só depois descobrem que não dá. Se o erro vem no meio, o coletor ficava vazio e o corpo inteiro
//! sumia da vista, mesmo com o erro numa única aba. Aqui o recálculo guarda o corpo de antes (a
//! cópia só divide a forma, não a refaz) e o devolve quando o comando termina em erro: o comando fica
//! marcado com a mensagem, e o que vem depois dele trabalha sobre o último estado bom.

use super::Body;
use fenkra_core::Id;
use std::collections::HashMap;

/// O corpo de cada coletor que o comando pode mexer, antes dele rodar (`None`: ainda não havia).
pub(crate) type Antes = Vec<(Id, Option<Body>)>;

pub(crate) fn guarda(corpos: &HashMap<Id, Body>, tocados: &[Id]) -> Antes {
    tocados.iter().map(|c| (*c, corpos.get(c).cloned())).collect()
}

/// Devolve os corpos ao que eram antes do comando que falhou.
pub(crate) fn restaura(corpos: &mut HashMap<Id, Body>, antes: Antes) {
    for (c, b) in antes {
        match b {
            Some(b) => corpos.insert(c, b),
            None => corpos.remove(&c),
        };
    }
}
