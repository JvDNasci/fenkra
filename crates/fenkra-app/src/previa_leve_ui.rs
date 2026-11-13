// SPDX-License-Identifier: LGPL-2.1-or-later
//! B356: desenho da prévia leve dos comandos pesados (ADR 0075). Com o comando aberto o cálculo não
//! refaz filete, chanfro, casca nem padrão: devolve `PreviaViz` e esta camada desenha só linhas, na
//! cor do tema, no lugar e no tamanho que o resultado final terá:
//!
//! - filete e chanfro: duas linhas paralelas à aresta, uma em cada face vizinha, na distância em que
//!   a superfície nova encosta na face (o chanfro usa a distância; o filete, `r / tan(a / 2)`, com `a`
//!   o ângulo entre as faces);
//! - casca: a caixa da cavidade, recuada da caixa do corpo pela espessura;
//! - padrão e espelho: a caixa da ferramenta em cada instância;
//! - filete face a face e completo: as linhas de tangência na aresta comum às faces clicadas;
//! - chapa e estrutural: a zona de cada comando e cada membro como a caixa da seção ao longo da guia.
//!
//! Sem malha nova e sem buffer novo na placa de vídeo; a geometria real só nasce ao concluir.

use crate::previa::{cor_linha, Fantasma};
use crate::render::{LineVertex, Scene};
use fenkra_kernel::Mesh;
use fenkra_solidos::previa_leve::{PreviaViz, ARESTAS_DA_CAIXA};
use fenkra_solidos::RecomputeResult;
use glam::Vec3;

/// Distância máxima (mm) entre o ponto médio guardado e a aresta da malha para contar como a mesma.
const SOBRE_A_ARESTA: f32 = 0.5;

/// Desenha a prévia leve do comando `f.id`, se o último cálculo trouxe uma.
pub fn desenhar(r: &RecomputeResult, f: &Fantasma, alvo: fenkra_core::Id, s: &mut Scene) {
    let col = cor_linha(f.cor);
    let aresta = cor_linha(f.entrada);
    for v in r.previas.iter().filter(|x| x.feature == f.id) {
        for c in &v.caixas {
            for [a, b] in ARESTAS_DA_CAIXA {
                s.overlay.push(LineVertex { pos: ponto(c[a]), col });
                s.overlay.push(LineVertex { pos: ponto(c[b]), col });
            }
        }
        if let Some(b) = r.bodies.iter().find(|b| b.collector == alvo) {
            linhas_de_borda(v, &b.mesh, aresta, &mut s.overlay);
        }
    }
}

fn ponto(p: [f64; 3]) -> [f32; 3] {
    p.map(|x| x as f32)
}

/// As linhas de tangência do filete ou do chanfro nas duas faces de cada aresta afetada.
fn linhas_de_borda(v: &PreviaViz, m: &Mesh, col: [f32; 4], dst: &mut Vec<LineVertex>) {
    if v.valor <= 0. || (v.arestas.is_empty() && v.pares.is_empty() && !v.todas) {
        return;
    }
    let nseg = m.edges.len() / 6;
    let mut edges: Vec<usize> = if v.todas {
        (0..m.edge_ranges.len()).filter(|e| m.edge_smooth.get(*e).is_none_or(|x| *x == 0)).collect()
    } else {
        v.arestas.iter().filter_map(|p| crate::entradas::aresta_perto(m, *p)).filter(|(_, d)| *d < SOBRE_A_ARESTA).map(|(e, _)| e).collect()
    };
    // face a face e completo: as arestas entre as duas faces clicadas
    for [pa, pb] in &v.pares {
        let face = |p: [f64; 3]| crate::picking::face_e_distancia(m, p).filter(|x| x.1 < SOBRE_A_ARESTA).map(|x| x.0 as u32);
        let (Some(fa), Some(fb)) = (face(*pa), face(*pb)) else { continue };
        edges.extend(m.edge_faces.iter().enumerate().filter(|(_, ff)| (ff[0] == fa && ff[1] == fb) || (ff[0] == fb && ff[1] == fa)).map(|(e, _)| e));
    }
    for e in edges {
        let Some(&[fa, fb]) = m.edge_faces.get(e) else { continue };
        let s0 = m.edge_ranges[e] as usize;
        let s1 = m.edge_ranges.get(e + 1).map_or(nseg, |x| *x as usize);
        let (Some(ia), Some(ib)) = (m.face_info.get(fa as usize), m.face_info.get(fb as usize)) else { continue };
        for sg in s0..s1 {
            let q = &m.edges[sg * 6..sg * 6 + 6];
            let (a, b) = (Vec3::new(q[0], q[1], q[2]), Vec3::new(q[3], q[4], q[5]));
            let t = (b - a).normalize_or_zero();
            if t == Vec3::ZERO {
                continue;
            }
            let meio = (a + b) / 2.;
            // direção, dentro de cada face plana, que se afasta da aresta
            let para_dentro = |i: &fenkra_kernel::FaceInfo| {
                if i.kind != 0 {
                    return None;
                }
                let n = Vec3::new(i.dir[0] as f32, i.dir[1] as f32, i.dir[2] as f32);
                let d = n.cross(t).normalize_or_zero();
                let ate = Vec3::new(i.point[0] as f32, i.point[1] as f32, i.point[2] as f32) - meio;
                (d != Vec3::ZERO).then(|| if d.dot(ate) < 0. { -d } else { d })
            };
            let (da, db) = (para_dentro(ia), para_dentro(ib));
            // ângulo entre as duas faces vistas da aresta; sem as duas faces planas, o filete não tem fórmula
            let dist = match (v.chanfro, da, db) {
                (true, _, _) => v.valor as f32,
                (false, Some(x), Some(y)) => {
                    let ang = x.dot(y).clamp(-1., 1.).acos();
                    (v.valor as f32 / (ang / 2.).tan().max(1e-3)).min(v.valor as f32 * 50.)
                }
                (false, _, _) => v.valor as f32,
            };
            for d in [da, db].into_iter().flatten() {
                dst.push(LineVertex { pos: (a + d * dist).into(), col });
                dst.push(LineVertex { pos: (b + d * dist).into(), col });
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_kernel::FaceInfo;

    /// Malha mínima: uma aresta em X na origem entre as faces z=0 (corpo em y>0) e y=0 (corpo em z>0).
    fn malha_de_quina() -> Mesh {
        Mesh {
            edges: vec![0., 0., 0., 10., 0., 0.],
            edge_ranges: vec![0],
            edge_smooth: vec![0],
            edge_faces: vec![[0, 1]],
            face_info: vec![FaceInfo { kind: 0, dir: [0., 0., -1.], area: 100., point: [5., 5., 0.] }, FaceInfo { kind: 0, dir: [0., -1., 0.], area: 100., point: [5., 0., 5.] }],
            ..Default::default()
        }
    }

    fn viz(valor: f64, chanfro: bool) -> PreviaViz {
        PreviaViz { feature: fenkra_core::Id(1), arestas: vec![[5., 0., 0.]], valor, chanfro, todas: false, caixas: vec![], pares: vec![] }
    }

    #[test]
    fn filete_em_quina_reta_afasta_o_raio_nas_duas_faces() {
        let mut dst = vec![];
        linhas_de_borda(&viz(2., false), &malha_de_quina(), [1.; 4], &mut dst);
        assert_eq!(dst.len(), 4, "uma linha em cada face");
        // face z=0: a linha corre em y = 2; face y=0: em z = 2
        assert!(dst.iter().take(2).all(|p| (p.pos[1] - 2.).abs() < 1e-4 && p.pos[2].abs() < 1e-4));
        assert!(dst.iter().skip(2).all(|p| (p.pos[2] - 2.).abs() < 1e-4 && p.pos[1].abs() < 1e-4));
    }

    #[test]
    fn chanfro_usa_a_distancia() {
        let mut dst = vec![];
        linhas_de_borda(&viz(3., true), &malha_de_quina(), [1.; 4], &mut dst);
        assert!(dst.iter().take(2).all(|p| (p.pos[1] - 3.).abs() < 1e-4));
    }

    /// Face a face: a aresta sai das duas faces clicadas (malha com os dois triângulos das faces).
    #[test]
    fn face_a_face_acha_a_aresta_entre_as_faces_clicadas() {
        let mut m = malha_de_quina();
        m.positions = vec![0., 0., 0., 10., 0., 0., 10., 10., 0., 0., 0., 10., 10., 0., 10.];
        m.normals = vec![0.; 15];
        m.indices = vec![0, 1, 2, 0, 4, 1, 0, 3, 4];
        m.face_ranges = vec![0, 3];
        let mut v = viz(2., false);
        v.arestas.clear();
        v.pares = vec![[[6., 2., 0.], [6., 0., 4.]]];
        let mut dst = vec![];
        linhas_de_borda(&v, &m, [1.; 4], &mut dst);
        assert_eq!(dst.len(), 4, "a aresta comum às faces clicadas, uma linha em cada face");
    }

    #[test]
    fn aresta_fora_da_malha_nao_desenha() {
        let mut v = viz(2., false);
        v.arestas = vec![[5., 40., 40.]];
        let mut dst = vec![];
        linhas_de_borda(&v, &malha_de_quina(), [1.; 4], &mut dst);
        assert!(dst.is_empty());
    }
}
