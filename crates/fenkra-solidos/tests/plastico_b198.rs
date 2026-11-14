// SPDX-License-Identifier: LGPL-2.1-or-later
//! B198: regra plástica do documento. Os testes montam a peça em código, recalculam e conferem
//! status, volume e caixa: a casca que cita a regra acompanha a mudança da regra, e o recurso que sai
//! dos limites avisa na árvore, mesmo quando só o limite mudou.

mod common;

use common::*;
use fenkra_core::regra_plastica::{RegraPlastica, VAR_ESPESSURA, VAR_NERVURA, VAR_SAIDA};
use fenkra_core::*;
use fenkra_solidos::{plastico, recompute, Status};
use std::collections::HashMap;

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

fn abs() -> RegraPlastica {
    RegraPlastica::de_fabrica().into_iter().find(|r| r.material == "ABS").unwrap()
}

/// Caixa 40 x 30 x 20 com casca aberta em cima.
fn caixa(espessura: &str, regra: Option<RegraPlastica>) -> (Document, Id) {
    let mut d = Document::new("caixa_plastica");
    d.plastico = regra;
    let a = sk_rect(&mut d, XY, [0., 0.], [40., 30.]);
    d.add_feature(ext(a, BoolOp::Add, "20"));
    let c = d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[20., 15., 20.]], thickness: espessura.into(), outward: false, face_keys: vec![], closed: false, regras: vec![] }));
    (d, c)
}

fn vol_casca(t: f64) -> f64 {
    40. * 30. * 20. - (40. - 2. * t) * (30. - 2. * t) * (20. - t)
}

#[test]
fn casca_que_cita_a_regra_acompanha_a_espessura_da_regra() {
    if !kernel_disponivel() {
        return;
    }
    let r = abs();
    let (mut d, _) = caixa(VAR_ESPESSURA, Some(r.clone()));
    confere("casca 2 mm", &d, &Esperado::prisma(vol_casca(r.espessura), [40., 30., 20.], [0.; 3]));
    // muda só a regra: a casca recalcula com a parede nova, sem tocar no comando
    d.plastico.as_mut().unwrap().espessura = 2.5;
    confere("casca 2,5 mm", &d, &Esperado::prisma(vol_casca(2.5), [40., 30., 20.], [0.; 3]));
    // a peça grava e lê a regra junto
    let t = serde_json::to_string(&d).unwrap();
    let lida: Document = serde_json::from_str(&t).unwrap();
    assert_eq!(lida.plastico, d.plastico);
    // documento antigo, sem o campo, abre sem regra
    let sem = serde_json::to_string(&Document::new("x")).unwrap();
    assert!(!sem.contains("plastico"));
}

#[test]
fn casca_abaixo_da_parede_minima_avisa_e_o_limite_novo_tira_o_aviso() {
    if !kernel_disponivel() {
        return;
    }
    // sobrescrita de 1 mm, abaixo da mínima de 1,2 mm do ABS
    let (mut d, c) = caixa("1", Some(abs()));
    let res = recompute(&d);
    assert_eq!(res.bodies.len(), 1);
    assert!((volume_total(&res) - vol_casca(1.)).abs() < 1e-4 * vol_casca(1.));
    match res.status.get(&c) {
        Some(Status::Warning(m)) => assert!(m.contains("Regra plástica ABS geral") && m.contains("abaixo da mínima"), "{m}"),
        s => panic!("esperava aviso da regra, veio {s:?}"),
    }
    // baixar só a parede mínima tira o aviso (o diagnóstico não depende do cache do comando)
    d.plastico.as_mut().unwrap().espessura_min = 0.8;
    let res2 = recompute(&d);
    assert_eq!(res2.status.get(&c), Some(&Status::Ok));
    // sem regra na peça, nada a conferir
    d.plastico = None;
    assert_eq!(recompute(&d).status.get(&c), Some(&Status::Ok));
}

#[test]
fn formula_da_regra_sem_regra_na_peca_e_erro_nomeado() {
    if !kernel_disponivel() {
        return;
    }
    let (d, c) = caixa(VAR_ESPESSURA, None);
    assert!(matches!(recompute(&d).status.get(&c), Some(Status::Error(m)) if m.contains(VAR_ESPESSURA)), "{:?}", recompute(&d).status.get(&c));
}

#[test]
fn nervura_e_saida_conferem_contra_a_regra() {
    let r = abs();
    let vars = r.vars(&HashMap::new());
    let nerv = |t: &str| FeatureKind::Stiffener(Stiffener { sketch: Id(1), thickness: t.into(), depth: "10".into(), reverse: false, to_body: false, side: 0 });
    assert!(plastico::avisos(&r, &nerv(VAR_NERVURA), &vars).is_empty(), "a nervura da regra passa");
    let a = plastico::avisos(&r, &nerv("2"), &vars);
    assert_eq!(a.len(), 1);
    assert!(a[0].contains("chupa"), "{a:?}");
    let saida = |g: &str| FeatureKind::Draft(Draft { faces: vec![], neutral: XY, angle: g.into(), reverse: false, face_keys: vec![], regras: vec![] });
    assert!(plastico::avisos(&r, &saida(VAR_SAIDA), &vars).is_empty());
    assert!(plastico::avisos(&r, &saida("0.2"), &vars)[0].contains("saída"));
    // comandos que a regra não governa não avisam
    assert!(plastico::avisos(&r, &ext(Id(1), BoolOp::Add, "0.1"), &vars).is_empty());
}
