// SPDX-License-Identifier: LGPL-2.1-or-later
//! B394: tempo da interferência no kart (85 peças). Ignorado por padrão: depende do modelo em
//! `validacao/kart/modelo` e leva alguns segundos. Rode com
//! `cargo test -p fenkra-montagem --release --test interferencia_kart -- --ignored --nocapture`.

use fenkra_montagem::interferencia::{analisar, OpcoesInterf};
use fenkra_montagem::load::PartCache;
use fenkra_montagem::Assembly;
use std::path::PathBuf;

#[test]
#[ignore]
fn interferencia_do_kart_cabe_em_30_s() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../validacao/kart/modelo");
    let Ok(txt) = std::fs::read_to_string(dir.join("KART.fka")) else { return };
    let k = Assembly::from_json(&txt).unwrap();
    let mut cache = PartCache::default();
    let op = OpcoesInterf { sub_unico: false, ..Default::default() };
    // a primeira passada calcula as peças; na interface elas já estão no cache
    let t = std::time::Instant::now();
    let a1 = analisar(&k, Some(&dir), &mut cache, &op);
    let frio = t.elapsed().as_secs_f64();
    let t = std::time::Instant::now();
    let a2 = analisar(&k, Some(&dir), &mut cache, &op);
    let quente = t.elapsed().as_secs_f64();
    println!("interferência do kart: {} achados; {frio:.1} s com as peças por calcular, {quente:.1} s com elas no cache", a1.len());
    for x in a2.iter().take(12) {
        println!("  {} x {}: {:.1} mm³", x.a.nome, x.b.nome, x.volume);
    }
    assert_eq!(a1.len(), a2.len());
    assert!(frio < 30., "{frio:.1} s");
}
