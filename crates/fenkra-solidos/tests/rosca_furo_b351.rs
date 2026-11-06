// SPDX-License-Identifier: LGPL-2.1-or-later
//! B351: rosca em furo. A rosca modelada do furo escareado não removia nada (a booleana devolvia o
//! corpo sem o sulco), o sulco do furo cego passava do fundo, e a prévia refazia a hélice a cada
//! edição. Cada teste monta o documento em código, recalcula e confere status, volume e geometria.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, recompute_cached, Cache, Status};
use std::time::Instant;

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

fn s(x: &str) -> String {
    x.into()
}

/// Bloco 40 x 30 x 30 com um furo roscado M8 de cima (sketch no plano z = 30).
fn bloco(ajusta: impl FnOnce(&mut Hole)) -> (Document, Id) {
    let mut d = Document::new("furo roscado");
    let sk = sk_rect(&mut d, XY, [-20., -15.], [20., 15.]);
    d.add_feature(ext(sk, BoolOp::Add, "30"));
    let topo = d.add_offset_plane(XY, "30");
    let sp = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[0., 0.]]);
    let mut h = Hole {
        sketch: sp,
        kind: HoleKind::Simples,
        diameter: s("6.8"),
        depth: Extent::Blind(s("20")),
        thread: Some(s("M8x1.25")),
        thread_modeled: true,
        standard: None,
        points: vec![],
        dir: None,
        reverse: false,
        tip: None,
        taper: s("0"),
        left_hand: false,
        extra: Default::default(),
    };
    ajusta(&mut h);
    let id = d.add_feature(FeatureKind::Hole(h));
    (d, id)
}

fn escareado(h: &mut Hole) {
    h.kind = HoleKind::Escareado { diameter: s("16"), angle: s("90") };
}

fn rebaixado(h: &mut Hole) {
    h.kind = HoleKind::Rebaixado { diameter: s("14"), depth: s("6") };
}

fn contrafurado(h: &mut Hole) {
    h.kind = HoleKind::Contrafurado { diameter: s("14"), depth: s("4"), angle: s("90") };
}

fn passante(h: &mut Hole) {
    h.depth = Extent::ThroughAll;
}

/// Volume que a rosca modelada tira a mais que a cosmética, no mesmo furo.
fn volume_do_sulco(ajusta: impl Fn(&mut Hole)) -> f64 {
    let (d, id) = bloco(|h| {
        ajusta(h);
        h.thread_modeled = false;
    });
    let r = recompute(&d);
    assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{:?}", r.status[&id]);
    let cosmetica = volume_total(&r);
    let (d, id) = bloco(|h| {
        ajusta(h);
        h.thread_modeled = true;
    });
    let r = recompute(&d);
    assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{:?}", r.status[&id]);
    cosmetica - volume_total(&r)
}

#[test]
fn escareado_cego_ganha_o_sulco_da_rosca() {
    if !kernel_disponivel() {
        return;
    }
    // antes da correção a hélice terminava exatamente no fundo e a booleana devolvia o corpo intacto
    let v = volume_do_sulco(escareado);
    assert!(v > 30., "a rosca modelada do furo escareado não tirou material: {v}");
}

#[test]
fn escareado_passante_ganha_o_sulco_da_rosca() {
    if !kernel_disponivel() {
        return;
    }
    let v = volume_do_sulco(|h| {
        escareado(h);
        passante(h);
    });
    assert!(v > 30., "a rosca modelada do furo escareado passante não tirou material: {v}");
}

#[test]
fn todas_as_bocas_e_profundidades_ganham_sulco() {
    if !kernel_disponivel() {
        return;
    }
    let bocas: [(&str, fn(&mut Hole)); 4] = [("simples", |_| {}), ("rebaixado", rebaixado), ("escareado", escareado), ("contrafurado", contrafurado)];
    for (nome, boca) in bocas {
        for (fundo, e_passante) in [("cego", false), ("passante", true)] {
            let v = volume_do_sulco(|h| {
                boca(h);
                if e_passante {
                    passante(h);
                }
            });
            assert!(v > 30., "{nome} {fundo}: o sulco tirou só {v} mm³");
        }
    }
}

#[test]
fn sulco_do_furo_cego_para_antes_do_fundo() {
    if !kernel_disponivel() {
        return;
    }
    // furo cego de 20 num bloco de 30: o fundo fica em z = 10. O sulco ocupa raios de 3,4 a 4: abaixo
    // do fundo, nesses raios, só existe vértice se o sulco passou do fundo.
    for boca in [(|_: &mut Hole| {}) as fn(&mut Hole), escareado, rebaixado] {
        let (d, _) = bloco(boca);
        let r = recompute(&d);
        let m = &r.bodies[0].mesh;
        let abaixo = m.positions.chunks_exact(3).filter(|p| p[2] < 10. - 0.02 && (p[0] * p[0] + p[1] * p[1]).sqrt() < 4.2).count();
        assert_eq!(abaixo, 0, "o sulco passou do fundo do furo cego ({abaixo} vértices abaixo de z = 10)");
    }
}

#[test]
fn rosca_modelada_continua_valida_e_parametrica() {
    if !kernel_disponivel() {
        return;
    }
    // trocar a profundidade do furo por parâmetro muda o comprimento do sulco
    let mut d = Document::new("paramétrico");
    d.add_param("prof", "20", Unit::Comprimento).unwrap();
    let sk = sk_rect(&mut d, XY, [-20., -15.], [20., 15.]);
    d.add_feature(ext(sk, BoolOp::Add, "30"));
    let topo = d.add_offset_plane(XY, "30");
    let sp = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[0., 0.]]);
    let mut h = Hole { sketch: sp, kind: HoleKind::Simples, diameter: s("6.8"), depth: Extent::Blind(s("prof")), thread: Some(s("M8x1.25")), thread_modeled: true, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: s("0"), left_hand: false, extra: Default::default() };
    escareado(&mut h);
    let id = d.add_feature(FeatureKind::Hole(h));
    let curto = volume_total(&recompute(&d));
    definir_param(&mut d, "prof", "26");
    let r = recompute(&d);
    assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{:?}", r.status[&id]);
    let longo = volume_total(&r);
    assert!(longo < curto - 100., "furo mais fundo e rosca mais longa deviam tirar mais material: {curto} -> {longo}");
    assert!(fenkra_solidos::final_shape(&d).is_some_and(|f| f.is_valid()));
}

#[test]
fn diametro_diferente_da_broca_de_rosca_avisa() {
    if !kernel_disponivel() {
        return;
    }
    let (d, id) = bloco(|h| h.diameter = s("8"));
    let r = recompute(&d);
    match &r.status[&id] {
        Status::Warning(m) => assert!(m.contains("broca de rosca"), "{m}"),
        outro => panic!("esperava aviso do diâmetro, veio {outro:?}"),
    }
}

#[test]
fn furo_curto_demais_para_a_rosca_avisa_em_vez_de_sumir() {
    if !kernel_disponivel() {
        return;
    }
    let (d, id) = bloco(|h| {
        escareado(h);
        h.depth = Extent::Blind(s("6"));
    });
    let r = recompute(&d);
    match &r.status[&id] {
        Status::Warning(m) => assert!(m.contains("não cabe"), "{m}"),
        outro => panic!("esperava aviso de que a rosca não cabe, veio {outro:?}"),
    }
}

// ------------------------------------------------------------------ prévia leve

#[test]
fn previa_leve_nao_constroi_a_helice_e_registra_a_posicao() {
    if !kernel_disponivel() {
        return;
    }
    let (d, id) = bloco(escareado);
    let completo = recompute(&d);
    let mut cache = Cache::default();
    cache.leve = Some(id);
    let leve = recompute_cached(&d, &mut cache);
    // sem sulco: o volume é o do furo cosmético
    let (cosm, _) = bloco(|h| {
        escareado(h);
        h.thread_modeled = false;
    });
    let v_cosm = volume_total(&recompute(&cosm));
    assert!((volume_total(&leve) - v_cosm).abs() < 1e-6, "{} vs {v_cosm}", volume_total(&leve));
    assert!(volume_total(&completo) < v_cosm - 30.);
    // a posição da rosca vai para a prévia: passo certo, comprimento até o fim da rosca, sem sulco
    let viz = leve.roscas.iter().find(|x| x.feature == id).expect("posição da rosca");
    assert!((viz.pitch - 1.25).abs() < 1e-9 && !viz.modelada && viz.interna);
    assert!(viz.len > 10. && viz.len < 15.4 + 1e-6, "comprimento {}", viz.len);
    assert!((viz.radius - 3.4).abs() < 1e-9);
    assert!(matches!(leve.status[&id], Status::Ok | Status::Warning(_)));
}

#[test]
fn concluir_depois_da_previa_leve_constroi_o_sulco() {
    if !kernel_disponivel() {
        return;
    }
    // o cache da edição não pode vazar para o resultado final
    let (d, id) = bloco(|h| {
        escareado(h);
        passante(h);
    });
    let mut cache = Cache::default();
    cache.leve = Some(id);
    let leve = volume_total(&recompute_cached(&d, &mut cache));
    cache.leve = None;
    let final_ = recompute_cached(&d, &mut cache);
    assert!(volume_total(&final_) < leve - 30., "ao concluir o sulco devia aparecer: {leve} -> {}", volume_total(&final_));
    assert!(final_.roscas.iter().any(|x| x.feature == id && x.modelada));
}

#[test]
fn previa_leve_e_bem_mais_rapida_que_a_helice() {
    if !kernel_disponivel() {
        return;
    }
    let (d, id) = bloco(|h| {
        passante(h);
        h.extra.thread_depth = String::new();
    });
    let t = Instant::now();
    let _ = recompute(&d);
    let completo = t.elapsed();
    let mut cache = Cache::default();
    cache.leve = Some(id);
    let t = Instant::now();
    let _ = recompute_cached(&d, &mut cache);
    let leve = t.elapsed();
    println!("B351 prévia: completo {completo:?}, leve {leve:?}");
    assert!(leve * 3 < completo, "a prévia leve ainda custa {leve:?} contra {completo:?}");
}

#[test]
fn comando_rosca_tem_a_mesma_previa_leve() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("eixo");
    let sk = sk_rect(&mut d, XY, [-20., -15.], [20., 15.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    let topo = d.add_offset_plane(XY, "20");
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Datum(topo), a: s("5"), b: s("20"), c: s("0"), d: s("0"), op: BoolOp::Add }));
    let sem = volume_total(&recompute(&d));
    let id = d.add_feature(FeatureKind::Thread(Thread { face: [5., 0., 30.], spec: s("M10x1.5"), length: s("15"), modeled: true, end: None, start: s("0") }));
    let mut cache = Cache::default();
    cache.leve = Some(id);
    let leve = recompute_cached(&d, &mut cache);
    assert!((volume_total(&leve) - sem).abs() < 1e-6, "a prévia leve não devia cortar nada");
    let viz = leve.roscas.iter().find(|x| x.feature == id).expect("posição");
    assert!((viz.len - 15.).abs() < 1e-9 && (viz.pitch - 1.5).abs() < 1e-9 && !viz.interna);
    cache.leve = None;
    let completo = recompute_cached(&d, &mut cache);
    assert!(volume_total(&completo) < sem - 50.);
}

/// O comando Rosca na face do furo escareado, com a face inteira: antes, a partir de 15,4 mm a
/// varredura de muitas voltas fazia a booleana devolver o corpo sem o sulco.
#[test]
fn comando_rosca_interna_no_comprimento_inteiro_do_furo() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, _) = bloco(|h| {
        escareado(h);
        h.thread = None;
    });
    let sem = volume_total(&recompute(&d));
    let id = d.add_feature(FeatureKind::Thread(Thread { face: [3.4, 0., 18.], spec: s("M8x1.25"), length: s("30"), modeled: true, end: None, start: s("0") }));
    let r = recompute(&d);
    let tirado = sem - volume_total(&r);
    assert!(tirado > 50., "rosca interna da face inteira tirou só {tirado} mm³ ({:?})", r.status[&id]);
    assert!(fenkra_solidos::final_shape(&d).is_some_and(|f| f.is_valid()));
}

/// Rosca externa comprida: o eixo de 40 mm roscado de ponta a ponta.
#[test]
fn comando_rosca_externa_comprida_remove_material_e_fica_valida() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("eixo");
    let sk = sk_rect(&mut d, XY, [-20., -15.], [20., 15.]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    let topo = d.add_offset_plane(XY, "10");
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Datum(topo), a: s("5"), b: s("40"), c: s("0"), d: s("0"), op: BoolOp::Add }));
    let sem = volume_total(&recompute(&d));
    let id = d.add_feature(FeatureKind::Thread(Thread { face: [5., 0., 30.], spec: s("M10x1.5"), length: s("40"), modeled: true, end: None, start: s("0") }));
    let r = recompute(&d);
    assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{:?}", r.status[&id]);
    // 36 mm de filete a ~3,3 mm³ por mm
    assert!(sem - volume_total(&r) > 80., "a rosca externa comprida tirou {}", sem - volume_total(&r));
    assert!(fenkra_solidos::final_shape(&d).is_some_and(|f| f.is_valid()));
}

#[test]
fn rosca_cosmetica_do_furo_nao_tem_geometria_mas_deixa_o_dado() {
    if !kernel_disponivel() {
        return;
    }
    let (d, id) = bloco(|h| {
        escareado(h);
        h.thread_modeled = false;
        h.left_hand = true;
    });
    let r = recompute(&d);
    let (sem, _) = bloco(|h| {
        escareado(h);
        h.thread = None;
    });
    assert!((volume_total(&r) - volume_total(&recompute(&sem))).abs() < 1e-6, "cosmética não corta nada");
    let viz = r.roscas.iter().find(|x| x.feature == id).expect("a face roscada precisa estar no resultado para a textura");
    assert!(viz.interna && viz.esquerda && !viz.modelada);
    // o dado continua no documento, para a chamada e o desenho
    let FeatureKind::Hole(h) = &d.feature(id).unwrap().kind else { panic!() };
    assert_eq!(h.thread.as_deref(), Some("M8x1.25"));
    assert!(matches!(r.status[&id], Status::Warning(ref m) if m.contains("rosca cosmética")));
}
