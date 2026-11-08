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

/// Onde vai o tempo da carga da montagem (B393): calcular cada peça, posicionar ou juntar.
#[test]
#[ignore]
fn fases_da_carga_do_conjunto_do_kart() {
    use fenkra_montagem::load::{full_path, PartCache};
    use fenkra_montagem::view::{placed_shape, shown};
    use fenkra_montagem::{Assembly, NodeKind};
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../validacao/kart/modelo");
    let a = Assembly::from_json(&std::fs::read_to_string(dir.join("KART.fka")).unwrap()).unwrap();
    let mut cache = PartCache::default();
    cache.so_forma = true;
    let (mut t_peca, mut t_junta) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    let mut lista = vec![];
    let mut n = 0;
    for nd in &a.nodes {
        let NodeKind::SubAssembly { path } = &nd.kind else { continue };
        let fp = full_path(Some(&dir), path);
        let sub = Assembly::from_json(&std::fs::read_to_string(&fp).unwrap()).unwrap();
        let sd = fp.parent().unwrap().to_path_buf();
        for pn in &sub.nodes {
            if !matches!(pn.kind, NodeKind::Part { .. }) || !shown(&sub, pn.id) {
                continue;
            }
            let t = Instant::now();
            let s = placed_shape(&sub, pn.id, Some(&sd), &mut cache);
            t_peca += t.elapsed();
            if let Some(s) = s {
                lista.push(s);
                n += 1;
            }
        }
    }
    let mut all = fenkra_kernel::Shape::empty();
    for s in &lista {
        let t = Instant::now();
        all = if all.is_null() { s.clone() } else { all.compound(s) };
        t_junta += t.elapsed();
    }
    let t = Instant::now();
    let _ = all.bbox();
    println!("bbox do aninhado: {:?}", t.elapsed());
    let _ = full_path(Some(&dir), "");
    println!("pecas diretas={n} calcular+posicionar={t_peca:?} juntar={t_junta:?}");
}
