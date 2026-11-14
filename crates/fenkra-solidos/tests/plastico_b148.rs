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

// ------------------------------------------------------------------ snap-fit

/// Placa 40 x 30 x 5 e uma linha de (10, y) a (30, y) no plano XY: o snap cresce para baixo.
fn placa_linha(regra: Option<RegraPlastica>, y: f64) -> (Document, Id) {
    let mut d = Document::new("placa_snap");
    d.plastico = regra;
    let a = sk_rect(&mut d, XY, [0., 0.], [40., 30.]);
    d.add_feature(ext(a, BoolOp::Add, "5"));
    let p = sk_poligono_aberto(&mut d, [10., y], [30., y]);
    (d, p)
}

fn sk_poligono_aberto(d: &mut Document, a: [f64; 2], b: [f64; 2]) -> Id {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    let pa = data.add_point(&mut al, a[0], a[1]);
    let pb = data.add_point(&mut al, b[0], b[1]);
    data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: pa, b: pb }, false);
    d.add_feature(FeatureKind::Sketch(Sketch { plane: XY, data }))
}

/// Volume do snap pela conta: viga + gancho triangular + dois filetes côncavos da raiz, vezes a largura.
fn vol_snap(larg: f64, t: f64, l: f64, y: f64, h: f64, r: f64) -> f64 {
    larg * (t * l + y * h / 2. + 2. * r * r * (1. - PI / 4.))
}

fn add_snap(d: &mut Document, s: Snap) -> Id {
    d.add_feature(FeatureKind::Plastico(RecursoPlastico::Snap(s)))
}

#[test]
fn snap_segue_a_regra_e_muda_com_ela() {
    if !kernel_disponivel() {
        return;
    }
    let r = abs();
    let (mut d, sk) = placa_linha(Some(r.clone()), 15.);
    let mut s = Snap::novo(sk, Some(&r));
    assert_eq!((s.espessura.as_str(), s.raio.as_str()), ("plastico_espessura", "plastico_raio"));
    s.reverse = true;
    add_snap(&mut d, s.clone());
    let vars = d.vars_com_plastico(d.eval_params().unwrap());
    let m = plastico_rec::snap_medidas(&s, &vars).unwrap();
    assert!((m.espessura - r.espessura).abs() < 1e-6 && (m.raio - r.raio).abs() < 1e-6);
    // 1,5 x 2 x 1 / 15² = 1,33%, abaixo dos 2% admissíveis
    assert!((m.deformacao() - 1.5 * 2. * 1. / 225.).abs() < 1e-12);
    let v = 6000. + vol_snap(20., r.espessura, 18., 1., 3., r.raio);
    confere("snap ABS", &d, &esperado(v, [40., 30., 23.], [0., 0., -18.]));
    d.plastico.as_mut().unwrap().espessura = 2.5;
    d.plastico.as_mut().unwrap().raio = 1.5;
    let v2 = 6000. + vol_snap(20., 2.5, 18., 1., 3., 1.5);
    confere("snap regra nova", &d, &esperado(v2, [40., 30., 23.], [0., 0., -18.]));
}

#[test]
fn snap_gancho_vai_para_o_lado_escolhido() {
    if !kernel_disponivel() {
        return;
    }
    let r = abs();
    // linha a 2,5 mm da borda: o gancho para fora passa 0,5 mm da placa (2,5 - t - y)
    let (mut d, sk) = placa_linha(Some(r.clone()), 2.5);
    let mut s = Snap::novo(sk, Some(&r));
    s.reverse = true;
    let id = add_snap(&mut d, s.clone());
    let v = 6000. + vol_snap(20., 2., 18., 1., 3., 1.);
    confere("gancho para fora", &d, &esperado(v, [40., 30.5, 23.], [0., -0.5, -18.]));
    if let FeatureKind::Plastico(RecursoPlastico::Snap(s)) = &mut d.features.iter_mut().find(|f| f.id == id).unwrap().kind {
        s.lado = true;
    }
    confere("gancho para dentro", &d, &esperado(v, [40., 30., 23.], [0., 0., -18.]));
}

#[test]
fn snap_acima_da_deformacao_admissivel_avisa_na_arvore() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, sk) = placa_linha(Some(abs()), 15.);
    let mut s = Snap::novo(sk, d.plastico.as_ref());
    s.reverse = true;
    s.gancho = "2.5".into(); // 1,5 x 2 x 2,5 / 15² = 3,33%
    let id = add_snap(&mut d, s);
    let res = recompute(&d);
    assert_eq!(res.bodies.len(), 1);
    let v = 6000. + vol_snap(20., 2., 18., 2.5, 3., 1.);
    assert!((volume_total(&res) - v).abs() < 1e-6 * v, "o snap continua sendo feito");
    match res.status.get(&id) {
        Some(Status::Warning(m)) => assert!(m.contains("3.33%") && m.contains("admissível de 2.00%"), "{m}"),
        s => panic!("esperava aviso de deformação, veio {s:?}"),
    }
    // o sketch sem linha é erro nomeado
    let (mut d2, sk2) = placa(Some(abs()), &[[20., 15.]]);
    let id2 = add_snap(&mut d2, Snap::novo(sk2, None));
    match recompute(&d2).status.get(&id2) {
        Some(Status::Error(m)) => assert!(m.contains("pelo menos uma linha"), "{m}"),
        s => panic!("esperava erro, veio {s:?}"),
    }
}

// ------------------------------------------------------------------ grelha

/// Placa 40 x 30 x 5 e a região da grelha de (5, 5) a (35, 25) no plano XY (face de baixo).
fn placa_grelha(regra: Option<RegraPlastica>) -> (Document, Id) {
    let mut d = Document::new("placa_grelha");
    d.plastico = regra;
    let a = sk_rect(&mut d, XY, [0., 0.], [40., 30.]);
    d.add_feature(ext(a, BoolOp::Add, "5"));
    let g = sk_rect(&mut d, XY, [5., 5.], [35., 25.]);
    (d, g)
}

fn add_grelha(d: &mut Document, g: Grelha) -> Id {
    d.add_feature(FeatureKind::Plastico(RecursoPlastico::Grelha(g)))
}

fn grelha_de(d: &mut Document, id: Id) -> &mut Grelha {
    match &mut d.features.iter_mut().find(|f| f.id == id).unwrap().kind {
        FeatureKind::Plastico(RecursoPlastico::Grelha(g)) => g,
        _ => unreachable!(),
    }
}

#[test]
fn grelha_vaza_a_parede_e_deixa_as_barras_da_regra() {
    if !kernel_disponivel() {
        return;
    }
    let r = abs();
    let (mut d, sk) = placa_grelha(Some(r.clone()));
    let mut g = Grelha::novo(sk, Some(&r));
    assert_eq!(g.nervura, "plastico_nervura");
    g.reverse = true;
    let id = add_grelha(&mut d, g);
    // abertura 26 x 16 (borda 2); barras de 1,2 com eixo em y = 10, 15 e 20, atravessando os 5 mm
    let v = 6000. - 5. * 26. * (16. - 3. * 1.2);
    confere("grelha", &d, &esperado(v, [40., 30., 5.], [0.; 3]));
    // cruzada: barras também em x = 10, 15, 20, 25 e 30
    grelha_de(&mut d, id).cruzada = true;
    confere("grelha cruzada", &d, &esperado(6000. - 5. * (26. - 5. * 1.2) * (16. - 3. * 1.2), [40., 30., 5.], [0.; 3]));
    // a 90 graus, só as barras em x
    grelha_de(&mut d, id).cruzada = false;
    grelha_de(&mut d, id).angulo = "90".into();
    confere("grelha a 90°", &d, &esperado(6000. - 5. * 16. * (26. - 5. * 1.2), [40., 30., 5.], [0.; 3]));
    // a parede da regra mais grossa engrossa as barras (0,6 x 3 = 1,8)
    grelha_de(&mut d, id).angulo = "0".into();
    d.plastico.as_mut().unwrap().espessura = 3.;
    confere("barra da regra nova", &d, &esperado(6000. - 5. * 26. * (16. - 3. * 1.8), [40., 30., 5.], [0.; 3]));
}

#[test]
fn grelha_com_barra_fora_da_regra_avisa_e_passo_ruim_e_erro() {
    if !kernel_disponivel() {
        return;
    }
    let mut r = abs();
    r.avancado = true;
    let (mut d, sk) = placa_grelha(Some(r));
    let mut g = Grelha::novo(sk, d.plastico.as_ref());
    g.reverse = true;
    g.nervura = "2".into(); // passa de 75% da parede de 2 mm
    let id = add_grelha(&mut d, g);
    match recompute(&d).status.get(&id) {
        Some(Status::Warning(m)) => assert!(m.contains("barra da grelha") && m.contains("chupa"), "{m}"),
        s => panic!("esperava aviso da regra, veio {s:?}"),
    }
    grelha_de(&mut d, id).passo = "1.5".into();
    match recompute(&d).status.get(&id) {
        Some(Status::Error(m)) => assert!(m.contains("passo da grelha"), "{m}"),
        s => panic!("esperava erro, veio {s:?}"),
    }
    // vazando para o lado sem material, a grelha não acha a parede
    grelha_de(&mut d, id).passo = "5".into();
    grelha_de(&mut d, id).reverse = false;
    match recompute(&d).status.get(&id) {
        Some(Status::Error(m)) => assert!(m.contains("não encontra a parede"), "{m}"),
        s => panic!("esperava erro, veio {s:?}"),
    }
}

// ------------------------------------------------------------------ lip e ranhura

/// Caixa 60 x 40 x 20 com casca da regra aberta em cima e o contorno externo no plano da borda (z = 20).
fn caixa_borda(regra: Option<RegraPlastica>) -> (Document, Id) {
    let mut d = Document::new("caixa_lip");
    d.plastico = regra;
    let a = sk_rect(&mut d, XY, [0., 0.], [60., 40.]);
    d.add_feature(ext(a, BoolOp::Add, "20"));
    d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[30., 20., 20.]], thickness: "plastico_espessura".into(), outward: false, face_keys: vec![], closed: false, regras: vec![] }));
    let topo = d.add_offset_plane(XY, "20");
    let c = sk_rect(&mut d, PlaneRef::Datum(topo), [0., 0.], [60., 40.]);
    (d, c)
}

fn vol_caixa_aberta(t: f64) -> f64 {
    60. * 40. * 20. - (60. - 2. * t) * (40. - 2. * t) * (20. - t)
}

/// Área entre dois retângulos 60 x 40 recuados `a` e `b` para dentro.
fn anel(a: f64, b: f64) -> f64 {
    (60. - 2. * a) * (40. - 2. * a) - (60. - 2. * b) * (40. - 2. * b)
}

#[test]
fn lip_sobe_da_borda_na_metade_de_dentro_da_parede() {
    if !kernel_disponivel() {
        return;
    }
    let r = abs();
    let (mut d, sk) = caixa_borda(Some(r.clone()));
    let l = Lip::novo(sk, Some(&r), false);
    assert_eq!(l.recuo, "plastico_espessura/2");
    let id = d.add_feature(FeatureKind::Plastico(RecursoPlastico::Lip(l)));
    // parede 2: lip de 1 a 2 mm para dentro do contorno, 2 mm de altura
    let v = vol_caixa_aberta(2.) + anel(1., 2.) * 2.;
    confere("lip", &d, &esperado(v, [60., 40., 22.], [0.; 3]));
    // a regra mais grossa leva a casca e o lip juntos
    d.plastico.as_mut().unwrap().espessura = 3.;
    let v3 = vol_caixa_aberta(3.) + anel(1.5, 3.) * 2.;
    confere("lip da regra nova", &d, &esperado(v3, [60., 40., 22.], [0.; 3]));
    // virar ranhura: o mesmo anel com 0,1 de folga dos lados e no fundo; por dentro passa da parede
    d.plastico.as_mut().unwrap().espessura = 2.;
    if let FeatureKind::Plastico(RecursoPlastico::Lip(l)) = &mut d.features.iter_mut().find(|f| f.id == id).unwrap().kind {
        l.ranhura = true;
    }
    let vr = vol_caixa_aberta(2.) - anel(0.9, 2.) * 2.1;
    confere("ranhura", &d, &esperado(vr, [60., 40., 20.], [0.; 3]));
}

#[test]
fn lip_e_ranhura_encaixam_com_a_folga() {
    if !kernel_disponivel() {
        return;
    }
    let r = abs();
    let vars = r.vars(&Default::default());
    let lip = plastico_rec::lip_medidas(&Lip::novo(Id(1), Some(&r), false), &vars).unwrap();
    let ran = plastico_rec::lip_medidas(&Lip::novo(Id(1), Some(&r), true), &vars).unwrap();
    // a ranhura envolve o lip com a folga de cada lado e no fundo (1e-6)
    assert!((lip.fora - ran.fora - 0.1).abs() < 1e-6 && (ran.dentro - lip.dentro - 0.1).abs() < 1e-6);
    assert!((ran.altura - lip.altura - 0.1).abs() < 1e-6);
    // sem o contorno é erro nomeado
    let (mut d, sk) = caixa_borda(Some(r));
    let id = d.add_feature(FeatureKind::Plastico(RecursoPlastico::Lip(Lip::novo(sk, d.plastico.as_ref(), false))));
    d.features.retain(|f| f.id != sk);
    match recompute(&d).status.get(&id) {
        Some(Status::Error(m)) => assert!(m.contains("referência perdida"), "{m}"),
        s => panic!("esperava erro de referência, veio {s:?}"),
    }
}
