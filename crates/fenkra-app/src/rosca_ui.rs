// SPDX-License-Identifier: LGPL-2.1-or-later
//! B351: a rosca cosmética (e a prévia de uma rosca modelada) aparece na face como textura, sem
//! malha de rosca. As faces roscadas são achadas pela geometria da malha de trabalho: o triângulo cujos
//! três vértices estão a um raio da rosca do eixo dela, com a normal perpendicular ao eixo, é parede
//! cilíndrica. Cada um é repetido na camada de fantasma com o eixo, a origem e o passo nos atributos do
//! vértice (nada de buffer novo na placa de vídeo); o `fs_ghost` pinta o filete helicoidal por pixel.
//!
//! Atributos do vértice da textura: `nrm` = eixo × comprimento da rosca; `mat.xyz` = origem; `mat.w` =
//! passo (positivo à direita, negativo à esquerda); `col` = cor do tema e opacidade.

use crate::render::MeshVertex;
use fenkra_kernel::Mesh;
use fenkra_solidos::RoscaViz;

/// Folga do raio para o vértice contar como estando na parede (mm), mais 0,2 % do raio.
const FOLGA_DO_RAIO: f32 = 0.02;
/// Quanto a normal pode se afastar de perpendicular ao eixo (cosseno) numa parede cilíndrica.
const FOLGA_DA_NORMAL: f32 = 0.2;
/// Opacidade do filete.
const OPACIDADE: f32 = 0.9;

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Triângulos (índices de vértice) da malha que são parede da rosca.
pub fn triangulos_da_parede(m: &Mesh, x: &RoscaViz) -> Vec<[u32; 3]> {
    let o = x.origin.map(|v| v as f32);
    let eixo = x.dir.map(|v| v as f32);
    let (raio, comp) = (x.radius as f32, x.len as f32);
    let tol = FOLGA_DO_RAIO + 0.002 * raio;
    let eixo_h = |i: u32| {
        let k = i as usize * 3;
        let p = [m.positions[k] - o[0], m.positions[k + 1] - o[1], m.positions[k + 2] - o[2]];
        let h = dot(p, eixo);
        let rad = (dot(p, p) - h * h).max(0.).sqrt();
        (h, rad, k)
    };
    let mut saida = vec![];
    for t in m.indices.chunks_exact(3) {
        let mut faixa = (f32::MAX, f32::MIN);
        let mut dentro = true;
        for &i in t {
            let (h, rad, k) = eixo_h(i);
            // a normal de uma parede cilíndrica é perpendicular ao eixo (descarta fundo e escareado, cujos
            // vértices também ficam no raio da boca)
            let n = [m.normals[k], m.normals[k + 1], m.normals[k + 2]];
            if (rad - raio).abs() > tol || dot(n, eixo).abs() > FOLGA_DA_NORMAL {
                dentro = false;
                break;
            }
            faixa = (faixa.0.min(h), faixa.1.max(h));
        }
        if dentro && faixa.0 <= comp + tol && faixa.1 >= -tol {
            saida.push([t[0], t[1], t[2]]);
        }
    }
    saida
}

/// Põe a textura de cada rosca sem sulco real sobre as paredes dela. `cor` vem do tema.
pub fn texturas(roscas: &[&RoscaViz], m: &Mesh, cor: [f32; 3], dst: &mut Vec<MeshVertex>) {
    for x in roscas.iter().filter(|x| !x.modelada && x.pitch > 1e-9 && x.len > 1e-9) {
        let eixo = x.dir.map(|v| v as f32);
        let nrm = eixo.map(|v| v * x.len as f32);
        let passo = x.pitch as f32 * if x.esquerda { -1. } else { 1. };
        let mat = [x.origin[0] as f32, x.origin[1] as f32, x.origin[2] as f32, passo];
        for t in triangulos_da_parede(m, x) {
            for i in t {
                let k = i as usize * 3;
                dst.push(MeshVertex { pos: [m.positions[k], m.positions[k + 1], m.positions[k + 2]], nrm, col: [cor[0], cor[1], cor[2], OPACIDADE], mat });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::*;

    fn s(x: &str) -> String {
        x.into()
    }

    /// Bloco com um furo cosmético M8 escareado, cego, de 20 mm.
    fn peca(esquerda: bool) -> (Document, Id) {
        let mut d = Document::new("rosca");
        let mut data = SketchData::default();
        let mut n = 100u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, [-20., -15.], [20., 15.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("30")), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30");
        let mut pts = SketchData::default();
        let p = pts.add_point(&mut al, 0., 0.);
        pts.add_curve(&mut al, fenkra_core::sketch::Curve::Point { p }, false);
        let sp = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: pts }));
        let id = d.add_feature(FeatureKind::Hole(Hole {
            sketch: sp,
            kind: HoleKind::Escareado { diameter: s("16"), angle: s("90") },
            diameter: s("6.8"),
            depth: Extent::Blind(s("20")),
            thread: Some(s("M8x1.25")),
            thread_modeled: false,
            standard: None,
            points: vec![],
            dir: None,
            reverse: false,
            tip: None,
            taper: s("0"),
            left_hand: esquerda,
            extra: Default::default(),
        }));
        (d, id)
    }

    /// A face roscada é achada só pela geometria: parede do furo sim; fundo, cone do escareado e topo não.
    #[test]
    fn so_a_parede_do_furo_leva_a_textura() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (d, id) = peca(false);
        let r = fenkra_solidos::recompute(&d);
        let x = r.roscas.iter().find(|x| x.feature == id).expect("rosca registrada");
        let m = &r.bodies[0].mesh;
        let tris = triangulos_da_parede(m, x);
        assert!(!tris.is_empty(), "nenhum triângulo da parede");
        for t in &tris {
            for &i in t {
                let k = i as usize * 3;
                let (px, py, pz) = (m.positions[k], m.positions[k + 1], m.positions[k + 2]);
                assert!(((px * px + py * py).sqrt() - 3.4).abs() < 0.05, "vértice fora da parede do furo: {px},{py},{pz}");
                assert!(pz < 25.4 + 0.05 && pz > 10. - 0.05, "fora da parte cilíndrica: z = {pz}");
            }
        }
        let mut v = vec![];
        texturas(&[x], m, [0.2, 0.4, 0.6], &mut v);
        assert_eq!(v.len(), tris.len() * 3);
        // o passo e o eixo certos seguem no vértice; o comprimento vai no módulo do eixo
        assert!((v[0].mat[3] - 1.25).abs() < 1e-6);
        let comp = (v[0].nrm[0].powi(2) + v[0].nrm[1].powi(2) + v[0].nrm[2].powi(2)).sqrt();
        assert!((comp as f64 - x.len).abs() < 1e-4);
        assert!(v.iter().all(|q| q.col[3] < 1. && q.col[3] > 0.));
    }

    #[test]
    fn rosca_a_esquerda_inverte_o_sinal_do_passo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (d, id) = peca(true);
        let r = fenkra_solidos::recompute(&d);
        let x = r.roscas.iter().find(|x| x.feature == id).expect("rosca registrada");
        let mut v = vec![];
        texturas(&[x], &r.bodies[0].mesh, [1., 1., 1.], &mut v);
        assert!(!v.is_empty() && (v[0].mat[3] + 1.25).abs() < 1e-6);
    }

    /// Rosca com sulco real não leva textura: a geometria já mostra o filete.
    #[test]
    fn rosca_modelada_nao_leva_textura() {
        let x = RoscaViz { feature: Id(1), origin: [0.; 3], dir: [0., 0., 1.], radius: 4., len: 10., pitch: 1.25, esquerda: false, interna: true, modelada: true };
        let m = Mesh::default();
        let mut v = vec![];
        texturas(&[&x], &m, [1.; 3], &mut v);
        assert!(v.is_empty());
    }
}
