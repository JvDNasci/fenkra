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

// ------------------------------------------------------------------------------------------
// Conjunto único (B95): árvore e viewport falam do mesmo conjunto, com as mesmas regras.

/// Como um clique mexe no conjunto: o mesmo para a árvore e para o 3D.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Modo {
    /// clique simples: só este fica
    Trocar,
    /// Shift: acrescenta
    Somar,
    /// Ctrl: acrescenta ou tira
    Alternar,
}

impl Modo {
    pub fn das_teclas(ctrl: bool, shift: bool) -> Modo {
        if ctrl {
            Modo::Alternar
        } else if shift {
            Modo::Somar
        } else {
            Modo::Trocar
        }
    }
}

/// Aplica um clique ao conjunto. `alvo = None` é o clique no vazio: só limpa se não for Shift/Ctrl.
/// Clique simples num item já selecionado o mantém (desmarcar é com Ctrl ou com o vazio), nos dois
/// lugares, para a mão não aprender duas regras.
pub fn aplicar<T: PartialEq>(conjunto: &mut Vec<T>, alvo: Option<T>, modo: Modo) {
    match (alvo, modo) {
        (None, Modo::Trocar) => conjunto.clear(),
        (None, _) => {}
        (Some(a), Modo::Alternar) => match conjunto.iter().position(|x| *x == a) {
            Some(i) => {
                conjunto.remove(i);
            }
            None => conjunto.push(a),
        },
        (Some(a), Modo::Somar) => {
            if !conjunto.contains(&a) {
                conjunto.push(a);
            }
        }
        (Some(a), Modo::Trocar) => {
            conjunto.clear();
            conjunto.push(a);
        }
    }
}

/// O item da árvore que "é dono" do que foi escolhido no 3D: a face aponta o comando que a gerou;
/// sketch, ponto e curva, o próprio sketch; elemento de referência, ele mesmo; aresta e vértice,
/// o coletor do corpo (`coletor_do_corpo` diz qual).
pub fn dono_na_arvore(p: &Pick, coletor_do_corpo: impl Fn(usize) -> Option<fenkra_core::Id>) -> Option<fenkra_core::Id> {
    use fenkra_core::PlaneRef;
    match p {
        Pick::Face { owner, .. } if owner.0 != 0 => Some(*owner),
        Pick::Face { body, .. } | Pick::Edge { body, .. } | Pick::Vertex { body, .. } => coletor_do_corpo(*body),
        Pick::Plane(PlaneRef::Datum(id)) | Pick::Geo(id) => Some(*id),
        Pick::Plane(_) => None,
        Pick::SkCurve { sketch, .. } | Pick::SkPoint { sketch, .. } => Some(*sketch),
        Pick::CurveVtx { curve, .. } => Some(*curve),
    }
}

/// Ids da árvore que a seleção toda ocupa: a seleção múltipla da árvore, o item único e o dono de
/// cada escolha do 3D, sem repetição e na ordem em que aparecem.
pub fn ids_do_conjunto(multi: &[fenkra_core::Id], unico: Option<fenkra_core::Id>, do_3d: impl Iterator<Item = fenkra_core::Id>) -> Vec<fenkra_core::Id> {
    let mut out: Vec<fenkra_core::Id> = vec![];
    for id in multi.iter().copied().chain(unico).chain(do_3d) {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Tipos de subelemento que o comando aberto aceita. Vale só enquanto o comando está aberto e some
/// sozinho ao fechar: nada é gravado nas preferências, então o filtro da barra volta intacto.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TiposTemporarios {
    pub faces: bool,
    pub arestas: bool,
    pub vertices: bool,
}

impl TiposTemporarios {
    /// Texto da etiqueta de status ("Filtro temporário: arestas").
    pub fn nome(self) -> &'static str {
        match (self.faces, self.arestas, self.vertices) {
            (true, false, false) => "faces",
            (false, true, false) => "arestas",
            (false, false, true) => "vértices",
            _ => "faces, arestas e vértices",
        }
    }
}

/// O que o comando aberto precisa escolher: faces (casca, saída, rosca…) ou arestas (filete e
/// chanfro com arestas escolhidas). Os demais comandos usam o filtro da barra.
pub fn tipos_do_comando(kind: &fenkra_core::FeatureKind) -> Option<TiposTemporarios> {
    use fenkra_core::{EdgeSel, FeatureKind};
    if crate::feat_ui::wants_faces(kind) {
        return Some(TiposTemporarios { faces: true, arestas: false, vertices: false });
    }
    match kind {
        FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) if matches!(o.edges, EdgeSel::Edges(_)) => Some(TiposTemporarios { faces: false, arestas: true, vertices: false }),
        _ => None,
    }
}

/// Estado do laço e do espelho do conjunto na cena.
#[derive(Default)]
pub struct Extra {
    /// o arrasto com o esquerdo desenha um laço em vez de uma caixa
    pub laco_ligado: bool,
    pub laco: Vec<Pos2>,
    /// conjunto da árvore que a cena mostrou por último (muda: a cena é refeita)
    pub visto: Vec<fenkra_core::Id>,
    /// "Selecionar outro" pelo menu: avança um candidato no próximo quadro
    pub avancar: i32,
}

impl Extra {
    /// Acrescenta um ponto ao laço se o mouse andou o bastante (evita milhares de pontos iguais).
    pub fn ponto(&mut self, p: Pos2) {
        if self.laco.last().is_none_or(|u| (*u - p).length() >= 3.) {
            self.laco.push(p);
        }
    }
}

/// Ponto dentro do polígono (par-ímpar). O laço se fecha sozinho entre o último e o primeiro ponto.
pub fn ponto_no_poligono(p: Pos2, poli: &[Pos2]) -> bool {
    let mut dentro = false;
    let n = poli.len();
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (poli[i], poli[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            dentro = !dentro;
        }
        j = i;
    }
    dentro
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn poligono_decide_dentro_e_fora_inclusive_em_forma_de_l() {
        let l = [Pos2::new(0., 0.), Pos2::new(20., 0.), Pos2::new(20., 10.), Pos2::new(10., 10.), Pos2::new(10., 30.), Pos2::new(0., 30.)];
        assert!(ponto_no_poligono(Pos2::new(5., 25.), &l), "na haste");
        assert!(ponto_no_poligono(Pos2::new(15., 5.), &l), "no braço");
        assert!(!ponto_no_poligono(Pos2::new(15., 20.), &l), "no vão do L");
        assert!(!ponto_no_poligono(Pos2::new(-1., 5.), &l));
        assert!(!ponto_no_poligono(Pos2::new(5., 5.), &[Pos2::new(0., 0.), Pos2::new(1., 1.)]), "menos de três pontos não fecha região");
    }

    #[test]
    fn mesmas_regras_para_arvore_e_viewport() {
        let mut c = vec![1, 2];
        aplicar(&mut c, Some(3), Modo::Somar);
        assert_eq!(c, vec![1, 2, 3]);
        aplicar(&mut c, Some(3), Modo::Somar);
        assert_eq!(c, vec![1, 2, 3], "Shift não repete");
        aplicar(&mut c, Some(2), Modo::Alternar);
        assert_eq!(c, vec![1, 3], "Ctrl tira o que já estava");
        aplicar(&mut c, Some(9), Modo::Alternar);
        assert_eq!(c, vec![1, 3, 9]);
        aplicar(&mut c, None, Modo::Alternar);
        assert_eq!(c, vec![1, 3, 9], "vazio com Ctrl não limpa");
        aplicar(&mut c, Some(3), Modo::Trocar);
        assert_eq!(c, vec![3], "clique simples fica só com este, mesmo que já estivesse");
        aplicar(&mut c, Some(3), Modo::Trocar);
        assert_eq!(c, vec![3], "e não desmarca ao repetir");
        aplicar(&mut c, None, Modo::Trocar);
        assert!(c.is_empty(), "clicar no vazio limpa");
        assert_eq!(Modo::das_teclas(true, true), Modo::Alternar);
        assert_eq!(Modo::das_teclas(false, true), Modo::Somar);
    }

    #[test]
    fn conjunto_junta_arvore_e_viewport_sem_repetir() {
        use fenkra_core::Id;
        let coletor = |b: usize| (b == 0).then_some(Id(7));
        let picks = [
            Pick::Face { body: 0, face: 1, owner: Id(5), point: [0.; 3], normal: [0., 0., 1.] },
            Pick::Edge { body: 0, edge: 3, mid: [0.; 3] },
            Pick::SkCurve { sketch: Id(5), curve: Id(99), at: [0.; 3] },
            Pick::Geo(Id(11)),
        ];
        let donos = picks.iter().filter_map(|p| dono_na_arvore(p, coletor));
        let ids = ids_do_conjunto(&[Id(2), Id(5)], Some(Id(11)), donos);
        assert_eq!(ids, vec![Id(2), Id(5), Id(11), Id(7)], "face e curva apontam o mesmo comando; a aresta, o corpo");
        assert_eq!(dono_na_arvore(&Pick::Plane(fenkra_core::PlaneRef::Origin(fenkra_core::OriginPlane::XY)), coletor), None);
    }

    #[test]
    fn comando_de_arestas_filtra_so_arestas_e_o_resto_usa_a_barra() {
        use fenkra_core::{EdgeMode, EdgeOp, EdgeProp, EdgeSel, FeatureKind};
        let fil = |edges| FeatureKind::Fillet(EdgeOp { value: "1".into(), edges, mode: EdgeMode::Constant, prop: EdgeProp::Tangente, keys: vec![], extra: Default::default() });
        assert_eq!(tipos_do_comando(&fil(EdgeSel::Edges(vec![]))).map(|t| t.nome()), Some("arestas"));
        assert_eq!(tipos_do_comando(&fil(EdgeSel::AllOfBody)), None, "todas as arestas não pede clique");
    }

    #[test]
    fn laco_ignora_pontos_quase_iguais() {
        let mut e = Extra::default();
        e.ponto(Pos2::new(0., 0.));
        e.ponto(Pos2::new(1., 1.));
        e.ponto(Pos2::new(10., 0.));
        assert_eq!(e.laco.len(), 2);
    }

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
