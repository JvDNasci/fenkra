// SPDX-License-Identifier: LGPL-2.1-or-later
//! B381: a vista de uma montagem que só tem subconjuntos percorre os subconjuntos (antes saía vazia).

use fenkra_desenho::source::Models;
use fenkra_desenho::{Dir, Drawing, Format, Source, ViewKind};
use fenkra_montagem::{Assembly, NodeKind, Placement};

fn montar(nome: &str) -> (std::path::PathBuf, Source) {
    let dir = std::env::temp_dir().join(format!("{nome}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    // subconjunto com uma peça; montagem só com dois subconjuntos (um deslocado 500 mm em X)
    let mut sub = Assembly::new("sub");
    sub.add_component("P", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
    std::fs::write(dir.join("sub.fka"), sub.to_json()).unwrap();
    let mut a = Assembly::new("raiz");
    a.add_component("S1", NodeKind::SubAssembly { path: "sub.fka".into() }, Placement::default(), None);
    a.add_component("S2", NodeKind::SubAssembly { path: "sub.fka".into() }, Placement { origin: [500., 0., 0.], ..Default::default() }, None);
    std::fs::write(dir.join("raiz.fka"), a.to_json()).unwrap();
    (dir, Source { path: "raiz.fka".into(), display_state: String::new(), hidden: vec![] })
}

#[test]
fn vista_de_montagem_so_com_subconjuntos_nao_sai_vazia() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (dir, src) = montar("fenkra_desenho_sub");
    let mut d = Drawing::new("t", Format::A3);
    let s = d.sheets[0].id;
    let v = d.add_view(s, src, ViewKind::Standard(Dir::Topo), [100., 100.], 1.);
    let mut m = Models::new(Some(dir.clone()));
    let p = m.proj(&d, v);
    assert!(p.error.is_none(), "{:?}", p.error);
    assert!(!p.lines.is_empty(), "a vista tem linhas");
    // a segunda cópia está 500 mm adiante: a largura da vista cobre as duas
    assert!(p.hi[0] - p.lo[0] > 500., "largura {}", p.hi[0] - p.lo[0]);
    let _ = std::fs::remove_dir_all(dir);
}
