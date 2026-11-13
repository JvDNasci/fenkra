// SPDX-License-Identifier: LGPL-2.1-or-later
//! Casos dos roteiros de mídia (B257): o documento de partida e o comando que o roteiro abre como
//! novo. Montados em código, sem arquivo, para a mídia sair igual em qualquer máquina.

use fenkra_core::*;

/// Casos de cota do PMI 3D (B185): a peça e as anotações já colocadas; o roteiro anima campos da
/// lista de anotações (`/0/offset/1`, `/0/style/arc_r`).
pub const CASOS_PMI: [&str; 9] = ["pmi_linear", "pmi_ordenada", "pmi_cadeia", "pmi_base", "pmi_raio", "pmi_diametro", "pmi_feature", "pmi_angulo", "pmi_arco"];

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

/// Caixa 60 x 40 x 30 com um furo passante de raio 8 em (10, 0). Devolve o comando do furo.
fn caixa_com_furo(d: &mut Document) -> Id {
    caixa(d);
    // o recorte entra no material a partir do plano do sketch: desenhado no topo, desce os 30 mm
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30");
    // a mídia mostra só a peça e a cota: plano e origem ficam escondidos
    if let Some(r) = d.references.iter_mut().find(|r| r.id == topo) {
        r.visible = false;
    }
    d.origin_visible = false;
    let sk = sketch(d, PlaneRef::Datum(topo), &|data, al| {
        let c = data.add_point(&mut &mut *al, 10., 0.);
        data.add_curve(&mut &mut *al, fenkra_core::sketch::Curve::Circle { c, r: 8. }, false);
    });
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Cut, extent: Extent::Blind(s("30")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }))
}

/// Documento de partida e as anotações do caso de cota; `None` se o nome não é de cota.
pub fn montar_pmi(caso: &str) -> Option<(Document, Vec<fenkra_core::pmi::Pmi>)> {
    use fenkra_core::pmi::{Pmi, PmiKind};
    if !CASOS_PMI.contains(&caso) {
        return None;
    }
    let mut d = Document::new("Mídia");
    let furo = caixa_com_furo(&mut d);
    let mut nova = |d: &mut Document, kind: PmiKind, anchors: Vec<[f64; 3]>, offset: [f64; 3]| Pmi { id: d.alloc(), name: String::new(), kind, anchors, offset, text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] };
    // aresta de cima e da frente (y = -20, z = 30), de x = -30 a 30; o texto sai para a frente
    let frente = |x: f64| [x, -20., 30.];
    let lista = match caso {
        "pmi_linear" => vec![nova(&mut d, PmiKind::Linear, vec![frente(-30.), frente(30.)], [30., -4., 0.])],
        "pmi_ordenada" => vec![nova(&mut d, PmiKind::Ordinate { axis: 0 }, vec![frente(-30.), frente(10.)], [20., -4., 0.])],
        "pmi_cadeia" => [(-30., -10.), (-10., 10.), (10., 30.)].iter().map(|(a, b)| nova(&mut d, PmiKind::Linear, vec![frente(*a), frente(*b)], [(b - a) / 2., -4., 0.])).collect(),
        "pmi_base" => [-10., 10., 30.].iter().map(|b| nova(&mut d, PmiKind::Linear, vec![frente(-30.), frente(*b)], [(b + 30.) / 2., -4., 0.])).collect(),
        "pmi_raio" => vec![nova(&mut d, PmiKind::Radius, vec![[10., 0., 30.], [15.657, -5.657, 30.]], [8., -8., 0.])],
        "pmi_diametro" => vec![nova(&mut d, PmiKind::Diameter, vec![[10., 0., 30.], [15.657, -5.657, 30.]], [8., -8., 0.])],
        "pmi_feature" => vec![nova(&mut d, PmiKind::FeatureNote { feature: furo, comp: Id(0) }, vec![[15.657, -5.657, 30.]], [8., -8., 4.])],
        "pmi_angulo" => {
            let mut a = nova(&mut d, PmiKind::Angle, vec![[-30., -20., 0.], [30., -20., 0.], [-30., -20., 30.]], [10., 0., 10.]);
            a.style.arc_r = 6.;
            vec![a]
        }
        "pmi_arco" => vec![nova(&mut d, PmiKind::ArcLength, vec![[10., 0., 30.], [18., 0., 30.], [10., -8., 30.]], [6., -6., 0.])],
        _ => return None,
    };
    Some((d, lista))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn todo_caso_de_cota_monta_a_peca_e_as_anotacoes() {
        for c in CASOS_PMI {
            let (d, lista) = montar_pmi(c).unwrap_or_else(|| panic!("{c} não monta"));
            assert!(!lista.is_empty(), "{c}: sem anotação");
            for p in &lista {
                assert_eq!(p.anchors.len(), p.kind.anchors(), "{c}: âncoras");
            }
            let r = fenkra_solidos::recompute(&d);
            assert!(r.status.values().all(|s| !matches!(s, fenkra_solidos::Status::Error(_))), "{c}: {:?}", r.status);
            // caixa menos o furo passante
            let v: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
            assert!((v - (60. * 40. * 30. - std::f64::consts::PI * 64. * 30.)).abs() < 1., "{c}: volume {v}");
        }
        assert!(montar_pmi("casca").is_none() && montar("pmi_linear").is_none());
    }

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
