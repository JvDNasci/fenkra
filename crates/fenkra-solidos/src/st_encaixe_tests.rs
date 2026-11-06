// SPDX-License-Identifier: LGPL-2.1-or-later
//! Testes do corte de encaixe (B155): fórmula fechada nos casos redondo e retangular, ausência de
//! invasão no modo automático, chanfro sobre o hospedeiro, orçamento e arquivos de saída.

use crate::st_dobra_corte::Camada;
use crate::st_dobra_saida::{dxf, svg};
use crate::st_encaixe::*;

/// Hospedeiro ao longo de Z a partir da origem; membro cortado saindo da origem a `graus` do eixo Z,
/// no plano XZ.
fn junta(host: Secao, b: Secao, graus: f64) -> (Membro, Membro) {
    let a = graus.to_radians();
    let h = Membro { secao: host, ponto: [0.; 3], x: [1., 0., 0.], y: [0., 1., 0.], para_dentro: [0., 0., 1.], centro: [0.; 2], comprimento: 2000. };
    let m = Membro { secao: b, ponto: [0.; 3], x: [a.cos(), 0., -a.sin()], y: [0., 1., 0.], para_dentro: [a.sin(), 0., a.cos()], centro: [0.; 2], comprimento: 800. };
    (m, h)
}

const R48: Secao = Secao::Redonda { d: 48.3, t: 3.2 };

fn quadrado(l: f64, t: f64) -> Secao {
    Secao::Retangular { b: l, h: l, t }
}

#[test]
fn boca_de_lobo_perpendicular_bate_com_a_formula() {
    let (m, h) = junta(R48, R48, 90.);
    let c = calcular(&m, &[h], Opcoes { orcamento: 100., ..Default::default() }).unwrap();
    let (r, ri) = (48.3 / 2., 48.3 / 2. - 3.2);
    for p in &c.pontos {
        let th = p.s / r;
        // a seção do membro cortado tem x ao longo do hospedeiro e y de lado: a posição lateral é r·sen θ
        let esperado = (r * r - (ri * th.sin()).powi(2)).sqrt();
        assert!((p.lambda - esperado).abs() < 1e-9, "s={} λ={} esperado={}", p.s, p.lambda, esperado);
        assert!(p.invasao < 1e-9, "o modo automático nunca invade");
    }
    assert!((c.lambda_max() - r).abs() < 1e-6, "no meio da boca o corte toca o eixo do hospedeiro");
    let n = c.pontos.len();
    for k in 1..n / 2 {
        assert!((c.pontos[k].lambda - c.pontos[n - k].lambda).abs() < 1e-9, "simétrica nos dois lados");
    }
}

#[test]
fn automatico_e_a_maior_das_duas_peles_e_alterna_a_parede() {
    let (m, h) = junta(Secao::Redonda { d: 60.3, t: 5. }, Secao::Redonda { d: 33.7, t: 2.6 }, 60.);
    let opc = |modo| Opcoes { modo, orcamento: 100., ..Default::default() };
    let auto = calcular(&m, &[h.clone()], opc(ModoParede::Automatico)).unwrap();
    let ext = calcular(&m, &[h.clone()], opc(ModoParede::Externa)).unwrap();
    let int = calcular(&m, &[h], opc(ModoParede::Interna)).unwrap();
    assert!(auto.invasao_max() < 1e-9);
    assert!(ext.invasao_max() > 0.05 || int.invasao_max() > 0.05, "alguma pele sozinha invade");
    for ((a, e), i) in auto.pontos.iter().zip(&ext.pontos).zip(&int.pontos) {
        assert!((a.lambda - e.lambda.max(i.lambda)).abs() < 1e-9, "tem de ser a maior das duas peles");
    }
    let [fe, fi, ..] = auto.fracao_por_parede();
    assert!(fe > 0. && fi > 0. && (fe + fi - 1.).abs() < 1e-9, "as duas paredes aparecem: fe={fe} fi={fi}");
    // a curva não tem degrau: a troca de pele acontece com abertura zero
    let salto = auto.pontos.windows(2).map(|w| (w[1].lambda - w[0].lambda).abs()).fold(0., f64::max);
    assert!(salto < 3., "salto de {salto} mm entre pontos vizinhos");
}

#[test]
fn entalhe_de_retangular_em_retangular_e_uma_reta() {
    // 30x30x2 chegando de lado num 40x40x3: a face do hospedeiro é plana, a 20 mm do eixo
    let (m, h) = junta(quadrado(40., 3.), quadrado(30., 2.), 90.);
    let c = calcular(&m, &[h], Opcoes::default()).unwrap();
    assert!((c.lambda_min() - 20.).abs() < 1e-9 && (c.lambda_max() - 20.).abs() < 1e-9, "{} {}", c.lambda_min(), c.lambda_max());
    assert!(c.folga_max() < 1e-9 && c.invasao_max() < 1e-9);
    assert!((c.perimetro - 120.).abs() < 1e-9);
    // com a folga de solda a ponta recua
    let (m, h) = junta(quadrado(40., 3.), quadrado(30., 2.), 90.);
    let c = calcular(&m, &[h], Opcoes { folga: 1.5, ..Default::default() }).unwrap();
    assert!((c.lambda_min() - 21.5).abs() < 1e-9);
}

#[test]
fn mitra_oblicua_de_retangular_tem_abertura_pelo_cosseno() {
    // a 45° a face do hospedeiro corta o tubo inclinado: λ varia linearmente com a posição
    let (m, h) = junta(quadrado(40., 3.), quadrado(30., 2.), 45.);
    let c = calcular(&m, &[h], Opcoes { orcamento: 100., ..Default::default() }).unwrap();
    assert!(c.invasao_max() < 1e-9);
    assert!(c.lambda_max() - c.lambda_min() > 20., "a esquadria inclinada percorre a largura do tubo");
    // 30 mm de largura a 45° da face dão 30 mm; a ponta do lado de dentro é a pele interna (13 em vez de 15), que
    // toca depois: 30 − 2 = 28 mm
    assert!((c.lambda_max() - c.lambda_min() - 28.).abs() < 1e-6, "{}", c.lambda_max() - c.lambda_min());
}

#[test]
fn com_chanfro_a_pele_externa_fica_sobre_o_hospedeiro() {
    let folga = 1.;
    let (m, h) = junta(R48, Secao::Redonda { d: 33.7, t: 2.6 }, 70.);
    let c = calcular(&m, &[h], Opcoes { perpendicular: false, folga, ..Default::default() }).unwrap();
    // desenrolando a curva (λ, s) e enrolando de volta, o ponto cai na superfície do hospedeiro
    for p in &c.pontos {
        let dist_eixo = p.p3[0].hypot(p.p3[1]);
        assert!((dist_eixo - (48.3 / 2. + folga)).abs() < 0.01, "s={} dist={}", p.s, dist_eixo);
        assert!(p.folga_raiz == 0. && p.invasao == 0.);
    }
    assert!(c.bisel_max() > 1., "o chanfro varia ao longo da curva");
}

#[test]
fn comprimento_de_corte_do_3d_e_do_desenho_batem() {
    let (m, h) = junta(R48, Secao::Redonda { d: 33.7, t: 2.6 }, 90.);
    let c = calcular(&m, &[h], Opcoes { perpendicular: false, ..Default::default() }).unwrap();
    let (plano, tri) = (c.comprimento_corte(), c.comprimento_3d());
    assert!((plano - tri).abs() / tri < 0.002, "plano {plano} 3D {tri}");
    let d = tubo_aberto("teste", 500., Some(&c), None);
    // a curva fecha na costura: o desenho acrescenta o último trecho até y = perímetro
    let ultimo = c.pontos.last().unwrap();
    let fecho = (c.perimetro - ultimo.s).hypot(c.pontos[0].lambda - ultimo.lambda);
    assert!((d.comprimento_de_corte() - plano - fecho).abs() < 1e-9);
}

#[test]
fn orcamento_de_folga_marca_o_trecho_e_sai_no_dxf_e_no_svg() {
    let (m, h) = junta(Secao::Redonda { d: 88.9, t: 4. }, Secao::Redonda { d: 60.3, t: 5. }, 50.);
    let largo = calcular(&m, &[h.clone()], Opcoes { orcamento: 50., ..Default::default() }).unwrap();
    assert!(largo.trechos_acima().is_empty());
    let apertado = calcular(&m, &[h], Opcoes { orcamento: 0.2, ..Default::default() }).unwrap();
    assert!(!apertado.trechos_acima().is_empty(), "abertura máxima {}", apertado.folga_max());
    let d = tubo_aberto("Encaixe", 600., Some(&apertado), Some(&largo));
    assert!(d.tracos.iter().any(|t| t.camada == Camada::Marcacao), "o trecho acima do orçamento é marcado");
    let (s, x) = (svg(&d), dxf(&d));
    assert!(s.contains("CORTE") && s.contains("<polyline"));
    assert!(x.contains("CORTE"));
    assert!(d.comprimento_de_corte() > apertado.comprimento_corte() + largo.comprimento_corte() - 1., "as duas pontas entram no comprimento de corte");
}

#[test]
fn varios_hospedeiros_valem_pelo_mais_alto() {
    let (m, h1) = junta(quadrado(40., 3.), quadrado(30., 2.), 90.);
    let mut h2 = h1.clone();
    h2.secao = quadrado(60., 3.);
    let c = calcular(&m, &[h1, h2], Opcoes::default()).unwrap();
    assert!((c.lambda_min() - 30.).abs() < 1e-9, "o hospedeiro maior segura a ponta mais longe");
}

#[test]
fn eixos_paralelos_dao_erro() {
    let (m, h) = junta(R48, R48, 0.);
    assert!(calcular(&m, &[h], Opcoes::default()).is_err());
}
