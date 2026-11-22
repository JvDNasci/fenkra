// SPDX-License-Identifier: LGPL-2.1-or-later
//! B491 e B492: cantos de quadro em meia-esquadria (ângulos diferentes de 90°, tubo quadrado, tubo
//! retangular e cantoneira) e corte de topo a 90° (canto de quadro e junta em T). Cada teste monta o
//! documento em código, recalcula e confere status, volume, interseção entre os membros (booleana
//! comum), folga (contato) e comprimentos da lista de corte.

use super::*;

fn s(x: &str) -> String {
    x.to_string()
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.)
}

fn comando(d: &mut Document, linhas: &[(V3, V3)], kind: SectionKind, dims: &[&str], corner: CornerMode, angle: &str) -> Id {
    let paths = linhas
        .iter()
        .map(|(a, b)| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(*a), b: PointRef::Fixed(*b), start: s("0"), end: s("0") })))
        .collect();
    d.add_feature(FeatureKind::StructMember(StructMember {
        paths,
        family: s("Perfil"),
        designation: s("teste"),
        kind,
        dims: dims.iter().map(|x| s(x)).collect(),
        anchor: 0,
        angle: s(angle),
        off_x: s("0"),
        off_y: s("0"),
        ext1: s("0"),
        ext2: s("0"),
        corner,
        gap: s("0"),
        joints: vec![],
        material: String::new(),
        mirror: false,
        tube_bend: None,
    }))
}

/// Quadro fechado pelos vértices (em ordem), um comando com uma linha por lado.
fn quadro(pts: &[V3], kind: SectionKind, dims: &[&str], corner: CornerMode, angle: &str) -> Document {
    let mut d = Document::new("quadro");
    let linhas: Vec<(V3, V3)> = (0..pts.len()).map(|i| (pts[i], pts[(i + 1) % pts.len()])).collect();
    comando(&mut d, &linhas, kind, dims, corner, angle);
    d
}

/// Leva o sólido do referencial local do membro para o mundo (inverso de `to_local`).
fn para_mundo(sh: &Shape, fr: &Frame) -> Shape {
    let m = [fr.x, fr.y(), fr.normal];
    let tr = m[0][0] + m[1][1] + m[2][2];
    let ang = ((tr - 1.) / 2.).clamp(-1., 1.).acos();
    let r = if ang.abs() < 1e-12 {
        sh.clone()
    } else {
        let axis = if (std::f64::consts::PI - ang).abs() < 1e-6 {
            let c = (0..3).max_by(|i, j| m[*i][*i].total_cmp(&m[*j][*j])).unwrap();
            geo3d::unit([m[0][c] + (c == 0) as u8 as f64, m[1][c] + (c == 1) as u8 as f64, m[2][c] + (c == 2) as u8 as f64]).unwrap()
        } else {
            geo3d::unit([m[2][1] - m[1][2], m[0][2] - m[2][0], m[1][0] - m[0][1]]).unwrap()
        };
        sh.transformed(Xform::Rotate([0.; 3], axis, -ang.to_degrees())).unwrap()
    };
    r.transformed(Xform::Translate(fr.origin)).unwrap()
}

fn mundo(d: &Document) -> Vec<(usize, Shape)> {
    structural_members(d).iter().map(|m| (m.index, para_mundo(&m.shape, &m.frame))).collect()
}

fn vol_comum(a: &Shape, b: &Shape) -> f64 {
    match a.boolean(b, Bool::Common) {
        Ok(c) => c.mass().volume,
        Err(_) => 0.,
    }
}

/// Folga entre duas peças: menor distância de um vértice da malha de `b` à superfície de `a`
/// (a face de corte compartilhada tem vértices de `b` sobre `a`; com vão, o mínimo passa de zero).
fn folga(a: &Shape, b: &Shape) -> f64 {
    let m = b.mesh_angle(0.5, 0.35).unwrap();
    let pts: Vec<V3> = m.positions.chunks(3).map(|v| [v[0] as f64, v[1] as f64, v[2] as f64]).collect();
    a.signed_distances(&pts).unwrap().iter().map(|d| d.abs()).fold(f64::MAX, f64::min)
}

fn ok(d: &Document) {
    let r = recompute(d);
    for (id, st) in &r.status {
        assert!(matches!(st, Status::Ok | Status::Warning(_)), "{id:?}: {st:?}");
    }
    assert!(!r.bodies.is_empty());
}

/// Confere um quadro fechado: membros vizinhos sem interseção e encostando um no outro; o volume do
/// corpo é a soma dos membros; ângulos da lista de corte iguais à metade do ângulo do canto.
fn confere_quadro(d: &Document, n: usize, angulos_canto: &[f64]) {
    ok(d);
    let ms = mundo(d);
    assert_eq!(ms.len(), n, "um sólido por lado");
    let soma: f64 = ms.iter().map(|(_, sh)| sh.mass().volume).sum();
    let corpo: f64 = recompute(d).bodies.iter().map(|b| b.mass.volume).sum();
    assert!(perto(corpo, soma, 1e-4), "volume do corpo {corpo} contra a soma dos membros {soma}: há sobreposição ou vão");
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, b) = (&ms[i].1, &ms[j].1);
        let inter = vol_comum(a, b);
        assert!(inter < 0.5, "membros {i} e {j} se cruzam: {inter} mm³");
        let g = folga(a, b).min(folga(b, a));
        assert!(g < 0.01, "membros {i} e {j} não se tocam (vão de {g} mm)");
    }
    let cl = cut_list(d);
    for r in &cl {
        for (k, a) in [r.angle1, r.angle2].into_iter().enumerate() {
            let a = a.unwrap_or_else(|| panic!("ponta {k} sem ângulo na lista de corte"));
            assert!(angulos_canto.iter().any(|c| (a - (90. - c / 2.).abs()).abs() < 0.2 || (a - c / 2.).abs() < 0.2), "ângulo {a} da lista não bate com os cantos {angulos_canto:?}");
        }
    }
}

const RET: [V3; 4] = [[0., 0., 0.], [400., 0., 0.], [400., 300., 0.], [0., 300., 0.]];

fn paralelogramo() -> [V3; 4] {
    let c = 60f64.to_radians();
    [[0., 0., 0.], [400., 0., 0.], [400. + 300. * c.cos(), 300. * c.sin(), 0.], [300. * c.cos(), 300. * c.sin(), 0.]]
}

#[test]
fn meia_esquadria_retangular_tubo_quadrado() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    confere_quadro(&quadro(&RET, SectionKind::TuboQuadrado, &["40", "3"], CornerMode::MeiaEsquadria, "0"), 4, &[90.]);
}

#[test]
fn meia_esquadria_retangular_tubo_retangular() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    confere_quadro(&quadro(&RET, SectionKind::TuboRetangular, &["60", "30", "3"], CornerMode::MeiaEsquadria, "0"), 4, &[90.]);
    // seção girada 90°: o lado largo passa a ser o vertical
    confere_quadro(&quadro(&RET, SectionKind::TuboRetangular, &["60", "30", "3"], CornerMode::MeiaEsquadria, "90"), 4, &[90.]);
}

#[test]
fn meia_esquadria_retangular_cantoneira() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    for ang in ["0", "90", "180", "270"] {
        confere_quadro(&quadro(&RET, SectionKind::Cantoneira, &["50", "50", "5"], CornerMode::MeiaEsquadria, ang), 4, &[90.]);
    }
    confere_quadro(&quadro(&RET, SectionKind::Cantoneira, &["60", "40", "5"], CornerMode::MeiaEsquadria, "0"), 4, &[90.]);
}

#[test]
fn meia_esquadria_60_120_tubo_quadrado() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    confere_quadro(&quadro(&paralelogramo(), SectionKind::TuboQuadrado, &["40", "3"], CornerMode::MeiaEsquadria, "0"), 4, &[60., 120.]);
}

#[test]
fn meia_esquadria_60_120_tubo_retangular() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    for ang in ["0", "90"] {
        confere_quadro(&quadro(&paralelogramo(), SectionKind::TuboRetangular, &["60", "30", "3"], CornerMode::MeiaEsquadria, ang), 4, &[60., 120.]);
    }
}

#[test]
fn meia_esquadria_60_120_cantoneira() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    for ang in ["0", "90", "180", "270"] {
        confere_quadro(&quadro(&paralelogramo(), SectionKind::Cantoneira, &["50", "50", "5"], CornerMode::MeiaEsquadria, ang), 4, &[60., 120.]);
    }
}

#[test]
fn meia_esquadria_quadro_inclinado_no_espaco() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // quadro de 60/120 num plano inclinado (nenhum lado paralelo aos eixos)
    let p = paralelogramo().map(|q| [q[0], q[1] * 0.8, q[1] * 0.6 + 50.]);
    confere_quadro(&quadro(&p, SectionKind::TuboRetangular, &["60", "30", "3"], CornerMode::MeiaEsquadria, "0"), 4, &[60., 120.]);
    confere_quadro(&quadro(&p, SectionKind::Cantoneira, &["50", "50", "5"], CornerMode::MeiaEsquadria, "0"), 4, &[60., 120.]);
}

/// Dois comandos (canto em L entre dois membros): um membro por comando.
fn canto_l(kind: SectionKind, dims: &[&str], corner: CornerMode, graus: f64) -> Document {
    let mut d = Document::new("l");
    let a = graus.to_radians();
    comando(&mut d, &[([0., 0., 0.], [400., 0., 0.])], kind, dims, corner, "0");
    comando(&mut d, &[([400., 0., 0.], [400. + 300. * a.cos(), 300. * a.sin(), 0.])], kind, dims, corner, "0");
    d
}

#[test]
fn meia_esquadria_em_dois_comandos_quadrado_retangular_cantoneira() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    for (kind, dims) in [(SectionKind::TuboQuadrado, vec!["40", "3"]), (SectionKind::TuboRetangular, vec!["60", "30", "3"]), (SectionKind::Cantoneira, vec!["50", "40", "5"])] {
        for g in [90., 120., 60.] {
            let d = canto_l(kind, &dims, CornerMode::MeiaEsquadria, g);
            ok(&d);
            let ms = mundo(&d);
            assert_eq!(ms.len(), 2);
            let inter = vol_comum(&ms[0].1, &ms[1].1);
            assert!(inter < 0.5, "{kind:?} {g}°: interseção {inter}");
            let vao = folga(&ms[0].1, &ms[1].1).min(folga(&ms[1].1, &ms[0].1));
            assert!(vao < 0.01, "{kind:?} {g}°: vão ({vao} mm)");
        }
    }
}

// ---- B492: corte de topo (90°) -------------------------------------------------------------------

/// Caixa envolvente da união dos membros.
fn caixa_uniao(ms: &[(usize, Shape)]) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for (_, sh) in ms {
        let (a, b) = sh.bbox().unwrap();
        for k in 0..3 {
            lo[k] = lo[k].min(a[k]);
            hi[k] = hi[k].max(b[k]);
        }
    }
    (lo, hi)
}

/// Quadro fechado com Aparar: cada canto tem um membro que passa direto e outro que para na face;
/// o conjunto não pode passar da caixa do quadro (sobra) nem se cruzar.
#[test]
fn topo_em_quadro_fechado_aparar_sem_sobra_nem_interferencia() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // (perfil, cotas, rotação, meia largura no plano do quadro)
    let casos: [(SectionKind, Vec<&str>, &str, f64); 4] = [
        (SectionKind::TuboQuadrado, vec!["40", "3"], "0", 20.),
        (SectionKind::TuboRetangular, vec!["60", "30", "3"], "0", 30.),
        (SectionKind::TuboRetangular, vec!["60", "30", "3"], "90", 15.),
        (SectionKind::TuboRetangular, vec!["30", "60", "3"], "0", 15.),
    ];
    for (kind, dims, ang, meia) in casos {
        let d = quadro(&RET, kind, &dims, CornerMode::Aparar, ang);
        ok(&d);
        let ms = mundo(&d);
        assert_eq!(ms.len(), 4);
        for i in 0..4 {
            let j = (i + 1) % 4;
            let inter = vol_comum(&ms[i].1, &ms[j].1);
            assert!(inter < 0.5, "{kind:?} {dims:?} rot {ang}: membros {i} e {j} se cruzam ({inter} mm³)");
            let vao = folga(&ms[i].1, &ms[j].1).min(folga(&ms[j].1, &ms[i].1));
            assert!(vao < 0.01, "{kind:?} {dims:?} rot {ang}: vão entre {i} e {j} ({vao} mm)");
        }
        let (lo, hi) = caixa_uniao(&ms);
        let esperado = ([-meia, -meia], [400. + meia, 300. + meia]);
        for (k, (l, h)) in [(lo[0], hi[0]), (lo[1], hi[1])].into_iter().enumerate() {
            assert!((l - esperado.0[k]).abs() < 0.01 && (h - esperado.1[k]).abs() < 0.01, "{kind:?} {dims:?} rot {ang}: sobra na caixa eixo {k}: {l}..{h}, esperado {}..{}", esperado.0[k], esperado.1[k]);
        }
    }
}

/// Lista de corte do T e do quadro com Aparar: comprimento de cada barra e ângulos de ponta.
#[test]
fn lista_de_corte_do_topo_confere_comprimentos() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let linhas = |d: &Document| cut_list(d).iter().map(|r| (r.length, r.count, r.angle1, r.angle2)).collect::<Vec<_>>();
    // T: o principal (1000) passa direto; o encostante sai da linha de centro (300) mas só existe da face
    // do principal (20) até a ponta: 280 de barra, com corte reto nas duas pontas
    let l = linhas(&topo(CornerMode::Aparar, SectionKind::TuboQuadrado, &["40", "3"], true, true));
    assert!(l.iter().any(|x| (x.0 - 1000.).abs() < 0.1 && x.2 == Some(0.) && x.3 == Some(0.)), "{l:?}");
    assert!(l.iter().any(|x| (x.0 - 280.).abs() < 0.1 && x.2 == Some(0.) && x.3 == Some(0.)), "{l:?}");
    // quadro 400x300 com Aparar: dois lados passam direto até a face de fora do vizinho
    let l = linhas(&quadro(&RET, SectionKind::TuboQuadrado, &["40", "3"], CornerMode::Aparar, "0"));
    let total: usize = l.iter().map(|x| x.1).sum();
    assert_eq!(total, 4);
    // linha 1 e linha 3 passam direto (+20 de cada lado até a face de fora do vizinho em uma ponta e
    // -20 na outra); cada medida abaixo é a barra real: 440 (400+20+20), 300 (300-20+20), 400 (400-20+20)
    // e 260 (300-20-20)
    let mut medidas: Vec<f64> = l.iter().map(|x| x.0).collect();
    medidas.sort_by(f64::total_cmp);
    assert_eq!(medidas, vec![260., 300., 400., 440.], "{l:?}");
    assert!(l.iter().all(|x| x.2 == Some(0.) && x.3 == Some(0.)), "corte de topo é reto: {l:?}");
}

/// T ou canto: um membro passa direto e o outro encosta. A ordem na árvore define quem passa
/// (o anterior é o membro 1, que continua); `continua_primeiro` escolhe o principal como o primeiro.
fn topo(corner: CornerMode, kind: SectionKind, dims: &[&str], t: bool, continua_primeiro: bool) -> Document {
    let mut d = Document::new("topo");
    let (principal, encosta) = if t { (([0., 0., 0.], [1000., 0., 0.]), ([500., 0., 0.], [500., 300., 0.])) } else { (([0., 0., 0.], [400., 0., 0.]), ([400., 0., 0.], [400., 300., 0.])) };
    let (a, b) = if continua_primeiro { (principal, encosta) } else { (encosta, principal) };
    comando(&mut d, &[a], kind, dims, CornerMode::Nenhum, "0");
    comando(&mut d, &[b], kind, dims, corner, "0");
    d
}

fn casos_perfil() -> Vec<(SectionKind, Vec<&'static str>)> {
    vec![(SectionKind::TuboQuadrado, vec!["40", "3"]), (SectionKind::TuboRetangular, vec!["60", "30", "3"]), (SectionKind::Cantoneira, vec!["50", "50", "5"])]
}

fn sem_interferencia_nem_vao(d: &Document, rotulo: &str) {
    ok(d);
    let ms = mundo(d);
    assert_eq!(ms.len(), 2, "{rotulo}");
    let inter = vol_comum(&ms[0].1, &ms[1].1);
    assert!(inter < 0.5, "{rotulo}: interseção {inter}");
    let vao = folga(&ms[0].1, &ms[1].1).min(folga(&ms[1].1, &ms[0].1));
    assert!(vao < 0.01, "{rotulo}: vão ({vao} mm)");
}

#[test]
fn topo_em_t_aparar_nao_invade_nem_deixa_vao() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    for (kind, dims) in casos_perfil() {
        for modo in [CornerMode::Aparar, CornerMode::Encaixe] {
            for primeiro in [true, false] {
                sem_interferencia_nem_vao(&topo(modo, kind, &dims, true, primeiro), &format!("T {kind:?} {modo:?} principal primeiro={primeiro}"));
            }
        }
    }
}

#[test]
fn topo_em_canto_de_quadro_aparar_sem_interferencia() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    for (kind, dims) in casos_perfil() {
        for primeiro in [true, false] {
            sem_interferencia_nem_vao(&topo(CornerMode::Aparar, kind, &dims, false, primeiro), &format!("canto {kind:?} principal primeiro={primeiro}"));
        }
    }
}
