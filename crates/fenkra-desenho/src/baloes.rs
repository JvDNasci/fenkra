// SPDX-License-Identifier: LGPL-2.1-or-later
//! Balões do desenho (B149): texto acima e abaixo, quantidade ao lado, largura fixa, nós nas chamadas,
//! alinhamento, renumeração pela lista de materiais e balões automáticos por janela.
//!
//! A forma do balão é a moldura do estilo (`PmiStyle::frame`: círculo, quadrado, triângulo, losango,
//! hexágono não; círculo ou retângulo riscado para o balão dividido) e a largura fixa é
//! `PmiStyle::frame_fixed`; o que é só de balão mora em [`DwgBalao`].

use crate::{Note, NoteKind, V2};
use fenkra_core::Id;
use serde::{Deserialize, Serialize};

/// O que só o balão tem. `Note::text` é o texto de cima (o número do item).
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DwgBalao {
    /// texto da metade de baixo (balão dividido); vazio = balão de um texto só
    pub abaixo: String,
    /// quantidade escrita ao lado, fora do balão ("2X")
    pub quantidade: String,
    /// nós de cada chamada, do balão para a ponta, na ordem de `setas_nota::pontas`
    pub nos: Vec<Vec<V2>>,
    /// a peça do balão (caminho e configuração, como na lista de materiais): o Atualizar renumera por ela
    pub peca: String,
}

fn d2(a: V2, b: V2) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Distância de `p` ao segmento `a`–`b`.
fn dist_seg(p: V2, a: V2, b: V2) -> f64 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let l2 = ab[0] * ab[0] + ab[1] * ab[1];
    if l2 < 1e-12 {
        return d2(p, a);
    }
    let t = (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / l2).clamp(0., 1.);
    d2(p, [a[0] + t * ab[0], a[1] + t * ab[1]])
}

/// Os pontos da chamada `k`: centro do balão, nós e ponta.
pub fn caminho(n: &Note, k: usize) -> Option<Vec<V2>> {
    let ponta = *crate::setas_nota::pontas(n).get(k)?;
    let mut v = vec![n.pos];
    if let Some(b) = &n.balao {
        if let Some(nos) = b.nos.get(k) {
            v.extend(nos.iter().copied());
        }
    }
    v.push(ponta);
    Some(v)
}

/// Acrescenta um nó na chamada `k`, no trecho mais perto de `p`. Devolve o índice do nó.
pub fn adicionar_no(n: &mut Note, k: usize, p: V2) -> Option<usize> {
    let c = caminho(n, k)?;
    let (i, _) = c.windows(2).enumerate().map(|(i, w)| (i, dist_seg(p, w[0], w[1]))).min_by(|a, b| a.1.total_cmp(&b.1))?;
    let b = n.balao.get_or_insert_with(Default::default);
    if b.nos.len() <= k {
        b.nos.resize(k + 1, vec![]);
    }
    // o trecho i vai do ponto i ao i+1 do caminho; o nó entra no lugar i dos nós
    b.nos[k].insert(i, p);
    Some(i)
}

/// Tira o nó `i` da chamada `k`.
pub fn remover_no(n: &mut Note, k: usize, i: usize) -> bool {
    match n.balao.as_mut().and_then(|b| b.nos.get_mut(k)) {
        Some(nos) if i < nos.len() => {
            nos.remove(i);
            true
        }
        _ => false,
    }
}

/// Nova chamada a partir do nó `i` da chamada `k`: segue os mesmos nós até ele e vai para `ponta`.
pub fn chamada_do_no(n: &mut Note, k: usize, i: usize, ponta: V2) -> Option<usize> {
    let nos = n.balao.as_ref()?.nos.get(k)?;
    let ate = nos.get(..=i)?.to_vec();
    let nova = crate::setas_nota::adicionar(n, ponta);
    let b = n.balao.get_or_insert_with(Default::default);
    if b.nos.len() <= nova {
        b.nos.resize(nova + 1, vec![]);
    }
    b.nos[nova] = ate;
    Some(nova)
}

/// Direção do alinhamento.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eixo {
    Horizontal,
    Vertical,
}

/// Alinha os balões `ids` numa linha: na horizontal ficam na altura do primeiro; na vertical, na coluna
/// do primeiro. Devolve quantos mudaram de lugar.
pub fn alinhar(notas: &mut [Note], ids: &[Id], eixo: Eixo) -> usize {
    let Some(base) = ids.iter().find_map(|id| notas.iter().find(|n| n.id == *id && n.kind == NoteKind::Balloon)).map(|n| n.pos) else { return 0 };
    let mut k = 0;
    for n in notas.iter_mut().filter(|n| ids.contains(&n.id) && n.kind == NoteKind::Balloon) {
        let c = if eixo == Eixo::Horizontal { 1 } else { 0 };
        if (n.pos[c] - base[c]).abs() > 1e-9 {
            n.pos[c] = base[c];
            k += 1;
        }
    }
    k
}

/// Atualizar: o número de cada balão ligado a uma peça passa a ser o item dela na lista (`itens`: peça,
/// item). Balão sem peça ou com peça que saiu da lista fica como está. Devolve quantos mudaram.
pub fn renumerar(notas: &mut [Note], itens: &[(String, String)]) -> usize {
    let mut k = 0;
    for n in notas.iter_mut().filter(|n| n.kind == NoteKind::Balloon) {
        let Some(peca) = n.balao.as_ref().map(|b| b.peca.clone()).filter(|p| !p.is_empty()) else { continue };
        if let Some((_, item)) = itens.iter().find(|(p, _)| *p == peca) {
            if n.text != *item {
                n.text = item.clone();
                k += 1;
            }
        }
    }
    k
}

/// Lado dos balões automáticos em volta da vista.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Lado {
    /// o lado mais perto de cada ponta
    #[default]
    Perto,
    Direita,
    Esquerda,
    Acima,
    Abaixo,
}

impl Lado {
    pub const TODOS: [Lado; 5] = [Lado::Perto, Lado::Direita, Lado::Esquerda, Lado::Acima, Lado::Abaixo];
    pub fn label(self) -> &'static str {
        match self {
            Lado::Perto => "Lado mais perto",
            Lado::Direita => "Direita",
            Lado::Esquerda => "Esquerda",
            Lado::Acima => "Acima",
            Lado::Abaixo => "Abaixo",
        }
    }
}

/// Janela de seleção (cantos em qualquer ordem).
pub fn dentro(p: V2, a: V2, b: V2) -> bool {
    p[0] >= a[0].min(b[0]) && p[0] <= a[0].max(b[0]) && p[1] >= a[1].min(b[1]) && p[1] <= a[1].max(b[1])
}

/// Posições dos balões automáticos: cada ponta vai para o lado pedido da vista (`lo`–`hi`), a
/// `distancia` da borda, em fila sem sobrepor (passo de `passo` mm), na ordem da ponta ao longo da borda.
pub fn posicoes(pontas: &[V2], lo: V2, hi: V2, lado: Lado, distancia: f64, passo: f64) -> Vec<V2> {
    let centro = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.];
    let lado_de = |p: V2| match lado {
        Lado::Perto => {
            let (dx, dy) = ((p[0] - centro[0]) / (hi[0] - lo[0]).max(1e-9), (p[1] - centro[1]) / (hi[1] - lo[1]).max(1e-9));
            if dx.abs() >= dy.abs() {
                if dx >= 0. { Lado::Direita } else { Lado::Esquerda }
            } else if dy >= 0. {
                Lado::Acima
            } else {
                Lado::Abaixo
            }
        }
        l => l,
    };
    let mut out = vec![[0.; 2]; pontas.len()];
    for l in [Lado::Direita, Lado::Esquerda, Lado::Acima, Lado::Abaixo] {
        let mut grupo: Vec<usize> = (0..pontas.len()).filter(|i| lado_de(pontas[*i]) == l).collect();
        let vertical = matches!(l, Lado::Direita | Lado::Esquerda);
        // ao longo da borda, na ordem das pontas: as chamadas não se cruzam
        grupo.sort_by(|a, b| {
            let (pa, pb) = (pontas[*a], pontas[*b]);
            if vertical { pb[1].total_cmp(&pa[1]) } else { pa[0].total_cmp(&pb[0]) }
        });
        let mut ultimo = f64::NAN;
        for i in grupo {
            let p = pontas[i];
            let mut s = if vertical { p[1] } else { p[0] };
            if !ultimo.is_nan() {
                s = if vertical { s.min(ultimo - passo) } else { s.max(ultimo + passo) };
            }
            ultimo = s;
            out[i] = match l {
                Lado::Direita => [hi[0] + distancia, s],
                Lado::Esquerda => [lo[0] - distancia, s],
                Lado::Acima => [s, hi[1] + distancia],
                _ => [s, lo[1] - distancia],
            };
        }
    }
    out
}

#[cfg(test)]
mod testes {
    use super::*;

    fn balao(id: u64, pos: V2, ponta: V2, texto: &str, peca: &str) -> Note {
        Note { id: Id(id), kind: NoteKind::Balloon, pos, leader: Some(ponta), text: texto.into(), balao: Some(DwgBalao { peca: peca.into(), ..Default::default() }), ..Default::default() }
    }

    #[test]
    fn no_entra_no_trecho_mais_perto_e_sai() {
        let mut n = balao(1, [0., 0.], [100., 0.], "1", "");
        assert_eq!(adicionar_no(&mut n, 0, [50., 10.]), Some(0));
        assert_eq!(caminho(&n, 0).unwrap(), vec![[0., 0.], [50., 10.], [100., 0.]]);
        // perto do fim: entra depois do primeiro nó
        assert_eq!(adicionar_no(&mut n, 0, [80., 5.]), Some(1));
        assert_eq!(caminho(&n, 0).unwrap()[2], [80., 5.]);
        assert!(remover_no(&mut n, 0, 0));
        assert_eq!(caminho(&n, 0).unwrap(), vec![[0., 0.], [80., 5.], [100., 0.]]);
        assert!(!remover_no(&mut n, 0, 5));
        assert!(adicionar_no(&mut n, 3, [1., 1.]).is_none(), "chamada inexistente");
    }

    #[test]
    fn nova_chamada_a_partir_de_um_no_segue_os_mesmos_nos() {
        let mut n = balao(1, [0., 0.], [100., 0.], "1", "");
        adicionar_no(&mut n, 0, [40., 20.]);
        adicionar_no(&mut n, 0, [70., 10.]);
        let k = chamada_do_no(&mut n, 0, 0, [40., -30.]).unwrap();
        assert_eq!(k, 1);
        assert_eq!(caminho(&n, 1).unwrap(), vec![[0., 0.], [40., 20.], [40., -30.]]);
        assert_eq!(crate::setas_nota::pontas(&n).len(), 2);
    }

    #[test]
    fn alinhar_na_horizontal_e_na_vertical_pelo_primeiro() {
        let mut v = vec![balao(1, [10., 50.], [0., 0.], "1", ""), balao(2, [30., 57.], [0., 0.], "2", ""), balao(3, [60., 41.], [0., 0.], "3", ""), Note { id: Id(4), pos: [0., 99.], ..Default::default() }];
        assert_eq!(alinhar(&mut v, &[Id(1), Id(2), Id(3), Id(4)], Eixo::Horizontal), 2);
        assert!(v[..3].iter().all(|n| n.pos[1] == 50.));
        assert_eq!(v[3].pos[1], 99., "nota que não é balão fica");
        assert_eq!(alinhar(&mut v, &[Id(3), Id(1)], Eixo::Vertical), 1);
        assert_eq!(v[0].pos[0], 60.);
    }

    #[test]
    fn atualizar_renumera_pela_lista_e_deixa_os_soltos() {
        let mut v = vec![balao(1, [0.; 2], [0.; 2], "1", "eixo.fkp"), balao(2, [0.; 2], [0.; 2], "2", "bucha.fkp"), balao(3, [0.; 2], [0.; 2], "9", "")];
        let lista = vec![("bucha.fkp".to_string(), "1".to_string()), ("eixo.fkp".to_string(), "2".to_string())];
        assert_eq!(renumerar(&mut v, &lista), 2);
        assert_eq!((v[0].text.as_str(), v[1].text.as_str(), v[2].text.as_str()), ("2", "1", "9"));
        assert_eq!(renumerar(&mut v, &lista), 0);
    }

    #[test]
    fn automaticos_vao_para_o_lado_pedido_em_fila_sem_sobrepor() {
        let (lo, hi) = ([0., 0.], [100., 60.]);
        let pontas = [[90., 30.], [10., 30.], [50., 55.], [92., 31.]];
        let p = posicoes(&pontas, lo, hi, Lado::Perto, 12., 8.);
        assert_eq!(p[0][0], 112.);
        assert_eq!(p[1][0], -12.);
        assert_eq!(p[2][1], 72.);
        // dois à direita quase na mesma altura: o de baixo desce o passo
        assert!((p[0][1] - p[3][1]).abs() >= 8. - 1e-9, "{p:?}");
        let d = posicoes(&pontas, lo, hi, Lado::Abaixo, 5., 8.);
        assert!(d.iter().all(|q| q[1] == -5.));
        let mut xs: Vec<f64> = d.iter().map(|q| q[0]).collect();
        xs.sort_by(f64::total_cmp);
        assert!(xs.windows(2).all(|w| w[1] - w[0] >= 8. - 1e-9), "{xs:?}");
        assert!(dentro([5., 5.], [10., 0.], [0., 10.]));
        assert!(!dentro([11., 5.], [10., 0.], [0., 10.]));
    }

    /// Vista de frente sem peça: a projeção é só os eixos, o bastante para medir cotas.
    fn vista() -> (crate::Drawing, crate::View, crate::source::Proj) {
        let mut d = crate::Drawing::new("t", crate::Format::A4);
        let s = d.sheets[0].id;
        let fonte = crate::Source { path: "p.fkp".into(), display_state: String::new(), hidden: vec![] };
        let id = d.add_view(s, fonte, crate::ViewKind::Standard(crate::Dir::Frente), [100., 100.], 1.);
        let p = crate::source::Proj { axes: Some(crate::source::Axes::of(crate::Dir::Frente)), ..Default::default() };
        let v = d.view(id).unwrap().clone();
        (d, v, p)
    }

    fn cota(v: &crate::View, p: &crate::source::Proj, kind: crate::DimKind, pts: &[V2]) -> crate::Dim {
        let ax = p.axes.unwrap();
        crate::Dim { id: Id(900), view: v.id, kind, anchors: pts.iter().map(|q| ax.unproject(*q)).collect(), ..Default::default() }
    }

    #[test]
    fn meia_cota_mede_o_dobro_e_tira_a_seta_do_eixo() {
        let (d, v, p) = vista();
        let mut c = cota(&v, &p, crate::DimKind::Horizontal, &[[0., 0.], [15., 0.]]);
        assert!((crate::layout::dim_value(&c, &v, &p).unwrap().0 - 15.).abs() < 1e-9);
        c.opcoes.meia = true;
        assert!((crate::layout::dim_value(&c, &v, &p).unwrap().0 - 30.).abs() < 1e-9);
        assert_eq!(crate::layout::dim_text(&d, &c, 30.), "30");
        // com como_diametro (o uso comum em meia vista): Ø30
        c.opcoes.como_diametro = true;
        assert_eq!(crate::layout::dim_text(&d, &c, 30.), "Ø30");
    }

    #[test]
    fn cota_de_angulo_vira_comprimento_do_arco() {
        let (d, v, p) = vista();
        // raio 10, 90 graus: arco de 15,71
        let mut c = cota(&v, &p, crate::DimKind::Angle, &[[0., 0.], [10., 0.], [0., 10.]]);
        assert!((crate::layout::dim_value(&c, &v, &p).unwrap().0 - 90.).abs() < 1e-6);
        c.opcoes.arco = true;
        let l = crate::layout::dim_value(&c, &v, &p).unwrap().0;
        assert!((l - 10. * std::f64::consts::FRAC_PI_2).abs() < 1e-6, "{l}");
        assert!(crate::layout::dim_text(&d, &c, l).starts_with('⌒'));
    }

    #[test]
    fn balao_dividido_com_quantidade_e_nos_desenha_tudo() {
        use crate::layout::Prim;
        let d = crate::Drawing::new("t", crate::Format::A4);
        let mut m = crate::source::Models::new(None);
        let mut n = balao(500, [100., 100.], [60., 60.], "3", "");
        let b = n.balao.as_mut().unwrap();
        b.abaixo = "B".into();
        b.quantidade = "2X".into();
        adicionar_no(&mut n, 0, [80., 60.]);
        let itens = crate::layout::note_items(&d, &n, &mut m);
        let textos: Vec<&str> = itens.iter().filter_map(|i| if let Prim::Text { text, .. } = &i.prim { Some(text.as_str()) } else { None }).collect();
        for t in ["3", "B", "2X"] {
            assert!(textos.contains(&t), "{t} em {textos:?}");
        }
        // a chamada passa pelo nó
        let passa = itens.iter().any(|i| matches!(&i.prim, Prim::Line { pts, .. } if pts.contains(&[80., 60.]) && pts.last() == Some(&[60., 60.])));
        assert!(passa, "chamada sem o nó");
        // balão simples com largura fixa: o círculo tem o diâmetro pedido
        let mut s = balao(501, [0., 0.], [30., 30.], "1", "");
        s.style.frame_fixed = 20.;
        let itens = crate::layout::note_items(&d, &s, &mut m);
        let raio = itens.iter().find_map(|i| if let Prim::Line { pts, .. } = &i.prim { (pts.len() > 20).then(|| pts.iter().map(|p| (p[0] * p[0] + p[1] * p[1]).sqrt()).fold(0., f64::max)) } else { None }).unwrap();
        assert!((raio - 10.).abs() < 0.2, "{raio}");
    }
}
