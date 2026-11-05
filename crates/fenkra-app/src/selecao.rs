// SPDX-License-Identifier: LGPL-2.1-or-later
//! "Selecionar outro" (B95): a roda com Alt percorre o que está sob o ponteiro, do mais próximo ao
//! mais distante, para alcançar uma face escondida atrás de outra sem girar a vista.
//!
//! A lógica é pura (sem egui além de `Pos2`) para ser testada: o picking entrega a pilha de
//! candidatos e esta máquina de estados decide qual deles está em foco.

use crate::picking::Pick;
use eframe::egui::Pos2;

/// Passo da roda, em pixels de rolagem, que vale um candidato.
const PASSO_RODA: f32 = 24.;
/// O ponteiro pode andar até aqui sem reiniciar o ciclo.
const TOLERANCIA_PX: f32 = 4.;

#[derive(Default)]
pub struct Estado {
    acumulado: f32,
    ancora: Option<Pos2>,
    pilha: Vec<Pick>,
    indice: usize,
    ativo: bool,
}

impl Estado {
    /// Soma a rolagem do quadro e devolve quantos candidatos avançar (negativo = voltar).
    /// A rolagem suave do egui chega aos poucos; o acumulador garante um passo por "clique" da roda.
    pub fn roda(&mut self, delta: f32) -> i32 {
        self.acumulado += delta;
        let passos = (self.acumulado / PASSO_RODA).trunc();
        self.acumulado -= passos * PASSO_RODA;
        passos as i32
    }

    /// O candidato em foco, se o ciclo está ativo no ponto `pos`; senão `None` e o picking normal vale.
    /// `passo != 0` avança o ciclo; `pilha` só é montada quando um ciclo novo começa.
    pub fn escolher(&mut self, pos: Pos2, passo: i32, pilha: impl FnOnce() -> Vec<Pick>) -> Option<Pick> {
        if self.ancora.is_some_and(|a| (a - pos).length() > TOLERANCIA_PX) {
            self.reiniciar();
        }
        if passo != 0 {
            if !self.ativo {
                self.pilha = pilha();
                self.indice = 0;
                self.ancora = Some(pos);
                self.ativo = true;
            }
            let n = self.pilha.len() as i32;
            if n > 0 {
                self.indice = (self.indice as i32 + passo).rem_euclid(n) as usize;
            }
        }
        if self.ativo { self.pilha.get(self.indice).copied() } else { None }
    }

    /// Candidato atual e quantos existem (para a etiqueta "2 de 3").
    pub fn posicao(&self) -> Option<(usize, usize)> {
        (self.ativo && !self.pilha.is_empty()).then_some((self.indice + 1, self.pilha.len()))
    }

    pub fn reiniciar(&mut self) {
        self.ancora = None;
        self.pilha.clear();
        self.indice = 0;
        self.ativo = false;
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::picking::{pilha_de_faces, Filter};
    use fenkra_core::{BoolOp, Document, Extent, Extrude, FeatureKind, OriginPlane, PlaneRef, Sketch, SketchData};

    fn face(k: usize) -> Pick {
        Pick::Face { body: 0, face: k, owner: fenkra_core::Id(1), point: [0.; 3], normal: [0., 0., 1.] }
    }

    #[test]
    fn roda_acumula_ate_um_passo() {
        let mut e = Estado::default();
        assert_eq!(e.roda(10.), 0);
        assert_eq!(e.roda(10.), 0);
        assert_eq!(e.roda(10.), 1, "30 px = um passo, sobram 6");
        assert_eq!(e.roda(-60.), -2);
    }

    #[test]
    fn ciclo_percorre_a_pilha_e_da_a_volta() {
        let mut e = Estado::default();
        let p = Pos2::new(100., 100.);
        let pilha = || vec![face(0), face(1), face(2)];
        assert_eq!(e.escolher(p, 0, pilha), None, "sem roda vale o picking normal");
        assert_eq!(e.escolher(p, 1, pilha), Some(face(1)), "primeiro passo vai para o segundo candidato");
        assert_eq!(e.posicao(), Some((2, 3)));
        assert_eq!(e.escolher(p, 1, || panic!("a pilha não é refeita no meio do ciclo")), Some(face(2)));
        assert_eq!(e.escolher(p, 1, || panic!()), Some(face(0)), "dá a volta");
        assert_eq!(e.escolher(p, -1, || panic!()), Some(face(2)), "a roda para o outro lado volta");
    }

    #[test]
    fn mover_o_ponteiro_reinicia_o_ciclo() {
        let mut e = Estado::default();
        let p = Pos2::new(100., 100.);
        e.escolher(p, 1, || vec![face(0), face(1)]);
        assert_eq!(e.escolher(Pos2::new(102., 101.), 0, || panic!()), Some(face(1)), "dentro da tolerância continua");
        assert_eq!(e.escolher(Pos2::new(140., 100.), 0, || panic!()), None, "andou: volta ao normal");
        assert_eq!(e.posicao(), None);
    }

    /// Um cubo visto de frente tem duas faces sob o ponteiro: a da frente e a de trás.
    #[test]
    fn pilha_traz_a_face_da_frente_e_a_de_tras_em_ordem() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = Document::new("cubo");
        let mut data = SketchData::default();
        let mut n = 90_000_000u64;
        let mut al = move || {
            n += 1;
            fenkra_core::Id(n)
        };
        data.add_rect(&mut al, [0., 0.], [20., 20.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("20".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let r = fenkra_solidos::recompute(&d);
        let rect = eframe::egui::Rect::from_min_size(Pos2::new(0., 0.), eframe::egui::vec2(800., 600.));
        let cam = crate::render::Camera { target: glam::Vec3::new(10., 10., 10.), yaw: 0., pitch: 1.5, dist: 200., ortho: true };
        let pos = cam.project(rect, glam::Vec3::new(10., 10., 10.)).expect("na tela");
        let f = Filter { faces: true, edges: false, vertices: false, planes: false, hidden_env: false, exclude_sketch: None, fantasma: None };
        let pilha = pilha_de_faces(&r, &d, &cam, rect, pos, &f);
        assert_eq!(pilha.len(), 2, "{pilha:?}");
        let z = |p: &Pick| match p {
            Pick::Face { point, .. } => point[2],
            _ => f32::NAN,
        };
        assert!((z(&pilha[0]) - 20.).abs() < 0.1 && z(&pilha[1]).abs() < 0.1, "topo antes do fundo: {pilha:?}");
        // e com a pick normal a primeira da pilha é a mesma face
        let normal = crate::picking::pick(&r, &d, &cam, rect, pos, &f);
        assert_eq!(normal.map(|p| z(&p).round()), Some(20.));
    }
}
