// SPDX-License-Identifier: LGPL-2.1-or-later
//! B95: faces escolhidas por regra (tangentes, concêntricas) gravadas na casca, na saída e na edição
//! direta, e refeitas no recompute: a regra acompanha o modelo, a lista fixa não.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn ok(r: &fenkra_solidos::RecomputeResult, id: Id) {
    assert!(matches!(r.status[&id], Status::Ok), "{:?}", r.status[&id]);
}

/// Bloco 40 × 40 × 20 com dois ressaltos coaxiais em (20, 20): raio 8 até z = 25 e raio 5 até
/// z = 30, gravado em STEP e lido de volta (a edição direta é para corpo sem histórico).
fn bloco_ressaltos(nome: &str) -> Document {
    let mut d = Document::new("t");
    let xy = PlaneRef::Origin(OriginPlane::XY);
    let sk = sk_rect(&mut d, xy, [0., 0.], [40., 40.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let grande = sk_circulos(&mut d, xy, &[([20., 20.], 8.)]);
    d.add_feature(ext(grande, BoolOp::Add, "25"));
    let fino = sk_circulos(&mut d, xy, &[([20., 20.], 5.)]);
    d.add_feature(ext(fino, BoolOp::Add, "30"));
    let dir = std::env::temp_dir().join(format!("fenkra_b95_{nome}"));
    std::fs::create_dir_all(&dir).unwrap();
    let alvo = dir.join("peca.step");
    fenkra_solidos::final_shape(&d).unwrap().export_step(alvo.to_str().unwrap()).unwrap();
    let imp = fenkra_solidos::importacao::importar_step(alvo.to_str().unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(dir);
    imp.doc
}

fn deslocar(regras: Vec<RegraAresta>) -> FeatureKind {
    let mut e = DirectEdit::novo(OpDireta::Deslocar);
    e.faces = vec![[25., 20., 28.]];
    e.value = s("1");
    e.regras = regras;
    FeatureKind::DirectEdit(e)
}

const CONC: RegraAresta = RegraAresta { semente: 0, regra: RegraSelecao::Concentricas };
const TANG: RegraAresta = RegraAresta { semente: 0, regra: RegraSelecao::Tangentes };

#[test]
fn deslocar_por_concentricas_leva_o_ressalto_coaxial_junto() {
    if !kernel_disponivel() {
        return;
    }
    let d0 = bloco_ressaltos("conc");
    let v0 = recompute(&d0).bodies[0].mass.volume;
    // só a face clicada: o ressalto fino muda
    let mut d = d0.clone();
    let so = d.add_feature(deslocar(vec![]));
    let r = recompute(&d);
    ok(&r, so);
    let dv_so = (r.bodies[0].mass.volume - v0).abs();
    // pela regra: os dois ressaltos coaxiais mudam juntos
    let mut d = d0.clone();
    let regra = d.add_feature(deslocar(vec![CONC]));
    let r = recompute(&d);
    ok(&r, regra);
    let dv_regra = (r.bodies[0].mass.volume - v0).abs();
    // raio 5 vira 6 em 5 mm de altura: π·(36 − 25)·5 ≈ 172,8; raio 8 vira 9: π·(81 − 64)·5 ≈ 267
    let (a, b) = (std::f64::consts::PI * 11. * 5., std::f64::consts::PI * 17. * 5.);
    assert!((dv_so - a).abs() < 2., "só o ressalto fino mudou {dv_so}, esperado {a}");
    assert!((dv_regra - a - b).abs() < 3., "os dois ressaltos mudaram {dv_regra}, esperado {}", a + b);
    let (lo, hi) = caixa_da_malha(&r);
    assert!((hi[0] - lo[0] - 40.).abs() < 0.01 && (hi[2] - lo[2] - 30.).abs() < 0.01, "{lo:?} {hi:?}");
}

/// Bloco 40 × 40 × 20 com as quatro arestas verticais arredondadas (r = 5) e saída de 3° pedida a
/// partir de uma parede. Devolve o documento, o filete e a saída.
fn bloco_com_saida(regras: Vec<RegraAresta>) -> (Document, Id, Id) {
    let mut d = Document::new("t");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [40., 40.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let fil = d.add_feature(filete("5", vec![[0., 0., 10.], [40., 0., 10.], [0., 40., 10.], [40., 40., 10.]], vec![]));
    let id = d.add_feature(FeatureKind::Draft(Draft { faces: vec![[20., 0., 10.]], neutral: PlaneRef::Origin(OriginPlane::XY), angle: s("3"), reverse: false, face_keys: vec![], regras }));
    (d, fil, id)
}

#[test]
fn regra_refeita_quando_nascem_cantos_tangentes() {
    if !kernel_disponivel() {
        return;
    }
    // sem o filete as paredes têm quina: a regra só acha a parede clicada e a saída inclina só ela
    let (mut d, fil, id) = bloco_com_saida(vec![TANG]);
    d.feature_mut(fil).unwrap().suppressed = true;
    let r = recompute(&d);
    ok(&r, id);
    let so_uma = 40. * 20f64.powi(2) * 3f64.to_radians().tan() / 2.;
    assert!((r.bodies[0].mass.volume - 40. * 40. * 20.).abs() - so_uma < so_uma * 0.1, "{}", r.bodies[0].mass.volume);
    // o filete volta: os cantos nascem tangentes à parede e a regra leva o contorno inteiro
    d.feature_mut(fil).unwrap().suppressed = false;
    let r = recompute(&d);
    ok(&r, id);
    let topo = r.bodies[0].mesh.face_info.len();
    // o contorno inteiro (4 paredes e 4 cantos de 20 de altura) inclina: volume perde ≈ perímetro · 20² · tg 3° / 2
    let perimetro = 4. * 30. + 2. * std::f64::consts::PI * 5.;
    let v_fil = 40. * 40. * 20. - 4. * (1. - std::f64::consts::PI / 4.) * 25. * 20.;
    let esperado = perimetro * 20f64.powi(2) * 3f64.to_radians().tan() / 2.;
    let dv = (r.bodies[0].mass.volume - v_fil).abs();
    assert!((dv - esperado).abs() < esperado * 0.25, "variação {dv}, esperado ≈ {esperado}");
    assert!(topo >= 10, "faces depois da saída: {topo}");
}

#[test]
fn saida_por_tangentes_inclina_paredes_e_cantos() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("t");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [40., 40.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    d.add_feature(filete("5", vec![[0., 0., 10.], [40., 0., 10.], [0., 40., 10.], [40., 40., 10.]], vec![]));
    let v0 = recompute(&d).bodies[0].mass.volume;
    let id = d.add_feature(FeatureKind::Draft(Draft {
        faces: vec![[20., 0., 10.]],
        neutral: PlaneRef::Origin(OriginPlane::XY),
        angle: s("3"),
        reverse: false,
        face_keys: vec![],
        regras: vec![RegraAresta { semente: 0, regra: RegraSelecao::Tangentes }],
    }));
    let r = recompute(&d);
    ok(&r, id);
    // as 4 paredes e os 4 cantos inclinam: o volume muda em todo o contorno (≈ perímetro · 20² · tg 3° / 2)
    let dv = (r.bodies[0].mass.volume - v0).abs();
    let perimetro = 4. * 30. + 2. * std::f64::consts::PI * 5.;
    let esperado = perimetro * 20f64.powi(2) * 3f64.to_radians().tan() / 2.;
    assert!((dv - esperado).abs() < esperado * 0.25, "variação {dv}, esperado ≈ {esperado}");
    let (lo, hi) = caixa_da_malha(&r);
    assert!((hi[2] - lo[2] - 20.).abs() < 0.01, "{lo:?} {hi:?}");
}

#[test]
fn casca_com_regra_em_face_plana_abre_so_a_semente() {
    if !kernel_disponivel() {
        return;
    }
    let caixa = |regras: Vec<RegraAresta>| {
        let mut d = Document::new("t");
        let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [60., 40.]);
        d.add_feature(ext(sk, BoolOp::Add, "30"));
        let id = d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[30., 20., 30.]], thickness: s("2"), outward: false, face_keys: vec![], closed: false, regras }));
        (d, id)
    };
    let (d, id) = caixa(vec![]);
    let r0 = recompute(&d);
    ok(&r0, id);
    // o topo tem quina com as paredes: a regra de tangentes não acha mais nada e a casca é a mesma
    let (d, id) = caixa(vec![RegraAresta { semente: 0, regra: RegraSelecao::Tangentes }]);
    let r = recompute(&d);
    ok(&r, id);
    assert!((r.bodies[0].mass.volume - r0.bodies[0].mass.volume).abs() < 1e-6);
}

#[test]
fn arquivo_antigo_sem_regra_abre() {
    let sh: Shell = serde_json::from_str(r#"{"faces":[[0,0,1]],"thickness":"2","outward":false}"#).unwrap();
    assert!(sh.regras.is_empty());
    let novo = serde_json::to_string(&sh).unwrap();
    assert!(!novo.contains("regras"), "lista vazia não é gravada: {novo}");
}
