// SPDX-License-Identifier: LGPL-2.1-or-later
//! B289: filete e chanfro com propagação tangente, parcial, assimétrico, inversão por aresta e erro
//! nomeado. Cada modo confere volume (fórmula fechada) e caixa envolvente.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};
use std::f64::consts::PI;

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

fn s(x: &str) -> String {
    x.into()
}

/// Caixa 40 × 20 × 10 com o canto em (0,0,0).
fn caixa() -> Document {
    let mut d = Document::new("caixa");
    let sk = sk_rect(&mut d, XY, [0., 0.], [40., 20.]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    d
}

/// Aresta de cima do lado y = 20, ao longo de x (entre a face de cima e a face do fundo da caixa).
const TOPO: [f64; 3] = [20., 20., 10.];
const V_CAIXA: f64 = 40. * 20. * 10.;

fn op(valor: &str, mode: EdgeMode, mids: Vec<[f64; 3]>) -> EdgeOp {
    EdgeOp { value: s(valor), edges: EdgeSel::Edges(mids), mode, prop: EdgeProp::Tangente, keys: vec![], extra: Default::default() }
}

fn medir(d: &Document) -> fenkra_solidos::RecomputeResult {
    let r = recompute(d);
    if let Some(p) = problema_de_status(d, &r) {
        panic!("comando com problema: {p}");
    }
    r
}

fn perto(a: f64, b: f64, tol: f64, nome: &str) {
    assert!((a - b).abs() <= tol * b.abs().max(1.), "{nome}: {a:.5} esperado {b:.5}");
}

fn caixa_ok(d: &Document) {
    let (lo, hi) = caixa_envolvente(d);
    for (i, t) in [40., 20., 10.].iter().enumerate() {
        perto(hi[i] - lo[i], *t, 1e-6, "caixa envolvente");
    }
}

#[test]
fn filete_constante_tira_o_quarto_de_circulo() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = caixa();
    d.add_feature(FeatureKind::Fillet(op("3", EdgeMode::Constant, vec![TOPO])));
    let r = medir(&d);
    perto(volume_total(&r), V_CAIXA - (1. - PI / 4.) * 9. * 40., 1e-4, "volume");
    caixa_ok(&d);
}

#[test]
fn chanfro_de_duas_distancias_e_inversao_por_aresta() {
    if !kernel_disponivel() {
        return;
    }
    let mk = |flip: bool| {
        let mut d = caixa();
        let mut o = op("5", EdgeMode::TwoDist(s("2")), vec![TOPO]);
        o.extra.flips = vec![flip];
        d.add_feature(FeatureKind::Chamfer(o));
        d
    };
    let (a, b) = (mk(false), mk(true));
    let (ra, rb) = (medir(&a), medir(&b));
    // o triângulo tirado vale d1·d2/2 por mm, de um jeito ou do outro
    perto(volume_total(&ra), V_CAIXA - 5. * 2. / 2. * 40., 1e-6, "volume sem inverter");
    perto(volume_total(&rb), V_CAIXA - 5. * 2. / 2. * 40., 1e-6, "volume invertido");
    caixa_ok(&a);
    caixa_ok(&b);
    // a 1ª distância muda de face: o centro de massa muda
    let (ca, cb) = (ra.bodies[0].mass.center, rb.bodies[0].mass.center);
    assert!((ca[1] - cb[1]).abs() > 1e-3 && (ca[2] - cb[2]).abs() > 1e-3, "inverter não mudou nada: {ca:?} {cb:?}");
}

/// Existe vértice da malha em (y, z) (qualquer x)?
fn tem_vertice(r: &fenkra_solidos::RecomputeResult, y: f64, z: f64) -> bool {
    r.bodies[0].mesh.positions.chunks_exact(3).any(|p| (p[1] as f64 - y).abs() < 1e-3 && (p[2] as f64 - z).abs() < 1e-3)
}

#[test]
fn filete_assimetrico_e_inversao_por_seta() {
    if !kernel_disponivel() {
        return;
    }
    let mk = |flip: bool| {
        let mut d = caixa();
        let mut o = op("6", EdgeMode::Asym(s("2")), vec![TOPO]);
        o.extra.flips = vec![flip];
        d.add_feature(FeatureKind::Fillet(o));
        d
    };
    let (a, b) = (mk(false), mk(true));
    let (ra, rb) = (medir(&a), medir(&b));
    // quarto de elipse de semieixos 6 e 2: sai (1 − π/4)·6·2 por mm
    let esperado = V_CAIXA - (1. - PI / 4.) * 6. * 2. * 40.;
    perto(volume_total(&ra), esperado, 1e-4, "volume sem inverter");
    perto(volume_total(&rb), esperado, 1e-4, "volume invertido");
    caixa_ok(&a);
    caixa_ok(&b);
    // os pontos de tangência: 6 na face de cima (y = 14, z = 10) e 2 na lateral (y = 20, z = 8),
    // ou o contrário (y = 18, z = 10 e y = 20, z = 4)
    let (grande_em_cima, grande_na_lateral) = ((14., 10.), (20., 4.));
    let (pequeno_em_cima, pequeno_na_lateral) = ((18., 10.), (20., 8.));
    let a_cima = tem_vertice(&ra, grande_em_cima.0, grande_em_cima.1) && tem_vertice(&ra, pequeno_na_lateral.0, pequeno_na_lateral.1);
    let a_lat = tem_vertice(&ra, pequeno_em_cima.0, pequeno_em_cima.1) && tem_vertice(&ra, grande_na_lateral.0, grande_na_lateral.1);
    assert!(a_cima ^ a_lat, "sem inverter: tangências nem num lado nem no outro");
    let b_cima = tem_vertice(&rb, grande_em_cima.0, grande_em_cima.1) && tem_vertice(&rb, pequeno_na_lateral.0, pequeno_na_lateral.1);
    let b_lat = tem_vertice(&rb, pequeno_em_cima.0, pequeno_em_cima.1) && tem_vertice(&rb, grande_na_lateral.0, grande_na_lateral.1);
    assert!(b_cima ^ b_lat, "invertido: tangências nem num lado nem no outro");
    assert_ne!(a_cima, b_cima, "a seta de inversão não trocou o lado do raio maior");
}

#[test]
fn filete_assimetrico_em_quina_concava_preenche() {
    if !kernel_disponivel() {
        return;
    }
    // L: 40×10 de base e um braço 10×20 em cima; a quina de dentro (10,10) é côncava
    let mut d = Document::new("L");
    let sk = sk_poligono(&mut d, XY, &[[0., 0.], [40., 0.], [40., 10.], [10., 10.], [10., 30.], [0., 30.]]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let v0 = volume_total(&recompute(&d));
    d.add_feature(FeatureKind::Fillet(op("4", EdgeMode::Asym(s("2")), vec![[10., 10., 10.]])));
    let r = medir(&d);
    // o que entra na quina côncava é (1 − π/4)·4·2 por mm, ao longo dos 20 de altura
    perto(volume_total(&r), v0 + (1. - PI / 4.) * 8. * 20., 1e-4, "volume com o preenchimento");
    let (lo, hi) = caixa_envolvente(&d);
    perto(hi[0] - lo[0], 40., 1e-6, "caixa x");
    perto(hi[1] - lo[1], 30., 1e-6, "caixa y");
    perto(hi[2] - lo[2], 20., 1e-6, "caixa z");
}

#[test]
fn filete_variavel_acompanha_a_lei_linear() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = caixa();
    d.add_feature(FeatureKind::Fillet(op("1", EdgeMode::Variable(s("3")), vec![TOPO])));
    let r = medir(&d);
    // raio de 1 a 3 ao longo de 40: a área tirada vai com r², média (r1² + r1·r2 + r2²)/3
    let esperado = V_CAIXA - (1. - PI / 4.) * (1. + 3. + 9.) / 3. * 40.;
    perto(volume_total(&r), esperado, 0.02, "volume");
    // a caixa exata do kernel é folgada em superfícies livres (B-spline); a da malha é a que vale
    let (lo, hi) = caixa_da_malha(&r);
    for (i, t) in [40., 20., 10.].iter().enumerate() {
        perto(hi[i] - lo[i], *t, 1e-4, "caixa da malha");
    }
}

#[test]
fn filete_parcial_so_no_trecho_pedido() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = caixa();
    let mut o = op("3", EdgeMode::Constant, vec![TOPO]);
    o.extra.partial = Some([s("10"), s("30")]);
    d.add_feature(FeatureKind::Fillet(o));
    let r = medir(&d);
    // 20 dos 40 mm ficam arredondados, e o filete termina reto nos dois extremos
    let esperado = V_CAIXA - (1. - PI / 4.) * 9. * 20.;
    perto(volume_total(&r), esperado, 0.01, "volume parcial");
    caixa_ok(&d);
    // o trecho virou parâmetro: mudar o fim muda o volume
    let mut d2 = caixa();
    let mut o2 = op("3", EdgeMode::Constant, vec![TOPO]);
    o2.extra.partial = Some([s("10"), s("20")]);
    d2.add_feature(FeatureKind::Fillet(o2));
    let v2 = volume_total(&medir(&d2));
    perto(v2, V_CAIXA - (1. - PI / 4.) * 9. * 10., 0.01, "volume com o fim em 20");
}

#[test]
fn propagacao_tangente_e_minima_diferem_na_cadeia() {
    if !kernel_disponivel() {
        return;
    }
    // caixa com o canto vertical arredondado (R5): a aresta de cima segue tangente pelo arco
    let mk = |prop: EdgeProp| {
        let mut d = caixa();
        d.add_feature(FeatureKind::Fillet(op("5", EdgeMode::Constant, vec![[40., 20., 5.]])));
        let mut o = op("1", EdgeMode::Constant, vec![[17.5, 20., 10.]]);
        o.prop = prop;
        d.add_feature(FeatureKind::Fillet(o));
        d
    };
    let vt = volume_total(&medir(&mk(EdgeProp::Tangente)));
    let vm = volume_total(&medir(&mk(EdgeProp::Minima)));
    assert!(vm > vt + 1e-3, "a mínima deveria tirar menos que a cadeia tangente: {vm} × {vt}");
}

#[test]
fn raio_grande_demais_da_erro_nomeado_com_o_limite() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = caixa();
    let f = d.add_feature(FeatureKind::Fillet(op("50", EdgeMode::Constant, vec![TOPO])));
    let r = recompute(&d);
    let Some(Status::Error(msg)) = r.status.get(&f) else { panic!("esperava erro, veio {:?}", r.status.get(&f)) };
    assert!(msg.contains("filete não cabe"), "{msg}");
    assert!(msg.contains("raio 50"), "o texto deve dizer o valor pedido: {msg}");
    assert!(msg.contains("maior raio que cabe"), "o texto deve dizer o limite: {msg}");
    // a caixa tem 10 de altura: o limite medido não passa de 10
    let limite: f64 = msg.split("cabe nesta seleção é ≈").nth(1).and_then(|t| t.split(|c: char| !(c.is_ascii_digit() || c == '.')).next()).and_then(|t| t.parse().ok()).expect(&msg.clone());
    assert!(limite > 1. && limite <= 10.01, "limite {limite}: {msg}");
    // e o valor que o texto sugere funciona de fato
    let mut d2 = caixa();
    d2.add_feature(FeatureKind::Fillet(op(&format!("{}", limite * 0.95), EdgeMode::Constant, vec![TOPO])));
    medir(&d2);
}

#[test]
fn chanfro_grande_demais_nomeia_a_face_estreita() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = caixa();
    let f = d.add_feature(FeatureKind::Chamfer(op("30", EdgeMode::Constant, vec![TOPO])));
    let r = recompute(&d);
    let Some(Status::Error(msg)) = r.status.get(&f) else { panic!("esperava erro, veio {:?}", r.status.get(&f)) };
    assert!(msg.contains("chanfro não cabe"), "{msg}");
    assert!(msg.contains("face vizinha"), "o motivo deve nomear a face vizinha: {msg}");
}
