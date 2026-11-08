// SPDX-License-Identifier: LGPL-2.1-or-later
//! B117: plano de inspeção. Monta o desenho da placa de exemplo com cotas de vários tipos de
//! tolerância, um quadro de tolerância geométrica e uma rugosidade, e confere a numeração, os
//! limites, a tabela, os balões na folha e as planilhas.

use fenkra_core::Id;
use fenkra_desenho::inspecao::{self, TipoCarac};
use fenkra_desenho::layout::{owner_boxes, sheet, Prim};
use fenkra_desenho::source::Models;
use fenkra_desenho::*;

fn perto(a: Option<f64>, b: f64) -> bool {
    a.is_some_and(|a| (a - b).abs() < 1e-6)
}

#[test]
fn plano_numera_limita_e_exporta_as_caracteristicas() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::env::temp_dir().join(format!("fenkra_desenho_inspecao_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    let src = Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] };
    let mut d = Drawing::new("Placa", Format::A3);
    let s = d.sheets[0].id;
    let v = d.add_view(s, src, ViewKind::Standard(Dir::Topo), [200., 160.], 1.);
    let mut m = Models::new(Some(dir.clone()));
    let p = m.proj(&d, v);
    assert!(p.error.is_none(), "{:?}", p.error);
    let ax = p.axes.unwrap();
    let (lo, hi) = (p.lo, p.hi);
    let canto = |x: f64, y: f64| ax.unproject([x, y]);
    let mut cota = |d: &mut Drawing, kind: DimKind, a: [f64; 2], b: [f64; 2], f: &dyn Fn(&mut Dim)| {
        let id = d.alloc();
        let mut dm = Dim { id, view: v, kind, anchors: vec![canto(a[0], a[1]), canto(b[0], b[1])], ..Default::default() };
        f(&mut dm);
        d.dims.push(dm);
        id
    };
    // largura embaixo com ±0,1; altura à esquerda com classe h9; largura em cima sem tolerância
    // (vale a geral ISO 2768-m da nota); uma de referência, que fica de fora
    let larg = cota(&mut d, DimKind::Horizontal, [lo[0], lo[1]], [hi[0], lo[1]], &|dm| {
        dm.tol_mode = TolMode::Simetrica;
        dm.tol_a = 0.1;
        dm.opcoes.inspecao = true;
    });
    let alt = cota(&mut d, DimKind::Vertical, [lo[0], lo[1]], [lo[0], hi[1]], &|dm| dm.tol = "h9".into());
    let geral = cota(&mut d, DimKind::Horizontal, [lo[0], hi[1]], [hi[0], hi[1]], &|_| {});
    cota(&mut d, DimKind::Vertical, [hi[0], lo[1]], [hi[0], hi[1]], &|dm| dm.opcoes.parenteses = true);
    let id = d.alloc();
    d.notes.push(Note { id, sheet: s, kind: NoteKind::Text, pos: [30., 40.], text: "Tolerâncias gerais ISO 2768-mK".into(), ..Default::default() });
    let gdt = d.alloc();
    d.notes.push(Note { id: gdt, sheet: s, kind: NoteKind::Gdt, pos: [330., 230.], leader: Some([300., 200.]), gdt: Some(DwgGdt { ch: fenkra_core::pmi::GdtChar::Planeza, tol: "0,05".into(), datums: String::new(), extra: Default::default() }), ..Default::default() });
    let rug = d.alloc();
    d.notes.push(Note { id: rug, sheet: s, kind: NoteKind::Roughness, pos: [60., 250.], text: "Ra 1,6".into(), ..Default::default() });

    let cs = inspecao::caracteristicas(&d, &mut m);
    assert_eq!(cs.len(), 5, "3 cotas (a de referência fica de fora), o quadro e a rugosidade: {cs:#?}");
    assert_eq!(cs.iter().map(|c| c.numero).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5]);
    // ordem de leitura: linha A (rugosidade), linha B da esquerda para a direita (largura de cima,
    // quadro), depois a altura (linha C) e a largura de baixo (linha D)
    let ordem: Vec<Id> = cs.iter().map(|c| c.dono).collect();
    assert_eq!(ordem, vec![rug, geral, gdt, alt, larg], "{:?}", cs.iter().map(|c| (c.dono, &c.zona, c.pos)).collect::<Vec<_>>());
    assert!(cs.iter().all(|c| !c.zona.is_empty()));
    let de = |id: Id| cs.iter().find(|c| c.dono == id).unwrap();
    let (cl, ca, cg) = (de(larg), de(alt), de(geral));
    let w = cl.nominal.unwrap();
    assert!(perto(cl.minimo, w - 0.1) && perto(cl.maximo, w + 0.1) && cl.cem_por_cento);
    // h9: o máximo é o nominal e o mínimo fica abaixo pelo IT9
    let h = ca.nominal.unwrap();
    assert!(perto(ca.maximo, h) && ca.minimo.unwrap() < h - 0.02, "{ca:?}");
    // ISO 2768-m, de 30 a 120 mm: ±0,3; de 120 a 400: ±0,5
    let tg = if w <= 120. { 0.3 } else { 0.5 };
    assert!(perto(cg.minimo, w - tg) && perto(cg.maximo, w + tg), "{cg:?}");
    assert_eq!(de(gdt).tipo, TipoCarac::Geometrica);
    assert!(de(gdt).requisito.contains("0,05") && perto(de(gdt).maximo, 0.05));
    assert_eq!(de(rug).unidade, "µm");

    // tabela na folha e balões numerados junto de cada característica
    let tid = d.alloc();
    d.tables.push(Table { id: tid, sheet: s, kind: TableKind::Caracteristicas, source: Source { path: String::new(), display_state: String::new(), hidden: vec![] }, pos: [250., 120.], level: Default::default(), cfg: Default::default() });
    let t = m.table(&d.tables[0], &d);
    assert_eq!(t.rows.len(), 5);
    assert_eq!(t.headers[0], "Nº");
    assert_eq!(t.rows[0][0], "1");
    d.inspecao.baloes = true;
    let items = sheet(&d, s, &mut m);
    let textos_de = |id: Id| items.iter().filter(|i| i.owner == Some(id)).filter_map(|i| if let Prim::Text { text, .. } = &i.prim { Some(text.clone()) } else { None }).collect::<Vec<_>>();
    for c in &cs {
        assert!(textos_de(c.dono).contains(&c.numero.to_string()), "balão {} junto de {:?}", c.numero, c.dono);
    }
    // o balão fica fora da caixa da cota, ao lado dela
    assert!(owner_boxes(&items).iter().any(|b| b.0 == larg));
    let sem = {
        let mut d2 = d.clone();
        d2.inspecao.baloes = false;
        sheet(&d2, s, &mut m).len()
    };
    assert!(items.len() > sem);

    // planilhas: CSV do formulário 3 e XLSX que abre (ZIP com a planilha)
    let csv = inspecao::csv(&d, &cs);
    let l: Vec<&str> = csv.lines().collect();
    assert_eq!(l.len(), 6);
    assert!(l[0].starts_with("5. Nº da característica;6. Local de referência"));
    assert!(l[1].starts_with("1;F1 "));
    let x = inspecao::xlsx(&d, &cs, "FK-0001", "Placa");
    assert_eq!(&x[..2], b"PK");
    // conferência manual com um leitor de ZIP/planilha: FENKRA_TESTE_XLSX=<arquivo> grava a saída
    if let Ok(p) = std::env::var("FENKRA_TESTE_XLSX") {
        std::fs::write(p, &x).unwrap();
    }
    let texto = String::from_utf8_lossy(&x);
    assert!(texto.contains("xl/worksheets/sheet1.xml") && texto.contains("FK-0001") && texto.contains("9. Resultado"));
    // o diretório central fecha o arquivo: assinatura do fim no lugar
    assert_eq!(&x[x.len() - 22..x.len() - 18], &[0x50, 0x4b, 0x05, 0x06]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn zona_segue_o_zoneamento_da_moldura() {
    // A3: 8 colunas e 6 linhas; canto de cima à esquerda é A1, embaixo à direita F8
    assert_eq!(inspecao::zona([420., 297.], [25., 280.]), "A1");
    assert_eq!(inspecao::zona([420., 297.], [405., 15.]), "F8");
}

#[test]
fn zip_sem_compressao_tem_crc_e_tamanhos_certos() {
    let z = inspecao::zip_sem_compressao(&[("a.txt", b"abc")]);
    // CRC32 de "abc" = 0x352441C2
    assert_eq!(&z[14..18], &0x3524_41C2u32.to_le_bytes());
    assert_eq!(&z[18..22], &3u32.to_le_bytes());
}

fn carac(numero: usize, min: Option<f64>, max: Option<f64>) -> inspecao::Caracteristica {
    inspecao::Caracteristica { numero, dono: Id(numero as u64), folha: Id(1), zona: "A1".into(), tipo: TipoCarac::Dimensao, requisito: "x".into(), nominal: None, minimo: min, maximo: max, unidade: "mm", cem_por_cento: false, pos: [0., 0.] }
}

#[test]
fn medicoes_voltam_do_formulario_preenchido_e_dizem_a_situacao() {
    // o CSV que o Fenkra grava, preenchido na coluna "9. Resultado" (uma linha com aspas e ponto e vírgula)
    let csv = "5. Nº da característica;6. Local de referência (folha/zona);7. Designador;8. Requisito;Nominal;Mínimo;Máximo;9. Resultado;10. Ferramenta;11. Não conformidade;14. Observações\n\
               1;F1 A1;;\"Ra 1;6 (µm)\";;;1,6;1,2;rugosímetro;;\n\
               2;F1 B2;;50 (mm);50;49,9;50,1;50,15;paquímetro;;\n\
               3;F1 B3;;\"Ø10 (mm)\";10;9,964;10;9,98;;;\n\
               4;F1 C1;;;;;;;;;\n\
               5;F1 D1;;;;;;visual ok;;;\n";
    let m = inspecao::importar_medicoes(csv).unwrap();
    assert_eq!(m.iter().map(|x| (x.numero, x.valor.as_str())).collect::<Vec<_>>(), vec![(1, "1,2"), (2, "50,15"), (3, "9,98"), (5, "visual ok")]);
    // planilha simples de duas colunas, sem cabeçalho, com vírgula
    let simples = inspecao::importar_medicoes("1,1.2\n2,50.05\n").unwrap();
    assert_eq!(simples.len(), 2);
    assert!(inspecao::importar_medicoes("").is_err());
    assert!(inspecao::importar_medicoes("a;b\nx;y\n").is_err());

    use inspecao::Situacao::*;
    assert_eq!(inspecao::situacao(&carac(1, None, Some(1.6)), "1,2"), Conforme);
    assert_eq!(inspecao::situacao(&carac(2, Some(49.9), Some(50.1)), "50,15"), NaoConforme);
    assert_eq!(inspecao::situacao(&carac(3, Some(9.964), Some(10.)), "10"), Conforme, "o limite é aceito");
    assert_eq!(inspecao::situacao(&carac(3, Some(9.964), Some(10.)), "9,96"), NaoConforme);
    assert_eq!(inspecao::situacao(&carac(4, None, None), "3"), SemLimite);
    assert_eq!(inspecao::situacao(&carac(5, None, Some(1.)), "visual ok"), Texto);

    // no desenho: o formulário 3 sai com o resultado e a não conformidade; o resumo conta
    let mut d = Drawing::new("Placa", Format::A3);
    d.inspecao.medicoes = m;
    let cs = vec![carac(1, None, Some(1.6)), carac(2, Some(49.9), Some(50.1)), carac(3, Some(9.964), Some(10.)), carac(4, None, None)];
    assert_eq!(inspecao::resumo(&d, &cs), (3, 1, 1));
    let saida = inspecao::csv(&d, &cs);
    let l: Vec<&str> = saida.lines().collect();
    assert!(l[2].contains(";50,15;") && l[2].ends_with(";Sim;Dimensão"), "{}", l[2]);
    assert!(!l[1].contains(";Sim;"), "{}", l[1]);
}

#[test]
fn xlsx_leva_os_formularios_1_2_e_3() {
    let mut d = Drawing::new("Placa", Format::A3);
    d.title.pn = "FK-0001".into();
    d.title.title = "Placa de apoio".into();
    d.title.material = "Aço SAE 1020".into();
    d.title.revision = "B".into();
    d.title.drawn_by = "JV".into();
    let s = d.sheets[0].id;
    let id = d.alloc();
    d.notes.push(Note { id, sheet: s, kind: NoteKind::Text, pos: [30., 40.], text: "Tolerâncias gerais ISO 2768-m\nZincado conforme NBR 10476, 8 µm\nQuebrar cantos vivos".into(), ..Default::default() });
    let cs = vec![carac(1, Some(49.9), Some(50.1)), carac(2, None, Some(1.6))];
    d.inspecao.medicoes = vec![inspecao::Medicao { numero: 1, valor: "50".into() }, inspecao::Medicao { numero: 2, valor: "1,4".into() }];
    let f1 = inspecao::form1(&d, &cs, "", "", false);
    let campo = |n: &str| f1.iter().find(|(k, _)| k.starts_with(n)).map(|(_, v)| v.as_str()).unwrap();
    assert_eq!(campo("1. "), "FK-0001");
    assert_eq!(campo("2. "), "Placa de apoio");
    assert_eq!(campo("5. "), "B");
    assert_eq!(campo("13. "), "Peça");
    assert_eq!(campo("15. "), "2");
    assert_eq!(campo("19. "), "Sim", "tudo medido e conforme");
    assert_eq!(campo("20. "), "JV");
    // formulário 2: o material e a nota com norma (não a tolerância geral nem a nota sem norma)
    let f2 = inspecao::form2(&d);
    assert_eq!(f2.len(), 2, "{f2:?}");
    assert_eq!(f2[0][0], "Aço SAE 1020");
    assert!(f2[1][0].starts_with("Zincado") && f2[1][1] == "NBR 10476", "{:?}", f2[1]);
    let x = inspecao::xlsx(&d, &cs, "FK-0001", "Placa de apoio");
    let texto = String::from_utf8_lossy(&x);
    for parte in ["xl/worksheets/sheet1.xml", "xl/worksheets/sheet2.xml", "xl/worksheets/sheet3.xml", "Formulario 1", "Formulario 2", "Formulario 3", "NBR 10476", "19. Resultado conforme", "9. Resultado"] {
        assert!(texto.contains(parte), "sem {parte}");
    }
    // o resultado do formulário 3 vai como número
    assert!(texto.contains("<v>50</v>"), "resultado numérico");
    // não conforme muda o campo 19
    d.inspecao.medicoes[0].valor = "51".into();
    assert!(inspecao::form1(&d, &cs, "", "", true).iter().any(|(k, v)| k.starts_with("19. ") && v.starts_with("Não")));
    assert!(inspecao::form1(&d, &cs, "", "", true).iter().any(|(k, v)| k.starts_with("13. ") && v == "Montagem"));
}
