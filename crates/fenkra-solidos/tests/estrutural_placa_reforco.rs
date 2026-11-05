// SPDX-License-Identifier: LGPL-2.1-or-later
//! B255 e B253: variantes da placa e do reforço (volume e caixa de cada uma), migração dos
//! arquivos que guardavam só o ponto e espelhamento da seção.

use fenkra_core::*;
use fenkra_solidos::{recompute, structural_members, MemberSolid, StructItem, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn membro(paths: Vec<CurveRef>, kind: SectionKind, dims: &[&str], anchor: u8) -> StructMember {
    StructMember {
        paths,
        family: s("Perfil"),
        designation: s("p"),
        kind,
        dims: dims.iter().map(|d| s(d)).collect(),
        anchor,
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
    }
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: "0".into(), end: "0".into() }))
}

/// Tubo 40x3 na vertical, de z = 0 a 500.
fn com_tubo() -> (Document, Id) {
    let mut d = Document::new("placa");
    let l = linha(&mut d, [0., 0., 0.], [0., 0., 500.]);
    let m = d.add_feature(FeatureKind::StructMember(membro(vec![l], SectionKind::TuboQuadrado, &["40", "3"], 0)));
    (d, m)
}

const V_TUBO: f64 = (40. * 40. - 34. * 34.) * 500.;

fn volume(d: &Document) -> f64 {
    recompute(d).bodies.iter().map(|b| b.mass.volume).sum()
}

fn caixa(d: &Document) -> ([f64; 3], [f64; 3]) {
    let r = recompute(d);
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for b in &r.bodies {
        for p in b.mesh.positions.chunks(3) {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k] as f64);
                hi[k] = hi[k].max(p[k] as f64);
            }
        }
    }
    (lo, hi)
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn ok(d: &Document, id: Id) {
    let r = recompute(d);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
}

#[test]
fn placa_externa_com_sobra_em_mm() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, _) = com_tubo();
    let p = d.add_feature(FeatureKind::StructPlate(StructPlate::nova([0., 0., 500.], "8", "5")));
    ok(&d, p);
    // 50 x 50 x 8 além da ponta
    assert!(perto(volume(&d), V_TUBO + 50. * 50. * 8., 1.), "{}", volume(&d));
    let (lo, hi) = caixa(&d);
    assert!(perto(hi[2], 508., 0.01) && perto(lo[2], 0., 0.01), "{lo:?} {hi:?}");
    assert!(perto(hi[0], 25., 0.01) && perto(lo[0], -25., 0.01));
}

#[test]
fn placa_interna_recuada_com_sobra_em_razao_da_parede() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, _) = com_tubo();
    let mut pl = StructPlate::nova([0., 0., 500.], "8", "-1");
    pl.side = PlateSide::Interno;
    pl.margin_mode = MarginMode::RazaoParede;
    pl.recess = s("2");
    let p = d.add_feature(FeatureKind::StructPlate(pl));
    ok(&d, p);
    // sobra -1 parede = -3 mm: 34 x 34 x 8 dentro do tubo; a caixa do conjunto não cresce
    assert!(perto(volume(&d), V_TUBO + 34. * 34. * 8., 1.), "{}", volume(&d));
    let (lo, hi) = caixa(&d);
    assert!(perto(hi[2], 500., 0.01) && perto(lo[2], 0., 0.01), "{lo:?} {hi:?}");
    // a placa fica de z = 490 a 498 (recuo de 2 mm da ponta)
    let ms = structural_members(&d);
    let placa = ms.iter().find(|m| m.kind == StructItem::Plate).unwrap();
    let base = placa.frame.origin[2];
    assert!(perto(base, 490., 1e-6), "base da placa {base}");
}

#[test]
fn placa_com_furos_de_fixacao_perde_o_volume_dos_furos() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, _) = com_tubo();
    let mut pl = StructPlate::nova([0., 0., 500.], "8", "5");
    pl.holes = Some(PlateHoles { diameter: s("6"), edge: s("8") });
    let p = d.add_feature(FeatureKind::StructPlate(pl));
    ok(&d, p);
    let furos = 4. * std::f64::consts::PI * 9. * 8.;
    assert!(perto(volume(&d), V_TUBO + 50. * 50. * 8. - furos, 2.), "{}", volume(&d));
    // furo grande demais: erro claro, sem placa
    let (mut d2, _) = com_tubo();
    let mut pl = StructPlate::nova([0., 0., 500.], "8", "5");
    pl.holes = Some(PlateHoles { diameter: s("30"), edge: s("8") });
    let p2 = d2.add_feature(FeatureKind::StructPlate(pl));
    assert!(matches!(recompute(&d2).status[&p2], Status::Error(_)));
}

#[test]
fn razao_da_parede_em_barra_maciça_e_recusada() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("maciça");
    let l = linha(&mut d, [0., 0., 0.], [0., 0., 300.]);
    d.add_feature(FeatureKind::StructMember(membro(vec![l], SectionKind::BarraRedonda, &["20"], 0)));
    let mut pl = StructPlate::nova([0., 0., 300.], "5", "1");
    pl.margin_mode = MarginMode::RazaoParede;
    let p = d.add_feature(FeatureKind::StructPlate(pl));
    assert!(matches!(recompute(&d).status[&p], Status::Error(_)));
}

#[test]
fn arquivo_antigo_so_com_o_ponto_continua_igual() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // placa e reforço gravados antes da B255: só o ponto, a espessura e a sobra (ou as abas)
    let antiga: StructPlate = serde_json::from_str(r#"{"at":[0.0,0.0,500.0],"thickness":"8","margin":"5"}"#).unwrap();
    assert_eq!(antiga, StructPlate::nova([0., 0., 500.], "8", "5"));
    let (mut d, _) = com_tubo();
    let p = d.add_feature(FeatureKind::StructPlate(antiga));
    ok(&d, p);
    assert!(perto(volume(&d), V_TUBO + 50. * 50. * 8., 1.));
    let velho: StructGusset = serde_json::from_str(r#"{"at":[0.0,0.0,0.0],"leg":"60","thickness":"6","chamfer":"10"}"#).unwrap();
    assert_eq!(velho, StructGusset::novo([0., 0., 0.], "60", "6", "10"));
    // e um membro antigo (sem `mirror`) continua sem espelhar
    let m: StructMember = serde_json::from_str(
        r#"{"paths":[],"family":"f","designation":"d","kind":"TuboQuadrado","dims":["40","3"],"anchor":0,"angle":"0","off_x":"0","off_y":"0","ext1":"0","ext2":"0","corner":"Nenhum","gap":"0"}"#,
    )
    .unwrap();
    assert!(!m.mirror);
}

#[test]
fn clicar_a_face_da_ponta_escolhe_essa_ponta_mesmo_com_outra_mais_perto() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // L: A sobe de (0,0,0) a (0,0,500); B sai da ponta de A para +x. Um clique na face do topo de A,
    // a 15 mm do eixo, está tão perto do começo de B quanto da ponta de A.
    let mut d = Document::new("l");
    let a = linha(&mut d, [0., 0., 0.], [0., 0., 500.]);
    let b = linha(&mut d, [0., 0., 500.], [400., 0., 500.]);
    d.add_feature(FeatureKind::StructMember(membro(vec![a], SectionKind::TuboQuadrado, &["40", "3"], 0)));
    d.add_feature(FeatureKind::StructMember(membro(vec![b], SectionKind::TuboQuadrado, &["40", "3"], 0)));
    let p = d.add_feature(FeatureKind::StructPlate(StructPlate::nova([0., 15., 500.], "8", "5")));
    ok(&d, p);
    let ms = structural_members(&d);
    let placa = ms.iter().find(|m| m.kind == StructItem::Plate).unwrap();
    // normal para fora da ponta de A (sobe), e não do começo de B (que sairia para -x)
    assert!(perto(placa.frame.normal[2], 1., 1e-9), "{:?}", placa.frame.normal);
}

fn dois_tubos_em_l() -> (Document, Id) {
    let mut d = Document::new("reforço");
    let a = linha(&mut d, [0., 0., 0.], [0., 0., 500.]);
    let b = linha(&mut d, [0., 0., 0.], [400., 0., 0.]);
    let m1 = d.add_feature(FeatureKind::StructMember(membro(vec![a], SectionKind::TuboQuadrado, &["40", "3"], 0)));
    d.add_feature(FeatureKind::StructMember(membro(vec![b], SectionKind::TuboQuadrado, &["40", "3"], 0)));
    (d, m1)
}

fn reforcos(d: &Document) -> Vec<MemberSolid> {
    structural_members(d).into_iter().filter(|m| m.kind == StructItem::Gusset).collect()
}

fn coord_na_espessura(m: &MemberSolid) -> f64 {
    (0..3).map(|i| m.frame.origin[i] * m.frame.normal[i]).sum()
}

fn largura_local(m: &MemberSolid) -> (f64, f64) {
    let (lo, hi) = m.shape.bbox().unwrap();
    (hi[0] - lo[0], hi[1] - lo[1])
}

#[test]
fn reforco_triangular_chanfrado_e_poligonal() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let vol = |g: StructGusset| {
        let (mut d, _) = dois_tubos_em_l();
        let id = d.add_feature(FeatureKind::StructGusset(g));
        ok(&d, id);
        let r = reforcos(&d);
        assert_eq!(r.len(), 1);
        r[0].shape.mass().volume
    };
    // triangular 60 x 60, espessura 6
    assert!(perto(vol(StructGusset::novo([0., 0., 0.], "60", "6", "0")), 60. * 60. / 2. * 6., 1.));
    // chanfrado: tira o triângulo do canto (10 x 10)
    assert!(perto(vol(StructGusset::novo([0., 0., 0.], "60", "6", "10")), (60. * 60. / 2. - 10. * 10. / 2.) * 6., 1.));
    // poligonal: quadrilátero (0,0) (30,0) (30,10) (0,40), área 750
    let mut g = StructGusset::novo([0., 0., 0.], "60", "6", "0");
    g.profile = GussetProfile::Poligonal;
    g.poly = [["0", "0"], ["30", "0"], ["30", "10"], ["0", "40"]].iter().map(|q| [s(q[0]), s(q[1])]).collect();
    assert!(perto(vol(g), 750. * 6., 1.));
    // poligonal sem vértices: erro
    let (mut d, _) = dois_tubos_em_l();
    let mut g = StructGusset::novo([0., 0., 0.], "60", "6", "0");
    g.profile = GussetProfile::Poligonal;
    let id = d.add_feature(FeatureKind::StructGusset(g));
    assert!(matches!(recompute(&d).status[&id], Status::Error(_)));
}

#[test]
fn abas_d1_e_d2_com_inversao() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mk = |swap: bool| {
        let (mut d, _) = dois_tubos_em_l();
        let mut g = StructGusset::novo([0., 0., 0.], "60", "6", "0");
        g.leg2 = s("40");
        g.swap = swap;
        let id = d.add_feature(FeatureKind::StructGusset(g));
        ok(&d, id);
        let r = reforcos(&d);
        (r[0].shape.mass().volume, largura_local(&r[0]))
    };
    let (v1, (u1, w1)) = mk(false);
    let (v2, (u2, w2)) = mk(true);
    assert!(perto(v1, 60. * 40. / 2. * 6., 1.) && perto(v2, v1, 1e-6), "{v1} {v2}");
    // D1 corre pelo primeiro membro (eixo X local); com a inversão, a aba longa troca de membro
    assert!(perto(u1, 60., 1e-6) && perto(w1, 40., 1e-6), "{u1} {w1}");
    assert!(perto(u2, 40., 1e-6) && perto(w2, 60., 1e-6), "{u2} {w2}");
}

#[test]
fn posicao_lado_e_repeticao_ao_longo_da_aresta() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // a aresta de encontro tem 40 mm (largura do tubo); espessura 6
    // coordenada da face de base da espessura ao longo da normal do reforço: a origem do referencial
    // fica no plano das abas deslocado pela faixa, então origem · normal é o início da faixa
    let pos = |f: &dyn Fn(&mut StructGusset)| -> Vec<f64> {
        let (mut d, _) = dois_tubos_em_l();
        let mut g = StructGusset::novo([0., 0., 0.], "60", "6", "0");
        f(&mut g);
        let id = d.add_feature(FeatureKind::StructGusset(g));
        ok(&d, id);
        let mut v: Vec<f64> = reforcos(&d).iter().map(coord_na_espessura).collect();
        v.sort_by(|a, b| a.total_cmp(b));
        v
    };
    let escala = |v: Vec<f64>| v;
    // coordenada da face de base da espessura (o ponto de referência do sólido) ao longo da normal
    // início: face encostada no começo da aresta (-20), meio com dois lados (-3), fim (14)
    let (ini, meio, fim) = (
        escala(pos(&|g| g.position = GussetPos::Inicio)),
        escala(pos(&|g| g.position = GussetPos::Meio)),
        escala(pos(&|g| g.position = GussetPos::Fim)),
    );
    // as três posições são distintas e ordenadas ao longo da aresta, com 17 e 17 mm de distância
    assert_eq!((ini.len(), meio.len(), fim.len()), (1, 1, 1));
    assert!(perto((meio[0] - ini[0]).abs(), 17., 1e-6), "{ini:?} {meio:?}");
    assert!(perto((fim[0] - meio[0]).abs(), 17., 1e-6), "{meio:?} {fim:?}");
    // afastamento de 4 mm empurra o início para dentro
    let ini4 = escala(pos(&|g| {
        g.position = GussetPos::Inicio;
        g.offset = s("4");
    }));
    assert!(perto((ini4[0] - ini[0]).abs(), 4., 1e-6), "{ini:?} {ini4:?}");
    // quantidade: 3 cópias, nas pontas e no meio, 17 mm entre elas
    let q = escala(pos(&|g| {
        g.repeat = GussetRepeat::Quantidade;
        g.count = s("3");
    }));
    assert_eq!(q.len(), 3);
    assert!(perto(q[1] - q[0], 17., 1e-6) && perto(q[2] - q[1], 17., 1e-6), "{q:?}");
    // espaçamento de 12 a partir do início: 3 cópias cabem em 40 mm
    let e = escala(pos(&|g| {
        g.position = GussetPos::Inicio;
        g.repeat = GussetRepeat::Espacamento;
        g.spacing = s("12");
    }));
    assert_eq!(e.len(), 3);
    assert!(perto(e[1] - e[0], 12., 1e-6) && perto(e[2] - e[1], 12., 1e-6), "{e:?}");
    // volume: cada cópia é um reforço inteiro
    let (mut d, _) = dois_tubos_em_l();
    let mut g = StructGusset::novo([0., 0., 0.], "60", "6", "0");
    g.repeat = GussetRepeat::Quantidade;
    g.count = s("3");
    let id = d.add_feature(FeatureKind::StructGusset(g));
    ok(&d, id);
    let total: f64 = reforcos(&d).iter().map(|m| m.shape.mass().volume).sum();
    assert!(perto(total, 3. * 60. * 60. / 2. * 6., 2.), "{total}");
    // não cabe: quatro reforços de 6 mm com espaçamento maior que a aresta
    let (mut d, _) = dois_tubos_em_l();
    let mut g = StructGusset::novo([0., 0., 0.], "60", "6", "0");
    g.repeat = GussetRepeat::Quantidade;
    g.count = s("12");
    let id = d.add_feature(FeatureKind::StructGusset(g));
    assert!(matches!(recompute(&d).status[&id], Status::Error(_)));
}

#[test]
fn espelhar_a_secao_troca_o_lado_do_perfil_u() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let caixa_y = |mirror: bool| {
        let mut d = Document::new("u");
        let l = linha(&mut d, [0., 0., 0.], [0., 0., 300.]);
        let mut m = membro(vec![l], SectionKind::PerfilU, &["100", "50", "5", "8"], 9);
        m.mirror = mirror;
        let id = d.add_feature(FeatureKind::StructMember(m));
        ok(&d, id);
        let (lo, hi) = caixa(&d);
        (lo[1], hi[1], volume(&d))
    };
    let (a0, a1, va) = caixa_y(false);
    let (b0, b1, vb) = caixa_y(true);
    assert!(perto(va, vb, 1e-3 * va), "o espelho conserva o volume");
    // ancorado no centróide, o U não é simétrico em torno da linha guia: espelhar inverte a caixa
    assert!(!perto(a0, -a1, 0.5), "o U ancorado no centróide não deveria ser simétrico: {a0} {a1}");
    assert!(perto(b0, -a1, 0.01) && perto(b1, -a0, 0.01), "{a0} {a1} / {b0} {b1}");
}
