// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desenvolvimento: `FENKRA_DEMO_SKETCH=<caso>` abre um sketch pronto, com o estado que só o mouse
//! produziria, para conferir por captura. Casos: `retangulo` e `canto` (painel do comando), `setas`,
//! `pegas`, `barra`, `conflito` (fantasma das restrições), `sombra`, `oculto` e `fatia`.

use crate::sketch_ui::{CKind, Ent, SketchEdit, Tool};
use fenkra_core::sketch::{Curve, Kind};
use fenkra_core::*;

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
    let (p, l) = s.add_rect(&mut al, [-40., -25.], [40., 25.], false);
    match caso.as_str() {
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
    let linhas: Vec<Id> = sk.curves.iter().filter(|c| matches!(c.geom, Curve::Line { .. })).map(|c| c.id).collect();
    match caso.as_str() {
        "retangulo" => {
            e.set_tool(Tool::RectCenter);
            e.demo_clique([0., 40.]);
            e.demo_cursor = Some([30., 58.]);
        }
        "canto" => e.set_tool(Tool::Fillet),
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
        _ => {}
    }
}
