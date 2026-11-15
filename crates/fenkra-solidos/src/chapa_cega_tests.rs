// SPDX-License-Identifier: LGPL-2.1-or-later
//! B40: dobra cega com furo de alívio: montada no documento, recalculada e conferida dobrada e plana.

use super::*;
use std::f64::consts::PI;

fn s(x: &str) -> String {
    x.to_string()
}

/// Chapa 100 × 50 × 2 (r = 2, K = 0,44) com uma dobra de 90° na linha x = 60, de y = `y0` a `y1`.
fn chapa_com_dobra_cega(y0: f64, y1: f64, relief: &str, flip: bool) -> Document {
    let mut d = Document::new("cega");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let mut data = SketchData::default();
    let mut n = 7_100_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [0., 0.], [100., 50.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    d.add_feature(FeatureKind::SmBend(SmBend {
        a: [60., y0, 2.],
        b: [60., y1, 2.],
        angle: s("90"),
        radius: None,
        flip,
        down: false,
        sketch: None,
        position: 0,
        cega: Some(DobraCega { relief: s(relief) }),
    }));
    d
}

fn confere(d: &Document) -> RecomputeResult {
    let r = recompute(d);
    for f in &d.features {
        assert!(matches!(r.status[&f.id], Status::Ok | Status::Warning(_)), "{}: {:?}", f.name, r.status[&f.id]);
    }
    r
}

fn caixa(r: &RecomputeResult) -> ([f64; 3], [f64; 3]) {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for p in r.bodies[0].mesh.positions.chunks_exact(3) {
        for j in 0..3 {
            lo[j] = lo[j].min(p[j] as f64);
            hi[j] = hi[j].max(p[j] as f64);
        }
    }
    (lo, hi)
}

#[test]
fn alivio_minimo_segue_a_regra() {
    use chapa_cega::diametro_alivio;
    assert_eq!(diametro_alivio(2., 2., None), (6., false));
    assert_eq!(diametro_alivio(0.5, 0.5, None), (3., false));
    assert_eq!(diametro_alivio(2., 2., Some(8.)), (8., false));
    assert_eq!(diametro_alivio(2., 2., Some(4.)), (6., true), "furo pequeno sobe ao mínimo");
}

#[test]
fn dobra_cega_gira_so_a_faixa_e_a_planificada_bate() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // aba móvel: o lado x < 60 (flip = false), faixa y de 10 a 40
    let d = chapa_com_dobra_cega(10., 40., "", false);
    let dobrada = confere(&d);
    let v_dobrada = dobrada.bodies[0].mass.volume;
    let (lo, hi) = caixa(&dobrada);
    // a faixa sobe (o lado móvel tem 60 mm) e o resto da chapa continua plano: z cobre bem mais que
    // a espessura, mas só dentro da faixa; fora dela (y < 7 ou y > 43) a chapa segue a 2 mm
    assert!(hi[2] > 40., "faixa levantada: z até {}", hi[2]);
    assert!(lo[2].abs() < 1e-3);
    let fora_da_faixa = dobrada.bodies[0].mesh.positions.chunks_exact(3).filter(|p| p[1] < 6.5 || p[1] > 43.5).map(|p| p[2]).fold(f32::MIN, f32::max);
    assert!(fora_da_faixa < 2.01, "fora da faixa a chapa fica plana: z {fora_da_faixa}");
    // planificada: só os furos de alívio (Ø6 em cada ponta da linha), volume exato da chapa
    let mut dp = d.clone();
    dp.sheet.as_mut().unwrap().flat = true;
    let plana = confere(&dp);
    let v_plana = plana.bodies[0].mass.volume;
    let esperado = 100. * 50. * 2. - 2. * PI * 9. * 2.;
    assert!((v_plana - esperado).abs() < 0.01 * esperado, "plana {v_plana} contra {esperado}");
    let (plo, phi) = caixa(&plana);
    assert!(phi[2] < 2.01 && plo[2] > -0.01, "planificada é plana");
    assert!((phi[0] - 100.).abs() < 1e-3 && (phi[1] - 50.).abs() < 1e-3, "contorno plano {phi:?}");
    // a faixa dobrada conserva o volume da plana (compensação de dobra), dentro de 2 %
    assert!((v_dobrada - v_plana).abs() < 0.02 * v_plana, "dobrada {v_dobrada} contra plana {v_plana}");
}

#[test]
fn furo_pedido_maior_vira_o_diametro_do_alivio() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = chapa_com_dobra_cega(10., 40., "8", false);
    d.sheet.as_mut().unwrap().flat = true;
    let r = confere(&d);
    let esperado = 100. * 50. * 2. - 2. * PI * 16. * 2.;
    assert!((r.bodies[0].mass.volume - esperado).abs() < 0.01 * esperado, "{} contra {esperado}", r.bodies[0].mass.volume);
}

#[test]
fn linha_mais_curta_que_o_furo_da_erro_claro() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let d = chapa_com_dobra_cega(10., 14., "", false);
    let r = recompute(&d);
    let id = d.features.last().unwrap().id;
    assert!(matches!(&r.status[&id], Status::Error(m) if m.contains("alívio")), "{:?}", r.status[&id]);
}

#[test]
fn sem_cega_a_dobra_continua_atravessando_a_chapa() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // a mesma linha curta, sem o modo cega: a chapa inteira do lado móvel gira (comportamento anterior)
    let mut d = chapa_com_dobra_cega(10., 40., "", false);
    if let FeatureKind::SmBend(b) = &mut d.features.last_mut().unwrap().kind {
        b.cega = None;
    }
    let r = confere(&d);
    let fora = r.bodies[0].mesh.positions.chunks_exact(3).filter(|p| p[1] < 6.5).map(|p| p[2]).fold(f32::MIN, f32::max);
    assert!(fora > 40., "sem o modo cega a chapa toda gira: z {fora}");
}

#[test]
fn alivio_de_canto_aeronautico_nunca_fica_abaixo_do_minimo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let volume = |kind: u8, tam: &str| {
        let mut d = Document::new("alivio");
        d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
        let mut data = SketchData::default();
        let mut n = 7_200_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, [0., 0.], [100., 50.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
        d.add_feature(FeatureKind::SmCornerRelief(SmCornerRelief { at: [100., 50., 2.], size: s(tam), kind }));
        confere(&d).bodies[0].mass.volume
    };
    let (v3, v0_pequeno, v0_minimo) = (volume(3, "3"), volume(0, "3"), volume(0, "6"));
    assert!((v3 - v0_minimo).abs() < 0.5, "aeronáutico de 3 mm sobe ao mínimo de 6 mm: {v3} contra {v0_minimo}");
    assert!(v3 < v0_pequeno - 5., "e tira mais material que o furo de 3 mm: {v3} contra {v0_pequeno}");
    assert!((volume(3, "10") - volume(0, "10")).abs() < 0.5, "acima do mínimo vale o tamanho pedido");
}
