// SPDX-License-Identifier: LGPL-2.1-or-later
//! Folhas do desenho como vetor nativo: as mesmas primitivas que a tela e o PDF desenham (linhas,
//! polígonos cheios, textos), em milímetros de papel com Y para cima. Não é PDF nem imagem: o
//! visualizador desenha em SVG, dá zoom sem perder nitidez, mede e pinta com as cores do tema.

use fenkra_core::Id;
use fenkra_desenho::layout::{self, Align, Item, Prim, Style};
use fenkra_desenho::source::Models;
use fenkra_desenho::Drawing;
use serde_json::{json, Value};

fn r3(x: f64) -> f64 {
    let v = (x * 1000.).round() / 1000.;
    if v == 0. {
        0.
    } else {
        v
    }
}

/// Código do tipo de linha: 0 visível, 1 fina, 2 oculta, 3 centro, 4 grossa.
pub fn codigo_estilo(s: Style) -> u8 {
    match s {
        Style::Visible => 0,
        Style::Thin => 1,
        Style::Hidden => 2,
        Style::Center => 3,
        Style::Thick => 4,
    }
}

fn item(i: &Item) -> Value {
    let mut v = match &i.prim {
        Prim::Line { pts, style } => json!({"t": "l", "p": pts.iter().flat_map(|p| [r3(p[0]), r3(p[1])]).collect::<Vec<_>>(), "e": codigo_estilo(*style)}),
        Prim::Fill { pts } => json!({"t": "f", "p": pts.iter().flat_map(|p| [r3(p[0]), r3(p[1])]).collect::<Vec<_>>()}),
        Prim::Text { pos, text, size, angle, align } => json!({
            "t": "x", "p": [r3(pos[0]), r3(pos[1])], "s": text, "h": r3(*size), "a": r3(*angle),
            "al": match align { Align::Left => 0, Align::Center => 1, Align::Right => 2 }
        }),
    };
    let o = v.as_object_mut().expect("objeto");
    if let Some(c) = i.color {
        o.insert("c".into(), json!(c));
    }
    if let Some(w) = i.width {
        o.insert("w".into(), json!(r3(w)));
    }
    if i.bold {
        o.insert("b".into(), json!(1));
    }
    if i.alert {
        o.insert("er".into(), json!(1));
    }
    if let Some(Id(d)) = i.owner {
        o.insert("o".into(), json!(d));
    }
    v
}

/// Uma folha: itens, tamanho e as caixas das vistas com a escala, para medir em milímetros do modelo.
pub fn folha(d: &Drawing, sheet: Id, m: &mut Models) -> Option<Value> {
    let sh = d.sheet(sheet)?;
    let itens = layout::sheet(d, sheet, m);
    let caixas = layout::owner_boxes(&itens);
    let vistas: Vec<Value> = d
        .views
        .iter()
        .filter(|v| v.sheet == sheet)
        .filter_map(|v| {
            let (_, lo, hi) = caixas.iter().find(|c| c.0 == v.id)?;
            Some(json!({"id": v.id.0, "nome": v.name, "escala": v.scale, "caixa": [r3(lo[0]), r3(lo[1]), r3(hi[0]), r3(hi[1])], "origem": v.source.path}))
        })
        .collect();
    let [w, h] = sh.size();
    Some(json!({
        "id": sheet.0, "nome": sh.name, "largura": w, "altura": h,
        "formato": sh.format.label(), "vistas": vistas,
        "itens": itens.iter().map(item).collect::<Vec<_>>(),
    }))
}

/// Texto simples de todas as tabelas do desenho, para a busca do visualizador.
pub fn tabelas(d: &Drawing, m: &mut Models) -> Vec<Value> {
    d.tables
        .iter()
        .map(|t| {
            let dados = m.table(t, d);
            json!({"id": t.id.0, "titulo": t.kind.label(), "cabecalho": dados.headers, "linhas": dados.rows})
        })
        .collect()
}
