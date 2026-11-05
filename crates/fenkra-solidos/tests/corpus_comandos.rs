// SPDX-License-Identifier: LGPL-2.1-or-later
//! B72: corpus por comando de sketch, sólidos e superfícies.
//!
//! Cada comando tem um teste nomeado que cobre o ciclo da ficha: criar e medir contra a fórmula
//! fechada, mudar a cota e conferir de novo, voltar a cota e reencontrar o valor de antes, salvar e
//! abrir (documento idêntico e mesmo volume) e validade do resultado. "Validade" aqui é um proxy
//! sem BRepCheck (a fachada Rust não o expõe): status Ok, corpo com volume e área positivos e
//! volume da malha (integral de divergência) igual ao volume do kernel, o que só acontece com
//! casca fechada e bem orientada. No fim, as combinações inválidas que devem dar erro nomeado.
//!
//! Só usa a API pública; nada aqui toca `src/`.

mod common;

use common::*;
use fenkra_core::sketch::{Curve, Kind};
use fenkra_core::*;
use fenkra_kernel::Shape;
use fenkra_solidos::{recompute, surface_brep, BodyResult, RecomputeResult, Status};
use std::f64::consts::PI;

macro_rules! precisa_kernel {
    () => {
        if !kernel_disponivel() {
            return;
        }
    };
}

fn s(x: &str) -> String {
    x.into()
}

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

// ------------------------------------------------------------------ ajudas do corpus

/// Volume assinado da malha (teorema da divergência). Casca aberta ou mal orientada não fecha a conta.
fn volume_da_malha(b: &BodyResult) -> f64 {
    let p = &b.mesh.positions;
    let v = |i: u32| [p[i as usize * 3] as f64, p[i as usize * 3 + 1] as f64, p[i as usize * 3 + 2] as f64];
    b.mesh
        .indices
        .chunks_exact(3)
        .map(|t| {
            let (a, b, c) = (v(t[0]), v(t[1]), v(t[2]));
            (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6.
        })
        .sum()
}

/// Validade do resultado de todos os corpos (ver o cabeçalho do arquivo).
fn valida(nome: &str, d: &Document) {
    let r = recompute(d);
    assert!(!r.bodies.is_empty(), "{nome}: nenhum corpo");
    for b in &r.bodies {
        assert!(b.mass.volume > 0. && b.mass.area > 0., "{nome}: volume/área não positivos {:?}", b.mass);
        assert!(!b.mesh.indices.is_empty(), "{nome}: malha vazia");
        let vm = volume_da_malha(b);
        assert!((vm - b.mass.volume).abs() <= 0.02 * b.mass.volume, "{nome}: volume da malha {vm:.3} ≠ do kernel {:.3} (casca aberta?)", b.mass.volume);
    }
}

/// Salvar e abrir: documento idêntico e o mesmo volume depois de recalcular.
fn salvar_e_abrir(nome: &str, d: &Document) {
    let volta = Document::from_json(&d.to_json()).unwrap_or_else(|e| panic!("{nome}: não reabriu: {e}"));
    assert_eq!(&volta, d, "{nome}: documento mudou ao salvar e abrir");
    let (a, b) = (volume_total(&recompute(d)), volume_total(&recompute(&volta)));
    assert!((a - b).abs() < 1e-6 * a.max(1.), "{nome}: volume {a} antes e {b} depois de abrir");
}

/// Ciclo completo: confere, muda o parâmetro e confere, devolve o parâmetro e reencontra o início.
fn ciclo(nome: &str, d: &mut Document, param: &str, novo: &str, antes: &Esperado, depois: &Esperado) {
    let original = d.params.iter().find(|p| p.name == param).unwrap_or_else(|| panic!("{nome}: parâmetro {param}")).expr.clone();
    let v0 = confere(&format!("{nome} [inicial]"), d, antes);
    valida(nome, d);
    salvar_e_abrir(nome, d);
    definir_param(d, param, novo);
    confere(&format!("{nome} [{param}={novo}]"), d, depois);
    valida(&format!("{nome} [{param}={novo}]"), d);
    definir_param(d, param, &original);
    let v1 = confere(&format!("{nome} [voltou]"), d, antes);
    assert!((v0 - v1).abs() < 1e-9 * v0.max(1.), "{nome}: volume {v0} antes e {v1} depois de voltar a cota");
}

fn param(d: &mut Document, n: &str, v: &str) {
    d.add_param(n, v, Unit::Comprimento).unwrap();
}

fn erro(r: &RecomputeResult, id: Id) -> String {
    match &r.status[&id] {
        Status::Error(m) => m.clone(),
        outro => panic!("esperava erro, veio {outro:?}"),
    }
}

fn ok(r: &RecomputeResult, id: Id) {
    assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{:?}", r.status[&id]);
}

/// Sketch retangular 0..L × 0..W, cotado por parâmetros, com o canto de origem preso.
fn sk_rect_cotado(d: &mut Document, plano: PlaneRef) -> Id {
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        let (p, l) = data.add_rect(&mut al, [0., 0.], [40., 30.], false);
        data.add_constraint(&mut al, Kind::Fixed(p[0]));
        data.add_constraint(&mut al, Kind::Length(l[0], s("L")));
        data.add_constraint(&mut al, Kind::Length(l[1], s("W")));
    }
    d.add_feature(FeatureKind::Sketch(Sketch { plane: plano, data }))
}

/// Documento com L=40, W=30, H=20 e a caixa cotada.
fn caixa_cotada() -> (Document, Id, Id) {
    let mut d = Document::new("caixa cotada");
    param(&mut d, "L", "40");
    param(&mut d, "W", "30");
    param(&mut d, "H", "20");
    let sk = sk_rect_cotado(&mut d, XY);
    let e = d.add_feature(ext(sk, BoolOp::Add, "H"));
    (d, sk, e)
}

fn prisma(vol: f64, tam: [f64; 3]) -> Esperado {
    Esperado::prisma(vol, tam, [0., 0., 0.])
}

// ================================================================== SKETCH

#[test]
fn sketch_retangulo_cotado_muda_cota_e_volta() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    ciclo("retângulo cotado (L)", &mut d, "L", "60", &prisma(24000., [40., 30., 20.]), &prisma(36000., [60., 30., 20.]));
    ciclo("retângulo cotado (W)", &mut d, "W", "10", &prisma(24000., [40., 30., 20.]), &prisma(8000., [40., 10., 20.]));
}

#[test]
fn sketch_circulo_cotado_por_diametro_muda_cota_e_volta() {
    precisa_kernel!();
    let mut d = Document::new("círculo cotado");
    param(&mut d, "D", "20");
    param(&mut d, "H", "10");
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        let c = data.add_point(&mut al, 0., 0.);
        let circ = data.add_curve(&mut al, Curve::Circle { c, r: 7. }, false);
        data.add_constraint(&mut al, Kind::Fixed(c));
        data.add_constraint(&mut al, Kind::Diameter(circ, s("D")));
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: XY, data }));
    d.add_feature(ext(sk, BoolOp::Add, "H"));
    let a = Esperado::curva(PI * 100. * 10., [20., 20., 10.]);
    let b = Esperado::curva(PI * 225. * 10., [30., 30., 10.]);
    ciclo("círculo cotado (Ø)", &mut d, "D", "30", &a, &b);
}

#[test]
fn sketch_ranhura_tem_a_area_de_dois_semicirculos_e_um_retangulo() {
    precisa_kernel!();
    let mut d = Document::new("ranhura");
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        data.add_slot(&mut al, [-20., 0.], [20., 0.], 6.);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: XY, data }));
    d.add_feature(ext(sk, BoolOp::Add, "5"));
    let area = 12. * 40. + PI * 36.;
    confere("ranhura", &d, &Esperado::curva(area * 5., [52., 12., 5.]));
    valida("ranhura", &d);
    salvar_e_abrir("ranhura", &d);
}

#[test]
fn sketch_poligono_regular_inscrito_no_circulo() {
    precisa_kernel!();
    let mut d = Document::new("hexágono");
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        data.add_polygon(&mut al, [0., 0.], 10., 6, 0.);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: XY, data }));
    d.add_feature(ext(sk, BoolOp::Add, "4"));
    // raio é o do círculo circunscrito: área = 3·√3/2·r²
    let area = 3. * 3f64.sqrt() / 2. * 100.;
    let v = confere("hexágono", &d, &Esperado { vol: area * 4., tol_vol: 1e-4, tam: [20., 17.32, 4.], tol_tam: 0.05, minimo: None, via_malha: false });
    assert!(v > 0.);
    valida("hexágono", &d);
}

#[test]
fn sketch_poligono_de_linhas_com_pontos_compartilhados() {
    precisa_kernel!();
    let mut d = Document::new("triângulo");
    let sk = sk_poligono(&mut d, XY, &[[0., 0.], [30., 0.], [0., 20.]]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    confere("triângulo", &d, &prisma(0.5 * 30. * 20. * 10., [30., 20., 10.]));
    valida("triângulo", &d);
    salvar_e_abrir("triângulo", &d);
}

#[test]
fn sketch_com_furo_interno_extruda_com_vazio() {
    precisa_kernel!();
    let mut d = Document::new("anel");
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        data.add_rect(&mut al, [0., 0.], [40., 40.], false);
        let c = data.add_point(&mut al, 20., 20.);
        data.add_curve(&mut al, Curve::Circle { c, r: 5. }, false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: XY, data }));
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    confere("laço com furo", &d, &prisma((1600. - PI * 25.) * 10., [40., 40., 10.]));
    valida("laço com furo", &d);
}

#[test]
fn sketch_cotas_contraditorias_nao_passam_em_silencio() {
    precisa_kernel!();
    let mut d = Document::new("conflito");
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        let (_, l) = data.add_rect(&mut al, [0., 0.], [40., 30.], false);
        data.add_constraint(&mut al, Kind::Length(l[0], s("40")));
        data.add_constraint(&mut al, Kind::Length(l[0], s("25")));
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: XY, data }));
    let r = recompute(&d);
    assert!(!matches!(r.status[&sk], Status::Ok), "duas cotas diferentes na mesma linha devem dar aviso ou erro, veio {:?}", r.status[&sk]);
}

// ================================================================== SÓLIDOS

#[test]
fn extrusao_aditiva_muda_altura_e_volta() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    ciclo("extrusão aditiva (H)", &mut d, "H", "35", &prisma(24000., [40., 30., 20.]), &prisma(42000., [40., 30., 35.]));
}

#[test]
fn extrusao_invertida_vai_para_baixo_do_plano() {
    precisa_kernel!();
    let mut d = Document::new("invertida");
    let sk = sk_rect(&mut d, XY, [0., 0.], [40., 30.]);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("20")), reverse: true, taper: s("0"), thin: None, dir: None, thin_side: 0 }));
    confere("extrusão invertida", &d, &Esperado::prisma(24000., [40., 30., 20.], [0., 0., -20.]));
    valida("extrusão invertida", &d);
}

#[test]
fn extrusao_simetrica_divide_o_total_pelos_dois_lados() {
    precisa_kernel!();
    let mut d = Document::new("simétrica");
    param(&mut d, "H", "20");
    let sk = sk_rect(&mut d, XY, [0., 0.], [40., 30.]);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Symmetric(s("H")), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0 }));
    let a = Esperado::prisma(24000., [40., 30., 20.], [0., 0., -10.]);
    let b = Esperado::prisma(48000., [40., 30., 40.], [0., 0., -20.]);
    ciclo("extrusão simétrica", &mut d, "H", "40", &a, &b);
}

#[test]
fn extrusao_em_dois_lados_com_distancias_diferentes() {
    precisa_kernel!();
    let mut d = Document::new("dois lados");
    param(&mut d, "H1", "5");
    let sk = sk_rect(&mut d, XY, [0., 0.], [40., 30.]);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::TwoSides(s("H1"), s("15")), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0 }));
    let a = Esperado { vol: 1200. * 20., tol_vol: 1e-4, tam: [40., 30., 20.], tol_tam: 1e-3, minimo: None, via_malha: false };
    let b = Esperado { vol: 1200. * 25., tol_vol: 1e-4, tam: [40., 30., 25.], tol_tam: 1e-3, minimo: None, via_malha: false };
    ciclo("extrusão em dois lados", &mut d, "H1", "10", &a, &b);
}

#[test]
fn extrusao_ate_um_plano_termina_exatamente_nele() {
    precisa_kernel!();
    let mut d = Document::new("até o plano");
    param(&mut d, "Z", "12");
    let plano = d.add_offset_plane(XY, "Z");
    let sk = sk_rect(&mut d, XY, [0., 0.], [40., 30.]);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::UpTo(PlaneRef::Datum(plano)), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0 }));
    ciclo("extrusão até plano", &mut d, "Z", "30", &prisma(1200. * 12., [40., 30., 12.]), &prisma(1200. * 30., [40., 30., 30.]));
}

#[test]
fn extrusao_de_parede_fina_e_um_anel_centrado_no_contorno() {
    precisa_kernel!();
    let mut d = Document::new("fina");
    param(&mut d, "esp", "2");
    let sk = sk_rect(&mut d, XY, [0., 0.], [40., 30.]);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("10")), reverse: false, taper: s("0"), thin: Some(s("esp")), dir: None, thin_side: 0 }));
    // perfil fechado: a parede fica centrada no contorno (metade h = e/2 para cada lado). O lado de
    // fora arredonda os cantos e o de dentro fica vivo: área = 2·P·h + (π − 4)·h², com P = 140
    let e = |esp: f64| {
        let h = esp / 2.;
        Esperado { vol: (280. * h + (PI - 4.) * h * h) * 10., tol_vol: 1e-3, tam: [40. + esp, 30. + esp, 10.], tol_tam: 0.01, minimo: None, via_malha: false }
    };
    ciclo("parede fina", &mut d, "esp", "4", &e(2.), &e(4.));
}

#[test]
fn extrusao_com_saida_vira_tronco_de_piramide() {
    precisa_kernel!();
    let mut d = Document::new("saída");
    param(&mut d, "A", "5");
    let sk = sk_rect(&mut d, XY, [0., 0.], [20., 20.]);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("20")), reverse: false, taper: s("A"), thin: None, dir: None, thin_side: 0 }));
    let tronco = |ang: f64| {
        let l2 = 20. - 2. * 20. * ang.to_radians().tan();
        let (a1, a2) = (400., l2 * l2);
        20. / 3. * (a1 + a2 + (a1 * a2).sqrt())
    };
    let esp = |ang: f64| Esperado { vol: tronco(ang), tol_vol: 2e-3, tam: [20., 20., 20.], tol_tam: 0.01, minimo: None, via_malha: false };
    ciclo("extrusão com saída", &mut d, "A", "3", &esp(5.), &esp(3.));
}

#[test]
fn corte_por_extrusao_tira_o_volume_do_bolso() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    param(&mut d, "P", "5");
    let topo = d.add_offset_plane(XY, "H");
    let bolso = sk_rect(&mut d, PlaneRef::Datum(topo), [10., 10.], [20., 20.]);
    d.add_feature(ext(bolso, BoolOp::Cut, "P"));
    ciclo("corte por extrusão", &mut d, "P", "8", &prisma(24000. - 500., [40., 30., 20.]), &prisma(24000. - 800., [40., 30., 20.]));
}

#[test]
fn corte_passante_atravessa_o_corpo_inteiro() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    let furo = sk_circulos(&mut d, XY, &[([20., 15.], 5.)]);
    // o sketch está na base (z = 0): a passante tem de ser invertida para entrar no corpo
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: furo, op: BoolOp::Cut, extent: Extent::ThroughAll, reverse: true, taper: s("0"), thin: None, dir: None, thin_side: 0 }));
    ciclo("corte passante", &mut d, "H", "30", &Esperado { vol: 24000. - PI * 25. * 20., tol_vol: 2e-3, tam: [40., 30., 20.], tol_tam: 0.01, minimo: None, via_malha: false }, &Esperado { vol: 36000. - PI * 25. * 30., tol_vol: 2e-3, tam: [40., 30., 30.], tol_tam: 0.01, minimo: None, via_malha: false });
}

#[test]
fn revolucao_de_perfil_retangular_vira_tubo() {
    precisa_kernel!();
    let mut d = Document::new("tubo");
    param(&mut d, "ang", "360");
    let sk = sk_rect(&mut d, XY, [5., 0.], [10., 20.]);
    d.add_feature(FeatureKind::Revolve(Revolve { sketch: sk, axis: AxisRef::Origin(OriginAxis::Y), angle: s("ang"), symmetric: false, op: BoolOp::Add, thin: None, thin_side: 0, angle2: None }));
    let tubo = PI * (100. - 25.) * 20.;
    let a = Esperado::curva(tubo, [20., 20., 20.]);
    ciclo("revolução (360)", &mut d, "ang", "180", &a, &Esperado { vol: tubo / 2., tol_vol: 2e-3, tam: [20., 20., 10.], tol_tam: 0.3, minimo: None, via_malha: true });
}

#[test]
fn revolucao_em_torno_de_eixo_do_sketch_e_simetrica_quando_pedido() {
    precisa_kernel!();
    let mut d = Document::new("simétrica");
    let sk = sk_rect(&mut d, XY, [5., 0.], [10., 20.]);
    d.add_feature(FeatureKind::Revolve(Revolve { sketch: sk, axis: AxisRef::Origin(OriginAxis::Y), angle: s("90"), symmetric: true, op: BoolOp::Add, thin: None, thin_side: 0, angle2: None }));
    let r = recompute(&d);
    let v = volume_total(&r);
    assert!((v - PI * 75. * 20. / 4.).abs() < 0.01 * v, "volume {v}");
    valida("revolução simétrica", &d);
}

#[test]
fn furo_simples_passante_muda_diametro_e_volta() {
    precisa_kernel!();
    let mut d = Document::new("furo");
    param(&mut d, "Dp", "8");
    let base = sk_rect(&mut d, XY, [-20., -20.], [20., 20.]);
    d.add_feature(ext(base, BoolOp::Add, "10"));
    let topo = d.add_offset_plane(XY, "10");
    let pts = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[0., 0.]]);
    d.add_feature(furo_passante(pts, "Dp"));
    let e = |dia: f64| Esperado { vol: 16000. - PI * dia * dia / 4. * 10., tol_vol: 2e-3, tam: [40., 40., 10.], tol_tam: 0.01, minimo: None, via_malha: false };
    ciclo("furo simples", &mut d, "Dp", "12", &e(8.), &e(12.));
}

fn placa_para_furos(d: &mut Document) -> Id {
    let base = sk_rect(d, XY, [-20., -20.], [20., 20.]);
    d.add_feature(ext(base, BoolOp::Add, "10"));
    let topo = d.add_offset_plane(XY, "10");
    sk_pontos(d, PlaneRef::Datum(topo), &[[0., 0.]])
}

fn furo_de(sk: Id, kind: HoleKind, dia: &str, depth: Extent) -> FeatureKind {
    FeatureKind::Hole(Hole { sketch: sk, kind, diameter: dia.into(), depth, thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: s("0"), left_hand: false })
}

#[test]
fn furo_rebaixado_soma_o_cilindro_maior() {
    precisa_kernel!();
    let mut d = Document::new("rebaixado");
    param(&mut d, "pr", "4");
    let pts = placa_para_furos(&mut d);
    d.add_feature(furo_de(pts, HoleKind::Rebaixado { diameter: s("12"), depth: s("pr") }, "6", Extent::ThroughAll));
    let e = |pr: f64| Esperado { vol: 16000. - PI * 9. * 10. - PI * (36. - 9.) * pr, tol_vol: 3e-3, tam: [40., 40., 10.], tol_tam: 0.01, minimo: None, via_malha: false };
    ciclo("furo rebaixado", &mut d, "pr", "6", &e(4.), &e(6.));
}

#[test]
fn furo_escareado_tem_o_tronco_de_cone_na_boca() {
    precisa_kernel!();
    let mut d = Document::new("escareado");
    let pts = placa_para_furos(&mut d);
    d.add_feature(furo_de(pts, HoleKind::Escareado { diameter: s("12"), angle: s("90") }, "6", Extent::ThroughAll));
    // cone de 90°: a boca Ø12 desce 3 mm até o Ø6
    let extra = PI * 3. / 3. * (36. + 18. + 9.) - PI * 9. * 3.;
    confere("furo escareado", &d, &Esperado { vol: 16000. - PI * 9. * 10. - extra, tol_vol: 3e-3, tam: [40., 40., 10.], tol_tam: 0.01, minimo: None, via_malha: false });
    valida("furo escareado", &d);
    salvar_e_abrir("furo escareado", &d);
}

#[test]
fn furo_cego_para_na_profundidade_pedida() {
    precisa_kernel!();
    let mut d = Document::new("cego");
    param(&mut d, "pf", "4");
    let pts = placa_para_furos(&mut d);
    d.add_feature(furo_de(pts, HoleKind::Simples, "6", Extent::Blind(s("pf"))));
    let e = |pf: f64| Esperado { vol: 16000. - PI * 9. * pf, tol_vol: 2e-3, tam: [40., 40., 10.], tol_tam: 0.01, minimo: None, via_malha: false };
    ciclo("furo cego", &mut d, "pf", "7", &e(4.), &e(7.));
}

#[test]
fn filete_em_uma_aresta_tira_o_triangulo_curvo_do_comprimento() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    param(&mut d, "r", "3");
    // aresta superior da face x=0, paralela a Y, no meio de W=30: (0, 15, 20)
    d.add_feature(filete("r", vec![[0., 15., 20.]], vec![]));
    let e = |r: f64| Esperado { vol: 24000. - (1. - PI / 4.) * r * r * 30., tol_vol: 2e-3, tam: [40., 30., 20.], tol_tam: 0.01, minimo: None, via_malha: false };
    ciclo("filete (1 aresta)", &mut d, "r", "5", &e(3.), &e(5.));
}

#[test]
fn chanfro_em_uma_aresta_tira_o_prisma_triangular() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    param(&mut d, "c", "3");
    d.add_feature(chanfro("c", vec![[0., 15., 20.]], vec![]));
    let e = |c: f64| prisma(24000. - 0.5 * c * c * 30., [40., 30., 20.]);
    ciclo("chanfro (1 aresta)", &mut d, "c", "6", &e(3.), &e(6.));
}

#[test]
fn filete_em_todas_as_arestas_segue_a_formula_fechada() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    param(&mut d, "r", "2");
    d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("r"), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![] }));
    let e = |r: f64| Esperado { vol: vol_caixa_arredondada(40., 30., 20., r), tol_vol: 2e-3, tam: [40., 30., 20.], tol_tam: 0.01, minimo: None, via_malha: true };
    ciclo("filete (todas)", &mut d, "r", "4", &e(2.), &e(4.));
}

#[test]
fn primitivas_seguem_as_formulas_fechadas() {
    precisa_kernel!();
    let casos: Vec<(&str, FeatureKind, f64, [f64; 3])> = vec![
        ("caixa", primitiva(PrimKind::Caixa, "40", "30", "20", "0"), 24000., [40., 30., 20.]),
        ("cilindro", primitiva(PrimKind::Cilindro, "10", "30", "0", "0"), PI * 100. * 30., [20., 20., 30.]),
        ("esfera", primitiva(PrimKind::Esfera, "10", "0", "0", "0"), 4. / 3. * PI * 1000., [20., 20., 20.]),
        ("cone", primitiva(PrimKind::Cone, "12", "6", "20", "0"), PI * 20. / 3. * (144. + 72. + 36.), [24., 24., 20.]),
        ("toro", primitiva(PrimKind::Toro, "20", "5", "0", "0"), 2. * PI * PI * 20. * 25., [50., 50., 10.]),
        ("cunha", primitiva(PrimKind::Cunha, "30", "20", "10", "10"), 20. * 10. * (30. + 10.) / 2., [30., 20., 10.]),
        ("prisma hexagonal", primitiva(PrimKind::Prisma, "6", "10", "8", "0"), 3. * 3f64.sqrt() / 2. * 100. * 8., [20., 17.32, 8.]),
    ];
    for (nome, k, vol, tam) in casos {
        let mut d = Document::new(nome);
        d.add_feature(k);
        confere(nome, &d, &Esperado { vol, tol_vol: 3e-3, tam, tol_tam: 0.3, minimo: None, via_malha: true });
        valida(nome, &d);
        salvar_e_abrir(nome, &d);
    }
}

#[test]
fn primitiva_muda_medida_e_volta() {
    precisa_kernel!();
    let mut d = Document::new("cilindro");
    param(&mut d, "R", "10");
    d.add_feature(primitiva(PrimKind::Cilindro, "R", "30", "0", "0"));
    let e = |r: f64| Esperado { vol: PI * r * r * 30., tol_vol: 3e-3, tam: [2. * r, 2. * r, 30.], tol_tam: 0.3, minimo: None, via_malha: true };
    ciclo("primitiva (R)", &mut d, "R", "15", &e(10.), &e(15.));
}

#[test]
fn casca_aberta_em_cima_muda_espessura_e_volta() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    param(&mut d, "t", "2");
    d.add_feature(FeatureKind::Shell(Shell { faces: vec![[20., 15., 20.]], thickness: s("t"), outward: false, face_keys: vec![], closed: false }));
    let e = |t: f64| prisma(24000. - (40. - 2. * t) * (30. - 2. * t) * (20. - t), [40., 30., 20.]);
    ciclo("casca", &mut d, "t", "3", &e(2.), &e(3.));
}

#[test]
fn espessura_na_face_de_cima_engrossa_o_corpo() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    param(&mut d, "g", "3");
    d.add_feature(FeatureKind::Thickness(Thickness { faces: vec![[20., 15., 20.]], value: s("g"), face_keys: vec![], remove: false }));
    ciclo("espessura na face", &mut d, "g", "5", &prisma(1200. * 23., [40., 30., 23.]), &prisma(1200. * 25., [40., 30., 25.]));
}

#[test]
fn saida_de_face_inclina_a_lateral_pelo_plano_neutro() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    param(&mut d, "ad", "5");
    d.add_feature(FeatureKind::Draft(Draft { faces: vec![[40., 15., 10.]], neutral: XY, angle: s("ad"), reverse: false, face_keys: vec![] }));
    // a face x=40 gira em torno de z=0: a cunha acrescentada ou tirada vale ½·H²·tan(a)·W
    let cunha = |a: f64| 0.5 * 400. * a.to_radians().tan() * 30.;
    for ang in [5., 8.] {
        definir_param(&mut d, "ad", &ang.to_string());
        let r = recompute(&d);
        assert!(problema_de_status(&d, &r).is_none(), "{:?}", problema_de_status(&d, &r));
        let v = volume_total(&r);
        assert!(((v - 24000.).abs() - cunha(ang)).abs() < 0.05 * cunha(ang), "ângulo {ang}: Δvolume {} esperado ±{}", v - 24000., cunha(ang));
        valida("saída de face", &d);
    }
    definir_param(&mut d, "ad", "5");
    salvar_e_abrir("saída de face", &d);
}

fn bloco_pequeno(d: &mut Document) -> Id {
    let sk = sk_rect(d, XY, [0., 0.], [10., 10.]);
    d.add_feature(ext(sk, BoolOp::Add, "5"))
}

#[test]
fn padrao_linear_repete_o_comando_na_direcao() {
    precisa_kernel!();
    let mut d = Document::new("padrão linear");
    param(&mut d, "n", "3");
    let b = bloco_pequeno(&mut d);
    d.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: s("n"), spacing: s("20"), dir2: None, count2: s("1"), spacing2: s("0"), total: false }, sources: vec![b] }));
    let e = |n: f64| Esperado { vol: 500. * n, tol_vol: 1e-4, tam: [10. + 20. * (n - 1.), 10., 5.], tol_tam: 1e-3, minimo: None, via_malha: false };
    // as instâncias são disjuntas: o kernel guarda um corpo só, com n sólidos
    ciclo("padrão linear", &mut d, "n", "5", &e(3.), &e(5.));
}

#[test]
fn padrao_circular_distribui_as_copias_em_volta_do_eixo() {
    precisa_kernel!();
    let mut d = Document::new("padrão circular");
    param(&mut d, "n", "4");
    let sk = sk_rect(&mut d, XY, [20., -5.], [30., 5.]);
    let b = d.add_feature(ext(sk, BoolOp::Add, "5"));
    d.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Z), count: s("n"), angle: s("360") }, sources: vec![b] }));
    let e = |n: f64, ty: f64| Esperado { vol: 500. * n, tol_vol: 1e-4, tam: [60., ty, 5.], tol_tam: 0.05, minimo: None, via_malha: false };
    // 4 cópias a 90° ocupam −30..30 nos dois eixos; com 6 a 60° o eixo y vai de ±(30·sen60° + 5·cos60°)
    ciclo("padrão circular", &mut d, "n", "6", &e(4., 60.), &e(6., 2. * (30. * 60f64.to_radians().sin() + 5. * 60f64.to_radians().cos())));
}

#[test]
fn espelho_duplica_o_comando_do_outro_lado_do_plano() {
    precisa_kernel!();
    let mut d = Document::new("espelho");
    param(&mut d, "x0", "10");
    let sk = sk_rect(&mut d, XY, [10., 0.], [20., 10.]);
    let b = d.add_feature(ext(sk, BoolOp::Add, "5"));
    d.add_feature(FeatureKind::Mirror(Mirror { plane: PlaneRef::Origin(OriginPlane::YZ), sources: vec![b] }));
    confere("espelho", &d, &Esperado { vol: 1000., tol_vol: 1e-4, tam: [40., 10., 5.], tol_tam: 1e-3, minimo: Some([-20., 0., 0.]), via_malha: false });
    valida("espelho", &d);
    salvar_e_abrir("espelho", &d);
}

#[test]
fn escala_multiplica_o_volume_pelo_cubo_do_fator() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    param(&mut d, "k", "2");
    d.add_feature(FeatureKind::Scale(Scale { center: PointRef::Origin, factor: s("k") }));
    ciclo("escala", &mut d, "k", "0.5", &prisma(24000. * 8., [80., 60., 40.]), &prisma(24000. / 8., [20., 15., 10.]));
}

#[test]
fn varredura_de_circulo_em_linha_e_um_cilindro() {
    precisa_kernel!();
    let mut d = Document::new("varredura");
    param(&mut d, "C", "30");
    let linha = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 0., 30.]), start: s("0"), end: s("0") });
    let perfil = sk_circulos(&mut d, XY, &[([0., 0.], 3.)]);
    d.add_feature(FeatureKind::Sweep(Sweep { sketch: perfil, spine: CurveRef::Ref(linha), mode: SweepMode::Corrigido, op: BoolOp::Add, thin: None }));
    confere("varredura", &d, &Esperado::curva(PI * 9. * 30., [6., 6., 30.]));
    valida("varredura", &d);
    salvar_e_abrir("varredura", &d);
}

#[test]
fn loft_reto_entre_dois_quadrados_e_um_tronco() {
    precisa_kernel!();
    let mut d = Document::new("loft");
    param(&mut d, "h", "10");
    let alto = d.add_offset_plane(XY, "h");
    let a = sk_rect(&mut d, XY, [-10., -10.], [10., 10.]);
    let b = sk_rect(&mut d, PlaneRef::Datum(alto), [-5., -5.], [5., 5.]);
    d.add_feature(FeatureKind::Loft(Loft { sections: vec![a, b], ruled: true, closed: false, op: BoolOp::Add }));
    let tronco = |h: f64| h / 3. * (400. + 100. + 200.);
    let e = |h: f64| Esperado { vol: tronco(h), tol_vol: 2e-3, tam: [20., 20., h], tol_tam: 0.01, minimo: None, via_malha: false };
    ciclo("loft reto", &mut d, "h", "16", &e(10.), &e(16.));
}

#[test]
fn mover_corpo_translada_sem_mudar_o_volume() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    param(&mut d, "dx", "25");
    d.add_feature(FeatureKind::MoveBody(MoveBody { xf: BodyXf::Translate { dir: AxisRef::Origin(OriginAxis::X), dist: s("dx") } }));
    let a = Esperado::prisma(24000., [40., 30., 20.], [25., 0., 0.]);
    let b = Esperado::prisma(24000., [40., 30., 20.], [-10., 0., 0.]);
    ciclo("mover corpo", &mut d, "dx", "-10", &a, &b);
}

#[test]
fn dividir_corpo_por_plano_fica_com_um_lado() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    param(&mut d, "zc", "8");
    let plano = d.add_offset_plane(XY, "zc");
    d.add_feature(FeatureKind::SplitBody(SplitBody { surface: SurfRef::Plane(PlaneRef::Datum(plano)), flip: false }));
    let r = recompute(&d);
    assert!(problema_de_status(&d, &r).is_none(), "{:?}", problema_de_status(&d, &r));
    let v = volume_total(&r);
    assert!((v - 1200. * 8.).abs() < 1.0 || (v - 1200. * 12.).abs() < 1.0, "volume {v}: esperava um dos dois lados (9600 ou 14400)");
    valida("dividir corpo", &d);
    salvar_e_abrir("dividir corpo", &d);
}

// ================================================================== SUPERFÍCIES

fn forma(d: &Document, id: Id) -> Shape {
    Shape::from_brep(&surface_brep(d, id).expect("superfície")).unwrap()
}

#[test]
fn superficie_esfera_tem_area_4_pi_r2() {
    precisa_kernel!();
    let mut d = Document::new("sup esfera");
    param(&mut d, "R", "10");
    let id = d.add_geometry(RefKind::SurfSphere { center: PointRef::Origin, radius: s("R") });
    for (r, esperado) in [("10", 4. * PI * 100.), ("15", 4. * PI * 225.), ("10", 4. * PI * 100.)] {
        definir_param(&mut d, "R", r);
        let res = recompute(&d);
        ok(&res, id);
        let a = forma(&d, id).mass().area;
        assert!((a - esperado).abs() < 2e-3 * esperado, "R={r}: área {a} esperada {esperado}");
    }
}

#[test]
fn superficie_cilindro_tem_area_lateral() {
    precisa_kernel!();
    let mut d = Document::new("sup cilindro");
    param(&mut d, "R", "10");
    let id = d.add_geometry(RefKind::SurfCylinder { point: PointRef::Origin, dir: AxisRef::Origin(OriginAxis::Z), radius: s("R"), start: s("0"), end: s("30") });
    for (r, esperado) in [("10", 2. * PI * 10. * 30.), ("20", 2. * PI * 20. * 30.), ("10", 2. * PI * 10. * 30.)] {
        definir_param(&mut d, "R", r);
        let res = recompute(&d);
        ok(&res, id);
        let a = forma(&d, id).mass().area;
        assert!((a - esperado).abs() < 2e-3 * esperado, "R={r}: área {a} esperada {esperado}");
    }
}

#[test]
fn superficie_extrudada_muda_comprimento_e_volta() {
    precisa_kernel!();
    let mut d = Document::new("sup extrusão");
    param(&mut d, "h", "30");
    let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([20., 0., 0.]), start: s("0"), end: s("0") });
    let id = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(l), dir: AxisRef::Origin(OriginAxis::Z), length: s("h"), length2: s("0"), until: None, until2: None });
    let areas = [("30", 600.), ("50", 1000.), ("30", 600.)];
    for (h, esperado) in areas {
        definir_param(&mut d, "h", h);
        let res = recompute(&d);
        ok(&res, id);
        let a = forma(&d, id).mass().area;
        assert!((a - esperado).abs() < 1e-3, "h={h}: área {a} esperada {esperado}");
    }
    let volta = Document::from_json(&d.to_json()).unwrap();
    assert_eq!(volta, d);
}

#[test]
fn superficie_de_revolucao_gera_a_lateral_do_cone() {
    precisa_kernel!();
    let mut d = Document::new("sup revolução");
    let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([10., 0., 0.]), b: PointRef::Fixed([0., 0., 10.]), start: s("0"), end: s("0") });
    let id = d.add_geometry(RefKind::SurfRevolve { curve: CurveRef::Ref(l), axis: AxisRef::Origin(OriginAxis::Z), angle: s("360"), angle2: s("0") });
    let res = recompute(&d);
    ok(&res, id);
    // lateral de cone: π·r·g com g = 10√2
    let esperado = PI * 10. * 10. * 2f64.sqrt();
    let a = forma(&d, id).mass().area;
    assert!((a - esperado).abs() < 3e-3 * esperado, "área {a} esperada {esperado}");
}

#[test]
fn superficie_ligada_entre_duas_linhas_e_um_retangulo() {
    precisa_kernel!();
    let mut d = Document::new("sup loft");
    param(&mut d, "dy", "10");
    let a = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([20., 0., 0.]), start: s("0"), end: s("0") });
    let b = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 10., 0.]), b: PointRef::Fixed([20., 10., 0.]), start: s("0"), end: s("0") });
    let id = d.add_geometry(RefKind::SurfLoft { sections: vec![CurveRef::Ref(a), CurveRef::Ref(b)], guide: None, ruled: true, closed: false });
    let res = recompute(&d);
    ok(&res, id);
    assert!((forma(&d, id).mass().area - 200.).abs() < 1e-3);
}

#[test]
fn superficie_deslocada_acompanha_a_distancia() {
    precisa_kernel!();
    let mut d = Document::new("sup deslocada");
    param(&mut d, "dist", "5");
    let esfera = d.add_geometry(RefKind::SurfSphere { center: PointRef::Origin, radius: s("10") });
    let id = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(esfera), dist: s("dist"), both: false, join: OffsetJoin::Automatico });
    for (dist, raio) in [("5", 15.), ("10", 20.), ("5", 15.)] {
        definir_param(&mut d, "dist", dist);
        let res = recompute(&d);
        ok(&res, id);
        let esperado = 4. * PI * raio * raio;
        let a = forma(&d, id).mass().area;
        assert!((a - esperado).abs() < 3e-3 * esperado, "dist={dist}: área {a} esperada {esperado}");
    }
}

#[test]
fn engrossar_superficie_gera_solido_valido() {
    precisa_kernel!();
    let mut d = Document::new("engrossar");
    param(&mut d, "t", "2");
    let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([20., 0., 0.]), start: s("0"), end: s("0") });
    let sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(l), dir: AxisRef::Origin(OriginAxis::Z), length: s("30"), length2: s("0"), until: None, until2: None });
    d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(sup), t1: s("t"), t2: s("0"), op: BoolOp::Add }));
    let e = |t: f64| Esperado { vol: 20. * 30. * t, tol_vol: 1e-3, tam: [20., t, 30.], tol_tam: 0.01, minimo: None, via_malha: false };
    ciclo("engrossar superfície", &mut d, "t", "4", &e(2.), &e(4.));
}

// ================================================================== COMBINAÇÕES INVÁLIDAS

/// Caixa 40×30×20 mais um comando; devolve o resultado e o id do comando.
fn com(f: impl FnOnce(&mut Document) -> FeatureKind) -> (RecomputeResult, Id) {
    let (mut d, _, _) = caixa_cotada();
    let k = f(&mut d);
    let id = d.add_feature(k);
    (recompute(&d), id)
}

#[test]
fn invalido_extrusao_de_sketch_vazio() {
    precisa_kernel!();
    let mut d = Document::new("vazio");
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: XY, data: SketchData::default() }));
    let e = d.add_feature(ext(sk, BoolOp::Add, "10"));
    let r = recompute(&d);
    erro(&r, e);
    assert!(r.bodies.is_empty());
}

#[test]
fn invalido_extrusao_de_perfil_aberto() {
    precisa_kernel!();
    let mut d = Document::new("aberto");
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        let p: Vec<Id> = [[0., 0.], [20., 0.], [20., 20.]].iter().map(|q| data.add_point(&mut al, q[0], q[1])).collect();
        data.add_curve(&mut al, Curve::Line { a: p[0], b: p[1] }, false);
        data.add_curve(&mut al, Curve::Line { a: p[1], b: p[2] }, false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: XY, data }));
    let e = d.add_feature(ext(sk, BoolOp::Add, "10"));
    // perfil aberto não vira erro: vira parede fina de 1 mm, mas o aviso tem de dizer isso
    let r = recompute(&d);
    match &r.status[&e] {
        Status::Warning(m) => assert!(m.contains("aberto") && m.contains("parede fina"), "{m}"),
        outro => panic!("esperava aviso nomeando o perfil aberto, veio {outro:?}"),
    }
}

#[test]
fn invalido_extrusao_de_distancia_zero() {
    precisa_kernel!();
    let (r, id) = com(|d| {
        let sk = sk_rect(d, XY, [0., 0.], [5., 5.]);
        ext(sk, BoolOp::Add, "0")
    });
    erro(&r, id);
}

#[test]
fn invalido_expressao_com_parametro_inexistente() {
    precisa_kernel!();
    let (r, id) = com(|d| {
        let sk = sk_rect(d, XY, [0., 0.], [5., 5.]);
        ext(sk, BoolOp::Add, "naoexiste*2")
    });
    erro(&r, id);
}

/// BUG achado (B72): `10/0` vira infinito em `expr.rs` (a divisão não confere `b == 0`) e a extrusão
/// leva o infinito ao kernel, que estoura a pilha do processo (STATUS_STACK_OVERFLOW) em vez de
/// devolver erro. Corrigir: expressão não finita é erro de parâmetro/expressão.
#[test]
fn invalido_expressao_com_divisao_por_zero() {
    precisa_kernel!();
    let (r, id) = com(|d| {
        let sk = sk_rect(d, XY, [0., 0.], [5., 5.]);
        ext(sk, BoolOp::Add, "10/0")
    });
    erro(&r, id);
}

#[test]
fn invalido_extrusao_apontando_para_comando_que_nao_e_sketch() {
    precisa_kernel!();
    let (r, id) = com(|d| ext(d.features[1].id, BoolOp::Add, "5"));
    erro(&r, id);
}

#[test]
fn invalido_corte_sem_corpo_para_cortar() {
    precisa_kernel!();
    let mut d = Document::new("corte no vazio");
    let sk = sk_rect(&mut d, XY, [0., 0.], [5., 5.]);
    let id = d.add_feature(ext(sk, BoolOp::Cut, "5"));
    let r = recompute(&d);
    erro(&r, id);
}

#[test]
fn invalido_filete_maior_que_o_corpo_aguenta() {
    precisa_kernel!();
    let (r, id) = com(|_| filete("50", vec![[0., 15., 20.]], vec![]));
    erro(&r, id);
}

#[test]
fn invalido_filete_sem_arestas() {
    precisa_kernel!();
    let (r, id) = com(|_| filete("2", vec![], vec![]));
    // nada é aplicado e o aviso manda clicar nas arestas
    match &r.status[&id] {
        Status::Warning(m) => assert!(m.contains("arestas"), "{m}"),
        outro => panic!("esperava aviso, veio {outro:?}"),
    }
    assert!((volume_total(&r) - 24000.).abs() < 1e-6);
}

#[test]
fn invalido_filete_com_raio_zero_ou_negativo() {
    precisa_kernel!();
    for v in ["0", "-3"] {
        let (r, id) = com(|_| filete(v, vec![[0., 15., 20.]], vec![]));
        erro(&r, id);
    }
}

#[test]
fn invalido_chanfro_maior_que_a_face() {
    precisa_kernel!();
    let (r, id) = com(|_| chanfro("60", vec![[0., 15., 20.]], vec![]));
    erro(&r, id);
}

/// BUG achado (B72): casca de espessura 30 num corpo de 20 de altura volta `Ok` e devolve o corpo
/// maciço (volume 24000 intacto). Devia ser erro nomeado (espessura maior que o corpo).
#[test]
#[ignore = "bug: casca mais grossa que o corpo devolve Ok com o sólido sem esvaziar"]
fn invalido_casca_mais_grossa_que_o_corpo() {
    precisa_kernel!();
    let (r, id) = com(|_| FeatureKind::Shell(Shell { faces: vec![[20., 15., 20.]], thickness: s("30"), outward: false, face_keys: vec![], closed: false }));
    erro(&r, id);
}

#[test]
fn invalido_casca_com_face_que_nao_existe() {
    precisa_kernel!();
    let (r, id) = com(|_| FeatureKind::Shell(Shell { faces: vec![[500., 500., 500.]], thickness: s("2"), outward: false, face_keys: vec![], closed: false }));
    erro(&r, id);
}

#[test]
fn invalido_furo_de_diametro_zero() {
    precisa_kernel!();
    let (r, id) = com(|d| {
        let pts = sk_pontos(d, XY, &[[20., 15.]]);
        furo_passante(pts, "0")
    });
    erro(&r, id);
}

#[test]
fn invalido_furo_sem_pontos() {
    precisa_kernel!();
    let (r, id) = com(|d| {
        let sk = sk_pontos(d, XY, &[]);
        furo_passante(sk, "4")
    });
    erro(&r, id);
}

#[test]
fn invalido_revolucao_de_angulo_zero() {
    precisa_kernel!();
    let mut d = Document::new("revolução nula");
    let sk = sk_rect(&mut d, XY, [5., 0.], [10., 20.]);
    let id = d.add_feature(FeatureKind::Revolve(Revolve { sketch: sk, axis: AxisRef::Origin(OriginAxis::Y), angle: s("0"), symmetric: false, op: BoolOp::Add, thin: None, thin_side: 0, angle2: None }));
    erro(&recompute(&d), id);
}

#[test]
fn invalido_loft_com_uma_secao_so() {
    precisa_kernel!();
    let mut d = Document::new("loft de uma seção");
    let a = sk_rect(&mut d, XY, [0., 0.], [10., 10.]);
    let id = d.add_feature(FeatureKind::Loft(Loft { sections: vec![a], ruled: true, closed: false, op: BoolOp::Add }));
    erro(&recompute(&d), id);
}

#[test]
fn invalido_varredura_com_espinha_que_nao_existe() {
    precisa_kernel!();
    let mut d = Document::new("varredura sem guia");
    let perfil = sk_circulos(&mut d, XY, &[([0., 0.], 3.)]);
    let id = d.add_feature(FeatureKind::Sweep(Sweep { sketch: perfil, spine: CurveRef::Ref(Id(99999)), mode: SweepMode::Corrigido, op: BoolOp::Add, thin: None }));
    erro(&recompute(&d), id);
}

#[test]
fn invalido_escala_zero() {
    precisa_kernel!();
    let (r, id) = com(|_| FeatureKind::Scale(Scale { center: PointRef::Origin, factor: s("0") }));
    erro(&r, id);
}

#[test]
fn invalido_padrao_com_zero_copias() {
    precisa_kernel!();
    let (r, id) = com(|d| {
        let b = d.features[1].id;
        FeatureKind::Pattern(Pattern { kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: s("0"), spacing: s("10"), dir2: None, count2: s("1"), spacing2: s("0"), total: false }, sources: vec![b] })
    });
    // zero cópias não é erro: o comando vira uma instância só e avisa
    match &r.status[&id] {
        Status::Warning(m) => assert!(m.contains("instância"), "{m}"),
        outro => panic!("esperava aviso, veio {outro:?}"),
    }
}

#[test]
fn invalido_dividir_corpo_por_plano_que_nao_o_corta() {
    precisa_kernel!();
    let (r, id) = com(|d| {
        let plano = d.add_offset_plane(XY, "500");
        FeatureKind::SplitBody(SplitBody { surface: SurfRef::Plane(PlaneRef::Datum(plano)), flip: false })
    });
    assert!(!matches!(r.status[&id], Status::Ok), "plano fora do corpo não pode passar como Ok: {:?}", r.status[&id]);
}

#[test]
fn invalido_engrossar_superficie_que_nao_existe() {
    precisa_kernel!();
    let mut d = Document::new("engrossar nada");
    let id = d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(Id(99999)), t1: s("2"), t2: s("0"), op: BoolOp::Add }));
    erro(&recompute(&d), id);
}

#[test]
fn invalido_superficie_de_esfera_com_raio_negativo() {
    precisa_kernel!();
    let mut d = Document::new("esfera negativa");
    let id = d.add_geometry(RefKind::SurfSphere { center: PointRef::Origin, radius: s("-5") });
    erro(&recompute(&d), id);
}

#[test]
fn invalido_superficie_extrudada_de_curva_que_nao_existe() {
    precisa_kernel!();
    let mut d = Document::new("curva fantasma");
    let id = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(Id(99999)), dir: AxisRef::Origin(OriginAxis::Z), length: s("10"), length2: s("0"), until: None, until2: None });
    erro(&recompute(&d), id);
}

#[test]
fn invalido_superficie_deslocada_de_superficie_que_nao_existe() {
    precisa_kernel!();
    let mut d = Document::new("offset fantasma");
    let id = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(Id(99999)), dist: s("5"), both: false, join: OffsetJoin::Automatico });
    erro(&recompute(&d), id);
}

/// BUG achado (B72): corte passante cujo sentido não alcança o corpo (sketch na base, sem inverter)
/// volta `Ok` com o volume intacto; um corte que não remove nada devia avisar.
#[test]
#[ignore = "bug: corte passante que não toca o corpo devolve Ok em silêncio"]
fn invalido_corte_que_nao_remove_nada_avisa() {
    precisa_kernel!();
    let (mut d, _, _) = caixa_cotada();
    let furo = sk_circulos(&mut d, XY, &[([20., 15.], 5.)]);
    let id = d.add_feature(FeatureKind::Extrude(Extrude { sketch: furo, op: BoolOp::Cut, extent: Extent::ThroughAll, reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0 }));
    let r = recompute(&d);
    assert!(!matches!(r.status[&id], Status::Ok), "o corte não removeu nada e o status é Ok (volume {})", volume_total(&r));
}

/// BUG achado (B72): um parâmetro chamado `e` é aceito por `add_param`, mas a expressão `e` vale a
/// constante de Euler (2,718...) e o valor do usuário é ignorado em silêncio (a parede fina de `e = 2`
/// saiu com 2,718). Ou `add_param` recusa nomes de constante, ou o parâmetro tem de vencer a constante.
#[test]
#[ignore = "bug: parâmetro chamado 'e' perde para a constante de Euler em silêncio (expr.rs)"]
fn parametro_com_nome_de_constante_nao_e_ignorado_em_silencio() {
    let mut d = Document::new("nome de constante");
    match d.add_param("e", "2", Unit::Comprimento) {
        Err(_) => {}
        Ok(_) => {
            let vars = d.eval_params().unwrap();
            let v = fenkra_core::expr::eval_str("e", &vars).unwrap();
            assert!((v - 2.).abs() < 1e-12, "a expressão `e` valeu {v}, não o parâmetro 2");
        }
    }
}
