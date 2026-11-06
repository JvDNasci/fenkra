// SPDX-License-Identifier: LGPL-2.1-or-later
//! B33: edição direta de faces em corpo vindo de STEP (mover, apagar com cura, deslocar, raio de
//! filete). Cada teste monta a peça em código, grava em STEP, lê de volta (o corpo chega sem
//! histórico) e confere status, volume e caixa envolvente.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::importacao::importar_step;
use fenkra_solidos::{final_shape, recompute, Status};
use std::f64::consts::PI;

const CAIXA: f64 = 40. * 30. * 20.;
/// volume tirado pelo filete vertical de raio r no canto da caixa de altura 20
fn tira_filete(r: f64) -> f64 {
    (1. - PI / 4.) * r * r * 20.
}

/// Caixa 40x30x20 (z de 0 a 20) com filete r=3 no canto (20,15), um boss r=5 h=10 em (0,0) e uma
/// reentrância r=4 de 8 mm de fundo no lado de baixo, em (-10,0); gravada em STEP e lida de volta.
fn peca_step(nome: &str) -> Document {
    let mut d = Document::new("peca");
    let xy = PlaneRef::Origin(OriginPlane::XY);
    let sk = sk_rect(&mut d, xy, [-20., -15.], [20., 15.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let canto = d.add_feature(filete("3", vec![[20., 15., 10.]], vec![None]));
    let _ = canto;
    // cilindro de 30 de altura: acima da caixa (z 20 a 30) sobra o boss
    let sb = sk_circulos(&mut d, xy, &[([0., 0.], 5.)]);
    d.add_feature(ext(sb, BoolOp::Add, "30"));
    let sp = sk_circulos(&mut d, xy, &[([-10., 0.], 4.)]);
    let mut corte = ext(sp, BoolOp::Cut, "8");
    if let FeatureKind::Extrude(e) = &mut corte {
        e.reverse = true;
    }
    d.add_feature(corte);
    let r = recompute(&d);
    assert!(problema_de_status(&d, &r).is_none(), "peça de partida: {:?}", problema_de_status(&d, &r));
    let dir = std::env::temp_dir().join(format!("solidos5_b33_{nome}"));
    std::fs::create_dir_all(&dir).unwrap();
    let alvo = dir.join("peca.step");
    final_shape(&d).unwrap().export_step(alvo.to_str().unwrap()).unwrap();
    let imp = importar_step(alvo.to_str().unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(dir);
    imp.doc
}

fn base() -> f64 {
    CAIXA - tira_filete(3.) + PI * 25. * 10. - PI * 16. * 8.
}

fn edita(d: &mut Document, op: OpDireta, faces: &[[f64; 3]], value: &str, delta: [&str; 3]) -> Id {
    let mut e = DirectEdit::novo(op);
    e.faces = faces.to_vec();
    e.value = value.into();
    e.delta = delta.map(String::from);
    d.add_feature(FeatureKind::DirectEdit(e))
}

fn medida(d: &Document) -> (f64, [f64; 3], [f64; 3]) {
    let r = recompute(d);
    let (lo, hi) = caixa_envolvente(d);
    (volume_total(&r), lo, hi)
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn corpo_step_chega_sem_historico_com_o_volume_certo() {
    if !kernel_disponivel() {
        return;
    }
    let d = peca_step("base");
    let (v, lo, hi) = medida(&d);
    assert!(perto(v, base(), 0.5), "{v} vs {}", base());
    assert!(perto(lo[0], -20., 1e-3) && perto(hi[2], 30., 1e-3));
}

#[test]
fn apagar_o_boss_cura_e_volta_a_caixa() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = peca_step("apagar");
    // lateral e topo do boss: sem eles as vizinhas (o topo da caixa) se fecham
    let f = edita(&mut d, OpDireta::Apagar, &[[5., 0., 25.], [0., 0., 30.]], "0", ["0", "0", "0"]);
    let r = recompute(&d);
    assert!(matches!(r.status[&f], Status::Ok), "{:?}", r.status[&f]);
    let (v, lo, hi) = medida(&d);
    assert!(perto(v, base() - PI * 25. * 10., 0.5), "volume {v}");
    assert!(perto(hi[2], 20., 1e-3) && perto(lo[0], -20., 1e-3) && perto(hi[0], 20., 1e-3), "{lo:?} {hi:?}");
}

#[test]
fn apagar_a_reentrancia_preenche() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = peca_step("apagar_furo");
    let f = edita(&mut d, OpDireta::Apagar, &[[-6., 0., 4.], [-10., 0., 8.]], "0", ["0", "0", "0"]);
    let r = recompute(&d);
    assert!(matches!(r.status[&f], Status::Ok), "{:?}", r.status[&f]);
    let (v, _, _) = medida(&d);
    assert!(perto(v, base() + PI * 16. * 8., 0.5), "volume {v}");
}

#[test]
fn deslocar_face_plana_estende_a_caixa() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = peca_step("deslocar_plana");
    // a face x = -20 anda 5 mm para fora: a caixa passa de 40 para 45 de comprimento
    let f = edita(&mut d, OpDireta::Deslocar, &[[-20., 0., 10.]], "5", ["0", "0", "0"]);
    let r = recompute(&d);
    assert!(matches!(r.status[&f], Status::Ok), "{:?}", r.status[&f]);
    let (v, lo, hi) = medida(&d);
    assert!(perto(lo[0], -25., 1e-3) && perto(hi[0], 20., 1e-3), "{lo:?} {hi:?}");
    // a reentrância não some; o volume cresce 5 x 30 x 20 menos o que o filete e o boss já tinham
    assert!(perto(v, base() + 5. * 30. * 20., 1.), "volume {v}");
}

#[test]
fn deslocar_face_cilindrica_muda_o_raio_do_boss() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = peca_step("deslocar_cilindro");
    let f = edita(&mut d, OpDireta::Deslocar, &[[5., 0., 25.]], "2", ["0", "0", "0"]);
    let r = recompute(&d);
    assert!(matches!(r.status[&f], Status::Ok), "{:?}", r.status[&f]);
    let (v, _, hi) = medida(&d);
    assert!(perto(v, base() + PI * (49. - 25.) * 10., 1.), "volume {v}");
    assert!(perto(hi[2], 30., 1e-3));
}

#[test]
fn mover_o_boss_leva_o_relevo_junto() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = peca_step("mover_boss");
    let f = edita(&mut d, OpDireta::Mover, &[[5., 0., 25.], [0., 0., 30.]], "0", ["10", "5", "0"]);
    let r = recompute(&d);
    assert!(matches!(r.status[&f], Status::Ok), "{:?}", r.status[&f]);
    let (v, lo, hi) = medida(&d);
    assert!(perto(v, base(), 0.5), "o volume não muda ao mover: {v}");
    assert!(perto(hi[2], 30., 1e-3) && perto(lo[0], -20., 1e-3) && perto(hi[0], 20., 1e-3));
    // o boss agora está em (10, 5): a lateral dele passa por (15, 5, 25) e o lugar antigo ficou liso
    let s = final_shape(&d).unwrap();
    assert!(s.face_index([15., 5., 25.], 1e-3).unwrap().is_some(), "boss não chegou em (10, 5)");
    assert!(s.face_index([5., 0., 25.], 1.0).unwrap().is_none(), "sobrou face no lugar antigo");
}

#[test]
fn mover_a_reentrancia_leva_o_furo_cego() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = peca_step("mover_furo");
    let f = edita(&mut d, OpDireta::Mover, &[[-6., 0., 4.], [-10., 0., 8.]], "0", ["0", "6", "0"]);
    let r = recompute(&d);
    assert!(matches!(r.status[&f], Status::Ok), "{:?}", r.status[&f]);
    let (v, _, _) = medida(&d);
    assert!(perto(v, base(), 0.5), "o volume não muda ao mover: {v}");
    let s = final_shape(&d).unwrap();
    assert!(s.face_index([-10., 6., 8.], 1e-3).unwrap().is_some(), "fundo do furo não chegou em y = 6");
    assert!(s.face_index([-10., 0., 8.], 1.0).unwrap().is_none(), "o furo continua no lugar antigo");
}

#[test]
fn trocar_o_raio_do_filete() {
    if !kernel_disponivel() {
        return;
    }
    // ponto do meio do arco do filete de r=3 no canto (20,15): centro (17,12) + 3 na diagonal
    let k = 3. / 2f64.sqrt();
    let face = [17. + k, 12. + k, 10.];
    let mut d = peca_step("raio");
    let f = edita(&mut d, OpDireta::RaioFilete, &[face], "5", ["0", "0", "0"]);
    let r = recompute(&d);
    assert!(matches!(r.status[&f], Status::Ok), "{:?}", r.status[&f]);
    let (v, lo, hi) = medida(&d);
    assert!(perto(v, base() + tira_filete(3.) - tira_filete(5.), 0.5), "volume {v}");
    assert!(perto(hi[0], 20., 1e-3) && perto(hi[1], 15., 1e-3) && perto(lo[2], 0., 1e-3), "{lo:?} {hi:?}");
    // raio zero desfaz o filete: volta o canto vivo
    let mut d = peca_step("raio0");
    let f = edita(&mut d, OpDireta::RaioFilete, &[face], "0", ["0", "0", "0"]);
    let r = recompute(&d);
    assert!(matches!(r.status[&f], Status::Ok), "{:?}", r.status[&f]);
    let (v, _, _) = medida(&d);
    assert!(perto(v, base() + tira_filete(3.), 0.5), "volume {v}");
}

#[test]
fn recusa_sem_estragar_o_corpo() {
    if !kernel_disponivel() {
        return;
    }
    let sem_edicao = medida(&peca_step("recusa_ref")).0;
    // face plana não é filete
    let mut d = peca_step("recusa_plana");
    let f = edita(&mut d, OpDireta::RaioFilete, &[[-20., 0., 10.]], "2", ["0", "0", "0"]);
    let r = recompute(&d);
    assert!(matches!(r.status[&f], Status::Error(_)), "{:?}", r.status[&f]);
    assert!(perto(volume_total(&r), sem_edicao, 0.5), "o corpo anterior tem de ficar intacto");
    // deslocar para dentro mais do que a peça tem desfaz o corpo
    let mut d = peca_step("recusa_fundo");
    let f = edita(&mut d, OpDireta::Deslocar, &[[-20., 0., 10.]], "-80", ["0", "0", "0"]);
    let r = recompute(&d);
    assert!(matches!(r.status[&f], Status::Error(_)), "{:?}", r.status[&f]);
    // sem faces escolhidas nada é aplicado, só avisa
    let mut d = peca_step("recusa_vazia");
    let f = edita(&mut d, OpDireta::Apagar, &[], "0", ["0", "0", "0"]);
    let r = recompute(&d);
    assert!(matches!(r.status[&f], Status::Warning(_)), "{:?}", r.status[&f]);
    assert!(perto(volume_total(&r), sem_edicao, 0.5));
}
