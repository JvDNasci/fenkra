// SPDX-License-Identifier: LGPL-2.1-or-later
//! B563: aba por superfície no padrão das outras abas. Nervura de asa (suporte plano e suporte curvo,
//! um perfil de asa de quatro dígitos) com as três posições do material (começo do raio, face interna,
//! face externa) e as quatro combinações de inversão (direção × lado da superfície): status ok, aba
//! encostada na superfície (distância zero), volume e caixa conferidos e planificada correta.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.to_string()
}

const T: f64 = 2.;
const R: f64 = 2.;
const LEN: f64 = 15.;

fn chapa(d: &mut Document, largura: f64, altura: f64) {
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [largura, altura]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
}

/// Web de 100 × 50 e o suporte plano x = 75 (o corte da nervura por uma superfície plana).
fn nervura_plana(side: u8, flip: bool, inv_lado: bool) -> (Document, Id) {
    let mut d = Document::new("nervura plana");
    chapa(&mut d, 100., 50.);
    let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::YZ), "75");
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Plane(PlaneRef::Datum(pl)), face: [40., 25., 2.], length: s("15"), radius: None, flip, keep_base: false, mode: 0, angle: s("90"), limites: Default::default(), side, inv_lado }));
    (d, aba)
}

/// Meia espessura de um perfil de asa de quatro dígitos (12 %), `x` em 0..1.
fn naca(x: f64) -> f64 {
    5. * 0.12 * (0.2969 * x.sqrt() - 0.1260 * x - 0.3516 * x * x + 0.2843 * x.powi(3) - 0.1015 * x.powi(4))
}

/// Web de 100 × 60 (a nervura) e, como suporte, o dorso do perfil de asa (corda de 100, espessura
/// 12 %, ampliada 1,4 vezes) extrudado em Z: a OML é uma curva lisa que corta a web de ponta a ponta.
fn nervura_naca(side: u8, flip: bool, inv_lado: bool) -> (Document, Id) {
    let mut d = Document::new("nervura naca");
    chapa(&mut d, 100., 60.);
    let pts: Vec<PointRef> = (0..=20)
        .map(|i| {
            let u = i as f64 / 20.;
            // espaçamento cossenoidal: mais pontos perto do bordo de ataque
            let x = (1. - (std::f64::consts::PI * u).cos()) / 2.;
            PointRef::Fixed([-5. + 110. * x, 28. + 1.4 * 100. * naca(x) * 0.5 + 6. * (1. - x), 0.])
        })
        .collect();
    let curva = d.add_geometry(RefKind::Spline { points: pts, closed: false, tangents: vec![] });
    let sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(curva), dir: AxisRef::Origin(OriginAxis::Z), length: s("60"), length2: s("0"), until: None, until2: None });
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Ref(sup), face: [50., 10., 2.], length: s("15"), radius: None, flip, keep_base: false, mode: 0, angle: s("90"), limites: Default::default(), side, inv_lado }));
    (d, aba)
}

fn calcula(d: &Document, flat: bool) -> RecomputeResult {
    let mut d = d.clone();
    d.sheet.as_mut().unwrap().flat = flat;
    recompute(&d)
}

/// O que cada posição pede, medido na peça dobrada com a aba para cima e a chapa do lado x < 75: o
/// ponto mais alto em x da peça. Começo do raio: a dobra nasce na superfície e a parede fica
/// `r + t` além dela; face interna: a face interna da parede está sobre a superfície; face externa:
/// a face externa está sobre a superfície.
fn x_maximo_esperado(side: u8) -> f64 {
    match side {
        0 => 75. + R + T,
        1 => 75. + T,
        _ => 75.,
    }
}

#[test]
fn plana_as_tres_posicoes_encostam_a_aba_na_superficie() {
    if !kernel_disponivel() {
        return;
    }
    for side in 0..=2u8 {
        let (d, aba) = nervura_plana(side, false, false);
        let r = calcula(&d, false);
        assert_eq!(r.status[&aba], Status::Ok, "posição {side}: {:?}", r.status[&aba]);
        let (lo, hi) = caixa_da_malha(&r);
        assert!((hi[0] - x_maximo_esperado(side)).abs() < 0.15, "posição {side}: x máximo {} (esperado {})", hi[0], x_maximo_esperado(side));
        assert!(lo[0].abs() < 0.1, "posição {side}: a base começa em x = 0 ({lo:?})");
        // a parede sobe 15 sobre a face da chapa, mais o raio e a espessura da dobra
        assert!((hi[2] - (LEN + R + T)).abs() < 0.3, "posição {side}: altura {}", hi[2]);
        let vol = volume_total(&r);
        assert!(vol > 7000. && vol < 10_000., "posição {side}: volume {vol}");
    }
}

#[test]
fn plana_face_interna_e_externa_tem_distancia_zero_da_superficie() {
    if !kernel_disponivel() {
        return;
    }
    // face interna (1): o vão entre a superfície e a parede some (x mínimo da parede = 75);
    // face externa (2): a parede termina na superfície (x máximo = 75)
    for (side, deve) in [(1u8, 75.), (2u8, 75.)] {
        let (d, aba) = nervura_plana(side, false, false);
        let r = calcula(&d, false);
        assert_eq!(r.status[&aba], Status::Ok);
        let alto: Vec<[f64; 3]> = r.bodies.iter().flat_map(|b| b.mesh.positions.chunks_exact(3).map(|p| [p[0] as f64, p[1] as f64, p[2] as f64])).filter(|p| p[2] > 8.).collect();
        assert!(!alto.is_empty(), "posição {side}: sem parede reta");
        let (xmin, xmax) = alto.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p[0]), b.max(p[0])));
        let medido = if side == 1 { xmin } else { xmax };
        assert!((medido - deve).abs() < 0.1, "posição {side}: a face da parede está em x = {medido}, devia estar em {deve}");
    }
}

#[test]
fn plana_as_quatro_inversoes_em_cada_posicao_ficam_validas_e_planificam() {
    if !kernel_disponivel() {
        return;
    }
    for side in 0..=2u8 {
        for flip in [false, true] {
            for inv in [false, true] {
                let nome = format!("plana posição {side}, direção invertida {flip}, lado invertido {inv}");
                let (d, aba) = nervura_plana(side, flip, inv);
                let r = calcula(&d, false);
                assert_eq!(r.status[&aba], Status::Ok, "{nome}: {:?}", r.status[&aba]);
                let (lo, hi) = caixa_da_malha(&r);
                // direção invertida: a aba desce (z negativo) em vez de subir
                if flip {
                    assert!(lo[2] < -(LEN + R) + 0.5, "{nome}: aba devia descer ({lo:?})");
                } else {
                    assert!(hi[2] > LEN + R - 0.5, "{nome}: aba devia subir ({hi:?})");
                }
                // lado invertido: a chapa que fica é a de x > 75 e a aba vai para o lado de x < 75... a
                // caixa em x acompanha o lado mantido
                if inv {
                    assert!(hi[0] > 99.9 && lo[0] > 60., "{nome}: devia ficar a parte de x > 75 ({lo:?} {hi:?})");
                } else {
                    assert!(lo[0].abs() < 0.1 && hi[0] < 85., "{nome}: devia ficar a parte de x < 75 ({lo:?} {hi:?})");
                }
                let vol = volume_total(&r);
                assert!(vol > 3000., "{nome}: volume {vol}");
                let plana = calcula(&d, true);
                assert_eq!(plana.status[&aba], Status::Ok, "{nome} planificada: {:?}", plana.status[&aba]);
                let (_, hip) = caixa_da_malha(&plana);
                assert!(hip[2] < 2.1, "{nome}: planificada fica plana ({})", hip[2]);
                assert!(volume_total(&plana) > 3000., "{nome}: volume planificado {}", volume_total(&plana));
            }
        }
    }
}

#[test]
fn curva_as_tres_posicoes_e_quatro_inversoes_de_uma_nervura_de_asa() {
    if !kernel_disponivel() {
        return;
    }
    for side in 0..=2u8 {
        for flip in [false, true] {
            for inv in [false, true] {
                let nome = format!("naca posição {side}, direção invertida {flip}, lado invertido {inv}");
                let (d, aba) = nervura_naca(side, flip, inv);
                let r = calcula(&d, false);
                assert_eq!(r.status[&aba], Status::Ok, "{nome}: {:?}", r.status[&aba]);
                assert!(r.bodies.iter().all(|b| !b.mesh.positions.is_empty() && b.mass.volume > 1000.), "{nome}: corpo vazio com status Ok");
                let (lo, hi) = caixa_da_malha(&r);
                if flip {
                    assert!(lo[2] < -(LEN + R) + 0.5, "{nome}: aba devia descer ({lo:?})");
                } else {
                    assert!(hi[2] > LEN + R - 0.5, "{nome}: aba devia subir ({hi:?})");
                }
                // a peça fica na caixa da web de 100 × 60 (mais a folga da parede que acompanha a curva nas pontas)
                assert!(lo[0] > -2. && hi[0] < 102. && lo[1] > -0.1 && hi[1] < 62., "{nome}: fora da caixa {lo:?} {hi:?}");
                let plana = calcula(&d, true);
                assert_eq!(plana.status[&aba], Status::Ok, "{nome} planificada: {:?}", plana.status[&aba]);
                let (_, hip) = caixa_da_malha(&plana);
                assert!(hip[2] < 2.1, "{nome}: planificada fica plana ({})", hip[2]);
            }
        }
    }
}

/// Documento gravado antes do B563 (sem `side` nem `inv_lado`) abre igual: começo do raio, sem
/// inversão de lado.
#[test]
fn documento_antigo_sem_posicao_abre_igual() {
    let (d, _) = nervura_plana(0, false, false);
    let json = serde_json::to_string(&d).unwrap();
    assert!(!json.contains("\"side\":0") || !json.contains("inv_lado"), "campos novos só aparecem quando mudam do padrão");
    let volta: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(volta.features.len(), d.features.len());
    if !kernel_disponivel() {
        return;
    }
    let a = recompute(&d);
    let b = recompute(&volta);
    assert!((volume_total(&a) - volume_total(&b)).abs() < 1e-6);
}
