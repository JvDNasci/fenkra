// SPDX-License-Identifier: LGPL-2.1-or-later
//! `FENKRA_DEMO_POLIMENTO=<caso>` (B346): peça de prova para conferir o polimento da superfície sem
//! mouse. `esfera`, `cilindro`, `toro`, `superficie`, `filete`, `bolso` e `filete_sup` abrem uma peça só;
//! `FENKRA_DEMO_QUALIDADE=<desvio relativo>,<ângulo em graus>` força a malha, em geral a mais grossa
//! que a configuração aceita (`0.02,45`).

use fenkra_core::*;

fn s(x: &str) -> String {
    x.to_string()
}

fn primitiva(kind: PrimKind, a: &str, b: &str) -> FeatureKind {
    FeatureKind::Primitive(Primitive { kind, plane: PlaneRef::Origin(OriginPlane::XY), a: s(a), b: s(b), c: String::new(), d: String::new(), op: BoolOp::Add })
}

/// Painel de dupla curvatura: quatro splines ligadas por uma superfície multi-seções.
fn superficie(d: &mut Document) {
    let mut secoes = vec![];
    for i in 0..4 {
        let y = i as f64 * 40.;
        let f = [0., 14., -10., 8.][i];
        let pontos = [[-70., y, f], [-30., y, 22. - f], [15., y, -8. + f], [70., y, 12. - f]];
        let c = d.add_geometry(RefKind::Spline { points: pontos.iter().map(|p| PointRef::Fixed(*p)).collect(), closed: false, tangents: vec![] });
        secoes.push(CurveRef::Ref(c));
    }
    d.add_geometry(RefKind::SurfLoft { sections: secoes, guide: None, ruled: false, closed: false, opts: Default::default() });
    for r in d.references.iter_mut().filter(|r| !matches!(r.kind, RefKind::SurfLoft { .. })) {
        r.visible = false;
    }
}

/// Bloco com as arestas de cima arredondadas: planos, cilindros e esferas numa peça só.
fn filete(d: &mut Document) {
    let mut data = SketchData::default();
    let mut n = 9_800_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [-40., -25.], [40., 25.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("30")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    let topo = vec![[0., -25., 30.], [40., 0., 30.], [0., 25., 30.], [-40., 0., 30.], [40., 25., 15.], [-40., 25., 15.], [40., -25., 15.], [-40., -25., 15.]];
    d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("10"), edges: EdgeSel::Edges(topo), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
}

/// Bloco com bolso de paredes retas: o caso das marcas nos cantos do render (B10).
fn bolso(d: &mut Document) {
    let mut n = 9_810_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let mut fora = SketchData::default();
    fora.add_rect(&mut al, [-40., -40.], [40., 40.], false);
    let mut dentro = SketchData::default();
    dentro.add_rect(&mut al, [-22., -22.], [22., 22.], false);
    let extrusao = |sketch, op, dist: &str, reverse| FeatureKind::Extrude(Extrude { sketch, op, extent: Extent::Blind(s(dist)), reverse, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() });
    let a = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: fora }));
    d.add_feature(extrusao(a, BoolOp::Add, "60", false));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "60");
    let b = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: dentro }));
    d.add_feature(extrusao(b, BoolOp::Cut, "35", false));
    for r in &mut d.references {
        r.visible = false;
    }
}

/// Duas superfícies ligadas por um filete de superfície (B10).
fn filete_de_superficie(d: &mut Document) {
    let linha = |d: &mut Document, a, b| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }));
    let c_base = linha(d, [0., -50., 0.], [100., -50., 0.]);
    let base = d.add_geometry(RefKind::SurfExtrude { curve: c_base, dir: AxisRef::Origin(OriginAxis::Y), length: s("60"), length2: s("0"), until: None, until2: None });
    let c_parede = linha(d, [50., -50., -20.], [50., 10., -20.]);
    let parede = d.add_geometry(RefKind::SurfExtrude { curve: c_parede, dir: AxisRef::Origin(OriginAxis::Z), length: s("50"), length2: s("0"), until: None, until2: None });
    let filete = d.add_geometry(RefKind::SurfFillet { a: SurfRef::Ref(base), b: SurfRef::Ref(parede), flip_a: false, flip_b: false, radius: s("8"), radius2: None });
    for r in d.references.iter_mut().filter(|r| r.id != filete) {
        r.visible = false;
    }
}

/// Documento do caso pedido.
pub fn documento(caso: &str) -> Document {
    let mut d = Document::new("Prova de polimento");
    match caso {
        "cilindro" => {
            d.add_feature(primitiva(PrimKind::Cilindro, "30", "80"));
        }
        "toro" => {
            d.add_feature(primitiva(PrimKind::Toro, "40", "15"));
        }
        "superficie" => superficie(&mut d),
        "filete" => filete(&mut d),
        "bolso" => bolso(&mut d),
        "filete_sup" => filete_de_superficie(&mut d),
        _ => {
            d.add_feature(primitiva(PrimKind::Esfera, "40", ""));
        }
    }
    d
}

/// `FENKRA_DEMO_GIRO=<graus por quadro>`: gira a câmera sem parar, para medir o quadro com
/// `FENKRA_MEDIR_QUADROS`.
pub fn giro(ctx: &eframe::egui::Context, vp: &mut crate::render::Viewport) {
    let Some(g) = std::env::var("FENKRA_DEMO_GIRO").ok().and_then(|v| v.parse::<f32>().ok()) else { return };
    vp.camera.yaw += g.to_radians();
    ctx.request_repaint();
}

/// Qualidade forçada por `FENKRA_DEMO_QUALIDADE`.
pub fn qualidade() -> Option<QualidadeMalha> {
    let v = std::env::var("FENKRA_DEMO_QUALIDADE").ok()?;
    let (a, b) = v.split_once(',')?;
    Some(QualidadeMalha { desvio_rel: a.trim().parse().ok()?, angulo_graus: b.trim().parse().ok()? })
}
