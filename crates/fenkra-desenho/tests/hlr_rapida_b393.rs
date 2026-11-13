// SPDX-License-Identifier: LGPL-2.1-or-later
//! B393: as linhas ocultas pelo mapa de profundidade (`hlr_rapida`) batem com a HLR exata do
//! OpenCASCADE na placa de exemplo: mesma caixa das linhas visíveis e quase o mesmo comprimento de
//! linha visível, em cada vista padrão.

use fenkra_desenho::source::{Axes, Models};
use fenkra_desenho::{Dir, Source};

type L = Vec<(Vec<[f64; 2]>, u8)>;

fn caixa(l: &L, kinds: &[u8]) -> ([f64; 2], [f64; 2]) {
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for (p, k) in l {
        if kinds.contains(k) {
            for q in p {
                for i in 0..2 {
                    lo[i] = lo[i].min(q[i]);
                    hi[i] = hi[i].max(q[i]);
                }
            }
        }
    }
    (lo, hi)
}

fn comprimento(l: &L, kinds: &[u8]) -> f64 {
    l.iter().filter(|(_, k)| kinds.contains(k)).flat_map(|(p, _)| p.windows(2).map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt())).sum()
}

#[test]
fn mapa_de_profundidade_bate_com_a_hlr_exata_na_placa() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::env::temp_dir().join(format!("fenkra_hlr_rapida_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    let mut m = Models::new(Some(dir.clone()));
    let forma = m.load(&Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] }).shape.clone().expect("forma da placa");
    let (lo, hi) = forma.bbox().unwrap();
    let big = (0..3).map(|k| hi[k] - lo[k]).fold(0., f64::max);
    let malha = forma.mesh(big * 0.0004).unwrap();
    for d in [Dir::Frente, Dir::Topo, Dir::Direita, Dir::Iso] {
        let a = Axes::of(d);
        let exata: L = forma.hlr(a.toward, a.x, true).unwrap();
        let t = std::time::Instant::now();
        let rapida = fenkra_desenho::hlr_rapida::projetar(&malha, &a, true, fenkra_desenho::hlr_rapida::RESOLUCAO);
        let tempo = t.elapsed();
        let vis = [0u8, 1, 2];
        let (ce, cr) = (caixa(&exata, &vis), caixa(&rapida, &vis));
        for i in 0..2 {
            assert!((ce.0[i] - cr.0[i]).abs() < 0.5 && (ce.1[i] - cr.1[i]).abs() < 0.5, "{d:?}: caixa {ce:?} contra {cr:?}");
        }
        let (le, lr) = (comprimento(&exata, &vis), comprimento(&rapida, &vis));
        println!("{d:?}: visível exata {le:.0} mm, rápida {lr:.0} mm; oculta {:.0} contra {:.0}; {tempo:?}", comprimento(&exata, &[3]), comprimento(&rapida, &[3]));
        assert!((le - lr).abs() < 0.08 * le, "{d:?}: comprimento visível {le} contra {lr}");
        let (oe, or) = (comprimento(&exata, &[3]), comprimento(&rapida, &[3]));
        assert!(or < 1.6 * oe + 50., "{d:?}: oculta {oe} contra {or}");
        assert!(tempo.as_secs_f64() < 5., "{d:?}: {tempo:?}");
    }
    let _ = std::fs::remove_dir_all(dir);
}
