// SPDX-License-Identifier: LGPL-2.1-or-later
//! B512: modelo de folha com bloco de título montado por caixas. Cria um modelo com três campos
//! ligados (código, material e uma propriedade personalizada), aplica numa folha e confere os valores;
//! muda a peça e confere a atualização; grava e relê o modelo e o desenho; edita o modelo e confere
//! que as duas folhas que o usam acompanham; exporta PDF e DXF e procura os textos na saída.

use fenkra_desenho::layout::{self, Prim};
use fenkra_desenho::modelo_folha::{BlocoTitulo, Conteudo, Corte, ModeloFolha};
use fenkra_desenho::source::Models;
use fenkra_desenho::*;
use std::path::Path;

fn grava_peca(dir: &Path, material: &str, cliente: &str) {
    let mut doc = fenkra_solidos::example_document();
    doc.props.pn = "PN-123".into();
    doc.props.description = "Suporte da base".into();
    doc.props.material = material.into();
    doc.props.custom = vec![("Cliente".into(), cliente.into())];
    let p = dir.join("suporte.fkp");
    std::fs::write(&p, doc.to_json()).unwrap();
    // o relógio do arquivo anda: a leitura em cache percebe a mudança mesmo dentro do mesmo segundo
    let f = std::fs::File::options().write(true).open(&p).unwrap();
    let agora = std::time::SystemTime::now() + std::time::Duration::from_secs(if cliente == "ACME Ltda" { 0 } else { 5 });
    f.set_modified(agora).unwrap();
}

/// Textos da folha que caem dentro da caixa dada.
fn textos_em(items: &[layout::Item], caixa: (V2, V2)) -> Vec<String> {
    let (lo, hi) = caixa;
    items
        .iter()
        .filter_map(|it| match &it.prim {
            Prim::Text { pos, text, .. } if pos[0] >= lo[0] - 0.01 && pos[0] <= hi[0] + 0.01 && pos[1] >= lo[1] - 0.01 && pos[1] <= hi[1] + 0.01 => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Modelo com três caixas lado a lado, cada uma com um campo ligado, e uma quarta embaixo com
/// texto fixo.
fn modelo_com_tres_campos() -> ModeloFolha {
    let mut m = ModeloFolha::novo("Cliente ACME", Format::A3, false);
    m.margens.direita = 12.;
    m.bloco = BlocoTitulo::vazio(160., 24.);
    let o = [0., 0.];
    let baixo = m.bloco.dividir(o, &[], Corte::Horizontal).unwrap(); // [0] em cima, [1] embaixo
    assert_eq!(baixo, vec![1]);
    m.bloco.dividir(o, &[0], Corte::Vertical).unwrap(); // [0,0] | [0,1]
    m.bloco.dividir(o, &[0, 1], Corte::Vertical).unwrap(); // [0,1,0] | [0,1,1]
    let cel = |m: &mut ModeloFolha, c: &[u8], rot: &str, txt: &str| {
        let k = m.bloco.celula_mut(c).unwrap();
        k.rotulo = rot.into();
        k.texto = txt.into();
    };
    cel(&mut m, &[0, 0], "CODIGO", "{codigo}");
    cel(&mut m, &[0, 1, 0], "MATERIAL", "{material}");
    cel(&mut m, &[0, 1, 1], "CLIENTE", "{cliente}");
    cel(&mut m, &[1], "", "Desenho de fabricacao");
    m
}

#[test]
fn modelo_com_tres_campos_preenche_atualiza_grava_e_exporta() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::env::temp_dir().join(format!("fenkra_b512_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    grava_peca(&dir, "Inox 304", "ACME Ltda");
    let src = Source { path: "suporte.fkp".into(), display_state: String::new(), hidden: vec![] };
    let mut d = Drawing::new("Suporte", Format::A4);
    let s1 = d.sheets[0].id;
    d.add_view(s1, src, ViewKind::Standard(Dir::Frente), [150., 180.], 1.);
    let modelo = modelo_com_tres_campos();
    assert_eq!(modelo.bloco.campos(), vec!["codigo", "material", "cliente"]);
    d.aplicar_modelo(s1, modelo.clone());
    // a folha passa a ter o formato do modelo
    assert_eq!(d.sheets[0].format, Format::A3);
    let mut m = Models::new(Some(dir.clone()));

    // 1) valores preenchidos, dentro do bloco (canto de baixo à direita, encostado na moldura)
    let sh = d.sheets[0].clone();
    let caixa = ocupacao::legenda_da_folha(&d, &sh);
    assert_eq!(caixa, ([420. - 12. - 160., 10.], [420. - 12., 34.]));
    let items = layout::sheet(&d, s1, &mut m);
    let t = textos_em(&items, caixa);
    for esperado in ["PN-123", "Inox 304", "ACME Ltda", "Desenho de fabricacao", "CODIGO", "MATERIAL", "CLIENTE"] {
        assert!(t.iter().any(|x| x == esperado), "falta {esperado} no bloco: {t:?}");
    }
    // a legenda pronta não é desenhada junto (nada de TÍTULO/FORMATO do ISO 7200)
    assert!(!items.iter().any(|it| matches!(&it.prim, Prim::Text { text, .. } if text == "FORMATO")));
    // o contorno do bloco é grosso e as três divisórias, finas
    let grossas = items.iter().filter(|it| matches!(&it.prim, Prim::Line { style: layout::Style::Thick, pts } if pts.len() == 5 && (pts[0][0] - caixa.0[0]).abs() < 1e-9)).count();
    assert_eq!(grossas, 1);

    // 2) muda a peça: material e cliente novos chegam ao bloco
    grava_peca(&dir, "Aluminio 6061", "Beta SA");
    assert!(m.refresh(), "a mudança no arquivo da peça não foi percebida");
    let t = textos_em(&layout::sheet(&d, s1, &mut m), caixa);
    assert!(t.iter().any(|x| x == "Aluminio 6061") && t.iter().any(|x| x == "Beta SA"), "{t:?}");
    assert!(!t.iter().any(|x| x == "ACME Ltda"), "{t:?}");

    // propriedade do desenho só completa o que a peça não tem
    d.propriedades = vec![("Cliente".into(), "Outro".into()), ("Obra".into(), "Galpao 3".into())];
    d.modelo_mut(&modelo.uid).unwrap().bloco.celula_mut(&[1]).unwrap().texto = "Obra: {obra}".into();
    let t = textos_em(&layout::sheet(&d, s1, &mut m), caixa);
    assert!(t.iter().any(|x| x == "Beta SA") && t.iter().any(|x| x == "Obra: Galpao 3"), "{t:?}");

    // 3) gravar e reler: o arquivo do modelo e o desenho inteiro
    let arq = dir.join("cliente.fkfolha");
    let atual = d.modelo(&modelo.uid).unwrap().clone();
    atual.gravar(&arq).unwrap();
    let relido = ModeloFolha::ler(&arq).unwrap();
    assert_eq!(relido, atual);
    let d2 = Drawing::from_json(&d.to_json()).unwrap();
    assert_eq!(d2.modelos, d.modelos);
    assert_eq!(d2.sheets[0].modelo.as_deref(), Some(modelo.uid.as_str()));
    assert_eq!(d2.propriedades, d.propriedades);
    let t2 = textos_em(&layout::sheet(&d2, s1, &mut m), caixa);
    assert!(t2.iter().any(|x| x == "PN-123") && t2.iter().any(|x| x == "Beta SA"), "{t2:?}");

    // 4) segunda folha igual; editar o modelo atualiza as duas (formato e bloco)
    let s2 = d.add_sheet_como(s1);
    assert_eq!(d.sheet(s2).unwrap().modelo.as_deref(), Some(modelo.uid.as_str()));
    let mut novo = d.modelo(&modelo.uid).unwrap().clone();
    novo.formato = Format::A2;
    novo.bloco.celula_mut(&[0, 1, 1]).unwrap().texto = "CLI {cliente}".into();
    assert_eq!(d.atualizar_modelo(novo), 2);
    assert!(d.sheets.iter().all(|s| s.format == Format::A2));
    for s in [s1, s2] {
        let sh = d.sheet(s).unwrap().clone();
        let t = textos_em(&layout::sheet(&d, s, &mut m), ocupacao::legenda_da_folha(&d, &sh));
        assert!(t.iter().any(|x| x == "CLI Beta SA"), "folha {s:?}: {t:?}");
    }

    // 5) exportação: os textos do bloco saem no PDF e no DXF
    let pdf = dir.join("suporte.pdf");
    export::save(&d, s1, &mut m, pdf.to_str().unwrap()).unwrap();
    let bytes = std::fs::read(&pdf).unwrap();
    let txt = String::from_utf8_lossy(&bytes);
    for esperado in ["(PN-123)", "(Aluminio 6061)", "(CLI Beta SA)", "(CODIGO)"] {
        assert!(txt.contains(esperado), "PDF sem {esperado}");
    }
    let dxf = dir.join("suporte.dxf");
    export::save(&d, s1, &mut m, dxf.to_str().unwrap()).unwrap();
    let txt = std::fs::read_to_string(&dxf).unwrap();
    for esperado in ["PN-123", "Aluminio 6061", "CLI Beta SA"] {
        assert!(txt.contains(esperado), "DXF sem {esperado}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Tabela de revisões, diedro e logotipo no bloco; a tabela que cairia sobre o bloco é avisada pela
/// caixa do modelo, não pela legenda pronta.
#[test]
fn revisoes_diedro_logotipo_e_ocupacao() {
    let dir = std::env::temp_dir().join(format!("fenkra_b512_logo_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // logotipo 8×4: metade esquerda azul, o resto transparente
    let mut img = image::RgbaImage::from_pixel(8, 4, image::Rgba([0, 0, 0, 0]));
    for x in 0..4 {
        for y in 0..4 {
            img.put_pixel(x, y, image::Rgba([0, 51, 204, 255]));
        }
    }
    img.save(dir.join("logo.png")).unwrap();
    let mut m = ModeloFolha::novo("Com revisões", Format::A3, false);
    m.bloco = BlocoTitulo::vazio(180., 40.);
    let o = [0., 0.];
    m.bloco.dividir(o, &[], Corte::Vertical).unwrap();
    m.bloco.dividir(o, &[0], Corte::Horizontal).unwrap();
    m.bloco.celula_mut(&[0, 0]).unwrap().conteudo = Conteudo::Imagem { caminho: "logo.png".into() };
    m.bloco.celula_mut(&[0, 1]).unwrap().conteudo = Conteudo::Diedro;
    m.bloco.celula_mut(&[1]).unwrap().conteudo = Conteudo::Revisoes { linhas: 2 };
    let mut d = Drawing::new("Rev", Format::A3);
    let s = d.sheets[0].id;
    d.revisoes = vec![
        modelo_folha::Revisao { rev: "A".into(), descricao: "Emissao".into(), data: "01/11/2026".into(), autor: "JV".into() },
        modelo_folha::Revisao { rev: "B".into(), descricao: "Furo 8".into(), data: "10/11/2026".into(), autor: "JV".into() },
        modelo_folha::Revisao { rev: "C".into(), descricao: "Chanfro".into(), data: "18/11/2026".into(), autor: "JV".into() },
    ];
    d.aplicar_modelo(s, m);
    let mut models = Models::new(Some(dir.clone()));
    let items = layout::sheet(&d, s, &mut models);
    let caixa = ocupacao::legenda_da_folha(&d, &d.sheets[0].clone());
    let t = textos_em(&items, caixa);
    // duas linhas: as revisões B e C (a mais nova embaixo); a A ficou de fora
    assert!(t.iter().any(|x| x == "Furo 8") && t.iter().any(|x| x == "Chanfro") && !t.iter().any(|x| x == "Emissao"), "{t:?}");
    assert!(t.iter().any(|x| x == "DESCRIÇÃO"), "{t:?}");
    // {revisao} passa a ser a última da tabela
    let f = layout::sheet_fields(&d, &d.sheets[0].clone(), &mut models);
    assert_eq!(f.get("revisao").map(String::as_str), Some("C"));
    // logotipo: retângulos azuis dentro da caixa dele
    let azul: Vec<&layout::Item> = items.iter().filter(|it| it.color == Some([0, 51, 204]) && matches!(it.prim, Prim::Fill { .. })).collect();
    assert!(!azul.is_empty(), "logotipo não desenhado");
    let logo_caixa = d.modelos[0].bloco.caixa_do_no(d.modelos[0].origem_do_bloco(d.sheets[0].size()), &[0, 0]).unwrap();
    for it in &azul {
        if let Prim::Fill { pts } = &it.prim {
            assert!(pts.iter().all(|p| p[0] >= logo_caixa.0[0] && p[0] <= logo_caixa.1[0] && p[1] >= logo_caixa.0[1] && p[1] <= logo_caixa.1[1]), "{pts:?} fora de {logo_caixa:?}");
        }
    }
    // diedro: o texto do método de projeção está no bloco
    assert!(t.iter().any(|x| x == d.projection.curto()), "{t:?}");
    // a área útil das vistas fica acima do bloco do modelo
    let (lo, _) = conjunto::area_util(&d, s);
    assert!((lo[1] - caixa.1[1]).abs() < 1e-9, "{lo:?} {caixa:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
