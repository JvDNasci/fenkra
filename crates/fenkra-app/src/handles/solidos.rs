// SPDX-License-Identifier: LGPL-2.1-or-later
//! Alças 3D extras da bancada Sólidos (B288), somadas às de `handles.rs`:
//!
//! - ângulo de saída da extrusão: arco na aresta do perfil, que abre a parede para fora;
//! - término de dois lados: uma seta por lado; plano médio: a seta das duas pontas;
//! - base das primitivas: largura e profundidade da caixa/cunha, raio da base dos demais;
//! - Mover corpo: seta de translação e arco de rotação; Afinidade: uma seta de escala por eixo.
//!
//! Toda alça tem cota editável e fica travada (🔒) quando o valor é um parâmetro, porque quem
//! desenha é `linear_handle`/`arc_handle`. A gravação do valor está em `aplicar`, sem interface,
//! para o teste arrastar cada alça e conferir o campo que mudou.

use super::*;
use fenkra_kernel::Frame;

/// A partir deste número o campo vive aqui; abaixo, em `handles.rs`.
pub(super) const PRIMEIRO: u8 = 10;
/// Segundo lado do término de dois lados.
pub(super) const LADO2: u8 = 10;
/// Outra ponta do plano médio (mesmo campo da primeira).
pub(super) const PLANO_MEDIO: u8 = 11;
/// Primeira e segunda medida da base de uma primitiva.
pub(super) const BASE_A: u8 = 12;
pub(super) const BASE_B: u8 = 13;
/// Distância de uma translação.
pub(super) const MOVER: u8 = 14;
/// Escala em X, Y e Z da afinidade (15, 16 e 17).
pub(super) const ESCALA_X: u8 = 15;
/// Ângulo de saída da extrusão.
pub(super) const SAIDA: u8 = 18;
/// Ângulo de uma rotação do corpo.
pub(super) const GIRAR: u8 = 19;

/// Limite do arco do ângulo de saída, em graus.
const LIM_SAIDA: (f64, f64) = (-45., 45.);

fn frame_do_plano(r: &fenkra_solidos::RecomputeResult, p: &PlaneRef) -> Option<Frame> {
    match p {
        PlaneRef::Origin(o) => Some(fenkra_solidos::origin_frame(*o)),
        PlaneRef::Datum(id) => r.planes.iter().find(|x| x.id == Some(*id)).map(|x| x.frame),
    }
}

/// Caixa (mínimo, máximo) da malha do corpo que o comando altera.
fn caixa_do_corpo(r: &fenkra_solidos::RecomputeResult, alvo: Id) -> Option<(Vec3, Vec3)> {
    let b = r.bodies.iter().find(|b| b.collector == alvo)?;
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for p in b.mesh.positions.chunks_exact(3) {
        let q = Vec3::new(p[0], p[1], p[2]);
        lo = lo.min(q);
        hi = hi.max(q);
    }
    (lo.x <= hi.x).then_some((lo, hi))
}

/// Base e sentido da extrusão (como em `handle_for`).
fn base_da_extrusao(r: &fenkra_solidos::RecomputeResult, e: &Extrude) -> Option<(Vec3, Vec3)> {
    let (c, n) = sketch_center(r, e.sketch)?;
    let into = (e.op == BoolOp::Cut) != e.reverse;
    Some((c, if into { -n } else { n }))
}

/// Alças lineares extras do comando `id`: (campo, alça).
pub(super) fn alcas(r: &fenkra_solidos::RecomputeResult, doc: &Document, id: Id) -> Vec<(u8, Linear)> {
    let Some(f) = doc.feature(id) else { return vec![] };
    match &f.kind {
        FeatureKind::Extrude(e) => {
            let Some((c, dir)) = base_da_extrusao(r, e) else { return vec![] };
            match &e.extent {
                Extent::TwoSides(_, b) => vec![(LADO2, Linear { base: c, dir: -dir, k: 1., expr: b.clone(), label: "Distância 2" })],
                Extent::Symmetric(d) => vec![(PLANO_MEDIO, Linear { base: c, dir: -dir, k: 2., expr: d.clone(), label: "Distância total" })],
                _ => vec![],
            }
        }
        FeatureKind::Primitive(p) => {
            let Some(fr) = frame_do_plano(r, &p.plane) else { return vec![] };
            let (o, x) = (v3(fr.origin), v3(fr.x));
            let y = v3(fr.normal).cross(x);
            let lin = |dir, k, expr: &String, label| Linear { base: o, dir, k, expr: expr.clone(), label };
            match p.kind {
                // caixa centrada no plano: a seta de cada medida sai do centro e mostra a metade
                PrimKind::Caixa => vec![(BASE_A, lin(x, 2., &p.a, "Largura")), (BASE_B, lin(y, 2., &p.b, "Profundidade"))],
                PrimKind::Cunha => vec![(BASE_A, lin(x, 1., &p.a, "Largura")), (BASE_B, lin(y, 1., &p.b, "Profundidade"))],
                PrimKind::Cilindro | PrimKind::Esfera => vec![(BASE_A, lin(x, 1., &p.a, "Raio"))],
                PrimKind::Cone => vec![(BASE_A, lin(x, 1., &p.a, "Raio da base"))],
                PrimKind::Toro => vec![(BASE_A, lin(x, 1., &p.a, "Raio maior"))],
                PrimKind::Prisma => vec![(BASE_B, lin(x, 1., &p.b, "Raio"))],
            }
        }
        FeatureKind::MoveBody(m) => {
            let Some((lo, hi)) = caixa_do_corpo(r, f.target) else { return vec![] };
            let centro = (lo + hi) * 0.5;
            match &m.xf {
                BodyXf::Translate { dir, dist } => match axis_pos(r, doc, dir) {
                    Some((_, d)) => vec![(MOVER, Linear { base: centro, dir: d, k: 1., expr: dist.clone(), label: "Distância" })],
                    None => vec![],
                },
                // escala: a seta de cada eixo mede o fator vezes a meia-extensão do corpo naquele eixo
                BodyXf::Affinity { plane, fx, fy, fz } => {
                    let Some(fr) = frame_do_plano(r, plane) else { return vec![] };
                    let (o, x, n) = (v3(fr.origin), v3(fr.x), v3(fr.normal));
                    let eixos = [(x, fx, "Escala X"), (n.cross(x), fy, "Escala Y"), (n, fz, "Escala Z")];
                    eixos
                        .into_iter()
                        .enumerate()
                        .map(|(i, (dir, expr, label))| {
                            // extensão do corpo ao longo do eixo, vista da origem do plano
                            let ext = [lo, hi].iter().map(|p| (*p - o).dot(dir).abs()).fold(0f32, f32::max).max(1.);
                            (ESCALA_X + i as u8, Linear { base: o, dir, k: 1. / ext, expr: expr.clone(), label })
                        })
                        .collect()
                }
                _ => vec![],
            }
        }
        _ => vec![],
    }
}

/// Grava o valor de uma alça extra no campo certo. `false` = o campo não existe nesse comando.
pub(super) fn aplicar(doc: &mut Document, id: Id, slot: u8, valor: String) -> bool {
    let Some(f) = doc.feature_mut(id) else { return false };
    match (&mut f.kind, slot) {
        (FeatureKind::Extrude(e), LADO2) => match &mut e.extent {
            Extent::TwoSides(_, b) => *b = valor,
            _ => return false,
        },
        (FeatureKind::Extrude(e), PLANO_MEDIO) => match &mut e.extent {
            Extent::Symmetric(d) => *d = valor,
            _ => return false,
        },
        (FeatureKind::Extrude(e), SAIDA) => e.taper = valor,
        (FeatureKind::Primitive(p), BASE_A) => p.a = valor,
        (FeatureKind::Primitive(p), BASE_B) => p.b = valor,
        (FeatureKind::MoveBody(m), MOVER) => match &mut m.xf {
            BodyXf::Translate { dist, .. } => *dist = valor,
            _ => return false,
        },
        (FeatureKind::MoveBody(m), GIRAR) => match &mut m.xf {
            BodyXf::Rotate { angle, .. } => *angle = valor,
            _ => return false,
        },
        (FeatureKind::MoveBody(m), s @ ESCALA_X..=17) => match &mut m.xf {
            BodyXf::Affinity { fx, fy, fz, .. } => *[fx, fy, fz][(s - ESCALA_X) as usize] = valor,
            _ => return false,
        },
        _ => return false,
    }
    true
}

/// Geometria do arco do ângulo de saída: (pivô na aresta do perfil, eixo do arco, vetor de partida
/// ao longo da extrusão). Girar o vetor de partida em torno do eixo pelo ângulo leva a parede para
/// dentro quando positivo (é o que o kernel faz: ângulo positivo afina a peça) e para fora quando negativo.
fn geometria_saida(r: &fenkra_solidos::RecomputeResult, e: &Extrude) -> Option<(Vec3, Vec3, Vec3)> {
    let s = r.sketches.iter().find(|s| s.feature == e.sketch)?;
    let (_, dir) = base_da_extrusao(r, e)?;
    let n = v3(s.frame.normal);
    // a primeira aresta do perfil: a parede que vai ganhar a saída
    let (a, b) = (Vec3::from(*s.lines.first()?), Vec3::from(*s.lines.get(1)?));
    let t = (b - a).try_normalize()?;
    let (c, _) = sketch_center(r, e.sketch)?;
    let pivo = (a + b) * 0.5;
    let mut fora = n.cross(t).try_normalize()?;
    if (pivo - c).dot(fora) < 0. {
        fora = -fora;
    }
    let comprimento = ((b - a).length() * 0.8).max(5.);
    let partida = dir * comprimento;
    // ângulo positivo fecha a parede (a peça afina para o fim da extrusão): gira para dentro
    let eixo = fora.cross(dir).try_normalize()?;
    Some((pivo, eixo, partida))
}

/// Arcos do comando aberto: ângulo de saída da extrusão e rotação do corpo.
pub(super) fn arcos(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let Some(r) = app.result.as_ref() else { return };
    let Some(f) = app.open.doc.feature(id) else { return };
    match f.kind.clone() {
        FeatureKind::Extrude(e) if matches!(e.extent, Extent::Blind(_)) => {
            if let Some((pivo, eixo, partida)) = geometria_saida(r, &e) {
                arc_handle(app, ui, rect, cam, id, pivo + partida, (pivo, eixo), &e.taper, false, SAIDA, LIM_SAIDA);
            }
        }
        FeatureKind::MoveBody(MoveBody { xf: BodyXf::Rotate { axis, angle } }) => {
            let Some((lo, hi)) = caixa_do_corpo(r, f.target) else { return };
            let Some((o, d)) = axis_pos(r, &app.open.doc, &axis) else { return };
            let centro = (lo + hi) * 0.5;
            let raio = ((hi - lo).length() * 0.5).max(5.);
            // ponto de partida: o centro do corpo, ou, se ele estiver sobre o eixo, um perpendicular
            let de_fora = centro - (o + d * (centro - o).dot(d));
            let base = if de_fora.length() > 1e-3 { de_fora.normalize() } else { d.any_orthonormal_vector() };
            let pe = o + d * (centro - o).dot(d);
            arc_handle(app, ui, rect, cam, id, pe + base * raio, (o, d), &angle, false, GIRAR, (-360., 360.));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_solidos::recompute;

    fn t(x: &str) -> String {
        x.into()
    }

    const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

    fn base_com_extrusao(extent: Extent, taper: &str) -> (Document, Id) {
        let mut d = Document::new("t");
        let mut data = SketchData::default();
        let mut al = || d.alloc();
        data.add_rect(&mut al, [-10., -10.], [10., 10.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: XY, data }));
        let ex = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent, reverse: false, taper: t(taper), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        (d, ex)
    }

    /// O mesmo trajeto da alça: o ponto do mouse vira distância ao longo da seta, e a distância vira valor.
    fn valor_no_ponto(h: &Linear, ponto: Vec3) -> f64 {
        ((ponto - h.base).dot(h.dir) * h.k) as f64
    }

    #[test]
    fn dois_lados_ganham_uma_seta_por_lado_e_cada_uma_grava_o_seu_campo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut d, ex) = base_com_extrusao(Extent::TwoSides(t("12"), t("5")), "0");
        let r = recompute(&d);
        let extras = alcas(&r, &d, ex);
        assert_eq!(extras.len(), 1);
        let (slot, h) = &extras[0];
        assert_eq!(*slot, LADO2);
        // a seta do lado 2 aponta para o outro lado da primeira
        let principal = handle_for_doc(&r, &d, ex).unwrap();
        assert!((principal.dir + h.dir).length() < 1e-5);
        // arrastar a ponta até 7 mm do plano grava 7 no lado 2, e só nele
        let alvo = h.base + h.dir * 7.;
        assert!((valor_no_ponto(h, alvo) - 7.).abs() < 1e-4);
        assert!(aplicar(&mut d, ex, *slot, "7".into()));
        let Some(FeatureKind::Extrude(e)) = d.feature(ex).map(|f| f.kind.clone()) else { panic!() };
        assert_eq!(e.extent, Extent::TwoSides(t("12"), t("7")));
        // e o sólido acompanha: 20 × 20 × (12 + 7)
        let v: f64 = recompute(&d).bodies.iter().map(|b| b.mass.volume).sum();
        assert!((v - 20. * 20. * 19.).abs() < 1e-3, "{v}");
    }

    /// A alça principal da extrusão, como o overlay a monta.
    fn handle_for_doc(r: &fenkra_solidos::RecomputeResult, d: &Document, id: Id) -> Option<Linear> {
        let Some(FeatureKind::Extrude(e)) = d.feature(id).map(|f| f.kind.clone()) else { return None };
        let (c, dir) = base_da_extrusao(r, &e)?;
        Some(Linear { base: c, dir, k: 1., expr: String::new(), label: "" })
    }

    #[test]
    fn plano_medio_tem_seta_nas_duas_pontas_e_as_duas_gravam_o_mesmo_campo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut d, ex) = base_com_extrusao(Extent::Symmetric(t("10")), "0");
        let r = recompute(&d);
        let extras = alcas(&r, &d, ex);
        assert_eq!(extras.len(), 1);
        let (slot, h) = &extras[0];
        assert_eq!(*slot, PLANO_MEDIO);
        assert_eq!(h.k, 2.);
        // a ponta de cada lado em 6 mm do plano médio = cota total 12
        assert!((valor_no_ponto(h, h.base + h.dir * 6.) - 12.).abs() < 1e-4);
        assert!(aplicar(&mut d, ex, *slot, "12".into()));
        let v: f64 = recompute(&d).bodies.iter().map(|b| b.mass.volume).sum();
        assert!((v - 20. * 20. * 12.).abs() < 1e-3, "{v}");
        // campo que não é do término: recusa em vez de gravar errado
        assert!(!aplicar(&mut d, ex, LADO2, "3".into()));
    }

    #[test]
    fn base_das_primitivas_grava_largura_profundidade_e_raio() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = Document::new("p");
        let mk = |k, a: &str, b: &str, c: &str| FeatureKind::Primitive(Primitive { kind: k, plane: XY, a: t(a), b: t(b), c: t(c), d: t("0"), op: BoolOp::Add });
        let caixa = d.add_feature(mk(PrimKind::Caixa, "20", "10", "5"));
        let r = recompute(&d);
        let hs = alcas(&r, &d, caixa);
        assert_eq!(hs.iter().map(|h| h.0).collect::<Vec<_>>(), [BASE_A, BASE_B]);
        // a caixa é centrada: a ponta da seta de 20 de largura está a 10 do centro
        let (_, ha) = &hs[0];
        assert!((valor_no_ponto(ha, ha.base + ha.dir * 15.) - 30.).abs() < 1e-4);
        assert!(aplicar(&mut d, caixa, BASE_A, "30".into()));
        assert!(aplicar(&mut d, caixa, BASE_B, "8".into()));
        let v: f64 = recompute(&d).bodies.iter().map(|b| b.mass.volume).sum();
        assert!((v - 30. * 8. * 5.).abs() < 1e-3, "{v}");
        // cilindro: a alça é o raio
        let mut d2 = Document::new("c");
        let cil = d2.add_feature(mk(PrimKind::Cilindro, "4", "10", "0"));
        let r2 = recompute(&d2);
        let hs2 = alcas(&r2, &d2, cil);
        assert_eq!((hs2.len(), hs2[0].1.label), (1, "Raio"));
        assert!(aplicar(&mut d2, cil, BASE_A, "6".into()));
        let v2: f64 = recompute(&d2).bodies.iter().map(|b| b.mass.volume).sum();
        assert!((v2 - std::f64::consts::PI * 36. * 10.).abs() / v2 < 1e-3, "{v2}");
        // esfera e toro também
        let mut d3 = Document::new("e");
        let esf = d3.add_feature(mk(PrimKind::Esfera, "5", "0", "0"));
        assert_eq!(alcas(&recompute(&d3), &d3, esf).len(), 1);
    }

    #[test]
    fn mover_corpo_tem_seta_de_translacao_e_setas_de_escala() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut d, _ex) = base_com_extrusao(Extent::Blind(t("10")), "0");
        let mv = d.add_feature(FeatureKind::MoveBody(MoveBody { xf: BodyXf::Translate { dir: AxisRef::Origin(OriginAxis::X), dist: t("15") } }));
        let r = recompute(&d);
        let hs = alcas(&r, &d, mv);
        assert_eq!(hs.len(), 1);
        let (slot, h) = &hs[0];
        assert_eq!(*slot, MOVER);
        assert!((h.dir - Vec3::X).length() < 1e-5);
        // o corpo, depois de movido 15 em X, tem o centro em x = 15: a seta sai dele
        assert!((h.base.x - 15.).abs() < 0.1, "{:?}", h.base);
        assert!(aplicar(&mut d, mv, *slot, "25".into()));
        let r = recompute(&d);
        let (lo, _) = caixa_do_corpo(&r, d.feature(mv).unwrap().target).unwrap();
        assert!((lo.x - 15.).abs() < 1e-3, "{lo:?}");

        // afinidade: uma seta por eixo, cada uma grava o fator do seu eixo
        let (mut d, _ex) = base_com_extrusao(Extent::Blind(t("10")), "0");
        let af = d.add_feature(FeatureKind::MoveBody(MoveBody { xf: BodyXf::Affinity { plane: XY, fx: t("1"), fy: t("1"), fz: t("1") } }));
        let r = recompute(&d);
        let hs = alcas(&r, &d, af);
        assert_eq!(hs.iter().map(|h| h.0).collect::<Vec<_>>(), [ESCALA_X, ESCALA_X + 1, ESCALA_X + 2]);
        // com fator 1, a ponta da seta de X fica na extremidade do corpo (x = 10)
        let (_, hx) = &hs[0];
        assert!((hx.base + hx.dir * (1. / hx.k) - Vec3::new(10., 0., 0.)).length() < 0.1);
        assert!(aplicar(&mut d, af, ESCALA_X, "2".into()));
        assert!(aplicar(&mut d, af, ESCALA_X + 2, "0,5".into()));
        let v: f64 = recompute(&d).bodies.iter().map(|b| b.mass.volume).sum();
        assert!((v - 40. * 20. * 5.).abs() < 1e-2, "{v}");
    }

    #[test]
    fn saida_ganha_arco_na_aresta_do_perfil_e_grava_o_angulo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut d, ex) = base_com_extrusao(Extent::Blind(t("20")), "0");
        let r = recompute(&d);
        let Some(FeatureKind::Extrude(e)) = d.feature(ex).map(|f| f.kind.clone()) else { panic!() };
        let (pivo, eixo, partida) = geometria_saida(&r, &e).expect("sem geometria do arco");
        // o pivô está na aresta do perfil (a 10 do centro), o eixo é perpendicular à extrusão e à parede
        assert!((pivo.length() - 10.).abs() < 1e-3, "{pivo:?}");
        assert!(eixo.dot(partida).abs() < 1e-5 && (eixo.length() - 1.).abs() < 1e-5);
        assert!(aplicar(&mut d, ex, SAIDA, "5".into()));
        let v0: f64 = {
            let (d0, _) = base_com_extrusao(Extent::Blind(t("20")), "0");
            recompute(&d0).bodies.iter().map(|b| b.mass.volume).sum()
        };
        let v: f64 = recompute(&d).bodies.iter().map(|b| b.mass.volume).sum();
        assert!((v - v0).abs() > 1., "a saída de 5° não mudou o volume: {v} × {v0}");
        // o arco acompanha o kernel: +ângulo afina a peça (volume cai) e o vetor gira para dentro
        let fora = pivo.normalize();
        let girado = glam::Quat::from_axis_angle(eixo, 5f32.to_radians()) * partida;
        assert!(girado.dot(fora) < 0., "o arco precisa girar para dentro com ângulo positivo");
        assert!(v < v0, "ângulo positivo afina a peça (volume {v} × {v0})");
    }

    #[test]
    fn teclas_mudam_o_passo_do_arraste() {
        let normal = crate::ui_ext::mult_teclas(egui::Modifiers::NONE);
        let shift = crate::ui_ext::mult_teclas(egui::Modifiers { shift: true, ..Default::default() });
        assert_eq!(normal, 1.);
        assert_ne!(shift, normal);
    }
}
