// SPDX-License-Identifier: LGPL-2.1-or-later
//! B566: dobrado e planificado calculados juntos e guardados no cache; alternar só troca a exibição.
//! Nervura de asa (web de 100 × 60 e, como suporte, o dorso de um perfil de asa de quatro dígitos) com
//! aba de 20 mm: dobra, planifica, e a alternância não recalcula nenhum comando.
//!
//! Como funciona: `recompute_ambos` calcula o estado do documento e, em seguida, o outro estado, no
//! mesmo `Cache`. Cada estado tem o seu conjunto de resultados (`Cache::trocar_estado`), então virar
//! `sheet.flat` depois só reaproveita tudo (`Cache::reused` igual ao número de comandos). A
//! assinatura de cada comando já traz o estado da chapa, e mudar um campo a montante invalida só o
//! que depende dele, nos dois estados.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{preaquecer_outro_estado, recompute, recompute_ambos, recompute_cached, Cache, Status};
use std::time::Instant;

fn s(x: &str) -> String {
    x.to_string()
}

/// Meia espessura de um perfil de asa de quatro dígitos (12 %), `x` em 0..1.
fn naca(x: f64) -> f64 {
    5. * 0.12 * (0.2969 * x.sqrt() - 0.1260 * x - 0.3516 * x * x + 0.2843 * x.powi(3) - 0.1015 * x.powi(4))
}

/// Nervura de asa: web de 100 × 60 com a aba por superfície no dorso do perfil.
fn nervura(comprimento: &str, side: u8) -> (Document, Id, Id) {
    let mut d = Document::new("nervura de asa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 60.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let pts: Vec<PointRef> = (0..=20)
        .map(|i| {
            let u = i as f64 / 20.;
            let x = (1. - (std::f64::consts::PI * u).cos()) / 2.;
            PointRef::Fixed([-5. + 110. * x, 28. + 1.4 * 100. * naca(x) * 0.5 + 6. * (1. - x), 0.])
        })
        .collect();
    let curva = d.add_geometry(RefKind::Spline { points: pts, closed: false, tangents: vec![] });
    let sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(curva), dir: AxisRef::Origin(OriginAxis::Z), length: s("60"), length2: s("0"), until: None, until2: None, mais: vec![] });
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Ref(sup), face: [50., 10., 2.], length: s(comprimento), radius: None, flip: false, keep_base: false, mode: 0, angle: s("90"), limites: Default::default(), side, inv_lado: false }));
    (d, aba, sup)
}

fn com_flat(d: &Document, flat: bool) -> Document {
    let mut d = d.clone();
    d.sheet.as_mut().unwrap().flat = flat;
    d
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.
}

/// Aceite principal: a nervura de asa com aba de 20 mm planifica, nas três posições do material.
#[test]
fn nervura_de_asa_com_aba_de_20_mm_dobra_e_planifica() {
    if !kernel_disponivel() {
        return;
    }
    for side in 0..=2u8 {
        let (d, aba, _) = nervura("20", side);
        let mut cache = Cache::default();
        let dobrada = recompute_ambos(&d, &mut cache);
        assert!(ok_ou_conformacao(&dobrada.status[&aba]), "posição {side} dobrada: {:?}", dobrada.status[&aba]);
        let (lo, hi) = caixa_da_malha(&dobrada);
        assert!(hi[2] > 20. + 2. - 0.5, "posição {side}: a aba sobe 20 mm além da dobra ({hi:?})");
        assert!(lo[0] > -6. && hi[0] < 106., "posição {side}: caixa em x {lo:?} {hi:?}");
        let plana = recompute_cached(&com_flat(&d, true), &mut cache);
        assert!(ok_ou_conformacao(&plana.status[&aba]), "posição {side} planificada: {:?}", plana.status[&aba]);
        let (_, hip) = caixa_da_malha(&plana);
        assert!(hip[2] < 2.1, "posição {side}: planificada fica plana (z máximo {})", hip[2]);
        // a web é relimitada pelo suporte (~11,9 mil mm³ medidos com 20 mm) e a faixa da aba vem em cima
        let vol_plana = volume_total(&plana);
        assert!(vol_plana > 11_000. && vol_plana < 14_000., "posição {side}: volume planificado {vol_plana}");
        // o volume do sólido dobrado e o do planificado diferem só pelo ajuste do fator K (poucos %)
        let vol_dobrada = volume_total(&dobrada);
        assert!((vol_dobrada - vol_plana).abs() / vol_plana < 0.08, "posição {side}: dobrada {vol_dobrada}, planificada {vol_plana}");
    }
}

/// Alternar dobrado e planificado depois do cálculo conjunto não recalcula nenhum comando.
#[test]
fn alternar_dobrado_e_planificado_usa_o_cache_sem_recalcular() {
    if !kernel_disponivel() {
        return;
    }
    let (d, aba, _) = nervura("20", 0);
    let n_comandos = d.features.len();
    let mut cache = Cache::default();
    let t0 = Instant::now();
    let dobrada = recompute_ambos(&d, &mut cache);
    let conjunto = ms(t0);
    assert!(ok_ou_conformacao(&dobrada.status[&aba]));
    let vol_dobrada = volume_total(&dobrada);

    // referência independente: o planificado calculado do zero
    let ref_plana = recompute(&com_flat(&d, true));
    let vol_plana = volume_total(&ref_plana);

    let (mut max_ms, mut soma) = (0f64, 0f64);
    let voltas = 6;
    for i in 0..voltas {
        let flat = i % 2 == 0;
        let doc = com_flat(&d, flat);
        let t = Instant::now();
        let r = recompute_cached(&doc, &mut cache);
        let dt = ms(t);
        max_ms = max_ms.max(dt);
        soma += dt;
        assert_eq!(cache.reused, n_comandos, "volta {i}: todos os comandos saem do cache (sem recálculo)");
        assert!(ok_ou_conformacao(&r.status[&aba]), "volta {i}");
        let esperado = if flat { vol_plana } else { vol_dobrada };
        assert!((volume_total(&r) - esperado).abs() < 1e-6 * esperado, "volta {i}: volume {} esperado {esperado}", volume_total(&r));
        let (_, hi) = caixa_da_malha(&r);
        assert_eq!(hi[2] < 2.1, flat, "volta {i}: a exibição é a do estado pedido");
    }
    println!("B566 conjunto (dobrado + planificado): {conjunto:.0} ms; alternar: média {:.1} ms, máximo {max_ms:.1} ms ({voltas} voltas)", soma / voltas as f64);
    // medido neste PC: poucos ms (a exibição vem de malha em cache). A meta do item é 50 ms; a
    // afirmação tem folga para máquina carregada
    assert!(max_ms < 250., "alternar levou {max_ms:.1} ms");
    assert!(cache.trocas_de_estado >= voltas, "cada alternância troca o conjunto guardado");
}

/// Mudou algo a montante: a alternância recalcula só o que depende, nos dois estados, e o resultado
/// é o mesmo de um cálculo do zero.
#[test]
fn mudar_a_montante_invalida_so_o_que_depende_nos_dois_estados() {
    if !kernel_disponivel() {
        return;
    }
    let (d, aba, _) = nervura("20", 0);
    let mut cache = Cache::default();
    recompute_ambos(&d, &mut cache);

    // nome e cor não invalidam nada
    let mut renomeado = d.clone();
    renomeado.feature_mut(aba).unwrap().name = s("Nervura");
    let _ = recompute_cached(&renomeado, &mut cache);
    assert_eq!(cache.reused, d.features.len(), "renomear não invalida");

    // comprimento da aba: só a aba refaz (a web e o sketch vêm do cache), nos dois estados
    let mut mais_curta = d.clone();
    if let Some(FeatureKind::SmSurfFlange(x)) = mais_curta.feature_mut(aba).map(|f| &mut f.kind) {
        x.length = s("12");
    }
    let nova = recompute_cached(&mais_curta, &mut cache);
    assert_eq!(cache.reused, d.features.len() - 1, "só a aba refaz");
    assert!(ok_ou_conformacao(&nova.status[&aba]));
    let plana_nova = recompute_cached(&com_flat(&mais_curta, true), &mut cache);
    assert_eq!(cache.reused, d.features.len() - 1, "o planificado antigo (20 mm) não vale: só a aba refaz");
    let zero_dobrada = recompute(&mais_curta);
    let zero_plana = recompute(&com_flat(&mais_curta, true));
    assert!((volume_total(&nova) - volume_total(&zero_dobrada)).abs() < 1e-6, "dobrada igual à do zero");
    assert!((volume_total(&plana_nova) - volume_total(&zero_plana)).abs() < 1e-6, "planificada igual à do zero");
    // e agora, preparado de novo, alternar volta a ser só troca
    assert!(preaquecer_outro_estado(&com_flat(&mais_curta, true), &mut cache));
    let _ = recompute_cached(&mais_curta, &mut cache);
    assert_eq!(cache.reused, d.features.len(), "tudo pronto de novo");

    // espessura da chapa (vale para o documento todo): refaz tudo
    let mut grossa = mais_curta.clone();
    grossa.sheet.as_mut().unwrap().thickness = s("2.5");
    let _ = recompute_cached(&grossa, &mut cache);
    assert_eq!(cache.reused, 0, "espessura nova refaz tudo");
}

/// Documento sem chapa não gasta tempo com o outro estado.
#[test]
fn sem_chapa_nao_ha_outro_estado() {
    let mut d = Document::new("bloco");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [10., 10.]);
    d.add_feature(ext(sk, BoolOp::Add, "5"));
    let mut cache = Cache::default();
    assert!(!preaquecer_outro_estado(&d, &mut cache));
    let r = recompute_ambos(&d, &mut cache);
    assert_eq!(cache.trocas_de_estado, 0);
    if kernel_disponivel() {
        assert!((volume_total(&r) - 500.).abs() < 1e-6);
    }
}
