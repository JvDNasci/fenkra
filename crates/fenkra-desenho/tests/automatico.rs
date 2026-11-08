// SPDX-License-Identifier: LGPL-2.1-or-later
//! B144: desenho automático por regras sobre a placa de exemplo (quatro furos com rebaixo e um
//! rasgo): cotas de contorno presas aos vértices, um diâmetro "4×" por tamanho (o rasgo não é furo), marcas
//! de centro, a tabela de furos fora da legenda, e rodar de novo não duplica nada.

use fenkra_desenho::automatico;
use fenkra_desenho::layout::{dim_anchors_ok, dim_value};
use fenkra_desenho::source::Models;
use fenkra_desenho::*;

#[test]
fn completa_a_folha_com_contorno_furos_e_tabela_sem_duplicar() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::env::temp_dir().join(format!("fenkra_desenho_auto_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    let src = Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] };
    let mut d = Drawing::new("Placa", Format::A3);
    let s = d.sheets[0].id;
    d.add_standard_set(s, src, [130., 190.], 1., 100.);
    let mut m = Models::new(Some(dir.clone()));
    let r = automatico::completar(&mut d, s, &mut m);
    assert!(r.avisos.is_empty(), "{:?}", r.avisos);
    // contorno: largura e altura na frente, profundidade na de cima e na lateral; um diâmetro
    let valores: Vec<(DimKind, f64, String)> = d
        .dims
        .iter()
        .map(|dm| {
            let v = d.view(dm.view).unwrap().clone();
            let p = m.proj(&d, v.id);
            assert!(dim_anchors_ok(dm, &p).iter().all(|b| *b), "cota {:?} solta da geometria", dm.kind);
            (dm.kind, dim_value(dm, &v, &p).unwrap().0, dm.text.clone())
        })
        .collect();
    let de = |k: DimKind| valores.iter().filter(|x| x.0 == k).map(|x| x.1).collect::<Vec<_>>();
    let h = de(DimKind::Horizontal);
    let v = de(DimKind::Vertical);
    assert!(h.iter().any(|x| (x - 120.).abs() < 0.5), "largura da placa: {valores:?}");
    // placa 120 × 72 × 10: a profundidade sai na de cima (vertical) e na lateral (horizontal)
    assert!(v.iter().any(|x| (x - 72.).abs() < 0.5) && h.iter().any(|x| (x - 72.).abs() < 0.5), "profundidade: {valores:?}");
    assert!(v.iter().any(|x| (x - 10.).abs() < 0.5), "espessura na frente: {valores:?}");
    // um diâmetro por tamanho: o furo Ø6,6 e o rebaixo Ø11, cada um "4×"; o rasgo fica de fora. A
    // borda do rebaixo só fica presa à geometria com a correção de `dim_anchors_ok` para dois
    // círculos no mesmo centro (antes ela comparava só com o primeiro círculo achado)
    let diam: Vec<&(DimKind, f64, String)> = valores.iter().filter(|x| x.0 == DimKind::Diameter).collect();
    assert_eq!(diam.len(), 2, "{valores:?}");
    assert!(diam.iter().any(|x| (x.1 - 6.6).abs() < 0.01) && diam.iter().any(|x| (x.1 - 11.).abs() < 0.01), "{valores:?}");
    assert!(diam.iter().all(|x| x.2.starts_with("4×")), "{diam:?}");
    assert!(r.centros >= 4, "{r:?}");
    assert!(r.tabela);
    let t = d.tables.iter().find(|t| t.kind == TableKind::Holes).unwrap();
    // a tabela não cai na legenda
    let (lo, hi) = layout::tabela_caixa(&mut m, &d, t, t.pos).unwrap();
    let leg = ocupacao::legenda_caixa(d.title.style, d.sheets[0].size());
    let cruza = lo[0] < leg.1[0] && hi[0] > leg.0[0] && lo[1] < leg.1[1] && hi[1] > leg.0[1];
    assert!(!cruza, "tabela {lo:?}-{hi:?} sobre a legenda {leg:?}");

    // de novo: nada se repete
    let (n_dims, n_centros, n_tab) = (d.dims.len(), d.centers.len(), d.tables.len());
    let r2 = automatico::completar(&mut d, s, &mut m);
    assert_eq!((r2.cotas, r2.centros, r2.tabela), (0, 0, false));
    assert_eq!((d.dims.len(), d.centers.len(), d.tables.len()), (n_dims, n_centros, n_tab));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn folha_vazia_avisa() {
    let mut d = Drawing::new("t", Format::A3);
    let s = d.sheets[0].id;
    let r = automatico::completar(&mut d, s, &mut Models::new(None));
    assert_eq!(r.avisos.len(), 1);
}
