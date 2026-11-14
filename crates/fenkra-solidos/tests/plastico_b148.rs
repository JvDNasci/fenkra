// SPDX-License-Identifier: LGPL-2.1-or-later
//! B148: recursos de plástico. Cada teste monta a peça em código, recalcula e confere status, volume
//! e caixa envolvente contra a conta feita à mão (Pappus para as peças de revolução, prismas para o
//! resto). Os campos que a regra plástica governa nascem com a fórmula dela: mudar a regra muda o
//! recurso, a sobrescrita fora dos limites avisa na árvore e o suporte apagado é erro nomeado.

mod common;

use common::*;
use fenkra_core::recurso_plastico::*;
use fenkra_core::regra_plastica::RegraPlastica;
use fenkra_core::*;
use fenkra_solidos::{plastico_rec, recompute, Status};
use std::f64::consts::PI;

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

fn abs() -> RegraPlastica {
    RegraPlastica::de_fabrica().into_iter().find(|r| r.material == "ABS").unwrap()
}

/// Placa 40 x 30 x 5 em z de 0 a 5 e os pontos no plano XY: o boss cresce para baixo (inverter).
fn placa(regra: Option<RegraPlastica>, pts: &[[f64; 2]]) -> (Document, Id) {
    let mut d = Document::new("placa_plastica");
    d.plastico = regra;
    let a = sk_rect(&mut d, XY, [0., 0.], [40., 30.]);
    d.add_feature(ext(a, BoolOp::Add, "5"));
    let p = sk_pontos(&mut d, XY, pts);
    (d, p)
}

fn add_boss(d: &mut Document, b: Boss) -> Id {
    d.add_feature(FeatureKind::Plastico(RecursoPlastico::Boss(b)))
}

/// Volume do filete côncavo da base (Pappus): a região entre as duas retas tangentes e o arco.
/// `rb` é o raio da base do cone, `a` a saída em radianos e `rf` o raio do filete.
fn vol_filete_base(rb: f64, a: f64, rf: f64) -> f64 {
    if rf <= 0. {
        return 0.;
    }
    let teta = PI / 2. + a; // ângulo entre o plano e a parede, do lado do filete
    let fi = PI - teta; // ângulo do arco
    let dt = rf / (teta / 2.).tan();
    let p = [rb, 0.];
    let t1 = [rb + dt, 0.];
    let t2 = [rb - dt * a.sin(), dt * a.cos()];
    let c = [rb + dt, rf];
    // dois triângulos (P, T, C) formam a pipa; o setor sai dela
    let tri = rf * dt / 2.;
    let r_tri1 = (p[0] + t1[0] + c[0]) / 3.;
    let r_tri2 = (p[0] + t2[0] + c[0]) / 3.;
    let a_set = fi / 2. * rf * rf;
    let dist = 2. * rf * (fi / 2.).sin() / (3. * fi / 2.);
    let pc = [p[0] - c[0], p[1] - c[1]];
    let r_set = c[0] + dist * pc[0] / pc[0].hypot(pc[1]);
    let area = 2. * tri - a_set;
    let r = (tri * r_tri1 + tri * r_tri2 - a_set * r_set) / area;
    2. * PI * r * area
}

/// Boss pela conta: tronco de cone + filete da base - furo - rebaixo do inserto (+ nervuras sem saída).
fn vol_boss(d: f64, h: f64, saida: f64, rf: f64, furo: f64, prof: f64, ins: f64, ins_prof: f64) -> f64 {
    let a = saida.to_radians();
    let rt = d / 2.;
    let rb = rt + h * a.tan();
    let cone = PI * h / 3. * (rb * rb + rb * rt + rt * rt);
    let rh = furo / 2.;
    let ri = ins / 2.;
    let vazio = PI * rh * rh * prof + if ins > 0. { PI * (ri * ri - rh * rh) * ins_prof } else { 0. };
    cone + vol_filete_base(rb, a, rf) - vazio
}

/// Uma nervura radial fora do cilindro de raio r: placa de espessura t, largura l na base e altura hg.
fn vol_nervura(r: f64, t: f64, l: f64, hg: f64) -> f64 {
    let s = t / 2. * (r * r - t * t / 4.).sqrt() + r * r * (t / (2. * r)).asin();
    hg * (t * r + t * l / 2. - s)
}

fn esperado(vol: f64, tam: [f64; 3], minimo: [f64; 3]) -> Esperado {
    Esperado { vol, tol_vol: 1e-6, tam, tol_tam: 1e-3, minimo: Some(minimo), via_malha: false }
}

fn boss_abs(sk: Id) -> Boss {
    let mut b = Boss::novo(sk, Some(&abs()));
    b.reverse = true;
    b.inserto = "4.2".into();
    b.inserto_prof = "4".into();
    b
}

#[test]
fn boss_segue_a_regra_no_filete_e_na_saida() {
    if !kernel_disponivel() {
        return;
    }
    let r = abs();
    let (mut d, sk) = placa(Some(r.clone()), &[[20., 15.]]);
    let b = boss_abs(sk);
    assert_eq!(b.saida, "plastico_saida");
    assert_eq!(b.raio, "plastico_raio");
    assert_eq!(b.nervura_esp, "plastico_nervura");
    add_boss(&mut d, b.clone());
    // as medidas avaliadas são as da regra (1e-6)
    let vars = d.vars_com_plastico(d.eval_params().unwrap());
    let m = plastico_rec::boss_medidas(&b, &vars).unwrap();
    assert!((m.raio - r.raio).abs() < 1e-6 && (m.saida - r.saida).abs() < 1e-6, "{m:?}");
    assert_eq!(m.nervuras, 0, "10 mm de altura não passa de 3 diâmetros (24 mm)");
    let v = 40. * 30. * 5. + vol_boss(8., 10., r.saida, r.raio, 3., 8., 4.2, 4.);
    confere("boss ABS", &d, &esperado(v, [40., 30., 15.], [0., 0., -10.]));
}

#[test]
fn mudar_a_regra_muda_o_boss() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, sk) = placa(Some(abs()), &[[10., 10.], [30., 20.]]);
    add_boss(&mut d, boss_abs(sk));
    let um = vol_boss(8., 10., 1., 1., 3., 8., 4.2, 4.);
    confere("dois bosses", &d, &esperado(6000. + 2. * um, [40., 30., 15.], [0., 0., -10.]));
    {
        let r = d.plastico.as_mut().unwrap();
        r.saida = 2.;
        r.raio = 1.5;
    }
    let dois = vol_boss(8., 10., 2., 1.5, 3., 8., 4.2, 4.);
    assert!((dois - um).abs() > 1., "a conta precisa mudar para o teste valer");
    confere("regra nova", &d, &esperado(6000. + 2. * dois, [40., 30., 15.], [0., 0., -10.]));
}

#[test]
fn boss_alto_ganha_nervuras_com_a_espessura_da_regra() {
    if !kernel_disponivel() {
        return;
    }
    // regra sem mínimos de saída e raio, para o boss sem saída nem filete sair sem aviso
    let mut r = abs();
    r.saida_min = 0.;
    r.raio_min = 0.;
    r.avancado = true;
    let (mut d, sk) = placa(Some(r.clone()), &[[20., 15.]]);
    let mut b = Boss::novo(sk, Some(&r));
    b.reverse = true;
    b.diametro = "6".into();
    b.altura = "20".into();
    b.saida = "0".into();
    b.raio = "0".into();
    b.furo = "2.5".into();
    b.furo_prof = "16".into();
    add_boss(&mut d, b.clone());
    let vars = d.vars_com_plastico(d.eval_params().unwrap());
    let m = plastico_rec::boss_medidas(&b, &vars).unwrap();
    assert_eq!(m.nervuras, 4, "20 mm passa de 3 x 6 mm");
    assert!((m.nervura_esp - r.nervura()).abs() < 1e-6);
    // nervuras: largura padrão = metade do diâmetro, altura padrão = 70% da altura
    let v = 6000. + vol_boss(6., 20., 0., 0., 2.5, 16., 0., 0.) + 4. * vol_nervura(3., r.nervura(), 3., 14.);
    confere("boss com nervuras", &d, &esperado(v, [40., 30., 25.], [0., 0., -20.]));
    // a parede da regra mais grossa engrossa as nervuras
    d.plastico.as_mut().unwrap().espessura = 3.;
    let t = 3. * r.nervura_fracao;
    let v2 = 6000. + vol_boss(6., 20., 0., 0., 2.5, 16., 0., 0.) + 4. * vol_nervura(3., t, 3., 14.);
    confere("nervura da regra nova", &d, &esperado(v2, [40., 30., 25.], [0., 0., -20.]));
}

#[test]
fn boss_alto_sem_nervuras_e_sobrescrita_avisam_na_arvore() {
    if !kernel_disponivel() {
        return;
    }
    let mut r = abs();
    r.avancado = true;
    let (mut d, sk) = placa(Some(r), &[[20., 15.]]);
    let mut b = boss_abs(sk);
    b.altura = "30".into();
    b.nervuras = "0".into();
    b.raio = "0.2".into();
    let id = add_boss(&mut d, b);
    let res = recompute(&d);
    match res.status.get(&id) {
        Some(Status::Warning(m)) => {
            assert!(m.contains("sem nervuras"), "{m}");
            assert!(m.contains("Regra plástica") && m.contains("filete da base") && m.contains("abaixo do mínimo"), "{m}");
        }
        s => panic!("esperava aviso, veio {s:?}"),
    }
}

#[test]
fn boss_sem_o_sketch_e_erro_nomeado_e_medida_ruim_tambem() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, sk) = placa(Some(abs()), &[[20., 15.]]);
    let mut b = boss_abs(sk);
    b.furo = "9".into();
    let id = add_boss(&mut d, b);
    match recompute(&d).status.get(&id) {
        Some(Status::Error(m)) => assert!(m.contains("menor que o diâmetro"), "{m}"),
        s => panic!("esperava erro, veio {s:?}"),
    }
    d.features.retain(|f| f.id != sk);
    match recompute(&d).status.get(&id) {
        Some(Status::Error(m)) => assert!(m.contains("referência perdida"), "{m}"),
        s => panic!("esperava erro de referência, veio {s:?}"),
    }
}
