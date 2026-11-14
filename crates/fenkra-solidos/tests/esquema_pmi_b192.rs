// SPDX-License-Identifier: LGPL-2.1-or-later
//! B192: esquema de cotagem automático sobre a placa de exemplo (120 × 72 × 10, quatro furos Ø6,6
//! com rebaixo Ø11 e um rasgo). Confere o DRF escolhido, as cotas e tolerâncias geradas em cada
//! modo, que tudo nasce na pasta do assistente e que regenerar não apaga a edição da pessoa.

mod common;

use common::*;
use fenkra_core::pmi::{GdtChar, Pmi, PmiFrame, PmiKind};
use fenkra_solidos::esquema_pmi::{self, ModoEsquema, RegrasEsquema};
use fenkra_solidos::{final_shape, recompute};

fn medida(p: &Pmi) -> f64 {
    p.measure(&p.anchors).unwrap_or(f64::NAN)
}

fn achar<'a>(l: &'a [Pmi], nome: &str) -> &'a Pmi {
    l.iter().find(|p| p.name == nome).unwrap_or_else(|| panic!("sem {nome}: {:?}", l.iter().map(|p| &p.name).collect::<Vec<_>>()))
}

#[test]
fn mais_menos_na_placa_drf_furos_rasgo_e_totais() {
    if !kernel_disponivel() {
        return;
    }
    let mut doc = fenkra_solidos::example_document();
    let r = recompute(&doc);
    assert!(problema_de_status(&doc, &r).is_none(), "{:?}", problema_de_status(&doc, &r));
    let s = final_shape(&doc).unwrap();
    let (lo, hi) = s.bbox().unwrap();
    let tam = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    assert!((tam[0] - 120.).abs() < 1e-3 && (tam[1] - 72.).abs() < 1e-3 && (tam[2] - 10.).abs() < 1e-3, "{tam:?}");
    let vol = volume_total(&r);
    assert!(vol > 0.8 * 120. * 72. * 10. && vol < 120. * 72. * 10., "{vol}");

    let regras = RegrasEsquema::default();
    let (drf, prop) = esquema_pmi::propor(&doc, &regras).unwrap();
    // A: a face de baixo (empata com a de cima, fica a de apoio); B e C: as laterais maiores
    assert_eq!(drf.len(), 3);
    assert_eq!(drf[0].normal, [0., 0., -1.]);
    // a face de baixo inteira menos as bocas dos furos e o rasgo
    assert!(drf[0].area < 120. * 72. && drf[0].area > 0.7 * 120. * 72., "{:?}", drf[0]);
    assert!(drf[1].normal[1].abs() > 0.999, "B na lateral de 120 mm: {:?}", drf[1]);
    assert!(drf[2].normal[0].abs() > 0.999, "C na lateral de 72 mm: {:?}", drf[2]);
    assert!(prop.iter().all(esquema_pmi::gerada), "tudo na pasta do assistente");

    // totais com ±0,1
    let totais: Vec<f64> = ["Total A", "Total B", "Total C"].iter().map(|n| medida(achar(&prop, n))).collect();
    assert!((totais[0] - 10.).abs() < 1e-6 && (totais[1] - 72.).abs() < 1e-6 && (totais[2] - 120.).abs() < 1e-6, "{totais:?}");
    assert_eq!(achar(&prop, "Total A").tol_plus, "0,1");
    // os quatro furos num grupo só, com a classe da regra e o rebaixo na chamada
    let furo = prop.iter().find(|p| p.name.starts_with("Furo ")).unwrap();
    assert!(matches!(furo.kind, PmiKind::Diameter));
    assert!((medida(furo) - 6.6).abs() < 1e-3, "{}", medida(furo));
    assert_eq!(furo.style.prefix, "4× ");
    assert!(furo.style.suffix.contains("H8") && furo.style.suffix.contains("⌴Ø11"), "{}", furo.style.suffix);
    assert_eq!(prop.iter().filter(|p| p.name.starts_with("Furo ")).count(), 1);
    // localização do primeiro furo a partir de B e de C, com ±
    let locais: Vec<&Pmi> = prop.iter().filter(|p| p.name.starts_with("Local ")).collect();
    assert_eq!(locais.len(), 2, "{:?}", locais.iter().map(|p| &p.name).collect::<Vec<_>>());
    assert!(locais.iter().all(|p| p.tol_plus == "0,1" && medida(p) > 1. && medida(p) < 119.));
    // o rasgo: largura entre as paredes que se encaram
    let rasgos: Vec<&Pmi> = prop.iter().filter(|p| p.name.starts_with("Rasgo ")).collect();
    assert!(!rasgos.is_empty(), "o rasgo da placa");
    assert!(rasgos.iter().all(|p| medida(p) > 0. && medida(p) < 72.));
    // sem GD&T no modo Mais/Menos
    assert!(!prop.iter().any(|p| matches!(p.kind, PmiKind::Gdt { .. })));

    // aplica, edita, regenera: nada duplica e a edição fica
    let r1 = esquema_pmi::aplicar(&mut doc, prop.clone(), false);
    assert_eq!((r1.novas, r1.mantidas, r1.removidas), (prop.len(), 0, 0));
    let total = doc.pmi.iter_mut().find(|p| p.name == "Total A").unwrap();
    total.tol_plus = "0,05".into();
    let r2 = esquema_pmi::gerar(&mut doc, &regras, false).unwrap();
    assert_eq!((r2.novas, r2.removidas), (0, 0));
    assert_eq!(doc.pmi.len(), prop.len());
    assert_eq!(achar(&doc.pmi, "Total A").tol_plus, "0,05");
    // as anotações do esquema não mexem no sólido
    let r = recompute(&doc);
    assert!(problema_de_status(&doc, &r).is_none());
    assert!((volume_total(&r) - vol).abs() < 1e-6);
}

#[test]
fn geometrico_troca_totais_por_planeza_perpendicularidade_e_posicao() {
    if !kernel_disponivel() {
        return;
    }
    let mut doc = fenkra_solidos::example_document();
    esquema_pmi::gerar(&mut doc, &RegrasEsquema::default(), false).unwrap();
    let antes = doc.pmi.len();
    let regras = RegrasEsquema { modo: ModoEsquema::Geometrico, ..Default::default() };
    let r = esquema_pmi::gerar(&mut doc, &regras, false).unwrap();
    // saem os três totais e as duas localizações com ±; entram planeza, duas perpendicularidades,
    // a posição e as duas localizações básicas
    assert_eq!(r.removidas, 5, "{r:?}");
    assert_eq!(r.novas, 6, "{r:?}");
    assert_eq!(doc.pmi.len(), antes + 1);
    let gdt = |ch: GdtChar| doc.pmi.iter().filter(move |p| matches!(&p.kind, PmiKind::Gdt { ch: c, .. } if *c == ch)).collect::<Vec<_>>();
    let pl = gdt(GdtChar::Planeza);
    assert_eq!(pl.len(), 1);
    assert!(matches!(&pl[0].kind, PmiKind::Gdt { tol, datums, .. } if tol == "0,05" && datums.is_empty()));
    let pp = gdt(GdtChar::Perpendicularidade);
    assert_eq!(pp.len(), 2);
    assert!(pp.iter().any(|p| matches!(&p.kind, PmiKind::Gdt { datums, .. } if datums == "A")));
    assert!(pp.iter().any(|p| matches!(&p.kind, PmiKind::Gdt { datums, .. } if datums == "A B")));
    let pos = gdt(GdtChar::Posicao);
    assert_eq!(pos.len(), 1);
    assert!(matches!(&pos[0].kind, PmiKind::Gdt { tol, datums, .. } if tol == "Ø0,1" && datums == "A B C"));
    // a localização vira cota básica, sem ±
    let locais: Vec<&Pmi> = doc.pmi.iter().filter(|p| p.name.starts_with("Básica ")).collect();
    assert_eq!(locais.len(), 2);
    assert!(locais.iter().all(|p| p.style.frame == PmiFrame::Caixa), "{:?}", locais.iter().map(|p| (&p.name, p.style.frame)).collect::<Vec<_>>());
    // a regra sem furo não gera nada de furo
    let mut sem = RegrasEsquema { modo: ModoEsquema::Geometrico, ..Default::default() };
    sem.furo.gerar = false;
    let (_, prop) = esquema_pmi::propor(&doc, &sem).unwrap();
    assert!(!prop.iter().any(|p| p.name.starts_with("Furo ") || p.name.starts_with("Posição ") || p.name.starts_with("Básica ")));
}
