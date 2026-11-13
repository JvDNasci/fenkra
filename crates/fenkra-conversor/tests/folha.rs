// SPDX-License-Identifier: LGPL-2.1-or-later
//! B211: ida e volta do desenho 2D pela folha. A folha montada em código sai em DXF (e em SVG) pela
//! exportação do desenho e volta pelo conversor como esboços na folha, com as mesmas entidades, o mesmo
//! tipo de linha, os textos e a mesma caixa.

use fenkra_conversor::folha::{abrir_como_desenho, caixa, importar_na_folha};
use fenkra_core::sketch::Curve;
use fenkra_core::Id;
use fenkra_desenho::esboco::{DonoEsboco, TipoLinha};
use fenkra_desenho::layout::{sheet, Item};
use fenkra_desenho::source::Models;
use fenkra_desenho::{export, Drawing, Format, Note, NoteKind};
use fenkra_dxf::{Desenho, Entidade};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra folha açaí {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Folha A3 com um esboço: retângulo visível, furo de centro, arco oculto, linha fina, e uma nota.
fn folha_de_prova() -> (Drawing, Id, Id) {
    let mut d = Drawing::new("Prova", Format::A3);
    let folha = d.sheets[0].id;
    let id = d.add_esboco(DonoEsboco::Folha(folha));
    let mut n = 10_000;
    let mut al = move || {
        n += 1;
        Id(n)
    };
    let e = d.esboco_mut(id).unwrap();
    e.cotas = false;
    e.dados.add_rect(&mut al, [60., 50.], [180., 130.], false);
    let c = e.dados.add_point(&mut al, 120., 90.);
    let furo = e.dados.add_curve(&mut al, Curve::Circle { c, r: 15. }, false);
    let (s, f) = (e.dados.add_point(&mut al, 220., 90.), e.dados.add_point(&mut al, 200., 110.));
    let o = e.dados.add_point(&mut al, 200., 90.);
    let arco = e.dados.add_curve(&mut al, Curve::Arc { c: o, s, e: f }, false);
    let (a, b) = (e.dados.add_point(&mut al, 40., 90.), e.dados.add_point(&mut al, 250., 90.));
    let eixo = e.dados.add_curve(&mut al, Curve::Line { a, b }, false);
    e.definir_tipo(furo, TipoLinha::Centro);
    e.definir_tipo(arco, TipoLinha::Oculta);
    e.definir_tipo(eixo, TipoLinha::Fina);
    let nota = d.alloc();
    d.notes.push(Note { id: nota, sheet: folha, kind: NoteKind::Text, pos: [70., 140.], text: "CHAPA 3 mm".into(), size: 5., ..Default::default() });
    (d, folha, id)
}

/// Só o que o esboço e a nota desenham, sem moldura nem legenda.
fn do_conteudo(d: &Drawing, folha: Id) -> Vec<Item> {
    let donos: Vec<Id> = d.esbocos.iter().map(|e| e.id).chain(d.notes.iter().map(|n| n.id)).collect();
    let mut m = Models::new(None);
    sheet(d, folha, &mut m).into_iter().filter(|i| i.owner.is_some_and(|o| donos.contains(&o))).collect()
}

/// Entidades por camada e tipo, com o raio de círculos e arcos para pegar troca de curva.
fn inventario(d: &Desenho) -> BTreeMap<(String, String), usize> {
    let mut m = BTreeMap::new();
    for i in &d.itens {
        let tipo = match &i.entidade {
            Entidade::Linha(..) => "LINE".to_string(),
            Entidade::Circulo { raio, .. } => format!("CIRCLE r{raio:.3}"),
            Entidade::Arco { raio, .. } => format!("ARC r{raio:.3}"),
            Entidade::Polilinha { .. } => "LWPOLYLINE".into(),
            Entidade::Texto { texto, .. } => format!("TEXT {texto}"),
            outra => format!("{outra:?}").split([' ', '(', '{']).next().unwrap_or("").to_string(),
        };
        *m.entry((i.camada.clone(), tipo)).or_insert(0) += 1;
    }
    m
}

fn perto(a: [f64; 2], b: [f64; 2], tol: f64) -> bool {
    (a[0] - b[0]).abs() < tol && (a[1] - b[1]).abs() < tol
}

#[test]
fn conteudo_da_folha_volta_com_as_mesmas_entidades_tipos_e_caixa() {
    let (d, folha, _) = folha_de_prova();
    let ida = fenkra_dxf::ler(export::dxf(&do_conteudo(&d, folha)).as_bytes()).unwrap();

    let mut volta = Drawing::new("Volta", Format::A3);
    let f2 = volta.sheets[0].id;
    let r = importar_na_folha(&mut volta, f2, &ida);
    assert_eq!((r.esbocos, r.textos, r.sem_unidade), (4, 1, false), "{r:?}");
    assert!(r.fora.is_empty(), "{:?}", r.fora);
    // cada camada de tipo virou um esboço com o tipo certo
    let tipo_de = |nome: &str| {
        let e = volta.esbocos.iter().find(|e| e.nome == nome).unwrap_or_else(|| panic!("sem esboço {nome}"));
        e.dados.curves.iter().map(|c| e.tipo(c.id)).collect::<Vec<_>>()
    };
    assert!(tipo_de("CENTRO").iter().all(|t| *t == TipoLinha::Centro));
    assert!(tipo_de("OCULTA").iter().all(|t| *t == TipoLinha::Oculta));
    assert!(tipo_de("FINA").iter().all(|t| *t == TipoLinha::Fina));
    assert_eq!(tipo_de("VISIVEL").len(), 4);
    // círculo e arco voltam exatos, não em pedaços
    let centro = volta.esbocos.iter().find(|e| e.nome == "CENTRO").unwrap();
    assert!(centro.dados.curves.iter().any(|c| matches!(&c.geom, Curve::Circle { c, r } if (*r - 15.).abs() < 1e-9 && perto(centro.dados.pos(*c), [120., 90.], 1e-9))));
    let oculta = volta.esbocos.iter().find(|e| e.nome == "OCULTA").unwrap();
    assert!(matches!(oculta.dados.curves[..], [ref c] if matches!(c.geom, Curve::Arc { .. })));
    let nota = volta.notes.iter().find(|n| n.text == "CHAPA 3 mm").expect("texto vira nota");
    assert!((nota.size - 5.).abs() < 1e-9);

    // e grava de novo igual: mesmas entidades por camada e mesma caixa
    let de_novo = fenkra_dxf::ler(export::dxf(&do_conteudo(&volta, f2)).as_bytes()).unwrap();
    assert_eq!(inventario(&de_novo), inventario(&ida));
    // o texto cai no mesmo ponto de inserção, com o mesmo anexo
    let texto = |d: &Desenho| d.itens.iter().find_map(|i| if let Entidade::Texto { pos, anexo, .. } = &i.entidade { Some((*pos, *anexo)) } else { None }).unwrap();
    let ((p1, a1), (p2, a2)) = (texto(&ida), texto(&de_novo));
    assert!(perto(p1, p2, 1e-6) && a1 == a2, "{p1:?} {p2:?}");
    let (a, b) = (caixa(&ida).unwrap(), caixa(&de_novo).unwrap());
    assert!(perto(a.0, b.0, 1e-6) && perto(a.1, b.1, 1e-6), "{a:?} {b:?}");
    // linhas de 40 a 250 em x e de 50 a 130 em y; o texto, acima delas, fecha a caixa por cima
    assert!(perto(a.0, [40., 50.], 1e-6) && perto(a.1, [250., p1[1]], 1e-6) && p1[1] > 130., "{a:?}");
}

#[test]
fn folha_inteira_em_dxf_abre_como_desenho_a3_sem_repetir_a_moldura() {
    let (d, folha, _) = folha_de_prova();
    let dir = pasta("inteira");
    let mut m = Models::new(None);
    let tudo = sheet(&d, folha, &mut m);
    let caminho = dir.join("folha.dxf");
    std::fs::write(&caminho, export::dxf(&tudo)).unwrap();

    let (aberto, r) = abrir_como_desenho(&caminho).unwrap();
    assert_eq!((aberto.sheets.len(), aberto.sheets[0].format, aberto.sheets[0].portrait), (1, Format::A3, false));
    assert!(aberto.esbocos.iter().all(|e| e.dono == DonoEsboco::Folha(aberto.sheets[0].id) && !e.cotas));
    // a moldura, as zonas e os campos iguais da legenda a folha nova já desenha: não entram de novo
    assert!(r.da_moldura > 50, "{r:?}");
    assert!(!aberto.esbocos.iter().any(|e| e.nome == "MOLDURA"), "{:?}", aberto.esbocos.iter().map(|e| &e.nome).collect::<Vec<_>>());
    assert!(!aberto.notes.iter().any(|n| n.text == "A3" || n.text == "1"), "{:?}", aberto.notes.iter().map(|n| &n.text).collect::<Vec<_>>());
    assert!(aberto.notes.iter().any(|n| n.text == "CHAPA 3 mm"));
    // o que sobra da legenda é só o que difere (o título, que era outro)
    assert!(r.textos <= 3, "{:?}", aberto.notes.iter().map(|n| &n.text).collect::<Vec<_>>());
    let ida = fenkra_dxf::ler(std::fs::read(&caminho).unwrap().as_slice()).unwrap();
    let (lo, hi) = caixa(&ida).unwrap();
    assert!(lo[0] >= -1e-6 && lo[1] >= -1e-6 && hi[0] <= 420. + 1e-6 && hi[1] <= 297. + 1e-6, "{lo:?} {hi:?}");
    // a folha aberta grava e o furo de centro continua lá
    let de_novo = fenkra_dxf::ler(export::dxf(&do_conteudo(&aberto, aberto.sheets[0].id)).as_bytes()).unwrap();
    assert!(de_novo.itens.iter().any(|i| i.camada == "CENTRO" && matches!(i.entidade, Entidade::Circulo { raio, .. } if (raio - 15.).abs() < 1e-9)));
    // e o desenho aberto sobrevive ao arquivo .fkd
    assert_eq!(Drawing::from_json(&aberto.to_json()).unwrap().esbocos, aberto.esbocos);
}

#[test]
fn svg_da_folha_tambem_abre_e_outro_formato_explica() {
    let (d, folha, _) = folha_de_prova();
    let dir = pasta("svg");
    let caminho = dir.join("folha.svg");
    std::fs::write(&caminho, export::svg(&do_conteudo(&d, folha), d.sheets[0].size())).unwrap();
    let (aberto, r) = abrir_como_desenho(&caminho).unwrap();
    assert!(r.curvas >= 7, "{r:?}");
    // o SVG da folha grava pontos com três casas: o círculo volta como círculo, com o raio a 0,01 mm
    let circulos = aberto.esbocos.iter().flat_map(|e| &e.dados.curves).filter(|c| matches!(&c.geom, Curve::Circle { r, .. } if (*r - 15.).abs() < 1e-2)).count();
    assert_eq!(circulos, 1);

    let step = dir.join("peca.step");
    std::fs::write(&step, "ISO-10303-21;\nHEADER;\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;").unwrap();
    let e = abrir_como_desenho(&step).unwrap_err().to_string();
    assert!(e.contains("Importar"), "{e}");
}

#[test]
fn hachura_fica_de_fora_e_e_contada() {
    let mut ida = Desenho::default();
    ida.itens.push(fenkra_dxf::Item::novo("0", Entidade::Hachura { lacos: vec![vec![[0., 0.], [10., 0.], [10., 10.]]] }));
    ida.itens.push(fenkra_dxf::Item::novo("0", Entidade::Linha([0., 0.], [10., 0.])));
    let mut d = Drawing::new("h", Format::A4);
    let f = d.sheets[0].id;
    let r = importar_na_folha(&mut d, f, &ida);
    assert_eq!((r.esbocos, r.curvas, r.sem_unidade), (1, 1, true));
    assert_eq!(r.fora.get("HATCH"), Some(&1));
}
