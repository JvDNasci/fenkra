// SPDX-License-Identifier: LGPL-2.1-or-later
//! Medição manual (B393): tempo de carga e de projeção das vistas do conjunto do kart.
//! `cargo test -p fenkra-desenho --test medir_kart -- --ignored --nocapture`

use fenkra_desenho::source::Models;
use fenkra_desenho::*;
use std::time::Instant;

#[test]
#[ignore]
fn tempo_das_vistas_do_conjunto_do_kart() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../validacao/kart/modelo");
    let src = Source { path: "KART.fka".into(), display_state: String::new(), hidden: vec![] };
    let mut d = Drawing::new("kart", Format::A1);
    let s = d.sheets[0].id;
    let mut m = Models::new(Some(dir));
    let t = Instant::now();
    let l = m.load(&src);
    println!("carga: {:?} pecas={} erro={:?}", t.elapsed(), l.pieces.len(), l.error);
    for dir in [Dir::Frente, Dir::Topo, Dir::Esquerda, Dir::Iso] {
        let v = d.add_view(s, src.clone(), ViewKind::Standard(dir), [100., 100.], 0.05);
        let t = Instant::now();
        let p = m.proj(&d, v);
        println!("{dir:?}: {:?} linhas={} erro={:?}", t.elapsed(), p.lines.len(), p.error);
    }
}
