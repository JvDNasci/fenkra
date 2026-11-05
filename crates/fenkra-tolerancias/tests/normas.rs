// SPDX-License-Identifier: LGPL-2.1-or-later
//! Conferência dos dados contra valores publicados e contra as fórmulas da própria norma.

use fenkra_tolerancias::ajustes::{formatar_desvio_mm, textos_desvios};
use fenkra_tolerancias::desvios::{limites_eixo, limites_furo};
use fenkra_tolerancias::geral::{ClasseForma, ClasseGeral};
use fenkra_tolerancias::it::FAIXAS_IT;
use fenkra_tolerancias::*;

fn c(s: &str) -> ClasseTol {
    ClasseTol::parse(s).unwrap_or_else(|| panic!("classe {s}"))
}

fn dev(s: &str, d: f64) -> (f64, f64) {
    let x = c(s).desvios(d).unwrap_or_else(|| panic!("{s} em {d}"));
    (x.sup_um, x.inf_um)
}

#[test]
fn graus_it_publicados() {
    let it = |s: &str, d: f64| ItGrau::parse(s).unwrap().valor_um(d).unwrap();
    assert_eq!(it("IT7", 20.), 21.);
    assert_eq!(it("IT7", 40.), 25.);
    assert_eq!(it("IT6", 25.), 13.);
    assert_eq!(it("IT8", 100.), 54.);
    assert_eq!(it("IT11", 3.), 60.);
    assert_eq!(it("IT01", 2.), 0.3);
    // limites de faixa: 30 pertence à faixa 18–30, 30,001 à 30–50
    assert_eq!(it("IT7", 30.), 21.);
    assert_eq!(it("IT7", 30.001), 25.);
    // IT14–IT18 não existem até 1 mm; fora de 0 < D ≤ 500 não há valor
    assert!(ItGrau::parse("IT14").unwrap().valor_um(0.8).is_none());
    assert!(ItGrau::parse("IT7").unwrap().valor_um(501.).is_none());
    assert!(ItGrau::parse("IT7").unwrap().valor_um(0.).is_none());
    assert_eq!(ItGrau::parse("it7").unwrap().nome(), "IT7");
    assert_eq!(ItGrau::para_tolerancia(22., 20.).unwrap().nome(), "IT7");
}

#[test]
fn graus_it_seguem_a_formula_da_norma() {
    // IT5…IT18 = fator × i, i = 0,45·∛D + 0,001·D (D = média geométrica da faixa, µm).
    // Os valores tabelados são arredondados à mão pela norma: tolera 8 % + 0,6 µm.
    let fatores = [(5, 7.), (6, 10.), (7, 16.), (8, 25.), (9, 40.), (10, 64.), (11, 100.), (12, 160.), (13, 250.), (14, 400.), (15, 640.), (16, 1000.), (17, 1600.), (18, 2500.)];
    for f in 1..13 {
        let (lo, hi) = (FAIXAS_IT[f - 1], FAIXAS_IT[f]);
        let dg = (lo * hi).sqrt();
        let i = 0.45 * dg.cbrt() + 0.001 * dg;
        for (g, k) in fatores {
            let v = ItGrau::novo(g).unwrap().valor_um(hi).unwrap();
            let esperado = k * i;
            assert!((v - esperado).abs() <= 0.08 * esperado + 0.6, "IT{g} {lo}-{hi}: tabela {v}, fórmula {esperado:.2}");
        }
    }
}

#[test]
fn graus_it_crescem_com_o_grau_e_com_a_faixa() {
    let grau = |g: usize| match g {
        0 => ItGrau::IT01,
        1 => ItGrau::IT0,
        n => ItGrau::novo(n as u8 - 1).unwrap(),
    };
    for d in [2., 5., 8., 15., 25., 45., 70., 100., 150., 220., 300., 380., 450.] {
        let mut ant = 0.;
        for g in 0..20 {
            let v = grau(g).valor_um(d).unwrap();
            assert!(v > ant, "IT cresce com o grau: d={d} g={g}");
            ant = v;
        }
    }
    for g in 0..20 {
        let mut ant = 0.;
        for d in [2., 5., 8., 15., 25., 45., 70., 100., 150., 220., 300., 380., 450.] {
            let v = grau(g).valor_um(d).unwrap();
            assert!(v >= ant, "IT cresce com a faixa: g={g} d={d}");
            ant = v;
        }
    }
}

#[test]
fn eixos_e_furos_publicados() {
    // exemplos clássicos: Ø20 H7 = +21/0, Ø20 f7 = −20/−41, Ø25 g6 = −7/−20
    assert_eq!(dev("H7", 20.), (21., 0.));
    assert_eq!(dev("f7", 20.), (-20., -41.));
    assert_eq!(dev("g6", 25.), (-7., -20.));
    assert_eq!(dev("h6", 10.), (0., -9.));
    assert_eq!(dev("k6", 20.), (15., 2.));
    assert_eq!(dev("m6", 20.), (21., 8.));
    assert_eq!(dev("n6", 20.), (28., 15.));
    assert_eq!(dev("p6", 20.), (35., 22.));
    assert_eq!(dev("s6", 20.), (48., 35.));
    assert_eq!(dev("e8", 30.), (-40., -73.));
    assert_eq!(dev("d9", 50.), (-80., -142.));
    assert_eq!(dev("js6", 25.), (6.5, -6.5));
    assert_eq!(dev("js7", 40.), (12., -13.), "IT7 ímpar usa (IT−1)/2: 25 → +12/−13");
    assert_eq!(dev("j6", 25.), (9., -4.));
    // furos
    assert_eq!(dev("G7", 20.), (28., 7.));
    assert_eq!(dev("F8", 20.), (53., 20.));
    assert_eq!(dev("D10", 20.), (149., 65.));
    assert_eq!(dev("J6", 25.), (8., -5.));
    assert_eq!(dev("J7", 25.), (12., -9.));
    assert_eq!(dev("K7", 25.), (6., -15.), "ES = −2 + Δ(8) = +6");
    assert_eq!(dev("M7", 15.), (0., -18.), "ES = −7 + Δ(7) = 0");
    assert_eq!(dev("N7", 15.), (-5., -23.), "ES = −12 + Δ(7) = −5");
    assert_eq!(dev("N9", 8.), (0., -36.), "acima de IT8 e de 3 mm, N tem ES = 0");
    assert_eq!(dev("N9", 2.), (-4., -29.), "até 3 mm o N de IT9 mantém −ei");
    assert_eq!(dev("P7", 4.), (-8., -20.));
    assert_eq!(dev("P6", 4.), (-9., -17.));
    assert_eq!(dev("R7", 40.), (-25., -50.), "R7 = −34 + Δ(9)");
    assert_eq!(dev("S7", 20.), (-35. + 8., -35. + 8. - 21.), "S7: −35 + Δ(8)");
}

#[test]
fn furo_e_eixo_espelham_nas_letras_a_h() {
    for l in ["A", "B", "C", "D", "E", "F", "G", "H"] {
        for g in 5..=11u8 {
            for d in [2., 7., 12., 25., 45., 90., 200., 400.] {
                let gr = ItGrau::novo(g).unwrap();
                let (Some(ei), Some(es)) = (limites_furo(Letra::parse(l).unwrap(), gr, d), limites_eixo(Letra::parse(l).unwrap(), gr, d)) else { continue };
                assert_eq!(ei.inf_um, -es.sup_um, "{l}{g} em {d}");
            }
        }
    }
}

#[test]
fn desvios_fundamentais_seguem_as_formulas() {
    // d, e, f, g: −16·D^0,44, −11·D^0,41, −5,5·D^0,41, −2,5·D^0,34 (D média geométrica da subfaixa).
    // A norma arredonda a valores preferidos: tolera 8 % + 1 µm.
    let formulas: [(Letra, fn(f64) -> f64); 4] = [
        (Letra::D, |d| -16. * d.powf(0.44)),
        (Letra::E, |d| -11. * d.powf(0.41)),
        (Letra::F, |d| -5.5 * d.powf(0.41)),
        (Letra::G, |d| -2.5 * d.powf(0.34)),
    ];
    for d in [3.5, 8., 12., 16., 20., 28., 35., 45., 60., 75., 90., 110., 130., 150., 170., 190., 210., 240., 270., 300., 340., 380., 420., 480.] {
        let dg = desvios_sub_media(d);
        for (l, f) in formulas {
            let es = limites_eixo(l, ItGrau::novo(7).unwrap(), d).unwrap().sup_um;
            let esperado = f(dg);
            assert!((es - esperado).abs() <= 0.08 * esperado.abs() + 1., "{l:?} em {d}: tabela {es}, fórmula {esperado:.1}");
        }
    }
    // m, n, p: ei = IT7 − IT6 (m), 5·D^0,34 (n), IT7 + 0…5 (p, arredondado a valores preferidos)
    for d in [8., 25., 60., 150., 300.] {
        let it7 = ItGrau::novo(7).unwrap().valor_um(d).unwrap();
        let it6 = ItGrau::novo(6).unwrap().valor_um(d).unwrap();
        let m = limites_eixo(Letra::M, ItGrau::novo(6).unwrap(), d).unwrap().inf_um;
        assert!((m - (it7 - it6)).abs() <= 2., "m6 em {d}: {m} contra IT7−IT6 = {}", it7 - it6);
        let p = limites_eixo(Letra::P, ItGrau::novo(6).unwrap(), d).unwrap().inf_um;
        assert!((p - it7).abs() <= 6., "p em {d}: {p} contra IT7 = {it7}");
    }
}

fn desvios_sub_media(d: f64) -> f64 {
    const SUB: [f64; 25] = [3., 6., 10., 14., 18., 24., 30., 40., 50., 65., 80., 100., 120., 140., 160., 180., 200., 225., 250., 280., 315., 355., 400., 450., 500.];
    let i = SUB.iter().position(|&h| d <= h).unwrap();
    let lo = if i == 0 { 1. } else { SUB[i - 1] };
    (lo * SUB[i]).sqrt()
}

#[test]
fn todas_as_letras_tem_zona_coerente() {
    // sup − inf = IT em toda combinação definida; nada de NaN
    let mut total = 0;
    for l in Letra::TODAS {
        for g in 1..=18u8 {
            for d in [1., 2.5, 5., 9., 13., 17., 22., 28., 35., 48., 60., 75., 90., 110., 130., 150., 170., 190., 210., 240., 260., 300., 330., 380., 430., 490.] {
                let gr = ItGrau::novo(g).unwrap();
                for f in [limites_furo(l, gr, d), limites_eixo(l, gr, d)].into_iter().flatten() {
                    let it = gr.valor_um(d).unwrap();
                    assert!((f.sup_um - f.inf_um - it).abs() < 1e-9, "{l:?} IT{g} D{d}");
                    assert!(f.sup_um.is_finite() && f.inf_um.is_finite());
                    total += 1;
                }
            }
        }
    }
    assert!(total > 4000, "cobertura: {total}");
    // combinações que a norma não tem
    assert!(limites_eixo(Letra::J, ItGrau::novo(8).unwrap(), 20.).is_none());
    assert!(limites_furo(Letra::J, ItGrau::novo(5).unwrap(), 20.).is_none());
    assert!(limites_eixo(Letra::CD, ItGrau::novo(7).unwrap(), 20.).is_none(), "cd só até 10 mm");
    assert!(limites_eixo(Letra::T, ItGrau::novo(7).unwrap(), 12.).is_none(), "t só a partir de 24 mm");
    assert!(limites_eixo(Letra::H, ItGrau::novo(7).unwrap(), 600.).is_none());
}

#[test]
fn ajustes_tipos_e_folgas() {
    let a = Ajuste::parse("H7/g6").unwrap();
    let r = a.analisar(20.).unwrap();
    // furo +21/0, eixo −7/−20 → folga máx 41, mín 7
    assert_eq!((r.folga_max_um, r.folga_min_um), (41., 7.));
    assert_eq!(r.tipo, TipoAjuste::Folga);
    assert_eq!(a.sistema(), "furo-base");
    assert_eq!(a.texto(), "H7/g6");

    let r = Ajuste::parse("H7/p6").unwrap().analisar(20.).unwrap();
    // eixo +35/+22 → folga máx −1, mín −35
    assert_eq!((r.folga_max_um, r.folga_min_um), (-1., -35.));
    assert_eq!(r.tipo, TipoAjuste::Interferencia);

    let r = Ajuste::parse("H7/k6").unwrap().analisar(20.).unwrap();
    assert_eq!((r.folga_max_um, r.folga_min_um), (19., -15.));
    assert_eq!(r.tipo, TipoAjuste::Incerto);

    let r = Ajuste::parse("H7-h6").unwrap().analisar(20.).unwrap();
    assert_eq!(r.folga_min_um, 0.);
    assert_eq!(r.tipo, TipoAjuste::Folga, "folga mínima zero ainda é folga");
    assert_eq!(Ajuste::parse("G7/h6").unwrap().sistema(), "eixo-base");

    // furo com letra minúscula ou eixo maiúsculo não é ajuste
    assert!(Ajuste::parse("h7/g6").is_none());
    assert!(Ajuste::parse("H7/G6").is_none());
    assert!(Ajuste::parse("H7").is_none());
    assert!(ClasseTol::parse("Hh7").is_none());
    assert!(ClasseTol::parse("H").is_none());
}

#[test]
fn limites_para_ap242_e_textos() {
    let l = LimitesAp242::de(&c("H7"), 20.).unwrap();
    assert_eq!((l.variancia_forma.as_str(), l.grau.as_str(), l.furo), ("H", "IT7", true));
    assert_eq!((l.desvio_sup_mm, l.desvio_inf_mm), (0.021, 0.));
    assert_eq!((l.limite_max_mm, l.limite_min_mm), (20.021, 20.));
    let e = LimitesAp242::de(&c("g6"), 25.).unwrap();
    assert_eq!((e.limite_max_mm, e.limite_min_mm), (24.993, 24.98));
    assert_eq!(e.fonte, "ISO 286-1");
    assert_eq!(textos_desvios(&c("H7"), 20.).unwrap(), ("+0,021".to_string(), "0".to_string()));
    assert_eq!(textos_desvios(&c("f7"), 20.).unwrap(), ("-0,020".to_string(), "-0,041".to_string()));
    assert_eq!(formatar_desvio_mm(0.0065), "+0,0065");
}

#[test]
fn tolerancias_gerais_iso_2768() {
    use ClasseGeral::*;
    assert_eq!(M.linear_mm(10.), Some(0.2));
    assert_eq!(M.linear_mm(6.), Some(0.1), "6 pertence à faixa 3–6");
    assert_eq!(M.linear_mm(6.01), Some(0.2));
    assert_eq!(F.linear_mm(100.), Some(0.15));
    assert_eq!(C.linear_mm(250.), Some(1.2));
    assert_eq!(V.linear_mm(3500.), Some(8.));
    assert_eq!(M.linear_mm(2000.), Some(1.2));
    assert_eq!(M.linear_mm(2000.5), Some(2.));
    assert_eq!(F.linear_mm(3000.), None, "classe f não vai acima de 1000");
    assert_eq!(V.linear_mm(2.), None, "classe v começa em 3 mm");
    assert_eq!(M.linear_mm(0.3), None);
    assert_eq!(M.linear_mm(5000.), None);
    // arestas quebradas
    assert_eq!(M.aresta_quebrada_mm(2.), Some(0.2));
    assert_eq!(M.aresta_quebrada_mm(5.), Some(0.5));
    assert_eq!(M.aresta_quebrada_mm(20.), Some(1.));
    assert_eq!(C.aresta_quebrada_mm(20.), Some(2.));
    // angulares
    assert_eq!(M.angular_min(8.), Some(60.));
    assert_eq!(M.angular_min(30.), Some(30.));
    assert_eq!(C.angular_min(100.), Some(30.));
    assert_eq!(V.angular_graus(500.), Some(20. / 60.));
    // forma e posição
    assert_eq!(ClasseForma::K.retitude_planeza_mm(50.), Some(0.2));
    assert_eq!(ClasseForma::H.retitude_planeza_mm(10.), Some(0.02));
    assert_eq!(ClasseForma::L.retitude_planeza_mm(2000.), Some(1.6));
    assert_eq!(ClasseForma::L.retitude_planeza_mm(4000.), None);
    assert_eq!(ClasseForma::H.perpendicularidade_mm(250.), Some(0.3));
    assert_eq!(ClasseForma::K.perpendicularidade_mm(50.), Some(0.4));
    assert_eq!(ClasseForma::L.simetria_mm(500.), Some(1.5));
    assert_eq!(ClasseForma::H.simetria_mm(2500.), Some(0.5));
    assert_eq!(ClasseForma::K.batimento_mm(), 0.2);
}

#[test]
fn it_para_tolerancia() {
    // uma zona de 100 µm em Ø30 comporta IT10 (84 µm); IT11 (130) já não cabe
    assert_eq!(ItGrau::para_tolerancia(100., 30.).unwrap().nome(), "IT10");
    assert!(ItGrau::para_tolerancia(0.1, 30.).is_none());
}
