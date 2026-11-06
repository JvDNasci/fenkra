// SPDX-License-Identifier: LGPL-2.1-or-later
//! Seleção da Forma livre (B302), sem egui: crescer, encolher, laço, anel, próximo e faixa. As regras
//! andam na malha inteira, para um laço atravessar o plano de simetria e voltar; o que cai no reflexo
//! sai no fim, porque só a metade guardada se edita.

use crate::forma_edit::Elem;
use fenkra_core::subd::{Gaiola, ESPELHO};
use std::collections::BTreeSet;

fn real(e: Elem) -> bool {
    match e {
        Elem::No(n) | Elem::Face(n) => n & ESPELHO == 0,
        Elem::Aresta(a, b) => (a | b) & ESPELHO == 0,
    }
}

fn ordem(a: u32, b: u32) -> Elem {
    if a < b {
        Elem::Aresta(a, b)
    } else {
        Elem::Aresta(b, a)
    }
}

/// Mantém a ordem da seleção e soma o que é novo.
fn somar(sel: &[Elem], novos: impl IntoIterator<Item = Elem>) -> Vec<Elem> {
    let mut out = sel.to_vec();
    for e in novos.into_iter().filter(|e| real(*e)) {
        if !out.contains(&e) {
            out.push(e);
        }
    }
    out
}

/// Vizinhos de mesmo tipo: nó pela aresta, aresta pelo nó, face pela aresta.
fn vizinhos(g: &Gaiola, e: Elem) -> Vec<Elem> {
    match e {
        Elem::No(n) => g.vizinhos_de(n).into_iter().map(Elem::No).collect(),
        Elem::Aresta(a, b) => {
            let mut v: Vec<Elem> = [(a, b), (b, a)].iter().flat_map(|(p, q)| g.vizinhos_de(*p).into_iter().filter(move |x| x != q).map(move |x| ordem(*p, x))).collect();
            v.sort_unstable();
            v
        }
        Elem::Face(f) => g.faces_vizinhas(f).into_iter().map(Elem::Face).collect(),
    }
}

/// Soma a volta inteira do que está selecionado.
pub fn crescer(g: &Gaiola, sel: &[Elem]) -> Vec<Elem> {
    let i = g.inteira();
    somar(sel, sel.iter().flat_map(|e| vizinhos(&i, *e)))
}

/// Tira o que tem vizinho de fora. Nó do plano de simetria não conta o reflexo como fora, e a seleção
/// nunca fica vazia por encolher: é o que evita perder o trabalho num clique a mais.
pub fn encolher(g: &Gaiola, sel: &[Elem]) -> Vec<Elem> {
    let i = g.inteira();
    let dentro: BTreeSet<Elem> = sel.iter().copied().collect();
    let fica: Vec<Elem> = sel.iter().copied().filter(|e| vizinhos(&i, *e).into_iter().filter(|v| real(*v)).all(|v| dentro.contains(&v))).collect();
    if fica.is_empty() {
        sel.to_vec()
    } else {
        fica
    }
}

/// Última aresta da seleção, ou a que une os dois últimos nós.
fn aresta_base(sel: &[Elem]) -> Option<(u32, u32)> {
    if let Some((a, b)) = sel.iter().rev().find_map(|e| if let Elem::Aresta(a, b) = e { Some((*a, *b)) } else { None }) {
        return Some((a, b));
    }
    let nos: Vec<u32> = sel.iter().rev().filter_map(|e| if let Elem::No(n) = e { Some(*n) } else { None }).take(2).collect();
    (nos.len() == 2).then(|| (nos[1], nos[0]))
}

/// Aresta que duas faces vizinhas dividem, ou a primeira da última face.
fn aresta_de_faces(g: &Gaiola, sel: &[Elem]) -> Option<(u32, u32)> {
    let fs: Vec<u32> = sel.iter().rev().filter_map(|e| if let Elem::Face(f) = e { Some(*f) } else { None }).take(2).collect();
    let f = g.face(*fs.first()?)?;
    let lados: Vec<(u32, u32)> = (0..f.nos.len()).map(|k| (f.nos[k], f.nos[(k + 1) % f.nos.len()])).collect();
    if let Some(o) = fs.get(1).and_then(|o| g.face(*o)) {
        if let Some(l) = lados.iter().find(|(a, b)| o.nos.contains(a) && o.nos.contains(b)) {
            return Some(*l);
        }
    }
    lados.first().copied()
}

/// Laço: arestas seguindo reto; com faces, a volta de faces que atravessa a aresta entre elas.
pub fn laco(g: &Gaiola, sel: &[Elem]) -> Result<Vec<Elem>, String> {
    let i = g.inteira();
    if let Some((a, b)) = aresta_base(sel) {
        let l = i.laco_de_arestas(a, b)?;
        return Ok(somar(sel, l.into_iter().map(|(x, y)| ordem(x, y))));
    }
    let (a, b) = aresta_de_faces(&i, sel).ok_or("selecione uma aresta, dois nós ou uma face")?;
    let (_, faces) = i.laco_de(a, b)?;
    Ok(somar(sel, faces.into_iter().map(Elem::Face)))
}

/// Anel: as arestas paralelas que um corte atravessaria.
pub fn anel(g: &Gaiola, sel: &[Elem]) -> Result<Vec<Elem>, String> {
    let i = g.inteira();
    let (a, b) = aresta_base(sel).ok_or("selecione uma aresta ou dois nós")?;
    let (arestas, _) = i.laco_de(a, b)?;
    Ok(somar(sel, arestas.into_iter().map(|(x, y)| ordem(x, y))))
}

/// Próximo: anda um passo na direção dos dois últimos selecionados, ou escolhe um primeiro passo.
pub fn proximo(g: &Gaiola, sel: &[Elem]) -> Result<Vec<Elem>, String> {
    let i = g.inteira();
    let &ultimo = sel.last().ok_or("selecione algo primeiro")?;
    let antes = sel.iter().rev().nth(1).copied();
    let novo = match ultimo {
        Elem::No(n) => {
            let de = antes.and_then(|e| if let Elem::No(x) = e { Some(x) } else { None });
            match de {
                Some(d) if i.vizinhos_de(n).contains(&d) => {
                    let l = i.laco_de_arestas(d, n)?;
                    l.iter().find(|(x, _)| *x == n).map(|(_, y)| *y).or_else(|| l.iter().find(|(_, y)| *y == n).map(|(x, _)| *x).filter(|x| *x != d)).map(Elem::No)
                }
                _ => i.vizinhos_de(n).into_iter().map(Elem::No).find(|e| real(*e) && !sel.contains(e)),
            }
        }
        Elem::Aresta(a, b) => i.laco_de_arestas(a, b)?.into_iter().map(|(x, y)| ordem(x, y)).skip_while(|e| *e != ultimo).nth(1).filter(|e| !sel.contains(e)).or_else(|| {
            i.laco_de_arestas(a, b).ok()?.into_iter().rev().map(|(x, y)| ordem(x, y)).skip_while(|e| *e != ultimo).nth(1)
        }),
        Elem::Face(f) => {
            let par_de = aresta_de_faces(&i, sel).ok_or("face sem arestas")?;
            let (_, faces) = i.laco_de(par_de.0, par_de.1)?;
            let k = faces.iter().position(|x| *x == f);
            let vizinhas = k.into_iter().flat_map(|k| [k + 1, k.wrapping_sub(1)]).filter_map(|k| faces.get(k).copied()).map(Elem::Face);
            vizinhas.into_iter().find(|e| real(*e) && !sel.contains(e))
        }
    };
    let novo = novo.filter(|e| real(*e)).ok_or("não há próximo nessa direção")?;
    Ok(somar(sel, [novo]))
}

/// Faixa entre os dois últimos: faces pelo caminho mais curto; nós pelas arestas.
pub fn faixa(g: &Gaiola, sel: &[Elem]) -> Result<Vec<Elem>, String> {
    let i = g.inteira();
    let dois = |f: fn(&Elem) -> Option<u32>| -> Vec<u32> { sel.iter().rev().filter_map(f).take(2).collect() };
    let faces = dois(|e| if let Elem::Face(f) = e { Some(*f) } else { None });
    if let [b, a] = faces[..] {
        let v = i.faixa_de_faces(a, b).ok_or("essas faces não estão na mesma malha")?;
        return Ok(somar(sel, v.into_iter().map(Elem::Face)));
    }
    let nos = dois(|e| if let Elem::No(n) = e { Some(*n) } else { None });
    if let [b, a] = nos[..] {
        let v = i.caminho_de_nos(a, b).ok_or("esses nós não estão na mesma malha")?;
        return Ok(somar(sel, v.into_iter().map(Elem::No)));
    }
    Err("selecione duas faces ou dois nós".into())
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::forma_edit::alternar_simetria;

    /// Grade de 4 por 4 quadrados de lado 10.
    fn grade() -> Gaiola {
        Gaiola::plano([20., 20., 0.], [40., 40.], [4, 4])
    }

    fn no(g: &Gaiola, x: f64, y: f64) -> u32 {
        g.nos.iter().find(|n| (n.p[0] - x).abs() < 1e-9 && (n.p[1] - y).abs() < 1e-9).expect("nó").id
    }

    fn face(g: &Gaiola, x: f64, y: f64) -> u32 {
        g.faces.iter().find(|f| {
            let c = g.centro_face(f.id).unwrap();
            (c[0] - x).abs() < 1. && (c[1] - y).abs() < 1.
        }).expect("face").id
    }

    #[test]
    fn crescer_e_encolher_voltam_ao_comeco() {
        let g = grade();
        let c = Elem::No(no(&g, 20., 20.));
        let mais = crescer(&g, &[c]);
        assert_eq!(mais.len(), 5);
        assert_eq!(encolher(&g, &mais), vec![c]);
        let f = Elem::Face(face(&g, 15., 15.));
        let mais = crescer(&g, &[f]);
        assert_eq!(mais.len(), 5);
        assert_eq!(encolher(&g, &mais), vec![f]);
        // encolher o que não tem miolo mantém a seleção
        assert_eq!(encolher(&g, &[c]), vec![c]);
    }

    #[test]
    fn crescer_aresta_pega_as_que_tocam_as_pontas() {
        let g = grade();
        let (a, b) = (no(&g, 10., 10.), no(&g, 20., 10.));
        let e = ordem(a, b);
        assert_eq!(crescer(&g, &[e]).len(), 7);
    }

    #[test]
    fn laco_e_anel_sao_perpendiculares() {
        let g = grade();
        let e = ordem(no(&g, 10., 20.), no(&g, 20., 20.));
        let l = laco(&g, &[e]).unwrap();
        assert_eq!(l.len(), 4, "a fileira inteira em y = 20");
        for x in &l {
            let Elem::Aresta(a, b) = *x else { panic!() };
            assert!((g.pos(a).unwrap()[1] - 20.).abs() < 1e-9 && (g.pos(b).unwrap()[1] - 20.).abs() < 1e-9);
        }
        let r = anel(&g, &[e]).unwrap();
        assert_eq!(r.len(), 5, "as paralelas de y = 0 a y = 40");
        let ys: BTreeSet<i64> = r.iter().map(|x| if let Elem::Aresta(a, _) = x { g.pos(*a).unwrap()[1] as i64 } else { -1 }).collect();
        assert_eq!(ys.len(), 5);
    }

    #[test]
    fn laco_de_faces_da_a_volta_na_caixa() {
        let g = Gaiola::caixa([0.; 3], [20.; 3], [2, 2, 2]);
        let f = g.faces.iter().find(|f| g.normal_face(f.id).unwrap()[2] > 0.9).unwrap().id;
        let l = laco(&g, &[Elem::Face(f)]).unwrap();
        assert_eq!(l.len(), 8, "{l:?}");
    }

    #[test]
    fn proximo_continua_a_direcao() {
        let g = grade();
        let s = vec![Elem::No(no(&g, 10., 20.)), Elem::No(no(&g, 20., 20.))];
        let s = proximo(&g, &s).unwrap();
        assert_eq!(s[2], Elem::No(no(&g, 30., 20.)));
        let s = proximo(&g, &s).unwrap();
        assert_eq!(s[3], Elem::No(no(&g, 40., 20.)));
        assert!(proximo(&g, &s).is_err(), "a borda termina o caminho");
        let e = vec![ordem(no(&g, 10., 20.), no(&g, 20., 20.))];
        assert_eq!(proximo(&g, &e).unwrap().len(), 2);
        let f = vec![Elem::Face(face(&g, 15., 15.)), Elem::Face(face(&g, 25., 15.))];
        assert_eq!(proximo(&g, &f).unwrap()[2], Elem::Face(face(&g, 35., 15.)));
    }

    #[test]
    fn faixa_entre_duas_faces_e_dois_nos() {
        let g = grade();
        let s = vec![Elem::Face(face(&g, 5., 5.)), Elem::Face(face(&g, 35., 5.))];
        assert_eq!(faixa(&g, &s).unwrap().len(), 4);
        let s = vec![Elem::No(no(&g, 0., 20.)), Elem::No(no(&g, 40., 20.))];
        assert_eq!(faixa(&g, &s).unwrap().len(), 5);
        assert!(faixa(&g, &[Elem::Face(face(&g, 5., 5.))]).is_err());
    }

    #[test]
    fn laco_atravessa_a_simetria_sem_selecionar_o_reflexo() {
        let mut g = grade();
        for n in &mut g.nos {
            n.p[0] -= 20.;
        }
        alternar_simetria(&mut g, 0).unwrap();
        let e = ordem(no(&g, 10., 20.), no(&g, 20., 20.));
        let l = laco(&g, &[e]).unwrap();
        assert_eq!(l.len(), 2);
        assert!(l.iter().all(|x| real(*x)));
    }
}
