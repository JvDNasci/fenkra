// SPDX-License-Identifier: LGPL-2.1-or-later
//! Alinhar à vista numa face (B544): a tecla N olha de frente para a face escolhida.
//!
//! Como funciona no kernel: cada face guarda uma assinatura (`FaceInfo`) com o tipo da superfície, um
//! vetor e um ponto SOBRE a face. Num plano o vetor é a normal externa, então olhar de frente é pôr o
//! olho nessa direção. Numa superfície curva (cilindro, cone, esfera, toro, livre) não existe uma
//! normal só: ela muda de ponto a ponto. Por isso olha-se pela normal no ponto clicado (a do triângulo
//! da malha que o raio acertou, já virada para o lado de quem olha), e no cilindro, cone e toro há a
//! segunda opção de olhar pelo eixo (o vetor da assinatura), que mostra o furo ou a peça de ponta.
//!
//! O sentido: no plano a normal externa pode ficar de costas para a pessoa (face vista por dentro, ou
//! escolhida na árvore sem clique). Havendo normal do clique, o sinal acompanha o lado que ela via;
//! no eixo, o sinal que menos gira a câmera em relação ao olhar atual.

use glam::Vec3;

/// Tipo de superfície de `FaceInfo::kind`: 0 plano, 1 cilindro, 2 cone, 3 esfera, 4 toro, 5 livre.
pub const PLANO: u8 = 0;

/// O que a face escolhida oferece para o alinhamento.
#[derive(Clone, Copy, Debug)]
pub struct FaceAlvo {
    pub tipo: u8,
    /// normal externa (plano) ou eixo (cilindro, cone, toro)
    pub vetor: Vec3,
    /// ponto sobre a face (assinatura do kernel)
    pub ponto_face: Vec3,
    /// ponto clicado na face
    pub clique: Vec3,
    /// normal da malha no clique, virada para quem olhava; zero se a face veio da árvore
    pub normal_clique: Vec3,
}

/// Como olhar uma face curva.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModoCurva {
    /// pela normal no ponto clicado
    NormalNoPonto,
    /// ao longo do eixo da superfície
    Eixo,
}

/// Faces que têm eixo para olhar de ponta.
pub fn tem_eixo(tipo: u8) -> bool {
    matches!(tipo, 1 | 2 | 4)
}

/// Direção do olho (do centro para a câmera) e ponto que vai para o centro da vista.
/// `olhar_atual` é a direção do centro para a câmera hoje. `None` quando a face não dá direção
/// (curva sem clique e sem eixo, ou vetor nulo).
pub fn alinhar(f: &FaceAlvo, modo: ModoCurva, olhar_atual: Vec3) -> Option<(Vec3, Vec3)> {
    let unit = |v: Vec3| v.try_normalize();
    if f.tipo == PLANO {
        let n = unit(f.vetor).or_else(|| unit(f.normal_clique))?;
        // o lado que a pessoa via vence a orientação da face
        let n = match unit(f.normal_clique) {
            Some(c) if n.dot(c) < 0. => -n,
            _ => n,
        };
        return Some((n, f.ponto_face));
    }
    if modo == ModoCurva::Eixo && tem_eixo(f.tipo) {
        let a = unit(f.vetor)?;
        let a = if a.dot(olhar_atual) < 0. { -a } else { a };
        return Some((a, centro_curvo(f)));
    }
    let n = unit(f.normal_clique)?;
    Some((n, centro_curvo(f)))
}

/// Numa face curva o centro da vista é o ponto clicado (o centro da assinatura pode estar no vazio,
/// como no meio de um furo).
fn centro_curvo(f: &FaceAlvo) -> Vec3 {
    if f.normal_clique != Vec3::ZERO {
        f.clique
    } else {
        f.ponto_face
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::render::Camera;

    fn plano(n: Vec3, clique: Vec3) -> FaceAlvo {
        FaceAlvo { tipo: PLANO, vetor: n, ponto_face: Vec3::new(5., 5., 20.), clique: Vec3::new(1., 1., 20.), normal_clique: clique }
    }

    #[test]
    fn plano_olha_pela_normal_externa() {
        let f = plano(Vec3::Z, Vec3::ZERO);
        let (n, c) = alinhar(&f, ModoCurva::NormalNoPonto, Vec3::X).unwrap();
        assert_eq!(n, Vec3::Z);
        assert_eq!(c, f.ponto_face);
    }

    #[test]
    fn plano_visto_por_dentro_olha_do_lado_que_a_pessoa_via() {
        let f = plano(Vec3::Z, -Vec3::Z);
        assert_eq!(alinhar(&f, ModoCurva::NormalNoPonto, Vec3::X).unwrap().0, -Vec3::Z);
        // vetor nulo (assinatura degenerada): cai na normal do clique
        let f = plano(Vec3::ZERO, Vec3::Y);
        assert_eq!(alinhar(&f, ModoCurva::NormalNoPonto, Vec3::X).unwrap().0, Vec3::Y);
        assert!(alinhar(&plano(Vec3::ZERO, Vec3::ZERO), ModoCurva::NormalNoPonto, Vec3::X).is_none());
    }

    fn cilindro() -> FaceAlvo {
        // cilindro de eixo Z; clique na lateral voltada para +X, na altura 7
        FaceAlvo { tipo: 1, vetor: Vec3::Z, ponto_face: Vec3::new(0., 0., 10.), clique: Vec3::new(8., 0., 7.), normal_clique: Vec3::X }
    }

    #[test]
    fn face_curva_olha_pela_normal_no_ponto_clicado_e_centra_no_clique() {
        let (n, c) = alinhar(&cilindro(), ModoCurva::NormalNoPonto, Vec3::Y).unwrap();
        assert_eq!(n, Vec3::X);
        assert_eq!(c, Vec3::new(8., 0., 7.));
        // outro ponto do mesmo cilindro dá outra direção
        let outro = FaceAlvo { normal_clique: Vec3::Y, clique: Vec3::new(0., 8., 7.), ..cilindro() };
        assert_eq!(alinhar(&outro, ModoCurva::NormalNoPonto, Vec3::Y).unwrap().0, Vec3::Y);
    }

    #[test]
    fn curva_sem_clique_nao_inventa_direcao_mas_o_eixo_serve() {
        let f = FaceAlvo { normal_clique: Vec3::ZERO, ..cilindro() };
        assert!(alinhar(&f, ModoCurva::NormalNoPonto, Vec3::Y).is_none());
        let (a, c) = alinhar(&f, ModoCurva::Eixo, Vec3::Y).unwrap();
        assert_eq!(a, Vec3::Z);
        assert_eq!(c, f.ponto_face);
    }

    #[test]
    fn eixo_escolhe_o_sentido_que_menos_gira_e_esfera_nao_tem_eixo() {
        let (a, _) = alinhar(&cilindro(), ModoCurva::Eixo, Vec3::new(1., 0., -1.)).unwrap();
        assert_eq!(a, -Vec3::Z);
        let (a, _) = alinhar(&cilindro(), ModoCurva::Eixo, Vec3::new(1., 0., 1.)).unwrap();
        assert_eq!(a, Vec3::Z);
        let esfera = FaceAlvo { tipo: 3, ..cilindro() };
        assert!(!tem_eixo(3) && tem_eixo(1) && tem_eixo(2) && tem_eixo(4));
        // pedir eixo numa esfera cai na normal do clique
        assert_eq!(alinhar(&esfera, ModoCurva::Eixo, Vec3::Y).unwrap().0, Vec3::X);
    }

    #[test]
    fn a_camera_olha_de_frente_para_a_face() {
        // topo (+Z), fundo (-Z), lateral (+X) e uma face inclinada
        for n in [Vec3::Z, -Vec3::Z, Vec3::X, Vec3::new(1., 1., 1.).normalize()] {
            let f = plano(n, Vec3::ZERO);
            let (dir, centro) = alinhar(&f, ModoCurva::NormalNoPonto, Vec3::X).unwrap();
            let mut cam = Camera::default();
            cam.look_along(dir, centro);
            let para_o_olho = (cam.eye() - cam.target).normalize();
            assert!((para_o_olho - n).length() < 1e-3, "{n:?}: olho em {para_o_olho:?}");
            assert_eq!(cam.target, centro);
        }
    }

    #[test]
    fn face_do_kernel_tem_a_normal_externa_e_o_cilindro_o_eixo() {
        use fenkra_core::{BoolOp, Document, Extent, Extrude, FeatureKind, OriginPlane, PlaneRef, Sketch, SketchData};
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = Document::new("cilindro");
        let mut n = 92_000_000u64;
        let mut al = move || {
            n += 1;
            fenkra_core::Id(n)
        };
        let mut data = SketchData::default();
        let c = data.add_point(&mut al, 0., 0.);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Circle { c, r: 10. }, false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("30".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let r = fenkra_solidos::recompute(&d);
        let m = &r.bodies.first().expect("um corpo").mesh;
        let alvo = |i: usize| {
            let f = &m.face_info[i];
            FaceAlvo { tipo: f.kind, vetor: Vec3::from(f.dir.map(|v| v as f32)), ponto_face: Vec3::from(f.point.map(|v| v as f32)), clique: Vec3::ZERO, normal_clique: Vec3::ZERO }
        };
        let topo = (0..m.face_info.len()).find(|i| m.face_info[*i].kind == PLANO && m.face_info[*i].point[2] > 29.).expect("tampa de cima");
        let (dir, _) = alinhar(&alvo(topo), ModoCurva::NormalNoPonto, Vec3::X).unwrap();
        assert!((dir - Vec3::Z).length() < 1e-4, "{dir:?}");
        let fundo = (0..m.face_info.len()).find(|i| m.face_info[*i].kind == PLANO && m.face_info[*i].point[2] < 1.).expect("tampa de baixo");
        let (dir, _) = alinhar(&alvo(fundo), ModoCurva::NormalNoPonto, Vec3::X).unwrap();
        assert!((dir + Vec3::Z).length() < 1e-4, "a tampa de baixo olha para -Z: {dir:?}");
        let lado = (0..m.face_info.len()).find(|i| m.face_info[*i].kind == 1).expect("parede");
        let (eixo, _) = alinhar(&alvo(lado), ModoCurva::Eixo, Vec3::new(1., 0., 1.)).unwrap();
        assert!((eixo - Vec3::Z).length() < 1e-4, "{eixo:?}");
    }
}
