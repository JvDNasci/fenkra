// SPDX-License-Identifier: LGPL-2.1-or-later
//! B350: varredura em guia com cantos. A seção gira em cada canto com junta de esquadria, então um tubo
//! de seção constante centrado na guia tem volume = área × comprimento da guia (o que a parede de dentro
//! perde no canto, a de fora ganha). Antes, L dava metade e U dava zero, com status ok.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_kernel::{Frame, Seg2, Shape};
use fenkra_solidos::{geo3d, recompute, Status};
use std::f64::consts::PI;

fn comprimento(p: &[[f64; 3]]) -> f64 {
    p.windows(2).map(|q| geo3d::dist(q[0], q[1])).sum()
}

/// Laços do perfil (círculos concêntricos) normais ao primeiro trecho da guia, no início dela.
fn perfil(p: &[[f64; 3]], raios: &[f64]) -> Shape {
    let t = geo3d::unit(geo3d::sub(p[1], p[0])).unwrap();
    let f = Frame { origin: p[0], x: geo3d::any_perp(t), normal: t };
    let lacos: Vec<Vec<Seg2>> = raios.iter().map(|r| vec![Seg2::Circle([0., 0.], *r)]).collect();
    Shape::face(&f, &lacos).unwrap()
}

fn confere_tubo(nome: &str, p: &[[f64; 3]], raios: &[f64]) -> Shape {
    let s = perfil(p, raios).sweep(p, 0).unwrap_or_else(|e| panic!("{nome}: {}", e.0));
    let area = PI * (raios[0] * raios[0] - raios.get(1).map_or(0., |r| r * r));
    let esperado = area * comprimento(p);
    assert!(s.is_valid(), "{nome}: sólido inválido");
    let v = s.mass().volume;
    assert!((v / esperado - 1.).abs() < 1e-3, "{nome}: volume {v:.2}, esperado {esperado:.2}");
    s
}

#[test]
fn guia_em_l_da_o_volume_do_tubo_inteiro() {
    if !kernel_disponivel() {
        return;
    }
    let s = confere_tubo("L", &[[0., 0., 0.], [50., 0., 0.], [50., 50., 0.]], &[2.]);
    // 4π × 100 = 1256,6 (antes: 628, a metade)
    assert!((s.mass().volume - 1256.64).abs() < 1.5, "{}", s.mass().volume);
    let (a, b) = s.bbox().unwrap();
    let tam = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    // o segundo trecho é um tubo de verdade ao longo de y: a caixa vai de y = -2 a 50 e x até 52
    assert!((tam[0] - 52.).abs() < 0.05 && (tam[1] - 52.).abs() < 0.05 && (tam[2] - 4.).abs() < 0.05, "{a:?} {b:?}");
}

#[test]
fn guia_em_u_e_guia_3d_seguem_a_secao() {
    if !kernel_disponivel() {
        return;
    }
    // U: antes dava volume zero
    confere_tubo("U", &[[0., 0., 0.], [50., 0., 0.], [50., 50., 0.], [0., 50., 0.]], &[2.]);
    // guia fora de um plano: cada canto num plano diferente
    confere_tubo("3D", &[[0., 0., 0.], [50., 0., 0.], [50., 50., 0.], [50., 50., 50.]], &[2.]);
    // canto agudo (135° de giro) e canto suave (30°)
    confere_tubo("agudo", &[[0., 0., 0.], [50., 0., 0.], [15., 35., 0.]], &[2.]);
    confere_tubo("suave", &[[0., 0., 0.], [50., 0., 0.], [93.3, 25., 0.]], &[2.]);
}

/// Guia reta de dois pontos não tem canto: segue pelo caminho de sempre (não pela esquadria).
#[test]
fn guia_reta_de_dois_pontos_continua_varrendo() {
    if !kernel_disponivel() {
        return;
    }
    confere_tubo("reta", &[[0., 0., 0.], [60., 0., 0.]], &[2.]);
    for modo in [0u8, 1, 2] {
        let p = [[0., 0., 0.], [0., 0., 40.]];
        let s = perfil(&p, &[3.]).sweep(&p, modo).unwrap_or_else(|e| panic!("modo {modo}: {}", e.0));
        assert!((s.mass().volume - PI * 9. * 40.).abs() < 0.05, "modo {modo}: {}", s.mass().volume);
    }
}

#[test]
fn perfil_com_furo_vira_tubo_oco_nos_cantos() {
    if !kernel_disponivel() {
        return;
    }
    confere_tubo("anel em L", &[[0., 0., 0.], [40., 0., 0.], [40., 30., 0.], [40., 30., 25.]], &[3., 2.]);
}

/// Seção retangular fora do centro: o volume não é mais área × comprimento, mas o sólido tem de ser
/// válido e a parede tem de virar com a guia (a caixa mostra a seção girada no segundo trecho).
#[test]
fn parede_retangular_vira_no_canto() {
    if !kernel_disponivel() {
        return;
    }
    let p = [[0., 0., 0.], [60., 0., 0.], [60., 40., 0.]];
    let f = Frame { origin: p[0], x: [0., 1., 0.], normal: [1., 0., 0.] };
    // parede de 2 de largura (y) por 10 de altura (z), centrada na guia
    let c = [[-1., 0.], [1., 0.], [1., 10.], [-1., 10.], [-1., 0.]];
    let laco: Vec<Seg2> = c.windows(2).map(|q| Seg2::Line(q[0], q[1])).collect();
    let s = Shape::face(&f, &[laco]).unwrap().sweep(&p, 0).unwrap();
    assert!(s.is_valid());
    assert!((s.mass().volume / (2. * 10. * 100.) - 1.).abs() < 1e-3, "{}", s.mass().volume);
    let (a, b) = s.bbox().unwrap();
    assert!((b[0] - 61.).abs() < 0.05 && (a[1] + 1.).abs() < 0.05 && (b[1] - 40.).abs() < 0.05, "{a:?} {b:?}");
}

fn doc_l(mode: SweepMode) -> (Document, Id) {
    let mut d = Document::new("guia");
    let pts = vec![[0., 0., 0.], [50., 0., 0.], [50., 50., 0.]];
    let g = d.add_geometry(RefKind::Polyline { points: pts.into_iter().map(PointRef::Fixed).collect(), closed: false });
    let perfil = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::YZ), &[([0., 0.], 2.)]);
    let id = d.add_feature(FeatureKind::Sweep(Sweep { sketch: perfil, spine: CurveRef::Ref(g), mode, op: BoolOp::Add, thin: None }));
    (d, id)
}

/// O comando no documento: status sem aviso, volume e caixa do tubo inteiro.
#[test]
fn comando_varredura_em_l_sai_sem_aviso_e_com_volume_certo() {
    if !kernel_disponivel() {
        return;
    }
    let (d, id) = doc_l(SweepMode::Corrigido);
    let r = recompute(&d);
    assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
    assert!((volume_total(&r) - 400. * PI).abs() < 1.5, "{}", volume_total(&r));
    let (lo, hi) = caixa_envolvente(&d);
    assert!((hi[0] - lo[0] - 52.).abs() < 0.1 && (hi[1] - lo[1] - 52.).abs() < 0.1, "{lo:?} {hi:?}");
    // orientação fixa pedida pela pessoa: a seção não gira, e o status diz isso
    let (d, id) = doc_l(SweepMode::Fixo);
    let st = recompute(&d).status[&id].clone();
    assert!(matches!(&st, Status::Warning(m) if m.contains("orientação fixa")), "{st:?}");
}

/// Arquivo gravado antes da B350 (formato 2): abre migrado, avisa no comando que mudou e, gravado de
/// novo, não carrega mais a marca.
#[test]
fn arquivo_antigo_migra_e_avisa_uma_vez() {
    if !kernel_disponivel() {
        return;
    }
    let (d, id) = doc_l(SweepMode::Corrigido);
    let mut j: serde_json::Value = serde_json::from_str(&d.to_json()).unwrap();
    assert!(j.get("migracoes").is_none(), "a marca de migração não é gravada");
    j["format_version"] = 2.into();
    let antigo = Document::from_json(&j.to_string()).unwrap();
    assert_eq!(antigo.format_version, fenkra_core::formato::Tipo::Peca.versao_atual());
    assert_eq!(antigo.migracoes, vec![fenkra_core::formato::MIGRACAO_CANTOS_VARREDURA.to_string()]);
    let r = recompute(&antigo);
    assert!(matches!(&r.status[&id], Status::Warning(m) if m.contains("B350")), "{:?}", r.status[&id]);
    assert!((volume_total(&r) - 400. * PI).abs() < 1.5, "o arquivo antigo também sai com o volume certo");
    let regravado = Document::from_json(&antigo.to_json()).unwrap();
    assert!(regravado.migracoes.is_empty());
    assert_eq!(recompute(&regravado).status[&id], Status::Ok);
    // peça antiga sem varredura não ganha marca
    let mut j: serde_json::Value = serde_json::from_str(&Document::new("vazia").to_json()).unwrap();
    j["format_version"] = 2.into();
    assert!(Document::from_json(&j.to_string()).unwrap().migracoes.is_empty());
}

/// Zigue-zague 3D com o perfil fora do centro e inclinado: se a esquadria não fecha, o comando não cai
/// em erro nem sai calado; refaz com orientação fixa e avisa.
#[test]
fn quando_a_esquadria_nao_fecha_o_comando_avisa_e_segue() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("zigue");
    let pts: Vec<[f64; 3]> = (0..14).map(|i| [10. * i as f64, if i % 3 == 0 { 0.9 } else { -0.4 }, if i % 2 == 0 { 0. } else { 10. }]).collect();
    let g = d.add_geometry(RefKind::Polyline { points: pts.into_iter().map(PointRef::Fixed).collect(), closed: false });
    let perfil = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::YZ), &[([0., 0.], 1.)]);
    let id = d.add_feature(FeatureKind::Sweep(Sweep { sketch: perfil, spine: CurveRef::Ref(g), mode: SweepMode::Corrigido, op: BoolOp::Add, thin: None }));
    let r = recompute(&d);
    match &r.status[&id] {
        Status::Ok => {}
        Status::Warning(m) => assert!(m.contains("orientação fixa") || m.contains("normal"), "{m}"),
        outro => panic!("{outro:?}"),
    }
    assert!(volume_total(&r) > 0.);
}
