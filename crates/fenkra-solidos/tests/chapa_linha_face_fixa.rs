// SPDX-License-Identifier: LGPL-2.1-or-later
//! B243: dobra por linha, joggle e veneziana pedem a face fixa e uma linha desenhada na face.
//!
//! A face fixa vira o `flip` do comando (`chapa_linha::flip_para_fixo`): o lado onde ela está fica
//! parado e o outro se move. Cada caso monta as três features nos dois lados, recalcula e confere
//! volume e caixa envolvente.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::chapa_linha::{flip_para_fixo, lado_movel};
use fenkra_solidos::{recompute, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn base() -> Document {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    d
}

fn ok(d: &Document) -> RecomputeResult {
    let r = recompute(d);
    for f in &d.features {
        assert_eq!(r.status[&f.id], Status::Ok, "{}: {:?}", f.name, r.status[&f.id]);
    }
    r
}

/// Vértices da malha: (x, z).
fn xz(r: &RecomputeResult) -> Vec<(f32, f32)> {
    r.bodies[0].mesh.positions.chunks_exact(3).map(|p| (p[0], p[2])).collect()
}

const NORMAL: [f64; 3] = [0., 0., 1.];

fn flip_de(a: [f64; 3], b: [f64; 3], fixo: [f64; 3]) -> bool {
    flip_para_fixo(a, b, NORMAL, fixo).expect("o ponto fixo decide o lado")
}

#[test]
fn face_fixa_decide_o_lado_da_dobra_por_linha() {
    if !kernel_disponivel() {
        return;
    }
    let (a, b) = ([60., 0., 2.], [60., 50., 2.]);
    // face fixa à esquerda: a parte da direita (40 mm) sobe
    let mut d = base();
    let flip = flip_de(a, b, [20., 25., 2.]);
    assert!(flip);
    assert_eq!(lado_movel(a, b, NORMAL, flip).unwrap(), [1., 0., 0.]);
    d.add_feature(FeatureKind::SmBend(SmBend { a, b, angle: s("90"), radius: None, flip, down: false, sketch: None, position: 0 }));
    let r = ok(&d);
    assert!((volume_total(&r) - 10000.).abs() < 20., "volume {}", volume_total(&r));
    let v = xz(&r);
    let zmax = v.iter().map(|p| p.1).fold(f32::MIN, f32::max);
    assert!((30. ..42.).contains(&zmax), "a parte móvel de 40 mm sobe: zmax {zmax}");
    assert!(v.iter().filter(|p| p.0 < 57.).all(|p| p.1 <= 2.01), "o lado fixo fica plano");
    let (xmin, xmax) = (v.iter().map(|p| p.0).fold(f32::MAX, f32::min), v.iter().map(|p| p.0).fold(f32::MIN, f32::max));
    assert!(xmin.abs() < 1e-3 && (60. ..70.).contains(&xmax), "caixa em x: {xmin}..{xmax}");

    // face fixa à direita: a parte da esquerda (60 mm) sobe, a direita fica plana
    let mut d = base();
    let flip = flip_de(a, b, [90., 25., 2.]);
    assert!(!flip);
    d.add_feature(FeatureKind::SmBend(SmBend { a, b, angle: s("90"), radius: None, flip, down: false, sketch: None, position: 0 }));
    let r = ok(&d);
    assert!((volume_total(&r) - 10000.).abs() < 20., "volume {}", volume_total(&r));
    let v = xz(&r);
    let zmax = v.iter().map(|p| p.1).fold(f32::MIN, f32::max);
    assert!((50. ..62.).contains(&zmax), "a parte móvel de 60 mm sobe: zmax {zmax}");
    assert!(v.iter().filter(|p| p.0 > 63.).all(|p| p.1 <= 2.01), "o lado fixo fica plano {:?}", v.iter().filter(|p| p.0 > 63. && p.1 > 2.01).take(5).collect::<Vec<_>>());
    assert!(v.iter().map(|p| p.0).fold(f32::MIN, f32::max) > 99.9, "a ponta do lado fixo continua em x = 100");
}

#[test]
fn face_fixa_decide_o_lado_do_joggle() {
    if !kernel_disponivel() {
        return;
    }
    let (a, b) = ([60., 0., 2.], [60., 50., 2.]);
    for (fixo, desce_x_maior) in [([20., 25., 2.], true), ([90., 25., 2.], false)] {
        let mut d = base();
        let flip = flip_de(a, b, fixo);
        d.add_feature(FeatureKind::SmJoggle(SmJoggle { a, b, depth: s("3"), run: s("5"), flip, angle: None, plane: None, keep_length: false }));
        let r = ok(&d);
        assert!((volume_total(&r) - 10000.).abs() < 10., "volume {}", volume_total(&r));
        let v = xz(&r);
        let zmin = v.iter().map(|p| p.1).fold(f32::MAX, f32::min);
        assert!((zmin + 3.).abs() < 0.01, "desvio de 3 mm: zmin {zmin}");
        let (parado, desce): (Box<dyn Fn(&(f32, f32)) -> bool>, Box<dyn Fn(&(f32, f32)) -> bool>) = if desce_x_maior { (Box::new(|p| p.0 < 59.), Box::new(|p| p.0 > 66.)) } else { (Box::new(|p| p.0 > 66.), Box::new(|p| p.0 < 59.)) };
        assert!(v.iter().filter(|p| parado(p)).all(|p| p.1 >= -0.01 && p.1 <= 2.01), "o lado fixo não desce");
        assert!(v.iter().filter(|p| desce(p)).all(|p| p.1 <= -0.99), "o lado móvel desce 3 mm");
    }
}

#[test]
fn face_fixa_decide_o_lado_da_veneziana() {
    if !kernel_disponivel() {
        return;
    }
    let (a, b) = ([30., 10., 2.], [30., 40., 2.]);
    // fixa à esquerda: a rampa sai para +x (12 mm de largura); fixa à direita: para -x
    for (fixo, x0, x1) in [([10., 25., 2.], 29.9f32, 42.1f32), ([60., 25., 2.], 17.9, 30.1)] {
        let mut d = base();
        let flip = flip_de(a, b, fixo);
        d.add_feature(FeatureKind::SmLouver(SmLouver { a, b, width: s("12"), height: s("5"), flip, down: false }));
        let r = ok(&d);
        assert!((volume_total(&r) - 10000.).abs() < 10., "volume {}", volume_total(&r));
        let v = xz(&r);
        let zmax = v.iter().map(|p| p.1).fold(f32::MIN, f32::max);
        assert!((zmax - 7.).abs() < 0.01, "abertura de 5 mm: zmax {zmax}");
        assert!(v.iter().filter(|p| p.1 > 2.01).all(|p| (x0..=x1).contains(&p.0)), "a rampa fica do lado móvel ({x0}..{x1})");
    }
}

/// O ponto sobre a linha não decide lado: a interface pede outro clique.
#[test]
fn ponto_sobre_a_linha_nao_decide_o_lado() {
    assert_eq!(flip_para_fixo([60., 0., 2.], [60., 50., 2.], NORMAL, [60., 10., 2.]), None);
    assert_eq!(flip_para_fixo([60., 0., 2.], [60., 0., 2.], NORMAL, [10., 10., 2.]), None);
}

/// Recém-criados, os três comandos ainda não têm linha (as duas pontas iguais). Isso é a espera
/// pela face fixa, não um erro: o recálculo avisa o que fazer e a chapa fica como estava (antes,
/// "os dois pontos da linha coincidem" deixava o comando em erro e a prévia inválida).
#[test]
fn sem_linha_ainda_espera_sem_erro() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let z = [0.; 3];
    for k in [
        FeatureKind::SmBend(SmBend { a: z, b: z, angle: s("90"), radius: None, flip: false, down: false, sketch: None, position: 0 }),
        FeatureKind::SmJoggle(SmJoggle { a: z, b: z, depth: s("3"), run: s("6"), flip: false, angle: None, plane: None, keep_length: false }),
        FeatureKind::SmLouver(SmLouver { a: z, b: z, width: s("10"), height: s("4"), flip: false, down: false }),
    ] {
        let mut d = base();
        let id = d.add_feature(k);
        let r = recompute(&d);
        assert!(matches!(&r.status[&id], Status::Warning(m) if m.contains("face fixa")), "{:?}", r.status[&id]);
        assert!((volume_total(&r) - 10000.).abs() < 1., "a chapa fica como estava: {}", volume_total(&r));
        let zmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max);
        assert!((zmax - 2.).abs() < 1e-3, "nada sai do plano: zmax {zmax}");
    }
}
