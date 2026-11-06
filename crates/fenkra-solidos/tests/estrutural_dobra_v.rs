// SPDX-License-Identifier: LGPL-2.1-or-later
//! B328, B333: rasgos em V e canto com gabarito no 3D (tubo fechado). Documentos montados em código,
//! recalculados no kernel; confere status, volume e caixa envolvente. A geometria é derivada, sem
//! protótipo: os testes conferem coerência com as contas da pesquisa, não o objeto real.

use fenkra_core::*;
use fenkra_solidos::{cut_list, recompute, RecomputeResult, Status};

const PI: f64 = std::f64::consts::PI;

fn s(x: &str) -> String {
    x.to_string()
}

fn membro(paths: Vec<CurveRef>, kind: SectionKind, dims: &[&str]) -> StructMember {
    StructMember {
        paths,
        family: s("Tubo"),
        designation: s("teste"),
        kind,
        dims: dims.iter().map(|d| s(d)).collect(),
        anchor: 0,
        angle: s("0"),
        off_x: s("0"),
        off_y: s("0"),
        ext1: s("0"),
        ext2: s("0"),
        corner: CornerMode::MeiaEsquadria,
        gap: s("0"),
        joints: vec![],
        material: String::new(),
        mirror: false,
    }
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }))
}

fn bend(mode: BendMode, raio: &str) -> TubeBend {
    TubeBend { mode, radius: s(raio), ..Default::default() }
}

/// L de 300 + 300 num comando só, com o nó entre as duas linhas dobrado.
fn l_dobrado(kind: SectionKind, dims: &[&str], b: TubeBend) -> (Document, Id) {
    let mut d = Document::new("dobra");
    let l1 = linha(&mut d, [0., 0., 0.], [300., 0., 0.]);
    let l2 = linha(&mut d, [300., 0., 0.], [300., 300., 0.]);
    let f = d.add_feature(FeatureKind::StructMember(membro(vec![l1, l2], kind, dims)));
    if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|f| &mut f.kind) {
        m.joints.push(StructJoint { mine: 1, other: f, other_path: 0, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(b) });
    }
    (d, f)
}

fn caixa(r: &RecomputeResult) -> ([f64; 3], [f64; 3]) {
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

fn volume(r: &RecomputeResult) -> f64 {
    r.bodies.iter().map(|b| b.mass.volume).sum()
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// Status sem erro; devolve o aviso, se houver.
fn sem_erro(r: &RecomputeResult, f: Id) -> String {
    match &r.status[&f] {
        Status::Ok => String::new(),
        Status::Warning(w) => w.clone(),
        outro => panic!("{outro:?}"),
    }
}

#[test]
fn canto_com_gabarito_do_caso_de_aceite_tem_o_volume_das_pecas() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // 30 x 30 x 1,2 a 90 graus, Ro = 30: placa 46,17, dedução 13,83
    let (d, f) = l_dobrado(SectionKind::TuboQuadrado, &["30", "1.2"], bend(BendMode::VComRaio, "30"));
    let r = recompute(&d);
    let aviso = sem_erro(&r, f);
    assert!(aviso.contains("a validar em peça real"), "{aviso}");
    assert_eq!(r.bodies.len(), 1);
    // pernas de 285 (o arco começa a Ro - H/2 = 15 antes do nó), placa em arco, dois setores e a
    // ponta da parede interna que vai até a bissetriz
    let a_tubo = 30. * 30. - 27.6 * 27.6;
    let pernas = 2. * a_tubo * 285.;
    let placa = 0.5 * (PI / 2.) * (30f64.powi(2) - 28.8f64.powi(2)) * 30.;
    let setores = 2. * 0.5 * (PI / 2.) * 28.8f64.powi(2) * 1.2;
    let cunha = 2. * 0.5 * 1.2 * 1.2 * 27.6;
    let esperado = pernas + placa + setores + cunha;
    assert!(perto(volume(&r), esperado, 0.003 * esperado), "volume {} contra {esperado}", volume(&r));
    // a caixa é a do canto vivo: o raio externo só arredonda por dentro da caixa
    let (lo, hi) = caixa(&r);
    assert!(perto(lo[0], 0., 0.2) && perto(hi[0], 315., 0.2) && perto(lo[1], -15., 0.2) && perto(hi[1], 300., 0.2), "{lo:?} {hi:?}");
    // comprimento de corte: 600 + (H tan(h) - dedução) = 616,17
    assert!(perto(cut_list(&d)[0].length, 616.17, 0.06), "{}", cut_list(&d)[0].length);
}

#[test]
fn canto_com_gabarito_retangular_e_com_folga_de_raiz() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // 20 x 40 x 1,5 com a lateral de 20 no plano: Ro = 20, pernas retas de 300 - 10
    let (d, f) = l_dobrado(SectionKind::TuboRetangular, &["20", "40", "1.5"], bend(BendMode::VComRaio, "20"));
    let r = recompute(&d);
    sem_erro(&r, f);
    assert_eq!(r.bodies.len(), 1);
    let a_tubo = 20. * 40. - 17. * 37.;
    let esperado = 2. * a_tubo * 290. + 0.5 * (PI / 2.) * (400. - 18.5f64.powi(2)) * 40. + 2. * 0.5 * (PI / 2.) * 18.5f64.powi(2) * 1.5 + 2. * 0.5 * 1.5 * 1.5 * 37.;
    assert!(perto(volume(&r), esperado, 0.004 * esperado), "volume {} contra {esperado}", volume(&r));
    // a folga de raiz tira material dos setores e não muda a placa
    let b = TubeBend { root_gap: s("2"), ..bend(BendMode::VComRaio, "20") };
    let (d2, f2) = l_dobrado(SectionKind::TuboRetangular, &["20", "40", "1.5"], b);
    let r2 = recompute(&d2);
    sem_erro(&r2, f2);
    assert_eq!(r2.bodies.len(), 1);
    let tirado = volume(&r) - volume(&r2);
    // faixa de 2 mm por 18,5 de comprimento por duas paredes de 1,5, no máximo
    assert!(tirado > 20. && tirado < 2. * 2. * 18.5 * 1.5 * 1.05, "tirado {tirado}");
}

#[test]
fn canto_com_gabarito_fora_da_faixa_falha_com_motivo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, f) = l_dobrado(SectionKind::TuboQuadrado, &["30", "1.2"], bend(BendMode::VComRaio, "200"));
    match &recompute(&d).status[&f] {
        Status::Error(e) => assert!(e.contains("grande demais") || e.contains("não cabe"), "{e}"),
        outro => panic!("{outro:?}"),
    }
}

#[test]
fn v_comum_fecha_em_esquadria_com_a_dobradica_arredondada() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // 30 x 30 x 1,2, ri = 1,8 (raio externo 3,0): esquadria com o canto de fora arredondado
    let (d, f) = l_dobrado(SectionKind::TuboQuadrado, &["30", "1.2"], TubeBend { k: s("0.33"), ..bend(BendMode::VComum, "1.8") });
    let r = recompute(&d);
    let aviso = sem_erro(&r, f);
    assert!(aviso.contains("a validar em peça real"), "{aviso}");
    assert_eq!(r.bodies.len(), 1);
    // o L em esquadria tem a seção vezes os eixos (600); o arredondamento muda pouco
    let a_tubo = 30. * 30. - 27.6 * 27.6;
    let esquadria = a_tubo * 600.;
    let dif = volume(&r) - esquadria;
    assert!(dif.abs() < 0.0015 * esquadria && dif.abs() > 0.5, "diferença {dif}");
    let (lo, hi) = caixa(&r);
    assert!(perto(hi[0], 315., 0.2) && perto(lo[1], -15., 0.2), "{lo:?} {hi:?}");
    // lista de corte pela fórmula do V: X1 + X2 - BD, com X medido até o canto de fora (315 + 315) e BD = 2,551
    assert!(perto(cut_list(&d)[0].length, 630. - 2.551, 0.06), "{}", cut_list(&d)[0].length);
}

#[test]
fn v_multiplo_forma_um_poligono_de_pecas_do_meio() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // n = 6, Rp = 45, 90 graus: cinco peças do meio entre as duas pernas, tudo num sólido só
    let b = TubeBend { cuts: s("6"), ..bend(BendMode::VComum, "45") };
    let (d, f) = l_dobrado(SectionKind::TuboQuadrado, &["30", "1.2"], b);
    let r = recompute(&d);
    let aviso = sem_erro(&r, f);
    assert!(aviso.contains("a validar em peça real"), "{aviso}");
    assert_eq!(r.bodies.len(), 1);
    // eixo do polígono: 608,87 (face interna da placa) - 6 x 2 x (H/2 - t) x tan(7,5 graus) = 587,07
    let a_tubo = 30. * 30. - 27.6 * 27.6;
    let esperado = a_tubo * 587.07;
    assert!(perto(volume(&r), esperado, 0.006 * esperado), "volume {} contra {esperado}", volume(&r));
    let (lo, hi) = caixa(&r);
    assert!(perto(hi[0], 315., 0.2) && perto(lo[1], -15., 0.2) && perto(hi[1], 300., 0.2), "{lo:?} {hi:?}");
    // lista de corte: X1i + X2i - 2s + (n-1)p, medida na face interna; aqui só confere a ordem de grandeza
    let corte = cut_list(&d)[0].length;
    assert!(corte > 560. && corte < 640., "{corte}");
}

#[test]
fn v_multiplo_com_raio_pequeno_falha_com_motivo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // Rp mínimo para n = 6 é 29,05
    let b = TubeBend { cuts: s("6"), ..bend(BendMode::VComum, "25") };
    let (d, f) = l_dobrado(SectionKind::TuboQuadrado, &["30", "1.2"], b);
    match &recompute(&d).status[&f] {
        Status::Error(e) => assert!(e.contains("V múltiplo"), "{e}"),
        outro => panic!("{outro:?}"),
    }
}

#[test]
fn canto_com_gabarito_em_angulo_diferente_de_noventa_graus() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // desvio de 60 graus (as linhas formam 120): canto com gabarito Ro = H
    let mut d = Document::new("v");
    let l1 = linha(&mut d, [0., 0., 0.], [300., 0., 0.]);
    let l2 = linha(&mut d, [300., 0., 0.], [450., 300. * 0.866025403784, 0.]);
    let f = d.add_feature(FeatureKind::StructMember(membro(vec![l1, l2], SectionKind::TuboQuadrado, &["30", "1.2"])));
    if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|f| &mut f.kind) {
        m.joints.push(StructJoint { mine: 1, other: f, other_path: 0, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(bend(BendMode::VComRaio, "30")) });
    }
    let r = recompute(&d);
    sem_erro(&r, f);
    assert_eq!(r.bodies.len(), 1);
    // pernas retas de 300 - (Ro - H/2) tan(30); placa; setores; cunha da parede interna
    let th = PI / 3.;
    let a_tubo = 30. * 30. - 27.6 * 27.6;
    let t30 = (PI / 6.).tan();
    let esperado = 2. * a_tubo * (300. - 15. * t30) + 0.5 * th * (900. - 28.8f64.powi(2)) * 30. + 2. * 0.5 * th * 28.8f64.powi(2) * 1.2 + 2. * 0.5 * 1.2 * 1.2 * t30 * 27.6;
    assert!(perto(volume(&r), esperado, 0.004 * esperado), "volume {} contra {esperado}", volume(&r));
}

#[test]
fn canto_com_gabarito_com_raio_menor_e_maior_que_a_altura_continua_um_solido() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // 40 x 40 x 2: Ro = 30 (menor que H, a esquadria também corta a perna) e Ro = 60 (maior que H,
    // as laterais passam do início do arco e a face de dentro as limita)
    for ro in ["30", "60"] {
        let (d, f) = l_dobrado(SectionKind::TuboQuadrado, &["40", "2"], bend(BendMode::VComRaio, ro));
        let r = recompute(&d);
        sem_erro(&r, f);
        assert_eq!(r.bodies.len(), 1, "Ro = {ro}");
        let rof: f64 = ro.parse().unwrap();
        let rf = rof - 2.;
        // volume somado: pernas retas, placa em arco e nervuras (setores), com folga para as cunhas
        let a_tubo = 40. * 40. - 36. * 36.;
        let perna = 300. - (rof - 20.);
        let topo = 40f64.min(rof);
        let nervuras = 2. * 0.5 * (PI / 2.) * (rf.powi(2) - (rof - topo).powi(2)) * 2.;
        let placa = 0.5 * (PI / 2.) * (rof.powi(2) - rf.powi(2)) * 40.;
        let base = 2. * a_tubo * perna + placa + nervuras;
        let v = volume(&r);
        assert!(v > base * 0.99 && v < base * 1.03, "Ro = {ro}: volume {v} contra {base}");
        let (lo, hi) = caixa(&r);
        assert!(perto(hi[0], 320., 0.3) && perto(lo[1], -20., 0.3), "Ro = {ro}: {lo:?} {hi:?}");
    }
}
