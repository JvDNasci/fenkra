// SPDX-License-Identifier: LGPL-2.1-or-later
//! Chapa pelo caminho da interface (B488). Os testes antigos montam a aba já com a aresta; a
//! interface faz diferente: o comando nasce com o painel aberto (`Cache::leve` = o comando), sem
//! aresta (ponto (0,0,0)) quando nada estava selecionado, recebe o ponto médio da aresta clicada (em
//! `f32`, como a malha) e grava a chave promovida pelo cálculo. Aqui o documento segue esses passos,
//! com o mesmo cache, e cada passo confere status, volume e caixa envolvente.

use super::*;
use std::f64::consts::PI;

fn s(x: &str) -> String {
    x.to_string()
}

fn aba_vazia() -> FeatureKind {
    FeatureKind::SmFlange(SmFlange { edge: [0.; 3], length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None })
}

fn valido(st: Option<&Status>) -> bool {
    matches!(st, Some(Status::Ok | Status::Warning(_)))
}

/// Volume de uma aba de 20 mm a 90° (espessura 2, raio 2) de largura `w`: arco mais trecho reto.
fn volume_aba(w: f64) -> f64 {
    PI / 4. * (16. - 4.) * w + 20. * 2. * w
}

fn caixa_de(d: &Document) -> ([f64; 3], [f64; 3]) {
    final_shape(d).expect("corpo").bbox().expect("caixa")
}

fn perto3(a: [f64; 3], b: [f64; 3], tol: f64) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() <= tol)
}

/// Documento novo da interface: parâmetros padrão da chapa e o retângulo 100 x 50 no XY.
fn documento_com_sketch() -> (Document, Id) {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams::default());
    let mut data = SketchData::default();
    let mut n = 7_700_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [0., 0.], [100., 50.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    (d, sk)
}

/// Parede base como a interface: painel aberto (prévia) e depois concluído.
fn parede_base(d: &mut Document, sk: Id, cache: &mut Cache) -> Id {
    let w = d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    cache.leve = Some(w);
    let r = recompute_cached(d, cache);
    assert!(valido(r.status.get(&w)), "parede com o painel aberto: {:?}", r.status.get(&w));
    assert_eq!(r.bodies.len(), 1, "a parede base aparece já com o painel aberto (não só a caixa da prévia)");
    assert!((r.bodies[0].mass.volume - 10_000.).abs() < 1e-6, "{}", r.bodies[0].mass.volume);
    cache.leve = None;
    let r = recompute_cached(d, cache);
    assert!(valido(r.status.get(&w)), "{:?}", r.status.get(&w));
    w
}

/// Aba como a interface: o comando abre sem aresta, a aresta clicada chega depois, conclui, e a
/// chave promovida volta para o documento.
fn aba_pela_interface(d: &mut Document, cache: &mut Cache, aresta: [f64; 3]) -> Id {
    let id = d.add_feature(aba_vazia());
    cache.leve = Some(id);
    let r = recompute_cached(d, cache);
    assert_eq!(r.status.get(&id), Some(&Status::Inactive), "aba sem aresta espera o clique, sem erro");
    if let FeatureKind::SmFlange(f) = &mut d.features.last_mut().unwrap().kind {
        f.edge = aresta.map(|v| v as f32 as f64);
    }
    let r = recompute_cached(d, cache);
    assert!(valido(r.status.get(&id)), "aba {aresta:?} com o painel aberto: {:?}", r.status.get(&id));
    cache.leve = None;
    let r = recompute_cached(d, cache);
    assert!(valido(r.status.get(&id)), "aba {aresta:?} concluída: {:?}", r.status.get(&id));
    if let (FeatureKind::SmFlange(f), Some(Promoted::Edges(k))) = (&mut d.features.last_mut().unwrap().kind, r.promoted.get(&id)) {
        if k.len() == 1 {
            f.edge_key = k[0].clone();
        }
    }
    id
}

#[test]
fn chapa_fluxo_base_e_uma_aba() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, sk) = documento_com_sketch();
    let mut cache = Cache::default();
    parede_base(&mut d, sk, &mut cache);
    aba_pela_interface(&mut d, &mut cache, [100., 25., 2.]);
    let r = recompute_cached(&d, &mut cache);
    assert!(d.features.iter().all(|f| valido(r.status.get(&f.id))), "{:?}", r.status);
    let v = r.bodies[0].mass.volume;
    assert!((v - (10_000. + volume_aba(50.))).abs() < 1e-3 * v, "volume {v}");
    let (lo, hi) = caixa_de(&d);
    assert!(perto3(lo, [0., 0., 0.], 1e-3) && perto3(hi, [104., 50., 24.], 1e-3), "caixa {lo:?} {hi:?}");
}

/// Base com quatro abas (uma por aresta de cima): cantos com a folga mínima, recalcula, planifica.
#[test]
fn chapa_fluxo_base_e_quatro_abas_com_cantos() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, sk) = documento_com_sketch();
    let mut cache = Cache::default();
    parede_base(&mut d, sk, &mut cache);
    for e in [[100., 25., 2.], [50., 50., 2.], [0., 25., 2.], [50., 0., 2.]] {
        aba_pela_interface(&mut d, &mut cache, e);
    }
    let r = recompute_cached(&d, &mut cache);
    assert!(d.features.iter().all(|f| valido(r.status.get(&f.id))), "{:?}", r.status);
    let v = r.bodies[0].mass.volume;
    let esperado = 10_000. + 2. * volume_aba(100.) + 2. * volume_aba(50.);
    assert!((v - esperado).abs() < 1e-3 * esperado, "volume {v} contra {esperado}");
    let (lo, hi) = caixa_de(&d);
    assert!(perto3(lo, [-4., -4., 0.], 1e-3) && perto3(hi, [104., 54., 24.], 1e-3), "caixa {lo:?} {hi:?}");
    // planificada: uma cruz plana de espessura 2, cada aba com o comprimento desenvolvido
    d.sheet.as_mut().unwrap().flat = true;
    let r = recompute_cached(&d, &mut cache);
    assert!(d.features.iter().all(|f| valido(r.status.get(&f.id))), "planificada: {:?}", r.status);
    let ba = chapa::compensacao(2., 2., 90., 0.44).ba;
    let vp = r.bodies[0].mass.volume;
    let plano = 10_000. + 2. * (ba + 20.) * 2. * (100. + 50.);
    assert!((vp - plano).abs() < 2e-3 * plano, "volume planificado {vp} contra {plano}");
    let (lo, hi) = caixa_de(&d);
    let ext = 20. + ba;
    assert!(perto3(lo, [-ext, -ext, 0.], 0.05) && perto3(hi, [100. + ext, 50. + ext, 2.], 0.05), "caixa planificada {lo:?} {hi:?}");
}

/// Bloco 100 x 50 x 30 com casca de 2 mm e a face de cima aberta, como se faz na bancada Sólidos.
fn caixa_aberta() -> Document {
    let mut d = Document::new("caixa");
    d.sheet = Some(SheetParams::default());
    let mut data = SketchData::default();
    let mut n = 7_800_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [0., 0.], [100., 50.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("30")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[50., 25., 30.]], thickness: s("2"), outward: false, face_keys: vec![], closed: false, regras: vec![] }));
    d
}

/// Converter a caixa de cantos vivos: as quatro arestas do fundo viram dobras de raio 2, os quatro
/// cantos ganham rasgo e alívio, a caixa externa não muda e a planificada é uma cruz plana.
#[test]
fn chapa_converte_caixa_aberta_e_planifica() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = caixa_aberta();
    // a face fixa: o fundo, clicado por fora
    let id = d.add_feature(FeatureKind::SmConvert(SmConvert { face: [50., 25., 0.], thickness: String::new(), open_faces: vec![], solido: false }));
    let r = recompute(&d);
    match &r.status[&id] {
        Status::Warning(m) => assert!(m.contains("4 aresta(s) viva(s)") && m.contains("4 canto(s)") && m.contains("4 dobra(s) no mapa"), "{m}"),
        o => panic!("esperava aviso, veio {o:?}"),
    }
    assert_eq!(r.bend_lines.len(), 4);
    let (lo, hi) = caixa_de(&d);
    assert!(perto3(lo, [0., 0., 0.], 0.05) && perto3(hi, [100., 50., 30.], 0.05), "a caixa externa não muda: {lo:?} {hi:?}");
    let v = r.bodies[0].mass.volume;
    assert!(v < 26_352. && v > 0.9 * 26_352., "dobrar tira um pouco de material dos cantos: {v}");
    // planificada: espessura 2 e o comprimento desenvolvido das paredes nos dois sentidos
    d.sheet.as_mut().unwrap().flat = true;
    let r = recompute(&d);
    assert!(valido(r.status.get(&id)), "{:?}", r.status.get(&id));
    let (lo, hi) = caixa_de(&d);
    let ba = chapa::compensacao(2., 2., 90., 0.44).ba;
    // fundo plano 92 x 42 (sem a zona das dobras), dobra desenvolvida e parede reta de 26
    let (cx, cy) = (92. + 2. * (ba + 26.), 42. + 2. * (ba + 26.));
    assert!((hi[2] - lo[2] - 2.).abs() < 0.05, "chapa plana: {lo:?} {hi:?}");
    assert!((hi[0] - lo[0] - cx).abs() < 0.2 && (hi[1] - lo[1] - cy).abs() < 0.2, "{lo:?} {hi:?} contra {cx} x {cy}");
}

/// Aba e bainha abertas sem aresta não viram erro nem pegam a aresta mais perto da origem.
#[test]
fn aba_sem_aresta_espera_a_selecao() {
    let z = [0.; 3];
    assert!(aresta_a_escolher(&aba_vazia()));
    assert!(aresta_a_escolher(&FeatureKind::SmHem(SmHem { edge: z, length: s("8"), gap: s("0"), flip: false, kind: 0, edge_key: None })));
    if let FeatureKind::SmFlange(mut f) = aba_vazia() {
        f.edge = [100., 25., 2.];
        assert!(!aresta_a_escolher(&FeatureKind::SmFlange(f)));
    }
}
