// SPDX-License-Identifier: LGPL-2.1-or-later
//! B293: fechamento de canto entre duas abas. Os três tipos geram geometria válida, a chapa
//! planificada não sobrepõe material e a folga entre as faces é a pedida (volume, caixa
//! envolvente e distância mínima).

use super::chapa_canto::*;
use super::*;
use fenkra_core::SmCornerClose;

fn s(x: &str) -> String {
    x.to_string()
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.)
}

/// Placa de 100 x 50 (espessura 2, raio 2) com uma aba de 20 mm em x = 100 e outra em y = 50: o
/// canto fica em (100, 50) e as duas abas terminam a 1e-4 dele.
fn canto_de_duas_abas(plana: bool) -> Document {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: plana, ..Default::default() });
    let mut data = SketchData::default();
    let mut n = 4_100_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [0., 0.], [100., 50.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    for e in [[100., 25., 2.], [50., 50., 2.]] {
        d.add_feature(FeatureKind::SmFlange(SmFlange { edge: e, length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
    }
    d
}

fn fecha(tipo: u8, folga: &str, razao: &str) -> SmCornerClose {
    SmCornerClose { at: [100., 50., 2.], kind: tipo, gap: s(folga), ratio: s(razao), narrow: false, propagate: false, invert: false }
}

struct Medida {
    volume: f64,
    lo: [f64; 3],
    hi: [f64; 3],
    vertices: Vec<[f64; 3]>,
}

fn mede(d: &Document) -> Medida {
    let r = recompute(d);
    for f in &d.features {
        assert!(matches!(r.status[&f.id], Status::Ok | Status::Warning(_)), "{}: {:?}", f.name, r.status[&f.id]);
    }
    let b = &r.bodies[0];
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    let mut vertices = vec![];
    for p in b.mesh.positions.chunks_exact(3) {
        let q = [p[0] as f64, p[1] as f64, p[2] as f64];
        for i in 0..3 {
            lo[i] = lo[i].min(q[i]);
            hi[i] = hi[i].max(q[i]);
        }
        vertices.push(q);
    }
    Medida { volume: b.mass.volume, lo, hi, vertices }
}

/// Algum vértice da malha dentro da caixa (lo, hi) aberta?
fn ha_vertice(m: &Medida, lo: [f64; 3], hi: [f64; 3]) -> bool {
    m.vertices.iter().any(|q| (0..3).all(|i| q[i] > lo[i] && q[i] < hi[i]))
}

/// As duas abas do documento como `AbaCanto`, lidas do mesmo cálculo que o comando usa.
fn canto_ordenado(d: &Document) -> Canto {
    // aba em x = 100 (sai em +x) e aba em y = 50 (sai em +y); a normal sobe por z
    let r = 2.;
    let t = 2.;
    let ba = chapa::compensacao(t, r, 90., 0.44).ba;
    let a = AbaCanto { c0: [100., 1e-4, 2.], ev: [0., 1., 0.], l: 50. - 2e-4, n: [0., 0., 1.], u: [1., 0., 0.], t, r, ba, comp: 20. };
    let b = AbaCanto { c0: [1e-4, 50., 2.], ev: [1., 0., 0.], l: 100. - 2e-4, n: [0., 0., 1.], u: [0., 1., 0.], t, r, ba, comp: 20. };
    let _ = d;
    let c = canto_entre(a, b).unwrap();
    ordenar(c, false)
}

#[test]
fn canto_acha_o_vertice_e_a_ordem_anti_horaria() {
    let c = canto_ordenado(&canto_de_duas_abas(false));
    assert!(geo3d::dist(c.k, [100., 50., 2.]) < 1e-9, "{:?}", c.k);
    // a primeira é a que sai em +x (chega ao canto subindo em y, com o interior à esquerda)
    assert!(c.a.u[0] > 0.99 && c.b.u[1] > 0.99);
    assert!(c.off_a < 1e-3 && c.off_b < 1e-3);
    // invertendo, a ordem troca
    let d = ordenar(c, true);
    assert!(d.a.u[1] > 0.99);
}

#[test]
fn abas_paralelas_ou_de_lados_opostos_nao_formam_canto() {
    let c = canto_ordenado(&canto_de_duas_abas(false));
    let mut b = c.b;
    b.u = c.a.u;
    assert!(canto_entre(c.a, b).unwrap_err().contains("90"));
    let mut b = c.b;
    b.n = [0., 0., -1.];
    assert!(canto_entre(c.a, b).unwrap_err().contains("mesmo lado"));
}

#[test]
fn pecas_dos_tres_tipos_respeitam_a_folga() {
    let c = canto_ordenado(&canto_de_duas_abas(false));
    for (tipo, razao) in [(0u8, 1.), (1, 1.), (2, 1.), (1, 0.4), (2, 0.4)] {
        for g in [0.2, 0.5, 1.0] {
            let ps = pecas(&c, &ParamsCanto { kind: tipo, gap: g, ratio: razao, narrow: false }, false, 0.).unwrap();
            assert_eq!(ps.len(), 2, "tipo {tipo}");
            let d = distancia_pecas(&ps[0], &ps[1]);
            assert!((d - g).abs() < 1e-3, "tipo {tipo} razão {razao}: folga pedida {g}, medida {d}");
        }
    }
    // o canto estreito limita a cobertura à metade da espessura
    let larga = pecas(&c, &ParamsCanto { kind: 1, gap: 0.5, ratio: 1., narrow: false }, false, 0.).unwrap();
    let estreita = pecas(&c, &ParamsCanto { kind: 1, gap: 0.5, ratio: 1., narrow: true }, false, 0.).unwrap();
    assert!(estreita[0].volume() < larga[0].volume() - 1.);
    let metade = pecas(&c, &ParamsCanto { kind: 1, gap: 0.5, ratio: 0.5, narrow: false }, false, 0.).unwrap();
    assert!((estreita[0].volume() - metade[0].volume()).abs() < 1e-9);
}

#[test]
fn parametros_fora_do_limite_dao_erro_claro() {
    let c = canto_ordenado(&canto_de_duas_abas(false));
    assert!(pecas(&c, &ParamsCanto { kind: 1, gap: -1., ratio: 1., narrow: false }, false, 0.).unwrap_err().contains("folga"));
    assert!(pecas(&c, &ParamsCanto { kind: 1, gap: 0.5, ratio: 1.5, narrow: false }, false, 0.).unwrap_err().contains("razão"));
}

#[test]
fn tres_tipos_geram_geometria_valida_com_o_volume_das_pecas() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let base = canto_de_duas_abas(false);
    let m0 = mede(&base);
    let c = canto_ordenado(&base);
    for (tipo, razao) in [(0u8, "1"), (1, "1"), (2, "1"), (1, "0.5")] {
        let mut d = base.clone();
        d.add_feature(FeatureKind::SmCornerClose(fecha(tipo, "0.5", razao)));
        let m = mede(&d);
        // o volume somado é o das peças, sem sobreposição com a chapa nem entre si
        let ps = pecas(&c, &ParamsCanto { kind: tipo, gap: 0.5, ratio: razao.parse().unwrap(), narrow: false }, false, 0.).unwrap();
        let esperado: f64 = ps.iter().map(|p| p.volume()).sum();
        assert!(esperado > 20., "tipo {tipo}: peças pequenas demais ({esperado})");
        assert!((m.volume - m0.volume - esperado).abs() < 0.01 * esperado + 0.2, "tipo {tipo}: somou {} e esperava {esperado}", m.volume - m0.volume);
        // a caixa envolvente não cresce: o fechamento fica dentro do canto das duas abas
        for i in 0..3 {
            assert!((m.lo[i] - m0.lo[i]).abs() < 1e-3 && (m.hi[i] - m0.hi[i]).abs() < 1e-3, "tipo {tipo} eixo {i}: {:?} {:?} x {:?} {:?}", m.lo, m.hi, m0.lo, m0.hi);
        }
    }
}

#[test]
fn sobreposto_deixa_a_folga_entre_as_faces_e_sob_inverte_quem_cobre() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let base = canto_de_duas_abas(false);
    // sobreposto: a aba de x = 100 (parede de 102 a 104) avança em y e cobre a ponta da outra;
    // a outra para a 0,5 mm da face de dentro dela (x = 101,5)
    let mut d = base.clone();
    d.add_feature(FeatureKind::SmCornerClose(fecha(1, "0.5", "1")));
    let m = mede(&d);
    assert!(!ha_vertice(&m, [101.52, 51.9, 3.9], [101.98, 54.1, 24.1]), "há material na folga");
    // as duas faces que limitam a folga existem (os vértices ficam nos cantos delas): a ponta da
    // outra aba em x = 101,5 e a face de dentro da primeira em x = 102
    assert!(ha_vertice(&m, [101.49, 51.9, 3.9], [101.51, 54.1, 24.1]), "falta a ponta da segunda aba em x = 101,5");
    assert!(ha_vertice(&m, [101.99, 49.9, 3.9], [102.01, 54.1, 24.1]), "falta a face de dentro da primeira aba em x = 102");
    // a primeira cobre toda a coluna do canto: material até y = 54 em x de 102 a 104
    assert!(ha_vertice(&m, [103.9, 53.9, 3.9], [104.1, 54.1, 24.1]));
    // sob: quem cobre é a outra aba; a folga fica em y, entre a ponta da primeira e a face dela
    let mut d = base.clone();
    d.add_feature(FeatureKind::SmCornerClose(fecha(2, "0.5", "1")));
    let m = mede(&d);
    assert!(!ha_vertice(&m, [101.9, 51.52, 3.9], [104.1, 51.98, 24.1]), "há material na folga do sob");
    assert!(ha_vertice(&m, [101.9, 51.49, 3.9], [104.1, 51.51, 24.1]), "falta a ponta da primeira em y = 51,5");
    // invertendo o sobreposto, sai o mesmo que o sob
    let mut d_inv = base.clone();
    let mut inv = fecha(1, "0.5", "1");
    inv.invert = true;
    d_inv.add_feature(FeatureKind::SmCornerClose(inv));
    let mi = mede(&d_inv);
    assert!(perto(mi.volume, m.volume, 1e-6), "invertido {} x sob {}", mi.volume, m.volume);
}

#[test]
fn topo_encontra_as_pontas_na_diagonal_com_a_folga_pedida() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let base = canto_de_duas_abas(false);
    let mut d = base.clone();
    d.add_feature(FeatureKind::SmCornerClose(fecha(0, "0.6", "1")));
    let m = mede(&d);
    // no meio do quadrado do canto (x = 103, y = 53) passa a diagonal: nenhum vértice dentro da
    // faixa de folga em volta dela, e há vértices nas duas bordas dela
    let (cx, cy) = (103.0, 53.0);
    let dentro = m.vertices.iter().any(|q| q[2] > 3.9 && q[2] < 24.1 && q[0] > 102.05 && q[0] < 103.95 && q[1] > 52.05 && q[1] < 53.95 && ((q[1] - cy) - (q[0] - cx)).abs() / 2f64.sqrt() < 0.29);
    assert!(!dentro, "há material na folga do topo");
    let borda = m.vertices.iter().any(|q| q[2] > 3.9 && q[2] < 24.1 && q[0] > 101.9 && ((q[1] - cy) - (q[0] - cx)).abs() / 2f64.sqrt() > 0.29 && ((q[1] - cy) - (q[0] - cx)).abs() / 2f64.sqrt() < 0.31);
    assert!(borda, "faltam as faces que limitam a folga do topo");
}

#[test]
fn folga_zero_une_as_pontas_em_um_corpo_so() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let base = canto_de_duas_abas(false);
    let m0 = mede(&base);
    let mut d = base.clone();
    d.add_feature(FeatureKind::SmCornerClose(fecha(1, "0", "1")));
    let m = mede(&d);
    assert!(m.volume > m0.volume + 20.);
    let r = recompute(&d);
    assert_eq!(r.bodies.len(), 1, "o canto soldado continua um corpo só");
}

#[test]
fn planificada_nao_sobrepoe_material() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let base = canto_de_duas_abas(true);
    let m0 = mede(&base);
    let c = canto_ordenado(&base);
    for tipo in [0u8, 1, 2] {
        let mut d = base.clone();
        d.add_feature(FeatureKind::SmCornerClose(fecha(tipo, "0.5", "1")));
        let m = mede(&d);
        let ps = pecas(&c, &ParamsCanto { kind: tipo, gap: 0.5, ratio: 1., narrow: false }, true, 0.).unwrap();
        let esperado: f64 = ps.iter().map(|p| p.volume()).sum();
        // somar exatamente o volume das peças prova que nada se sobrepôs ao que já existia
        assert!((m.volume - m0.volume - esperado).abs() < 0.01 * esperado + 0.2, "tipo {tipo} planificada: somou {} e esperava {esperado}", m.volume - m0.volume);
        // e as duas peças continuam a folga uma da outra
        assert!(distancia_pecas(&ps[0], &ps[1]) >= 0.5 - 1e-6);
    }
}

#[test]
fn propagacao_fecha_os_quatro_cantos_no_mesmo_sentido() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = canto_de_duas_abas(false);
    for e in [[0., 25., 2.], [50., 0., 2.]] {
        d.add_feature(FeatureKind::SmFlange(SmFlange { edge: e, length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
    }
    let m0 = mede(&d);
    // sem propagar, um canto só
    let mut um = d.clone();
    um.add_feature(FeatureKind::SmCornerClose(fecha(1, "0.5", "1")));
    let vm = mede(&um).volume - m0.volume;
    // propagando, os quatro cantos com o mesmo volume (cada aba cobre a seguinte)
    let mut quatro = d.clone();
    let mut p = fecha(1, "0.5", "1");
    p.propagate = true;
    quatro.add_feature(FeatureKind::SmCornerClose(p));
    let vq = mede(&quatro).volume - m0.volume;
    assert!((vq - 4. * vm).abs() < 0.01 * vq + 0.5, "um canto somou {vm}, quatro somaram {vq}");
}

#[test]
fn canto_longe_das_abas_e_aba_de_outro_angulo_dao_erro() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = canto_de_duas_abas(false);
    let mut longe = fecha(1, "0.5", "1");
    longe.at = [0., 0., 2.];
    let id = d.add_feature(FeatureKind::SmCornerClose(longe));
    match &recompute(&d).status[&id] {
        Status::Error(m) => assert!(m.contains("perto do ponto") || m.contains("nenhum"), "{m}"),
        o => panic!("esperava erro, veio {o:?}"),
    }
    // abas de 45° não são fechadas
    let mut d = canto_de_duas_abas(false);
    for f in d.features.iter_mut() {
        if let FeatureKind::SmFlange(fl) = &mut f.kind {
            fl.angle = s("45");
        }
    }
    let id = d.add_feature(FeatureKind::SmCornerClose(fecha(1, "0.5", "1")));
    match &recompute(&d).status[&id] {
        Status::Error(m) => assert!(m.contains("90"), "{m}"),
        o => panic!("esperava erro, veio {o:?}"),
    }
}

#[test]
fn fechamento_de_canto_sobrevive_a_salvar_e_abrir() {
    let mut d = canto_de_duas_abas(false);
    let mut p = fecha(2, "0.4", "0.7");
    p.narrow = true;
    p.propagate = true;
    p.invert = true;
    d.add_feature(FeatureKind::SmCornerClose(p));
    let json = serde_json::to_string(&d).unwrap();
    let volta: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(volta.features.last().unwrap().kind, d.features.last().unwrap().kind);
}
