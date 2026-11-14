// SPDX-License-Identifier: LGPL-2.1-or-later
//! B148: casos de `FENKRA_DEMO_PREVIA` dos recursos de plástico. Cada um monta uma caixa injetada
//! (casca aberta em cima, com a regra plástica ABS na peça) e devolve o recurso a abrir como novo,
//! para a captura mostrar o painel na coluna, a seta e a prévia sem mouse.

use crate::previa::DemoPrevia;
use fenkra_core::regra_plastica::RegraPlastica;
use fenkra_core::sketch::Curve;
use fenkra_core::*;

fn s(x: &str) -> String {
    x.to_string()
}

fn sketch(d: &mut Document, plane: PlaneRef, monta: &dyn Fn(&mut SketchData, &mut dyn FnMut() -> Id)) -> Id {
    let mut data = SketchData::default();
    let mut n = 9_800_000u64 + d.features.len() as u64 * 1000;
    let mut al = || {
        n += 1;
        Id(n)
    };
    monta(&mut data, &mut al);
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

/// Caixa 60 x 40 x 30 com casca de parede da regra, aberta em cima; devolve o plano do piso interno.
fn caixa(d: &mut Document) -> PlaneRef {
    *d = Document::new("Caixa injetada");
    d.plastico = RegraPlastica::de_fabrica().into_iter().find(|r| r.material == "ABS");
    let sk = sketch(d, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
        data.add_rect(&mut &mut *al, [-30., -20.], [30., 20.], false);
    });
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("30")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0., 0., 30.]], thickness: s("plastico_espessura"), outward: false, face_keys: vec![], closed: false, regras: vec![] }));
    PlaneRef::Datum(d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "plastico_espessura"))
}

pub fn demo(nome: &str, doc: &mut Document) -> Option<DemoPrevia> {
    match nome {
        "boss" => {
            let piso = caixa(doc);
            let sk = sketch(doc, piso, &|data, al| {
                for p in [[-18., -8.], [18., 8.]] {
                    let id = data.add_point(&mut &mut *al, p[0], p[1]);
                    data.add_curve(&mut &mut *al, Curve::Point { p: id }, false);
                }
            });
            let mut b = recurso_plastico::Boss::novo(sk, doc.plastico.as_ref());
            b.diametro = s("7");
            b.altura = s("22");
            b.inserto = s("4.2");
            b.inserto_prof = s("4");
            Some(DemoPrevia::Comando(FeatureKind::Plastico(recurso_plastico::RecursoPlastico::Boss(b))))
        }
        _ => None,
    }
}
