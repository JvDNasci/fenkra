// SPDX-License-Identifier: LGPL-2.1-or-later
//! B356: prévia leve em todos os comandos. Mede o recálculo de cada comando pesado com o campo
//! mudando (cache aquecido), completo contra leve, e confere que o leve bate em posição e tamanho
//! com o resultado final.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, recompute_cached, Cache};
use std::time::Instant;

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

fn s(x: &str) -> String {
    x.into()
}

/// Placa 120 x 80 x 20 com uma grade de furos (muitas arestas).
fn placa(nx: usize, ny: usize) -> Document {
    let mut d = Document::new("placa");
    let sk = sk_rect(&mut d, XY, [-60., -40.], [60., 40.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let topo = d.add_offset_plane(XY, "20");
    let mut cs = vec![];
    for i in 0..nx {
        for j in 0..ny {
            cs.push([-50. + 100. * i as f64 / (nx.max(2) - 1) as f64, -30. + 60. * j as f64 / (ny.max(2) - 1) as f64]);
        }
    }
    let sc = sk_pontos(&mut d, PlaneRef::Datum(topo), &cs);
    d.add_feature(furo_passante(sc, "4"));
    d
}

fn muda_valor(d: &mut Document, id: Id, v: &str) {
    match d.feature_mut(id).map(|f| &mut f.kind) {
        Some(FeatureKind::Fillet(o)) | Some(FeatureKind::Chamfer(o)) => o.value = s(v),
        Some(FeatureKind::Shell(x)) => x.thickness = s(v),
        Some(FeatureKind::Pattern(p)) => {
            if let PatternKind::Linear { count, .. } | PatternKind::Circular { count, .. } = &mut p.kind {
                *count = s(v);
            }
        }
        _ => panic!("tipo sem campo"),
    }
}

fn mede(nome: &str, mut d: Document, id: Id, v1: &str, v2: &str) {
    if !kernel_disponivel() {
        return;
    }
    muda_valor(&mut d, id, v1);
    let mut cache = Cache::default();
    let _ = recompute_cached(&d, &mut cache);
    muda_valor(&mut d, id, v2);
    let t = Instant::now();
    let r = recompute_cached(&d, &mut cache);
    let completo = t.elapsed().as_secs_f64() * 1000.;
    let mut cache2 = Cache::default();
    cache2.leve = Some(id);
    muda_valor(&mut d, id, v1);
    let _ = recompute_cached(&d, &mut cache2);
    muda_valor(&mut d, id, v2);
    let t = Instant::now();
    let l = recompute_cached(&d, &mut cache2);
    let leve = t.elapsed().as_secs_f64() * 1000.;
    println!("B356 {nome}: completo {completo:.0} ms, leve {leve:.0} ms, vol {:.0} -> {:.0}, status {:?}", volume_total(&r), volume_total(&l), r.status.get(&id));
}

#[test]
fn medida_filete() {
    let mut d = placa(6, 4);
    let id = d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("1"), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
    mede("filete", d, id, "1", "1.2");
}

#[test]
fn medida_chanfro() {
    let mut d = placa(6, 4);
    let id = d.add_feature(FeatureKind::Chamfer(EdgeOp { value: s("1"), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
    mede("chanfro", d, id, "1", "1.2");
}

#[test]
fn medida_casca() {
    let mut d = placa(6, 4);
    let id = d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0., 0., 20.]], thickness: s("2"), outward: false, face_keys: vec![], closed: false, regras: vec![] }));
    mede("casca", d, id, "2", "2.2");
}

#[test]
fn medida_padrao_linear() {
    let mut d = Document::new("p");
    let sk = sk_rect(&mut d, XY, [-10., -40.], [400., 40.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let topo = d.add_offset_plane(XY, "20");
    let sp = sk_circulos(&mut d, PlaneRef::Datum(topo), &[([0., 0.], 3.)]);
    let furo = d.add_feature(ext(sp, BoolOp::Cut, "20"));
    let id = d.add_feature(FeatureKind::Pattern(Pattern {
        kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: s("30"), spacing: s("12"), dir2: Some(AxisRef::Origin(OriginAxis::Y)), count2: s("3"), spacing2: s("15"), total: false },
        sources: vec![furo],
        skip: Default::default(),
    }));
    mede("padrao_linear", d, id, "30", "31");
}

#[test]
fn medida_padrao_circular() {
    let mut d = Document::new("p");
    let sk = sk_rect(&mut d, XY, [-60., -60.], [60., 60.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let topo = d.add_offset_plane(XY, "20");
    let sp = sk_circulos(&mut d, PlaneRef::Datum(topo), &[([40., 0.], 3.)]);
    let furo = d.add_feature(ext(sp, BoolOp::Cut, "20"));
    let id = d.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Z), count: s("40"), angle: s("360") }, sources: vec![furo], skip: Default::default() }));
    mede("padrao_circular", d, id, "40", "41");
}

#[test]
fn medida_grande_filete_chanfro_casca() {
    for (nome, k) in [("filete_grande", 0), ("chanfro_grande", 1), ("casca_grande", 2)] {
        let mut d = placa(20, 12);
        let op = EdgeOp { value: s("0.5"), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() };
        let (id, a, b) = match k {
            0 => (d.add_feature(FeatureKind::Fillet(op)), "0.5", "0.6"),
            1 => (d.add_feature(FeatureKind::Chamfer(op)), "0.5", "0.6"),
            _ => (d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0., 0., 20.]], thickness: s("2"), outward: false, face_keys: vec![], closed: false, regras: vec![] })), "2", "2.2"),
        };
        mede(nome, d, id, a, b);
    }
}

#[test]
fn medida_padrao_grande() {
    let mut d = Document::new("p");
    let sk = sk_rect(&mut d, XY, [-10., -10.], [600., 400.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let topo = d.add_offset_plane(XY, "20");
    let sp = sk_circulos(&mut d, PlaneRef::Datum(topo), &[([0., 0.], 3.)]);
    let furo = d.add_feature(ext(sp, BoolOp::Cut, "20"));
    let id = d.add_feature(FeatureKind::Pattern(Pattern {
        kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: s("40"), spacing: s("12"), dir2: Some(AxisRef::Origin(OriginAxis::Y)), count2: s("25"), spacing2: s("12"), total: false },
        sources: vec![furo],
        skip: Default::default(),
    }));
    mede("padrao_1000", d, id, "40", "39");
}

#[test]
fn medida_loft() {
    let mut d = Document::new("l");
    let mut secs = vec![];
    for i in 0..6 {
        let pl = if i == 0 { XY } else { PlaneRef::Datum(d.add_offset_plane(XY, &format!("{}", i * 15))) };
        let r = 20. - i as f64 * 2.;
        secs.push(sk_circulos(&mut d, pl, &[([0., 0.], r)]));
    }
    let id = d.add_feature(FeatureKind::Loft(Loft { sections: secs, ruled: false, op: BoolOp::Add, closed: false, torcao: vec![], inicio: Default::default(), fim: Default::default() }));
    if !kernel_disponivel() { return; }
    let mut cache = Cache::default();
    let _ = recompute_cached(&d, &mut cache);
    let t = Instant::now();
    let r = recompute(&d);
    println!("B356 loft: {:.0} ms, status {:?}", t.elapsed().as_secs_f64() * 1000., r.status.get(&id));
}

// ------------------------------------------------------------------ comportamento da prévia leve

fn perto(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn caixa_dos_cantos(c: &[[f64; 3]; 8]) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for p in c {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    (lo, hi)
}

fn leve_e_final(d: &Document, id: Id) -> (fenkra_solidos::RecomputeResult, fenkra_solidos::RecomputeResult) {
    let mut cache = Cache::default();
    cache.leve = Some(id);
    let leve = recompute_cached(d, &mut cache);
    cache.leve = None;
    let final_ = recompute_cached(d, &mut cache);
    (leve, final_)
}

#[test]
fn filete_leve_deixa_o_corpo_e_diz_onde_e_quanto() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = placa(2, 2);
    let sem = volume_total(&recompute(&d));
    // aresta do topo, paralela a X em y = -40
    let id = d.add_feature(filete("3", vec![[0., -40., 20.]], vec![]));
    let (leve, final_) = leve_e_final(&d, id);
    assert!(matches!(leve.status.get(&id), Some(fenkra_solidos::Status::Ok)), "{:?}", leve.status.get(&id));
    assert!(perto(volume_total(&leve), sem), "a prévia não mexe no corpo");
    let v = leve.previas.iter().find(|p| p.feature == id).expect("prévia");
    assert!(perto(v.valor, 3.) && !v.chanfro && v.arestas.len() == 1);
    assert!(v.arestas[0][1] < -39.9 && perto(v.arestas[0][2], 20.), "{:?}", v.arestas);
    // ao concluir, o cálculo real: o filete tira material e a caixa não muda
    let esperado = sem - (1. - std::f64::consts::FRAC_PI_4) * 9. * 120.;
    assert!(matches!(final_.status.get(&id), Some(fenkra_solidos::Status::Ok)));
    assert!((volume_total(&final_) - esperado).abs() < 1., "{} contra {esperado}", volume_total(&final_));
    assert!(final_.previas.is_empty());
    assert_eq!(caixa_da_malha(&leve), caixa_da_malha(&final_));
}

#[test]
fn casca_leve_recua_a_espessura_e_abre_a_face_escolhida() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("c");
    let sk = sk_rect(&mut d, XY, [-30., -20.], [30., 20.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let id = d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0., 0., 20.]], thickness: s("3"), outward: false, face_keys: vec![], closed: false, regras: vec![] }));
    let (leve, final_) = leve_e_final(&d, id);
    assert!(perto(volume_total(&leve), 60. * 40. * 20.));
    let v = leve.previas.iter().find(|p| p.feature == id).expect("prévia");
    let (lo, hi) = caixa_dos_cantos(&v.caixas[0]);
    // cavidade: recuada 3 mm nas laterais e no fundo, aberta em cima
    assert!(perto(lo[0], -27.) && perto(hi[0], 27.) && perto(lo[1], -17.) && perto(hi[1], 17.));
    assert!(perto(lo[2], 3.) && perto(hi[2], 20.), "{lo:?} {hi:?}");
    // o volume final é o da caixa menos essa mesma cavidade
    let esperado = 60. * 40. * 20. - 54. * 34. * 17.;
    assert!((volume_total(&final_) - esperado).abs() < 1., "{} contra {esperado}", volume_total(&final_));
    assert_eq!(caixa_da_malha(&leve), caixa_da_malha(&final_));
}

#[test]
fn padrao_leve_poe_a_caixa_em_cada_instancia() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("p");
    let sk = sk_rect(&mut d, XY, [-10., -20.], [100., 20.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let topo = d.add_offset_plane(XY, "20");
    let sp = sk_rect(&mut d, PlaneRef::Datum(topo), [0., -2.], [4., 2.]);
    let furo = d.add_feature(ext(sp, BoolOp::Cut, "5"));
    let sem_padrao = volume_total(&recompute(&d));
    let id = d.add_feature(FeatureKind::Pattern(Pattern {
        kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: s("4"), spacing: s("20"), dir2: None, count2: s("1"), spacing2: s("0"), total: false },
        sources: vec![furo],
        skip: Default::default(),
    }));
    let (leve, final_) = leve_e_final(&d, id);
    assert!(perto(volume_total(&leve), sem_padrao), "a prévia não replica nada");
    let v = leve.previas.iter().find(|p| p.feature == id).expect("prévia");
    assert_eq!(v.caixas.len(), 3, "as três cópias, sem a original");
    for (i, c) in v.caixas.iter().enumerate() {
        let (lo, hi) = caixa_dos_cantos(c);
        let x0 = 20. * (i + 1) as f64;
        assert!(perto(lo[0], x0) && perto(hi[0], x0 + 4.) && perto(lo[1], -2.) && perto(hi[1], 2.), "cópia {i}: {lo:?} {hi:?}");
        assert!(perto(hi[2], 20.), "a cópia fica na face de cima");
    }
    // o resultado final tira exatamente quatro bolsos
    assert!((volume_total(&final_) - (110. * 40. * 20. - 4. * 4. * 4. * 5.)).abs() < 1., "{}", volume_total(&final_));
    assert_eq!(caixa_da_malha(&leve), caixa_da_malha(&final_));
}

#[test]
fn padrao_circular_leve_gira_a_caixa_em_volta_do_eixo() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("pc");
    let sk = sk_rect(&mut d, XY, [-60., -60.], [60., 60.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let topo = d.add_offset_plane(XY, "20");
    let sp = sk_rect(&mut d, PlaneRef::Datum(topo), [30., -2.], [40., 2.]);
    let furo = d.add_feature(ext(sp, BoolOp::Cut, "5"));
    let id = d.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Z), count: s("4"), angle: s("360") }, sources: vec![furo], skip: Default::default() }));
    let (leve, _) = leve_e_final(&d, id);
    let v = leve.previas.iter().find(|p| p.feature == id).expect("prévia");
    assert_eq!(v.caixas.len(), 3);
    // a cópia a 90 graus fica em y de 30 a 40 (x entre -2 e 2)
    let (lo, hi) = caixa_dos_cantos(&v.caixas[0]);
    assert!((lo[1] - 30.).abs() < 1e-6 && (hi[1] - 40.).abs() < 1e-6 && (lo[0] + 2.).abs() < 1e-6 && (hi[0] - 2.).abs() < 1e-6, "{lo:?} {hi:?}");
}

#[test]
fn previa_leve_do_padrao_e_bem_mais_rapida() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("p");
    let sk = sk_rect(&mut d, XY, [-10., -10.], [400., 200.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let topo = d.add_offset_plane(XY, "20");
    let sp = sk_circulos(&mut d, PlaneRef::Datum(topo), &[([0., 0.], 3.)]);
    let furo = d.add_feature(ext(sp, BoolOp::Cut, "20"));
    let id = d.add_feature(FeatureKind::Pattern(Pattern {
        kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: s("30"), spacing: s("12"), dir2: Some(AxisRef::Origin(OriginAxis::Y)), count2: s("12"), spacing2: s("12"), total: false },
        sources: vec![furo],
        skip: Default::default(),
    }));
    let mut cache = Cache::default();
    let t = Instant::now();
    let _ = recompute_cached(&d, &mut cache);
    let completo = t.elapsed();
    let mut cache = Cache::default();
    cache.leve = Some(id);
    let _ = recompute_cached(&d, &mut cache);
    muda_valor(&mut d, id, "29");
    let t = Instant::now();
    let _ = recompute_cached(&d, &mut cache);
    let leve = t.elapsed();
    println!("B356 padrão de 360 instâncias: completo {completo:?}, leve {leve:?}");
    assert!(leve * 5 < completo, "{leve:?} contra {completo:?}");
}

/// B356: multisseções com o comando aberto não costura os perfis: o corpo fica como estava e a
/// prévia é a caixa que as seções ocupam; ao concluir, o tronco real tem a mesma caixa.
#[test]
fn multissecoes_leve_mostra_a_caixa_das_secoes() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("loft");
    let alto = d.add_offset_plane(XY, "30");
    let a = sk_rect(&mut d, XY, [-10., -10.], [10., 10.]);
    let b = sk_rect(&mut d, PlaneRef::Datum(alto), [-5., -5.], [5., 5.]);
    let id = d.add_feature(FeatureKind::Loft(Loft { sections: vec![a, b], ruled: true, closed: false, op: BoolOp::Add, torcao: vec![], inicio: Default::default(), fim: Default::default() }));
    let (leve, final_) = leve_e_final(&d, id);
    assert!(matches!(leve.status.get(&id), Some(fenkra_solidos::Status::Ok)), "{:?}", leve.status.get(&id));
    assert!(volume_total(&leve) < 1e-6, "a prévia não costura nada");
    let v = leve.previas.iter().find(|p| p.feature == id).expect("prévia");
    let (lo, hi) = caixa_dos_cantos(&v.caixas[0]);
    assert!(perto(lo[0], -10.) && perto(hi[0], 10.) && perto(lo[2], 0.) && perto(hi[2], 30.), "{lo:?} {hi:?}");
    let tronco = 30. / 3. * (400. + 100. + 200.);
    assert!((volume_total(&final_) - tronco).abs() < 1., "{} contra {tronco}", volume_total(&final_));
    assert!(final_.previas.is_empty());
}
