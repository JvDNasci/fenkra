// SPDX-License-Identifier: LGPL-2.1-or-later
//! B92: materiais, massa por corpo e total, centro, matriz de inércia e eixos principais.

mod common;

use common::*;
use fenkra_core::materiais;
use fenkra_core::*;
use fenkra_solidos::massa::{massa_do_modelo_kg, relatorio_massa};

/// Placa a x b x h com o canto no meio da base: V = a·b·h, centro em (0, 0, h/2).
fn placa(d: &mut Document, a: f64, b: f64, h: f64, cx: f64) -> Id {
    let sk = sk_rect(d, PlaneRef::Origin(OriginPlane::XY), [cx - a / 2., -b / 2.], [cx + a / 2., b / 2.]);
    d.add_feature(ext(sk, BoolOp::Add, &h.to_string()))
}

#[test]
fn caixa_de_aluminio_tem_massa_inercia_e_eixos_conhecidos() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("m");
    d.props.material = "Alumínio 6061".into();
    placa(&mut d, 100., 60., 20., 0.);
    let r = relatorio_massa(&d);
    assert_eq!(r.corpos.len(), 1);
    let t = r.total.expect("tem corpo");
    let m = 120_000. * 2.7e-6; // kg
    assert!((t.massa_kg - m).abs() < 1e-6, "massa {} esperada {m}", t.massa_kg);
    assert!((t.centro[2] - 10.).abs() < 1e-6 && t.centro[0].abs() < 1e-6);
    let (a, b, h) = (100., 60., 20.);
    let ixx = m * (b * b + h * h) / 12.;
    let iyy = m * (a * a + h * h) / 12.;
    let izz = m * (a * a + b * b) / 12.;
    for (got, want, nome) in [(t.inercia[0], ixx, "Ixx"), (t.inercia[1], iyy, "Iyy"), (t.inercia[2], izz, "Izz")] {
        assert!((got - want).abs() / want < 1e-6, "{nome}: {got} esperado {want}");
    }
    assert!(t.inercia[3].abs() < 1e-6 * ixx && t.inercia[4].abs() < 1e-6 * ixx && t.inercia[5].abs() < 1e-6 * ixx, "produtos de inércia nulos");
    // momentos principais crescentes: menor em torno do eixo X (o comprimento), maior em torno de Z
    assert!((t.principais[0] - ixx).abs() / ixx < 1e-6 && (t.principais[2] - izz).abs() / izz < 1e-6);
    assert!(t.eixos[0][0].abs() > 0.999 && t.eixos[2][2].abs() > 0.999);
    assert!((massa_do_modelo_kg(&d) - m).abs() < 1e-6);
}

#[test]
fn material_do_corpo_muda_a_massa_e_o_total_soma_os_corpos() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("dois");
    d.props.material = "Aço 1045".into();
    placa(&mut d, 20., 20., 10., 0.);
    let seg = d.add_collector("Segundo");
    d.active_collector = seg;
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [100., 0.], [120., 20.]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    d.collectors.iter_mut().find(|c| c.id == seg).unwrap().material = "Cobre".into();
    let r = relatorio_massa(&d);
    assert_eq!(r.corpos.len(), 2, "{:?}", r.corpos.iter().map(|c| &c.nome).collect::<Vec<_>>());
    let aco = r.corpos.iter().find(|c| c.material == "Aço 1045").unwrap();
    let cobre = r.corpos.iter().find(|c| c.material == "Cobre").unwrap();
    assert!((aco.massa_kg - 4000. * 7.85e-6).abs() < 1e-9);
    assert!((cobre.massa_kg - 4000. * 8.96e-6).abs() < 1e-9);
    let t = r.total.unwrap();
    assert!((t.massa_kg - aco.massa_kg - cobre.massa_kg).abs() < 1e-12);
    // centro entre os dois, mais perto do mais pesado (cobre, em x = 110)
    let xc = (aco.massa_kg * aco.centro[0] + cobre.massa_kg * cobre.centro[0]) / t.massa_kg;
    assert!((t.centro[0] - xc).abs() < 1e-6);
    // eixos paralelos: Izz do conjunto passa a ser maior que a soma das inércias próprias
    assert!(t.inercia[2] > aco.inercia[2] + cobre.inercia[2]);
}

#[test]
fn material_fora_da_tabela_usa_o_padrao_e_a_tabela_tem_os_basicos() {
    assert_eq!(materiais::densidade("sem nome"), materiais::DENSIDADE_PADRAO);
    for n in ["Aço 1020", "Alumínio 6061", "Cobre", "Titânio Ti-6Al-4V", "ABS"] {
        assert!(materiais::material(n).is_some(), "{n}");
    }
}
