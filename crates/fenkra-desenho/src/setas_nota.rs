// SPDX-License-Identifier: LGPL-2.1-or-later
//! B370: setas das notas e legendas da folha com o mesmo modelo do PMI. A nota tem várias pontas
//! (a primeira é `Note::leader`, as outras `Note::leaders_extra`); o lado da caixa de onde sai cada
//! chamada e a seta de cada ponta ficam em `PmiStyle::leader_side` e `PmiStyle::leader_arrow`,
//! os mesmos campos e a mesma convenção de `fenkra_core::pmi_leader` (0 automático, 1 esquerda,
//! 2 direita, 3 topo, 4 base; seta `Padrao` herda a do estilo). A moldura acompanha porque a
//! chamada sai da caixa do texto já com moldura (ver `layout::note`).

use crate::layout::to_sheet;
use crate::source::Models;
use crate::{Drawing, Note, V2, V3};
use fenkra_core::pmi::PmiArrow;

/// Todas as pontas da nota, na ordem (a primeira é a de `leader`).
pub fn pontas(n: &Note) -> Vec<V2> {
    n.leader.into_iter().chain(n.leaders_extra.iter().copied()).collect()
}

/// Acrescenta uma ponta. Devolve o índice dela.
pub fn adicionar(n: &mut Note, p: V2) -> usize {
    if n.leader.is_none() {
        n.leader = Some(p);
        0
    } else {
        n.leaders_extra.push(p);
        n.leaders_extra.len()
    }
}

/// Move a ponta `k`. `false` se ela não existe.
pub fn mover(n: &mut Note, k: usize, p: V2) -> bool {
    match k {
        0 if n.leader.is_some() => n.leader = Some(p),
        k if k >= 1 && k <= n.leaders_extra.len() => n.leaders_extra[k - 1] = p,
        _ => return false,
    }
    true
}

/// Tira a ponta `k` (com o lado e a seta dela). `false` se não existe.
pub fn remover(n: &mut Note, k: usize) -> bool {
    if k >= pontas(n).len() {
        return false;
    }
    if k == 0 {
        n.leader = if n.leaders_extra.is_empty() { None } else { Some(n.leaders_extra.remove(0)) };
        // a primeira ponta passa a ser a próxima: leva a âncora do modelo junto
        let prox = if n.anchors_extra.is_empty() { None } else { n.anchors_extra.remove(0) };
        n.anchor3 = prox;
        n.snapped = n.snapped && prox.is_some();
    } else {
        n.leaders_extra.remove(k - 1);
        if k - 1 < n.anchors_extra.len() {
            n.anchors_extra.remove(k - 1);
        }
    }
    if k < n.style.leader_side.len() {
        n.style.leader_side.remove(k);
    }
    if k < n.style.leader_arrow.len() {
        n.style.leader_arrow.remove(k);
    }
    true
}

/// Ponto do modelo onde a ponta `k` encosta, se ela está presa à vista da nota.
pub fn ancora(n: &Note, k: usize) -> Option<V3> {
    if k == 0 {
        n.anchor3
    } else {
        n.anchors_extra.get(k - 1).copied().flatten()
    }
}

/// Prende a ponta `k` ao ponto `a` do modelo (ou solta, com `None`). `false` se a ponta não existe.
/// A ponta extra só acompanha o modelo se a nota pertence a uma vista.
pub fn prender(n: &mut Note, k: usize, a: Option<V3>) -> bool {
    if k >= pontas(n).len() {
        return false;
    }
    if k == 0 {
        n.anchor3 = a;
        n.snapped = a.is_some();
    } else {
        if n.anchors_extra.len() < k {
            n.anchors_extra.resize(k, None);
        }
        n.anchors_extra[k - 1] = a;
        while n.anchors_extra.last().is_some_and(|x| x.is_none()) {
            n.anchors_extra.pop();
        }
    }
    true
}

/// Leva as pontas presas ao modelo para onde a vista as projeta hoje, em `n` (cópia de trabalho
/// para desenhar). As soltas ficam como estão.
pub fn acompanhar_modelo(d: &Drawing, n: &mut Note, m: &mut Models) {
    let Some(v) = n.view.and_then(|id| d.view(id)) else { return };
    if n.anchor3.is_none() && n.anchors_extra.iter().all(|a| a.is_none()) {
        return;
    }
    let p = m.proj(d, v.id);
    let Some(ax) = p.axes else { return };
    if let Some(a3) = n.anchor3 {
        n.leader = Some(to_sheet(v, &p, ax.project(a3)));
    }
    for (k, a) in n.anchors_extra.clone().into_iter().enumerate() {
        if let (Some(a3), Some(t)) = (a, n.leaders_extra.get_mut(k)) {
            *t = to_sheet(v, &p, ax.project(a3));
        }
    }
}

/// Lado da caixa de onde sai a chamada da ponta `k`.
pub fn lado(n: &Note, k: usize) -> u8 {
    n.style.leader_side.get(k).copied().unwrap_or(0).min(4)
}

/// Fixa o lado da ponta `k`. `true` se mudou.
pub fn definir_lado(n: &mut Note, k: usize, lado: u8) -> bool {
    if k >= pontas(n).len() {
        return false;
    }
    let lado = lado.min(4);
    if n.style.leader_side.len() <= k {
        n.style.leader_side.resize(k + 1, 0);
    }
    let mudou = n.style.leader_side[k] != lado;
    n.style.leader_side[k] = lado;
    mudou
}

/// Ponto da borda da caixa do texto de onde sai a chamada, para o lado 1 esquerda, 2 direita,
/// 3 topo, 4 base (a mesma regra do desenho da chamada). `lado` 0 = o automático para `ponta`.
pub fn ponto_de_saida(lo: V2, hi: V2, ponta: V2, lado: u8) -> V2 {
    let c = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.];
    let lado = if lado != 0 { lado } else { lado_pelo_ponto(lo, hi, ponta) };
    match lado {
        1 => [lo[0], c[1]],
        2 => [hi[0], c[1]],
        3 => [c[0], hi[1]],
        _ => [c[0], lo[1]],
    }
}

/// Lado da caixa mais próximo de `p` (1 esquerda, 2 direita, 3 topo, 4 base): o que a alça
/// do início da seta escolhe quando é arrastada.
pub fn lado_pelo_ponto(lo: V2, hi: V2, p: V2) -> u8 {
    let c = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.];
    let (w, h) = ((hi[0] - lo[0]).max(1e-6), (hi[1] - lo[1]).max(1e-6));
    let d = [p[0] - c[0], p[1] - c[1]];
    if d[0].abs() * h >= d[1].abs() * w {
        if d[0] < 0. { 1 } else { 2 }
    } else if d[1] > 0. {
        3
    } else {
        4
    }
}

/// Seta da ponta `k` (`Padrao` = a do estilo).
pub fn seta(n: &Note, k: usize) -> PmiArrow {
    n.style.leader_arrow.get(k).copied().unwrap_or_default()
}

/// Fixa a seta da ponta `k`. `true` se mudou.
pub fn definir_seta(n: &mut Note, k: usize, a: PmiArrow) -> bool {
    if k >= pontas(n).len() {
        return false;
    }
    if n.style.leader_arrow.len() <= k {
        n.style.leader_arrow.resize(k + 1, PmiArrow::Padrao);
    }
    let mudou = n.style.leader_arrow[k] != a;
    n.style.leader_arrow[k] = a;
    mudou
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::note_items;
    use crate::source::Models;
    use crate::layout::Prim;
    use crate::{Drawing, Format, NoteKind};

    fn nota() -> Note {
        Note { id: crate::Id(900), kind: NoteKind::Text, pos: [100., 100.], leader: Some([60., 100.]), text: "Quebrar cantos".into(), ..Default::default() }
    }

    #[test]
    fn varias_pontas_cada_uma_com_lado_e_seta() {
        let d = Drawing::new("t", Format::A3);
        let mut m = Models::new(None);
        let mut n = nota();
        let uma = note_items(&d, &n, &mut m).len();
        assert_eq!(adicionar(&mut n, [140., 130.]), 1);
        assert_eq!(adicionar(&mut n, [100., 60.]), 2);
        assert_eq!(pontas(&n).len(), 3);
        let tres = note_items(&d, &n, &mut m);
        assert!(tres.len() > uma, "cada ponta acrescenta chamada e seta");
        // lado fixo: a ponta 1 sai pela base, mesmo estando à direita e acima
        assert!(definir_lado(&mut n, 1, 4));
        assert_eq!(lado(&n, 1), 4);
        assert!(!definir_lado(&mut n, 1, 4), "sem mudança não conta");
        // seta própria: ponta sem seta desenha menos que a cheia
        let com = note_items(&d, &n, &mut m);
        assert!(definir_seta(&mut n, 2, PmiArrow::Nenhuma));
        let sem = note_items(&d, &n, &mut m);
        let preenchidos = |v: &[crate::layout::Item]| v.iter().filter(|i| matches!(i.prim, Prim::Fill { .. })).count();
        assert_eq!(preenchidos(&sem) + 1, preenchidos(&com), "a ponta 2 perdeu a seta cheia");
    }

    #[test]
    fn tirar_a_primeira_promove_a_proxima_e_leva_lado_e_seta_juntos() {
        let mut n = nota();
        adicionar(&mut n, [140., 130.]);
        definir_lado(&mut n, 1, 3);
        definir_seta(&mut n, 1, PmiArrow::Ponto);
        assert!(remover(&mut n, 0));
        assert_eq!(n.leader, Some([140., 130.]));
        assert_eq!((lado(&n, 0), seta(&n, 0)), (3, PmiArrow::Ponto));
        assert!(remover(&mut n, 0));
        assert!(n.leader.is_none() && pontas(&n).is_empty());
        assert!(!remover(&mut n, 0));
    }

    #[test]
    fn ponta_extra_presa_ao_modelo_acompanha_a_vista_e_a_solta_nao() {
        let mut d = Drawing::new("t", Format::A3);
        let s = d.sheets[0].id;
        let v = d.add_view(s, crate::Source { path: "x.fkp".into(), display_state: String::new(), hidden: vec![] }, crate::ViewKind::Standard(crate::Dir::Frente), [100., 100.], 1.);
        let mut m = Models::new(None);
        // sem arquivo não há eixos: monta a projeção de uma vista conhecida
        let p = m.proj(&d, v);
        let mut n = nota();
        n.view = Some(v);
        adicionar(&mut n, [140., 130.]);
        adicionar(&mut n, [100., 60.]);
        assert!(prender(&mut n, 2, Some([1., 2., 3.])));
        assert_eq!(ancora(&n, 2), Some([1., 2., 3.]));
        assert_eq!(ancora(&n, 1), None);
        assert!(!prender(&mut n, 9, None));
        // tirar a ponta 1 leva a âncora da 2 para a posição 1
        assert!(remover(&mut n, 1));
        assert_eq!(ancora(&n, 1), Some([1., 2., 3.]));
        // promover a próxima como primeira leva a âncora junto
        assert!(remover(&mut n, 0));
        assert_eq!(n.anchor3, Some([1., 2., 3.]));
        assert!(n.anchors_extra.is_empty());
        let _ = p;
    }

    #[test]
    fn alca_de_saida_segue_o_lado_e_o_arraste_escolhe_o_lado() {
        let (lo, hi) = ([90., 95.], [110., 105.]);
        assert_eq!(ponto_de_saida(lo, hi, [0., 0.], 2), [110., 100.]);
        assert_eq!(ponto_de_saida(lo, hi, [0., 0.], 3), [100., 105.]);
        // automático: ponta à esquerda sai pela esquerda
        assert_eq!(ponto_de_saida(lo, hi, [40., 100.], 0), [90., 100.]);
        assert_eq!(lado_pelo_ponto(lo, hi, [100., 150.]), 3);
        assert_eq!(lado_pelo_ponto(lo, hi, [100., 50.]), 4);
        assert_eq!(lado_pelo_ponto(lo, hi, [200., 101.]), 2);
        // a caixa que a interface usa é a mesma do desenho da chamada
        let d = Drawing::new("t", Format::A3);
        let mut m = Models::new(None);
        let n = nota();
        let (l, h) = crate::layout::caixa_da_nota(&d, &n, &mut m).expect("nota com chamada");
        assert!(l[0] < n.pos[0] + 1. && h[0] > n.pos[0] - 1. && l[1] < h[1]);
    }

    #[test]
    fn nota_antiga_sem_pontas_extras_le_igual() {
        let n = nota();
        let j = serde_json::to_string(&n).unwrap();
        assert!(!j.contains("leaders_extra") && !j.contains("anchors_extra"), "vazio não polui o arquivo");
        let volta: Note = serde_json::from_str(&j).unwrap();
        assert_eq!(volta, n);
    }
}
