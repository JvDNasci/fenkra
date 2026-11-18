// SPDX-License-Identifier: LGPL-2.1-or-later
//! B412: conjunto de vistas padrão cuja distância entre vistas considera o tamanho da peça.
//! `Drawing::add_standard_set` recebe a distância entre centros pronta; com peça grande e escala
//! alta a distância fixa fazia as vistas se cobrirem por inteiro, sem aviso.

use crate::source::Models;
use crate::{Dir, Drawing, Id, Source, ViewKind, V2};

/// Distância mínima entre centros para que as vistas do conjunto não se toquem: metade da maior
/// dimensão de cada par de vizinhas, na escala, mais a `folga` pedida (mm da folha). A isométrica
/// fica à direita da lateral, a `gap` dela, e também entra na conta.
pub fn distancia_minima(frente: V2, lateral: V2, topo: V2, iso: V2, escala: f64, folga: f64) -> f64 {
    // lado a lado (frente e lateral, lateral e isométrica): largura; empilhadas (frente e
    // superior): altura
    let horizontal = ((frente[0] + lateral[0]) / 2.).max((lateral[0] + iso[0]) / 2.) * escala;
    let vertical = (frente[1] + topo[1]) / 2. * escala;
    horizontal.max(vertical) + folga
}

/// Distância entre centros que o conjunto padrão precisa nesta escala. Mede num desenho de
/// rascunho (`medidas`), que ganha vistas temporárias com ids altos: passe uma cópia ou um rascunho.
pub fn distancia(medidas: &mut Drawing, m: &mut Models, folha: Id, fonte: &Source, escala: f64, folga: f64) -> f64 {
    // ids altos: o cache de projeções é por id e não pode confundir com as vistas reais
    medidas.reserve_ids(1 << 42);
    let mut ext = |dir: Dir| {
        let id = medidas.add_view(folha, fonte.clone(), ViewKind::Standard(dir), [0., 0.], 1.);
        let p = m.proj(medidas, id);
        [(p.hi[0] - p.lo[0]).max(0.), (p.hi[1] - p.lo[1]).max(0.)]
    };
    let (f, l, t, i) = (ext(Dir::Frente), ext(Dir::Esquerda), ext(Dir::Topo), ext(Dir::Iso));
    distancia_minima(f, l, t, i, escala, folga)
}

/// Cria as quatro vistas padrão sem se sobreporem. `folga` é o vão mínimo pedido; se a distância
/// pedida for pequena para a peça, ela cresce. Devolve os ids e a distância usada entre centros.
pub fn criar_sem_sobrepor(d: &mut Drawing, m: &mut Models, folha: Id, fonte: Source, centro: V2, escala: f64, folga: f64) -> (Vec<Id>, f64) {
    let gap = distancia(&mut d.clone(), m, folha, &fonte, escala, folga);
    (d.add_standard_set(folha, fonte, centro, escala, gap), gap)
}

/// Medidas de uma vista em escala 1: caixa do contorno relativa ao ponto de posição da vista
/// (o `center` da projeção). Na escala `s` a caixa é só multiplicada por `s`.
pub type MedidaVista = (V2, V2);

/// B473: medidas em escala 1 das quatro vistas do conjunto padrão, na ordem frente, superior,
/// lateral e isométrica (as direções seguem o método de projeção do desenho).
pub fn medir_conjunto(medidas: &mut Drawing, m: &mut Models, folha: Id, fonte: &Source) -> [MedidaVista; 4] {
    medidas.reserve_ids(1 << 43);
    let (sup, lat) = medidas.projection.vistas_da_frente();
    let mut ext = |dir: Dir| {
        let id = medidas.add_view(folha, fonte.clone(), ViewKind::Standard(dir), [0., 0.], 1.);
        let p = m.proj(medidas, id);
        ([p.lo[0] - p.center[0], p.lo[1] - p.center[1]], [p.hi[0] - p.center[0], p.hi[1] - p.center[1]])
    };
    [ext(Dir::Frente), ext(sup), ext(lat), ext(Dir::Iso)]
}

/// Caixa que as quatro vistas ocupam na escala `s`, com o centro da frente em (0, 0): a mesma
/// disposição de `Drawing::add_standard_set` e a mesma distância de `distancia` (mínimo de 40 mm).
pub fn caixa_do_conjunto(med: &[MedidaVista; 4], projecao: crate::novo::Projecao, s: f64, folga: f64) -> (V2, V2) {
    let tam = |v: MedidaVista| [(v.1[0] - v.0[0]).max(0.), (v.1[1] - v.0[1]).max(0.)];
    let gap = distancia_minima(tam(med[0]), tam(med[2]), tam(med[1]), tam(med[3]), s, folga).max(40.);
    let (ps, pl) = projecao.posicoes();
    let centros = [[0., 0.], [ps[0] * gap, ps[1] * gap], [pl[0] * gap, pl[1] * gap], [2. * gap, 0.]];
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for (c, v) in centros.iter().zip(med) {
        for k in 0..2 {
            lo[k] = lo[k].min(c[k] + v.0[k] * s);
            hi[k] = hi[k].max(c[k] + v.1[k] * s);
        }
    }
    (lo, hi)
}

/// Maior escala da lista normalizada em que as quatro vistas e os vãos entre elas cabem em `area`
/// (dentro da moldura e fora do carimbo). Devolve a escala, a caixa nela (relativa ao centro da
/// frente) e se coube; se nem a menor serve, devolve a menor e `false`.
pub fn escala_do_conjunto(med: &[MedidaVista; 4], projecao: crate::novo::Projecao, area: (V2, V2), folga: f64) -> (f64, (V2, V2), bool) {
    let (aw, ah) = (area.1[0] - area.0[0], area.1[1] - area.0[1]);
    for s in crate::novo::ESCALAS_AUTO {
        let c = caixa_do_conjunto(med, projecao, s, folga);
        if c.1[0] - c.0[0] <= aw + 1e-9 && c.1[1] - c.0[1] <= ah + 1e-9 {
            return (s, c, true);
        }
    }
    let s = crate::novo::ESCALAS_AUTO[crate::novo::ESCALAS_AUTO.len() - 1];
    (s, caixa_do_conjunto(med, projecao, s, folga), false)
}

/// Leva o centro da frente `at` para onde a `caixa` (relativa a ele) fica toda dentro de `area`.
/// Se a caixa for maior que a área, alinha pelo canto inferior esquerdo.
pub fn centro_dentro(at: V2, caixa: (V2, V2), area: (V2, V2)) -> V2 {
    let mut p = at;
    for k in 0..2 {
        if p[k] + caixa.1[k] > area.1[k] {
            p[k] = area.1[k] - caixa.1[k];
        }
        if p[k] + caixa.0[k] < area.0[k] {
            p[k] = area.0[k] - caixa.0[k];
        }
    }
    p
}

/// Área útil das vistas na folha: dentro da moldura e acima do carimbo (mm, y de baixo).
pub fn area_util(d: &Drawing, folha: Id) -> (V2, V2) {
    let size = d.sheet(folha).map(|s| s.size()).unwrap_or([420., 297.]);
    let (m0, m1) = d.sheet(folha).map(|sh| crate::ocupacao::moldura_da_folha(d, sh)).unwrap_or(([20., 10.], [size[0] - 10., size[1] - 10.]));
    let legenda = d.sheet(folha).map(|sh| crate::ocupacao::legenda_da_folha(d, sh)).unwrap_or_else(|| crate::ocupacao::legenda_caixa(d.title.style, size));
    ([m0[0], legenda.1[1]], m1)
}
