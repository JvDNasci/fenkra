// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desenvolvimento: `FENKRA_DEMO_SKETCH=<caso>` abre um sketch pronto, com o estado que só o mouse
//! produziria, para conferir por captura. Casos: `retangulo` e `canto` (painel do comando), `setas`,
//! `pegas`, `barra`, `conflito` (fantasma das restrições), `sombra`, `regiao`, `oculto`, `fatia`, `poligono`,
//! `chanfro` e `copia`. Relimitar: `aparar`, `estender`, `aparar_shift`. Seleção: `janela`, `cruzada`,
//! `cadeia`, `cadeia_offset`. Snaps: `snap`, `arraste`, `arraste_ctrl`, `inferencia`, `perpendicular`.
//! Cotas: `cota_raio`, `cota_diametro`, `cota_angulo`, `cota_suplemento`. Vínculos: `vinculo_tipo`,
//! `vinculo_pontos`, `vinculo_quebrado`. Origem e eixos (B437): `origem`, `origem_escolha`, `origem_mouse`.
//! `FENKRA_DEMO_SKETCH_CURSOR=x,y` troca o lugar do cursor.

use crate::sketch_ui::{CKind, Ent, SketchEdit, Tool};
use fenkra_core::projecao::Exata;
use fenkra_core::sketch::{Curve, Kind, LinkSrc};
use fenkra_core::*;

/// Os casos de origem (B437) mostram o painel "Origem e eixos" aberto.
pub fn abre_origem() -> bool {
    std::env::var("FENKRA_DEMO_SKETCH").is_ok_and(|c| c.starts_with("origem"))
}

/// Retângulo 20 × 10 preso só à origem e aos eixos do sketch (B437).
fn retangulo_nos_eixos(s: &mut SketchData, al: &mut impl FnMut() -> Id) {
    let e = s.garantir_eixos(al);
    let p = [[-9., -4.], [11., -6.], [9., 6.], [-11., 4.]].map(|q| s.add_point(al, q[0], q[1]));
    let l = [0, 1, 2, 3].map(|i| s.add_curve(al, Curve::Line { a: p[i], b: p[(i + 1) % 4] }, false));
    let diag = s.add_curve(al, Curve::Line { a: p[0], b: p[2] }, true);
    s.add_constraint(al, Kind::Horizontal(l[0]));
    s.add_constraint(al, Kind::Parallel(l[2], e.h));
    s.add_constraint(al, Kind::Vertical(l[1]));
    s.add_constraint(al, Kind::Parallel(l[3], e.v));
    s.add_constraint(al, Kind::Midpoint(e.origem, diag));
    s.add_constraint(al, Kind::PointLineDistance(p[0], e.h, "5".into()));
    s.add_constraint(al, Kind::PointLineDistance(p[1], e.v, "10".into()));
}

/// Os casos de vínculo mostram o painel aberto, para a captura ver o estado de cada um.
pub fn abre_vinculos() -> bool {
    std::env::var("FENKRA_DEMO_SKETCH").is_ok_and(|c| c.starts_with("vinculo"))
}

/// Põe a cota no lugar `uv`, pelo mesmo caminho do arraste do rótulo.
fn leva_cota(s: &mut SketchData, cota: Id, ancora: [f64; 2], uv: [f64; 2]) {
    let outra = s.constraints.iter().find(|c| c.id == cota).and_then(|c| crate::sketch_cota::pelo_cursor(s, &c.kind, uv));
    if let Some(c) = s.constraints.iter_mut().find(|c| c.id == cota) {
        c.label = Some([uv[0] - ancora[0], uv[1] - ancora[1]]);
        if let Some(k) = outra {
            c.kind = k;
        }
    }
}

/// Corpo com um furo redondo de raio 12: a aresta de cima do furo fica em z = 20.
fn corpo_com_furo(doc: &mut Document) {
    let mut d = SketchData::default();
    let mut n = 9_800_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    d.add_rect(&mut al, [-40., -25.], [40., 25.], false);
    let c = d.add_point(&mut al, 0., 0.);
    d.add_curve(&mut al, Curve::Circle { c, r: 12. }, false);
    let sk = doc.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: d }));
    doc.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("20".into()), reverse: false, taper: "0".into(), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
}

/// Monta o sketch do caso no documento e devolve o id dele.
pub fn preparar(doc: &mut Document) -> Option<Id> {
    let caso = std::env::var("FENKRA_DEMO_SKETCH").ok()?;
    *doc = Document::new("Sketch");
    let mut n = 9_700_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let mut s = SketchData::default();
    if caso.starts_with("origem") {
        // B437: `origem` tem a origem no centro da face lateral (x = 40) e H seguindo Y;
        // `origem_escolha` está escolhendo o centro da face com o cursor sobre a face de cima
        corpo_com_furo(doc);
        retangulo_nos_eixos(&mut s, &mut al);
        use fenkra_core::sketch_origem::{OrigemSk, OrientaSk};
        s.pos = if caso == "origem" {
            fenkra_core::sketch::SketchPos { modo: OrigemSk::CentroFace(LinkSrc::Face { point: [40., 10., 10.] }), orienta: OrientaSk::Eixo(OriginAxis::Y), ..Default::default() }
        } else {
            fenkra_core::sketch::SketchPos { modo: OrigemSk::OrigemPeca, origem: [-62., -38.], ..Default::default() }
        };
        if caso == "origem_mouse" {
            // linha solta para o teste de mouse prender ao eixo H pelo botão direito
            let (a, b) = (s.add_point(&mut al, 70., 20.), s.add_point(&mut al, 110., 32.));
            s.add_curve(&mut al, Curve::Line { a, b }, false);
        }
        return Some(doc.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s })));
    }
    if caso.starts_with("vinculo") {
        corpo_com_furo(doc);
        // a origem é a aresta redonda do furo; o vínculo foi feito quando ela era outra coisa
        let fonte = LinkSrc::Edge { mid: [-12., 0., 20.] };
        if caso == "vinculo_quebrado" {
            // a aresta sumiu: nada perto do lugar guardado
            s.add_link_exata(&mut al, LinkSrc::Edge { mid: [300., 300., 300.] }, false, &Exata::Reta([-12., 0.], [12., 0.]), false);
        } else if caso == "vinculo_tipo" {
            s.add_link_exata(&mut al, fonte, false, &Exata::Reta([-12., 0.], [12., 0.]), false);
        } else {
            // a reta do outro sketch corta o plano XZ em um ponto; o círculo do furo corta em dois
            s.add_link_points(&mut al, fonte, &[[-12., 20.]], false);
        }
        let plano = if caso == "vinculo_pontos" { OriginPlane::XZ } else { OriginPlane::XY };
        return Some(doc.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(plano), data: s })));
    }
    let (p, l) = s.add_rect(&mut al, [-40., -25.], [40., 25.], false);
    match caso.as_str() {
        "aparar" => {
            let (a, b) = (s.add_point(&mut al, -60., 10.), s.add_point(&mut al, 60., 10.));
            s.add_curve(&mut al, Curve::Line { a, b }, false);
            let c = s.add_point(&mut al, 0., 0.);
            s.add_curve(&mut al, Curve::Circle { c, r: 15. }, false);
        }
        "estender" | "aparar_shift" => {
            let (a, b) = (s.add_point(&mut al, -20., -10.), s.add_point(&mut al, 20., -10.));
            s.add_curve(&mut al, Curve::Line { a, b }, false);
        }
        "janela" | "cruzada" | "cadeia" | "cadeia_offset" | "snap" | "arraste" | "arraste_ctrl" | "inferencia" | "perpendicular" => {
            let c = s.add_point(&mut al, 80., 0.);
            s.add_curve(&mut al, Curve::Circle { c, r: 18. }, false);
            let (a, b) = (s.add_point(&mut al, -40., 45.), s.add_point(&mut al, 40., 75.));
            s.add_curve(&mut al, Curve::Line { a, b }, false);
        }
        "cota_raio" | "cota_diametro" => {
            let c = s.add_point(&mut al, 0., 0.);
            let k = s.add_curve(&mut al, Curve::Circle { c, r: 15. }, false);
            let cota = s.add_constraint(&mut al, Kind::Radius(k, "15".into()));
            let uv = if caso == "cota_raio" { [5., 4.] } else { [24., 12.] };
            leva_cota(&mut s, cota, [0., 0.], uv);
        }
        "cota_angulo" | "cota_suplemento" => {
            let (o, a, b) = (s.add_point(&mut al, 0., 50.), s.add_point(&mut al, 40., 50.), s.add_point(&mut al, 20., 85.));
            let l1 = s.add_curve(&mut al, Curve::Line { a: o, b: a }, false);
            let l2 = s.add_curve(&mut al, Curve::Line { a: o, b }, false);
            let cota = s.add_constraint(&mut al, Kind::Angle(l1, l2, "60".into()));
            let uv = if caso == "cota_angulo" { [16., 58.] } else { [-14., 62.] };
            leva_cota(&mut s, cota, [0., 50.], uv);
        }
        "setas" => {
            s.add_constraint(&mut al, Kind::Fixed(p[0]));
            s.add_constraint(&mut al, Kind::Length(l[1], "50".into()));
        }
        "pegas" => {
            crate::sketch_ui::filete_com_raio(&mut s, &mut al, p[2], 12.);
        }
        "barra" | "conflito" => {
            let (a, b) = (s.add_point(&mut al, -40., 40.), s.add_point(&mut al, 40., 52.));
            s.add_curve(&mut al, Curve::Line { a, b }, false);
            if caso == "conflito" {
                s.add_constraint(&mut al, Kind::Vertical(l[1]));
            }
        }
        "regiao" => {
            // quadrado dentro do retângulo, vizinho ao lado e cadeia aberta que não faz região
            s.add_rect(&mut al, [-15., -10.], [15., 10.], false);
            s.add_rect(&mut al, [60., -15.], [90., 15.], false);
            let q: Vec<Id> = [[-40., 40.], [0., 55.], [40., 40.]].iter().map(|u| s.add_point(&mut al, u[0], u[1])).collect();
            s.add_curve(&mut al, Curve::Line { a: q[0], b: q[1] }, false);
            s.add_curve(&mut al, Curve::Line { a: q[1], b: q[2] }, false);
        }
        "sombra" | "oculto" | "fatia" => {
            let c = s.add_point(&mut al, 0., 0.);
            s.add_curve(&mut al, Curve::Circle { c, r: 12. }, false);
            let (a, b) = (s.add_point(&mut al, -55., 0.), s.add_point(&mut al, 55., 0.));
            s.add_curve(&mut al, Curve::Line { a, b }, true);
            s.add_constraint(&mut al, Kind::Length(l[0], "80".into()));
            s.add_constraint(&mut al, Kind::Horizontal(l[0]));
            // cadeia aberta ao lado: ela não pinta
            let q: Vec<Id> = [[60., -20.], [80., -20.], [80., 20.]].iter().map(|u| s.add_point(&mut al, u[0], u[1])).collect();
            s.add_curve(&mut al, Curve::Line { a: q[0], b: q[1] }, false);
            s.add_curve(&mut al, Curve::Line { a: q[1], b: q[2] }, false);
        }
        _ => {}
    }
    let sk = doc.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
    if caso == "fatia" {
        // corpo que atravessa o plano do sketch, para o corte aparecer
        let topo = doc.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "-30");
        let mut base = SketchData::default();
        let mut m = 9_800_000u64;
        let mut al2 = || {
            m += 1;
            Id(m)
        };
        base.add_rect(&mut al2, [-30., -30.], [30., 30.], false);
        let bs = doc.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: base }));
        doc.add_feature(FeatureKind::Extrude(Extrude { sketch: bs, op: BoolOp::Add, extent: Extent::Blind("60".into()), reverse: false, taper: "0".into(), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    }
    Some(sk)
}

/// Põe o editor no estado do caso.
pub fn aplicar(e: &mut SketchEdit, sk: &SketchData) {
    let Ok(caso) = std::env::var("FENKRA_DEMO_SKETCH") else { return };
    e.gesto = crate::sketch_gesto::Gesto::do_ambiente();
    // os eixos (B437) também são linhas, mas não são do desenho
    let linhas: Vec<Id> = sk.curves.iter().filter(|c| matches!(c.geom, Curve::Line { .. }) && !sk.e_eixo(c.id)).map(|c| c.id).collect();
    match caso.as_str() {
        "retangulo" => {
            e.set_tool(Tool::RectCenter);
            e.demo_clique([0., 40.]);
            e.demo_cursor = Some([30., 58.]);
        }
        "canto" => e.set_tool(Tool::Fillet),
        "poligono" => {
            e.set_tool(Tool::Polygon);
            e.polygon_sides = 8;
            e.polygon_circunscrito = true;
            e.demo_clique([0., 0.]);
            e.demo_cursor = Some([18., 8.]);
        }
        "chanfro" => {
            e.set_tool(Tool::Chamfer);
            e.chanfro_modo = crate::sketch_canto::ModoChanfro::DistanciaAngulo;
            e.chanfro_angulo = 30.;
            e.definir_canto(true, Some(20.));
            e.demo_cursor = Some([36., 25.]);
        }
        "copia" => {
            e.set_tool(Tool::Copy);
            e.sel = linhas.iter().copied().map(Ent::Cv).collect();
            e.demo_clique([0., 0.]);
            e.demo_cursor = Some([90., 40.]);
        }
        "setas" => {
            e.demo_cursor = Some([0., 25.]);
        }
        "pegas" => {
            let arco = sk.curves.iter().find(|c| matches!(c.geom, Curve::Arc { .. })).map(|c| c.id);
            e.sel = arco.map(Ent::Cv).into_iter().collect();
        }
        "barra" => {
            e.sel = vec![Ent::Cv(linhas[0]), Ent::Cv(*linhas.last().unwrap_or(&linhas[0]))];
            e.demo_opcao = Some(CKind::Parallel);
        }
        "conflito" => {
            e.sel = vec![Ent::Cv(linhas[0]), Ent::Cv(linhas[1])];
            e.demo_opcao = Some(CKind::Parallel);
        }
        "oculto" => {
            e.exibir.cotas = false;
            e.exibir.restricoes = false;
            e.exibir.construcao = false;
            e.exibir.pontos = false;
        }
        "fatia" => e.fatiar = true,
        "aparar" => {
            e.set_tool(Tool::Trim);
            e.demo_cursor = Some([-25., 10.]);
        }
        "estender" => {
            e.set_tool(Tool::Extend);
            e.demo_cursor = Some([17., -10.]);
        }
        "aparar_shift" => {
            e.set_tool(Tool::Trim);
            e.demo_shift = true;
            e.demo_cursor = Some([17., -10.]);
        }
        "janela" => {
            e.demo_janela = Some([-60., -40.]);
            e.demo_cursor = Some([60., 35.]);
        }
        "cruzada" => {
            e.demo_janela = Some([50., 35.]);
            e.demo_cursor = Some([-10., -10.]);
        }
        "cadeia" => e.sel = crate::sketch_cadeia::cadeia(sk, linhas[0]).curvas().into_iter().map(Ent::Cv).collect(),
        "cadeia_offset" => {
            e.set_tool(Tool::Offset);
            e.sel = crate::sketch_cadeia::cadeia(sk, linhas[0]).curvas().into_iter().map(Ent::Cv).collect();
            e.demo_cursor = Some([0., 33.]);
        }
        "snap" => {
            e.set_tool(Tool::Line);
            e.demo_clique_linha(sk, [-20., -60.]);
            e.demo_cursor = Some([1., 26.]);
        }
        "arraste" | "arraste_ctrl" => {
            let solta = sk.curves.iter().rev().find_map(|c| if let Curve::Line { a, .. } = c.geom { Some(a) } else { None });
            if let Some(p) = solta {
                e.demo_arraste(p, sk.pos(p));
            }
            e.demo_ctrl = caso == "arraste_ctrl";
            e.demo_cursor = Some([-39.6, 25.4]);
        }
        "inferencia" => {
            e.set_tool(Tool::Line);
            e.demo_clique_linha(sk, [-90., 60.]);
            e.demo_cursor = Some([-60., 26.]);
        }
        "perpendicular" => {
            e.set_tool(Tool::Line);
            let fim = sk.curves.iter().rev().filter(|c| !sk.e_eixo(c.id)).find_map(|c| if let Curve::Line { b, .. } = c.geom { Some(b) } else { None });
            e.demo_cadeia(fim);
            e.demo_cursor = Some([40. - 12., 75. + 32.]);
        }
        "origem_escolha" => {
            e.posicao.pedido = Some(crate::sketch_posicao_ui::Pedido::CentroFace);
            // sobre a face de cima do corpo, longe do furo (coordenadas do sketch deslocado)
            e.demo_cursor = Some([90., 50.]);
        }
        _ => {}
    }
    if let Some(q) = std::env::var("FENKRA_DEMO_SKETCH_CURSOR").ok().and_then(|t| {
        let (x, y) = t.split_once(',')?;
        Some([x.trim().parse().ok()?, y.trim().parse().ok()?])
    }) {
        e.demo_cursor = Some(q);
    }
}
