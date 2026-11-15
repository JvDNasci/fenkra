// SPDX-License-Identifier: LGPL-2.1-or-later
//! B464: Deslocar face numa peça com boss e filete feitos por histórico (sem passar por STEP).
//!
//! Causa: o offset do OpenCASCADE só fecha o resultado em sólido quando a entrada é do tipo SOLID.
//! A união do boss com a base devolve um COMPOUND com o sólido dentro, e o offset devolvia a casca
//! solta ("não deixou um sólido fechado"). Lida de STEP, a mesma peça chega como SOLID e passava.
//! Cada teste monta a peça em código, recalcula e confere status, volume e caixa envolvente; o mesmo
//! deslocamento na peça vinda de STEP tem de dar o mesmo resultado.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::importacao::importar_step;
use fenkra_solidos::{final_shape, recompute, Status};
use std::f64::consts::PI;

/// Caixa 40x30x20 (z de 0 a 20), filete r=3 no canto vertical (20,15) e boss r=5 até z=30 em (0,0).
fn peca() -> Document {
    let mut d = Document::new("peca");
    let xy = PlaneRef::Origin(OriginPlane::XY);
    let sk = sk_rect(&mut d, xy, [-20., -15.], [20., 15.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    d.add_feature(filete("3", vec![[20., 15., 10.]], vec![None]));
    let sb = sk_circulos(&mut d, xy, &[([0., 0.], 5.)]);
    d.add_feature(ext(sb, BoolOp::Add, "30"));
    let r = recompute(&d);
    assert!(problema_de_status(&d, &r).is_none(), "peça de partida: {:?}", problema_de_status(&d, &r));
    d
}

/// A mesma peça gravada em STEP e lida de volta (sem histórico).
fn peca_step(nome: &str) -> Document {
    let d = peca();
    let dir = std::env::temp_dir().join(format!("solidos_b464_{nome}"));
    std::fs::create_dir_all(&dir).unwrap();
    let alvo = dir.join("peca.step");
    final_shape(&d).unwrap().export_step(alvo.to_str().unwrap()).unwrap();
    let imp = importar_step(alvo.to_str().unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(dir);
    imp.doc
}

fn base() -> f64 {
    40. * 30. * 20. - (1. - PI / 4.) * 9. * 20. + PI * 25. * 10.
}

fn edita(d: &mut Document, faces: &[[f64; 3]], value: &str) -> Id {
    let mut e = DirectEdit::novo(OpDireta::Deslocar);
    e.faces = faces.to_vec();
    e.value = value.into();
    e.delta = ["0", "0", "0"].map(String::from);
    d.add_feature(FeatureKind::DirectEdit(e))
}

/// Desloca nas duas versões da peça; devolve (volume, caixa) da versão por histórico depois de
/// conferir que as duas deram Ok, o mesmo volume e a mesma caixa.
fn desloca_nas_duas(nome: &str, faces: &[[f64; 3]], value: &str) -> (f64, [f64; 3], [f64; 3]) {
    let mut medidas = vec![];
    for (origem, mut d) in [("histórico", peca()), ("STEP", peca_step(nome))] {
        let f = edita(&mut d, faces, value);
        let r = recompute(&d);
        assert!(matches!(r.status[&f], Status::Ok), "{nome} ({origem}): {:?}", r.status[&f]);
        let s = final_shape(&d).unwrap();
        assert!(s.is_solid() && s.is_valid(), "{nome} ({origem}): sólido inválido");
        let (lo, hi) = caixa_envolvente(&d);
        medidas.push((volume_total(&r), lo, hi));
    }
    let (a, b) = (medidas[0], medidas[1]);
    assert!((a.0 - b.0).abs() < 0.5, "{nome}: volume {} (histórico) vs {} (STEP)", a.0, b.0);
    for i in 0..3 {
        assert!((a.1[i] - b.1[i]).abs() < 1e-3 && (a.2[i] - b.2[i]).abs() < 1e-3, "{nome}: caixa {a:?} vs {b:?}");
    }
    a
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn deslocar_a_lateral_da_peca_feita_por_historico() {
    if !kernel_disponivel() {
        return;
    }
    // a face x = -20 anda 5 mm para fora
    let (v, lo, hi) = desloca_nas_duas("lateral", &[[-20., 0., 10.]], "5");
    assert!(perto(v, base() + 5. * 30. * 20., 1.), "volume {v}");
    assert!(perto(lo[0], -25., 1e-3) && perto(hi[0], 20., 1e-3) && perto(hi[2], 30., 1e-3), "{lo:?} {hi:?}");
    // para dentro também
    let (v, lo, _) = desloca_nas_duas("lateral_dentro", &[[-20., 0., 10.]], "-2");
    assert!(perto(v, base() - 2. * 30. * 20., 1.), "volume {v}");
    assert!(perto(lo[0], -18., 1e-3), "{lo:?}");
}

#[test]
fn deslocar_o_topo_e_a_lateral_do_boss() {
    if !kernel_disponivel() {
        return;
    }
    // topo do boss sobe 3 mm
    let (v, lo, hi) = desloca_nas_duas("boss_topo", &[[0., 0., 30.]], "3");
    assert!(perto(v, base() + PI * 25. * 3., 1.), "volume {v}");
    assert!(perto(hi[2], 33., 1e-3) && perto(lo[2], 0., 1e-3), "{lo:?} {hi:?}");
    // lateral do boss: o raio passa de 5 para 7
    let (v, _, hi) = desloca_nas_duas("boss_lateral", &[[5., 0., 25.]], "2");
    assert!(perto(v, base() + PI * (49. - 25.) * 10., 1.), "volume {v}");
    assert!(perto(hi[2], 30., 1e-3), "{hi:?}");
}

#[test]
fn deslocar_o_topo_da_base_em_volta_do_boss() {
    if !kernel_disponivel() {
        return;
    }
    // o topo da base sobe 2 mm: o boss fica 2 mm mais curto acima dela, a altura total não muda
    let (v, _, hi) = desloca_nas_duas("topo_base", &[[-10., -10., 20.]], "2");
    let area_topo = 40. * 30. - (1. - PI / 4.) * 9. - PI * 25.;
    assert!(perto(v, base() + area_topo * 2., 1.), "volume {v}");
    assert!(perto(hi[2], 30., 1e-3), "{hi:?}");
}

#[test]
fn corpo_com_dois_solidos_desloca_so_o_escolhido() {
    if !kernel_disponivel() {
        return;
    }
    // dois blocos separados no mesmo corpo (COMPOUND com dois sólidos): só o da esquerda cresce
    let mut d = Document::new("dois");
    let xy = PlaneRef::Origin(OriginPlane::XY);
    let sk = sk_rect(&mut d, xy, [-30., -5.], [-10., 5.]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    let sk2 = sk_rect(&mut d, xy, [10., -5.], [30., 5.]);
    d.add_feature(ext(sk2, BoolOp::Add, "10"));
    let r = recompute(&d);
    assert!(problema_de_status(&d, &r).is_none(), "{:?}", problema_de_status(&d, &r));
    let v0 = volume_total(&r);
    let f = edita(&mut d, &[[-30., 0., 5.]], "4");
    let r = recompute(&d);
    assert!(matches!(r.status[&f], Status::Ok), "{:?}", r.status[&f]);
    let (lo, hi) = caixa_envolvente(&d);
    assert!(perto(volume_total(&r), v0 + 4. * 10. * 10., 0.5), "volume {}", volume_total(&r));
    assert!(perto(lo[0], -34., 1e-3) && perto(hi[0], 30., 1e-3), "{lo:?} {hi:?}");
}
