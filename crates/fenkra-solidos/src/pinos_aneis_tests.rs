// SPDX-License-Identifier: LGPL-2.1-or-later
//! Testes de pinos, anéis de retenção e pino com colar (B63): cotas contra as fontes e sólido gerado.

use crate::biblioteca::{self, familia, pino_colar_pela_pega, tamanho, Camada};
use crate::fixadores::{fixador_documento, gerar};
use crate::{recompute, Status};
use fenkra_core::*;
use std::f64::consts::PI;

const POL: f64 = 25.4;

/// Volume, mínimo e máximo do corpo único, com todos os comandos em Ok.
fn medir(doc: &Document) -> (f64, [f64; 3], [f64; 3]) {
    let rc = recompute(doc);
    assert!(rc.param_error.is_none(), "{:?}", rc.param_error);
    assert!(doc.features.iter().all(|f| rc.status[&f.id] == Status::Ok), "{}", doc.name);
    assert_eq!(rc.bodies.len(), 1, "{}", doc.name);
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for p in rc.bodies[0].mesh.positions.chunks_exact(3) {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i] as f64);
            hi[i] = hi[i].max(p[i] as f64);
        }
    }
    (rc.bodies[0].mass.volume, lo, hi)
}

fn v(id: &str, tam: &str, c: &str) -> f64 {
    tamanho(id, tam).unwrap().valor(c).unwrap()
}

#[test]
fn aneis_respeitam_a_ordem_dos_diametros_da_norma() {
    // anel externo: livre d3 < fundo do canal d2 < eixo d1; anel interno: furo d1 < canal d2 < livre d3
    for t in &familia("din471").unwrap().tamanhos {
        let d1 = t.diametro_nominal().unwrap();
        assert!(t.valor("d3").unwrap() < t.valor("d2").unwrap() && t.valor("d2").unwrap() < d1, "471 {}", t.designacao);
        assert!(t.valor("m").unwrap() >= t.valor("s").unwrap(), "canal mais largo que a espessura, 471 {}", t.designacao);
    }
    for t in &familia("din472").unwrap().tamanhos {
        let d1 = t.diametro_nominal().unwrap();
        assert!(d1 < t.valor("d2").unwrap() && t.valor("d2").unwrap() < t.valor("d3").unwrap(), "472 {}", t.designacao);
    }
}

#[test]
fn cotas_dos_aneis_batem_com_as_fontes() {
    // linhas lidas em duas fontes (fasten.it e Westfield)
    let f = familia("din471").unwrap();
    let t = f.tamanho("Ø30").unwrap();
    let esperado = [("s", 1.5), ("d3", 27.9), ("a", 5.), ("b", 3.5), ("d5", 2.), ("d2", 28.6), ("m", 1.6), ("n", 2.1)];
    for (c, x) in esperado {
        assert!((t.valor(c).unwrap() - x).abs() < 1e-9, "471 Ø30 {c}");
    }
    assert_eq!(t.confirmado_por, 2);
    let t = familia("din472").unwrap().tamanho("Ø50").unwrap();
    for (c, x) in [("s", 2.), ("d3", 54.2), ("a", 6.5), ("b", 4.6), ("d5", 2.5), ("d2", 53.), ("m", 2.15), ("n", 4.5)] {
        assert!((t.valor(c).unwrap() - x).abs() < 1e-9, "472 Ø50 {c}");
    }
    // linha com uma fonte só não passa por conferida
    let fraca: Vec<_> = f.tamanhos.iter().filter(|t| t.confirmado_por < 2).collect();
    assert!(!fraca.is_empty() && fraca.iter().all(|t| t.nota.as_deref().is_some_and(|n| n.contains("PENDENTE"))));
}

#[test]
fn anel_externo_din471_d30_tem_coroa_de_27_9_a_34_9() {
    let doc = fixador_documento("din471", "Ø30", None).unwrap();
    let (vol, lo, hi) = medir(&doc);
    let (s, d3, b) = (1.5, 27.9, 3.5);
    let de = d3 + 2. * b;
    assert!((lo[2] + s).abs() < 1e-3 && hi[2].abs() < 1e-3, "{lo:?} {hi:?}");
    assert!((hi[0] - lo[0] - de).abs() < 0.1, "Ø externo {de}: {}", hi[0] - lo[0]);
    let esp = PI / 4. * (de * de - d3 * d3) * s;
    // a malha subestima os cilindros em cerca de 0,5%
    assert!((vol - esp).abs() < 0.01 * esp, "volume {vol} esperado {esp}");
}

#[test]
fn anel_interno_din472_d50_tem_o_externo_igual_ao_livre() {
    let doc = fixador_documento("din472", "Ø50", None).unwrap();
    let (vol, lo, hi) = medir(&doc);
    let (s, d3, b) = (2., 54.2, 4.6);
    let di = d3 - 2. * b;
    assert!((lo[2] + s).abs() < 1e-3);
    assert!((hi[0] - lo[0] - d3).abs() < 0.1);
    let esp = PI / 4. * (d3 * d3 - di * di) * s;
    assert!((vol - esp).abs() < 0.01 * esp, "volume {vol} esperado {esp}");
}

#[test]
fn pino_cilindrico_din7_d6_x20() {
    assert!((v("din7", "Ø6", "c") - 0.9).abs() < 1e-9 && tamanho("din7", "Ø6").unwrap().confirmado_por == 3);
    let doc = fixador_documento("din7", "Ø6", Some(20.)).unwrap();
    let (vol, lo, hi) = medir(&doc);
    assert!((lo[2] + 20.).abs() < 1e-3 && hi[2].abs() < 1e-3);
    assert!((hi[0] - lo[0] - 6.).abs() < 0.05);
    let esp = PI * 9. * 20.;
    assert!((vol - esp).abs() < 0.01 * esp, "{vol} {esp}");
}

#[test]
fn pino_elastico_iso8752_d6_e_um_tubo_de_parede_1_2() {
    let f = familia("iso8752").unwrap();
    let t = f.tamanho("Ø6").unwrap();
    // três fontes concordam; a série de comprimentos da SPIROL vai de 14 a 70 mm
    assert_eq!((t.confirmado_por, t.valor("s"), t.valor("D1max"), t.valor("d2")), (3, Some(1.2), Some(6.7), Some(4.0)));
    assert_eq!(f.limites_de_comprimento(t), Some((14., 70.)));
    assert!(fixador_documento("iso8752", "Ø6", Some(13.)).is_err(), "abaixo da série");
    let doc = fixador_documento("iso8752", "Ø6", Some(30.)).unwrap();
    let (vol, lo, hi) = medir(&doc);
    assert!((lo[2] + 30.).abs() < 1e-3 && hi[2].abs() < 1e-3);
    let esp = PI / 4. * (36. - 3.6 * 3.6) * 30.;
    assert!((vol - esp).abs() < 0.02 * esp, "{vol} {esp}");
}

#[test]
fn cupilha_din94_d3_2_tem_hastes_e_olho() {
    let t = tamanho("din94", "Ø3,2").unwrap();
    assert_eq!((t.valor("dmax"), t.valor("cmax"), t.valor("b")), (Some(2.9), Some(5.8), Some(6.4)));
    assert_eq!(t.confirmado_por, 2);
    let doc = fixador_documento("din94", "Ø3,2", Some(25.)).unwrap();
    let (vol, lo, hi) = medir(&doc);
    assert!((lo[2] + 25.).abs() < 1e-3 && (hi[2] - 2.9).abs() < 1e-3, "{lo:?} {hi:?}");
    assert!((hi[0] - lo[0] - 5.8).abs() < 0.05, "largura = Ø do olho");
    let esp = PI * (1.45f64).powi(2) * 25. + PI * (2.9f64 / 2.).powi(2) * 0. + PI * (5.8f64 / 2.).powi(2) * 2.9;
    assert!((vol - esp).abs() < 0.02 * esp, "{vol} {esp}");
    // o d 16 diverge entre as fontes: uma só conta
    let t16 = tamanho("din94", "Ø16").unwrap();
    assert!(t16.confirmado_por == 1 && t16.nota.as_deref().unwrap().contains("PENDENTE"));
}

#[test]
fn hi_lok_hl18pb5_tem_l_pela_pega() {
    let f = familia("hl18").unwrap();
    let t = f.tamanho("HL18PB5").unwrap();
    // -3: pega 3/16 pol, L 0,500 pol; -14: pega 14/16 pol, L 1,187 pol (listas de distribuidor)
    let l = |n: f64| (n / 16. + 0.3125) * POL;
    assert!(t.comprimentos.iter().any(|x| (x / POL - 0.5).abs() < 1e-3));
    assert!(t.comprimentos.iter().any(|x| (x / POL - 1.1875).abs() < 1e-3));
    // pega de 3/16 pol pede o -3; um pouco mais de pega pede o -4
    assert!((pino_colar_pela_pega(f, t, 3. / 16. * POL).unwrap() - l(3.)).abs() < 1e-2);
    assert!((pino_colar_pela_pega(f, t, 3. / 16. * POL + 0.1).unwrap() - l(4.)).abs() < 1e-2);
    assert!(pino_colar_pela_pega(f, t, 20. * POL).is_none());
    let doc = fixador_documento("hl18", "HL18PB5", Some(l(3.))).unwrap();
    let (vol, lo, hi) = medir(&doc);
    let (dk, k, d) = (0.252 * POL, 0.042 * POL, 0.163 * POL);
    assert!((lo[2] + l(3.)).abs() < 1e-2 && (hi[2] - k).abs() < 1e-3, "{lo:?} {hi:?}");
    assert!((hi[0] - lo[0] - dk).abs() < 0.05);
    let esp = PI * (dk / 2.).powi(2) * k + PI * (d / 2.).powi(2) * l(3.);
    assert!((vol - esp).abs() < 0.01 * esp, "{vol} {esp}");
    // PB6 tem uma fonte só e diz isso
    let t6 = f.tamanho("HL18PB6").unwrap();
    assert!(t6.confirmado_por == 1 && t6.nota.as_deref().unwrap().contains("PENDENTE"));
}

#[test]
fn colar_entra_abaixo_da_pega_maxima() {
    // família de usuário (medidas nossas) com colar: Dc e hc entram no sólido
    let json = r#"{"id":"pc_teste","nome":"Pino com colar de teste","norma":"Teste B63","revisao":"1","gerador":"pino_colar","tipo":"pino_com_colar",
      "nivel":"A","licenca":"CC BY 4.0 (dados)","ilustrativo":true,
      "fontes":[{"rotulo":"medida nossa","url":"https://example.org/teste"}],
      "campos":["dk","k","sobra","Dc","hc"],
      "tamanhos":[{"designacao":"T5","d_nominal":4,"valores":{"dk":7,"k":1,"sobra":8,"Dc":6,"hc":5},"confirmado_por":0,"comprimentos":[10,12,14]}]}"#;
    let lib = biblioteca::Biblioteca::de_jsons(Camada::Usuario, &[("pc.json", json)]).unwrap();
    let fam = lib.familia("pc_teste").unwrap();
    let doc = gerar(fam, "T5", Some(12.)).unwrap();
    let (vol, lo, hi) = medir(&doc);
    assert!((lo[2] + 12.).abs() < 1e-3 && (hi[2] - 1.).abs() < 1e-3, "{lo:?} {hi:?}");
    assert!((hi[0] - lo[0] - 7.).abs() < 0.05, "a cabeça é a parte mais larga");
    // colar de Ø6 × 5 logo abaixo da pega máxima (L − sobra = 4): de z = −9 a −4
    let sem_colar = PI * 3.5f64.powi(2) * 1. + PI * 4. * 12.;
    let colar = PI * (9. - 4.) * 5.;
    assert!((vol - (sem_colar + colar)).abs() < 0.015 * (sem_colar + colar), "{vol} {}", sem_colar + colar);
    // colar mais alto que a sobra é recusado
    let ruim = json.replace(r#""hc":5"#, r#""hc":9"#);
    let lib = biblioteca::Biblioteca::de_jsons(Camada::Usuario, &[("pc.json", ruim.as_str())]).unwrap();
    assert!(gerar(lib.familia("pc_teste").unwrap(), "T5", Some(12.)).is_err());
}
