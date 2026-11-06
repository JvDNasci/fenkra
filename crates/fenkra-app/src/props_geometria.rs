// SPDX-License-Identifier: LGPL-2.1-or-later
//! Propriedades de geometria do corpo (B336): faces, arestas, vértices, triângulos da malha e
//! caixa envolvente. Antes isso era um texto no canto da viewport; agora vive nas propriedades do
//! objeto, ao lado das propriedades de massa. Nada mais do aplicativo lia aquele texto: era só
//! desenho sobre a viewport.

use fenkra_solidos::BodyResult;

#[derive(Clone, Debug, PartialEq)]
pub struct Geometria {
    pub faces: u32,
    pub arestas: usize,
    pub vertices: usize,
    pub triangulos: usize,
    /// caixa envolvente da malha (mín, máx), em mm; `None` se o corpo não tem malha
    pub caixa: Option<([f32; 3], [f32; 3])>,
}

impl Geometria {
    pub fn de(b: &BodyResult) -> Geometria {
        let m = &b.mesh;
        let mut caixa: Option<([f32; 3], [f32; 3])> = None;
        for p in m.positions.chunks_exact(3) {
            let (lo, hi) = caixa.get_or_insert(([p[0], p[1], p[2]], [p[0], p[1], p[2]]));
            for i in 0..3 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
        Geometria { faces: b.faces, arestas: m.edge_ranges.len(), vertices: m.vertices.len() / 3, triangulos: m.indices.len() / 3, caixa }
    }

    /// Dimensões da caixa envolvente (x, y, z).
    pub fn dimensoes(&self) -> Option<[f32; 3]> {
        self.caixa.map(|(lo, hi)| [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]])
    }
}

pub static TEXTOS: &[(&str, &str)] = &[
    ("Propriedades de geometria", "Geometry properties"),
    ("faces", "faces"),
    ("malha", "mesh"),
    ("arestas", "edges"),
    ("vértices", "vertices"),
    ("triângulos da malha", "mesh triangles"),
    ("caixa envolvente", "bounding box"),
    ("Contagens do corpo calculado; a caixa envolve a malha de exibição.", "Counts of the computed body; the box wraps the display mesh."),
];

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn caixa_conta_e_volume_do_exemplo_batem_com_o_recalculo() {
        let r = fenkra_solidos::recompute(&fenkra_solidos::example_document());
        if !fenkra_kernel::AVAILABLE || r.bodies.is_empty() {
            return;
        }
        let b = &r.bodies[0];
        let g = Geometria::de(b);
        assert_eq!(g.faces, b.faces);
        assert!(g.faces > 0 && g.arestas > 0 && g.vertices > 0 && g.triangulos > 0);
        let d = g.dimensoes().expect("corpo com malha tem caixa");
        assert!(d.iter().all(|x| *x > 0.), "{d:?}");
        // a caixa contém todo vértice B-Rep
        let (lo, hi) = g.caixa.unwrap();
        for v in b.mesh.vertices.chunks_exact(3) {
            for i in 0..3 {
                assert!(v[i] >= lo[i] - 0.5 && v[i] <= hi[i] + 0.5);
            }
        }
        assert!(b.mass.volume > 0.);
    }

    #[test]
    fn corpo_vazio_nao_tem_caixa() {
        let g = Geometria { faces: 0, arestas: 0, vertices: 0, triangulos: 0, caixa: None };
        assert!(g.dimensoes().is_none());
    }

    #[test]
    fn textos_novos_tem_traducao() {
        for (pt, en) in TEXTOS {
            assert!(!pt.is_empty() && !en.is_empty());
        }
    }
}
