// SPDX-License-Identifier: LGPL-2.1-or-later
//! B412: conjunto de vistas padrão cuja distância entre vistas considera o tamanho da peça.
//! `Drawing::add_standard_set` recebe a distância entre centros pronta; com peça grande e escala
//! alta a distância fixa fazia as vistas se cobrirem por inteiro, sem aviso.

use crate::source::Models;
use crate::{Dir, Drawing, Id, Source, ViewKind, V2};

/// Distância mínima entre centros para que frente, superior e lateral não se toquem: metade da
/// maior dimensão de cada par de vizinhas, na escala, mais a `folga` pedida (mm da folha).
pub fn distancia_minima(frente: V2, lateral: V2, topo: V2, escala: f64, folga: f64) -> f64 {
    // lado a lado (frente e lateral): largura; empilhadas (frente e superior): altura
    let horizontal = (frente[0] + lateral[0]) / 2. * escala;
    let vertical = (frente[1] + topo[1]) / 2. * escala;
    horizontal.max(vertical) + folga
}

/// Cria as quatro vistas padrão sem se sobreporem. `folga` é o vão mínimo pedido; se a distância
/// pedida for pequena para a peça, ela cresce. Devolve os ids e a distância usada entre centros.
pub fn criar_sem_sobrepor(d: &mut Drawing, m: &mut Models, folha: Id, fonte: Source, centro: V2, escala: f64, folga: f64) -> (Vec<Id>, f64) {
    let mut medidas = d.clone();
    // ids altos: o cache de projeções é por id e não pode confundir com as vistas reais
    medidas.reserve_ids(1 << 42);
    let mut ext = |dir: Dir| {
        let id = medidas.add_view(folha, fonte.clone(), ViewKind::Standard(dir), [0., 0.], 1.);
        let p = m.proj(&medidas, id);
        [(p.hi[0] - p.lo[0]).max(0.), (p.hi[1] - p.lo[1]).max(0.)]
    };
    let (f, l, t) = (ext(Dir::Frente), ext(Dir::Esquerda), ext(Dir::Topo));
    let gap = distancia_minima(f, l, t, escala, folga);
    (d.add_standard_set(folha, fonte, centro, escala, gap), gap)
}
