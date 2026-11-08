// SPDX-License-Identifier: LGPL-2.1-or-later
//! B285: cordão de solda por conjuntos de faces. Tubo 40x40x3 em pé com placa de extremidade 50x50x8
//! no alto; o cordão corre pelo encontro das quatro faces do tubo com a face de baixo da placa
//! (contorno fechado de 160 mm).

use fenkra_core::*;
use fenkra_solidos::{group_members, recompute, structural_members, weld_beads, Status, StructItem};

fn s(x: &str) -> String {
    x.to_string()
}

fn montagem() -> (Document, Id) {
    let mut d = Document::new("solda");
    let l = CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 0., 500.]), start: s("0"), end: s("0") }));
    let m = StructMember {
        paths: vec![l],
        family: s("Perfil"),
        designation: s("p"),
        kind: SectionKind::TuboQuadrado,
        dims: vec![s("40"), s("3")],
        anchor: 0,
        angle: s("0"),
        off_x: s("0"),
        off_y: s("0"),
        ext1: s("0"),
        ext2: s("0"),
        corner: CornerMode::Nenhum,
        gap: s("0"),
        joints: vec![],
        material: String::new(),
        mirror: false,
    };
    let id = d.add_feature(FeatureKind::StructMember(m));
    d.add_feature(FeatureKind::StructPlate(StructPlate::nova([0., 0., 500.], "8", "5")));
    (d, id)
}

/// Cordão nas quatro faces do tubo contra a face de baixo da placa.
fn cordao(configura: impl Fn(&mut StructWeld)) -> (Document, Id) {
    let (mut d, _) = montagem();
    let mut w = StructWeld::novo("5");
    for p in [[20., 0., 250.], [-20., 0., 250.], [0., 20., 250.], [0., -20., 250.]] {
        w.adicionar(p, None);
    }
    w.picking_b = true;
    w.adicionar([22., 0., 500.], None);
    configura(&mut w);
    let id = d.add_feature(FeatureKind::StructWeld(w));
    (d, id)
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn volume(d: &Document) -> f64 {
    recompute(d).bodies.iter().map(|b| b.mass.volume).sum()
}

const V_BASE: f64 = (40. * 40. - 34. * 34.) * 500. + 50. * 50. * 8.;

#[test]
fn cosmetico_acha_o_contorno_e_nao_muda_o_volume() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, id) = cordao(|_| {});
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    assert!(perto(volume(&d), V_BASE, 1.), "o cosmético não gera material: {}", volume(&d));
    let c = weld_beads(&d);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].modo, BeadMode::Cosmetico);
    assert!(perto(c[0].comprimento_total(), 160., 0.01), "{}", c[0].comprimento_total());
    assert_eq!(c[0].trechos(), c[0].caminhos.len());
    assert!(c[0].caminhos[0].fechado);
    assert!(structural_members(&d).iter().all(|m| m.kind != StructItem::Weld));
}

#[test]
fn solido_soma_o_volume_do_filete_e_entra_na_lista() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, id) = cordao(|w| w.mode = BeadMode::Solido);
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    // seção a^2/2 sobre o perímetro da linha do centroide: 160 + 8 a/3 (esquadria nas quinas)
    let esperado = 12.5 * (160. + 8. * 5. / 3.);
    let v = volume(&d) - V_BASE;
    assert!(perto(v, esperado, 0.01 * esperado), "volume do cordão {v}, esperado {esperado}");
    let c = weld_beads(&d);
    assert!(perto(c[0].volume, esperado, 0.01 * esperado), "{}", c[0].volume);
    let itens: Vec<_> = structural_members(&d).into_iter().filter(|m| m.kind == StructItem::Weld).collect();
    assert_eq!(itens.len(), 1);
    let grupos = group_members(&itens);
    assert_eq!(grupos[0].name, "Cordão z5");
}

#[test]
fn formato_convexo_e_concavo_mudam_o_volume() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let vol = |f: BeadShape| {
        let (d, id) = cordao(|w| {
            w.mode = BeadMode::Solido;
            w.shape = f;
        });
        assert_eq!(recompute(&d).status[&id], Status::Ok);
        weld_beads(&d)[0].volume
    };
    let (p, cx, cv) = (vol(BeadShape::Plano), vol(BeadShape::Convexo), vol(BeadShape::Concavo));
    assert!(cx > p * 1.05 && cv < p * 0.95, "{cx} {p} {cv}");
}

#[test]
fn intermitente_conta_os_trechos_e_so_solda_o_pedido() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // contorno fechado de 160 mm; trechos de 10 a cada 20 = 8
    let (d, id) = cordao(|w| {
        w.pattern = BeadPattern::ComprimentoEVao;
        w.seg_len = s("10");
        w.spacing = s("10");
    });
    assert_eq!(recompute(&d).status[&id], Status::Ok);
    let c = &weld_beads(&d)[0];
    assert_eq!(c.trechos(), 8, "{}", c.trechos());
    assert!(perto(c.comprimento_soldado(), 80., 0.01));
    // por passo: comprimento 10, passo 20 dá o mesmo cordão
    let (d2, _) = cordao(|w| {
        w.pattern = BeadPattern::ComprimentoEPasso;
        w.seg_len = s("10");
        w.spacing = s("20");
    });
    assert_eq!(weld_beads(&d2)[0].trechos(), 8);
    // passo menor que o trecho é erro claro
    let (d3, id3) = cordao(|w| {
        w.pattern = BeadPattern::ComprimentoEPasso;
        w.seg_len = s("10");
        w.spacing = s("5");
    });
    assert!(matches!(recompute(&d3).status[&id3], Status::Error(_)));
}

#[test]
fn intermitente_solido_gera_um_solido_por_trecho_com_o_volume_certo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, id) = cordao(|w| {
        w.mode = BeadMode::Solido;
        w.pattern = BeadPattern::ComprimentoEVao;
        w.seg_len = s("10");
        w.spacing = s("10");
    });
    assert_eq!(recompute(&d).status[&id], Status::Ok);
    let itens: Vec<_> = structural_members(&d).into_iter().filter(|m| m.kind == StructItem::Weld).collect();
    assert_eq!(itens.len(), 8);
    // cada trecho reto de 10 mm tem 12,5 x 10 = 125 mm3 (os que pegam quina são um pouco maiores)
    let v = volume(&d) - V_BASE;
    assert!(v >= 8. * 125. - 1. && v < 8. * 125. + 60., "{v}");
}

#[test]
fn termino_em_plano_corta_o_cordao() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // o plano XZ (normal +y) deixa só a metade y >= 0 do contorno
    let (d, id) = cordao(|w| {
        w.end_plane = Some(PlaneRef::Origin(OriginPlane::XZ));
        w.end_keep_front = true;
    });
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    let c = &weld_beads(&d)[0];
    assert!(perto(c.comprimento_total(), 80., 0.5), "{}", c.comprimento_total());
}

#[test]
fn alternado_nao_desloca_o_primeiro_caminho() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, _) = cordao(|w| {
        w.pattern = BeadPattern::ComprimentoEVao;
        w.seg_len = s("10");
        w.spacing = s("10");
        w.staggered = true;
    });
    let c = &weld_beads(&d)[0];
    assert_eq!(c.caminhos.len(), 1);
    assert_eq!(c.trechos(), 8);
}

#[test]
fn sem_faces_ou_sem_encontro_avisa_ou_falha_sem_derrubar() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, _) = montagem();
    let id = d.add_feature(FeatureKind::StructWeld(StructWeld::novo("5")));
    assert!(matches!(recompute(&d).status[&id], Status::Warning(_) | Status::Ok));
    // conjuntos que não se tocam: duas faces opostas do tubo
    let (mut d2, _) = montagem();
    let mut w = StructWeld::novo("5");
    w.adicionar([20., 0., 250.], None);
    w.picking_b = true;
    w.adicionar([-20., 0., 250.], None);
    let id2 = d2.add_feature(FeatureKind::StructWeld(w));
    match &recompute(&d2).status[&id2] {
        // a mensagem diz a que distância um conjunto ficou do outro (faces opostas do tubo 40)
        Status::Error(e) => assert!(e.contains("cerca de 40.0 mm"), "{e}"),
        o => panic!("devia falhar: {o:?}"),
    }
    // perna inválida
    let (d3, id3) = cordao(|w| w.leg = s("0"));
    assert!(matches!(recompute(&d3).status[&id3], Status::Error(_)));
}

#[test]
fn ida_e_volta_em_json_e_campos_novos_com_padrao() {
    let (d, _) = cordao(|w| w.mode = BeadMode::Solido);
    let j = serde_json::to_string(&d).unwrap();
    let d2: Document = serde_json::from_str(&j).unwrap();
    assert_eq!(serde_json::to_string(&d2).unwrap(), j);
    // só os campos obrigatórios: o resto assume o padrão (cosmético, contínuo)
    let w: StructWeld = serde_json::from_str(r#"{"faces":[[0.0,0.0,0.0]],"split":1,"leg":"4"}"#).unwrap();
    assert_eq!(w.mode, BeadMode::Cosmetico);
    assert_eq!(w.pattern, BeadPattern::Continuo);
    assert_eq!(w.conjuntos().0.len(), 1);
}
