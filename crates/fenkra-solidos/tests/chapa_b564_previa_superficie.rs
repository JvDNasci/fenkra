// SPDX-License-Identifier: LGPL-2.1-or-later
//! B564: prévia simplificada da aba por superfície grande e curva durante a edição.
//!
//! Como funciona: com o comando aberto (`Cache::leve`), a aba por superfície relimita a web pelo
//! suporte e devolve a parede como contorno em linhas (`PreviaViz::linhas`, OML amostrada em 60
//! pontos), sem a varredura da dobra nem a união com a chapa, que são o grosso do tempo. A superfície
//! de suporte (varredura de spline e deslocamento) fica guardada no `Cache` entre quadros. O
//! resultado final, ao concluir, é o do cálculo completo de sempre.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, recompute_cached, Cache, Status};
use std::time::Instant;

fn s(x: &str) -> String {
    x.to_string()
}

fn naca(x: f64) -> f64 {
    5. * 0.12 * (0.2969 * x.sqrt() - 0.1260 * x - 0.3516 * x * x + 0.2843 * x.powi(3) - 0.1015 * x.powi(4))
}

/// Asa de corda `c` e envergadura `e` (mm): web de `c` × 0,6 `c`; suporte: o dorso do perfil
/// extrudado e deslocado de `off` mm (superfície grande e curva).
fn asa(c: f64, e: f64, off: Option<&str>, comprimento: &str) -> (Document, Id) {
    let mut d = Document::new("asa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [c, c * 0.6]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let pts: Vec<PointRef> = (0..=60)
        .map(|i| {
            let u = i as f64 / 60.;
            let x = (1. - (std::f64::consts::PI * u).cos()) / 2.;
            PointRef::Fixed([-0.05 * c + 1.1 * c * x, 0.28 * c + 1.4 * c * naca(x) * 0.5 + 0.06 * c * (1. - x), 0.])
        })
        .collect();
    let curva = d.add_geometry(RefKind::Spline { points: pts, closed: false, tangents: vec![] });
    let mut sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(curva), dir: AxisRef::Origin(OriginAxis::Z), length: format!("{e}"), length2: s("0"), until: None, until2: None, mais: vec![] });
    if let Some(o) = off {
        sup = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(sup), dist: s(o), both: false, join: OffsetJoin::Automatico });
    }
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Ref(sup), face: [c / 2., c * 0.1, 2.], length: s(comprimento), radius: None, flip: false, keep_base: false, mode: 0, angle: s("90"), limites: Default::default(), side: 0, inv_lado: false }));
    (d, aba)
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.
}

fn muda_comprimento(d: &mut Document, aba: Id, v: &str) {
    if let Some(FeatureKind::SmSurfFlange(x)) = d.feature_mut(aba).map(|f| &mut f.kind) {
        x.length = s(v);
    }
}

/// Asa grande (corda de 1 m, 2 m de envergadura, suporte deslocado de 20 mm): a prévia de cada
/// alteração do comprimento sai em menos de 300 ms, com o suporte vindo do cache.
#[test]
fn previa_de_asa_grande_edita_em_menos_de_300_ms() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, aba) = asa(1000., 2000., Some("20"), "20");
    let mut cache = Cache::default();
    // a primeira prévia ao abrir o comando constrói o suporte e a malha das referências
    cache.leve = Some(aba);
    let t = Instant::now();
    let r = recompute_cached(&d, &mut cache);
    let primeira = ms(t);
    assert!(matches!(r.status[&aba], Status::Ok | Status::Warning(_)), "{:?}", r.status[&aba]);
    let viz: Vec<_> = r.previas.iter().filter(|p| p.feature == aba).collect();
    assert_eq!(viz.len(), 1);
    assert!(viz[0].linhas.len() > 100, "contorno com {} segmentos", viz[0].linhas.len());
    // a web fica inteira na prévia (1000 × 600 × 2 = 1,2 milhão de mm³); o corte vem no final
    let vol = volume_total(&r);
    assert!((vol - 1_200_000.).abs() < 1., "web inteira: volume {vol}");

    let t = Instant::now();
    let _ = recompute_cached(&d, &mut cache);
    println!("B564 base sem mudança: {:.0} ms (reused {})", ms(t), cache.reused);
    let mut maior = 0f64;
    for (i, v) in ["22", "24", "26", "28", "30"].iter().enumerate() {
        muda_comprimento(&mut d, aba, v);
        let t = Instant::now();
        let r = recompute_cached(&d, &mut cache);
        let dt = ms(t);
        maior = maior.max(dt);
        assert_eq!(cache.reused, 2, "edição {i}: só a aba refaz");
        assert!(matches!(r.status[&aba], Status::Ok | Status::Warning(_)), "edição {i}: {:?}", r.status[&aba]);
        let topo = r.previas.iter().filter(|p| p.feature == aba).flat_map(|p| p.linhas.iter().flat_map(|l| l.iter())).map(|p| p[2]).fold(f64::MIN, f64::max);
        let esperado: f64 = 2. + v.parse::<f64>().unwrap() + 4.;
        assert!((topo - esperado).abs() < 12., "edição {i}: o contorno acompanha o comprimento ({topo} contra {esperado})");
    }
    println!("B564 asa grande: primeira prévia {primeira:.0} ms; edição: maior {maior:.0} ms");
    assert!(maior < 300., "a prévia levou {maior:.0} ms");
}

/// O que a prévia não pode fazer: mudar o resultado final. Asa de corda 200: concluir depois de editar
/// dá o mesmo sólido de um cálculo do zero.
#[test]
fn resultado_final_depois_da_previa_e_igual_ao_de_um_calculo_do_zero() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, aba) = asa(200., 60., None, "20");
    let mut cache = Cache::default();
    for v in ["18", "20", "22"] {
        muda_comprimento(&mut d, aba, v);
        cache.leve = Some(aba);
        let _ = recompute_cached(&d, &mut cache);
    }
    cache.leve = None;
    let final_ = recompute_cached(&d, &mut cache);
    let zero = recompute(&d);
    assert_eq!(final_.status[&aba], Status::Ok, "{:?}", final_.status[&aba]);
    assert_eq!(zero.status[&aba], Status::Ok);
    assert!(final_.previas.iter().all(|p| p.feature != aba), "o resultado final não traz prévia");
    let (a, b) = (volume_total(&final_), volume_total(&zero));
    assert!((a - b).abs() < 1e-6 * b, "final {a} contra do zero {b}");
    let ((lo1, hi1), (lo2, hi2)) = (caixa_da_malha(&final_), caixa_da_malha(&zero));
    for i in 0..3 {
        assert!((lo1[i] - lo2[i]).abs() < 1e-3 && (hi1[i] - hi2[i]).abs() < 1e-3, "caixa {lo1:?} {hi1:?} contra {lo2:?} {hi2:?}");
    }
    // e a aba existe de verdade: sobe 22 mm mais a dobra
    assert!(hi1[2] > 22. + 2., "aba final {hi1:?}");
}

/// O suporte construído fica no cache: a segunda prévia não constrói a superfície de novo.
#[test]
fn suporte_fica_no_cache_entre_quadros() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, aba) = asa(400., 600., Some("10"), "20");
    let mut cache = Cache::default();
    cache.leve = Some(aba);
    let t = Instant::now();
    let _ = recompute_cached(&d, &mut cache);
    let primeira = ms(t);
    muda_comprimento(&mut d, aba, "25");
    let t = Instant::now();
    let _ = recompute_cached(&d, &mut cache);
    let segunda = ms(t);
    println!("B564 suporte em cache (corda 400): primeira {primeira:.0} ms, segunda {segunda:.0} ms");
    assert!(segunda < primeira, "a segunda ({segunda:.0} ms) é mais rápida que a primeira ({primeira:.0} ms)");
}

/// Aviso claro e o trecho: uma aba mais longa que o raio de curvatura côncavo da OML não planifica.
#[test]
fn validacao_aponta_o_trecho_que_nao_planifica() {
    if !kernel_disponivel() {
        return;
    }
    // suporte: cilindro de raio 30 (a aba sobe pelo lado de dentro da curva)
    let mut d = Document::new("arco");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 100.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let cil = d.add_geometry(RefKind::SurfCylinder { point: PointRef::Fixed([50., 50., -5.]), dir: AxisRef::Origin(OriginAxis::Z), radius: s("30"), start: s("0"), end: s("10") });
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Ref(cil), face: [50., 90., 2.], length: s("60"), radius: None, flip: false, keep_base: false, mode: 0, angle: s("90"), limites: Default::default(), side: 0, inv_lado: false }));
    let mut cache = Cache::default();
    cache.leve = Some(aba);
    let r = recompute_cached(&d, &mut cache);
    // o cilindro de raio 30 fecha em volta da aba de 60 mm: a validação avisa onde e até quanto vale
    match &r.status[&aba] {
        Status::Warning(m) => assert!(m.contains("não planifica") && m.contains("29.") && m.contains("use comprimento até"), "{m}"),
        outro => panic!("devia avisar que não planifica: {outro:?}"),
    }
}
