// SPDX-License-Identifier: LGPL-2.1-or-later
//! B326, B327, B330, B331: tubo único com dobra no nó. Documentos montados em código, recalculados
//! no kernel; confere status, volume e caixa envolvente.

use fenkra_core::*;
use fenkra_solidos::{cut_list, recompute, tubos_dobrados, RecomputeResult, Status};

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
fn l_dobrado(kind: SectionKind, dims: &[&str], b: TubeBend, regras: Vec<TubeRule>) -> (Document, Id) {
    let mut d = Document::new("dobra");
    d.tube_rules = regras;
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

#[test]
fn dobradeira_redonda_vira_um_solido_com_o_desenvolvido_da_pesquisa() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // D 50 x 3, 90 graus, raio de centro 100: pernas de 200 + arco 157,08 + 200 = 557,08 mm
    let (d, f) = l_dobrado(SectionKind::TuboRedondo, &["50", "3"], bend(BendMode::Dobradeira, "100"), vec![]);
    let r = recompute(&d);
    assert_eq!(r.status[&f], Status::Ok, "{:?}", r.status[&f]);
    assert_eq!(fenkra_solidos::structural_members(&d).len(), 1, "perna, arco e perna fundidos num sólido só");
    let area = std::f64::consts::PI * (25f64.powi(2) - 22f64.powi(2));
    let esperado = area * (200. + 200. + 100. * std::f64::consts::FRAC_PI_2);
    assert!(perto(volume(&r), esperado, 0.003 * esperado), "volume {} contra {esperado}", volume(&r));
    // o arco tem centro em (200, 100) e raio externo 125: x até 325; y de -25 até 300
    let (lo, hi) = caixa(&r);
    assert!(perto(lo[0], 0., 0.5) && perto(hi[0], 325., 0.5), "x {:?}", (lo[0], hi[0]));
    assert!(perto(lo[1], -25., 0.5) && perto(hi[1], 300., 0.5), "y {:?}", (lo[1], hi[1]));
    assert!(perto(lo[2], -25., 0.5) && perto(hi[2], 25., 0.5), "z {:?}", (lo[2], hi[2]));
    // lista de corte: uma barra de 557,08 mm
    let rows = cut_list(&d);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(perto(rows[0].length, 557.1, 0.06), "{}", rows[0].length);
    // B331: a linha leva o modo, o raio e o ângulo da dobra
    let tubo = rows[0].tubo.as_ref().expect("linha do tubo dobrado sem as dobras");
    assert_eq!(tubo.resumo(), "R100.0×90.0°");
    assert_eq!(tubo.modo(), Some(BendMode::Dobradeira));
    // tubo desenvolvido: início da dobra em 200 e fim em 357,08
    let t = tubos_dobrados(&d);
    assert_eq!(t.len(), 1);
    assert!(perto(t[0].desenvolvido, 557.08, 0.01), "{}", t[0].desenvolvido);
    assert!(perto(t[0].soma_pernas, 600., 1e-9));
    assert!(perto(t[0].dobras[0].inicio, 200., 1e-9) && perto(t[0].dobras[0].fim, 357.08, 0.01));
    assert!(t[0].dobras[0].resultado.estimativa);
    assert!(t[0].erros.is_empty());
}

#[test]
fn dobradeira_quadrada_conserva_a_secao_e_o_volume() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // 30 x 30 x 1,2, raio de centro 60, 90 graus
    let (d, f) = l_dobrado(SectionKind::TuboQuadrado, &["30", "1.2"], bend(BendMode::Dobradeira, "60"), vec![]);
    let r = recompute(&d);
    assert_eq!(r.status[&f], Status::Ok, "{:?}", r.status[&f]);
    assert_eq!(r.bodies.len(), 1);
    let area = 30. * 30. - 27.6 * 27.6;
    // a seção é simétrica: o centro de massa da curva fica no eixo, volume = área x comprimento do eixo
    let esperado = area * (240. + 240. + 60. * std::f64::consts::FRAC_PI_2);
    assert!(perto(volume(&r), esperado, 0.004 * esperado), "volume {} contra {esperado}", volume(&r));
    let (lo, hi) = caixa(&r);
    // centro do arco (240, 60), raio externo 75
    assert!(perto(hi[0], 315., 0.5) && perto(hi[1], 300., 0.5) && perto(lo[1], -15., 0.5), "{lo:?} {hi:?}");
    // 557,08 vira 94,25 de arco: 240 + 94,25 + 240
    assert!(perto(cut_list(&d)[0].length, 574.25, 0.06), "{}", cut_list(&d)[0].length);
}

#[test]
fn retangular_respeita_qual_dimensao_fica_no_plano() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // 60 x 30 x 3 dobrado no plano XY: com a largura (60) no plano a altura em Z é 30; com a altura (30)
    // no plano, a largura (60) fica em Z
    let z = |plano: u8| {
        let b = TubeBend { plane_dim: plano, ..bend(BendMode::Dobradeira, "100") };
        let (d, f) = l_dobrado(SectionKind::TuboRetangular, &["60", "30", "3"], b, vec![]);
        let r = recompute(&d);
        assert_eq!(r.status[&f], Status::Ok, "plano {plano}: {:?}", r.status[&f]);
        let (lo, hi) = caixa(&r);
        (hi[2] - lo[2], hi[1])
    };
    let (z0, y0) = z(0);
    let (z1, y1) = z(1);
    assert!(perto(z0, 30., 0.5), "largura no plano: z = {z0}");
    assert!(perto(z1, 60., 0.5), "altura no plano: z = {z1}");
    // a curva externa é maior quando a dimensão do plano é a maior; a perna vertical vai até 300 nos dois
    assert!(perto(y0, 300., 0.5) && perto(y1, 300., 0.5));
    // o alongamento estimado da fibra externa muda com a dimensão do plano
    let al = |plano: u8| {
        let b = TubeBend { plane_dim: plano, ..bend(BendMode::Dobradeira, "100") };
        let (d, _) = l_dobrado(SectionKind::TuboRetangular, &["60", "30", "3"], b, vec![]);
        tubos_dobrados(&d)[0].dobras[0].resultado.alongamento.unwrap()
    };
    assert!(perto(al(0), 0.30, 1e-9) && perto(al(1), 0.15, 1e-9), "{} {}", al(0), al(1));
}

#[test]
fn raio_que_nao_cabe_falha_com_motivo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // tangente = 400 passa dos 300 da perna
    let (d, f) = l_dobrado(SectionKind::TuboRedondo, &["50", "3"], bend(BendMode::Dobradeira, "400"), vec![]);
    let r = recompute(&d);
    match &r.status[&f] {
        Status::Error(e) => assert!(e.contains("o raio não cabe"), "{e}"),
        outro => panic!("esperava erro, veio {outro:?}"),
    }
    // raio menor que a meia seção: a curva se cruza
    let (d, f) = l_dobrado(SectionKind::TuboRedondo, &["50", "3"], bend(BendMode::Dobradeira, "20"), vec![]);
    assert!(matches!(recompute(&d).status[&f], Status::Error(_)));
}

#[test]
fn regra_cadastrada_avisa_e_diz_de_onde_vem() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let regra = TubeRule { name: s("interna"), source: s("ensaio de bancada"), min_radius: s("120"), springback: s("3"), ..Default::default() };
    let b = TubeBend { rule: s("interna"), ..bend(BendMode::Dobradeira, "100") };
    let (d, f) = l_dobrado(SectionKind::TuboRedondo, &["50", "3"], b, vec![regra]);
    let r = recompute(&d);
    match &r.status[&f] {
        Status::Warning(w) => assert!(w.contains("abaixo do mínimo") && w.contains("ensaio de bancada"), "{w}"),
        outro => panic!("esperava aviso, veio {outro:?}"),
    }
    // o sólido sai mesmo assim: a regra avisa, não impede
    assert_eq!(r.bodies.len(), 1);
    let t = &tubos_dobrados(&d)[0];
    let res = &t.dobras[0].resultado;
    assert_eq!(res.retorno.v, 3.);
    assert!(res.retorno.origem.contains("interna"));
    assert!((res.angulo_maquina - 93.).abs() < 1e-9);
    // regra que não existe mais: erro claro
    let b = TubeBend { rule: s("sumiu"), ..bend(BendMode::Dobradeira, "100") };
    let (d, f) = l_dobrado(SectionKind::TuboRedondo, &["50", "3"], b, vec![]);
    match &recompute(&d).status[&f] {
        Status::Error(e) => assert!(e.contains("sumiu"), "{e}"),
        outro => panic!("{outro:?}"),
    }
}

#[test]
fn trocar_a_regra_muda_o_desenvolvido() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let neutra = TubeRule { name: s("n1"), neutral_shift: s("1"), ..Default::default() };
    let sem = l_dobrado(SectionKind::TuboRedondo, &["50", "3"], bend(BendMode::Dobradeira, "100"), vec![neutra.clone()]).0;
    let com = l_dobrado(SectionKind::TuboRedondo, &["50", "3"], TubeBend { rule: s("n1"), ..bend(BendMode::Dobradeira, "100") }, vec![neutra]).0;
    let a = tubos_dobrados(&sem)[0].desenvolvido;
    let b = tubos_dobrados(&com)[0].desenvolvido;
    // deslocar a neutra 1 mm para dentro encurta o arco em 1 x pi/2
    assert!(perto(a - b, std::f64::consts::FRAC_PI_2, 1e-9), "{a} {b}");
    assert!(perto(cut_list(&com)[0].length, b, 0.06));
}

#[test]
fn corte_simples_e_dois_membros_em_esquadria() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, f) = l_dobrado(SectionKind::TuboQuadrado, &["40", "3"], bend(BendMode::CorteSimples, "0"), vec![]);
    let r = recompute(&d);
    assert_eq!(r.status[&f], Status::Ok, "{:?}", r.status[&f]);
    assert_eq!(fenkra_solidos::structural_members(&d).len(), 2, "o corte simples deixa dois membros");
    let esperado = (40. * 40. - 34. * 34.) * 600.;
    assert!(perto(volume(&r), esperado, 0.001 * esperado), "{}", volume(&r));
    // mesma caixa da meia-esquadria de um L: o canto passa meia seção da linha de centro
    let (lo, hi) = caixa(&r);
    assert!(perto(hi[0], 320., 0.5) && perto(lo[1], -20., 0.5), "{lo:?} {hi:?}");
    // solda de 144,85 para 30x30 vira 2*40*raiz(2) + 80 = 193,1 em 40x40
    let t = &tubos_dobrados(&d)[0];
    assert!(perto(t.dobras[0].resultado.solda.unwrap(), 2. * 40. * 2f64.sqrt() + 80., 0.01));
    assert!(t.so_corte_simples());
}

#[test]
fn modos_em_v_ficam_preparados_e_avisam() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    for (modo, raio, quem) in [(BendMode::VComum, "2", "B328"), (BendMode::VComRaio, "40", "B333"), (BendMode::CorteDobradeira, "2", "B328")] {
        let (d, f) = l_dobrado(SectionKind::TuboQuadrado, &["40", "3"], bend(modo, raio), vec![]);
        let r = recompute(&d);
        match &r.status[&f] {
            Status::Warning(w) => assert!(w.contains(quem), "{modo:?}: {w}"),
            outro => panic!("{modo:?}: {outro:?}"),
        }
        // o canto vivo aparece (duas pernas em esquadria): nada de rasgo ainda
        assert_eq!(fenkra_solidos::structural_members(&d).len(), 2, "{modo:?}");
    }
    // tubo redondo não faz V
    let (d, f) = l_dobrado(SectionKind::TuboRedondo, &["40", "3"], bend(BendMode::VComum, "2"), vec![]);
    assert!(matches!(recompute(&d).status[&f], Status::Error(_)));
}

#[test]
fn dobra_entre_comandos_diferentes_e_recusada() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("x");
    let l1 = linha(&mut d, [0., 0., 0.], [300., 0., 0.]);
    let l2 = linha(&mut d, [300., 0., 0.], [300., 300., 0.]);
    let a = d.add_feature(FeatureKind::StructMember(membro(vec![l1], SectionKind::TuboRedondo, &["50", "3"])));
    let b = d.add_feature(FeatureKind::StructMember(membro(vec![l2], SectionKind::TuboRedondo, &["50", "3"])));
    if let Some(FeatureKind::StructMember(m)) = d.feature_mut(b).map(|f| &mut f.kind) {
        m.joints.push(StructJoint { mine: 0, other: a, other_path: 0, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(bend(BendMode::Dobradeira, "100")) });
    }
    let r = recompute(&d);
    match &r.status[&b] {
        Status::Error(e) => assert!(e.contains("mesmo comando"), "{e}"),
        outro => panic!("{outro:?}"),
    }
}

#[test]
fn tres_pernas_com_duas_dobras_fazem_um_tubo_so() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("u");
    let l1 = linha(&mut d, [0., 0., 0.], [300., 0., 0.]);
    let l2 = linha(&mut d, [300., 0., 0.], [300., 300., 0.]);
    let l3 = linha(&mut d, [300., 300., 0.], [0., 300., 0.]);
    let f = d.add_feature(FeatureKind::StructMember(membro(vec![l1, l2, l3], SectionKind::TuboRedondo, &["30", "2"])));
    if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|f| &mut f.kind) {
        m.joints.push(StructJoint { mine: 1, other: f, other_path: 0, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(bend(BendMode::Dobradeira, "50")) });
        m.joints.push(StructJoint { mine: 2, other: f, other_path: 1, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(bend(BendMode::Dobradeira, "50")) });
    }
    let r = recompute(&d);
    assert_eq!(r.status[&f], Status::Ok, "{:?}", r.status[&f]);
    assert_eq!(fenkra_solidos::structural_members(&d).len(), 1);
    let area = std::f64::consts::PI * (15f64.powi(2) - 13f64.powi(2));
    let eixo = 250. + 200. + 250. + 2. * 50. * std::f64::consts::FRAC_PI_2;
    assert!(perto(volume(&r), area * eixo, 0.004 * area * eixo), "{} contra {}", volume(&r), area * eixo);
    let t = &tubos_dobrados(&d)[0];
    assert_eq!(t.dobras.len(), 2);
    assert!(perto(t.desenvolvido, eixo, 0.01), "{}", t.desenvolvido);
    // marcas: a primeira dobra começa em 250, a segunda 250 mais o arco adiante
    assert!(perto(t.dobras[0].inicio, 250., 1e-9));
    assert!(perto(t.dobras[1].inicio, 250. + 50. * std::f64::consts::FRAC_PI_2 + 200., 0.01), "{}", t.dobras[1].inicio);
    let rows = cut_list(&d);
    assert_eq!(rows.len(), 1);
    assert!(perto(rows[0].length, eixo, 0.06));
}

#[test]
fn salvar_e_abrir_preserva_a_dobra_e_as_regras() {
    let regra = TubeRule { name: s("interna"), material: s("aço"), section: Some(SectionKind::TuboRedondo), tool: s("mandril"), min_radius: s("120"), springback: s("3"), max_strain: s("0.2"), source: s("ensaio"), ..Default::default() };
    let b = TubeBend { rule: s("interna"), k: s("0.4"), plane_dim: 1, cuts: s("3"), root_gap: s("1.5"), segments: s("2"), ..bend(BendMode::VComRaio, "40") };
    let (d, _) = l_dobrado(SectionKind::TuboRedondo, &["50", "3"], b, vec![regra]);
    let json = d.to_json();
    let volta = Document::from_json(&json).expect("abre de novo");
    assert_eq!(volta.tube_rules, d.tube_rules);
    assert_eq!(volta, d);
    // documento sem regras não grava o campo: arquivos antigos e novos continuam iguais
    let (vazio, _) = l_dobrado(SectionKind::TuboRedondo, &["50", "3"], bend(BendMode::Dobradeira, "100"), vec![]);
    assert!(!vazio.to_json().contains("tube_rules"));
    // uma junta de arquivo antigo (sem o campo bend) abre sem dobra
    let antigo = StructJoint { mine: 1, other: Id(7), other_path: 0, mode: CornerMode::Encaixe, first: true, gap: None, bend: None };
    let txt = serde_json::to_string(&antigo).unwrap();
    assert!(!txt.contains("bend"));
    let lido: StructJoint = serde_json::from_str(&txt).unwrap();
    assert_eq!(lido, antigo);
}
