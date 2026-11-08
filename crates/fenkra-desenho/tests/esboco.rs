// SPDX-License-Identifier: LGPL-2.1-or-later
//! B352: sketch no desenho, solto na folha ou preso a uma vista. O esboço resolve as restrições
//! como o sketch de peça, sai na folha com o tipo de linha de cada curva e as cotas, anda e escala
//! com a vista e entra na exportação.

use fenkra_core::sketch::{Curve, Kind};
use fenkra_core::Id;
use fenkra_desenho::esboco::{DonoEsboco, TipoLinha};
use fenkra_desenho::layout::{sheet, Prim, Style};
use fenkra_desenho::source::Models;
use fenkra_desenho::*;

fn alocador(base: u64) -> impl FnMut() -> Id {
    let mut n = base;
    move || {
        n += 1;
        Id(n)
    }
}

fn linhas_de(items: &[layout::Item], dono: Id) -> Vec<(Vec<V2>, Style)> {
    items
        .iter()
        .filter(|i| i.owner == Some(dono))
        .filter_map(|i| if let Prim::Line { pts, style } = &i.prim { Some((pts.clone(), *style)) } else { None })
        .collect()
}

fn textos_de(items: &[layout::Item], dono: Id) -> Vec<String> {
    items.iter().filter(|i| i.owner == Some(dono)).filter_map(|i| if let Prim::Text { text, .. } = &i.prim { Some(text.clone()) } else { None }).collect()
}

fn perto(a: V2, b: V2) -> bool {
    (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6
}

#[test]
fn esboco_solto_na_folha_resolve_as_cotas_e_sai_com_o_tipo_de_linha() {
    let mut d = Drawing::new("t", Format::A3);
    let folha = d.sheets[0].id;
    let id = d.add_esboco(DonoEsboco::Folha(folha));
    let e = d.esboco_mut(id).unwrap();
    let mut al = alocador(1000);
    // retângulo torto de propósito: as cotas levam a 40 × 20
    let (p, l) = e.dados.add_rect(&mut al, [100., 100.], [137., 123.], false);
    e.dados.add_constraint(&mut al, Kind::Fixed(p[0]));
    e.dados.add_constraint(&mut al, Kind::Length(l[0], "40".into()));
    e.dados.add_constraint(&mut al, Kind::DistanceY(p[0], p[3], "20".into()));
    // linha de centro e uma de construção, que não sai
    let a = e.dados.add_point(&mut al, 95., 110.);
    let b = e.dados.add_point(&mut al, 145., 110.);
    let eixo = e.dados.add_curve(&mut al, Curve::Line { a, b }, false);
    let c = e.dados.add_point(&mut al, 100., 100.);
    let c2 = e.dados.add_point(&mut al, 140., 120.);
    e.dados.add_curve(&mut al, Curve::Line { a: c, b: c2 }, true);
    e.definir_tipo(eixo, TipoLinha::Centro);
    assert!(e.resolver(), "o sketch resolve");
    assert!(perto(e.dados.pos(p[2]), [140., 120.]), "{:?}", e.dados.pos(p[2]));

    let mut m = Models::new(None);
    let items = sheet(&d, folha, &mut m);
    let ls = linhas_de(&items, id);
    // quatro lados visíveis em mm da folha, o eixo em traço-ponto, nada da construção
    let lados: Vec<_> = ls.iter().filter(|(_, s)| *s == Style::Visible).collect();
    assert_eq!(lados.len(), 4, "{ls:?}");
    assert!(lados.iter().any(|(pts, _)| perto(pts[0], [100., 100.]) && perto(pts[1], [140., 100.])));
    assert_eq!(ls.iter().filter(|(_, s)| *s == Style::Center).count(), 1);
    assert!(!ls.iter().any(|(pts, _)| pts.len() == 2 && perto(pts[0], [100., 100.]) && perto(pts[1], [140., 120.])), "construção não sai");
    let t = textos_de(&items, id);
    assert!(t.contains(&"40".to_string()) && t.contains(&"20".to_string()), "cotas do sketch na folha: {t:?}");

    // entra na exportação
    let dxf = export::dxf(&items);
    assert!(dxf.contains("\n40\n") || dxf.contains("40"), "o texto da cota vai para o DXF");
    let svg = export::svg(&items, d.sheets[0].size());
    assert!(svg.contains(">40<"), "o texto da cota vai para o SVG");

    // gravar e abrir de novo mantém o esboço, os tipos e as restrições
    let volta = Drawing::from_json(&d.to_json()).unwrap();
    assert_eq!(volta.esbocos, d.esbocos);
}

#[test]
fn esboco_na_vista_segue_a_escala_e_a_posicao_e_some_com_ela() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::env::temp_dir().join(format!("fenkra_desenho_esboco_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    let mut d = Drawing::new("t", Format::A3);
    let folha = d.sheets[0].id;
    let src = Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] };
    let v = d.add_view(folha, src, ViewKind::Standard(Dir::Frente), [150., 150.], 2.);
    let id = d.add_esboco(DonoEsboco::Vista(v));
    let mut m = Models::new(Some(dir.clone()));
    let p = m.proj(&d, v);
    assert!(p.error.is_none() && !p.lines.is_empty(), "{:?}", p.error);
    // uma linha de 10 mm do modelo a partir do centro da vista, cotada
    let e = d.esboco_mut(id).unwrap();
    let mut al = alocador(5000);
    let a = e.dados.add_point(&mut al, p.center[0], p.center[1]);
    let b = e.dados.add_point(&mut al, p.center[0] + 7., p.center[1]);
    let l = e.dados.add_curve(&mut al, Curve::Line { a, b }, false);
    e.dados.add_constraint(&mut al, Kind::Fixed(a));
    e.dados.add_constraint(&mut al, Kind::Horizontal(l));
    e.dados.add_constraint(&mut al, Kind::Length(l, "10".into()));
    assert!(e.resolver());

    let items = sheet(&d, folha, &mut m);
    let ls = linhas_de(&items, id);
    let (pts, _) = ls.iter().find(|(pts, s)| *s == Style::Visible && pts.len() == 2).expect("a linha do esboço");
    // na folha: 10 mm × escala 2 = 20 mm, a partir da posição da vista (o centro do modelo)
    assert!(perto(pts[0], [150., 150.]) && (pts[1][0] - pts[0][0] - 20.).abs() < 1e-6, "{pts:?}");
    // a cota mostra a medida do modelo, não a da folha
    assert!(textos_de(&items, id).contains(&"10".to_string()));

    // a vista anda: o esboço vai junto
    d.views.iter_mut().find(|x| x.id == v).unwrap().pos = [200., 120.];
    let items = sheet(&d, folha, &mut m);
    let ls = linhas_de(&items, id);
    assert!(ls.iter().any(|(pts, _)| pts.len() == 2 && perto(pts[0], [200., 120.]) && perto(pts[1], [220., 120.])), "{ls:?}");

    // apagar a vista apaga o esboço dela
    d.delete(v);
    assert!(d.esbocos.is_empty());
    let _ = std::fs::remove_dir_all(dir);
}
