// SPDX-License-Identifier: LGPL-2.1-or-later
//! B105: DXF para sketch, com camadas e unidades, conferido pelo sólido que o sketch gera.

use fenkra_conversor::{Formato, Importado, OpcoesImportar, Registro, UnidadeArquivo};
use fenkra_core::sketch::Curve;
use fenkra_core::{BoolOp, Document, Extent, Extrude, FeatureKind, Id};
use fenkra_dxf::{Anexo, Camada, Desenho, Entidade, Item, Unidade, Vertice};
use std::f64::consts::PI;
use std::path::PathBuf;

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra dxf açaí {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn importar(d: &Desenho, teste: &str, o: &OpcoesImportar) -> Importado {
    let dir = pasta(teste);
    let arquivo = dir.join("perfil ç.dxf");
    std::fs::write(&arquivo, fenkra_dxf::gravar(d)).unwrap();
    Registro::default().importar(&arquivo, o).unwrap()
}

fn vertices(p: &[[f64; 2]]) -> Vec<Vertice> {
    p.iter().map(|q| Vertice { p: *q, bojo: 0. }).collect()
}

/// Placa 100 x 50 com furo de raio 10 na camada Contorno, e cota solta na camada Cotas.
fn placa(unidade: Unidade) -> Desenho {
    Desenho {
        unidade,
        camadas: vec![Camada::nova("Contorno", 7, "Continuous"), Camada::nova("Cotas", 3, "Continuous")],
        itens: vec![
            Item::novo("Contorno", Entidade::Polilinha { vertices: vertices(&[[0., 0.], [100., 0.], [100., 50.], [0., 50.]]), fechada: true }),
            Item::novo("Contorno", Entidade::Circulo { centro: [50., 25.], raio: 10. }),
            Item::novo("Cotas", Entidade::Linha([0., -10.], [100., -10.])),
            Item::novo("Cotas", Entidade::Texto { pos: [50., -12.], altura: 3., angulo: 0., anexo: Anexo::Centro, texto: "100".into() }),
        ],
        ..Default::default()
    }
}

fn sketch(doc: &Document, nome: &str) -> Id {
    doc.features.iter().find(|f| f.name == nome && matches!(f.kind, FeatureKind::Sketch(_))).map(|f| f.id).unwrap_or_else(|| panic!("sem sketch {nome}: {:?}", doc.features.iter().map(|f| &f.name).collect::<Vec<_>>()))
}

/// Extruda o sketch e devolve o volume e as medidas do corpo.
fn extrudar(doc: &Document, sk: Id, altura: &str) -> (f64, [f64; 3]) {
    let mut d = doc.clone();
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(altura.into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let r = fenkra_solidos::recompute(&d);
    assert!(!r.status.values().any(|s| matches!(s, fenkra_solidos::Status::Error(_))), "{:?}", r.status);
    assert_eq!(r.bodies.len(), 1, "{:?}", r.status);
    let b = &r.bodies[0];
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for q in b.mesh.positions.chunks_exact(3) {
        for i in 0..3 {
            lo[i] = lo[i].min(q[i] as f64);
            hi[i] = hi[i].max(q[i] as f64);
        }
    }
    (b.mass.volume, [0, 1, 2].map(|i| hi[i] - lo[i]))
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.)
}

#[test]
fn cada_camada_vira_um_sketch_e_o_contorno_faz_o_solido() {
    let i = importar(&placa(Unidade::Milimetro), "camadas", &OpcoesImportar::default());
    let r = &i.relatorio;
    assert_eq!(r.formato, "DXF");
    assert!(r.esquema.contains("AC1015"), "{}", r.esquema);
    assert_eq!(r.camadas, vec![("Contorno".to_string(), 2), ("Cotas".to_string(), 1)]);
    assert!(i.conjunto.is_none());
    let (v, m) = extrudar(&i.doc, sketch(&i.doc, "Contorno"), "10");
    assert!(perto(v, (5000. - PI * 100.) * 10., 1e-6), "{v}");
    assert!(perto(m[0], 100., 1e-6) && perto(m[1], 50., 1e-6) && perto(m[2], 10., 1e-6), "{m:?}");
    let (lo, hi) = r.caixa.unwrap();
    assert!(perto(hi[0] - lo[0], 100., 1e-9) && perto(hi[1], 50., 1e-9) && perto(lo[1], -10., 1e-9), "{lo:?} {hi:?}");
    assert!(!i.previa.arestas.is_empty() && i.previa.tris.is_empty());
    assert!(r.texto_pt().contains("Fora do sketch: TEXT/MTEXT (1)"), "{}", r.texto_pt());
    assert!(r.texto_pt().contains("Camadas lidas:\n- Contorno: 2"), "{}", r.texto_pt());
    assert_eq!(i.doc.origem.as_ref().unwrap().nome, "perfil ç.dxf");
}

#[test]
fn camadas_desligadas_juntam_tudo_em_um_sketch() {
    let i = importar(&placa(Unidade::Milimetro), "uma", &OpcoesImportar { camadas: false, ..Default::default() });
    let sketches = i.doc.features.iter().filter(|f| matches!(f.kind, FeatureKind::Sketch(_))).count();
    assert_eq!(sketches, 1);
    assert_eq!(i.relatorio.camadas, vec![("*".to_string(), 3)]);
}

#[test]
fn polegada_declarada_vira_milimetro() {
    let i = importar(&placa(Unidade::Polegada), "pol", &OpcoesImportar::default());
    assert_eq!(i.relatorio.escala, 25.4);
    assert_eq!(i.relatorio.unidades, "polegada");
    let (_, m) = extrudar(&i.doc, sketch(&i.doc, "Contorno"), "10");
    assert!(perto(m[0], 2540., 1e-6) && perto(m[1], 1270., 1e-6), "{m:?}");
}

#[test]
fn sem_unidade_avisa_e_a_escolha_da_pessoa_vale() {
    let d = placa(Unidade::SemUnidade);
    let i = importar(&d, "sem", &OpcoesImportar::default());
    assert!(i.relatorio.texto_pt().contains("não declara a unidade"), "{}", i.relatorio.texto_pt());
    assert_eq!(i.relatorio.escala, 1.);
    let i = importar(&d, "metro", &OpcoesImportar { unidade: UnidadeArquivo::Metro, ..Default::default() });
    assert!(!i.relatorio.texto_pt().contains("não declara a unidade"));
    let (_, m) = extrudar(&i.doc, sketch(&i.doc, "Contorno"), "10");
    assert!(perto(m[0], 100_000., 1e-6), "{m:?}");
}

#[test]
fn unidade_escolhida_passa_na_frente_da_declarada() {
    let i = importar(&placa(Unidade::Polegada), "troca", &OpcoesImportar { unidade: UnidadeArquivo::Milimetro, ..Default::default() });
    let (_, m) = extrudar(&i.doc, sketch(&i.doc, "Contorno"), "10");
    assert!(perto(m[0], 100., 1e-6), "{m:?}");
}

#[test]
fn folga_dentro_da_tolerancia_fecha_o_laco_e_avisa() {
    let d = Desenho {
        unidade: Unidade::Milimetro,
        itens: vec![
            Item::novo("A", Entidade::Linha([0., 0.], [10., 0.])),
            Item::novo("A", Entidade::Linha([10.004, 0.], [10., 10.])),
            Item::novo("A", Entidade::Linha([10., 10.003], [0., 10.])),
            Item::novo("A", Entidade::Linha([0., 10.], [0., 0.002])),
        ],
        ..Default::default()
    };
    let i = importar(&d, "folga", &OpcoesImportar::default());
    assert!(i.relatorio.texto_pt().contains("viraram o mesmo ponto"), "{}", i.relatorio.texto_pt());
    let (v, _) = extrudar(&i.doc, sketch(&i.doc, "A"), "5");
    assert!(perto(v, 500., 1e-2), "{v}");
}

#[test]
fn rasgo_de_polilinha_com_bojo_fecha_com_arcos_exatos() {
    // rasgo 40 x 20: dois meios círculos por bojo, nos dois sentidos de percurso
    for (nome, sinal, p) in [("anti", 1., [[0., 0.], [40., 0.], [40., 20.], [0., 20.]]), ("hor", -1., [[0., 20.], [40., 20.], [40., 0.], [0., 0.]])] {
        let v = vec![Vertice { p: p[0], bojo: 0. }, Vertice { p: p[1], bojo: sinal }, Vertice { p: p[2], bojo: 0. }, Vertice { p: p[3], bojo: sinal }];
        let d = Desenho { unidade: Unidade::Milimetro, itens: vec![Item::novo("R", Entidade::Polilinha { vertices: v, fechada: true })], ..Default::default() };
        let i = importar(&d, nome, &OpcoesImportar::default());
        let FeatureKind::Sketch(s) = &i.doc.feature(sketch(&i.doc, "R")).unwrap().kind else { panic!() };
        assert_eq!(s.data.curves.iter().filter(|c| matches!(c.geom, Curve::Arc { .. })).count(), 2);
        let (vol, m) = extrudar(&i.doc, sketch(&i.doc, "R"), "1");
        assert!(perto(vol, 800. + PI * 100., 1e-6), "{nome}: {vol}");
        assert!(perto(m[0], 60., 1e-6), "{m:?}");
    }
}

#[test]
fn arco_e_linhas_em_entidades_separadas_fecham_um_perfil() {
    let d = Desenho {
        unidade: Unidade::Milimetro,
        itens: vec![
            Item::novo("M", Entidade::Linha([-10., 0.], [10., 0.])),
            Item::novo("M", Entidade::Arco { centro: [0., 0.], raio: 10., ini: 0., fim: 180. }),
        ],
        ..Default::default()
    };
    let i = importar(&d, "meia", &OpcoesImportar::default());
    let (v, _) = extrudar(&i.doc, sketch(&i.doc, "M"), "2");
    assert!(perto(v, PI * 100., 1e-6), "{v}");
}

#[test]
fn elipse_e_spline_chegam_como_curvas_de_verdade() {
    let d = Desenho {
        unidade: Unidade::Milimetro,
        itens: vec![
            Item::novo("E", Entidade::Elipse { centro: [0., 0.], maior: [20., 0.], razao: 0.5, t0: 0., t1: 2. * PI }),
            Item::novo("S", Entidade::Spline { grau: 3, controle: vec![], nos: vec![], pesos: vec![], ajuste: vec![[0., 0.], [10., 5.], [20., 0.]], fechada: false }),
            Item::novo("N", Entidade::Spline { grau: 2, controle: vec![[0., 0.], [5., 10.], [10., 0.]], nos: vec![0., 0., 0., 1., 1., 1.], pesos: vec![], ajuste: vec![], fechada: false }),
            Item::novo("P", Entidade::Ponto([3., 4.])),
        ],
        ..Default::default()
    };
    let i = importar(&d, "curvas", &OpcoesImportar::default());
    let (v, m) = extrudar(&i.doc, sketch(&i.doc, "E"), "1");
    assert!(perto(v, PI * 20. * 10., 1e-3), "{v}");
    assert!(perto(m[0], 40., 1e-3) && perto(m[1], 20., 1e-3), "{m:?}");
    let curva = |n: &str| {
        let FeatureKind::Sketch(s) = &i.doc.feature(sketch(&i.doc, n)).unwrap().kind else { panic!() };
        s.data.curves[0].geom.clone()
    };
    assert!(matches!(curva("S"), Curve::Spline { pts } if pts.len() == 3));
    assert!(matches!(curva("N"), Curve::Spline { pts } if pts.len() > 3));
    assert!(matches!(curva("P"), Curve::Point { .. }));
}

#[test]
fn arquivo_so_com_texto_explica_e_formato_vem_do_conteudo() {
    let dir = pasta("texto");
    let so_texto = Desenho { itens: vec![Item::novo("T", Entidade::Texto { pos: [0., 0.], altura: 1., angulo: 0., anexo: Anexo::Esquerda, texto: "x".into() })], ..Default::default() };
    let arquivo = dir.join("t.dxf");
    std::fs::write(&arquivo, fenkra_dxf::gravar(&so_texto)).unwrap();
    let e = Registro::default().importar(&arquivo, &OpcoesImportar::default()).err().unwrap();
    assert!(e.to_string().contains("não tem curvas"), "{e}");

    let renomeado = dir.join("a.dat");
    std::fs::write(&renomeado, fenkra_dxf::gravar(&placa(Unidade::Milimetro))).unwrap();
    let i = Registro::default().importar(&renomeado, &OpcoesImportar::default()).unwrap();
    assert_eq!(i.relatorio.formato, Formato::Dxf.nome());
    assert!(i.relatorio.texto_pt().contains("não combina com o conteúdo"));
}

#[test]
fn documento_do_dxf_sobrevive_ao_json_com_o_relatorio() {
    let i = importar(&placa(Unidade::Milimetro), "json", &OpcoesImportar::default());
    let doc = Document::from_json(&i.doc.to_json()).unwrap();
    let (v, _) = extrudar(&doc, sketch(&doc, "Contorno"), "10");
    assert!(perto(v, (5000. - PI * 100.) * 10., 1e-6), "{v}");
    let r = doc.origem.as_ref().and_then(|o| o.relatorio.as_deref()).and_then(fenkra_conversor::Relatorio::de_json);
    assert!(r.is_some_and(|r| r.camadas.len() == 2));
}
