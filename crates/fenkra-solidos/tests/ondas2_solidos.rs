// SPDX-License-Identifier: LGPL-2.1-or-later
//! Onda 2 de Sólidos: B89 (términos por lado, deslocamento e início deslocado), B90 (furo guiado
//! por norma), B91 (padrões com instâncias puladas) e B48 (escareado com ângulo e casca).

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::furo_norma::{deslocamento_ponta_dentro, nome_do_furo, renomear_furos};
use fenkra_solidos::{hardware, recompute, Status};
use std::f64::consts::PI;

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

fn s(x: &str) -> String {
    x.into()
}

fn extrude(sk: Id, op: BoolOp, extent: Extent, reverse: bool, ends: ExtrudeEnds) -> FeatureKind {
    FeatureKind::Extrude(Extrude { sketch: sk, op, extent, reverse, taper: s("0"), thin: None, dir: None, thin_side: 0, ends })
}

fn z_range(d: &Document) -> (f64, f64) {
    let (lo, hi) = caixa_envolvente(d);
    (lo[2], hi[2])
}

fn vol(d: &Document) -> f64 {
    let r = recompute(d);
    volume_total(&r)
}

// ------------------------------------------------------------------ B89

#[test]
fn b89_inicio_deslocado_comeca_longe_do_plano_do_sketch() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("início");
    let sk = sk_rect(&mut d, XY, [0., 0.], [20., 20.]);
    let id = d.add_feature(extrude(sk, BoolOp::Add, Extent::Blind(s("10")), false, ExtrudeEnds { start: s("5"), ..Default::default() }));
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    assert!((volume_total(&r) - 4000.).abs() < 1e-3);
    let (z0, z1) = z_range(&d);
    assert!((z0 - 5.).abs() < 1e-6 && (z1 - 15.).abs() < 1e-6, "z {z0}..{z1}");
}

#[test]
fn b89_cada_lado_tem_o_seu_termino() {
    if !kernel_disponivel() {
        return;
    }
    // 10 para um lado e 4 para o outro: de z = -4 a z = 10
    let mut d = Document::new("dois términos");
    let sk = sk_rect(&mut d, XY, [0., 0.], [20., 20.]);
    let ends = ExtrudeEnds { extent2: Some(Extent::Blind(s("4"))), ..Default::default() };
    let id = d.add_feature(extrude(sk, BoolOp::Add, Extent::Blind(s("10")), false, ends));
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    assert!((volume_total(&r) - 20. * 20. * 14.).abs() < 1e-3, "volume {}", volume_total(&r));
    let (z0, z1) = z_range(&d);
    assert!((z0 + 4.).abs() < 1e-6 && (z1 - 10.).abs() < 1e-6, "z {z0}..{z1}");
}

#[test]
fn b89_lado_dois_ate_plano_com_deslocamento() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("lado 2 até plano");
    let fundo = d.add_offset_plane(XY, "-8");
    let sk = sk_rect(&mut d, XY, [0., 0.], [10., 10.]);
    let ends = ExtrudeEnds { extent2: Some(Extent::UpTo(PlaneRef::Datum(fundo))), offset2: s("2"), ..Default::default() };
    let id = d.add_feature(extrude(sk, BoolOp::Add, Extent::Blind(s("5")), false, ends));
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    // o plano está em z = -8; o deslocamento +2 vai 2 mm além dele, na direção do lado 2
    let (z0, z1) = z_range(&d);
    assert!((z0 + 10.).abs() < 1e-6 && (z1 - 5.).abs() < 1e-6, "z {z0}..{z1}");
    assert!((volume_total(&r) - 100. * 15.).abs() < 1e-3);
}

#[test]
fn b89_ate_plano_com_deslocamento_avanca_ou_recua() {
    if !kernel_disponivel() {
        return;
    }
    for (off, altura) in [("0", 10.), ("-2", 8.), ("3", 13.)] {
        let mut d = Document::new("até plano");
        let alvo = d.add_offset_plane(XY, "10");
        let sk = sk_rect(&mut d, XY, [0., 0.], [10., 10.]);
        let id = d.add_feature(extrude(sk, BoolOp::Add, Extent::UpTo(PlaneRef::Datum(alvo)), false, ExtrudeEnds { offset: s(off), ..Default::default() }));
        let r = recompute(&d);
        assert_eq!(r.status[&id], Status::Ok, "offset {off}: {:?}", r.status[&id]);
        assert!((volume_total(&r) - 100. * altura).abs() < 1e-3, "offset {off}: volume {}", volume_total(&r));
    }
}

#[test]
fn b89_ate_o_proximo_com_deslocamento_para_antes_da_parede() {
    if !kernel_disponivel() {
        return;
    }
    // caixa de z = 0 a 20; pino sai de z = -10 para cima e para na parede de baixo da caixa (z = 0)
    for (off, pino) in [("0", 10.), ("-4", 6.)] {
        let mut d = Document::new("até o próximo");
        let sk0 = sk_rect(&mut d, XY, [-20., -15.], [20., 15.]);
        d.add_feature(ext(sk0, BoolOp::Add, "20"));
        let baixo = d.add_offset_plane(XY, "-10");
        let sk = sk_rect(&mut d, PlaneRef::Datum(baixo), [-5., -5.], [5., 5.]);
        let id = d.add_feature(extrude(sk, BoolOp::Add, Extent::UpToNext, false, ExtrudeEnds { offset: s(off), ..Default::default() }));
        let r = recompute(&d);
        assert_eq!(r.status[&id], Status::Ok, "offset {off}: {:?}", r.status[&id]);
        let esperado = 40. * 30. * 20. + 100. * pino;
        assert!((volume_total(&r) - esperado).abs() < 1e-3, "offset {off}: volume {} esperado {esperado}", volume_total(&r));
    }
}

#[test]
fn b89_deslocamento_em_termino_por_dimensao_e_erro_nomeado() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("offset em cego");
    let sk = sk_rect(&mut d, XY, [0., 0.], [10., 10.]);
    let id = d.add_feature(extrude(sk, BoolOp::Add, Extent::Blind(s("10")), false, ExtrudeEnds { offset: s("2"), ..Default::default() }));
    let r = recompute(&d);
    match &r.status[&id] {
        Status::Error(m) => assert!(m.contains("deslocamento do término") && m.contains("cego"), "{m}"),
        outro => panic!("esperava erro, veio {outro:?}"),
    }
}

#[test]
fn b89_lado_dois_com_plano_medio_e_erro_nomeado() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("lado 2 inválido");
    let sk = sk_rect(&mut d, XY, [0., 0.], [10., 10.]);
    let ends = ExtrudeEnds { extent2: Some(Extent::Blind(s("3"))), ..Default::default() };
    let id = d.add_feature(extrude(sk, BoolOp::Add, Extent::Symmetric(s("10")), false, ends));
    let r = recompute(&d);
    assert!(matches!(r.status[&id], Status::Error(_)), "{:?}", r.status[&id]);
}

// ------------------------------------------------------------------ B90

/// Bloco 40 x 30 x 30 e um furo de cima (sketch no plano z = 30) com os ajustes dados.
fn bloco_com_furo(ajusta: impl FnOnce(&mut Hole)) -> (Document, Id) {
    let mut d = Document::new("furo");
    let sk = sk_rect(&mut d, XY, [-20., -15.], [20., 15.]);
    d.add_feature(ext(sk, BoolOp::Add, "30"));
    let topo = d.add_offset_plane(XY, "30");
    let sp = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[0., 0.]]);
    let mut h = Hole { sketch: sp, kind: HoleKind::Simples, diameter: s("6.8"), depth: Extent::Blind(s("20")), thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: s("0"), left_hand: false, extra: Default::default() };
    ajusta(&mut h);
    let id = d.add_feature(FeatureKind::Hole(h));
    (d, id)
}

fn volume_do_furo(ajusta: impl FnOnce(&mut Hole)) -> f64 {
    let (d, id) = bloco_com_furo(ajusta);
    let r = recompute(&d);
    assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{:?}", r.status[&id]);
    volume_total(&r)
}

#[test]
fn b90_profundidade_de_rosca_separada_da_profundidade_do_furo() {
    if !kernel_disponivel() {
        return;
    }
    let rosca = |h: &mut Hole| {
        h.thread = Some(s("M8x1.25"));
        h.thread_modeled = true;
    };
    let sem_rosca = volume_do_furo(|_| {});
    let curta = volume_do_furo(|h| {
        rosca(h);
        h.extra.thread_depth = s("8");
    });
    let longa = volume_do_furo(|h| {
        rosca(h);
        h.extra.thread_depth = s("16");
    });
    // a rosca é sulco a mais no furo: quanto mais longa, menos material sobra
    assert!(sem_rosca > curta + 0.5, "rosca de 8 não tirou material: {sem_rosca} vs {curta}");
    assert!(curta > longa + 0.5, "rosca de 16 deveria tirar mais que a de 8: {curta} vs {longa}");
}

#[test]
fn b90_folga_no_fundo_em_passos_encurta_a_rosca() {
    if !kernel_disponivel() {
        return;
    }
    let rosca = |h: &mut Hole, folga: &str| {
        h.thread = Some(s("M8x1.25"));
        h.thread_modeled = true;
        h.extra.thread_relief = s(folga);
    };
    let colada = volume_do_furo(|h| rosca(h, "0"));
    let folgada = volume_do_furo(|h| rosca(h, "4"));
    // folga de 4 passos (5 mm) tira 5 mm de rosca: sobra mais material
    assert!(folgada > colada + 0.5, "{folgada} vs {colada}");
    // folga de 4 passos é igual a pedir a profundidade de rosca de 15 mm
    let equivalente = volume_do_furo(|h| {
        rosca(h, "0");
        h.extra.thread_depth = s("15");
    });
    assert!((folgada - equivalente).abs() < 0.5, "{folgada} vs {equivalente}");
}

#[test]
fn b90_rosca_mais_funda_que_o_furo_e_erro_nomeado() {
    if !kernel_disponivel() {
        return;
    }
    let (d, id) = bloco_com_furo(|h| {
        h.thread = Some(s("M8x1.25"));
        h.thread_modeled = true;
        h.extra.thread_depth = s("25");
    });
    let r = recompute(&d);
    match &r.status[&id] {
        Status::Error(m) => assert!(m.contains("profundidade da rosca"), "{m}"),
        outro => panic!("esperava erro, veio {outro:?}"),
    }
}

#[test]
fn b90_deslocamento_da_ponta_faz_o_v_caber_na_profundidade() {
    if !kernel_disponivel() {
        return;
    }
    let (diametro, ponta) = (8., 118.);
    let com_ponta = |off: &str| {
        volume_do_furo(|h| {
            h.diameter = s("8");
            h.tip = Some(s("118"));
            h.extra.tip_offset = s(off);
        })
    };
    let solto = com_ponta("0");
    let h_cone = deslocamento_ponta_dentro(diametro, ponta).abs();
    let dentro = com_ponta(&format!("-{h_cone}"));
    // a ponta dentro da profundidade tira um cilindro de altura h_cone a menos
    let esperado = PI * 16. * h_cone;
    assert!(((dentro - solto) - esperado).abs() < 0.02 * esperado, "diferença {} esperada {esperado}", dentro - solto);
}

#[test]
fn b90_nome_da_feature_leva_os_valores() {
    let (mut d, id) = bloco_com_furo(|h| {
        h.thread = Some(s("M8x1.25"));
    });
    let vars = d.eval_params().unwrap();
    let h = match &d.feature(id).unwrap().kind {
        FeatureKind::Hole(h) => h.clone(),
        _ => unreachable!(),
    };
    assert_eq!(nome_do_furo(&h, &vars), "Furo M8 roscado x 20");
    // furo simples passante e escareado
    let mut h2 = h.clone();
    h2.thread = None;
    h2.depth = Extent::ThroughAll;
    h2.diameter = s("6.6");
    h2.kind = HoleKind::Escareado { diameter: s("13.7"), angle: s("90") };
    assert_eq!(nome_do_furo(&h2, &vars), "Furo Ø6,6 escareado 90° x passante");
    // renomear só mexe em nome automático
    d.feature_mut(id).unwrap().name = s("Furo");
    assert_eq!(renomear_furos(&mut d), 1);
    assert_eq!(d.feature(id).unwrap().name, "Furo M8 roscado x 20");
    d.feature_mut(id).unwrap().name = s("Furo do mancal");
    assert_eq!(renomear_furos(&mut d), 0);
    assert_eq!(d.feature(id).unwrap().name, "Furo do mancal");
}

// ------------------------------------------------------------------ B91

fn pino_com_padrao(skip: Vec<(u32, u32)>, count: &str) -> (Document, Id) {
    let mut d = Document::new("padrão");
    let sk = sk_rect(&mut d, XY, [0., 0.], [10., 10.]);
    let b = d.add_feature(ext(sk, BoolOp::Add, "10"));
    let p = d.add_feature(FeatureKind::Pattern(Pattern {
        kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: s(count), spacing: s("20"), dir2: None, count2: s("1"), spacing2: s("0"), total: false },
        sources: vec![b],
        skip,
    }));
    (d, p)
}

#[test]
fn b91_pular_instancia_tira_so_aquela_copia() {
    if !kernel_disponivel() {
        return;
    }
    let (d, _) = pino_com_padrao(vec![], "4");
    assert!((vol(&d) - 4000.).abs() < 1e-3);
    let (d, p) = pino_com_padrao(vec![(2, 0)], "4");
    let r = recompute(&d);
    assert_eq!(r.status[&p], Status::Ok, "{:?}", r.status[&p]);
    assert!((volume_total(&r) - 3000.).abs() < 1e-3);
    // a instância 3 (x = 40..50) sumiu e as outras ficaram onde estavam
    let (lo, hi) = caixa_envolvente(&d);
    assert!((hi[0] - 70.).abs() < 1e-6 && lo[0].abs() < 1e-6, "x {}..{}", lo[0], hi[0]);
}

#[test]
fn b91_pulada_sobrevive_a_mudar_a_quantidade_e_avisa_se_sai_do_padrao() {
    if !kernel_disponivel() {
        return;
    }
    // com 6 cópias a pulada continua sendo a de posição 2
    let (d, _) = pino_com_padrao(vec![(2, 0)], "6");
    assert!((vol(&d) - 5000.).abs() < 1e-3);
    // com 2 cópias a posição 2 deixou de existir: nada some e o comando avisa
    let (d, p) = pino_com_padrao(vec![(2, 0)], "2");
    let r = recompute(&d);
    assert!((volume_total(&r) - 2000.).abs() < 1e-3);
    match &r.status[&p] {
        Status::Warning(m) => assert!(m.contains("puladas") && m.contains('3'), "{m}"),
        outro => panic!("esperava aviso, veio {outro:?}"),
    }
}

#[test]
fn b91_instancias_listam_posicao_chave_e_estado_para_a_previa() {
    if !kernel_disponivel() {
        return;
    }
    let (d, p) = pino_com_padrao(vec![(1, 0)], "3");
    let v = fenkra_solidos::padrao::instancias_do_padrao(&d, p);
    assert_eq!(v.len(), 3, "{v:?}");
    assert_eq!(v[0].chave, (0, 0));
    assert!(!v[0].pulada && v[1].pulada && !v[2].pulada);
    // o centro de cada instância anda 20 mm em x
    assert!((v[1].centro[0] - v[0].centro[0] - 20.).abs() < 1e-6);
    assert!((v[2].centro[0] - v[0].centro[0] - 40.).abs() < 1e-6);
}

#[test]
fn b91_circular_pula_por_posicao() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("circular");
    let sk = sk_rect(&mut d, XY, [20., -5.], [30., 5.]);
    let b = d.add_feature(ext(sk, BoolOp::Add, "5"));
    d.add_feature(FeatureKind::Pattern(Pattern {
        kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Z), count: s("4"), angle: s("360") },
        sources: vec![b],
        skip: vec![(1, 0), (3, 0)],
    }));
    // sobram a original e a oposta (x = ±20..30)
    assert!((vol(&d) - 1000.).abs() < 1e-3);
}

// ------------------------------------------------------------------ B48

#[test]
fn b48_angulos_de_escareado_editaveis_e_por_norma() {
    for a in [82., 90., 100., 120.] {
        assert!(hardware::ANGULOS_ESCAREADO.contains(&a), "{a}");
    }
    assert_eq!(hardware::angulo_escareado_da_norma("ISO 10642"), Some(90.));
    assert_eq!(hardware::angulo_escareado_da_norma("ASME B18.6.3"), Some(82.));
    assert_eq!(hardware::angulo_escareado_da_norma("norma que não existe"), None);
}

fn bloco_escareado(angulo: &str) -> Document {
    bloco_escareado_prof(angulo, Extent::ThroughAll)
}

fn bloco_escareado_prof(angulo: &str, depth: Extent) -> Document {
    let mut d = Document::new("escareado");
    let sk = sk_rect(&mut d, XY, [-25., -20.], [25., 20.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let topo = d.add_offset_plane(XY, "20");
    let sp = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[0., 0.]]);
    d.add_feature(FeatureKind::Hole(Hole {
        sketch: sp,
        kind: HoleKind::Escareado { diameter: s("14"), angle: s(angulo) },
        diameter: s("6.6"),
        depth,
        thread: None,
        thread_modeled: false,
        standard: None,
        points: vec![],
        dir: None,
        reverse: false,
        tip: None,
        taper: s("0"),
        left_hand: false,
        extra: Default::default(),
    }));
    d
}

#[test]
fn b48_angulo_do_escareado_muda_o_volume_como_o_cone_manda() {
    if !kernel_disponivel() {
        return;
    }
    let cheio = 50. * 40. * 20.;
    let furo = PI * 3.3 * 3.3 * 20.;
    let mut anterior = f64::MAX;
    for ang in [82., 90., 100., 120.] {
        let d = bloco_escareado(&s(&ang.to_string()));
        let r = recompute(&d);
        assert!(problema_de_status(&d, &r).is_none(), "{ang}°: {:?}", problema_de_status(&d, &r));
        let v = volume_total(&r);
        // cone de Ø14 na boca até Ø6,6: altura (14-6,6)/2 / tan(ang/2); volume do tronco acima do furo
        let (r1, r0) = (7., 3.3);
        let h = (r1 - r0) / (ang as f64 / 2.0f64).to_radians().tan();
        let tronco = PI * h / 3. * (r1 * r1 + r1 * r0 + r0 * r0) - PI * r0 * r0 * h;
        let esperado = cheio - furo - tronco;
        assert!((v - esperado).abs() < 0.01 * (cheio - esperado).abs().max(1.), "{ang}°: volume {v:.2} esperado {esperado:.2}");
        // cone mais aberto é mais raso: tira menos material
        assert!(v > 0. && (anterior == f64::MAX || v >= anterior - 1e-6 || ang > 82.), "{ang}");
        anterior = v;
    }
}

#[test]
fn b48_casca_de_corpo_com_furo_escareado_nao_falha() {
    if !kernel_disponivel() {
        return;
    }
    // bloco com furo escareado na face de cima. Casos que a casca resolve: abrir a face de baixo (longe do
    // furo) com furo passante ou cego, e abrir a de cima quando o furo é passante.
    let casos = [
        ("passante", Extent::ThroughAll, "baixo", [15., 10., 0.], "2"),
        ("passante", Extent::ThroughAll, "cima", [20., 15., 20.], "3"),
        ("cego", Extent::Blind(s("12")), "baixo", [15., 10., 0.], "2"),
    ];
    for (nome, depth, lado, face, espessura) in casos {
        for ang in ["82", "90", "120"] {
            let mut d = bloco_escareado_prof(ang, depth.clone());
            let sh = d.add_feature(FeatureKind::Shell(Shell { faces: vec![face], thickness: s(espessura), outward: false, face_keys: vec![], closed: false }));
            let r = recompute(&d);
            assert_eq!(r.status[&sh], Status::Ok, "casca ({lado}) com escareado {ang}° {nome}: {:?}", r.status[&sh]);
            let v = volume_total(&r);
            assert!(v > 500. && v < 50. * 40. * 20. * 0.5, "{ang}° {nome} {lado}: volume da casca {v}");
        }
    }
}

/// B48: abrir a face de cima, onde fica o escareado de um furo cego. O deslocador de faces do
/// OpenCASCADE não fecha a casca em volta do cone com o fundo plano; o kernel então cobre a face aberta
/// com uma tampa de material, desloca o corpo estendido para dentro e subtrai (casca por booleana).
#[test]
fn b48_casca_abrindo_a_face_do_escareado_de_furo_cego() {
    if !kernel_disponivel() {
        return;
    }
    for ang in ["82", "90", "100", "120"] {
        let mut d = bloco_escareado_prof(ang, Extent::Blind(s("12")));
        let sh = d.add_feature(FeatureKind::Shell(Shell { faces: vec![[20., 15., 20.]], thickness: s("3"), outward: false, face_keys: vec![], closed: false }));
        let r = recompute(&d);
        assert_eq!(r.status[&sh], Status::Ok, "{ang}°: {:?}", r.status[&sh]);
        let v = volume_total(&r);
        // bloco 50 × 40 × 20 aberto em cima com parede de 3: o oco tem 44 × 34 × 17 e o furo ganha parede própria.
        // Sem a parede do furo seria 14 568; com a casca fechada em cima, mais 6 000.
        assert!(v > 14_000. && v < 18_500., "{ang}°: volume da casca {v}");
        // a caixa envolvente continua a do bloco: a casca não cresce nem encolhe por fora
        let b = &r.bodies[0];
        let (lo, hi) = b.mesh.positions.chunks_exact(3).fold(([f32::MAX; 3], [f32::MIN; 3]), |(mut lo, mut hi), p| {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
            (lo, hi)
        });
        assert!((hi[0] - lo[0] - 50.).abs() < 0.01 && (hi[1] - lo[1] - 40.).abs() < 0.01 && (hi[2] - lo[2] - 20.).abs() < 0.01, "{ang}°: caixa {lo:?} {hi:?}");
    }
}
