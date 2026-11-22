// SPDX-License-Identifier: LGPL-2.1-or-later
//! B575: membro curvo como um membro só. Caminho com cantos vira um tubo dobrado com raio (L, U e 3D),
//! spline como guia única, cadeia de guias unidas pela caixa universal e a tabela de dobras para a
//! dobradeira. Documentos montados em código, recalculados no kernel; confere status, volume, caixa
//! envolvente e o desenvolvido contra a conta à mão.

use fenkra_core::campo_dados::DadosCampo;
use fenkra_core::*;
use fenkra_solidos::st_dobra_maquina::{rota_do_tubo, tabela_de_dobras, Modo, Opcoes};
use fenkra_solidos::{cut_list, recompute, structural_members, tubos_dobrados, RecomputeResult, Status};
use std::f64::consts::{FRAC_PI_2, PI};

fn s(x: &str) -> String {
    x.to_string()
}

fn membro(paths: Vec<CurveRef>, kind: SectionKind, dims: &[&str], raio: Option<&str>) -> StructMember {
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
        tube_bend: raio.map(|r| TubeBend { mode: BendMode::Dobradeira, radius: s(r), ..Default::default() }),
        fit: Default::default(),
    }
}

fn polilinha(d: &mut Document, pts: &[[f64; 3]]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::Polyline { points: pts.iter().map(|p| PointRef::Fixed(*p)).collect(), closed: false }))
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }))
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

fn area_redondo(d: f64, t: f64) -> f64 {
    PI * ((d / 2.).powi(2) - (d / 2. - t).powi(2))
}

const L: [[f64; 3]; 3] = [[0., 0., 0.], [300., 0., 0.], [300., 300., 0.]];
const U: [[f64; 3]; 4] = [[0., 0., 0.], [400., 0., 0.], [400., 300., 0.], [0., 300., 0.]];

/// Tubo em L por uma polilinha só: um sólido, arco de verdade no canto, sem encaixe de canto.
#[test]
fn tubo_em_l_com_raio_e_um_membro_so() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("l");
    let p = polilinha(&mut d, &L);
    // D 50 x 3, raio de centro 100: pernas de 200 + arco 157,08 + 200 (mesma conta do nó de dobra)
    let f = d.add_feature(FeatureKind::StructMember(membro(vec![p], SectionKind::TuboRedondo, &["50", "3"], Some("100"))));
    let r = recompute(&d);
    assert_eq!(r.status[&f], Status::Ok, "{:?}", r.status[&f]);
    assert_eq!(structural_members(&d).len(), 1, "o L é um tubo só, não dois membros");
    let esperado = area_redondo(50., 3.) * (200. + 200. + 100. * FRAC_PI_2);
    assert!(perto(volume(&r), esperado, 0.003 * esperado), "volume {} contra {esperado}", volume(&r));
    // arco com centro em (200, 100) e raio externo 125: x até 325, y de -25 a 300
    let (lo, hi) = caixa(&r);
    assert!(perto(lo[0], 0., 0.5) && perto(hi[0], 325., 0.5), "x {:?}", (lo[0], hi[0]));
    assert!(perto(lo[1], -25., 0.5) && perto(hi[1], 300., 0.5), "y {:?}", (lo[1], hi[1]));
    assert!(perto(lo[2], -25., 0.5) && perto(hi[2], 25., 0.5), "z {:?}", (lo[2], hi[2]));
    // lista de corte: uma barra de comprimento desenvolvido 557,08 mm, com as dobras na linha
    let rows = cut_list(&d);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(perto(rows[0].length, 557.1, 0.06), "{}", rows[0].length);
    let tubo = rows[0].tubo.as_ref().expect("linha com as dobras do tubo");
    assert_eq!(tubo.resumo(), "R100.0×90.0°");
    let t = tubos_dobrados(&d);
    assert_eq!(t.len(), 1);
    assert!(perto(t[0].desenvolvido, 600. + 100. * FRAC_PI_2 - 2. * 100., 0.01), "{}", t[0].desenvolvido);
}

/// O raio vazio é o mínimo automático pela seção: 2 vezes o diâmetro (parede normal), 3 vezes em parede
/// fina; fica igual a digitar o número.
#[test]
fn raio_automatico_pela_secao() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let vol = |dims: &[&str], raio: Option<&str>| {
        let mut d = Document::new("auto");
        let p = polilinha(&mut d, &[[0., 0., 0.], [500., 0., 0.], [500., 500., 0.]]);
        let mut m = membro(vec![p], SectionKind::TuboRedondo, dims, raio);
        if raio.is_none() {
            m.tube_bend = Some(TubeBend { mode: BendMode::Dobradeira, ..Default::default() });
        }
        let f = d.add_feature(FeatureKind::StructMember(m));
        let r = recompute(&d);
        assert_eq!(r.status[&f], Status::Ok, "{:?}", r.status[&f]);
        (volume(&r), tubos_dobrados(&d)[0].dobras[0].resultado.raio)
    };
    // D 50 x 3 (esbeltez 16,7): 2 x 50 = 100
    let (v_auto, raio) = vol(&["50", "3"], None);
    assert!(perto(raio, 100., 1e-9), "{raio}");
    let (v_fixo, _) = vol(&["50", "3"], Some("100"));
    assert!(perto(v_auto, v_fixo, 1e-6 * v_fixo), "{v_auto} x {v_fixo}");
    // D 60 x 1,5 (esbeltez 40, parede fina): 3 x 60 = 180
    let (_, raio_fino) = vol(&["60", "1.5"], None);
    assert!(perto(raio_fino, 180., 1e-9), "{raio_fino}");
}

/// Tubo em U: duas dobras de 90 graus no mesmo plano.
#[test]
fn tubo_em_u_com_raio_confere_volume_e_desenvolvido() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("u");
    let p = polilinha(&mut d, &U);
    let f = d.add_feature(FeatureKind::StructMember(membro(vec![p], SectionKind::TuboRedondo, &["25.4", "1.5"], Some("100"))));
    let r = recompute(&d);
    assert_eq!(r.status[&f], Status::Ok, "{:?}", r.status[&f]);
    assert_eq!(structural_members(&d).len(), 1);
    // retos 400-100, 300-200, 400-100 e dois arcos de 100 x 90 graus
    let desenvolvido = 300. + 100. + 300. + 2. * 100. * FRAC_PI_2;
    let esperado = area_redondo(25.4, 1.5) * desenvolvido;
    assert!(perto(volume(&r), esperado, 0.004 * esperado), "volume {} contra {esperado}", volume(&r));
    let t = tubos_dobrados(&d);
    assert!(perto(t[0].desenvolvido, desenvolvido, 1e-6), "{}", t[0].desenvolvido);
    assert_eq!(t[0].dobras.len(), 2);
    // as duas dobras no mesmo lado: rotação 0 entre elas
    let rota = rota_do_tubo(&t[0]).unwrap();
    assert!(rota.dobras.iter().all(|b| perto(b.angulo, 90., 1e-6) && perto(b.rotacao, 0., 1e-6)), "{:?}", rota.dobras);
    // caixa: x de 0 a 412,7 (a dobra de fora passa de 400 pela meia espessura do tubo) e y de -12,7 a 312,7
    let (lo, hi) = caixa(&r);
    assert!(perto(lo[0], 0., 0.5) && perto(hi[0], 412.7, 0.5), "x {:?}", (lo[0], hi[0]));
    assert!(perto(lo[1], -12.7, 0.5) && perto(hi[1], 312.7, 0.5), "y {:?}", (lo[1], hi[1]));
}

/// Raio que não cabe nas pernas dá erro nomeado e não deixa corpo parcial.
#[test]
fn raio_grande_demais_da_erro_e_nao_deixa_corpo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("erro");
    let p = polilinha(&mut d, &[[0., 0., 0.], [100., 0., 0.], [100., 100., 0.]]);
    let f = d.add_feature(FeatureKind::StructMember(membro(vec![p], SectionKind::TuboRedondo, &["50", "3"], Some("300"))));
    let r = recompute(&d);
    let msg = match &r.status[&f] {
        Status::Ok => String::new(),
        other => format!("{other:?}"),
    };
    assert!(r.bodies.is_empty(), "sem corpo parcial; {msg}");
    assert!(msg.contains("raio não cabe") || msg.contains("não cabe"), "{msg}");
}

/// Spline 3D (hélice interpolada por 5 pontos) como guia de um membro só: status ok, volume = área x comprimento.
#[test]
fn spline_3d_e_um_membro_so_varrido_pela_guia() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("spline");
    // hélice de raio 100 e passo de 50 mm por radiano, de 0 a 180 graus: comprimento sqrt(100² + 50²) x π
    let pts: Vec<PointRef> = (0..=8)
        .map(|k| {
            let a = PI * k as f64 / 8.;
            PointRef::Fixed([100. * a.cos(), 100. * a.sin(), 50. * a])
        })
        .collect();
    let g = d.add_geometry(RefKind::Spline { points: pts, closed: false, tangents: vec![] });
    let f = d.add_feature(FeatureKind::StructMember(membro(vec![CurveRef::Ref(g)], SectionKind::TuboRedondo, &["20", "2"], None)));
    let r = recompute(&d);
    assert_eq!(r.status[&f], Status::Ok, "{:?}", r.status[&f]);
    assert_eq!(structural_members(&d).len(), 1, "a spline é um membro só, não trechos");
    let comprimento = (100f64.powi(2) + 50f64.powi(2)).sqrt() * PI;
    let esperado = area_redondo(20., 2.) * comprimento;
    assert!(perto(volume(&r), esperado, 0.01 * esperado), "volume {} contra {esperado}", volume(&r));
    // a hélice sobe de 0 a 50 π = 157,08 mm; o tubo de raio externo 10 abre 10 mm para cada lado
    let (lo, hi) = caixa(&r);
    assert!(perto(hi[2], 50. * PI + 10., 3.) && perto(lo[2], -10., 3.), "z {:?}", (lo[2], hi[2]));
    assert!(perto(lo[0], -110., 3.) && perto(hi[0], 110., 3.), "x {:?}", (lo[0], hi[0]));
    // a seção fica normal à guia: sem torção, a parede mantém a espessura (volume dentro de 1% acima)
    assert!(r.bodies.iter().all(|b| b.mass.volume > 0.));
}

/// Três guias retas unidas na caixa universal viram uma cadeia e um tubo só; separadas, ficam três membros.
#[test]
fn varias_guias_unidas_em_cadeia_ou_separadas() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let monta = |unir: bool| {
        let mut d = Document::new("cadeia");
        let a = linha(&mut d, U[0], U[1]);
        let b = linha(&mut d, U[1], U[2]);
        let c = linha(&mut d, U[2], U[3]);
        let f = d.add_feature(FeatureKind::StructMember(membro(vec![a, b, c], SectionKind::TuboRedondo, &["25.4", "1.5"], Some("100"))));
        if unir {
            let mut campo = DadosCampo::novo(f, "caminhos");
            campo.unir(&[0, 1, 2]);
            d.set_dados_campo(campo);
        }
        let r = recompute(&d);
        (d, f, r)
    };
    let (d, f, r) = monta(true);
    assert_eq!(r.status[&f], Status::Ok, "{:?}", r.status[&f]);
    assert_eq!(structural_members(&d).len(), 1, "cadeia unida: um tubo só");
    let esperado = area_redondo(25.4, 1.5) * (700. + 2. * 100. * FRAC_PI_2);
    assert!(perto(volume(&r), esperado, 0.004 * esperado), "volume {} contra {esperado}", volume(&r));
    assert_eq!(tubos_dobrados(&d).len(), 1);
    // separadas: cada linha é um membro e não há dobra (cada uma é um caminho)
    let (d2, f2, r2) = monta(false);
    assert_eq!(r2.status[&f2], Status::Ok, "{:?}", r2.status[&f2]);
    assert_eq!(structural_members(&d2).len(), 3, "sem unir, três membros");
    assert!(tubos_dobrados(&d2).is_empty());
}

/// Arquivo antigo (sem `tube_bend`) abre e recalcula igual: cada trecho reto é um membro e o JSON não
/// ganha campo novo.
#[test]
fn arquivo_antigo_abre_igual() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("antigo");
    let p = polilinha(&mut d, &U);
    let f = d.add_feature(FeatureKind::StructMember(membro(vec![p], SectionKind::TuboQuadrado, &["30", "2"], None)));
    let json = d.to_json();
    assert!(!json.contains("tube_bend"), "sem dobra nada novo vai ao arquivo");
    let reaberto = Document::from_json(&json).expect("abre");
    let (a, b) = (recompute(&d), recompute(&reaberto));
    assert_eq!(a.status[&f], Status::Ok);
    assert_eq!(structural_members(&d).len(), 3, "como antes: um membro por trecho reto");
    assert!(perto(volume(&a), volume(&b), 1e-9));
    assert_eq!(caixa(&a), caixa(&b));
    // com a dobra ligada, o arquivo grava o campo e reabre igual
    let mut d2 = d.clone();
    if let Some(FeatureKind::StructMember(m)) = d2.feature_mut(f).map(|x| &mut x.kind) {
        m.tube_bend = Some(TubeBend { mode: BendMode::Dobradeira, radius: s("80"), ..Default::default() });
    }
    let j2 = d2.to_json();
    assert!(j2.contains("tube_bend"));
    let r2 = Document::from_json(&j2).expect("abre");
    assert!(perto(volume(&recompute(&d2)), volume(&recompute(&r2)), 1e-9));
}

/// Tubo 3D com 3 dobras de 90 graus (X 300, Y 200, Z 250, X 300), raio 60: a tabela de dobras bate com a
/// conta à mão (avanço, rotação, ângulo, raio, pontos e desenvolvido).
#[test]
fn tabela_de_dobras_de_tubo_3d_com_tres_dobras_confere_a_mao() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let pontos = [[0., 0., 0.], [300., 0., 0.], [300., 200., 0.], [300., 200., 250.], [600., 200., 250.]];
    let mut d = Document::new("3d");
    let p = polilinha(&mut d, &pontos);
    let f = d.add_feature(FeatureKind::StructMember(membro(vec![p], SectionKind::TuboRedondo, &["25.4", "1.5"], Some("60"))));
    let r = recompute(&d);
    assert_eq!(r.status[&f], Status::Ok, "{:?}", r.status[&f]);
    assert_eq!(structural_members(&d).len(), 1);
    let area = area_redondo(25.4, 1.5);
    let arco = 60. * FRAC_PI_2;
    // à mão: tangente = 60 x tan 45 = 60; retos 300-60, 200-120, 250-120, 300-60; três arcos de 60 x 90 graus
    let retos = [240., 80., 130., 240.];
    let desenvolvido: f64 = retos.iter().sum::<f64>() + 3. * arco;
    assert!(perto(desenvolvido, 972.743, 0.001));
    assert!(perto(volume(&r), area * desenvolvido, 0.006 * area * desenvolvido), "volume {} contra {}", volume(&r), area * desenvolvido);
    let t = tubos_dobrados(&d);
    assert_eq!(t.len(), 1);
    assert!(perto(t[0].desenvolvido, desenvolvido, 1e-6), "{}", t[0].desenvolvido);
    let rota = rota_do_tubo(&t[0]).unwrap();
    for (a, b) in rota.retos.iter().zip(retos) {
        assert!(perto(*a, b, 1e-6), "{:?}", rota.retos);
    }
    // normais dos planos de dobra: X x Y = Z, Y x Z = X, Z x X = Y; o giro entre elas em torno do avanço é
    // 0 (referência), +90 e +90
    let rotacoes = [0., 90., 90.];
    for (b, rot) in rota.dobras.iter().zip(rotacoes) {
        assert!(perto(b.angulo, 90., 1e-6) && perto(b.raio, 60., 1e-9) && perto(b.rotacao, rot, 1e-6), "{:?}", rota.dobras);
    }
    // a tabela em CSV
    let csv = tabela_de_dobras(&t[0], Opcoes::default()).unwrap();
    let linhas: Vec<&str> = csv.lines().collect();
    assert!(csv.contains("n;avanco;rotacao;angulo;raio;arco;x;y;z;inicio;fim"), "{csv}");
    let dobras: Vec<Vec<&str>> = linhas.iter().filter(|l| l.chars().next().is_some_and(|c| c.is_ascii_digit()) && l.matches(';').count() == 10).map(|l| l.split(';').collect()).collect();
    assert_eq!(dobras.len(), 3, "{csv}");
    let num = |s: &str| s.parse::<f64>().unwrap();
    let avancos = [240., 80., 130.];
    let nos = [[300., 0., 0.], [300., 200., 0.], [300., 200., 250.]];
    // posição do início de cada dobra no tubo reto: retos e arcos anteriores
    let mut inicio = 0.;
    for (k, c) in dobras.iter().enumerate() {
        inicio += avancos[k];
        assert!(perto(num(c[1]), avancos[k], 1e-5), "avanço {k}: {csv}");
        assert!(perto(num(c[2]), rotacoes[k], 1e-4), "rotação {k}: {csv}");
        assert!(perto(num(c[3]), 90., 1e-5) && perto(num(c[4]), 60., 1e-5) && perto(num(c[5]), arco, 1e-4), "{csv}");
        for j in 0..3 {
            assert!(perto(num(c[6 + j]), nos[k][j], 1e-4), "ponto {k}: {csv}");
        }
        assert!(perto(num(c[9]), inicio, 1e-4) && perto(num(c[10]), inicio + arco, 1e-4), "início/fim {k}: {csv}");
        inicio += arco;
    }
    assert!(linhas.iter().any(|l| l.starts_with("fim;240.0")), "{csv}");
    assert!(csv.contains("desenvolvido;972.74"), "{csv}");
    // modo absoluto: avanço e rotação acumulados desde o começo do tubo
    let abs = tabela_de_dobras(&t[0], Opcoes { modo: Modo::Absoluto, ..Default::default() }).unwrap();
    assert!(abs.contains("\n2;414.247") && abs.contains(";180.000000;"), "{abs}");
}
