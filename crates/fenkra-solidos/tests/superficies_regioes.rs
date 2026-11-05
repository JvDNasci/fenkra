// SPDX-License-Identifier: LGPL-2.1-or-later
//! B277: Aparar e dividir por região, com o documento montado em código, recalculado e conferido por
//! status, área e caixa envolvente.

use fenkra_core::*;
use fenkra_kernel::Shape;
use fenkra_solidos::regioes::RegiaoNota;
use fenkra_solidos::{recompute, surface_brep, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.into()
}

fn ok(r: &RecomputeResult, id: Id) {
    assert!(matches!(r.status[&id], Status::Ok), "{:?}", r.status[&id]);
}

fn forma(d: &Document, id: Id) -> Shape {
    Shape::from_brep(&surface_brep(d, id).expect("superfície")).unwrap()
}

fn area(d: &Document, id: Id) -> f64 {
    forma(d, id).mass().area
}

fn perto(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-3, "{a} != {b}");
}

fn caixa(d: &Document, id: Id) -> ([f64; 3], [f64; 3]) {
    forma(d, id).bbox().expect("caixa")
}

fn reta(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }))
}

/// Folha vertical pela reta, com altura `h` para cima e 30 mm para baixo.
fn folha(d: &mut Document, a: [f64; 3], b: [f64; 3], h: &str) -> SurfRef {
    let c = reta(d, a, b);
    SurfRef::Ref(d.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(OriginAxis::Z), length: s(h), length2: s("30"), until: None, until2: None }))
}

fn corte(surface: SurfRef, tools: Vec<SurfRef>, mutual: bool, remove: bool) -> RefKind {
    RefKind::SurfRegions { surface, tools, curve: None, mutual, remove, extend: false, marks: vec![] }
}

fn regioes(r: &RecomputeResult, id: Id) -> &[RegiaoNota] {
    &r.notes[&id].regioes
}

/// Grava a escolha como o clique grava: a âncora vem do pedaço calculado.
fn clicar(d: &mut Document, g: &RegiaoNota, id: Id) {
    let rf = d.references.iter_mut().find(|x| x.id == id).unwrap();
    let RefKind::SurfRegions { marks, .. } = &mut rf.kind else { panic!("outro comando") };
    marks.push(g.marca());
}

fn pedaco<'a>(r: &'a RecomputeResult, id: Id, teste: impl Fn(&RegiaoNota) -> bool) -> &'a RegiaoNota {
    regioes(r, id).iter().find(|g| teste(g)).expect("pedaço")
}

/// Aceite: duas folhas cruzadas dão quatro regiões, a escolha persiste ao mudar uma cota e a área e a
/// caixa envolvente conferem.
#[test]
fn superficies_regioes_duas_folhas_cruzadas() {
    let mut d = Document::new("t");
    d.add_param("altura", "30", Unit::Comprimento).unwrap();
    let a = folha(&mut d, [-40., 0., 0.], [40., 0., 0.], "altura");
    let b = folha(&mut d, [0., -40., 0.], [0., 40., 0.], "altura");
    let id = d.add_geometry(corte(a, vec![b], true, true));
    let r = recompute(&d);
    ok(&r, id);
    assert_eq!(regioes(&r, id).len(), 4);
    for g in regioes(&r, id) {
        perto(g.area, 40. * 60.);
        assert!(g.fica, "sem clique, tudo fica");
    }
    perto(area(&d, id), 4. * 40. * 60.);

    // sai a metade -X da folha A: a caixa encolhe em X e a área cai um quarto
    let g = pedaco(&r, id, |g| g.fonte == 0 && g.ponto[0] < 0.).clone();
    clicar(&mut d, &g, id);
    let r = recompute(&d);
    ok(&r, id);
    assert_eq!(regioes(&r, id).iter().filter(|g| !g.fica).count(), 1);
    perto(area(&d, id), 3. * 40. * 60.);
    let (lo, hi) = caixa(&d, id);
    perto(lo[0], 0.);
    perto(hi[0], 40.);
    perto(lo[1], -40.);
    perto(hi[1], 40.);

    // sai também a metade +Y da folha B
    let g = pedaco(&r, id, |g| g.fonte == 1 && g.ponto[1] > 0.).clone();
    clicar(&mut d, &g, id);
    let r = recompute(&d);
    ok(&r, id);
    perto(area(&d, id), 2. * 40. * 60.);
    let (lo, hi) = caixa(&d, id);
    perto(hi[1], 0.);
    perto(lo[1], -40.);

    // a cota muda e a escolha fica: dois pedaços saem, os mesmos, com a nova altura
    d.params.iter_mut().find(|p| p.name == "altura").unwrap().expr = s("50");
    let r = recompute(&d);
    ok(&r, id);
    assert_eq!(regioes(&r, id).iter().filter(|g| !g.fica).count(), 2);
    let saem: Vec<_> = regioes(&r, id).iter().filter(|g| !g.fica).map(|g| g.fonte).collect();
    assert!(saem.contains(&0) && saem.contains(&1), "{saem:?}");
    perto(area(&d, id), 2. * 40. * 80.);
    let (lo, hi) = caixa(&d, id);
    perto(lo[0], 0.);
    perto(hi[0], 40.);
    perto(lo[1], -40.);
    perto(hi[1], 0.);
    perto(hi[2], 50.);
    perto(lo[2], -30.);
}

/// A cota faz a folha passar da ferramenta: ela fica com um rasgo e deixa de ter duas metades. A marca
/// da metade não pode apagar a folha inteira.
#[test]
fn superficies_regioes_folha_que_passa_da_ferramenta_nao_some() {
    let mut d = Document::new("t");
    d.add_param("altura", "30", Unit::Comprimento).unwrap();
    let a = folha(&mut d, [-40., 0., 0.], [40., 0., 0.], "altura");
    let b = folha(&mut d, [0., -40., 0.], [0., 40., 0.], "30");
    let id = d.add_geometry(corte(a, vec![b], true, true));
    let r = recompute(&d);
    let g = pedaco(&r, id, |g| g.fonte == 0 && g.ponto[0] < 0.).clone();
    clicar(&mut d, &g, id);
    recompute(&d);
    perto(area(&d, id), 3. * 40. * 60.);
    d.params.iter_mut().find(|p| p.name == "altura").unwrap().expr = s("50");
    let r = recompute(&d);
    assert_eq!(regioes(&r, id).len(), 3, "a folha A não se separa mais");
    assert!(matches!(&r.status[&id], Status::Warning(w) if w.contains("escolha")), "{:?}", r.status[&id]);
    perto(area(&d, id), 80. * 80. + 2. * 40. * 60.);
}

/// A ferramenta anda para além do ponto guardado: a escolha segue o lado, não o ponto.
#[test]
fn superficies_regioes_escolha_segue_o_lado_quando_o_corte_anda() {
    let mut d = Document::new("t");
    let a = folha(&mut d, [-40., 0., 0.], [40., 0., 0.], "30");
    let plano = d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::YZ), offset: s("10") });
    let id = d.add_geometry(corte(a, vec![SurfRef::Plane(PlaneRef::Datum(plano))], false, true));
    let r = recompute(&d);
    ok(&r, id);
    assert_eq!(regioes(&r, id).len(), 2);
    let g = pedaco(&r, id, |g| g.ponto[0] > 10.).clone();
    assert!(g.ponto[0] < 30., "a âncora nasce no meio do pedaço: {:?}", g.ponto);
    clicar(&mut d, &g, id);
    let r = recompute(&d);
    ok(&r, id);
    let (lo, hi) = caixa(&d, id);
    perto(lo[0], -40.);
    perto(hi[0], 10.);

    // o corte passa por cima da âncora: ela cai no pedaço de -X, mas quem saiu foi o de +X
    let rf = d.references.iter_mut().find(|x| x.id == plano).unwrap();
    let RefKind::OffsetPlane { offset, .. } = &mut rf.kind else { panic!("plano") };
    *offset = s("35");
    let r = recompute(&d);
    ok(&r, id);
    let (lo, hi) = caixa(&d, id);
    perto(lo[0], -40.);
    perto(hi[0], 35.);
    perto(area(&d, id), 75. * 60.);
}

/// A ordem dos pedaços não é a escolha: manter o clicado vale para qualquer ordem do kernel.
#[test]
fn superficies_regioes_manter_so_os_clicados() {
    let mut d = Document::new("t");
    let a = folha(&mut d, [-40., 0., 0.], [40., 0., 0.], "30");
    let id = d.add_geometry(corte(a, vec![SurfRef::Plane(PlaneRef::Origin(OriginPlane::YZ))], false, false));
    let r = recompute(&d);
    assert!(matches!(&r.status[&id], Status::Error(e) if e.contains("clique")), "{:?}", r.status[&id]);
    assert_eq!(regioes(&r, id).len(), 2, "os pedaços aparecem mesmo sem nada escolhido");
    let g = pedaco(&r, id, |g| g.ponto[0] > 0.).clone();
    clicar(&mut d, &g, id);
    let r = recompute(&d);
    ok(&r, id);
    let (lo, hi) = caixa(&d, id);
    perto(lo[0], 0.);
    perto(hi[0], 40.);
    perto(area(&d, id), 40. * 60.);
}

/// Folha dobrada: duas faces antes do corte, mas o corte só separa dois pedaços, e não quatro.
#[test]
fn superficies_regioes_folha_dobrada_continua_inteira_de_cada_lado() {
    let mut d = Document::new("t");
    let a = folha(&mut d, [-40., 0., 0.], [0., 0., 0.], "30");
    let b = folha(&mut d, [0., 0., 0.], [0., 40., 0.], "30");
    let dobrada = SurfRef::Ref(d.add_geometry(RefKind::SurfJoin { items: vec![a, b], tol: s("0.001"), curves: vec![] }));
    let id = d.add_geometry(corte(dobrada, vec![SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY))], false, true));
    let r = recompute(&d);
    ok(&r, id);
    assert_eq!(regioes(&r, id).len(), 2, "acima e abaixo do plano");
    for g in regioes(&r, id) {
        perto(g.area, 80. * 30.);
    }
}

/// Três cortes em cruz em uma folha: as ferramentas que se cruzam vão ao corte separadas.
#[test]
fn superficies_regioes_duas_ferramentas_que_se_cruzam() {
    let mut d = Document::new("t");
    let a = folha(&mut d, [-40., 0., 0.], [40., 0., 0.], "30");
    let id = d.add_geometry(corte(a, vec![SurfRef::Plane(PlaneRef::Origin(OriginPlane::YZ)), SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY))], false, true));
    let r = recompute(&d);
    ok(&r, id);
    assert_eq!(regioes(&r, id).len(), 4);
    perto(regioes(&r, id).iter().map(|g| g.area).sum(), 80. * 60.);
    let g = pedaco(&r, id, |g| g.ponto[0] > 0. && g.ponto[2] > 0.).clone();
    clicar(&mut d, &g, id);
    let r = recompute(&d);
    ok(&r, id);
    perto(area(&d, id), 3. * 40. * 30.);
}

/// O pedaço marcado sumiu: a escolha não vai para outro pedaço, e a tela avisa.
#[test]
fn superficies_regioes_pedaco_que_sumiu_avisa() {
    let mut d = Document::new("t");
    let a = folha(&mut d, [-40., 0., 0.], [40., 0., 0.], "30");
    let p1 = d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::YZ), offset: s("0") });
    let p2 = d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::YZ), offset: s("20") });
    let id = d.add_geometry(corte(a, vec![SurfRef::Plane(PlaneRef::Datum(p1)), SurfRef::Plane(PlaneRef::Datum(p2))], false, true));
    let r = recompute(&d);
    ok(&r, id);
    assert_eq!(regioes(&r, id).len(), 3);
    let g = pedaco(&r, id, |g| g.ponto[0] > 20.).clone();
    clicar(&mut d, &g, id);
    recompute(&d);
    perto(area(&d, id), 60. * 60.);
    // o segundo corte sai da folha: já não há pedaço depois dele
    let rf = d.references.iter_mut().find(|x| x.id == p2).unwrap();
    let RefKind::OffsetPlane { offset, .. } = &mut rf.kind else { panic!("plano") };
    *offset = s("100");
    let r = recompute(&d);
    assert!(matches!(&r.status[&id], Status::Warning(w) if w.contains("escolha")), "{:?}", r.status[&id]);
    perto(area(&d, id), 80. * 60.);
}

/// Sem cruzar nada não há o que escolher.
#[test]
fn superficies_regioes_ferramenta_longe_e_erro() {
    let mut d = Document::new("t");
    let a = folha(&mut d, [-40., 0., 0.], [40., 0., 0.], "30");
    let longe = d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::YZ), offset: s("500") });
    let id = d.add_geometry(corte(a, vec![SurfRef::Plane(PlaneRef::Datum(longe))], false, true));
    let r = recompute(&d);
    assert!(matches!(&r.status[&id], Status::Error(e) if e.contains("atravessa")), "{:?}", r.status[&id]);
}

/// Ferramenta curta: sem prolongar não separa; prolongada, corta a folha inteira.
#[test]
fn superficies_regioes_prolongar_a_ferramenta() {
    let mut d = Document::new("t");
    let a = folha(&mut d, [-40., 0., 0.], [40., 0., 0.], "30");
    let c = reta(&mut d, [0., -10., 0.], [0., 10., 0.]);
    let curta = SurfRef::Ref(d.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(OriginAxis::Z), length: s("5"), length2: s("0"), until: None, until2: None }));
    let id = d.add_geometry(RefKind::SurfRegions { surface: a, tools: vec![curta], curve: None, mutual: false, remove: true, extend: true, marks: vec![] });
    let r = recompute(&d);
    ok(&r, id);
    assert_eq!(regioes(&r, id).len(), 2);
    perto(regioes(&r, id)[0].area, 40. * 60.);
}
