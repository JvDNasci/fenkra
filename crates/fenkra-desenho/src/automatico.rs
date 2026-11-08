// SPDX-License-Identifier: LGPL-2.1-or-later
//! B144: desenho automático por regras.
//!
//! Sobre as vistas que a folha já tem (ou as três vistas padrão que o comando cria quando a folha
//! está vazia), as regras são:
//!
//! 1. **Contorno**: na vista principal (frente), a largura embaixo e a altura à esquerda; nas
//!    projetadas, só a medida que a frente não mostra (a profundidade): vertical na de cima ou de
//!    baixo, horizontal nas laterais. A cota vai de vértice a vértice da projeção, então fica presa
//!    à geometria e se refaz quando a peça muda.
//! 2. **Furos**: na vista com mais círculos, uma cota de diâmetro por tamanho de furo, com "n×"
//!    quando o tamanho se repete, e a marca de centro em cada furo.
//! 3. **Tabela de furos** num canto livre da folha, se a peça tem furos e a folha ainda não tem a
//!    tabela.
//! 4. **Carimbo**: os campos automáticos ({descricao}, {codigo}, {material}, {massa}) vêm da peça.
//!
//! Rodar de novo não duplica: cada regra confere se a cota, a marca ou a tabela já existe.

use crate::centros::{self, ModoCentro};
use crate::layout::{anchor_candidates, dim_value};
use crate::source::Models;
use crate::{Dim, DimKind, Dir, Drawing, Table, TableKind, View, ViewKind, V2};
use fenkra_core::Id;

/// O que o automático fez.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Relatorio {
    pub cotas: usize,
    pub centros: usize,
    pub tabela: bool,
    pub avisos: Vec<String>,
}

/// Papel da vista nas regras de contorno.
fn medidas_da_vista(v: &View) -> Option<(bool, bool)> {
    match &v.kind {
        ViewKind::Standard(Dir::Iso) => None,
        ViewKind::Standard(_) => Some((true, true)),
        ViewKind::Projected { dir: Dir::Topo | Dir::Base, .. } => Some((false, true)),
        ViewKind::Projected { dir: Dir::Esquerda | Dir::Direita | Dir::Frente | Dir::Tras, .. } => Some((true, false)),
        _ => None,
    }
}

/// Furos da projeção: círculos inteiros (os arcos do mesmo centro e raio somam a volta toda). As
/// pontas de um rasgo são meios círculos e ficam de fora.
pub fn circulos(p: &crate::source::Proj) -> Vec<(V2, f64)> {
    let mut grupos: Vec<(V2, f64, f64)> = vec![];
    for (pl, k) in &p.lines {
        if !matches!(k, 0 | 2 | 3) {
            continue;
        }
        let Some((c, r)) = crate::layout::arc_center(pl) else { continue };
        // ângulo varrido pela polilinha
        let ang = |q: &V2| (q[1] - c[1]).atan2(q[0] - c[0]);
        let mut varre = 0.;
        for w in pl.windows(2) {
            let mut da = ang(&w[1]) - ang(&w[0]);
            if da > std::f64::consts::PI {
                da -= std::f64::consts::TAU;
            } else if da < -std::f64::consts::PI {
                da += std::f64::consts::TAU;
            }
            varre += da.abs();
        }
        match grupos.iter_mut().find(|g| (g.0[0] - c[0]).abs() < 1e-3 && (g.0[1] - c[1]).abs() < 1e-3 && (g.1 - r).abs() < 1e-3) {
            Some(g) => g.2 += varre,
            None => grupos.push((c, r, varre)),
        }
    }
    grupos.into_iter().filter(|g| g.2 >= std::f64::consts::TAU * 0.97).map(|g| (g.0, g.1)).collect()
}

fn perto(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

/// Já há na vista uma cota do mesmo tipo com o mesmo valor?
fn ja_tem(d: &Drawing, m: &mut Models, v: &View, kind: DimKind, valor: f64) -> bool {
    let p = m.proj(d, v.id);
    d.dims.iter().filter(|x| x.view == v.id && x.kind == kind).any(|x| dim_value(x, v, &p).is_some_and(|(val, _)| perto(val, valor)))
}

/// Aplica as regras na folha. Devolve o que foi feito.
pub fn completar(d: &mut Drawing, sheet: Id, m: &mut Models) -> Relatorio {
    let mut rel = Relatorio::default();
    let vistas: Vec<View> = d.views.iter().filter(|v| v.sheet == sheet).cloned().collect();
    if vistas.is_empty() {
        rel.avisos.push("a folha não tem vistas: crie as vistas antes".into());
        return rel;
    }
    // 1. contorno
    for v in &vistas {
        let Some((horiz, vert)) = medidas_da_vista(v) else { continue };
        let p = m.proj(d, v.id);
        let Some(ax) = p.axes.filter(|_| p.error.is_none() && !p.calculando) else {
            rel.avisos.push(format!("a vista {} ainda não tem projeção", v.name));
            continue;
        };
        let c = anchor_candidates(&p);
        if c.len() < 2 {
            continue;
        }
        let ext = |f: &dyn Fn(&V2) -> (f64, f64)| -> (V2, V2) {
            let mut o = c.clone();
            o.sort_by(|a, b| f(a).partial_cmp(&f(b)).unwrap_or(std::cmp::Ordering::Equal));
            (o[0], o[o.len() - 1])
        };
        let mut nova = |d: &mut Drawing, kind: DimKind, a: V2, b: V2, offset: f64| {
            let id = d.alloc();
            d.dims.push(Dim { id, view: v.id, kind, anchors: vec![ax.unproject(a), ax.unproject(b)], offset, ..Default::default() });
            rel.cotas += 1;
        };
        if horiz {
            // do vértice mais à esquerda ao mais à direita, acima da vista: embaixo fica o rótulo dela
            let (a, b) = ext(&|q| (q[0], q[1]));
            if (b[0] - a[0]).abs() > 1e-6 && !ja_tem(d, m, v, DimKind::Horizontal, (b[0] - a[0]).abs()) {
                nova(d, DimKind::Horizontal, a, b, 8.);
            }
        }
        if vert {
            let (a, b) = ext(&|q| (q[1], q[0]));
            if (b[1] - a[1]).abs() > 1e-6 && !ja_tem(d, m, v, DimKind::Vertical, (b[1] - a[1]).abs()) {
                nova(d, DimKind::Vertical, a, b, 8.);
            }
        }
    }
    // 2. furos, na vista que mostra mais círculos
    let mut melhor: Option<(View, Vec<(V2, f64)>)> = None;
    for v in vistas.iter().filter(|v| medidas_da_vista(v).is_some()) {
        let p = m.proj(d, v.id);
        let arcs: Vec<(V2, f64)> = circulos(&p).into_iter().filter(|(_, r)| *r > 0.2).collect();
        if arcs.len() > melhor.as_ref().map_or(0, |b| b.1.len()) {
            melhor = Some((v.clone(), arcs));
        }
    }
    if let Some((v, arcs)) = melhor {
        let p = m.proj(d, v.id);
        let ax = p.axes.expect("vista com arcos tem eixos");
        let mut tamanhos: Vec<(f64, Vec<V2>)> = vec![];
        for (c, r) in &arcs {
            match tamanhos.iter_mut().find(|t| (t.0 - r).abs() <= r * 0.01 + 1e-3) {
                Some(t) => t.1.push(*c),
                None => tamanhos.push((*r, vec![*c])),
            }
        }
        for (k, (r, cs)) in tamanhos.into_iter().enumerate() {
            if ja_tem(d, m, &v, DimKind::Diameter, 2. * r) {
                continue;
            }
            // cada tamanho num furo diferente (furo e rebaixo não se cobrem) e a chamada para fora
            let n = cs.len();
            let c = cs[k % n];
            let fora = [c[0] - p.center[0], c[1] - p.center[1]];
            let l = (fora[0] * fora[0] + fora[1] * fora[1]).sqrt();
            let u = if l > 1e-9 { [fora[0] / l, fora[1] / l] } else { [std::f64::consts::FRAC_1_SQRT_2; 2] };
            let id = d.alloc();
            let borda = [c[0] + r * u[0], c[1] + r * u[1]];
            let text = if n > 1 { format!("{n}× <>") } else { String::new() };
            d.dims.push(Dim { id, view: v.id, kind: DimKind::Diameter, anchors: vec![ax.unproject(c), ax.unproject(borda)], offset: 8., text, ..Default::default() });
            rel.cotas += 1;
        }
        // marca de centro em cada furo que ainda não tem
        for (c, _) in &arcs {
            let tem = d.centers.iter().any(|mc| mc.view == v.id && mc.anchors.iter().any(|a| {
                let q = ax.project(*a);
                (q[0] - c[0]).abs() < 1e-3 && (q[1] - c[1]).abs() < 1e-3
            }));
            if tem {
                continue;
            }
            let id = d.alloc();
            if let Some(mc) = centros::montar(id, v.id, ModoCentro::Simples, &[*c], &p) {
                d.centers.push(mc);
                rel.centros += 1;
            }
        }
        // 3. tabela de furos num canto livre
        if !d.tables.iter().any(|t| t.sheet == sheet && t.kind == TableKind::Holes) {
            let id = d.alloc();
            d.tables.push(Table { id, sheet, kind: TableKind::Holes, source: v.source.clone(), pos: [25., 120.], level: Default::default(), cfg: Default::default() });
            match crate::ocupacao::posicao_livre(m, d, id) {
                Some(p) => {
                    if let Some(t) = d.tables.iter_mut().find(|t| t.id == id) {
                        t.pos = p;
                    }
                }
                None => rel.avisos.push("a tabela de furos não coube na folha".into()),
            }
            rel.tabela = true;
        }
    }
    rel
}
