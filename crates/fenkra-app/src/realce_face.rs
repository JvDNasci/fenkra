// SPDX-License-Identifier: LGPL-2.1-or-later
//! B361: realce de face só pelo contorno externo.
//!
//! Antes a face selecionada ou sob o mouse era pintada inteira: cada triângulo dela era projetado e
//! enchido a cada quadro, o que cobria a peça e pesava em face com muitos triângulos. Agora só o
//! contorno vira linha. Ele sai das arestas do B-Rep que tocam a face (`Mesh::edge_faces`): já são
//! polilinhas prontas, sem olhar triângulo nenhum. Furos dentro da face são outro laço de arestas,
//! sem ponta em comum com o laço de fora, e por isso ficam de fora: pegamos o laço de maior extensão.
//! O custo por quadro é proporcional às arestas do corpo, e a projeção só às do contorno.

use fenkra_kernel::{Mesh, NO_FACE};
use std::collections::HashMap;

/// Chave de uma ponta de aresta: soldadas a 1 µm, as pontas de arestas vizinhas coincidem.
fn chave(p: [f32; 3]) -> [i64; 3] {
    p.map(|v| (v as f64 * 1000.).round() as i64)
}

fn raiz(pai: &mut [usize], mut i: usize) -> usize {
    while pai[i] != i {
        pai[i] = pai[pai[i]];
        i = pai[i];
    }
    i
}

/// Polilinhas das arestas do contorno externo da face `face`, em coordenadas do modelo.
/// Vazio quando a malha não traz arestas para a face (malha importada sem B-Rep).
pub fn contorno_externo(m: &Mesh, face: usize) -> Vec<Vec<[f32; 3]>> {
    let nseg = m.edges.len() / 6;
    let alvo = face as u32;
    let mut arestas: Vec<Vec<[f32; 3]>> = vec![];
    for (e, f) in m.edge_faces.iter().enumerate() {
        if (f[0] != alvo && f[1] != alvo) || alvo == NO_FACE {
            continue;
        }
        let Some(&s0) = m.edge_ranges.get(e) else { continue };
        let s1 = m.edge_ranges.get(e + 1).map_or(nseg, |x| *x as usize);
        let (s0, s1) = (s0 as usize, s1.min(nseg));
        if s1 <= s0 {
            continue;
        }
        let seg = &m.edges[s0 * 6..s1 * 6];
        let mut pts: Vec<[f32; 3]> = seg.chunks_exact(6).map(|q| [q[0], q[1], q[2]]).collect();
        pts.push([seg[seg.len() - 3], seg[seg.len() - 2], seg[seg.len() - 1]]);
        arestas.push(pts);
    }
    if arestas.len() < 2 {
        return arestas;
    }
    // laços: arestas que dividem uma ponta caem no mesmo grupo
    let mut pai: Vec<usize> = (0..arestas.len()).collect();
    let mut ponta: HashMap<[i64; 3], usize> = HashMap::new();
    for (i, a) in arestas.iter().enumerate() {
        for p in [a[0], a[a.len() - 1]] {
            match ponta.get(&chave(p)) {
                Some(&j) => {
                    let (ri, rj) = (raiz(&mut pai, i), raiz(&mut pai, j));
                    pai[ri] = rj;
                }
                None => {
                    ponta.insert(chave(p), i);
                }
            }
        }
    }
    // extensão de cada laço: diagonal da caixa dos seus pontos
    let mut caixa: HashMap<usize, ([f32; 3], [f32; 3])> = HashMap::new();
    for (i, a) in arestas.iter().enumerate() {
        let r = raiz(&mut pai, i);
        let c = caixa.entry(r).or_insert(([f32::MAX; 3], [f32::MIN; 3]));
        for p in a {
            for k in 0..3 {
                c.0[k] = c.0[k].min(p[k]);
                c.1[k] = c.1[k].max(p[k]);
            }
        }
    }
    let diag = |c: &([f32; 3], [f32; 3])| (0..3).map(|k| (c.1[k] - c.0[k]).powi(2)).sum::<f32>();
    let Some((&fora, _)) = caixa.iter().max_by(|a, b| diag(a.1).total_cmp(&diag(b.1)).then(b.0.cmp(a.0))) else { return arestas };
    arestas.into_iter().enumerate().filter(|(i, _)| raiz(&mut pai, *i) == fora).map(|(_, a)| a).collect()
}

/// Contorno de uma malha sem B-Rep: os lados de triângulo que só um triângulo da face usa.
pub fn contorno_da_malha(m: &Mesh, face: usize) -> Vec<Vec<[f32; 3]>> {
    let Some(&ini) = m.face_ranges.get(face) else { return vec![] };
    let fim = m.face_ranges.get(face + 1).map_or(m.indices.len(), |x| *x as usize).min(m.indices.len());
    let pos = |i: u32| {
        let k = i as usize * 3;
        [m.positions[k], m.positions[k + 1], m.positions[k + 2]]
    };
    let mut uso: HashMap<([i64; 3], [i64; 3]), (u32, [f32; 3], [f32; 3])> = HashMap::new();
    for t in m.indices[ini as usize..fim].chunks_exact(3) {
        for k in 0..3 {
            let (a, b) = (pos(t[k]), pos(t[(k + 1) % 3]));
            let (ka, kb) = (chave(a), chave(b));
            let par = if ka <= kb { (ka, kb) } else { (kb, ka) };
            uso.entry(par).or_insert((0, a, b)).0 += 1;
        }
    }
    uso.into_values().filter(|u| u.0 == 1).map(|u| vec![u.1, u.2]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::*;
    use fenkra_solidos::recompute;

    fn s(x: &str) -> String {
        x.to_string()
    }

    fn extrusao(sketch: Id, op: BoolOp) -> FeatureKind {
        FeatureKind::Extrude(Extrude { sketch, op, extent: Extent::Blind(s("20")), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() })
    }

    /// Caixa 40 × 30 × 20, com um furo de raio 4 passando pela tampa quando `furo`.
    fn peca(furo: bool) -> Document {
        let mut d = Document::new("caixa");
        let mut data = SketchData::default();
        let mut al = || d.alloc();
        data.add_rect(&mut al, [-20., -15.], [20., 15.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(extrusao(sk, BoolOp::Add));
        if furo {
            let mut dados = SketchData::default();
            let mut al = || d.alloc();
            let c = dados.add_point(&mut al, 0., 0.);
            dados.add_curve(&mut al, fenkra_core::sketch::Curve::Circle { c, r: 4. }, false);
            let sc = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: dados }));
            let mut corte = extrusao(sc, BoolOp::Cut);
            if let FeatureKind::Extrude(e) = &mut corte {
                // o corte sai do plano para baixo: inverte para entrar no corpo
                e.reverse = true;
            }
            d.add_feature(corte);
        }
        d
    }

    fn tampa(m: &Mesh) -> usize {
        (0..m.face_ranges.len())
            .find(|&f| {
                let v = m.indices[m.face_ranges[f] as usize] as usize;
                (m.normals[v * 3 + 2] - 1.).abs() < 1e-3 && (m.positions[v * 3 + 2] - 20.).abs() < 1e-3
            })
            .expect("face da tampa")
    }

    #[test]
    fn tampa_da_caixa_tem_so_as_quatro_arestas_de_fora() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let r = recompute(&peca(false));
        let m = &r.bodies[0].mesh;
        let c = contorno_externo(m, tampa(m));
        assert_eq!(c.len(), 4, "quatro arestas");
        assert!(c.iter().flatten().all(|p| (p[2] - 20.).abs() < 1e-3), "todas no plano da tampa");
        assert!(c.iter().flatten().all(|p| (p[0].abs() - 20.).abs() < 1e-3 || (p[1].abs() - 15.).abs() < 1e-3), "só nas bordas 40 x 30");
    }

    #[test]
    fn malha_sem_arestas_usa_o_limite_dos_triangulos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let r = recompute(&peca(false));
        let m = &r.bodies[0].mesh;
        let c = contorno_da_malha(m, tampa(m));
        assert!(c.len() >= 4, "{}", c.len());
        assert!(c.iter().flatten().all(|p| (p[0].abs() - 20.).abs() < 1e-3 || (p[1].abs() - 15.).abs() < 1e-3), "nenhuma diagonal da triangulação");
    }

    #[test]
    fn furo_na_tampa_nao_entra_no_contorno() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let r = recompute(&peca(true));
        let m = &r.bodies[0].mesh;
        let f = tampa(m);
        assert!(r.bodies[0].mass.volume < 24000. - 500., "o furo tirou material: {}", r.bodies[0].mass.volume);
        let todas = m.edge_faces.iter().filter(|e| e[0] == f as u32 || e[1] == f as u32).count();
        assert!(todas >= 5, "a tampa tem as quatro bordas e o furo: {todas}");
        let c = contorno_externo(m, f);
        assert_eq!(c.len(), 4, "só a borda de fora");
        assert!(c.iter().flatten().all(|p| p[0].hypot(p[1]) > 10.), "nada do furo (raio 4)");
    }

    /// B361: o custo do contorno não depende dos triângulos da face. Mede o tempo por chamada e
    /// imprime (`cargo test -- --nocapture`); o limite largo só pega regressão grossa.
    #[test]
    fn custo_por_quadro_do_contorno() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let r = recompute(&peca(true));
        let m = &r.bodies[0].mesh;
        let f = tampa(m);
        let n = 2000;
        let t = std::time::Instant::now();
        let mut total = 0;
        for _ in 0..n {
            total += contorno_externo(m, f).len();
        }
        let us = t.elapsed().as_secs_f64() * 1e6 / n as f64;
        println!("contorno_externo: {us:.2} us por chamada, {} arestas no corpo, {} triangulos", m.edge_faces.len(), m.indices.len() / 3);
        assert_eq!(total, 4 * n);
        assert!(us < 500., "contorno lento: {us} us");
    }
}
