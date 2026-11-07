// SPDX-License-Identifier: LGPL-2.1-or-later
//! Variações das peças do kart que exercitam os comandos ainda não usados em cada bancada.
use crate::apoio::*;
use crate::pecas::{aba, chapa, nome, parede, pc, Pub};
use crate::quadro::membro;
use fenkra_core::*;

type R = Result<Document, String>;

pub fn assoalho_estampado() -> R {
    let mut d = Document::new("c");
    chapa(&mut d, "1.5");
    let sk = sk_rect(&mut d, oxy(), [0., 0.], [300., 200.]);
    parede(&mut d, sk);
    let t = d.add_offset_plane(oxy(), "1.5");
    let e = sk_circ(&mut d, PlaneRef::Datum(t), &[([70., 100.], 30.)]);
    d.add_feature(FeatureKind::SmStamp(SmStamp { sketch: e, depth: "3".into(), flip: false, kind: 0, width: String::new(), punch: None }));
    d.add_feature(FeatureKind::SmLouver(SmLouver { a: [160., 50., 1.5], b: [260., 50., 1.5], width: "20".into(), height: "5".into(), flip: false, down: false }));
    let f = sk_pts(&mut d, PlaneRef::Datum(t), &[[210., 140.]]);
    d.add_feature(FeatureKind::SmFlangedHole(SmFlangedHole { sketch: f, diameter: "20".into(), height: "6".into(), flip: false, points: vec![], standard: String::new() }));
    Ok(d)
}

pub fn protetor_bainha() -> R {
    let mut d = Document::new("c");
    chapa(&mut d, "1.5");
    let sk = sk_rect(&mut d, oxy(), [0., 0.], [290., 60.]);
    parede(&mut d, sk);
    d.add_feature(FeatureKind::SmHem(SmHem { edge: [145., 60., 1.5], length: "10".into(), gap: "0".into(), flip: false, kind: 0, edge_key: None }));
    d.add_feature(FeatureKind::SmJoggle(SmJoggle { a: [100., 0., 1.5], b: [100., 60., 1.5], depth: "6".into(), run: "12".into(), flip: false, angle: None, plane: None, keep_length: false }));
    Ok(d)
}

pub fn caixa_alivio() -> R {
    let mut d = Document::new("c");
    chapa(&mut d, "2");
    let sk = sk_rect(&mut d, oxy(), [0., 0.], [120., 80.]);
    parede(&mut d, sk);
    aba(&mut d, [60., 0., 2.], "30", "90");
    aba(&mut d, [120., 40., 2.], "30", "90");
    d.add_feature(FeatureKind::SmCornerRelief(SmCornerRelief { at: [120., 0., 2.], size: "6".into(), kind: 0 }));
    Ok(d)
}

pub fn canal_convertido() -> R {
    let mut d = Document::new("c");
    let sk = sk_poli(&mut d, oxy(), &[[0., 0.], [60., 0.], [60., 40.], [56., 40.], [56., 4.], [4., 4.], [4., 40.], [0., 40.]]);
    d.add_feature(ext(sk, BoolOp::Add, blind("150")));
    d.sheet = Some(SheetParams { thickness: "4".into(), radius: "0.5".into(), k: "0.44".into(), ..Default::default() });
    d.add_feature(FeatureKind::SmConvert(SmConvert { face: [30., 0., 75.], thickness: "4".into() }));
    Ok(d)
}

/// Pinça com furo rebaixado, furo roscado cosmético e modelado (B373), espessura e reforço.
pub fn furos_e_rosca() -> R {
    let mut d = Document::new("c");
    let b = sk_rect(&mut d, oxy(), [-45., -32.], [45., 32.]);
    d.add_feature(ext(b, BoolOp::Add, blind("50")));
    let t = d.add_offset_plane(oxy(), "50");
    let base = Hole { sketch: Id(0), kind: HoleKind::Simples, diameter: "6.8".into(), depth: Extent::Blind("25".into()), thread: Some("M8x1.25".into()), thread_modeled: false, standard: Some("M8 normal".into()), points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default() };
    let s1 = sk_pts(&mut d, PlaneRef::Datum(t), &[[-30., 0.]]);
    d.add_feature(FeatureKind::Hole(Hole { sketch: s1, ..base.clone() }));
    let s2 = sk_pts(&mut d, PlaneRef::Datum(t), &[[0., 0.]]);
    d.add_feature(FeatureKind::Hole(Hole { sketch: s2, thread_modeled: true, ..base.clone() }));
    let s3 = sk_pts(&mut d, PlaneRef::Datum(t), &[[30., 0.]]);
    d.add_feature(FeatureKind::Hole(Hole { sketch: s3, kind: HoleKind::Rebaixado { diameter: "14".into(), depth: "8".into() }, diameter: "8.5".into(), depth: Extent::ThroughAll, thread: None, standard: None, ..base.clone() }));
    let s4 = sk_pts(&mut d, PlaneRef::Datum(t), &[[0., 22.]]);
    d.add_feature(FeatureKind::Hole(Hole { sketch: s4, kind: HoleKind::Escareado { diameter: "12".into(), angle: "90".into() }, diameter: "6.5".into(), depth: Extent::ThroughAll, thread: None, standard: None, ..base.clone() }));
    d.add_feature(FeatureKind::Thickness(Thickness { faces: vec![[45., 0., 25.]], value: "3".into(), face_keys: vec![], remove: false }));
    Ok(d)
}

/// Suporte do motor com nervura, escala, dividir corpo e padrão por pontos.
pub fn nervura_escala() -> R {
    let mut d = Document::new("c");
    let b = sk_rect(&mut d, oxy(), [-100., -60.], [100., 60.]);
    d.add_feature(ext(b, BoolOp::Add, blind("10")));
    let m = d.add_offset_plane(oxz(), "0");
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        let a = data.add_point(&mut al, -60., 10.);
        let c = data.add_point(&mut al, 60., 10.);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a, b: c }, false);
    }
    let s = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(m), data }));
    d.add_feature(FeatureKind::Stiffener(Stiffener { sketch: s, thickness: "6".into(), depth: "30".into(), reverse: false, to_body: false, side: 0 }));
    d.add_feature(FeatureKind::Scale(Scale { center: PointRef::Origin, factor: "1.1".into() }));
    let c = d.add_offset_plane(oyz(), "60");
    d.add_feature(FeatureKind::SplitBody(SplitBody { surface: SurfRef::Plane(PlaneRef::Datum(c)), flip: false }));
    Ok(d)
}

/// Linha de freio, segunda tentativa: varredura sólida de um círculo na mesma spline.
pub fn linha_varredura(m: &Pub) -> R {
    let mut d = Document::new("c");
    crate::mestre::liga(&mut d, m, &["EE"], &[])?;
    let pts: Vec<Id> = [("600", "-120", "60"), ("420", "-300", "60"), ("260", "-330", "90"), ("120", "-280", "120"), ("60", "-160", "110")].iter().enumerate().map(|(i, (x, y, z))| ponto(&mut d, &format!("L{i}"), &format!("{x}*mestre_EE/1040"), y, z)).collect();
    let c = d.add_geometry(RefKind::Spline { points: pts.iter().map(|p| PointRef::Ref(*p)).collect(), closed: false, tangents: vec![] });
    let pl = d.add_geometry(RefKind::PlaneNormalCurve { curve: CurveRef::Ref(c), at: OnCurve::Ratio("0".into()), reverse: false, point: None, project: false });
    let sk = sk_circ(&mut d, PlaneRef::Datum(pl), &[([0., 0.], 2.5), ([0., 0.], 1.5)]);
    d.add_feature(FeatureKind::Sweep(Sweep { sketch: sk, spine: CurveRef::Ref(c), mode: SweepMode::Corrigido, op: BoolOp::Add, thin: None }));
    Ok(d)
}

/// Quadro de teste com placa de ponta, reforço e cordão de solda.
pub fn estrutural_extra() -> R {
    let mut d = Document::new("c");
    let a = ponto(&mut d, "A", "0", "0", "0");
    let b = ponto(&mut d, "B", "400", "0", "0");
    let c = ponto(&mut d, "C", "400", "300", "0");
    let l1 = d.add_geometry(RefKind::Polyline { points: vec![PointRef::Ref(a), PointRef::Ref(b)], closed: false });
    let l2 = d.add_geometry(RefKind::Polyline { points: vec![PointRef::Ref(b), PointRef::Ref(c)], closed: false });
    let mut t = tubo(vec![CurveRef::Ref(l1), CurveRef::Ref(l2)], "30", "2.4", "Ø30x2,4");
    t.corner = CornerMode::MeiaEsquadria;
    let f = membro(&mut d, "Canto", t);
    nome(&mut d, f, "Canto em meia-esquadria");
    d.add_feature(FeatureKind::StructPlate(StructPlate::nova([0., 0., 0.], "6", "8")));
    d.add_feature(FeatureKind::StructGusset(StructGusset::novo([400., 0., 0.], "60", "4", "10")));
    let mut w = StructWeld::novo("3");
    w.faces = vec![[0., 15., 15.], [-6., 0., 0.]];
    w.face_keys = vec![None, None];
    w.split = 1;
    d.add_feature(FeatureKind::StructWeld(w));
    Ok(d)
}

/// Superfícies: extrusão, deslocada, cilindro, fechar e costurar sobre a concha do tanque.
pub fn superficies() -> R {
    let mut d = Document::new("c");
    let pts: Vec<Id> = [("-100", "0"), ("-40", "50"), ("40", "50"), ("100", "0")].iter().enumerate().map(|(i, (x, y))| ponto(&mut d, &format!("P{i}"), x, y, "0")).collect();
    let c = d.add_geometry(RefKind::Spline { points: pts.iter().map(|p| PointRef::Ref(*p)).collect(), closed: false, tangents: vec![] });
    let e = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(c), dir: AxisRef::Origin(OriginAxis::Z), length: "120".into(), length2: "0".into(), until: None, until2: None });
    let o = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(e), dist: "5".into(), both: false, join: Default::default() });
    d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(o), t1: "2".into(), t2: "0".into(), op: BoolOp::Add }));
    Ok(d)
}

pub fn preencher() -> R {
    let mut d = Document::new("c");
    let p: Vec<Id> = [("0", "0", "0"), ("200", "0", "20"), ("200", "150", "0"), ("0", "150", "30")].iter().enumerate().map(|(i, (x, y, z))| ponto(&mut d, &format!("Q{i}"), x, y, z)).collect();
    let mut cs = vec![];
    for i in 0..4 {
        let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Ref(p[i]), b: PointRef::Ref(p[(i + 1) % 4]), start: "0".into(), end: "0".into() });
        cs.push(CurveRef::Ref(l));
    }
    let s = d.add_geometry(RefKind::SurfFill { curves: cs, conds: vec![], points: vec![], tol: "0.1".into() });
    d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(s), t1: "1.5".into(), t2: "0".into(), op: BoolOp::Add }));
    Ok(d)
}

/// Filete variável não entra: eixo escalonado do gerador e padrão por pontos.
pub fn padrao_pontos() -> R {
    let mut d = Document::new("c");
    let b = sk_rect(&mut d, oxy(), [-60., -60.], [60., 60.]);
    d.add_feature(ext(b, BoolOp::Add, blind("10")));
    let t = d.add_offset_plane(oxy(), "10");
    let c = sk_pts(&mut d, PlaneRef::Datum(t), &[[-40., -40.]]);
    let h = d.add_feature(furo(c, "6"));
    d.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Points { points: vec![PointRef::Fixed([40., -40., 10.]), PointRef::Fixed([40., 40., 10.]), PointRef::Fixed([-40., 40., 10.])], base: PointRef::Fixed([-40., -40., 10.]) }, sources: vec![h], skip: vec![] }));
    pc(&mut d, FeatureKind::Chamfer(EdgeOp { value: "2".into(), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
    Ok(d)
}
