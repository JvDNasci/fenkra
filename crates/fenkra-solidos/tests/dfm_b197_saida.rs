// SPDX-License-Identifier: LGPL-2.1-or-later
//! B197: análise de saída e undercut por face, pela API reutilizável `dfm_saida`. Cada teste monta a
//! peça em código, recalcula, confere status, volume e caixa, e analisa a malha do corpo calculado.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::dfm::perfis_padrao;
use fenkra_solidos::dfm_geo::Classe;
use fenkra_solidos::dfm_saida::{self as saida, Faixa, Limites};
use fenkra_solidos::{final_shape, recompute};
use std::f64::consts::PI;

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);
const YZ: PlaneRef = PlaneRef::Origin(OriginPlane::YZ);

fn com_saida(sk: Id, dist: &str, taper: &str) -> FeatureKind {
    FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(dist.into()), reverse: false, taper: taper.into(), thin: None, dir: None, thin_side: 0, ends: Default::default() })
}

/// Caixa 30 x 20 x 15 extrudada com saída nas laterais.
fn caixa(taper: &str) -> Document {
    let mut d = Document::new("caixa_saida");
    let a = sk_rect(&mut d, XY, [0., 0.], [30., 20.]);
    d.add_feature(com_saida(a, "15", taper));
    d
}

#[test]
fn caixa_com_saida_de_2_graus_tem_as_laterais_com_saida_ok() {
    if !kernel_disponivel() {
        return;
    }
    let d = caixa("2");
    let r = recompute(&d);
    assert!(problema_de_status(&d, &r).is_none(), "{:?}", problema_de_status(&d, &r));
    assert_eq!(r.bodies.len(), 1);
    // tronco de pirâmide: a tampa encolhe 15 tan 2° de cada lado
    let k = 15. * 2f64.to_radians().tan();
    let (a1, a2) = (30. * 20., (30. - 2. * k) * (20. - 2. * k));
    let vol = 15. / 3. * (a1 + a2 + (a1 * a2).sqrt());
    assert!((volume_total(&r) - vol).abs() < 1e-3 * vol, "{} x {vol}", volume_total(&r));
    let (lo, hi) = caixa_envolvente(&d);
    assert!((hi[0] - lo[0] - 30.).abs() < 1e-3 && (hi[1] - lo[1] - 20.).abs() < 1e-3 && (hi[2] - lo[2] - 15.).abs() < 1e-3);

    let inj = perfis_padrao().into_iter().find(|p| p.processo == "injecao").unwrap();
    let a = saida::analisar_malha(&r.bodies[0].mesh, [0., 0., 1.], Limites::do_perfil(&inj)).unwrap();
    assert_eq!(a.faces.len(), 6);
    // as quatro laterais: 2 graus, o dobro do mínimo de 1 grau do perfil de injeção
    let lat: Vec<_> = a.faces.iter().filter(|f| f.saida_max < 80.).collect();
    assert_eq!(lat.len(), 4, "{:#?}", a.faces);
    for f in &lat {
        assert!((f.saida_min - 2.).abs() < 0.05, "{f:?}");
        assert_eq!(a.faixa(f), Faixa::Suficiente);
        assert_eq!(f.classe, Classe::Cavidade, "a lateral com saída olha para a abertura");
    }
    assert_eq!(a.contar_faixa(Faixa::Tampa), 2);
    assert_eq!(a.contar_classe(Classe::Nucleo), 1, "só o fundo olha para baixo");
    assert!(a.moldavel());
    // exigindo 3 graus, as mesmas laterais caem para "sem saída"
    let rig = saida::analisar_malha(&r.bodies[0].mesh, [0., 0., 1.], Limites { saida_min: 3., undercut_max: 0. }).unwrap();
    assert_eq!(rig.contar_faixa(Faixa::SemSaida), 4);
    assert!(!rig.moldavel());
}

#[test]
fn parede_reta_e_lateral_sem_saida() {
    if !kernel_disponivel() {
        return;
    }
    let d = caixa("0");
    confere("caixa reta", &d, &Esperado::prisma(30. * 20. * 15., [30., 20., 15.], [0.; 3]));
    let a = saida::analisar_forma(&final_shape(&d).unwrap(), [0., 0., 1.], Limites::default()).unwrap();
    assert_eq!(a.contar_classe(Classe::Lateral), 4);
    assert_eq!(a.contar_faixa(Faixa::SemSaida), 4);
    assert_eq!(a.contar_classe(Classe::Cavidade) + a.contar_classe(Classe::Nucleo), 2);
    assert_eq!(a.undercuts().count(), 0);
}

/// Bloco 30 x 20 x 20 com um furo lateral de 6 mm ao longo de x.
fn furo_lateral() -> Document {
    let mut d = Document::new("furo_lateral");
    let a = sk_rect(&mut d, XY, [0., 0.], [30., 20.]);
    d.add_feature(ext(a, BoolOp::Add, "20"));
    let c = sk_circulos(&mut d, YZ, &[([10., 10.], 3.)]);
    let mut corte = ext(c, BoolOp::Cut, "30");
    if let FeatureKind::Extrude(e) = &mut corte {
        // o corte entra contra a normal do plano YZ; a peça fica do lado +x
        e.reverse = true;
    }
    d.add_feature(corte);
    d
}

#[test]
fn furo_lateral_e_undercut_e_some_abrindo_pelo_eixo_do_furo() {
    if !kernel_disponivel() {
        return;
    }
    let d = furo_lateral();
    let vol = 30. * 20. * 20. - PI * 9. * 30.;
    confere("furo lateral", &d, &Esperado { vol, tol_vol: 1e-4, tam: [30., 20., 20.], tol_tam: 1e-3, minimo: Some([0.; 3]), via_malha: false });
    let r = recompute(&d);
    let m = &r.bodies[0].mesh;
    let a = saida::analisar_malha(m, [0., 0., 1.], Limites::default()).unwrap();
    let u: Vec<_> = a.undercuts().collect();
    assert!(!u.is_empty(), "{:#?}", a.faces);
    // só as faces do furo ficam bloqueadas: a metade de baixo olha para cima e bate na de cima, a de
    // cima olha para baixo e bate na de baixo. Fica de fora só a faixa quase vertical (lateral)
    for f in &u {
        let (y, z) = (f.ponto[1], f.ponto[2]);
        assert!(((y - 10.).powi(2) + (z - 10.).powi(2)).sqrt() < 3.2, "face fora do furo em undercut: {f:?}");
    }
    let bloqueada: f64 = u.iter().map(|f| f.area_undercut).sum();
    let parede = 2. * PI * 3. * 30.;
    assert!(bloqueada > 0.9 * parede && bloqueada <= 1.001 * parede, "{bloqueada} x {parede}");
    assert!(!a.moldavel());
    // abrindo o molde ao longo do furo (x) não há undercut: o pino sai pelo eixo
    let ax = saida::analisar_malha(m, [1., 0., 0.], Limites::default()).unwrap();
    assert_eq!(ax.undercuts().count(), 0, "{:#?}", ax.faces);
    // um processo que aceita até 1000 mm² bloqueados por face deixa o furo passar
    let folgado = saida::analisar_malha(m, [0., 0., 1.], Limites { saida_min: 1., undercut_max: 1000. }).unwrap();
    assert_eq!(folgado.undercuts().count(), 0);
}

/// Grava a peça do furo lateral num `.fkp` para a captura (`FENKRA_GRAVAR_PECA=<arquivo>`).
#[test]
#[ignore]
fn grava_peca_do_furo_lateral_para_captura() {
    if let Ok(p) = std::env::var("FENKRA_GRAVAR_PECA") {
        std::fs::write(p, furo_lateral().to_json()).unwrap();
    }
}
