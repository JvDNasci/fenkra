// SPDX-License-Identifier: LGPL-2.1-or-later
//! ADR 0107, pendências das ondas 1 e 2 (B561, B562): a Extrusão de superfície e a Varredura de
//! superfície recebem o perfil como lista de curvas de origens mistas (`curve` + `mais`), e a junção na
//! hora gravada no campo `g_pf` vale no cálculo: o grupo unido vai ao kernel como uma cadeia só.
//!
//! Aceite: duas curvas tangentes (reta + arco) unidas na hora numa extrusão de superfície saem como
//! **uma** superfície contínua. Documentos montados em código, recalculados, conferindo status, faces,
//! bordas livres, área e caixa envolvente.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_kernel::Shape;
use fenkra_solidos::{recompute, surface_brep, RecomputeResult, Status};
use std::f64::consts::PI;

fn s(x: &str) -> String {
    x.into()
}

fn ok(r: &RecomputeResult, d: &Document, id: Id) {
    assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{}: {:?}", d.name_of(id), r.status.get(&id));
}

fn superficie(d: &Document, id: Id) -> Shape {
    Shape::from_brep(&surface_brep(d, id).expect("superfície avaliada")).expect("B-rep legível")
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn ponto(p: [f64; 3]) -> PointRef {
    PointRef::Fixed(p)
}

fn reta(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: ponto(a), b: ponto(b), start: s("0"), end: s("0") }))
}

/// Perfil tangente no plano XY: reta de (-20,5) a (0,5) e arco de raio 10, centro (0,15), de -90° a
/// 0°. A reta sai em +X e o arco começa em +X (G1). Comprimento total 20 + 5π.
fn perfil_tangente(d: &mut Document) -> (CurveRef, CurveRef) {
    let l = reta(d, [-20., 5., 0.], [0., 5., 0.]);
    let a = CurveRef::Ref(d.add_geometry(RefKind::Circle { center: ponto([0., 15., 0.]), plane: PlaneRef::Origin(OriginPlane::XY), radius: s("10"), start: s("-90"), end: s("0") }));
    (l, a)
}

fn unir(d: &mut Document, dono: Id, campo: &str, itens: &[usize]) {
    let mut c = DadosCampo::novo(dono, campo);
    c.juncoes = vec![itens.to_vec()];
    d.set_dados_campo(c);
}

const COMPRIMENTO: f64 = 20. + 5. * PI;

fn extrusao(curve: CurveRef, mais: Vec<CurveRef>, l1: &str, l2: &str) -> RefKind {
    RefKind::SurfExtrude { curve, dir: AxisRef::Origin(OriginAxis::Z), length: s(l1), length2: s(l2), until: None, until2: None, mais }
}

// ------------------------------------------------------------------ extrusão de superfície

/// Aceite do usuário: reta + arco tangentes, unidos na hora, extrudados 30 mm em Z dão **uma** superfície
/// contínua: duas faces (plano e cilindro) costuradas pela aresta comum, um domínio só e 6 bordas livres
/// (as duas curvas embaixo e em cima e as duas retas das pontas). Sem a junção são duas superfícies
/// soltas (dois domínios, 8 bordas livres). A área é comprimento × altura nos dois casos.
#[test]
fn extrusao_de_reta_e_arco_unidos_e_uma_superficie_continua() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("extrusao_unida");
    let (l, a) = perfil_tangente(&mut d);
    let ex = d.add_geometry(extrusao(l, vec![a], "30", "0"));

    let r = recompute(&d);
    ok(&r, &d, ex);
    let solta = superficie(&d, ex);
    let rep = solta.sew_report().unwrap();
    assert_eq!(solta.face_count(), 2);
    assert_eq!(rep.domains, 2, "sem junção, cada curva é extrudada sozinha: duas superfícies");
    assert_eq!(rep.free_edges, 8, "duas faixas soltas, quatro bordas cada");

    unir(&mut d, ex, "g_pf", &[0, 1]);
    let r = recompute(&d);
    ok(&r, &d, ex);
    let unida = superficie(&d, ex);
    let rep = unida.sew_report().unwrap();
    assert_eq!(unida.face_count(), 2, "um trecho exato por face: reta (plano) e arco (cilindro)");
    assert_eq!(rep.domains, 1, "com a junção, uma superfície contínua");
    assert_eq!(rep.free_edges, 6, "a aresta comum deixou de ser borda livre");
    let area = unida.mass().area;
    assert!(perto(area, COMPRIMENTO * 30., 0.05), "área {area} != {}", COMPRIMENTO * 30.);
    assert!(perto(area, solta.mass().area, 1e-6), "a junção muda a ligação, não as faces");
    let (mn, mx) = unida.bbox().unwrap();
    assert!(perto(mn[0], -20., 1e-3) && perto(mx[0], 10., 1e-3), "caixa em X {mn:?} {mx:?}");
    assert!(perto(mn[1], 5., 1e-3) && mx[1] >= 15. - 1e-3 && mx[1] <= 15. + 0.5, "caixa em Y {mn:?} {mx:?}");
    assert!(perto(mn[2], 0., 1e-3) && perto(mx[2], 30., 1e-3), "caixa em Z {mn:?} {mx:?}");
    let nota = r.notes.get(&ex).map(|n| n.text.clone()).unwrap_or_default();
    assert!(nota.contains("G1"), "a nota diz a continuidade da junção: {nota}");
}

/// O perfil em lista anda junto com os dois limites e com o inverter (sinal dos limites): a faixa unida
/// vai de -10 a 25 e, invertida, de -25 a 10, sempre um domínio só.
#[test]
fn extrusao_unida_com_dois_limites_e_invertida() {
    if !kernel_disponivel() {
        return;
    }
    for (l1, l2, z0, z1) in [("25", "10", -10., 25.), ("-25", "-10", -25., 10.)] {
        let mut d = Document::new("limites");
        let (l, a) = perfil_tangente(&mut d);
        let ex = d.add_geometry(extrusao(l, vec![a], l1, l2));
        unir(&mut d, ex, "g_pf", &[0, 1]);
        let r = recompute(&d);
        ok(&r, &d, ex);
        let sh = superficie(&d, ex);
        assert_eq!(sh.sew_report().unwrap().domains, 1, "{l1}/{l2}");
        let (mn, mx) = sh.bbox().unwrap();
        assert!(perto(mn[2], z0, 1e-3) && perto(mx[2], z1, 1e-3), "{l1}/{l2}: {mn:?} {mx:?}");
        assert!(perto(sh.mass().area, COMPRIMENTO * 35., 0.05));
    }
}

/// Arquivo antigo: a extrusão sem `mais` grava o mesmo JSON de antes (o campo vazio não sai) e lê de
/// volta igual; o resultado é a faixa de uma curva só, com a mesma área e caixa.
#[test]
fn extrusao_antiga_abre_igual() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("antigo");
    let l = reta(&mut d, [0., 0., 0.], [40., 0., 0.]);
    let k = extrusao(l, vec![], "30", "0");
    let json = serde_json::to_string(&k).unwrap();
    assert!(!json.contains("mais"), "o campo vazio não entra no arquivo: {json}");
    let lido: RefKind = serde_json::from_str(&json).unwrap();
    assert_eq!(lido, k);
    let ex = d.add_geometry(lido);
    let r = recompute(&d);
    ok(&r, &d, ex);
    let sh = superficie(&d, ex);
    assert_eq!(sh.face_count(), 1);
    assert!(perto(sh.mass().area, 1200., 1e-6));
    let (mn, mx) = sh.bbox().unwrap();
    assert!(perto(mx[0] - mn[0], 40., 1e-3) && perto(mx[2] - mn[2], 30., 1e-3), "{mn:?} {mx:?}");
}

// ------------------------------------------------------------------ varredura de superfície

/// A varredura de superfície também lê o perfil em lista: reta + arco unidos, varridos ao longo de uma
/// guia reta em Z (60 mm), saem como uma superfície contínua, com a mesma área que a extrusão daria.
#[test]
fn varredura_com_perfil_unido_e_uma_superficie_continua() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("varredura");
    let (l, a) = perfil_tangente(&mut d);
    let guia = reta(&mut d, [-20., 5., 0.], [-20., 5., 60.]);
    let sw = d.add_geometry(RefKind::SurfSweep { profile: SweepProfile::Curve(l), spine: guia, mode: SweepMode::Corrigido, mais: vec![a] });

    let r = recompute(&d);
    ok(&r, &d, sw);
    assert_eq!(superficie(&d, sw).sew_report().unwrap().domains, 2, "sem junção, duas varreduras soltas");

    unir(&mut d, sw, "g_pf", &[0, 1]);
    let r = recompute(&d);
    ok(&r, &d, sw);
    let sh = superficie(&d, sw);
    let rep = sh.sew_report().unwrap();
    assert_eq!(rep.domains, 1, "perfil unido: uma superfície contínua");
    assert_eq!(sh.face_count(), 2);
    assert!(perto(sh.mass().area, COMPRIMENTO * 60., 0.5), "área {}", sh.mass().area);
    let (mn, mx) = sh.bbox().unwrap();
    assert!(perto(mn[2], 0., 1e-3) && perto(mx[2], 60., 1e-3) && perto(mx[0], 10., 1e-2), "{mn:?} {mx:?}");
}
