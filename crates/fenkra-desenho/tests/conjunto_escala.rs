// SPDX-License-Identifier: LGPL-2.1-or-later
//! B473: a escala automática do conjunto de quatro vistas considera as quatro vistas e os vãos entre
//! elas, escolhe da lista normalizada a maior que cabe dentro da moldura e fora do carimbo, e o
//! conjunto criado fica todo dentro da área útil em A4, A3 e A2 (peça longa e montagem).

use fenkra_core::sketch::Curve;
use fenkra_core::*;
use fenkra_desenho::conjunto::{self, area_util, centro_dentro, escala_do_conjunto, medir_conjunto};
use fenkra_desenho::novo::ESCALAS_AUTO;
use fenkra_desenho::source::Models;
use fenkra_desenho::*;
use fenkra_montagem::{Assembly, NodeKind, Placement};

/// Eixo de 800 mm de comprimento e 20 mm de diâmetro: bem mais comprido que largo.
fn eixo_longo() -> Document {
    let mut d = Document::new("Eixo");
    let mut data = SketchData::default();
    let mut n = 12_000_000u64;
    let mut al = move || {
        n += 1;
        Id(n)
    };
    let c = data.add_point(&mut al, 0., 0.);
    data.add_curve(&mut al, Curve::Circle { c, r: 10. }, false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("800".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    d
}

fn pasta(nome: &str) -> (std::path::PathBuf, Models) {
    let dir = std::env::temp_dir().join(format!("{nome}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("eixo.fkp"), eixo_longo().to_json()).unwrap();
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    // montagem: o eixo e três placas espalhadas, para um conjunto largo
    let mut a = Assembly::new("conj");
    a.add_component("E", NodeKind::Part { path: "eixo.fkp".into() }, Placement::default(), None);
    for k in 1..4 {
        a.add_component("P", NodeKind::Part { path: "placa.fkp".into() }, Placement { origin: [200. * k as f64, 100., 0.], ..Default::default() }, None);
    }
    std::fs::write(dir.join("conj.fka"), a.to_json()).unwrap();
    (dir.clone(), Models::new(Some(dir)))
}

fn fonte(p: &str) -> Source {
    Source { path: p.into(), display_state: String::new(), hidden: vec![] }
}

/// Cria o conjunto como a ferramenta cria e confere cada vista contra a área útil.
fn confere(m: &mut Models, formato: Format, src: &Source, clique: V2) -> f64 {
    let mut d = Drawing::new("t", formato);
    let folha = d.sheets[0].id;
    let area = area_util(&d, folha);
    let med = medir_conjunto(&mut d.clone(), m, folha, src);
    let (escala, caixa, cabe) = escala_do_conjunto(&med, d.projection, area, 25.);
    assert!(cabe, "{formato:?}: nem a menor escala serve");
    assert!(ESCALAS_AUTO.contains(&escala), "escala fora da lista normalizada: {escala}");
    // a maior que cabe: a escala acima da lista não cabe
    if let Some(i) = ESCALAS_AUTO.iter().position(|s| *s == escala).filter(|i| *i > 0) {
        let c = conjunto::caixa_do_conjunto(&med, d.projection, ESCALAS_AUTO[i - 1], 25.);
        assert!(c.1[0] - c.0[0] > area.1[0] - area.0[0] + 1e-9 || c.1[1] - c.0[1] > area.1[1] - area.0[1] + 1e-9, "{formato:?}: a escala {} também cabia", ESCALAS_AUTO[i - 1]);
    }
    let centro = centro_dentro(clique, caixa, area);
    let gap = conjunto::distancia(&mut d.clone(), m, folha, src, escala, 25.).max(40.);
    let ids = d.add_standard_set(folha, src.clone(), centro, escala, gap);
    assert_eq!(ids.len(), 4);
    for id in ids {
        let v = d.view(id).unwrap().clone();
        let p = m.proj(&d, id);
        let (a, b) = (layout::to_sheet(&v, &p, p.lo), layout::to_sheet(&v, &p, p.hi));
        let (lo, hi) = ([a[0].min(b[0]), a[1].min(b[1])], [a[0].max(b[0]), a[1].max(b[1])]);
        let t = 0.01;
        assert!(lo[0] >= area.0[0] - t && lo[1] >= area.0[1] - t && hi[0] <= area.1[0] + t && hi[1] <= area.1[1] + t, "{formato:?} {src:?}: vista {id:?} fora da área útil {area:?}: {lo:?} {hi:?}");
    }
    escala
}

#[test]
fn peca_longa_e_montagem_ficam_dentro_da_area_util_em_a4_a3_e_a2() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (dir, mut m) = pasta("fenkra_desenho_conjunto_escala");
    for src in [fonte("eixo.fkp"), fonte("conj.fka")] {
        let mut anterior = 0.;
        for f in [Format::A4, Format::A3, Format::A2] {
            // clique no canto: o conjunto é levado para dentro da área
            let e = confere(&mut m, f, &src, [5., 5.]);
            assert!(e >= anterior, "folha maior não pode dar escala menor: {e} < {anterior}");
            anterior = e;
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn escala_auto_nao_e_maior_do_que_cabe_na_peca_longa() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (dir, mut m) = pasta("fenkra_desenho_conjunto_escala_a4");
    // 800 mm de eixo numa A4 (297 x 210): só uma escala bem reduzida serve
    let e = confere(&mut m, Format::A4, &fonte("eixo.fkp"), [150., 150.]);
    assert!(e <= 0.2, "escala {e}");
    let _ = std::fs::remove_dir_all(dir);
}
