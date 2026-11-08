// SPDX-License-Identifier: LGPL-2.1-or-later
//! B415: tabela sobre a legenda ou fora da folha avisa e vai para um canto livre; a escala automática
//! considera largura e altura e fica na lista padronizada.

use fenkra_core::sketch::Curve;
use fenkra_core::*;
use fenkra_desenho::ocupacao::{self, Problema};
use fenkra_desenho::source::Models;
use fenkra_desenho::*;

fn flange(rc: f64) -> Document {
    let mut d = Document::new("Flange");
    let mut data = SketchData::default();
    let mut n = 11_000_000u64;
    let mut al = move || {
        n += 1;
        Id(n)
    };
    let c = data.add_point(&mut al, 0., 0.);
    data.add_curve(&mut al, Curve::Circle { c, r: 45. }, false);
    data.add_curve(&mut al, Curve::Circle { c, r: 16. }, false);
    for k in 0..6 {
        let a = k as f64 / 6. * std::f64::consts::TAU;
        let p = data.add_point(&mut al, rc * a.cos(), rc * a.sin());
        data.add_curve(&mut al, Curve::Circle { c: p, r: 4. }, false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    d
}

fn montar(nome: &str, pos: [f64; 2]) -> (Drawing, Models, Id, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("{nome}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("flange.fkp"), flange(32.).to_json()).unwrap();
    let src = Source { path: "flange.fkp".into(), display_state: String::new(), hidden: vec![] };
    let mut d = Drawing::new("Flange", Format::A3);
    let s = d.sheets[0].id;
    d.add_view(s, src.clone(), ViewKind::Standard(Dir::Topo), [150., 190.], 1.);
    let tid = d.alloc();
    d.tables.push(Table { id: tid, sheet: s, kind: TableKind::Holes, source: src, pos, level: Default::default(), cfg: Default::default() });
    (d, Models::new(Some(dir.clone())), tid, dir)
}

#[test]
fn tabela_sobre_a_legenda_avisa_e_vai_para_area_livre() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // y = 30 conta de baixo: a tabela desce por cima da legenda e sai da folha (K-19)
    let (mut d, mut m, tid, dir) = montar("fenkra_desenho_ocupacao", [300., 30.]);
    let av = ocupacao::avisos_de_tabelas(&mut m, &d);
    assert_eq!(av.len(), 1, "{av:?}");
    assert!(matches!(av[0].problema, Problema::ForaDaFolha | Problema::SobreLegenda));
    let mudou = ocupacao::realocar_tabelas(&mut m, &mut d);
    assert_eq!(mudou, vec![tid]);
    assert!(ocupacao::avisos_de_tabelas(&mut m, &d).is_empty(), "depois de realocar, nada a avisar");
    let (lo, hi) = ocupacao::caixa_da_tabela(&mut m, &d, tid).unwrap();
    let sh = &d.sheets[0];
    let (a0, a1) = ocupacao::area_moldura(sh);
    assert!(lo[0] >= a0[0] && lo[1] >= a0[1] && hi[0] <= a1[0] && hi[1] <= a1[1]);
    let leg = ocupacao::legenda_caixa(d.title.style, sh.size());
    assert!(lo[1] >= leg.1[1] || hi[0] <= leg.0[0], "fora da legenda");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn conjunto_padrao_com_peca_grande_nao_cobre_as_vistas() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, mut m, _t, dir) = montar("fenkra_desenho_conjunto", [300., 270.]);
    d.views.clear();
    d.tables.clear();
    let s = d.sheets[0].id;
    let src = Source { path: "flange.fkp".into(), display_state: String::new(), hidden: vec![] };
    // flange de 90 mm em escala 5 (450 mm por vista) com folga de 60 mm: o gap fixo as cobria
    let (ids, gap) = fenkra_desenho::conjunto::criar_sem_sobrepor(&mut d, &mut m, s, src, [400., 400.], 5., 60.);
    assert_eq!(ids.len(), 4);
    assert!(gap >= 90. * 5. + 60. - 1e-6, "distancia entre centros {gap}");
    let (f, l) = (d.view(ids[0]).unwrap().pos, d.view(ids[2]).unwrap().pos);
    assert!(((f[0] - l[0]).abs() - gap).abs() < 1e-6, "lateral a `gap` da frente");
    // a conta da interface (distancia) concorda com a do conjunto real
    let mut rascunho = d.clone();
    let g2 = fenkra_desenho::conjunto::distancia(&mut rascunho, &mut m, s, &d.view(ids[0]).unwrap().source.clone(), 5., 60.);
    assert!((g2 - gap).abs() < 1e-9);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn tabela_no_lugar_certo_nao_se_mexe() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, mut m, _tid, dir) = montar("fenkra_desenho_ocupacao_ok", [300., 270.]);
    assert!(ocupacao::avisos_de_tabelas(&mut m, &d).is_empty());
    assert!(ocupacao::realocar_tabelas(&mut m, &mut d).is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn escala_automatica_olha_altura_e_largura_na_lista_padronizada() {
    // largura cabe em 1:1, mas a altura (400 mm) numa área de 200 mm só cabe em 1:2
    let (e, cabe) = ocupacao::escala_que_cabe(100., 400., 300., 200.);
    assert_eq!((e, cabe), (0.5, true));
    assert!(fenkra_desenho::novo::ESCALAS_AUTO.contains(&e));
    // nada na lista serve: devolve a menor e avisa
    let (e, cabe) = ocupacao::escala_que_cabe(1e6, 1e6, 300., 200.);
    assert!(!cabe && e == 0.01);
}
