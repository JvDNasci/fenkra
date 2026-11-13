// SPDX-License-Identifier: LGPL-2.1-or-later
//! Casos dos roteiros de mídia (B257): o documento de partida e o comando que o roteiro abre como
//! novo. Montados em código, sem arquivo, para a mídia sair igual em qualquer máquina.

use fenkra_core::*;

/// Nomes dos casos, na ordem em que a esteira gera as mídias.
pub const CASOS: [&str; 16] = ["extrusao", "revolucao", "casca", "espessura", "rosca", "prim_caixa", "prim_cilindro", "prim_esfera", "prim_cone", "reforco", "filete", "chanfro", "furo", "saida", "recorte", "padrao_linear"];

fn s(x: &str) -> String {
    x.to_string()
}

fn sketch(d: &mut Document, plane: PlaneRef, monta: &dyn Fn(&mut SketchData, &mut dyn FnMut() -> Id)) -> Id {
    let mut data = SketchData::default();
    let mut n = 9_600_000u64 + d.features.len() as u64 * 1000;
    let mut al = || {
        n += 1;
        Id(n)
    };
    monta(&mut data, &mut al);
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

fn caixa(d: &mut Document) {
    let sk = sketch(d, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
        data.add_rect(&mut &mut *al, [-30., -20.], [30., 20.], false);
    });
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("30")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
}

fn primitivo(kind: PrimKind, a: &str, b: &str, c: &str) -> FeatureKind {
    FeatureKind::Primitive(Primitive { kind, plane: PlaneRef::Origin(OriginPlane::XY), a: s(a), b: s(b), c: s(c), d: s("0"), op: BoolOp::Add })
}

/// Documento de partida e comando novo do caso; `None` se o nome não existe.
pub fn montar(caso: &str) -> Option<(Document, FeatureKind)> {
    let mut d = Document::new("Mídia");
    let kind = match caso {
        "extrusao" => {
            let sk = sketch(&mut d, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
                data.add_rect(&mut &mut *al, [-30., -20.], [30., 20.], false);
            });
            FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("5")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() })
        }
        "revolucao" => {
            let sk = sketch(&mut d, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
                data.add_rect(&mut &mut *al, [10., 0.], [26., 34.], false);
            });
            FeatureKind::Revolve(Revolve { sketch: sk, axis: AxisRef::Origin(OriginAxis::Y), angle: s("30"), symmetric: false, op: BoolOp::Add, thin: None, thin_side: 0, angle2: None })
        }
        "casca" => {
            caixa(&mut d);
            FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0., 0., 30.]], thickness: s("1"), outward: false, face_keys: vec![], closed: false, regras: vec![] })
        }
        "espessura" => {
            caixa(&mut d);
            FeatureKind::Thickness(Thickness { faces: vec![[0., 0., 30.]], value: s("0"), face_keys: vec![], remove: false, regras: vec![] })
        }
        "rosca" => {
            d.add_feature(primitivo(PrimKind::Cilindro, "6", "40", "0"));
            FeatureKind::Thread(Thread { face: [6., 0., 20.], spec: s("M10x1.5"), length: s("4"), modeled: false, end: None, start: s("6"), face_key: None })
        }
        "prim_caixa" => primitivo(PrimKind::Caixa, "20", "20", "10"),
        "prim_cilindro" => primitivo(PrimKind::Cilindro, "6", "40", "0"),
        "prim_esfera" => primitivo(PrimKind::Esfera, "5", "0", "0"),
        "prim_cone" => primitivo(PrimKind::Cone, "25", "24", "40"),
        "reforco" => {
            caixa(&mut d);
            let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30");
            let sk = sketch(&mut d, PlaneRef::Datum(topo), &|data, al| {
                let a = data.add_point(&mut &mut *al, -25., 0.);
                let b = data.add_point(&mut &mut *al, 25., 0.);
                data.add_curve(&mut &mut *al, fenkra_core::sketch::Curve::Line { a, b }, false);
            });
            FeatureKind::Stiffener(Stiffener { sketch: sk, thickness: s("2"), depth: s("5"), reverse: true, to_body: false, side: 1 })
        }
        // B60: mídias dos comandos de sólidos que faltavam, todas sobre a mesma caixa 60 x 40 x 30
        "filete" | "chanfro" => {
            caixa(&mut d);
            let op = EdgeOp { value: s("1"), edges: EdgeSel::Edges(vec![[0., 20., 30.], [0., -20., 30.], [30., 0., 30.], [-30., 0., 30.]]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() };
            if caso == "filete" {
                FeatureKind::Fillet(op)
            } else {
                FeatureKind::Chamfer(op)
            }
        }
        "furo" => {
            caixa(&mut d);
            // centro num sketch do topo: com ponto 3D solto o furo espera o clique e não mostra a prévia
            let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30");
            let centro = sketch(&mut d, PlaneRef::Datum(topo), &|data, al| {
                let p = data.add_point(&mut &mut *al, 0., 0.);
                data.add_curve(&mut &mut *al, fenkra_core::sketch::Curve::Point { p }, false);
            });
            FeatureKind::Hole(Hole {
                sketch: centro,
                kind: HoleKind::Simples,
                diameter: s("4"),
                depth: Extent::Blind(s("20")),
                thread: None,
                thread_modeled: false,
                standard: None,
                points: vec![],
                dir: None,
                reverse: true,
                tip: None,
                taper: s("0"),
                left_hand: false,
                extra: Default::default(),
            })
        }
        "saida" => {
            caixa(&mut d);
            FeatureKind::Draft(Draft { faces: vec![[0., -20., 15.]], neutral: PlaneRef::Origin(OriginPlane::XY), angle: s("1"), reverse: false, face_keys: vec![], regras: vec![] })
        }
        "recorte" => {
            caixa(&mut d);
            let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30");
            let sk = sketch(&mut d, PlaneRef::Datum(topo), &|data, al| {
                data.add_rect(&mut &mut *al, [-12., -10.], [12., 10.], false);
            });
            FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Cut, extent: Extent::Blind(s("2")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() })
        }
        "padrao_linear" => {
            caixa(&mut d);
            let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30");
            let sk = sketch(&mut d, PlaneRef::Datum(topo), &|data, al| {
                data.add_rect(&mut &mut *al, [-26., -4.], [-20., 4.], false);
            });
            let ressalto = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("6")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
            FeatureKind::Pattern(Pattern {
                kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: s("4"), spacing: s("8"), dir2: None, count2: s("1"), spacing2: s("10"), total: false },
                sources: vec![ressalto],
                skip: vec![],
            })
        }
        _ => return None,
    };
    Some((d, kind))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn todo_caso_monta_documento_e_comando() {
        for c in CASOS {
            let (d, k) = montar(c).unwrap_or_else(|| panic!("{c} não monta"));
            // o comando novo ainda não está no documento: o app o adiciona ao abrir
            assert!(d.features.iter().all(|f| f.kind != k), "{c}: comando já no documento");
        }
        assert!(montar("nao_existe").is_none());
    }
}
