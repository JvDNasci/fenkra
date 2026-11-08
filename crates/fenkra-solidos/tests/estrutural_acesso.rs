// SPDX-License-Identifier: LGPL-2.1-or-later
//! B136: escada, descanso e guarda-corpo gerados por parâmetros viram membros comuns que recalculam,
//! com a caixa e as alturas da NR-12 (Anexo III) e a lista de corte esperadas.

use fenkra_core::*;
use fenkra_solidos::st_acesso::*;
use fenkra_solidos::{cut_list, recompute, Status};

fn caixa(d: &Document) -> ([f64; 3], [f64; 3]) {
    let r = recompute(d);
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

#[test]
fn escada_de_dois_lances_com_guarda_corpo_recalcula() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let e = Escada { altura: 4400., ..Default::default() };
    let (dm, pecas) = pecas_escada(&e).unwrap();
    assert!(verificar(&e, &dm).is_empty(), "{:?}", verificar(&e, &dm));
    let mut d = Document::new("escada");
    let ids = inserir(&mut d, &pecas, [0.; 3], 0.);
    let r = recompute(&d);
    for id in &ids {
        assert_eq!(r.status[id], Status::Ok, "{} {:?}", d.name_of(*id), r.status[id]);
    }
    let volume: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
    assert!(volume > 0.);
    let (lo, hi) = caixa(&d);
    // topo do guarda-corpo a 1100 mm sobre a chegada (4400): o último trecho é inclinado e termina em
    // corte reto, então o ponto mais alto fica um pouco abaixo dos 5500
    assert!(hi[2] > 5480. && hi[2] < 5500.5, "topo {hi:?}");
    // largura: útil 800 + 2 longarinas 9,5 + 2 montantes 33,7 + 2 intermediários 33,7
    let meia = 400. + 9.5 + 2. * 33.7;
    assert!((hi[0] - meia).abs() < 0.5 && (lo[0] + meia).abs() < 0.5, "largura {lo:?} {hi:?}");
    // 4400 / 220 = 20 espelhos de 220; g = 630 − 440 = 190 sobe ao mínimo de 200 (com espelho).
    assert!((dm.h - 220.).abs() < 1e-9 && (dm.g - 200.).abs() < 1e-9, "{dm:?}");
    // em planta: 2 × 9 pisos de 200 mais o descanso de 800 e o recuo de um piso
    assert!((dm.comprimento - (18. * 200. + 800. + 200.)).abs() < 1e-9, "{}", dm.comprimento);

    // lista de corte: 18 degraus iguais de 800, um descanso, montantes iguais
    let lista = cut_list(&d);
    let degraus: usize = lista.iter().filter(|c| c.designation.contains("200x6,35")).map(|c| c.count).sum();
    assert_eq!(degraus, 18, "{lista:#?}");
    assert!(lista.iter().filter(|c| c.designation.contains("200x6,35")).all(|c| (c.length - 800.).abs() < 0.05));
    assert!(lista.iter().any(|c| c.designation.contains("800x6,35") && c.count == 1));
}

#[test]
fn guarda_corpo_de_plataforma_gira_e_fica_no_lugar() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let gc = GuardaCorpo::default();
    let borda = [[0., 0., 1500.], [2000., 0., 1500.], [2000., 1200., 1500.]];
    let pecas = guarda_corpo(&borda, true, &gc, "Plataforma: ");
    assert!(verificar_guarda(&gc).is_empty());
    let mut d = Document::new("plataforma");
    let ids = inserir(&mut d, &pecas, [100., 0., 0.], 90.);
    let r = recompute(&d);
    for id in &ids {
        assert_eq!(r.status[id], Status::Ok, "{:?}", r.status[id]);
    }
    let (lo, hi) = caixa(&d);
    assert!((hi[2] - 2600.).abs() < 0.5, "{hi:?}");
    // o pé do montante desce 150 mm
    assert!((lo[2] - 1350.).abs() < 0.5, "{lo:?}");
    // girado 90°: o primeiro trecho (2000 em X) passa a correr em Y
    assert!(hi[1] > 1990. && hi[1] < 2100., "{lo:?} {hi:?}");
}
