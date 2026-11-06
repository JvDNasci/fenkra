// SPDX-License-Identifier: LGPL-2.1-or-later
//! B34 e B290: multi-seções com guias e conectores, superfície regrada e análises, com o documento
//! montado em código e a conferência de status, área e caixa envolvente.

use fenkra_core::*;
use fenkra_solidos::analise::{Analise, Nivel};
use fenkra_solidos::{loft, recompute, RecomputeResult, Status};

fn t(x: &str) -> String {
    x.to_string()
}

fn fx(p: [f64; 3]) -> PointRef {
    PointRef::Fixed(p)
}

fn curva(d: &mut Document, pts: &[[f64; 3]]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::Spline { points: pts.iter().map(|p| fx(*p)).collect(), closed: false }))
}

fn reta(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: fx(a), b: fx(b), start: t("0"), end: t("0") }))
}

fn ok(r: &RecomputeResult, id: Id) {
    assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{:?}", r.status.get(&id));
}

fn caixa(r: &RecomputeResult, id: Id) -> ([f32; 3], [f32; 3]) {
    let m = &r.surfaces.iter().find(|s| s.id == id).expect("superfície").mesh;
    let (mut a, mut b) = ([f32::MAX; 3], [f32::MIN; 3]);
    for q in m.positions.chunks_exact(3) {
        for i in 0..3 {
            a[i] = a[i].min(q[i]);
            b[i] = b[i].max(q[i]);
        }
    }
    (a, b)
}

/// Área pela malha: basta para comparar com a conta de cada teste.
fn area(r: &RecomputeResult, id: Id) -> f64 {
    let m = &r.surfaces.iter().find(|s| s.id == id).expect("superfície").mesh;
    use fenkra_solidos::geo3d::{cross, norm, sub};
    let p = |i: u32| [0, 1, 2].map(|k| m.positions[i as usize * 3 + k] as f64);
    m.indices.chunks_exact(3).map(|c| norm(cross(sub(p(c[1]), p(c[0])), sub(p(c[2]), p(c[0])))) / 2.).sum()
}

/// Distância de `q` até a malha da superfície.
fn longe(r: &RecomputeResult, id: Id, q: [f32; 3]) -> f32 {
    let m = &r.surfaces.iter().find(|s| s.id == id).expect("superfície").mesh;
    m.positions.chunks_exact(3).map(|p| ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()).fold(f32::MAX, f32::min)
}

/// Duas seções retas no X, em z = 0 e z = 60, e duas guias em arco que as cruzam nas pontas.
fn com_guias(d: &mut Document, guias: usize) -> (Id, Vec<CurveRef>) {
    let s1 = reta(d, [0., 0., 0.], [40., 0., 0.]);
    let s2 = reta(d, [0., 0., 60.], [40., 0., 60.]);
    let g1 = curva(d, &[[0., 0., 0.], [0., 15., 30.], [0., 0., 60.]]);
    let g2 = curva(d, &[[40., 0., 0.], [40., -10., 30.], [40., 0., 60.]]);
    let g: Vec<CurveRef> = [g1, g2].into_iter().take(guias).collect();
    let opts = LoftOpts { guides: g.clone(), ..Default::default() };
    (d.add_geometry(RefKind::SurfLoft { sections: vec![s1, s2], guide: None, ruled: false, closed: false, opts }), g)
}

#[test]
fn loft_toca_as_duas_guias_e_as_secoes() {
    let mut d = Document::new("guias");
    let (id, _) = com_guias(&mut d, 2);
    let r = recompute(&d);
    ok(&r, id);
    // as guias cruzam y = 15 e y = -10 no meio: a superfície passa por lá, sem fugir das seções
    assert!(longe(&r, id, [0., 15., 30.]) < 1.5, "{}", longe(&r, id, [0., 15., 30.]));
    assert!(longe(&r, id, [40., -10., 30.]) < 1.5, "{}", longe(&r, id, [40., -10., 30.]));
    let (a, b) = caixa(&r, id);
    assert!((a[0] - 0.).abs() < 0.2 && (b[0] - 40.).abs() < 0.2, "{a:?} {b:?}");
    assert!((a[2] - 0.).abs() < 0.05 && (b[2] - 60.).abs() < 0.05, "{a:?} {b:?}");
    assert!(b[1] > 14. && a[1] < -9., "{a:?} {b:?}");
    // sem as guias seria um retângulo de 2400 mm²; com elas ela estica
    assert!(area(&r, id) > 2450., "{}", area(&r, id));
}

#[test]
fn loft_com_uma_guia_desloca_as_secoes_junto() {
    let mut d = Document::new("guia");
    let (id, _) = com_guias(&mut d, 1);
    let r = recompute(&d);
    ok(&r, id);
    assert!(longe(&r, id, [0., 15., 30.]) < 1.5);
    // sem a segunda guia, a outra ponta anda o mesmo tanto
    assert!(longe(&r, id, [40., 15., 30.]) < 1.5);
}

#[test]
fn loft_com_tres_guias_e_tres_secoes() {
    let mut d = Document::new("grade");
    let sec = |d: &mut Document, z: f64, y: f64| curva(d, &[[0., y, z], [20., y + 4., z], [40., y, z]]);
    let (s1, s2, s3) = (sec(&mut d, 0., 0.), sec(&mut d, 30., 5.), sec(&mut d, 60., 0.));
    let g1 = curva(&mut d, &[[0., 0., 0.], [0., 5., 30.], [0., 0., 60.]]);
    let g2 = curva(&mut d, &[[20., 4., 0.], [20., 9., 30.], [20., 4., 60.]]);
    let g3 = curva(&mut d, &[[40., 0., 0.], [40., 5., 30.], [40., 0., 60.]]);
    let opts = LoftOpts { guides: vec![g1, g2, g3], ..Default::default() };
    let id = d.add_geometry(RefKind::SurfLoft { sections: vec![s1, s2, s3], guide: None, ruled: false, closed: false, opts });
    let r = recompute(&d);
    ok(&r, id);
    assert!(longe(&r, id, [20., 9., 30.]) < 1.2, "{}", longe(&r, id, [20., 9., 30.]));
}

#[test]
fn guia_que_nao_toca_a_secao_vira_erro_com_motivo() {
    let mut d = Document::new("fora");
    let s1 = reta(&mut d, [0., 0., 0.], [40., 0., 0.]);
    let s2 = reta(&mut d, [0., 0., 60.], [40., 0., 60.]);
    let g = reta(&mut d, [0., 20., 0.], [0., 20., 60.]);
    let opts = LoftOpts { guides: vec![g], ..Default::default() };
    let id = d.add_geometry(RefKind::SurfLoft { sections: vec![s1, s2], guide: None, ruled: false, closed: false, opts });
    let r = recompute(&d);
    assert!(matches!(r.status.get(&id), Some(Status::Error(m)) if m.contains("não toca")), "{:?}", r.status.get(&id));
}

/// Folha plana 40 × 40 em z = 0, do lado y negativo.
fn folha(d: &mut Document, y0: f64, y1: f64, z: f64) -> Id {
    let c = reta(d, [0., y0, z], [0., y1, z]);
    let eixo = d.add_geometry(RefKind::LinePoints { a: fx([0.; 3]), b: fx([40., 0., 0.]), start: t("0"), end: t("0") });
    d.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Ref(eixo), length: t("40"), length2: t("0"), until: None, until2: None })
}

#[test]
fn regrada_nas_tres_direcoes() {
    let mut d = Document::new("regrada");
    let f = folha(&mut d, -40., 0., 0.);
    let borda = reta(&mut d, [0., 0., 0.], [40., 0., 0.]);
    let faixa = |d: &mut Document, mode, dir, flip| {
        d.add_geometry(RefKind::SurfRuled { curves: vec![borda.clone()], support: Some(SurfRef::Ref(f)), mode, dir, length: t("10"), flip })
    };
    let normal = faixa(&mut d, RuledMode::Normal, AxisRef::Origin(OriginAxis::Z), false);
    let tang = faixa(&mut d, RuledMode::Tangente, AxisRef::Origin(OriginAxis::Z), false);
    let vetor = faixa(&mut d, RuledMode::Vetor, AxisRef::Origin(OriginAxis::Y), true);
    let r = recompute(&d);
    for id in [normal, tang, vetor] {
        ok(&r, id);
        assert!((area(&r, id) - 400.).abs() < 0.5, "{}", area(&r, id));
    }
    let (a, b) = caixa(&r, normal);
    assert!((b[2] - a[2] - 10.).abs() < 1e-3 && (b[1] - a[1]).abs() < 1e-3, "{a:?} {b:?}");
    // a tangente continua a folha para fora da borda
    let (a, b) = caixa(&r, tang);
    assert!((a[1] - 0.).abs() < 1e-3 && (b[1] - 10.).abs() < 1e-3, "{a:?} {b:?}");
    let (a, b) = caixa(&r, vetor);
    assert!((a[1] + 10.).abs() < 1e-3 && b[1].abs() < 1e-3, "{a:?} {b:?}");
}

fn analise(r: &RecomputeResult, id: Id) -> &Analise {
    r.notes.get(&id).and_then(|n| n.analise.as_ref()).expect("nota da análise")
}

fn check(d: &mut Document, mode: CheckMode, a: SurfRef, b: SurfRef, curve: CurveRef, samples: &str, scale: &str) -> Id {
    d.add_geometry(RefKind::SurfCheck { mode, a, b, curve, samples: t(samples), scale: t(scale) })
}

#[test]
fn conexao_mede_folga_e_angulo() {
    let mut d = Document::new("conexao");
    let a = folha(&mut d, -40., 0., 0.);
    // a segunda começa 0,5 mm adiante e 3 mm acima, inclinada não: só folga
    let b = folha(&mut d, 0.5, 40., 0.);
    let id = check(&mut d, CheckMode::Conexao, SurfRef::Ref(a), SurfRef::Ref(b), CurveRef::default(), "8", "2");
    let r = recompute(&d);
    ok(&r, id);
    let Analise::Conexao(c) = analise(&r, id) else { panic!("conexão") };
    assert!((c.folga - 0.5).abs() < 1e-6, "{}", c.folga);
    assert!(c.angulo < 1e-6, "{}", c.angulo);
    assert_eq!(c.nivel(), Nivel::Aberta);
    assert!(c.amostras.iter().all(|s| s.ponto[1].abs() < 1e-9), "só a borda que olha para a outra entra");
    assert_eq!(c.amostras.len(), 8);
}

#[test]
fn conexao_em_angulo_conhecido() {
    let mut d = Document::new("dobra");
    let a = folha(&mut d, -40., 0., 0.);
    // sobe 30° a partir da borda
    let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    let perfil = reta(&mut d, [0., 0., 0.], [0., 40. * c, 40. * s]);
    let eixo = d.add_geometry(RefKind::LinePoints { a: fx([0.; 3]), b: fx([40., 0., 0.]), start: t("0"), end: t("0") });
    let b = d.add_geometry(RefKind::SurfExtrude { curve: perfil, dir: AxisRef::Ref(eixo), length: t("40"), length2: t("0"), until: None, until2: None });
    let id = check(&mut d, CheckMode::Conexao, SurfRef::Ref(a), SurfRef::Ref(b), CurveRef::default(), "6", "0.1");
    let r = recompute(&d);
    ok(&r, id);
    let Analise::Conexao(x) = analise(&r, id) else { panic!("conexão") };
    assert!((x.angulo - 30.).abs() < 1e-6, "{}", x.angulo);
    assert!(x.folga < 1e-6);
    assert_eq!(x.nivel(), Nivel::G0);
}

#[test]
fn conexao_mede_salto_de_curvatura_entre_plano_e_cilindro() {
    let mut d = Document::new("salto");
    let cil = d.add_geometry(RefKind::SurfCylinder { point: fx([0., 0., 20.]), dir: AxisRef::Origin(OriginAxis::X), radius: t("20"), start: t("0"), end: t("40") });
    // plano tangente ao cilindro no ponto mais baixo: a junção é uma geratriz, não uma borda; corta-se a
    // folha na linha e mede-se a partir das bordas da folha
    let plano = folha(&mut d, -40., 0., 0.);
    let id = check(&mut d, CheckMode::Conexao, SurfRef::Ref(plano), SurfRef::Ref(cil), CurveRef::default(), "6", "0.01");
    let r = recompute(&d);
    ok(&r, id);
    let Analise::Conexao(x) = analise(&r, id) else { panic!("conexão") };
    assert!(x.folga < 1e-6, "{}", x.folga);
    assert!(x.angulo < 0.01, "{}", x.angulo);
    assert!((x.dif_curvatura - 1. / 20.).abs() < 1e-4, "{}", x.dif_curvatura);
    assert_eq!(x.nivel(), Nivel::G1);
}

#[test]
fn desvio_entre_folhas_paralelas_e_a_distancia() {
    let mut d = Document::new("desvio");
    let a = folha(&mut d, -20., 20., 0.);
    let b = folha(&mut d, -20., 20., 3.);
    let id = check(&mut d, CheckMode::Desvio, SurfRef::Ref(a), SurfRef::Ref(b), CurveRef::default(), "2", "1");
    let r = recompute(&d);
    ok(&r, id);
    let Analise::Desvio(m) = analise(&r, id) else { panic!("desvio") };
    assert!((m.max.abs() - 3.).abs() < 1e-6 && (m.min.abs() - 3.).abs() < 1e-6, "{} {}", m.min, m.max);
    assert!((m.rms - 3.).abs() < 1e-6);
    assert_eq!(m.valores.len(), m.malha.positions.len() / 3);
}

#[test]
fn curvatura_da_malha_nao_depende_da_malha() {
    // a mesma superfície, em duas qualidades: o raio medido é o mesmo
    let mut d = Document::new("curvatura");
    let id = d.add_geometry(RefKind::SurfCylinder { point: fx([0.; 3]), dir: AxisRef::Origin(OriginAxis::Z), radius: t("25"), start: t("0"), end: t("30") });
    let r = recompute(&d);
    ok(&r, id);
    let m = &r.surfaces.iter().find(|s| s.id == id).unwrap().mesh;
    assert_eq!(m.curvature.len(), m.positions.len() / 3 * 2);
    for k in m.curvature.chunks_exact(2) {
        let (mx, mn) = (k[0].abs().max(k[1].abs()), k[0].abs().min(k[1].abs()));
        assert!((mx - 1. / 25.).abs() < 1e-5 && mn < 1e-5, "{k:?}");
    }
}

#[test]
fn pente_pela_arvore() {
    let mut d = Document::new("pente");
    let c = d.add_geometry(RefKind::Circle { center: fx([0.; 3]), plane: PlaneRef::Origin(OriginPlane::XY), radius: t("10"), start: t("0"), end: t("360") });
    let id = check(&mut d, CheckMode::Pente, SurfRef::default(), SurfRef::default(), CurveRef::Ref(c), "32", "50");
    let r = recompute(&d);
    ok(&r, id);
    let Analise::Pente(p) = analise(&r, id) else { panic!("pente") };
    assert_eq!(p.len(), 33);
    assert!(p.iter().all(|x| (x.k - 0.1).abs() < 2e-3), "{:?}", p[0]);
}

#[test]
fn loft_nao_muda_sem_opcoes() {
    // as opções vazias seguem pelo caminho do kernel
    assert!(loft::setas_de_tangencia(&[]).is_none());
    let mut d = Document::new("simples");
    let a = reta(&mut d, [0., 0., 0.], [40., 0., 0.]);
    let b = reta(&mut d, [0., 0., 30.], [40., 0., 30.]);
    let id = d.add_geometry(RefKind::SurfLoft { sections: vec![a, b], guide: None, ruled: true, closed: false, opts: Default::default() });
    let r = recompute(&d);
    ok(&r, id);
    assert!((area(&r, id) - 1200.).abs() < 0.1);
}
