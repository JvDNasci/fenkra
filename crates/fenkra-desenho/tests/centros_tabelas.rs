// SPDX-License-Identifier: LGPL-2.1-or-later
//! B294 e B295 com peças de verdade: o flange de seis furos ganha uma marca de centro circular que
//! acompanha o diâmetro do círculo de furos, e a tabela de furos reflui e etiqueta as vistas.

use fenkra_core::sketch::Curve;
use fenkra_core::*;
use fenkra_desenho::centros::{self, ModoCentro};
use fenkra_desenho::layout;
use fenkra_desenho::source::Models;
use fenkra_desenho::tabela::{self, ConfigTabela};
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

#[test]
fn flange_de_seis_furos_marca_circular_atualiza_com_o_diametro_do_circulo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::env::temp_dir().join("fenkra_desenho_centros");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("flange.fkp"), flange(32.).to_json()).unwrap();
    let src = Source { path: "flange.fkp".into(), display_state: String::new(), hidden: vec![] };
    let mut d = Drawing::new("Flange", Format::A3);
    let s = d.sheets[0].id;
    let v = d.add_view(s, src, ViewKind::Standard(Dir::Topo), [200., 150.], 1.);
    let mut m = Models::new(Some(dir.clone()));
    let p = m.proj(&d, v);
    assert!(p.error.is_none(), "{:?}", p.error);
    // os seis furos de raio 4: a marca circular nasce de cliques neles
    let furos: Vec<_> = centros::arcos(&p).into_iter().filter(|(_, r)| (r - 4.).abs() < 0.05).collect();
    assert_eq!(furos.len(), 6);
    let cliques: Vec<[f64; 2]> = furos.iter().map(|(c, _)| *c).collect();
    let id = d.alloc();
    let mc = centros::montar(id, v, ModoCentro::Circular, &cliques, &p).expect("marca circular");
    d.centers.push(mc.clone());
    let t = centros::tracos(&mc, &p, 1.);
    assert!(!t.quebrada);
    assert_eq!(t.circulos.len(), 1);
    assert!((t.circulos[0].1 - 32.).abs() < 0.1, "círculo de furos de raio 32, veio {}", t.circulos[0].1);
    let antes = layout::sheet(&d, s, &mut m).len();
    assert!(antes > 30);
    // o diâmetro do círculo de furos passa de 64 para 84: a marca segue os furos sem ser refeita
    std::fs::write(dir.join("flange.fkp"), flange(42.).to_json()).unwrap();
    let mut m2 = Models::new(Some(dir));
    let p2 = m2.proj(&d, v);
    let t2 = centros::tracos(&d.centers[0], &p2, 1.);
    assert!(!t2.quebrada, "a marca reencontra os seis furos");
    assert!((t2.circulos[0].1 - 42.).abs() < 0.1, "raio novo {}", t2.circulos[0].1);
    assert_eq!(t2.segs.len(), 6 * 2 + 2);
    // esconder a vista tira as linhas da marca da folha
    let com = layout::sheet(&d, s, &mut m2).len();
    d.centros_ocultos.push(v);
    let sem = layout::sheet(&d, s, &mut m2).len();
    assert!(com > sem, "{com} contra {sem}");
}

#[test]
fn tabela_de_furos_reflui_ao_tirar_coluna_e_a_etiqueta_aponta_a_linha() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::env::temp_dir().join("fenkra_desenho_tabela_cfg");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    let src = Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] };
    let mut d = Drawing::new("Placa", Format::A3);
    let s = d.sheets[0].id;
    let ids = d.add_standard_set(s, src.clone(), [130., 190.], 1., 100.);
    let topo = ids[1];
    let tid = d.alloc();
    d.tables.push(Table { id: tid, sheet: s, kind: TableKind::Holes, source: src, pos: [25., 120.], level: Default::default(), cfg: ConfigTabela { etiquetas: true, ..Default::default() } });
    let mut m = Models::new(Some(dir));
    let cheia = tabela::dados(&mut m, &d.tables[0], &d);
    assert!(cheia.data.rows.len() >= 4, "furos: {}", cheia.data.rows.len());
    let largura = cheia.data.headers.len();
    let itens_antes = layout::sheet(&d, s, &mut m).len();
    // tirar a coluna Rosca reflui a tabela: uma coluna a menos em todas as linhas
    let cols: Vec<String> = cheia.data.headers.iter().filter(|h| *h != "Rosca").cloned().collect();
    d.tables[0].cfg.colunas = cols;
    let menor = tabela::dados(&mut m, &d.tables[0], &d);
    assert_eq!(menor.data.headers.len(), largura - 1);
    assert!(menor.data.rows.iter().all(|r| r.len() == largura - 1));
    assert!(layout::sheet(&d, s, &mut m).len() < itens_antes, "menos texto na folha");
    // etiquetas A1.. na vista de topo, uma por furo, cada uma ligada a uma linha da tabela
    let ets = tabela::etiquetas(&mut m, &d, &d.tables[0], &menor);
    let na_topo: Vec<_> = ets.iter().filter(|e| e.view == topo).collect();
    assert_eq!(na_topo.len(), menor.data.rows.len());
    let e = na_topo[1];
    let linha = menor.origem.iter().position(|o| *o == e.linha).unwrap();
    assert_eq!(menor.data.rows[linha][0], e.texto, "a etiqueta tem o mesmo texto da linha");
    // clicar na etiqueta acha a mesma etiqueta (o app seleciona a linha por ela)
    let achada = tabela::etiqueta_em(&ets, [e.pos[0] + 0.5, e.pos[1] + 0.5], 1.).expect("etiqueta sob o clique");
    assert_eq!((achada.view, achada.linha), (e.view, e.linha));
    // ordenar pelo cabeçalho X e dividir em blocos: as etiquetas continuam apontando para a linha certa
    d.tables[0].cfg.ordenar = Some(("X".into(), false));
    d.tables[0].cfg.dividir = 2;
    d.tables[0].cfg.renumerar = true;
    let f = tabela::dados(&mut m, &d.tables[0], &d);
    let ets = tabela::etiquetas(&mut m, &d, &d.tables[0], &f);
    for e in ets.iter().filter(|e| e.view == topo) {
        let pos = f.origem.iter().position(|o| *o == e.linha).unwrap();
        assert_eq!(f.data.rows[pos][0], e.texto);
        assert_eq!(e.texto, format!("A{}", pos + 1));
    }
    // com vista de origem, X e Y vêm do plano dela
    d.tables[0].cfg = ConfigTabela { vista_origem: Some(topo), etiquetas: true, ..Default::default() };
    let f = tabela::dados(&mut m, &d.tables[0], &d);
    assert_eq!(f.data.rows.len(), cheia.data.rows.len());
    assert!(layout::sheet(&d, s, &mut m).len() > 50);
}
