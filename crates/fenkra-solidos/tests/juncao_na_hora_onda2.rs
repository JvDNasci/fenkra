// SPDX-License-Identifier: LGPL-2.1-or-later
//! ADR 0107, onda 2 (B561, B562): a junção na hora gravada no campo do comando (`Document::campos`)
//! tem efeito geométrico. O cálculo junta os itens de cada grupo numa cadeia exata antes do kernel
//! (trechos encadeados, G0/G1), tolera índices fora da lista e avisa quando as pontas não se tocam. A
//! extrusão de superfície inverte pelo sinal dos limites, como a extrusão sólida faz com `reverse`.
//!
//! Documentos montados em código, recalculados, conferindo status, faces, área e caixa envolvente.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_kernel::Shape;
use fenkra_solidos::{recompute, recompute_cached, surface_brep, Cache, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.into()
}

fn ok(r: &RecomputeResult, d: &Document, id: Id) {
    assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{}: {:?}", d.name_of(id), r.status.get(&id));
}

fn superficie(d: &Document, id: Id) -> Shape {
    Shape::from_brep(&surface_brep(d, id).expect("superfície avaliada")).expect("B-rep legível")
}

fn caixa(sh: &Shape) -> ([f64; 3], [f64; 3]) {
    sh.bbox().expect("caixa envolvente")
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

/// Arco no plano XY, centro `c`, raio `r`, de `a0` a `a1` graus.
fn arco(d: &mut Document, c: [f64; 3], r: &str, a0: &str, a1: &str) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::Circle { center: ponto(c), plane: PlaneRef::Origin(OriginPlane::XY), radius: s(r), start: s(a0), end: s(a1) }))
}

fn unir(d: &mut Document, dono: Id, campo: &str, itens: &[usize]) {
    let mut c = DadosCampo::novo(dono, campo);
    c.juncoes = vec![itens.to_vec()];
    d.set_dados_campo(c);
}

/// Perfil tangente: reta de (-20,5) a (0,5) e arco de raio 10 com centro (0,15), de -90° a 0°: a reta
/// sai na direção +X e o arco começa na mesma direção (G1).
fn perfil_tangente(d: &mut Document) -> (CurveRef, CurveRef) {
    let l = reta(d, [-20., 5., 0.], [0., 5., 0.]);
    let a = arco(d, [0., 15., 0.], "10", "-90", "0");
    (l, a)
}

// ------------------------------------------------------------------ revolução com junção

/// Aceite (revolução): duas curvas tangentes unidas na hora giram como **uma** superfície contínua (um
/// domínio, as duas faces costuradas pela aresta comum); sem a junção são duas superfícies soltas. A
/// área é a mesma nos dois casos (Pappus: as faces são as mesmas, só a ligação muda).
#[test]
fn revolucao_de_curvas_tangentes_unidas_e_uma_superficie_continua() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("revolucao");
    let (l, a) = perfil_tangente(&mut d);
    let rv = d.add_geometry(RefKind::SurfRevolve { curve: l, axis: AxisRef::Origin(OriginAxis::X), angle: s("90"), angle2: s("0"), mais: vec![a] });

    let r = recompute(&d);
    ok(&r, &d, rv);
    let solta = superficie(&d, rv);
    let rep = solta.sew_report().unwrap();
    assert_eq!(solta.face_count(), 2);
    assert_eq!(rep.domains, 2, "sem junção, cada curva gira sozinha: duas superfícies");

    unir(&mut d, rv, "g_pf", &[0, 1]);
    let r = recompute(&d);
    ok(&r, &d, rv);
    let unida = superficie(&d, rv);
    let rep = unida.sew_report().unwrap();
    assert_eq!(unida.face_count(), 2, "um trecho exato por face: reta (cilindro) e arco (toro)");
    assert_eq!(rep.domains, 1, "com a junção, uma superfície contínua");
    assert!(rep.free_edges < 8, "a aresta comum deixou de ser borda livre: {}", rep.free_edges);
    // área: cilindro r=5, 20 mm, 90° = 2π·5·20/4; o arco gira com Pappus (centroide do arco a
    // y = 15 - 20/π) e comprimento 5π
    let cil = std::f64::consts::PI * 5. * 20. / 2.;
    let toro = (std::f64::consts::PI / 2.) * (15. - 20. / std::f64::consts::PI) * (5. * std::f64::consts::PI);
    assert!(perto(unida.mass().area, cil + toro, 0.5), "área {} != {}", unida.mass().area, cil + toro);
    assert!(perto(unida.mass().area, solta.mass().area, 1e-6));
    let (mn, mx) = caixa(&unida);
    assert!(perto(mn[0], -20., 1e-3) && perto(mx[0], 10., 1e-3), "caixa em X {mn:?} {mx:?}");
    // a caixa do kernel em face de revolução é folgada (polos do toro): confere só a faixa
    assert!(mx[1] >= 15. - 1e-3 && mx[1] <= 25. + 1e-3, "o perfil vai até y=15 e o toro fica no raio 25: {mx:?}");
    // a nota do comando diz a continuidade da junção
    let nota = r.notes.get(&rv).map(|n| n.text.clone()).unwrap_or_default();
    assert!(nota.contains("G1"), "nota: {nota}");
}

/// A lista mudou fora da caixa (o item 5 não existe): o cálculo não quebra, ignora o índice e diz.
#[test]
fn indice_fora_da_lista_nao_quebra_o_calculo() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("fora");
    let (l, a) = perfil_tangente(&mut d);
    let rv = d.add_geometry(RefKind::SurfRevolve { curve: l, axis: AxisRef::Origin(OriginAxis::X), angle: s("90"), angle2: s("0"), mais: vec![a] });
    unir(&mut d, rv, "g_pf", &[1, 5]);
    let r = recompute(&d);
    ok(&r, &d, rv);
    assert_eq!(superficie(&d, rv).sew_report().unwrap().domains, 2, "o grupo desfez: as curvas giram sozinhas");
    let nota = r.notes.get(&rv).map(|n| n.text.clone()).unwrap_or_default();
    assert!(nota.contains("não existem mais"), "nota: {nota}");
}

/// Pontas que não se tocam na tolerância: aviso no comando, e os itens seguem separados (o resultado
/// existe, nada some em silêncio).
#[test]
fn pontas_afastadas_dao_aviso_e_os_itens_seguem() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("folga");
    let l = reta(&mut d, [-20., 5., 0.], [0., 5., 0.]);
    let m = reta(&mut d, [3., 5., 0.], [20., 5., 0.]);
    let rv = d.add_geometry(RefKind::SurfRevolve { curve: l, axis: AxisRef::Origin(OriginAxis::X), angle: s("90"), angle2: s("0"), mais: vec![m] });
    unir(&mut d, rv, "g_pf", &[0, 1]);
    let r = recompute(&d);
    assert!(matches!(r.status.get(&rv), Some(Status::Warning(_))), "a junção que não fecha deixa o comando em aviso: {:?}", r.status.get(&rv));
    let n = r.notes.get(&rv).expect("nota do comando");
    assert!(n.warn, "pontas afastadas são aviso");
    assert!(n.text.contains("não se tocam") && n.text.contains("3.000 mm"), "{}", n.text);
    assert_eq!(superficie(&d, rv).face_count(), 2);
}

/// Unir ou separar refaz o cálculo mesmo com o cache cheio: a junção mora fora do `kind` e entra na
/// assinatura do comando.
#[test]
fn unir_refaz_com_o_cache() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("cache");
    let (l, a) = perfil_tangente(&mut d);
    let rv = d.add_geometry(RefKind::SurfRevolve { curve: l, axis: AxisRef::Origin(OriginAxis::X), angle: s("90"), angle2: s("0"), mais: vec![a] });
    let mut cache = Cache::default();
    let r = recompute_cached(&d, &mut cache);
    ok(&r, &d, rv);
    let antes = r.notes.get(&rv).map(|n| n.text.clone()).unwrap_or_default();
    assert!(!antes.contains("junção"));
    unir(&mut d, rv, "g_pf", &[0, 1]);
    let r = recompute_cached(&d, &mut cache);
    let depois = r.notes.get(&rv).map(|n| n.text.clone()).unwrap_or_default();
    assert!(depois.contains("junção"), "o cache devolveu o resultado sem a junção: {depois}");
}

// ------------------------------------------------------------------ loft e regrada

/// Loft: itens unidos formam **uma** seção. Cada seção é feita de duas retas de 10 mm unidas na hora
/// (quatro itens, duas junções): o loft regrado é o retângulo 20 × 30. Sem as junções seriam quatro
/// seções, duas a duas no mesmo nível.
#[test]
fn loft_com_secao_unida_na_hora() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("loft");
    let b1 = reta(&mut d, [0., 0., 0.], [10., 0., 0.]);
    let b2 = reta(&mut d, [10., 0., 0.], [20., 0., 0.]);
    let c1 = reta(&mut d, [0., 0., 30.], [10., 0., 30.]);
    let c2 = reta(&mut d, [10., 0., 30.], [20., 0., 30.]);
    let lf = d.add_geometry(RefKind::SurfLoft { sections: vec![b1, b2, c1, c2], guide: None, ruled: true, closed: false, opts: Default::default() });
    let mut c = DadosCampo::novo(lf, "g_sec");
    c.juncoes = vec![vec![0, 1], vec![2, 3]];
    d.set_dados_campo(c);
    let r = recompute(&d);
    ok(&r, &d, lf);
    let sh = superficie(&d, lf);
    assert!(perto(sh.mass().area, 600., 0.5), "área {}", sh.mass().area);
    let (mn, mx) = caixa(&sh);
    assert!(perto(mn[2], 0., 1e-3) && perto(mx[2], 30., 1e-3) && perto(mx[0], 20., 1e-3), "{mn:?} {mx:?}");
}

/// Regrada: duas curvas unidas dão uma faixa só pela cadeia (mesma área das duas faixas juntas).
#[test]
fn regrada_com_curvas_unidas() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("regrada");
    let (l, a) = perfil_tangente(&mut d);
    let rg = d.add_geometry(RefKind::SurfRuled { curves: vec![l, a], support: None, mode: RuledMode::Vetor, dir: AxisRef::Origin(OriginAxis::Z), length: s("10"), flip: false });
    unir(&mut d, rg, "g_rc", &[0, 1]);
    let r = recompute(&d);
    ok(&r, &d, rg);
    let sh = superficie(&d, rg);
    let comprimento = 20. + 5. * std::f64::consts::PI;
    assert!(perto(sh.mass().area, comprimento * 10., 0.5), "área {} != {}", sh.mass().area, comprimento * 10.);
    assert_eq!(sh.sew_report().unwrap().domains, 1);
}

// ------------------------------------------------------------------ inverter da extrusão de superfície

fn extrusao(curve: CurveRef, l1: &str, l2: &str) -> RefKind {
    RefKind::SurfExtrude { curve, dir: AxisRef::Origin(OriginAxis::Z), length: s(l1), length2: s(l2), until: None, until2: None }
}

/// B562: inverter a extrusão de superfície troca o sinal dos dois limites; a faixa vira o espelho dela
/// pela curva (a caixa em Z vai de [-10, 30] para [-30, 10]), com a mesma área.
#[test]
fn extrusao_de_superficie_invertida_pelo_sinal() {
    if !kernel_disponivel() {
        return;
    }
    for (l1, l2, z0, z1) in [("30", "0", 0., 30.), ("30", "10", -10., 30.), ("0", "15", -15., 0.)] {
        let mut d = Document::new("inv");
        let c = reta(&mut d, [0., 0., 0.], [40., 0., 0.]);
        let id = d.add_geometry(extrusao(c.clone(), l1, l2));
        let r = recompute(&d);
        ok(&r, &d, id);
        let sh = superficie(&d, id);
        let (mn, mx) = caixa(&sh);
        assert!(perto(mn[2], z0, 1e-3) && perto(mx[2], z1, 1e-3), "{l1}/{l2}: {mn:?} {mx:?}");
        let area = sh.mass().area;
        // invertida: os dois limites com o sinal trocado
        let (n1, n2) = (fenkra_core::expr::negar(l1), fenkra_core::expr::negar(l2));
        assert!(fenkra_core::expr::sinal_negativo(&n1), "o estado invertido é o sinal do limite 1: {n1}");
        let mut d2 = Document::new("inv2");
        let c = reta(&mut d2, [0., 0., 0.], [40., 0., 0.]);
        let id = d2.add_geometry(extrusao(c, &n1, &n2));
        let r = recompute(&d2);
        ok(&r, &d2, id);
        let sh = superficie(&d2, id);
        let (mn, mx) = caixa(&sh);
        assert!(perto(mn[2], -z1, 1e-3) && perto(mx[2], -z0, 1e-3), "invertida {n1}/{n2}: {mn:?} {mx:?}");
        assert!(perto(sh.mass().area, area, 1e-6));
        // e trocar de novo volta ao texto de antes
        assert_eq!((fenkra_core::expr::negar(&n1), fenkra_core::expr::negar(&n2)), (l1.to_string(), l2.to_string()));
    }
}

/// Arquivo antigo: limite 2 negativo (começo deslocado) continua dando a mesma faixa de antes.
#[test]
fn extrusao_com_limite_2_negativo_abre_igual() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("antigo");
    let c = reta(&mut d, [0., 0., 0.], [40., 0., 0.]);
    let id = d.add_geometry(extrusao(c, "30", "-10"));
    let r = recompute(&d);
    ok(&r, &d, id);
    let (mn, mx) = caixa(&superficie(&d, id));
    assert!(perto(mn[2], 10., 1e-3) && perto(mx[2], 30., 1e-3), "{mn:?} {mx:?}");
}
