// SPDX-License-Identifier: LGPL-2.1-or-later
//! B351: casos de `FENKRA_DEMO_PREVIA` da rosca em furo, para a captura mostrar a prévia em linhas e a
//! textura da rosca cosmética sem mouse.
//!
//! - `rosca_furo`: bloco com um furo escareado de rosca cosmética à direita e um passante de rosca
//!   cosmética à esquerda (os dois já concluídos); o comando aberto é um terceiro furo escareado de
//!   rosca modelada, cuja prévia são só linhas;
//! - `rosca_furo_modelada`: o mesmo bloco, com o comando aberto sobre um furo cego simples modelado;
//! - `rosca_interna`: furo passante e o comando Rosca na face dele, modelada.
//!
//! Com `FENKRA_DEMO_CONCLUIR=1` o comando é concluído logo depois de aberto (o sulco aparece).

use crate::previa::DemoPrevia;
use fenkra_core::sketch::Curve;
use fenkra_core::*;

fn s(x: &str) -> String {
    x.to_string()
}

fn pontos(d: &mut Document, plano: PlaneRef, xs: &[f64]) -> Id {
    let mut data = SketchData::default();
    let mut n = 9_800_000u64 + d.features.len() as u64 * 1000;
    let mut al = || {
        n += 1;
        Id(n)
    };
    for x in xs {
        let p = data.add_point(&mut al, *x, 0.);
        data.add_curve(&mut al, Curve::Point { p }, false);
    }
    d.add_feature(FeatureKind::Sketch(Sketch { plane: plano, data }))
}

fn bloco(d: &mut Document) -> PlaneRef {
    *d = Document::new("Rosca em furo");
    let mut data = SketchData::default();
    let mut n = 9_700_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [-45., -20.], [45., 20.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("30")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    PlaneRef::Datum(d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30"))
}

fn furo(sk: Id, kind: HoleKind, depth: Extent, modelada: bool, esquerda: bool) -> Hole {
    Hole { sketch: sk, kind, diameter: s("6.8"), depth, thread: Some(s("M8x1.25")), thread_modeled: modelada, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: s("0"), left_hand: esquerda, extra: Default::default() }
}

fn escareado() -> HoleKind {
    HoleKind::Escareado { diameter: s("16"), angle: s("90") }
}

pub fn demo(nome: &str, doc: &mut Document) -> Option<DemoPrevia> {
    match nome {
        "rosca_furo" | "rosca_furo_modelada" => {
            let topo = bloco(doc);
            let a = pontos(doc, topo, &[-30.]);
            doc.add_feature(FeatureKind::Hole(furo(a, escareado(), Extent::Blind(s("20")), false, false)));
            let b = pontos(doc, topo, &[0.]);
            doc.add_feature(FeatureKind::Hole(furo(b, HoleKind::Simples, Extent::ThroughAll, false, true)));
            let c = pontos(doc, topo, &[30.]);
            let aberto = if nome == "rosca_furo" { furo(c, escareado(), Extent::Blind(s("20")), true, false) } else { furo(c, HoleKind::Simples, Extent::Blind(s("20")), true, false) };
            Some(DemoPrevia::Comando(FeatureKind::Hole(aberto)))
        }
        "rosca_interna" => {
            let topo = bloco(doc);
            let a = pontos(doc, topo, &[0.]);
            doc.add_feature(FeatureKind::Hole(Hole { thread: None, diameter: s("6.8"), ..furo(a, escareado(), Extent::Blind(s("20")), false, false) }));
            Some(DemoPrevia::Comando(FeatureKind::Thread(Thread { face: [3.4, 0., 18.], spec: s("M8x1.25"), length: s("14"), modeled: true, end: None, start: s("0"), face_key: None })))
        }
        // B373: furo M16x2 cego, rosca modelada com chanfro de entrada e início a 120°; com
        // `FENKRA_DEMO_CONCLUIR=1` o sulco real aparece e a câmera olha de perto a boca do furo
        "rosca_m16" | "rosca_m16_aberto" => {
            let topo = bloco(doc);
            let a = pontos(doc, topo, &[0.]);
            let mut h = Hole { diameter: s("14"), thread: Some(s("M16x2")), ..furo(a, HoleKind::Simples, Extent::Blind(s("20")), true, false) };
            h.extra.thread_chamfer = s("1");
            h.extra.thread_start_angle = s("120");
            if nome == "rosca_m16" {
                // a captura do sulco: o furo já concluído e o painel de outro comando
                doc.add_feature(FeatureKind::Hole(h));
                let b = pontos(doc, topo, &[30.]);
                Some(DemoPrevia::Comando(FeatureKind::Hole(Hole { diameter: s("14"), thread: Some(s("M16x2")), ..furo(b, HoleKind::Simples, Extent::Blind(s("20")), true, false) })))
            } else {
                Some(DemoPrevia::Comando(FeatureKind::Hole(h)))
            }
        }
        _ => None,
    }
}
