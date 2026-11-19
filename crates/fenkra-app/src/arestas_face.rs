// SPDX-License-Identifier: LGPL-2.1-or-later
//! Filete e chanfro por face (B555): com o comando aberto, clicar numa face soma ao comando todas as
//! arestas do contorno dela (todos os laços, o de fora e os dos furos), e clicar de novo na mesma
//! face tira essas arestas. A adjacência vem da malha do kernel: cada aresta guarda as duas faces
//! vizinhas (`Mesh::edge_faces`), então as arestas de uma face são as que citam o índice dela. A
//! costura de um cilindro (a mesma face dos dois lados) e a aresta degenerada ficam de fora.

use fenkra_core::{carry_keys, EdgeKey, EdgeOp, EdgeSel};
use fenkra_solidos::BodyResult;

/// Uma aresta do contorno de uma face: o ponto médio (como o comando guarda) e a chave v2.
#[derive(Clone, Debug, PartialEq)]
pub struct ArestaDaFace {
    pub ponto: [f64; 3],
    pub chave: Option<EdgeKey>,
}

/// Todas as arestas do contorno da face `face` do corpo (mesma ordem de índices do picking).
pub fn arestas_da_face(b: &BodyResult, face: usize) -> Vec<ArestaDaFace> {
    let m = &b.mesh;
    let f = face as u32;
    let mut out = vec![];
    for (e, par) in m.edge_faces.iter().enumerate() {
        let [p, q] = *par;
        if (p != f && q != f) || p == q {
            continue;
        }
        let Some(v) = m.edge_mids.get(e * 3..e * 3 + 3) else { continue };
        if v.iter().all(|x| x.abs() < 1e-9) {
            continue;
        }
        out.push(ArestaDaFace { ponto: [v[0] as f64, v[1] as f64, v[2] as f64], chave: b.edge_key(e) });
    }
    out
}

fn perto(a: &[f64; 3], b: &[f64; 3]) -> bool {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt() < 0.01
}

/// Clique na face: se todas as arestas dela já estão no comando, tira todas; senão soma as que
/// faltam. As chaves e as inversões por aresta acompanham a lista. Devolve quantas arestas o
/// comando tem depois (0 se a seleção não é por arestas).
pub fn alternar_face(o: &mut EdgeOp, novas: &[ArestaDaFace]) -> usize {
    let EdgeSel::Edges(list) = &mut o.edges else { return 0 };
    if novas.is_empty() {
        return list.len();
    }
    let antes = list.clone();
    let todas = novas.iter().all(|n| list.iter().any(|q| perto(q, &n.ponto)));
    if todas {
        list.retain(|q| !novas.iter().any(|n| perto(q, &n.ponto)));
    } else {
        for n in novas {
            if !list.iter().any(|q| perto(q, &n.ponto)) {
                list.push(n.ponto);
            }
        }
    }
    let mut chaves = carry_keys(&antes, &o.keys, list);
    for (i, p) in list.iter().enumerate() {
        if chaves[i].is_none() {
            chaves[i] = novas.iter().find(|n| perto(p, &n.ponto)).and_then(|n| n.chave.clone());
        }
    }
    o.keys = chaves;
    o.extra.carry_flips(&antes, list);
    list.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::*;
    use fenkra_solidos::{recompute, Status};

    fn s(x: &str) -> String {
        x.to_string()
    }

    /// Cubo de 20 mm, de z = 0 a 20.
    fn cubo() -> Document {
        let mut d = Document::new("cubo");
        let mut data = SketchData::default();
        let mut n = 7_100_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, [-10., -10.], [10., 10.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("20")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
        d
    }

    fn volume_e_caixa(d: &Document) -> (f64, [f64; 3], [f64; 3], Status) {
        let r = recompute(d);
        let v = r.bodies.iter().map(|b| b.mass.volume).sum();
        let (lo, hi) = fenkra_solidos::final_shape(d).and_then(|s| s.bbox()).unwrap_or(([0.; 3], [0.; 3]));
        let st = r.status.values().find(|s| matches!(s, Status::Error(_))).cloned().unwrap_or(Status::Ok);
        (v, lo, hi, st)
    }

    fn face_de_cima(b: &BodyResult) -> usize {
        (0..b.mesh.face_info.len()).find(|i| b.mesh.face_info[*i].dir[2] > 0.9 && (b.mesh.face_info[*i].point[2] - 20.).abs() < 1e-6).expect("face de cima")
    }

    #[test]
    fn clicar_na_face_de_cima_filete_as_quatro_arestas_e_clicar_de_novo_tira() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = cubo();
        let (v0, lo0, hi0, _) = volume_e_caixa(&d);
        assert!((v0 - 8000.).abs() < 1e-3, "{v0}");
        let r = recompute(&d);
        let corpo = &r.bodies[0];
        let novas = arestas_da_face(corpo, face_de_cima(corpo));
        assert_eq!(novas.len(), 4, "o contorno da face de cima tem 4 arestas");
        assert!(novas.iter().all(|n| n.chave.is_some()), "cada aresta tem as duas faces vizinhas nomeadas");

        let mut op = EdgeOp { value: s("2"), edges: EdgeSel::Edges(vec![]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() };
        assert_eq!(alternar_face(&mut op, &novas), 4);
        assert_eq!(op.keys.len(), 4, "as chaves ficam paralelas às arestas");
        assert!(op.keys.iter().all(|k| k.is_some()));
        let id = d.add_feature(FeatureKind::Fillet(op.clone()));
        let (v, lo, hi, st) = volume_e_caixa(&d);
        assert_eq!(st, Status::Ok);
        // cada aresta de 20 mm perde (1 - pi/4) r^2 = 0,858 mm2 de seção: ~68,7 mm3 no total, menos a
        // sobreposição pequena dos cantos
        assert!(v < 8000. - 60. && v > 8000. - 70., "volume {v}");
        for k in 0..3 {
            assert!((lo[k] - lo0[k]).abs() < 1e-3 && (hi[k] - hi0[k]).abs() < 1e-3, "a caixa envolvente não muda");
        }

        // clicar de novo na mesma face tira as 4
        let FeatureKind::Fillet(o) = &mut d.feature_mut(id).unwrap().kind else { panic!() };
        assert_eq!(alternar_face(o, &novas), 0);
        assert!(o.keys.is_empty());
    }

    #[test]
    fn face_parcial_soma_so_as_que_faltam_e_mantem_chaves_e_inversoes() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = cubo();
        let r = recompute(&d);
        let corpo = &r.bodies[0];
        let novas = arestas_da_face(corpo, face_de_cima(corpo));
        let mut op = EdgeOp { value: s("2"), edges: EdgeSel::Edges(vec![novas[0].ponto]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![novas[0].chave.clone()], extra: EdgeExtra { flips: vec![true], ..Default::default() } };
        assert_eq!(alternar_face(&mut op, &novas), 4, "uma já estava: soma as 3 que faltam");
        assert_eq!(op.keys.len(), 4);
        assert_eq!(op.extra.flips, vec![true, false, false, false], "a inversão da primeira acompanha a aresta");
        assert_eq!(op.keys[0], novas[0].chave);
    }

    #[test]
    fn remover_pela_caixa_mantem_chaves_e_inversoes_alinhadas() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = cubo();
        let r = recompute(&d);
        let corpo = &r.bodies[0];
        let novas = arestas_da_face(corpo, face_de_cima(corpo));
        let mut op = EdgeOp { value: s("2"), edges: EdgeSel::Edges(vec![]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: EdgeExtra { flips: vec![false, true, false, true], ..Default::default() } };
        alternar_face(&mut op, &novas);
        op.extra.flips = vec![false, true, false, true];
        // o mesmo caminho do painel: tira os itens 1 e 2 da lista e leva chaves e inversões junto
        let EdgeSel::Edges(list) = &mut op.edges else { panic!() };
        let antes = list.clone();
        let mut marcas = crate::caixa_selecao::Marcas::default();
        marcas.clicar(1, false, false);
        marcas.clicar(2, false, true);
        crate::caixa_selecao::remover_marcados(list, &mut marcas, 0, None);
        let nova = list.clone();
        op.keys = carry_keys(&antes, &op.keys, &nova);
        op.extra.carry_flips(&antes, &nova);
        assert_eq!(nova.len(), 2);
        assert_eq!(op.keys, vec![novas[0].chave.clone(), novas[3].chave.clone()]);
        assert_eq!(op.extra.flips, vec![false, true]);
    }
}
