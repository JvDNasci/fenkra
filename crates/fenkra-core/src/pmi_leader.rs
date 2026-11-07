// SPDX-License-Identifier: LGPL-2.1-or-later
//! Chamada com várias pontas e vértices intermediários (B23).
//!
//! Cada ponta é uma âncora da anotação (`Pmi::anchors`). O caminho do rótulo até a ponta `k` passa
//! pelos vértices de `PmiStyle::leader_via[k]`, em coordenadas do modelo: ao girar a vista a chamada
//! gira junto e continua ligada ao mesmo lugar. Este módulo só mexe nos dados; o desenho e as alças
//! ficam em `pmi_ui.rs`.

use crate::pmi::{Pmi, PmiTarget};

type P = [f64; 3];

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Distância do ponto `p` ao segmento `a`–`b`.
fn dist_seg(p: P, a: P, b: P) -> f64 {
    let ab = sub(b, a);
    let l2 = dot(ab, ab);
    let t = if l2 < 1e-18 { 0. } else { (dot(sub(p, a), ab) / l2).clamp(0., 1.) };
    let q = [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t];
    let d = sub(p, q);
    dot(d, d).sqrt()
}

/// Mantém um caminho por ponta: sobra é descartada, falta vira caminho reto.
pub fn fit(pmi: &mut Pmi) {
    let n = pmi.anchors.len();
    pmi.style.leader_via.resize(n, vec![]);
}

/// Pontos do caminho do rótulo (`label`) até a ponta `k`: vértices intermediários e a ponta.
pub fn route(pmi: &Pmi, label: P, k: usize) -> Vec<P> {
    let mut v = vec![label];
    if let Some(via) = pmi.style.leader_via.get(k) {
        v.extend(via.iter().copied());
    }
    if let Some(t) = pmi.anchors.get(k) {
        v.push(*t);
    }
    v
}

/// Posição do vértice novo que cabe no segmento do caminho mais perto de `p`: devolve o índice em
/// `leader_via[k]` onde ele entra.
pub fn insert_index(pmi: &Pmi, label: P, k: usize, p: P) -> usize {
    let r = route(pmi, label, k);
    r.windows(2)
        .enumerate()
        .min_by(|a, b| dist_seg(p, a.1[0], a.1[1]).total_cmp(&dist_seg(p, b.1[0], b.1[1])))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Acrescenta um vértice intermediário na ponta `k`, no segmento mais perto de `p`. Devolve o
/// índice dele; `None` se a ponta não existe.
pub fn add_vertex(pmi: &mut Pmi, label: P, k: usize, p: P) -> Option<usize> {
    if k >= pmi.anchors.len() {
        return None;
    }
    fit(pmi);
    let i = insert_index(pmi, label, k, p);
    pmi.style.leader_via[k].insert(i, p);
    Some(i)
}

/// Acrescenta um vértice no meio do último trecho da ponta `k` (ponto de partida para arrastar).
pub fn add_vertex_mid(pmi: &mut Pmi, label: P, k: usize) -> Option<usize> {
    let r = route(pmi, label, k);
    if r.len() < 2 {
        return None;
    }
    let (a, b) = (r[r.len() - 2], r[r.len() - 1]);
    let m = [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2., (a[2] + b[2]) / 2.];
    fit(pmi);
    let i = pmi.style.leader_via[k].len();
    pmi.style.leader_via[k].push(m);
    Some(i)
}

/// Tira um vértice intermediário. `false` se não existe.
pub fn remove_vertex(pmi: &mut Pmi, k: usize, i: usize) -> bool {
    match pmi.style.leader_via.get_mut(k) {
        Some(v) if i < v.len() => {
            v.remove(i);
            true
        }
        _ => false,
    }
}

/// Move um vértice intermediário.
pub fn move_vertex(pmi: &mut Pmi, k: usize, i: usize, p: P) -> bool {
    match pmi.style.leader_via.get_mut(k).and_then(|v| v.get_mut(i)) {
        Some(q) => {
            *q = p;
            true
        }
        None => false,
    }
}

/// Acrescenta uma ponta (nova âncora e o alvo dela) sem vértices.
pub fn add_tip(pmi: &mut Pmi, p: P, target: Option<PmiTarget>) {
    pmi.targets.resize(pmi.anchors.len(), None);
    pmi.anchors.push(p);
    pmi.targets.push(target);
    fit(pmi);
}

/// Tira a ponta `k`; a última ponta não sai (a anotação sem ponta não marca nada). `false` se não tirou.
pub fn remove_tip(pmi: &mut Pmi, k: usize) -> bool {
    if pmi.anchors.len() <= 1 || k >= pmi.anchors.len() {
        return false;
    }
    fit(pmi);
    pmi.anchors.remove(k);
    if k < pmi.targets.len() {
        pmi.targets.remove(k);
    }
    pmi.style.leader_via.remove(k);
    if k < pmi.style.leader_side.len() {
        pmi.style.leader_side.remove(k);
    }
    if k < pmi.style.leader_arrow.len() {
        pmi.style.leader_arrow.remove(k);
    }
    true
}

/// Lado da caixa de onde sai a chamada da ponta `k`: 0 automático, 1 esquerda, 2 direita, 3 topo,
/// 4 base. Arquivo antigo (lista vazia) = automático, o comportamento anterior (B368).
pub fn side(pmi: &Pmi, k: usize) -> u8 {
    pmi.style.leader_side.get(k).copied().unwrap_or(0).min(4)
}

/// Fixa o lado da caixa de onde sai a chamada da ponta `k`; cada ponta tem o seu.
pub fn set_side(pmi: &mut Pmi, k: usize, lado: u8) -> bool {
    if k >= pmi.anchors.len() {
        return false;
    }
    let lado = lado.min(4);
    if pmi.style.leader_side.len() <= k {
        pmi.style.leader_side.resize(k + 1, 0);
    }
    let mudou = pmi.style.leader_side[k] != lado;
    pmi.style.leader_side[k] = lado;
    mudou
}

/// Ponta (seta) da chamada `k`; `Padrao` herda a do estilo.
pub fn arrow(pmi: &Pmi, k: usize) -> crate::pmi::PmiArrow {
    pmi.style.leader_arrow.get(k).copied().unwrap_or_default()
}

pub fn set_arrow(pmi: &mut Pmi, k: usize, a: crate::pmi::PmiArrow) -> bool {
    if k >= pmi.anchors.len() {
        return false;
    }
    if pmi.style.leader_arrow.len() <= k {
        pmi.style.leader_arrow.resize(k + 1, crate::pmi::PmiArrow::Padrao);
    }
    let mudou = pmi.style.leader_arrow[k] != a;
    pmi.style.leader_arrow[k] = a;
    mudou
}

/// Lado mais próximo de `p` entre os quatro meios da caixa `[x0, y0, x1, y1]` (para a alça de
/// arrastar o início da chamada).
pub fn side_nearest(box_: [f64; 4], p: [f64; 2]) -> u8 {
    let (cx, cy) = ((box_[0] + box_[2]) / 2., (box_[1] + box_[3]) / 2.);
    let pts = [(1u8, box_[0], cy), (2, box_[2], cy), (3, cx, box_[1]), (4, cx, box_[3])];
    pts.iter().min_by(|a, b| ((a.1 - p[0]).hypot(a.2 - p[1])).total_cmp(&((b.1 - p[0]).hypot(b.2 - p[1])))).map(|x| x.0).unwrap_or(0)
}

/// O caminho se cruza? Dois segmentos não vizinhos que passam a menos de `tol` um do outro, no
/// espaço do modelo. Serve para avisar de uma chamada ilegível (a conferência na tela, depois de
/// projetar, fica em `pmi_ui.rs`).
pub fn self_touching(path: &[P], tol: f64) -> bool {
    let n = path.len();
    if n < 4 {
        return false;
    }
    for i in 0..n - 1 {
        for j in i + 2..n - 1 {
            // distância entre segmentos aproximada pelos extremos de um ao outro
            let d = [dist_seg(path[i], path[j], path[j + 1]), dist_seg(path[i + 1], path[j], path[j + 1]), dist_seg(path[j], path[i], path[i + 1]), dist_seg(path[j + 1], path[i], path[i + 1])].into_iter().fold(f64::MAX, f64::min);
            if d < tol {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pmi::PmiKind;
    use crate::Id;

    fn nota(anchors: Vec<P>) -> Pmi {
        Pmi { id: Id(1), name: "n".into(), kind: PmiKind::Note, anchors, offset: [0.; 3], text: "x".into(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] }
    }

    #[test]
    fn nota_com_tres_pontas_e_dois_vertices() {
        let mut n = nota(vec![[0., 0., 0.]]);
        let label = [0., 0., 30.];
        add_tip(&mut n, [20., 0., 0.], None);
        add_tip(&mut n, [-20., 0., 0.], None);
        assert_eq!(n.anchors.len(), 3);
        assert_eq!(n.style.leader_via.len(), 3);
        assert_eq!(n.targets.len(), 3);
        // dois vértices intermediários na primeira ponta, em ordem de caminho
        let a = add_vertex(&mut n, label, 0, [0., 0., 20.]).unwrap();
        let b = add_vertex(&mut n, label, 0, [0., 0., 8.]).unwrap();
        assert_eq!((a, b), (0, 1), "o segundo entra depois do primeiro, mais perto da ponta");
        assert_eq!(route(&n, label, 0), vec![label, [0., 0., 20.], [0., 0., 8.], [0., 0., 0.]]);
        // as outras pontas seguem retas até o rótulo
        assert_eq!(route(&n, label, 1), vec![label, [20., 0., 0.]]);
        assert_eq!(route(&n, label, 2), vec![label, [-20., 0., 0.]]);
    }

    #[test]
    fn vertice_entra_no_segmento_mais_proximo() {
        let mut n = nota(vec![[0., 0., 0.]]);
        let label = [0., 0., 30.];
        add_vertex(&mut n, label, 0, [5., 0., 20.]).unwrap();
        add_vertex(&mut n, label, 0, [5., 0., 5.]).unwrap();
        // ponto novo perto do trecho rótulo→1º vértice entra antes dos dois
        let i = add_vertex(&mut n, label, 0, [2., 0., 26.]).unwrap();
        assert_eq!(i, 0);
        assert_eq!(n.style.leader_via[0].len(), 3);
        // e perto do último trecho (vértice→ponta) entra no fim
        let i = add_vertex(&mut n, label, 0, [2., 0., 1.]).unwrap();
        assert_eq!(i, 3);
    }

    #[test]
    fn mover_e_remover_vertices() {
        let mut n = nota(vec![[0., 0., 0.]]);
        let label = [0., 0., 30.];
        add_vertex_mid(&mut n, label, 0).unwrap();
        assert_eq!(n.style.leader_via[0], vec![[0., 0., 15.]]);
        assert!(move_vertex(&mut n, 0, 0, [9., 0., 15.]));
        assert_eq!(route(&n, label, 0)[1], [9., 0., 15.]);
        assert!(!move_vertex(&mut n, 0, 5, [0.; 3]));
        assert!(!move_vertex(&mut n, 3, 0, [0.; 3]));
        assert!(remove_vertex(&mut n, 0, 0));
        assert!(!remove_vertex(&mut n, 0, 0));
        assert_eq!(route(&n, label, 0), vec![label, [0., 0., 0.]]);
        // ponta inexistente
        assert_eq!(add_vertex(&mut n, label, 4, [0.; 3]), None);
    }

    #[test]
    fn tirar_ponta_leva_os_vertices_junto() {
        let mut n = nota(vec![[0., 0., 0.]]);
        add_tip(&mut n, [10., 0., 0.], None);
        add_tip(&mut n, [20., 0., 0.], None);
        let label = [0., 0., 30.];
        add_vertex(&mut n, label, 1, [10., 0., 15.]).unwrap();
        add_vertex(&mut n, label, 2, [20., 0., 15.]).unwrap();
        assert!(remove_tip(&mut n, 1));
        assert_eq!(n.anchors, vec![[0., 0., 0.], [20., 0., 0.]]);
        assert_eq!(n.style.leader_via, vec![vec![], vec![[20., 0., 15.]]], "o vértice da ponta que ficou continua com ela");
        assert!(remove_tip(&mut n, 1));
        // a última ponta não sai
        assert!(!remove_tip(&mut n, 0));
        assert_eq!(n.anchors.len(), 1);
    }

    #[test]
    fn caminho_que_se_cruza_e_detectado() {
        // zigue-zague limpo
        assert!(!self_touching(&[[0., 0., 0.], [10., 0., 5.], [20., 0., 0.], [30., 0., 5.]], 0.5));
        // o terceiro trecho volta e toca o primeiro
        assert!(self_touching(&[[0., 0., 0.], [10., 0., 0.], [10., 0., 5.], [5., 0., 0.], [5., 0., -5.]], 0.5));
        assert!(!self_touching(&[[0.; 3], [1., 0., 0.]], 0.5));
    }

    #[test]
    fn caminhos_sobrevivem_a_salvar_e_abrir() {
        let mut n = nota(vec![[0., 0., 0.]]);
        add_tip(&mut n, [5., 0., 0.], None);
        add_vertex(&mut n, [0., 0., 20.], 0, [0., 0., 10.]).unwrap();
        let j = serde_json::to_string(&n).unwrap();
        let m: Pmi = serde_json::from_str(&j).unwrap();
        assert_eq!(m, n);
        // arquivo antigo, sem o campo: abre com chamada reta
        let mut v: serde_json::Value = serde_json::from_str(&j).unwrap();
        v["style"].as_object_mut().unwrap().remove("leader_via");
        let velho: Pmi = serde_json::from_value(v).unwrap();
        assert!(velho.style.leader_via.is_empty());
        assert_eq!(route(&velho, [0., 0., 20.], 0), vec![[0., 0., 20.], [0., 0., 0.]]);
    }

    #[test]
    fn cada_seta_tem_a_sua_ancoragem_e_arquivo_antigo_continua_automatico() {
        let mut n = nota(vec![[0., 0., 0.]]);
        add_tip(&mut n, [20., 0., 0.], None);
        add_tip(&mut n, [-20., 0., 0.], None);
        assert_eq!((side(&n, 0), side(&n, 1), side(&n, 2)), (0, 0, 0), "sem escolha = automático");
        assert!(set_side(&mut n, 1, 1));
        assert!(set_side(&mut n, 2, 2));
        assert_eq!((side(&n, 0), side(&n, 1), side(&n, 2)), (0, 1, 2), "uma esquerda e outra direita na mesma nota");
        assert!(!set_side(&mut n, 1, 1), "repetir não muda");
        assert!(!set_side(&mut n, 9, 1), "ponta que não existe");
        // tirar a ponta do meio leva a ancoragem junto
        assert!(remove_tip(&mut n, 1));
        assert_eq!((side(&n, 0), side(&n, 1)), (0, 2));
        // ida e volta no arquivo
        let v = serde_json::to_value(&n).unwrap();
        let de: Pmi = serde_json::from_value(v).unwrap();
        assert_eq!(side(&de, 1), 2);
        // arquivo sem o campo abre como hoje
        let mut v = serde_json::to_value(&n).unwrap();
        v["style"].as_object_mut().unwrap().remove("leader_side");
        v["style"].as_object_mut().unwrap().remove("leader_arrow");
        let velho: Pmi = serde_json::from_value(v).unwrap();
        assert_eq!(side(&velho, 1), 0);
    }

    #[test]
    fn alca_do_inicio_escolhe_o_lado_mais_proximo() {
        let b = [0., 0., 100., 20.];
        assert_eq!(side_nearest(b, [-30., 10.]), 1);
        assert_eq!(side_nearest(b, [130., 10.]), 2);
        assert_eq!(side_nearest(b, [50., -40.]), 3);
        assert_eq!(side_nearest(b, [50., 60.]), 4);
    }
}
